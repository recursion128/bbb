use wgpu::util::DeviceExt;

use crate::pipeline_builder::RenderPipelineBuilder;

macro_rules! screen_blur_shader {
    ($direction:literal) => {
        concat!(
            r#"
@group(0) @binding(0)
var source_texture: texture_2d<f32>;

@group(0) @binding(1)
var source_sampler: sampler;

@group(0) @binding(2)
var<uniform> blur_config: vec4<f32>;

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
    let size = textureDimensions(source_texture);
    let one_texel = vec2<f32>(1.0 / f32(size.x), 1.0 / f32(size.y));
    let sample_step = one_texel * "#,
            $direction,
            r#";
    let radius = floor(blur_config.x + 0.5);
    var blurred = vec4<f32>(0.0);
    var offset = -radius + 0.5;
    loop {
        if offset > radius {
            break;
        }
        blurred += textureSample(source_texture, source_sampler, input.uv + sample_step * offset);
        offset += 2.0;
    }
    blurred += textureSample(source_texture, source_sampler, input.uv + sample_step * radius) * 0.5;
    return blurred / (radius + 0.5);
}
"#,
        )
    };
}

pub const VANILLA_DEFAULT_MENU_BACKGROUND_BLURRINESS: u8 = 5;
pub const VANILLA_MAX_MENU_BACKGROUND_BLURRINESS: u8 = 10;

const SCREEN_BLUR_HORIZONTAL_SHADER: &str = screen_blur_shader!("vec2<f32>(1.0, 0.0)");
const SCREEN_BLUR_VERTICAL_SHADER: &str = screen_blur_shader!("vec2<f32>(0.0, 1.0)");

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ScreenBlurUniform {
    radius: [f32; 4],
}

pub(super) struct ScreenBlurTarget {
    pub(super) texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    _swap_texture: wgpu::Texture,
    pub(super) swap_view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(super) bind_group: wgpu::BindGroup,
    pub(super) swap_bind_group: wgpu::BindGroup,
    pub(super) uniform_buffer: wgpu::Buffer,
}

pub(super) fn sanitize_menu_background_blurriness(value: u8) -> f32 {
    f32::from(value.min(VANILLA_MAX_MENU_BACKGROUND_BLURRINESS))
}

pub(super) fn create_screen_blur_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-screen-blur-bind-group-layout"),
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
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

pub(super) fn create_screen_blur_target(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> ScreenBlurTarget {
    let texture = create_screen_blur_texture(
        device,
        format,
        width,
        height,
        "bbb-screen-blur-color",
        wgpu::TextureUsages::COPY_DST,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let swap_texture = create_screen_blur_texture(
        device,
        format,
        width,
        height,
        "bbb-screen-blur-swap",
        wgpu::TextureUsages::empty(),
    );
    let swap_view = swap_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bbb-screen-blur-linear-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    });
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-screen-blur-uniform"),
        contents: bytemuck::bytes_of(&ScreenBlurUniform {
            radius: [
                f32::from(VANILLA_DEFAULT_MENU_BACKGROUND_BLURRINESS),
                0.0,
                0.0,
                0.0,
            ],
        }),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let bind_group = create_screen_blur_bind_group(
        device,
        layout,
        "bbb-screen-blur-color-bind-group",
        &view,
        &sampler,
        &uniform_buffer,
    );
    let swap_bind_group = create_screen_blur_bind_group(
        device,
        layout,
        "bbb-screen-blur-swap-bind-group",
        &swap_view,
        &sampler,
        &uniform_buffer,
    );
    ScreenBlurTarget {
        texture,
        view,
        _swap_texture: swap_texture,
        swap_view,
        _sampler: sampler,
        bind_group,
        swap_bind_group,
        uniform_buffer,
    }
}

pub(super) fn write_screen_blur_radius(
    queue: &wgpu::Queue,
    target: &ScreenBlurTarget,
    radius: f32,
) {
    queue.write_buffer(
        &target.uniform_buffer,
        0,
        bytemuck::bytes_of(&ScreenBlurUniform {
            radius: [radius, 0.0, 0.0, 0.0],
        }),
    );
}

pub(super) fn create_screen_blur_horizontal_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_screen_blur_pipeline(
        device,
        format,
        layout,
        "bbb-screen-blur-horizontal-shader",
        "bbb-screen-blur-horizontal-pipeline",
        SCREEN_BLUR_HORIZONTAL_SHADER,
    )
}

pub(super) fn create_screen_blur_vertical_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_screen_blur_pipeline(
        device,
        format,
        layout,
        "bbb-screen-blur-vertical-shader",
        "bbb-screen-blur-vertical-pipeline",
        SCREEN_BLUR_VERTICAL_SHADER,
    )
}

fn create_screen_blur_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layout: &wgpu::BindGroupLayout,
    shader_label: &str,
    pipeline_label: &str,
    shader: &str,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, pipeline_label)
        .shader(shader_label, shader)
        .layout("bbb-screen-blur-pipeline-layout", &[layout])
        .color_target(format, None)
        .build()
}

fn create_screen_blur_texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    label: &str,
    extra_usage: wgpu::TextureUsages,
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
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | extra_usage,
        view_formats: &[],
    })
}

fn create_screen_blur_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    label: &str,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    uniform_buffer: &wgpu::Buffer,
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
            wgpu::BindGroupEntry {
                binding: 2,
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_blur_radius_matches_vanilla_option_range() {
        assert_eq!(sanitize_menu_background_blurriness(0), 0.0);
        assert_eq!(sanitize_menu_background_blurriness(5), 5.0);
        assert_eq!(sanitize_menu_background_blurriness(10), 10.0);
        assert_eq!(sanitize_menu_background_blurriness(u8::MAX), 10.0);
    }

    #[test]
    fn box_blur_shader_keeps_vanilla_linear_half_step_sampling() {
        for shader in [SCREEN_BLUR_HORIZONTAL_SHADER, SCREEN_BLUR_VERTICAL_SHADER] {
            assert!(shader.contains("var offset = -radius + 0.5"));
            assert!(shader.contains("offset += 2.0"));
            assert!(shader.contains("sample_step * radius) * 0.5"));
            assert!(shader.contains("blurred / (radius + 0.5)"));
        }
    }
}
