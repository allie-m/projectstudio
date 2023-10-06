use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource, Buffer,
    BufferUsages, Extent3d, FilterMode, IndexFormat, Queue, RenderPass, SamplerDescriptor,
    ShaderStages, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor,
};

use crate::heightmap::{TriangulatedHeightmap, TriangulatedMesh};
use wgpu::Device;

pub fn generate_normal_map(
    device: &Device,
    queue: &Queue,
    layout: &BindGroupLayout,
    heightmaps: &[TriangulatedHeightmap],
    chunk_size: u32,
) -> BindGroup {
    let mut normals = vec![];
    for hm in heightmaps.iter() {
        normals.extend_from_slice(&hm.normals);
    }
    let normals = normals
        .iter()
        .map(|n| {
            [
                (n[0] * 127.0) as i8,
                (n[1] * 127.0) as i8,
                (n[2] * 127.0) as i8,
                (n[3] * 127.0) as i8,
            ]
        })
        .collect::<Vec<_>>();

    let tex = device.create_texture_with_data(
        queue,
        &TextureDescriptor {
            label: None,
            size: Extent3d {
                width: chunk_size,
                height: chunk_size,
                depth_or_array_layers: heightmaps.len() as u32,
            },
            // TODO: mipmaps
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Snorm,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        unsafe { std::slice::from_raw_parts(normals.as_ptr() as *const u8, 1 * 4 * normals.len()) },
    );
    let view = tex.create_view(&TextureViewDescriptor {
        ..Default::default()
    });
    let sampler = device.create_sampler(&SamplerDescriptor {
        min_filter: FilterMode::Linear,
        mag_filter: FilterMode::Linear,
        ..Default::default()
    });
    device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&view),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(&sampler),
            },
        ],
    })
}

pub struct Chunk {
    pub lods: Vec<ChunkLOD>,
    pub bl_pos: (f32, f32),
}

// one chunk for every LOD
pub struct ChunkLOD {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: u32,
    normal_map_index: i32,
}

impl ChunkLOD {
    pub fn create(device: &Device, mesh: &TriangulatedMesh, normal_map_index: i32) -> Self {
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: unsafe {
                std::slice::from_raw_parts(
                    mesh.vertices.as_ptr() as *const u8,
                    mesh.vertices.len() * std::mem::size_of_val(&mesh.vertices),
                )
            },
            usage: BufferUsages::COPY_DST | BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: unsafe {
                std::slice::from_raw_parts(
                    mesh.indices.as_ptr() as *const u8,
                    mesh.indices.len() * 4,
                )
            },
            usage: BufferUsages::COPY_DST | BufferUsages::INDEX,
        });
        Self {
            vertex_buffer,
            index_buffer,
            index_count: mesh.indices.len() as u32,
            normal_map_index,
        }
    }

    pub fn draw<'a>(&'a self, rpass: &mut RenderPass<'a>) {
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint32);
        rpass.set_push_constants(
            ShaderStages::VERTEX_FRAGMENT,
            68,
            &self.normal_map_index.to_ne_bytes(),
        );
        rpass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}
