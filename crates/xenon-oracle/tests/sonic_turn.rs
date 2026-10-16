//! sub_82349538 (Sonic's turning) against the port, on a guest-memory Sonic.

use std::sync::{Arc, Mutex};
use unleashed_game::sonic::posture::{turn, RotationForceRate, TurnInput, TurnState};
use unleashed_game::sonic::xenon_math;
use xenon_oracle::Oracle;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn unit(&mut self) -> f32 {
        (self.next() % 2_000_001) as f32 / 1_000_000.0 - 1.0
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (self.next() % 1_000_001) as f32 / 1_000_000.0 * (hi - lo)
    }
    fn quat(&mut self) -> [f32; 4] {
        let q = [self.unit(), self.unit(), self.unit(), self.unit()];
        let n = (q.iter().map(|c| c * c).sum::<f32>()).sqrt().max(1e-3);
        q.map(|c| c / n)
    }
}

fn bits(v: [f32; 4]) -> [u32; 4] {
    v.map(f32::to_bits)
}

#[test]
fn turn_matches_sub_82349538() {
    let Some(mut o) = Oracle::get() else { return };
    let posture = o.alloc(0x40);
    let timing = o.alloc(0x40);
    let ctx = o.alloc(0x800);
    let body = o.alloc(0x500);
    let vtable = o.alloc(0x10);
    let flags = o.alloc(0x10);
    let flag_bytes = o.alloc(0x40);
    let input = o.alloc(16);
    o.write_u32(posture + 8, ctx);
    o.write_u32(posture + 12, timing);
    o.write_u32(vtable + 8, 0x823161E0); // body vf2: returns body+0x440 (its matrix)
    o.write_u32(body, vtable);
    o.write_u32(ctx + 0x10, body);
    o.write_u32(ctx + 0x4AC, flags);
    o.write_u32(flags + 4, flag_bytes);

    let rate = RotationForceRate { rate_min: 0.5, min_angle: 30.0, rate_max: 1.0, max_angle: 60.0 };
    o.mock(0x8245DB60, move |r| {
        let v = [rate.rate_min, rate.min_angle, rate.rate_max, rate.max_angle][(r.r(4) - 0x88) as usize];
        r.set_f(1, v as f64);
    });
    let body_delta = Arc::new(Mutex::new(None));
    let d = body_delta.clone();
    o.mock(0x82308300, move |r| *d.lock().unwrap() = Some(r.read_vec4(r.r(4) as u32)));

    let mut rng = Rng(0x7123_0000_0000_0001);
    let (mut turned, mut steered) = (0, 0);
    for i in 0..20_000 {
        let rotation = rng.quat();
        let body_m = xenon_math::quat_to_matrix(rng.quat());
        let speed = if i % 7 == 0 { 0.0 } else { rng.range(0.0, 60.0) };
        let velocity = [rng.unit() * speed, rng.unit(), rng.unit() * speed, 0.0];
        let up = if i % 3 == 0 { [0.0, 1.0, 0.0, 0.0] } else { xenon_math::normalize3([rng.unit(), 1.0, rng.unit(), 0.0]) };
        let mag = if i % 11 == 0 { 0.0 } else { rng.range(0.0, 1.0) };
        let inp = TurnInput {
            input: [rng.unit() * mag, 0.0, rng.unit() * mag, 0.0],
            force: rng.range(0.0, 200.0),
            max_turn_speed: rng.range(0.0, 20.0),
            velocity_force: rng.range(0.0, 200.0),
            turn_velocity: i % 4 != 0,
        };
        let s = TurnState {
            rotation,
            forward: xenon_math::forward(&body_m),
            velocity,
            up,
            input_scale: if i % 5 == 0 { 1.0 } else { rng.range(0.0, 2.0) },
            flag_no_turn: i % 13 == 0,
            flag_keep_facing: i % 9 == 0,
            dt: if i % 2 == 0 { 1.0 / 30.0 } else { 1.0 / 60.0 },
            rate,
        };
        for (r, row) in body_m.iter().enumerate() {
            o.write_vec4(body + 0x440 + 16 * r as u32, *row);
        }
        o.write_vec4(ctx + 0x440, s.rotation);
        o.write_vec4(ctx + 0x460, [f32::NAN; 4]);
        o.write_vec4(ctx + 0x220, s.velocity);
        o.write_u8(ctx + 0x565, 0);
        o.write_u8(ctx + 0x566, 0);
        o.write_vec4(ctx + 0x2F0, s.up);
        o.write_f32(ctx + 0x554, s.input_scale);
        o.write_u8(flag_bytes + 3, s.flag_no_turn as u8);
        o.write_u8(flag_bytes + 37, s.flag_keep_facing as u8);
        o.write_f32(timing + 36, s.dt);
        o.write_vec4(input, inp.input);
        *body_delta.lock().unwrap() = None;

        o.set_r(3, posture as u64);
        o.set_r(4, input as u64);
        o.set_r(7, inp.turn_velocity as u64);
        o.set_f(1, inp.force as f64);
        o.set_f(2, inp.max_turn_speed as f64);
        o.set_f(3, inp.velocity_force as f64);
        o.call(0x82349538);

        let port = turn(&s, &inp);
        let target = o.read_vec4(ctx + 0x460);
        let delta = *body_delta.lock().unwrap();
        match port.rotation {
            Some((q, dq)) => {
                turned += 1;
                assert_eq!(bits(target), bits(q), "sample {i}: target rotation guest={target:?} port={q:?}");
                assert_eq!(delta.map(bits), Some(bits(dq)), "sample {i}: body delta guest={delta:?} port={dq:?}");
            }
            None => {
                assert!(target.iter().all(|c| c.is_nan()), "sample {i}: guest set a rotation, port did not ({target:?})");
                assert!(delta.is_none(), "sample {i}: guest turned the body, port did not");
            }
        }
        let v = o.read_vec4(ctx + 0x220);
        match port.velocity {
            Some(pv) => {
                steered += 1;
                assert_eq!(bits(v), bits(pv), "sample {i}: velocity guest={v:?} port={pv:?}");
                assert_eq!(o.read(ctx + 0x566, 1)[0], 1, "sample {i}: velocity not marked dirty");
            }
            None => assert_eq!(bits(v), bits(s.velocity), "sample {i}: guest changed the velocity, port did not ({v:?})"),
        }
    }
    eprintln!("turned facing {turned}x, velocity {steered}x of 20000");
    assert!(turned > 10_000 && steered > 5_000, "too few samples exercised the turning paths");
    assert_eq!(o.import_hits(), 0);
}

