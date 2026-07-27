use wgpu::util::DeviceExt;

use crate::pipeline_builder::RenderPipelineBuilder;

mod graph;

use graph::{
    CameraPostEffect, PostUniform, CREEPER_BITS_UNIFORM, CREEPER_COLOR_UNIFORM,
    IDENTITY_BLIT_UNIFORM, INVERT_UNIFORM, SPIDER_BLIT_UNIFORM, SPIDER_PASS_UNIFORMS,
};

const POST_SHADER_BINDINGS: &str = r#"
struct PostUniform {
    p0: vec4<f32>,
    p1: vec4<f32>,
    p2: vec4<f32>,
    p3: vec4<f32>,
};

@group(0) @binding(0)
var source_texture: texture_2d<f32>;

@group(0) @binding(1)
var blur_texture: texture_2d<f32>;

@group(0) @binding(2)
var source_sampler: sampler;

@group(0) @binding(3)
var<uniform> config: PostUniform;
"#;

const FULLSCREEN_VERTEX_SHADER: &str = r#"
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
"#;

const CREEPER_COLOR_FRAGMENT_SHADER: &str = r#"
@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let input_texel = textureSample(source_texture, source_sampler, input.uv);
    let output_color = vec3<f32>(
        dot(input_texel.rgb, config.p0.xyz),
        dot(input_texel.rgb, config.p1.xyz),
        dot(input_texel.rgb, config.p2.xyz),
    );
    let luma = dot(output_color, vec3<f32>(0.3, 0.59, 0.11));
    let chroma = output_color - vec3<f32>(luma);
    return vec4<f32>(chroma * 1.8 + vec3<f32>(luma), 1.0);
}
"#;

const CREEPER_BITS_FRAGMENT_SHADER: &str = r#"
@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let dimensions = textureDimensions(source_texture);
    let input_size = vec2<f32>(f32(dimensions.x), f32(dimensions.y));
    let mosaic_input_size = input_size / config.p0.y;
    let fractional_pixel = fract(input.uv * mosaic_input_size) / mosaic_input_size;
    var base_texel = textureSample(source_texture, source_sampler, input.uv - fractional_pixel);
    let fractional_texel =
        base_texel.rgb - fract(base_texel.rgb * config.p0.x) / config.p0.x;
    let luma = dot(fractional_texel, vec3<f32>(0.3, 0.59, 0.11));
    let chroma = (fractional_texel - vec3<f32>(luma)) * 1.5;
    base_texel = vec4<f32>(vec3<f32>(luma) + chroma, 1.0);
    return base_texel;
}
"#;

const INVERT_FRAGMENT_SHADER: &str = r#"
@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let diffuse_color = textureSample(source_texture, source_sampler, input.uv);
    let inverse_color = vec4<f32>(1.0) - diffuse_color;
    let output_color = mix(diffuse_color, inverse_color, config.p0.x);
    return vec4<f32>(output_color.rgb, 1.0);
}
"#;

const BLIT_FRAGMENT_SHADER: &str = r#"
@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(source_texture, source_sampler, input.uv) * config.p0;
}
"#;

const SPIDER_VERTEX_SHADER: &str = r#"
struct SpiderVertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) vanilla_uv: vec2<f32>,
    @location(1) scaled_vanilla_uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> SpiderVertexOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let position = positions[vertex_index];
    let vanilla_uv = vec2<f32>(
        position.x * 0.5 + 0.5,
        position.y * 0.5 + 0.5,
    );
    let rotation = config.p1.x * 0.0174532925;
    let cosine = cos(rotation);
    let sine = sin(rotation);
    let rotated_uv = vec2<f32>(
        vanilla_uv.x * cosine - vanilla_uv.y * sine,
        vanilla_uv.y * cosine + vanilla_uv.x * sine,
    );
    var out: SpiderVertexOut;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.vanilla_uv = vanilla_uv;
    out.scaled_vanilla_uv = rotated_uv * config.p0.xy + config.p0.zw;
    return out;
}
"#;

