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

pub fn load_texture(
    file_name: &str,
    is_normal_map: bool,
    device: &Device,
    queue: &Queue,
) -> io::Result<Texture> {
    Ok(Texture::from_bytes(
        device,
        queue,
        &fs::read(resource_directory()?.join(file_name))?,
        None,
        is_normal_map,
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
                load_texture(&material.diffuse_texture.unwrap(), false, device, queue)?;
            let normal_texture =
                load_texture(&material.normal_texture.unwrap(), true, device, queue)?;

            Ok(Material::new(
                device,
                &material.name,
                diffuse_texture,
                normal_texture,
                layout,
            ))
        })
        .collect::<io::Result<Vec<_>>>()?;

    let meshes = models
        .into_iter()
        .map(|model| {
            let mut vertices = (0..model.mesh.positions.len() / 3)
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
                    tangent: [0.0; 3],
                    bitangent: [0.0; 3],
                })
                .collect::<Vec<_>>();

            let indices = &model.mesh.indices;
            let mut triangles_included = vec![0; vertices.len()];

            // Calculate tangents and bitangets. We're going to
            // use the triangles, so we need to loop through the
            // indices in chunks of 3
            for c in indices.chunks(3) {
                let v0 = vertices[c[0] as usize];
                let v1 = vertices[c[1] as usize];
                let v2 = vertices[c[2] as usize];

                let pos0: cgmath::Vector3<_> = v0.position.into();
                let pos1: cgmath::Vector3<_> = v1.position.into();
                let pos2: cgmath::Vector3<_> = v2.position.into();

                let uv0: cgmath::Vector2<_> = v0.texture_coordinates.into();
                let uv1: cgmath::Vector2<_> = v1.texture_coordinates.into();
                let uv2: cgmath::Vector2<_> = v2.texture_coordinates.into();

                // Calculate the edges of the triangle
                let delta_pos1 = pos1 - pos0;
                let delta_pos2 = pos2 - pos0;

                // This will give us a direction to calculate the
                // tangent and bitangent
                let delta_uv1 = uv1 - uv0;
                let delta_uv2 = uv2 - uv0;

                // Solving the following system of equations will
                // give us the tangent and bitangent.
                //     delta_pos1 = delta_uv1.x * T + delta_u.y * B
                //     delta_pos2 = delta_uv2.x * T + delta_uv2.y * B
                // Luckily, the place I found this equation provided
                // the solution!
                let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
                let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
                // We flip the bitangent to enable right-handed normal
                // maps with wgpu texture coordinate system
                let bitangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * -r;

                // We'll use the same tangent/bitangent for each vertex in the triangle
                vertices[c[0] as usize].tangent =
                    (tangent + cgmath::Vector3::from(vertices[c[0] as usize].tangent)).into();
                vertices[c[1] as usize].tangent =
                    (tangent + cgmath::Vector3::from(vertices[c[1] as usize].tangent)).into();
                vertices[c[2] as usize].tangent =
                    (tangent + cgmath::Vector3::from(vertices[c[2] as usize].tangent)).into();
                vertices[c[0] as usize].bitangent =
                    (bitangent + cgmath::Vector3::from(vertices[c[0] as usize].bitangent)).into();
                vertices[c[1] as usize].bitangent =
                    (bitangent + cgmath::Vector3::from(vertices[c[1] as usize].bitangent)).into();
                vertices[c[2] as usize].bitangent =
                    (bitangent + cgmath::Vector3::from(vertices[c[2] as usize].bitangent)).into();

                // Used to average the tangents/bitangents
                triangles_included[c[0] as usize] += 1;
                triangles_included[c[1] as usize] += 1;
                triangles_included[c[2] as usize] += 1;
            }

            // Average the tangents/bitangents
            for (i, n) in triangles_included.into_iter().enumerate() {
                let denom = 1.0 / n as f32;
                let vertex = &mut vertices[i];
                vertex.tangent = (cgmath::Vector3::from(vertex.tangent) * denom).into();
                vertex.bitangent = (cgmath::Vector3::from(vertex.bitangent) * denom).into();
            }

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
