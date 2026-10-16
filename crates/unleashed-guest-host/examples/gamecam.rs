use unleashed_guest_host::*;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    println!("game camera {:08X?}", g.game_camera);
    for f in 0..90 {
        let s = g.step(&Pad { left: [0.0, 1.0], ..Default::default() });
        if f % 15 == 14 {
            println!("f{f} sonic {:6.1?} player cam {:6.1?} game view {:6.1?}", s.body_position, g.camera().map(|c| c.eye), g.game_view().map(|v| (v.eye, v.target, v.fov_y)));
        }
    }
    println!("missing {:08X?}", g.missing());
}