/// sub_823472A0(posture r3, speed f1): turning force from speed.
#[test]
fn rotation_force_matches_sub_823472a0() {
    use unleashed_game::sonic::posture::{rotation_force, RotationForceState};
    let Some(mut o) = Oracle::get() else { return };
    let (posture, ctx, flags, flag_bytes, flags2, flag2_bytes) =
        (o.alloc(0x40), o.alloc(0x1000), o.alloc(0x10), o.alloc(0x40), o.alloc(0x10), o.alloc(0x40));
    o.write_u32(posture + 8, ctx);
    o.write_u32(ctx + 0x4AC, flags);
    o.write_u32(flags + 4, flag_bytes);
    o.write_u32(ctx + 0x4B0, flags2);
    o.write_u32(flags2 + 4, flag2_bytes);
    let table = Arc::new(Mutex::new([0f32; 256]));
    let t = table.clone();
    o.mock(0x8245DB60, move |r| {
        let v = t.lock().unwrap()[r.r(4) as usize];
        r.set_f(1, v as f64);
    });
    let mut rng = Rng(0x7124_0000_0000_0002);
    for i in 0..20_000 {
        let mut p = [0f32; 256];
        // Shipped values most of the time, random ones otherwise.
        let shipped = [(0x95, 30.0), (0x96, 1.0), (0x9D, 100.0), (0x9E, 20.0), (0x9F, 80.0), (0xA0, 40.0), (0xA2, 50.0), (0xA3, 1.0)];
        for (id, v) in shipped {
            p[id] = if i % 4 == 0 { rng.range(-50.0, 150.0) } else { v };
        }
        *table.lock().unwrap() = p;
        let s = RotationForceState {
            flag_14: i % 5 == 0,
            flag_22: i % 3 == 0,
            blend: rng.range(0.0, 1.0),
            flag2_7: i % 7 == 0,
            blocked_count: if i % 6 == 0 { (rng.next() % 40) as i32 } else { 0 },
        };
        let speed = if i % 9 == 0 { 0.0 } else { rng.range(-10.0, 80.0) };
        o.write_u8(flag_bytes + 14, s.flag_14 as u8);
        o.write_u8(flag_bytes + 22, s.flag_22 as u8);
        o.write_f32(ctx + 0x510, s.blend);
        o.write_u8(flag2_bytes + 7, s.flag2_7 as u8);
        o.write_u32(ctx + 0xCC8, s.blocked_count as u32);
        o.set_r(3, posture as u64);
        o.set_f(1, speed as f64);
        o.call(0x823472A0);
        let guest = o.f(1) as f32;
        let port = rotation_force(&s, speed, |id| p[id as usize]);
        assert_eq!(guest.to_bits(), port.to_bits(), "sample {i}: speed={speed} {s:?} guest={guest} port={port}");
    }
}

