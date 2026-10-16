// The game's camera motion blur (the Fx pipeline job SWA::CFxMotionBlur, sub_82BA9990),
// its two pixel programs in one pass:
//
//   FxZBufferToVelocity.xpu: each pixel's place on screen in the previous frame, from its
//       depth and g_MtxPrevInvViewProjection; the velocity is half the clip-space
//       difference (y flipped), clamped to [-1, 1] (texture coordinate units). The game
//       stores it in a 16-bit-per-axis map; here it is used directly.
//   FxMotionBlur.xpu: 7 taps along the velocity (i / 7 of it, i = 1..7), each kept only
//       where it is not nearer than the pixel (1 - z_i >= 0.99 - z); the average of the
//       kept taps and the pixel, blended in by
//       g_BlurRate.x * saturate(0.125 * |velocity in pixels|), nothing if no tap was kept.
//
// Translated from XenosRecomp's HLSL of the microcode (re/xenos/shader). The game's
// z-buffer is 1 at the near plane (0.1) and 0 far away, which is what this camera's
// depth holds too. Models that move (FxVelocityMap in the game: Sonic) get their own
// velocity: pixels inside a cylinder standing on Sonic's feet move with him.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct MotionBlur {
    world_from_clip: mat4x4<f32>,
    prev_clip_from_world: mat4x4<f32>,
    // Sonic's feet and the radius of the cylinder that moves with him.
    sonic: vec4<f32>,
    // His up axis and the cylinder's height.
    sonic_up: vec4<f32>,
    // How far he moved in the last game frame; w: where the cylinder starts above his feet.
    sonic_delta: vec4<f32>,
    // x: g_BlurRate.x (0 = the job is off); y: the part of the motion between the two
    // matrices that is one game frame's; zw: g_ViewportSize.xy (pixels).
    params: vec4<f32>,
}

@group(0) @binding(0) var sampColor: texture_2d<f32>;
#ifdef MULTISAMPLED
@group(0) @binding(1) var sampZBuffer: texture_depth_multisampled_2d;
#else
@group(0) @binding(1) var sampZBuffer: texture_depth_2d;
#endif
@group(0) @binding(2) var color_sampler: sampler;
@group(0) @binding(3) var<uniform> job: MotionBlur;

fn z_buffer(uv: vec2<f32>) -> f32 {
    let size = vec2<i32>(textureDimensions(sampZBuffer));
    let at = clamp(vec2<i32>(uv * vec2<f32>(size)), vec2<i32>(0), size - vec2<i32>(1));
    return textureLoad(sampZBuffer, at, 0);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let scene = textureSampleLevel(sampColor, color_sampler, in.uv, 0.0);
    if job.params.x == 0.0 {
        return scene;
    }
    let z = z_buffer(in.uv);

    // FxZBufferToVelocity: where this pixel was a frame ago.
    let clip = vec2<f32>(in.uv.x * 2.0 - 1.0, 1.0 - in.uv.y * 2.0);
    var world = job.world_from_clip * vec4<f32>(clip, z, 1.0);
    if world.w > 0.0 {
        let at = world.xyz / world.w;
        let from_feet = at - job.sonic.xyz;
        let height = dot(from_feet, job.sonic_up.xyz);
        let aside = length(from_feet - job.sonic_up.xyz * height);
        if aside < job.sonic.w && height > job.sonic_delta.w && height < job.sonic_up.w {
            world = vec4<f32>(at - job.sonic_delta.xyz, 1.0);
        }
    }
    let before = job.prev_clip_from_world * world;
    let moved = (clip - before.xy / max(before.w, 1.0e-6)) * job.params.y;
    let velocity = clamp(vec2<f32>(0.5 * moved.x, -0.5 * moved.y), vec2<f32>(-1.0), vec2<f32>(1.0));

    // FxMotionBlur.
    var sum = vec4<f32>(scene.xyz, 1.0);
    let limit = 0.99 - z;
    for (var i = 1; i <= 7; i++) {
        let uv = velocity * f32(i) * 0.142857 + in.uv;
        let colour = textureSampleLevel(sampColor, color_sampler, uv, 0.0).xyz;
        let kept = f32(1.0 - z_buffer(uv) >= limit);
        sum += vec4<f32>(colour * kept, kept);
    }
    let any = saturate(sum.w - 1.0);
    let blurred = sum.xyz / sum.w;
    let speed = saturate(0.125 * dot(abs(velocity), job.params.zw));
    let alpha = job.params.x * speed * any;
    return vec4<f32>(mix(scene.xyz, blurred, alpha), scene.w);
}
