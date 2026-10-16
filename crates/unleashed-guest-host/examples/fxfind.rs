//! Where does a particle object keep its live particles? For each particle object of each
//! live effect at the chosen frame, takes every block reachable from the object through up
//! to two pointers and reports the floats in them that change steadily over the next two
//! frames (a spinning mesh's angle, an age, a scrolling texture coordinate), with the
//! per-frame change.
//!
//! usage: fxfind <82346F08_pre.bin> <frame>
use std::collections::BTreeMap;
use unleashed_guest_host::*;
use xenon_rt::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: fxfind <82346F08_pre.bin> <frame>");
    let frame: u32 = args.next().expect("frame").parse().unwrap();
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
    let pad = |f: u32| {
        let mut down = 0;
        if (60..75).contains(&f) {
            down |= button::A;
        }
        if (150..270).contains(&f) {
            down |= button::X;
        }
        Pad { down, left: [0.0, 1.0], ..Default::default() }
    };
    for f in 0..=frame {
        g.step(&pad(f));
    }
    let b = g.base();
    let rd = |a: u32| unsafe { PPC_LOAD_U32(b, a) };
    let heap = |p: u32| (0x2_0000..0x7FEA_0000).contains(&p) && p & 3 == 0;
    // block address -> (owner label, path)
    let mut blocks: BTreeMap<u32, String> = BTreeMap::new();
    for effect in g.sparkle_effects() {
        blocks.entry(effect).or_insert(format!("effect {effect:08X}"));
        for (k, (emitter, particles)) in g.sparkle_emitters(effect).into_iter().enumerate() {
            blocks.entry(emitter).or_insert(format!("emitter {k}"));
            for (j, p) in particles.into_iter().enumerate() {
                blocks.entry(p).or_insert(format!("particle {k}.{j}"));
                for o1 in (0..0x1C0u32).step_by(4) {
                    let q = rd(p + o1);
                    if !heap(q) {
                        continue;
                    }
                    blocks.entry(q).or_insert(format!("particle {k}.{j} *(+{o1:#X})"));
                    for o2 in (0..0x100u32).step_by(4) {
                        let r = rd(q + o2);
                        if heap(r) {
                            blocks.entry(r).or_insert(format!("particle {k}.{j} *(*(+{o1:#X})+{o2:#X})"));
                        }
                    }
                }
            }
        }
    }
    const SPAN: u32 = 0x180;
    let shot = |blocks: &BTreeMap<u32, String>| -> Vec<Vec<f32>> {
        blocks.keys().map(|&a| (0..SPAN / 4).map(|i| f32::from_bits(rd(a + 4 * i))).collect()).collect()
    };
    let s0 = shot(&blocks);
    g.step(&pad(frame + 1));
    let s1 = shot(&blocks);
    g.step(&pad(frame + 2));
    let s2 = shot(&blocks);
    println!("frame {frame}: {} blocks", blocks.len());
    for (bi, (addr, label)) in blocks.iter().enumerate() {
        let mut hits = Vec::new();
        for i in 0..(SPAN / 4) as usize {
            let (a, c, e) = (s0[bi][i], s1[bi][i], s2[bi][i]);
            let (d1, d2) = (c - a, e - c);
            if a.is_finite() && c.is_finite() && e.is_finite() && a.abs() < 1e6 && d1.abs() > 1e-4 && (d1 - d2).abs() <= 0.02 * d1.abs().max(1e-3) {
                hits.push(format!("+{:#X}: {a:.4} ({d1:+.4}/frame)", 4 * i));
            }
        }
        if !hits.is_empty() && hits.len() <= 24 {
            println!("{label} @{addr:08X}: {}", hits.join("; "));
            // A block with a fast-changing value (a spin angle): show all of it.
            let fast = (0..(SPAN / 4) as usize).any(|i| { let d = s1[bi][i] - s0[bi][i]; d.is_finite() && d.abs() > 0.5 && d.abs() < 100.0 && (s2[bi][i] - s1[bi][i] - d).abs() < 0.02 * d.abs() });
            if fast && !label.starts_with("emitter") {
                let cells: Vec<String> = (0..(SPAN / 4) as usize).filter(|&i| rd(addr + 4 * i as u32) != 0).map(|i| {
                    let w = rd(addr + 4 * i as u32);
                    let f = f32::from_bits(w);
                    if f.is_finite() && f.abs() > 1e-4 && f.abs() < 1e5 { format!("+{:X}={f:.3}", 4 * i) } else { format!("+{:X}={w:08X}", 4 * i) }
                }).collect();
                println!("      {}", cells.join(" "));
            }
        }
    }
}
