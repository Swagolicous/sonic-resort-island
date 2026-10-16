//! Strings in a guest address range (default the executable's read-only data) that contain
//! any of the given substrings, case-insensitively: `address string`.
//!
//! usage: rostrings <X_pre.bin> [--range lo hi] <substring>...
use xenon_rt::*;

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let path = args.remove(0);
    let (mut lo, mut hi) = (0x8200_0000u32, 0x8220_0000u32);
    if args.first().is_some_and(|a| a == "--range") {
        lo = u32::from_str_radix(&args[1], 16).unwrap();
        hi = u32::from_str_radix(&args[2], 16).unwrap();
        args.drain(..3);
    }
    let needles: Vec<String> = args.iter().map(|s| s.to_lowercase()).collect();
    let mut snap = Snapshot::open(path.as_ref()).unwrap();
    let mut mem = GuestMemory::new();
    snap.load_into(&mut mem).unwrap();
    let bytes = unsafe { core::slice::from_raw_parts(mem.base.add(lo as usize), (hi - lo) as usize) };
    let mut start = 0;
    for i in 0..bytes.len() {
        let c = bytes[i];
        if c.is_ascii_graphic() || c == b' ' {
            continue;
        }
        if c == 0 && i - start >= 4 {
            let s = String::from_utf8_lossy(&bytes[start..i]);
            let l = s.to_lowercase();
            if needles.iter().any(|n| l.contains(n.as_str())) {
                println!("{:08X} {s}", lo + start as u32);
            }
        }
        start = i + 1;
    }
}
