// Hedgehog Engine "Common_d/dn/dp/dpn" and "Blend_dndn" terrain shading,
// ported from the original Xbox 360 shaders (shader.ar, translated with
// XenosRecomp; see re/xenos/hlsl/Common_dpn@@.hlsl and Default_@n@p.hlsl).
//
// Values stay in the game's raw (gamma) space: textures are sampled without
// sRGB decoding, GI colour is stored as sqrt and squared on use, and the
// result goes to the HDR target for the ported tone mapper (post.rs).

#import bevy_pbr::{
    mesh_functions,
    mesh_view_bindings::{view, lights},
    mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT,
    shadows::fetch_directional_shadow,
    view_transformations::position_world_to_clip,
}

struct HedgehogMaterial {
    // Material parameters (g_Diffuse, g_Ambient, g_Specular, g_PowerGlossLevel,
    // g_OpacityReflectionRefractionSpectype).
    diffuse: vec4<f32>,
    ambient: vec4<f32>,
    specular: vec4<f32>,
    power_gloss_level: vec4<f32>,
    opacity: vec4<f32>,
    // mrgGIAtlasParam: GI uv = TEXCOORD1 * xy + zw.
    gi_atlas: vec4<f32>,
    // Scene: g_GI0Scale, g_GI1Scale, mrgGlobalLight_*.
    gi0_scale: vec4<f32>,
    gi1_scale: vec4<f32>,
    light_direction: vec4<f32>,
    light_diffuse: vec4<f32>,
    light_specular: vec4<f32>,
    // g_LightScattering_*: (Ray, Mie, Ray2, Mie2), ((1-g)^2, 1+g^2, -2g, density),
    // (far, near, distance scale, inscatter scale), colour.
    ls_ray_mie: vec4<f32>,
    ls_const_g: vec4<f32>,
    ls_far_near: vec4<f32>,
    ls_color: vec4<f32>,
    // x: feature bits, y: alpha threshold (0 = no alpha test).
    flags: vec4<f32>,
};

const HAS_GI: u32 = 1u;
const HAS_NORMAL: u32 = 2u;
const HAS_GLOSS: u32 = 4u;
const BLEND: u32 = 8u;
const UNLIT: u32 = 16u;
const NO_SCATTERING: u32 = 32u;

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: HedgehogMaterial;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var diffuse_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var diffuse_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var normal_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var normal_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var gloss_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var gloss_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(7) var gi_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(8) var gi_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(9) var diffuse2_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(10) var diffuse2_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(11) var normal2_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(12) var normal2_sampler: sampler;
// Per-instance mrgGIAtlasParam, indexed by the draw's MeshTag (see GiTiles).
@group(#{MATERIAL_BIND_GROUP}) @binding(13) var<storage, read> gi_tiles: array<vec4<f32>>;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) uv_gi: vec2<f32>,
    @location(4) tangent: vec4<f32>,
    @location(5) color: vec4<f32>,
    @location(6) binormal: vec3<f32>,
    @location(7) uv2: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec3<f32>,
    @location(3) binormal: vec3<f32>,
    @location(4) uv: vec2<f32>,
    @location(5) uv2: vec2<f32>,
    @location(6) gi_uv: vec2<f32>,
    @location(7) color: vec4<f32>,
};

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(v.instance_index);
    let world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(v.position, 1.0));
    out.position = position_world_to_clip(world.xyz);
    out.world_position = world.xyz;
    out.normal = mesh_functions::mesh_normal_local_to_world(v.normal, v.instance_index);
    let m = mat3x3<f32>(world_from_local[0].xyz, world_from_local[1].xyz, world_from_local[2].xyz);
    out.tangent = m * v.tangent.xyz;
    // Default_@n@p passes the negated binormal (oColor1 = -binormal).
    out.binormal = -(m * v.binormal);
    out.uv = v.uv;
    out.uv2 = v.uv2;
    let gi_atlas = gi_tiles[mesh_functions::get_tag(v.instance_index)];
    out.gi_uv = v.uv_gi * gi_atlas.xy + gi_atlas.zw;
    out.color = v.color;
    return out;
}

