use std::mem;

use anyhow::{anyhow, bail, Result};
use bbb_render_types::{AtlasMipPatch, DynamicLightsMode};

use crate::clouds::CloudUniform;
use crate::render_core::{
    camera::CameraUniform,
    pipeline_builder::{depth_stencil_state, RenderPipelineBuilder},
    texture_sampling::{validate_rgba, write_texture_rgba},
};

// Vanilla 26.1 allocates both the main target and every fabulous-transparency
// target as `TextureFormat.DEPTH32` (`RenderTarget.allocateDepthAttachment` and
// `MainTarget.allocateDepthAttachment`). Keep the sampled depth values feeding
// `transparency.fsh` at the same precision instead of accepting wgpu's
// implementation-defined 24-bit-or-greater alternative.
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub const DEPTH_TARGET_USAGE: wgpu::TextureUsages = wgpu::TextureUsages::RENDER_ATTACHMENT
    .union(wgpu::TextureUsages::TEXTURE_BINDING)
    .union(wgpu::TextureUsages::COPY_SRC)
    .union(wgpu::TextureUsages::COPY_DST);

pub const TERRAIN_POS_CENTER_OFFSET: f32 = 8.0;
pub const TERRAIN_POS_SCALE: f32 = 2048.0;
pub const TERRAIN_UV_SCALE: f32 = 32768.0;

const TERRAIN_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Sint16x4, 1 => Unorm8x4, 2 => Uint16x2];
const TERRAIN_SECTION_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![3 => Sint32x3, 4 => Float32];
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
const TERRAIN_ALPHA_CUTOUT_PLACEHOLDER: &str = "    // __BBB_ALPHA_CUTOUT__";
const TERRAIN_SAMPLE_PLACEHOLDER: &str = "    // __BBB_TERRAIN_SAMPLE__";
const TERRAIN_VERTEX_OUT_DYNAMIC_LIGHTS_PLACEHOLDER: &str =
    "    // __BBB_VERTEX_OUT_DYNAMIC_LIGHTS__";
const TERRAIN_VS_DYNAMIC_LIGHTS_PLACEHOLDER: &str = "    // __BBB_VS_DYNAMIC_LIGHTS__";
const TERRAIN_FS_DYNAMIC_LIGHTS_PLACEHOLDER: &str = "    // __BBB_FS_DYNAMIC_LIGHTS__";
const TERRAIN_CUTOUT_ALPHA_THRESHOLD: &str = "0.5";
const TERRAIN_TRANSLUCENT_ALPHA_THRESHOLD: &str = "0.01";
const TERRAIN_ATLAS_MAG_FILTER: wgpu::FilterMode = wgpu::FilterMode::Linear;
const TERRAIN_ATLAS_MIN_FILTER: wgpu::FilterMode = wgpu::FilterMode::Linear;
const TERRAIN_ATLAS_MIPMAP_FILTER: wgpu::MipmapFilterMode = wgpu::MipmapFilterMode::Linear;

const TERRAIN_SHADER: &str = concat!(
    crate::wgsl::camera_full!(),
    crate::wgsl::linear_fog_value!(),
    r#"
"#,
    crate::wgsl::evaluate_dynamic_lights!(),
    r#"
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

struct VertexIn {
    @location(0) pos_light: vec4<i32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<u32>,
    @location(3) section_origin: vec3<i32>,
    @location(4) section_visibility: f32,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) vertex_color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) spherical_distance: f32,
    @location(3) cylindrical_distance: f32,
    @location(4) @interpolate(flat) section_visibility: f32,
    // __BBB_VERTEX_OUT_DYNAMIC_LIGHTS__
};

fn fog_spherical_distance(pos: vec3<f32>) -> f32 {
    return length(pos);
}

