//! Finds the boost gauge by how it behaves: a value anywhere on the guest heap (read as
//! a float or as an integer) that falls while boosting and stays put once boost is
//! released. On a flat slab: 2 s boosting, then 2 s running without boost.
//!
//! usage: hudscan <82346F08_pre.bin>
use unleashed_guest_host::*;
use xenon_rt::*;

fn main() {
    let path = std::env::args().nth(1).expect("usage: hudscan <82346F08_pre.bin>");
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
    let rd = |a: u32| unsafe { PPC_LOAD_U32(b, a) };
    // A word as a number: a float of ordinary size, else a small integer.
    let value = |w: u32| -> Option<f64> {
        let f = f32::from_bits(w);
        if f.is_finite() && f.abs() > 1e-3 && f.abs() < 1e7 {
            Some(f as f64)
        } else if (1..1_000_000).contains(&w) {
            Some(w as f64)
        } else {
            None
        }
    };
    let mut live: Vec<(u32, u32, u32)> = Vec::new(); // address, first word, last word
    let mut a = 0x0002_0000u32;
    while a < 0x7FEA_0000 {
        let w = rd(a);
        if w != 0 && value(w).is_some() {
            live.push((a, w, w));
        }
        a += 4;
    }
    println!("{} candidates", live.len());
    // While boosting: keep what falls. Afterwards: only record.
    let mut after: Vec<Vec<u32>> = Vec::new();
    for f in 0..240u32 {
        let boosting = f < 120;
        g.step(&Pad { down: if boosting { button::X } else { 0 }, left: [0.0, 1.0], ..Default::default() });
        if f % 40 == 39 {
            if boosting {
                live.retain_mut(|(a, _, last)| {
                    let now = rd(*a);
                    let keep = match (value(now), value(*last)) {
                        (Some(n), Some(l)) => n < l,
                        (None, Some(_)) => now == 0,
                        _ => false,
                    };
                    *last = now;
                    keep
                });
                println!("frame {:3}: {} left", f + 1, live.len());
            } else {
                after.push(live.iter().map(|c| rd(c.0)).collect());
            }
        }
    }
    // Gauge-like: started between 1 and 1000 as a number, lost at least a tenth of it.
    for (i, (a, first, last)) in live.iter().enumerate() {
        let (Some(v0), v1) = (value(*first), value(*last).unwrap_or(0.0)) else { continue };
        if !(0.5..=1000.0).contains(&v0) || v1 > v0 * 0.9 || v1 < 0.0 {
            continue;
        }
        let near = if a.wrapping_sub(g.context) < 0x4000 { format!(" (context+{:#X})", a - g.context) } else { String::new() };
        let later: Vec<f64> = after.iter().map(|s| value(s[i]).unwrap_or(0.0)).collect();
        println!("   {a:08X}{near}: {v0:.3} -> {v1:.3} while boosting; then {later:.3?}");
    }
}
