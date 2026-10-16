//! Differential tests: the port's arithmetic against the original guest functions.

use unleashed_game::sonic::xenon_math;
use xenon_oracle::Oracle;

/// Small deterministic generator so failures reproduce.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    /// Mix of ordinary, tiny, huge and exactly-zero components.
    fn f32(&mut self) -> f32 {
        match self.next() % 8 {
            0 => 0.0,
            1 => f32::from_bits((self.next() as u32 & 0x007F_FFFF) | 0x3400_0000) * if self.next() & 1 == 0 { 1.0 } else { -1.0 },
            2 => (self.next() % 2_000_000) as f32 * 1e3 - 1e9,
            _ => (self.next() % 2_000_001) as f32 / 1000.0 - 1000.0,
        }
    }
}

/// sub_8231A468 with ctx+0x565 = 0: length of ctx+0x220 (the game's rsqrt + Newton idiom).
#[test]
fn speed_length_matches_sub_8231a468() {
    let Some(mut o) = Oracle::get() else {
        eprintln!("xenon oracle not built; skipping");
        return;
    };
    let ctx = o.alloc(0x800);
    let mut rng = Rng(0x5EED_1234_ABCD_0001);
    for i in 0..20_000 {
        let v = [rng.f32(), rng.f32(), rng.f32()];
        o.write_vec4(ctx + 0x220, [v[0], v[1], v[2], 0.0]);
        o.write_u8(ctx + 0x565, 0);
        o.set_r(3, ctx as u64);
        o.call(0x8231A468);
        let guest = o.f(1) as f32;
        let port = xenon_math::length3(v);
        assert_eq!(guest.to_bits(), port.to_bits(), "sample {i}: v={v:?} guest={guest} port={port}");
    }
    assert_eq!(o.import_hits(), 0);
}

fn bits4(v: [f32; 4]) -> [u32; 4] {
    v.map(f32::to_bits)
}

/// sub_82E86198(out r3, v r4): normalize.
#[test]
fn normalize_matches_sub_82e86198() {
    let Some(mut o) = Oracle::get() else { return };
    let (vin, out) = (o.alloc(16), o.alloc(16));
    let mut rng = Rng(0x0BAD_F00D_0000_0002);
    for i in 0..20_000 {
        let v = [rng.f32(), rng.f32(), rng.f32(), rng.f32()];
        o.write_vec4(vin, v);
        o.set_r(3, out as u64);
        o.set_r(4, vin as u64);
        o.call(0x82E86198);
        let guest = o.read_vec4(out);
        let port = xenon_math::normalize3(v);
        assert_eq!(bits4(guest), bits4(port), "sample {i}: v={v:?} guest={guest:?} port={port:?}");
    }
    assert_eq!(o.import_hits(), 0);
}

/// sub_82E84A90(out r3, matrix r4, v r5): rotate by the upper rows.
#[test]
fn transform_matches_sub_82e84a90() {
    let Some(mut o) = Oracle::get() else { return };
    let (m, vin, out) = (o.alloc(64), o.alloc(16), o.alloc(16));
    let mut rng = Rng(0xFEED_0000_0000_0003);
    for i in 0..20_000 {
        let mat: xenon_math::Mat4 = std::array::from_fn(|_| [rng.f32(), rng.f32(), rng.f32(), rng.f32()]);
        let v = [rng.f32(), rng.f32(), rng.f32(), rng.f32()];
        for (r, row) in mat.iter().enumerate() {
            o.write_vec4(m + 16 * r as u32, *row);
        }
        o.write_vec4(vin, v);
        o.set_r(3, out as u64);
        o.set_r(4, m as u64);
        o.set_r(5, vin as u64);
        o.call(0x82E84A90);
        let guest = o.read_vec4(out);
        let port = xenon_math::transform3(&mat, v);
        assert_eq!(bits4(guest), bits4(port), "sample {i}: guest={guest:?} port={port:?}");
    }
}

/// sub_82E84BE0(out r3, quaternion r4): rotation matrix.
#[test]
fn quat_to_matrix_matches_sub_82e84be0() {
    let Some(mut o) = Oracle::get() else { return };
    let (q, out) = (o.alloc(16), o.alloc(64));
    let mut rng = Rng(0x1234_5678_9ABC_0004);
    for i in 0..20_000 {
        let quat = [rng.f32(), rng.f32(), rng.f32(), rng.f32()];
        o.write_vec4(q, quat);
        o.set_r(3, out as u64);
        o.set_r(4, q as u64);
        o.call(0x82E84BE0);
        let port = xenon_math::quat_to_matrix(quat);
        for r in 0..4 {
            let guest = o.read_vec4(out + 16 * r as u32);
            assert_eq!(bits4(guest), bits4(port[r]), "sample {i} row {r}: q={quat:?} guest={guest:?} port={:?}", port[r]);
        }
    }
}