const SPIDER_CLIP_FRAGMENT_SHADER: &str = r#"
@fragment
fn fs_main(input: SpiderVertexOut) -> @location(0) vec4<f32> {
    let source_uv = vec2<f32>(
        input.scaled_vanilla_uv.x,
        1.0 - input.scaled_vanilla_uv.y,
    );
    let blur_uv = vec2<f32>(input.vanilla_uv.x, 1.0 - input.vanilla_uv.y);
    let scaled_texel = textureSample(source_texture, source_sampler, source_uv);
    let blur_texel = textureSample(blur_texture, source_sampler, blur_uv);
    var output_texel = scaled_texel;

    if input.scaled_vanilla_uv.x < config.p2.x {
        output_texel = blur_texel;
    }
    if input.scaled_vanilla_uv.y < config.p2.y {
        output_texel = blur_texel;
    }
    if input.scaled_vanilla_uv.x > config.p2.z {
        output_texel = blur_texel;
    }
    if input.scaled_vanilla_uv.y > config.p2.w {
        output_texel = blur_texel;
    }

    if input.scaled_vanilla_uv.x < config.p3.x {
        let amount =
            (config.p2.x - input.scaled_vanilla_uv.x) / (config.p2.x - config.p3.x);
        output_texel = mix(blur_texel, output_texel, amount);
    }
    if input.scaled_vanilla_uv.y < config.p3.y {
        let amount =
            (config.p2.y - input.scaled_vanilla_uv.y) / (config.p2.y - config.p3.y);
        output_texel = mix(blur_texel, output_texel, amount);
    }
    if input.scaled_vanilla_uv.x > config.p3.z {
        let amount =
            (config.p2.z - input.scaled_vanilla_uv.x) / (config.p2.z - config.p3.z);
        output_texel = mix(blur_texel, output_texel, amount);
    }
    if input.scaled_vanilla_uv.y > config.p3.w {
        let amount =
            (config.p2.w - input.scaled_vanilla_uv.y) / (config.p2.w - config.p3.w);
        output_texel = mix(blur_texel, output_texel, amount);
    }
    return vec4<f32>(output_texel.rgb, 1.0);
}
"#;

struct CameraPostEffectPipelines {
    creeper_color: wgpu::RenderPipeline,
    creeper_bits: wgpu::RenderPipeline,
    invert: wgpu::RenderPipeline,
    blit: wgpu::RenderPipeline,
    spider_clip: wgpu::RenderPipeline,
}

struct CameraPostEffectUniforms {
    large_blur: wgpu::Buffer,
    small_blur: wgpu::Buffer,
    creeper_color: wgpu::Buffer,
    creeper_bits: wgpu::Buffer,
    invert: wgpu::Buffer,
    identity_blit: wgpu::Buffer,
    spider_passes: [wgpu::Buffer; 5],
    spider_blit: wgpu::Buffer,
}

struct PostTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

struct CameraPostEffectTargets {
    source: PostTarget,
    temp: PostTarget,
    swap: PostTarget,
    large_blur: PostTarget,
    small_blur: PostTarget,
    large_blur_horizontal: wgpu::BindGroup,
    large_blur_vertical: wgpu::BindGroup,
    small_blur_horizontal: wgpu::BindGroup,
    small_blur_vertical: wgpu::BindGroup,
    creeper_color: wgpu::BindGroup,
    creeper_bits: wgpu::BindGroup,
    invert: wgpu::BindGroup,
    identity_blit: wgpu::BindGroup,
    spider_passes: [wgpu::BindGroup; 5],
    spider_blit: wgpu::BindGroup,
}

pub(super) struct CameraPostEffectGpu {
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    bind_group_layout: wgpu::BindGroupLayout,
    nearest_sampler: wgpu::Sampler,
    linear_sampler: wgpu::Sampler,
    pipelines: CameraPostEffectPipelines,
    uniforms: CameraPostEffectUniforms,
    targets: CameraPostEffectTargets,
}

