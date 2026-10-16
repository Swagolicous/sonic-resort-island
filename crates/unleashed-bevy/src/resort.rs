//! Resort Island: original OBJ/PNG scenery, triangle collision and the guest
//! Unleashed player. All scene tuning is in assets/resort-island/level.json.

use std::{collections::BTreeMap, fs, path::{Path, PathBuf}, time::Instant};

use anyhow::{Context, Result, bail};
use bevy::{
    asset::{RenderAssetUsages, embedded_asset},
    image::{CompressedImageFormats, ImageAddressMode, ImageSampler, ImageSamplerDescriptor, ImageType},
    light::{CascadeShadowConfigBuilder, NotShadowCaster},
    mesh::PrimitiveTopology,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError},
    shader::ShaderRef,
    window::WindowResolution,
};
use unleashed_assets::{resources::ResourceDb, stage::day::{DayStage, Spawn}};
use unleashed_guest_host::{GuestSonic, Pad};
use unleashed_guest_host::terrain::{WeldedTriangle, weld_mesh};

use crate::{Stage, StageScoring, Run, anim, camera, chr, csd_hud, debug, guide,
    fx, guest, hedgehog::SceneLighting, paths, physics::CollisionWorld, player, world::Converted};

const OCEAN_HALF: f32 = 1200.0;

// The rip calls these structural meshes decorations. Include their complete
// decks and supports, even where their timber/stone atlas differs from cliffs.
fn bridge_object(name: &str) -> bool {
    matches!(name, "Dec5" | "Dec10" | "Dec11" | "Dec12" | "Dec13" |
        "Dec21" | "Dec22" | "Dec23" | "Dec38" | "Dec39" | "Dec40" |
        "Dec41" | "Dec42" | "Dec43" | "Dec44" | "Dec45" | "Dec46" |
        "Dec47" | "Dec48" | "Dec56" | "Dec65" | "Dec71")
}

#[derive(Resource, Clone, Debug)]
pub struct ResortSettings {
    pub directory: PathBuf,
    pub scale: f32,
    pub spawn_obj: Vec3,
    pub yaw: f32,
    /// Import origin follows the unchanged guest's resting feet.
    origin: Vec3,
    orientation: Quat,
    pub sun_direction: Vec3,
    pub sun_illuminance: f32,
    pub ambient_brightness: f32,
    pub exposure: f32,
    pub camera_distance_factor: f32,
    pub camera_pitch_bias: f32,
    pub camera_focus_offset: f32,
    pub sonic_ambient: Vec3,
    pub sonic_sun: Vec3,
    pub sonic_specular: f32,
    sky_horizon: Vec3,
    sky_zenith: Vec3,
    cloud_amount: f32,
    cloud_scale: f32,
    rock_gain: f32,
    terrain_gain: f32,
    water_height: f32,
    water_tiling: f32,
    water_tint: Vec3,
    sand_tint: Vec3,
}

impl ResortSettings {
    fn load(directory: PathBuf) -> Result<Self> {
        let path = directory.join("level.json");
        let json: serde_json::Value = serde_json::from_slice(&fs::read(&path)
            .with_context(|| format!("reading {}", path.display()))?)?;
        let number = |key: &str| -> Result<f32> {
            let n = json[key].as_f64().with_context(|| format!("missing numeric {key}"))? as f32;
            if !n.is_finite() { bail!("{key} must be finite"); }
            Ok(n)
        };
        let vector = |key: &str| -> Result<Vec3> {
            let a = json[key].as_array().with_context(|| format!("missing vector {key}"))?;
            if a.len() != 3 { bail!("{key} must contain three numbers"); }
            let mut v = [0.0; 3];
            for (i, n) in a.iter().enumerate() {
                v[i] = n.as_f64().context("vector coordinate must be numeric")? as f32;
                if !v[i].is_finite() { bail!("{key} must be finite"); }
            }
            Ok(Vec3::from(v))
        };
        let settings = Self {
            directory: directory.clone(), scale: number("scale")?,
            spawn_obj: vector("spawn_obj")?, yaw: number("yaw_degrees")?, origin: Vec3::ZERO,
            orientation: Quat::IDENTITY,
            sun_direction: vector("sun_direction")?.normalize_or(Vec3::NEG_Y),
            sun_illuminance: number("sun_illuminance")?,
            ambient_brightness: number("ambient_brightness")?,
            exposure: number("exposure_ev100")?, rock_gain: number("rock_gain")?,
            camera_distance_factor: number("camera_distance_factor")?,
            camera_pitch_bias: number("camera_pitch_bias_degrees")?.to_radians(),
            camera_focus_offset: number("camera_focus_offset")?,
            sonic_ambient: vector("sonic_ambient")?, sonic_sun: vector("sonic_sun")?,
            sonic_specular: number("sonic_specular")?,
            sky_horizon: vector("sky_horizon")?, sky_zenith: vector("sky_zenith")?,
            cloud_amount: number("cloud_amount")?, cloud_scale: number("cloud_scale")?,
            terrain_gain: number("terrain_gain")?, water_height: number("water_height_obj")?,
            water_tiling: number("water_tiling")?,
            water_tint: vector("water_tint_linear")?, sand_tint: vector("sand_tint_linear")?,
        };
        if settings.scale <= 0.0 || settings.camera_distance_factor <= 0.0 {
            bail!("scale and camera distance factor must be positive");
        }
        Ok(settings)
    }

