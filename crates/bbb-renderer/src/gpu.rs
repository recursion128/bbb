use std::mem;

use anyhow::{anyhow, bail, Result};
use bbb_render_types::{AtlasMipPatch, CoreShaderSourceProjection};
use wgpu::util::DeviceExt;

use crate::{
    camera::CameraUniform,
    pipeline_builder::{depth_stencil_state, RenderPipelineBuilder},
    terrain,
};

// Vanilla 26.1 allocates both the main target and every fabulous-transparency
// target as `TextureFormat.DEPTH32` (`RenderTarget.allocateDepthAttachment` and
// `MainTarget.allocateDepthAttachment`). Keep the sampled depth values feeding
// `transparency.fsh` at the same precision instead of accepting wgpu's
// implementation-defined 24-bit-or-greater alternative.
pub(super) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub(super) const DEPTH_TARGET_USAGE: wgpu::TextureUsages = wgpu::TextureUsages::RENDER_ATTACHMENT
    .union(wgpu::TextureUsages::TEXTURE_BINDING)
    .union(wgpu::TextureUsages::COPY_SRC)
    .union(wgpu::TextureUsages::COPY_DST);

const TERRAIN_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Unorm8x4, 2 => Float32x2, 3 => Sint16x2];
const TERRAIN_PIPELINE_OPAQUE_BLEND: Option<wgpu::BlendState> = Some(wgpu::BlendState::REPLACE);
const TERRAIN_PIPELINE_TRANSLUCENT_BLEND: Option<wgpu::BlendState> =
    Some(wgpu::BlendState::ALPHA_BLENDING);
const TERRAIN_PIPELINE_CULL_MODE: Option<wgpu::Face> = Some(wgpu::Face::Back);
const TERRAIN_PIPELINE_OPAQUE_DEPTH_WRITE_ENABLED: bool = true;
// Vanilla `RenderPipelines.TRANSLUCENT_TERRAIN` inherits
// `GENERIC_BLOCKS_SNIPPET` -> `DepthStencilState.DEFAULT`, whose writeDepth is
// true. Blending does not disable depth writes for chunk translucent terrain.
const TERRAIN_PIPELINE_TRANSLUCENT_DEPTH_WRITE_ENABLED: bool = true;
const TERRAIN_PIPELINE_DEPTH_COMPARE: wgpu::CompareFunction = wgpu::CompareFunction::LessEqual;
const CAMERA_BIND_GROUP_VISIBILITY: wgpu::ShaderStages = wgpu::ShaderStages::VERTEX_FRAGMENT;
const TERRAIN_SECTION_BIND_GROUP_VISIBILITY: wgpu::ShaderStages = wgpu::ShaderStages::FRAGMENT;
const TERRAIN_ALPHA_CUTOUT_PLACEHOLDER: &str = "    // __BBB_ALPHA_CUTOUT__";
const TERRAIN_SAMPLE_PLACEHOLDER: &str = "    // __BBB_TERRAIN_SAMPLE__";
const TERRAIN_CUTOUT_ALPHA_THRESHOLD: &str = "0.5";
const TERRAIN_TRANSLUCENT_ALPHA_THRESHOLD: &str = "0.01";
const TERRAIN_ATLAS_MAG_FILTER: wgpu::FilterMode = wgpu::FilterMode::Linear;
const TERRAIN_ATLAS_MIN_FILTER: wgpu::FilterMode = wgpu::FilterMode::Linear;
const TERRAIN_ATLAS_MIPMAP_FILTER: wgpu::FilterMode = wgpu::FilterMode::Linear;

const TERRAIN_SHADER: &str = r#"
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
var terrain_atlas: texture_2d<f32>;

@group(0) @binding(2)
var terrain_sampler: sampler;

@group(1) @binding(0)
var lightmap_texture: texture_2d<f32>;

@group(1) @binding(1)
var lightmap_sampler: sampler;

struct TerrainSection {
    visibility: vec4<f32>,
};

@group(2) @binding(0)
var<uniform> terrain_section: TerrainSection;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) light: vec2<i32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) vertex_color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) spherical_distance: f32,
    @location(3) cylindrical_distance: f32,
};

fn fog_spherical_distance(pos: vec3<f32>) -> f32 {
    return length(pos);
}

fn fog_cylindrical_distance(pos: vec3<f32>) -> f32 {
    return max(length(pos.xz), abs(pos.y));
}

fn linear_fog_value(vertex_distance: f32, fog_start: f32, fog_end: f32) -> f32 {
    if (vertex_distance <= fog_start) {
        return 0.0;
    }
    if (vertex_distance >= fog_end) {
        return 1.0;
    }
    return (vertex_distance - fog_start) / (fog_end - fog_start);
}