fn fog_cylindrical_distance(pos: vec3<f32>) -> f32 {
    return max(length(pos.xz), abs(pos.y));
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
    let safe_screen_size = max(texel_screen_size, vec2<f32>(1e-6));
    texel_offset = (texel_offset - vec2<f32>(0.5)) * pixel_size / safe_screen_size
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
    if blend_factor == 0.0 {
        return sample_terrain_nearest_with_derivatives(
            uv, pixel_size, du, dv, texel_screen_size,
        );
    }

    let du_length = length(du);
    let dv_length = length(dv);
    let effective_derivative = sqrt(min(du_length, dv_length) * max(du_length, dv_length));
    let mip_level_exact = max(0.0, log2(effective_derivative / min_pixel_size));

    let sample_uv_0 = uv + vec2<f32>(0.125, 0.375) * pixel_size;
    let sample_uv_1 = uv + vec2<f32>(-0.125, -0.375) * pixel_size;
    let sample_uv_2 = uv + vec2<f32>(0.375, -0.125) * pixel_size;
    let sample_uv_3 = uv + vec2<f32>(-0.375, 0.125) * pixel_size;
    // The linear mip sampler blends adjacent levels at each RGSS offset.
    let rgss_color = (
        textureSampleLevel(terrain_atlas, terrain_sampler, sample_uv_0, mip_level_exact)
        + textureSampleLevel(terrain_atlas, terrain_sampler, sample_uv_1, mip_level_exact)
        + textureSampleLevel(terrain_atlas, terrain_sampler, sample_uv_2, mip_level_exact)
        + textureSampleLevel(terrain_atlas, terrain_sampler, sample_uv_3, mip_level_exact)
    ) * 0.25;
    if blend_factor == 1.0 {
        return rgss_color;
    }
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
    let local_pos = vec3<f32>(input.pos_light.xyz) * (1.0 / 2048.0) + vec3<f32>(8.0);
    let cam_block = vec3<i32>(floor(camera.camera_position.xyz));
    let cam_fract = camera.camera_position.xyz - vec3<f32>(cam_block);
    let fog_pos = vec3<f32>(input.section_origin - cam_block) - cam_fract + local_pos;
    out.position = camera.projection * (camera.sky_model_view * vec4<f32>(fog_pos, 1.0));
    let light_bits = u32(input.pos_light.w) & 0xffffu;
    let light = vec2<i32>(i32(light_bits & 0xffu), i32((light_bits >> 8u) & 0xffu));
    out.vertex_color = input.color * vec4<f32>(sample_lightmap(light), 1.0);
    // __BBB_VS_DYNAMIC_LIGHTS__
    out.uv = vec2<f32>(input.uv) * (1.0 / 32768.0);
    out.spherical_distance = fog_spherical_distance(fog_pos);
    out.cylindrical_distance = fog_cylindrical_distance(fog_pos);
    out.section_visibility = input.section_visibility;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let atlas_size = vec2<f32>(textureDimensions(terrain_atlas));
    // __BBB_TERRAIN_SAMPLE__
    // __BBB_ALPHA_CUTOUT__
    var color = mix(
        vec4<f32>(camera.fog_color.rgb, texel.a * input.vertex_color.a),
        texel * input.vertex_color,
        input.section_visibility,
    );
    // __BBB_FS_DYNAMIC_LIGHTS__
    return apply_fog(color, input.spherical_distance, input.cylindrical_distance);
}
"#,
);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TerrainGpuVertex {
    pub pos_light: [i16; 4],
    pub color: [u8; 4],
    pub uv: [u16; 2],
}

const _: () = assert!(mem::size_of::<TerrainGpuVertex>() == 16);

impl TerrainGpuVertex {
    /// Vanilla `BlockModelLighter` vertex color: `ARGB.gray(ao)`, the cardinal
    /// shade and the tint each truncate; `light` is Vanilla packed light coordinates.
    pub fn pack(
        local_position: [f32; 3],
        uv: [f32; 2],
        light: u32,
        tint: crate::terrain::TerrainTint,
        shade: f32,
        ambient_occlusion: f32,
    ) -> Self {
        let ambient = (ambient_occlusion.clamp(0.0, 1.0) * 255.0).floor() as u8;
        let shaded = u16::from((f32::from(ambient) * shade.clamp(0.0, 1.0)).floor() as u8);
        let channel = |tint: u8| (shaded * u16::from(tint) / 255) as u8;
        let quantize_pos = |coord: f32| {
            ((coord - TERRAIN_POS_CENTER_OFFSET) * TERRAIN_POS_SCALE)
                .round()
                .clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16
        };
        let quantize_uv = |coord: f32| {
            (coord * TERRAIN_UV_SCALE)
                .round()
                .clamp(0.0, f32::from(u16::MAX)) as u16
        };
        let block_light = (light & 0xFFFF).min(255) as u8;
        let sky_light = ((light >> 16) & 0xFFFF).min(255) as u8;
        let packed_light = u16::from_le_bytes([block_light, sky_light]) as i16;
        Self {
            pos_light: [
                quantize_pos(local_position[0]),
                quantize_pos(local_position[1]),
                quantize_pos(local_position[2]),
                packed_light,
            ],
            color: [channel(tint.r), channel(tint.g), channel(tint.b), 255],
            uv: [quantize_uv(uv[0]), quantize_uv(uv[1])],
        }
    }

    #[inline]
    pub fn local_position(self) -> [f32; 3] {
        [
            f32::from(self.pos_light[0]) * (1.0 / TERRAIN_POS_SCALE) + TERRAIN_POS_CENTER_OFFSET,
            f32::from(self.pos_light[1]) * (1.0 / TERRAIN_POS_SCALE) + TERRAIN_POS_CENTER_OFFSET,
            f32::from(self.pos_light[2]) * (1.0 / TERRAIN_POS_SCALE) + TERRAIN_POS_CENTER_OFFSET,
        ]
    }

    #[inline]
    pub fn world_position(self, section_origin: [i32; 3]) -> [f32; 3] {
        let local = self.local_position();
        [
            local[0] + section_origin[0] as f32,
            local[1] + section_origin[1] as f32,
            local[2] + section_origin[2] as f32,
        ]
    }

    #[inline]
    pub fn uv_f32(self) -> [f32; 2] {
        [
            f32::from(self.uv[0]) * (1.0 / TERRAIN_UV_SCALE),
            f32::from(self.uv[1]) * (1.0 / TERRAIN_UV_SCALE),
        ]
    }

