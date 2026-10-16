//! Regression: the unchanged guest Sonic must stand and jump on imported triangles.
use unleashed_guest_host::{GuestSonic, Pad, button, terrain::weld_mesh};

fn main() {
    let path = std::env::args().nth(1).expect("course_triangle <snapshot>");
    for kind in ["box", "triangle", "welded"] { check(&path, kind); }
}

fn check(path: &str, kind: &str) {
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    let world = g.havok_world().unwrap();
    for (body, motion, _, shape) in g.rigid_bodies(world) {
        if motion == 7 && shape != 0 { g.remove_body(world, body); }
    }
    let [x,y,z] = g.sonic().body_position;
    let points = [[x-1000.0,y,z-1000.0], [x-1000.0,y,z+1000.0],
        [x+1000.0,y,z+1000.0], [x+1000.0,y,z-1000.0]];
    if kind == "box" {
        g.add_fixed_box(world, [x,y-0.5,z], [1000.0,0.5,1000.0], [0.0,0.0,0.0,1.0], 0.0, 7);
    } else {
        add_triangles(&mut g,world,kind,&[[points[0],points[1],points[2]],[points[0],points[2],points[3]]]);
    }
    for _ in 0..240 { g.step(&Pad::default()); }
    let stand = g.sonic().body_position;
    println!("{kind}: standing feet {stand:?}, velocity {:?}, expected ground y {y}; missing {:08X?}", g.sonic().velocity, g.missing());
    assert!((stand[1] - y).abs() < 0.25, "guest fell through course triangles");
    let mut highest = stand[1];
    for _ in 0..18 {
        highest = highest.max(g.step(&Pad { down: button::A, ..Default::default() }).body_position[1]);
    }
    for _ in 0..60 { highest = highest.max(g.step(&Pad::default()).body_position[1]); }
    println!("{kind}: held jump apex {highest:.3}; camera {:?}; missing {:08X?}", g.game_view(), g.missing());
    assert!(highest > stand[1] + 0.5, "guest jump did not leave imported ground");
    for _ in 0..120 { g.step(&Pad::default()); }
    assert!((g.sonic().body_position[1] - y).abs() < 0.25, "guest did not land on course triangles");
    assert!(g.missing().is_empty(), "collision ran an unlifted game call");
    println!("PASS {kind}: original guest stands, jumps and lands; apex {highest:.3}, missing {:08X?}", g.missing());
    // A floor ray alone does not prove that vertical course faces stop Sonic.
    let stand = g.sonic().body_position;
    let camera = g.game_view().unwrap();
    let (fx, fz) = (camera.target[0] - camera.eye[0], camera.target[2] - camera.eye[2]);
    let length = (fx * fx + fz * fz).sqrt();
    let (fx, fz) = (fx / length, fz / length);
    let (rx, rz) = (-fz, fx);
    let c = [stand[0] + fx * 15.0, stand[1] + 4.5, stand[2] + fz * 15.0];
    if kind == "box" {
        let half_yaw = fx.atan2(fz) * 0.5;
        g.add_fixed_box(world, c, [20.0,5.5,0.1], [0.0,half_yaw.sin(),0.0,half_yaw.cos()], 0.0, 7);
    } else {
        let corners = [[c[0]-rx*20.0,stand[1]-1.0,c[2]-rz*20.0],
            [c[0]-rx*20.0,stand[1]+10.0,c[2]-rz*20.0],
            [c[0]+rx*20.0,stand[1]+10.0,c[2]+rz*20.0],
            [c[0]+rx*20.0,stand[1]-1.0,c[2]+rz*20.0]];
        add_triangles(&mut g,world,kind,&[[corners[0],corners[1],corners[2]],[corners[0],corners[2],corners[3]]]);
    }
    let mut farthest = 0.0f32;
    for i in 0..240 {
        let s = g.step(&Pad { left: [0.0,1.0], down: if i >= 180 { button::X } else { 0 }, ..Default::default() });
        farthest = farthest.max((s.body_position[0]-stand[0])*fx + (s.body_position[2]-stand[2])*fz);
    }
    println!("{kind}: forward travel {farthest:.3}, wall at 15m; missing {:08X?}", g.missing());
    assert!(farthest > 5.0 && farthest < 15.2, "guest crossed a vertical course wall");
    assert!(g.missing().is_empty(), "wall contact ran an unlifted game call");
    println!("PASS {kind}: running and boosting stop at the wall");
}

fn add_triangles(g: &mut GuestSonic, world: u32, kind: &str, triangles: &[[[f32;3];3]]) {
    if kind == "welded" {
        for tri in weld_mesh(triangles) { g.add_fixed_triangle_welded(world,tri.vertices,7,tri.welding_info); }
    } else { for &tri in triangles { g.add_fixed_triangle(world,tri,7); } }
}
