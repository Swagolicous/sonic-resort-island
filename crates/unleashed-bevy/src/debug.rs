//! Automation hooks for verifying the port without a human at the controls.
//!
//! ```text
//! UNLEASHED_SHOTS=<dir>        save a screenshot every UNLEASHED_SHOT_EVERY seconds (default 3)
//! UNLEASHED_AUTOPILOT=<script> comma list of `seconds:actions`, actions from
//!                              f(orward) b(oost) j(ump, pressed once) l/r (steer) s(tomp), R(respawn)
//!                              g (follow the stage guide path, jumping when stuck)
//!                              e.g. "1:f,3:fb,4:fbj,9:f"
//! UNLEASHED_EXIT_AFTER=<secs>  quit after N seconds
//! UNLEASHED_TELEMETRY=1        print position / speed / state every 0.5 s
//! UNLEASHED_START=x,y,z[,yaw]  start somewhere other than the stage spawn
//! ```

use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};

use crate::{
    Run,
    guide::GuidePath,
    objects::{Kind, StageObject},
    player::{self, PlayerInput, Sonic},
};

pub struct DebugPlugin;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ScriptInput;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        let config = DebugConfig {
            shots: env("UNLEASHED_SHOTS"),
            shot_every: env("UNLEASHED_SHOT_EVERY")
                .and_then(|v| v.parse().ok())
                .unwrap_or(3.0),
            exit_after: env("UNLEASHED_EXIT_AFTER").and_then(|v| v.parse().ok()),
            telemetry: env("UNLEASHED_TELEMETRY").is_some(),
            autopilot: env("UNLEASHED_AUTOPILOT").map(|s| parse_script(&s)).unwrap_or_default(),
            start: env("UNLEASHED_START").map(|s| {
                s.split(',')
                    .filter_map(|v| v.trim().parse::<f32>().ok())
                    .collect()
            }),
            next_shot: 1.5,
            next_log: 0.0,
            shot_index: 0,
            last_jump_step: usize::MAX,
            last_respawn_step: usize::MAX,
            guide_index: usize::MAX,
            best_index: usize::MAX,
            stuck: 0.0,
        };
        app.insert_resource(config)
            .add_systems(PostStartup, apply_start)
            .add_systems(
                Update,
                autopilot
                    .in_set(ScriptInput)
                    .after(player::read_input)
                    .before(crate::guest::step_guest)
                    .before(player::simulate),
            )
            .add_systems(Last, (telemetry, screenshots, exit_after));
    }
}

#[derive(Resource)]
struct DebugConfig {
    shots: Option<String>,
    shot_every: f32,
    exit_after: Option<f32>,
    telemetry: bool,
    autopilot: Vec<(f32, String)>,
    start: Option<Vec<f32>>,
    next_shot: f32,
    next_log: f32,
    shot_index: u32,
    last_jump_step: usize,
    last_respawn_step: usize,
    guide_index: usize,
    best_index: usize,
    stuck: f32,
}

fn parse_script(script: &str) -> Vec<(f32, String)> {
    script
        .split(',')
        .filter_map(|step| {
            let (t, actions) = step.split_once(':')?;
            Some((t.trim().parse().ok()?, actions.trim().to_owned()))
        })
        .collect()
}

fn apply_start(config: Res<DebugConfig>, mut sonic: Query<&mut Sonic>) {
    let Some(start) = &config.start else { return };
    let Ok(mut s) = sonic.single_mut() else { return };
    if start.len() >= 3 {
        let yaw = start.get(3).copied().unwrap_or(0.0).to_radians();
        s.checkpoint = (Vec3::new(start[0], start[1], start[2]), Quat::from_rotation_y(yaw) * Vec3::Z);
        s.respawn();
    }
}

