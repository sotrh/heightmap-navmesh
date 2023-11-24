use crate::resources::{
    camera::{CameraBinder, CameraBinding},
    model::{Model, Vertex},
};

pub struct Fur {
    draw: wgpu::RenderPipeline,
    num_layers: u32,
}

impl Fur {
    pub fn new(
        device: &wgpu::Device,
        num_layers: u32,
        surface_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        camera_binder: &CameraBinder,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("fur.wgsl"));
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[camera_binder.layout()],
            push_constant_ranges: &[],
        });
        let draw = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Fur"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "displace_vertices",
                buffers: &[Vertex::LAYOUT],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                bias: wgpu::DepthBiasState::default(),
                stencil: wgpu::StencilState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "shade_fur",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::all(),
                })],
            }),
            multiview: None,
        });

        Self { draw, num_layers }
    }

    pub fn draw<'a: 'b, 'b>(
        &'a self,
        pass: &'b mut wgpu::RenderPass<'a>,
        model: &'a Model,
        camera: &'a CameraBinding,
    ) {
        pass.set_pipeline(&self.draw);
        pass.set_bind_group(0, camera.bind_group(), &[]);
        pass.set_index_buffer(model.index_buffer().slice(..), model.index_format());
        pass.set_vertex_buffer(0, model.vertex_buffer().slice(..));
        pass.draw_indexed(0..model.num_indices(), 0, 0..self.num_layers);
    }
}
