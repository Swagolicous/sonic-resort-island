#import bevy_pbr::{forward_io::VertexOutput, mesh_view_bindings::view}

struct SkyParameters {
    horizon: vec4<f32>,
    zenith: vec4<f32>,
    clouds: vec4<f32>,
};
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> sky_parameters: SkyParameters;

fn hash(p: vec2<f32>) -> f32 {
    let q = vec2<i32>(p);
    var n = bitcast<u32>(q.x) * 374761393u + bitcast<u32>(q.y) * 668265263u;
    n = (n ^ (n >> 13u)) * 1274126177u;
    return f32(n ^ (n >> 16u)) / 4294967295.0;
}
fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}
fn fbm(p: vec2<f32>) -> f32 {
    return noise(p) * 0.53 + noise(p * 2.03 + 13.2) * 0.27
        + noise(p * 4.07 - 7.1) * 0.13 + noise(p * 8.11) * 0.07;
}
fn linear(c: vec3<f32>) -> vec3<f32> {
    return pow(c, vec3<f32>(2.2));
}
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let ray = normalize(in.world_position.xyz - view.world_position);
    let elevation = max(ray.y, 0.0);
    let sky = mix(sky_parameters.horizon.xyz, sky_parameters.zenith.xyz,
        pow(clamp(elevation * 1.4, 0.0, 1.0), 0.35));
    let p = ray.xz / max(ray.y + 0.28, 0.12) * sky_parameters.clouds.x + vec2<f32>(3.5, 6.8);
    let density = fbm(p);
    let coverage = smoothstep(0.49 - sky_parameters.clouds.y, 0.65 - sky_parameters.clouds.y, density)
        * smoothstep(-0.04, 0.10, ray.y);
    let white = mix(vec3<f32>(0.73, 0.84, 0.97), vec3<f32>(1.0), smoothstep(0.48, 0.62, density));
    return vec4<f32>(linear(mix(sky, white, coverage)), 1.0);
}
