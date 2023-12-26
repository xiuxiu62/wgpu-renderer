use cgmath::*;
use std::f32::consts::FRAC_PI_2;

mod controller;
mod projection;
mod uniform;

pub use controller::CameraController;
pub use projection::Projection;
pub use uniform::CameraUniform;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 0.5, 0.5,
        0.0, 0.0, 0.0, 1.0,
    );

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

#[derive(Debug)]
pub struct Camera {
    pub position: Point3<f32>,
    yaw: Rad<f32>,
    pitch: Rad<f32>,
}

impl Camera {
    pub fn new<V: Into<Point3<f32>>, Y: Into<Rad<f32>>, P: Into<Rad<f32>>>(
        position: V,
        yaw: Y,
        pitch: P,
    ) -> Self {
        Self {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
        }
    }

    pub fn matrix(&self) -> Matrix4<f32> {
        let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();

        Matrix4::look_to_rh(
            self.position,
            Vector3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
            Vector3::unit_y(),
        )
    }
}

// use cgmath::{InnerSpace, Matrix4, Point3, Rad, Vector3};
// use std::{f32::consts::FRAC_PI_2, time::Duration};
// use winit::{
//     dpi::PhysicalPosition,
//     event::{ElementState, MouseScrollDelta},
//     keyboard::KeyCode,
// };

// const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

// #[rustfmt::skip]
// const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
//     1.0, 0.0, 0.0, 0.0,
//     0.0, 1.0, 0.0, 0.0,
//     0.0, 0.0, 0.5, 0.5,
//     0.0, 0.0, 0.0, 1.0,
// );

// pub struct Camera {
//     pub position: Point3<f32>,
//     yaw: Rad<f32>,
//     pitch: Rad<f32>,
// }

// impl Camera {
//     pub fn new<V: Into<Point3<f32>>, Y: Into<Rad<f32>>, P: Into<Rad<f32>>>(
//         position: V,
//         yaw: Y,
//         pitch: P,
//     ) -> Self {
//         Self {
//             position: position.into(),
//             yaw: yaw.into(),
//             pitch: pitch.into(),
//         }
//     }

//     pub fn matrix(&self) -> Matrix4<f32> {
//         let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
//         let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();

//         Matrix4::look_to_rh(
//             self.position,
//             Vector3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
//             Vector3::unit_y(),
//         )
//     }
// }

// pub struct Projection {
//     aspect: f32,
//     fovy: Rad<f32>,
//     z_near: f32,
//     z_far: f32,
// }

// impl Projection {
//     pub fn new<FOV: Into<Rad<f32>>>(
//         width: u32,
//         height: u32,
//         fovy: FOV,
//         z_near: f32,
//         z_far: f32,
//     ) -> Self {
//         Self {
//             aspect: width as f32 / height as f32,
//             fovy: fovy.into(),
//             z_near,
//             z_far,
//         }
//     }

//     pub fn resize(&mut self, width: u32, height: u32) {
//         self.aspect = width as f32 / height as f32;
//     }

//     pub fn matrix(&self) -> Matrix4<f32> {
//         OPENGL_TO_WGPU_MATRIX * cgmath::perspective(self.fovy, self.aspect, self.z_near, self.z_far)
//     }
// }

// pub struct CameraController {
//     amount_left: f32,
//     amount_right: f32,
//     amount_forward: f32,
//     amount_backward: f32,
//     amount_up: f32,
//     amount_down: f32,

//     rotate_horizontal: f32,
//     rotate_vertical: f32,
//     scroll: f32,
//     speed: f32,
//     sensitivity: f32,
// }

// impl CameraController {
//     pub fn new(speed: f32, sensitivity: f32) -> Self {
//         Self {
//             amount_left: 0.0,
//             amount_right: 0.0,
//             amount_forward: 0.0,
//             amount_backward: 0.0,
//             amount_up: 0.0,
//             amount_down: 0.0,

//             rotate_horizontal: 0.0,
//             rotate_vertical: 0.0,
//             scroll: 0.0,
//             speed,
//             sensitivity,
//         }
//     }

//     pub fn handle_key_input(&mut self, key: &KeyCode, state: &ElementState) {
//         let amount = if state.is_pressed() { 1.0 } else { 0.0 };
//         match key {
//             KeyCode::KeyW => self.amount_forward = amount,
//             KeyCode::KeyS => self.amount_backward = amount,
//             KeyCode::KeyA => self.amount_left = amount,
//             KeyCode::KeyD => self.amount_right = amount,
//             KeyCode::Space => self.amount_up = amount,
//             KeyCode::ShiftLeft => self.amount_down = amount,
//             _ => {}
//         }
//     }

//     pub fn handle_mouse_input(&mut self, mouse_dx: f32, mouse_dy: f32) {
//         self.rotate_horizontal = mouse_dx;
//         self.rotate_vertical = mouse_dy;
//     }

//     pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
//         self.scroll = -match delta {
//             MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
//             MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => *scroll as f32,
//         };
//     }

//     pub fn update(&mut self, camera: &mut Camera, dt: Duration) {
//         let dt = dt.as_secs_f32();
//         let (yaw_sin, yaw_cos) = camera.yaw.0.sin_cos();
//         let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
//         let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
//         camera.position += forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
//         camera.position += right * (self.amount_right - self.amount_left) * self.speed * dt;

//         let (pitch_sin, pitch_cos) = camera.pitch.0.sin_cos();
//         let scrollward =
//             Vector3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
//         camera.position += scrollward * self.scroll * self.speed * self.sensitivity * dt;
//         self.scroll = 0.0;

//         camera.position.y += (self.amount_up - self.amount_down) * self.speed * dt;

//         camera.yaw += Rad(self.rotate_horizontal) * self.sensitivity * dt;
//         camera.pitch += Rad(-self.rotate_vertical) * self.sensitivity * dt;

//         self.rotate_horizontal = 0.0;
//         self.rotate_vertical = 0.0;

//         let max_angle = Rad(-SAFE_FRAC_PI_2);
//         if camera.pitch < -max_angle {
//             camera.pitch = -max_angle;

//             return;
//         }

//         if camera.pitch > max_angle {
//             camera.pitch = max_angle;
//         }
//     }
// }