/// sub_8231B990(ctx r3) and sub_82346FE0(posture r3, alt r4).
#[test]
fn rotation_speed_max_and_side_drag_match() {
    use unleashed_game::sonic::posture::{rotation_speed_max, side_drag_force, RotationForceState};
    let Some(mut o) = Oracle::get() else { return };
    let (posture, ctx, flags, flag_bytes, flags2, flag2_bytes) =
        (o.alloc(0x40), o.alloc(0x1000), o.alloc(0x10), o.alloc(0x40), o.alloc(0x10), o.alloc(0x40));
    o.write_u32(posture + 8, ctx);
    o.write_u32(ctx + 0x4AC, flags);
    o.write_u32(flags + 4, flag_bytes);
    o.write_u32(ctx + 0x4B0, flags2);
    o.write_u32(flags2 + 4, flag2_bytes);
    let table = Arc::new(Mutex::new([0f32; 256]));
    let t = table.clone();
    o.mock(0x8245DB60, move |r| {
        let v = t.lock().unwrap()[r.r(4) as usize];
        r.set_f(1, v as f64);
    });
    let mut rng = Rng(0x7125_0000_0000_0003);
    for i in 0..20_000 {
        let mut p = [0f32; 256];
        for (id, v) in [(0x97, 720.0), (0x98, 30.0), (0xA1, 1000.0)] {
            p[id] = if i % 4 == 0 { rng.range(-100.0, 2000.0) } else { v };
        }
        *table.lock().unwrap() = p;
        let s = RotationForceState {
            flag_14: i % 5 == 0,
            flag2_7: i % 7 == 0,
            blocked_count: if i % 3 == 0 { (rng.next() % 40) as i32 - 5 } else { 0 },
            ..Default::default()
        };
        o.write_u8(flag_bytes + 14, s.flag_14 as u8);
        o.write_u8(flag2_bytes + 7, s.flag2_7 as u8);
        o.write_u32(ctx + 0xCC8, s.blocked_count as u32);
        o.set_r(3, ctx as u64);
        o.call(0x8231B990);
        let (guest, port) = (o.f(1) as f32, rotation_speed_max(&s, |id| p[id as usize]));
        assert_eq!(guest.to_bits(), port.to_bits(), "sample {i}: {s:?} guest={guest} port={port}");
        let alt = i % 2 == 0;
        o.set_r(3, posture as u64);
        o.set_r(4, alt as u64);
        o.call(0x82346FE0);
        let (guest, port) = (o.f(1) as f32, side_drag_force(s.flag_14, alt));
        assert_eq!(guest.to_bits(), port.to_bits(), "sample {i}: side drag guest={guest} port={port}");
    }
}

/// sub_823083F0(ctx r3, out r4) and sub_8231AA60(out r3, ctx r4, v r5).
#[test]
fn input_direction_and_remove_into_normal_match() {
    use unleashed_game::sonic::posture::{input_direction, remove_into_normal};
    let Some(mut o) = Oracle::get() else { return };
    let (ctx, vtable, out, v) = (o.alloc(0x800), o.alloc(0x80), o.alloc(16), o.alloc(16));
    o.write_u32(ctx, vtable);
    let threshold = Arc::new(Mutex::new(0f32));
    let t = threshold.clone();
    o.mock(0x8245DB60, move |r| {
        assert_eq!(r.r(4), 12);
        let v = *t.lock().unwrap();
        r.set_f(1, v as f64);
    });
    let mut rng = Rng(0x7126_0000_0000_0004);
    for i in 0..20_000 {
        let locked = i % 7 == 0;
        // vf14: sub_82DE0888 returns 1, sub_8246E640 returns 0.
        o.write_u32(vtable + 56, if locked { 0x82DE0888 } else { 0x8246E640 });
        let th = if i % 2 == 0 { 0.02 } else { rng.range(0.0, 1.0) };
        *threshold.lock().unwrap() = th;
        let s = if i % 5 == 0 { 0.01 } else { 1.0 };
        let stick = [rng.unit() * s, rng.unit() * s, rng.unit() * s, rng.unit()];
        o.write_vec4(ctx + 0x120, stick);
        o.set_r(3, ctx as u64);
        o.set_r(4, out as u64);
        o.call(0x823083F0);
        let (guest, port) = (o.read_vec4(out), input_direction(stick, locked, th));
        assert_eq!(bits(guest), bits(port), "sample {i}: input guest={guest:?} port={port:?}");

        let normal = if i % 6 == 0 { [0.0; 4] } else { [rng.unit(), rng.unit(), rng.unit(), rng.unit()] };
        let vel = [rng.unit() * 50.0, rng.unit() * 50.0, rng.unit() * 50.0, rng.unit()];
        o.write_vec4(ctx + 0x6B0, normal);
        o.write_vec4(v, vel);
        o.set_r(3, out as u64);
        o.set_r(4, ctx as u64);
        o.set_r(5, v as u64);
        o.call(0x8231AA60);
        let (guest, port) = (o.read_vec4(out), remove_into_normal(normal, vel));
        assert_eq!(bits(guest), bits(port), "sample {i}: normal guest={guest:?} port={port:?}");
    }
}

