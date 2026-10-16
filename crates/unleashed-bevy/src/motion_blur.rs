//! The game's camera motion blur (see `motion_blur.wgsl`): the Fx pipeline job
//! SWA::CFxMotionBlur, as a pass on the 3D camera before the HDR chain. The game runs it
//! whenever the camera moves; a boost is where it shows most.
//!
//! g_BlurRate.x is ms_BlurRate (0x83302DCC, 1.0 in retail) times the job's own scale
//! (1 unless a message overrides it); the job is on while ms_IsEnable (0x83302D80) is.
//! UNLEASHED_NO_MOTION_BLUR turns the pass off.

use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    core_pipeline::{Core3dSystems, FullscreenShader, schedule::Core3d, tonemapping::tonemapping},
    prelude::*,
    render::{
        RenderApp, RenderStartup,
        extract_component::{ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
        render_resource::{
            binding_types::{sampler, texture_2d, texture_depth_2d, texture_depth_2d_multisampled, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice, ViewQuery},
        view::{ViewDepthTexture, ViewTarget},
    },
};

use crate::player::Sonic;

/// ms_BlurRate.
const BLUR_RATE: f32 = 1.0;
/// The part of the picture that moves with Sonic: a cylinder on his feet along his up
/// axis, this wide and this tall, starting this far above the ground (metres).
const SONIC_RADIUS: f32 = 0.6;
const SONIC_HEIGHT: f32 = 1.4;
const SONIC_FLOOR: f32 = 0.06;

pub struct MotionBlurPlugin;

impl Plugin for MotionBlurPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "motion_blur.wgsl");
        app.add_plugins((ExtractComponentPlugin::<GameMotionBlur>::default(), UniformComponentPlugin::<GameMotionBlur>::default()))
            .add_systems(PostUpdate, update_motion_blur.after(TransformSystems::Propagate));
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else { return };
        render_app
            .add_systems(RenderStartup, init_pipelines)
            .add_systems(Core3d, motion_blur.before(tonemapping).before(crate::post::game_post).in_set(Core3dSystems::PostProcess));
    }
}

/// This frame's constants (the uniform motion_blur.wgsl reads).
#[derive(Component, ExtractComponent, Clone, Copy, ShaderType, Default)]
pub struct GameMotionBlur {
    world_from_clip: Mat4,
    prev_clip_from_world: Mat4,
    /// Sonic's feet and the cylinder's radius; his up axis and its height; how far he
    /// moved in the last game frame and where the cylinder starts above his feet.
    sonic: Vec4,
    sonic_up: Vec4,
    sonic_delta: Vec4,
    params: Vec4,
}

/// The camera and Sonic as of the last two game frames. The game steps 60 times a second
/// whatever the display does, and the blur is one game frame's motion: frames drawn
/// between two steps keep the same pair.
#[derive(Clone, Copy)]
struct Steps {
    clip_from_world: [Mat4; 2],
    feet: [Vec3; 2],
    /// When the newer of the two was seen, and how many game frames lie between them
    /// (more than one when the display is slower than the game).
    seen: f32,
    frames: f32,
}

fn update_motion_blur(
    mut commands: Commands,
    mut cameras: Query<(Entity, &Camera, &GlobalTransform, &Projection, &mut Camera3d)>,
    sonic: Query<&Sonic>,
    time: Res<Time>,
    mut steps: Local<Option<Steps>>,
    mut off: Local<Option<bool>>,
    resort: Option<Res<crate::resort::ResortSettings>>,
) {
    let off = *off.get_or_insert_with(|| std::env::var_os("UNLEASHED_NO_MOTION_BLUR").is_some());
    let Ok((entity, camera, transform, projection, mut camera_3d)) = cameras.single_mut() else { return };
    // Resort Island uses interpolated display poses. Keep motion blur completely
    // out of its render graph, rather than running a zero-strength fullscreen pass.
    if off || resort.is_some() {
        commands.entity(entity).remove::<GameMotionBlur>();
        *steps = None;
        return;
    }
    // The pass reads the depth the scene was drawn with.
    let usages = TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
    if camera_3d.depth_texture_usages.0 != usages.bits() {
        camera_3d.depth_texture_usages = usages.into();
    }
    let clip_from_view = match projection {
        Projection::Perspective(p) => bevy::camera::CameraProjection::get_clip_from_view(p),
        _ => return,
    };
    let clip_from_world = clip_from_view * Mat4::from(transform.affine()).inverse();
    let (feet, up) = match sonic.single() {
        Ok(s) => match s.guest_pose {
            Some((feet, rotation)) => (feet, rotation * Vec3::Y),
            None => (s.center - s.up * 0.5, s.up),
        },
        Err(_) => (Vec3::ZERO, Vec3::Y),
    };
    let now = time.elapsed_secs();
    let mut pair = steps.unwrap_or(Steps { clip_from_world: [clip_from_world; 2], feet: [feet; 2], seen: now, frames: 1.0 });
    if pair.clip_from_world[1] != clip_from_world || pair.feet[1] != feet {
        pair = Steps {
            clip_from_world: [pair.clip_from_world[1], clip_from_world],
            feet: [pair.feet[1], feet],
            seen: now,
            frames: ((now - pair.seen) * 60.0).round().max(1.0),
        };
    }
    *steps = Some(pair);
    // A respawn or a warp is not motion.
    let delta = pair.feet[1] - pair.feet[0];
    let delta = if delta.length() < 5.0 { delta } else { Vec3::ZERO };
    let size = camera.physical_viewport_size().map_or(Vec2::new(1280.0, 720.0), |s| s.as_vec2());
    commands.entity(entity).insert(GameMotionBlur {
        world_from_clip: clip_from_world.inverse(),
        prev_clip_from_world: pair.clip_from_world[0],
        sonic: feet.extend(SONIC_RADIUS),
        sonic_up: up.extend(SONIC_HEIGHT),
        sonic_delta: delta.extend(SONIC_FLOOR),
        params: Vec4::new(BLUR_RATE, 1.0 / pair.frames, size.x, size.y),
    });
}

