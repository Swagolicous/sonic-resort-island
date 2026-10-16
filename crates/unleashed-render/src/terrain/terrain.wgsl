struct Camera {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
};

@vertex
fn vs_position(@location(0) position: vec3<f32>) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = camera.view_projection * vec4<f32>(position, 1.0);
    output.normal = vec3<f32>(0.0, 1.0, 0.0);
    return output;
}

@vertex
fn vs_position_normal(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = camera.view_projection * vec4<f32>(position, 1.0);
    output.normal = normal;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(input.normal);
    let light_direction = normalize(vec3<f32>(0.35, 0.85, 0.40));
    let diffuse = max(dot(n, light_direction), 0.0);
    let brightness = 0.28 + 0.72 * diffuse;
    let terrain_color = vec3<f32>(0.46, 0.54, 0.38);
    return vec4<f32>(terrain_color * brightness, 1.0);
}