    #[inline]
    pub fn light_coords(self) -> [i16; 2] {
        let [block, sky] = (self.pos_light[3] as u16).to_le_bytes();
        [i16::from(block), i16::from(sky)]
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct TerrainSectionInstance {
    pub(crate) origin_blocks: [i32; 3],
    pub(crate) visibility: f32,
}

const _: () = assert!(mem::size_of::<TerrainSectionInstance>() == 16);

pub struct DepthTarget {
    pub _texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

pub struct TerrainAtlasGpu {
    pub texture: wgpu::Texture,
    view: wgpu::TextureView,
    pub(crate) mip_views: Vec<wgpu::TextureView>,
    sampler: wgpu::Sampler,
    pub width: u32,
    pub height: u32,
    pub mip_level_count: u32,
}

impl TerrainAtlasGpu {
    pub fn view_and_sampler(&self) -> (&wgpu::TextureView, &wgpu::Sampler) {
        (&self.view, &self.sampler)
    }
}

pub fn create_depth_target(device: &wgpu::Device, width: u32, height: u32) -> DepthTarget {
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

pub fn create_terrain_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
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

pub(crate) struct FrameCameraBuffers {
    buffer: wgpu::Buffer,
    stride: usize,
}

impl FrameCameraBuffers {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let alignment = device.limits().min_uniform_buffer_offset_alignment as usize;
        let stride = mem::size_of::<CameraUniform>().div_ceil(alignment) * alignment;
        let total_size = stride * 3 + mem::size_of::<CloudUniform>();
        let mut initial = vec![0u8; total_size];
        Self::copy_uniforms_slice(
            stride,
            &mut initial,
            &[CameraUniform::identity(); 3],
            Some(CloudUniform::default()),
        );
        Self {
            buffer: crate::frame_buffers::create_initialized_buffer(
                device,
                Some("bbb-frame-camera-buffer"),
                &initial,
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            ),
            stride,
        }
    }

    fn binding(&self, index: usize) -> wgpu::BufferBinding<'_> {
        wgpu::BufferBinding {
            buffer: &self.buffer,
            offset: (self.stride * index) as u64,
            size: wgpu::BufferSize::new(mem::size_of::<CameraUniform>() as u64),
        }
    }

    pub(crate) fn world(&self) -> wgpu::BufferBinding<'_> {
        self.binding(0)
    }

    pub(crate) fn hud3d(&self) -> wgpu::BufferBinding<'_> {
        self.binding(1)
    }

    pub(crate) fn gui(&self) -> wgpu::BufferBinding<'_> {
        self.binding(2)
    }

    pub(crate) fn cloud(&self) -> wgpu::BufferBinding<'_> {
        wgpu::BufferBinding {
            buffer: &self.buffer,
            offset: (self.stride * 3) as u64,
            size: wgpu::BufferSize::new(mem::size_of::<CloudUniform>() as u64),
        }
    }

    pub(crate) fn stage_cloud(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        staging: &mut crate::frame_buffers::UploadBelt,
        uniform: CloudUniform,
    ) {
        staging.write_bytes(
            encoder,
            &self.buffer,
            self.cloud().offset,
            bytemuck::bytes_of(&uniform),
        );
    }

    pub(crate) fn stage(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        staging: &mut crate::frame_buffers::UploadBelt,
        uniforms: [CameraUniform; 3],
        cloud: Option<CloudUniform>,
    ) {
        let bytes = if cloud.is_some() {
            self.buffer.size() as usize
        } else {
            self.stride * 2 + mem::size_of::<CameraUniform>()
        };
        let mut stack_buf = [0u8; 8192];
        if let Some(dst) = stack_buf.get_mut(..bytes) {
            Self::copy_uniforms_slice(self.stride, dst, &uniforms, cloud);
            staging.write_bytes(encoder, &self.buffer, 0, dst);
        } else {
            let mut heap_buf = vec![0u8; bytes];
            Self::copy_uniforms_slice(self.stride, &mut heap_buf, &uniforms, cloud);
            staging.write_bytes(encoder, &self.buffer, 0, &heap_buf);
        }
    }

    fn copy_uniforms_slice(
        stride: usize,
        destination: &mut [u8],
        uniforms: &[CameraUniform; 3],
        cloud: Option<CloudUniform>,
    ) {
        destination.fill(0);
        for (index, uniform) in uniforms.iter().enumerate() {
            let offset = stride * index;
            destination[offset..offset + mem::size_of::<CameraUniform>()]
                .copy_from_slice(bytemuck::bytes_of(uniform));
        }
        if let Some(cloud) = cloud {
            destination[stride * 3..stride * 3 + mem::size_of::<CloudUniform>()]
                .copy_from_slice(bytemuck::bytes_of(&cloud));
        }
    }
}

pub fn create_camera_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    crate::frame_buffers::create_initialized_buffer(
        device,
        Some("bbb-terrain-camera-buffer"),
        bytemuck::bytes_of(&CameraUniform::identity()),
        wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    )
}

pub fn create_terrain_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<TerrainAtlasGpu> {
    create_terrain_atlas_mips_gpu(device, queue, width, height, &[rgba], 1)
}

pub fn create_terrain_atlas_mips_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    mip_rgba: &[&[u8]],
    max_anisotropy: u16,
) -> Result<TerrainAtlasGpu> {
    let mip_dimensions = validate_terrain_atlas_mips(width, height, mip_rgba)?;
    let max_dim = device.limits().max_texture_dimension_2d;
    if width > max_dim || height > max_dim {
        bail!("terrain texture atlas dimensions {width}x{height} exceed device limit {max_dim}");
    }
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
    let mip_views = (0..mip_level_count)
        .map(|level| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("bbb-terrain-animation-atlas-mip"),
                base_mip_level: level,
                mip_level_count: Some(1),
                usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
                ..Default::default()
            })
        })
        .collect();
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
        mip_views,
        sampler,
        width,
        height,
        mip_level_count,
    })
}