    fn position(&self, p: Vec3) -> Vec3 {
        self.origin + self.orientation * ((p - self.spawn_obj) * self.scale)
    }
}

#[derive(Default)]
struct Surface {
    positions: Vec<[f32; 3]>, normals: Vec<[f32; 3]>, uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
}

#[derive(Resource)]
struct IslandMesh {
    surfaces: BTreeMap<String, Surface>,
    collision: Vec<[[f32; 3]; 3]>,
    terrain_count: usize,
    floor_boxes: Vec<FloorBox>,
    textures: BTreeMap<String, PathBuf>,
}

#[derive(Clone)]
struct FloorBox { center: [f32;3], half: [f32;3], rotation: [f32;4] }

fn obj_index(s: &str, count: usize) -> Result<usize> {
    let value: isize = s.parse().context("invalid OBJ index")?;
    let index = if value < 0 { count as isize + value } else { value - 1 };
    if index < 0 || index as usize >= count { bail!("OBJ index {value} outside {count} entries"); }
    Ok(index as usize)
}

impl IslandMesh {
    fn load(settings: &ResortSettings) -> Result<Self> {
        let text = fs::read_to_string(settings.directory.join("island.obj"))?;
        let mtl = fs::read_to_string(settings.directory.join("island.mtl"))?;
        let mut textures = BTreeMap::new();
        let mut material = String::new();
        // The rip's `d 0` entries are exported incorrectly. PNG alpha is authoritative.
        for line in mtl.lines() {
            if let Some(name) = line.strip_prefix("newmtl ") { material = name.trim().to_owned(); }
            if let Some(name) = line.strip_prefix("map_Kd ") {
                let name = name.trim();
                let path = fs::read_dir(&settings.directory)?
                    .filter_map(|entry| entry.ok())
                    .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
                    .with_context(|| format!("missing texture {name}"))?.path();
                textures.insert(material.clone(), path);
            }
        }
        let mut vertices = Vec::<Vec3>::new();
        let mut uvs = Vec::<Vec2>::new();
        let mut normals = Vec::<Vec3>::new();
        let mut surfaces = BTreeMap::<String, Surface>::new();
        let mut collision = Vec::new();
        let mut object = "";
        let mut material = "";
        for (line_number, line) in text.lines().enumerate() {
            let mut s = line.split_whitespace();
            let Some(kind) = s.next() else { continue };
            match kind {
                "v" | "vn" => {
                    let v: Vec<f32> = s.take(3).map(str::parse).collect::<std::result::Result<_, _>>()?;
                    if v.len() != 3 || v.iter().any(|x| !x.is_finite()) { bail!("invalid vector at line {}", line_number + 1); }
                    if kind == "v" { vertices.push(Vec3::new(v[0], v[1], v[2])); }
                    else { normals.push(Vec3::new(v[0], v[1], v[2]).normalize_or(Vec3::Y)); }
                }
                "vt" => {
                    let v: Vec<f32> = s.take(2).map(str::parse).collect::<std::result::Result<_, _>>()?;
                    if v.len() != 2 { bail!("invalid texture coordinate"); }
                    uvs.push(Vec2::new(v[0], 1.0 - v[1]));
                }
                "o" => object = s.next().unwrap_or(""),
                "usemtl" => material = s.next().context("empty usemtl")?,
                "f" => {
                    let mut face = Vec::new();
                    for token in s {
                        let parts: Vec<&str> = token.split('/').collect();
                        let p = obj_index(parts[0], vertices.len())?;
                        let uv = parts.get(1).filter(|x| !x.is_empty())
                            .map(|x| obj_index(x, uvs.len())).transpose()?;
                        let n = parts.get(2).filter(|x| !x.is_empty())
                            .map(|x| obj_index(x, normals.len())).transpose()?;
                        face.push((p, uv, n));
                    }
                    if face.len() < 3 { bail!("face has fewer than three vertices"); }
                    for i in 1..face.len() - 1 {
                        let f = [face[0], face[i], face[i + 1]];
                        let p = f.map(|(p, _, _)| settings.position(vertices[p]));
                        let cross = (p[1] - p[0]).cross(p[2] - p[0]);
                        if cross.length_squared() < 1e-12 { continue; }
                        let normal = cross.normalize();
                        let uv = f.map(|(_, uv, _)| uv.map(|u| uvs[u]).unwrap_or(Vec2::ZERO));
                        let mean_uv = (uv[0] + uv[1] + uv[2]) / 3.0;
                        // Blue pond/waterfall tile only; the adjoining gate panels
                        // occupy the rest of this atlas quadrant.
                        let water = material == "5" && mean_uv.x >= 0.5 && mean_uv.x < 0.8125 && mean_uv.y < 0.25;
                        let rock = material == "1" && mean_uv.x > 0.375 && mean_uv.y > 0.5;
                        let gain = if rock { settings.rock_gain } else { settings.terrain_gain };
                        let sand = material == "3" && mean_uv.x < 0.25 && mean_uv.y < 0.75;
                        let tint = if sand { settings.sand_tint } else { Vec3::ONE } * gain;
                        let surface = surfaces.entry(material.to_owned()).or_default();
                        for k in 0..3 {
                            surface.positions.push(p[k].to_array());
                            surface.normals.push(f[k].2.map(|n| settings.orientation * normals[n]).unwrap_or(normal).to_array());
                            surface.uvs.push(uv[k].to_array());
                            surface.colors.push(tint.extend(1.0).to_array());
                        }
                        // The rip labels cliffs and stone bridges as decorations too.
                        // Rock atlas faces are solid; grass cards and collectibles are not.
                        let solid = object.starts_with("Trk") || (object.starts_with("Dec") && rock)
                            || bridge_object(object) || water;
                        if solid {
                            collision.push(p.map(|v| v.to_array()));
                        }
                    }
                }
                _ => {}
            }
        }
        if collision.is_empty() || surfaces.is_empty() { bail!("OBJ contains no usable course"); }
        for name in surfaces.keys() {
            if !textures.contains_key(name) { bail!("material {name} has no diffuse texture"); }
        }
        let terrain_count = collision.len();
        let mut floor_boxes = Vec::new();
        // Same dimensions and top height as the rendered ocean. One continuous
        // box avoids seams while allowing Sonic to stand/run on the water.
        let water_y = settings.origin.y + (settings.water_height - settings.spawn_obj.y) * settings.scale;
        let (x,z) = (settings.origin.x, settings.origin.z);
        let ocean = [[x-OCEAN_HALF,water_y,z-OCEAN_HALF], [x-OCEAN_HALF,water_y,z+OCEAN_HALF],
            [x+OCEAN_HALF,water_y,z+OCEAN_HALF], [x+OCEAN_HALF,water_y,z-OCEAN_HALF]];
        collision.push([ocean[0],ocean[1],ocean[2]]);
        collision.push([ocean[0],ocean[2],ocean[3]]);
        floor_boxes.push(FloorBox { center: [x,water_y-0.5,z], half: [OCEAN_HALF,0.5,OCEAN_HALF],
            rotation: Quat::IDENTITY.to_array() });
        // Merge opaque cells into rectangles using Minecraft's fixed box path.
        if let (Some(floor), Some(path)) = (surfaces.get("Material.001"), textures.get("Material.001")) {
            let image = png(path, false)?;
            let pixels = image.data.as_ref().context("floor texture has no pixel data")?;
            let (width, height) = (image.width() as usize, image.height() as usize);
            if pixels.len() != width * height * 4 { bail!("floor texture must decode to RGBA8"); }
            // Recover the quad's UV basis so collision follows the imported floor
            // even when it is rotated to the guest's original starting heading.
            let p0 = Vec3::from(floor.positions[0]);
            let uv0 = Vec2::from(floor.uvs[0]);
            let axis_u = (Vec3::from(floor.positions[1]) - p0) / (floor.uvs[1][0] - uv0.x);
            let axis_v = (Vec3::from(floor.positions[5]) - p0) / (floor.uvs[5][1] - uv0.y);
            let origin = p0 - axis_u * uv0.x - axis_v * uv0.y;
            let cells = 256usize;
            let opaque = |x: usize, z: usize| {
                let px = ((x as f32 + 0.5) / cells as f32 * width as f32) as usize;
                let py = ((z as f32 + 0.5) / cells as f32 * height as f32) as usize;
                pixels[(py * width + px) * 4 + 3] >= 128
            };
            let mut active = BTreeMap::<(usize,usize),usize>::new();
            let mut rectangles = Vec::new();
            for z in 0..=cells {
                let mut runs = Vec::new();
                let mut x = 0;
                while z < cells && x < cells {
                    if !opaque(x,z) { x += 1; continue; }
                    let start = x;
                    while x < cells && opaque(x,z) { x += 1; }
                    runs.push((start,x));
                }
                let finished: Vec<_> = active.keys().copied().filter(|k| !runs.contains(k)).collect();
                for (x0,x1) in finished {
                    let z0 = active.remove(&(x0,x1)).unwrap();
                    rectangles.push((x0,x1,z0,z));
                }
                for run in runs { active.entry(run).or_insert(z); }
            }
            for (x0,x1,z0,z1) in rectangles {
                let corner = |x: usize, z: usize| (origin + axis_u * (x as f32 / cells as f32)
                    + axis_v * (z as f32 / cells as f32)).to_array();
                let (a,b,c,d) = (corner(x0,z0),corner(x0,z1),corner(x1,z1),corner(x1,z0));
                collision.push([a,b,c]); collision.push([a,c,d]);
                let center = (Vec3::from(a) + Vec3::from(c)) * 0.5 - Vec3::Y * 0.5;
                floor_boxes.push(FloorBox { center: center.to_array(), half: [
                    axis_u.length() * (x1-x0) as f32 / cells as f32 * 0.5, 0.5,
                    axis_v.length() * (z1-z0) as f32 / cells as f32 * 0.5],
                    rotation: settings.orientation.to_array() });
            }
        }
        Ok(Self { surfaces, collision, terrain_count, floor_boxes, textures })
    }
}

