//! Does Sonic slide when B is held at speed? On a flat slab: boost forward for 4 s, then
//! hold B (stick still forward, boost released or held per the argument) for 2 s, then
//! release. Prints his speed every 10 frames (a slide bleeds speed; running holds it) and
//! the functions not lifted yet.
//!
//! usage: slide <82346F08_pre.bin> [boost during slide: 0|1]
use unleashed_guest_host::*;

fn main() {
    let path = std::env::args().nth(1).expect("usage: slide <82346F08_pre.bin> [boost during slide: 0|1]");
    let boost_in_slide = std::env::args().nth(2).is_some_and(|s| s == "1");
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    let world = g.havok_world().expect("no hkpWorld");
    for (body, motion, _, shape) in g.rigid_bodies(world) {
        if motion == 7 && shape != 0 {
            g.remove_body(world, body);
        }
    }
    let feet = g.sonic().body_position;
    g.add_fixed_box(world, [feet[0], feet[1] - 0.5, feet[2]], [8000.0, 0.5, 8000.0], [0.0, 0.0, 0.0, 1.0], 0.0, 0x7);
    let mut line = String::new();
    for f in 0..480u32 {
        let mut pad = Pad { left: [0.0, 1.0], ..Default::default() };
        let phase = match f {
            0..=239 => {
                pad.down |= button::X;
                "boost"
            }
            240..=359 => {
                pad.down |= button::B | if boost_in_slide { button::X } else { 0 };
                "B held"
            }
            _ => "released",
        };
        let s = g.step(&pad);
        if f % 10 == 9 {
            let speed = (s.velocity[0].powi(2) + s.velocity[2].powi(2)).sqrt();
            if f % 120 == 9 {
                line += &format!("\n   {phase:8}");
            }
            line += &format!(" {speed:.0}{}", if g.boosting() { "*" } else { "" });
        }
    }
    println!("speed m/s every 10 frames (* = boost flag set):{line}");
    println!("missing {:08X?}", g.missing());
}
