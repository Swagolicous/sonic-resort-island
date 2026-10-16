//! Replay pad input on the actual Resort Island road; report reverse corrections.
use std::{fs, path::Path};
use unleashed_guest_host::{GuestSonic, Pad, button, terrain::weld_mesh};
use xenon_rt::PPC_LOAD_U32;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let path = &args[1];
    let directory = Path::new(&args[2]);
    let mode = args.get(3).map(String::as_str).unwrap_or("welded");
    let settings: serde_json::Value = serde_json::from_slice(&fs::read(directory.join("level.json")).unwrap()).unwrap();
    let scale = settings["scale"].as_f64().unwrap() as f32;
    let spawn: [f32; 3] = std::array::from_fn(|k| settings["spawn_obj"][k].as_f64().unwrap() as f32);
    let mut g = GuestSonic::load(Path::new(path)).unwrap();
    let world = g.havok_world().unwrap();
    for (body, motion, _, shape) in g.rigid_bodies(world) {
        if motion == 7 && shape != 0 { g.remove_body(world, body); }
    }
    let original = g.sonic().body_position;
    let slab = g.add_fixed_box(world, [original[0],original[1]-0.5,original[2]], [1000.0,0.5,1000.0], [0.0,0.0,0.0,1.0], 0.0, 7);
    for _ in 0..300 { g.step(&Pad::default()); }
    let origin = g.sonic().body_position;
    g.remove_body(world, slab);
    let camera = g.game_view().unwrap();
    let f = [camera.target[0]-camera.eye[0],camera.target[2]-camera.eye[2]];
    let len = (f[0]*f[0]+f[1]*f[1]).sqrt();
    let forward = [f[0]/len,f[1]/len];
    let yaw = f[0].atan2(f[1]) - (settings["yaw_degrees"].as_f64().unwrap() as f32).to_radians();
    let transform = |p: [f32;3]| {
        let q: [f32;3] = std::array::from_fn(|k| (p[k]-spawn[k])*scale);
        [origin[0]+yaw.cos()*q[0]+yaw.sin()*q[2], origin[1]+q[1], origin[2]-yaw.sin()*q[0]+yaw.cos()*q[2]]
    };
    let obj = fs::read_dir(directory).unwrap().map(|p| p.unwrap().path()).find(|p| p.extension().is_some_and(|s| s=="obj")).unwrap();
    let mut vertices = Vec::<[f32;3]>::new();
    let mut uvs = Vec::<[f32;2]>::new();
    let mut triangles = Vec::new();
    let mut object = String::new();
    let mut material = String::new();
    for line in fs::read_to_string(obj).unwrap().lines() {
        let s: Vec<_> = line.split_whitespace().collect();
        if s.is_empty() { continue; }
        match s[0] {
            "v" => vertices.push([s[1].parse().unwrap(),s[2].parse().unwrap(),s[3].parse().unwrap()]),
            "vt" => uvs.push([s[1].parse().unwrap(),1.0-s[2].parse::<f32>().unwrap()]),
            "o" => object = s[1].into(),
            "usemtl" => material = s[1].into(),
            "f" => {
                let face: Vec<_> = s[1..].iter().map(|token| {
                    let fields: Vec<_> = token.split('/').collect();
                    (fields[0].parse::<usize>().unwrap()-1, fields[1].parse::<usize>().unwrap()-1)
                }).collect();
                for i in 1..face.len()-1 {
                    let indices = [face[0],face[i],face[i+1]];
                    let uv: [f32;2] = std::array::from_fn(|k| indices.iter().map(|&(_,u)| uvs[u][k]).sum::<f32>()/3.0);
                    let rock = material=="1" && uv[0]>0.375 && uv[1]>0.5;
                    let water = material=="5" && uv[0]>0.5 && uv[1]<0.5;
                    if (object.starts_with("Trk") || (object.starts_with("Dec") && rock)) && !water {
                        triangles.push(indices.map(|(p,_)| transform(vertices[p])));
                    }
                }
            }
            _ => {}
        }
    }
    // The central lawn's base plane, below the raised road, stays continuous.
    g.add_fixed_box(world, [origin[0],origin[1]-spawn[1]*scale-0.5,origin[2]], [1000.0,0.5,1000.0], [0.0,0.0,0.0,1.0], 0.0, 7);
    if mode == "raw" {
        for &tri in &triangles { g.add_fixed_triangle(world, tri, 7); }
    } else {
        let welded = weld_mesh(&triangles);
        let internal = welded.iter().filter(|t| t.welding_info != 0x7fff).count();
        println!("welded {} triangles, {} have internal edges", welded.len(), internal);
        for tri in welded { g.add_fixed_triangle_welded(world, tri.vertices, 7, tri.welding_info); }
    }
    for _ in 0..60 { g.step(&Pad::default()); }
    // Verify the runtime's angle table, rather than relying on a different SDK version.
    for code in [0,15,30,31] {
        let row: [f32;6] = std::array::from_fn(|k| unsafe { f32::from_bits(PPC_LOAD_U32(g.base(), 0x8339_a8e0+code*24+k as u32*4)) });
        println!("welding angle {code}: {row:?}");
    }
    let mut previous = g.sonic().body_position;
    let mut reverse = 0;
    let mut largest_reverse = 0.0f32;
    let mut height_changes = 0.0f32;
    let mut max_speed = 0.0f32;
    for frame in 0..300 {
        let s = g.step(&Pad { left: [0.0,1.0], down: if frame>=180 { button::X } else { 0 }, ..Default::default() });
        let p = s.body_position;
        let progress = (p[0]-previous[0])*forward[0]+(p[2]-previous[2])*forward[1];
        if progress < -0.01 { reverse += 1; largest_reverse=largest_reverse.max(-progress); }
        if frame < 80 { height_changes=height_changes.max((p[1]-previous[1]).abs()); }
        let speed = (s.velocity[0]*s.velocity[0]+s.velocity[2]*s.velocity[2]).sqrt();
        max_speed=max_speed.max(speed);
        if frame%10==0 || progress < -0.01 { println!("{mode} frame {frame}: feet {p:?}, velocity {:?}, advance {progress:.4}",s.velocity); }
        previous = p;
    }
    println!("RESULT {mode}: reverse frames {reverse}, largest reverse {largest_reverse:.5}m, early vertical step {height_changes:.5}m, max speed {max_speed:.2}, missing {:08X?}",g.missing());
}
