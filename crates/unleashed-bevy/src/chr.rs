//! Sonic drawn with his own shaders (guest mode).
//!
//! Each of Sonic's shader lists (sheets/sonic_materials.csv) is the game's vertex + pixel
//! program pair, composed by re/xenos/gen_chr.py into `chr/<list>.wgsl` (the programs'
//! code untouched; resources moved into the material group). `ChrMaterial` binds what the
//! game binds: the vertex / pixel constant blocks, laid out per program (chr/layout.rs)
//! and filled by constant name, and one texture per sampler slot. The vertex program
//! skins with g_MtxPalette, which holds the game's own skinning matrices for the submesh.

use bevy::{
    asset::uuid_handle,
    mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef, VertexFormat},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError},
    shader::ShaderRef,
};

mod layout;
pub use layout::{ChrLayout, LAYOUTS};

/// The game's vertex inputs, at the locations its vertex programs read them.
pub const ATTR_POSITION: MeshVertexAttribute = MeshVertexAttribute::new("Hh_Position0", 0x5a11_0000, VertexFormat::Float32x4);
/// Normal / tangent / binormal as raw float bits (the programs `asfloat` them when the
/// R11G11B10 specialisation is off, as here).
pub const ATTR_NORMAL: MeshVertexAttribute = MeshVertexAttribute::new("Hh_Normal0", 0x5a11_0001, VertexFormat::Uint32x4);
pub const ATTR_TANGENT: MeshVertexAttribute = MeshVertexAttribute::new("Hh_Tangent0", 0x5a11_0002, VertexFormat::Uint32x4);
pub const ATTR_BINORMAL: MeshVertexAttribute = MeshVertexAttribute::new("Hh_Binormal0", 0x5a11_0003, VertexFormat::Uint32x4);
pub const ATTR_TEXCOORD: [MeshVertexAttribute; 4] = [
    MeshVertexAttribute::new("Hh_TexCoord0", 0x5a11_0004, VertexFormat::Float32x4),
    MeshVertexAttribute::new("Hh_TexCoord1", 0x5a11_0005, VertexFormat::Float32x4),
    MeshVertexAttribute::new("Hh_TexCoord2", 0x5a11_0006, VertexFormat::Float32x4),
    MeshVertexAttribute::new("Hh_TexCoord3", 0x5a11_0007, VertexFormat::Float32x4),
];
pub const ATTR_COLOR: MeshVertexAttribute = MeshVertexAttribute::new("Hh_Color0", 0x5a11_0008, VertexFormat::Float32x4);
pub const ATTR_BLEND_INDICES: MeshVertexAttribute = MeshVertexAttribute::new("Hh_BlendIndices0", 0x5a11_0009, VertexFormat::Uint32x4);
pub const ATTR_BLEND_WEIGHT: MeshVertexAttribute = MeshVertexAttribute::new("Hh_BlendWeight0", 0x5a11_000a, VertexFormat::Float32x4);

/// One shader per list, by index into `LAYOUTS`.
pub const SHADERS: [Handle<Shader>; 5] = [
    uuid_handle!("6f0d4b2e-1a3c-4e5f-9a7b-000000000001"),
    uuid_handle!("6f0d4b2e-1a3c-4e5f-9a7b-000000000002"),
    uuid_handle!("6f0d4b2e-1a3c-4e5f-9a7b-000000000003"),
    uuid_handle!("6f0d4b2e-1a3c-4e5f-9a7b-000000000004"),
    uuid_handle!("6f0d4b2e-1a3c-4e5f-9a7b-000000000005"),
];

pub struct ChrPlugin;

impl Plugin for ChrPlugin {
    fn build(&self, app: &mut App) {
        let sources: [(&str, &str); 5] = [
            ("SonicCloth_dsn_b", include_str!("chr/SonicCloth_dsn_b.wgsl")),
            ("SonicEnamel_dspe_bc", include_str!("chr/SonicEnamel_dspe_bc.wgsl")),
            ("SonicEye_dpne_bcu", include_str!("chr/SonicEye_dpne_bcu.wgsl")),
            ("SonicMetal_dpe_b", include_str!("chr/SonicMetal_dpe_b.wgsl")),
            ("SonicSkin_dspf_b", include_str!("chr/SonicSkin_dspf_b.wgsl")),
        ];
        let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
        for (i, l) in LAYOUTS.iter().enumerate() {
            let (_, src) = sources.iter().find(|(n, _)| *n == l.shader).expect("shader for layout");
            let _ = shaders.insert(SHADERS[i].id(), Shader::from_wgsl(*src, format!("chr/{}.wgsl", l.shader)));
        }
        app.add_plugins(MaterialPlugin::<ChrMaterial>::default())
            .init_resource::<ChrSkin>()
            .add_systems(PostUpdate, update_constants);
    }
}