/// sub_8230A4E0(out r3, ctx r4): forward axis of the matrix ctx->[0x10]->vf2() returns.
#[test]
fn forward_matches_sub_8230a4e0() {
    let Some(mut o) = Oracle::get() else { return };
    let (ctx, obj, vtable, out) = (o.alloc(0x40), o.alloc(0x500), o.alloc(0x10), o.alloc(16));
    // vtable slot 2 -> sub_823161E0 (addi r3,r3,0x440; blr): the matrix sits at obj+0x440.
    o.write_u32(vtable + 8, 0x823161E0);
    o.write_u32(obj, vtable);
    o.write_u32(ctx + 0x10, obj);
    let mut rng = Rng(0xFACE_0000_0000_0005);
    for i in 0..20_000 {
        let m: xenon_math::Mat4 = std::array::from_fn(|_| [rng.f32(), rng.f32(), rng.f32(), rng.f32()]);
        for (r, row) in m.iter().enumerate() {
            o.write_vec4(obj + 0x440 + 16 * r as u32, *row);
        }
        o.set_r(3, out as u64);
        o.set_r(4, ctx as u64);
        o.call(0x8230A4E0);
        let (guest, port) = (o.read_vec4(out), xenon_math::forward(&m));
        assert_eq!(bits4(guest), bits4(port), "sample {i}: guest={guest:?} port={port:?}");
    }
}

/// sub_8249A6A8(angle* r3, reference f1): wrap to within pi of the reference.
#[test]
fn wrap_near_matches_sub_8249a6a8() {
    let Some(mut o) = Oracle::get() else { return };
    let p = o.alloc(16);
    let mut rng = Rng(0xA261_0000_0000_0006);
    for i in 0..20_000 {
        let (a, r) = ((rng.next() % 40_000) as f32 / 1000.0 - 20.0, (rng.next() % 40_000) as f32 / 1000.0 - 20.0);
        let (a, r) = if i % 5 == 0 { (rng.f32(), rng.f32()) } else { (a, r) };
        o.write_f32(p, a);
        o.set_r(3, p as u64);
        o.set_f(1, r as f64);
        o.call(0x8249A6A8);
        let (guest, port) = (o.read_f32(p), xenon_math::wrap_near(a, r));
        assert_eq!(guest.to_bits(), port.to_bits(), "sample {i}: a={a} r={r} guest={guest} port={port}");
    }
}

/// sub_831B3A68(f1 = y, f2 = x): the CRT atan2, in double.
#[test]
fn atan2_matches_sub_831b3a68() {
    use unleashed_game::sonic::crt_math;
    let Some(mut o) = Oracle::get() else { return };
    let mut rng = Rng(0xA7A2_0000_0000_0007);
    let special = [0.0, -0.0, 1.0, -1.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN, 1e-300, -1e300];
    for i in 0..40_000 {
        let (y, x) = if i < special.len() * special.len() {
            (special[i / special.len()], special[i % special.len()])
        } else {
            (rng.f32() as f64 * if i % 3 == 0 { 1.0 + 1e-9 } else { 1.0 }, rng.f32() as f64)
        };
        o.set_f(1, y);
        o.set_f(2, x);
        o.call(0x831B3A68);
        let (guest, port) = (o.f(1), crt_math::atan2(y, x));
        assert!(guest.to_bits() == port.to_bits() || (guest.is_nan() && port.is_nan()),
            "sample {i}: atan2({y:e}, {x:e}) guest={guest:e} port={port:e}");
    }
}

