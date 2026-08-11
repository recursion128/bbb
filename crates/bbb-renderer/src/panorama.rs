use anyhow::{bail, Context, Result};
use glam::Mat4;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

use crate::{
    hud::{create_standalone_hud_sprite_gpu, HudSpriteGpu},
    pipeline_builder::RenderPipelineBuilder,
    texture_sampling::standalone_sampler_descriptor,
    Renderer, StandaloneTextureSampling,
};

const PANORAMA_FOV_DEGREES: f32 = 85.0;
const PANORAMA_X_ROTATION_DEGREES: f32 = 10.0;

const PANORAMA_SHADER: &str = r#"
struct PanoramaUniform {
    inverse_view_rotation: mat4x4<f32>,
    ray_scale: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> panorama: PanoramaUniform;
@group(0) @binding(1)
var panorama_texture: texture_cube<f32>;
@group(0) @binding(2)
var panorama_sampler: sampler;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) view_ray: vec3<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var position = vec2<f32>(-1.0, -1.0);
    if vertex_index == 1u {
        position = vec2<f32>(3.0, -1.0);
    } else if vertex_index == 2u {
        position = vec2<f32>(-1.0, 3.0);
    }
    var out: VertexOut;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.view_ray = vec3<f32>(
        position.x * panorama.ray_scale.x,
        position.y * panorama.ray_scale.y,
        1.0,
    );
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let direction = (panorama.inverse_view_rotation * vec4<f32>(input.view_ray, 0.0)).xyz;
    return textureSample(panorama_texture, panorama_sampler, direction);
}
"#;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PanoramaUniform {
    inverse_view_rotation: [[f32; 4]; 4],
    ray_scale: [f32; 4],
}

pub(super) struct PanoramaGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    overlay: HudSpriteGpu,
    resident_bytes: u64,
}