/// A constant block: the program's struct is a prefix of this (vec4-aligned members).
#[derive(ShaderType, Clone, Copy)]
pub struct ConstBlock {
    pub v: [Vec4; 128],
}

impl Default for ConstBlock {
    fn default() -> Self {
        ConstBlock { v: [Vec4::ZERO; 128] }
    }
}

/// SharedConstants as the converted programs declare it.
#[derive(ShaderType, Clone, Copy, Default)]
pub struct Shared {
    pub booleans: u32,
    pub swapped_texcoords: u32,
    pub half_pixel_offset: Vec2,
    pub alpha_threshold: f32,
    pub spec_constants: u32,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
#[bind_group_data(ChrKey)]
pub struct ChrMaterial {
    #[uniform(0)]
    pub vertex: ConstBlock,
    #[uniform(1)]
    pub pixel: ConstBlock,
    #[uniform(2)]
    pub vertex_shared: Shared,
    #[uniform(3)]
    pub pixel_shared: Shared,
    /// Slot 0: g_ShadowMapSampler.
    #[texture(16)]
    #[sampler(19)]
    pub shadow: Option<Handle<Image>>,
    /// Slot 1: sampDif.
    #[texture(20)]
    #[sampler(23)]
    pub diffuse: Option<Handle<Image>>,
    /// Slot 2: sampEnv (2D or cube, by program).
    #[texture(24)]
    #[sampler(27)]
    pub env_2d: Option<Handle<Image>>,
    #[texture(26, dimension = "cube")]
    pub env_cube: Option<Handle<Image>>,
    /// Slot 3: sampFalloff.
    #[texture(28)]
    #[sampler(31)]
    pub falloff: Option<Handle<Image>>,
    /// Slot 4: sampNrm.
    #[texture(32)]
    #[sampler(35)]
    pub normal: Option<Handle<Image>>,
    /// Slot 5: sampPower.
    #[texture(36)]
    #[sampler(39)]
    pub power: Option<Handle<Image>>,
    /// Slot 6: sampSpec.
    #[texture(40)]
    #[sampler(43)]
    pub specular: Option<Handle<Image>>,
    /// Index into LAYOUTS / SHADERS.
    pub program: u8,
    pub alpha_mode: AlphaMode,
    pub double_sided: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChrKey {
    program: u8,
    double_sided: bool,
}

impl From<&ChrMaterial> for ChrKey {
    fn from(m: &ChrMaterial) -> Self {
        ChrKey { program: m.program, double_sided: m.double_sided }
    }
}

impl ChrMaterial {
    pub fn layout(&self) -> &'static ChrLayout {
        &LAYOUTS[self.program as usize]
    }

    /// Writes constant `name` (one vec4 per entry) into the vertex and pixel blocks of
    /// this material's programs, wherever they declare it.
    pub fn set(&mut self, name: &str, values: &[Vec4]) {
        let l = self.layout();
        for (block, members) in [(&mut self.vertex, l.vertex), (&mut self.pixel, l.pixel)] {
            if let Some(&(_, count, offset)) = members.iter().find(|(n, _, _)| *n == name) {
                for (i, v) in values.iter().take(count as usize).enumerate() {
                    block.v[offset as usize + i] = *v;
                }
            }
        }
    }

