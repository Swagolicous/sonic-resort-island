//! The game's own surface shading (see `hedgehog.wgsl`): a Bevy material
//! carrying the original shader constants, plus the scene-wide values those
//! shaders read (sun from `light.light`, GI / light-scattering settings from
//! `SceneEffect.prm.xml`, derived the way `sub_82AE…` / `sub_8258D378` do).

use bevy::{
    asset::embedded_asset,
    mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef, VertexFormat},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{
        AsBindGroup, Face, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};
use unleashed_assets::{mirage::material::parse_light, params::parse_param_xml, resources::ResourceDb};

pub const ATTRIBUTE_BINORMAL: MeshVertexAttribute =
    MeshVertexAttribute::new("Hedgehog_Binormal", 0x5EB1_0001, VertexFormat::Float32x3);
pub const ATTRIBUTE_UV_2: MeshVertexAttribute =
    MeshVertexAttribute::new("Hedgehog_Uv2", 0x5EB1_0002, VertexFormat::Float32x2);

const SHADER: &str = "embedded://windmill_isle/hedgehog.wgsl";

pub struct HedgehogPlugin;

impl Plugin for HedgehogPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "hedgehog.wgsl");
        app.add_plugins(MaterialPlugin::<HedgehogMaterial>::default())
            .init_resource::<GiTiles>();
    }
}

/// The shared GI tile buffer (entry 0: no GI).
#[derive(Resource, Clone)]
pub struct GiTiles(pub Handle<bevy::render::storage::ShaderBuffer>);

impl FromWorld for GiTiles {
    fn from_world(world: &mut World) -> Self {
        let mut buffers = world.resource_mut::<Assets<bevy::render::storage::ShaderBuffer>>();
        GiTiles(buffers.add(bevy::render::storage::ShaderBuffer::from(vec![[1.0f32, 1.0, 0.0, 0.0]])))
    }
}

pub mod flags {
    pub const HAS_GI: u32 = 1;
    pub const HAS_NORMAL: u32 = 2;
    pub const HAS_GLOSS: u32 = 4;
    pub const BLEND: u32 = 8;
    pub const UNLIT: u32 = 16;
    pub const NO_SCATTERING: u32 = 32;
}

#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct HedgehogUniform {
    pub diffuse: Vec4,
    pub ambient: Vec4,
    pub specular: Vec4,
    pub power_gloss_level: Vec4,
    pub opacity: Vec4,
    pub gi_atlas: Vec4,
    pub gi0_scale: Vec4,
    pub gi1_scale: Vec4,
    pub light_direction: Vec4,
    pub light_diffuse: Vec4,
    pub light_specular: Vec4,
    pub ls_ray_mie: Vec4,
    pub ls_const_g: Vec4,
    pub ls_far_near: Vec4,
    pub ls_color: Vec4,
    /// x: feature bits (see [`flags`]), y: alpha-test threshold.
    pub flags: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
#[bind_group_data(HedgehogKey)]
pub struct HedgehogMaterial {
    #[uniform(0)]
    pub uniform: HedgehogUniform,
    #[texture(1)]
    #[sampler(2)]
    pub diffuse: Option<Handle<Image>>,
    #[texture(3)]
    #[sampler(4)]
    pub normal: Option<Handle<Image>>,
    #[texture(5)]
    #[sampler(6)]
    pub gloss: Option<Handle<Image>>,
    #[texture(7)]
    #[sampler(8)]
    pub gi: Option<Handle<Image>>,
    #[texture(9)]
    #[sampler(10)]
    pub diffuse2: Option<Handle<Image>>,
    #[texture(11)]
    #[sampler(12)]
    pub normal2: Option<Handle<Image>>,
    /// Every instance's mrgGIAtlasParam (GI uv = TEXCOORD1 * xy + zw), shared by all
    /// materials and indexed by the draw's MeshTag. Keeping it out of the uniform lets
    /// every instance of a model share one material, so the draws batch.
    #[storage(13, read_only)]
    pub gi_tiles: Handle<bevy::render::storage::ShaderBuffer>,
    pub alpha_mode: AlphaMode,
    pub double_sided: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct HedgehogKey {
    double_sided: bool,
}

impl From<&HedgehogMaterial> for HedgehogKey {
    fn from(m: &HedgehogMaterial) -> Self {
        Self { double_sided: m.double_sided }
    }
}

impl Material for HedgehogMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            Mesh::ATTRIBUTE_UV_1.at_shader_location(3),
            Mesh::ATTRIBUTE_TANGENT.at_shader_location(4),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(5),
            ATTRIBUTE_BINORMAL.at_shader_location(6),
            ATTRIBUTE_UV_2.at_shader_location(7),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        descriptor.primitive.cull_mode = if key.bind_group_data.double_sided { None } else { Some(Face::Back) };
        Ok(())
    }
}

/// Scene-wide shader constants for the stage.
#[derive(Resource, Clone, Debug)]
pub struct SceneLighting {
    pub gi0_scale: Vec4,
    pub gi1_scale: Vec4,
    /// Sun travel direction (mrgGlobalLight_Direction).
    pub light_direction: Vec3,
    pub light_color: Vec3,
    pub ls_ray_mie: Vec4,
    pub ls_const_g: Vec4,
    pub ls_far_near: Vec4,
    pub ls_color: Vec4,
    pub hdr: HdrParams,
    pub bloom: BloomParams,
}