/// sub_82349ED8(posture r3): sets / clears flag 26.
#[test]
fn no_pad_stop_walk_matches_sub_82349ed8() {
    use unleashed_game::sonic::posture::{no_pad_stop_walk, StopWalkState};
    let Some(mut o) = Oracle::get() else { return };
    let (posture, ctx, ctx_vtable, body, body_vtable, flags, flag_bytes) =
        (o.alloc(0x40), o.alloc(0x800), o.alloc(0x80), o.alloc(0x500), o.alloc(0x10), o.alloc(0x10), o.alloc(0x40));
    o.write_u32(posture + 8, ctx);
    o.write_u32(ctx, ctx_vtable);
    o.write_u32(ctx_vtable + 56, 0x8246E640); // input not locked
    o.write_u32(body_vtable + 8, 0x823161E0);
    o.write_u32(body, body_vtable);
    o.write_u32(ctx + 0x10, body);
    o.write_u32(ctx + 0x4AC, flags);
    o.write_u32(flags + 4, flag_bytes);
    let table = Arc::new(Mutex::new([0f32; 256]));
    let t = table.clone();
    o.mock(0x8245DB60, move |r| {
        let v = t.lock().unwrap()[r.r(4) as usize];
        r.set_f(1, v as f64);
    });
    let mut rng = Rng(0x7127_0000_0000_0005);
    let mut changed = [0usize; 2];
    for i in 0..20_000 {
        let mut p = [0f32; 256];
        p[0x0C] = 0.0; // input threshold: keep the stick as written
        for (id, v) in [(0x8C, 0.3), (0x8E, 0.1), (0x8F, 15.0)] {
            p[id] = if i % 4 == 0 { rng.range(0.0, 20.0) } else { v };
        }
        *table.lock().unwrap() = p;
        let m = xenon_math::quat_to_matrix(rng.quat());
        for (r, row) in m.iter().enumerate() {
            o.write_vec4(body + 0x440 + 16 * r as u32, *row);
        }
        let mag = if i % 3 == 0 { 0.0 } else { rng.range(0.0, 1.0) };
        let input = [rng.unit() * mag, 0.0, rng.unit() * mag, 0.0];
        let speed = rng.range(0.0, 30.0);
        let s = StopWalkState { input, forward: xenon_math::forward(&m), grounded: i % 5 != 0,
            timer: rng.range(0.0, 0.3), speed: xenon_math::length3([speed, 0.0, 0.0]) };
        o.write_vec4(ctx + 0x120, input);
        o.write_u8(ctx + 0x3C0, s.grounded as u8);
        o.write_f32(ctx + 0x54C, s.timer);
        o.write_vec4(ctx + 0x220, [speed, 0.0, 0.0, 0.0]);
        o.write_u8(ctx + 0x565, 0);
        let before = (rng.next() & 1) as u8;
        o.write_u8(flag_bytes + 26, before);
        o.set_r(3, posture as u64);
        o.call(0x82349ED8);
        let guest = o.read(flag_bytes + 26, 1)[0];
        let port = match no_pad_stop_walk(&s, |id| p[id as usize]) { Some(v) => v as u8, None => before };
        if port != before { changed[port as usize] += 1; }
        assert_eq!(guest, port, "sample {i}: {s:?} before={before} guest={guest} port={port}");
    }
    eprintln!("cleared {}x set {}x", changed[0], changed[1]);
    assert!(changed[0] > 100 && changed[1] > 100);
}

