use std::{mem, ops::Index};

use anyhow::Result;
use bbb_render_types::ParticleFacingCameraMode;
use glam::{Mat3, Quat, Vec3};

use crate::{
    gpu::DEPTH_FORMAT,
    particles::{ParticleSpriteUv, ParticleUvRect},
    pipeline_builder::{depth_stencil_state, RenderPipelineBuilder},
    texture_sampling::{
        create_camera_texture_bind_group, create_nearest_clamp_sampler, upload_rgba_texture,
        validate_rgba, write_texture_rgba,
    },
};

pub(crate) struct ParticleAtlasGpu {
    pub(crate) texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) bind_group: wgpu::BindGroup,
    /// Indexed by particle-atlas sprite index.
    pub(crate) sprite_uvs: Vec<ParticleUvRect>,
}

const ORIENTATION_TAG_LOOK_AT_XYZ: i16 = -32767;
const ORIENTATION_TAG_LOOK_AT_Y: i16 = -16384;

/// Compact 52-byte per-particle instance record expanded into a 6-vertex quad
/// in `PARTICLE_SHADER` (`VertexStepMode::Instance`).
///
/// `orientation` (`Snorm16x4`) encodes three modes without extra attributes:
/// - `w < -0.75` (`-32767`): camera-facing `LookAtXyz` billboard with `(cos_roll, sin_roll)` in `xy`
/// - `-0.75 <= w < -0.25` (`-16384`): upright `LookAtY` billboard with `(cos_roll, sin_roll)` in `xy`
/// - `w >= -0.25` (`w >= 0`): canonicalized world-space unit quaternion `(qx, qy, qz, qw)`
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleGpuInstance {
    pub(crate) center_and_size: [f32; 4],
    pub uv: [f32; 4],
    pub(crate) orientation: [i16; 4],
    pub(crate) color: [u16; 4],
    pub(crate) light: [u16; 2],
}

impl ParticleGpuInstance {
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn billboard(
        center: Vec3,
        half_size: f32,
        uv: [f32; 4],
        facing_mode: ParticleFacingCameraMode,
        cos_roll: f32,
        sin_roll: f32,
        color: [f32; 4],
        light: [f32; 2],
    ) -> Self {
        let tag = match facing_mode {
            ParticleFacingCameraMode::LookAtXyz => ORIENTATION_TAG_LOOK_AT_XYZ,
            ParticleFacingCameraMode::LookAtY => ORIENTATION_TAG_LOOK_AT_Y,
        };
        Self {
            center_and_size: [center.x, center.y, center.z, half_size],
            uv,
            orientation: [f32_to_snorm16(cos_roll), f32_to_snorm16(sin_roll), 0, tag],
            color: color.map(f32_to_f16),
            light: light.map(f32_to_f16),
        }
    }

    #[inline]
    pub fn rotated(
        center: Vec3,
        half_size: f32,
        uv: [f32; 4],
        rotation: Quat,
        color: [f32; 4],
        light: [f32; 2],
    ) -> Self {
        let q = rotation.normalize();
        let q = if q.w < 0.0 { -q } else { q };
        Self {
            center_and_size: [center.x, center.y, center.z, half_size],
            uv,
            orientation: [
                f32_to_snorm16(q.x),
                f32_to_snorm16(q.y),
                f32_to_snorm16(q.z),
                f32_to_snorm16(q.w).max(0),
            ],
            color: color.map(f32_to_f16),
            light: light.map(f32_to_f16),
        }
    }

    pub(crate) fn from_quad(quad: ParticleQuad) -> Self {
        let [bl, br, tr, tl] = quad.corners.map(Vec3::from_array);
        let center = (bl + br + tr + tl) * 0.25;
        let right_half = (br - bl) * 0.5;
        let up_half = (tl - bl) * 0.5;
        let half_size = right_half.length();
        let rotation = if half_size <= 1e-7 {
            Quat::IDENTITY
        } else {
            let x_axis = right_half / half_size;
            let y_axis = up_half.normalize_or_zero();
            let z_axis = x_axis.cross(y_axis).normalize_or_zero();
            if y_axis.length_squared() <= 1e-12 || z_axis.length_squared() <= 1e-12 {
                Quat::IDENTITY
            } else {
                Quat::from_mat3(&Mat3::from_cols(x_axis, y_axis, z_axis))
            }
        };
        Self::rotated(center, half_size, quad.uv, rotation, quad.color, quad.light)
    }