pub fn run(root: PathBuf, directory: Option<PathBuf>) -> AppExit {
    let directory = directory.unwrap_or_else(|| root.join("unleashed-rs/assets/resort-island"));
    match prepare(root, directory) {
        Ok(mut app) => app.run(),
        Err(e) => { eprintln!("[resort-island] {e:#}"); AppExit::error() }
    }
}

fn prepare(root: PathBuf, directory: PathBuf) -> Result<App> {
    let mut settings = ResortSettings::load(directory)?;
    // Use the very same guest snapshot and GuestPlugin as `guest minecraft`.
    let snapshot = std::env::var_os("UNLEASHED_SNAPSHOT").map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("snapshots/82346F08_pre.bin"));
    eprintln!("[resort-island] loading Minecraft's guest controller from {}", snapshot.display());
    let mut sonic = GuestSonic::load(&snapshot).map_err(anyhow::Error::msg)?;
    settings.origin = settle_guest(&mut sonic)?;
    let facing = sonic.game_view().or_else(|| sonic.camera())
        .map(|c| Vec3::from(c.target) - Vec3::from(c.eye))
        .unwrap_or_else(|| Quat::from_array(sonic.sonic().body_rotation).normalize() * Vec3::Z);
    let heading = facing.x.atan2(facing.z);
    settings.orientation = Quat::from_rotation_y(heading - settings.yaw.to_radians());
    settings.sun_direction = settings.orientation * settings.sun_direction;
    eprintln!("[resort-island] course aligned to unchanged guest heading {:.1} degrees", heading.to_degrees());
    let island = IslandMesh::load(&settings)?;
    eprintln!("[resort-island] {} materials, {} collision triangles, scale {}",
        island.surfaces.len(), island.collision.len(), settings.scale);
    // This host mesh is only for presentation-camera obstruction. Sonic's actual
    // physics runs entirely in the guest Havok world below.
    let collision = CollisionWorld::from_triangles(&island.collision);
    let course = GuestCourse::new(&island.collision[..island.terrain_count], &island.floor_boxes);
    course.load(&mut sonic);
    let mut db = ResourceDb::new();
    for stem in crate::COMMON_ARCHIVES { db.load_arl(&root.join("game"), stem)?; }
    let scene_lighting = SceneLighting::load(&db);
    let stage = DayStage {
        name: "Resort Island".into(), root, db,
        spawn: Spawn { position: sonic.sonic().position, yaw_degrees: heading.to_degrees() },
        sky_model: None, instances: Vec::new(), models: Default::default(),
        collision: Vec::new(), objects: Vec::new(), diagnostics: Vec::new(),
    };
    let mut app = App::new();
    app.add_plugins(crate::game_plugins().set(WindowPlugin {
        primary_window: Some(Window {
            title: "Sonic Unleashed in Rust - Resort Island".into(),
            resolution: WindowResolution::new(1600, 900), ..default()
        }), ..default()
    }))
        .insert_resource(Stage(stage))
        .insert_resource(StageScoring::default())
        .insert_resource(scene_lighting)
        .insert_resource(Run { countdown: 0.0, ..default() })
        .insert_resource(guest::GuestMode)
        .insert_non_send(guest::GuestPlayer::new(sonic))
        .insert_non_send(course)
        .insert_resource(collision)
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.88, 0.93, 1.0), brightness: settings.ambient_brightness, ..default()
        })
        .insert_resource(ClearColor(Color::srgb(0.22, 0.48, 0.84)))
        .insert_resource(settings)
        .insert_resource(island)
        .init_resource::<Converted>()
        .init_resource::<paths::StagePaths>()
        .init_resource::<guide::GuidePath>()
        .add_plugins((player::PlayerPlugin, guest::GuestPlugin, camera::CameraPlugin, chr::ChrPlugin, fx::FxPlugin,
            anim::AnimPlugin, csd_hud::CsdHudPlugin, debug::DebugPlugin, ResortPlugin));
    Ok(app)
}

