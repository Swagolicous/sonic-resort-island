//! Runs frames holding forward and prints watched guest addresses (vec3) each 15 frames.
use unleashed_guest_host::*;
fn main() {
    let mut a = std::env::args().skip(1);
    let path = a.next().unwrap();
    let watch: Vec<u32> = a.map(|s| u32::from_str_radix(&s, 16).unwrap()).collect();
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    let pad = Pad { left: [0.0, 1.0], ..Default::default() };
    let b = g.base();
    let v = |x: u32| -> [f32; 3] { std::array::from_fn(|i| f32::from_bits(unsafe { xenon_rt::PPC_LOAD_U32(b, x + 4 * i as u32) })) };
    for f in 0..60 {
        if f == 59 { println!("missing {:08X?}", g.missing()); }
        let s = g.step(&pad);
        if f % 15 == 14 {
            print!("f{f:2} proxy {:7.1?}", s.position);
            for &w in &watch {
                print!("  {w:08X} {:7.1?}", v(w));
            }
            println!();
        }
    }
}
