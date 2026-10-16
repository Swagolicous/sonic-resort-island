//! Sonic Unleashed's original HUD, rendered from the game's CSD projects
//! (`ui_playscreen.yncp`, `ui_start.yncp`) with their own textures, fonts and
//! keyframe animations.
//!
//! Each displayed scene is a slot with a visibility flag, a motion and a
//! frame. Game state drives the slots the same way the retail HUD code drives
//! its scenes: digit fonts for rings / time / score / lives, the `size` /
//! `total_quantity` motions of the ring-energy gauge, the speed gauge motion,
//! and the `Intro_Anim` motions of the stage start / clear banners.

use bevy::{
    camera::{ClearColorConfig, ScalingMode, visibility::RenderLayers},
    prelude::*,
    sprite::Anchor,
};
use unleashed_assets::csd::{CastInfo, CastKind, Project, Property, Scene, parse_project};

use crate::{Run, Stage, StageScoring, convert, player::Sonic, world::Converted};

pub struct CsdHudPlugin;

impl Plugin for CsdHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_hud).add_systems(
            PostUpdate,
            (drive_hud, render_hud).chain().before(TransformSystems::Propagate),
        );
    }
}

const SCREEN: Vec2 = Vec2::new(1280.0, 720.0);
const MAX_GLYPHS: usize = 10;
const HUD_LAYER: usize = 1;
const CSD_FPS: f32 = 60.0;

/// (project file, scene) pairs the HUD instantiates.
const SLOTS: &[(&str, &str)] = &[
    ("ui_playscreen.yncp", "so_speed_gauge"),
    ("ui_playscreen.yncp", "so_ringenagy_gauge"),
    ("ui_playscreen.yncp", "gauge_frame"),
    ("ui_playscreen.yncp", "player_count"),
    ("ui_playscreen.yncp", "time_count"),
    ("ui_playscreen.yncp", "score_count"),
    ("ui_playscreen.yncp", "ring_count"),
    ("ui_playscreen.yncp", "ring_get"),
    ("ui_start.yncp", "Start"),
    ("ui_start.yncp", "Clear"),
    ("ui_result.yncp", "main/result_title"),
    ("ui_result.yncp", "main/result_num_1"),
    ("ui_result.yncp", "main/result_num_2"),
    ("ui_result.yncp", "main/result_num_3"),
    ("ui_result.yncp", "main/result_num_4"),
    ("ui_result.yncp", "main/result_num_5"),
    ("ui_result.yncp", "main/result_num_6"),
    ("ui_result.yncp", "rank/result_rank_S"),
    ("ui_result.yncp", "rank/result_rank_A"),
    ("ui_result.yncp", "rank/result_rank_B"),
    ("ui_result.yncp", "rank/result_rank_C"),
    ("ui_result.yncp", "rank/result_rank_D"),
    ("ui_result.yncp", "rank/result_rank_E"),
    ("ui_result.yncp", "footer/result_footer"),
];

/// Seconds after the goal before the results screen replaces the clear banner.
const RESULT_DELAY: f32 = 2.6;

/// Results-screen numbers, from the ported score manager
/// (`unleashed_game::score`) and the stage's sequence score settings.
pub struct Results {
    pub time: f32,
    pub rings: u32,
    pub speed_bonus: u32,
    pub enemy_bonus: u32,
    pub trick_bonus: u32,
    pub total: u32,
    pub rank: char,
}

impl Results {
    pub fn from_run(run: &Run, scoring: &StageScoring) -> Self {
        let r = run.score.results(&scoring.params, &scoring.stage, run.time, run.rings);
        Self {
            time: run.time,
            rings: run.rings,
            speed_bonus: r.speed_bonus,
            enemy_bonus: r.enemy_bonus,
            trick_bonus: r.trick_bonus,
            total: r.total,
            rank: r.rank,
        }
    }
}

struct Layer {
    project: Project,
    /// texture index -> (image, pixel size)
    textures: Vec<Option<(Handle<Image>, Vec2)>>,
}

struct Slot {
    layer: usize,
    scene: usize,
    /// One entity per sprite cast, MAX_GLYPHS per font cast, in cast order.
    entities: Vec<Entity>,
    visible: bool,
    /// (motion, frame) pairs applied in order.
    motions: Vec<(String, f32)>,
    /// Play-screen scenes are placed by the HUD code: negative root offsets
    /// measure from the right / bottom edge.
    edge_anchor: bool,
}