fn total_fog_value(spherical_distance: f32, cylindrical_distance: f32) -> f32 {
    return max(
        linear_fog_value(spherical_distance, camera.fog_distances.x, camera.fog_distances.y),
        linear_fog_value(cylindrical_distance, camera.fog_distances.z, camera.fog_distances.w),
    );
}

fn apply_fog(color: vec4<f32>, spherical_distance: f32, cylindrical_distance: f32) -> vec4<f32> {
    let fog_value = total_fog_value(spherical_distance, cylindrical_distance);
    return vec4<f32>(mix(color.rgb, camera.fog_color.rgb, fog_value * camera.fog_color.a), color.a);
}

fn sample_lightmap(light: vec2<i32>) -> vec3<f32> {
    let uv = clamp(
        vec2<f32>(light) / 256.0 + vec2<f32>(0.5 / 16.0),
        vec2<f32>(0.5 / 16.0),
        vec2<f32>(15.5 / 16.0),
    );
    return textureSampleLevel(lightmap_texture, lightmap_sampler, uv, 0.0).rgb;
}

fn sample_terrain_nearest_with_derivatives(
    uv: vec2<f32>,
    pixel_size: vec2<f32>,
    du: vec2<f32>,
    dv: vec2<f32>,
    texel_screen_size: vec2<f32>,
) -> vec4<f32> {
    let uv_texel_coords = uv / pixel_size;
    let texel_center = round(uv_texel_coords) - vec2<f32>(0.5);
    var texel_offset = uv_texel_coords - texel_center;
    texel_offset = (texel_offset - vec2<f32>(0.5)) * pixel_size / texel_screen_size
        + vec2<f32>(0.5);
    texel_offset = clamp(texel_offset, vec2<f32>(0.0), vec2<f32>(1.0));
    let sample_uv = (texel_center + texel_offset) * pixel_size;
    return textureSampleGrad(terrain_atlas, terrain_sampler, sample_uv, du, dv);
}

fn sample_terrain_nearest(uv: vec2<f32>, pixel_size: vec2<f32>) -> vec4<f32> {
    let du = dpdx(uv);
    let dv = dpdy(uv);
    let texel_screen_size = sqrt(du * du + dv * dv);
    return sample_terrain_nearest_with_derivatives(
        uv,
        pixel_size,
        du,
        dv,
        texel_screen_size,
    );
}

fn sample_terrain_rgss(uv: vec2<f32>, pixel_size: vec2<f32>) -> vec4<f32> {
    let du = dpdx(uv);
    let dv = dpdy(uv);
    let texel_screen_size = sqrt(du * du + dv * dv);
    let max_texel_size = max(texel_screen_size.x, texel_screen_size.y);
    let min_pixel_size = min(pixel_size.x, pixel_size.y);
    let blend_factor = smoothstep(min_pixel_size, min_pixel_size * 2.0, max_texel_size);

    let du_length = length(du);
    let dv_length = length(dv);
    let effective_derivative = sqrt(min(du_length, dv_length) * max(du_length, dv_length));
    let mip_level_exact = max(0.0, log2(effective_derivative / min_pixel_size));
    let mip_level_low = floor(mip_level_exact);
    let mip_level_high = mip_level_low + 1.0;
    let mip_blend = fract(mip_level_exact);

    // Naga 0.19 requires constant indexing for function-local value arrays, so
    // keep Vanilla's four rotated-grid offsets explicit rather than looping.
    let sample_uv_0 = uv + vec2<f32>(0.125, 0.375) * pixel_size;
    let sample_uv_1 = uv + vec2<f32>(-0.125, -0.375) * pixel_size;
    let sample_uv_2 = uv + vec2<f32>(0.375, -0.125) * pixel_size;
    let sample_uv_3 = uv + vec2<f32>(-0.375, 0.125) * pixel_size;
    let rgss_color_low = (
        textureSampleLevel(terrain_atlas, terrain_sampler, sample_uv_0, mip_level_low)
        + textureSampleLevel(terrain_atlas, terrain_sampler, sample_uv_1, mip_level_low)
        + textureSampleLevel(terrain_atlas, terrain_sampler, sample_uv_2, mip_level_low)
        + textureSampleLevel(terrain_atlas, terrain_sampler, sample_uv_3, mip_level_low)
    ) * 0.25;
    let rgss_color_high = (
        textureSampleLevel(terrain_atlas, terrain_sampler, sample_uv_0, mip_level_high)
        + textureSampleLevel(terrain_atlas, terrain_sampler, sample_uv_1, mip_level_high)
        + textureSampleLevel(terrain_atlas, terrain_sampler, sample_uv_2, mip_level_high)
        + textureSampleLevel(terrain_atlas, terrain_sampler, sample_uv_3, mip_level_high)
    ) * 0.25;
    let rgss_color = mix(rgss_color_low, rgss_color_high, mip_blend);
    let nearest_color = sample_terrain_nearest_with_derivatives(
        uv,
        pixel_size,
        du,
        dv,
        texel_screen_size,
    );
    return mix(nearest_color, rgss_color, blend_factor);
}

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.vertex_color = input.color * vec4<f32>(sample_lightmap(input.light), 1.0);
    out.uv = input.uv;
    let fog_pos = input.position - camera.camera_position.xyz;
    out.spherical_distance = fog_spherical_distance(fog_pos);
    out.cylindrical_distance = fog_cylindrical_distance(fog_pos);
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let atlas_size = vec2<f32>(textureDimensions(terrain_atlas));
    // __BBB_TERRAIN_SAMPLE__
    // __BBB_ALPHA_CUTOUT__
    let color = mix(
        vec4<f32>(camera.fog_color.rgb, texel.a * input.vertex_color.a),
        texel * input.vertex_color,
        terrain_section.visibility.x,
    );
    return apply_fog(color, input.spherical_distance, input.cylindrical_distance);
}
"#;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct TerrainGpuVertex {
    position: [f32; 3],
    color: [u8; 4],
    uv: [f32; 2],
    light: [i16; 2],
}

