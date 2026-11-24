//! Shared construction for embedded and selected render pipelines.

use anyhow::Result;

#[allow(clippy::too_many_arguments)]
pub(crate) fn create_surface_pipeline<S: AsRef<str>>(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: Option<&wgpu::BindGroupLayout>,
    shaders: Option<(&str, &str)>,
    label: &str,
    embedded_shader: impl FnOnce() -> S,
    defines: &[(&str, Option<&str>)],
    vertex_layouts: &[wgpu::VertexBufferLayout<'_>],
    blend: Option<wgpu::BlendState>,
    depth_write_enabled: bool,
    depth_compare: wgpu::CompareFunction,
    cull_mode: Option<wgpu::Face>,
) -> Result<wgpu::RenderPipeline> {
    if let Some((vertex, fragment)) = shaders {
        return crate::core_shaders::create_selected_surface_pipeline(
            device,
            format,
            bind_group_layout,
            lightmap_bind_group_layout,
            label,
            vertex,
            fragment,
            defines,
            vertex_layouts,
            blend,
            wgpu::ColorWrites::ALL,
            Some(depth_stencil_state(
                crate::gpu::DEPTH_FORMAT,
                depth_write_enabled,
                depth_compare,
            )),
            cull_mode,
        );
    }
    let lit_layouts;
    let layouts = if let Some(lightmap) = lightmap_bind_group_layout {
        lit_layouts = [bind_group_layout, lightmap];
        lit_layouts.as_slice()
    } else {
        std::slice::from_ref(&bind_group_layout)
    };
    let shader = embedded_shader();
    Ok(RenderPipelineBuilder::new(device, label)
        .shader(label, shader.as_ref())
        .layout(label, layouts)
        .vertex_buffers(vertex_layouts)
        .color_target(format, blend)
        .cull_mode(cull_mode)
        .depth_stencil(depth_stencil_state(
            crate::gpu::DEPTH_FORMAT,
            depth_write_enabled,
            depth_compare,
        ))
        .build())
}

pub struct RenderPipelineBuilder<'a> {
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
    pub fn new(device: &'a wgpu::Device, pipeline_label: &'a str) -> Self {
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
    pub fn shader(mut self, label: &'a str, wgsl_source: &'a str) -> Self {
        self.shader = Some((label, wgsl_source));
        self
    }

    pub fn layout(
        mut self,
        label: &'a str,
        bind_group_layouts: &'a [&'a wgpu::BindGroupLayout],
    ) -> Self {
        self.layout = Some((label, bind_group_layouts));
        self
    }

    pub fn vertex_buffers(mut self, buffers: &'a [wgpu::VertexBufferLayout<'a>]) -> Self {
        self.vertex_buffers = buffers;
        self
    }

    pub fn color_target(
        mut self,
        format: wgpu::TextureFormat,
        blend: Option<wgpu::BlendState>,
    ) -> Self {
        self.color_target = Some((format, blend));
        self
    }

    pub fn color_write_mask(mut self, write_mask: wgpu::ColorWrites) -> Self {
        self.color_write_mask = write_mask;
        self
    }

    /// Accepts both a bare [`wgpu::DepthStencilState`] and an `Option` (for callers whose
    /// depth state is already computed as an `Option`, like the sky pipelines).
    pub fn depth_stencil(
        mut self,
        depth_stencil: impl Into<Option<wgpu::DepthStencilState>>,
    ) -> Self {
        self.depth_stencil = depth_stencil.into();
        self
    }

    pub fn topology(mut self, topology: wgpu::PrimitiveTopology) -> Self {
        self.topology = topology;
        self
    }

    pub fn cull_mode(mut self, cull_mode: Option<wgpu::Face>) -> Self {
        self.cull_mode = cull_mode;
        self
    }

    pub fn build(self) -> wgpu::RenderPipeline {
        let (shader_label, shader_source) = self
            .shader
            .expect("render pipeline builder requires a shader");
        let (layout_label, bind_group_layouts) = self
            .layout
            .expect("render pipeline builder requires a pipeline layout");
        let (color_format, blend) = self
            .color_target
            .expect("render pipeline builder requires a color target");
        let bind_group_layouts = bind_group_layouts
            .iter()
            .copied()
            .map(Some)
            .collect::<Vec<_>>();
        let vertex_buffers = self
            .vertex_buffers
            .iter()
            .cloned()
            .map(Some)
            .collect::<Vec<_>>();
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
                bind_group_layouts: &bind_group_layouts,
                immediate_size: 0,
            });

        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(self.pipeline_label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &vertex_buffers,
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
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: color_format,
                        blend,
                        write_mask: self.color_write_mask,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
    }
}

/// The crate's dominant depth/stencil shape: default stencil state and default bias.
pub fn depth_stencil_state(
    format: wgpu::TextureFormat,
    depth_write_enabled: bool,
    depth_compare: wgpu::CompareFunction,
) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format,
        depth_write_enabled: Some(depth_write_enabled),
        depth_compare: Some(depth_compare),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

pub fn optional_bind_group_layouts<'a>(
    layouts: &'a [&'a wgpu::BindGroupLayout],
) -> Vec<Option<&'a wgpu::BindGroupLayout>> {
    layouts.iter().copied().map(Some).collect()
}

pub fn optional_vertex_layouts<'a>(
    layouts: &[wgpu::VertexBufferLayout<'a>],
) -> Vec<Option<wgpu::VertexBufferLayout<'a>>> {
    layouts.iter().cloned().map(Some).collect()
}
