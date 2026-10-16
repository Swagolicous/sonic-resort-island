//! Windmill Isle Act 1 (Sonic Unleashed, Xbox 360 data) running natively in
//! Rust + Bevy. Every model, texture, collision triangle and placed object is
//! read from the original game files at startup; nothing is pre-converted.

mod anim;
mod camera;
mod chr;
mod convert;
mod csd_hud;
mod debug;
mod fx;
mod guest;
mod guide;
mod hedgehog;
mod hud;
mod minecraft;
mod motion_blur;
mod objects;
mod paths;
mod physics;
mod post;
mod player;
mod resort;
mod world;

use std::path::{Path, PathBuf};

use bevy::{app::PluginGroupBuilder, prelude::*, window::WindowResolution};
use unleashed_assets::{
    resources::ResourceDb,
    sequence::stage_score_settings,
    stage::day::{DayStage, load_day_stage},
};
use unleashed_game::score::{ScoreKind, ScoreParams, ScoreTracker, StageScore};

const DEFAULT_STAGE: &str = "ActD_MykonosAct1";
const COMMON_ARCHIVES: &[&str] = &[
    "SystemCommonCore",
    "SystemCommon",
    // The particle effects' shared textures (eff_comon_*).
    "SonicCommon",
    "SonicCommonGeneral",
    "ActionCommon",
    "#ActionCommon",
    "SonicActionCommon",
    "#SonicActionCommon",
    "EvilActionCommonGeneral",
    "SonicActionCommonGeneral",
    "#SonicActionCommonGeneral",
    "Sonic",
    "#Sonic",
];

#[derive(Resource)]
pub struct Stage(pub DayStage);

/// Seconds of the original "3, 2, 1, GO" before Sonic can move
/// (`ui_start.yncp` Start/Intro_Anim shows GO at frame 205 of 60 fps).
pub const COUNTDOWN: f32 = 205.0 / 60.0;

/// Whole-run state shared by gameplay systems.
#[derive(Resource)]
pub struct Run {
    pub rings: u32,
    /// Stage timer: starts at GO.
    pub time: f32,
    /// Seconds since the stage (re)started, including the countdown.
    pub elapsed: f32,
    pub countdown: f32,
    pub cleared: bool,
    pub deaths: u32,
    pub enemies: u32,
    /// Score manager state (checkpoint speeds, enemy and trick points).
    pub score: ScoreTracker,
}

/// Score tuning (executable defaults) and this stage's score settings.
#[derive(Resource, Default)]
pub struct StageScoring {
    pub params: ScoreParams,
    pub stage: StageScore,
}

impl StageScoring {
    /// Reads the stage's ChangeStage entry from `#Application/*.seq.xml`
    /// (default trial-mission case), falling back to the executable defaults.
    fn load(root: &Path, stage_name: &str) -> Self {
        let mut out = Self::default();
        let mut db = ResourceDb::new();
        if let Err(e) = db.load_arl(&root.join("game"), "#Application") {
            eprintln!("[score] #Application: {e:#}; using executable defaults");
            return out;
        }
        let names: Vec<String> = db.names().filter(|n| n.ends_with(".seq.xml")).map(str::to_owned).collect();
        for name in names {
            let Some(bytes) = db.get(&name) else { continue };
            let Ok(Some(s)) = stage_score_settings(bytes, stage_name, "eFlag_TrialMissionType_Default") else {
                continue;
            };
            if s.rank_table.is_none() && s.base_time.is_none() {
                continue;
            }
            if let Some(t) = s.rank_table {
                out.stage.rank_table = t;
            }
            if let Some(b) = s.base_time {
                out.stage.base_time = b;
            }
            if let Some(c) = s.checkpoint_num {
                out.stage.checkpoint_num = c;
            }
            if !s.enable_score.is_empty() {
                out.stage.enabled = s.enable_score.iter().filter_map(|k| ScoreKind::from_name(k)).collect();
            }
            eprintln!("[score] {stage_name}: settings from {name}: {:?}", out.stage);
            return out;
        }
        eprintln!("[score] {stage_name}: no sequence score settings; using executable defaults");
        out
    }
}

impl Default for Run {
    fn default() -> Self {
        let skip = std::env::var_os("UNLEASHED_SKIP_COUNTDOWN").is_some();
        Self {
            rings: 0,
            time: 0.0,
            elapsed: 0.0,
            countdown: if skip { 0.0 } else { COUNTDOWN },
            cleared: false,
            deaths: 0,
            enemies: 0,
            score: ScoreTracker::default(),
        }
    }
}

/// Keyboard-only fallback for controller backends that panic during discovery.
fn game_plugins() -> PluginGroupBuilder {
    let plugins = DefaultPlugins.build();
    if std::env::var_os("UNLEASHED_NO_GAMEPAD").is_some() {
        eprintln!("[input] gamepad backend disabled; keyboard and mouse controls enabled");
        plugins.disable::<bevy::gilrs::GilrsPlugin>()
    } else {
        plugins
    }
}

