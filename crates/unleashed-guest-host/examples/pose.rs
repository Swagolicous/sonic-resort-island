//! Is the game posing Sonic? Prints a few bones of the hkQsTransform array at <addr> while
//! running (stick forward), plus how many of the 112 changed since the previous print.
use unleashed_guest_host::*;
fn main() {
    let mut a = std::env::args().skip(1);
    let path = a.next().unwrap();
    let addr = u32::from_str_radix(&a.next().unwrap_or("04B3CC00".into()), 16).unwrap();
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    let b = g.base();
    let f = |x: u32| f32::from_bits(unsafe { xenon_rt::PPC_LOAD_U32(b, x) });
    let read = || -> Vec<[f32; 12]> { (0..112).map(|i| std::array::from_fn(|k| f(addr + 48 * i + 4 * k as u32))).collect() };
    let mut prev = read();
    for fr in 0..60 {
        g.step(&Pad { left: [0.0, 1.0], ..Default::default() });
        if fr % 10 == 9 {
            let now = read();
            let changed = now.iter().zip(&prev).filter(|(x, y)| x != y).count();
            println!("f{fr}: {changed} bones changed; bone 3 q {:6.3?} bone 20 q {:6.3?}", &now[3][4..8], &now[20][4..8]);
            prev = now;
        }
    }
}
