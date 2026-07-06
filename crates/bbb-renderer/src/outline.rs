use crate::gpu::{DepthTarget, DEPTH_FORMAT};
use crate::pipeline_builder::RenderPipelineBuilder;

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

const ENTITY_OUTLINE_SOBEL_SHADER: &str = r#"
@group(0) @binding(0)
var outline_texture: texture_2d<f32>;

@group(0) @binding(1)
var outline_sampler: sampler;

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
    let size = textureDimensions(outline_texture);
    let one_texel = vec2<f32>(1.0 / f32(size.x), 1.0 / f32(size.y));

    let center = textureSample(outline_texture, outline_sampler, input.uv);
    let left = textureSample(outline_texture, outline_sampler, input.uv - vec2<f32>(one_texel.x, 0.0));
    let right = textureSample(outline_texture, outline_sampler, input.uv + vec2<f32>(one_texel.x, 0.0));
    let up = textureSample(outline_texture, outline_sampler, input.uv - vec2<f32>(0.0, one_texel.y));
    let down = textureSample(outline_texture, outline_sampler, input.uv + vec2<f32>(0.0, one_texel.y));

    let left_diff = abs(center.a - left.a);
    let right_diff = abs(center.a - right.a);
    let up_diff = abs(center.a - up.a);
    let down_diff = abs(center.a - down.a);
    let total = clamp(left_diff + right_diff + up_diff + down_diff, 0.0, 1.0);
    let out_color = center.rgb * center.a
        + left.rgb * left.a
        + right.rgb * right.a
        + up.rgb * up.a
        + down.rgb * down.a;
    return vec4<f32>(out_color * 0.2, total);
}
"#;

const ENTITY_OUTLINE_BLUR_HORIZONTAL_SHADER: &str = r#"
@group(0) @binding(0)
var outline_texture: texture_2d<f32>;

@group(0) @binding(1)
var outline_sampler: sampler;

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
    let size = textureDimensions(outline_texture);
    let one_texel = vec2<f32>(1.0 / f32(size.x), 1.0 / f32(size.y));
    let sample_step = one_texel * vec2<f32>(1.0, 0.0);

    var blurred = textureSample(outline_texture, outline_sampler, input.uv + sample_step * -1.5);
    blurred += textureSample(outline_texture, outline_sampler, input.uv + sample_step * 0.5);
    blurred += textureSample(outline_texture, outline_sampler, input.uv + sample_step * 2.0) * 0.5;
    return vec4<f32>((blurred / 2.5).rgb, blurred.a);
}
"#;

const ENTITY_OUTLINE_BLUR_VERTICAL_SHADER: &str = r#"
@group(0) @binding(0)
var outline_texture: texture_2d<f32>;

@group(0) @binding(1)
var outline_sampler: sampler;

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
    let size = textureDimensions(outline_texture);
    let one_texel = vec2<f32>(1.0 / f32(size.x), 1.0 / f32(size.y));
    let sample_step = one_texel * vec2<f32>(0.0, 1.0);

    var blurred = textureSample(outline_texture, outline_sampler, input.uv + sample_step * -1.5);
    blurred += textureSample(outline_texture, outline_sampler, input.uv + sample_step * 0.5);
    blurred += textureSample(outline_texture, outline_sampler, input.uv + sample_step * 2.0) * 0.5;
    return vec4<f32>((blurred / 2.5).rgb, blurred.a);
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
    pub(super) _swap_texture: wgpu::Texture,
    pub(super) swap_view: wgpu::TextureView,
    pub(super) _sampler: wgpu::Sampler,
    pub(super) _linear_sampler: wgpu::Sampler,
    pub(super) depth: DepthTarget,
    pub(super) bind_group: wgpu::BindGroup,
    pub(super) linear_bind_group: wgpu::BindGroup,
    pub(super) swap_bind_group: wgpu::BindGroup,
    pub(super) swap_linear_bind_group: wgpu::BindGroup,
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
    let texture = create_entity_outline_color_texture(
        device,
        format,
        width,
        height,
        "bbb-entity-outline-color",
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let swap_texture = create_entity_outline_color_texture(
        device,
        format,
        width,
        height,
        "bbb-entity-outline-swap",
    );
    let swap_view = swap_texture.create_view(&wgpu::TextureViewDescriptor::default());
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
    let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bbb-entity-outline-linear-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = create_entity_outline_texture_bind_group(
        device,
        layout,
        "bbb-entity-outline-bind-group",
        &view,
        &sampler,
    );
    let linear_bind_group = create_entity_outline_texture_bind_group(
        device,
        layout,
        "bbb-entity-outline-linear-bind-group",
        &view,
        &linear_sampler,
    );
    let swap_bind_group = create_entity_outline_texture_bind_group(
        device,
        layout,
        "bbb-entity-outline-swap-bind-group",
        &swap_view,
        &sampler,
    );
    let swap_linear_bind_group = create_entity_outline_texture_bind_group(
        device,
        layout,
        "bbb-entity-outline-swap-linear-bind-group",
        &swap_view,
        &linear_sampler,
    );
    let depth = create_entity_outline_depth_target(device, width, height);

    EntityOutlineTarget {
        _texture: texture,
        view,
        _swap_texture: swap_texture,
        swap_view,
        _sampler: sampler,
        _linear_sampler: linear_sampler,
        depth,
        bind_group,
        linear_bind_group,
        swap_bind_group,
        swap_linear_bind_group,
    }
}