#[derive(Resource)]
struct Hud {
    layers: Vec<Layer>,
    slots: Vec<Slot>,
    stage_clock: f32,
    clear_clock: f32,
    ring_flash: f32,
    last_rings: u32,
}

#[derive(Component)]
struct HudSprite;

fn spawn_hud(
    mut commands: Commands,
    stage: Res<Stage>,
    mut converted: ResMut<Converted>,
    mut images: ResMut<Assets<Image>>,
    minecraft: Option<Res<crate::minecraft::MinecraftMode>>,
    resort: Option<Res<crate::resort::ResortSettings>>,
) {
    let db = &stage.0.db;
    let mut layers: Vec<(String, Layer)> = Vec::new();
    for (file, _) in SLOTS {
        if layers.iter().any(|(f, _)| f == file) {
            continue;
        }
        let project = match db.get(file).map(parse_project) {
            Some(Ok(p)) => p,
            Some(Err(e)) => {
                warn!("{file}: {e:#}");
                continue;
            }
            None => {
                warn!("{file}: missing");
                continue;
            }
        };
        let textures = project
            .textures
            .iter()
            .map(|name| {
                let stem = name.trim_end_matches(".dds");
                convert::load_dds(db, &mut converted.0, &mut images, stem, true).map(|h| {
                    let size = images.get(&h).map(|i| i.size().as_vec2()).unwrap_or(Vec2::ONE);
                    (h, size)
                })
            })
            .collect();
        layers.push((file.to_string(), Layer { project, textures }));
    }

    let hud_camera = commands.spawn((
        Name::new("HUD camera"),
        Camera2d,
        // Must match the HDR main camera, or this camera's target replaces the 3D frame.
        bevy::camera::Hdr,
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: SCREEN.x,
                height: SCREEN.y,
            },
            ..OrthographicProjection::default_2d()
        }),
        RenderLayers::layer(HUD_LAYER),
    )).id();
    // The Minecraft-mode main camera is not HDR (camera.rs): neither is the HUD's there.
    if minecraft.is_some() || resort.is_some() {
        commands.entity(hud_camera).remove::<bevy::camera::Hdr>();
    }

    let mut slots = Vec::new();
    for (file, scene_name) in SLOTS {
        let Some(layer) = layers.iter().position(|(f, _)| f == file) else { continue };
        let project = &layers[layer].1.project;
        let Some(scene) = project.scenes.iter().position(|s| s.name == *scene_name) else {
            warn!("{file}: no scene {scene_name}");
            continue;
        };
        let count: usize = project.scenes[scene]
            .casts
            .iter()
            .map(|c| if c.kind == CastKind::Font { MAX_GLYPHS } else { 1 })
            .sum();
        let entities = (0..count)
            .map(|_| {
                commands
                    .spawn((
                        HudSprite,
                        Sprite::default(),
                        Anchor::CENTER,
                        Transform::default(),
                        Visibility::Hidden,
                        RenderLayers::layer(HUD_LAYER),
                    ))
                    .id()
            })
            .collect();
        slots.push(Slot {
            layer,
            scene,
            entities,
            visible: false,
            motions: Vec::new(),
            edge_anchor: *file == "ui_playscreen.yncp",
        });
    }
    info!("CSD HUD: {} scenes from {} projects", slots.len(), layers.len());
    commands.insert_resource(Hud {
        layers: layers.into_iter().map(|(_, l)| l).collect(),
        slots,
        stage_clock: 0.0,
        clear_clock: 0.0,
        ring_flash: f32::MAX,
        last_rings: 0,
    });
}