/// sub_8234C5F8(out r3, posture r4, input r5, alt r7, cap f1, accel f2, decel f3).
#[test]
fn acceleration_matches_sub_8234c5f8() {
    use unleashed_game::sonic::posture::{acceleration, AccelInput, AccelState};
    let Some(mut o) = Oracle::get() else { return };
    let (posture, timing, ctx, ctx_vtable, body, body_vtable, flags, flag_bytes, input, out) = (
        o.alloc(0x40), o.alloc(0x40), o.alloc(0x800), o.alloc(0x100), o.alloc(0x500), o.alloc(0x10),
        o.alloc(0x10), o.alloc(0x40), o.alloc(16), o.alloc(16));
    o.write_u32(posture + 8, ctx);
    o.write_u32(posture + 12, timing);
    o.write_u32(ctx, ctx_vtable);
    o.write_u32(ctx_vtable + 112, 0x823167E0); // vf28 -> lfs f1, 0x54C(r3)
    o.write_u32(body_vtable + 8, 0x823161E0);
    o.write_u32(body, body_vtable);
    o.write_u32(ctx + 0x10, body);
    o.write_u32(ctx + 0x4AC, flags);
    o.write_u32(flags + 4, flag_bytes);
    let table = Arc::new(Mutex::new([0f32; 256]));
    let t = table.clone();
    o.mock(0x8245DB60, move |r| {
        let v = t.lock().unwrap()[r.r(4) as usize];
        r.set_f(1, v as f64);
    });
    let mut rng = Rng(0x7128_0000_0000_0006);
    let mut nonzero = 0;
    for i in 0..30_000 {
        let mut p = [0f32; 256];
        for (id, v) in [(0x8D, 10.0), (0x11, 15.0), (0x12, 30.0)] {
            p[id] = if i % 4 == 0 { rng.range(0.0, 80.0) } else { v };
        }
        *table.lock().unwrap() = p;
        let m = xenon_math::quat_to_matrix(rng.quat());
        for (r, row) in m.iter().enumerate() {
            o.write_vec4(body + 0x440 + 16 * r as u32, *row);
        }
        let speed = if i % 10 == 0 { 0.0 } else { rng.range(0.0, 60.0) };
        let mag = if i % 8 == 0 { 0.0 } else { rng.range(0.0, 1.0) };
        let s = AccelState {
            boosting: i % 6 == 0,
            forward: xenon_math::forward(&m),
            velocity: [rng.unit() * speed, rng.unit() * 2.0, rng.unit() * speed, 0.0],
            normal: if i % 3 == 0 { [0.0; 4] } else { [rng.unit(), rng.unit(), rng.unit(), 0.0] },
            grounded: i % 5 != 0,
            no_pad_stop_walk: i % 7 == 0,
            accel_force: rng.range(0.0, 60.0),
            fallback_div: rng.range(0.5, 80.0),
            fallback_scale: rng.range(0.0, 2.0),
            dt: if i % 2 == 0 { 1.0 / 30.0 } else { 1.0 / 60.0 },
        };
        let a = AccelInput {
            input: [rng.unit() * mag, 0.0, rng.unit() * mag, 0.0],
            cap: rng.range(0.0, 90.0),
            accel_factor: if i % 3 == 0 { 1.0 } else { rng.range(0.0, 2.0) },
            decel_factor: if i % 3 == 0 { 1.0 } else { rng.range(0.0, 2.0) },
            alt: i % 4 == 1,
        };
        o.write_u8(flag_bytes + 14, s.boosting as u8);
        o.write_u8(flag_bytes + 26, s.no_pad_stop_walk as u8);
        o.write_u8(ctx + 0x3C0, s.grounded as u8);
        o.write_vec4(ctx + 0x220, s.velocity);
        o.write_u8(ctx + 0x565, 0);
        o.write_vec4(ctx + 0x6B0, s.normal);
        o.write_f32(ctx + 0x54C, s.accel_force);
        o.write_f32(ctx + 0x4B8, s.fallback_div);
        o.write_f32(ctx + 0x540, s.fallback_scale);
        o.write_f32(timing + 36, s.dt);
        o.write_vec4(input, a.input);
        o.set_r(3, out as u64);
        o.set_r(4, posture as u64);
        o.set_r(5, input as u64);
        o.set_r(7, a.alt as u64);
        o.set_f(1, a.cap as f64);
        o.set_f(2, a.accel_factor as f64);
        o.set_f(3, a.decel_factor as f64);
        o.call(0x8234C5F8);
        let (guest, port) = (o.read_vec4(out), acceleration(&s, &a, |id| p[id as usize]));
        if port != [0.0; 4] { nonzero += 1; }
        assert!((0..4).all(|k| guest[k].to_bits() == port[k].to_bits() || (guest[k].is_nan() && port[k].is_nan())),
            "sample {i}: {s:?} {a:?} guest={guest:?} port={port:?}");
    }
    eprintln!("non-zero velocity changes: {nonzero}/30000");
}

