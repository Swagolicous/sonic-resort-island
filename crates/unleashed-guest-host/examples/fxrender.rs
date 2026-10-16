//! Runs the game's particle render code and prints the draws it issues (gfx.rs capture).
//!
//! Script: jump at frame 60, boost from 150 (as sparkle.rs). At the chosen frame this
//! calls SWA::CSparkleManager::CTypicalRenderer's Render (sub_827D9358: renderer, render
//! info, category symbol, 0) for the categories "opaque", "punchthrough", "transparent",
//! with a render info built here: {0, the rendering device wrapper, a camera block with
//! the view matrix at +16}.
//!
//! usage: fxrender <82346F08_pre.bin> <frame>
use unleashed_guest_host::*;
use xenon_rt::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: fxrender <82346F08_pre.bin> <frame>");
    let frame: u32 = args.next().map_or(80, |s| s.parse().unwrap());
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    g.sparkle = true;
    g.message_pass = true;
    let world = g.havok_world().expect("no hkpWorld");
    for (body, motion, _, shape) in g.rigid_bodies(world) {
        if motion == 7 && shape != 0 {
            g.remove_body(world, body);
        }
    }
    let feet = g.sonic().body_position;
    g.add_fixed_box(world, [feet[0], feet[1] - 0.5, feet[2]], [8000.0, 0.5, 8000.0], [0.0, 0.0, 0.0, 1.0], 0.0, 0x7);
    for f in 0..=frame {
        let mut down = 0;
        if (60..75).contains(&f) {
            down |= button::A;
        }
        if (150..270).contains(&f) {
            down |= button::X;
        }
        g.step(&Pad { down, left: [0.0, 1.0], ..Default::default() });
    }
    let b = g.base();
    let rd = |a: u32| unsafe { PPC_LOAD_U32(b, a) };
    let rf = |a: u32| f32::from_bits(rd(a));
    let heap = || (0x0002_0000u32..0x7FEA_0000).step_by(4);
    let manager = g.particle_manager.expect("no particle manager");
    let renderer = heap().find(|&p| rd(p) == 0x8207_561C && rd(p + 8) == manager).expect("no typical renderer");
    let infrastructure = rd(manager + 16);
    println!("manager {manager:08X}, renderer {renderer:08X}, infrastructure {infrastructure:08X} -> device {:08X}", rd(infrastructure));
    // The rendering device wrapper: an object whose first word is the infrastructure and
    // whose byte +230 is a constant register (sub_82E17B40).
    let wrappers: Vec<u32> = heap().filter(|&p| rd(p) == infrastructure && unsafe { PPC_LOAD_U8(b, p + 230) } < 0xF8 && p != manager + 16).collect();
    println!("wrapper candidates: {:08X?}; their +230 bytes: {:?}", &wrappers[..wrappers.len().min(8)], wrappers.iter().take(8).map(|&w| unsafe { PPC_LOAD_U8(b, w + 230) }).collect::<Vec<_>>());
    // A view matrix inside the game camera: orthonormal rows, and the eye maps to zero.
    let camera = g.game_camera.expect("no game camera");
    let eye = [rf(camera + 0x1B0), rf(camera + 0x1B4), rf(camera + 0x1B8)];
    let mut views = Vec::new();
    for o in (0..0x600u32).step_by(4) {
        let m: [[f32; 4]; 4] = std::array::from_fn(|r| std::array::from_fn(|c| rf(camera + o + 16 * r as u32 + 4 * c as u32)));
        let unit = (0..3).all(|r| ((m[r][0] * m[r][0] + m[r][1] * m[r][1] + m[r][2] * m[r][2]).sqrt() - 1.0).abs() < 0.02);
        let at_eye: f32 = (0..3).map(|c| (eye[0] * m[0][c] + eye[1] * m[1][c] + eye[2] * m[2][c] + m[3][c]).abs()).sum();
        if unit && at_eye < 0.05 && (m[3][3] - 1.0).abs() < 1e-3 {
            views.push(o);
        }
    }
    println!("game camera {camera:08X} eye {eye:.2?}; view matrices at offsets {views:X?}");
    let Some(&view) = views.first() else {
        println!("no view matrix");
        return;
    };
    // A render info the game itself built (left over on a stack or in an object): a block
    // {x, wrapper, camera} whose third word is the game camera's Mirage camera block.
    let mirage_camera = camera + view - 16;
    let mut infos = Vec::new();
    let mut p = 0x0001_0000u32;
    while p < 0x7FFF_0000 {
        if rd(p + 8) == mirage_camera && wrappers.contains(&rd(p + 4)) {
            infos.push((p, rd(p), rd(p + 4)));
        }
        p += 4;
    }
    println!("render infos found in memory (address, +0, wrapper): {:08X?}", &infos[..infos.len().min(12)]);
    let holders: Vec<u32> = { let mut v = Vec::new(); let mut p = 0x0001_0000u32; while p < 0x7FFF_0000 && v.len() < 40 { if rd(p) == mirage_camera { v.push(p); } p += 4; } v };
    println!("pointers to the Mirage camera block {mirage_camera:08X}: {:08X?}", holders);
    for &h in holders.iter().take(40) {
        println!("   {h:08X}: prev words {:08X} {:08X} {:08X}", rd(h - 12), rd(h - 8), rd(h - 4));
    }
    // The game's own render info (infrastructure, rendering device wrapper, camera).
    let Some(&(info, _, wrapper)) = infos.first() else {
        println!("no render info to use");
        return;
    };
    println!("using render info {info:08X} (wrapper {wrapper:08X})");
    g.gfx().capture = true;
    for (name, symbol) in [("opaque", 0x8336_9060u32), ("punchthrough", 0x8336_905C), ("transparent", 0x8336_9058)] {
        let before = g.gfx().draws.len();
        g.guest_call(0x827D_9358, &[renderer, info, rd(symbol), 0]);
        let (n, calls) = (g.gfx().draws.len() - before, g.gfx().calls.clone());
        println!("{name} (symbol {:08X}): {n} draws; hooks so far {calls:?}; missing {:08X?}", rd(symbol), g.missing());
    }
    println!("hooks reached: {:?}", g.gfx().calls);
    let draws = g.gfx().draws.clone();
    for (i, d) in draws.iter().enumerate().take(24) {
        println!("draw {i}: primitive {} x{} ({} vertices, stride {}), indexed {}, {} vertex bytes, {} indices", d.primitive, d.primitive_count, d.vertex_count, d.stride, d.indexed, d.vertices.len(), d.indices.len());
        for (kind, pointer) in [("vb", d.vertex_buffer), ("ib", d.index_buffer), ("decl", d.declaration), ("vs", d.vertex_shader), ("ps", d.pixel_shader), ("tex0", d.textures[0]), ("tex1", d.textures[1])] {
            if pointer != 0 {
                println!("   {kind} {pointer:08X}: {:?}", g.resource_names(pointer));
            }
        }
        let states: Vec<String> = d.render_states.iter().map(|(s, v)| format!("{s}={v:#X}")).collect();
        println!("   render states {}", states.join(" "));
        println!("   sampler 0 {:08X?}", d.samplers.first());
        // First vertex, as big-endian words and floats.
        let first: Vec<String> = d.vertices.chunks(4).take((d.stride / 4) as usize).map(|c| {
            let w = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
            let f = f32::from_bits(w);
            if f.is_finite() && f.abs() > 1e-4 && f.abs() < 1e5 { format!("{f:.3}") } else { format!("{w:08X}") }
        }).collect();
        println!("   first vertex: {}", first.join(" "));
        let rows = |c: &Vec<[f32; 4]>, from: usize, n: usize| -> String { c.iter().skip(from).take(n).map(|r| format!("[{:.3} {:.3} {:.3} {:.3}]", r[0], r[1], r[2], r[3])).collect::<Vec<_>>().join(" ") };
        if !d.indexed {
            // Every vertex of the first quad, as words.
            for v in d.vertices.chunks(d.stride as usize).take(4) {
                let words: Vec<String> = v.chunks(4).map(|c| format!("{:08X}", u32::from_be_bytes([c[0], c[1], c[2], c[3]]))).collect();
                let uv = [f32::from_bits(u32::from_be_bytes([v[12], v[13], v[14], v[15]])), f32::from_bits(u32::from_be_bytes([v[16], v[17], v[18], v[19]]))];
                println!("      {}  uv {uv:.3?}", words.join(" "));
            }
        }
        if d.indexed {
            // What the mesh particle programs read: the world matrix, the high vertex
            // constants, and the pixel program's diffuse, opacity and particle parameter.
            println!("   world: {}", rows(&d.vertex_constants, 8, 4));
            let high = |c: &Vec<[f32; 4]>, from: usize| -> String {
                c.iter().enumerate().skip(from).filter(|(_, r)| r.iter().any(|x| *x != 0.0)).map(|(k, r)| format!("c{k}=[{:.3} {:.3} {:.3} {:.3}]", r[0], r[1], r[2], r[3])).collect::<Vec<_>>().join(" ")
            };
            println!("   vertex high: {}", high(&d.vertex_constants, 150));
            println!("   pixel c16 {:.3?} c21 {:.3?} high: {}", d.pixel_constants.get(16), d.pixel_constants.get(21), high(&d.pixel_constants, 100));
        }
        if d.indexed && i < 2 {
            println!("   vertex c0..c11: {}", rows(&d.vertex_constants, 0, 12));
            let used: Vec<String> = d.vertex_constants.iter().enumerate().skip(12).filter(|(_, r)| r.iter().any(|x| *x != 0.0)).take(40).map(|(k, r)| format!("c{k}=[{:.3} {:.3} {:.3} {:.3}]", r[0], r[1], r[2], r[3])).collect();
            println!("   vertex other: {}", used.join(" "));
            let used: Vec<String> = d.pixel_constants.iter().enumerate().filter(|(_, r)| r.iter().any(|x| *x != 0.0)).take(40).map(|(k, r)| format!("c{k}=[{:.3} {:.3} {:.3} {:.3}]", r[0], r[1], r[2], r[3])).collect();
            println!("   pixel: {}", used.join(" "));
        }
    }
    println!("missing {:08X?}", g.missing());
}
