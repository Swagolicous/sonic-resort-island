//! sub_82347448 (Posture3DStandard ground step) against the port, on a guest-memory
//! Sonic laid out from sheets/context_fields.csv.

use std::sync::{Arc, Mutex};
use unleashed_game::sonic::context::{flag, offset::posture as po, offset::speed_context as co};
use unleashed_game::sonic::posture::{ground_step, ContextVelocity, PostureHost, SpeedContext, VelocityFrame};
use unleashed_game::sonic::xenon_math::{self, Mat4};
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

/// Host side of the port: parameters and the body, mirroring the guest fixture.
struct Host {
    params: [f32; 512],
    body: Mat4,
    body_rotation: [f32; 4],
    rotations: Vec<[f32; 4]>,
    guide: [f32; 4],
}

impl PostureHost for Host {
    fn param(&self, id: u16) -> f32 {
        self.params[id as usize]
    }
    fn body_matrix(&self) -> Mat4 {
        self.body
    }
    fn body_rotation(&self) -> [f32; 4] {
        self.body_rotation
    }
    fn rotate_body(&mut self, delta: [f32; 4]) {
        self.rotations.push(delta);
    }
    fn keep_running_direction(&mut self) -> [f32; 4] {
        self.guide
    }
}

/// Shipped parameter values the ground step reads (sheets/param_values.csv, ForwardView).
const SHIPPED: &[(usize, f32)] = &[
    (0x0C, 0.02), (0x11, 15.0), (0x12, 30.0), (0x88, 0.5), (0x89, 30.0), (0x8A, 1.0), (0x8B, 60.0),
    (0x8C, 0.3), (0x8D, 10.0), (0x8E, 0.1), (0x8F, 15.0), (0x95, 30.0), (0x96, 1.0), (0x97, 720.0),
    (0x98, 30.0), (0x9D, 100.0), (0x9E, 20.0), (0x9F, 80.0), (0xA0, 40.0), (0xA1, 1000.0), (0xA2, 50.0),
    (0xA3, 1.0), (0xB6, 15.0),
];