    pub fn has(&self, name: &str) -> bool {
        let l = self.layout();
        l.vertex.iter().chain(l.pixel.iter()).any(|(n, _, _)| *n == name)
    }
}

impl Material for ChrMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADERS[4].clone().into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADERS[4].clone().into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
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
        let shader = SHADERS[key.bind_group_data.program as usize].clone();
        descriptor.vertex.shader = shader.clone();
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader = shader;
        }
        let attrs = [
            ATTR_POSITION.at_shader_location(0),
            ATTR_NORMAL.at_shader_location(1),
            ATTR_TANGENT.at_shader_location(2),
            ATTR_BINORMAL.at_shader_location(3),
            ATTR_TEXCOORD[0].at_shader_location(4),
            ATTR_TEXCOORD[1].at_shader_location(5),
            ATTR_TEXCOORD[2].at_shader_location(6),
            ATTR_TEXCOORD[3].at_shader_location(7),
            ATTR_COLOR.at_shader_location(8),
            ATTR_BLEND_INDICES.at_shader_location(9),
            ATTR_BLEND_WEIGHT.at_shader_location(10),
        ];
        descriptor.vertex.buffers = vec![layout.0.get_layout(&attrs)?];
        // The model's triangles are clockwise-front (Direct3D) and the programs keep the
        // game's clip space, so they stay clockwise on screen.
        descriptor.primitive.front_face = bevy::render::render_resource::FrontFace::Cw;
        descriptor.primitive.cull_mode = if key.bind_group_data.double_sided { None } else { Some(bevy::render::render_resource::Face::Back) };
        Ok(())
    }
}

/// The game's vertex stream for a submesh, in the programs' input format.
pub fn chr_mesh(sub: &unleashed_assets::mirage::mesh::Submesh) -> Mesh {
    use bevy::asset::RenderAssetUsages;
    use bevy::mesh::{Indices, PrimitiveTopology};
    let n = sub.positions.len();
    let bits = |v: &[[f32; 3]], i: usize| -> [u32; 4] {
        let x = v.get(i).copied().unwrap_or([0.0; 3]);
        [x[0].to_bits(), x[1].to_bits(), x[2].to_bits(), 0f32.to_bits()]
    };
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(ATTR_POSITION, sub.positions.iter().map(|p| [p[0], p[1], p[2], 1.0]).collect::<Vec<_>>());
    mesh.insert_attribute(ATTR_NORMAL, (0..n).map(|i| bits(&sub.normals, i)).collect::<Vec<_>>());
    mesh.insert_attribute(ATTR_TANGENT, (0..n).map(|i| bits(&sub.tangents, i)).collect::<Vec<_>>());
    mesh.insert_attribute(ATTR_BINORMAL, (0..n).map(|i| bits(&sub.binormals, i)).collect::<Vec<_>>());
    for k in 0..4 {
        let uv = &sub.uvs[k];
        mesh.insert_attribute(
            ATTR_TEXCOORD[k],
            (0..n).map(|i| uv.get(i).map_or([0.0; 4], |t| [t[0], t[1], 0.0, 0.0])).collect::<Vec<_>>(),
        );
    }
    mesh.insert_attribute(ATTR_COLOR, (0..n).map(|i| sub.colors.get(i).copied().unwrap_or([1.0; 4])).collect::<Vec<_>>());
    // The programs swizzle BLENDINDICES to .wxyz and then pick from that .y, .z, .w, .x
    // for weights x, y, z and the remainder (1 - x - y - z): weight k pairs with index k
    // of the stream, and the fourth index takes the remainder.
    mesh.insert_attribute(
        ATTR_BLEND_INDICES,
        (0..n)
            .map(|i| {
                let j = sub.joints.get(i).copied().unwrap_or([0; 4]);
                [j[0] as u32, j[1] as u32, j[2] as u32, j[3] as u32]
            })
            .collect::<Vec<_>>(),
    );
    mesh.insert_attribute(ATTR_BLEND_WEIGHT, (0..n).map(|i| sub.weights.get(i).copied().unwrap_or([1.0, 0.0, 0.0, 0.0])).collect::<Vec<_>>());
    mesh.insert_indices(Indices::U32(sub.triangles.iter().flat_map(|t| t.iter().copied()).collect()));
    mesh
}

/// Constant registers from glam matrices: register i = column i (the programs multiply
/// row vectors: out = x M[0] + y M[1] + z M[2] + w M[3]).
pub fn mat_registers(m: Mat4) -> [Vec4; 4] {
    [m.x_axis, m.y_axis, m.z_axis, m.w_axis]
}

/// Render layer of Sonic's shadow-only meshes: no camera draws it, the sun's shadow maps
/// do (the sun is on layers 0 and this one). His own programs skin in their vertex stage
/// from g_MtxPalette, which Bevy's shadow pass cannot run, so anim.rs keeps a second copy
/// of each submesh there, skinned by Bevy from the joints the game's matrices drive.
pub const SHADOW_CASTER_LAYER: usize = 2;

