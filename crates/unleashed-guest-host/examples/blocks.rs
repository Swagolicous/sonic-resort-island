//! Does Sonic stand on a box this host adds to the game's Havok world? Three runs from
//! the same snapshot, neutral stick, printing his feet height:
//!   1. as captured (stage collision);
//!   2. the stage's fixed bodies removed: he should fall;
//!   3. fixed bodies removed and a box under his feet with `filter`: he should stand.
//!
//! usage: blocks <82346F08_pre.bin> [filter hex]...
use unleashed_guest_host::*;
use xenon_rt::*;

fn run(path: &str, frames: u32, setup: impl FnOnce(&mut GuestSonic, u32)) {
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    let world = g.havok_world().expect("no hkpWorld");
    setup(&mut g, world);
    let mut line = String::new();
    for f in 0..frames {
        let s = g.step(&Pad::default());
        if f % 20 == 19 {
            line += &format!(" {:.2}", s.body_position[1]);
        }
    }
    println!("   feet y every 20 frames:{line}");
    let s = g.sonic();
    println!("   ends at {:.1?}, velocity {:.1?}", s.body_position, s.velocity);
    if !g.missing().is_empty() {
        println!("   missing {:08X?}", g.missing());
    }
}

fn remove_stage(g: &mut GuestSonic, world: u32) -> usize {
    // Fixed bodies with a shape: the stage (the world's own fixed body has none).
    let stage: Vec<u32> = g.rigid_bodies(world).into_iter().filter(|&(_, motion, _, shape)| motion == 7 && shape != 0).map(|r| r.0).collect();
    for &body in &stage {
        g.remove_body(world, body);
    }
    stage.len()
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: blocks <82346F08_pre.bin> [filter hex]...");
    let filters: Vec<u32> = args.map(|a| u32::from_str_radix(&a, 16).unwrap()).collect();
    let filters = if filters.is_empty() { vec![0x7, 0x3CA, 0x6, 0x8] } else { filters };

    {
        let g = GuestSonic::load(path.as_ref()).unwrap();
        let (b, world) = (g.base(), g.havok_world().expect("no hkpWorld"));
        let bodies = g.rigid_bodies(world);
        println!("world {world:08X}: {} bodies, lock count {}", bodies.len(), unsafe { PPC_LOAD_U32(b, world + 132) });
        for (body, _, filter, shape) in bodies.iter().filter(|r| unsafe { PPC_LOAD_U32(b, r.3) } == VT_HKP_BOX_SHAPE).take(4) {
            let f = |a: u32| f32::from_bits(unsafe { PPC_LOAD_U32(b, a) });
            println!("   game box {body:08X} filter {filter:08X}: radius {} half extents {:?}", f(shape + 16), [f(shape + 32), f(shape + 36), f(shape + 40)]);
        }
    }

    println!("1. as captured");
    run(&path, 120, |_, _| {});
    println!("2. stage removed");
    run(&path, 120, |g, world| println!("   removed {} fixed bodies", remove_stage(g, world)));
    for filter in filters {
        println!("3. stage removed, box under his feet, filter {filter:#X}");
        run(&path, 120, |g, world| {
            remove_stage(g, world);
            let feet = g.sonic().body_position;
            let body = g.add_fixed_box(world, [feet[0], feet[1] - 0.5, feet[2]], [500.0, 0.5, 500.0], [0.0, 0.0, 0.0, 1.0], 0.0, filter);
            let b = g.base();
            println!("   box body {body:08X} world {:08X} filter {:08X}", unsafe { PPC_LOAD_U32(b, body + 8) }, unsafe { PPC_LOAD_U32(b, body + 44) });
        });
    }
}