/// sub_8231A0E8(ctx r3): recomputes ctx+0x220 / +0x230 and clears +0x565 / +0x566.
#[test]
fn split_velocity_matches_sub_8231a0e8() {
    use unleashed_game::sonic::posture::{split_velocity, VelocityFrame};
    let Some(mut o) = Oracle::get() else { return };
    let (ctx, body) = (o.alloc(0x800), o.alloc(0x200));
    o.write_u32(ctx + 0x10, body);
    let mut rng = Rng(0x7129_0000_0000_0007);
    for i in 0..30_000 {
        let to_local = xenon_math::quat_to_matrix(rng.quat());
        let to_world = if i % 2 == 0 { to_local } else { xenon_math::quat_to_matrix(rng.quat()) };
        let s = match i % 5 { 0 => 0.0, 1 => 0.004, _ => 40.0 };
        let f = VelocityFrame {
            velocity: [rng.unit() * s, rng.unit() * if i % 3 == 0 { 0.0 } else { s }, rng.unit() * s, rng.unit()],
            to_local,
            to_world,
            body_rotation: rng.quat(),
        };
        o.write_vec4(ctx + 0x210, f.velocity);
        for r in 0..4 {
            o.write_vec4(ctx + 0x250 + 16 * r as u32, f.to_local[r]);
            o.write_vec4(ctx + 0x290 + 16 * r as u32, f.to_world[r]);
        }
        o.write_vec4(body + 0x60, f.body_rotation);
        o.write_u8(ctx + 0x565, 1);
        o.write_u8(ctx + 0x566, 1);
        o.set_r(3, ctx as u64);
        o.call(0x8231A0E8);
        let (h, v) = split_velocity(&f);
        let (gh, gv) = (o.read_vec4(ctx + 0x220), o.read_vec4(ctx + 0x230));
        assert_eq!(bits(gh), bits(h), "sample {i}: horizontal guest={gh:?} port={h:?}");
        assert_eq!(bits(gv), bits(v), "sample {i}: vertical guest={gv:?} port={v:?}");
        assert_eq!(o.read(ctx + 0x565, 2), vec![0, 0], "sample {i}: flags not cleared");
    }
}

/// sub_82320820(ctx r3, v r4, scale f1): applies a velocity change.
#[test]
fn apply_velocity_change_matches_sub_82320820() {
    use unleashed_game::sonic::posture::{apply_velocity_change, ApplyState, ContextVelocity, VelocityFrame};
    let Some(mut o) = Oracle::get() else { return };
    let (ctx, body, flags, flag_bytes, guide_obj, v) =
        (o.alloc(0x800), o.alloc(0x200), o.alloc(0x10), o.alloc(0x40), o.alloc(0x40), o.alloc(16));
    o.write_u32(ctx + 0x10, body);
    o.write_u32(ctx + 0x4AC, flags);
    o.write_u32(flags + 4, flag_bytes);
    let guide = Arc::new(Mutex::new([0f32; 4]));
    let g = guide.clone();
    o.mock(0x822D20B0, move |r| {
        let out = r.r(4) as u32;
        r.write_vec4(out, *g.lock().unwrap());
        r.set_r(3, out as u64);
    });
    let min = Arc::new(Mutex::new(15f32));
    let m = min.clone();
    o.mock(0x8245DB60, move |r| {
        assert_eq!(r.r(4), 0xB6);
        let v = *m.lock().unwrap();
        r.set_f(1, v as f64);
    });
    let mut rng = Rng(0x712A_0000_0000_0008);
    let mut kept = 0;
    for i in 0..30_000 {
        let to_local = xenon_math::quat_to_matrix(rng.quat());
        let frame = VelocityFrame {
            velocity: [rng.unit() * 40.0, rng.unit() * 5.0, rng.unit() * 40.0, 0.0],
            to_local,
            to_world: if i % 2 == 0 { to_local } else { xenon_math::quat_to_matrix(rng.quat()) },
            body_rotation: rng.quat(),
        };
        let mut cv = ContextVelocity {
            frame,
            horizontal: [rng.unit() * 30.0, 0.0, rng.unit() * 30.0, 0.0],
            vertical: [0.0, rng.unit() * 3.0, 0.0, 0.0],
            split_stale: i % 3 == 0,
            velocity_dirty: i % 3 == 1,
        };
        let has_guide = i % 4 != 0;
        let s = ApplyState {
            flag_frozen: i % 17 == 0,
            keep_running: i % 2 == 0,
            guide: has_guide.then(|| xenon_math::normalize3([rng.unit(), 0.0, rng.unit(), 0.0])),
            guide_forward: i % 5 != 0,
            grounded: i % 3 != 2,
        };
        let dv = [rng.unit() * 30.0, rng.unit() * 2.0, rng.unit() * 30.0, 0.0];
        let scale = if i % 2 == 0 { 1.0 / 30.0 } else { rng.range(0.0, 1.0) };
        *min.lock().unwrap() = if i % 3 == 0 { 15.0 } else { rng.range(0.0, 50.0) };
        *guide.lock().unwrap() = s.guide.unwrap_or([0.0; 4]);
        o.write_vec4(ctx + 0x210, cv.frame.velocity);
        for r in 0..4 {
            o.write_vec4(ctx + 0x250 + 16 * r as u32, cv.frame.to_local[r]);
            o.write_vec4(ctx + 0x290 + 16 * r as u32, cv.frame.to_world[r]);
        }
        o.write_vec4(body + 0x60, cv.frame.body_rotation);
        o.write_vec4(ctx + 0x220, cv.horizontal);
        o.write_vec4(ctx + 0x230, cv.vertical);
        o.write_u8(ctx + 0x565, cv.split_stale as u8);
        o.write_u8(ctx + 0x566, cv.velocity_dirty as u8);
        o.write_u8(flag_bytes + 3, s.flag_frozen as u8);
        o.write_u8(flag_bytes + 31, s.keep_running as u8);
        o.write_u32(ctx + 0x650, if has_guide { guide_obj } else { 0 });
        o.write_u8(ctx + 0x658, s.guide_forward as u8);
        o.write_u8(ctx + 0x3C0, s.grounded as u8);
        o.write_vec4(v, dv);
        o.set_r(3, ctx as u64);
        o.set_r(4, v as u64);
        o.set_f(1, scale as f64);
        o.call(0x82320820);
        let p = *min.lock().unwrap();
        let h_before = cv.horizontal;
        apply_velocity_change(&mut cv, &s, dv, scale, |_| p);
        if cv.velocity_dirty && cv.horizontal != h_before { kept += 1; }
        let guest = (o.read_vec4(ctx + 0x210), o.read_vec4(ctx + 0x220), o.read_vec4(ctx + 0x230), o.read(ctx + 0x565, 2));
        assert_eq!(bits(guest.0), bits(cv.frame.velocity), "sample {i}: velocity guest={:?} port={:?}", guest.0, cv.frame.velocity);
        assert_eq!(guest.3, vec![cv.split_stale as u8, cv.velocity_dirty as u8], "sample {i}: flags");
        if !cv.split_stale {
            assert_eq!(bits(guest.1), bits(cv.horizontal), "sample {i}: horizontal guest={:?} port={:?}", guest.1, cv.horizontal);
            assert_eq!(bits(guest.2), bits(cv.vertical), "sample {i}: vertical guest={:?} port={:?}", guest.2, cv.vertical);
        }
    }
    eprintln!("KeepRunning clamped {kept}x");
    assert!(kept > 500);
}

