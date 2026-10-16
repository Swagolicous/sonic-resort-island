//! Third-person chase camera: follows Sonic's heading at speed, free look on
//! mouse / right stick, pulls in when terrain blocks the view.

use bevy::{
    prelude::*,
};

use crate::{
    hedgehog::SceneLighting,
    physics::CollisionWorld,
    post::GamePost,
    player::{PlayerInput, PlayerSet, Sonic},
};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn_camera)
            .add_systems(Update, follow.after(PlayerSet));
    }
}

#[derive(Component)]
pub struct ChaseCam {
    yaw: f32,
    pitch: f32,
    distance: f32,
    idle: f32,
    fov: f32,
}

/// The original guest camera, with the level's framing distance. The game keeps
/// owning camera motion and pad steering; this is shared by all rendered meshes.
pub fn guest_view(g: &crate::guest::GuestPlayer,
    resort: Option<&crate::resort::ResortSettings>, world: &CollisionWorld)
    -> Option<unleashed_guest_host::Camera> {
    let mut c = if resort.is_some() { g.rendered_view() } else { g.sonic.game_view().or_else(|| g.sonic.camera()) }?;
    if let Some(r) = resort {
        let delta = Vec3::from(c.eye) - Vec3::from(c.target);
        let target = Vec3::from(c.target) + Vec3::Y * r.camera_focus_offset;
        let back = delta.normalize_or(Vec3::Z);
        let right = Vec3::Y.cross(back).normalize_or(Vec3::X);
        let back = Quat::from_axis_angle(right, r.camera_pitch_bias) * back;
        let mut distance = delta.length() * r.camera_distance_factor;
        if let Some((hit, _)) = world.ray(target, back, distance + 0.2) {
            distance = distance.min((hit - 0.2).max(0.6));
        }
        c.eye = (target + back * distance).to_array();
        c.target = target.to_array();
    }
    Some(c)
}

fn spawn_camera(
    mut commands: Commands,
    sonic: Query<&Sonic>,
    scene: Res<SceneLighting>,
    minecraft: Option<Res<crate::minecraft::MinecraftMode>>,
    resort: Option<Res<crate::resort::ResortSettings>>,
) {
    let (yaw, at) = sonic
        .single()
        .map(|s| (s.facing.x.atan2(s.facing.z), s.center))
        .unwrap_or((0.0, Vec3::ZERO));
    if minecraft.is_some() || resort.is_some() {
        let (pitch, distance, fov) = (-0.2, 6.5, 60f32.to_radians());
        // Minecraft's textures and light are display colours: no game HDR chain.
        commands.spawn((
            Name::new("Chase camera"),
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection { fov, near: 0.1, far: 4000.0, ..default() }),
            Transform::from_translation(at + Vec3::new(-yaw.sin() * distance, 2.0, -yaw.cos() * distance)),
            ChaseCam { yaw, pitch, distance, idle: 10.0, fov },
            bevy::camera::Exposure { ev100: resort.as_ref().map(|r| r.exposure).unwrap_or(9.7) },
            bevy::core_pipeline::tonemapping::Tonemapping::None,
        ));
        return;
    }
    commands.spawn((
        Name::new("Chase camera"),
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 60f32.to_radians(),
            near: 0.1,
            ..default()
        }),
        Transform::from_translation(at + Vec3::new(0.0, 3.0, 0.0)),
        ChaseCam {
            yaw,
            pitch: -0.2,
            distance: 6.5,
            idle: 10.0,
            fov: 60f32.to_radians(),
        },
        // The game's own HDR chain (post.rs) replaces Bevy's fog, bloom and tonemapper;
        // light scattering is done in the surface shaders.
        bevy::camera::Hdr,
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        GamePost::from_scene(&scene),
    ));
}

fn wrap_angle(a: f32) -> f32 {
    (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

fn follow(
    time: Res<Time>,
    input: Res<PlayerInput>,
    world: Res<CollisionWorld>,
    sonic: Query<&Sonic>,
    mut camera: Query<(&mut Transform, &mut ChaseCam, &mut Projection)>,
    guest: Option<NonSend<crate::guest::GuestPlayer>>,
    resort: Option<Res<crate::resort::ResortSettings>>,
) {
    let Ok(s) = sonic.single() else { return };
    let Ok((mut transform, mut cam, mut projection)) = camera.single_mut() else { return };
    // Guest mode: the game's own player camera (CPlayer3DNormalCamera3) places the view.
    if let Some(c) = guest.as_ref().and_then(|g| guest_view(g, resort.as_deref(), &world)) {
        *transform = Transform::from_translation(Vec3::from(c.eye)).looking_at(Vec3::from(c.target), Vec3::Y);
        if let Projection::Perspective(p) = projection.as_mut() {
            p.fov = c.fov_y;
        }
        if (time.elapsed_secs() * 2.0) as u32 != ((time.elapsed_secs() - time.delta_secs()) * 2.0) as u32 {
            let fwd = transform.forward();
            info!("guest camera: frame {:.1} ms; eye {:?} target {:?} fov {:.3} forward {:?} sonic {:?} pose {:?}", time.delta_secs() * 1000.0,
                c.eye, c.target, c.fov_y, fwd, s.center, s.guest_pose.map(|p| p.0));
        }
        return;
    }
    let dt = time.delta_secs().min(0.1);

    if input.look.length_squared() > 1e-8 {
        cam.yaw -= input.look.x;
        cam.pitch = (cam.pitch - input.look.y).clamp(-1.2, 0.7);
        cam.idle = 0.0;
    } else {
        cam.idle += dt;
    }

    let flat = s.vel.with_y(0.0);
    let speed = s.speed();
    if cam.idle > 0.8 && flat.length() > 4.0 && s.dead <= 0.0 {
        let desired = flat.x.atan2(flat.z);
        let rate = (1.2 + flat.length() * 0.05) * dt;
        cam.yaw += wrap_angle(desired - cam.yaw) * rate.min(1.0);
        cam.pitch += (-0.22 - cam.pitch) * (1.5 * dt).min(1.0);
    }

    let target_distance = 6.5 + (speed * 0.035).min(3.5);
    cam.distance += (target_distance - cam.distance) * (3.0 * dt).min(1.0);
    let focus = s.center + Vec3::Y * 0.9;
    let forward = Vec3::new(
        cam.yaw.sin() * cam.pitch.cos(),
        cam.pitch.sin(),
        cam.yaw.cos() * cam.pitch.cos(),
    );
    let back = -forward;
    let mut distance = cam.distance;
    if let Some((hit, _)) = world.ray(focus, back, distance + 0.3) {
        distance = (hit - 0.3).max(0.6);
    }
    let desired = focus + back * distance;
    // Smooth position slightly; snap when far (respawn, teleports).
    let current = transform.translation;
    let next = if current.distance(desired) > 40.0 {
        desired
    } else {
        current.lerp(desired, (18.0 * dt).min(1.0))
    };
    *transform = Transform::from_translation(next).looking_at(focus, Vec3::Y);

    let target_fov = if s.boosting { 75.0 } else { 60.0 + (speed / 85.0).min(1.0) * 6.0 }.to_radians();
    cam.fov += (target_fov - cam.fov) * (4.0 * dt).min(1.0);
    if let Projection::Perspective(p) = projection.as_mut() {
        p.fov = cam.fov;
    }
}