impl From<terrain::TerrainVertex> for TerrainGpuVertex {
    fn from(vertex: terrain::TerrainVertex) -> Self {
        let ambient = float_color_channel(vertex.ambient_occlusion);
        let shaded = (f32::from(ambient) * vertex.shade.clamp(0.0, 1.0)).floor() as u8;
        let color = std::array::from_fn(|channel| {
            if channel == 3 {
                255
            } else {
                let tint = (vertex.tint[channel].clamp(0.0, 1.0) * 255.0).round() as u16;
                (u16::from(shaded) * tint / 255) as u8
            }
        });
        let light = vertex.light.map(|value| {
            let level = (value.clamp(0.0, 1.0) * 15.0).round() as i16;
            level * 16
        });
        Self {
            position: vertex.position,
            color,
            uv: vertex.uv,
            light,
        }
    }
}

fn float_color_channel(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).floor() as u8
}

pub(super) struct DepthTarget {
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
}

pub(super) struct TerrainAtlasGpu {
    pub(super) texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) mip_level_count: u32,
}

impl TerrainAtlasGpu {
    pub(crate) fn view_and_sampler(&self) -> (&wgpu::TextureView, &wgpu::Sampler) {
        (&self.view, &self.sampler)
    }
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
        usage: DEPTH_TARGET_USAGE,
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
                visibility: CAMERA_BIND_GROUP_VISIBILITY,
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

pub(super) fn create_terrain_section_bind_group_layout(
    device: &wgpu::Device,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-terrain-section-bind-group-layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: TERRAIN_SECTION_BIND_GROUP_VISIBILITY,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

pub(super) fn create_terrain_section_uniform(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    visibility: f32,
) -> (wgpu::Buffer, wgpu::BindGroup) {
    let value = [visibility.clamp(0.0, 1.0), 0.0, 0.0, 0.0];
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-terrain-section-uniform"),
        contents: bytemuck::cast_slice(&value),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-terrain-section-bind-group"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    });
    (buffer, bind_group)
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
    create_terrain_atlas_mips_gpu(device, queue, width, height, &[rgba], 1)
}

pub(super) fn create_terrain_atlas_mips_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    mip_rgba: &[&[u8]],
    max_anisotropy: u16,
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
        // Vanilla 26.1 `TextureAtlas` uses `TextureFormat.RGBA8`, mapped by
        // `GlConst.toGlInternalId` to `GL_RGBA8`, not an sRGB texture.
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::RENDER_ATTACHMENT,
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
        // Vanilla 26.1's chunk-layer sampler is LINEAR/LINEAR; its terrain
        // shader implements both nearest-looking and RGSS sampling explicitly.
        mag_filter: TERRAIN_ATLAS_MAG_FILTER,
        min_filter: TERRAIN_ATLAS_MIN_FILTER,
        mipmap_filter: TERRAIN_ATLAS_MIPMAP_FILTER,
        // Vanilla passes `effectiveMaxAnisotropy()` only under
        // `TextureFilteringMethod.ANISOTROPIC`, else 1 (`LevelRenderer`
        // chunk-layer sampler creation).
        anisotropy_clamp: max_anisotropy,
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

pub(super) fn write_terrain_atlas_mip_patches_gpu(
    queue: &wgpu::Queue,
    atlas: &TerrainAtlasGpu,
    patches: &[AtlasMipPatch],
) -> Result<()> {
    write_texture_atlas_mip_patches_gpu(
        queue,
        &atlas.texture,
        atlas.width,
        atlas.height,
        atlas.mip_level_count,
        patches,
    )
}

pub(super) fn write_texture_atlas_mip_patches_gpu(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
    mip_level_count: u32,
    patches: &[AtlasMipPatch],
) -> Result<()> {
    for patch in patches {
        validate_atlas_mip_patch(width, height, mip_level_count, patch)?;
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: patch.level,
                origin: wgpu::Origin3d {
                    x: patch.rect.x,
                    y: patch.rect.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &patch.rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(patch.rect.width * 4),
                rows_per_image: Some(patch.rect.height),
            },
            wgpu::Extent3d {
                width: patch.rect.width,
                height: patch.rect.height,
                depth_or_array_layers: 1,
            },
        );
    }
    Ok(())
}

fn validate_atlas_mip_patch(
    width: u32,
    height: u32,
    mip_level_count: u32,
    patch: &AtlasMipPatch,
) -> Result<()> {
    if patch.level >= mip_level_count {
        bail!(
            "atlas patch mip level {} exceeds level count {}",
            patch.level,
            mip_level_count
        );
    }
    let mip_width = width.checked_shr(patch.level).unwrap_or(0);
    let mip_height = height.checked_shr(patch.level).unwrap_or(0);
    let right = patch
        .rect
        .x
        .checked_add(patch.rect.width)
        .ok_or_else(|| anyhow!("atlas patch width overflow"))?;
    let bottom = patch
        .rect
        .y
        .checked_add(patch.rect.height)
        .ok_or_else(|| anyhow!("atlas patch height overflow"))?;
    if right > mip_width || bottom > mip_height {
        bail!(
            "atlas patch {}x{} at {},{} exceeds mip {} dimensions {}x{}",
            patch.rect.width,
            patch.rect.height,
            patch.rect.x,
            patch.rect.y,
            patch.level,
            mip_width,
            mip_height
        );
    }
    validate_terrain_atlas_rgba(patch.rect.width, patch.rect.height, &patch.rgba)
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

pub(super) fn create_terrain_solid_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    section_bind_group_layout: &wgpu::BindGroupLayout,
    texture_filtering: crate::renderer::TerrainTextureFiltering,
) -> wgpu::RenderPipeline {
    create_terrain_pipeline_with_options(
        device,
        format,
        camera_bind_group_layout,
        lightmap_bind_group_layout,
        section_bind_group_layout,
        "bbb-terrain-pipeline",
        TERRAIN_PIPELINE_OPAQUE_DEPTH_WRITE_ENABLED,
        TERRAIN_PIPELINE_OPAQUE_BLEND,
        None,
        texture_filtering,
    )
}

pub(super) fn create_terrain_cutout_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    section_bind_group_layout: &wgpu::BindGroupLayout,
    texture_filtering: crate::renderer::TerrainTextureFiltering,
) -> wgpu::RenderPipeline {
    create_terrain_pipeline_with_options(
        device,
        format,
        camera_bind_group_layout,
        lightmap_bind_group_layout,
        section_bind_group_layout,
        "bbb-terrain-cutout-pipeline",
        TERRAIN_PIPELINE_OPAQUE_DEPTH_WRITE_ENABLED,
        TERRAIN_PIPELINE_OPAQUE_BLEND,
        Some(TERRAIN_CUTOUT_ALPHA_THRESHOLD),
        texture_filtering,
    )
}

