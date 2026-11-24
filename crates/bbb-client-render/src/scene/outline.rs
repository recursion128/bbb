use anyhow::Result;

use crate::gpu::{DepthTarget, DEPTH_FORMAT};
use crate::pipeline_builder::RenderPipelineBuilder;
use crate::texture_sampling::{create_nearest_clamp_sampler, create_texture_bind_group};

const ENTITY_OUTLINE_COMPOSITE_SHADER: &str = r#"
@group(0) @binding(0)
var entity_outline_texture: texture_2d<f32>;

@group(0) @binding(1)
var entity_outline_sampler: sampler;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let position = positions[vertex_index];
    var out: VertexOut;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.uv = vec2<f32>(position.x * 0.5 + 0.5, 0.5 - position.y * 0.5);
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(entity_outline_texture, entity_outline_sampler, input.uv);
}
"#;

pub(crate) const ENTITY_OUTLINE_COMPOSITE_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Zero,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

pub(crate) const ENTITY_OUTLINE_COMPOSITE_WRITE_MASK: wgpu::ColorWrites = wgpu::ColorWrites::COLOR;

pub(crate) struct EntityOutlineTarget {
    pub(crate) _texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) _sampler: wgpu::Sampler,
    pub(crate) depth: DepthTarget,
    pub(crate) bind_group: wgpu::BindGroup,
}

pub(crate) fn create_entity_outline_bind_group_layout(
    device: &wgpu::Device,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-entity-outline-bind-group-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

pub(crate) fn create_entity_outline_target(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> EntityOutlineTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-entity-outline-color"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = create_nearest_clamp_sampler(device, "bbb-entity-outline-sampler");
    let bind_group = create_texture_bind_group(
        device,
        layout,
        "bbb-entity-outline-bind-group",
        &view,
        &sampler,
    );
    EntityOutlineTarget {
        _texture: texture,
        view,
        _sampler: sampler,
        depth: create_entity_outline_depth_target(device, width, height),
        bind_group,
    }
}

pub(crate) fn create_entity_outline_composite_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedShaderPair>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        let (vertex, fragment) = crate::core_shaders::compile_selected_shader_stages(
            device,
            "bbb-selected-entity-outline-composite",
            &shaders.vertex,
            &shaders.fragment,
        )?;
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bbb-selected-entity-outline-composite-layout"),
            bind_group_layouts: &[Some(bind_group_layout)],
            immediate_size: 0,
        });
        return Ok(
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("bbb-selected-entity-outline-composite-pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &vertex,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &fragment,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(ENTITY_OUTLINE_COMPOSITE_BLEND),
                        write_mask: ENTITY_OUTLINE_COMPOSITE_WRITE_MASK,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            }),
        );
    }
    Ok(create_entity_outline_fullscreen_pipeline(
        device,
        format,
        bind_group_layout,
        "bbb-entity-outline-composite-shader",
        "bbb-entity-outline-composite-pipeline-layout",
        "bbb-entity-outline-composite-pipeline",
        ENTITY_OUTLINE_COMPOSITE_SHADER,
        Some(ENTITY_OUTLINE_COMPOSITE_BLEND),
        ENTITY_OUTLINE_COMPOSITE_WRITE_MASK,
    ))
}

#[allow(clippy::too_many_arguments)]
fn create_entity_outline_fullscreen_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shader_label: &str,
    pipeline_layout_label: &str,
    pipeline_label: &str,
    shader_source: &str,
    blend: Option<wgpu::BlendState>,
    write_mask: wgpu::ColorWrites,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, pipeline_label)
        .shader(shader_label, shader_source)
        .layout(pipeline_layout_label, &[bind_group_layout])
        .color_target(format, blend)
        .color_write_mask(write_mask)
        .build()
}

fn create_entity_outline_depth_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> DepthTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-entity-outline-depth"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    DepthTarget {
        _texture: texture,
        view,
    }
}

#[cfg(test)]
mod tests {
    use super::{ENTITY_OUTLINE_COMPOSITE_BLEND, ENTITY_OUTLINE_COMPOSITE_WRITE_MASK};

    #[test]
    fn entity_outline_composite_blend_matches_vanilla_blit() {
        assert_eq!(
            ENTITY_OUTLINE_COMPOSITE_BLEND.color.src_factor,
            wgpu::BlendFactor::SrcAlpha
        );
        assert_eq!(
            ENTITY_OUTLINE_COMPOSITE_BLEND.color.dst_factor,
            wgpu::BlendFactor::OneMinusSrcAlpha
        );
        assert_eq!(
            ENTITY_OUTLINE_COMPOSITE_BLEND.alpha.src_factor,
            wgpu::BlendFactor::Zero
        );
        assert_eq!(
            ENTITY_OUTLINE_COMPOSITE_BLEND.alpha.dst_factor,
            wgpu::BlendFactor::One
        );
        assert_eq!(
            ENTITY_OUTLINE_COMPOSITE_WRITE_MASK,
            wgpu::ColorWrites::COLOR
        );
    }
}