/// sub_82317858, sub_82316DF8, sub_8231CF10 and sub_82317020.
#[test]
fn small_context_helpers_match() {
    use unleashed_game::sonic::posture::{clamped_acos, down_force_per_speed, ContextVelocity, VelocityFrame};
    let Some(mut o) = Oracle::get() else { return };
    let (ctx, out, v) = (o.alloc(0x800), o.alloc(16), o.alloc(16));
    let p = Arc::new(Mutex::new([0f32; 2]));
    let q = p.clone();
    o.mock(0x8245DB60, move |r| {
        let v = q.lock().unwrap()[(r.r(4) == 0x31) as usize];
        r.set_f(1, v as f64);
    });
    let mut rng = Rng(0x712B_0000_0000_0009);
    for i in 0..20_000 {
        let (g, e) = (rng.range(0.0, 60.0), if i % 50 == 0 { 0.0 } else { rng.range(-5.0, 5.0) });
        *p.lock().unwrap() = [g, e];
        o.set_r(3, ctx as u64);
        o.call(0x82317858);
        let port = down_force_per_speed(|id| if id == 0x31 { e } else { g });
        assert!(o.f(1) as f32 == port || (port.is_nan() && o.f(1).is_nan()), "sample {i}: down force");

        let x = match i % 4 { 0 => rng.range(-1.5, 1.5), 1 => rng.unit(), 2 => if i % 8 == 2 { f32::NAN } else { 1.0 }, _ => -1.0 };
        o.set_f(1, x as f64);
        o.call(0x82316DF8);
        let port = clamped_acos(x);
        assert!((o.f(1) as f32).to_bits() == port.to_bits() || (port.is_nan() && o.f(1).is_nan()), "sample {i}: acos({x})");

        let to_local = xenon_math::quat_to_matrix(rng.quat());
        let mut cv = ContextVelocity {
            frame: VelocityFrame { velocity: [rng.unit() * 9.0; 4], to_local, to_world: to_local, body_rotation: rng.quat() },
            horizontal: [rng.unit() * 9.0, 0.0, rng.unit() * 9.0, rng.unit()],
            vertical: [0.0, rng.unit(), 0.0, rng.unit()],
            split_stale: i % 3 == 0,
            velocity_dirty: i % 2 == 0,
        };
        o.write_vec4(ctx + 0x210, cv.frame.velocity);
        o.write_vec4(ctx + 0x220, cv.horizontal);
        o.write_vec4(ctx + 0x230, cv.vertical);
        o.write_u8(ctx + 0x565, cv.split_stale as u8);
        o.write_u8(ctx + 0x566, cv.velocity_dirty as u8);
        o.set_r(3, out as u64);
        o.set_r(4, ctx as u64);
        o.call(0x8231CF10);
        assert_eq!(bits(o.read_vec4(out)), bits(cv.velocity()), "sample {i}: velocity get");
        assert_eq!(o.read(ctx + 0x565, 2), vec![cv.split_stale as u8, cv.velocity_dirty as u8]);
        let nv = [rng.unit(), rng.unit(), rng.unit(), rng.unit()];
        o.write_vec4(v, nv);
        o.set_r(3, ctx as u64);
        o.set_r(4, v as u64);
        o.call(0x82317020);
        cv.set_velocity(nv);
        assert_eq!(bits(o.read_vec4(ctx + 0x210)), bits(cv.frame.velocity), "sample {i}: velocity set");
        assert_eq!(o.read(ctx + 0x565, 2), vec![cv.split_stale as u8, cv.velocity_dirty as u8]);
    }
}

