//! After N frames running forward: Sonic's body position and every vec3 in the player
//! camera's first 0x200 bytes that lies near him (candidate eye / target fields).
use unleashed_guest_host::*;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    let mut s = g.sonic();
    for _ in 0..90 {
        s = g.step(&Pad { left: [0.0, 1.0], ..Default::default() });
    }
    let b = g.base();
    let cam = g.camera.unwrap();
    let f = |a: u32| f32::from_bits(unsafe { xenon_rt::PPC_LOAD_U32(b, a) });
    println!("body {:7.2?}  proxy {:7.2?}", s.body_position, s.position);
    for o in (0..0x200).step_by(4) {
        let v = [f(cam + o), f(cam + o + 4), f(cam + o + 8)];
        let d = ((v[0] - s.body_position[0]).powi(2) + (v[1] - s.body_position[1]).powi(2) + (v[2] - s.body_position[2]).powi(2)).sqrt();
        if d < 15.0 {
            println!("  +{o:#05X} {v:8.2?}  dist {d:5.2}");
        }
    }
}
