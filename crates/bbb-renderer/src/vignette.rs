//! Camera-overlay vignette state.
//!
//! The GPU owner lives here rather than in the general HUD command stream
//! because vanilla's `RenderPipelines.VIGNETTE` uses a destination-multiplying
//! blend that cannot share the ordinary alpha-blended HUD pipeline.

use anyhow::{anyhow, bail, Result};
use bytemuck::{Pod, Zeroable};

use crate::{
    pipeline_builder::RenderPipelineBuilder, texture_sampling::standalone_sampler_descriptor,
    Renderer, StandaloneTextureSampling,
};

/// Vanilla `Gui.VIGNETTE_LOCATION`.
pub const VIGNETTE_TEXTURE_PATH: &str = "textures/misc/vignette.png";

pub(crate) const VIGNETTE_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Zero,
        dst_factor: wgpu::BlendFactor::OneMinusSrc,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Zero,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

pub(crate) const VIGNETTE_SHADER: &str = r#"
@group(0) @binding(0)
var vignette_texture: texture_2d<f32>;

@group(0) @binding(1)
var vignette_sampler: sampler;

struct VignetteUniform {
    color: vec4<f32>,
};

@group(0) @binding(2)
var<uniform> vignette: VignetteUniform;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var position = vec2<f32>(-1.0, -1.0);
    if (vertex_index == 1u) {
        position = vec2<f32>(3.0, -1.0);
    } else if (vertex_index == 2u) {
        position = vec2<f32>(-1.0, 3.0);
    }
    var out: VertexOut;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.uv = vec2<f32>(position.x * 0.5 + 0.5, 0.5 - position.y * 0.5);
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(vignette_texture, vignette_sampler, input.uv) * vignette.color;
}
"#;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(crate) struct VignetteUniform {
    pub(crate) color: [f32; 4],
}

pub(crate) struct VignetteTextureGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) uniform_buffer: wgpu::Buffer,
    pub(crate) bind_group: wgpu::BindGroup,
}

/// One frame's vanilla `Gui.extractVignette` projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudVignette {
    /// Smoothed `Gui.vignetteBrightness`, clamped to `[0, 1]`.
    pub brightness: f32,
    /// World-border warning strength after the vanilla clamp.
    pub border_warning_strength: f32,
    /// Opaque RGB color multiplied into `textures/misc/vignette.png`.
    pub color: [f32; 3],
}

impl HudVignette {
    fn sanitized(self) -> Option<Self> {
        if !self.brightness.is_finite()
            || !self.border_warning_strength.is_finite()
            || !self.color.iter().all(|channel| channel.is_finite())
        {
            return None;
        }
        Some(Self {
            brightness: self.brightness.clamp(0.0, 1.0),
            border_warning_strength: self.border_warning_strength.clamp(0.0, 1.0),
            color: self.color.map(|channel| channel.clamp(0.0, 1.0)),
        })
    }
}

impl Renderer {
    /// Replaces the latest camera-overlay vignette projection.
    pub fn set_hud_vignette(&mut self, vignette: Option<HudVignette>) {
        self.hud_vignette = vignette.and_then(HudVignette::sanitized);
    }

    /// Returns the sanitized vignette state currently owned by the renderer.
    pub fn hud_vignette(&self) -> Option<HudVignette> {
        self.hud_vignette
    }

    /// Uploads the selected pack generation's
    /// `textures/misc/vignette.png`.
    pub fn upload_hud_vignette_texture(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
        blur: bool,
        clamp: bool,
    ) -> Result<()> {
        self.upload_hud_vignette_texture_with_sampling(
            width,
            height,
            rgba,
            StandaloneTextureSampling::new(blur, clamp),
        )
    }

    /// Uploads the selected pack generation's
    /// `textures/misc/vignette.png` with its decoded standalone sampling state.
    pub fn upload_hud_vignette_texture_with_sampling(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
        sampling: StandaloneTextureSampling,
    ) -> Result<()> {
        self.hud_vignette_texture = Some(create_vignette_texture_gpu(
            &self.device,
            &self.queue,
            &self.vignette_bind_group_layout,
            width,
            height,
            rgba,
            sampling,
        )?);
        Ok(())
    }