impl CameraPostEffectGpu {
    pub(super) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        screen_blur_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let bind_group_layout = create_post_bind_group_layout(device);
        let nearest_sampler = create_sampler(device, "bbb-camera-post-nearest-sampler", false);
        let linear_sampler = create_sampler(device, "bbb-camera-post-linear-sampler", true);
        let pipelines = CameraPostEffectPipelines {
            creeper_color: create_post_pipeline(
                device,
                format,
                &bind_group_layout,
                "bbb-camera-post-creeper-color",
                FULLSCREEN_VERTEX_SHADER,
                CREEPER_COLOR_FRAGMENT_SHADER,
            ),
            creeper_bits: create_post_pipeline(
                device,
                format,
                &bind_group_layout,
                "bbb-camera-post-creeper-bits",
                FULLSCREEN_VERTEX_SHADER,
                CREEPER_BITS_FRAGMENT_SHADER,
            ),
            invert: create_post_pipeline(
                device,
                format,
                &bind_group_layout,
                "bbb-camera-post-invert",
                FULLSCREEN_VERTEX_SHADER,
                INVERT_FRAGMENT_SHADER,
            ),
            blit: create_post_pipeline(
                device,
                format,
                &bind_group_layout,
                "bbb-camera-post-blit",
                FULLSCREEN_VERTEX_SHADER,
                BLIT_FRAGMENT_SHADER,
            ),
            spider_clip: create_post_pipeline(
                device,
                format,
                &bind_group_layout,
                "bbb-camera-post-spider-clip",
                SPIDER_VERTEX_SHADER,
                SPIDER_CLIP_FRAGMENT_SHADER,
            ),
        };
        let uniforms = create_uniforms(device);
        let targets = create_targets(
            device,
            format,
            width,
            height,
            screen_blur_layout,
            &bind_group_layout,
            &nearest_sampler,
            &linear_sampler,
            &uniforms,
        );
        Self {
            format,
            width: width.max(1),
            height: height.max(1),
            bind_group_layout,
            nearest_sampler,
            linear_sampler,
            pipelines,
            uniforms,
            targets,
        }
    }

    pub(super) fn resize(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        screen_blur_layout: &wgpu::BindGroupLayout,
    ) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.targets = create_targets(
            device,
            self.format,
            self.width,
            self.height,
            screen_blur_layout,
            &self.bind_group_layout,
            &self.nearest_sampler,
            &self.linear_sampler,
            &self.uniforms,
        );
    }

    pub(super) fn render(
        &self,
        id: Option<&str>,
        active: bool,
        frame_texture: &wgpu::Texture,
        frame_view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        blur_horizontal_pipeline: &wgpu::RenderPipeline,
        blur_vertical_pipeline: &wgpu::RenderPipeline,
    ) -> u64 {
        let Some(effect) = active.then(|| CameraPostEffect::from_id(id)).flatten() else {
            return 0;
        };
        encoder.copy_texture_to_texture(
            frame_texture.as_image_copy(),
            self.targets.source.texture.as_image_copy(),
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        match effect {
            CameraPostEffect::Creeper => self.render_creeper(frame_view, encoder),
            CameraPostEffect::Spider => self.render_spider(
                frame_view,
                encoder,
                blur_horizontal_pipeline,
                blur_vertical_pipeline,
            ),
            CameraPostEffect::Invert => self.render_invert(frame_view, encoder),
        }
        effect.draw_calls()
    }

    fn render_creeper(&self, frame_view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        draw_fullscreen(
            encoder,
            "bbb-camera-post-creeper-color-pass",
            &self.targets.temp.view,
            &self.pipelines.creeper_color,
            &self.targets.creeper_color,
        );
        draw_fullscreen(
            encoder,
            "bbb-camera-post-creeper-bits-pass",
            frame_view,
            &self.pipelines.creeper_bits,
            &self.targets.creeper_bits,
        );
    }

    fn render_invert(&self, frame_view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        draw_fullscreen(
            encoder,
            "bbb-camera-post-invert-pass",
            &self.targets.temp.view,
            &self.pipelines.invert,
            &self.targets.invert,
        );
        draw_fullscreen(
            encoder,
            "bbb-camera-post-invert-blit-pass",
            frame_view,
            &self.pipelines.blit,
            &self.targets.identity_blit,
        );
    }

    fn render_spider(
        &self,
        frame_view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        blur_horizontal_pipeline: &wgpu::RenderPipeline,
        blur_vertical_pipeline: &wgpu::RenderPipeline,
    ) {
        draw_fullscreen(
            encoder,
            "bbb-camera-post-spider-large-blur-horizontal-pass",
            &self.targets.temp.view,
            blur_horizontal_pipeline,
            &self.targets.large_blur_horizontal,
        );
        draw_fullscreen(
            encoder,
            "bbb-camera-post-spider-large-blur-vertical-pass",
            &self.targets.large_blur.view,
            blur_vertical_pipeline,
            &self.targets.large_blur_vertical,
        );
        draw_fullscreen(
            encoder,
            "bbb-camera-post-spider-small-blur-horizontal-pass",
            &self.targets.temp.view,
            blur_horizontal_pipeline,
            &self.targets.small_blur_horizontal,
        );
        draw_fullscreen(
            encoder,
            "bbb-camera-post-spider-small-blur-vertical-pass",
            &self.targets.small_blur.view,
            blur_vertical_pipeline,
            &self.targets.small_blur_vertical,
        );
        draw_fullscreen(
            encoder,
            "bbb-camera-post-spider-center-pass",
            &self.targets.temp.view,
            &self.pipelines.spider_clip,
            &self.targets.spider_passes[0],
        );
        draw_fullscreen(
            encoder,
            "bbb-camera-post-spider-left-pass",
            &self.targets.swap.view,
            &self.pipelines.spider_clip,
            &self.targets.spider_passes[1],
        );
        draw_fullscreen(
            encoder,
            "bbb-camera-post-spider-right-pass",
            &self.targets.temp.view,
            &self.pipelines.spider_clip,
            &self.targets.spider_passes[2],
        );
        draw_fullscreen(
            encoder,
            "bbb-camera-post-spider-lower-left-pass",
            &self.targets.swap.view,
            &self.pipelines.spider_clip,
            &self.targets.spider_passes[3],
        );
        draw_fullscreen(
            encoder,
            "bbb-camera-post-spider-lower-right-pass",
            &self.targets.temp.view,
            &self.pipelines.spider_clip,
            &self.targets.spider_passes[4],
        );
        draw_fullscreen(
            encoder,
            "bbb-camera-post-spider-blit-pass",
            frame_view,
            &self.pipelines.blit,
            &self.targets.spider_blit,
        );
    }
}

