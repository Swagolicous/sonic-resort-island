//! Daytime Sonic: input, surface-following movement, and his original model.
//!
//! Running, turning, acceleration, braking, slopes, boost, jumping and gravity
//! come from `unleashed_game::sonic` (ported from the executable, driven by its
//! parameter table plus `Sonic.prm.xml`). This module owns collision against
//! the stage, set-object launches, and the moves not ported yet (homing attack,
//! stomp, quickstep), which use the retail parameter values where known.

use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use unleashed_assets::{mirage::mesh::parse_model, params::parse_param_xml};
use unleashed_game::sonic::{self as game, Controller, GroundState, SonicParams, View};

use crate::{
    Run, Stage,
    paths::StagePaths,
    convert::{self, Shading},
    physics::{CollisionWorld, SKIN},
    world::Converted,
};

pub const RADIUS: f32 = 0.5;
/// Homing lock-on reach (HomingLockonCollisionFar is 20; the cone is approximated).
const HOMING_RANGE: f32 = 20.0;
/// Placeholder quickstep (RunQuickStepMoveDistance not ported yet).
const QUICKSTEP_SPEED: f32 = 28.0;
const QUICKSTEP_TIME: f32 = 0.13;
const SUBSTEP: f32 = 1.0 / 240.0;
/// Steepest surface (normal.y) Sonic can stand on at low speed.
const GROUND_MIN_Y: f32 = 0.45;
/// Above this speed Sonic follows curved walls, ceilings, loops, corkscrews...
const WALL_RUN_SPEED: f32 = 30.0;
/// ...but only across gradual curvature (cos of the max normal change).
const WALL_RUN_CURVE: f32 = 0.8;