fn main() -> AppExit {
    let mut args = std::env::args().skip(1);
    let first = args.next();
    if first.as_deref() == Some("resort-island") {
        return resort::run(find_install_root(), args.next().map(PathBuf::from));
    }
    // `guest [82346F08_pre.bin] [minecraft [seed]]`: Sonic run by the game's own (lifted) code.
    let mut guest = None;
    let first = if first.as_deref() == Some("guest") {
        let mut next = args.next();
        let snap = match next.as_deref() {
            Some("minecraft") | Some("flat") | None => "snapshots/82346F08_pre.bin".to_string(),
            Some(path) => {
                let path = path.to_string();
                next = args.next();
                path
            }
        };
        eprintln!("[guest] loading {snap} ...");
        let started = std::time::Instant::now();
        match unleashed_guest_host::GuestSonic::load(Path::new(&snap)) {
            Ok(g) => {
                eprintln!("[guest] ready in {:?}: posture {:#X} proxy {:#X} pad {:#X}", started.elapsed(), g.posture, g.proxy, g.pad);
                guest = Some(g);
            }
            Err(e) => {
                eprintln!("[guest] {snap}: {e}");
                return AppExit::error();
            }
        }
        next
    } else {
        first
    };
    // `guest flat`: the Minecraft-mode app with a flat slab instead of blocks.
    if first.as_deref() == Some("flat") && guest.is_some() {
        // SAFETY: single-threaded, before anything else runs.
        unsafe { std::env::set_var("UNLEASHED_FLAT", "1") };
        return run_minecraft(find_install_root(), 7, guest);
    }
    if first.as_deref() == Some("minecraft") {
        let seed = args.next().and_then(|s| s.parse().ok()).unwrap_or(12345);
        return run_minecraft(find_install_root(), seed, guest);
    }
    let (root, stage_name) = match first {
        Some(arg) if Path::new(&arg).join("game").is_dir() => {
            (PathBuf::from(arg), args.next().unwrap_or_else(|| DEFAULT_STAGE.into()))
        }
        Some(stage) => (find_install_root(), stage),
        None => (find_install_root(), DEFAULT_STAGE.into()),
    };

    let started = std::time::Instant::now();
    let stage = match load_day_stage(&root, &stage_name, COMMON_ARCHIVES) {
        Ok(stage) => stage,
        Err(e) => {
            eprintln!("failed to load {stage_name} from {}: {e:#}", root.display());
            eprintln!("usage: windmill-isle [install_root] [ActD_stage]");
            return AppExit::error();
        }
    };
    eprintln!("[stage] {stage_name} loaded in {:?}", started.elapsed());
    let scoring = StageScoring::load(&root, &stage_name);
    let scene_lighting = hedgehog::SceneLighting::load(&stage.db);
    eprintln!("[light] {scene_lighting:?}");
    for line in &stage.diagnostics {
        eprintln!("[stage]   {line}");
    }

    let mut app = App::new();
    if let Some(g) = guest {
        app.insert_resource(guest::GuestMode).insert_non_send(guest::GuestPlayer::new(g));
    }
    app.add_plugins(guest::GuestPlugin);
    app
        .add_plugins(game_plugins().set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("Sonic Unleashed in Rust - {stage_name}"),
                resolution: WindowResolution::new(1600, 900),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Stage(stage))
        .insert_resource(scoring)
        .insert_resource(scene_lighting)
        .init_resource::<Run>()
        .add_plugins((
            hedgehog::HedgehogPlugin,
            post::GamePostPlugin,
            world::WorldPlugin,
            physics::PhysicsPlugin,
            player::PlayerPlugin,
            camera::CameraPlugin,
            objects::ObjectsPlugin,
            hud::HudPlugin,
            csd_hud::CsdHudPlugin,
            anim::AnimPlugin,
            debug::DebugPlugin,
            guide::GuidePlugin,
            paths::PathsPlugin,
        ))
        .add_plugins((chr::ChrPlugin, fx::FxPlugin))
        .run()
}