// Normal maps keep X in alpha (x = r * a), Y in green.
fn decode_normal(t: vec4<f32>) -> vec2<f32> {
    return vec2(t.r * t.a * 2.0 - 1.0, t.g * 2.0 - 1.0);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    let bits = u32(material.flags.x);
    var dif = textureSample(diffuse_texture, diffuse_sampler, in.uv);
    if ((bits & BLEND) != 0u) {
        let dif2 = textureSample(diffuse2_texture, diffuse2_sampler, in.uv2);
        dif = mix(dif, dif2, in.color.r);
    }
    let alpha = dif.a * material.opacity.x;
    if (material.flags.y > 0.0 && alpha < material.flags.y) {
        discard;
    }
    if ((bits & UNLIT) != 0u) {
        return vec4(min(dif.rgb * material.diffuse.rgb, vec3(4.0)), alpha);
    }

    let to_eye_raw = view.world_position - in.world_position;
    let to_eye = normalize(to_eye_raw);
    var vertex_n = normalize(in.normal);
    if (!is_front) {
        vertex_n = -vertex_n;
    }

    // Per-pixel normal: nx * T + ny * B + nz * N (not renormalized, as in the original).
    var n = vertex_n;
    if ((bits & HAS_NORMAL) != 0u) {
        var t = textureSample(normal_texture, normal_sampler, in.uv);
        if ((bits & BLEND) != 0u) {
            t = mix(t, textureSample(normal2_texture, normal2_sampler, in.uv2), in.color.r);
        }
        let nxy = decode_normal(t);
        let nz = sqrt(abs(1.0 - dot(nxy, nxy)));
        n = nxy.x * normalize(in.tangent) + nxy.y * normalize(in.binormal) + nz * vertex_n;
    }

    // Baked GI: rgb stored as sqrt, alpha = sun visibility.
    var gi_shadow = 1.0;
    var indirect = material.ambient.rgb * material.gi0_scale.rgb;
    if ((bits & HAS_GI) != 0u) {
        let gi = textureSample(gi_texture, gi_sampler, in.gi_uv);
        gi_shadow = mix(material.gi0_scale.w, 1.0, gi.a);
        let c = gi.rgb * gi.rgb;
        let luma = dot(c, vec3(0.2989, 0.5866, 0.1144));
        indirect *= saturate((c - luma) * material.gi1_scale.w + luma);
    }

    // Realtime sun shadow (the game uses its own PCF map; Bevy's cascades stand in).
    var rt_shadow = 1.0;
    if (lights.n_directional_lights > 0u
        && (lights.directional_lights[0].flags & DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
        let view_z = dot(vec4(view.view_from_world[0].z, view.view_from_world[1].z,
            view.view_from_world[2].z, view.view_from_world[3].z), vec4(in.world_position, 1.0));
        rt_shadow = fetch_directional_shadow(0u, vec4(in.world_position, 1.0), vertex_n, view_z, in.position.xy);
    }
    let shadow = gi_shadow * rt_shadow;

    let l = material.light_direction.xyz;
    let n_dot_l = saturate(dot(n, -l));
    let direct = n_dot_l * material.light_diffuse.rgb * shadow;

    // Blinn-Phong: exponent 2^(gloss*10+2) * gloss map, clamped to [1, 512].
    var power = 1.0;
    if ((bits & HAS_GLOSS) != 0u) {
        power = textureSample(gloss_texture, gloss_sampler, in.uv).r;
    }
    let exponent = clamp(exp2(material.power_gloss_level.y * 10.0 + 2.0) * power, 1.0, 512.0);
    let h = normalize(to_eye - l);
    let n_dot_h = saturate(dot(h, n));
    let spec_light = pow(max(n_dot_h, 1e-6), exponent) * material.light_specular.rgb * n_dot_l * shadow;
    // Vertex-shader Fresnel term (oTexCoord3.w).
    let fresnel = 0.3 + 0.7 * pow(1.0 - saturate(dot(vertex_n, to_eye)), 5.0);
    let specular = material.specular.rgb * (exponent * 0.125) * spec_light * fresnel;

    var color = min(dif.rgb * (material.diffuse.rgb * direct + indirect) + specular, vec3(4.0));

    // Light scattering (Default_@n@p): extinction and in-scatter by view depth.
    if ((bits & NO_SCATTERING) == 0u) {
        let depth = -dot(vec4(view.view_from_world[0].z, view.view_from_world[1].z,
            view.view_from_world[2].z, view.view_from_world[3].z), vec4(in.world_position, 1.0));
        let far_near = material.ls_far_near;
        let s = saturate((depth - far_near.y) / (far_near.x - far_near.y)) * far_near.z;
        let beta = material.ls_ray_mie.x + material.ls_ray_mie.y;
        let extinction = exp(-beta * s);
        let cos_theta = dot(to_eye, -l);
        let hg = material.ls_const_g.y + material.ls_const_g.z * cos_theta;
        let phase = material.ls_ray_mie.z * (1.0 + cos_theta * cos_theta)
            + material.ls_ray_mie.w * material.ls_const_g.x / pow(abs(hg), 1.5);
        let inscatter = material.ls_color.rgb * (phase / beta) * (1.0 - extinction) * far_near.w;
        color = color * extinction + inscatter;
    }
    return vec4(color, alpha);
}
