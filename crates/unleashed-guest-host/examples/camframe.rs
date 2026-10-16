//! Which call of a frame writes the frame Sonic steers against (speed context +0x290,
//! the rotation sub_8234EE88 turns the stick by)? Holds the stick forward on a flat slab
//! and prints, every second, the 16 floats there after each call of the frame, next to
//! the drawn view's yaw and Sonic's heading.
//!
//! usage: camframe <82346F08_pre.bin> [offset hex]
use unleashed_guest_host::*;
use xenon_rt::*;

fn main() {
    let path = std::env::args().nth(1).expect("usage: camframe <82346F08_pre.bin> [offset hex]");
    let offset = std::env::args().nth(2).map_or(0x290, |s| u32::from_str_radix(&s, 16).unwrap());
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    let world = g.havok_world().expect("no hkpWorld");
    for (body, motion, _, shape) in g.rigid_bodies(world) {
        if motion == 7 && shape != 0 {
            g.remove_body(world, body);
        }
    }
    let feet = g.sonic().body_position;
    g.add_fixed_box(world, [feet[0], feet[1] - 0.5, feet[2]], [4000.0, 0.5, 4000.0], [0.0, 0.0, 0.0, 1.0], 0.0, 0x7);
    g.watch = Some(g.context + offset);
    let pad = Pad { left: [0.0, 1.0], ..Default::default() };
    for f in 0..300 {
        let s = g.step(&pad);
        if f % 60 == 59 {
            let c = g.game_view().unwrap();
            let view = (c.target[0] - c.eye[0]).atan2(c.target[2] - c.eye[2]).to_degrees();
            let heading = s.velocity[0].atan2(s.velocity[2]).to_degrees();
            println!("frame {f}: view yaw {view:+.1}, heading {heading:+.1}");
            for (name, w) in ["parallel", "serial  ", "plr cam ", "game cam"].iter().zip(&g.watch_log) {
                let q = w.map(f32::from_bits);
                println!("   after {name} [{:+.3} {:+.3} {:+.3} {:+.3}]", q[0], q[1], q[2], q[3]);
            }
            let b = g.base();
            let m: Vec<String> = (0..16).map(|i| format!("{:+.3}", f32::from_bits(unsafe { PPC_LOAD_U32(b, g.context + offset + 4 * i) }))).collect();
            println!("   16 floats: {}", m.join(" "));
        }
    }
}
