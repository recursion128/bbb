use std::{collections::BTreeMap, mem};

use anyhow::{anyhow, bail, Result};

use crate::{gpu::DEPTH_FORMAT, particles::ParticleSpriteUv, particles::ParticleUvRect};

pub(crate) struct ParticleAtlasGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) sprite_uvs: BTreeMap<String, ParticleUvRect>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ParticleVertex {
    pub(crate) position: [f32; 3],
    pub(crate) uv: [f32; 2],
    pub(crate) color: [f32; 4],
}

const PARTICLE_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4];

const PARTICLE_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var particle_atlas: texture_2d<f32>;

@group(0) @binding(2)
var particle_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(particle_atlas, particle_sampler, input.uv) * input.color;
    if texel.a <= 0.01 {
        discard;
    }
    return texel;
}
"#;

pub(crate) fn create_particle_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-particle-shader"),
        source: wgpu::ShaderSource::Wgsl(PARTICLE_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-particle-pipeline-layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-particle-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[particle_vertex_layout()],
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
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

pub(crate) fn create_particle_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    width: u32,
    height: u32,
    rgba: &[u8],
    sprite_uvs: Vec<ParticleSpriteUv>,
) -> Result<ParticleAtlasGpu> {
    validate_particle_atlas_rgba(width, height, rgba)?;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-particle-atlas-texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bbb-particle-atlas-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-particle-atlas-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    Ok(ParticleAtlasGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        sprite_uvs: sprite_uvs
            .into_iter()
            .map(|sprite| (sprite.id, sprite.uv))
            .collect(),
    })
}

fn validate_particle_atlas_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<()> {
    if width == 0 || height == 0 {
        bail!("particle atlas dimensions must be non-zero");
    }
    let expected_len = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("particle atlas size overflow"))?;
    if rgba.len() != expected_len {
        bail!(
            "particle atlas has {} RGBA bytes, expected {} for {}x{}",
            rgba.len(),
            expected_len,
            width,
            height
        );
    }
    Ok(())
}

fn particle_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<ParticleVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &PARTICLE_VERTEX_ATTRIBUTES,
    }
}
