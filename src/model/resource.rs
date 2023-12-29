use super::{Material, Mesh, Model, ModelVertex};
use crate::Texture;
use std::{
    env, fs,
    io::{self, BufReader, Cursor},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};
use tobj::LoadOptions;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource, BufferUsages, Device,
    Queue,
};

pub fn resource_directory() -> io::Result<&'static PathBuf> {
    static RESOURCE_DIRECTORY: OnceLock<PathBuf> = OnceLock::new();

    Ok(RESOURCE_DIRECTORY.get_or_init(|| PathBuf::from(env::var("RESOURCE_DIRECTORY").unwrap())))
}

pub fn load_texture(file_name: &str, device: &Device, queue: &Queue) -> io::Result<Texture> {
    Ok(Texture::from_bytes(
        device,
        queue,
        &fs::read(resource_directory()?.join(file_name))?,
        None,
    ))
}

pub fn load_model(
    file_name: &str,
    device: &Device,
    queue: &Queue,
    layout: &BindGroupLayout,
) -> io::Result<Model> {
    let object_cursor = Cursor::new(fs::read(resource_directory()?.join(file_name))?);
    let mut object_reader = BufReader::new(object_cursor);
    let (models, object_materials) = tobj::load_obj_buf(
        &mut object_reader,
        &LoadOptions {
            single_index: true,
            triangulate: true,
            ..Default::default()
        },
        move |path| {
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(
                &fs::read(resource_directory().unwrap().join(path)).unwrap(),
            )))
        },
    )
    .unwrap();

    let materials = object_materials
        .unwrap()
        .into_iter()
        .map(|material| -> io::Result<Material> {
            let diffuse_texture: Texture =
                load_texture(&material.diffuse_texture.unwrap(), device, queue)?;
            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&diffuse_texture.view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&diffuse_texture.sampler),
                    },
                ],
                label: None,
            });

            Ok(Material {
                name: material.name,
                diffuse_texture,
                bind_group,
            })
        })
        .collect::<io::Result<Vec<_>>>()?;

    let meshes = models
        .into_iter()
        .map(|model| {
            let vertices = (0..model.mesh.positions.len() / 3)
                .map(|i| ModelVertex {
                    position: [
                        model.mesh.positions[i * 3],
                        model.mesh.positions[i * 3 + 1],
                        model.mesh.positions[i * 3 + 2],
                    ],
                    texture_coordinates: [
                        model.mesh.texcoords[i * 2],
                        model.mesh.texcoords[i * 2 + 1],
                    ],
                    normal: [
                        model.mesh.normals[i * 3],
                        model.mesh.normals[i * 3 + 1],
                        model.mesh.normals[i * 3 + 2],
                    ],
                })
                .collect::<Vec<_>>();

            let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: Some("Vertex buffer ({name})"),
                contents: bytemuck::cast_slice(&vertices),
                usage: BufferUsages::VERTEX,
            });

            let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: Some("Index buffer ({name})"),
                contents: bytemuck::cast_slice(&model.mesh.indices),
                usage: BufferUsages::INDEX,
            });

            Mesh {
                name: file_name.to_owned(),
                vertex_buffer,
                index_buffer,
                element_count: model.mesh.indices.len() as u32,
                material: model.mesh.material_id.unwrap_or(0),
            }
        })
        .collect();

    Ok(Model { meshes, materials })
}
