use crate::gpu::{DepthTarget, DEPTH_FORMAT};

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
    let positions = array<vec2<f32>, 3>(
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

/// Vanilla `BlendFunction.ENTITY_OUTLINE_BLIT`:
/// colour `srcAlpha / oneMinusSrcAlpha`, alpha `zero / one`.
pub(super) const ENTITY_OUTLINE_COMPOSITE_BLEND: wgpu::BlendState = wgpu::BlendState {
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

pub(super) const ENTITY_OUTLINE_COMPOSITE_WRITE_MASK: wgpu::ColorWrites = wgpu::ColorWrites::COLOR;

pub(super) struct EntityOutlineTarget {
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) _sampler: wgpu::Sampler,
    pub(super) depth: DepthTarget,
    pub(super) bind_group: wgpu::BindGroup,
}

pub(super) fn create_entity_outline_bind_group_layout(
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

pub(super) fn create_entity_outline_target(
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
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bbb-entity-outline-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-entity-outline-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    let depth = create_entity_outline_depth_target(device, width, height);

    EntityOutlineTarget {
        _texture: texture,
        view,
        _sampler: sampler,
        depth,
        bind_group,
    }
}

pub(super) fn create_entity_outline_composite_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-entity-outline-composite-shader"),
        source: wgpu::ShaderSource::Wgsl(ENTITY_OUTLINE_COMPOSITE_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-entity-outline-composite-pipeline-layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-entity-outline-composite-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(ENTITY_OUTLINE_COMPOSITE_BLEND),
                write_mask: ENTITY_OUTLINE_COMPOSITE_WRITE_MASK,
            })],
        }),
        multiview: None,
    })
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
