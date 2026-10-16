//! The game's HDR post-processing (see `post.wgsl`): luminance measurement,
//! eye adaptation, Reinhard tone mapping and bright-pass bloom, parameterised
//! from `SceneEffect.prm.xml`. Replaces Bevy's tonemapper on the 3D camera.

use std::collections::HashMap;

use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    core_pipeline::{FullscreenShader, schedule::Core3d, tonemapping::tonemapping, Core3dSystems},
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice, ViewQuery},
        view::ViewTarget,
    },
};

use crate::hedgehog::SceneLighting;

pub struct GamePostPlugin;

impl Plugin for GamePostPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "post.wgsl");
        app.add_plugins((
            ExtractComponentPlugin::<GamePost>::default(),
            UniformComponentPlugin::<GamePost>::default(),
        ))
        .add_systems(Update, update_post_settings);
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else { return };
        render_app
            .init_resource::<GamePostTextures>()
            .add_systems(RenderStartup, init_pipelines)
            .add_systems(Render, forget_removed_views.in_set(RenderSystems::Cleanup))
            .add_systems(Core3d, game_post.before(tonemapping).in_set(Core3dSystems::PostProcess));
    }
}

/// Per-camera settings (the uniform the post shaders read).
#[derive(Component, ExtractComponent, Clone, Copy, ShaderType, Default, Debug)]
pub struct GamePost {
    pub middle_gray: f32,
    pub scale: f32,
    pub luminance_low: f32,
    pub luminance_high: f32,
    pub dt: f32,
    pub adaptation: f32,
    pub bloom_threshold: f32,
    pub bloom_knee: f32,
    pub bloom_scale: f32,
    pub blur_step: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

impl GamePost {
    pub fn from_scene(scene: &SceneLighting) -> Self {
        let (hdr, bloom) = (scene.hdr, scene.bloom);
        Self {
            middle_gray: hdr.middle_gray,
            scale: hdr.scale,
            luminance_low: hdr.luminance_low,
            luminance_high: hdr.luminance_high,
            dt: 1.0 / 60.0,
            adaptation: if hdr.adaptation { 1.0 } else { 0.0 },
            bloom_threshold: bloom.param1.x,
            bloom_knee: bloom.param1.y,
            // Bloom_FinalBlendAdd adds the filtered bloom; ms_BloomScale sizes it.
            bloom_scale: if bloom.enabled { bloom.bloom_scale * 0.25 } else { 0.0 },
            blur_step: 1.5,
            _pad0: 0.0,
            _pad1: 0.0,
        }
    }
}

fn update_post_settings(time: Res<Time>, mut posts: Query<&mut GamePost>) {
    for mut post in &mut posts {
        post.dt = time.delta_secs();
    }
}

const LUM_SIZE: (u32, u32) = (64, 36);
const LUM_FORMAT: TextureFormat = TextureFormat::R16Float;
const BLOOM_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
const HDR_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

#[derive(Resource)]
pub(crate) struct GamePostPipelines {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    measure: CachedRenderPipelineId,
    reduce: CachedRenderPipelineId,
    adapt: CachedRenderPipelineId,
    bright: CachedRenderPipelineId,
    blur_h: CachedRenderPipelineId,
    blur_v: CachedRenderPipelineId,
    final_pass: CachedRenderPipelineId,
}

fn init_pipelines(
    mut commands: Commands,
    device: Res<RenderDevice>,
    fullscreen: Res<FullscreenShader>,
    assets: Res<AssetServer>,
    cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "game_post_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                texture_2d(TextureSampleType::Float { filterable: true }),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<GamePost>(true),
            ),
        ),
    );
    let sampler = device.create_sampler(&SamplerDescriptor {
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        ..default()
    });
    let shader: Handle<Shader> = load_embedded_asset!(assets.as_ref(), "post.wgsl");
    let pipeline = |entry: &'static str, format: TextureFormat| {
        cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some(format!("game_post_{entry}").into()),
            layout: vec![layout.clone()],
            vertex: fullscreen.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: shader.clone(),
                entry_point: Some(entry.into()),
                targets: vec![Some(ColorTargetState { format, blend: None, write_mask: ColorWrites::ALL })],
                ..default()
            }),
            ..default()
        })
    };
    commands.insert_resource(GamePostPipelines {
        measure: pipeline("measure", LUM_FORMAT),
        reduce: pipeline("reduce", LUM_FORMAT),
        adapt: pipeline("adapt", LUM_FORMAT),
        bright: pipeline("bright", BLOOM_FORMAT),
        blur_h: pipeline("blur_h", BLOOM_FORMAT),
        blur_v: pipeline("blur_v", BLOOM_FORMAT),
        final_pass: pipeline("final_pass", HDR_FORMAT),
        layout,
        sampler,
    });
}

pub(crate) struct ViewTextures {
    size: (u32, u32),
    luminance: TextureView,
    current: TextureView,
    /// Adapted luminance ping-pong (read one, write the other).
    adapted: [TextureView; 2],
    bloom: [TextureView; 2],
    frame: usize,
}

