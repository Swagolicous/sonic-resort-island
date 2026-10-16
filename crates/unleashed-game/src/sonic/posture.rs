//! CPlayerSpeedPosture3DStandard and its helpers, ported from the recompiled
//! executable and checked against it by the xenon-oracle tests. Each function names
//! the guest function it reproduces; sheets/port_functions.csv tracks coverage.

use super::constants::DEG_TO_RAD;
use super::xenon_math::{dot3, normalize3};

/// `fmadds` as XenonRecomp translates it: a * b + c evaluated in double, rounded once.
#[inline]
pub(crate) fn fmadds(a: f32, b: f32, c: f32) -> f32 {
    (a as f64 * b as f64 + c as f64) as f32
}

/// The four Rotation parameters sub_82318850 reads (ids 0x88-0x8B).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RotationForceRate {
    /// 0x088 RotationForceRateMin
    pub rate_min: f32,
    /// 0x089 RotationForceMinAngle (degrees)
    pub min_angle: f32,
    /// 0x08A RotationForceRateMax
    pub rate_max: f32,
    /// 0x08B RotationForceMaxAngle (degrees)
    pub max_angle: f32,
}

/// sub_82318850: how much of the rotation force applies, from the angle between
/// `dir` (normalized here) and `reference`: RateMin at MinAngle, RateMax at MaxAngle,
/// linear in the cosine between them and clamped.
pub fn rotation_force_rate(p: &RotationForceRate, dir: [f32; 4], reference: [f32; 4]) -> f32 {
    use super::crt_math::cos;
    let cos_min = cos((p.min_angle * DEG_TO_RAD) as f64) as f32;
    let cos_max = cos((p.max_angle * DEG_TO_RAD) as f64) as f32;
    let n = normalize3(dir);
    let d = dot3([reference[0], reference[1], reference[2]], [n[0], n[1], n[2]]);
    let t = (d - cos_min) / (cos_max - cos_min);
    // ble/bge on unordered compares let NaN through unclamped.
    let t = if t > 1.0 {
        1.0
    } else if t < 0.0 {
        0.0
    } else {
        t
    };
    fmadds(p.rate_max - p.rate_min, t, p.rate_min)
}

use super::constants::{TURN_ANGLE_EPSILON, TURN_INPUT_EPSILON, TURN_MIN_SPEED};
use super::crt_math::atan2;
use super::xenon_math::{axis_angle_quat, length3, quat_to_matrix, transform3, wrap_near, Mat4};

/// sub_8231A3A8 with `apply` set: store `rotation` as the new target (ctx+0x460) and
/// return the delta the body is turned by (sub_82308300), current (ctx+0x440) times
/// target, summed in the order the game's dot products use.
pub fn rotation_delta(current: [f32; 4], target: [f32; 4]) -> [f32; 4] {
    let [xo, yo, zo, wo] = current;
    let [xn, yn, zn, wn] = target;
    [
        (wn * xo + zn * yo) + (-yn * zo + xn * wo),
        (wn * yo + -zn * xo) + (yn * wo + xn * zo),
        (wn * zo + zn * wo) + (yn * xo + -xn * yo),
        (wn * wo + -zn * zo) + (-yn * yo + -xn * xo),
    ]
}

fn transpose(m: &Mat4) -> Mat4 {
    std::array::from_fn(|r| std::array::from_fn(|c| m[c][r]))
}

/// Heading of `v` in the frame's ground plane: atan2(x, z), rounded to f32.
fn heading(v: [f32; 4]) -> f32 {
    atan2(v[0] as f64, v[2] as f64) as f32
}

/// What sub_82349538 reads from the speed context and posture.
#[derive(Clone, Copy, Debug)]
pub struct TurnState {
    /// ctx+0x440: current rotation quaternion.
    pub rotation: [f32; 4],
    /// Forward axis of the body frame (sub_8230A4E0).
    pub forward: [f32; 4],
    /// ctx+0x220: horizontal velocity (sub_8231D0D0; must be up to date, ctx+0x565 clear).
    pub velocity: [f32; 4],
    /// ctx+0x2F0: up vector (sub_82317040).
    pub up: [f32; 4],
    /// ctx+0x554 (sub_823161D8): scale on the turning input.
    pub input_scale: f32,
    /// ctx flag 3 (sub_823171C8): no turning at all.
    pub flag_no_turn: bool,
    /// ctx flag 37: the facing is not turned.
    pub flag_keep_facing: bool,
    /// posture+0xC -> +0x24 (sub_82E627D0): frame time.
    pub dt: f32,
    pub rate: RotationForceRate,
}

