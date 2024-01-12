use super::{Camera, Projection};
use crate::vec4;
use cgmath::{Matrix4, SquareMatrix, Vector3, Vector4};

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_position: Vector4<f32>,
    view_projection: Matrix4<f32>,
}

impl CameraUniform {
    pub fn new(camera: &Camera, projection: &Projection) -> Self {
        Self {
            view_position: camera.position.to_homogeneous().into(),
            view_projection: (projection.matrix() * camera.matrix()).into(),
        }
    }

    pub fn update(&mut self, camera: &Camera, projection: &Projection) {
        self.view_position = camera.position.to_homogeneous().into();
        self.view_projection = (projection.matrix() * camera.matrix()).into();
    }
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self {
            view_position: vec4!(0.0, 0.0, 0.0, 0.0),
            view_projection: Matrix4::identity().into(),
        }
    }
}
