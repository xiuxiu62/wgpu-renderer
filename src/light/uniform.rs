use bytemuck::{Pod, Zeroable};
use cgmath::{Deg, Quaternion, Rotation3, Vector3};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferUsages, Device, Queue,
    ShaderStages,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct LightUniform {
    position: Vector3<f32>,
    _padding_1: u32,
    color: Vector3<f32>,
    _padding_2: u32,
}

impl LightUniform {
    pub fn new(position: Vector3<f32>, color: Vector3<f32>) -> Self {
        Self {
            position,
            _padding_1: 0,
            color,
            _padding_2: 0,
        }
    }

    pub fn prepared(self, device: &Device) -> LightBundle {
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("([Light] buffer"),
            contents: bytemuck::bytes_of(&self),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        // SAFETY: size_of::<LightUniform> will never be zero and if a uniform size is zero it shouldn't exist in the first place
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("[Light] bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                    // min_binding_size: Some(unsafe {
                    //     mem::transmute(mem::size_of::<LightUniform>())
                    // }),
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("[Light] bind group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        LightBundle {
            uniform: self,
            buffer,
            bind_group,
            bind_group_layout,
        }
    }
}

pub struct LightBundle {
    pub uniform: LightUniform,
    pub buffer: Buffer,
    pub bind_group: BindGroup,
    pub bind_group_layout: BindGroupLayout,
}

impl LightBundle {
    pub fn update(&mut self, queue: &Queue) {
        let old_position: Vector3<f32> = self.uniform.position.into();
        self.uniform.position =
            (Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), Deg(1.0)) * old_position).into();

        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&self.uniform));
    }
}

#[cfg(test)]
mod test {
    use super::LightUniform;
    use crate::vec3;
    use std::ptr;

    #[test]
    fn aligned() {
        let size = std::mem::size_of::<LightUniform>();
        println!("Size of [LightUniform] {size} bytes");
        assert_eq!(size, 32);

        let uniform = LightUniform::new(vec3!(1.0, 2.0, 3.0), vec3!(4.0, 5.0, 6.0));
        let position_ptr = ptr::addr_of!(uniform.position).cast::<u8>();
        let color_ptr = ptr::addr_of!(uniform.color).cast::<u8>();
        assert_eq!(unsafe { color_ptr.offset_from(position_ptr) }, 16);
    }
}
