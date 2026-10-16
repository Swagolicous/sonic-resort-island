struct Camera {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var diffuse_texture: texture_2d<f32>;

@group(1) @binding(1)
var diffuse_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

fn make_vertex_output(
    position: vec3<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = camera.view_projection * vec4<f32>(position, 1.0);
    output.normal = normal;
    output.uv = uv;
    return output;
}

@vertex
fn vs_position(@location(0) position: vec3<f32>) -> VertexOutput {
    return make_vertex_output(
        position,
        vec3<f32>(0.0, 1.0, 0.0),
        vec2<f32>(0.0, 0.0),
    );
}

@vertex
fn vs_position_normal(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
) -> VertexOutput {
    return make_vertex_output(position, normal, vec2<f32>(0.0, 0.0));
}

@vertex
fn vs_position_uv(
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>,
) -> VertexOutput {
    return make_vertex_output(position, vec3<f32>(0.0, 1.0, 0.0), uv);
}

@vertex
fn vs_position_normal_uv(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
) -> VertexOutput {
    return make_vertex_output(position, normal, uv);
}

@fragment
fn fs_diffuse(input: VertexOutput) -> @location(0) vec4<f32> {
    let albedo = textureSample(diffuse_texture, diffuse_sampler, input.uv);

    let n = normalize(input.normal);
    let light_direction = normalize(vec3<f32>(0.35, 0.85, 0.40));
    let diffuse = max(dot(n, light_direction), 0.0);
    let brightness = 0.28 + 0.72 * diffuse;

    return vec4<f32>(albedo.rgb * brightness, albedo.a);
}
