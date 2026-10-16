//! Lists guest addresses holding a given big-endian word in a snapshot (e.g. objects
//! with a vtable), plus the first words at a few addresses.
//!
//! usage: cargo run --release -p unleashed-guest --example findword -- <X_pre.bin> <word hex>... [@addr...]

use xenon_rt::*;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut snap = Snapshot::open(args[0].as_ref()).unwrap();
    let mut mem = GuestMemory::new();
    snap.load_into(&mut mem).unwrap();
    let base = mem.base;
    for a in &args[1..] {
        if let Some(addr) = a.strip_prefix('@') {
            let addr = u32::from_str_radix(addr.trim_start_matches("0x"), 16).unwrap();
            let words: Vec<String> = (0..16).map(|i| format!("{:08X}", unsafe { PPC_LOAD_U32(base, addr + 4 * i) })).collect();
            println!("@{addr:08X}: {}", words.join(" "));
            continue;
        }
        let w = u32::from_str_radix(a.trim_start_matches("0x"), 16).unwrap();
        let mut hits = Vec::new();
        let mut p = 0x1000u32;
        while p < 0xFFFF_FFF0 && hits.len() < 40 {
            if unsafe { PPC_LOAD_U32(base, p) } == w {
                hits.push(format!("{p:08X}"));
            }
            p += 4;
        }
        println!("{w:08X}: {}", hits.join(" "));
    }
}
