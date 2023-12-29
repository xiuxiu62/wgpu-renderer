struct Camera {
    view_position: vec4<f32>,
    view_projection: mat4x4<f32>,
}

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) texture_coordinates: vec2<f32>,
}

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) texture_coordinates: vec2<f32>,
}

@group(0) @binding(0)
var texture_diffuse: texture_2d<f32>;

@group(0) @binding(1)
var sampler_diffuse: sampler;

@group(1) @binding(0) 
var<uniform> camera: Camera;

@group(2) @binding(0)
var<uniform> light: Light;

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    var out: VertexOutput;
    
    out.clip_position = camera.view_projection * model_matrix * vec4<f32>(model.position, 1.0);
    out.texture_coordinates = model.texture_coordinates;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color: vec4<f32> = textureSample(texture_diffuse, sampler_diffuse, in.texture_coordinates); 
    let ambient_strength = 0.1;
    let ambient_color = light.color * ambient_strength;
    
    return vec4<f32>(object_color.xyz * ambient_color, object_color.a);
}