/// Maps game state onto scene visibility and motions.
fn drive_hud(
    time: Res<Time>,
    run: Res<Run>,
    scoring: Res<StageScoring>,
    hud: Option<ResMut<Hud>>,
    sonic: Query<&Sonic>,
) {
    let Some(mut hud) = hud else { return };
    let Ok(s) = sonic.single() else { return };
    let dt = time.delta_secs();
    let hud = &mut *hud;
    hud.stage_clock = run.elapsed;
    hud.clear_clock = if run.cleared { hud.clear_clock + dt } else { 0.0 };
    if run.rings > hud.last_rings {
        hud.ring_flash = 0.0;
    }
    hud.last_rings = run.rings;
    hud.ring_flash += dt;

    let intro_frame = hud.stage_clock * CSD_FPS;
    let results = (hud.clear_clock - RESULT_DELAY).max(0.0) * CSD_FPS;
    let show_results = run.cleared && hud.clear_clock >= RESULT_DELAY;
    let rank = Results::from_run(&run, &scoring).rank;
    for slot in &mut hud.slots {
        let scene = &hud.layers[slot.layer].project.scenes[slot.scene];
        let end = |name: &str| scene.animation(name).map(|a| a.end).unwrap_or(0.0);
        let playing = !run.cleared;
        let (visible, motions): (bool, Vec<(&str, f32)>) = match scene.name.as_str() {
            "so_ringenagy_gauge" => (playing, vec![("total_quantity", 100.0), ("size", s.boost_gauge() * 100.0)]),
            "gauge_frame" => (playing, vec![("total_quantity", 100.0)]),
            "so_speed_gauge" => (playing, vec![("DefaultAnim", (s.speed() / 85.0 * 100.0).clamp(0.0, 100.0))]),
            "ring_get" => {
                let frame = hud.ring_flash * CSD_FPS;
                (playing && frame < end("DefaultAnim"), vec![("DefaultAnim", frame)])
            }
            "Start" => (intro_frame < end("Intro_Anim"), vec![("Intro_Anim", intro_frame)]),
            "Clear" => (
                run.cleared && !show_results,
                vec![("Intro_Anim", (hud.clear_clock * CSD_FPS).min(end("Intro_Anim")))],
            ),
            name if name.starts_with("rank/result_rank_") => {
                let mine = name.ends_with(rank);
                let intro = end("Intro_Anim");
                // Rank stamps land after the score rows have counted in.
                let f = results - 100.0;
                let motions = if f < intro || scene.animation("Usual_Anim").is_none() {
                    vec![("Intro_Anim", f.clamp(0.0, intro))]
                } else {
                    vec![("Usual_Anim", (f - intro) % end("Usual_Anim").max(1.0))]
                };
                (show_results && mine && f >= 0.0, motions)
            }
            name if name.starts_with("main/") || name.starts_with("footer/") => {
                let motion = if scene.animation("Intro_so_Anim").is_some() { "Intro_so_Anim" } else { "Usual_Anim" };
                (show_results, vec![(motion, results.min(end(motion)))])
            }
            _ => (playing, vec![("DefaultAnim", 0.0)]),
        };
        slot.visible = visible;
        slot.motions = motions.into_iter().map(|(m, f)| (m.to_owned(), f)).collect();
    }
}

/// Root casts are positioned relative to a screen edge: negative offsets
/// measure from the right / bottom.
fn anchor(v: f32) -> f32 {
    if v < 0.0 { 1.0 + v } else { v }
}

fn rgba(c: u32) -> Vec4 {
    Color::srgba_u8((c >> 24) as u8, (c >> 16) as u8, (c >> 8) as u8, c as u8)
        .to_srgba()
        .to_vec4()
}

#[derive(Clone, Copy)]
struct World2d {
    pos: Vec2,
    rot: f32,
    scale: Vec2,
    color: Vec4,
    hidden: bool,
}

fn text_for(scene: &str, cast: &str, run: &Run, scoring: &StageScoring, default: Option<&String>) -> String {
    let minutes = (run.time / 60.0) as u32 % 100;
    let seconds = run.time as u32 % 60;
    let centis = (run.time.fract() * 100.0) as u32;
    match (scene, cast) {
        ("ring_count", _) => format!("{:03}", run.rings.min(999)),
        ("time_count", "time100") => format!("{minutes:02}"),
        ("time_count", "time010") => format!("{seconds:02}"),
        ("time_count", "time001") => format!("{centis:02}"),
        // Live score: what has been earned so far (time/speed bonuses come at the goal).
        ("score_count", _) => {
            let live = scoring.params.ring_bonus_efficient * run.rings + run.score.enemy + run.score.trick;
            format!("{:08}", live.min(99_999_999))
        }
        ("player_count", _) => "02".to_owned(),
        (scene, _) if scene.starts_with("main/result_num_") => {
            let r = Results::from_run(run, scoring);
            let m = (r.time / 60.0) as u32 % 100;
            let s = r.time as u32 % 60;
            let c = (r.time.fract() * 100.0) as u32;
            match scene.as_bytes().last() {
                Some(b'1') => format!("{m:02}:{s:02}:{c:02}"),
                Some(b'2') => format!("{}", r.rings),
                Some(b'3') => format!("{}", r.speed_bonus),
                Some(b'4') => format!("{}", r.enemy_bonus),
                Some(b'5') => format!("{}", r.trick_bonus),
                _ => format!("{}", r.total),
            }
        }
        _ => default.cloned().unwrap_or_default(),
    }
}