pub(super) fn create_entity_outline_sobel_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_entity_outline_fullscreen_pipeline(
        device,
        format,
        bind_group_layout,
        "bbb-entity-outline-sobel-shader",
        "bbb-entity-outline-sobel-pipeline-layout",
        "bbb-entity-outline-sobel-pipeline",
        ENTITY_OUTLINE_SOBEL_SHADER,
        None,
        wgpu::ColorWrites::ALL,
    )
}

pub(super) fn create_entity_outline_blur_horizontal_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_entity_outline_fullscreen_pipeline(
        device,
        format,
        bind_group_layout,
        "bbb-entity-outline-blur-horizontal-shader",
        "bbb-entity-outline-blur-horizontal-pipeline-layout",
        "bbb-entity-outline-blur-horizontal-pipeline",
        ENTITY_OUTLINE_BLUR_HORIZONTAL_SHADER,
        None,
        wgpu::ColorWrites::ALL,
    )
}

pub(super) fn create_entity_outline_blur_vertical_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_entity_outline_fullscreen_pipeline(
        device,
        format,
        bind_group_layout,
        "bbb-entity-outline-blur-vertical-shader",
        "bbb-entity-outline-blur-vertical-pipeline-layout",
        "bbb-entity-outline-blur-vertical-pipeline",
        ENTITY_OUTLINE_BLUR_VERTICAL_SHADER,
        None,
        wgpu::ColorWrites::ALL,
    )
}

pub(super) fn create_entity_outline_blit_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_entity_outline_fullscreen_pipeline(
        device,
        format,
        bind_group_layout,
        "bbb-entity-outline-blit-shader",
        "bbb-entity-outline-blit-pipeline-layout",
        "bbb-entity-outline-blit-pipeline",
        ENTITY_OUTLINE_COMPOSITE_SHADER,
        None,
        wgpu::ColorWrites::ALL,
    )
}

pub(super) fn create_entity_outline_composite_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_entity_outline_fullscreen_pipeline(
        device,
        format,
        bind_group_layout,
        "bbb-entity-outline-composite-shader",
        "bbb-entity-outline-composite-pipeline-layout",
        "bbb-entity-outline-composite-pipeline",
        ENTITY_OUTLINE_COMPOSITE_SHADER,
        Some(ENTITY_OUTLINE_COMPOSITE_BLEND),
        ENTITY_OUTLINE_COMPOSITE_WRITE_MASK,
    )
}

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

fn create_entity_outline_color_texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    label: &str,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
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
    })
}

fn create_entity_outline_texture_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    label: &str,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
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
    use super::{
        ENTITY_OUTLINE_BLUR_HORIZONTAL_SHADER, ENTITY_OUTLINE_BLUR_VERTICAL_SHADER,
        ENTITY_OUTLINE_COMPOSITE_BLEND, ENTITY_OUTLINE_COMPOSITE_WRITE_MASK,
        ENTITY_OUTLINE_SOBEL_SHADER,
    };

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

    #[test]
    fn entity_outline_post_shaders_match_vanilla_kernel_shape() {
        assert!(
            ENTITY_OUTLINE_SOBEL_SHADER.contains("left_diff + right_diff + up_diff + down_diff")
        );
        assert!(ENTITY_OUTLINE_SOBEL_SHADER.contains("out_color * 0.2"));
        for shader in [
            ENTITY_OUTLINE_BLUR_HORIZONTAL_SHADER,
            ENTITY_OUTLINE_BLUR_VERTICAL_SHADER,
        ] {
            assert!(shader.contains("sample_step * -1.5"));
            assert!(shader.contains("sample_step * 0.5"));
            assert!(shader.contains("sample_step * 2.0"));
            assert!(shader.contains("blurred / 2.5"));
        }
        assert!(ENTITY_OUTLINE_BLUR_HORIZONTAL_SHADER.contains("vec2<f32>(1.0, 0.0)"));
        assert!(ENTITY_OUTLINE_BLUR_VERTICAL_SHADER.contains("vec2<f32>(0.0, 1.0)"));
    }
}
