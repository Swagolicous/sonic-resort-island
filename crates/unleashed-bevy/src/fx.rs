//! Guest mode: the game's effects.
//!
//! Boost shockwave: SWA::CFxBoostEffect3D (a job of the game's Fx pipeline, run each
//! frame by crates/unleashed-guest-host) with the game's FxBoostBlur pixel program
//! (boost_shockwave.wgsl), as a fullscreen pass on the main camera.
//!
//! Camera motion blur: motion_blur.rs (SWA::CFxMotionBlur).
//!
//! Particle effects (the jump ball, the boost, ...): the game's particle system runs in
//! the guest, and each frame its own render code is run with the draw calls captured
//! (GuestSonic::render_effects). This module turns those draws into entities: billboard
//! draws carry their vertices (world space, as the game built them); mesh draws name a
//! texture, which selects the game model that uses it, placed by the draw's world
//! matrix. Both are shaded by particle.wgsl, the game's two particle programs.

use std::collections::HashMap;

use bevy::{
    asset::{RenderAssetUsages, embedded_asset},
    camera::visibility::NoFrustumCulling,
    core_pipeline::fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin},
    light::NotShadowCaster,
    mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::{
        extract_component::ExtractComponent,
        render_resource::{
            AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, Face, FrontFace, RenderPipelineDescriptor, ShaderType,
            SpecializedMeshPipelineError,
        },
    },
    shader::ShaderRef,
};
use unleashed_assets::mirage::{
    material::{parse_material, parse_texset, parse_texture},
    mesh::{MirageModel, parse_model},
};

use crate::{Stage, world::Converted};

/// The FxBoostBlur constants for this frame (see boost_shockwave.wgsl).
#[derive(Component, ExtractComponent, Clone, Copy, ShaderType, Default)]
pub struct BoostShockwave {
    param: Vec4,
    param2: Vec4,
    param3: Vec4,
    host: Vec4,
}

impl FullscreenMaterial for BoostShockwave {
    fn fragment_shader() -> ShaderRef {
        "embedded://windmill_isle/boost_shockwave.wgsl".into()
    }
}

pub struct FxPlugin;

impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "boost_shockwave.wgsl");
        embedded_asset!(app, "particle.wgsl");
        app.add_plugins((
            FullscreenMaterialPlugin::<BoostShockwave>::default(),
            MaterialPlugin::<ParticleMaterial>::default(),
        ))
            .add_systems(PostUpdate, (update_shockwave, draw_effects));
        if app.world().contains_resource::<crate::resort::ResortSettings>() {
            eprintln!("[resort-island] motion blur disabled; render pass omitted");
        } else {
            app.add_plugins(crate::motion_blur::MotionBlurPlugin);
        }
    }
}

fn update_shockwave(
    mut commands: Commands,
    guest: Option<NonSend<crate::guest::GuestPlayer>>,
    camera: Query<Entity, With<Camera3d>>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    let (Some(guest), Ok(camera)) = (guest, camera.single()) else { return };
    let aspect = window.single().map_or(16.0 / 9.0, |w| w.width() / w.height().max(1.0));
    // UNLEASHED_SHOCKWAVE=<radius>: draw the ring at a fixed radius around the screen centre
    // with the retail constants (to look at the pass without timing a boost).
    if let Some(radius) = std::env::var("UNLEASHED_SHOCKWAVE").ok().and_then(|s| s.parse::<f32>().ok()) {
        commands.entity(camera).insert(BoostShockwave {
            param: Vec4::new(0.5, 2.0, 1.0, 0.0),
            param2: Vec4::new(0.5, 0.5, 0.5, radius),
            param3: Vec4::new(0.06, 0.09, 0.0, 0.0),
            host: Vec4::new(aspect, 1.0, 0.0, 0.0),
        });
        return;
    }
    let job = match guest.sonic.boost_shockwave() {
        Some(c) => BoostShockwave {
            param: Vec4::from(c[0]),
            param2: Vec4::from(c[1]),
            param3: Vec4::from(c[2]),
            host: Vec4::new(aspect, 1.0, 0.0, 0.0),
        },
        None => BoostShockwave { host: Vec4::new(aspect, 0.0, 0.0, 0.0), ..default() },
    };
    commands.entity(camera).insert(job);
}

