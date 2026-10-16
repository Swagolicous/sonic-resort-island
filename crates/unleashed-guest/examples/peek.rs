//! Prints what is at guest addresses in a snapshot: as a C string, as words, as floats.
//!
//! usage: cargo run --release -p unleashed-guest --example peek -- <X_pre.bin> <address hex>...
use xenon_rt::*;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut snap = Snapshot::open(args[0].as_ref()).unwrap();
    let mut mem = GuestMemory::new();
    snap.load_into(&mut mem).unwrap();
    let base = mem.base;
    for a in &args[1..] {
        let a = u32::from_str_radix(a.trim_start_matches("0x"), 16).unwrap();
        let text: String = (0..80).map(|i| unsafe { PPC_LOAD_U8(base, a + i) }).take_while(|&c| c != 0).map(|c| if c.is_ascii_graphic() || c == b' ' { c as char } else { '.' }).collect();
        let words: Vec<u32> = (0..8).map(|i| unsafe { PPC_LOAD_U32(base, a + 4 * i) }).collect();
        println!("{a:08X}: \"{text}\"\n   {:08X?}\n   {:?}", words, words.iter().map(|w| f32::from_bits(*w)).collect::<Vec<_>>());
    }
}
