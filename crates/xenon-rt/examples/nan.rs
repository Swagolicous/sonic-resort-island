fn main() {
    let x = std::hint::black_box(-1.0f64);
    println!("c_sqrt {:016X} sqrt {:016X}", xenon_rt::c_sqrt(x).to_bits(), x.sqrt().to_bits());
}