/// `HDR/Basic` (g_MiddleGray_Scale_LuminanceLow_LuminanceHigh) + adaptation flag.
#[derive(Clone, Copy, Debug)]
pub struct HdrParams {
    pub middle_gray: f32,
    pub scale: f32,
    pub luminance_low: f32,
    pub luminance_high: f32,
    pub adaptation: bool,
}

/// `BloomStar/Basic`.
#[derive(Clone, Copy, Debug)]
pub struct BloomParams {
    pub enabled: bool,
    /// g_BloomStar_Param1 (threshold, knee, ?).
    pub param1: Vec3,
    pub bloom_scale: f32,
    pub star_scale: f32,
}

impl SceneLighting {
    pub fn load(db: &ResourceDb) -> Self {
        let scene = db.get("SceneEffect.prm.xml").and_then(|b| parse_param_xml(b).ok()).unwrap_or_default();
        let f = |path: &str, default: f32| scene.f32(path).unwrap_or(default);
        let (light_direction, light_color) = match db.get("light.light").map(parse_light) {
            Some(Ok(l)) if l.kind == 0 => (Vec3::from(l.vector).normalize_or(Vec3::NEG_Y), Vec3::from(l.color)),
            _ => (Vec3::new(-0.3, -1.0, -0.2).normalize(), Vec3::ONE),
        };
        // sub_8258D378: g_GI0Scale = (GI_Scale_R/G/B, 0); g_GI1Scale.w = saturation.
        let gi0_scale = Vec4::new(
            f("GI/GI_Adjust_Normal_Mode/GI_Scale_R", 1.0),
            f("GI/GI_Adjust_Normal_Mode/GI_Scale_G", 1.0),
            f("GI/GI_Adjust_Normal_Mode/GI_Scale_B", 1.0),
            0.0,
        );
        let gi1_scale = Vec4::new(1.0, 1.0, 1.0, f("GI/GI_Adjust_Normal_Mode/GI_SaturationScalingRate", 1.0));
        // CEffectLightScattering: Ray2 = 3 Ray / 16pi, Mie2 = Mie / 4pi,
        // ConstG = ((1-g)^2, 1+g^2, -2g, density).
        let ray = f("LightScattering/LightScattering/ms_Ray_Mie_Ray2_Mie2.x", 0.0);
        let mie = f("LightScattering/LightScattering/ms_Ray_Mie_Ray2_Mie2.y", 0.0);
        let g = f("LightScattering/LightScattering/ms_G", 0.0);
        let ls_ray_mie = Vec4::new(ray, mie, ray * 3.0 / 50.265_484, mie / 12.566_371);
        let ls_const_g = Vec4::new(
            (1.0 - g) * (1.0 - g),
            g * g + 1.0,
            g * -2.0,
            f("LightScattering/Fog/ms_ConstG_FogDensity.w", 0.0),
        );
        let ls_far_near = Vec4::new(
            f("LightScattering/Fog/ms_FarNearScale.x", 1000.0),
            f("LightScattering/Fog/ms_FarNearScale.y", 0.0),
            f("LightScattering/Common/ms_FarNearScale.z", 0.0),
            f("LightScattering/LightScattering/ms_FarNearScale.w", 0.0),
        );
        let ls_color = Vec4::new(
            f("LightScattering/Common/ms_Color.x", 0.0),
            f("LightScattering/Common/ms_Color.y", 0.0),
            f("LightScattering/Common/ms_Color.z", 0.0),
            1.0,
        );
        let hdr = HdrParams {
            middle_gray: f("HDR/Basic/Middle_Gray", 0.5),
            scale: f("HDR/Basic/Scale", 1.0),
            luminance_low: f("HDR/Basic/Luminance_Low", 0.25),
            luminance_high: f("HDR/Basic/Luminance_High", 2.0),
            adaptation: scene.bool("HDR/Basic/ms_IsAdaptation").unwrap_or(true),
        };
        let bloom = BloomParams {
            enabled: scene.bool("BloomStar/Basic/ms_IsEnable").unwrap_or(true),
            param1: Vec3::new(
                f("BloomStar/Basic/ms_BloomStar_Param1.x", 1.5),
                f("BloomStar/Basic/ms_BloomStar_Param1.y", 6.0),
                f("BloomStar/Basic/ms_BloomStar_Param1.z", 0.75),
            ),
            bloom_scale: f("BloomStar/Basic/ms_BloomScale", 1.0),
            star_scale: f("BloomStar/Basic/ms_StarScale", 0.0),
        };
        Self {
            gi0_scale,
            gi1_scale,
            light_direction,
            light_color,
            ls_ray_mie,
            ls_const_g,
            ls_far_near,
            ls_color,
            hdr,
            bloom,
        }
    }

    /// The scene part of a material uniform.
    pub fn apply(&self, u: &mut HedgehogUniform) {
        u.gi0_scale = self.gi0_scale;
        u.gi1_scale = self.gi1_scale;
        u.light_direction = self.light_direction.extend(0.0);
        u.light_diffuse = self.light_color.extend(1.0);
        u.light_specular = self.light_color.extend(1.0);
        u.ls_ray_mie = self.ls_ray_mie;
        u.ls_const_g = self.ls_const_g;
        u.ls_far_near = self.ls_far_near;
        u.ls_color = self.ls_color;
    }
}