/// Sonic in a MinecraftOSS world (`windmill-isle minecraft [seed]`). Sonic's model,
/// animations and parameters still come from the game files; Minecraft's files are
/// fetched once from Mojang into `<install root>/minecraft`.
///
/// With `guest` (`windmill-isle guest minecraft [seed]`) Sonic is the game's own code:
/// the world is placed so its spawn point is the ground under Sonic where the snapshot
/// has him, the game's coordinates are the world's, and minecraft::guest_blocks gives
/// the game's Havok world the blocks around him.
fn run_minecraft(root: PathBuf, seed: i64, guest: Option<unleashed_guest_host::GuestSonic>) -> AppExit {
    use std::collections::HashMap;
    use unleashed_assets::stage::day::Spawn;

    let files = match unleashed_minecraft::setup::ensure(&root.join("minecraft")) {
        Ok(files) => files,
        Err(e) => {
            eprintln!("[minecraft] could not fetch Minecraft's files from Mojang: {e}");
            return AppExit::error();
        }
    };
    let mut world = match unleashed_minecraft::McWorld::load(&files, seed, 8) {
        Ok(world) => world,
        Err(e) => {
            eprintln!("[minecraft] world failed to load: {e}");
            return AppExit::error();
        }
    };
    // Sonic is placed once the ground under the spawn exists.
    let started = std::time::Instant::now();
    while !(world.spawn_ready() && !world.collision_near([0.0; 3], 1).is_empty())
        && started.elapsed() < std::time::Duration::from_secs(30)
    {
        world.tick([0.0; 3]);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    // Guest mode: the top of the spawn column goes under Sonic's feet (the body object's
    // position), so positions in the game's memory are positions in this world.
    let start = match &guest {
        Some(g) => {
            let feet = g.sonic().body_position;
            let (x, z) = (world.origin[0].floor() as i32, world.origin[2].floor() as i32);
            let ground = world.top_solid_y(x, z).map_or(world.origin[1], |y| y as f64 + 1.0);
            world.origin = [world.origin[0] - feet[0] as f64, ground - feet[1] as f64, world.origin[2] - feet[2] as f64];
            eprintln!("[minecraft] guest: Sonic's feet {feet:?} are on the spawn column's top (block y {ground})");
            // His collision is minecraft::guest_blocks' Havok boxes, not triangles.
            world.triangle_collision = false;
            feet
        }
        None => [0.0, 2.0, 0.0],
    };
    // The collision was built relative to the old origin: rebuilt as the world streams.
    let collision = if guest.is_some() {
        physics::CollisionWorld::from_triangles(&[])
    } else {
        physics::CollisionWorld::from_triangles(&world.collision_near([0.0; 3], 2))
    };
    eprintln!("[minecraft] seed {seed}: spawn {:?} streamed in {:?}", world.origin, started.elapsed());

    // Sonic's own resources, without a stage.
    let mut db = unleashed_assets::resources::ResourceDb::new();
    for stem in COMMON_ARCHIVES {
        if let Err(e) = db.load_arl(&root.join("game"), stem) {
            eprintln!("[minecraft] archive {stem} skipped: {e:#}");
        }
    }
    let scene_lighting = hedgehog::SceneLighting::load(&db);
    let stage = DayStage {
        name: "Minecraft".into(),
        root: root.clone(),
        db,
        spawn: Spawn { position: start, yaw_degrees: 0.0 },
        sky_model: None,
        instances: Vec::new(),
        models: HashMap::new(),
        collision: Vec::new(),
        objects: Vec::new(),
        diagnostics: Vec::new(),
    };

    let mut app = App::new();
    let is_guest = guest.is_some();
    if let Some(g) = guest {
        app.insert_resource(guest::GuestMode).insert_non_send(guest::GuestPlayer::new(g));
    }
    app.add_plugins(game_plugins().set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("Sonic Unleashed in Rust - Minecraft world {seed}"),
                resolution: WindowResolution::new(1600, 900),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Stage(stage))
        .insert_resource(StageScoring::default())
        .insert_resource(scene_lighting)
        .insert_resource(Run { countdown: 0.0, ..default() })
        .insert_resource(collision)
        .insert_resource(minecraft::MinecraftMode)
        .insert_resource(ClearColor(Color::srgb(0.55, 0.75, 0.95)))
        .init_resource::<GlobalAmbientLight>()
        .init_resource::<paths::StagePaths>()
        .init_resource::<world::Converted>()
        .insert_non_send(minecraft::MinecraftWorld(world))
        .add_plugins((
            hedgehog::HedgehogPlugin,
            post::GamePostPlugin,
            player::PlayerPlugin,
            camera::CameraPlugin,
            anim::AnimPlugin,
            minecraft::MinecraftPlugin,
        ));
    if is_guest {
        app.add_plugins((guest::GuestPlugin, chr::ChrPlugin, csd_hud::CsdHudPlugin, fx::FxPlugin));
    }
    app.run()
}

/// The install root is the folder containing `game/`. Search upwards from the
/// executable and the working directory so `cargo run` and a copied exe both work.
fn find_install_root() -> PathBuf {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        candidates.extend(exe.ancestors().map(Path::to_path_buf));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.extend(cwd.ancestors().map(Path::to_path_buf));
    }
    candidates
        .into_iter()
        .find(|dir| dir.join("game").join(format!("#{DEFAULT_STAGE}.arl")).is_file())
        .unwrap_or_else(|| PathBuf::from(".."))
}
