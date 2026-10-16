//! Replays a capture-build snapshot through the lifted Rust and compares the result with
//! what the game wrote: loads pre.bin, runs the snapshotted function, diffs guest memory
//! against post.bin.
//!
//! usage: cargo run --release -p unleashed-guest --example replay -- <snapshot dir>

use std::collections::BTreeMap;
use std::path::PathBuf;
use xenon_rt::*;

struct ReplayHost {
    kernel: kernel::Kernel,
    imports: BTreeMap<&'static str, u32>,
    /// Unlifted functions reached (each skipped: the run is then only good for discovery).
    missing: BTreeMap<u32, u32>,
}

impl Host for ReplayHost {
    fn import(&mut self, name: &'static str, ctx: &mut PPCContext, base: *mut u8) {
        *self.imports.entry(name).or_default() += 1;
        if !self.kernel.import(name, ctx, base) {
            ctx.r3.u64 = 0;
        }
    }
    fn hook(&mut self, address: u32, ctx: &mut PPCContext, base: *mut u8) {
        assert!(self.kernel.hook(address, ctx, base), "host function {address:08X} not implemented");
    }
    fn missing(&mut self, address: u32, ctx: &mut PPCContext, _base: *mut u8) {
        *self.missing.entry(address).or_default() += 1;
        ctx.r3.u64 = 0;
    }
}

