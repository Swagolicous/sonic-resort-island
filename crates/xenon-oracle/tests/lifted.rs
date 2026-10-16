//! The lifted Rust (crates/unleashed-guest, re/xenon/ppc2rust.py) against the recompiled
//! C++ (the oracle), on the same guest memory: each case fills its argument buffers,
//! runs the C++, restores the buffers, runs the Rust, and compares every buffer byte
//! and the result registers. Memory must match byte for byte; a NaN in f1 only has to
//! be a NaN (see below).

use xenon_oracle::Oracle;
use xenon_rt::PPCContext;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn f32(&mut self) -> f32 {
        let sign = if self.next() & 1 == 0 { 1.0 } else { -1.0 };
        match self.next() % 10 {
            0 => 0.0,
            1 => f32::from_bits((self.next() as u32 & 0x007F_FFFF) | 0x3400_0000) * sign,
            2 => (self.next() % 2_000_000) as f32 * 1e3 - 1e9,
            3 => f32::from_bits(self.next() as u32 & 0x007F_FFFF) * sign, // denormal
            4..=6 => (self.next() % 2_000_001) as f32 / 2_000_000.0 * sign,
            _ => (self.next() % 2_000_001) as f32 / 1000.0 - 1000.0,
        }
    }
    fn f64(&mut self) -> f64 {
        match self.next() % 6 {
            0 => 0.0,
            1 => (self.next() % 2_000_001) as f64 / 1_000_000.0 - 1.0,
            2 => (self.next() as i64 as f64) * 1e-12,
            _ => (self.next() % 2_000_001) as f64 / 100.0 - 10_000.0,
        }
    }
}

/// A guest function and its arguments: pointer registers with buffer sizes (filled with
/// floats, `zero` byte offsets cleared), and float registers.
struct Case {
    address: u32,
    ptrs: &'static [(u32, u32)],
    floats: &'static [u32],
    zero: &'static [u32],
}

const CASES: &[Case] = &[
    Case { address: 0x82E86198, ptrs: &[(3, 16), (4, 16)], floats: &[], zero: &[] }, // normalize
    Case { address: 0x82E84A90, ptrs: &[(3, 16), (4, 64), (5, 16)], floats: &[], zero: &[] }, // transform
    Case { address: 0x82E84BE0, ptrs: &[(3, 64), (4, 16)], floats: &[], zero: &[] }, // quat -> matrix
    Case { address: 0x82E85BA8, ptrs: &[(3, 16), (4, 16), (5, 16)], floats: &[], zero: &[] },
    Case { address: 0x82E85988, ptrs: &[(3, 16), (4, 16)], floats: &[1], zero: &[] }, // axis-angle
    Case { address: 0x82E860B8, ptrs: &[(3, 16)], floats: &[], zero: &[] },
    Case { address: 0x831B3A68, ptrs: &[], floats: &[1, 2], zero: &[] }, // atan2
    Case { address: 0x831B1788, ptrs: &[], floats: &[1], zero: &[] }, // sin
    Case { address: 0x831B1868, ptrs: &[], floats: &[1], zero: &[] }, // cos
    Case { address: 0x831B3D58, ptrs: &[], floats: &[1], zero: &[] }, // acos
    // sub_8231A468: length of ctx+0x220, flag ctx+0x565 cleared.
    Case { address: 0x8231A468, ptrs: &[(3, 0x800)], floats: &[], zero: &[0x564] },
];

#[test]
fn lifted_functions_match_the_recompiled_cpp() {
    let Some(mut o) = Oracle::get() else {
        eprintln!("xenon oracle not built; skipping");
        return;
    };
    let base = o.base();
    let (r1, r13) = (o.r(1), o.r(13));
    for case in CASES {
        assert!(unleashed_guest::LIFTED.contains(&case.address), "{:08X} not lifted", case.address);
        eprintln!("case sub_{:08X}", case.address);
        let bufs: Vec<(u32, u32, u32)> = case.ptrs.iter().map(|&(r, n)| (r, o.alloc(n), n)).collect();
        let mut rng = Rng(0xC0FF_EE00_0000_0001 ^ case.address as u64);
        for sample in 0..5_000 {
            let mut init = Vec::new();
            for &(_, addr, n) in &bufs {
                let mut bytes: Vec<u8> = (0..n / 4).flat_map(|_| rng.f32().to_be_bytes()).collect();
                for &z in case.zero {
                    if (z as usize) < bytes.len() {
                        let end = (z as usize + 4).min(bytes.len());
                        bytes[z as usize..end].fill(0);
                    }
                }
                o.write(addr, &bytes);
                init.push(bytes);
            }
            let fargs: Vec<f64> = case.floats.iter().map(|_| rng.f64()).collect();

            for &(r, addr, _) in &bufs {
                o.set_r(r as i32, addr as u64);
            }
            for (&f, &v) in case.floats.iter().zip(&fargs) {
                o.set_f(f as i32, v);
            }
            o.set_r(1, r1);
            if case.floats.is_empty() {
                o.set_f(1, 0.0);
            }
            if bufs.is_empty() {
                o.set_r(3, 0);
            }
            o.call(case.address);
            let want: Vec<Vec<u8>> = bufs.iter().map(|&(_, a, n)| o.read(a, n as usize)).collect();
            let (want_r3, want_f1) = (o.r(3), o.f(1));

            for (&(_, addr, _), bytes) in bufs.iter().zip(&init) {
                o.write(addr, bytes);
            }
            let mut ctx = PPCContext::default();
            unsafe {
                ctx.r1.u64 = r1;
                ctx.r13.u64 = r13;
                for &(r, addr, _) in &bufs {
                    ctx.gpr(r).unwrap().u64 = addr as u64;
                }
                for (&f, &v) in case.floats.iter().zip(&fargs) {
                    ctx.fpr(f).unwrap().f64 = v;
                }
                ctx.fpscr.loadFromHost();
                #[allow(deprecated)]
                let csr = std::arch::x86_64::_mm_getcsr();
                unleashed_guest::call_indirect(&mut ctx, base, case.address);
                #[allow(deprecated)]
                std::arch::x86_64::_mm_setcsr(csr);
            }
            let got: Vec<Vec<u8>> = bufs.iter().map(|&(_, a, n)| o.read(a, n as usize)).collect();
            assert_eq!(got, want, "sub_{:08X} sample {sample}: memory differs", case.address);
            let (got_r3, got_f1) = unsafe { (ctx.r3.u64, ctx.f1.f64) };
            assert_eq!(got_r3, want_r3, "sub_{:08X} sample {sample}: r3", case.address);
            // LLVM leaves a NaN's sign and payload unspecified (clang's /fp:strict does
            // not), so a NaN result only has to be a NaN; every other result is bit-exact.
            let same = got_f1.to_bits() == want_f1.to_bits() || (got_f1.is_nan() && want_f1.is_nan());
            assert!(same, "sub_{:08X} sample {sample}: f1 {got_f1} vs {want_f1} args {fargs:?}", case.address);
        }
    }
    assert_eq!(o.import_hits(), 0);
}
