//! Same snapshot, three stick inputs: Sonic should end up in three different places.
use unleashed_guest_host::*;

fn main() {
    let path = std::env::args().nth(1).expect("usage: steer <82346F08_pre.bin>");
    for (name, stick) in [("neutral", [0.0, 0.0]), ("forward", [0.0, 1.0]), ("left", [-1.0, 0.5]), ("right", [1.0, 0.5])] {
        let mut g = GuestSonic::load(path.as_ref()).unwrap();
        if name == "neutral" {
            println!("posture {:#X} input {:#X} context {:#X} proxy {:#X} pad {:#X} dt {}", g.posture, g.posture_input, g.context, g.proxy, g.pad, g.snapshot_dt());
        }
        let pad = Pad { left: stick, ..Default::default() };
        let mut s = g.sonic();
        for _ in 0..90 {
            s = g.step(&pad);
        }
        println!("          camera {:?}", g.camera());
        println!("{name:8}: pos {:8.2} {:8.2} {:8.2}  vel {:7.2} {:7.2} {:7.2}  missing {:?}",
            s.position[0], s.position[1], s.position[2], s.velocity[0], s.velocity[1], s.velocity[2],
            g.missing().iter().map(|a| format!("{a:08X}")).collect::<Vec<_>>());
    }
}
