//! Spawns the stage's terrain instances, sky dome and sun from the loaded
//! `DayStage`.

use std::collections::HashMap;

use bevy::{light::CascadeShadowConfigBuilder, prelude::*};
use unleashed_assets::mirage::{material::parse_light, mesh::parse_model};

use crate::{
    Stage,
    convert::{self, MirageCache, Shading},
    hedgehog::{self, HedgehogMaterial, SceneLighting},
};
use unleashed_assets::gi::load_gi_atlases;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.55, 0.75, 0.95)))
            .insert_resource(GlobalAmbientLight {
                color: Color::srgb(0.75, 0.85, 1.0),
                brightness: 700.0,
                ..default()
            })
            .init_resource::<Converted>()
            .add_systems(Startup, (spawn_terrain, spawn_sky, spawn_sun).chain())
            .add_systems(PostUpdate, follow_camera_with_sky);
    }
}

/// Shared converter caches so terrain, objects and Sonic reuse textures.
#[derive(Resource, Default)]
pub struct Converted(pub MirageCache);

#[derive(Component)]
pub struct Terrain;

#[derive(Component)]
struct SkyDome;

/// A terrain submesh: shared mesh plus either a ported Hedgehog material
/// (cloned per instance to carry its GI tile) or a fallback PBR material.
enum Part {
    /// Mesh, material, and the material's identity (game material name + mesh slot): parts
    /// with the same identity share one material, whatever model they belong to.
    Hedgehog(Handle<Mesh>, HedgehogMaterial, String),
    Standard(Handle<Mesh>, Handle<StandardMaterial>),
}

enum Child {
    Hedgehog(Handle<Mesh>, Handle<HedgehogMaterial>, u32),
    Standard(Handle<Mesh>, Handle<StandardMaterial>),
}

fn spawn_terrain(
    mut commands: Commands,
    stage: Res<Stage>,
    scene: Res<SceneLighting>,
    mut converted: ResMut<Converted>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut hedgehog: ResMut<Assets<HedgehogMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut buffers: ResMut<Assets<bevy::render::storage::ShaderBuffer>>,
    gi_tiles: Res<hedgehog::GiTiles>,
) {
    let stage = &stage.0;
    let cache = &mut converted.0;
    let pfd = stage.root.join("game").join("Packed").join(&stage.name).join("Stage.pfd");
    let gi = match load_gi_atlases(&pfd) {
        Ok(gi) => gi,
        Err(e) => {
            warn!("GI atlases: {e:#}");
            Default::default()
        }
    };
    let gi_images: Vec<Option<Handle<Image>>> =
        gi.textures.iter().map(|(_, dds)| convert::gi_image(&mut images, dds)).collect();

    let mut built: HashMap<&str, Vec<Part>> = HashMap::new();
    let (mut ported, mut fallback) = (0, 0);
    for (name, model) in &stage.models {
        let parts = model
            .submeshes
            .iter()
            .filter(|s| !s.triangles.is_empty())
            .map(|sub| {
                let channel = convert::diffuse_channel(&stage.db, &sub.material);
                match convert::hedgehog_material(&stage.db, cache, &mut images, &scene, &sub.material, sub.slot.as_ref()) {
                    Some(material) => {
                        ported += 1;
                        let identity = format!("{}|{:?}", sub.material, sub.slot);
                        Part::Hedgehog(meshes.add(convert::hedgehog_mesh(sub, channel)), material, identity)
                    }
                    None => {
                        fallback += 1;
                        let tint = convert::tints_with_vertex_color(&stage.db, &sub.material);
                        let mesh = meshes.add(convert::submesh_mesh_uv(sub, tint, channel));
                        let material = convert::material(
                            &stage.db,
                            cache,
                            &mut images,
                            &mut materials,
                            &sub.material,
                            sub.slot.as_ref(),
                            Shading::Lit,
                        );
                        Part::Standard(mesh, material)
                    }
                }
            })
            .collect();
        built.insert(name.as_str(), parts);
    }

    let root = commands
        .spawn((Name::new("Terrain"), Transform::default(), Visibility::default()))
        .id();
    // One material per (model, part, GI page); each instance's GI tile goes in the shared
    // buffer and its draws carry the index as MeshTag (0 = no GI).
    let mut tiles: Vec<[f32; 4]> = vec![[1.0, 1.0, 0.0, 0.0]];
    let mut shared: HashMap<(&str, usize), Handle<HedgehogMaterial>> = HashMap::new();
    let (mut spawned, mut with_gi) = (0, 0);
    for instance in &stage.instances {
        let Some(parts) = built.get(instance.model.as_str()) else {
            continue;
        };
        let tile = gi.entries.get(&instance.name);
        if tile.is_some() {
            with_gi += 1;
        }
        let transform = Transform::from_matrix(convert::row_major_to_mat4(&instance.matrix));
        let gi_page = tile.and_then(|t| gi_images.get(t.texture).and_then(|i| i.as_ref()).map(|i| (t, i)));
        let tag = match gi_page {
            Some((t, _)) => {
                tiles.push(t.param);
                (tiles.len() - 1) as u32
            }
            None => 0,
        };
        let mut children = Vec::with_capacity(parts.len());
        for part in parts {
            match part {
                Part::Hedgehog(mesh, base, identity) => {
                    let page = gi_page.map_or(usize::MAX, |(t, _)| t.texture);
                    let handle = shared
                        .entry((identity.as_str(), page))
                        .or_insert_with(|| {
                            let mut material = base.clone();
                            material.gi_tiles = gi_tiles.0.clone();
                            if let Some((_, image)) = gi_page {
                                material.gi = Some(image.clone());
                                let bits = material.uniform.flags.x as u32 | hedgehog::flags::HAS_GI;
                                material.uniform.flags.x = bits as f32;
                            }
                            hedgehog.add(material)
                        })
                        .clone();
                    children.push(Child::Hedgehog(mesh.clone(), handle, tag));
                }
                Part::Standard(mesh, material) => children.push(Child::Standard(mesh.clone(), material.clone())),
            }
        }
        commands.entity(root).with_children(|terrain| {
            let mut entity = terrain.spawn((Name::new(instance.name.clone()), Terrain, transform, Visibility::default()));
            entity.with_children(|parent| {
                for child in children {
                    match child {
                        Child::Hedgehog(mesh, material, tag) => {
                            parent.spawn((Mesh3d(mesh), MeshMaterial3d(material), bevy::mesh::MeshTag(tag)))
                        }
                        Child::Standard(mesh, material) => parent.spawn((Mesh3d(mesh), MeshMaterial3d(material))),
                    };
                }
            });
        });
        spawned += 1;
    }
    let materials_used = shared.len();
    if let Some(mut buffer) = buffers.get_mut(&gi_tiles.0) {
        *buffer = bevy::render::storage::ShaderBuffer::from(tiles);
    }
    info!("terrain: {materials_used} shared materials");
    info!(
        "terrain: {spawned} instances ({with_gi} with GI), {ported} ported submeshes, {fallback} fallback, {} image failures",
        cache.failed_images.len()
    );
    for failure in cache.failed_images.iter().take(20) {
        warn!("image {failure}");
    }
}

