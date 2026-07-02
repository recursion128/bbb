//! Shared construction for the crate's render pipelines.
//!
//! Every render pipeline in this crate has the same shape: one WGSL module providing the
//! `vs_main`/`fs_main` entry points, one pipeline layout without push constants, and exactly
//! one color target. The builder defaults encode the dominant conventions (`TriangleList`,
//! `Ccw`, `Fill`, no multisampling, no depth, `ColorWrites::ALL`) so call sites only state
//! what differs.

pub(crate) struct RenderPipelineBuilder<'a> {
    device: &'a wgpu::Device,
    pipeline_label: &'a str,
    shader: Option<(&'a str, &'a str)>,
    layout: Option<(&'a str, &'a [&'a wgpu::BindGroupLayout])>,
    vertex_buffers: &'a [wgpu::VertexBufferLayout<'a>],
    color_target: Option<(wgpu::TextureFormat, Option<wgpu::BlendState>)>,
    color_write_mask: wgpu::ColorWrites,
    depth_stencil: Option<wgpu::DepthStencilState>,
    topology: wgpu::PrimitiveTopology,
    cull_mode: Option<wgpu::Face>,
}

impl<'a> RenderPipelineBuilder<'a> {
    pub(crate) fn new(device: &'a wgpu::Device, pipeline_label: &'a str) -> Self {
        Self {
            device,
            pipeline_label,
            shader: None,
            layout: None,
            vertex_buffers: &[],
            color_target: None,
            color_write_mask: wgpu::ColorWrites::ALL,
            depth_stencil: None,
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
        }
    }

    /// The WGSL module providing both the `vs_main` and `fs_main` entry points.
    pub(crate) fn shader(mut self, label: &'a str, wgsl_source: &'a str) -> Self {
        self.shader = Some((label, wgsl_source));
        self
    }

    pub(crate) fn layout(
        mut self,
        label: &'a str,
        bind_group_layouts: &'a [&'a wgpu::BindGroupLayout],
    ) -> Self {
        self.layout = Some((label, bind_group_layouts));
        self
    }

    pub(crate) fn vertex_buffers(mut self, buffers: &'a [wgpu::VertexBufferLayout<'a>]) -> Self {
        self.vertex_buffers = buffers;
        self
    }

    pub(crate) fn color_target(
        mut self,
        format: wgpu::TextureFormat,
        blend: Option<wgpu::BlendState>,
    ) -> Self {
        self.color_target = Some((format, blend));
        self
    }

    pub(crate) fn color_write_mask(mut self, write_mask: wgpu::ColorWrites) -> Self {
        self.color_write_mask = write_mask;
        self
    }

    /// Accepts both a bare [`wgpu::DepthStencilState`] and an `Option` (for callers whose
    /// depth state is already computed as an `Option`, like the sky pipelines).
    pub(crate) fn depth_stencil(
        mut self,
        depth_stencil: impl Into<Option<wgpu::DepthStencilState>>,
    ) -> Self {
        self.depth_stencil = depth_stencil.into();
        self
    }

    pub(crate) fn topology(mut self, topology: wgpu::PrimitiveTopology) -> Self {
        self.topology = topology;
        self
    }

    pub(crate) fn cull_mode(mut self, cull_mode: Option<wgpu::Face>) -> Self {
        self.cull_mode = cull_mode;
        self
    }

    pub(crate) fn build(self) -> wgpu::RenderPipeline {
        let (shader_label, shader_source) = self
            .shader
            .expect("render pipeline builder requires a shader");
        let (layout_label, bind_group_layouts) = self
            .layout
            .expect("render pipeline builder requires a pipeline layout");
        let (color_format, blend) = self
            .color_target
            .expect("render pipeline builder requires a color target");
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(shader_label),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });
        let layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(layout_label),
                bind_group_layouts,
                push_constant_ranges: &[],
            });

        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(self.pipeline_label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: self.vertex_buffers,
                },
                primitive: wgpu::PrimitiveState {
                    topology: self.topology,
                    cull_mode: self.cull_mode,
                    ..Default::default()
                },
                depth_stencil: self.depth_stencil,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: color_format,
                        blend,
                        write_mask: self.color_write_mask,
                    })],
                }),
                multiview: None,
            })
    }
}

/// The crate's dominant depth/stencil shape: default stencil state and default bias.
pub(crate) fn depth_stencil_state(
    format: wgpu::TextureFormat,
    depth_write_enabled: bool,
    depth_compare: wgpu::CompareFunction,
) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format,
        depth_write_enabled,
        depth_compare,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}