/// The game's particle programs (particle.wgsl).
#[derive(Asset, TypePath, AsBindGroup, Clone)]
#[bind_group_data(ParticleKey)]
pub struct ParticleMaterial {
    /// (g_ParticleParam.rgb, g_Diffuse.a * opacity * g_ParticleParam.a) for a mesh
    /// particle; ones for a billboard.
    #[uniform(0)]
    pub particle: Vec4,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
    /// D3DRS_DESTBLEND is ONE (additive) rather than INVSRCALPHA.
    pub additive: bool,
    /// D3DRS_CULLMODE: 0 none, 1 counter-clockwise faces culled, 2 clockwise.
    pub cull: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParticleKey {
    additive: bool,
    cull: u8,
}

impl From<&ParticleMaterial> for ParticleKey {
    fn from(m: &ParticleMaterial) -> Self {
        Self { additive: m.additive, cull: m.cull }
    }
}

impl Material for ParticleMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://windmill_isle/particle.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "embedded://windmill_isle/particle.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        // SRCBLEND is SRCALPHA in both cases.
        if self.additive { AlphaMode::Add } else { AlphaMode::Blend }
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
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        // The game's models are wound clockwise (as chr.rs).
        descriptor.primitive.front_face = FrontFace::Cw;
        descriptor.primitive.cull_mode = match key.bind_group_data.cull {
            1 => Some(Face::Back),
            2 => Some(Face::Front),
            _ => None,
        };
        // The game's blend, exactly: SRCALPHA into ONE (additive) or INVSRCALPHA. (Bevy's
        // own Add mode expects premultiplied output, which these programs do not write.)
        let color = BlendComponent {
            src_factor: BlendFactor::SrcAlpha,
            dst_factor: if key.bind_group_data.additive { BlendFactor::One } else { BlendFactor::OneMinusSrcAlpha },
            operation: BlendOperation::Add,
        };
        let alpha = BlendComponent { src_factor: BlendFactor::Zero, dst_factor: BlendFactor::One, operation: BlendOperation::Add };
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(BlendState { color, alpha });
            }
        }
        Ok(())
    }
}

/// An entity drawing one captured effect draw; replaced every frame.
#[derive(Component)]
struct EffectDrawEntity;

/// The game's effect models, by the diffuse texture they use.
#[derive(Default)]
struct EffectAssets {
    /// texture (picture) name -> (model, submesh, strip length, vertex size)
    by_texture: Option<HashMap<String, Vec<(String, usize, usize, usize)>>>,
    models: HashMap<String, MirageModel>,
    meshes: HashMap<(String, usize, usize), Handle<Mesh>>,
    /// guest texture object -> its image (None when no resource matched its names)
    textures: HashMap<u32, Option<(String, Handle<Image>)>>,
    reported: Vec<u32>,
    picked: Vec<(u32, u32)>,
}

