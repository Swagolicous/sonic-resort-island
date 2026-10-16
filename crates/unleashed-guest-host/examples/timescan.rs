//! Finds the game's clocks by comparing two snapshots of one session taken at different
//! moments: every copy of a running clock differs by the same number of seconds, so the
//! commonest float difference between the two memories is the time between them, and
//! the addresses that differ by it are clocks.
//!
//! usage: timescan <a_pre.bin> <b_pre.bin>
use std::collections::HashMap;
use xenon_rt::*;

fn load(path: &str) -> GuestMemory {
    let mut snap = Snapshot::open(path.as_ref()).unwrap();
    let mut mem = GuestMemory::new();
    snap.load_into(&mut mem).unwrap();
    mem
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (a, b) = (load(&args[0]), load(&args[1]));
    let rf = |m: &GuestMemory, p: u32| f32::from_bits(unsafe { PPC_LOAD_U32(m.base, p) });
    let mut bins: HashMap<i64, Vec<u32>> = HashMap::new();
    let mut p = 0x0002_0000u32;
    while p < 0x7FEA_0000 {
        let (x, y) = (rf(&a, p), rf(&b, p));
        if x != y && (1.0..36000.0).contains(&x) && (1.0..36000.0).contains(&y) {
            let d = (y - x) as f64;
            if d.abs() > 0.004 && d.abs() < 0.2 {
                bins.entry((d * 2000.0).round() as i64).or_default().push(p);
            }
        }
        p += 4;
    }
    let mut top: Vec<_> = bins.into_iter().collect();
    top.sort_by_key(|(_, v)| std::cmp::Reverse(v.len()));
    for (d, addresses) in top.iter().take(6) {
        println!("difference {:+.4} s: {} floats", *d as f64 / 2000.0, addresses.len());
        for &p in addresses.iter().take(14) {
            println!("   {p:08X}: {:.3} -> {:.3}", rf(&a, p), rf(&b, p));
        }
    }
    // The game document's score info, for reference (CGameDocument at *0x83367900).
    for m in [&a, &b] {
        let rd = |p: u32| unsafe { PPC_LOAD_U32(m.base, p) };
        let member = rd(rd(0x8336_7900) + 8);
        let s = member + 0x1D8;
        println!("game document member {member:08X}: enemy score {}, trick score {}, point markers {}", rd(s + 0x34), rd(s + 0x38), rd(s + 0x30));
    }
}