    pub fn light_f32(&self) -> [f32; 2] {
        self.light.map(f16_to_f32)
    }

    fn vertices(&self, cam_right: Vec3, cam_up: Vec3) -> [ParticleVertex; 6] {
        let center = Vec3::new(
            self.center_and_size[0],
            self.center_and_size[1],
            self.center_and_size[2],
        );
        let half_size = self.center_and_size[3];
        let orient = self.orientation.map(snorm16_to_f32);
        let corners = if orient[3] < -0.25 {
            let base_up = if orient[3] < -0.75 { cam_up } else { Vec3::Y };
            let right = (cam_right * orient[0] + base_up * orient[1]) * half_size;
            let up = (-cam_right * orient[1] + base_up * orient[0]) * half_size;
            [
                (center - right - up).to_array(),
                (center + right - up).to_array(),
                (center + right + up).to_array(),
                (center - right + up).to_array(),
            ]
        } else {
            let q = Quat::from_xyzw(orient[0], orient[1], orient[2], orient[3]).normalize();
            [
                (center + q * Vec3::new(-half_size, -half_size, 0.0)).to_array(),
                (center + q * Vec3::new(half_size, -half_size, 0.0)).to_array(),
                (center + q * Vec3::new(half_size, half_size, 0.0)).to_array(),
                (center + q * Vec3::new(-half_size, half_size, 0.0)).to_array(),
            ]
        };
        let [u0, v0, u1, v1] = self.uv;
        let uvs = [[u0, v1], [u1, v1], [u1, v0], [u0, v0]];
        let color = self.color.map(f16_to_f32);
        let light = self.light.map(f16_to_f32);
        [0, 1, 2, 0, 2, 3].map(|corner| ParticleVertex {
            position: corners[corner],
            uv: uvs[corner],
            color,
            light,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ParticleQuadBuffer {
    instances: Vec<ParticleGpuInstance>,
    cam_right: Vec3,
    cam_up: Vec3,
}

impl Default for ParticleQuadBuffer {
    fn default() -> Self {
        Self {
            instances: Vec::new(),
            cam_right: Vec3::X,
            cam_up: Vec3::Y,
        }
    }
}

impl PartialEq for ParticleQuadBuffer {
    fn eq(&self, other: &Self) -> bool {
        self.instances == other.instances
    }
}

impl ParticleQuadBuffer {
    #[inline]
    pub fn clear(&mut self) {
        self.instances.clear();
        self.cam_right = Vec3::X;
        self.cam_up = Vec3::Y;
    }

    #[inline]
    pub fn set_camera_axes(&mut self, right: Vec3, up: Vec3) {
        self.cam_right = right;
        self.cam_up = up;
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    #[inline]
    pub fn first(&self) -> Option<&ParticleGpuInstance> {
        self.instances.first()
    }

    #[inline]
    pub fn push_instance(&mut self, instance: ParticleGpuInstance) {
        self.instances.push(instance);
    }

    #[cfg(test)]
    pub(crate) fn push(&mut self, quad: ParticleQuad) {
        self.instances.push(ParticleGpuInstance::from_quad(quad));
    }

    #[inline]
    pub(crate) fn as_instances(&self) -> &[ParticleGpuInstance] {
        &self.instances
    }
}

impl Extend<ParticleQuad> for ParticleQuadBuffer {
    fn extend<T: IntoIterator<Item = ParticleQuad>>(&mut self, iter: T) {
        self.instances
            .extend(iter.into_iter().map(ParticleGpuInstance::from_quad));
    }
}

impl Index<usize> for ParticleQuadBuffer {
    type Output = ParticleGpuInstance;

    fn index(&self, index: usize) -> &Self::Output {
        &self.instances[index]
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ParticleQuad {
    pub(crate) corners: [[f32; 3]; 4],
    pub(crate) uv: [f32; 4],
    pub(crate) color: [f32; 4],
    pub(crate) light: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ParticleVertex {
    position: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
    light: [f32; 2],
}

impl super::ParticleBatch {
    pub(crate) fn upload_staged(
        &self,
        buffer: &mut crate::frame_buffers::FrameDataBuffer,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging: &mut crate::frame_buffers::UploadBelt,
        instanced: bool,
    ) -> bool {
        if instanced {
            return buffer.upload_staged(
                device,
                encoder,
                staging,
                bytemuck::cast_slice(self.quads.as_instances()),
            );
        }
        // A pack vertex shader can observe the original vertex and instance IDs.
        let cam_right = self.quads.cam_right;
        let cam_up = self.quads.cam_up;
        let vertices: Vec<_> = self
            .quads
            .as_instances()
            .iter()
            .flat_map(|instance| instance.vertices(cam_right, cam_up))
            .collect();
        buffer.upload_staged(device, encoder, staging, bytemuck::cast_slice(&vertices))
    }
}

const PARTICLE_INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
    0 => Float32x4,
    1 => Float32x4,
    2 => Snorm16x4,
    3 => Float16x4,
    4 => Float16x2
];
const PARTICLE_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4, 3 => Float32x2];
const PARTICLE_PIPELINE_CULL_MODE: Option<wgpu::Face> = Some(wgpu::Face::Back);
const PARTICLE_PIPELINE_DEPTH_WRITE_ENABLED: bool = true;
const PARTICLE_PIPELINE_DEPTH_COMPARE: wgpu::CompareFunction = wgpu::CompareFunction::LessEqual;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParticlePipelineKind {
    Opaque,
    Translucent,
}

impl ParticlePipelineKind {
    fn pipeline_label(self) -> &'static str {
        match self {
            Self::Opaque => "bbb-opaque-particle-pipeline",
            Self::Translucent => "bbb-translucent-particle-pipeline",
        }
    }

    fn pipeline_layout_label(self) -> &'static str {
        match self {
            Self::Opaque => "bbb-opaque-particle-pipeline-layout",
            Self::Translucent => "bbb-translucent-particle-pipeline-layout",
        }
    }

    fn blend_state(self) -> Option<wgpu::BlendState> {
        match self {
            Self::Opaque => None,
            Self::Translucent => Some(wgpu::BlendState::ALPHA_BLENDING),
        }
    }
}

const PARTICLE_SHADER: &str = concat!(
    crate::wgsl::camera_full!(),
    crate::wgsl::linear_fog_value!(),
    crate::wgsl::apply_fog!(),
    crate::wgsl::sample_lightmap!(),
    r#"
@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var particle_atlas: texture_2d<f32>;

@group(0) @binding(2)
var particle_sampler: sampler;

@group(1) @binding(0)
var lightmap_texture: texture_2d<f32>;

@group(1) @binding(1)
var lightmap_sampler: sampler;

struct VertexIn {
    @location(0) center_and_size: vec4<f32>,
    @location(1) uv: vec4<f32>,
    @location(2) orientation: vec4<f32>,
    @location(3) color: vec4<f32>,
    @location(4) light: vec2<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) light: vec2<f32>,
    @location(3) spherical_distance: f32,
    @location(4) cylindrical_distance: f32,
};

@vertex
fn vs_main(input: VertexIn, @builtin(vertex_index) vertex: u32) -> VertexOut {
    var out: VertexOut;
    var corner_xy: vec2<f32>;
    switch vertex {
        case 0u, 3u: { corner_xy = vec2<f32>(-1.0, -1.0); out.uv = input.uv.xw; }
        case 1u: { corner_xy = vec2<f32>(1.0, -1.0); out.uv = input.uv.zw; }
        case 2u, 4u: { corner_xy = vec2<f32>(1.0, 1.0); out.uv = input.uv.zy; }
        default: { corner_xy = vec2<f32>(-1.0, 1.0); out.uv = input.uv.xy; }
    }
    let half_size = input.center_and_size.w;
    var offset: vec3<f32>;
    if input.orientation.w < -0.25 {
        let cam_right = vec3<f32>(
            camera.sky_model_view[0].x,
            camera.sky_model_view[1].x,
            camera.sky_model_view[2].x,
        );
        var base_up = vec3<f32>(0.0, 1.0, 0.0);
        if input.orientation.w < -0.75 {
            base_up = vec3<f32>(
                camera.sky_model_view[0].y,
                camera.sky_model_view[1].y,
                camera.sky_model_view[2].y,
            );
        }
        let cos_roll = input.orientation.x;
        let sin_roll = input.orientation.y;
        let right = cam_right * cos_roll + base_up * sin_roll;
        let up = -cam_right * sin_roll + base_up * cos_roll;
        offset = (right * corner_xy.x + up * corner_xy.y) * half_size;
    } else {
        let q = normalize(input.orientation);
        let local = vec3<f32>(corner_xy * half_size, 0.0);
        let t = 2.0 * cross(q.xyz, local);
        offset = local + q.w * t + cross(q.xyz, t);
    }
    let position = input.center_and_size.xyz + offset;
    out.position = camera.view_proj * vec4<f32>(position, 1.0);
    out.color = input.color;
    out.light = input.light;
    let fog_pos = position - camera.camera_position.xyz;
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
    let light_color = sample_lightmap(input.light);
    return apply_fog(vec4<f32>(texel.rgb * light_color, texel.a), input.spherical_distance, input.cylindrical_distance);
}
"#,
);

pub(crate) fn create_particle_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    kind: ParticlePipelineKind,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, kind.pipeline_label())
        .shader("bbb-particle-shader", PARTICLE_SHADER)
        .layout(
            kind.pipeline_layout_label(),
            &[bind_group_layout, lightmap_bind_group_layout],
        )
        .vertex_buffers(&[wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ParticleGpuInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &PARTICLE_INSTANCE_ATTRIBUTES,
        }])
        .color_target(format, kind.blend_state())
        .cull_mode(PARTICLE_PIPELINE_CULL_MODE)
        .depth_stencil(depth_stencil_state(
            DEPTH_FORMAT,
            PARTICLE_PIPELINE_DEPTH_WRITE_ENABLED,
            PARTICLE_PIPELINE_DEPTH_COMPARE,
        ))
        .build()
}

