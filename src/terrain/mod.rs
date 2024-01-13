use bytemuck::{Pod, Zeroable};
use cgmath::{Vector2, Vector3};
use thiserror::Error;
use wgpu::{vertex_attr_array, VertexAttribute, VertexBufferLayout, VertexStepMode};

use crate::VertexBufferFormat;

pub struct HeightMap {
    width: usize,
    data: Vec<f32>,
}

impl std::fmt::Debug for HeightMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HeightMap {{ width: {} }}", self.width)
    }
}

impl HeightMap {
    pub fn new(data: &[u8]) -> HeightMapResult<Self> {
        let prim_len = std::mem::size_of::<f32>();
        let size = data.len() / prim_len;
        let remainder = data.len() % prim_len;
        if remainder > 0 {
            return Err(HeightMapError::InvalidSize(remainder));
        }

        let width = ((size / prim_len) as f32).sqrt() as usize;
        let data = bytemuck::cast_slice(data).to_vec();

        Ok(Self { width, data })
    }

    pub fn load(path: &str) -> HeightMapResult<Self> {
        Self::new(&std::fs::read(path)?)
    }
}

pub type HeightMapResult<T> = Result<T, HeightMapError>;

#[derive(Debug, Error)]
pub enum HeightMapError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Data is of an invalid size, having {0} too many bytes")]
    InvalidSize(usize),
}

struct TriangleList {
    width: usize,
    depth: usize,
}

impl TriangleList {
    pub fn create(width: usize, depth: usize) -> Self {
        todo!()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct TerrainVertex {
    position: Vector3<f32>,
    normal: Vector3<f32>,
    uv: Vector2<f32>,
}

impl VertexBufferFormat<3> for TerrainVertex {
    const ATTRIBUTES: [VertexAttribute; 3] = vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
        2 => Float32x3,
    ];

    fn descriptor() -> wgpu::VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}
