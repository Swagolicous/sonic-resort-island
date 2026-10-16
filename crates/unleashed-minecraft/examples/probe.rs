//! Fetches Minecraft's files if needed, opens a world and streams its spawn:
//! probe [cache dir] [seed]
fn main() {
    let mut args = std::env::args().skip(1);
    let cache = std::path::PathBuf::from(args.next().unwrap_or_else(|| "../minecraft".into()));
    let seed: i64 = args.next().map(|s| s.parse().unwrap()).unwrap_or(12345);
    let t = std::time::Instant::now();
    let files = unleashed_minecraft::setup::ensure(&cache).expect("minecraft files");
    eprintln!("files ready in {:?}: {}", t.elapsed(), files.display());
    let t = std::time::Instant::now();
    let mut world = unleashed_minecraft::McWorld::load(&files, seed, 4).expect("world");
    eprintln!("world opened in {:?}; spawn {:?}", t.elapsed(), world.origin);
    let t = std::time::Instant::now();
    let mut ticks = 0;
    while !(world.spawn_ready() && world.collision_near([0.0; 3], 1).len() > 100) && ticks < 2000 {
        world.tick([0.0; 3]);
        std::thread::sleep(std::time::Duration::from_millis(5));
        ticks += 1;
    }
    let tris = world.collision_near([0.0; 3], 2);
    eprintln!("spawn streamed in {:?} ({ticks} ticks): {} collision triangles within 2 chunks", t.elapsed(), tris.len());
    let below = tris.iter().filter(|t| t.iter().all(|v| v[0].abs() < 1.0 && v[2].abs() < 1.0)).map(|t| t[0][1]).fold(f32::MIN, f32::max);
    eprintln!("highest collision face under spawn: y = {below}");
    let o = world.origin;
    let cam = unleashed_minecraft::Camera {
        position: glam::DVec3::new(o[0], o[1] + 1.6, o[2]),
        forward: glam::Vec3::new(0.0, 0.0, 1.0),
        fov_degrees: 70.0,
        aspect: 16.0 / 9.0,
        yaw_degrees: 0.0,
        pitch_degrees: 0.0,
    };
    let t = std::time::Instant::now();
    let mut shown = 0;
    while t.elapsed() < std::time::Duration::from_secs(20) && shown < 3 {
        world.tick([0.0; 3]);
        let update = world.frame(&cam);
        for (pos, mesh) in &update.uploads {
            if mesh.vertices.is_empty() { continue; }
            let mut lo = [f32::MAX; 3];
            let mut hi = [f32::MIN; 3];
            for v in &mesh.vertices { for k in 0..3 { lo[k] = lo[k].min(v.position[k]); hi[k] = hi[k].max(v.position[k]); } }
            eprintln!("section {pos:?} (x16 = {:?}): {} vertices, bounds {lo:?}..{hi:?}, light {:?}", (pos.0 * 16, pos.1 * 16, pos.2 * 16), mesh.vertices.len(), mesh.vertices[0].light);
            shown += 1;
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    eprintln!("sun angle {} sky light level {}", world.day.sun_angle_degrees(), world.day.sky_light_level());
}
