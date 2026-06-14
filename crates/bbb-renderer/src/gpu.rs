use std::mem;

use anyhow::{anyhow, bail, Result};
use wgpu::util::DeviceExt;

use crate::{camera::CameraUniform, terrain};

pub(super) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

const TERRAIN_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 8] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x2, 4 => Float32x3, 5 => Float32, 6 => Float32, 7 => Sint32];

const TERRAIN_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var terrain_atlas: texture_2d<f32>;

@group(0) @binding(2)
var terrain_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) light: vec2<f32>,
    @location(4) tint: vec3<f32>,
    @location(5) shade: f32,
    @location(6) ambient_occlusion: f32,
    @location(7) block_state_id: i32,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) light: vec2<f32>,
    @location(3) tint: vec3<f32>,
    @location(4) shade: f32,
    @location(5) ambient_occlusion: f32,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.normal = input.normal;
    out.uv = input.uv;
    out.light = input.light;
    out.tint = input.tint;
    out.shade = input.shade;
    out.ambient_occlusion = input.ambient_occlusion;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(terrain_atlas, terrain_sampler, input.uv);
    if texel.a <= 0.01 {
        discard;
    }
    let base = texel.rgb * input.tint;
    let block_light = input.light.x;
    let sky_light = input.light.y;
    let light_level = max(block_light, sky_light * 0.95);
    let shade = (0.16 + light_level * 0.84) * input.shade * input.ambient_occlusion;
    return vec4<f32>(base * shade, texel.a);
}
"#;

pub(super) struct DepthTarget {
    _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
}

pub(super) struct TerrainAtlasGpu {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    width: u32,
    height: u32,
    mip_level_count: u32,
}

pub(super) fn create_depth_target(device: &wgpu::Device, width: u32, height: u32) -> DepthTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-terrain-depth"),
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

pub(super) fn create_terrain_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-terrain-bind-group-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
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
        ],
    })
}

pub(super) fn create_camera_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-terrain-camera-buffer"),
        contents: bytemuck::bytes_of(&CameraUniform::identity()),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    })
}

pub(super) fn create_terrain_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    atlas: &TerrainAtlasGpu,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-terrain-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&atlas.view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&atlas.sampler),
            },
        ],
    })
}

pub(super) fn create_terrain_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<TerrainAtlasGpu> {
    create_terrain_atlas_mips_gpu(device, queue, width, height, &[rgba])
}

pub(super) fn create_terrain_atlas_mips_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    mip_rgba: &[&[u8]],
) -> Result<TerrainAtlasGpu> {
    let mip_dimensions = validate_terrain_atlas_mips(width, height, mip_rgba)?;
    let mip_level_count = mip_rgba.len() as u32;

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-terrain-texture-atlas"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, (rgba, (mip_width, mip_height))) in
        mip_rgba.iter().zip(mip_dimensions.iter()).enumerate()
    {
        write_texture_rgba(queue, &texture, level as u32, *mip_width, *mip_height, rgba);
    }
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bbb-terrain-texture-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: if mip_level_count > 1 {
            wgpu::FilterMode::Linear
        } else {
            wgpu::FilterMode::Nearest
        },
        ..Default::default()
    });
    Ok(TerrainAtlasGpu {
        texture,
        view,
        sampler,
        width,
        height,
        mip_level_count,
    })
}

pub(super) fn write_terrain_atlas_gpu(
    queue: &wgpu::Queue,
    atlas: &TerrainAtlasGpu,
    rgba: &[u8],
) -> Result<()> {
    if atlas.mip_level_count != 1 {
        bail!(
            "terrain texture atlas has {} mip levels; use mip-chain update",
            atlas.mip_level_count
        );
    }
    validate_terrain_atlas_rgba(atlas.width, atlas.height, rgba)?;
    write_texture_rgba(queue, &atlas.texture, 0, atlas.width, atlas.height, rgba);
    Ok(())
}

pub(super) fn write_terrain_atlas_mips_gpu(
    queue: &wgpu::Queue,
    atlas: &TerrainAtlasGpu,
    mip_rgba: &[&[u8]],
) -> Result<()> {
    if mip_rgba.len() as u32 != atlas.mip_level_count {
        bail!(
            "terrain texture atlas has {} mip levels, update supplied {}",
            atlas.mip_level_count,
            mip_rgba.len()
        );
    }
    let mip_dimensions = validate_terrain_atlas_mips(atlas.width, atlas.height, mip_rgba)?;
    for (level, (rgba, (mip_width, mip_height))) in
        mip_rgba.iter().zip(mip_dimensions.iter()).enumerate()
    {
        write_texture_rgba(
            queue,
            &atlas.texture,
            level as u32,
            *mip_width,
            *mip_height,
            rgba,
        );
    }
    Ok(())
}

