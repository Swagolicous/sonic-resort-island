//! Sonic in a Minecraft world, as the 2010-rust-rewrite-mashup puts Skate 3 in one:
//! MinecraftOSS (crates/unleashed-minecraft) generates and streams a seeded vanilla
//! world around Sonic; its block collision becomes the `CollisionWorld` his physics
//! already runs on, its section meshes are drawn with Minecraft's lightmap, and its
//! day cycle drives the sun and sky. Sonic is lit as Minecraft lights an entity: the
//! lightmap at his block, plus the day cycle's sun.

use std::collections::HashMap;

use bevy::{
    asset::{RenderAssetUsages, embedded_asset},
    image::ImageSampler,
    light::{NotShadowCaster, NotShadowReceiver},
    mesh::{Indices, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology, VertexFormat},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{
        AsBindGroup, Extent3d, Face, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
        TextureDimension, TextureFormat,
    },
    shader::ShaderRef,
};
use unleashed_minecraft::{McWorld, SectionMesh, SectionPos, Sky, lightmap};

use crate::{
    physics::CollisionWorld,
    player::{PlayerSet, Sonic},
};

/// Sky and block light at each vertex, as levels (0..15).
pub const ATTRIBUTE_MC_LIGHT: MeshVertexAttribute =
    MeshVertexAttribute::new("Mc_Light", 0x4D43_0001, VertexFormat::Float32x2);

const SHADER: &str = "embedded://windmill_isle/minecraft.wgsl";
/// Chunks around Sonic whose blocks are in his collision.
const COLLISION_RADIUS: i32 = 2;
/// The integrated server runs at vanilla's 20 ticks a second.
const TICK: f32 = 1.0 / 20.0;

/// `windmill-isle guest flat`: a flat test world. Sonic runs on one large slab at the
/// height the snapshot has him, drawn as a checkered plane; no blocks are drawn or
/// collided with.
pub fn flat() -> bool {
    std::env::var_os("UNLEASHED_FLAT").is_some()
}

/// Present when the app runs the Minecraft world instead of a stage.
#[derive(Resource)]
pub struct MinecraftMode;

/// The streamed world (not Send: the terrain stream owns worker channels).
pub struct MinecraftWorld(pub McWorld);

#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct McTerrainUniform {
    pub ambient: Vec4,
    pub sky_light: Vec4,
    pub block_tint: Vec4,
    pub fog_color: Vec4,
    pub fog_range: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
#[bind_group_data(McTerrainKey)]
pub struct McTerrainMaterial {
    #[uniform(0)]
    pub uniform: McTerrainUniform,
    #[texture(1)]
    #[sampler(2)]
    pub atlas: Handle<Image>,
    /// Translucent sections (water, glass): blended, not cut out.
    pub blend: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct McTerrainKey {
    blend: bool,
}

impl From<&McTerrainMaterial> for McTerrainKey {
    fn from(m: &McTerrainMaterial) -> Self {
        Self { blend: m.blend }
    }
}

impl Material for McTerrainMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        if self.blend { AlphaMode::Blend } else { AlphaMode::Opaque }
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(2),
            ATTRIBUTE_MC_LIGHT.at_shader_location(3),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        // Translucent faces (water) are seen from both sides, as in vanilla.
        descriptor.primitive.cull_mode = if key.bind_group_data.blend { None } else { Some(Face::Back) };
        Ok(())
    }
}

#[derive(Component)]
struct McSun;

#[derive(Component)]
struct McSection;

/// Render-side state of the world.
#[derive(Default)]
struct Runtime {
    sections: HashMap<SectionPos, Vec<Entity>>,
    opaque: Handle<McTerrainMaterial>,
    translucent: Handle<McTerrainMaterial>,
    tick: f32,
    collision_chunk: Option<(i32, i32)>,
    collision_dirty: bool,
}

pub struct MinecraftPlugin;

