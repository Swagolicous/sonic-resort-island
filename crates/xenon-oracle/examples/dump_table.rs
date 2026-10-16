//! Prints math_tables.csv rows for a guest table: dump_table <name> <hex base> <off:kind>...
//! (kind f64 or f32; used by sheets/seed.py math).
fn main() {
    let o = xenon_oracle::Oracle::get().expect("oracle not built");
    let mut args = std::env::args().skip(1);
    let name = args.next().unwrap();
    let base = u32::from_str_radix(args.next().unwrap().trim_start_matches("0x"), 16).unwrap();
    for spec in args {
        let (off, kind) = spec.split_once(':').unwrap();
        let off: u32 = off.parse().unwrap();
        let bits = match kind {
            "f64" => format!("0x{:016X}", u64::from_be_bytes(o.read(base + off, 8).try_into().unwrap())),
            _ => format!("0x{:08X}", o.read_u32(base + off)),
        };
        println!("{name},0x{base:08X},{off},{kind},{bits}");
    }
}
