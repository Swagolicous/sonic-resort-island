//! `windmill-isle guest <82346F08_pre.bin>`: Sonic driven by the game's own code. The
//! lifted guest functions (crates/unleashed-guest-host) run at the game's 60 Hz on a
//! capture-build snapshot of ActD_MykonosAct1; this module only feeds them the pad and
//! puts Sonic's model where the game says he is.

use bevy::prelude::*;
use unleashed_guest_host::{button, GuestSonic, Pad};

use crate::player::{PlayerInput, PlayerSet, Sonic};

/// Marker: the hand-written controller (player::simulate) stands down.
#[derive(Resource)]
pub struct GuestMode;

/// The guest state (4 GiB of guest memory: not Send, lives on the main thread).
pub struct GuestPlayer {
    pub sonic: GuestSonic,
    acc: f64,
    reported: usize,
    /// Counts respawns (reloads of the snapshot): systems that keep things in the game's
    /// memory start over when it changes.
    pub generation: u32,
    presentation: Option<(RenderFrame, RenderFrame)>,
}

struct RenderFrame {
    body: Mat4,
    view: Option<unleashed_guest_host::Camera>,
    skin: Vec<[f32;16]>,
}

impl GuestPlayer {
    pub fn new(sonic: GuestSonic) -> Self {
        assert!(sonic.snapshot_dt().is_finite() && sonic.snapshot_dt() > 0.0,
            "guest snapshot must contain a positive frame duration");
        GuestPlayer { sonic, acc: 0.0, reported: 0, generation: 0, presentation: None }
    }

    pub fn reset_frame_clock(&mut self) {
        self.acc = 0.0;
        self.reported = 0;
        self.generation += 1;
        self.presentation = None;
    }

    fn record_render_frame(&mut self) {
        let frame = RenderFrame {
            body: Mat4::from_cols_array(&self.sonic.body_matrix()),
            view: self.sonic.game_view().or_else(|| self.sonic.camera()),
            skin: self.sonic.skin().map(|a| self.sonic.matrices(a,111)).unwrap_or_default(),
        };
        self.presentation = Some(match self.presentation.take() {
            Some((_, current)) => (current, frame),
            None => (RenderFrame { body: frame.body, view: frame.view, skin: frame.skin.clone() }, frame),
        });
    }

    fn render_alpha(&self) -> f32 { (self.acc / self.sonic.snapshot_dt()).clamp(0.0,1.0) as f32 }

    pub fn rendered_body(&self) -> [f32;16] {
        let Some((previous,current)) = &self.presentation else { return self.sonic.body_matrix(); };
        let (s0,r0,p0) = previous.body.to_scale_rotation_translation();
        let (s1,r1,p1) = current.body.to_scale_rotation_translation();
        if p0.distance(p1) > 5.0 { return current.body.to_cols_array(); }
        let a = self.render_alpha();
        Mat4::from_scale_rotation_translation(s0.lerp(s1,a),r0.slerp(r1,a),p0.lerp(p1,a)).to_cols_array()
    }

    pub fn rendered_view(&self) -> Option<unleashed_guest_host::Camera> {
        let Some((previous,current)) = &self.presentation else {
            return self.sonic.game_view().or_else(|| self.sonic.camera());
        };
        let (Some(p),Some(mut c)) = (previous.view,current.view) else { return current.view; };
        let a = self.render_alpha();
        c.eye = Vec3::from(p.eye).lerp(Vec3::from(c.eye),a).to_array();
        c.target = Vec3::from(p.target).lerp(Vec3::from(c.target),a).to_array();
        c.fov_y = p.fov_y + (c.fov_y-p.fov_y)*a;
        Some(c)
    }

    pub fn rendered_skin(&self, array: u32, count: usize) -> Vec<[f32;16]> {
        let Some((p,c)) = &self.presentation else { return self.sonic.matrices(array,count); };
        if p.skin.len()!=count || c.skin.len()!=count { return self.sonic.matrices(array,count); }
        let a = self.render_alpha();
        p.skin.iter().zip(&c.skin).map(|(p,c)| std::array::from_fn(|i| p[i]+(c[i]-p[i])*a)).collect()
    }
}

pub struct GuestPlugin;

impl Plugin for GuestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, step_guest.in_set(PlayerSet).after(crate::player::read_input).before(crate::player::PlayerVisualSet))
            .init_resource::<FrameMarks>()
            .add_systems(First, |mut m: ResMut<FrameMarks>| m.0[0] = Some(std::time::Instant::now()))
            .add_systems(PreUpdate, |mut m: ResMut<FrameMarks>| m.0[1] = Some(std::time::Instant::now()))
            .add_systems(PostUpdate, |mut m: ResMut<FrameMarks>| m.0[2] = Some(std::time::Instant::now()))
            .add_systems(Last, log_frame_time);
    }
}