impl Plugin for MinecraftPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "minecraft.wgsl");
        // Section meshes stream in all the time. Bevy's mesh allocator starts its vertex
        // slab at 1 MiB and, each time new meshes outgrow it, makes a bigger one and
        // copies everything across: a 60-160 ms frame whenever chunks load. One slab big
        // enough for the view distance up front never has to grow.
        if let Some(render) = app.get_sub_app_mut(bevy::render::RenderApp) {
            let mut settings = bevy::render::mesh::allocator::MeshAllocatorSettings::default();
            settings.slab_allocator_settings.min_slab_size = 256 << 20;
            render.insert_resource(settings);
        }
        app.add_plugins(MaterialPlugin::<McTerrainMaterial>::default())
            .insert_non_send(Runtime::default())
            .add_systems(Startup, setup)
            .add_systems(Update, (stream, guest_blocks, update_lighting).chain().before(PlayerSet))
            .add_systems(PostUpdate, update_sections);
    }
}

fn setup(
    mut commands: Commands,
    world: NonSend<MinecraftWorld>,
    mut runtime: NonSendMut<Runtime>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<McTerrainMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard: ResMut<Assets<StandardMaterial>>,
    guest: Option<NonSend<crate::guest::GuestPlayer>>,
) {
    let atlas = &world.0.atlas.pixels;
    let mut image = Image::new(
        Extent3d { width: atlas.width(), height: atlas.height(), depth_or_array_layers: 1 },
        TextureDimension::D2,
        atlas.as_raw().clone(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    // Minecraft's texels are crisp.
    image.sampler = ImageSampler::nearest();
    let atlas = images.add(image);
    runtime.opaque = materials.add(McTerrainMaterial { uniform: default(), atlas: atlas.clone(), blend: false });
    runtime.translucent = materials.add(McTerrainMaterial { uniform: default(), atlas, blend: true });
    runtime.collision_dirty = true;
    // The sun's shadow map holds Sonic only (sections are NotShadowCaster): his shadow on
    // the blocks (minecraft.wgsl) and on himself.
    commands.spawn((
        Name::new("Minecraft sun"),
        McSun,
        DirectionalLight { shadow_maps_enabled: true, ..default() },
        Transform::default(),
        bevy::light::CascadeShadowConfigBuilder { num_cascades: 2, minimum_distance: 0.3, first_cascade_far_bound: 20.0, maximum_distance: 90.0, ..default() }
            .build(),
        bevy::camera::visibility::RenderLayers::from_layers(&[0, crate::chr::SHADOW_CASTER_LAYER]),
    ));
    info!("minecraft: world seed {} origin {:?}", world.0.seed, world.0.origin);
    if let (true, Some(guest)) = (flat(), guest) {
        // The slab's top, as a plane with 4 m checks.
        let feet = guest.sonic.sonic().body_position;
        let mut checks = Image::new(
            Extent3d { width: 2, height: 2, depth_or_array_layers: 1 },
            TextureDimension::D2,
            vec![70, 110, 70, 255, 110, 150, 110, 255, 110, 150, 110, 255, 70, 110, 70, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        checks.sampler = ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
            address_mode_u: bevy::image::ImageAddressMode::Repeat,
            address_mode_v: bevy::image::ImageAddressMode::Repeat,
            ..bevy::image::ImageSamplerDescriptor::nearest()
        });
        let size = 8000.0;
        let mut plane = Mesh::from(Plane3d::default().mesh().size(size, size));
        if let Some(bevy::mesh::VertexAttributeValues::Float32x2(uvs)) = plane.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
            for uv in uvs {
                *uv = [uv[0] * size / 8.0, uv[1] * size / 8.0];
            }
        }
        commands.spawn((
            Name::new("Flat ground"),
            Mesh3d(meshes.add(plane)),
            MeshMaterial3d(standard.add(StandardMaterial { base_color_texture: Some(images.add(checks)), perceptual_roughness: 1.0, ..default() })),
            Transform::from_xyz(feet[0], feet[1], feet[2]),
        ));
    }
}

/// Server ticks around Sonic; his collision follows the chunks near him.
fn stream(
    time: Res<Time>,
    mut world: NonSendMut<MinecraftWorld>,
    mut runtime: NonSendMut<Runtime>,
    sonic: Query<&Sonic>,
    mut collision: ResMut<CollisionWorld>,
    guest: Option<Res<crate::guest::GuestMode>>,
) {
    let at = sonic.single().map(|s| s.center).unwrap_or(Vec3::ZERO);
    runtime.tick += time.delta_secs();
    let started = std::time::Instant::now();
    while runtime.tick >= TICK {
        runtime.tick -= TICK;
        if world.0.tick(at.to_array()) {
            runtime.collision_dirty = true;
        }
    }
    if started.elapsed().as_millis() >= 8 {
        info!("minecraft: server tick took {:?}", started.elapsed());
    }
    let block = world.0.block_at(at.to_array());
    let chunk = (block.0 >> 4, block.2 >> 4);
    if runtime.collision_chunk != Some(chunk) {
        runtime.collision_dirty = true;
    }
    if runtime.collision_dirty && guest.is_none() {
        let tris = world.0.collision_near(at.to_array(), COLLISION_RADIUS);
        *collision = CollisionWorld::from_triangles(&tris);
        runtime.collision_chunk = Some(chunk);
        runtime.collision_dirty = false;
    }
}

/// Chunk sections (16 blocks a side) around Sonic whose blocks are in the game's Havok
/// world: this many each way horizontally, one each way vertically.
const GUEST_SECTIONS: i32 = 2;
/// Time a frame may spend adding sections Sonic cannot touch yet. Only his own column
/// of sections is added regardless.
const GUEST_BLOCKS_BUDGET: std::time::Duration = std::time::Duration::from_millis(2);
/// Collision filter info of the stage terrain's bodies (hkpMoppBvTreeShape, layer 7):
/// blocks collide with Sonic and the camera as the stage's ground does.
const GUEST_BLOCK_FILTER: u32 = 0x7;

/// Guest mode: the blocks around Sonic as bodies of the game's own Havok world.
#[derive(Default)]
struct GuestBlocks {
    generation: u32,
    havok: u32,
    /// Chunk section (chunk x, block y >> 4, chunk z) -> its fixed boxes.
    sections: HashMap<(i32, i32, i32), Vec<u32>>,
}

/// Guest mode: Sonic's physics and camera are the game's code, colliding in the game's
/// hkpWorld, so the world's blocks go into it. Once, the stage's fixed bodies come out;
/// then every frame the chunk sections around Sonic are kept in it as fixed hkpBoxShape
/// bodies (McWorld::section_boxes), built by the game's own Havok
/// (GuestSonic::add_fixed_box).
fn guest_blocks(
    guest: Option<NonSendMut<crate::guest::GuestPlayer>>,
    mut world: NonSendMut<MinecraftWorld>,
    mut state: Local<GuestBlocks>,
) {
    let Some(mut guest) = guest else { return };
    if state.generation != guest.generation {
        // Respawned: the game's memory is the snapshot's again, with none of our bodies.
        *state = GuestBlocks { generation: guest.generation, ..default() };
    }
    let g = &mut guest.sonic;
    if state.havok == 0 {
        let Some(havok) = g.havok_world() else { return };
        // The stage: its fixed bodies (terrain) and every other body that is not Sonic's
        // own (his move with him: they sit at his position). Stage objects left in the
        // world would be invisible things to run into, answered by object code.
        let at = Vec3::from(g.sonic().position);
        let stage: Vec<u32> = g
            .rigid_bodies(havok)
            .into_iter()
            .filter(|&(body, motion, _, shape)| shape != 0 && (motion == 7 || Vec3::from(g.rigid_body_position(body)).distance(at) > 2.0))
            .map(|r| r.0)
            .collect();
        for &body in &stage {
            g.remove_body(havok, body);
        }
        info!("minecraft: guest Havok world {havok:#X}: {} stage bodies removed", stage.len());
        state.havok = havok;
        if flat() {
            let feet = g.sonic().body_position;
            g.add_fixed_box(havok, [feet[0], feet[1] - 0.5, feet[2]], [4000.0, 0.5, 4000.0], [0.0, 0.0, 0.0, 1.0], 0.0, GUEST_BLOCK_FILTER);
        }
    }
    if flat() {
        return;
    }
    let havok = state.havok;
    let block = world.0.block_at(g.sonic().position);
    let at = (block.0 >> 4, block.1 >> 4, block.2 >> 4);
    let reach = |k: &(i32, i32, i32)| ((k.0 - at.0).abs().max((k.2 - at.2).abs()), (k.1 - at.1).abs());

    // Sections left behind (one section of slack, so a boundary does not thrash).
    // They are out of reach, so there is no hurry: their bodies come out a few at a time
    // (thousands of removals in one frame was a 150 ms stall).
    let removing = std::time::Instant::now();
    let gone: Vec<_> = state.sections.keys().filter(|k| reach(k).0 > GUEST_SECTIONS + 1 || reach(k).1 > 2).copied().collect();
    'gone: for k in gone {
        while let Some(body) = state.sections.get_mut(&k).and_then(Vec::pop) {
            g.remove_body(havok, body);
            if removing.elapsed() >= GUEST_BLOCKS_BUDGET {
                break 'gone;
            }
        }
        state.sections.remove(&k);
    }
    // Sections to add, nearest first; the ones Sonic can touch this frame all at once.
    let mut need = Vec::new();
    for dy in -1..=1 {
        for dz in -GUEST_SECTIONS..=GUEST_SECTIONS {
            for dx in -GUEST_SECTIONS..=GUEST_SECTIONS {
                let k = (at.0 + dx, at.1 + dy, at.2 + dz);
                if !state.sections.contains_key(&k) {
                    need.push(k);
                }
            }
        }
    }
    need.sort_by_key(|k| reach(k).0 * 2 + reach(k).1);
    let started = std::time::Instant::now();
    let mut added = (0, 0);
    for k in need {
        let near = reach(&k).0 == 0;
        if !near && started.elapsed() >= GUEST_BLOCKS_BUDGET {
            break;
        }
        let Some((boxes, ramps)) = world.0.section_boxes(k.0, k.1, k.2) else { continue };
        let mut bodies: Vec<u32> = boxes
            .iter()
            .map(|b| {
                let center = [(b[0] + b[3]) * 0.5, (b[1] + b[4]) * 0.5, (b[2] + b[5]) * 0.5];
                let half = [(b[3] - b[0]) * 0.5, (b[4] - b[1]) * 0.5, (b[5] - b[2]) * 0.5];
                g.add_fixed_box(havok, center, half, [0.0, 0.0, 0.0, 1.0], 0.0, GUEST_BLOCK_FILTER)
            })
            .collect();
        // One-block steps get a slope, so Sonic runs up them (his proxy does not step up).
        bodies.extend(ramps.iter().map(|r| g.add_fixed_box(havok, r.center, r.half_extents, r.rotation, 0.0, GUEST_BLOCK_FILTER)));
        added = (added.0 + 1, added.1 + bodies.len());
        state.sections.insert(k, bodies);
    }
    if started.elapsed().as_millis() >= 8 {
        info!("minecraft: guest blocks: {} sections ({} boxes) took {:?}", added.0, added.1, started.elapsed());
    }
}

