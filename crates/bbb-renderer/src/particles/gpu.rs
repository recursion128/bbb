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
    pub(crate) light: [f32; 2],
}

const PARTICLE_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4, 3 => Float32x2];

const PARTICLE_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    lightmap_factors: vec4<f32>,
    lightmap_effects: vec4<f32>,
    block_light_tint: vec4<f32>,
    sky_light_color: vec4<f32>,
    ambient_color: vec4<f32>,
    night_vision_color: vec4<f32>,
    camera_position: vec4<f32>,
    fog_color: vec4<f32>,
    fog_distances: vec4<f32>,
    fog_visibility_ends: vec4<f32>,
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
    @location(3) light: vec2<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) light: vec2<f32>,
    @location(3) spherical_distance: f32,
    @location(4) cylindrical_distance: f32,
};

fn linear_fog_value(vertex_distance: f32, fog_start: f32, fog_end: f32) -> f32 {
    if (vertex_distance <= fog_start) {
        return 0.0;
    }
    if (vertex_distance >= fog_end) {
        return 1.0;
    }
    return (vertex_distance - fog_start) / (fog_end - fog_start);
}

fn apply_fog(color: vec4<f32>, spherical_distance: f32, cylindrical_distance: f32) -> vec4<f32> {
    let fog_value = max(
        linear_fog_value(spherical_distance, camera.fog_distances.x, camera.fog_distances.y),
        linear_fog_value(cylindrical_distance, camera.fog_distances.z, camera.fog_distances.w),
    );
    return vec4<f32>(mix(color.rgb, camera.fog_color.rgb, fog_value * camera.fog_color.a), color.a);
}

fn lightmap_brightness(level: f32) -> f32 {
    return level / (4.0 - 3.0 * level);
}

fn parabolic_mix_factor(level: f32) -> f32 {
    let centered = 2.0 * level - 1.0;
    return centered * centered;
}

fn not_gamma(color: vec3<f32>) -> vec3<f32> {
    let max_component = max(max(color.x, color.y), color.z);
    if (max_component <= 0.0) {
        return color;
    }
    let max_inverted = 1.0 - max_component;
    let max_scaled = 1.0 - max_inverted * max_inverted * max_inverted * max_inverted;
    return color * (max_scaled / max_component);
}

fn apply_lightmap_brightness(color: vec3<f32>) -> vec3<f32> {
    let clamped = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
    let not_gamma_color = not_gamma(clamped);
    return mix(clamped, not_gamma_color, camera.lightmap_effects.y);
}

fn packed_lightmap_color(light: vec2<f32>) -> vec3<f32> {
    let block_brightness = lightmap_brightness(light.x) * camera.lightmap_factors.y;
    let sky_brightness = lightmap_brightness(light.y) * camera.lightmap_factors.x;
    let night_vision_color = camera.night_vision_color.rgb * camera.lightmap_factors.z;
    var color = max(camera.ambient_color.rgb, night_vision_color);
    color += camera.sky_light_color.rgb * sky_brightness;
    let block_light_color = mix(
        camera.block_light_tint.rgb,
        vec3<f32>(1.0),
        0.9 * parabolic_mix_factor(light.x),
    );
    color += block_light_color * block_brightness;
    color = mix(color, color * vec3<f32>(0.7, 0.6, 0.6), camera.lightmap_effects.x);
    color -= vec3<f32>(camera.lightmap_factors.w);
    return apply_lightmap_brightness(color);
}

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.color = input.color;
    out.light = input.light;
    let fog_pos = input.position - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(particle_atlas, particle_sampler, input.uv) * input.color;
    if texel.a <= 0.01 {
        discard;
    }
    let light_color = packed_lightmap_color(input.light);
    return apply_fog(vec4<f32>(texel.rgb * light_color, texel.a), input.spherical_distance, input.cylindrical_distance);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particle_vertex_layout_carries_vanilla_light_coords() {
        assert_eq!(PARTICLE_VERTEX_ATTRIBUTES.len(), 4);
        assert_eq!(PARTICLE_VERTEX_ATTRIBUTES[3].shader_location, 3);
        assert_eq!(
            PARTICLE_VERTEX_ATTRIBUTES[3].format,
            wgpu::VertexFormat::Float32x2
        );
        assert!(PARTICLE_SHADER.contains("@location(3) light: vec2<f32>"));
        assert!(PARTICLE_SHADER.contains("level / (4.0 - 3.0 * level)"));
        assert!(PARTICLE_SHADER.contains("lightmap_brightness(light.x)"));
        assert!(PARTICLE_SHADER.contains("lightmap_brightness(light.y)"));
        assert!(PARTICLE_SHADER.contains("camera.block_light_tint.rgb"));
        assert!(PARTICLE_SHADER.contains("texel.rgb * light_color"));
    }
}