pub(crate) struct ParticleCorePipelines {
    pub(crate) opaque: wgpu::RenderPipeline,
    pub(crate) translucent: wgpu::RenderPipeline,
}

pub(crate) fn create_particle_core_pipelines(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedParticleShaders>,
) -> Result<ParticleCorePipelines> {
    let Some(shaders) = shaders else {
        return Ok(ParticleCorePipelines {
            opaque: create_particle_pipeline(
                device,
                format,
                bind_group_layout,
                lightmap_bind_group_layout,
                ParticlePipelineKind::Opaque,
            ),
            translucent: create_particle_pipeline(
                device,
                format,
                bind_group_layout,
                lightmap_bind_group_layout,
                ParticlePipelineKind::Translucent,
            ),
        });
    };
    Ok(ParticleCorePipelines {
        opaque: create_selected_particle_pipeline(
            device,
            format,
            bind_group_layout,
            lightmap_bind_group_layout,
            "bbb-selected-opaque-particle",
            &shaders.vertex,
            &shaders.fragment,
            particle_vertex_layout(),
            None,
            PARTICLE_PIPELINE_CULL_MODE,
            PARTICLE_PIPELINE_DEPTH_WRITE_ENABLED,
        )?,
        translucent: create_selected_particle_pipeline(
            device,
            format,
            bind_group_layout,
            lightmap_bind_group_layout,
            "bbb-selected-translucent-particle",
            &shaders.vertex,
            &shaders.fragment,
            particle_vertex_layout(),
            Some(wgpu::BlendState::ALPHA_BLENDING),
            PARTICLE_PIPELINE_CULL_MODE,
            PARTICLE_PIPELINE_DEPTH_WRITE_ENABLED,
        )?,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn create_selected_particle_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    label: &'static str,
    vertex_source: &str,
    fragment_source: &str,
    vertex_layout: wgpu::VertexBufferLayout<'_>,
    blend: Option<wgpu::BlendState>,
    cull_mode: Option<wgpu::Face>,
    depth_write_enabled: bool,
) -> Result<wgpu::RenderPipeline> {
    let (vertex, fragment) = crate::core_shaders::compile_selected_shader_stages(
        device,
        label,
        vertex_source,
        fragment_source,
    )?;
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-selected-particle-pipeline-layout"),
        bind_group_layouts: &[Some(bind_group_layout), Some(lightmap_bind_group_layout)],
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
                buffers: &[Some(vertex_layout)],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode,
                ..Default::default()
            },
            depth_stencil: Some(depth_stencil_state(
                DEPTH_FORMAT,
                depth_write_enabled,
                PARTICLE_PIPELINE_DEPTH_COMPARE,
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn create_particle_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::BufferBinding<'_>,
    width: u32,
    height: u32,
    rgba: &[u8],
    sprite_uvs: Vec<ParticleSpriteUv>,
) -> Result<ParticleAtlasGpu> {
    validate_rgba(width, height, rgba)?;
    let (texture, view) = upload_rgba_texture(
        device,
        queue,
        [width, height],
        rgba,
        "bbb-particle-atlas-texture",
    )?;
    let sampler = create_nearest_clamp_sampler(device, "bbb-particle-atlas-sampler");
    let bind_group = create_camera_texture_bind_group(
        device,
        layout,
        "bbb-particle-atlas-bind-group",
        camera_buffer,
        &view,
        &sampler,
    );

    Ok(ParticleAtlasGpu {
        texture,
        _view: view,
        _sampler: sampler,
        width,
        height,
        bind_group,
        sprite_uvs: sprite_uvs.into_iter().map(|sprite| sprite.uv).collect(),
    })
}

pub(crate) fn update_particle_atlas_gpu(
    queue: &wgpu::Queue,
    atlas: &ParticleAtlasGpu,
    rgba: &[u8],
) -> Result<()> {
    validate_rgba(atlas.width, atlas.height, rgba)?;
    write_texture_rgba(queue, &atlas.texture, 0, atlas.width, atlas.height, rgba);
    Ok(())
}

fn particle_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<ParticleVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &PARTICLE_VERTEX_ATTRIBUTES,
    }
}

#[inline]
fn f32_to_snorm16(value: f32) -> i16 {
    if !value.is_finite() {
        return 0;
    }
    (value.clamp(-1.0, 1.0) * 32767.0).round() as i16
}

#[inline]
fn snorm16_to_f32(value: i16) -> f32 {
    (f32::from(value) / 32767.0).clamp(-1.0, 1.0)
}

#[inline]
fn f32_to_f16(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exp = ((bits >> 23) & 0xff) as i32;
    let mantissa = bits & 0x007f_ffff;
    if exp == 0xff {
        let nan_payload = if mantissa != 0 { 0x0200 } else { 0 };
        return sign | 0x7c00 | nan_payload;
    }
    let unbiased = exp - 127 + 15;
    if unbiased >= 31 {
        return sign | 0x7c00;
    }
    if unbiased <= 0 {
        if unbiased < -10 {
            return sign;
        }
        let full_mantissa = mantissa | 0x0080_0000;
        let shift = (14 - unbiased) as u32;
        let half_bit = 1u32 << (shift - 1);
        let tie_adjust = ((full_mantissa >> shift) & 1) + half_bit - 1;
        return sign | (((full_mantissa + tie_adjust) >> shift) as u16);
    }
    let half_exp = (unbiased as u16) << 10;
    let tie_adjust = ((mantissa >> 13) & 1) + 0x0fff;
    let rounded = mantissa + tie_adjust;
    sign | (half_exp + ((rounded >> 13) as u16))
}

#[inline]
fn f16_to_f32(bits: u16) -> f32 {
    let sign = u32::from(bits & 0x8000) << 16;
    let exp = u32::from((bits >> 10) & 0x1f);
    let mantissa = u32::from(bits & 0x03ff);
    let f32_bits = if exp == 0 {
        if mantissa == 0 {
            sign
        } else {
            let shift = mantissa.leading_zeros() - 21;
            let norm_mantissa = (mantissa << shift) & 0x03ff;
            let f32_exp = (127 - 15 + 1 - shift) << 23;
            sign | f32_exp | (norm_mantissa << 13)
        }
    } else if exp == 0x1f {
        sign | 0x7f80_0000 | (mantissa << 13)
    } else {
        sign | ((exp + 127 - 15) << 23) | (mantissa << 13)
    };
    f32::from_bits(f32_bits)
}