fn uniform(sky: &Sky, blend: bool) -> McTerrainUniform {
    let v3 = |a: [f32; 3], w: f32| Vec4::new(a[0], a[1], a[2], w);
    McTerrainUniform {
        ambient: v3(sky.ambient, sky.sky_light_factor),
        sky_light: v3(sky.sky_light_color, sky.block_factor),
        block_tint: v3(sky.block_light_tint, if blend { 0.0 } else { 0.1 }),
        fog_color: v3(sky.fog, 1.0),
        fog_range: Vec4::new(sky.fog_start, sky.fog_end, 0.0, 0.0),
    }
}

/// The day cycle: sky colour, sun, terrain lightmap inputs, and Sonic's light.
fn update_lighting(
    time: Res<Time>,
    mut world: NonSendMut<MinecraftWorld>,
    runtime: NonSend<Runtime>,
    sonic: Query<&Sonic>,
    mut clear: ResMut<ClearColor>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut sun: Query<(&mut DirectionalLight, &mut Transform), With<McSun>>,
    mut materials: ResMut<Assets<McTerrainMaterial>>,
) {
    world.0.day.advance(time.delta_secs() as f64);
    let sky = world.0.sky();
    clear.0 = Color::linear_rgb(sky.sky[0], sky.sky[1], sky.sky[2]);
    for (handle, blend) in [(&runtime.opaque, false), (&runtime.translucent, true)] {
        if let Some(mut m) = materials.get_mut(handle) {
            m.uniform = uniform(&sky, blend);
        }
    }
    // Minecraft lights an entity by the lightmap at its block.
    let at = sonic.single().map(|s| s.center).unwrap_or(Vec3::ZERO);
    let (sky_level, block_level) = world.0.light_at(at.to_array());
    let lit = lightmap(&sky, sky_level as f32, block_level as f32);
    ambient.color = Color::linear_rgb(lit[0], lit[1], lit[2]);
    ambient.brightness = 900.0;
    if let Ok((mut light, mut transform)) = sun.single_mut() {
        let toward = Vec3::from_array(sky.sun_direction).normalize_or(Vec3::Y);
        // The sun only reaches Sonic under open sky, and sets with the day.
        let open = sky_level as f32 / 15.0;
        light.illuminance = 6_000.0 * sky.sky_light_factor * open * toward.y.max(0.0).sqrt();
        light.color = Color::linear_rgb(sky.light[0], sky.light[1], sky.light[2]);
        *transform = Transform::default().looking_to(-toward, Vec3::Z);
    }
}