struct ResortPlugin;
impl Plugin for ResortPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "resort_sky.wgsl");
        app.add_plugins(MaterialPlugin::<ResortSky>::default())
            .add_systems(Startup, spawn_island)
            .add_systems(Update, update_guest_course.after(player::read_input).after(debug::ScriptInput)
                .before(guest::step_guest))
            .add_systems(PostUpdate, follow_sky);
    }
}

fn png(path: &Path, repeat: bool) -> Result<Image> {
    let address = if repeat { ImageAddressMode::Repeat } else { ImageAddressMode::ClampToEdge };
    let sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: address, address_mode_v: address,
        ..ImageSamplerDescriptor::linear()
    });
    Ok(Image::from_buffer(&fs::read(path)?, ImageType::Extension("png"),
        CompressedImageFormats::NONE, true, sampler, RenderAssetUsages::RENDER_WORLD)?)
}

/// Filter the original RGBA atlas at distance without changing its source PNG.
fn mipmaps(image: &mut Image) {
    let (mut width, mut height) = (image.width() as usize, image.height() as usize);
    let Some(data) = image.data.as_mut() else { return; };
    if data.len() != width * height * 4 { return; }
    let mut offset = 0;
    let mut levels = 1;
    while width > 1 || height > 1 {
        let (nw, nh) = ((width / 2).max(1), (height / 2).max(1));
        let mut next = vec![0; nw * nh * 4];
        for y in 0..nh {
            for x in 0..nw {
                for c in 0..4 {
                    let mut sum = 0u32;
                    for dy in 0..2 {
                        for dx in 0..2 {
                            let sx = (x * 2 + dx).min(width - 1);
                            let sy = (y * 2 + dy).min(height - 1);
                            sum += data[offset + (sy * width + sx) * 4 + c] as u32;
                        }
                    }
                    next[(y * nw + x) * 4 + c] = ((sum + 2) / 4) as u8;
                }
            }
        }
        offset = data.len();
        data.extend_from_slice(&next);
        (width, height) = (nw, nh);
        levels += 1;
    }
    image.texture_descriptor.mip_level_count = levels;
}

