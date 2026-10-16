//! Prints big-endian f64 words: peek64 <hex address> <offset>... (offsets in bytes, decimal).
fn main() {
    let o = xenon_oracle::Oracle::get().expect("oracle not built");
    let mut args = std::env::args().skip(1);
    let base = u32::from_str_radix(args.next().unwrap().trim_start_matches("0x"), 16).unwrap();
    for off in args {
        let off: u32 = off.parse().unwrap();
        let bits = u64::from_be_bytes(o.read(base + off, 8).try_into().unwrap());
        println!("{off:4}  {bits:#018X}  {:e}", f64::from_bits(bits));
    }
}