    /// Uploads the selected pack generation's
    /// `textures/misc/credits_vignette.png`.
    pub fn upload_hud_win_credits_vignette(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
        blur: bool,
        clamp: bool,
    ) -> Result<()> {
        self.upload_hud_win_credits_vignette_with_sampling(
            width,
            height,
            rgba,
            StandaloneTextureSampling::new(blur, clamp),
        )
    }

    /// Uploads the selected pack generation's
    /// `textures/misc/credits_vignette.png` with its decoded standalone sampling state.
    pub fn upload_hud_win_credits_vignette_with_sampling(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
        sampling: StandaloneTextureSampling,
    ) -> Result<()> {
        self.hud_win_credits_vignette = Some(create_vignette_texture_gpu(
            &self.device,
            &self.queue,
            &self.vignette_bind_group_layout,
            width,
            height,
            rgba,
            sampling,
        )?);
        Ok(())
    }
}

pub(crate) fn create_vignette_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-vignette-bind-group-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
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

pub(crate) fn create_vignette_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, "bbb-vignette-pipeline")
        .shader("bbb-vignette-shader", VIGNETTE_SHADER)
        .layout("bbb-vignette-pipeline-layout", &[bind_group_layout])
        .color_target(format, Some(VIGNETTE_BLEND))
        .build()
}

fn create_vignette_texture_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    width: u32,
    height: u32,
    rgba: &[u8],
    sampling: StandaloneTextureSampling,
) -> Result<VignetteTextureGpu> {
    if width == 0 || height == 0 {
        bail!("vignette texture must have non-zero dimensions");
    }
    let expected_len = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("vignette texture size overflow"))?;
    if rgba.len() != expected_len {
        bail!(
            "vignette texture rgba length {} does not match {}x{}",
            rgba.len(),
            width,
            height
        );
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-vignette-texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let bytes_per_row = width
        .checked_mul(4)
        .ok_or_else(|| anyhow!("vignette texture row size overflow"))?;
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
            bytes_per_row: Some(bytes_per_row),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&standalone_sampler_descriptor(
        "bbb-vignette-sampler",
        sampling,
    ));
    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bbb-vignette-uniform"),
        size: std::mem::size_of::<VignetteUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-vignette-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    });
    Ok(VignetteTextureGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        uniform_buffer,
        bind_group,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vignette_projection_sanitizes_ranges_and_rejects_non_finite_values() {
        assert_eq!(
            HudVignette {
                brightness: 2.0,
                border_warning_strength: -1.0,
                color: [-0.5, 0.5, 1.5],
            }
            .sanitized(),
            Some(HudVignette {
                brightness: 1.0,
                border_warning_strength: 0.0,
                color: [0.0, 0.5, 1.0],
            })
        );
        assert_eq!(
            HudVignette {
                brightness: f32::NAN,
                border_warning_strength: 0.0,
                color: [1.0; 3],
            }
            .sanitized(),
            None
        );
    }

    #[test]
    fn vignette_pipeline_matches_vanilla_blend_and_blurred_texture_sampling() {
        assert_eq!(VIGNETTE_BLEND.color.src_factor, wgpu::BlendFactor::Zero);
        assert_eq!(
            VIGNETTE_BLEND.color.dst_factor,
            wgpu::BlendFactor::OneMinusSrc
        );
        assert_eq!(VIGNETTE_BLEND.alpha.src_factor, wgpu::BlendFactor::Zero);
        assert_eq!(VIGNETTE_BLEND.alpha.dst_factor, wgpu::BlendFactor::One);
        assert!(VIGNETTE_SHADER.contains("textureSample(vignette_texture, vignette_sampler"));
        assert!(VIGNETTE_SHADER.contains("* vignette.color"));
    }
}