/// sub_82317E08, sub_82317D58 and sub_82317A28: slope limits.
#[test]
fn slope_checks_match() {
    use unleashed_game::sonic::posture::{land_enable, land_enable_normal, move_enable_ground_angle};
    let Some(mut o) = Oracle::get() else { return };
    let (ctx, flags, flag_bytes, a, b) = (o.alloc(0x800), o.alloc(0x10), o.alloc(0x40), o.alloc(16), o.alloc(16));
    o.write_u32(ctx + 0x4AC, flags);
    o.write_u32(flags + 4, flag_bytes);
    let table = Arc::new(Mutex::new([0f32; 256]));
    let t = table.clone();
    o.mock(0x8245DB60, move |r| {
        let v = t.lock().unwrap()[r.r(4) as usize];
        r.set_f(1, v as f64);
    });
    let mut rng = Rng(0x712C_0000_0000_000A);
    let unit = |rng: &mut Rng| xenon_math::normalize3([rng.unit(), rng.unit(), rng.unit(), 0.0]);
    for i in 0..20_000 {
        let mut p = [0f32; 256];
        p[0x73] = if i % 3 == 0 { rng.range(0.0, 90.0) } else { 45.0 };
        p[0x78] = if i % 3 == 0 { rng.range(0.0, 90.0) } else { 32.0 };
        p[0x79] = if i % 3 == 0 { rng.range(0.0, 90.0) } else { 36.0 };
        *table.lock().unwrap() = p;
        let (va, vb) = (unit(&mut rng), if i % 4 == 0 { unit(&mut rng) } else { xenon_math::normalize3([rng.unit() * 0.5, 1.0, rng.unit() * 0.5, 0.0]) });
        let (flag9, grounded, inner) = (i % 11 == 0, i % 2 == 0, i % 3 == 1);
        o.write_u8(flag_bytes + 9, flag9 as u8);
        o.write_u8(ctx + 0x3C0, grounded as u8);
        o.write_vec4(a, va);
        o.write_vec4(b, vb);
        o.set_r(3, ctx as u64);
        o.set_r(7, a as u64);
        o.set_r(8, b as u64);
        o.set_r(9, inner as u64);
        o.call(0x82317E08);
        assert_eq!(o.r(3) & 1 != 0, move_enable_ground_angle(va, vb, inner, |id| p[id as usize]), "sample {i}: ground angle");
        o.set_r(3, ctx as u64);
        o.set_r(4, a as u64);
        o.set_r(5, b as u64);
        o.call(0x82317D58);
        assert_eq!(o.r(3) & 1 != 0, land_enable(flag9, grounded, va, vb, |id| p[id as usize]), "sample {i}: land enable");
        let (up, dir) = (unit(&mut rng), [rng.unit(), rng.unit(), rng.unit(), 0.0]);
        o.write_vec4(ctx + 0x3D0, up);
        o.write_vec4(ctx + 0x6A0, dir);
        o.set_r(3, ctx as u64);
        o.set_r(4, b as u64);
        o.call(0x82317A28);
        assert_eq!(o.r(3) & 1 != 0, land_enable_normal(flag9, up, dir, vb, |id| p[id as usize]), "sample {i}: land normal");
    }
}