fn render_hud(
    hud: Option<Res<Hud>>,
    run: Res<Run>,
    scoring: Res<StageScoring>,
    mut sprites: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<HudSprite>>,
) {
    let Some(hud) = hud else { return };
    for slot in &hud.slots {
        let layer = &hud.layers[slot.layer];
        let scene = &layer.project.scenes[slot.scene];
        if !slot.visible {
            for e in &slot.entities {
                if let Ok((_, _, mut vis)) = sprites.get_mut(*e) {
                    *vis = Visibility::Hidden;
                }
            }
            continue;
        }

        let mut infos: Vec<CastInfo> = scene.casts.iter().map(|c| c.info).collect();
        for (motion, frame) in &slot.motions {
            apply_motion(scene, motion, *frame, &mut infos);
        }

        let mut world: Vec<World2d> = Vec::with_capacity(scene.casts.len());
        let mut entity_base = 0;
        for (ci, cast) in scene.casts.iter().enumerate() {
            let info = infos[ci];
            let local = Vec2::from(info.translation) * SCREEN;
            let gradient = info.gradient.iter().map(|g| rgba(*g)).sum::<Vec4>() / 4.0;
            let color = rgba(info.color) * gradient;
            let w = match cast.parent {
                Some(p) => {
                    let pw = world[p];
                    let inherit = cast.flags[1];
                    let offset = Vec2::from_angle(pw.rot.to_radians()).rotate(local * pw.scale);
                    World2d {
                        pos: pw.pos + offset,
                        rot: pw.rot + info.rotation,
                        scale: Vec2::new(
                            info.scale[0] * if inherit & 0x400 != 0 { pw.scale.x } else { 1.0 },
                            info.scale[1] * if inherit & 0x800 != 0 { pw.scale.y } else { 1.0 },
                        ),
                        color: color * pw.color,
                        hidden: info.hide || pw.hidden,
                    }
                }
                None => World2d {
                    pos: if slot.edge_anchor {
                        Vec2::new(anchor(info.translation[0]), anchor(info.translation[1])) * SCREEN
                    } else {
                        Vec2::from(info.translation) * SCREEN
                    },
                    rot: info.rotation,
                    scale: Vec2::from(info.scale),
                    color,
                    hidden: info.hide,
                },
            };
            world.push(w);

            let z = scene.z_index + ci as f32 * 0.001;
            let tl = Vec2::from(cast.corners[0]) * SCREEN;
            let br = Vec2::from(cast.corners[3]) * SCREEN;
            let visible = cast.enabled && !w.hidden && w.color.w > 0.004;
            match cast.kind {
                CastKind::Sprite => {
                    let sub_slot = if info.sub_image >= 0.0 { info.sub_image as usize } else { 0 };
                    let sub = cast.sub_images.get(sub_slot).copied().unwrap_or(-1);
                    place(layer, scene, sub, tl, br, &w, visible, z, slot.entities[entity_base], &mut sprites);
                    entity_base += 1;
                }
                CastKind::Font => {
                    let text = text_for(&scene.name, &cast.name, &run, &scoring, cast.text.as_ref());
                    let font = cast.font.as_deref().and_then(|f| layer.project.fonts.get(f));
                    let chars: Vec<char> = text.chars().collect();
                    // Glyphs flow left to right from the cast's top-left corner,
                    // each sized by its own sub-image plus the font spacing.
                    let mut x = tl.x;
                    for g in 0..MAX_GLYPHS {
                        let sub = chars
                            .get(g)
                            .and_then(|ch| font.and_then(|f| f.iter().find(|(c, _)| c == ch)))
                            .map(|(_, s)| *s as i32)
                            .unwrap_or(-1);
                        let size = sub_image_pixels(layer, scene, sub).unwrap_or(Vec2::ZERO);
                        let glyph_tl = Vec2::new(x, tl.y);
                        x += size.x + cast.spacing * SCREEN.x;
                        let show = visible && g < chars.len();
                        place(layer, scene, sub, glyph_tl, glyph_tl + size, &w, show, z, slot.entities[entity_base + g], &mut sprites);
                    }
                    entity_base += MAX_GLYPHS;
                }
                CastKind::Null => {
                    if let Ok((_, _, mut vis)) = sprites.get_mut(slot.entities[entity_base]) {
                        *vis = Visibility::Hidden;
                    }
                    entity_base += 1;
                }
            }
        }
    }
}

