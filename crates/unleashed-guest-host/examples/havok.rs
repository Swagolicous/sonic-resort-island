//! What Sonic collides with in a snapshot: the hkpWorld, its rigid bodies grouped by
//! (motion type, collision filter info, shape class), and the bodies nearest Sonic.
//!
//! Layouts are Havok 5.5's (32-bit): hkpWorldObject {+8 world, +16 collidable {+16 shape,
//! +44 collisionFilterInfo}}, hkpEntity motion at +208 {+216 type, +272 position}.
//!
//! usage: havok <82346F08_pre.bin>
use std::collections::BTreeMap;
use unleashed_guest_host::*;
use xenon_rt::*;

const VT_WORLD: u32 = 0x8212_57A8;
const VT_RIGID_BODY: u32 = 0x8212_5110;

fn main() {
    let path = std::env::args().nth(1).expect("usage: havok <82346F08_pre.bin>");
    let g = GuestSonic::load(path.as_ref()).unwrap();
    let b = g.base();
    let rd = |a: u32| unsafe { PPC_LOAD_U32(b, a) };
    let rf = |a: u32| f32::from_bits(rd(a));
    let s = g.sonic();
    println!("sonic {:?}  proxy {:08X}", s.position, g.proxy);

    let mut worlds = Vec::new();
    let mut bodies = Vec::new();
    let mut p = 0x0002_0000u32;
    while p < 0x7FEA_0000 {
        match rd(p) {
            VT_WORLD => worlds.push(p),
            VT_RIGID_BODY => bodies.push(p),
            _ => {}
        }
        p += 16;
    }
    for &w in &worlds {
        let words: Vec<String> = (0..64).map(|i| format!("{:08X}", rd(w + 4 * i))).collect();
        println!("hkpWorld {w:08X}:");
        for row in words.chunks(8) {
            println!("   {}", row.join(" "));
        }
    }

    // (world, motion type, filter info, shape vtable) -> count, an example, nearest distance
    let mut groups: BTreeMap<(u32, u32, u32, u32), (usize, u32, f32)> = BTreeMap::new();
    let mut near = Vec::new();
    for &r in &bodies {
        let shape = rd(r + 16);
        let shape_vt = if shape >= 0x1000 { rd(shape) } else { 0 };
        let motion = unsafe { PPC_LOAD_U8(b, r + 216) } as u32;
        let pos = [rf(r + 272), rf(r + 276), rf(r + 280)];
        let d = (0..3).map(|i| (pos[i] - s.position[i]).powi(2)).sum::<f32>().sqrt();
        let e = groups.entry((rd(r + 8), motion, rd(r + 44), shape_vt)).or_insert((0, r, f32::MAX));
        e.0 += 1;
        if d < e.2 {
            e.2 = d;
            e.1 = r;
        }
        near.push((d, r, motion, rd(r + 44), shape_vt, pos));
    }
    println!("{} hkpRigidBody; groups (world, motion, filter, shape vtable): count, nearest body, distance", bodies.len());
    for ((w, m, f, sv), (n, ex, d)) in &groups {
        println!("   world {w:08X} motion {m} filter {f:08X} shape {sv:08X}: {n:5}  {ex:08X}  {d:9.2}");
    }
    near.sort_by(|a, c| a.0.total_cmp(&c.0));
    println!("nearest bodies:");
    for (d, r, m, f, sv, pos) in near.iter().take(12) {
        println!("   {r:08X} d {d:8.2} motion {m} filter {f:08X} shape {sv:08X} userData {:08X} pos {pos:?}", rd(r + 12));
    }
    // The character proxy's own Havok objects: every pointer in the proxy whose target
    // is a world object of one of the worlds.
    println!("proxy world objects:");
    for o in (0..0x400).step_by(4) {
        let q = rd(g.proxy + o);
        if q >= 0x2_0000 && q < 0x7FEA_0000 && q & 3 == 0 && worlds.contains(&rd(q + 8)) {
            println!("   proxy+{o:#X} -> {q:08X} vtable {:08X} filter {:08X} shape {:08X}", rd(q), rd(q + 44), rd(q + 16));
        }
    }
}
