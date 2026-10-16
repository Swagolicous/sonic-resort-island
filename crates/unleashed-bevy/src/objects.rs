//! Set objects from the stage's `.set.xml` layers: rings, dash panels,
//! springs, jump boards, the up-reel, checkpoints, enemies, breakables and
//! the goal ring. Models are the originals from the common object archives.

use std::collections::HashMap;

use bevy::prelude::*;
use unleashed_assets::{mirage::mesh::parse_model, stage::day::SetObject};

use crate::{
    StageScoring,
    paths::StagePaths,
    Run, Stage,
    convert::{self, Shading},
    player::{HomingTarget, PlayerSet, RADIUS, Sonic},
    world::Converted,
};

pub struct ObjectsPlugin;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectSet;

impl Plugin for ObjectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_objects)
            .add_systems(
                Update,
                (spin, interact, restart_after_goal)
                    .chain()
                    .in_set(ObjectSet)
                    .after(PlayerSet),
            );
    }
}

#[derive(Clone, Debug)]
pub enum Kind {
    Ring,
    SuperRing,
    DashPanel { speed: f32, out_of_control: f32 },
    Spring { speed: f32, out_of_control: f32, keep: f32 },
    JumpBoard { normal: f32, boost: f32, out_of_control: f32, pitch: f32 },
    /// `position` is the pulley; the handle hangs `length` below it.
    UpReel { speed: f32, length: f32 },
    Goal,
    PointMarker { width: f32 },
    Enemy,
    Breakable,
    /// Invisible ledge launcher.
    JumpCollision { normal: f32, boost: f32, out_of_control: f32, pitch: f32, speed_min: f32 },
    AutorunStart { speed: f32 },
    AutorunFinish,
    /// ChangeMode_3Dto2D (`to_2d`) / ChangeMode_2Dto3D / ChangeMode_3DtoForward.
    ChangeMode { to_2d: bool, ease: f32 },
}

#[derive(Component)]
pub struct StageObject {
    pub kind: Kind,
    pub position: Vec3,
    pub rotation: Quat,
    pub radius: f32,
    pub active: bool,
    pub cooldown: f32,
    /// Oriented trigger box half extents (local space) for volume objects.
    pub volume: Option<Vec3>,
    pub initial: (Vec3, Quat),
}

/// Original object model for each set object type.
fn model_for(object: &SetObject) -> Option<String> {
    Some(
        match object.kind.as_str() {
            "Ring" => "cmn_obj_ms_ringnormalM_000",
            "SuperRing" => "cmn_obj_ms_superringM_000",
            "GoalRing" => "cmn_obj_km_goalringM_000",
            "Spring" => "cmn_obj_kt3_springredM_000",
            "DashPanel" => "cmn_obj_ms_dashpanelM_000",
            "JumpBoard" => {
                if object.f32("AngleType").unwrap_or(15.0) >= 30.0 {
                    "cmn_obj_ms_jumpboard30"
                } else {
                    "cmn_obj_ms_jumpboard15"
                }
            }
            "JumpBoard3D" => {
                if object.f32("SizeType").unwrap_or(0.0) >= 1.0 {
                    "cmn_obj_ms_jumppanelL2_000"
                } else {
                    "cmn_obj_ms_jumppanelM2_000"
                }
            }
            "PointMarker" => "cmn_obj_km_pointmarker_L_M_000",
            "HintRing" => "cmn_obj_sk2_hintring",
            "ObjectPhysics" => return object.params.get("Type").cloned(),
            _ => return None,
        }
        .to_owned(),
    )
}

type Parts = Vec<(Handle<Mesh>, Handle<StandardMaterial>)>;