impl PanoramaGpu {
    #[allow(clippy::too_many_arguments)]
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target_format: wgpu::TextureFormat,
        hud_bind_group_layout: &wgpu::BindGroupLayout,
        face_width: u32,
        face_height: u32,
        faces: [&[u8]; 6],
        overlay_width: u32,
        overlay_height: u32,
        overlay_rgba: &[u8],
        overlay_sampling: StandaloneTextureSampling,
    ) -> Result<Self> {
        validate_faces(device, face_width, face_height, faces)?;
        let overlay = create_standalone_hud_sprite_gpu(
            device,
            queue,
            hud_bind_group_layout,
            overlay_width,
            overlay_height,
            overlay_rgba,
            overlay_sampling,
        )
        .context("create panorama overlay GPU texture")?;

        let layout = create_bind_group_layout(device);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bbb-panorama-cubemap"),
            size: wgpu::Extent3d {
                width: face_width,
                height: face_height,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (layer, rgba) in faces.into_iter().enumerate() {
            // Vanilla flips each PNG while packing `CubeMapTexture` because its
            // OpenGL cube upload addresses the first row from the opposite
            // texture origin. WGPU's copy and sampling convention already
            // preserves decoded top-to-bottom rows, so flipping here would
            // invert every displayed face. The asymmetric offscreen test owns
            // this backend compensation rather than a source-string guard.
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: u32::try_from(layer).expect("six panorama layers fit u32"),
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(face_width * 4),
                    rows_per_image: Some(face_height),
                },
                wgpu::Extent3d {
                    width: face_width,
                    height: face_height,
                    depth_or_array_layers: 1,
                },
            );
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("bbb-panorama-cubemap-view"),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            usage: None,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(6),
            aspect: wgpu::TextureAspect::All,
            format: None,
        });
        let sampler = device.create_sampler(&standalone_sampler_descriptor(
            "bbb-panorama-cubemap-sampler",
            StandaloneTextureSampling::new(true, false),
        ));
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bbb-panorama-uniform"),
            contents: bytemuck::bytes_of(&panorama_uniform(PhysicalSize::new(1, 1), 0.0)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bbb-panorama-bind-group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
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
        let pipeline = RenderPipelineBuilder::new(device, "bbb-panorama-pipeline")
            .shader("bbb-panorama-shader", PANORAMA_SHADER)
            .layout("bbb-panorama-pipeline-layout", &[&layout])
            .color_target(target_format, None)
            .build();
        let overlay_bytes = image_bytes(overlay_width, overlay_height)?;
        let resident_bytes = image_bytes(face_width, face_height)?
            .checked_mul(6)
            .and_then(|bytes| bytes.checked_add(overlay_bytes))
            .context("panorama GPU resident byte count overflow")?;

        Ok(Self {
            _texture: texture,
            _view: view,
            _sampler: sampler,
            bind_group,
            uniform_buffer,
            pipeline,
            overlay,
            resident_bytes,
        })
    }

    pub(super) fn overlay(&self) -> &HudSpriteGpu {
        &self.overlay
    }

    pub(super) fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }

    pub(super) fn draw(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        surface_size: PhysicalSize<u32>,
        rotation_degrees: f32,
    ) {
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&panorama_uniform(surface_size, rotation_degrees)),
        );
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("bbb-panorama-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

impl Renderer {
    #[allow(clippy::too_many_arguments)]
    pub fn upload_hud_panorama(
        &mut self,
        face_width: u32,
        face_height: u32,
        faces: [&[u8]; 6],
        overlay_width: u32,
        overlay_height: u32,
        overlay_rgba: &[u8],
        overlay_sampling: StandaloneTextureSampling,
    ) -> Result<()> {
        let next = PanoramaGpu::new(
            &self.device,
            &self.queue,
            self.config.format,
            &self.hud_bind_group_layout,
            face_width,
            face_height,
            faces,
            overlay_width,
            overlay_height,
            overlay_rgba,
            overlay_sampling,
        )?;
        self.counters.panorama_resource_ready = true;
        self.counters.panorama_resident_bytes = next.resident_bytes();
        self.hud_panorama = Some(next);
        Ok(())
    }

    pub(super) fn sync_panorama_resource_counters(&mut self) {
        self.counters.panorama_resource_ready = self.hud_panorama.is_some();
        self.counters.panorama_resident_bytes = self
            .hud_panorama
            .as_ref()
            .map_or(0, PanoramaGpu::resident_bytes);
    }
}

fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-panorama-bind-group-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::Cube,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
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

fn validate_faces(device: &wgpu::Device, width: u32, height: u32, faces: [&[u8]; 6]) -> Result<()> {
    if width == 0 || height == 0 {
        bail!("panorama cubemap dimensions must be non-zero");
    }
    if width != height {
        bail!("panorama cubemap faces must be square, got {width}x{height}");
    }
    if width > device.limits().max_texture_dimension_2d {
        bail!(
            "panorama cubemap dimension {width} exceeds device limit {}",
            device.limits().max_texture_dimension_2d
        );
    }
    if device.limits().max_texture_array_layers < 6 {
        bail!(
            "panorama cubemap requires 6 array layers, device supports {}",
            device.limits().max_texture_array_layers
        );
    }
    let expected = usize::try_from(image_bytes(width, height)?)
        .context("panorama face byte count does not fit usize")?;
    for (layer, rgba) in faces.into_iter().enumerate() {
        if rgba.len() != expected {
            bail!(
                "panorama cubemap layer {layer} has {} RGBA bytes, expected {expected} for {width}x{height}",
                rgba.len()
            );
        }
    }
    Ok(())
}

fn image_bytes(width: u32, height: u32) -> Result<u64> {
    u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .context("panorama image byte count overflow")
}

fn panorama_uniform(surface_size: PhysicalSize<u32>, rotation_degrees: f32) -> PanoramaUniform {
    let width = surface_size.width.max(1) as f32;
    let height = surface_size.height.max(1) as f32;
    let half_fov_tangent = (PANORAMA_FOV_DEGREES.to_radians() * 0.5).tan();
    let rotation_degrees = if rotation_degrees.is_finite() {
        rotation_degrees
    } else {
        0.0
    };
    let view_rotation = Mat4::from_rotation_x(PANORAMA_X_ROTATION_DEGREES.to_radians())
        * Mat4::from_rotation_y(rotation_degrees.to_radians());
    PanoramaUniform {
        inverse_view_rotation: view_rotation.inverse().to_cols_array_2d(),
        ray_scale: [
            half_fov_tangent * width / height,
            half_fov_tangent,
            0.0,
            0.0,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_preserves_vanilla_fov_aspect_and_rotation_order() {
        let uniform = panorama_uniform(PhysicalSize::new(1_600, 900), -90.0);
        let expected_tangent = (85.0_f32.to_radians() * 0.5).tan();
        assert!((uniform.ray_scale[0] - expected_tangent * 16.0 / 9.0).abs() < 1.0e-6);
        assert!((uniform.ray_scale[1] - expected_tangent).abs() < 1.0e-6);
        let actual = Mat4::from_cols_array_2d(&uniform.inverse_view_rotation);
        let expected = (Mat4::from_rotation_x(10.0_f32.to_radians())
            * Mat4::from_rotation_y(-90.0_f32.to_radians()))
        .inverse();
        assert!(actual.abs_diff_eq(expected, 1.0e-6));
        assert!(actual.transform_vector3(glam::Vec3::Z).is_finite());
    }

    #[test]
    fn shader_uses_one_cube_texture_and_fullscreen_perspective_ray() {
        assert!(PANORAMA_SHADER.contains("texture_cube<f32>"));
        assert!(PANORAMA_SHADER.contains("position.x * panorama.ray_scale.x"));
        assert!(PANORAMA_SHADER.contains("panorama.inverse_view_rotation"));
        assert!(PANORAMA_SHADER.contains("textureSample(panorama_texture"));
    }
}