fn create_post_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-camera-post-bind-group-layout"),
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
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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

fn create_sampler(device: &wgpu::Device, label: &str, linear: bool) -> wgpu::Sampler {
    let filter = if linear {
        wgpu::FilterMode::Linear
    } else {
        wgpu::FilterMode::Nearest
    };
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some(label),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: filter,
        min_filter: filter,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    })
}

fn create_post_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layout: &wgpu::BindGroupLayout,
    label: &str,
    vertex_shader: &str,
    fragment_shader: &str,
) -> wgpu::RenderPipeline {
    let shader = format!("{POST_SHADER_BINDINGS}\n{vertex_shader}\n{fragment_shader}");
    RenderPipelineBuilder::new(device, label)
        .shader(label, &shader)
        .layout("bbb-camera-post-pipeline-layout", &[layout])
        .color_target(format, None)
        .build()
}

fn create_uniform_buffer(
    device: &wgpu::Device,
    label: &str,
    uniform: &PostUniform,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::bytes_of(uniform),
        usage: wgpu::BufferUsages::UNIFORM,
    })
}

fn create_blur_uniform_buffer(device: &wgpu::Device, label: &str, radius: f32) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(&[radius, 0.0_f32, 0.0, 0.0]),
        usage: wgpu::BufferUsages::UNIFORM,
    })
}

