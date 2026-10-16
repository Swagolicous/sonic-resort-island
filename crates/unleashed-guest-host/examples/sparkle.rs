//! Runs the game's particle system (SWA::CSparkleManager) alongside Sonic: 1 s running, a
//! jump, 1.5 s running, 2 s boosting, 1 s running, on a flat slab. GuestSonic::step
//! services the manager when UNLEASHED_SPARKLE is set (message pump and update unit);
//! this prints what it reaches that is not lifted, and how many heap words change per
//! phase inside the particle manager's object.
//!
//! usage: sparkle <82346F08_pre.bin> [frames]
use unleashed_guest_host::*;

fn main() {
    let path = std::env::args().nth(1).expect("usage: sparkle <82346F08_pre.bin> [frames]");
    let frames: u32 = std::env::args().nth(2).map_or(400, |s| s.parse().unwrap());
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    let world = g.havok_world().expect("no hkpWorld");
    for (body, motion, _, shape) in g.rigid_bodies(world) {
        if motion == 7 && shape != 0 {
            g.remove_body(world, body);
        }
    }
    let feet = g.sonic().body_position;
    g.add_fixed_box(world, [feet[0], feet[1] - 0.5, feet[2]], [8000.0, 0.5, 8000.0], [0.0, 0.0, 0.0, 1.0], 0.0, 0x7);
    println!("sparkle manager {:08X?}", g.sparkle_manager);
    let mut reported = 0;
    let mut last: Vec<u32> = Vec::new();
    for f in 0..frames {
        let mut down = 0;
        if (60..75).contains(&f) {
            down |= button::A;
        }
        if (150..270).contains(&f) {
            down |= button::X;
        }
        let s = g.step(&Pad { down, left: [0.0, 1.0], ..Default::default() });
        if g.missing().len() > reported {
            reported = g.missing().len();
            println!("frame {f}: missing {:08X?}", g.missing());
        }
        let effects = g.sparkle_effects();
        if effects != last {
            let shape: Vec<String> = effects
                .iter()
                .map(|&e| {
                    let em = g.sparkle_emitters(e);
                    format!("{e:08X}[{}]", em.iter().map(|(_, p)| p.len().to_string()).collect::<Vec<_>>().join(","))
                })
                .collect();
            println!("frame {f} (y {:.2}{}): {} effects: {}", s.body_position[1], if down != 0 { format!(", buttons {down:#X}") } else { String::new() }, effects.len(), shape.join(" "));
            last = effects;
        }
    }
    println!("missing {:08X?}", g.missing());
}
