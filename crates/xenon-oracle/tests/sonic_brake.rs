//! sub_82349FF0 (Sonic's brake) against the port.

use std::sync::{Arc, Mutex};
use unleashed_game::sonic::context::{flag, offset::posture as po, offset::speed_context as co};
use unleashed_game::sonic::posture::{brake, ApplyState, BrakeState, ContextVelocity, VelocityFrame};
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
fn brake_matches_sub_82349ff0() {
    let Some(mut o) = Oracle::get() else { return };
    let (posture, timing, ctx, body, body_vtable) = (o.alloc(0x80), o.alloc(0x40), o.alloc(0x1000), o.alloc(0x500), o.alloc(0x10));
    let (flags, flag_bytes, control, input, dir) = (o.alloc(0x10), o.alloc(0x40), o.alloc(0x40), o.alloc(16), o.alloc(16));
    o.write_u32(posture + po::CONTEXT, ctx);
    o.write_u32(posture + po::TIMING, timing);
    o.write_u32(body_vtable + 8, 0x823161E0);
    o.write_u32(body, body_vtable);
    o.write_u32(ctx + co::BODY, body);
    o.write_u32(ctx + co::FLAGS, flags);
    o.write_u32(flags + 4, flag_bytes);
    let params = Arc::new(Mutex::new([0f32; 256]));
    let p = params.clone();
    o.mock(0x8245DB60, move |r| {
        let v = p.lock().unwrap()[r.r(4) as usize];
        r.set_f(1, v as f64);
    });
    o.mock(0x82316EB0, move |r| r.set_r(3, control as u64));
    o.mock(0x822D20B0, move |r| r.set_r(3, r.r(4)));

    let mut rng = Rng(0xB4A3_0000_0000_0001);
    let (mut ended, mut kept) = (0, 0);
    for i in 0..20_000 {
        let mut table = [0f32; 256];
        table[0x23] = if i % 3 == 0 { rng.range(0.0, 180.0) } else { 100.0 };
        table[0xB6] = 15.0;
        *params.lock().unwrap() = table;
        let to_local = xenon_math::quat_to_matrix(rng.quat());
        let body_rotation = rng.quat();
        let body_m = xenon_math::quat_to_matrix(body_rotation);
        let control_m = xenon_math::quat_to_matrix(rng.quat());
        let speed = match i % 5 { 0 => rng.range(0.0, 12.0), 1 => 0.0, _ => rng.range(0.0, 50.0) };
        let mut cv = ContextVelocity {
            frame: VelocityFrame { velocity: [rng.unit() * speed, rng.unit(), rng.unit() * speed, 0.0], to_local, to_world: to_local, body_rotation },
            horizontal: [rng.unit() * speed, 0.0, rng.unit() * speed, 0.0],
            vertical: [0.0, rng.unit(), 0.0, 0.0],
            split_stale: i % 2 == 0,
            velocity_dirty: i % 4 == 1,
        };
        let mag = if i % 4 == 0 { 0.0 } else { rng.range(0.0, 1.0) };
        let s = BrakeState {
            input: [rng.unit() * mag, 0.0, rng.unit() * mag, 0.0],
            brake_dir: xenon_math::normalize3([rng.unit(), 0.0, rng.unit(), 0.0]),
            force: if i % 2 == 0 { 100.0 } else { rng.range(0.0, 200.0) },
            min_time: if i % 3 == 0 { rng.range(0.0, 0.5) } else { 0.0 },
            grounded: i % 3 != 2,
            forward: xenon_math::forward(&body_m),
            control: control_m,
            dt: if i % 2 == 0 { 1.0 / 30.0 } else { 1.0 / 60.0 },
        };
        let keep_running = i % 7 == 0;
        let apply = ApplyState { flag_frozen: false, keep_running, guide: None, guide_forward: true, grounded: s.grounded };
        let input_scale = if i % 2 == 0 { 1.0 } else { rng.range(0.5, 1.5) };
        let mut timer = if i % 3 == 0 { rng.range(0.0, 0.6) } else { 0.0 };

        o.write_vec4(ctx + co::VELOCITY, cv.frame.velocity);
        o.write_vec4(ctx + co::HORIZONTAL_VELOCITY, cv.horizontal);
        o.write_vec4(ctx + co::VERTICAL_VELOCITY, cv.vertical);
        for r in 0..4 {
            o.write_vec4(ctx + co::TO_LOCAL + 16 * r as u32, to_local[r]);
            o.write_vec4(ctx + co::TO_WORLD + 16 * r as u32, to_local[r]);
            o.write_vec4(body + 0x440 + 16 * r as u32, body_m[r]);
            o.write_vec4(control + 16 * r as u32, control_m[r]);
        }
        o.write_vec4(body + 0x60, body_rotation);
        o.write_u8(ctx + co::SPLIT_STALE, cv.split_stale as u8);
        o.write_u8(ctx + co::VELOCITY_DIRTY, cv.velocity_dirty as u8);
        o.write_u8(ctx + co::GROUNDED, s.grounded as u8);
        o.write(flag_bytes, &[0; 64]);
        o.write_u8(flag_bytes + flag::KEEP_RUNNING, keep_running as u8);
        o.write_u32(ctx + co::KEEP_RUNNING_GUIDE, 0);
        o.write_f32(ctx + co::INPUT_SCALE, input_scale);
        o.write_vec4(ctx + co::CONTACT_NORMAL, [0.0; 4]);
        o.write_f32(posture + 0x64, timer);
        o.write_f32(timing + 36, s.dt);
        o.write_vec4(input, s.input);
        o.write_vec4(dir, s.brake_dir);
        o.set_r(3, posture as u64);
        o.set_r(6, input as u64);
        o.set_r(8, dir as u64);
        o.set_f(1, s.force as f64);
        o.set_f(2, s.min_time as f64);
        o.call(0x82349FF0);
        let guest_on = o.r(3) & 0xFF != 0;

        let on = brake(&mut cv, &apply, input_scale, &mut timer, &s, |id| table[id as usize]);
        if on { kept += 1 } else { ended += 1 }
        assert_eq!(guest_on, on, "sample {i}: brake continues guest={guest_on} port={on}");
        assert_eq!(o.read_f32(posture + 0x64).to_bits(), timer.to_bits(), "sample {i}: timer");
        assert_eq!(bits(o.read_vec4(ctx + co::VELOCITY)), bits(cv.frame.velocity), "sample {i}: velocity");
        assert_eq!(o.read(ctx + co::SPLIT_STALE, 2), vec![cv.split_stale as u8, cv.velocity_dirty as u8], "sample {i}: flags");
        if !cv.split_stale {
            assert_eq!(bits(o.read_vec4(ctx + co::HORIZONTAL_VELOCITY)), bits(cv.horizontal), "sample {i}: horizontal");
        }
    }
    eprintln!("brake ended {ended}x, continued {kept}x");
    assert!(ended > 2_000 && kept > 2_000);
}