pub fn write_terrain_atlas_gpu(
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
    validate_rgba(atlas.width, atlas.height, rgba)?;
    write_texture_rgba(queue, &atlas.texture, 0, atlas.width, atlas.height, rgba);
    Ok(())
}

pub fn write_terrain_atlas_mips_gpu(
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

pub fn write_terrain_atlas_mip_patches_gpu(
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

pub fn write_texture_atlas_mip_patches_gpu(
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
            wgpu::TexelCopyTextureInfo {
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
            wgpu::TexelCopyBufferLayout {
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
    validate_rgba(patch.rect.width, patch.rect.height, &patch.rgba)
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
        validate_rgba(mip_width, mip_height, rgba)?;
        dimensions.push((mip_width, mip_height));
    }
    Ok(dimensions)
}

#[allow(clippy::too_many_arguments)]
fn create_embedded_terrain_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    label: &'static str,
    depth_write_enabled: bool,
    blend: Option<wgpu::BlendState>,
    alpha_cutout: Option<&str>,
    texture_filtering: TerrainTextureFiltering,
    dynamic_lights_mode: DynamicLightsMode,
) -> wgpu::RenderPipeline {
    let shader = terrain_shader(alpha_cutout, texture_filtering, dynamic_lights_mode);
    RenderPipelineBuilder::new(device, label)
        .shader("bbb-terrain-shader", &shader)
        .layout(
            "bbb-terrain-pipeline-layout",
            &[camera_bind_group_layout, lightmap_bind_group_layout],
        )
        .vertex_buffers(&TERRAIN_VERTEX_BUFFERS)
        .color_target(format, blend)
        .cull_mode(TERRAIN_PIPELINE_CULL_MODE)
        .depth_stencil(depth_stencil_state(
            DEPTH_FORMAT,
            depth_write_enabled,
            TERRAIN_PIPELINE_DEPTH_COMPARE,
        ))
        .build()
}

pub struct TerrainCorePipelines {
    pub solid: wgpu::RenderPipeline,
    pub cutout: wgpu::RenderPipeline,
    pub translucent: wgpu::RenderPipeline,
}

impl TerrainCorePipelines {
    pub(crate) fn get(&self, layer: crate::renderer::TerrainLayer) -> &wgpu::RenderPipeline {
        match layer {
            crate::renderer::TerrainLayer::Solid => &self.solid,
            crate::renderer::TerrainLayer::Cutout => &self.cutout,
            crate::renderer::TerrainLayer::Translucent => &self.translucent,
        }
    }
}

pub fn create_embedded_terrain_pipelines(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    texture_filtering: TerrainTextureFiltering,
    dynamic_lights_mode: DynamicLightsMode,
) -> TerrainCorePipelines {
    let pipeline = |label, depth_write_enabled, blend, alpha_cutout| {
        create_embedded_terrain_pipeline(
            device,
            format,
            camera_bind_group_layout,
            lightmap_bind_group_layout,
            label,
            depth_write_enabled,
            blend,
            alpha_cutout,
            texture_filtering,
            dynamic_lights_mode,
        )
    };
    TerrainCorePipelines {
        solid: pipeline(
            "bbb-terrain-pipeline",
            TERRAIN_PIPELINE_OPAQUE_DEPTH_WRITE_ENABLED,
            TERRAIN_PIPELINE_OPAQUE_BLEND,
            None,
        ),
        cutout: pipeline(
            "bbb-terrain-cutout-pipeline",
            TERRAIN_PIPELINE_OPAQUE_DEPTH_WRITE_ENABLED,
            TERRAIN_PIPELINE_OPAQUE_BLEND,
            Some(TERRAIN_CUTOUT_ALPHA_THRESHOLD),
        ),
        translucent: pipeline(
            "bbb-terrain-translucent-pipeline",
            TERRAIN_PIPELINE_TRANSLUCENT_DEPTH_WRITE_ENABLED,
            TERRAIN_PIPELINE_TRANSLUCENT_BLEND,
            Some(TERRAIN_TRANSLUCENT_ALPHA_THRESHOLD),
        ),
    }
}

pub fn create_terrain_core_pipelines(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::render_core::core_shaders::AdaptedTerrainShaders>,
    texture_filtering: TerrainTextureFiltering,
    dynamic_lights_mode: DynamicLightsMode,
) -> Result<TerrainCorePipelines> {
    let Some(adapted) = shaders else {
        return Ok(create_embedded_terrain_pipelines(
            device,
            format,
            camera_bind_group_layout,
            lightmap_bind_group_layout,
            texture_filtering,
            dynamic_lights_mode,
        ));
    };
    let pipeline = |label, depth_write_enabled, blend, alpha_cutout| {
        create_selected_terrain_pipeline(
            device,
            format,
            camera_bind_group_layout,
            lightmap_bind_group_layout,
            label,
            adapted,
            depth_write_enabled,
            blend,
            alpha_cutout,
        )
    };
    Ok(TerrainCorePipelines {
        solid: pipeline(
            "bbb-selected-terrain-pipeline",
            TERRAIN_PIPELINE_OPAQUE_DEPTH_WRITE_ENABLED,
            TERRAIN_PIPELINE_OPAQUE_BLEND,
            None,
        )?,
        cutout: pipeline(
            "bbb-selected-terrain-cutout-pipeline",
            TERRAIN_PIPELINE_OPAQUE_DEPTH_WRITE_ENABLED,
            TERRAIN_PIPELINE_OPAQUE_BLEND,
            Some(TERRAIN_CUTOUT_ALPHA_THRESHOLD),
        )?,
        translucent: pipeline(
            "bbb-selected-terrain-translucent-pipeline",
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
    label: &'static str,
    shaders: &crate::render_core::core_shaders::AdaptedTerrainShaders,
    depth_write_enabled: bool,
    blend: Option<wgpu::BlendState>,
    alpha_cutout: Option<&str>,
) -> Result<wgpu::RenderPipeline> {
    let defines = alpha_cutout.map_or_else(Vec::new, |value| vec![("ALPHA_CUTOUT", Some(value))]);
    let vertex_source =
        crate::render_core::core_shaders::shader_with_defines(&shaders.vertex, &defines);
    let fragment_source =
        crate::render_core::core_shaders::shader_with_defines(&shaders.fragment, &defines);
    let (vertex, fragment) = crate::render_core::core_shaders::compile_selected_shader_stages(
        device,
        label,
        &vertex_source,
        &fragment_source,
    )?;
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-selected-terrain-pipeline-layout"),
        bind_group_layouts: &[
            Some(camera_bind_group_layout),
            Some(lightmap_bind_group_layout),
        ],
        immediate_size: 0,
    });
    Ok(
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &vertex,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                buffers: &TERRAIN_VERTEX_BUFFERS.map(Some),
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
                entry_point: Some("main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        }),
    )
}

fn terrain_shader(
    alpha_cutout: Option<&str>,
    texture_filtering: TerrainTextureFiltering,
    dynamic_lights_mode: DynamicLightsMode,
) -> String {
    let alpha_cutout = alpha_cutout.map_or_else(String::new, |threshold| {
        format!("    if texel.a < {threshold} {{\n        discard;\n    }}")
    });
    let terrain_sample = match texture_filtering {
        // Vanilla `terrain.fsh` has no anisotropic branch: `UseRgss == 0`
        // takes `sampleNearest` and the anisotropy lives in the sampler.
        TerrainTextureFiltering::None | TerrainTextureFiltering::Anisotropic => {
            "    let texel = sample_terrain_nearest(input.uv, vec2<f32>(1.0) / atlas_size);"
        }
        TerrainTextureFiltering::Rgss => {
            "    let texel = sample_terrain_rgss(input.uv, vec2<f32>(1.0) / atlas_size);"
        }
    };
    let (vertex_out_dyn, vs_dyn, fs_dyn) = match dynamic_lights_mode {
        DynamicLightsMode::Off => ("", "", ""),
        DynamicLightsMode::Fast => (
            "",
            r#"    if (u32(camera.dynamic_light_params.y) == 1u && u32(camera.dynamic_light_params.x) > 0u) {
        let dyn_light = evaluate_dynamic_lights(fog_pos);
        out.vertex_color = vec4<f32>(
            max(out.vertex_color.rgb, input.color.rgb * dyn_light),
            out.vertex_color.a,
        );
    }"#,
            "",
        ),
        DynamicLightsMode::Fancy => (
            "    @location(5) fog_pos: vec3<f32>,\n    @location(6) base_tint_ao: vec3<f32>,",
            "    out.fog_pos = fog_pos;\n    out.base_tint_ao = input.color.rgb;",
            r#"    if (u32(camera.dynamic_light_params.y) == 2u && u32(camera.dynamic_light_params.x) > 0u) {
        let dyn_light = evaluate_dynamic_lights(input.fog_pos);
        let dyn_rgb = texel.rgb * input.base_tint_ao * dyn_light;
        color = vec4<f32>(
            mix(color.rgb, max(color.rgb, dyn_rgb), input.section_visibility),
            color.a,
        );
    }"#,
        ),
    };
    TERRAIN_SHADER
        .replace(TERRAIN_SAMPLE_PLACEHOLDER, terrain_sample)
        .replace(TERRAIN_ALPHA_CUTOUT_PLACEHOLDER, &alpha_cutout)
        .replace(
            TERRAIN_VERTEX_OUT_DYNAMIC_LIGHTS_PLACEHOLDER,
            vertex_out_dyn,
        )
        .replace(TERRAIN_VS_DYNAMIC_LIGHTS_PLACEHOLDER, vs_dyn)
        .replace(TERRAIN_FS_DYNAMIC_LIGHTS_PLACEHOLDER, fs_dyn)
}

// Buffer 1 carries each section's integer block origin and fade (`ChunkVisibility`).
const TERRAIN_VERTEX_BUFFERS: [wgpu::VertexBufferLayout<'static>; 2] = [
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<TerrainGpuVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &TERRAIN_VERTEX_ATTRIBUTES,
    },
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<TerrainSectionInstance>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &TERRAIN_SECTION_ATTRIBUTES,
    },
];