pub(crate) fn step_guest(
    time: Res<Time>,
    input: Res<PlayerInput>,
    guest: Option<NonSendMut<GuestPlayer>>,
    mut sonic: Query<(&mut Sonic, &mut Transform)>,
    mut run: ResMut<crate::Run>,
    gamepads: Query<(Option<&Name>, &Gamepad)>,
    mut last_pad: Local<(String, u32)>,
    resort: Option<Res<crate::resort::ResortSettings>>,
) {
    let Some(mut guest) = guest else { return };
    let Ok((mut s, mut transform)) = sonic.single_mut() else { return };
    let mut down = 0;
    if input.jump_held {
        down |= button::A;
    }
    // The pad as the game has it: X is boost, the triggers are only the triggers.
    if input.x_held {
        down |= button::X;
    }
    // Held state, as the pad reports it: a press that lasts one render frame would be
    // lost on the render frames that run no game frame, and slide needs B held.
    if input.stomp_held {
        down |= button::B;
    }
    if input.y_held {
        down |= button::Y;
    }
    if input.left_bumper {
        down |= button::LEFT_BUMPER;
    }
    if input.right_bumper {
        down |= button::RIGHT_BUMPER;
    }
    if input.left_trigger > 0.5 {
        down |= button::LEFT_TRIGGER;
    }
    if input.right_trigger > 0.5 {
        down |= button::RIGHT_TRIGGER;
    }
    if input.start {
        down |= 0x400;
    }
    // SPadState as the game's input layer fills it: sticks in [-1, 1] (+y up), triggers 0..1.
    // UNLEASHED_AUTORUN: hold forward (test runs that need Sonic to cover ground), with
    // boost held unless it is "jump" (then A is held 0.3 s of every 1.5 s) or "boostpulse"
    // (boost held 1 s of every 2.5 s).
    let autorun_mode = std::env::var("UNLEASHED_AUTORUN").ok();
    let autorun = autorun_mode.is_some();
    match autorun_mode.as_deref() {
        Some("jump") => {
            if time.elapsed_secs() % 1.5 < 0.3 {
                down |= button::A;
            }
        }
        Some("boostpulse") => {
            if time.elapsed_secs() % 2.5 < 1.0 {
                down |= button::X;
            }
        }
        Some(_) => down |= button::X,
        None => {}
    }
    let pad = Pad {
        down,
        left: if autorun { [0.0, 1.0] } else { [game_axis(input.stick.x), game_axis(input.stick.y)] },
        right: [game_axis(input.right_stick.x), game_axis(input.right_stick.y)],
        left_trigger: input.left_trigger,
        right_trigger: input.right_trigger,
    };

    // Respawn (R, or Select / Share on a pad): the game starts over from the snapshot.
    if input.respawn {
        let started = std::time::Instant::now();
        match guest.sonic.reload() {
            Ok(()) => {
                guest.generation += 1;
                guest.acc = 0.0;
                info!("guest: respawned (snapshot reloaded in {:?})", started.elapsed());
            }
            Err(e) => warn!("guest: respawn failed: {e}"),
        }
    }
    // Whenever the buttons change: what the input library reports and what the game gets.
    let pressed: String = gamepads
        .iter()
        .map(|(name, g)| format!("{} {:?}", name.map_or("gamepad", |n| n.as_str()), g.get_pressed().collect::<Vec<_>>()))
        .collect::<Vec<_>>()
        .join("; ");
    if (pressed.as_str(), pad.down) != (last_pad.0.as_str(), last_pad.1) {
        info!("guest pad: {pressed} -> game buttons {:#06X}, triggers ({:.2}, {:.2})", pad.down, pad.left_trigger, pad.right_trigger);
        *last_pad = (pressed, pad.down);
    }

    // The snapshot supplies the simulation step. The display clock remains
    // independent, with one shared interpolation phase for camera and meshes.
    let frame = guest.sonic.snapshot_dt();
    // Bevy already bounds a long display frame to 250 ms. In Resort Island,
    // catch up that elapsed time instead of silently discarding it after four
    // game steps; every individual physics step still uses the captured dt.
    let catchup_limit = if resort.is_some() { 0.25 + frame } else { frame * 4.0 };
    guest.acc = (guest.acc + time.delta_secs_f64()).min(catchup_limit);
    while guest.acc >= frame {
        guest.acc -= frame;
        let g = guest.sonic.step(&pad);
        if resort.is_some() { guest.record_render_frame(); }
        let rotation = Quat::from_xyzw(g.rotation[0], g.rotation[1], g.rotation[2], g.rotation[3]);
        s.center = Vec3::from(g.position);
        s.vel = Vec3::from(g.velocity);
        s.facing = rotation * Vec3::Z;
        s.up = rotation * Vec3::Y;
        s.grounded = true;
        let body = Quat::from_xyzw(g.body_rotation[0], g.body_rotation[1], g.body_rotation[2], g.body_rotation[3]);
        s.guest_pose = Some((Vec3::from(g.body_position), body.normalize()));
        // The stage clock counts game frames (the game's own timer object is not in the loop).
        run.time += frame as f32;
        run.elapsed += frame as f32;
    }
    // The HUD's numbers are the game's: rings and the ring energy gauge.
    let (rings, energy) = guest.sonic.hud();
    run.rings = rings;
    let (enemy, trick) = guest.sonic.scores();
    run.score.enemy = enemy;
    run.score.trick = trick;
    run.countdown = 0.0;
    s.guest_boost = Some((energy / 100.0).clamp(0.0, 1.0));
    // His effects are the game's own (fx.rs draws what its particle system renders); the
    // old stand-in boost aura and spin ball stay hidden.
    s.boosting = false;
    // Twice a second: how far the game's view points off Sonic's travel, with the pad
    // that reached the game (for chasing camera and steering complaints).
    let now = time.elapsed_secs();
    if (now * 2.0) as u32 != ((now - time.delta_secs()) * 2.0) as u32 {
        if let Some(c) = guest.sonic.game_view().or_else(|| guest.sonic.camera()) {
            let view = Vec2::new(c.target[0] - c.eye[0], c.target[2] - c.eye[2]);
            let v = Vec2::new(s.vel.x, s.vel.z);
            let angle = (v.x * view.y - v.y * view.x).atan2(v.dot(view)).to_degrees();
            info!(
                "guest steer: view {angle:+.1} deg off travel, heading {:+.0}, speed {:.0}, stick ({:+.2}, {:+.2}), right stick ({:+.2}, {:+.2})",
                v.x.atan2(v.y).to_degrees(), v.length(), pad.left[0], pad.left[1], pad.right[0], pad.right[1]
            );
        }
    }
    let missing = guest.sonic.missing().len();
    if missing > guest.reported {
        guest.reported = missing;
        warn!("guest: not lifted yet (calls skipped): {:08X?}", guest.sonic.missing());
    }
    // The game renders Sonic at his body object's position (his feet).
    if resort.is_some() {
        let body = Mat4::from_cols_array(&guest.rendered_body());
        let (_, rotation, feet) = body.to_scale_rotation_translation();
        s.guest_pose = Some((feet,rotation));
    }
    transform.translation = s.guest_pose.map_or(s.center, |(p, _)| p);
}