/// Guest mode: where the game's skinning matrices for SonicRoot.model are (set by anim).
#[derive(Resource, Default)]
pub struct ChrSkin(pub Option<u32>);

/// A Sonic submesh drawn with his own programs: its palette (model bone per slot).
#[derive(Component)]
pub struct ChrPart {
    pub palette: Vec<u8>,
}

/// Sonic's material from its `.material` / texset: the program of its shader list, its
/// parameters by constant name, and its textures in Hedgehog's sampler roles (diffuse ->
/// sampDif, specular -> sampSpec, gloss -> sampPower, normal -> sampNrm, reflection ->
/// sampEnv, displacement -> sampFalloff).
pub fn material_for(
    db: &unleashed_assets::resources::ResourceDb,
    cache: &mut crate::convert::MirageCache,
    images: &mut Assets<Image>,
    name: &str,
    slot: Option<&unleashed_assets::mirage::mesh::MeshSlot>,
) -> Option<ChrMaterial> {
    use unleashed_assets::mirage::material::{parse_material, parse_texset, parse_texture};
    use unleashed_assets::mirage::mesh::MeshSlot;
    let mat = db.get(&format!("{name}.material")).map(parse_material)?.ok()?;
    let program = LAYOUTS.iter().position(|l| l.shader_list == mat.shader)? as u8;
    let names = db.get(&format!("{}.texset", mat.texset)).map(parse_texset).and_then(Result::ok).unwrap_or_default();
    let textures: Vec<_> = names
        .iter()
        .filter_map(|n| db.get(&format!("{n}.texture")).map(parse_texture).and_then(Result::ok))
        .collect();
    let mut load = |kind: &str| {
        textures.iter().find(|t| t.kind == kind).and_then(|t| crate::convert::load_dds(db, cache, images, &t.dds, false))
    };
    let diffuse = load("diffuse");
    let specular = load("specular");
    let power = load("gloss");
    let normal = load("normal");
    let falloff = load("displacement");
    let env = load("reflection");
    let (mut env_2d, mut env_cube) = (None, None);
    if let Some(h) = env {
        let cube = images.get(&h).is_some_and(|i| i.texture_descriptor.size.depth_or_array_layers == 6);
        if cube {
            if let Some(mut img) = images.get_mut(&h) {
                img.texture_view_descriptor = Some(bevy::render::render_resource::TextureViewDescriptor {
                    dimension: Some(bevy::render::render_resource::TextureViewDimension::Cube),
                    ..default()
                });
            }
            env_cube = Some(h);
        } else {
            env_2d = Some(h);
        }
    }
    let threshold = mat.alpha_threshold as f32 / 255.0;
    let alpha_mode = match slot {
        Some(MeshSlot::Transparent) | Some(MeshSlot::Special(_)) => AlphaMode::Blend,
        Some(MeshSlot::PunchThrough) => AlphaMode::Mask(threshold),
        _ => AlphaMode::Opaque,
    };
    let mut m = ChrMaterial {
        vertex: ConstBlock::default(),
        pixel: ConstBlock::default(),
        // g_Booleans bit 0: mrgHasBone (Sonic's meshes are skinned).
        vertex_shared: Shared { booleans: 1, ..default() },
        pixel_shared: Shared { booleans: 1, alpha_threshold: threshold, ..default() },
        shadow: None,
        diffuse,
        env_2d,
        env_cube,
        falloff,
        normal,
        power,
        specular,
        program,
        alpha_mode: if mat.additive { AlphaMode::Add } else { alpha_mode },
        double_sided: mat.double_sided,
    };
    // Material parameters: the standard ones are stored under their Mirage names and
    // bound to the g_* constants; the rest already carry the constant's name.
    for p in &mat.params {
        let values: Vec<Vec4> = p.values.iter().map(|v| Vec4::from(*v)).collect();
        let name = p.name.trim();
        let constant = match name {
            "diffuse" => "g_Diffuse",
            "ambient" => "g_Ambient",
            "specular" => "g_Specular",
            "emissive" => "g_Emissive",
            "power_gloss_level" => "g_PowerGlossLevel",
            "opacity_reflection_refraction_spectype" => "g_OpacityReflectionRefractionSpectype",
            other => other,
        };
        m.set(constant, &values);
    }
    info!(
        "chr {name}: {} via {}; textures {:?}; dif {} spec {} power {} nrm {} fal {} env {}/{}; params {:?}",
        mat.shader,
        LAYOUTS[program as usize].shader,
        textures.iter().map(|t| (t.kind.as_str(), t.dds.as_str())).collect::<Vec<_>>(),
        m.diffuse.is_some(), m.specular.is_some(), m.power.is_some(), m.normal.is_some(), m.falloff.is_some(),
        m.env_2d.is_some(), m.env_cube.is_some(),
        mat.params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>()
    );
    Some(m)
}

