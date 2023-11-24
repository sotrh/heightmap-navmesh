pub struct RenderFormat {
    targets: Vec<wgpu::ColorTargetState>,
    depth: Option<wgpu::DepthStencilState>,
}

impl RenderFormat {
    pub fn new(targets: Vec<wgpu::ColorTargetState>, depth: Option<wgpu::DepthStencilState>) -> Self {
        Self { targets, depth }
    }

    pub fn targets(&self) -> &[wgpu::ColorTargetState] {
        &self.targets
    }
}