#[derive(Resource)]
struct MotionBlurPipelines {
    /// Bind group layouts and pipelines for a single-sample and a multisampled depth
    /// buffer; each with a pipeline for an HDR and for a display-format target.
    layouts: [BindGroupLayoutDescriptor; 2],
    pipelines: [[CachedRenderPipelineId; 2]; 2],
    sampler: Sampler,
}

fn init_pipelines(mut commands: Commands, device: Res<RenderDevice>, fullscreen: Res<FullscreenShader>, assets: Res<AssetServer>, cache: Res<PipelineCache>) {
    let layout = |multisampled: bool| {
        BindGroupLayoutDescriptor::new(
            "game_motion_blur_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    if multisampled { texture_depth_2d_multisampled() } else { texture_depth_2d() },
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<GameMotionBlur>(true),
                ),
            ),
        )
    };
    let layouts = [layout(false), layout(true)];
    let shader: Handle<Shader> = load_embedded_asset!(assets.as_ref(), "motion_blur.wgsl");
    let pipeline = |multisampled: bool, format: TextureFormat| {
        cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("game_motion_blur".into()),
            layout: vec![layouts[multisampled as usize].clone()],
            vertex: fullscreen.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: shader.clone(),
                shader_defs: if multisampled { vec!["MULTISAMPLED".into()] } else { Vec::new() },
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState { format, blend: None, write_mask: ColorWrites::ALL })],
                ..default()
            }),
            ..default()
        })
    };
    let formats = [TextureFormat::Rgba8UnormSrgb, ViewTarget::TEXTURE_FORMAT_HDR];
    let pipelines = [false, true].map(|multisampled| formats.map(|format| pipeline(multisampled, format)));
    let sampler = device.create_sampler(&SamplerDescriptor {
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        ..default()
    });
    commands.insert_resource(MotionBlurPipelines { layouts, pipelines, sampler });
}

fn motion_blur(
    view: ViewQuery<(&ViewTarget, &ViewDepthTexture, &DynamicUniformIndex<GameMotionBlur>)>,
    pipelines: Option<Res<MotionBlurPipelines>>,
    cache: Res<PipelineCache>,
    uniforms: Res<ComponentUniforms<GameMotionBlur>>,
    mut ctx: RenderContext,
) {
    let (target, depth, uniform_index) = view.into_inner();
    let Some(pipelines) = pipelines else { return };
    if !depth.texture.usage().contains(TextureUsages::TEXTURE_BINDING) {
        return;
    }
    let multisampled = (depth.texture.sample_count() > 1) as usize;
    let hdr = (target.main_texture_format() == ViewTarget::TEXTURE_FORMAT_HDR) as usize;
    let Some(pipeline) = cache.get_render_pipeline(pipelines.pipelines[multisampled][hdr]) else { return };
    let Some(binding) = uniforms.uniforms().binding() else { return };
    let device = ctx.render_device().clone();
    let layout = cache.get_bind_group_layout(&pipelines.layouts[multisampled]);
    let post = target.post_process_write();
    let group = device.create_bind_group(
        "game_motion_blur_group",
        &layout,
        &BindGroupEntries::sequential((post.source, depth.view(), &pipelines.sampler, binding.clone())),
    );
    let mut pass = ctx.command_encoder().begin_render_pass(&RenderPassDescriptor {
        label: Some("game_motion_blur"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: post.destination,
            depth_slice: None,
            resolve_target: None,
            ops: Operations { load: LoadOp::Load, store: StoreOp::Store },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &group, &[uniform_index.index()]);
    pass.draw(0..3, 0..1);
}