fn spawn_sky(
    mut commands: Commands,
    stage: Res<Stage>,
    scene: Res<SceneLighting>,
    mut converted: ResMut<Converted>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut hedgehog: ResMut<Assets<HedgehogMaterial>>,
    mut images: ResMut<Assets<Image>>,
    gi_tiles: Res<hedgehog::GiTiles>,
) {
    let stage = &stage.0;
    let Some(sky) = &stage.sky_model else { return };
    let model = match stage.db.get(&format!("{sky}.model")).map(parse_model) {
        Some(Ok(model)) => model,
        Some(Err(e)) => return warn!("sky {sky}: {e:#}"),
        None => return warn!("sky {sky}: missing"),
    };
    commands
        .spawn((
            Name::new(format!("Sky {sky}")),
            SkyDome,
            Transform::default(),
            Visibility::default(),
        ))
        .with_children(|parent| {
            for sub in &model.submeshes {
                let ported = convert::hedgehog_material(&stage.db, &mut converted.0, &mut images, &scene, &sub.material, sub.slot.as_ref());
                match ported {
                    Some(mut material) => {
                        material.double_sided = true;
                        material.gi_tiles = gi_tiles.0.clone();
                        parent.spawn((
                            Mesh3d(meshes.add(convert::hedgehog_mesh(sub, 0))),
                            MeshMaterial3d(hedgehog.add(material)),
                            bevy::light::NotShadowCaster,
                            bevy::light::NotShadowReceiver,
                        ));
                    }
                    None => {
                        let material = convert::material(
                            &stage.db,
                            &mut converted.0,
                            &mut images,
                            &mut materials,
                            &sub.material,
                            sub.slot.as_ref(),
                            Shading::Sky,
                        );
                        parent.spawn((
                            Mesh3d(meshes.add(convert::submesh_mesh(sub, false))),
                            MeshMaterial3d(material),
                            bevy::light::NotShadowCaster,
                            bevy::light::NotShadowReceiver,
                        ));
                    }
                }
            }
        });
}

fn spawn_sun(mut commands: Commands, stage: Res<Stage>) {
    // `light.light` stores the travel direction and colour of the stage sun.
    let (direction, color) = match stage.0.db.get("light.light").map(parse_light) {
        Some(Ok(light)) if light.kind == 0 => (Vec3::from(light.vector), Vec3::from(light.color)),
        _ => (Vec3::new(-0.5, -0.6, 0.6), Vec3::ONE),
    };
    let direction = direction.normalize_or(Vec3::NEG_Y);
    let warm = color.lerp(Vec3::ONE, 0.35);
    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            color: Color::srgb(warm.x, warm.y, warm.z),
            illuminance: 9_000.0,
            shadow_maps_enabled: std::env::var("UNLEASHED_NO_SHADOWS").is_err(),
            ..default()
        },
        Transform::default().looking_to(direction, Vec3::Y),
        CascadeShadowConfigBuilder {
            num_cascades: 4,
            minimum_distance: 0.3,
            first_cascade_far_bound: 25.0,
            maximum_distance: 400.0,
            ..default()
        }
        .build(),
        // Also lights the shadow-caster layer (guest mode: Sonic's shadow-only meshes).
        bevy::camera::visibility::RenderLayers::from_layers(&[0, crate::chr::SHADOW_CASTER_LAYER]),
    ));
}

fn follow_camera_with_sky(
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut sky: Query<&mut Transform, With<SkyDome>>,
) {
    let Ok(camera) = camera.single() else { return };
    for mut transform in &mut sky {
        transform.translation = camera.translation();
    }
}
