//! The game's boost effect (SWA::CFxBoostEffect3D, a job of the render director's Fx
//! pipeline) driven by this host's frame: 1 s running, 2 s boosting, 2 s running on a
//! flat slab, with GuestSonic::step delivering Sonic's messages and running the render
//! director. Prints every word of the effect object that changes, at sample frames.
//!
//! usage: boostfx <82346F08_pre.bin>
use unleashed_guest_host::*;
use xenon_rt::*;

const VT_BOOST_EFFECT: u32 = 0x820C_A6C0;
const WORDS: u32 = 0x140 / 4;

fn main() {
    let path = std::env::args().nth(1).expect("usage: boostfx <82346F08_pre.bin>");
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    let world = g.havok_world().expect("no hkpWorld");
    for (body, motion, _, shape) in g.rigid_bodies(world) {
        if motion == 7 && shape != 0 {
            g.remove_body(world, body);
        }
    }
    let feet = g.sonic().body_position;
    g.add_fixed_box(world, [feet[0], feet[1] - 0.5, feet[2]], [8000.0, 0.5, 8000.0], [0.0, 0.0, 0.0, 1.0], 0.0, 0x7);
    let fx = g.find_object(VT_BOOST_EFFECT).expect("no CFxBoostEffect3D");
    let b = g.base();
    let samples = [59u32, 60, 62, 70, 90, 120, 179, 180, 182, 190, 210, 299];
    let mut shots: Vec<Vec<u32>> = Vec::new();
    for f in 0..300u32 {
        let down = if (60..180).contains(&f) { button::X } else { 0 };
        g.step(&Pad { down, left: [0.0, 1.0], ..Default::default() });
        if samples.contains(&f) {
            shots.push((0..WORDS).map(|i| unsafe { PPC_LOAD_U32(b, fx + 4 * i) }).collect());
        }
    }
    println!("effect {fx:08X}; boost held frames 60..180; samples at frames {samples:?}");
    for i in 0..WORDS as usize {
        let v: Vec<u32> = shots.iter().map(|s| s[i]).collect();
        if v.iter().any(|x| *x != v[0]) {
            let as_float = v.iter().all(|x| { let f = f32::from_bits(*x); *x == 0 || (f.is_finite() && f.abs() > 1e-4 && f.abs() < 1e5) });
            let text: Vec<String> = v.iter().map(|x| if as_float { format!("{:.3}", f32::from_bits(*x)) } else { format!("{x:08X}") }).collect();
            println!("   +{:#05X}: {}", 4 * i, text.join(" "));
        }
    }
    println!("missing {:08X?}", g.missing());
}