/// Finished section meshes from the terrain workers become (and replace) entities.
fn update_sections(
    mut commands: Commands,
    mut world: NonSendMut<MinecraftWorld>,
    mut runtime: NonSendMut<Runtime>,
    camera: Query<(&GlobalTransform, &Projection), With<Camera3d>>,
    windows: Query<&Window>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if flat() {
        return;
    }
    let Ok((transform, projection)) = camera.single() else { return };
    let fov = match projection {
        Projection::Perspective(p) => p.fov.to_degrees(),
        _ => 70.0,
    };
    let aspect = windows.iter().next().map(|w| w.width() / w.height().max(1.0)).unwrap_or(16.0 / 9.0);
    let cam = world.0.cull_camera(transform.translation().to_array(), transform.forward().as_vec3().to_array(), fov, aspect);
    let started = std::time::Instant::now();
    let update = world.0.frame(&cam);
    let frame_time = started.elapsed();
    let uploads = update.uploads.len();
    let origin = world.0.origin;
    for pos in update.removed {
        for e in runtime.sections.remove(&pos).unwrap_or_default() {
            commands.entity(e).despawn();
        }
    }
    for (pos, mesh) in update.uploads {
        for e in runtime.sections.remove(&pos).unwrap_or_default() {
            commands.entity(e).despawn();
        }
        if mesh.vertices.is_empty() {
            continue;
        }
        let split = mesh.transparent_start.map_or(mesh.indices.len(), |s| s as usize);
        let mut spawned = Vec::new();
        for (range, material) in [(0..split, runtime.opaque.clone()), (split..mesh.indices.len(), runtime.translucent.clone())] {
            if range.is_empty() {
                continue;
            }
            let handle = meshes.add(section_mesh(&mesh, range, origin));
            spawned.push(
                commands
                    .spawn((McSection, Mesh3d(handle), MeshMaterial3d(material), Transform::default(), NotShadowCaster, NotShadowReceiver))
                    .id(),
            );
        }
        runtime.sections.insert(pos, spawned);
    }
    if started.elapsed().as_millis() >= 8 {
        info!("minecraft: sections: frame() {frame_time:?}, {uploads} uploads, total {:?}", started.elapsed());
    }
}

fn section_mesh(mesh: &SectionMesh, indices: std::ops::Range<usize>, origin: [f64; 3]) -> Mesh {
    let o = [origin[0] as f32, origin[1] as f32, origin[2] as f32];
    let positions: Vec<[f32; 3]> = mesh.vertices.iter().map(|v| [v.position[0] - o[0], v.position[1] - o[1], v.position[2] - o[2]]).collect();
    let uvs: Vec<[f32; 2]> = mesh.vertices.iter().map(|v| v.uv).collect();
    let colors: Vec<[f32; 4]> = mesh.vertices.iter().map(|v| v.color.map(|c| c as f32 / 255.0)).collect();
    // The vertex stores each level times 16.
    let light: Vec<[f32; 2]> = mesh.vertices.iter().map(|v| [v.light[0] as f32 / 16.0, v.light[1] as f32 / 16.0]).collect();
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .with_inserted_attribute(ATTRIBUTE_MC_LIGHT, light)
        .with_inserted_indices(Indices::U32(mesh.indices[indices].to_vec()))
}
