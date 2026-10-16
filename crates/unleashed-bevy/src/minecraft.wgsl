// Minecraft terrain sections (MinecraftOSS meshes): atlas texel x vertex tint (face
// shade and biome colour) x the lightmap. The lightmap follows vanilla 26.3's
// inputs as MinecraftOSS extracts them (ambient, sky light colour and factor, block
// light tint and factor; see third_party/minecraft_terrain/src/environment.rs
// open_sky_light): brightness(level) = f / (4 - 3f), then the default-gamma lerp.

#import bevy_pbr::{
    mesh_functions,
    mesh_view_bindings::{view, lights},
    mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT,
    shadows::fetch_directional_shadow,
    view_transformations::position_world_to_clip,
}

struct McTerrain {
    // rgb: ambient_light_color, a: sky_light_factor
    ambient: vec4<f32>,
    // rgb: sky_light_color, a: block_factor
    sky_light: vec4<f32>,
    // rgb: block_light_tint, a: alpha cutoff (0 = blend)
    block_tint: vec4<f32>,
    // rgb: fog colour, x of fog_range: start, y: end (metres from the camera)
    fog_color: vec4<f32>,
    fog_range: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> mc: McTerrain;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var atlas: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var atlas_sampler: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) light: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) light: vec2<f32>,
};

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(v.instance_index);
    let world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(v.position, 1.0));
    out.clip = position_world_to_clip(world.xyz);
    out.world = world.xyz;
    out.uv = v.uv;
    out.color = v.color;
    out.light = v.light;
    return out;
}

fn brightness(level: f32) -> f32 {
    let f = clamp(level / 15.0, 0.0, 1.0);
    return f / (4.0 - 3.0 * f);
}

fn lightmap(sky: f32, block: f32) -> vec3<f32> {
    let raw = clamp(
        mc.ambient.rgb
            + mc.sky_light.rgb * (brightness(sky) * mc.ambient.a)
            + mc.block_tint.rgb * (brightness(block) * mc.sky_light.a),
        vec3<f32>(0.0), vec3<f32>(1.0));
    let m = max(raw.r, max(raw.g, raw.b));
    var gamma = vec3<f32>(0.0);
    if m > 0.0 {
        gamma = raw * ((1.0 - pow(1.0 - m, 4.0)) / m);
    }
    return mix(raw, gamma, 0.5);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(atlas, atlas_sampler, in.uv);
    let cutoff = mc.block_tint.a;
    if cutoff > 0.0 && texel.a < cutoff {
        discard;
    }
    var rgb = texel.rgb * in.color.rgb * lightmap(in.light.x, in.light.y);
    // Sonic's shadow (the only caster): the sun's shadow map darkens the ground by up to
    // 45% where the block sees the sky and the sun is up. Terrain itself keeps Minecraft's
    // baked sky light.
    if (lights.n_directional_lights > 0u
        && (lights.directional_lights[0].flags & DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
        let to_light = lights.directional_lights[0].direction_to_light;
        let view_z = dot(vec4(view.view_from_world[0].z, view.view_from_world[1].z,
            view.view_from_world[2].z, view.view_from_world[3].z), vec4(in.world, 1.0));
        let lit = fetch_directional_shadow(0u, vec4(in.world, 1.0), to_light, view_z, in.clip.xy);
        let strength = 0.45 * clamp(in.light.x / 15.0, 0.0, 1.0) * sqrt(clamp(to_light.y, 0.0, 1.0));
        rgb *= 1.0 - strength * (1.0 - lit);
    }
    let distance = length(in.world - view.world_position);
    let fog = clamp((distance - mc.fog_range.x) / max(mc.fog_range.y - mc.fog_range.x, 1.0), 0.0, 1.0);
    rgb = mix(rgb, mc.fog_color.rgb, fog);
    return vec4<f32>(rgb, select(texel.a * in.color.a, 1.0, cutoff > 0.0));
}