fn create_uniforms(device: &wgpu::Device) -> CameraPostEffectUniforms {
    CameraPostEffectUniforms {
        large_blur: create_blur_uniform_buffer(device, "bbb-camera-post-large-blur-uniform", 15.0),
        small_blur: create_blur_uniform_buffer(device, "bbb-camera-post-small-blur-uniform", 7.0),
        creeper_color: create_uniform_buffer(
            device,
            "bbb-camera-post-creeper-color-uniform",
            &CREEPER_COLOR_UNIFORM,
        ),
        creeper_bits: create_uniform_buffer(
            device,
            "bbb-camera-post-creeper-bits-uniform",
            &CREEPER_BITS_UNIFORM,
        ),
        invert: create_uniform_buffer(device, "bbb-camera-post-invert-uniform", &INVERT_UNIFORM),
        identity_blit: create_uniform_buffer(
            device,
            "bbb-camera-post-identity-blit-uniform",
            &IDENTITY_BLIT_UNIFORM,
        ),
        spider_passes: std::array::from_fn(|index| {
            create_uniform_buffer(
                device,
                "bbb-camera-post-spider-pass-uniform",
                &SPIDER_PASS_UNIFORMS[index],
            )
        }),
        spider_blit: create_uniform_buffer(
            device,
            "bbb-camera-post-spider-blit-uniform",
            &SPIDER_BLIT_UNIFORM,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn create_targets(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    screen_blur_layout: &wgpu::BindGroupLayout,
    post_layout: &wgpu::BindGroupLayout,
    nearest_sampler: &wgpu::Sampler,
    linear_sampler: &wgpu::Sampler,
    uniforms: &CameraPostEffectUniforms,
) -> CameraPostEffectTargets {
    let source = create_target(
        device,
        format,
        width,
        height,
        "bbb-camera-post-source",
        wgpu::TextureUsages::COPY_DST,
    );
    let temp = create_target(
        device,
        format,
        width,
        height,
        "bbb-camera-post-temp",
        wgpu::TextureUsages::empty(),
    );
    let swap = create_target(
        device,
        format,
        width,
        height,
        "bbb-camera-post-swap",
        wgpu::TextureUsages::empty(),
    );
    let large_blur = create_target(
        device,
        format,
        width,
        height,
        "bbb-camera-post-large-blur",
        wgpu::TextureUsages::empty(),
    );
    let small_blur = create_target(
        device,
        format,
        width,
        height,
        "bbb-camera-post-small-blur",
        wgpu::TextureUsages::empty(),
    );
    let large_blur_horizontal = create_blur_bind_group(
        device,
        screen_blur_layout,
        "bbb-camera-post-large-blur-horizontal-bind-group",
        &source.view,
        linear_sampler,
        &uniforms.large_blur,
    );
    let large_blur_vertical = create_blur_bind_group(
        device,
        screen_blur_layout,
        "bbb-camera-post-large-blur-vertical-bind-group",
        &temp.view,
        linear_sampler,
        &uniforms.large_blur,
    );
    let small_blur_horizontal = create_blur_bind_group(
        device,
        screen_blur_layout,
        "bbb-camera-post-small-blur-horizontal-bind-group",
        &source.view,
        linear_sampler,
        &uniforms.small_blur,
    );
    let small_blur_vertical = create_blur_bind_group(
        device,
        screen_blur_layout,
        "bbb-camera-post-small-blur-vertical-bind-group",
        &temp.view,
        linear_sampler,
        &uniforms.small_blur,
    );
    let creeper_color = create_post_bind_group(
        device,
        post_layout,
        "bbb-camera-post-creeper-color-bind-group",
        &source.view,
        &source.view,
        nearest_sampler,
        &uniforms.creeper_color,
    );
    let creeper_bits = create_post_bind_group(
        device,
        post_layout,
        "bbb-camera-post-creeper-bits-bind-group",
        &temp.view,
        &temp.view,
        nearest_sampler,
        &uniforms.creeper_bits,
    );
    let invert = create_post_bind_group(
        device,
        post_layout,
        "bbb-camera-post-invert-bind-group",
        &source.view,
        &source.view,
        nearest_sampler,
        &uniforms.invert,
    );
    let identity_blit = create_post_bind_group(
        device,
        post_layout,
        "bbb-camera-post-identity-blit-bind-group",
        &temp.view,
        &temp.view,
        nearest_sampler,
        &uniforms.identity_blit,
    );
    let spider_passes = [
        create_post_bind_group(
            device,
            post_layout,
            "bbb-camera-post-spider-center-bind-group",
            &source.view,
            &large_blur.view,
            nearest_sampler,
            &uniforms.spider_passes[0],
        ),
        create_post_bind_group(
            device,
            post_layout,
            "bbb-camera-post-spider-left-bind-group",
            &small_blur.view,
            &temp.view,
            nearest_sampler,
            &uniforms.spider_passes[1],
        ),
        create_post_bind_group(
            device,
            post_layout,
            "bbb-camera-post-spider-right-bind-group",
            &small_blur.view,
            &swap.view,
            nearest_sampler,
            &uniforms.spider_passes[2],
        ),
        create_post_bind_group(
            device,
            post_layout,
            "bbb-camera-post-spider-lower-left-bind-group",
            &small_blur.view,
            &temp.view,
            nearest_sampler,
            &uniforms.spider_passes[3],
        ),
        create_post_bind_group(
            device,
            post_layout,
            "bbb-camera-post-spider-lower-right-bind-group",
            &small_blur.view,
            &swap.view,
            nearest_sampler,
            &uniforms.spider_passes[4],
        ),
    ];
    let spider_blit = create_post_bind_group(
        device,
        post_layout,
        "bbb-camera-post-spider-blit-bind-group",
        &temp.view,
        &temp.view,
        nearest_sampler,
        &uniforms.spider_blit,
    );
    CameraPostEffectTargets {
        source,
        temp,
        swap,
        large_blur,
        small_blur,
        large_blur_horizontal,
        large_blur_vertical,
        small_blur_horizontal,
        small_blur_vertical,
        creeper_color,
        creeper_bits,
        invert,
        identity_blit,
        spider_passes,
        spider_blit,
    }
}

fn create_target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    label: &str,
    extra_usage: wgpu::TextureUsages,
) -> PostTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
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
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    PostTarget { texture, view }
}

fn create_blur_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    label: &str,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    uniform: &wgpu::Buffer,
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
                resource: uniform.as_entire_binding(),
            },
        ],
    })
}