fn spawn_island(
    mut commands: Commands, settings: Res<ResortSettings>, island: Res<IslandMesh>,
    mut meshes: ResMut<Assets<Mesh>>, mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>, mut skies: ResMut<Assets<ResortSky>>,
) {
    for (name, surface) in &island.surfaces {
        let mut image = png(&island.textures[name], false).expect("validated course texture");
        mipmaps(&mut image);
        let image = images.add(image);
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, surface.positions.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, surface.normals.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, surface.uvs.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, surface.colors.clone());
        commands.spawn((
            Name::new(format!("Resort Island / material {name}")),
            Mesh3d(meshes.add(mesh)), MeshMaterial3d(materials.add(StandardMaterial {
                base_color_texture: Some(image), alpha_mode: AlphaMode::Mask(0.5),
                perceptual_roughness: 1.0, reflectance: 0.0,
                cull_mode: None, double_sided: true, ..default()
            })),
        ));
    }
    // ISLAND06 is an atlas: the right half is unused white padding. Extract
    // the water tile before repeating it, otherwise the ocean has white stripes.
    let mut water_tile = png(&settings.directory.join("ISLAND06.png"), true).expect("water texture");
    let source_width = water_tile.width() as usize;
    let tile_width = source_width / 2;
    let height = water_tile.height() as usize;
    let source = water_tile.data.take().expect("water pixel data");
    let mut tile = Vec::with_capacity(tile_width * height * 4);
    for row in 0..height {
        tile.extend_from_slice(&source[row * source_width * 4..(row * source_width + tile_width) * 4]);
    }
    water_tile.data = Some(tile);
    water_tile.texture_descriptor.size.width = tile_width as u32;
    mipmaps(&mut water_tile);
    let water_image = images.add(water_tile);
    let half = OCEAN_HALF;
    let mut water = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    let y = settings.origin.y + (settings.water_height - settings.spawn_obj.y) * settings.scale;
    let (x,z) = (settings.origin.x, settings.origin.z);
    let vertices = [[x-half,y,z-half],[x-half,y,z+half],[x+half,y,z+half],[x-half,y,z-half],[x+half,y,z+half],[x+half,y,z-half]];
    water.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices.to_vec());
    water.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0,1.0,0.0]; 6]);
    water.insert_attribute(Mesh::ATTRIBUTE_UV_0, vertices.map(|p| {
        let local = settings.orientation.inverse() * (Vec3::from(p) - settings.origin);
        [local.x * settings.water_tiling, local.z * settings.water_tiling]
    }).to_vec());
    commands.spawn((Name::new("Resort Island ocean"), Mesh3d(meshes.add(water)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(water_image),
            base_color: Color::linear_rgb(settings.water_tint.x, settings.water_tint.y, settings.water_tint.z),
            unlit: true, ..default()
        })), NotShadowCaster));
    commands.spawn((Name::new("Resort Island sun"), DirectionalLight {
        color: Color::srgb(1.0, 0.98, 0.92), illuminance: settings.sun_illuminance,
        shadow_maps_enabled: true, ..default()
    }, Transform::default().looking_to(settings.sun_direction, Vec3::Y),
        CascadeShadowConfigBuilder { num_cascades: 4, minimum_distance: 0.1,
            first_cascade_far_bound: 16.0, maximum_distance: 450.0, ..default() }.build(),
        bevy::camera::visibility::RenderLayers::from_layers(&[0, chr::SHADOW_CASTER_LAYER])));
    commands.spawn((Name::new("Resort Island cloud sky"), ResortSkyDome,
        Mesh3d(meshes.add(Sphere::new(1800.0).mesh().uv(48, 24))),
        MeshMaterial3d(skies.add(ResortSky {
            horizon: settings.sky_horizon.extend(1.0), zenith: settings.sky_zenith.extend(1.0),
            clouds: Vec4::new(settings.cloud_scale, settings.cloud_amount, 0.0, 0.0),
        })), NotShadowCaster));
}