fn main() {
    let dir = PathBuf::from(std::env::args().nth(1).expect("usage: replay <snapshot dir>"));
    let mut pres: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("snapshot dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.to_string_lossy().ends_with("_pre.bin"))
        .collect();
    pres.sort();
    assert!(!pres.is_empty(), "no <function>_pre.bin in {}", dir.display());
    for pre in pres {
        let post = PathBuf::from(pre.to_string_lossy().replace("_pre.bin", "_post.bin"));
        replay(&pre, &post);
    }
}

fn replay(pre_path: &std::path::Path, post_path: &std::path::Path) {
    let mut pre = Snapshot::open(pre_path).expect("pre snapshot");
    println!(
        "sub_{:08X}: {} pages, dt {:.6}, pc fixes {}",
        pre.function, pre.page_count, pre.delta_time, pre.pc_fixes
    );

    let mut mem = GuestMemory::new();
    pre.load_into(&mut mem).expect("read pre snapshot");
    let mut ctx = pre.ctx;
    set_pc_fixes(pre.pc_fixes);
    set_delta_time(pre.delta_time);
    let mut host = ReplayHost {
        kernel: kernel::Kernel::from_memory(mem.base).expect("no o1heap user heap in the snapshot"),
        imports: BTreeMap::new(),
        missing: BTreeMap::new(),
    };
    let t = std::time::Instant::now();
    with_host(&mut host, || unsafe {
        #[allow(deprecated)]
        let csr = x86_64::_mm_getcsr();
        #[allow(deprecated)]
        x86_64::_mm_setcsr(pre.csr);
        unleashed_guest::call_indirect(&mut ctx, mem.base, pre.function);
        #[allow(deprecated)]
        x86_64::_mm_setcsr(csr);
    });
    println!("ran in {:?}; imports {:?}", t.elapsed(), host.imports);
    if !host.missing.is_empty() {
        // Not lifted yet: add them to sheets/lift_roots.csv (sheetc.py lift) and rerun.
        println!("MISSING {}", host.missing.keys().map(|a| format!("{a:08X}")).collect::<Vec<_>>().join(" "));
        return;
    }

    // Compare, streaming both files (pages ascending; a page left out is all zero).
    // Other game threads keep running while the capture build writes the dumps, so the
    // game's changes include theirs: what this call wrote is judged where Rust wrote.
    // Bytes Rust changed must equal the game's result; bytes the game changed in the
    // same 4 KiB blocks are suspects for writes Rust missed. Below the caller's stack
    // pointer is the call's own dead stack (temporaries), reported apart.
    let mut before_f = Snapshot::open(pre_path).expect("pre snapshot");
    let mut after_f = Snapshot::open(post_path).expect("post snapshot");
    let (mut bbuf, mut abuf) = (vec![0u8; SNAPSHOT_PAGE], vec![0u8; SNAPSHOT_PAGE]);
    let mut bnext = before_f.next_page(&mut bbuf).unwrap();
    let mut anext = after_f.next_page(&mut abuf).unwrap();
    let zero = vec![0u8; SNAPSHOT_PAGE];
    let r1 = unsafe { pre.ctx.r1.u32 };
    let dead_stack = |a: u32| a < r1 && a >= r1.saturating_sub(0x10_0000);
    let (mut stack_wrote, mut stack_differs) = (0usize, 0usize);
    let (mut rust_wrote, mut rust_right, mut missed, mut game_other) = (0usize, 0usize, 0usize, 0usize);
    let (mut wrong, mut missed_list) = (Vec::new(), Vec::new());
    // Coalesced ranges Rust wrote outside the dead stack (what the call outputs).
    let mut ranges: Vec<(u32, u32)> = Vec::new();
    for p in 1..0x10000u32 {
        let before: &[u8] = if bnext == Some(p) { &bbuf } else { &zero };
        let after: &[u8] = if anext == Some(p) { &abuf } else { &zero };
        let ours = mem.bytes(p * SNAPSHOT_PAGE as u32, SNAPSHOT_PAGE);
        if before != after || ours != before {
            for block in 0..SNAPSHOT_PAGE / 0x1000 {
                let r = block * 0x1000..(block + 1) * 0x1000;
                let touched = ours[r.clone()] != before[r.clone()];
                if !touched && after[r.clone()] == before[r.clone()] {
                    continue;
                }
                for i in r {
                    let a = p * SNAPSHOT_PAGE as u32 + i as u32;
                    let (b, g, o) = (before[i], after[i], ours[i]);
                    if dead_stack(a) {
                        if o != b {
                            stack_wrote += 1;
                            stack_differs += (o != g) as usize;
                        }
                    } else if o != b {
                        rust_wrote += 1;
                        match ranges.last_mut() {
                            Some(r) if a <= r.1 + 16 => r.1 = a,
                            _ => ranges.push((a, a)),
                        }
                        if o == g {
                            rust_right += 1;
                        } else if wrong.len() < 30 {
                            wrong.push(format!("  {a:08X}: was {b:02X} game {g:02X} rust {o:02X}"));
                        }
                    } else if g != b {
                        if touched {
                            missed += 1;
                            if missed_list.len() < 30 {
                                missed_list.push(format!("  {a:08X}: was {b:02X} game {g:02X}"));
                            }
                        } else {
                            game_other += 1;
                        }
                    }
                }
            }
        }
        if bnext == Some(p) {
            bnext = before_f.next_page(&mut bbuf).unwrap();
        }
        if anext == Some(p) {
            anext = after_f.next_page(&mut abuf).unwrap();
        }
    }
    println!("dead stack below r1 {r1:#010X}: rust wrote {stack_wrote} bytes, {stack_differs} differ from the game");
    println!("rust wrote {rust_wrote} bytes, {rust_right} as the game did; game also changed {missed} bytes next to them (suspects) and {game_other} elsewhere (other threads)");
    println!(" written: {}", ranges.iter().map(|(a, b)| format!("{a:08X}-{b:08X}")).collect::<Vec<_>>().join(" "));
    if std::env::var("REPLAY_ARGS").is_ok() {
        let r4 = unsafe { pre.ctx.r4.u32 };
        let words: Vec<String> = (0..16).map(|i| format!("{:08X}", unsafe { PPC_LOAD_U32(mem.base, r4 + 4 * i) })).collect();
        println!(" r4 {r4:08X}: {}", words.join(" "));
    }
    if !wrong.is_empty() {
        println!(" rust differs:");
        wrong.iter().for_each(|l| println!("{l}"));
    }
    if !missed_list.is_empty() {
        println!(" suspects:");
        missed_list.iter().for_each(|l| println!("{l}"));
    }
    let regs = unsafe { (ctx.r3.u64, ctx.f1.f64) };
    println!("r3 {:#X} f1 {}", regs.0, regs.1);
}