fn validate_terrain_atlas_mips(
    width: u32,
    height: u32,
    mip_rgba: &[&[u8]],
) -> Result<Vec<(u32, u32)>> {
    if mip_rgba.is_empty() {
        bail!("terrain texture atlas must include at least one mip level");
    }
    let mut dimensions = Vec::with_capacity(mip_rgba.len());
    for (level, rgba) in mip_rgba.iter().enumerate() {
        let level =
            u32::try_from(level).map_err(|_| anyhow!("terrain atlas mip level overflow"))?;
        let mip_width = width.checked_shr(level).unwrap_or(0);
        let mip_height = height.checked_shr(level).unwrap_or(0);
        if mip_width == 0 || mip_height == 0 {
            bail!(
                "terrain texture atlas mip level {} has zero-sized dimensions from {}x{}",
                level,
                width,
                height
            );
        }
        validate_terrain_atlas_rgba(mip_width, mip_height, rgba)?;
        dimensions.push((mip_width, mip_height));
    }
    Ok(dimensions)
}

fn validate_terrain_atlas_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<()> {
    if width == 0 || height == 0 {
        bail!("terrain texture atlas dimensions must be non-zero");
    }
    let expected_len = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("terrain texture atlas size overflow"))?;
    if rgba.len() != expected_len {
        bail!(
            "terrain texture atlas has {} bytes, expected {} for {}x{} RGBA",
            rgba.len(),
            expected_len,
            width,
            height
        );
    }
    Ok(())
}

fn write_texture_rgba(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    mip_level: u32,
    width: u32,
    height: u32,
    rgba: &[u8],
) {
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture,
            mip_level,
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
}

pub(super) fn create_terrain_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_terrain_pipeline_with_options(
        device,
        format,
        camera_bind_group_layout,
        "bbb-terrain-pipeline",
        true,
        Some(wgpu::BlendState::REPLACE),
    )
}

pub(super) fn create_terrain_translucent_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_terrain_pipeline_with_options(
        device,
        format,
        camera_bind_group_layout,
        "bbb-terrain-translucent-pipeline",
        false,
        Some(wgpu::BlendState::ALPHA_BLENDING),
    )
}

fn create_terrain_pipeline_with_options(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    label: &'static str,
    depth_write_enabled: bool,
    blend: Option<wgpu::BlendState>,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-terrain-shader"),
        source: wgpu::ShaderSource::Wgsl(TERRAIN_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-terrain-pipeline-layout"),
        bind_group_layouts: &[camera_bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[terrain_vertex_layout()],
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
            depth_write_enabled,
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
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

fn terrain_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<terrain::TerrainVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &TERRAIN_VERTEX_ATTRIBUTES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_vertex_layout_exposes_ambient_occlusion_before_block_state_id() {
        let layout = terrain_vertex_layout();

        assert_eq!(
            layout.array_stride,
            mem::size_of::<terrain::TerrainVertex>() as wgpu::BufferAddress
        );
        assert_eq!(TERRAIN_VERTEX_ATTRIBUTES.len(), 8);
        assert_eq!(TERRAIN_VERTEX_ATTRIBUTES[6].shader_location, 6);
        assert_eq!(
            TERRAIN_VERTEX_ATTRIBUTES[6].format,
            wgpu::VertexFormat::Float32
        );
        assert_eq!(TERRAIN_VERTEX_ATTRIBUTES[7].shader_location, 7);
        assert_eq!(
            TERRAIN_VERTEX_ATTRIBUTES[7].format,
            wgpu::VertexFormat::Sint32
        );
    }

    #[test]
    fn terrain_shader_multiplies_shade_by_vertex_ambient_occlusion() {
        assert!(TERRAIN_SHADER.contains("@location(6) ambient_occlusion: f32"));
        assert!(TERRAIN_SHADER.contains("* input.shade * input.ambient_occlusion"));
        assert!(!TERRAIN_SHADER.contains("light_dir"));
    }

    #[test]
    fn terrain_atlas_mip_validation_tracks_shifted_dimensions() {
        let base = vec![0; 4 * 4 * 4];
        let mip1 = vec![0; 2 * 2 * 4];
        let mip2 = vec![0; 4];

        let dimensions =
            validate_terrain_atlas_mips(4, 4, &[base.as_slice(), mip1.as_slice(), mip2.as_slice()])
                .unwrap();

        assert_eq!(dimensions, vec![(4, 4), (2, 2), (1, 1)]);
    }

    #[test]
    fn terrain_atlas_mip_validation_rejects_missing_or_invalid_levels() {
        let empty = validate_terrain_atlas_mips(4, 4, &[]).unwrap_err();
        assert!(empty
            .to_string()
            .contains("must include at least one mip level"));

        let wrong_size = vec![0; 3];
        let err = validate_terrain_atlas_mips(4, 4, &[wrong_size.as_slice()]).unwrap_err();
        assert!(err.to_string().contains("expected 64 for 4x4 RGBA"));

        let base = vec![0; 4];
        let too_deep =
            validate_terrain_atlas_mips(1, 1, &[base.as_slice(), base.as_slice()]).unwrap_err();
        assert!(too_deep
            .to_string()
            .contains("mip level 1 has zero-sized dimensions"));
    }
}