#[derive(Resource, Default)]
pub(crate) struct GamePostTextures(HashMap<Entity, ViewTextures>);

fn forget_removed_views(mut textures: ResMut<GamePostTextures>, views: Query<Entity, With<GamePost>>) {
    textures.0.retain(|e, _| views.contains(*e));
}

fn texture(device: &RenderDevice, label: &'static str, (w, h): (u32, u32), format: TextureFormat) -> TextureView {
    device
        .create_texture(&TextureDescriptor {
            label: Some(label),
            size: Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&TextureViewDescriptor::default())
}

pub(crate) fn game_post(
    view: ViewQuery<(&ViewTarget, &DynamicUniformIndex<GamePost>)>,
    pipelines: Option<Res<GamePostPipelines>>,
    cache: Res<PipelineCache>,
    uniforms: Res<ComponentUniforms<GamePost>>,
    mut textures: ResMut<GamePostTextures>,
    mut ctx: RenderContext,
) {
    let entity = view.entity();
    let (target, uniform_index) = view.into_inner();
    let Some(pipelines) = pipelines else { return };
    let ids = [
        pipelines.measure,
        pipelines.reduce,
        pipelines.adapt,
        pipelines.bright,
        pipelines.blur_h,
        pipelines.blur_v,
        pipelines.final_pass,
    ];
    let Some(ps) = ids.iter().map(|id| cache.get_render_pipeline(*id)).collect::<Option<Vec<_>>>() else {
        return;
    };
    let [measure, reduce, adapt, bright, blur_h, blur_v, final_pass] = ps[..] else { return };
    let Some(binding) = uniforms.uniforms().binding() else { return };
    if target.main_texture_format() != HDR_FORMAT {
        return;
    }

    let main = target.main_texture();
    let size = (main.width(), main.height());
    let device = ctx.render_device().clone();
    let fresh = !textures.0.get(&entity).is_some_and(|t| t.size == size);
    if fresh {
        let quarter = (size.0 / 4, size.1 / 4);
        textures.0.insert(
            entity,
            ViewTextures {
                size,
                luminance: texture(&device, "game_post_luminance", LUM_SIZE, LUM_FORMAT),
                current: texture(&device, "game_post_current", (1, 1), LUM_FORMAT),
                adapted: [
                    texture(&device, "game_post_adapted_a", (1, 1), LUM_FORMAT),
                    texture(&device, "game_post_adapted_b", (1, 1), LUM_FORMAT),
                ],
                bloom: [
                    texture(&device, "game_post_bloom_a", quarter, BLOOM_FORMAT),
                    texture(&device, "game_post_bloom_b", quarter, BLOOM_FORMAT),
                ],
                frame: 0,
            },
        );
    }
    let tex = textures.0.get_mut(&entity).expect("inserted above");
    tex.frame += 1;
    let (prev, next) = (&tex.adapted[tex.frame % 2], &tex.adapted[(tex.frame + 1) % 2]);

    let layout = cache.get_bind_group_layout(&pipelines.layout);
    let group = |a: &TextureView, b: &TextureView, c: &TextureView| {
        device.create_bind_group(
            "game_post_group",
            &layout,
            &BindGroupEntries::sequential((a, b, c, &pipelines.sampler, binding.clone())),
        )
    };
    let encoder = ctx.command_encoder();
    let mut pass = |label: &'static str, dest: &TextureView, pipeline: &RenderPipeline, bind: &BindGroup| {
        let ops = Operations { load: LoadOp::Load, store: StoreOp::Store };
        let mut rp = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(RenderPassColorAttachment { view: dest, depth_slice: None, resolve_target: None, ops })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        rp.set_pipeline(pipeline);
        rp.set_bind_group(0, bind, &[uniform_index.index()]);
        rp.draw(0..3, 0..1);
    };

    let scene = target.main_texture_view();
    let lum = &tex.luminance;
    // Luminance: measure -> reduce -> adapt. New textures are zeroed by wgpu,
    // and the adapt shader adopts the measured value when the previous is 0.
    pass("game_post_measure", lum, measure, &group(scene, &tex.current, &tex.current));
    pass("game_post_reduce", &tex.current, reduce, &group(lum, lum, lum));
    pass("game_post_adapt", next, adapt, &group(&tex.current, prev, prev));
    // Bloom: bright pass, then two separable blur rounds.
    let [b0, b1] = &tex.bloom;
    pass("game_post_bright", b0, bright, &group(scene, next, next));
    for _ in 0..2 {
        pass("game_post_blur_h", b1, blur_h, &group(b0, b0, b0));
        pass("game_post_blur_v", b0, blur_v, &group(b1, b1, b1));
    }
    // Tone map + bloom into the post-process destination.
    let post = target.post_process_write();
    pass("game_post_final", post.destination, final_pass, &group(post.source, b0, next));
}