/// Whether `n` can become Sonic's ground given his current up and speed.
fn can_stand(n: Vec3, up: Vec3, speed: f32) -> bool {
    n.y > GROUND_MIN_Y || (speed > WALL_RUN_SPEED && n.dot(up) > WALL_RUN_CURVE)
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerInput>()
            .add_systems(Startup, spawn_sonic)
            .add_systems(
                Update,
                (grab_cursor, read_input, simulate, animate_visual.in_set(PlayerVisualSet))
                    .chain()
                    .in_set(PlayerSet),
            );
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlayerSet;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlayerVisualSet;

#[derive(Resource, Default)]
pub struct PlayerInput {
    pub stick: Vec2,
    pub look: Vec2,
    pub jump_pressed: bool,
    pub jump_held: bool,
    pub boost_pressed: bool,
    pub boost_held: bool,
    pub stomp_pressed: bool,
    /// B and Y as held buttons (guest mode sends the pad's held state to the game: B is
    /// slide on the ground and stomp in the air, Y the light speed dash).
    pub stomp_held: bool,
    /// The X button alone (boost_held also takes the right trigger, a binding of the
    /// hand-written controller that the game does not have).
    pub x_held: bool,
    pub y_held: bool,
    pub step: f32,
    pub respawn: bool,
    /// Raw pad state for guest mode (SWA::SPadState): right stick, triggers, bumpers.
    pub right_stick: Vec2,
    pub left_trigger: f32,
    pub right_trigger: f32,
    pub left_bumper: bool,
    pub right_bumper: bool,
    pub start: bool,
    /// Flattened camera basis from the previous frame.
    pub cam_forward: Vec3,
    pub cam_right: Vec3,
}

/// Something the homing attack can lock on to.
#[derive(Component)]
pub struct HomingTarget;

#[derive(Component)]
pub struct Sonic {
    pub center: Vec3,
    pub vel: Vec3,
    pub up: Vec3,
    pub facing: Vec3,
    pub grounded: bool,
    pub jumping: bool,
    pub air_action: bool,
    pub boosting: bool,
    /// The ported movement rules and their state (boost, energy, posture).
    pub ctl: Controller,
    pub out_of_control: f32,
    /// Seconds of gravity-free flight left after a spring or jump panel.
    pub constant_velocity: f32,
    pub homing: Option<Entity>,
    pub stomping: bool,
    pub quickstep: f32,
    pub quickstep_dir: Vec3,
    pub dead: f32,
    pub checkpoint: (Vec3, Vec3),
    pub invulnerable: f32,
    /// Forced running speed inside autorun sections.
    pub autorun: Option<f32>,
    /// 2D section: locked to a side-view path's midline.
    pub path_lock: Option<PathLock>,
    /// ForwardView / SideView parameter sets (the game's two view stores).
    params_forward: SonicParams,
    params_side: SonicParams,
    visual_up: Vec3,
    spin: f32,
    /// Guest mode: the transform the game renders Sonic with (feet position, rotation).
    pub guest_pose: Option<(Vec3, Quat)>,
    /// Guest mode: the game's ring energy gauge, 0..1.
    pub guest_boost: Option<f32>,
}

impl Sonic {
    pub fn speed(&self) -> f32 {
        self.vel.length()
    }

    /// Ring-energy gauge fill, 0..1.
    pub fn boost_gauge(&self) -> f32 {
        self.guest_boost.unwrap_or_else(|| self.ctl.energy_fraction())
    }

    pub fn braking(&self) -> bool {
        self.grounded && self.ctl.ground == GroundState::Brake
    }

    /// ChangeMode_3Dto2D: lock to side path `path` and use the SideView store.
    pub fn enter_2d(&mut self, path: usize, index: usize, ease_time: f32) {
        self.path_lock = Some(PathLock { path, index, ease: 0.0, ease_time: ease_time.max(0.01) });
        self.ctl.params = self.params_side.clone();
    }

    pub fn exit_2d(&mut self) {
        if self.path_lock.take().is_some() {
            self.ctl.params = self.params_forward.clone();
        }
    }

    fn body(&self) -> game::Body {
        game::Body { vel: self.vel, up: self.up, facing: self.facing, grounded: self.grounded }
    }

    fn set_body(&mut self, b: &game::Body) {
        self.vel = b.vel;
        self.up = b.up;
        self.facing = b.facing;
        self.grounded = b.grounded;
        self.boosting = self.ctl.boosting;
    }

    pub fn feet(&self) -> Vec3 {
        self.center - self.up * RADIUS
    }

    /// Launch from a spring / jump panel / dash panel.
    pub fn launch(&mut self, velocity: Vec3, out_of_control: f32, keep_distance: f32, airborne: bool) {
        self.vel = velocity;
        self.out_of_control = out_of_control;
        self.homing = None;
        // A launch is not a jump: releasing the button must not cut it.
        self.ctl.jump = None;
        self.stomping = false;
        if airborne {
            self.grounded = false;
            self.jumping = false;
            self.air_action = true;
            self.up = Vec3::Y;
            let speed = velocity.length().max(1.0);
            self.constant_velocity = keep_distance / speed;
        }
    }

    pub fn respawn(&mut self) {
        let (pos, facing) = self.checkpoint;
        self.center = pos + Vec3::Y * (RADIUS + 0.1);
        self.vel = Vec3::ZERO;
        self.up = Vec3::Y;
        self.facing = facing;
        self.grounded = false;
        self.jumping = false;
        self.boosting = false;
        self.ctl.boosting = false;
        self.ctl.jump = None;
        self.ctl.ground = GroundState::LowSpeed { timer: 0.0 };
        self.out_of_control = 0.0;
        self.constant_velocity = 0.0;
        self.homing = None;
        self.stomping = false;
        self.dead = 0.0;
        self.invulnerable = 1.0;
        self.autorun = None;
        self.exit_2d();
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PathLock {
    pub path: usize,
    pub index: usize,
    /// 0..1 blend from free movement to fully locked (m_PathEaseTime).
    pub ease: f32,
    pub ease_time: f32,
}

/// Parent of Sonic's skinned model and skeleton (see `anim.rs`).
#[derive(Component)]
pub struct SonicBody;

#[derive(Component)]
struct SpinBall;

#[derive(Component)]
struct BoostAura;

fn spawn_sonic(
    mut commands: Commands,
    stage: Res<Stage>,
    mut converted: ResMut<Converted>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let stage = &stage.0;
    let spawn = Vec3::from(stage.spawn.position);
    let facing = Quat::from_rotation_y(stage.spawn.yaw_degrees.to_radians()) * Vec3::Z;
    let params_forward = load_params(stage, View::Forward);
    let params_side = load_params(stage, View::Side);
    let mut sonic = Sonic {
        ctl: Controller::new(params_forward.clone()),
        path_lock: None,
        params_forward,
        params_side,
        center: spawn,
        vel: Vec3::ZERO,
        up: Vec3::Y,
        facing,
        grounded: false,
        jumping: false,
        air_action: false,
        boosting: false,
        out_of_control: 0.0,
        constant_velocity: 0.0,
        homing: None,
        stomping: false,
        quickstep: 0.0,
        quickstep_dir: Vec3::ZERO,
        dead: 0.0,
        checkpoint: (spawn, facing),
        invulnerable: 0.0,
        autorun: None,
        visual_up: Vec3::Y,
        spin: 0.0,
        guest_pose: None,
        guest_boost: None,
    };
    sonic.respawn();

    let mut spawn_model = |parent: &mut ChildSpawnerCommands, file: &str, alpha: Option<AlphaMode>| {
        let model = match stage.db.get(file).map(parse_model) {
            Some(Ok(model)) => model,
            Some(Err(e)) => return warn!("{file}: {e:#}"),
            None => return warn!("{file}: missing"),
        };
        for sub in &model.submeshes {
            let handle = convert::material(
                &stage.db,
                &mut converted.0,
                &mut images,
                &mut materials,
                &sub.material,
                sub.slot.as_ref(),
                Shading::Lit,
            );
            if let Some(mode) = alpha {
                if let Some(mut m) = materials.get_mut(&handle) {
                    m.alpha_mode = mode;
                    m.unlit = true;
                }
            }
            parent.spawn((
                Mesh3d(meshes.add(convert::submesh_mesh(sub, false))),
                MeshMaterial3d(handle),
            ));
        }
    };

    commands
        .spawn((
            Name::new("Sonic"),
            sonic,
            Transform::from_translation(spawn),
            Visibility::default(),
        ))
        .with_children(|root| {
            root.spawn((SonicBody, Transform::default(), Visibility::default()));
            root.spawn((SpinBall, Transform::default(), Visibility::Hidden))
                .with_children(|ball| spawn_model(ball, "eff_mod_sonic_spinball4.model", Some(AlphaMode::Add)));
            root.spawn((BoostAura, Transform::default(), Visibility::Hidden))
                .with_children(|aura| spawn_model(aura, "eff_mod_boost_3.model", Some(AlphaMode::Add)));
        });
}

/// One view's parameters: the executable's defaults plus `Sonic.prm.xml`.
fn load_params(stage: &unleashed_assets::stage::day::DayStage, view: View) -> SonicParams {
    let file = stage.db.get("Sonic.prm.xml").map(|b| parse_param_xml(b));
    let xml: Vec<(String, f32)> = match file {
        Some(Ok(f)) => f.values.iter().filter_map(|(k, v)| Some((k.clone(), v.trim().parse().ok()?))).collect(),
        Some(Err(e)) => {
            warn!("Sonic.prm.xml: {e:#}");
            Vec::new()
        }
        None => {
            warn!("Sonic.prm.xml missing; using executable defaults only");
            Vec::new()
        }
    };
    let (params, applied) = SonicParams::load(view, xml.iter().map(|(k, v)| (k.as_str(), *v)));
    info!("Sonic {view:?} parameters: executable table + {applied} Sonic.prm.xml overrides");
    params
}

fn grab_cursor(
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
    if keys.just_pressed(KeyCode::Escape) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

pub fn read_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    gamepads: Query<&Gamepad>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut input: ResMut<PlayerInput>,
) {
    let key = |codes: &[KeyCode]| codes.iter().any(|k| keys.pressed(*k));
    let key_down = |codes: &[KeyCode]| codes.iter().any(|k| keys.just_pressed(*k));

    let mut stick = Vec2::new(
        key(&[KeyCode::KeyD, KeyCode::ArrowRight]) as i32 as f32
            - key(&[KeyCode::KeyA, KeyCode::ArrowLeft]) as i32 as f32,
        key(&[KeyCode::KeyW, KeyCode::ArrowUp]) as i32 as f32
            - key(&[KeyCode::KeyS, KeyCode::ArrowDown]) as i32 as f32,
    );
    let mut look = if cursor.grab_mode == CursorGrabMode::Locked {
        mouse_motion.delta * 0.0025
    } else {
        Vec2::ZERO
    };
    let mut jump_pressed = key_down(&[KeyCode::Space]);
    let mut jump_held = key(&[KeyCode::Space]);
    let mut boost_pressed = key_down(&[KeyCode::ShiftLeft, KeyCode::KeyJ]);
    let mut boost_held = key(&[KeyCode::ShiftLeft, KeyCode::KeyJ]);
    let mut stomp_pressed = key_down(&[KeyCode::ControlLeft, KeyCode::KeyK]);
    let mut stomp_held = key(&[KeyCode::ControlLeft, KeyCode::KeyK]);
    let mut y_held = key(&[KeyCode::KeyF]);
    let mut x_held = key(&[KeyCode::ShiftLeft, KeyCode::KeyJ]);
    let mut step = key_down(&[KeyCode::KeyE]) as i32 as f32 - key_down(&[KeyCode::KeyQ]) as i32 as f32;
    let mut respawn = key_down(&[KeyCode::KeyR]);

    let mut right_stick = Vec2::ZERO;
    let (mut lt, mut rt, mut lb, mut rb, mut start): (f32, f32, bool, bool, bool) = (0.0, 0.0, key(&[KeyCode::KeyQ]), key(&[KeyCode::KeyE]), key(&[KeyCode::Enter]));
    if key(&[KeyCode::KeyL]) { rt = 1.0; }
    for pad in &gamepads {
        right_stick = pad.right_stick();
        lt = pad.get(GamepadButton::LeftTrigger2).unwrap_or(0.0);
        rt = rt.max(pad.get(GamepadButton::RightTrigger2).unwrap_or(0.0));
        lb |= pad.pressed(GamepadButton::LeftTrigger);
        rb |= pad.pressed(GamepadButton::RightTrigger);
        start |= pad.pressed(GamepadButton::Start);
        let ls = pad.left_stick();
        if ls.length() > 0.2 {
            stick = ls;
        }
        let rs = pad.right_stick();
        if rs.length() > 0.15 {
            look += Vec2::new(rs.x, -rs.y) * 0.05;
        }
        jump_pressed |= pad.just_pressed(GamepadButton::South);
        jump_held |= pad.pressed(GamepadButton::South);
        boost_pressed |= pad.just_pressed(GamepadButton::West)
            || pad.just_pressed(GamepadButton::RightTrigger2);
        boost_held |=
            pad.pressed(GamepadButton::West) || pad.pressed(GamepadButton::RightTrigger2);
        stomp_pressed |= pad.just_pressed(GamepadButton::East);
        stomp_held |= pad.pressed(GamepadButton::East);
        y_held |= pad.pressed(GamepadButton::North);
        x_held |= pad.pressed(GamepadButton::West);
        if pad.just_pressed(GamepadButton::RightTrigger) {
            step = 1.0;
        }
        if pad.just_pressed(GamepadButton::LeftTrigger) {
            step = -1.0;
        }
        respawn |= pad.just_pressed(GamepadButton::Select);
    }

    if let Ok(cam) = camera.single() {
        let forward = Vec3::from(cam.forward()).with_y(0.0).normalize_or(Vec3::Z);
        input.cam_forward = forward;
        input.cam_right = forward.cross(Vec3::Y);
    }
    input.stick = stick.clamp_length_max(1.0);
    input.look = look;
    input.jump_pressed = jump_pressed;
    input.jump_held = jump_held;
    input.boost_pressed = boost_pressed;
    input.boost_held = boost_held;
    input.stomp_pressed = stomp_pressed;
    input.stomp_held = stomp_held;
    input.y_held = y_held;
    input.x_held = x_held;
    input.step = step;
    input.respawn = respawn;
    input.right_stick = right_stick;
    input.left_trigger = lt;
    input.right_trigger = rt;
    input.left_bumper = lb;
    input.right_bumper = rb;
    input.start = start;
}

pub fn simulate(
    time: Res<Time>,
    world: Res<CollisionWorld>,
    mut input: ResMut<PlayerInput>,
    mut run: ResMut<Run>,
    mut sonic: Query<(&mut Sonic, &mut Transform)>,
    targets: Query<(Entity, &GlobalTransform), With<HomingTarget>>,
    paths: Res<StagePaths>,
    guest: Option<Res<crate::guest::GuestMode>>,
) {
    if guest.is_some() {
        return; // guest::step_guest moves Sonic with the game's own code
    }
    let Ok((mut s, mut transform)) = sonic.single_mut() else { return };
    let dt = time.delta_secs().min(0.1);
    run.elapsed += dt;
    if run.countdown > 0.0 {
        // Waiting for GO: no control, timer stopped.
        run.countdown -= dt;
        input.stick = Vec2::ZERO;
        input.jump_pressed = false;
        input.jump_held = false;
        input.boost_pressed = false;
        input.boost_held = false;
        input.stomp_pressed = false;
        input.step = 0.0;
    } else if !run.cleared {
        run.time += dt;
    }

    if input.respawn {
        s.respawn();
    }
    if s.dead > 0.0 {
        s.dead -= dt;
        if s.dead <= 0.0 {
            s.respawn();
        }
        return;
    }

    // One-shot actions are consumed by the first substep only.
    let mut first = true;
    let mut remaining = dt;
    while remaining > 1e-6 {
        let h = remaining.min(SUBSTEP);
        remaining -= h;
        step(&mut s, &input, first, run.rings, &world, &targets, &paths, h);
        first = false;
        if world.in_kill_volume(s.center, RADIUS) {
            s.dead = 1.2;
            s.vel = Vec3::ZERO;
            run.deaths += 1;
            run.rings = 0;
            break;
        }
    }
    input.jump_pressed = false;
    transform.translation = s.center;
}

fn project(v: Vec3, n: Vec3) -> Vec3 {
    v - n * v.dot(n)
}

fn step(
    s: &mut Sonic,
    input: &PlayerInput,
    first: bool,
    rings: u32,
    world: &CollisionWorld,
    targets: &Query<(Entity, &GlobalTransform), With<HomingTarget>>,
    paths: &StagePaths,
    h: f32,
) {
    s.out_of_control = (s.out_of_control - h).max(0.0);
    s.constant_velocity = (s.constant_velocity - h).max(0.0);
    s.invulnerable = (s.invulnerable - h).max(0.0);
    s.quickstep = (s.quickstep - h).max(0.0);

    let controllable = s.out_of_control <= 0.0;
    let mut raw = input.cam_forward * input.stick.y + input.cam_right * input.stick.x;
    // 2D: the stick only moves Sonic along the path.
    let lock_frame = s.path_lock.and_then(|l| paths.side.get(l.path).map(|p| p.frame(l.index)));
    if let Some((_, tangent, _)) = lock_frame {
        raw = tangent * raw.dot(tangent);
    }
    let pad = if controllable {
        game::Input {
            dir: raw.clamp_length_max(1.0),
            jump_pressed: first && input.jump_pressed,
            jump_held: input.jump_held,
            boost_pressed: first && input.boost_pressed,
            boost_held: input.boost_held,
        }
    } else {
        // Out of control (dash panels, springs): no stick, keep any jump hold.
        game::Input { jump_held: input.jump_held, ..default() }
    };

    if s.grounded {
        let n = s.up;
        if controllable {
            let mut body = s.body();
            let events = s.ctl.ground_step(&mut body, &pad, rings, h);
            s.set_body(&body);
            if events.jumped {
                s.jumping = true;
                s.air_action = true;
                s.center += n * 0.05;
            }
        } else {
            // Momentum is kept along the surface while control is locked.
            s.vel = project(s.vel, n);
        }
        if let Some(autorun) = s.autorun {
            let speed = s.vel.length();
            if speed < autorun {
                s.vel = s.vel.try_normalize().unwrap_or(s.facing) * autorun;
            }
        }
        // Quickstep: lateral dodge while running.
        if s.grounded && first && input.step != 0.0 && controllable && s.speed() > 5.0 && s.quickstep <= 0.0 {
            let forward = s.vel.normalize_or(s.facing);
            s.quickstep = QUICKSTEP_TIME;
            s.quickstep_dir = n.cross(forward).normalize_or_zero() * -input.step;
        }
    } else {
        let p = s.ctl.params.clone();
        if let Some(target) = s.homing.and_then(|e| targets.get(e).ok()) {
            let to = target.1.translation() + Vec3::Y * 0.6 - s.center;
            if to.length() < 1.0 {
                s.homing = None;
            } else {
                s.vel = to.normalize() * p.homing_speed;
            }
        } else {
            s.homing = None;
            if s.stomping {
                // Stomp: StompingDownFirstVelocity, then gravity up to MaxDownVelocity.
                s.vel.y = (s.vel.y - p.gravity * h).max(-p.max_down_velocity);
            } else if controllable {
                let mut body = s.body();
                s.ctl.air_step(&mut body, &pad, h, 1.0);
                s.set_body(&body);
            } else if s.constant_velocity <= 0.0 {
                s.vel.y -= p.gravity * h;
            }
        }

        // Air actions: homing attack / air dash on a second jump press.
        if first && input.jump_pressed && controllable && s.air_action && s.homing.is_none() {
            s.air_action = false;
            s.jumping = true;
            let look = s.facing.with_y(0.0).normalize_or(Vec3::Z);
            let best = targets
                .iter()
                .filter_map(|(e, t)| {
                    let to = t.translation() - s.center;
                    let dist = to.length();
                    let ahead = to.with_y(0.0).normalize_or_zero().dot(look);
                    (dist < HOMING_RANGE && ahead > 0.3 && to.y < 6.0).then_some((e, dist))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1));
            match best {
                Some((entity, _)) => {
                    s.homing = Some(entity);
                    s.ctl.jump = None;
                }
                None => {
                    let flat = s.vel.with_y(0.0).length().max(30.0);
                    s.vel = look * flat + Vec3::Y * 2.0;
                }
            }
        }
        if first && input.stomp_pressed && controllable {
            s.stomping = true;
            s.homing = None;
            s.vel = Vec3::NEG_Y * p.stomping_down_first_velocity;
        }
        s.up = s.up.slerp(Vec3::Y, (8.0 * h).min(1.0)).normalize_or(Vec3::Y);
    }

    let mut motion = s.vel * h;
    if s.quickstep > 0.0 {
        motion += s.quickstep_dir * QUICKSTEP_SPEED * h;
    }
    move_and_slide(s, world, motion);
    if s.grounded {
        snap_to_ground(s, world, h);
    }
    lock_to_path(s, paths, h);
    let (center, _) = world.depenetrate(s.center, RADIUS);
    s.center = center;
}

/// 2D sections: pull Sonic onto the path midline (eased in over m_PathEaseTime)
/// and cancel sideways velocity; leaving either end of the path ends 2D.
fn lock_to_path(s: &mut Sonic, paths: &StagePaths, h: f32) {
    let Some(mut lock) = s.path_lock else { return };
    let Some(path) = paths.side.get(lock.path) else {
        s.exit_2d();
        return;
    };
    lock.index = path.nearest(s.center, lock.index);
    lock.ease = (lock.ease + h / lock.ease_time).min(1.0);
    let (center, tangent, side) = path.frame(lock.index);
    let past_end = (lock.index + 1 >= path.center.len() && (s.center - center).dot(tangent) > 0.0)
        || (lock.index == 0 && (s.center - center).dot(tangent) < -1.0);
    if past_end {
        s.exit_2d();
        return;
    }
    let lateral = (s.center - center).dot(side);
    s.center -= side * lateral * lock.ease;
    s.vel -= side * s.vel.dot(side) * lock.ease;
    s.path_lock = Some(lock);
}

fn land(s: &mut Sonic, normal: Vec3) {
    s.grounded = true;
    s.jumping = false;
    s.stomping = false;
    s.homing = None;
    s.air_action = false;
    s.constant_velocity = 0.0;
    s.up = normal;
    s.vel = project(s.vel, normal);
    let body = s.body();
    s.ctl.land(&body);
}

fn move_and_slide(s: &mut Sonic, world: &CollisionWorld, motion: Vec3) {
    let mut remaining = motion;
    for _ in 0..5 {
        if remaining.length_squared() < 1e-10 {
            return;
        }
        let Some(hit) = world.sweep(s.center, RADIUS, remaining) else {
            s.center += remaining;
            return;
        };
        s.center += remaining * hit.fraction;
        remaining *= 1.0 - hit.fraction;
        let n = hit.normal;
        s.center += n * SKIN;

        if s.grounded && n.dot(s.up) > 0.45 && can_stand(n, s.up, s.vel.length()) {
            // Running onto a new slope (ramps, quarter pipes, loops): carry
            // the momentum onto the new surface.
            let speed = s.vel.length();
            let len = remaining.length();
            s.up = n;
            s.vel = project(s.vel, n).normalize_or_zero() * speed;
            remaining = project(remaining, n).normalize_or_zero() * len;
            continue;
        }
        if !s.grounded && n.y > GROUND_MIN_Y && s.vel.dot(n) <= 0.0 {
            land(s, n);
            remaining = project(remaining, n);
            continue;
        }
        if s.homing.is_some() {
            s.homing = None;
        }
        // Curbs and stairs: lift by a step and retry the move (StairsMax in
        // the retail parameters; provisional until that routine is ported).
        if s.grounded && try_step_up(s, world, remaining) {
            return;
        }
        // Wall or ceiling: slide along it.
        let into = s.vel.dot(n);
        if into < 0.0 {
            s.vel -= n * into;
        }
        let into = remaining.dot(n);
        if into < 0.0 {
            remaining -= n * into;
        }
    }
}

const STEP_HEIGHT: f32 = 0.45;

fn try_step_up(s: &mut Sonic, world: &CollisionWorld, remaining: Vec3) -> bool {
    let along = project(remaining, s.up);
    if along.length_squared() < 1e-8 {
        return false;
    }
    let lift = s.up * STEP_HEIGHT;
    if world.sweep(s.center, RADIUS, lift).is_some() {
        return false;
    }
    let raised = s.center + lift;
    let probe = along.normalize() * along.length().max(0.05);
    if world.sweep(raised, RADIUS, probe).is_some() {
        return false;
    }
    // Settle onto the step top.
    let landed = raised + probe;
    match world.sweep(landed, RADIUS, -lift * 1.05) {
        Some(hit) if can_stand(hit.normal, s.up, s.vel.length()) => {
            s.center = landed - lift * 1.05 * hit.fraction + hit.normal * SKIN;
            true
        }
        _ => false,
    }
}

fn snap_to_ground(s: &mut Sonic, world: &CollisionWorld, h: f32) {
    let speed = s.vel.length();
    let reach = 0.3 + speed * h * 0.75;
    match world.sweep(s.center, RADIUS, -s.up * reach) {
        Some(hit) if hit.normal.dot(s.up) > 0.35 && can_stand(hit.normal, s.up, speed) => {
            s.center += -s.up * reach * hit.fraction + hit.normal * SKIN;
            s.up = hit.normal;
            s.vel = project(s.vel, s.up).normalize_or_zero() * speed;
        }
        _ => {
            // Ran off an edge: keep momentum, start falling.
            s.grounded = false;
        }
    }
    // Too slow to stick to walls and ceilings.
    if s.grounded && s.up.y < GROUND_MIN_Y && speed < WALL_RUN_SPEED * 0.7 {
        s.grounded = false;
        s.center += s.up * 0.05;
    }
}

fn animate_visual(
    time: Res<Time>,
    mut sonic: Query<&mut Sonic>,
    mut body: Query<(&mut Transform, &mut Visibility), (With<SonicBody>, Without<SpinBall>, Without<BoostAura>)>,
    mut ball: Query<(&mut Transform, &mut Visibility), (With<SpinBall>, Without<SonicBody>, Without<BoostAura>)>,
    mut aura: Query<(&mut Transform, &mut Visibility), (With<BoostAura>, Without<SonicBody>, Without<SpinBall>)>,
) {
    let Ok(mut s) = sonic.single_mut() else { return };
    let dt = time.delta_secs();
    let target_up = s.up;
    s.visual_up = s.visual_up.slerp(target_up, (18.0 * dt).min(1.0)).normalize_or(Vec3::Y);
    let up = s.visual_up;
    let forward = project(s.facing, up).normalize_or(Vec3::Z);
    let right = up.cross(forward).normalize_or(Vec3::X);
    let rotation = Quat::from_mat3(&Mat3::from_cols(right, up, forward.normalize()));
    let guest = s.guest_pose.map(|(_, r)| r);
    let rotation = guest.unwrap_or(rotation);
    let rolling = (s.jumping || s.homing.is_some()) && !s.grounded;
    s.spin += dt * (20.0 + s.speed() * 0.6);
    let dead = s.dead > 0.0;

    if let Ok((mut t, mut vis)) = body.single_mut() {
        t.rotation = rotation;
        // Guest mode: the parent sits at the game's body position (his feet) already.
        t.translation = if guest.is_some() { Vec3::ZERO } else { -up * RADIUS };
        *vis = if rolling || dead { Visibility::Hidden } else { Visibility::Inherited };
    }
    if let Ok((mut t, mut vis)) = ball.single_mut() {
        t.rotation = rotation * Quat::from_rotation_x(s.spin);
        t.translation = Vec3::ZERO;
        t.scale = Vec3::splat(1.0);
        *vis = if rolling && !dead { Visibility::Inherited } else { Visibility::Hidden };
    }
    if let Ok((mut t, mut vis)) = aura.single_mut() {
        t.rotation = rotation;
        t.translation = Vec3::ZERO;
        *vis = if s.boosting && !dead { Visibility::Inherited } else { Visibility::Hidden };
    }
}