fn apply_motion(scene: &Scene, motion: &str, frame: f32, infos: &mut [CastInfo]) {
    let Some(anim) = scene.animation(motion) else { return };
    for track in &anim.tracks {
        let v = track.sample(frame);
        let info = &mut infos[track.cast];
        match track.property {
            Property::Hide => info.hide = v != 0.0,
            Property::X => info.translation[0] = v,
            Property::Y => info.translation[1] = v,
            Property::Rotation => info.rotation = v,
            Property::ScaleX => info.scale[0] = v,
            Property::ScaleY => info.scale[1] = v,
            Property::SubImage => info.sub_image = v,
            Property::Color => info.color = v.to_bits(),
            Property::GradientTL => info.gradient[0] = v.to_bits(),
            Property::GradientBL => info.gradient[1] = v.to_bits(),
            Property::GradientTR => info.gradient[2] = v.to_bits(),
            Property::GradientBR => info.gradient[3] = v.to_bits(),
            Property::Unknown(_) => {}
        }
    }
}

/// Pixel size of a sub-image in its texture.
fn sub_image_pixels(layer: &Layer, scene: &Scene, sub: i32) -> Option<Vec2> {
    let s = scene.sub_images.get(usize::try_from(sub).ok()?)?;
    let (_, size) = layer.textures.get(s.texture as usize)?.as_ref()?;
    Some((Vec2::from(s.bottom_right) - Vec2::from(s.top_left)).abs() * *size)
}

#[allow(clippy::too_many_arguments)]
fn place(
    layer: &Layer,
    scene: &Scene,
    sub: i32,
    tl: Vec2,
    br: Vec2,
    w: &World2d,
    visible: bool,
    z: f32,
    entity: Entity,
    sprites: &mut Query<(&mut Sprite, &mut Transform, &mut Visibility), With<HudSprite>>,
) {
    let Ok((mut sprite, mut transform, mut vis)) = sprites.get_mut(entity) else { return };
    let image = usize::try_from(sub)
        .ok()
        .and_then(|i| scene.sub_images.get(i))
        .and_then(|s| layer.textures.get(s.texture as usize)?.as_ref().map(|t| (s, t)));
    let Some((sub_image, (handle, size))) = image.filter(|_| visible) else {
        *vis = Visibility::Hidden;
        return;
    };
    *vis = Visibility::Visible;
    sprite.image = handle.clone();
    sprite.rect = Some(Rect::from_corners(
        Vec2::from(sub_image.top_left) * *size,
        Vec2::from(sub_image.bottom_right) * *size,
    ));
    sprite.custom_size = Some((br - tl).abs() * w.scale.abs());
    sprite.flip_x = w.scale.x < 0.0;
    sprite.flip_y = w.scale.y < 0.0;
    sprite.color = Color::srgba(w.color.x, w.color.y, w.color.z, w.color.w);
    let centre = w.pos + Vec2::from_angle(w.rot.to_radians()).rotate((tl + br) * 0.5 * w.scale);
    // CSD space: origin top-left, +y down. Bevy 2D: origin centre, +y up.
    transform.translation = Vec3::new(centre.x - SCREEN.x * 0.5, SCREEN.y * 0.5 - centre.y, z);
    transform.rotation = Quat::from_rotation_z(-w.rot.to_radians());
}