/// Light scale for Sonic in the Minecraft / flat worlds (no HDR chain there); picked by
/// eye against how the stage's chain renders him, not derived.
const MINECRAFT_EXPOSURE: f32 = 0.45;

/// Per frame: the camera, Sonic's world matrix and skinning palette, and the scene light,
/// into every ChrMaterial's constant blocks.
fn update_constants(
    guest: Option<NonSendMut<crate::guest::GuestPlayer>>,
    skin: Res<ChrSkin>,
    scene: Option<Res<crate::hedgehog::SceneLighting>>,
    minecraft: Option<NonSend<crate::minecraft::MinecraftWorld>>,
    resort: Option<Res<crate::resort::ResortSettings>>,
    world: Res<crate::physics::CollisionWorld>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    parts: Query<(&ChrPart, &MeshMaterial3d<ChrMaterial>)>,
    mut materials: ResMut<Assets<ChrMaterial>>,
) {
    let (Some(mut guest), Some(skin_array)) = (guest, skin.0) else { return };
    // The game's light-field sample at Sonic (82E2B8A8): g_aLightField.
    let body_pos = { let m = guest.sonic.body_matrix(); [m[12], m[13], m[14]] };
    let mut light_field = guest.sonic.light_field_sample(body_pos);
    let mut lights = guest.sonic.lights();
    if let Some(world) = minecraft.as_deref() {
        // In a Minecraft world the stage's light field and sun are not there: Sonic's
        // programs get Minecraft's light instead, the lightmap at his block from every
        // direction (as Minecraft lights an entity) and the day cycle's sun under open sky.
        let sky = world.0.sky();
        let (sky_level, block_level) = world.0.light_at([body_pos[0], body_pos[1] + 0.5, body_pos[2]]);
        let lit = unleashed_minecraft::lightmap(&sky, sky_level as f32, block_level as f32);
        // His programs write scene-referred values for the game's HDR chain (exposure and
        // tone mapping, post.rs), which this world's camera does not have: the light they
        // are given is scaled so a fully lit Sonic lands where that chain would put him.
        let lit = lit.map(|c| c * MINECRAFT_EXPOSURE);
        light_field = Some(([[lit[0], lit[1], lit[2], 1.0]; 6], 1.0));
        let toward = Vec3::from_array(sky.sun_direction).normalize_or(Vec3::Y);
        let sun = MINECRAFT_EXPOSURE * sky.sky_light_factor * (sky_level as f32 / 15.0) * toward.y.max(0.0).sqrt();
        if let Some(l) = lights.as_mut() {
            l.global_direction = (-toward).extend(0.0).to_array();
            l.global_diffuse = [sky.light[0] * sun, sky.light[1] * sun, sky.light[2] * sun, 1.0];
            l.global_specular = l.global_diffuse;
            l.eye_diffuse = l.eye_diffuse.map(|c| c * MINECRAFT_EXPOSURE);
            l.eye_specular = l.eye_specular.map(|c| c * MINECRAFT_EXPOSURE);
        }
    }
    if let Some(r) = resort.as_deref() {
        // The character still uses the original game shaders and skinning. Supply
        // this level's daylight instead of Windmill Isle's captured light field.
        light_field = Some(([r.sonic_ambient.extend(1.0).to_array(); 6], 1.0));
        if let Some(l) = lights.as_mut() {
            l.global_direction = r.sun_direction.extend(0.0).to_array();
            l.global_diffuse = r.sonic_sun.extend(1.0).to_array();
            l.global_specular = (r.sonic_sun * r.sonic_specular).extend(1.0).to_array();
            l.eye_diffuse = l.eye_diffuse.map(|c| c * MINECRAFT_EXPOSURE);
            l.eye_specular = l.eye_specular.map(|c| c * MINECRAFT_EXPOSURE * r.sonic_specular);
        }
    }
    let Some(view) = crate::camera::guest_view(&guest, resort.as_deref(), &world) else { return };
    let aspect = window.single().map_or(16.0 / 9.0, |w| w.width() / w.height().max(1.0));
    let eye = Vec3::from(view.eye);
    let target = Vec3::from(view.target);
    let world_to_view = Mat4::look_at_rh(eye, target, Vec3::Y);
    let projection = Mat4::perspective_rh(view.fov_y, aspect, 0.1, 5000.0);
    let forward = (target - eye).normalize_or(Vec3::NEG_Z);
    let world_rows = if resort.is_some() { guest.rendered_body() } else { guest.sonic.body_matrix() };
    let world: [Vec4; 4] = std::array::from_fn(|r| Vec4::from_slice(&world_rows[4 * r..4 * r + 4]));
    let skins = if resort.is_some() { guest.rendered_skin(skin_array,111) } else { guest.sonic.matrices(skin_array, 111) };
    let (light_dir, light_color, ls) = match scene.as_deref() {
        Some(s) => (s.light_direction, s.light_color, [s.ls_ray_mie, s.ls_const_g, s.ls_far_near, s.ls_color]),
        None => (Vec3::NEG_Y, Vec3::ONE, [Vec4::ZERO; 4]),
    };
    for (part, handle) in &parts {
        let Some(mut m) = materials.get_mut(&handle.0) else { continue };
        m.set("g_MtxProjection", &mat_registers(projection));
        m.set("g_MtxView", &mat_registers(world_to_view));
        m.set("g_MtxWorld", &world);
        // Palette: per slot the 4x3 skinning matrix as three registers, the columns of the
        // game's row-major matrix (register k = (m[0][k], m[1][k], m[2][k], m[3][k])).
        let mut palette = Vec::with_capacity(part.palette.len() * 3);
        for &b in &part.palette {
            let f = skins.get(b as usize).copied().unwrap_or([1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
            for k in 0..3 {
                palette.push(Vec4::new(f[k], f[4 + k], f[8 + k], f[12 + k]));
            }
        }
        m.set("g_MtxPalette", &palette);
        m.set("g_EyePosition", &[eye.extend(1.0)]);
        m.set("g_EyeDirection", &[forward.extend(0.0)]);
        if let Some(l) = lights {
            // The game's own light manager (live): global and eye light.
            m.set("mrgGlobalLight_Direction", &[Vec4::from(l.global_direction)]);
            m.set("mrgGlobalLight_Diffuse", &[Vec4::from(l.global_diffuse)]);
            m.set("mrgGlobalLight_Specular", &[Vec4::from(l.global_specular)]);
            m.set("mrgEyeLight_Diffuse", &[Vec4::from(l.eye_diffuse)]);
            m.set("mrgEyeLight_Specular", &[Vec4::from(l.eye_specular)]);
            m.set("mrgEyeLight_Range", &[Vec4::from(l.eye_range)]);
            m.set("mrgEyeLight_Attribute", &[Vec4::from(l.eye_attribute)]);
        } else {
            m.set("mrgGlobalLight_Direction", &[light_dir.extend(0.0)]);
            m.set("mrgGlobalLight_Diffuse", &[light_color.extend(1.0)]);
            m.set("mrgGlobalLight_Specular", &[light_color.extend(1.0)]);
        }
        m.set("g_LightScattering_Ray_Mie_Ray2_Mie2_", &[ls[0]]);
        m.set("g_LightScattering_ConstG_FogDensity", &[ls[1]]);
        m.set("g_LightScatteringFarNearScale", &[ls[2]]);
        m.set("g_LightScatteringColor", &[ls[3]]);
        if let Some((colours, _shadow)) = light_field {
            let lf: Vec<Vec4> = colours.iter().map(|c| Vec4::from(*c)).collect();
            m.set("g_aLightField", &lf);
        }
        // Shadows: every point projects to depth 0.5, and the programs' shadow-map taps
        // are answered by the host's sun shadow (re/xenos/compose.py host_shadow).
        m.set("g_MtxLightViewProjection", &[Vec4::ZERO, Vec4::ZERO, Vec4::ZERO, Vec4::new(0.5, 0.5, 0.5, 1.0)]);
    }
}
