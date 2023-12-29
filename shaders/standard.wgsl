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
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
}

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) normal_matrix_0: vec3<f32>,
    @location(10) normal_matrix_1: vec3<f32>,
    @location(11) normal_matrix_2: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) texture_coordinates: vec2<f32>,
    @location(1) tangent_position: vec3<f32>,
    @location(2) tangent_light_position: vec3<f32>,
    @location(3) tangent_view_position: vec3<f32>,
}

@group(0) @binding(0)
var texture_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var sampler_diffuse: sampler;
@group(0) @binding(2)
var texture_normal: texture_2d<f32>;
@group(0) @binding(3)
var sampler_normal: sampler;

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
    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2,
    );
    
    let world_normal = normalize(normal_matrix * model.normal);
    let world_tangent = normalize(normal_matrix * model.tangent);
    let world_bitangent = normalize(normal_matrix * model.bitangent);
    let tangent_matrix = transpose(mat3x3<f32>(
        world_tangent,
        world_bitangent,
        world_normal,
    ));
    
    var world_position: vec4<f32> = model_matrix * vec4<f32>(model.position, 1.0);
    
    var out: VertexOutput;
    // out.clip_position = camera.view_projection * model_matrix * vec4<f32>(model.position, 1.0);
    out.texture_coordinates = model.texture_coordinates;

    
    // out.world_normal = normal_matrix * model.normal;
    // out.world_position = world_position.xyz;
    out.clip_position = camera.view_projection * world_position;

    out.tangent_position = tangent_matrix * world_position.xyz;
    out.tangent_light_position = tangent_matrix * light.position;
    out.tangent_view_position = tangent_matrix * camera.view_position.xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color: vec4<f32> = textureSample(texture_diffuse, sampler_diffuse, in.texture_coordinates); 
    let object_normal: vec4<f32> = textureSample(texture_normal, sampler_normal, in.texture_coordinates); 
    
    let ambient_strength = 0.1;
    let ambient_color = light.color * ambient_strength;
    
    let tangent_normal = object_normal.xyz * 2.0 - 1.0;
    // let light_direction = normalize(light.position - in.world_position);
    // let view_direction = normalize(camera.view_position.xyz - in.world_position);
    let light_direction = normalize(in.tangent_light_position - in.tangent_position);
    let view_direction = normalize(in.tangent_view_position.xyz - in.tangent_position);
    let half_direction = normalize(view_direction + light_direction);

    let diffuse_strength = max(dot(tangent_normal, light_direction), 0.0);
    let diffuse_color = light.color * diffuse_strength;
    
    let specular_strength = pow(max(dot(tangent_normal, half_direction), 0.0), 32.0);
    let specular_color = specular_strength * light.color;
    
    return vec4<f32>((ambient_color + diffuse_color + specular_color) * object_color.xyz, object_color.a);
    // return vec4<f32>(specular_color, object_color.a);
}
