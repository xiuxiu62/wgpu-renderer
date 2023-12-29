struct Camera {
    view_position: vec4<f32>,
    view_projection: mat4x4<f32>,
}

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
}