fn autopilot(
    time: Res<Time>,
    guide: Res<GuidePath>,
    sonic: Query<&Sonic>,
    objects: Query<&StageObject>,
    mut config: ResMut<DebugConfig>,
    mut input: ResMut<PlayerInput>,
) {
    if config.autopilot.is_empty() {
        return;
    }
    let t = time.elapsed_secs();
    let Some(index) = config.autopilot.iter().rposition(|(start, _)| *start <= t) else {
        return;
    };
    let actions = config.autopilot[index].1.clone();
    // A scripted verification owns the pad, so live key presses do not alter it.
    input.jump_held = false;
    input.x_held = false;
    input.boost_held = false;
    input.stomp_held = false;
    input.y_held = false;
    let mut stick = Vec2::ZERO;
    for c in actions.chars() {
        match c {
            'f' => stick.y = 1.0,
            'l' => stick.x = -0.6,
            'r' => stick.x = 0.6,
            'b' => {
                input.boost_pressed |= !input.boost_held;
                input.boost_held = true;
                input.x_held = true;
            }
            'j' if config.last_jump_step != index => {
                input.jump_pressed = true;
                input.jump_held = true;
                config.last_jump_step = index;
            }
            'j' => input.jump_held = true,
            's' => input.stomp_pressed = true,
            'R' if config.last_respawn_step != index => {
                input.respawn = true;
                config.last_respawn_step = index;
            }
            'g' => {
                let Ok(s) = sonic.single() else { continue };
                if guide.points.is_empty() {
                    continue;
                }
                let i = guide.nearest(s.center, config.guide_index);
                config.guide_index = i;
                let mut ahead = guide.points[(i + 8).min(guide.points.len() - 1)];
                if let Some(goal) = objects
                    .iter()
                    .find(|o| matches!(o.kind, Kind::Goal) && o.position.distance(s.center) < 100.0)
                {
                    ahead = goal.position;
                }
                let to = (ahead - s.center).with_y(0.0).normalize_or_zero();
                // Enemy on the path ahead: boost through it like a player would.
                let enemy_ahead = objects.iter().any(|o| {
                    let d = o.position - s.center;
                    o.active
                        && matches!(o.kind, Kind::Enemy)
                        && d.length() < 18.0
                        && d.y.abs() < 3.0
                        && d.with_y(0.0).normalize_or_zero().dot(to) > 0.7
                });
                if enemy_ahead && s.grounded {
                    input.boost_pressed |= !input.boost_held;
                    input.boost_held = true;
                }
                stick = Vec2::new(to.dot(input.cam_right), to.dot(input.cam_forward));
                if stick.length() > 0.01 {
                    stick = stick.normalize();
                }
                // Stuck = no progress along the guide for a while: jump.
                if s.dead > 0.0 {
                    // Respawning at a checkpoint: progress restarts from there.
                    config.best_index = usize::MAX;
                    config.stuck = 0.0;
                } else if config.best_index == usize::MAX || i > config.best_index {
                    config.best_index = i;
                    config.stuck = 0.0;
                } else {
                    config.stuck += time.delta_secs();
                }
                // Stuck below something to home onto (floating springs, enemies on
                // ledges): get under it, jump, and home at the apex like a player.
                let overhead = (config.stuck > 0.5 && config.stuck < 3.0)
                    .then(|| {
                        objects
                            .iter()
                            .filter(|o| o.active && matches!(o.kind, Kind::Spring { .. } | Kind::Enemy))
                            .filter(|o| {
                                let d = o.position - s.center;
                                d.y > 1.5 && d.y < 8.0 && d.with_y(0.0).length() < 30.0
                            })
                            .min_by(|a, b| {
                                a.position.distance(s.center).total_cmp(&b.position.distance(s.center))
                            })
                    })
                    .flatten();
                if let Some(o) = overhead {
                    let to = (o.position - s.center).with_y(0.0);
                    let dir = to.normalize_or_zero();
                    stick = Vec2::new(dir.dot(input.cam_right), dir.dot(input.cam_forward)).normalize_or_zero();
                    if s.grounded {
                        // Under it, or blocked by a ledge on the way: jump.
                        if to.length() < 5.0 || s.speed() < 2.0 {
                            input.jump_pressed = true;
                            input.jump_held = true;
                        }
                    } else if s.vel.y > 1.0 {
                        input.jump_held = true;
                    } else if s.homing.is_none() {
                        input.jump_pressed = true;
                    }
                } else if config.stuck > 1.0 && s.grounded {
                    input.jump_pressed = true;
                    input.jump_held = true;
                    config.stuck = 0.4;
                } else if !s.grounded && s.vel.y > 0.0 {
                    input.jump_held = true;
                } else if config.stuck > 1.0 && !s.grounded && s.vel.y < 1.0 {
                    // Near the apex: homing attack / air dash.
                    input.jump_pressed = true;
                    config.stuck = 0.0;
                }
            }
            _ => {}
        }
    }
    input.stick = stick;
}

fn telemetry(
    time: Res<Time>,
    run: Res<Run>,
    mut config: ResMut<DebugConfig>,
    sonic: Query<&Sonic>,
) {
    if !config.telemetry || time.elapsed_secs() < config.next_log {
        return;
    }
    config.next_log = time.elapsed_secs() + 0.5;
    let Ok(s) = sonic.single() else { return };
    println!(
        "[t={:6.2}] guide={:4} pos=({:8.2},{:8.2},{:8.2}) speed={:5.1} up=({:.2},{:.2},{:.2}) grounded={} boost={} gauge={:.2} rings={} dead={} fps={:.0}",
        time.elapsed_secs(),
        config.guide_index as i64,
        s.center.x,
        s.center.y,
        s.center.z,
        s.speed(),
        s.up.x,
        s.up.y,
        s.up.z,
        s.grounded,
        s.boosting,
        s.boost_gauge(),
        run.rings,
        s.dead > 0.0,
        1.0 / time.delta_secs().max(1e-4),
    );
}

fn screenshots(mut commands: Commands, time: Res<Time>, mut config: ResMut<DebugConfig>) {
    let Some(dir) = config.shots.clone() else { return };
    if time.elapsed_secs() < config.next_shot {
        return;
    }
    config.next_shot = time.elapsed_secs() + config.shot_every;
    let path = format!("{dir}/shot_{:03}.png", config.shot_index);
    config.shot_index += 1;
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
}

fn exit_after(time: Res<Time>, config: Res<DebugConfig>, mut exit: MessageWriter<AppExit>) {
    if config.exit_after.is_some_and(|t| time.elapsed_secs() > t) {
        exit.write(AppExit::Success);
    }
}
