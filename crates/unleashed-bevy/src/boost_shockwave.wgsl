// The game's FxBoostBlur pixel program (shader.ar, FxBoostBlur.xpu), drawn by the Fx
// pipeline job SWA::CFxBoostEffect3D while Sonic boosts: a ring expanding from Sonic's
// place on screen that pushes the picture outward and brightens it.
//
// Translated instruction for instruction from XenosRecomp's HLSL of the microcode (20
// instructions; re/xenos/shader/FxBoostBlur.xpu). The constants are the job's: g_Param,
// g_Param2 and g_Param3 as sub_82BA6870 uploads them, and g_CameraNearFarAspect.z.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct BoostShockwave {
    // g_Param: z is the alpha the program writes.
    param: vec4<f32>,
    // g_Param2: xy the ring's centre in texture coordinates, w its radius.
    param2: vec4<f32>,
    // g_Param3: x the ring's width, y how far it pushes the picture.
    param3: vec4<f32>,
    // x: g_CameraNearFarAspect.z (the aspect ratio); y: 1 while the job draws.
    host: vec4<f32>,
}

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> job: BoostShockwave;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    if job.host.y == 0.0 {
        return textureSample(screen_texture, texture_sampler, in.uv);
    }
    // r1.xy = -r0.xy + g_Param2.xy;  r1.z = r1.y * rcp(aspect)
    let to_centre = job.param2.xy - in.uv;
    let y = to_centre.y * (1.0 / job.host.x);
    // r0.z = r1.x * r1.x + r1.z * r1.z
    let d2 = to_centre.x * to_centre.x + y * y;
    // r1.w = sqrt(|r0.z|) - g_Param2.w;  r1.xyz = r1.xzw * (rsqrt, rsqrt, rcp(g_Param3.x))
    let inv = inverseSqrt(abs(d2));
    let direction = vec2<f32>(to_centre.x, y) * inv;
    let ring = (sqrt(abs(d2)) - job.param2.w) * (1.0 / job.param3.x);
    // clamp to [-1, 1];  (x * 0.25 + 0.5) * 6.2831855 - 3.1415927;  cos
    let wave = cos((clamp(ring, -1.0, 1.0) * 0.25 + 0.5) * 6.2831855 - 3.1415927);
    // r0.xy = r1.xy * wave * g_Param3.y + r0.xy
    let uv = direction * wave * job.param3.y + in.uv;
    let colour = textureSample(screen_texture, texture_sampler, uv).xyz;
    // oC0.xyz = (wave * 0.3 + 1) * colour;  oC0.w = g_Param.z
    return vec4<f32>((wave * 0.3 + 1.0) * colour, job.param.z);
}