/// Vanilla 26.1 terrain texture filtering modes currently owned by the native renderer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TerrainTextureFiltering {
    /// Vanilla's `TextureFilteringMethod.NONE` custom nearest-looking shader path.
    #[default]
    None,
    /// Vanilla's four-tap rotated-grid super-sampling shader path.
    Rgss,
    /// Vanilla's `TextureFilteringMethod.ANISOTROPIC`: the same nearest-looking
    /// shader path, with sampler anisotropy and matching atlas padding.
    Anisotropic,
}
/// Vanilla `Options.effectiveMaxAnisotropy` is `min(1 << maxAnisotropyBit,
/// device max)`; `LevelRenderer` passes it to the chunk-layer sampler only
/// under `TextureFilteringMethod.ANISOTROPIC`, else 1. wgpu has no queryable
/// device maximum, so the bit range 1..=3 (2x/4x/8x) is clamped to wgpu's
/// 16x sampler ceiling and the backend clamps to hardware support.
pub fn effective_terrain_max_anisotropy(
    filtering: TerrainTextureFiltering,
    max_anisotropy_bit: u32,
) -> u16 {
    if filtering != TerrainTextureFiltering::Anisotropic {
        return 1;
    }
    1_u32
        .checked_shl(max_anisotropy_bit)
        .unwrap_or(u32::from(u16::MAX))
        .min(16) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgss_hardware_mip_sampling_stays_within_unorm_quantization() {
        use crate::renderer::{
            TerrainChunkMeshOwner, TerrainLayer, TerrainSectionMeshOwner, TerrainSectionRenderState,
        };
        use crate::terrain::{
            TerrainMesh, TerrainSectionMesh, TerrainSectionMeshLayers, TerrainTint,
        };

        // Keep manual mip interpolation independent of the hardware sampling path.
        const MANUAL_RGSS: &str = r#"fn sample_terrain_rgss(uv: vec2<f32>, pixel_size: vec2<f32>) -> vec4<f32> {
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
"#;
        let Some(mut renderer) = crate::Renderer::new_offscreen_with_terrain_filtering(
            96,
            96,
            TerrainTextureFiltering::Rgss,
            true,
        ) else {
            return;
        };
        renderer.set_clear_color(crate::ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        renderer.set_camera_pose(Some(crate::CameraPose {
            position: [0.0; 3],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }));

        let mips: Vec<Vec<u8>> = (0..5)
            .map(|level| {
                let width = 256 >> level;
                (0..width * width)
                    .flat_map(|index| {
                        let x = index % width;
                        let y = index / width;
                        let red = if (x + y) % 2 == 0 { 224 } else { 32 };
                        let alpha = [0, 126, 127, 128, 129, 255][(x / 3 + y / 5) % 6];
                        [red, 32 + level as u8 * 40, 255 - red, alpha]
                    })
                    .collect()
            })
            .collect();
        renderer
            .upload_terrain_texture_atlas_mips(
                256,
                256,
                &mips.iter().map(Vec::as_slice).collect::<Vec<_>>(),
            )
            .unwrap();
        let owner = TerrainSectionMeshOwner {
            chunk: TerrainChunkMeshOwner {
                level_generation: 0,
                chunk_x: 0,
                chunk_z: 0,
            },
            section_y: 0,
        };

        let cutout_alpha = TERRAIN_CUTOUT_ALPHA_THRESHOLD.parse::<f32>().unwrap() * 255.0;
        let cutout_alpha_quantization = cutout_alpha.floor() as u8..=cutout_alpha.ceil() as u8;
        let mut solid_frames = Vec::new();
        for (layer, alpha_cutout) in [
            (TerrainLayer::Solid, None),
            (TerrainLayer::Cutout, Some(TERRAIN_CUTOUT_ALPHA_THRESHOLD)),
        ] {
            let optimized_pipeline = renderer.pipelines.terrain.get(layer).clone();
            let mut reference_source = terrain_shader(
                alpha_cutout,
                TerrainTextureFiltering::Rgss,
                DynamicLightsMode::Off,
            )
            .replacen("fn sample_terrain_rgss(", "fn unused_rgss(", 1);
            reference_source.push_str(MANUAL_RGSS);
            let reference_pipeline =
                RenderPipelineBuilder::new(&renderer.device, "bbb-rgss-manual-mip-reference")
                    .shader("bbb-rgss-manual-mip-reference", &reference_source)
                    .layout(
                        "bbb-rgss-reference-layout",
                        &[
                            &renderer.terrain_bind_group_layout,
                            &renderer.lightmap_sample_bind_group_layout,
                        ],
                    )
                    .vertex_buffers(&TERRAIN_VERTEX_BUFFERS)
                    .color_target(renderer.config.format, TERRAIN_PIPELINE_OPAQUE_BLEND)
                    .cull_mode(TERRAIN_PIPELINE_CULL_MODE)
                    .depth_stencil(depth_stencil_state(
                        DEPTH_FORMAT,
                        true,
                        TERRAIN_PIPELINE_DEPTH_COMPARE,
                    ))
                    .build();
            for (case, (uv_span, far_z)) in [
                (0.125, 2.0),
                (0.375, 2.0),
                (0.75, 2.0),
                (1.0, 2.0),
                (0.75, 8.0),
                (1.0, 24.0),
            ]
            .into_iter()
            .enumerate()
            {
                let uv_min = 0.5 - uv_span * 0.5;
                let uv_max = 0.5 + uv_span * 0.5;
                let vertices = [
                    ([-1.0, -1.0, 2.0], [uv_min, uv_min]),
                    ([1.0, -1.0, far_z], [uv_max, uv_min]),
                    ([1.0, 1.0, far_z], [uv_max, uv_max]),
                    ([-1.0, 1.0, 2.0], [uv_min, uv_max]),
                ]
                .map(|(position, uv)| {
                    TerrainGpuVertex::pack(position, uv, 0x00F0_00F0, TerrainTint::WHITE, 1.0, 1.0)
                });
                let mesh = TerrainSectionMesh {
                    section_y: 0,
                    mesh: TerrainMesh {
                        vertices: vertices.to_vec(),
                        indices: vec![0, 1, 2, 2, 3, 0, 0, 3, 2, 2, 1, 0],
                        source_sections: 1,
                        opaque_faces: usize::from(layer == TerrainLayer::Solid) * 2,
                        cutout_faces: usize::from(layer == TerrainLayer::Cutout) * 2,
                        ..TerrainMesh::default()
                    },
                };
                let mut layers = TerrainSectionMeshLayers {
                    source_sections: 1,
                    ..TerrainSectionMeshLayers::default()
                };
                if layer == TerrainLayer::Cutout {
                    layers.cutout.push(mesh);
                } else {
                    layers.opaque.push(mesh);
                }
                renderer.update_terrain_section_mesh_layer_refs(
                    1,
                    [(owner, case as u64 + 1, &layers, [0.0; 3])],
                    [],
                );
                renderer.set_terrain_section_states(
                    case as u64 + 1,
                    &[TerrainSectionRenderState {
                        owner,
                        visibility: 1.0,
                        nearby: true,
                    }],
                );
                let pipeline = if layer == TerrainLayer::Cutout {
                    &mut renderer.pipelines.terrain.cutout
                } else {
                    &mut renderer.pipelines.terrain.solid
                };
                *pipeline = optimized_pipeline.clone();
                let actual = renderer.render_offscreen_frame().unwrap();
                let pipeline = if layer == TerrainLayer::Cutout {
                    &mut renderer.pipelines.terrain.cutout
                } else {
                    &mut renderer.pipelines.terrain.solid
                };
                *pipeline = reference_pipeline.clone();
                let expected = renderer.render_offscreen_frame().unwrap();
                assert!(
                    expected
                        .rgba
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .filter(|p| p[0] > 32)
                        .count()
                        > 32
                );
                let maximum_error = actual
                    .rgba
                    .iter()
                    .zip(&expected.rgba)
                    .map(|(&a, &b)| a.abs_diff(b))
                    .max()
                    .unwrap();
                eprintln!("RGSS {layer:?}, UV span {uv_span}, far z {far_z}: maximum byte error {maximum_error}");
                if layer == TerrainLayer::Solid {
                    assert!(
                        maximum_error <= 1,
                        "Solid, UV span {uv_span}, far z {far_z}"
                    );
                    solid_frames.push((actual, expected));
                } else {
                    let (solid_actual, solid_expected) = &solid_frames[case];
                    for (pixel, (&actual, &expected)) in actual
                        .rgba
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .zip(expected.rgba.as_chunks::<4>().0)
                        .enumerate()
                    {
                        let clear = [0, 0, 255, 255];
                        if (actual == clear) != (expected == clear) {
                            // One UNORM step can cross the cutout threshold; only its
                            // two adjacent alpha values may explain a coverage change.
                            let alpha_offset = pixel * 4 + 3;
                            assert!(
                                [
                                    solid_actual.rgba[alpha_offset],
                                    solid_expected.rgba[alpha_offset]
                                ]
                                .into_iter()
                                .all(|alpha| cutout_alpha_quantization.contains(&alpha)),
                                "Cutout coverage, UV span {uv_span}, far z {far_z}, pixel {pixel}"
                            );
                        } else {
                            assert!(
                                actual
                                    .iter()
                                    .zip(expected)
                                    .all(|(&a, b)| a.abs_diff(b) <= 1),
                                "Cutout color, UV span {uv_span}, far z {far_z}, pixel {pixel}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn terrain_vertex_layout_matches_vanilla_block_format() {
        assert_eq!(TERRAIN_VERTEX_BUFFERS[0].array_stride, 16);
        assert_eq!(TERRAIN_VERTEX_BUFFERS[1].array_stride, 16);
        assert_eq!(TERRAIN_VERTEX_ATTRIBUTES.len(), 3);
        assert_eq!(
            TERRAIN_VERTEX_ATTRIBUTES[0].format,
            wgpu::VertexFormat::Sint16x4
        );
        assert_eq!(
            TERRAIN_VERTEX_ATTRIBUTES[1].format,
            wgpu::VertexFormat::Unorm8x4
        );
        assert_eq!(
            TERRAIN_VERTEX_ATTRIBUTES[2].format,
            wgpu::VertexFormat::Uint16x2
        );
        assert_eq!(
            TERRAIN_SECTION_ATTRIBUTES[0].format,
            wgpu::VertexFormat::Sint32x3
        );
        assert_eq!(
            TERRAIN_SECTION_ATTRIBUTES[1].format,
            wgpu::VertexFormat::Float32
        );
    }

    #[test]
    fn terrain_gpu_vertex_packs_vanilla_color_and_light_coordinates() {
        let packed = TerrainGpuVertex::pack(
            [1.0, 2.0, 3.0],
            [0.25, 0.75],
            2 << 4 | 13 << 20,
            crate::terrain::TerrainTint::from_rgb_u8(128, 255, 64),
            0.8,
            0.75,
        );

        // ARGB.gray(AO), cardinal scale, then tint multiplication all truncate.
        assert_eq!(packed.color, [76, 152, 38, 255]);
        assert_eq!(packed.local_position(), [1.0, 2.0, 3.0]);
        assert_eq!(packed.uv_f32(), [0.25, 0.75]);
        assert_eq!(packed.light_coords(), [32, 208]);
        assert_eq!(std::mem::size_of::<TerrainGpuVertex>(), 16);
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
        const {
            assert!(TERRAIN_PIPELINE_OPAQUE_DEPTH_WRITE_ENABLED);
            assert!(TERRAIN_PIPELINE_TRANSLUCENT_DEPTH_WRITE_ENABLED);
        }
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
        let solid = terrain_shader(None, TerrainTextureFiltering::None, DynamicLightsMode::Off);
        let cutout = terrain_shader(
            Some(TERRAIN_CUTOUT_ALPHA_THRESHOLD),
            TerrainTextureFiltering::None,
            DynamicLightsMode::Off,
        );
        let translucent = terrain_shader(
            Some(TERRAIN_TRANSLUCENT_ALPHA_THRESHOLD),
            TerrainTextureFiltering::None,
            DynamicLightsMode::Off,
        );

        assert!(!solid.contains("discard"));
        assert!(cutout.contains("if texel.a < 0.5"));
        assert!(translucent.contains("if texel.a < 0.01"));
        assert!(!cutout.contains("texel.a < 0.01"));
    }

    #[test]
    fn terrain_shader_specializes_dynamic_lights_and_uses_squared_distance_early_out() {
        let off = terrain_shader(None, TerrainTextureFiltering::None, DynamicLightsMode::Off);
        let fast = terrain_shader(None, TerrainTextureFiltering::None, DynamicLightsMode::Fast);
        let fancy = terrain_shader(
            None,
            TerrainTextureFiltering::None,
            DynamicLightsMode::Fancy,
        );

        assert!(!off.contains("base_tint_ao"));
        assert!(!off.contains("evaluate_dynamic_lights(input.fog_pos)"));
        assert!(!off.contains("evaluate_dynamic_lights(fog_pos)"));
        assert!(!fast.contains("base_tint_ao"));
        assert!(fast.contains("evaluate_dynamic_lights(fog_pos)"));
        assert!(!fast.contains("evaluate_dynamic_lights(input.fog_pos)"));
        assert!(fancy.contains("base_tint_ao"));
        assert!(fancy.contains("evaluate_dynamic_lights(input.fog_pos)"));
        assert!(fancy.contains("if (dist_norm_sq >= 1.0)"));
    }

    #[test]
    fn terrain_shader_consumes_prepacked_vanilla_vertex_color() {
        assert!(TERRAIN_SHADER.contains("@location(1) color: vec4<f32>"));
        assert!(TERRAIN_SHADER.contains("texel * input.vertex_color,"));
        assert!(TERRAIN_SHADER.contains("input.section_visibility,"));
        assert!(!TERRAIN_SHADER.contains("ambient_occlusion"));
        assert!(!TERRAIN_SHADER.contains("input.shade"));
        assert!(!TERRAIN_SHADER.contains("light_dir"));
    }

    #[test]
    fn anisotropic_filtering_keeps_the_nearest_shader_and_scales_the_sampler() {
        // Vanilla `terrain.fsh` has no anisotropic branch (`UseRgss == 0`
        // takes `sampleNearest`); the mode only changes the sampler.
        let anisotropic = terrain_shader(
            None,
            TerrainTextureFiltering::Anisotropic,
            DynamicLightsMode::Off,
        );
        assert!(anisotropic
            .contains("let texel = sample_terrain_nearest(input.uv, vec2<f32>(1.0) / atlas_size)"));
        assert!(!anisotropic.contains("let texel = sample_terrain_rgss"));

        // Vanilla `effectiveMaxAnisotropy()` = `min(1 << bit, device max)`,
        // consumed only under `ANISOTROPIC` (else the sampler gets 1).
        use crate::render_core::gpu::{effective_terrain_max_anisotropy, TerrainTextureFiltering};
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
        let nearest = terrain_shader(None, TerrainTextureFiltering::None, DynamicLightsMode::Off);
        let rgss = terrain_shader(None, TerrainTextureFiltering::Rgss, DynamicLightsMode::Off);

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
        assert_eq!(TERRAIN_ATLAS_MIPMAP_FILTER, wgpu::MipmapFilterMode::Linear);
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
        assert!(TERRAIN_SHADER
            .contains("out.vertex_color = input.color * vec4<f32>(sample_lightmap(light), 1.0)"));
        assert_eq!(TERRAIN_SHADER.matches("sample_lightmap(light)").count(), 1);
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
        assert!(validate_terrain_atlas_mips(4, 4, &[wrong_size.as_slice()]).is_err());

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
