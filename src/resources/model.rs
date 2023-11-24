use std::mem::size_of;

use anyhow::{bail, Context};
use bytemuck::{cast_slice, Pod, Zeroable};
use wgpu::util::DeviceExt;

use super::load_binary;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    position: glam::Vec3,
    normal: glam::Vec3,
    tex_coord: glam::Vec2,
}

impl Vertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Self>() as _,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
            2 => Float32x2,
        ],
    };
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Morphs {
    d0_position: glam::Vec3,
    d0_normal: glam::Vec3,
    d1_position: glam::Vec3,
    d1_normal: glam::Vec3,
}

pub struct Model {
    vertex_buffer: wgpu::Buffer,
    morph_buffer: Option<wgpu::Buffer>,
    index_buffer: wgpu::Buffer,
    index_format: wgpu::IndexFormat,
    num_indices: u32,
}

impl Model {
    pub async fn load(device: &wgpu::Device, queue: &wgpu::Queue, path: &str) -> anyhow::Result<Self> {
        let bytes = load_binary(path).await?;
        let (document, buffers, images) = gltf::import_slice(&bytes)?;
        Self::from_gltf(device, queue, &document, &buffers, &images)
    }

    pub fn from_gltf(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        document: &gltf::Document,
        buffers: &[gltf::buffer::Data],
        images: &[gltf::image::Data],
    ) -> anyhow::Result<Self> {
        // For this example we'll assume the file only has one mesh,
        // which has one primitive.
        let mesh = document
            .meshes()
            .next()
            .with_context(|| "Model should have 1 mesh")?;
        let prim = mesh
            .primitives()
            .next()
            .with_context(|| "Mesh should have 1 primitive")?;

        // We need to index format to render properly.
        let indices = prim.indices().unwrap();
        let index_format = match indices.data_type() {
            gltf::accessor::DataType::U16 => wgpu::IndexFormat::Uint16,
            gltf::accessor::DataType::U32 => wgpu::IndexFormat::Uint32,
            dt => bail!("Unsupported index type {:?}", dt),
        };

        // The index buffer usually doesn't have a stride,  so we can
        // upload the data to the gpu directly.
        let index_data = Self::get_data_for_accessor(&indices, buffers).unwrap();
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: index_data,
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = indices.count() as u32;

        // Map each attribute to the ones we care about.
        let mut positions = None;
        let mut normals = None;
        let mut tex_coords = None;
        prim.attributes().for_each(|(s, a)| match s {
            gltf::Semantic::Positions => positions = Some(a),
            gltf::Semantic::Normals => normals = Some(a),
            gltf::Semantic::TexCoords(0) => tex_coords = Some(a),
            _ => (), // Ignore other attributes
        });

        let positions = positions.unwrap();
        let normals = normals.unwrap();
        let tex_coords = tex_coords.unwrap();

        // This shape-keys.glb model has vertex components separated
        // we'll combine them so the GPU doesn't have to jump around
        // when preparing for the vertex shader.
        let pos_data: &[glam::Vec3] =
            cast_slice(Self::get_data_for_accessor(&positions, buffers).unwrap());
        let norm_data: &[glam::Vec3] =
            cast_slice(Self::get_data_for_accessor(&normals, buffers).unwrap());
        let tex_coord_data: &[glam::Vec2] =
            cast_slice(Self::get_data_for_accessor(&tex_coords, buffers).unwrap());
        let vertices = (0..pos_data.len().min(norm_data.len()))
            .map(|i| Vertex {
                position: pos_data[i],
                normal: norm_data[i],
                tex_coord: tex_coord_data[i],
            })
            .collect::<Vec<_>>();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // We need to do a similar thing to the morph data that we did
        // with the vertex data.
        let mut morphs = prim.morph_targets();
        let m0 = morphs.next();
        let m1 = morphs.next();

        let morph_buffer = match (m0, m1) {
            (Some(m0), Some(m1)) => {
                let mp0 = m0.positions().unwrap();
                let mn0 = m0.normals().unwrap();

                let mp1 = m1.positions().unwrap();
                let mn1 = m1.normals().unwrap();

                let mp0_data: &[glam::Vec3] =
                    cast_slice(Self::get_data_for_accessor(&mp0, buffers).unwrap());
                let mn0_data: &[glam::Vec3] =
                    cast_slice(Self::get_data_for_accessor(&mn0, buffers).unwrap());
                let mp1_data: &[glam::Vec3] =
                    cast_slice(Self::get_data_for_accessor(&mp1, buffers).unwrap());
                let mn1_data: &[glam::Vec3] =
                    cast_slice(Self::get_data_for_accessor(&mn1, buffers).unwrap());
                let len = mp0_data
                    .len()
                    .min(mp1_data.len())
                    .min(mn0_data.len())
                    .min(mn1_data.len());
                let morphs = (0..len)
                    .map(|i| Morphs {
                        d0_position: mp0_data[i],
                        d0_normal: mn0_data[i],
                        d1_position: mp1_data[i],
                        d1_normal: mn1_data[i],
                    })
                    .collect::<Vec<_>>();
                let morph_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Morphs"),
                    contents: cast_slice(&morphs),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                Some(morph_buffer)
            }
            _ => None,
        };

        Ok(Self {
            vertex_buffer,
            morph_buffer,
            index_format,
            index_buffer,
            num_indices,
        })
    }

    pub fn index_buffer(&self) -> &wgpu::Buffer {
        &self.index_buffer
    }

    pub fn vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub fn num_indices(&self) -> u32 {
        self.num_indices
    }

    pub fn index_format(&self) -> wgpu::IndexFormat {
        self.index_format
    }

    /// Gets slice of the buffer for this accessor ignoring stride
    fn get_data_for_accessor<'a>(
        a: &gltf::Accessor<'a>,
        buffers: &'a [gltf::buffer::Data],
    ) -> Option<&'a [u8]> {
        let view = a.view()?;
        Some(&buffers[view.buffer().index()].0[view.offset()..view.offset() + view.length()])
    }
}
