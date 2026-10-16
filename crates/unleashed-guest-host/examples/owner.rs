//! Which object holds a guest address? Prints the vtable-like words (0x82......) in the
//! 0x400 bytes before each address, nearest first, with the offset of the address from
//! them; look the vtables up in Decomp/rtti_classes.jsonl.
//!
//! usage: owner <X_pre.bin> <address hex>...
use xenon_rt::*;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut snap = Snapshot::open(args[0].as_ref()).unwrap();
    let mut mem = GuestMemory::new();
    snap.load_into(&mut mem).unwrap();
    let rd = |p: u32| unsafe { PPC_LOAD_U32(mem.base, p) };
    for a in &args[1..] {
        let a = u32::from_str_radix(a, 16).unwrap();
        let mut found = Vec::new();
        let mut p = a & !3;
        while p > a.saturating_sub(0x400) && found.len() < 4 {
            let w = rd(p);
            if (0x8200_0000..0x8220_0000).contains(&w) {
                found.push(format!("{w:08X}@-{:#X}", a - p));
            }
            p -= 4;
        }
        println!("{a:08X} = {} ({}): {}", f32::from_bits(rd(a)), rd(a), found.join(" "));
    }
}
