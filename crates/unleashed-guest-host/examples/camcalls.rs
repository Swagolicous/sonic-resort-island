//! Which camera code does one game frame run through virtual calls, and how often?
//! Counts indirect-call targets in the camera code (0x82460000..0x82490000) over one
//! frame while running straight.
//!
//! usage: camcalls <82346F08_pre.bin>
use unleashed_guest_host::*;
use xenon_rt::*;

fn main() {
    let path = std::env::args().nth(1).expect("usage: camcalls <82346F08_pre.bin>");
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    let pad = Pad { left: [0.0, 1.0], ..Default::default() };
    for _ in 0..60 {
        g.step(&pad);
    }
    trace_indirect(true);
    g.step(&pad);
    let calls = trace_indirect(false).unwrap();
    println!("player camera {:08X?}, game camera {:08X?}", g.camera, g.game_camera);
    for (address, count) in calls.iter().filter(|(a, _)| (0x8246_0000..0x8249_0000).contains(*a)) {
        println!("   {address:08X} x{count}");
    }
    println!("{} indirect targets in the frame", calls.len());
}