/// A raw stick axis as the game's SPadState holds it. The game's input layer applies a
/// dead zone to each axis on its own and rescales the rest to 0..1; this host writes
/// SPadState directly, so it has to do the same. Measured from a real session
/// (re/capture/traces): with the stick pushed forward 59% of samples have x exactly 0,
/// and the smallest values step 0.0074, 0.018, 0.0287, ... (an 8-bit pad through a dead
/// zone of about 8689 of 32767, XInput's thumb dead zone). Without it a forward push
/// that is a few degrees off axis steers Sonic, and the camera follows him round.
fn game_axis(v: f32) -> f32 {
    const DEAD_ZONE: f32 = 8689.0 / 32767.0;
    if v.abs() <= DEAD_ZONE { 0.0 } else { v.signum() * ((v.abs() - DEAD_ZONE) / (1.0 - DEAD_ZONE)).min(1.0) }
}

/// Average frame time every 2 s (any mode), for chasing lag.
/// When this frame's First, PreUpdate and PostUpdate schedules ran (a system in each).
#[derive(Resource, Default)]
struct FrameMarks([Option<std::time::Instant>; 3]);

fn log_frame_time(time: Res<Time>, marks: Res<FrameMarks>, mut acc: Local<(f32, u32)>, mut last_end: Local<Option<std::time::Instant>>) {
    // Where a slow frame went: between the previous frame's end and this one's start
    // (rendering, presenting, the window), or inside this frame's schedules.
    let now = std::time::Instant::now();
    if let ([Some(first), Some(pre), Some(post)], Some(end)) = (marks.0, *last_end) {
        let total = now.duration_since(end);
        if total.as_millis() >= 30 && time.elapsed_secs() > 5.0 {
            info!(
                "slow frame: {:.1} ms = {:.1} between frames (render) + {:.1} First + {:.1} PreUpdate/Update + {:.1} PostUpdate/Last",
                total.as_secs_f32() * 1e3,
                first.duration_since(end).as_secs_f32() * 1e3,
                pre.duration_since(first).as_secs_f32() * 1e3,
                post.duration_since(pre).as_secs_f32() * 1e3,
                now.duration_since(post).as_secs_f32() * 1e3,
            );
        }
    }
    *last_end = Some(now);
    acc.0 += time.delta_secs();
    acc.1 += 1;
    if acc.0 >= 2.0 {
        info!("frame time: {:.1} ms average over {} frames", acc.0 * 1000.0 / acc.1 as f32, acc.1);
        *acc = (0.0, 0);
    }
}
