use crate::Texture;
use bytemuck::{Pod, Zeroable};
use std::ops::Range;
use wgpu::{
    vertex_attr_array, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindingResource, Buffer, BufferAddress, Device, RenderPass, VertexBufferLayout, VertexStepMode,
};

pub mod resource;

pub trait VertexBufferFormat {
    type Attributes;
    const ATTRIBUTES: Self::Attributes;

    fn descriptor() -> VertexBufferLayout<'static>;
}

#[derive(Debug)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

#[derive(Debug)]
pub struct Material {
    pub name: String,
    pub diffuse_texture: Texture,
    pub normal_texture: Texture,
    pub bind_group: BindGroup,
}

impl Material {
    pub fn new(
        device: &Device,
        name: &str,
        diffuse_texture: Texture,
        normal_texture: Texture,
        layout: &BindGroupLayout,
    ) -> Self {
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some(&format!("Texture bind group ({name})")),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
                },
            ],
            layout,
        });

        Self {
            name: name.to_owned(),
            diffuse_texture,
            normal_texture,
            bind_group,
        }
    }
}

#[derive(Debug)]
pub struct Mesh {
    pub name: String,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub element_count: u32,
    pub material: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub texture_coordinates: [f32; 2],
    pub normal: [f32; 3],

    pub tangent: [f32; 3],
    pub bitangent: [f32; 3],
}

impl VertexBufferFormat for ModelVertex {
    type Attributes = [wgpu::VertexAttribute; 5];
    const ATTRIBUTES: Self::Attributes = vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
        2 => Float32x3,
        3 => Float32x3,
        4 => Float32x3,
    ];

    fn descriptor() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<ModelVertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

pub trait DrawModel<'a> {
    fn draw_mesh(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        camera_bind_group: &'a BindGroup,
        light_bind_group: &'a BindGroup,
    ) {
        self.draw_mesh_instanced(mesh, material, 0..1, camera_bind_group, light_bind_group);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a BindGroup,
        light_bind_group: &'a BindGroup,
    );

    fn draw_model(
        &mut self,
        model: &'a Model,
        camera_bind_group: &'a BindGroup,
        light_bind_group: &'a BindGroup,
    ) {
        self.draw_model_instanced(model, 0..1, camera_bind_group, light_bind_group);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        camera_bind_group: &'a BindGroup,
        light_bind_group: &'a BindGroup,
    );

    fn draw_model_instanced_with_material(
        &mut self,
        model: &'a Model,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a BindGroup,
        light_bind_group: &'a BindGroup,
    );
}

impl<'a> DrawModel<'a> for RenderPass<'a> {
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a BindGroup,
        light_bind_group: &'a BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, camera_bind_group, &[]);
        self.set_bind_group(2, light_bind_group, &[]);
        self.draw_indexed(0..mesh.element_count, 0, instances);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        camera_bind_group: &'a BindGroup,
        light_bind_group: &'a BindGroup,
    ) {
        model.meshes.iter().for_each(move |mesh| {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(
                mesh,
                material,
                instances.clone(),
                camera_bind_group,
                light_bind_group,
            );
        });
    }

    fn draw_model_instanced_with_material(
        &mut self,
        model: &'a Model,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a BindGroup,
        light_bind_group: &'a BindGroup,
    ) {
        model.meshes.iter().for_each(move |mesh| {
            self.draw_mesh_instanced(
                mesh,
                material,
                instances.clone(),
                camera_bind_group,
                light_bind_group,
            )
        });
    }
}