pub(super) fn create_terrain_translucent_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    section_bind_group_layout: &wgpu::BindGroupLayout,
    texture_filtering: crate::renderer::TerrainTextureFiltering,
) -> wgpu::RenderPipeline {
    create_terrain_pipeline_with_options(
        device,
        format,
        camera_bind_group_layout,
        lightmap_bind_group_layout,
        section_bind_group_layout,
        "bbb-terrain-translucent-pipeline",
        TERRAIN_PIPELINE_TRANSLUCENT_DEPTH_WRITE_ENABLED,
        TERRAIN_PIPELINE_TRANSLUCENT_BLEND,
        Some(TERRAIN_TRANSLUCENT_ALPHA_THRESHOLD),
        texture_filtering,
    )
}

fn create_terrain_pipeline_with_options(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    section_bind_group_layout: &wgpu::BindGroupLayout,
    label: &'static str,
    depth_write_enabled: bool,
    blend: Option<wgpu::BlendState>,
    alpha_cutout: Option<&str>,
    texture_filtering: crate::renderer::TerrainTextureFiltering,
) -> wgpu::RenderPipeline {
    let shader = terrain_shader(alpha_cutout, texture_filtering);
    RenderPipelineBuilder::new(device, label)
        .shader("bbb-terrain-shader", &shader)
        .layout(
            "bbb-terrain-pipeline-layout",
            &[
                camera_bind_group_layout,
                lightmap_bind_group_layout,
                section_bind_group_layout,
            ],
        )
        .vertex_buffers(&[terrain_vertex_layout()])
        .color_target(format, blend)
        .cull_mode(TERRAIN_PIPELINE_CULL_MODE)
        .depth_stencil(depth_stencil_state(
            DEPTH_FORMAT,
            depth_write_enabled,
            TERRAIN_PIPELINE_DEPTH_COMPARE,
        ))
        .build()
}

