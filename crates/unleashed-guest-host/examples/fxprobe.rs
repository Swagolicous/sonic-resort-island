//! Do the game's effect objects react to Sonic in the frame loop? Counts live objects of
//! given classes (by vtable) on the heap before boosting, one second into a boost and one
//! second after releasing it, and lists the indirect-call targets that only the boosting
//! frames reach.
//!
//! usage: fxprobe <82346F08_pre.bin> <vtable hex>=<class>...
use unleashed_guest_host::*;
use xenon_rt::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: fxprobe <82346F08_pre.bin> <vtable hex>=<class>...");
    let classes: Vec<(u32, String)> = args.map(|a| {
        let (v, n) = a.split_once('=').unwrap();
        (u32::from_str_radix(v, 16).unwrap(), n.to_string())
    }).collect();
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    let world = g.havok_world().expect("no hkpWorld");
    for (body, motion, _, shape) in g.rigid_bodies(world) {
        if motion == 7 && shape != 0 {
            g.remove_body(world, body);
        }
    }
    let feet = g.sonic().body_position;
    g.add_fixed_box(world, [feet[0], feet[1] - 0.5, feet[2]], [8000.0, 0.5, 8000.0], [0.0, 0.0, 0.0, 1.0], 0.0, 0x7);
    let b = g.base();
    let count = |classes: &[(u32, String)]| -> Vec<usize> {
        let mut n = vec![0; classes.len()];
        let mut p = 0x0002_0000u32;
        while p < 0x7FEA_0000 {
            let w = unsafe { PPC_LOAD_U32(b, p) };
            if w >> 24 == 0x82 {
                if let Some(i) = classes.iter().position(|c| c.0 == w) {
                    n[i] += 1;
                }
            }
            p += 4;
        }
        n
    };
    let run = Pad { left: [0.0, 1.0], ..Default::default() };
    let boost = Pad { down: button::X, ..run };
    for _ in 0..60 {
        g.step(&run);
    }
    let before = count(&classes);
    trace_indirect(true);
    for _ in 0..30 {
        g.step(&run);
    }
    let quiet = trace_indirect(false).unwrap();
    trace_indirect(true);
    for _ in 0..60 {
        g.step(&boost);
    }
    let boosting = trace_indirect(false).unwrap();
    let during = count(&classes);
    for _ in 0..60 {
        g.step(&run);
    }
    let after = count(&classes);
    println!("objects (before / boosting / after):");
    for (i, (v, name)) in classes.iter().enumerate() {
        if before[i] + during[i] + after[i] > 0 {
            println!("   {v:08X} {name}: {} / {} / {}", before[i], during[i], after[i]);
        }
    }
    let only: Vec<String> = boosting.keys().filter(|a| !quiet.contains_key(a)).map(|a| format!("{a:08X}x{}", boosting[a])).collect();
    println!("indirect targets reached only while boosting ({}): {}", only.len(), only.join(" "));
    println!("missing {:08X?}", g.missing());
}
