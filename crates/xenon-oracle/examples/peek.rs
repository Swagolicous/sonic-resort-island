//! Prints guest image constants: peek <hex address> [count] (f32, big-endian).
fn main() {
    let mut o = xenon_oracle::Oracle::get().expect("oracle not built");
    let mut args = std::env::args().skip(1);
    let addr = u32::from_str_radix(args.next().expect("address").trim_start_matches("0x"), 16).unwrap();
    let n: u32 = args.next().map(|s| s.parse().unwrap()).unwrap_or(1);
    for i in 0..n {
        let a = addr + 4 * i;
        println!("{a:#010X}  {:#010X}  {}", o.read_u32(a), o.read_f32(a));
    }
    let _ = &mut o;
}