pub(super) struct TerrainCorePipelines {
    pub(super) solid: wgpu::RenderPipeline,
    pub(super) cutout: wgpu::RenderPipeline,
    pub(super) translucent: wgpu::RenderPipeline,
}

pub(super) fn create_terrain_core_pipelines(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    section_bind_group_layout: &wgpu::BindGroupLayout,
    resources: &CoreShaderSourceProjection,
    texture_filtering: crate::renderer::TerrainTextureFiltering,
    allow_embedded_fallback: bool,
) -> Result<TerrainCorePipelines> {
    let adapted = crate::core_shaders::adapt_terrain_shaders(
        resources,
        texture_filtering == crate::renderer::TerrainTextureFiltering::Rgss,
    )?;
    let Some(adapted) = adapted else {
        if !allow_embedded_fallback {
            bail!("prepared core shaders are missing minecraft:core/terrain");
        }
        return Ok(TerrainCorePipelines {
            solid: create_terrain_solid_pipeline(
                device,
                format,
                camera_bind_group_layout,
                lightmap_bind_group_layout,
                section_bind_group_layout,
                texture_filtering,
            ),
            cutout: create_terrain_cutout_pipeline(
                device,
                format,
                camera_bind_group_layout,
                lightmap_bind_group_layout,
                section_bind_group_layout,
                texture_filtering,
            ),
            translucent: create_terrain_translucent_pipeline(
                device,
                format,
                camera_bind_group_layout,
                lightmap_bind_group_layout,
                section_bind_group_layout,
                texture_filtering,
            ),
        });
    };
    Ok(TerrainCorePipelines {
        solid: create_selected_terrain_pipeline(
            device,
            format,
            camera_bind_group_layout,
            lightmap_bind_group_layout,
            section_bind_group_layout,
            "bbb-selected-terrain-pipeline",
            &adapted,
            TERRAIN_PIPELINE_OPAQUE_DEPTH_WRITE_ENABLED,
            TERRAIN_PIPELINE_OPAQUE_BLEND,
            None,
        )?,
        cutout: create_selected_terrain_pipeline(
            device,
            format,
            camera_bind_group_layout,
            lightmap_bind_group_layout,
            section_bind_group_layout,
            "bbb-selected-terrain-cutout-pipeline",
            &adapted,
            TERRAIN_PIPELINE_OPAQUE_DEPTH_WRITE_ENABLED,
            TERRAIN_PIPELINE_OPAQUE_BLEND,
            Some(TERRAIN_CUTOUT_ALPHA_THRESHOLD),
        )?,
        translucent: create_selected_terrain_pipeline(
            device,
            format,
            camera_bind_group_layout,
            lightmap_bind_group_layout,
            section_bind_group_layout,
            "bbb-selected-terrain-translucent-pipeline",
            &adapted,
            TERRAIN_PIPELINE_TRANSLUCENT_DEPTH_WRITE_ENABLED,
            TERRAIN_PIPELINE_TRANSLUCENT_BLEND,
            Some(TERRAIN_TRANSLUCENT_ALPHA_THRESHOLD),
        )?,
    })
}