/// sub_82349538's arguments: the world input direction (r4), its force (f1), the
/// facing's max turn rate (f2), the velocity's turn force (f3), whether to turn the
/// velocity too (r7).
#[derive(Clone, Copy, Debug)]
pub struct TurnInput {
    pub input: [f32; 4],
    pub force: f32,
    pub max_turn_speed: f32,
    pub velocity_force: f32,
    pub turn_velocity: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TurnResult {
    /// New target rotation (ctx+0x460) and the delta the body turned by.
    pub rotation: Option<([f32; 4], [f32; 4])>,
    /// New horizontal velocity (ctx+0x220, sub_8231CF50).
    pub velocity: Option<[f32; 4]>,
}

/// sub_82349538: Sonic's turning. The facing turns toward the input heading by at
/// most `max_turn_speed * dt` (about local Y); then, if asked, the velocity turns
/// about `up` by the smaller of the facing's and its own heading change.
pub fn turn(s: &TurnState, i: &TurnInput) -> TurnResult {
    let mut out = TurnResult::default();
    let input = i.input;
    if dot3_v(input, input) < TURN_INPUT_EPSILON || s.flag_no_turn {
        return out;
    }
    let world_to_local = transpose(&quat_to_matrix(s.rotation));
    let speed = length3([s.velocity[0], s.velocity[1], s.velocity[2]]);
    let local_forward = transform3(&world_to_local, s.forward);
    let facing = heading(local_forward);
    let rate = rotation_force_rate(&s.rate, input, s.forward);
    let n = normalize3(input);
    let push = n.map(|c| c * i.force).map(|c| c * rate).map(|c| c * s.input_scale);
    let local_push = transform3(&world_to_local, push).map(|c| c * s.dt);
    let reach = if speed < TURN_MIN_SPEED { TURN_MIN_SPEED } else { speed };
    let ahead: [f32; 4] = std::array::from_fn(|k| super::xenon_math::madd(local_forward[k], reach, local_push[k]));
    let mut target = wrap_near(heading(ahead), facing);
    if !s.flag_keep_facing {
        let limit = s.dt * i.max_turn_speed;
        target = if target - facing > 0.0 {
            let d = target - facing;
            (if d < limit { d } else { limit }) + facing
        } else {
            let d = facing - target;
            facing - if d < limit { d } else { limit }
        };
        let q = axis_angle_quat([0.0, 1.0, 0.0, 0.0], target);
        out.rotation = Some((q, rotation_delta(s.rotation, q)));
    }
    if !i.turn_velocity {
        return out;
    }
    let v = s.velocity;
    if !(dot3_v(v, v) > TURN_INPUT_EPSILON) {
        return out;
    }
    let local_v = transform3(&world_to_local, v);
    let pull = n.map(|c| c * i.velocity_force).map(|c| c * s.input_scale);
    let v_heading = heading(local_v);
    let local_pull = transform3(&world_to_local, pull);
    let steered: [f32; 4] = std::array::from_fn(|k| super::xenon_math::madd(local_pull[k], s.dt, local_v[k]));
    let v_target = wrap_near(heading(steered), v_heading);
    let delta = v_target - v_heading;
    let sign = if delta.abs() < TURN_ANGLE_EPSILON {
        0.0
    } else if delta > 0.0 {
        1.0
    } else {
        -1.0
    };
    let (a, b) = ((target - v_heading).abs(), delta.abs());
    let angle = sign * if a < b { a } else { b };
    let turn = quat_to_matrix(axis_angle_quat(s.up, angle));
    let len = length3([v[0], v[1], v[2]]);
    out.velocity = Some(transform3(&turn, normalize3(v)).map(|c| c * len));
    out
}

fn dot3_v(a: [f32; 4], b: [f32; 4]) -> f32 {
    dot3([a[0], a[1], a[2]], [b[0], b[1], b[2]])
}

/// The context state sub_823472A0 reads besides parameters.
#[derive(Clone, Copy, Debug, Default)]
pub struct RotationForceState {
    /// ctx flag 14 (boosting).
    pub flag_14: bool,
    /// ctx flag 22: blend the force toward a fixed target by `blend`.
    pub flag_22: bool,
    /// ctx+0x510 (sub_823165B0).
    pub blend: f32,
    /// Second flag set (ctx+0x4B0) bit 7: enemy blocking ignored.
    pub flag2_7: bool,
    /// ctx+0xCC8: enemies blocking Sonic.
    pub blocked_count: i32,
}

/// sub_8231B7D8: Sonic is held back by enemies.
pub fn enemy_blocked(s: &RotationForceState) -> bool {
    !s.flag2_7 && s.blocked_count != 0
}

/// sub_823472A0: turning force at `speed`. RotationForce0 below RotationForceVelocity0,
/// RotationForce1 from RotationForceVelocity1, linear between; RotationForceInBoost
/// while boosting; reduced per blocking enemy; blended toward a fixed value under
/// flag 22; times parameter 0xA3. `param` is the context's parameter lookup.
pub fn rotation_force(s: &RotationForceState, speed: f32, param: impl Fn(u16) -> f32) -> f32 {
    use super::constants::{ROTATION_FORCE_BLEND_TARGET, ROTATION_FORCE_BLEND_TARGET_BOOST};
    let mut force;
    if enemy_blocked(s) {
        // sub_82317EC0: EnemyBlockedRotationForce1 - count * Dec, not below 0.
        let (dec, base) = (param(0x96), param(0x95));
        let n = s.blocked_count as i64 as f64 as f32;
        let r = (-(n as f64 * dec as f64 - base as f64)) as f32;
        force = if r >= 0.0 { r } else { 0.0 };
    } else {
        force = if s.flag_14 {
            param(0xA2)
        } else if !(speed > param(0x9E)) {
            param(0x9D)
        } else if !(speed < param(0xA0)) {
            param(0x9F)
        } else {
            let (f1, f0, v1) = (param(0x9F), param(0x9D), param(0xA0));
            let rise = f1 - f0;
            let run = v1 - param(0x9E);
            let slope = rise / run;
            let v0 = param(0x9E);
            let f0 = param(0x9D);
            let offset = v0 * slope;
            let t = (slope as f64 * speed as f64 - offset as f64) as f32;
            f0 + t
        };
        if s.flag_22 {
            let target = if s.flag_14 { ROTATION_FORCE_BLEND_TARGET_BOOST } else { ROTATION_FORCE_BLEND_TARGET };
            force = fmadds(target - force, s.blend, force);
        }
    }
    param(0xA3) * force
}

/// sub_8231B990: facing turn-rate limit in rad/s, RotationSpeedMax (degrees/s), or
/// while enemies block Sonic EnemyBlockedRotationSpeedMax1 - count * ...Dec (>= 0).
pub fn rotation_speed_max(s: &RotationForceState, param: impl Fn(u16) -> f32) -> f32 {
    use super::constants::DEG_TO_RAD;
    if enemy_blocked(s) {
        let (dec, base) = (param(0x98), param(0x97));
        let n = s.blocked_count as i64 as f64 as f32;
        let r = (-(n as f64 * dec as f64 - base as f64)) as f32;
        (if r >= 0.0 { r } else { 0.0 }) * DEG_TO_RAD
    } else {
        param(0xA1) * DEG_TO_RAD
    }
}

/// sub_82346FE0: sideways drag force for the ground step.
pub fn side_drag_force(boosting: bool, alt: bool) -> f32 {
    use super::constants::{SIDE_DRAG_FORCE, SIDE_DRAG_FORCE_ALT, SIDE_DRAG_FORCE_BOOST};
    if boosting {
        SIDE_DRAG_FORCE_BOOST
    } else if alt {
        SIDE_DRAG_FORCE_ALT
    } else {
        SIDE_DRAG_FORCE
    }
}

/// sub_823083F0: the world-space stick direction (ctx+0x120), zero when input is
/// locked (context vf14) or shorter than InputThreshold (parameter 0x00C).
pub fn input_direction(stick: [f32; 4], input_locked: bool, threshold: f32) -> [f32; 4] {
    if input_locked || dot3_v(stick, stick) < threshold * threshold {
        [0.0; 4]
    } else {
        stick
    }
}

/// sub_8231AA60: removes the part of `v` pushing into the contact normal (ctx+0x6B0,
/// ignored while shorter than 1e-2).
pub fn remove_into_normal(normal: [f32; 4], v: [f32; 4]) -> [f32; 4] {
    if !(dot3_v(normal, normal) > TURN_INPUT_EPSILON) {
        return v;
    }
    let n = normalize3(normal);
    let d = dot3_v(v, n);
    let into = if d > 0.0 { 0.0 } else { d };
    std::array::from_fn(|k| v[k] - n[k] * into)
}

/// What sub_82349ED8 reads.
#[derive(Clone, Copy, Debug)]
pub struct StopWalkState {
    /// World input direction (sub_823083F0).
    pub input: [f32; 4],
    /// Body forward axis (sub_8230A4E0).
    pub forward: [f32; 4],
    /// ctx+0x3C0 (sub_82316118): on the ground.
    pub grounded: bool,
    /// ctx+0x54C (sub_823167E0): time since the stick changed.
    pub timer: f32,
    /// |ctx+0x220| (sub_8231A468).
    pub speed: f32,
}

/// sub_82349ED8: NoPadStopWalk (ctx flag 26). Some(true) sets it, Some(false) clears
/// it, None leaves it. Parameters: 0x08C pad length limit, 0x08E delay, 0x08F speed.
pub fn no_pad_stop_walk(s: &StopWalkState, param: impl Fn(u16) -> f32) -> Option<bool> {
    let forward_input = dot3_v(s.input, s.forward);
    if s.grounded && s.timer < param(0x8E) {
        let limit = param(0x8C);
        if forward_input < limit * limit && s.speed < param(0x8F) {
            return Some(true);
        }
        return None;
    }
    let limit = param(0x8C);
    if forward_input > limit * limit { Some(false) } else { None }
}

/// What the acceleration step (sub_8234C5F8, sub_82349D88, sub_82316160) reads.
#[derive(Clone, Copy, Debug)]
pub struct AccelState {
    /// ctx flag 14 (boosting).
    pub boosting: bool,
    /// Body forward axis (sub_8230A4E0).
    pub forward: [f32; 4],
    /// ctx+0x220 horizontal velocity (up to date).
    pub velocity: [f32; 4],
    /// ctx+0x6B0 contact normal.
    pub normal: [f32; 4],
    /// ctx+0x3C0 grounded, flag 26 NoPadStopWalk.
    pub grounded: bool,
    pub no_pad_stop_walk: bool,
    /// Context vf28: the character's acceleration force.
    pub accel_force: f32,
    /// ctx+0x4B8 and ctx+0x540 (sub_82316160).
    pub fallback_div: f32,
    pub fallback_scale: f32,
    pub dt: f32,
}

/// sub_8234C5F8's arguments: input direction (r5), speed cap (f1), acceleration
/// factor (f2), deceleration factor (f3), the alternate mode (r7).
#[derive(Clone, Copy, Debug)]
pub struct AccelInput {
    pub input: [f32; 4],
    pub cap: f32,
    pub accel_factor: f32,
    pub decel_factor: f32,
    pub alt: bool,
}

/// sub_82349D88: deceleration toward `target` from `current` (speeds along the
/// velocity), as a velocity change.
fn deceleration(s: &AccelState, target: f32, current: f32, cap: f32, k: f32, alt: bool, param: &impl Fn(u16) -> f32) -> [f32; 4] {
    let force = if s.grounded && s.no_pad_stop_walk {
        param(0x8D)
    } else if alt {
        param(0x11)
    } else {
        param(0x12)
    };
    let amount = if current > 0.0 {
        let gap = target - current;
        let gap = if gap > 0.0 { 0.0 } else { gap };
        let max = force * k;
        let a = -(gap / s.dt);
        -(if a < max { a } else { max })
    } else {
        let max = force * k;
        let a = (target - current) / s.dt;
        if a < max { a } else { max }
    };
    let v = normalize3(s.velocity).map(|c| c * amount);
    if current > cap { v } else { remove_into_normal(s.normal, v) }
}

/// sub_8234C5F8: this frame's velocity change toward the input's target speed.
pub fn acceleration(s: &AccelState, i: &AccelInput, param: impl Fn(u16) -> f32) -> [f32; 4] {
    use super::constants::{ACCEL_HOLD_GAP, ACCEL_HOLD_SPEED, ACCEL_TARGET_EPSILON, DECEL_MIN_SQUARED};
    let input = i.input;
    let amount = if s.boosting {
        length3([input[0], input[1], input[2]])
    } else if i.alt {
        let d = dot3_v(s.forward, input);
        if d > 0.0 { d } else { 0.0 }
    } else {
        dot3_v(s.forward, input)
    };
    let want = s.forward.map(|c| c * amount);
    let target_v = want.map(|c| c * i.cap);
    let target = length3([target_v[0], target_v[1], target_v[2]]);
    // bge: anything but `target < eps` (NaN included) takes the projection.
    let current = if target < ACCEL_TARGET_EPSILON {
        length3([s.velocity[0], s.velocity[1], s.velocity[2]])
    } else {
        dot3_v(s.velocity, normalize3(target_v))
    };
    if target.abs() > ACCEL_HOLD_SPEED && (target - current).abs() < ACCEL_HOLD_GAP {
        return [0.0; 4];
    }
    if target > current {
        let max = s.accel_force * i.accel_factor;
        let a = (target - current) / s.dt;
        let accel = if a < max { a } else { max };
        if i.alt {
            return normalize3(want).map(|c| c * accel);
        }
        let along = dot3_v(normalize3(want).map(|c| c * accel), normalize3(input));
        return normalize3(input).map(|c| c * along);
    }
    let target = if i.alt { length3([input[0], input[1], input[2]]) * i.cap } else { target };
    let d = deceleration(s, target, current, i.cap, i.decel_factor, i.alt, &param);
    if !(dot3_v(d, d) < DECEL_MIN_SQUARED) || !(current < i.cap) {
        return d;
    }
    let v2 = dot3_v(s.velocity, s.velocity);
    let factor = s.accel_force / (s.fallback_div * s.fallback_div) * s.fallback_scale;
    normalize3(s.velocity).map(|c| c * (factor * v2))
}

/// What sub_8231A0E8 reads.
#[derive(Clone, Copy, Debug)]
pub struct VelocityFrame {
    /// ctx+0x210: world velocity.
    pub velocity: [f32; 4],
    /// ctx+0x250: world to ground-frame rotation; ctx+0x290: back.
    pub to_local: Mat4,
    pub to_world: Mat4,
    /// Body rotation quaternion (sub_82308308: ctx+0x10 -> +0x60).
    pub body_rotation: [f32; 4],
}

/// sub_8231A0E8: splits the world velocity into its ground-plane part (ctx+0x220) and
/// its part along the ground normal (ctx+0x230), each a direction in the ground frame
/// times its length. Below 0.01 the plane direction is the body's flattened facing
/// and the normal direction is straight up (flattened facing renormalized in place by
/// sub_82E861D8; body rotation via sub_82308308).
pub fn split_velocity(f: &VelocityFrame) -> ([f32; 4], [f32; 4]) {
    use super::constants::ACCEL_TARGET_EPSILON as SMALL; // DAT_82009f28, 0.01
    let local = transform3(&f.to_local, f.velocity);
    let vertical = [0.0, local[1], 0.0, 0.0];
    let horizontal = [local[0], 0.0, local[2], local[3]];
    let v_len = length3([vertical[0], vertical[1], vertical[2]]);
    let h_len = length3([horizontal[0], horizontal[1], horizontal[2]]);
    let mut h_dir = normalize3(horizontal);
    let mut v_dir = normalize3(vertical);
    if h_len < SMALL {
        let facing = super::xenon_math::forward(&quat_to_matrix(f.body_rotation));
        let mut flat = transform3(&f.to_local, facing);
        flat[1] = 0.0;
        h_dir = normalize3(flat);
    }
    if v_len < SMALL {
        v_dir = [0.0, 1.0, 0.0, 1.0];
    }
    (
        transform3(&f.to_world, h_dir).map(|c| c * h_len),
        transform3(&f.to_world, v_dir).map(|c| c * v_len),
    )
}

/// The speed context's velocity (ctx+0x210) and its ground-frame split (+0x220
/// horizontal, +0x230 vertical), kept lazily: +0x565 marks the split stale, +0x566
/// the velocity.
#[derive(Clone, Copy, Debug)]
pub struct ContextVelocity {
    pub frame: VelocityFrame,
    pub horizontal: [f32; 4],
    pub vertical: [f32; 4],
    pub split_stale: bool,
    pub velocity_dirty: bool,
}

impl ContextVelocity {
    fn resplit(&mut self) {
        let (h, v) = split_velocity(&self.frame);
        self.horizontal = h;
        self.vertical = v;
        self.split_stale = false;
        self.velocity_dirty = false;
    }