fn spawn_objects(
    mut commands: Commands,
    stage: Res<Stage>,
    mut converted: ResMut<Converted>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let stage = &stage.0;
    let mut models: HashMap<String, Option<Parts>> = HashMap::new();
    let mut load = |name: &str,
                    meshes: &mut Assets<Mesh>,
                    materials: &mut Assets<StandardMaterial>,
                    images: &mut Assets<Image>|
     -> Option<Parts> {
        models
            .entry(name.to_owned())
            .or_insert_with(|| {
                let bytes = [format!("{name}.model"), format!("{name}_000.model")]
                    .iter()
                    .find_map(|f| stage.db.get(f))?;
                let model = parse_model(bytes)
                    .inspect_err(|e| warn!("object model {name}: {e:#}"))
                    .ok()?;
                Some(
                    model
                        .submeshes
                        .iter()
                        .map(|sub| {
                            let mat = convert::material(
                                &stage.db,
                                &mut converted.0,
                                images,
                                materials,
                                &sub.material,
                                sub.slot.as_ref(),
                                Shading::Lit,
                            );
                            (meshes.add(convert::submesh_mesh(sub, false)), mat)
                        })
                        .collect(),
                )
            })
            .clone()
    };

    let fallback_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.2, 0.15),
        perceptual_roughness: 0.5,
        ..default()
    });
    let reel_mesh = meshes.add(Cylinder::new(0.12, 1.0));
    let enemy_mesh = meshes.add(Capsule3d::new(0.6, 0.9));

    let mut counts: HashMap<&str, usize> = HashMap::new();
    for object in &stage.objects {
        let position = Vec3::from(object.position);
        let rotation = Quat::from_array(object.rotation).normalize();
        let kind = match object.kind.as_str() {
            "Ring" => Kind::Ring,
            "SuperRing" => Kind::SuperRing,
            "DashPanel" => Kind::DashPanel {
                speed: object.f32("Speed").unwrap_or(60.0),
                out_of_control: object.f32("OutOfControl").unwrap_or(0.25),
            },
            "Spring" => Kind::Spring {
                speed: object.f32("FirstSpeed").unwrap_or(25.0),
                out_of_control: object.f32("OutOfControl").unwrap_or(0.5),
                keep: object.f32("KeepVelocityDistance").unwrap_or(0.0),
            },
            "JumpBoard" => Kind::JumpBoard {
                normal: object.f32("ImpulseSpeedOnNormal").unwrap_or(40.0),
                boost: object.f32("ImpulseSpeedOnBoost").unwrap_or(40.0),
                out_of_control: object.f32("OutOfControl").unwrap_or(0.6),
                pitch: object.f32("AngleType").unwrap_or(15.0),
            },
            "JumpBoard3D" => Kind::JumpBoard {
                normal: object.f32("ImpulseSpeedOnNormal").unwrap_or(40.0),
                boost: object.f32("ImpulseSpeedOnBoost").unwrap_or(40.0),
                out_of_control: object.f32("OutOfControl").unwrap_or(0.8),
                pitch: 20.0,
            },
            "UpReel" => {
                let length = object.f32("Length").unwrap_or(6.0);
                let impulse = object.f32("ImpulseVelocity").unwrap_or(15.0);
                Kind::UpReel {
                    speed: (2.0 * 38.0 * length + impulse * impulse).sqrt(),
                    length,
                }
            }
            "GoalRing" => Kind::Goal,
            "PointMarker" => Kind::PointMarker {
                width: object.f32("Width").unwrap_or(2.5),
            },
            "JumpCollision" => Kind::JumpCollision {
                normal: object.f32("ImpulseSpeedOnNormal").unwrap_or(40.0),
                boost: object.f32("ImpulseSpeedOnBoost").unwrap_or(40.0),
                out_of_control: object.f32("OutOfControl").unwrap_or(1.0),
                pitch: object.f32("Pitch").unwrap_or(0.0),
                speed_min: object.f32("SpeedMin").unwrap_or(0.0),
            },
            "AutorunStartCollision" => Kind::AutorunStart {
                speed: object.f32("Speed").unwrap_or(60.0),
            },
            "AutorunFinishCollision" => Kind::AutorunFinish,
            "ChangeMode_3Dto2D" => Kind::ChangeMode { to_2d: true, ease: object.f32("m_PathEaseTime").unwrap_or(1.0) },
            "ChangeMode_2Dto3D" | "ChangeMode_3DtoForward" => Kind::ChangeMode { to_2d: false, ease: 0.0 },
            "eFighterTutorial" | "eAirCannonNormal" | "eFighterNormal" => Kind::Enemy,
            "ObjectPhysics" => Kind::Breakable,
            "HintRing" => {
                // Decorative only.
                if let Some(parts) = model_for(object).and_then(|m| load(&m, &mut meshes, &mut materials, &mut images)) {
                    spawn_parts(&mut commands, &parts, Transform::from_translation(position).with_rotation(rotation));
                }
                continue;
            }
            _ => continue,
        };
        *counts.entry(object.kind.as_str()).or_default() += 1;
        let radius = match kind {
            Kind::Ring => 1.2,
            Kind::SuperRing => 1.6,
            Kind::DashPanel { .. } => 1.4,
            Kind::Spring { .. } => 1.3,
            Kind::JumpBoard { .. } => 2.2,
            Kind::UpReel { .. } => 1.4,
            Kind::Goal => 4.0,
            Kind::PointMarker { width } => width + 1.0,
            Kind::Enemy => 1.3,
            Kind::Breakable => 1.1,
            Kind::JumpCollision { .. } | Kind::AutorunStart { .. } | Kind::AutorunFinish | Kind::ChangeMode { .. } => 0.0,
        };
        let volume = match kind {
            Kind::JumpCollision { .. }
            | Kind::AutorunStart { .. }
            | Kind::AutorunFinish
            | Kind::ChangeMode { .. } => Some(Vec3::new(
                object.f32("Collision_Width").unwrap_or(10.0) * 0.5,
                object.f32("Collision_Height").unwrap_or(10.0) * 0.5,
                1.5,
            )),
            _ => None,
        };
        let transform = Transform::from_translation(position).with_rotation(rotation);
        let mut entity = commands.spawn((
            Name::new(format!("{} #{}", object.kind, object.id)),
            StageObject {
                kind: kind.clone(),
                position,
                rotation,
                radius,
                active: true,
                cooldown: 0.0,
                volume,
                initial: (position, rotation),
            },
            transform,
            Visibility::default(),
        ));
        if matches!(kind, Kind::Enemy | Kind::Spring { .. }) {
            entity.insert(HomingTarget);
        }
        let parts = model_for(object).and_then(|m| load(&m, &mut meshes, &mut materials, &mut images));
        entity.with_children(|parent| match (&kind, parts) {
            (_, Some(parts)) => {
                for (mesh, material) in parts {
                    parent.spawn((Mesh3d(mesh), MeshMaterial3d(material)));
                }
            }
            (Kind::UpReel { .. }, None) => {
                let length = object.f32("Length").unwrap_or(6.0);
                parent.spawn((
                    Mesh3d(reel_mesh.clone()),
                    MeshMaterial3d(fallback_material.clone()),
                    Transform::from_xyz(0.0, -length * 0.5, 0.0).with_scale(Vec3::new(1.0, length, 1.0)),
                ));
            }
            (Kind::Enemy, None) => {
                parent.spawn((
                    Mesh3d(enemy_mesh.clone()),
                    MeshMaterial3d(fallback_material.clone()),
                    Transform::from_xyz(0.0, 1.05, 0.0),
                ));
            }
            _ => {}
        });
    }
    info!("objects: {counts:?}");
}

