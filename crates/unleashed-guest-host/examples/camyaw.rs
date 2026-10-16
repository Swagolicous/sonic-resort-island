//! Does the game's camera hold straight behind Sonic when he runs straight? On a flat
//! slab: `rest` frames with the stick neutral, then the stick held (default exactly
//! forward). Prints every 20 frames the angle from Sonic's direction of travel to the
//! camera's view direction (degrees, about up), his heading and speed.
//!
//! usage: camyaw <82346F08_pre.bin> [rest frames] [stick x] [stick y] [frames]
use unleashed_guest_host::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: camyaw <82346F08_pre.bin> [rest frames] [stick x] [stick y] [frames]");
    let mut num = |d: f32| args.next().map_or(d, |s| s.parse().unwrap());
    let (rest, x, y, frames) = (num(0.0) as u32, num(0.0), num(1.0), num(600.0) as u32);
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    let world = g.havok_world().expect("no hkpWorld");
    for (body, motion, _, shape) in g.rigid_bodies(world) {
        if motion == 7 && shape != 0 {
            g.remove_body(world, body);
        }
    }
    let feet = g.sonic().body_position;
    g.add_fixed_box(world, [feet[0], feet[1] - 0.5, feet[2]], [4000.0, 0.5, 4000.0], [0.0, 0.0, 0.0, 1.0], 0.0, 0x7);
    for _ in 0..rest {
        g.step(&Pad::default());
    }
    let pad = Pad { left: [x, y], ..Default::default() };
    let (mut line, mut worst) = (String::new(), 0f32);
    for f in 0..frames {
        let s = g.step(&pad);
        let Some(c) = g.game_view().or_else(|| g.camera()) else { continue };
        let view = [c.target[0] - c.eye[0], c.target[2] - c.eye[2]];
        let v = [s.velocity[0], s.velocity[2]];
        let speed = (v[0] * v[0] + v[1] * v[1]).sqrt();
        let angle = (v[0] * view[1] - v[1] * view[0]).atan2(v[0] * view[0] + v[1] * view[1]).to_degrees();
        if f >= 240 {
            worst = worst.max(angle.abs());
        }
        if f % 20 == 19 {
            line += &format!(" {angle:+.1}(h{:+.0} {speed:.0})", v[0].atan2(v[1]).to_degrees());
        }
    }
    println!("rest {rest}, stick ({x}, {y}): view angle off travel (heading, speed m/s), every 20 frames:\n{line}");
    println!("largest after 4 s: {worst:.2} degrees; missing {:08X?}", g.missing());
}
