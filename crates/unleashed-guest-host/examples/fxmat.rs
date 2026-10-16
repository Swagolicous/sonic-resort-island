//! Finds the transforms the particle system keeps for an effect: every place on the heap
//! holding a 4x4 matrix (three unit-length rows, then a translation) whose translation is
//! within 2 m of Sonic at the chosen frame, and how much each one's rotation turns in the
//! next frame (the jump ball's meshes spin at fixed rates).
//!
//! usage: fxmat <82346F08_pre.bin> <frame>
use unleashed_guest_host::*;
use xenon_rt::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: fxmat <82346F08_pre.bin> <frame>");
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
    let rf = |a: u32| f32::from_bits(unsafe { PPC_LOAD_U32(b, a) });
    let rows = |a: u32| -> [[f32; 3]; 4] { std::array::from_fn(|r| std::array::from_fn(|c| rf(a + 16 * r as u32 + 4 * c as u32))) };
    let at = g.sonic().body_position;
    let unit = |v: [f32; 3]| ((v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt() - 1.0).abs();
    let mut found = Vec::new();
    let mut a = 0x0002_0000u32;
    while a < 0x7FEA_0000 {
        // translation row first (cheap reject), matrix starts 48 bytes before it
        let x = rf(a + 48);
        if (x - at[0]).abs() < 2.0 && x.abs() > 1.0 {
            let m = rows(a);
            if (m[3][1] - at[1]).abs() < 2.5 && (m[3][2] - at[2]).abs() < 2.0 && m[..3].iter().all(|r| unit(*r) < 0.6 && r.iter().all(|v| v.is_finite())) {
                found.push((a, m));
            }
        }
        a += 4;
    }
    println!("frame {frame}: Sonic feet {at:.2?}; {} matrices near him", found.len());
    g.step(&pad(frame + 1));
    for (a, m) in found {
        let n = rows(a);
        // angle between the two rotations, from the trace of R1^T R2 (rows normalised)
        let norm = |r: [f32; 3]| { let l = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt().max(1e-6); [r[0] / l, r[1] / l, r[2] / l] };
        let trace: f32 = (0..3).map(|i| { let (p, q) = (norm(m[i]), norm(n[i])); p[0] * q[0] + p[1] * q[1] + p[2] * q[2] }).sum();
        let angle = ((trace - 1.0) * 0.5).clamp(-1.0, 1.0).acos().to_degrees();
        let scale = (m[0][0] * m[0][0] + m[0][1] * m[0][1] + m[0][2] * m[0][2]).sqrt();
        if angle > 0.5 {
            println!("   {a:08X}: turned {angle:6.2} deg in a frame ({:.0} deg/s), row scale {scale:.3}, translation {:.2?} -> {:.2?}", angle * 60.0, m[3], n[3]);
        }
    }
}
