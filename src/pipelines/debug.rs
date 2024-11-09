use bytemuck::{Pod, Zeroable};

use crate::resources::{
    buffer::{Batch, CpuBuffer},
    camera::{CameraBinder, CameraBinding},
};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct DebugVertex {
    position: glam::Vec3,
    color: glam::Vec3,
}

impl DebugVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Self>() as _,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
        ],
    };

    pub fn new(position: glam::Vec3, color: glam::Vec3) -> Self {
        Self { position, color }
    }
}

pub struct DebugPipeline {
    draw_lines: wgpu::RenderPipeline,
    vertex_buffer: CpuBuffer<DebugVertex>,
    index_buffer: CpuBuffer<u32>,
}

impl DebugPipeline {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        // depth_format: wgpu::TextureFormat,
        camera_binder: &CameraBinder,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("debug.wgsl"));
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[camera_binder.layout()],
            push_constant_ranges: &[],
        });
        let draw_lines = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("debug"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "displace_vertices",
                buffers: &[DebugVertex::LAYOUT],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "draw",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::all(),
                })],
            }),
            multiview: None,
        });

        Self {
            draw_lines,
            vertex_buffer: CpuBuffer::with_capacity(device, 64, wgpu::BufferUsages::VERTEX),
            index_buffer: CpuBuffer::with_capacity(device, 64, wgpu::BufferUsages::INDEX),
        }
    }

    pub fn batch<'a>(
        &'a mut self,
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
    ) -> DebugBatch<'a> {
        DebugBatch::new(self, device, queue)
    }

    pub fn clear(&mut self) {
        self.vertex_buffer.clear();
        self.index_buffer.clear();
    }

    pub fn draw_lines<'a: 'b, 'b>(
        &'a self,
        pass: &'b mut wgpu::RenderPass<'a>,
        camera: &'a CameraBinding,
    ) {
        pass.set_pipeline(&self.draw_lines);
        pass.set_bind_group(0, camera.bind_group(), &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice());
        pass.set_index_buffer(self.index_buffer.slice(), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.index_buffer.len(), 0, 0..1);
    }
}

pub struct DebugBatch<'a> {
    current_vertex: u32,
    vertices: Batch<'a, DebugVertex>,
    indices: Batch<'a, u32>,
}

impl<'a> DebugBatch<'a> {
    pub fn new(
        pipeline: &'a mut DebugPipeline,
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
    ) -> Self {
        Self {
            current_vertex: 0,
            vertices: pipeline.vertex_buffer.batch(device, queue),
            indices: pipeline.index_buffer.batch(device, queue),
        }
    }

    #[inline]
    pub fn push_vertex(&mut self, vertex: DebugVertex) -> &mut Self {
        self.vertices.push(vertex);
        self.indices.push(self.current_vertex);
        self.current_vertex += 1;
        self
    }
}