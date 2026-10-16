//! How many fixed boxes the game's Havok world takes, and what each costs: adds 1 m
//! blocks in a square under Sonic, timing every 256, then steps the game.
//!
//! usage: manyboxes <82346F08_pre.bin> [count]
use std::time::Instant;
use unleashed_guest_host::*;

fn main() {
    let path = std::env::args().nth(1).expect("usage: manyboxes <82346F08_pre.bin> [count]");
    let count: usize = std::env::args().nth(2).map_or(4096, |s| s.parse().unwrap());
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    let world = g.havok_world().expect("no hkpWorld");
    let stage: Vec<u32> = g.rigid_bodies(world).into_iter().filter(|&(_, motion, _, shape)| motion == 7 && shape != 0).map(|r| r.0).collect();
    for &body in &stage {
        g.remove_body(world, body);
    }
    let feet = g.sonic().body_position;
    let side = (count as f32).sqrt().ceil() as usize;
    let mut started = Instant::now();
    for i in 0..count {
        let (x, z) = ((i % side) as f32 - side as f32 / 2.0, (i / side) as f32 - side as f32 / 2.0);
        g.add_fixed_box(world, [feet[0] + x + 0.5, feet[1] - 0.5, feet[2] + z + 0.5], [0.5, 0.5, 0.5], [0.0, 0.0, 0.0, 1.0], 0.0, 0x7);
        if i % 256 == 255 {
            println!("{:5} boxes: last 256 in {:?}; missing {:08X?}", i + 1, started.elapsed(), g.missing());
            started = Instant::now();
        }
    }
    let started = Instant::now();
    let mut s = g.sonic();
    for _ in 0..120 {
        s = g.step(&Pad::default());
    }
    println!("120 frames in {:?}; feet {:.2?}; missing {:08X?}", started.elapsed(), s.body_position, g.missing());
}