fn draw_effects(
    mut commands: Commands,
    guest: Option<NonSendMut<crate::guest::GuestPlayer>>,
    stage: Res<Stage>,
    mut converted: ResMut<Converted>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ParticleMaterial>>,
    mut assets: Local<EffectAssets>,
    old: Query<Entity, With<EffectDrawEntity>>,
) {
    for e in &old {
        commands.entity(e).despawn();
    }
    let Some(mut guest) = guest else { return };
    let draws = guest.sonic.render_effects();
    if draws.is_empty() {
        return;
    }
    let db = &stage.0.db;
    let assets = &mut *assets;
    // Effect models (eff_*.model) indexed by their diffuse texture, once.
    let by_texture = assets.by_texture.get_or_insert_with(|| {
        let mut index: HashMap<String, Vec<(String, usize, usize, usize)>> = HashMap::new();
        let names: Vec<String> = db.names().filter(|n| n.starts_with("eff_") && n.ends_with(".model")).map(str::to_owned).collect();
        for name in names {
            let Some(Ok(model)) = db.get(&name).map(parse_model) else { continue };
            for (i, sub) in model.submeshes.iter().enumerate() {
                let Some(Ok(material)) = db.get(&format!("{}.material", sub.material)).map(parse_material) else { continue };
                let textures = db.get(&format!("{}.texset", material.texset)).map(parse_texset).and_then(Result::ok).unwrap_or_default();
                for texture in textures {
                    if let Some(Ok(t)) = db.get(&format!("{texture}.texture")).map(parse_texture) {
                        if t.kind == "diffuse" {
                            index.entry(t.dds).or_default().push((name.clone(), i, sub.strip_len, sub.vertex_size));
                        }
                    }
                }
            }
            assets.models.insert(name, model);
        }
        info!("effects: {} effect models indexed by {} textures", assets.models.len(), index.len());
        index
    });

    for d in &draws {
        let draw = &d.draw;
        // The texture: the resource whose name ends one of the strings found next to it.
        let texture = assets.textures.entry(draw.textures[0]).or_insert_with(|| {
            d.textures.iter().find_map(|candidate| {
                (0..candidate.len()).filter(|&i| candidate.is_char_boundary(i)).map(|i| &candidate[i..]).find(|name| db.get(&format!("{name}.dds")).is_some()).map(str::to_owned)
            }).and_then(|name| crate::convert::load_dds(db, &mut converted.0, &mut images, &name, false).map(|h| (name, h)))
        });
        let Some((texture_name, image)) = texture.clone() else {
            if !assets.reported.contains(&draw.textures[0]) {
                assets.reported.push(draw.textures[0]);
                warn!("effects: no texture resource for {:08X} (names {:?})", draw.textures[0], d.textures);
            }
            continue;
        };
        let state = |s: u32| draw.render_states.get(&s).copied().unwrap_or(0);
        let additive = state(76) == 1; // D3DRS_DESTBLEND == D3DBLEND_ONE
        if !draw.vertices.is_empty() && draw.primitive == 13 && draw.stride >= 24 {
            // Billboards: quads of {position f32x3, texcoord i16x2 (normalised), a second
            // texcoord the same way, colour ARGB8, 4 more bytes} = 28 bytes.
            let stride = draw.stride as usize;
            let word = |v: &[u8], o: usize| u32::from_be_bytes([v[o], v[o + 1], v[o + 2], v[o + 3]]);
            let (mut positions, mut uvs, mut colors, mut indices) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
            for (i, v) in draw.vertices.chunks_exact(stride).enumerate() {
                positions.push([f32::from_bits(word(v, 0)), f32::from_bits(word(v, 4)), f32::from_bits(word(v, 8))]);
                // TEXCOORD0 is two normalised 16-bit values (0x7FFF = 1).
                let short = |o: usize| i16::from_be_bytes([v[o], v[o + 1]]) as f32 / 32767.0;
                uvs.push([short(12), short(14)]);
                let c = word(v, 20);
                colors.push([(c >> 16 & 255) as f32 / 255.0, (c >> 8 & 255) as f32 / 255.0, (c & 255) as f32 / 255.0, (c >> 24) as f32 / 255.0]);
                if i % 4 == 3 {
                    let q = (i - 3) as u32;
                    indices.extend_from_slice(&[q, q + 1, q + 2, q, q + 2, q + 3]);
                }
            }
            let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
                .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
                .with_inserted_indices(Indices::U32(indices));
            commands.spawn((
                EffectDrawEntity,
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(materials.add(ParticleMaterial { particle: Vec4::ONE, texture: image, additive, cull: 0 })),
                Transform::default(),
                NotShadowCaster,
                NoFrustumCulling,
            ));
        } else if draw.indexed && draw.vertex_constants.len() > 187 && draw.pixel_constants.len() > 150 {
            // A mesh particle: the model that uses this texture, under g_MtxWorld (c8..c11).
            let Some(candidates) = by_texture.get(&texture_name) else {
                if !assets.reported.contains(&draw.vertex_buffer) {
                    assets.reported.push(draw.vertex_buffer);
                    warn!("effects: no effect model uses texture {texture_name}");
                }
                continue;
            };
            // The game's draw call takes the strip's index count; the vertex size settles ties.
            let wanted = draw.primitive_count as usize;
            let stride = draw.stride as usize;
            let Some((model, submesh, strip, size)) =
                candidates.iter().min_by_key(|(_, _, strip, size)| (strip.abs_diff(wanted), *size != stride)).cloned()
            else {
                continue;
            };
            if !assets.picked.contains(&(draw.vertex_buffer, draw.primitive_count)) {
                assets.picked.push((draw.vertex_buffer, draw.primitive_count));
                info!("effects: mesh draw of {wanted} strip indices (stride {stride}) with {texture_name} -> {model} submesh {submesh} ({strip} indices, vertex size {size})");
            }
            let channel = (draw.vertex_constants[187][0].trunc() as usize).min(3); // mrgTexcoordIndex.x
            let handle = assets.meshes.entry((model.clone(), submesh, channel)).or_insert_with(|| {
                let sub = &assets.models[&model].submeshes[submesh];
                let channel = if sub.uvs[channel].is_empty() { 0 } else { channel };
                let mut mesh = crate::convert::submesh_mesh_uv(sub, true, channel);
                if mesh.attribute(Mesh::ATTRIBUTE_COLOR).is_none() {
                    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.0f32; 4]; sub.positions.len()]);
                }
                meshes.add(mesh)
            }).clone();
            let (vc, pc) = (&draw.vertex_constants, &draw.pixel_constants);
            let world = Mat4::from_cols(Vec4::from(vc[8]), Vec4::from(vc[9]), Vec4::from(vc[10]), Vec4::from(vc[11]));
            let param = Vec4::from(pc[150]);
            let particle = param.truncate().extend(pc[16][3] * pc[21][0] * param.w);
            let cull = match state(56) { 0 => 0, 6 => 1, _ => 2 };
            commands.spawn((
                EffectDrawEntity,
                Mesh3d(handle),
                MeshMaterial3d(materials.add(ParticleMaterial { particle, texture: image, additive, cull })),
                Transform::from_matrix(world),
                NotShadowCaster,
                NoFrustumCulling,
            ));
        }
    }
}