#[derive(Component)]
struct ResortSkyDome;

/// Let the snapshot's running Sonic come to rest through neutral pad frames.
/// This avoids writing over his physics, posture, animation or camera state.
fn settle_guest(g: &mut GuestSonic) -> Result<Vec3> {
    let world = g.havok_world().context("guest snapshot has no Havok world")?;
    let at = Vec3::from(g.sonic().position);
    let old: Vec<_> = g.rigid_bodies(world).into_iter()
        .filter(|&(body, motion, _, shape)| shape != 0 && (motion == 7 ||
            Vec3::from(g.rigid_body_position(body)).distance(at) > 2.0))
        .map(|r| r.0).collect();
    for body in old { g.remove_body(world, body); }
    let feet = g.sonic().body_position;
    let slab = g.add_fixed_box(world, [feet[0], feet[1] - 0.5, feet[2]],
        [2000.0, 0.5, 2000.0], [0.0,0.0,0.0,1.0], 0.0, 7);
    for _ in 0..300 { g.step(&Pad::default()); }
    let feet = Vec3::from(g.sonic().body_position);
    g.remove_body(world, slab);
    if !feet.is_finite() || Vec3::from(g.sonic().velocity).length() > 0.5 {
        bail!("guest controller failed to settle at the course start");
    }
    eprintln!("[resort-island] original guest settled at {feet:?}, camera {:?}", g.game_view());
    Ok(feet)
}

