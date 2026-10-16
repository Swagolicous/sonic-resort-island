// The game's particle programs (shader.ar), both of them in one material:
//
//   BillboardParticle_@v@ / BillboardParticle_d@v@: world-space quads, the pixel is
//       texture * vertex colour, clamped to 4.
//   MeshParticle_@v@ / MeshParticle_d@v@: a model under g_MtxWorld; the pixel is
//       (texture.rgb * g_ParticleParam.rgb, g_Diffuse.a * texture.a
//        * g_OpacityReflectionRefractionSpectype.x * g_ParticleParam.a) * vertex colour,
//       clamped to 4.
//
// Read from XenosRecomp's HLSL of the four microcode programs. The host passes the
// billboard's vertices as the game built them (already in world space), the mesh's world
// matrix as the entity transform, and `particle` = (g_ParticleParam.rgb, the product of
// the three alpha factors) for a mesh, (1, 1, 1, 1) for a billboard.

#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
}

struct Particle {
    particle: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> job: Particle;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var sampDif: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var sampDif_sampler: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(v.instance_index);
    let world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(v.position, 1.0));
    out.clip = position_world_to_clip(world.xyz);
    out.uv = v.uv;
    out.color = v.color;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(sampDif, sampDif_sampler, in.uv);
    return min(texel * job.particle * in.color, vec4<f32>(4.0));
}