fn create_post_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    label: &str,
    source_view: &wgpu::TextureView,
    blur_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    uniform: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(source_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(blur_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: uniform.as_entire_binding(),
            },
        ],
    })
}

fn draw_fullscreen(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    destination: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: destination,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind_group, &[]);
    pass.draw(0..3, 0..1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_post_effect_shaders_preserve_vanilla_formula_constants() {
        assert!(CREEPER_COLOR_FRAGMENT_SHADER.contains("chroma * 1.8"));
        assert!(CREEPER_BITS_FRAGMENT_SHADER.contains("fractional_texel"));
        assert!(CREEPER_BITS_FRAGMENT_SHADER.contains("* 1.5"));
        assert!(INVERT_FRAGMENT_SHADER.contains("mix(diffuse_color, inverse_color, config.p0.x)"));
        assert!(SPIDER_VERTEX_SHADER.contains("0.0174532925"));
        assert!(SPIDER_VERTEX_SHADER.contains("position.y * 0.5 + 0.5"));
        assert!(SPIDER_CLIP_FRAGMENT_SHADER.contains("1.0 - input.scaled_vanilla_uv.y"));
        assert!(SPIDER_CLIP_FRAGMENT_SHADER.contains("mix(blur_texel, output_texel, amount)"));
    }

    #[test]
    fn camera_post_effect_execution_uses_one_frame_copy_and_exact_pass_counts() {
        let source = include_str!("camera_post_effect.rs");
        let render = source
            .split("pub(super) fn render(")
            .nth(1)
            .and_then(|source| source.split_once("fn render_creeper("))
            .map(|(render, _)| render)
            .expect("camera post-effect render body");
        assert_eq!(render.matches("copy_texture_to_texture").count(), 1);
        assert!(!render.contains("create_texture"));
        assert!(!render.contains("create_buffer"));
        assert!(!render.contains("create_bind_group"));
        assert!(!render.contains("create_post_pipeline"));

        let creeper = source
            .split("fn render_creeper(")
            .nth(1)
            .and_then(|source| source.split_once("fn render_invert("))
            .map(|(body, _)| body)
            .expect("creeper graph");
        let invert = source
            .split("fn render_invert(")
            .nth(1)
            .and_then(|source| source.split_once("fn render_spider("))
            .map(|(body, _)| body)
            .expect("invert graph");
        let spider = source
            .split("fn render_spider(")
            .nth(1)
            .and_then(|source| source.split_once("fn create_post_bind_group_layout("))
            .map(|(body, _)| body)
            .expect("spider graph");
        assert_eq!(creeper.matches("draw_fullscreen(").count(), 2);
        assert_eq!(invert.matches("draw_fullscreen(").count(), 2);
        assert_eq!(spider.matches("draw_fullscreen(").count(), 10);
    }
}