    /// sub_8231D0D0: the horizontal velocity, re-split first when stale.
    pub fn horizontal(&mut self) -> [f32; 4] {
        if self.split_stale {
            self.resplit();
        }
        self.horizontal
    }

    /// sub_8231CF10: the world velocity, rebuilt from the split first when that was edited.
    pub fn velocity(&mut self) -> [f32; 4] {
        if self.velocity_dirty {
            self.frame.velocity = std::array::from_fn(|k| self.horizontal[k] + self.vertical[k]);
            self.velocity_dirty = false;
            self.split_stale = false;
        }
        self.frame.velocity
    }

    /// sub_82317020: replaces the world velocity (the split goes stale).
    pub fn set_velocity(&mut self, v: [f32; 4]) {
        self.frame.velocity = v;
        self.split_stale = true;
        self.velocity_dirty = false;
    }

    /// sub_8231CF50: replaces the horizontal velocity.
    pub fn set_horizontal(&mut self, h: [f32; 4]) {
        if self.split_stale {
            self.resplit();
        }
        self.horizontal = h;
        self.velocity_dirty = true;
        self.split_stale = false;
    }

    /// sub_8231D020 / sub_8231D078: adds to the world velocity (rebuilt from the split
    /// first when that was edited).
    pub fn add(&mut self, dv: [f32; 4]) {
        if self.velocity_dirty {
            self.frame.velocity = std::array::from_fn(|k| self.horizontal[k] + self.vertical[k]);
            self.velocity_dirty = false;
            self.split_stale = false;
        }
        self.frame.velocity = std::array::from_fn(|k| self.frame.velocity[k] + dv[k]);
        self.split_stale = true;
        self.velocity_dirty = false;
    }
}

/// What sub_82320820 reads besides the velocity.
#[derive(Clone, Copy, Debug)]
pub struct ApplyState {
    /// ctx flag 3: frozen; flag 31: KeepRunning.
    pub flag_frozen: bool,
    pub keep_running: bool,
    /// KeepRunning's guide direction (sub_822D20B0 on ctx+0x650), when it has one.
    pub guide: Option<[f32; 4]>,
    /// ctx+0x658: guide already points forward (otherwise it is reversed).
    pub guide_forward: bool,
    /// ctx+0x3C0.
    pub grounded: bool,
}

/// sub_82320820: velocity += v * scale. Under KeepRunning the speed along the guide
/// direction may only fall to KeepRunningMinVelocity (0x0B6) through this (reversals
/// are sub_82E86250's in-place scale by -1).
pub fn apply_velocity_change(cv: &mut ContextVelocity, s: &ApplyState, v: [f32; 4], scale: f32, param: impl Fn(u16) -> f32) {
    if s.flag_frozen {
        return;
    }
    if !s.keep_running {
        cv.add(v.map(|c| c * scale));
        return;
    }
    let dir = match s.guide {
        Some(g) => {
            let g = if s.guide_forward { g } else { g.map(|c| c * -1.0) };
            if s.grounded {
                g
            } else {
                let h = normalize3(cv.horizontal());
                if dot3_v(g, h) < 0.0 { h.map(|c| c * -1.0) } else { h }
            }
        }
        None => normalize3(cv.horizontal()),
    };
    let before = dot3_v(cv.horizontal(), dir);
    cv.add(v.map(|c| c * scale));
    let after = dot3_v(cv.horizontal(), dir);
    if !(after < before) {
        return;
    }
    let min = param(0xB6);
    let keep = if min > after { min } else { after };
    cv.set_horizontal(dir.map(|c| c * keep));
}

/// sub_82321A90: the ground step's apply, with v scaled by ctx+0x554 first.
pub fn apply_scaled(cv: &mut ContextVelocity, s: &ApplyState, v: [f32; 4], input_scale: f32, dt: f32, param: impl Fn(u16) -> f32) {
    apply_velocity_change(cv, s, v.map(|c| c * input_scale), dt, param)
}

/// sub_8230A430: the body's side axis, (-1, 0, 0, 0) * m.
pub fn side_axis(m: &Mat4) -> [f32; 4] {
    transform3(m, [-1.0, 0.0, 0.0, 0.0])
}

/// Engine services the posture code calls (sheets/boundaries.csv). A host game
/// implements this to run Sonic's movement on its own world and character body.
pub trait PostureHost {
    /// sub_8245DB60: parameter by id.
    fn param(&self, id: u16) -> f32;
    /// Body object vf2: the body's matrix (its forward/side axes come from it).
    fn body_matrix(&self) -> Mat4;
    /// Body +0x60: the body's rotation quaternion.
    fn body_rotation(&self) -> [f32; 4];
    /// sub_82308300: turn the body by a rotation delta.
    fn rotate_body(&mut self, delta: [f32; 4]);
    /// sub_822D20B0: KeepRunning's guide direction (only asked when the context has a
    /// guide object).
    fn keep_running_direction(&mut self) -> [f32; 4];
}

/// The speed context state the ground step reads and writes (sheets/context_fields.csv).
#[derive(Clone, Copy, Debug)]
pub struct SpeedContext {
    pub velocity: ContextVelocity,
    pub stick: [f32; 4],
    pub up: [f32; 4],
    pub grounded: bool,
    pub rotation: [f32; 4],
    pub target_rotation: [f32; 4],
    /// Flag set (ctx+0x4AC), by index (context::flag).
    pub flags: [bool; 64],
    pub flags2_7: bool,
    pub fallback_div: f32,
    pub rotation_blend: f32,
    pub fallback_scale: f32,
    pub stick_timer: f32,
    pub input_scale: f32,
    pub contact_normal: [f32; 4],
    pub blocked_count: i32,
    pub lean_input: f32,
    pub has_guide: bool,
    pub guide_forward: bool,
    /// Context vf14 / vf28.
    pub input_locked: bool,
    pub accel_force: f32,
    /// posture timing +0x24.
    pub dt: f32,
}

impl SpeedContext {
    fn flag(&self, i: u32) -> bool {
        self.flags[i as usize]
    }

