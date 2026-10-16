//! Daytime Sonic's movement rules, ported from the decompiled executable.
//!
//! Scalar logic (thresholds, timers, state changes, force selection, ring and
//! energy formulas) follows the decompiled functions named in each doc
//! comment. Their VMX128 vector code does not decompile cleanly, so the vector
//! steps are reconstructed from what the scalar code feeds them: forces along
//! the input / facing direction in the ground plane, angular rates of
//! `force / max(speed, 0.1)`. Collision is the caller's job.

use glam::Vec3;

use super::params::SonicParams;

// Generated from sheets/constants.csv.
pub use super::constants::{
    AIR_INPUT_HOLD, AIR_ROTATION_FORCE, AIR_ROTATION_FORCE_RATE, AIR_ROTATION_SPEED_MAX, HIGH_SPEED_DELAY,
    JUMP_TOP_SPEED, LOW_SPEED_LIMIT,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GroundState {
    /// CPlayerSpeedPosture3DStandard::CStateLowSpeed
    LowSpeed { timer: f32 },
    /// CPlayerSpeedPosture3DStandard::CStateHighSpeed
    HighSpeed,
    /// CPlayerSpeedPosture3DStandard::CStateBrakeLow
    Brake,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JumpPhase {
    /// Button still held: releasing now cuts the jump (sub_823386E0).
    Short { time: f32 },
    /// Held past JumpShortReleaseTime: full ball jump.
    Full,
}

/// What the game keeps in CPlayerSpeedContext for movement.
#[derive(Clone, Debug)]
pub struct Body {
    pub vel: Vec3,
    /// Ground normal when grounded, world up in the air.
    pub up: Vec3,
    pub facing: Vec3,
    pub grounded: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    /// Camera-relative stick in world space; length 0..1.
    pub dir: Vec3,
    pub jump_pressed: bool,
    pub jump_held: bool,
    pub boost_pressed: bool,
    pub boost_held: bool,
}

/// Things the presentation layer reacts to.
#[derive(Clone, Copy, Debug, Default)]
pub struct Events {
    pub jumped: bool,
    pub boost_started: bool,
    pub boost_ended: bool,
    pub brake_started: bool,
}

#[derive(Clone, Debug)]
pub struct Controller {
    pub params: SonicParams,
    pub ground: GroundState,
    pub jump: Option<JumpPhase>,
    pub boosting: bool,
    pub energy: f32,
    /// (level, max level) for the boost speed scale (ctx+0x5d8 / 0x5dc).
    pub speed_level: (i32, i32),
    /// (level, max level) for the energy gauge size (ctx+0x5d0 / 0x5d4).
    pub energy_level: (i32, i32),
    /// Vertical air-drag multiplier (ctx+0x544; 1 except during special states).
    pub air_drag_rate: f32,
    boost_air_time: f32,
    boost_end_time: f32,
}

fn project(v: Vec3, n: Vec3) -> Vec3 {
    v - n * v.dot(n)
}

/// Rotates unit vector `from` toward `to` by at most `max_angle` radians.
fn rotate_towards(from: Vec3, to: Vec3, max_angle: f32, axis_hint: Vec3) -> Vec3 {
    let angle = from.angle_between(to);
    if !angle.is_finite() || angle <= max_angle {
        return to;
    }
    let axis = from.cross(to).try_normalize().unwrap_or(axis_hint);
    glam::Quat::from_axis_angle(axis, max_angle) * from
}

fn level_t((level, max): (i32, i32)) -> f32 {
    if max <= 1 { 0.0 } else { (level - 1) as f32 / (max - 1) as f32 }
}

impl Controller {
    pub fn new(params: SonicParams) -> Self {
        let mut c = Self {
            params,
            ground: GroundState::LowSpeed { timer: 0.0 },
            jump: None,
            boosting: false,
            energy: 0.0,
            speed_level: (1, 10),
            energy_level: (1, 10),
            air_drag_rate: 1.0,
            boost_air_time: 0.0,
            boost_end_time: 0.0,
        };
        c.energy = c.energy_max();
        c
    }

    // ---- formulas -------------------------------------------------------

    /// Boost top-speed multiplier by level (sub_82371138).
    pub fn boost_max_velocity_scale(&self) -> f32 {
        let p = &self.params;
        p.boost_max_velocity_scale_lv0
            + (p.boost_max_velocity_scale_lv_max - p.boost_max_velocity_scale_lv0) * level_t(self.speed_level)
    }

    /// Top speed from rings carried (sub_82373390).
    pub fn max_velocity(&self, rings: u32, boosting: bool) -> f32 {
        let p = &self.params;
        let per = (p.increase_max_velocity_ring_num as u32).max(1);
        let mut v = p.max_velocity_basis + p.max_velocity_increment_by_100 * (rings / per).min(4) as f32;
        if rings > 100 {
            v += p.max_velocity_increment_by_200 * ((rings - 100) / per).min(4) as f32;
        }
        if rings > 200 {
            v += p.max_velocity_increment_over_200 * ((rings - 200) / per) as f32;
        }
        if boosting {
            v *= self.boost_max_velocity_scale();
        }
        v.min(p.max_velocity_final_max)
    }

    /// Ground acceleration force (CSonicContext vf28 = sub_823703E0).
    pub fn acceleration_force(&self, horizontal_speed: f32) -> f32 {
        let p = &self.params;
        if !self.boosting {
            p.acceleration_force
        } else if horizontal_speed <= p.max_velocity_basis * p.boost_start_velocity_rate {
            p.boost_acceleration_force_when_slow
        } else {
            p.boost_acceleration_force
        }
    }

    /// Deceleration force (sub_82349D88).
    pub fn deceleration_force(&self, high_speed: bool, no_pad_stop_walk: bool) -> f32 {
        let p = &self.params;
        if no_pad_stop_walk {
            p.no_pad_stop_walk_deceleration_force
        } else if high_speed {
            p.deceleration_force_high
        } else {
            p.deceleration_force_low
        }
    }

    /// Turning force from speed (sub_823472A0).
    pub fn rotation_force(&self, speed: f32) -> f32 {
        let p = &self.params;
        let f = if self.boosting {
            p.rotation_force_in_boost
        } else if speed <= p.rotation_force_velocity0 {
            p.rotation_force0
        } else if speed >= p.rotation_force_velocity1 {
            p.rotation_force1
        } else {
            let slope = (p.rotation_force1 - p.rotation_force0) / (p.rotation_force_velocity1 - p.rotation_force_velocity0);
            p.rotation_force0 + slope * (speed - p.rotation_force_velocity0)
        };
        f * p.rotation_force_scale
    }

    /// Facing turn-rate limit in rad/s (sub_8231B990).
    pub fn rotation_speed_max(&self) -> f32 {
        self.params.rotation_speed_max.to_radians()
    }

    /// Chaos-energy capacity by level (sub_82318DF0).
    pub fn energy_max(&self) -> f32 {
        let p = &self.params;
        p.chaos_energy_max_lv0 + (p.chaos_energy_max_lv_max - p.chaos_energy_max_lv0) * level_t(self.energy_level)
    }

    /// Energy from collecting `rings` at `speed` (sub_8231F9F0).
    pub fn collect_rings(&mut self, rings: u32, speed: f32) {
        let p = &self.params;
        let gain = (speed * p.ring_chaos_energy_increase_scale + p.ring_chaos_energy_increase_bias) * rings as f32;
        self.add_energy(gain);
    }

    /// sub_8231C628: add and clamp to [0, max].
    pub fn add_energy(&mut self, delta: f32) {
        self.energy = (self.energy + delta).min(self.energy_max()).max(0.0);
    }

    /// 0..1 gauge fill (sub_8231C5F0).
    pub fn energy_fraction(&self) -> f32 {
        self.energy / self.energy_max()
    }

    /// Stand -> run blend of the slope resistance (sub_8231AFE0).
    pub fn stand_run_blend(&self, speed: f32) -> f32 {
        let p = &self.params;
        let lo = p.resistance_stand_run_border - p.resistance_stand_run_interpolate;
        let hi = p.resistance_stand_run_border + p.resistance_stand_run_interpolate;
        ((speed - lo) / (hi - lo)).clamp(0.0, 1.0)
    }

    /// Slope angle (as a sine) gravity has to beat before it moves Sonic (sub_82321B78).
    pub fn slope_resistance(&self, speed: f32) -> f32 {
        let p = &self.params;
        let stand = p.dynamic_resistance_force_when_stand.to_radians().sin();
        let run = p.dynamic_resistance_force_when_run.to_radians().sin();
        stand + (run - stand) * self.stand_run_blend(speed)
    }

    /// Vertical drag coefficients so falls / rises top out at MaxDown/UpVelocity
    /// (sub_82316F50 / sub_82316FB0).
    pub fn air_drag(&self) -> (f32, f32) {
        let p = &self.params;
        let down = self.air_drag_rate / (p.max_down_velocity * p.max_down_velocity) * p.gravity;
        let up = self.air_drag_rate / (p.max_up_velocity * p.max_up_velocity) * p.gravity;
        (down, up)
    }

    /// Ground pinning acceleration per unit speed: Gravity / DownForceEfficient (sub_82317858).
    pub fn down_force_per_speed(&self) -> f32 {
        self.params.gravity / self.params.down_force_efficient
    }

    pub fn high_speed(&self) -> bool {
        self.ground == GroundState::HighSpeed
    }

    // ---- boost ----------------------------------------------------------

    /// Boost start (sub_82377628) plus the start-speed kick (sub_82370780).
    fn try_start_boost(&mut self, body: &mut Body, events: &mut Events) {
        let p = self.params.clone();
        let (need, cost, rate) = if body.grounded {
            (p.boost_enable_chaos_energy, p.boost_first_decrease, p.boost_start_velocity_rate)
        } else {
            (p.air_boost_enable_chaos_energy, p.air_boost_first_decrease, p.air_boost_start_velocity_rate)
        };
        if self.boosting || self.energy < need {
            return;
        }
        let start_speed = p.max_velocity_basis * rate;
        self.add_energy(-cost);
        self.boosting = true;
        self.boost_air_time = 0.0;
        self.boost_end_time = 0.0;
        events.boost_started = true;
        if body.grounded {
            let tangential = project(body.vel, body.up);
            let dir = tangential.try_normalize().unwrap_or(body.facing);
            body.vel = dir * tangential.length().max(start_speed);
            self.ground = GroundState::HighSpeed;
        } else {
            let flat = body.vel.with_y(0.0);
            let dir = flat.try_normalize().unwrap_or(body.facing.with_y(0.0).normalize_or(Vec3::Z));
            body.vel = dir * flat.length().max(start_speed);
        }
    }

    /// Per-frame boost upkeep (sub_8239E3B8).
    fn update_boost(&mut self, body: &Body, input: &Input, dt: f32, events: &mut Events) {
        if !self.boosting {
            return;
        }
        let drain = self.params.boost_time_decrease * dt;
        self.add_energy(-drain);
        let p = &self.params;
        let speed = project(body.vel, body.up).length();
        let mut ending = self.energy <= 0.0 || (body.grounded && speed < p.boost_min_velocity);
        let mut stop = false;
        if !input.boost_held {
            ending = true;
            stop = true;
        }
        if body.grounded {
            self.boost_air_time = 0.0;
        } else {
            self.boost_air_time += dt;
            stop |= self.boost_air_time > p.boost_in_air_keep_time;
        }
        if ending {
            self.boost_end_time += dt;
            stop |= self.boost_end_time > p.boost_end_keep_time;
        } else {
            self.boost_end_time = 0.0;
        }
        if stop {
            self.boosting = false;
            events.boost_ended = true;
        }
    }

    // ---- ground ---------------------------------------------------------

    /// One grounded frame: Posture3DStandard's state update (sub_823478B8 ->
    /// LowSpeed/HighSpeed/BrakeLow) and its speed control (sub_82347448).
    pub fn ground_step(&mut self, body: &mut Body, input: &Input, rings: u32, dt: f32) -> Events {
        let mut events = Events::default();
        if input.boost_pressed {
            self.try_start_boost(body, &mut events);
        }
        self.update_boost(body, input, dt, &mut events);

        let p = self.params.clone();
        let n = body.up;
        let mut tangential = project(body.vel, n);
        let mut speed = tangential.length();
        let facing = project(body.facing, n).normalize_or(body.facing);
        let stick = project(input.dir, n);
        let magnitude = input.dir.length().min(1.0);
        let stick_dir = stick.normalize_or_zero();
        // Boost runs forward on its own.
        let (want_dir, want_mag) = if self.boosting {
            (if stick_dir == Vec3::ZERO { facing } else { stick_dir }, 1.0)
        } else {
            (stick_dir, magnitude)
        };
        let move_dir = tangential.try_normalize().unwrap_or(facing);

        // Brake: hard reversal at speed (BrakeEnableSpeed / BrakeEnableAngle).
        let reversal = if stick_dir != Vec3::ZERO { move_dir.angle_between(stick_dir).to_degrees() } else { 0.0 };
        if self.ground != GroundState::Brake
            && !self.boosting
            && magnitude > 0.5
            && speed >= p.brake_enable_speed
            && reversal >= p.brake_enable_angle
        {
            self.ground = GroundState::Brake;
            events.brake_started = true;
        }

        let mut new_facing = facing;
        match self.ground {
            GroundState::Brake => {
                // sub_823486C0: BrakeForce until stopped or the stick comes back.
                speed = (speed - p.brake_force * dt).max(0.0);
                tangential = move_dir * speed;
                if speed <= 0.5 {
                    if stick_dir != Vec3::ZERO {
                        new_facing = stick_dir;
                    }
                    self.ground = GroundState::LowSpeed { timer: 0.0 };
                } else if magnitude < 0.5 || reversal < p.brake_cancel_angle {
                    self.ground = GroundState::LowSpeed { timer: 0.0 };
                }
            }
            GroundState::LowSpeed { .. } | GroundState::HighSpeed => {
                // Facing turns toward the stick at RotationSpeedMax (sub_82349538).
                if want_mag > 1e-4 && want_dir != Vec3::ZERO {
                    new_facing = rotate_towards(facing, want_dir, self.rotation_speed_max() * dt, n);
                }
                // Velocity follows the facing at rotation_force / speed, never past it.
                let force = self.rotation_force(speed);
                let rate = force / speed.max(0.1);
                let dir = if speed > 1e-3 { rotate_towards(move_dir, new_facing, rate * dt, n) } else { new_facing };

                // Speed toward stick * cap (sub_8234C5F8 / sub_82349D88).
                let high = self.ground == GroundState::HighSpeed;
                let cap = if high { self.max_velocity(rings, self.boosting) } else { LOW_SPEED_LIMIT };
                let target = want_mag * cap;
                // Within 0.1 of a non-trivial target the speed is left alone.
                if target.abs() <= 0.25 || (target - speed).abs() >= 0.1 {
                    if target > speed {
                        speed = (speed + self.acceleration_force(speed) * dt).min(target);
                    } else {
                        let no_pad = magnitude < p.no_pad_stop_walk_pad_length_limit
                            && speed < p.no_pad_stop_walk_begin_max_velocity;
                        speed = (speed - self.deceleration_force(high, no_pad) * dt).max(target);
                    }
                }
                tangential = dir * speed;

                // Posture state changes (sub_82347B18 / sub_82347DE0).
                self.ground = match self.ground {
                    GroundState::LowSpeed { timer } => {
                        if speed < LOW_SPEED_LIMIT * 0.9 {
                            GroundState::LowSpeed { timer: 0.0 }
                        } else if timer + dt > HIGH_SPEED_DELAY {
                            GroundState::HighSpeed
                        } else {
                            GroundState::LowSpeed { timer: timer + dt }
                        }
                    }
                    GroundState::HighSpeed if speed < LOW_SPEED_LIMIT && !self.boosting => {
                        GroundState::LowSpeed { timer: 0.0 }
                    }
                    s => s,
                };
            }
        }

        // Gravity along the slope past the resistance angle (sub_82322EC8 / sub_82321B78).
        let downhill = project(Vec3::NEG_Y * p.gravity, n);
        let sin_slope = downhill.length() / p.gravity;
        let resistance = self.slope_resistance(speed);
        if sin_slope > resistance {
            tangential += downhill.normalize() * p.gravity * (sin_slope - resistance) * dt;
        }

        body.vel = tangential.clamp_length_max(p.max_horz_velocity);
        if new_facing != Vec3::ZERO {
            body.facing = new_facing;
        }

        if input.jump_pressed {
            self.jump(body);
            events.jumped = true;
        }
        events
    }

    /// Jump take-off (sub_82322C48): ground velocity times JumpGroundVelocityRate
    /// with its rise capped at MaxUpSpeedWhenJump, plus JumpPower along the normal.
    pub fn jump(&mut self, body: &mut Body) {
        let p = &self.params;
        let mut v = project(body.vel, body.up) * p.jump_ground_velocity_rate;
        if v.y > p.max_up_speed_when_jump {
            v.y = p.max_up_speed_when_jump;
        }
        body.vel = v + body.up * p.jump_power;
        body.grounded = false;
        self.jump = Some(JumpPhase::Short { time: 0.0 });
        self.ground = GroundState::LowSpeed { timer: 0.0 };
    }

    // ---- air ------------------------------------------------------------

    /// One airborne frame: Posture3DAir (sub_82346880) plus the short-jump
    /// state (sub_823386E0) and gravity with vertical drag.
    pub fn air_step(&mut self, body: &mut Body, input: &Input, dt: f32, gravity_scale: f32) -> Events {
        let mut events = Events::default();
        if input.boost_pressed {
            self.try_start_boost(body, &mut events);
        }
        self.update_boost(body, input, dt, &mut events);
        let p = self.params.clone();

        // Short jump: releasing within JumpShortReleaseTime cuts the rise to
        // what a JumpPowerMin jump would have now (sub_8231D588).
        if let Some(JumpPhase::Short { time }) = self.jump {
            let time = time + dt;
            if !input.jump_held {
                let cut = (p.jump_power_min - p.gravity * time).max(0.0);
                if body.vel.y > cut {
                    body.vel.y = cut;
                }
                self.jump = Some(JumpPhase::Full);
            } else if time > p.jump_short_release_time {
                self.jump = Some(JumpPhase::Full);
            } else {
                self.jump = Some(JumpPhase::Short { time });
            }
        }

        // Horizontal control: accelerate up to max(InAirMaxAccelVelocity, current).
        let flat = body.vel.with_y(0.0);
        let mut speed = flat.length();
        let magnitude = input.dir.length().min(1.0);
        let stick_dir = input.dir.with_y(0.0).normalize_or_zero();
        let facing = body.facing.with_y(0.0).normalize_or(Vec3::Z);
        let (want_dir, want_mag) = if self.boosting {
            (if stick_dir == Vec3::ZERO { facing } else { stick_dir }, 1.0)
        } else {
            (stick_dir, magnitude)
        };
        let mut new_facing = facing;
        if want_mag > 1e-4 && want_dir != Vec3::ZERO {
            new_facing = rotate_towards(facing, want_dir, AIR_ROTATION_SPEED_MAX * dt, Vec3::Y);
        }
        let move_dir = flat.try_normalize().unwrap_or(facing);
        let force = AIR_ROTATION_FORCE * AIR_ROTATION_FORCE_RATE;
        let dir = if speed > 1e-3 { rotate_towards(move_dir, new_facing, force / speed.max(0.1) * dt, Vec3::Y) } else { new_facing };

        let cap = if self.boosting {
            self.max_velocity(0, true)
        } else {
            p.in_air_max_accel_velocity.max(speed)
        };
        let target = want_mag * cap;
        let braking = stick_dir != Vec3::ZERO && magnitude > 0.5 && move_dir.dot(stick_dir) < -0.5 && speed > 0.1;
        if braking {
            // Posture3DAir::CStateBrake: BrakeForce scaled by InAirForceBrakeRate.
            speed = (speed - p.brake_force * p.in_air_force_brake_rate * dt).max(0.0);
        } else if target > speed {
            speed = (speed + self.acceleration_force(speed) * p.in_air_force_accel_rate * dt).min(target);
        } else if target < speed {
            let rate = if magnitude > AIR_INPUT_HOLD { 0.0 } else { p.in_air_force_dec_rate };
            speed = (speed - self.deceleration_force(false, false) * rate * dt).max(target);
        }
        let flat = if braking { move_dir * speed } else { dir * speed };

        // Gravity with quadratic vertical drag toward MaxDown/UpVelocity.
        let (k_down, k_up) = self.air_drag();
        let g = p.gravity * gravity_scale;
        let mut vy = body.vel.y;
        let drag = if vy < 0.0 { k_down * vy * vy } else { -k_up * vy * vy };
        vy += (-g + drag) * dt;
        body.vel = flat.clamp_length_max(p.max_horz_velocity) + Vec3::Y * vy;
        if new_facing != Vec3::ZERO {
            body.facing = new_facing;
        }
        events
    }

    /// Landing resets the jump and picks the posture from speed.
    pub fn land(&mut self, body: &Body) {
        self.jump = None;
        let speed = project(body.vel, body.up).length();
        self.ground = if speed >= LOW_SPEED_LIMIT { GroundState::HighSpeed } else { GroundState::LowSpeed { timer: 0.0 } };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sonic::params::View;

    fn controller() -> Controller {
        let (p, _) = SonicParams::load(View::Forward, [("ForwardView/Velocity/MaxVelocityBasis", 60.0)]);
        Controller::new(p)
    }

    #[test]
    fn top_speed_grows_with_rings() {
        let c = controller();
        assert_eq!(c.max_velocity(0, false), 60.0);
        assert_eq!(c.max_velocity(25, false), 62.5);
        assert_eq!(c.max_velocity(100, false), 70.0);
        assert_eq!(c.max_velocity(200, false), 74.0);
        assert!((c.max_velocity(0, true) - 84.0).abs() < 1e-4);
    }

    #[test]
    fn rotation_force_interpolates() {
        let c = controller();
        assert_eq!(c.rotation_force(10.0), 100.0);
        assert_eq!(c.rotation_force(30.0), 90.0);
        assert_eq!(c.rotation_force(50.0), 80.0);
    }

    #[test]
    fn energy_and_rings() {
        let mut c = controller();
        assert_eq!(c.energy_max(), 50.0);
        c.energy = 0.0;
        c.collect_rings(1, 50.0);
        assert!((c.energy - 2.25).abs() < 1e-5);
    }

    #[test]
    fn accelerates_to_low_cap_then_switches_to_high_speed() {
        let mut c = controller();
        let mut body = Body { vel: Vec3::ZERO, up: Vec3::Y, facing: Vec3::Z, grounded: true };
        let input = Input { dir: Vec3::Z, ..Default::default() };
        let dt = 1.0 / 60.0;
        for _ in 0..60 {
            c.ground_step(&mut body, &input, 0, dt);
        }
        assert!(c.high_speed(), "{:?}", c.ground);
        for _ in 0..600 {
            c.ground_step(&mut body, &input, 0, dt);
        }
        assert!((body.vel.length() - 60.0).abs() < 0.01, "{}", body.vel.length());
    }

    #[test]
    fn full_and_short_jump_heights() {
        let dt = 1.0 / 240.0;
        let height = |hold: f32| {
            let mut c = controller();
            let mut body = Body { vel: Vec3::ZERO, up: Vec3::Y, facing: Vec3::Z, grounded: true };
            c.jump(&mut body);
            let (mut y, mut t, mut top) = (0.0f32, 0.0f32, 0.0f32);
            while t < 3.0 {
                let input = Input { jump_held: t < hold, ..Default::default() };
                c.air_step(&mut body, &input, dt, 1.0);
                y += body.vel.y * dt;
                top = top.max(y);
                t += dt;
                if y < 0.0 {
                    break;
                }
            }
            top
        };
        let full = height(10.0);
        let short = height(0.02);
        assert!(full > 3.5 && full < 4.2, "{full}");
        assert!(short < 1.0, "{short}");
    }
}
