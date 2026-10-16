// The game's HDR post chain, ported from shader.ar (XenosRecomp output in
// re/xenos/hlsl): ToneMap_MeasureLuminance_Begin/End, ToneMap_CalculateAdaptedLuminance,
// Bloom_BrightPassHDR, a blur standing in for the Bloom/Gaussian/Star filters,
// and ToneMap_ToneMap + Bloom_FinalBlendAdd.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct GamePost {
    // g_MiddleGray_Scale_LuminanceLow_LuminanceHigh
    middle_gray: f32,
    scale: f32,
    luminance_low: f32,
    luminance_high: f32,
    // seconds since last frame; adaptation on (1) / off (0)
    dt: f32,
    adaptation: f32,
    // g_BloomStar_Param1.xy and ms_BloomScale
    bloom_threshold: f32,
    bloom_knee: f32,
    bloom_scale: f32,
    // blur step in texels for the current blur pass
    blur_step: f32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0) var tex0: texture_2d<f32>;
@group(0) @binding(1) var tex1: texture_2d<f32>;
@group(0) @binding(2) var tex2: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;
@group(0) @binding(4) var<uniform> post: GamePost;

const LUMA: vec3<f32> = vec3(0.2125, 0.7154, 0.0722);

// MeasureLuminance_Begin: log2 of clamped luminance, averaged over 9 taps.
@fragment
fn measure(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // One texel of the 64x36 luminance target (post.rs LUM_SIZE).
    let block = 1.0 / vec2(64.0, 36.0);
    var acc = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let uv = in.uv + vec2(f32(x), f32(y)) * block / 3.0;
            let c = saturate(textureSampleLevel(tex0, samp, uv, 0.0).rgb);
            let l = clamp(dot(c, LUMA), post.luminance_low, post.luminance_high);
            acc += log2(l + 0.0001);
        }
    }
    return vec4(acc / 9.0, 0.0, 0.0, 1.0);
}

// MeasureLuminance_End: mean of the log values, back to luminance.
@fragment
fn reduce(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let dims = textureDimensions(tex0);
    var acc = 0.0;
    for (var y = 0u; y < dims.y; y++) {
        for (var x = 0u; x < dims.x; x++) {
            acc += textureLoad(tex0, vec2(x, y), 0).r;
        }
    }
    let l = min(exp2(acc / f32(dims.x * dims.y)), 65504.0);
    return vec4(l, 0.0, 0.0, 1.0);
}

// CalculateAdaptedLuminance: prev + (cur - prev) * (1 - 2^(-2.6231 dt)).
@fragment
fn adapt(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let current = textureLoad(tex0, vec2(0u, 0u), 0).r;
    let previous = textureLoad(tex1, vec2(0u, 0u), 0).r;
    var adapted = current;
    if (post.adaptation > 0.5 && previous > 0.0) {
        adapted = previous + (current - previous) * (1.0 - exp2(-2.6231 * post.dt));
    }
    return vec4(min(adapted, 65504.0), 0.0, 0.0, 1.0);
}

fn exposure_scale(adapted: f32) -> f32 {
    return post.middle_gray / (adapted + 0.001);
}

// Bloom_BrightPassHDR at quarter resolution.
@fragment
fn bright(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(tex0));
    var c = vec3(0.0);
    for (var y = -1; y <= 1; y += 2) {
        for (var x = -1; x <= 1; x += 2) {
            c += textureSampleLevel(tex0, samp, in.uv + vec2(f32(x), f32(y)) * texel, 0.0).rgb;
        }
    }
    c *= 0.25;
    let adapted = textureLoad(tex1, vec2(0u, 0u), 0).r;
    let x = max(c * exposure_scale(adapted) - vec3(post.bloom_threshold), vec3(0.0));
    return vec4(x / (x + vec3(post.bloom_knee)), 1.0);
}

const WEIGHTS: array<f32, 5> = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);

fn blur(uv: vec2<f32>, dir: vec2<f32>) -> vec4<f32> {
    let texel = dir * post.blur_step / vec2<f32>(textureDimensions(tex0));
    var c = textureSampleLevel(tex0, samp, uv, 0.0).rgb * WEIGHTS[0];
    for (var i = 1; i < 5; i++) {
        c += textureSampleLevel(tex0, samp, uv + texel * f32(i), 0.0).rgb * WEIGHTS[i];
        c += textureSampleLevel(tex0, samp, uv - texel * f32(i), 0.0).rgb * WEIGHTS[i];
    }
    return vec4(c, 1.0);
}

@fragment
fn blur_h(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    return blur(in.uv, vec2(1.0, 0.0));
}

@fragment
fn blur_v(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    return blur(in.uv, vec2(0.0, 1.0));
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3(2.4));
    return select(hi, lo, c <= vec3(0.04045));
}

// ToneMap_ToneMap (Reinhard on luminance) + Bloom_FinalBlendAdd. The game
// presents these values as-is, so undo the sRGB encode Bevy applies later.
@fragment
fn final_pass(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let scene = textureSampleLevel(tex0, samp, in.uv, 0.0);
    let adapted = textureLoad(tex2, vec2(0u, 0u), 0).r;
    let l = dot(scene.rgb, LUMA);
    let scaled = l * exposure_scale(adapted);
    let mapped = scaled / (1.0 + scaled);
    var c = scene.rgb * (mapped / max(l, 1e-6));
    c += textureSampleLevel(tex1, samp, in.uv, 0.0).rgb * post.bloom_scale;
    return vec4(srgb_to_linear(saturate(c)), scene.a);
}