/// Imported geometry is registered in the same Havok world Minecraft uses.
/// Load before play so physics import cannot stall movement.
struct GuestCourse {
    triangles: Vec<WeldedTriangle>,
    boxes: Vec<FloorBox>,
}

impl GuestCourse {
    fn new(triangles: &[[[f32;3];3]], boxes: &[FloorBox]) -> Self {
        Self { triangles: weld_mesh(triangles), boxes: boxes.to_vec() }
    }

    fn load(&self, g: &mut GuestSonic) {
        let Some(world) = g.havok_world() else { return };
        let started = Instant::now();
        for tri in &self.triangles { g.add_fixed_triangle_welded(world, tri.vertices, 7, tri.welding_info); }
        for b in &self.boxes { g.add_fixed_box(world, b.center, b.half, b.rotation, 0.0, 7); }
        eprintln!("[resort-island] loaded {} welded triangles and {} ground/water boxes in {:?}",
            self.triangles.len(), self.boxes.len(), started.elapsed());
    }

}

fn update_guest_course(
    settings: Res<ResortSettings>, mut input: ResMut<player::PlayerInput>,
    mut guest: NonSendMut<guest::GuestPlayer>, course: NonSend<GuestCourse>,
) {
    let fallen = guest.sonic.sonic().body_position[1] < settings.origin.y - 18.0;
    if input.respawn || fallen {
        match guest.sonic.reload().map_err(anyhow::Error::msg)
            .and_then(|_| settle_guest(&mut guest.sonic)) {
            Ok(_) => {
                // Reset GuestPlugin's frame accumulator together with the snapshot.
                guest.reset_frame_clock();
                course.load(&mut guest.sonic);
                info!("resort-island: guest restarted on the start line");
            }
            Err(e) => error!("resort-island: could not restart: {e:#}"),
        }
        input.respawn = false;
    }
}

fn follow_sky(camera: Query<&Transform, With<camera::ChaseCam>>,
    mut sky: Query<&mut Transform, (With<ResortSkyDome>, Without<camera::ChaseCam>)>) {
    let Ok(camera) = camera.single() else { return };
    for mut sky in &mut sky { sky.translation = camera.translation; }
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct ResortSky {
    #[uniform(0)] horizon: Vec4,
    #[uniform(0)] zenith: Vec4,
    #[uniform(0)] clouds: Vec4,
}

impl Material for ResortSky {
    fn fragment_shader() -> ShaderRef { "embedded://windmill_isle/resort_sky.wgsl".into() }
    fn specialize(_pipeline: &MaterialPipeline, descriptor: &mut RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef, _key: MaterialPipelineKey<Self>)
        -> std::result::Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::obj_index;
    #[test]
    fn obj_indices_support_relative_faces_and_reject_out_of_range() {
        assert_eq!(obj_index("1", 4).unwrap(), 0);
        assert_eq!(obj_index("-1", 4).unwrap(), 3);
        assert_eq!(obj_index("-4", 4).unwrap(), 0);
        for s in ["0", "5", "-5", "no"] { assert!(obj_index(s, 4).is_err()); }
    }
}