fn same_f64(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

/// Angles: ordinary, near multiples of pi/2, tiny, huge, specials.
fn angle(rng: &mut Rng, i: usize) -> f64 {
    let special = [0.0, -0.0, 1e-310, f64::INFINITY, f64::NAN, 2.2e8, -2.3e8, 1e18, 9.3e18];
    if i < special.len() {
        return special[i];
    }
    match rng.next() % 4 {
        0 => (rng.next() % 64) as f64 * std::f64::consts::FRAC_PI_2 + ((rng.next() % 2001) as f64 - 1000.0) * 1e-12,
        1 => rng.f32() as f64,
        _ => ((rng.next() % 2_000_001) as f64 / 1e5 - 10.0) * if rng.next() & 1 == 0 { 1.0 } else { 37.0 },
    }
}

/// sub_831B1788 / sub_831B1868: CRT sin and cos, in double.
#[test]
fn sin_cos_match_sub_831b1788_and_sub_831b1868() {
    use unleashed_game::sonic::crt_math;
    let Some(mut o) = Oracle::get() else { return };
    let mut rng = Rng(0x51C0_0000_0000_0008);
    for i in 0..40_000 {
        let x = angle(&mut rng, i);
        o.set_f(1, x);
        o.call(0x831B1788);
        let (guest, port) = (o.f(1), crt_math::sin(x));
        assert!(same_f64(guest, port), "sample {i}: sin({x:e}) guest={guest:e} port={port:e}");
        o.set_f(1, x);
        o.call(0x831B1868);
        let (guest, port) = (o.f(1), crt_math::cos(x));
        assert!(same_f64(guest, port), "sample {i}: cos({x:e}) guest={guest:e} port={port:e}");
    }
}

/// sub_82453900(f1, sin* r4, cos* r5).
#[test]
fn sincos_matches_sub_82453900() {
    use unleashed_game::sonic::crt_math;
    let Some(mut o) = Oracle::get() else { return };
    let (s, c) = (o.alloc(4), o.alloc(4));
    let mut rng = Rng(0x51C0_0000_0000_0009);
    for i in 0..20_000 {
        let x = angle(&mut rng, i) as f32 as f64;
        o.set_f(1, x);
        o.set_r(4, s as u64);
        o.set_r(5, c as u64);
        o.call(0x82453900);
        let port = crt_math::sincos(x);
        let guest = (o.read_f32(s), o.read_f32(c));
        assert!(same_f64(guest.0 as f64, port.0 as f64) && same_f64(guest.1 as f64, port.1 as f64),
            "sample {i}: sincos({x:e}) guest={guest:?} port={port:?}");
    }
}

/// sub_82E85988(out r3, axis r4, angle f1): axis-angle quaternion.
#[test]
fn axis_angle_quat_matches_sub_82e85988() {
    let Some(mut o) = Oracle::get() else { return };
    let (axis, out) = (o.alloc(16), o.alloc(16));
    let mut rng = Rng(0xA815_0000_0000_000A);
    for i in 0..20_000 {
        let a = [rng.f32(), rng.f32(), rng.f32(), rng.f32()];
        let ang = angle(&mut rng, i) as f32;
        o.write_vec4(axis, a);
        o.set_r(3, out as u64);
        o.set_r(4, axis as u64);
        o.set_f(1, ang as f64);
        o.call(0x82E85988);
        let (guest, port) = (o.read_vec4(out), xenon_math::axis_angle_quat(a, ang));
        assert!((0..4).all(|k| same_f64(guest[k] as f64, port[k] as f64)), "sample {i}: guest={guest:?} port={port:?}");
    }
}

/// sub_82318850(ctx r3, dir r4, reference r5): rotation force rate from the angle
/// between them. The parameter lookup (sub_8245DB60) is mocked with the test's values.
#[test]
fn rotation_force_rate_matches_sub_82318850() {
    use std::sync::{Arc, Mutex};
    use unleashed_game::sonic::posture::{rotation_force_rate, RotationForceRate};
    let Some(mut o) = Oracle::get() else { return };
    let (ctx, dir, reference) = (o.alloc(0x200), o.alloc(16), o.alloc(16));
    let params = Arc::new(Mutex::new([0f32; 4]));
    let p = params.clone();
    o.mock(0x8245DB60, move |regs| {
        let v = p.lock().unwrap()[(regs.r(4) - 0x88) as usize];
        regs.set_f(1, v as f64);
    });
    let mut rng = Rng(0x0707_0000_0000_000B);
    for i in 0..20_000 {
        // The shipped values (0.5 @ 30 deg, 1.0 @ 60 deg) half the time, anything otherwise.
        let p = if i % 2 == 0 {
            RotationForceRate { rate_min: 0.5, min_angle: 30.0, rate_max: 1.0, max_angle: 60.0 }
        } else {
            RotationForceRate { rate_min: rng.f32(), min_angle: rng.f32(), rate_max: rng.f32(), max_angle: rng.f32() }
        };
        *params.lock().unwrap() = [p.rate_min, p.min_angle, p.rate_max, p.max_angle];
        let (d, r) = ([rng.f32(), rng.f32(), rng.f32(), rng.f32()], [rng.f32(), rng.f32(), rng.f32(), rng.f32()]);
        o.write_vec4(dir, d);
        o.write_vec4(reference, r);
        o.set_r(3, ctx as u64);
        o.set_r(4, dir as u64);
        o.set_r(5, reference as u64);
        o.call(0x82318850);
        let (guest, port) = (o.f(1) as f32, rotation_force_rate(&p, d, r));
        assert!(same_f64(guest as f64, port as f64), "sample {i}: {p:?} guest={guest} port={port}");
    }
}

/// sub_831B3B98 (r4 = 0 asin, 1 acos) and sub_831B3D58 (acos).
#[test]
fn asin_acos_match_sub_831b3b98() {
    use unleashed_game::sonic::crt_math;
    let Some(mut o) = Oracle::get() else { return };
    let mut rng = Rng(0xAC05_0000_0000_000C);
    let special = [0.0, -0.0, 0.5, -0.5, 1.0, -1.0, 1.0000001, -2.0, f64::NAN, f64::INFINITY, 1e-300];
    for i in 0..40_000 {
        let x = if i < special.len() { special[i] } else {
            match rng.next() % 3 { 0 => rng.f32() as f64, _ => ((rng.next() % 2_000_001) as f64 / 1e6 - 1.0) * 1.0000001 }
        };
        for acos in [false, true] {
            o.set_f(1, x);
            o.set_r(4, acos as u64);
            o.call(0x831B3B98);
            let port = if acos { crt_math::acos(x) } else { crt_math::asin(x) };
            assert!(same_f64(o.f(1), port), "sample {i}: {}({x:e}) guest={:e} port={port:e}", if acos { "acos" } else { "asin" }, o.f(1));
        }
    }
}