    fn horizontal(&mut self, host: &impl PostureHost) -> [f32; 4] {
        self.velocity.frame.body_rotation = host.body_rotation();
        self.velocity.horizontal()
    }

    fn rotation_force_state(&self) -> RotationForceState {
        use super::context::flag;
        RotationForceState {
            flag_14: self.flag(flag::BOOSTING),
            flag_22: self.flag(flag::ROTATION_FORCE_BLEND),
            blend: self.rotation_blend,
            flag2_7: self.flags2_7,
            blocked_count: self.blocked_count,
        }
    }

    fn apply(&mut self, host: &mut impl PostureHost, v: [f32; 4]) {
        use super::context::flag;
        let guide = if self.flag(flag::KEEP_RUNNING) && self.has_guide { Some(host.keep_running_direction()) } else { None };
        let s = ApplyState {
            flag_frozen: self.flag(flag::FROZEN),
            keep_running: self.flag(flag::KEEP_RUNNING),
            guide,
            guide_forward: self.guide_forward,
            grounded: self.grounded,
        };
        self.velocity.frame.body_rotation = host.body_rotation();
        let (scale, dt) = (self.input_scale, self.dt);
        apply_scaled(&mut self.velocity, &s, v, scale, dt, |id| host.param(id));
    }
}

/// sub_82347448: Posture3DStandard's speed control for one frame: sideways drag
/// (mode 1), turning (sub_82349538), NoPadStopWalk, then acceleration toward `cap`.
///
/// Field accessors are inlined as SpeedContext fields: sub_8315CF68 / sub_82A563A8
/// (posture -> context), sub_823171C8 / sub_82333D78 (flag test), sub_823171D0 /
/// sub_823171D8 / sub_82333D90 / sub_82333DA0 (flag set / clear), sub_823161D8,
/// sub_823161E0, sub_82316118, sub_823165B0, sub_823167E0, sub_82317040, sub_82E627D0,
/// sub_82489960; sub_8231A468 / sub_8231A4E8 are length3 / normalize3 of the
/// horizontal velocity; sub_82456238 wraps sub_823083F0; sub_82316990 adds lean input.
pub fn ground_step(cx: &mut SpeedContext, host: &mut impl PostureHost, mode: i32, cap: f32) {
    use super::context::flag;
    let input = input_direction(cx.stick, cx.input_locked, host.param(0x0C));
    let side = side_axis(&host.body_matrix());
    cx.lean_input = dot3_v(side, input) + cx.lean_input;
    if mode == 1 {
        let drag = side_drag_force(cx.flag(flag::BOOSTING), true);
        let side = side_axis(&host.body_matrix());
        let h = cx.horizontal(host);
        let d = dot3_v(normalize3(h), side);
        let limit = d.abs() * drag;
        let speed = length3([h[0], h[1], h[2]]);
        let amount = if speed > 0.0 {
            let a = speed / cx.dt;
            -(if a < limit { a } else { limit })
        } else {
            let a = -(speed / cx.dt);
            if a < limit { a } else { limit }
        };
        let h = cx.horizontal(host);
        let v = remove_into_normal(cx.contact_normal, normalize3(h).map(|c| c * amount));
        cx.apply(host, v);
    }
    let h = cx.horizontal(host);
    let speed = length3([h[0], h[1], h[2]]);
    let rf = cx.rotation_force_state();
    let force = rotation_force(&rf, speed, |id| host.param(id));
    let fwd = super::xenon_math::forward(&host.body_matrix());
    let h = cx.horizontal(host);
    let along = dot3_v(normalize3(h), fwd);
    let velocity_force = (if along >= 0.0 { along } else { 0.0 }) * force;
    let max_turn_speed = rotation_speed_max(&rf, |id| host.param(id));
    let h = cx.horizontal(host);
    let ts = TurnState {
        rotation: cx.rotation,
        forward: super::xenon_math::forward(&host.body_matrix()),
        velocity: h,
        up: cx.up,
        input_scale: cx.input_scale,
        flag_no_turn: cx.flag(flag::FROZEN),
        flag_keep_facing: cx.flag(flag::KEEP_FACING),
        dt: cx.dt,
        rate: RotationForceRate {
            rate_min: host.param(0x88),
            min_angle: host.param(0x89),
            rate_max: host.param(0x8A),
            max_angle: host.param(0x8B),
        },
    };
    let r = turn(&ts, &TurnInput { input, force, max_turn_speed, velocity_force, turn_velocity: true });
    if let Some((q, delta)) = r.rotation {
        cx.target_rotation = q;
        host.rotate_body(delta);
    }
    if let Some(v) = r.velocity {
        cx.velocity.set_horizontal(v);
    }
    // sub_82349ED8 reads the speed only on one path; fetch it the same way.
    let sw = StopWalkState {
        input: input_direction(cx.stick, cx.input_locked, host.param(0x0C)),
        forward: super::xenon_math::forward(&host.body_matrix()),
        grounded: cx.grounded,
        timer: cx.stick_timer,
        speed: f32::NAN,
    };
    if let Some(on) = no_pad_stop_walk_with(&sw, || {
        let h = cx.horizontal(host);
        length3([h[0], h[1], h[2]])
    }, |id| host.param(id)) {
        cx.flags[flag::NO_PAD_STOP_WALK as usize] = on;
    }
    let h = cx.horizontal(host);
    let accel = acceleration(
        &AccelState {
            boosting: cx.flag(flag::BOOSTING),
            forward: super::xenon_math::forward(&host.body_matrix()),
            velocity: h,
            normal: cx.contact_normal,
            grounded: cx.grounded,
            no_pad_stop_walk: cx.flag(flag::NO_PAD_STOP_WALK),
            accel_force: cx.accel_force,
            fallback_div: cx.fallback_div,
            fallback_scale: cx.fallback_scale,
            dt: cx.dt,
        },
        &AccelInput { input, cap, accel_factor: 1.0, decel_factor: 1.0, alt: mode == 1 },
        |id| host.param(id),
    );
    cx.apply(host, accel);
}

/// no_pad_stop_walk with the speed fetched only where sub_82349ED8 reads it.
fn no_pad_stop_walk_with(s: &StopWalkState, speed: impl FnOnce() -> f32, param: impl Fn(u16) -> f32) -> Option<bool> {
    let forward_input = dot3_v(s.input, s.forward);
    let limit = param(0x8C);
    if s.grounded && s.timer < param(0x8E) {
        if forward_input < limit * limit && speed() < param(0x8F) {
            return Some(true);
        }
        return None;
    }
    if forward_input > limit * limit { Some(false) } else { None }
}

/// What sub_82349FF0 (the brake) reads besides the context velocity.
#[derive(Clone, Copy, Debug)]
pub struct BrakeState {
    /// World input direction (r6).
    pub input: [f32; 4],
    /// Direction braked against: the velocity direction when the brake began
    /// (CStateBrakeLow +0x60, r8).
    pub brake_dir: [f32; 4],
    /// Brake force (f1) and the time the brake must at least last (f2).
    pub force: f32,
    pub min_time: f32,
    pub grounded: bool,
    /// Body forward axis (grounded) and the control frame's matrix (sub_82316EB0, air).
    pub forward: [f32; 4],
    pub control: Mat4,
    pub dt: f32,
}

/// sub_82349FF0: one braking frame. Updates `timer` (posture+0x64, time spent nearly
/// stopped along the brake direction), applies the braking velocity change, and
/// returns whether the brake goes on (false: it ended this frame).
pub fn brake(
    cv: &mut ContextVelocity,
    apply: &ApplyState,
    input_scale: f32,
    timer: &mut f32,
    s: &BrakeState,
    param: impl Fn(u16) -> f32,
) -> bool {
    use super::constants::{ACCEL_HOLD_GAP, BRAKE_STOP_SPEED, BRAKE_VIEW_ANGLE};
    use super::crt_math::cos;
    let input_len = length3([s.input[0], s.input[1], s.input[2]]);
    let along = dot3_v(cv.horizontal(), s.brake_dir);
    *timer = if along < ACCEL_HOLD_GAP { s.dt + *timer } else { 0.0 };
    let done = if input_len > ACCEL_HOLD_GAP {
        let n = s.input.map(|c| c * (1.0 / input_len));
        let facing = if s.grounded {
            s.forward
        } else {
            let v = normalize3(cv.horizontal());
            // sub_82E84EE0: the control frame's -Z column.
            let view = [-s.control[0][2], -s.control[1][2], -s.control[2][2], -s.control[3][2]];
            if dot3_v(v, view) > cos(BRAKE_VIEW_ANGLE as f64) as f32 { view } else { v }
        };
        if *timer > s.min_time {
            true
        } else {
            let cancel = cos((param(0x23) * super::constants::DEG_TO_RAD) as f64) as f32;
            dot3_v(facing, n) > cancel
        }
    } else {
        let h = cv.horizontal();
        length3([h[0], h[1], h[2]]) > BRAKE_STOP_SPEED
    };
    let mut change = [0.0; 4];
    if !done {
        let h = normalize3(cv.horizontal());
        let v = h.map(|c| -c).map(|c| c * s.force);
        change = v;
        let step = s.dt * length3([v[0], v[1], v[2]]);
        let h = cv.horizontal();
        if step > length3([h[0], h[1], h[2]]) {
            // Would overshoot: cancel the velocity exactly this frame.
            let h = cv.horizontal().map(|c| -c);
            let inv = 1.0 / s.dt;
            change = h.map(|c| c * inv);
        }
    }
    apply_scaled(cv, apply, change, input_scale, s.dt, &param);
    !done
}

/// sub_82317858: ground pinning per unit speed, Gravity (0x0E) / DownForceEfficient (0x31).
pub fn down_force_per_speed(param: impl Fn(u16) -> f32) -> f32 {
    param(0x0E) / param(0x31)
}

/// sub_82316DF8: acos with its argument clamped to [-1, 1] (NaN passes), as f32.
pub fn clamped_acos(x: f32) -> f32 {
    let c = if x > 1.0 {
        1.0
    } else if x < -1.0 {
        -1.0
    } else {
        x
    };
    super::crt_math::acos(c as f64) as f32
}

fn cos_deg(deg: f32) -> f32 {
    super::crt_math::cos((deg * super::constants::DEG_TO_RAD) as f64) as f32
}

/// sub_82317E08: `a` and `b` within MoveEnableGroundAngleInner (0x78, `inner`) or
/// ...Outer (0x79) of each other.
pub fn move_enable_ground_angle(a: [f32; 4], b: [f32; 4], inner: bool, param: impl Fn(u16) -> f32) -> bool {
    let limit = cos_deg(param(if inner { 0x78 } else { 0x79 }));
    !(dot3_v(b, a) < limit)
}

/// sub_82317D58: `a` and `b` within LandEnableMaxSlope (0x73), 2.5 degrees more while
/// grounded; always true under context flag 9.
pub fn land_enable(flag_9: bool, grounded: bool, a: [f32; 4], b: [f32; 4], param: impl Fn(u16) -> f32) -> bool {
    use super::constants::LAND_SLOPE_HYSTERESIS;
    if flag_9 {
        return true;
    }
    let deg = if grounded { param(0x73) + LAND_SLOPE_HYSTERESIS } else { param(0x73) };
    !(dot3_v(a, b) < cos_deg(deg))
}

/// sub_82317A28: a surface `normal` Sonic may land on: within LandEnableMaxSlope of
/// ctx+0x3D0, or its reverse within it of ctx+0x6A0's direction; true under flag 9.
pub fn land_enable_normal(flag_9: bool, up_3d0: [f32; 4], dir_6a0: [f32; 4], normal: [f32; 4], param: impl Fn(u16) -> f32) -> bool {
    if flag_9 || dot3_v(up_3d0, normal) > cos_deg(param(0x73)) {
        return true;
    }
    let reverse = normal.map(|c| -c);
    dot3_v(reverse, normalize3(dir_6a0)) > cos_deg(param(0x73))
}