#[allow(clippy::too_many_arguments)]
fn create_selected_terrain_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    section_bind_group_layout: &wgpu::BindGroupLayout,
    label: &'static str,
    shaders: &crate::core_shaders::AdaptedTerrainShaders,
    depth_write_enabled: bool,
    blend: Option<wgpu::BlendState>,
    alpha_cutout: Option<&str>,
) -> Result<wgpu::RenderPipeline> {
    let defines = alpha_cutout.map_or_else(Vec::new, |value| vec![("ALPHA_CUTOUT", Some(value))]);
    let vertex_source = crate::core_shaders::shader_with_defines(&shaders.vertex, &defines);
    let fragment_source = crate::core_shaders::shader_with_defines(&shaders.fragment, &defines);
    let (vertex, fragment) = crate::core_shaders::compile_selected_shader_stages(
        device,
        label,
        &vertex_source,
        &fragment_source,
    )?;
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-selected-terrain-pipeline-layout"),
        bind_group_layouts: &[
            camera_bind_group_layout,
            lightmap_bind_group_layout,
            section_bind_group_layout,
        ],
        push_constant_ranges: &[],
    });
    Ok(
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &vertex,
                entry_point: "main",
                buffers: &[terrain_vertex_layout()],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: TERRAIN_PIPELINE_CULL_MODE,
                ..Default::default()
            },
            depth_stencil: Some(depth_stencil_state(
                DEPTH_FORMAT,
                depth_write_enabled,
                TERRAIN_PIPELINE_DEPTH_COMPARE,
            )),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &fragment,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        }),
    )
}

fn terrain_shader(
    alpha_cutout: Option<&str>,
    texture_filtering: crate::renderer::TerrainTextureFiltering,
) -> String {
    let alpha_cutout = alpha_cutout.map_or_else(String::new, |threshold| {
        format!("    if texel.a < {threshold} {{\n        discard;\n    }}")
    });
    let terrain_sample = match texture_filtering {
        // Vanilla `terrain.fsh` has no anisotropic branch: `UseRgss == 0`
        // takes `sampleNearest` and the anisotropy lives in the sampler.
        crate::renderer::TerrainTextureFiltering::None
        | crate::renderer::TerrainTextureFiltering::Anisotropic => {
            "    let texel = sample_terrain_nearest(input.uv, vec2<f32>(1.0) / atlas_size);"
        }
        crate::renderer::TerrainTextureFiltering::Rgss => {
            "    let texel = sample_terrain_rgss(input.uv, vec2<f32>(1.0) / atlas_size);"
        }
    };
    TERRAIN_SHADER
        .replace(TERRAIN_SAMPLE_PLACEHOLDER, terrain_sample)
        .replace(TERRAIN_ALPHA_CUTOUT_PLACEHOLDER, &alpha_cutout)
}