fn spawn_parts(commands: &mut Commands, parts: &Parts, transform: Transform) {
    commands
        .spawn((transform, Visibility::default()))
        .with_children(|parent| {
            for (mesh, material) in parts {
                parent.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())));
            }
        });
}

fn spin(time: Res<Time>, mut objects: Query<(&StageObject, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (object, mut transform) in &mut objects {
        if matches!(object.kind, Kind::Ring | Kind::SuperRing) {
            transform.rotation = object.rotation * Quat::from_rotation_y(t * 4.0);
        }
    }
}

fn interact(
    time: Res<Time>,
    mut run: ResMut<Run>,
    paths: Res<StagePaths>,
    scoring: Res<StageScoring>,
    mut sonic: Query<&mut Sonic>,
    mut objects: Query<(&mut StageObject, &mut Visibility)>,
) {
    let Ok(mut s) = sonic.single_mut() else { return };
    let dt = time.delta_secs();
    if s.dead > 0.0 {
        return;
    }
    let attacking = s.jumping || s.homing.is_some() || s.boosting || s.stomping;
    for (mut object, mut visibility) in &mut objects {
        object.cooldown = (object.cooldown - dt).max(0.0);
        if !object.active || object.cooldown > 0.0 {
            continue;
        }
        let up = object.rotation * Vec3::Y;
        let forward = object.rotation * Vec3::Z;
        let focus = match object.kind {
            Kind::Spring { .. } => object.position + up * 0.6,
            Kind::Enemy => object.position + Vec3::Y * 1.0,
            Kind::Goal => object.position + Vec3::Y * 1.5,
            Kind::UpReel { length, .. } => object.position - up * length,
            _ => object.position,
        };
        let to_sonic = s.center - focus;
        let close = match object.kind {
            _ if object.volume.is_some() => {
                let local = object.rotation.inverse() * to_sonic;
                local.abs().cmple(object.volume.unwrap()).all()
            }
            Kind::DashPanel { .. } | Kind::JumpBoard { .. } | Kind::PointMarker { .. } => {
                let along_up = to_sonic.dot(up);
                (to_sonic - up * along_up).length() < object.radius && along_up.abs() < 2.0
            }
            _ => to_sonic.length() < object.radius + RADIUS * 0.5,
        };
        if !close {
            continue;
        }
        match object.kind.clone() {
            Kind::Ring => {
                run.rings += 1;
                let speed = s.speed();
                s.ctl.collect_rings(1, speed);
                object.active = false;
                *visibility = Visibility::Hidden;
            }
            Kind::SuperRing => {
                run.rings += 10;
                let speed = s.speed();
                s.ctl.collect_rings(10, speed);
                object.active = false;
                *visibility = Visibility::Hidden;
            }
            Kind::DashPanel {
                speed,
                out_of_control,
            } => {
                let dir = (forward - s.up * forward.dot(s.up)).normalize_or(forward);
                let keep = s.speed().max(speed);
                s.launch(dir * keep, out_of_control, 0.0, false);
                s.facing = dir;
                object.cooldown = 0.4;
            }
            Kind::Spring {
                speed,
                out_of_control,
                keep,
            } => {
                s.center = object.position + up * (RADIUS + 0.6);
                s.launch(up * speed, out_of_control, keep, true);
                if up.with_y(0.0).length() > 0.2 {
                    s.facing = up.with_y(0.0).normalize();
                }
                object.cooldown = 0.5;
            }
            Kind::JumpBoard {
                normal,
                boost,
                out_of_control,
                pitch,
            } => {
                // Jump boards launch off their local -Z edge.
                let forward = -forward;
                let speed = if s.boosting { boost } else { normal };
                let pitch = pitch.to_radians();
                let dir = (forward * pitch.cos() + Vec3::Y * pitch.sin()).normalize();
                s.launch(dir * speed, out_of_control, 0.0, true);
                s.facing = forward.with_y(0.0).normalize_or(s.facing);
                object.cooldown = 0.6;
            }
            Kind::UpReel { speed, length } => {
                s.center = object.position - up * (length - RADIUS);
                s.launch(up * speed, 0.6, 0.0, true);
                object.cooldown = 1.0;
            }
            Kind::Goal => {
                let (speed, boosting, side) = (s.speed(), s.boosting, s.path_lock.is_some());
                run.score.set_last_speed(&scoring.params, speed, boosting, side);
                run.cleared = true;
                object.active = false;
            }
            Kind::PointMarker { .. } => {
                // Speed bonus samples are taken at checkpoints (sub_8245EDF0).
                let (speed, boosting, side) = (s.speed(), s.boosting, s.path_lock.is_some());
                run.score.record_checkpoint_speed(&scoring.params, &scoring.stage, speed, boosting, side);
                let facing = forward.with_y(0.0).normalize_or(s.facing);
                // Face along the player's travel direction through the gate.
                let facing = if facing.dot(s.vel) < 0.0 { -facing } else { facing };
                s.checkpoint = (object.position, facing);
                object.active = false;
            }
            Kind::JumpCollision {
                normal,
                boost,
                out_of_control,
                pitch,
                speed_min,
            } => {
                if !s.grounded || s.speed() < speed_min {
                    continue;
                }
                let speed = if s.boosting { boost } else { normal };
                let dir = object.rotation * (Quat::from_rotation_x(-pitch.to_radians()) * Vec3::Z);
                s.launch(dir * speed, out_of_control, 0.0, true);
                s.facing = dir.with_y(0.0).normalize_or(s.facing);
                s.autorun = None;
                object.cooldown = 1.0;
            }
            Kind::AutorunStart { speed } => {
                s.autorun = Some(speed);
                object.cooldown = 1.0;
            }
            Kind::AutorunFinish => {
                s.autorun = None;
                object.cooldown = 1.0;
            }
            Kind::ChangeMode { to_2d: true, ease } => {
                if s.path_lock.is_none() {
                    if let Some((path, index)) = paths.nearest_side(s.center, 30.0) {
                        s.enter_2d(path, index, ease);
                    }
                }
                object.cooldown = 1.0;
            }
            Kind::ChangeMode { to_2d: false, .. } => {
                s.exit_2d();
                object.cooldown = 1.0;
            }
            Kind::Enemy | Kind::Breakable => {
                if attacking || s.invulnerable > 0.0 {
                    object.active = false;
                    *visibility = Visibility::Hidden;
                    if matches!(object.kind, Kind::Enemy) {
                        run.enemies += 1;
                        // Provisional per-enemy score: the game reads it from the
                        // enemy object (+0x240), not traced to its data yet.
                        run.score.add_enemy(&scoring.params, 100, false);
                    }
                    // Provisional: enemy energy reward not located in the decompile yet.
                    s.ctl.add_energy(4.0);
                    if s.homing.is_some() || (s.jumping && !s.grounded) {
                        s.homing = None;
                        let bounce = s.ctl.params.homing_attack_after_up_velocity;
                        s.vel = s.vel.with_y(0.0) * 0.2 + Vec3::Y * bounce;
                        s.ctl.jump = None;
                        s.air_action = true;
                    }
                } else if matches!(object.kind, Kind::Enemy) {
                    // Hit: scatter rings, knock back.
                    if run.rings == 0 {
                        s.dead = 1.2;
                        run.deaths += 1;
                    } else {
                        run.rings = 0;
                        let away = to_sonic.with_y(0.0).normalize_or(-s.facing);
                        s.launch(away * 10.0 + Vec3::Y * 9.0, 0.7, 0.0, true);
                        s.invulnerable = 1.5;
                    }
                }
            }
        }
    }
}

fn restart_after_goal(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut run: ResMut<Run>,
    stage: Res<Stage>,
    mut sonic: Query<&mut Sonic>,
    mut objects: Query<(&mut StageObject, &mut Visibility)>,
) {
    let pressed = keys.just_pressed(KeyCode::Enter)
        || gamepads.iter().any(|g| g.just_pressed(GamepadButton::Start));
    if !(run.cleared && pressed) {
        return;
    }
    *run = Run::default();
    for (mut object, mut visibility) in &mut objects {
        object.active = true;
        (object.position, object.rotation) = object.initial;
        *visibility = Visibility::Inherited;
    }
    if let Ok(mut s) = sonic.single_mut() {
        let spawn = Vec3::from(stage.0.spawn.position);
        let facing = Quat::from_rotation_y(stage.0.spawn.yaw_degrees.to_radians()) * Vec3::Z;
        s.checkpoint = (spawn, facing);
        s.respawn();
        let full = s.ctl.energy_max();
        s.ctl.energy = full;
    }
}
