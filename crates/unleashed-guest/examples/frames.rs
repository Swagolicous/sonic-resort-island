//! Runs the lifted per-frame update repeatedly from a snapshot and prints Sonic's
//! character proxy (position +0x120, velocity +0x130) each frame.
//!
//! usage: cargo run --release -p unleashed-guest --example frames -- <X_pre.bin> [frames]

use xenon_rt::*;

struct H(kernel::Kernel, std::collections::BTreeSet<u32>);
impl Host for H {
    fn missing(&mut self, address: u32, ctx: &mut PPCContext, _base: *mut u8) {
        self.1.insert(address);
        ctx.r3.u64 = 0;
    }
    fn import(&mut self, name: &'static str, ctx: &mut PPCContext, base: *mut u8) {
        if !self.0.import(name, ctx, base) {
            ctx.r3.u64 = 0;
        }
    }
    fn hook(&mut self, address: u32, ctx: &mut PPCContext, base: *mut u8) {
        assert!(self.0.hook(address, ctx, base), "host function {address:08X}");
    }
}

const PROXY_VTABLE: u32 = 0x820C0444;

fn rd(base: *mut u8, a: u32) -> u32 {
    unsafe { PPC_LOAD_U32(base, a) }
}
fn rf(base: *mut u8, a: u32) -> f32 {
    f32::from_bits(rd(base, a))
}

/// The proxy as the capture build finds it: a pointer at context+o, or one more hop.
fn find_proxy(base: *mut u8, context: u32) -> Option<(u32, String)> {
    let ok = |p: u32| p >= 0x1000 && p & 3 == 0 && rd(base, p) == PROXY_VTABLE;
    for o in (0..0x1000).step_by(4) {
        let p = rd(base, context + o);
        if ok(p) {
            return Some((p, format!("ctx+{o:#X}")));
        }
    }
    for o in (0..0x1000).step_by(4) {
        let q = rd(base, context + o);
        if q < 0x1000 || q & 3 != 0 {
            continue;
        }
        for o2 in (0..0x400).step_by(4) {
            let p = rd(base, q + o2);
            if ok(p) {
                return Some((p, format!("*(ctx+{o:#X})+{o2:#X}")));
            }
        }
    }
    None
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: frames <X_pre.bin> [frames]");
    let frames: usize = std::env::args().nth(2).map_or(60, |s| s.parse().unwrap());
    let mut snap = Snapshot::open(path.as_ref()).unwrap();
    let mut mem = GuestMemory::new();
    snap.load_into(&mut mem).unwrap();
    let base = mem.base;
    let start = snap.ctx;
    let posture = unsafe { start.r3.u32 };
    let context = rd(base, posture + 8);
    println!("posture {posture:#010X} context {context:#010X} f1 {}", unsafe { start.f1.f64 });
    let (proxy, path_s) = find_proxy(base, context).expect("no character proxy");
    println!("proxy {proxy:#010X} at {path_s}");
    set_pc_fixes(snap.pc_fixes);
    set_delta_time(snap.delta_time);
    let mut host = H(kernel::Kernel::from_memory(base).unwrap(), Default::default());
    for f in 0..frames {
        let mut ctx = start;
        with_host(&mut host, || unsafe {
            #[allow(deprecated)]
            let csr = x86_64::_mm_getcsr();
            #[allow(deprecated)]
            x86_64::_mm_setcsr(snap.csr);
            unleashed_guest::call_indirect(&mut ctx, base, snap.function);
            #[allow(deprecated)]
            x86_64::_mm_setcsr(csr);
        });
        let p = [rf(base, proxy + 0x120), rf(base, proxy + 0x124), rf(base, proxy + 0x128)];
        let v = [rf(base, proxy + 0x130), rf(base, proxy + 0x134), rf(base, proxy + 0x138)];
        if !host.1.is_empty() {
            println!("MISSING {}", host.1.iter().map(|a| format!("{a:08X}")).collect::<Vec<_>>().join(" "));
            return;
        }
        println!("frame {f:3}: pos {:9.3} {:9.3} {:9.3}  vel {:8.3} {:8.3} {:8.3}", p[0], p[1], p[2], v[0], v[1], v[2]);
    }
}