fn terrain_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<TerrainGpuVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &TERRAIN_VERTEX_ATTRIBUTES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_vertex_layout_matches_vanilla_block_format() {
        let layout = terrain_vertex_layout();

        assert_eq!(layout.array_stride, 28);
        assert_eq!(TERRAIN_VERTEX_ATTRIBUTES.len(), 4);
        assert_eq!(
            TERRAIN_VERTEX_ATTRIBUTES[1].format,
            wgpu::VertexFormat::Unorm8x4
        );
        assert_eq!(
            TERRAIN_VERTEX_ATTRIBUTES[3].format,
            wgpu::VertexFormat::Sint16x2
        );
    }

    #[test]
    fn terrain_gpu_vertex_packs_vanilla_color_and_light_coordinates() {
        let packed = TerrainGpuVertex::from(terrain::TerrainVertex {
            position: [1.0, 2.0, 3.0],
            normal: [0.0, 1.0, 0.0],
            uv: [0.25, 0.75],
            light: [2.0 / 15.0, 13.0 / 15.0],
            tint: [128.0 / 255.0, 1.0, 64.0 / 255.0],
            shade: 0.8,
            ambient_occlusion: 0.75,
            block_state_id: 42,
        });

        // ARGB.gray(AO), cardinal scale, then tint multiplication all truncate.
        assert_eq!(packed.color, [76, 152, 38, 255]);
        assert_eq!(packed.light, [32, 208]);
        assert_eq!(std::mem::size_of::<TerrainGpuVertex>(), 28);
    }

    #[test]
    fn depth_targets_are_bindable_for_vanilla_transparency_depth_inputs() {
        assert_eq!(
            DEPTH_FORMAT,
            wgpu::TextureFormat::Depth32Float,
            "Vanilla 26.1 RenderTarget and MainTarget allocate DEPTH32 attachments"
        );
        assert!(DEPTH_TARGET_USAGE.contains(wgpu::TextureUsages::RENDER_ATTACHMENT));
        assert!(DEPTH_TARGET_USAGE.contains(wgpu::TextureUsages::TEXTURE_BINDING));
        assert!(DEPTH_TARGET_USAGE.contains(wgpu::TextureUsages::COPY_SRC));
        assert!(DEPTH_TARGET_USAGE.contains(wgpu::TextureUsages::COPY_DST));
        assert_eq!(
            DEPTH_FORMAT.sample_type(None, None),
            Some(wgpu::TextureSampleType::Depth),
            "transparency post-chain samples MainDepth and sorting-target depth textures"
        );
    }

    #[test]
    fn terrain_pipeline_state_matches_vanilla_terrain_render_pipelines() {
        assert_eq!(TERRAIN_PIPELINE_CULL_MODE, Some(wgpu::Face::Back));
        assert!(TERRAIN_PIPELINE_OPAQUE_DEPTH_WRITE_ENABLED);
        assert!(TERRAIN_PIPELINE_TRANSLUCENT_DEPTH_WRITE_ENABLED);
        assert_eq!(
            TERRAIN_PIPELINE_DEPTH_COMPARE,
            wgpu::CompareFunction::LessEqual
        );
        assert_eq!(
            TERRAIN_PIPELINE_OPAQUE_BLEND,
            Some(wgpu::BlendState::REPLACE)
        );

        let translucent_blend = TERRAIN_PIPELINE_TRANSLUCENT_BLEND
            .expect("translucent terrain uses BlendFunction.TRANSLUCENT");
        assert_eq!(
            translucent_blend.color.src_factor,
            wgpu::BlendFactor::SrcAlpha
        );
        assert_eq!(
            translucent_blend.color.dst_factor,
            wgpu::BlendFactor::OneMinusSrcAlpha
        );
        assert_eq!(translucent_blend.alpha.src_factor, wgpu::BlendFactor::One);
        assert_eq!(
            translucent_blend.alpha.dst_factor,
            wgpu::BlendFactor::OneMinusSrcAlpha
        );
    }

    #[test]
    fn terrain_shaders_match_vanilla_layer_alpha_cutoffs() {
        let solid = terrain_shader(None, crate::renderer::TerrainTextureFiltering::None);
        let cutout = terrain_shader(
            Some(TERRAIN_CUTOUT_ALPHA_THRESHOLD),
            crate::renderer::TerrainTextureFiltering::None,
        );
        let translucent = terrain_shader(
            Some(TERRAIN_TRANSLUCENT_ALPHA_THRESHOLD),
            crate::renderer::TerrainTextureFiltering::None,
        );

        assert!(!solid.contains("discard"));
        assert!(cutout.contains("if texel.a < 0.5"));
        assert!(translucent.contains("if texel.a < 0.01"));
        assert!(!cutout.contains("texel.a < 0.01"));
    }

    #[test]
    fn terrain_shader_consumes_prepacked_vanilla_vertex_color() {
        assert!(TERRAIN_SHADER.contains("@location(1) color: vec4<f32>"));
        assert!(TERRAIN_SHADER.contains("texel * input.vertex_color,"));
        assert!(TERRAIN_SHADER.contains("terrain_section.visibility.x"));
        assert!(!TERRAIN_SHADER.contains("ambient_occlusion"));
        assert!(!TERRAIN_SHADER.contains("input.shade"));
        assert!(!TERRAIN_SHADER.contains("light_dir"));
    }

    #[test]
    fn anisotropic_filtering_keeps_the_nearest_shader_and_scales_the_sampler() {
        // Vanilla `terrain.fsh` has no anisotropic branch (`UseRgss == 0`
        // takes `sampleNearest`); the mode only changes the sampler.
        let anisotropic =
            terrain_shader(None, crate::renderer::TerrainTextureFiltering::Anisotropic);
        assert!(anisotropic
            .contains("let texel = sample_terrain_nearest(input.uv, vec2<f32>(1.0) / atlas_size)"));
        assert!(!anisotropic.contains("let texel = sample_terrain_rgss"));

        // Vanilla `effectiveMaxAnisotropy()` = `min(1 << bit, device max)`,
        // consumed only under `ANISOTROPIC` (else the sampler gets 1).
        use crate::renderer::{effective_terrain_max_anisotropy, TerrainTextureFiltering};
        assert_eq!(
            effective_terrain_max_anisotropy(TerrainTextureFiltering::Anisotropic, 1),
            2
        );
        assert_eq!(
            effective_terrain_max_anisotropy(TerrainTextureFiltering::Anisotropic, 2),
            4
        );
        assert_eq!(
            effective_terrain_max_anisotropy(TerrainTextureFiltering::Anisotropic, 3),
            8
        );
        assert_eq!(
            effective_terrain_max_anisotropy(TerrainTextureFiltering::Anisotropic, 9),
            16
        );
        assert_eq!(
            effective_terrain_max_anisotropy(TerrainTextureFiltering::None, 3),
            1
        );
        assert_eq!(
            effective_terrain_max_anisotropy(TerrainTextureFiltering::Rgss, 3),
            1
        );
    }

    #[test]
    fn terrain_shader_matches_vanilla_none_and_rgss_sampling_paths() {
        let nearest = terrain_shader(None, crate::renderer::TerrainTextureFiltering::None);
        let rgss = terrain_shader(None, crate::renderer::TerrainTextureFiltering::Rgss);

        assert!(nearest
            .contains("let texel = sample_terrain_nearest(input.uv, vec2<f32>(1.0) / atlas_size)"));
        assert!(
            rgss.contains("let texel = sample_terrain_rgss(input.uv, vec2<f32>(1.0) / atlas_size)")
        );
        for offset in [
            "vec2<f32>(0.125, 0.375)",
            "vec2<f32>(-0.125, -0.375)",
            "vec2<f32>(0.375, -0.125)",
            "vec2<f32>(-0.375, 0.125)",
        ] {
            assert!(rgss.contains(offset));
        }
        assert!(rgss.contains("textureSampleLevel("));
        assert!(rgss.contains("textureSampleGrad("));
        assert!(rgss.contains("smoothstep(min_pixel_size, min_pixel_size * 2.0"));
        assert_eq!(TERRAIN_ATLAS_MAG_FILTER, wgpu::FilterMode::Linear);
        assert_eq!(TERRAIN_ATLAS_MIN_FILTER, wgpu::FilterMode::Linear);
        assert_eq!(TERRAIN_ATLAS_MIPMAP_FILTER, wgpu::FilterMode::Linear);
    }

    #[test]
    fn terrain_shader_samples_dynamic_lightmap_texture() {
        assert!(TERRAIN_SHADER.contains("@group(1) @binding(0)"));
        assert!(TERRAIN_SHADER.contains("var lightmap_texture: texture_2d<f32>"));
        assert!(TERRAIN_SHADER.contains("@group(1) @binding(1)"));
        assert!(TERRAIN_SHADER.contains("var lightmap_sampler: sampler"));
        assert!(TERRAIN_SHADER.contains("fn sample_lightmap(light: vec2<i32>) -> vec3<f32>"));
        assert!(TERRAIN_SHADER.contains("vec2<f32>(light) / 256.0 + vec2<f32>(0.5 / 16.0)"));
        assert!(TERRAIN_SHADER.contains("vec2<f32>(15.5 / 16.0)"));
        assert!(TERRAIN_SHADER
            .contains("textureSampleLevel(lightmap_texture, lightmap_sampler, uv, 0.0).rgb"));
        assert!(TERRAIN_SHADER.contains(
            "out.vertex_color = input.color * vec4<f32>(sample_lightmap(input.light), 1.0)"
        ));
        assert_eq!(
            TERRAIN_SHADER
                .matches("sample_lightmap(input.light)")
                .count(),
            1
        );
        assert!(!TERRAIN_SHADER.contains("fn lightmap_brightness"));
        assert!(!TERRAIN_SHADER.contains("camera.lightmap_factors.y"));
    }

    #[test]
    fn shared_camera_bind_group_is_visible_to_entity_fragments() {
        assert_eq!(
            CAMERA_BIND_GROUP_VISIBILITY,
            wgpu::ShaderStages::VERTEX_FRAGMENT
        );
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

    #[test]
    fn terrain_atlas_mip_patch_validation_rejects_wrong_level_bounds_and_bytes() {
        let valid = AtlasMipPatch {
            level: 1,
            rect: bbb_render_types::AtlasRect {
                x: 2,
                y: 3,
                width: 4,
                height: 2,
            },
            rgba: vec![0; 4 * 2 * 4],
        };
        validate_atlas_mip_patch(16, 16, 3, &valid).unwrap();

        let mut invalid = valid.clone();
        invalid.level = 3;
        assert!(validate_atlas_mip_patch(16, 16, 3, &invalid)
            .unwrap_err()
            .to_string()
            .contains("exceeds level count"));
        invalid = valid.clone();
        invalid.rect.x = 5;
        assert!(validate_atlas_mip_patch(16, 16, 3, &invalid)
            .unwrap_err()
            .to_string()
            .contains("exceeds mip 1 dimensions"));
        invalid = valid;
        invalid.rgba.pop();
        assert!(validate_atlas_mip_patch(16, 16, 3, &invalid)
            .unwrap_err()
            .to_string()
            .contains("expected 32"));
    }
}