#[test]
fn ground_step_matches_sub_82347448() {
    let Some(mut o) = Oracle::get() else { return };
    let posture = o.alloc(0x40);
    let timing = o.alloc(0x40);
    let ctx = o.alloc(0x1000);
    let ctx_vtable = o.alloc(0x100);
    let body = o.alloc(0x500);
    let body_vtable = o.alloc(0x10);
    let (flags, flag_bytes, flags2, flag2_bytes) = (o.alloc(0x10), o.alloc(0x40), o.alloc(0x10), o.alloc(0x40));
    let guide_obj = o.alloc(0x40);
    o.write_u32(posture + po::CONTEXT, ctx);
    o.write_u32(posture + po::TIMING, timing);
    o.write_u32(ctx, ctx_vtable);
    o.write_u32(ctx_vtable + 112, 0x82F20278); // vf28 accel force: lfs f1, 0x18(r3)
    o.write_u32(body_vtable + 8, 0x823161E0); // vf2 matrix: body+0x440
    o.write_u32(body, body_vtable);
    o.write_u32(ctx + co::BODY, body);
    o.write_u32(ctx + co::FLAGS, flags);
    o.write_u32(flags + 4, flag_bytes);
    o.write_u32(ctx + co::FLAGS2, flags2);
    o.write_u32(flags2 + 4, flag2_bytes);

    let params = Arc::new(Mutex::new([0f32; 512]));
    let p = params.clone();
    o.mock(0x8245DB60, move |r| {
        let v = p.lock().unwrap()[r.r(4) as usize];
        r.set_f(1, v as f64);
    });
    let rotations = Arc::new(Mutex::new(Vec::new()));
    let rs = rotations.clone();
    o.mock(0x82308300, move |r| rs.lock().unwrap().push(r.read_vec4(r.r(4) as u32)));
    let guide = Arc::new(Mutex::new([0f32; 4]));
    let g = guide.clone();
    o.mock(0x822D20B0, move |r| {
        let out = r.r(4) as u32;
        r.write_vec4(out, *g.lock().unwrap());
        r.set_r(3, out as u64);
    });

    let mut rng = Rng(0x5047_0000_0000_0001);
    let mut turned = 0;
    for i in 0..10_000 {
        let mut table = [0f32; 512];
        for &(id, v) in SHIPPED {
            table[id] = if i % 5 == 0 { v * rng.range(0.5, 1.5) } else { v };
        }
        *params.lock().unwrap() = table;
        let body_rotation = rng.quat();
        let body_m = xenon_math::quat_to_matrix(body_rotation);
        let to_local = xenon_math::quat_to_matrix(rng.quat());
        let speed = match i % 6 {
            0 => 0.0,
            1 => 0.005,
            _ => rng.range(0.0, 70.0),
        };
        let mag = if i % 7 == 0 { 0.0 } else { rng.range(0.0, 1.0) };
        let mut flags_v = [false; 64];
        for (f, every) in [
            (flag::FROZEN, 23),
            (flag::BOOSTING, 5),
            (flag::ROTATION_FORCE_BLEND, 9),
            (flag::NO_PAD_STOP_WALK, 4),
            (flag::KEEP_RUNNING, 6),
            (flag::KEEP_FACING, 11),
        ] {
            flags_v[f as usize] = (rng.next() % every) == 0;
        }
        let mut cx = SpeedContext {
            velocity: ContextVelocity {
                frame: VelocityFrame {
                    velocity: [rng.unit() * speed, rng.unit() * 3.0, rng.unit() * speed, 0.0],
                    to_local,
                    to_world: if i % 2 == 0 { to_local } else { xenon_math::quat_to_matrix(rng.quat()) },
                    body_rotation,
                },
                horizontal: [rng.unit() * speed, 0.0, rng.unit() * speed, 0.0],
                vertical: [0.0, rng.unit(), 0.0, 0.0],
                split_stale: i % 2 == 0,
                velocity_dirty: i % 4 == 1,
            },
            stick: [rng.unit() * mag, 0.0, rng.unit() * mag, 0.0],
            up: xenon_math::normalize3([rng.unit() * 0.3, 1.0, rng.unit() * 0.3, 0.0]),
            grounded: i % 5 != 0,
            rotation: rng.quat(),
            target_rotation: [f32::NAN; 4],
            flags: flags_v,
            flags2_7: i % 13 == 0,
            fallback_div: rng.range(1.0, 60.0),
            rotation_blend: rng.range(0.0, 1.0),
            fallback_scale: rng.range(0.0, 1.0),
            stick_timer: rng.range(0.0, 0.3),
            input_scale: if i % 3 == 0 { 1.0 } else { rng.range(0.0, 2.0) },
            contact_normal: if i % 4 == 0 { [0.0; 4] } else { xenon_math::normalize3([rng.unit(), 1.0, rng.unit(), 0.0]) },
            blocked_count: if i % 9 == 0 { (rng.next() % 8) as i32 } else { 0 },
            lean_input: rng.range(-5.0, 5.0),
            has_guide: i % 3 != 0,
            guide_forward: i % 2 == 0,
            input_locked: i % 19 == 0,
            accel_force: rng.range(10.0, 60.0),
            dt: if i % 2 == 0 { 1.0 / 30.0 } else { 1.0 / 60.0 },
        };
        let mode = if i % 3 == 0 { 1 } else { 0 };
        let cap = rng.range(0.0, 90.0);
        let guide_v = xenon_math::normalize3([rng.unit(), 0.0, rng.unit(), 0.0]);
        *guide.lock().unwrap() = guide_v;
        rotations.lock().unwrap().clear();

        // Guest state from the port's.
        let v = &cx.velocity;
        o.write_vec4(ctx + co::VELOCITY, v.frame.velocity);
        o.write_vec4(ctx + co::HORIZONTAL_VELOCITY, v.horizontal);
        o.write_vec4(ctx + co::VERTICAL_VELOCITY, v.vertical);
        for r in 0..4 {
            o.write_vec4(ctx + co::TO_LOCAL + 16 * r as u32, v.frame.to_local[r]);
            o.write_vec4(ctx + co::TO_WORLD + 16 * r as u32, v.frame.to_world[r]);
            o.write_vec4(body + 0x440 + 16 * r as u32, body_m[r]);
        }
        o.write_vec4(body + 0x60, body_rotation);
        o.write_u8(ctx + co::SPLIT_STALE, v.split_stale as u8);
        o.write_u8(ctx + co::VELOCITY_DIRTY, v.velocity_dirty as u8);
        o.write_vec4(ctx + co::STICK, cx.stick);
        o.write_vec4(ctx + co::UP, cx.up);
        o.write_u8(ctx + co::GROUNDED, cx.grounded as u8);
        o.write_vec4(ctx + co::ROTATION, cx.rotation);
        o.write_vec4(ctx + co::TARGET_ROTATION, cx.target_rotation);
        for (k, &f) in cx.flags.iter().enumerate() {
            o.write_u8(flag_bytes + k as u32, f as u8);
        }
        o.write_u8(flag2_bytes + 7, cx.flags2_7 as u8);
        o.write_f32(ctx + co::FALLBACK_DIV, cx.fallback_div);
        o.write_f32(ctx + co::ROTATION_BLEND, cx.rotation_blend);
        o.write_f32(ctx + co::FALLBACK_SCALE, cx.fallback_scale);
        o.write_f32(ctx + co::STICK_TIMER, cx.stick_timer);
        o.write_f32(ctx + co::INPUT_SCALE, cx.input_scale);
        o.write_vec4(ctx + co::CONTACT_NORMAL, cx.contact_normal);
        o.write_u32(ctx + co::BLOCKED_COUNT, cx.blocked_count as u32);
        o.write_f32(ctx + co::LEAN_INPUT, cx.lean_input);
        o.write_u32(ctx + co::KEEP_RUNNING_GUIDE, if cx.has_guide { guide_obj } else { 0 });
        o.write_u8(ctx + co::GUIDE_FORWARD, cx.guide_forward as u8);
        o.write_u32(ctx_vtable + 56, if cx.input_locked { 0x82DE0888 } else { 0x8246E640 });
        o.write_f32(ctx + 0x18, cx.accel_force);
        o.write_f32(timing + 36, cx.dt);

        o.set_r(3, posture as u64);
        o.set_r(4, mode as u64);
        o.set_f(1, cap as f64);
        o.call(0x82347448);

        let mut host = Host { params: table, body: body_m, body_rotation, rotations: Vec::new(), guide: guide_v };
        ground_step(&mut cx, &mut host, mode, cap);
        if !host.rotations.is_empty() {
            turned += 1;
        }

        let guest_rot = rotations.lock().unwrap().clone();
        assert_eq!(
            guest_rot.iter().map(|q| bits(*q)).collect::<Vec<_>>(),
            host.rotations.iter().map(|q| bits(*q)).collect::<Vec<_>>(),
            "sample {i}: body rotations"
        );
        let v = &cx.velocity;
        assert_eq!(bits(o.read_vec4(ctx + co::VELOCITY)), bits(v.frame.velocity), "sample {i}: velocity");
        assert_eq!(o.read(ctx + co::SPLIT_STALE, 2), vec![v.split_stale as u8, v.velocity_dirty as u8], "sample {i}: velocity flags");
        if !v.split_stale {
            assert_eq!(bits(o.read_vec4(ctx + co::HORIZONTAL_VELOCITY)), bits(v.horizontal), "sample {i}: horizontal");
            assert_eq!(bits(o.read_vec4(ctx + co::VERTICAL_VELOCITY)), bits(v.vertical), "sample {i}: vertical");
        }
        assert_eq!(bits(o.read_vec4(ctx + co::TARGET_ROTATION)), bits(cx.target_rotation), "sample {i}: target rotation");
        assert_eq!(o.read_f32(ctx + co::LEAN_INPUT).to_bits(), cx.lean_input.to_bits(), "sample {i}: lean input");
        let gflags: Vec<bool> = o.read(flag_bytes, 64).iter().map(|&b| b != 0).collect();
        assert_eq!(gflags, cx.flags.to_vec(), "sample {i}: flags");
    }
    eprintln!("facing turned in {turned}/10000 frames");
    assert!(turned > 3_000);
    assert_eq!(o.import_hits(), 0);
}
