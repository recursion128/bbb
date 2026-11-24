use std::mem;
use std::sync::Arc;

use anyhow::Result;

use super::{HudGuiSpriteScaling, HudUvRect, HudVertex};
use crate::pipeline_builder::RenderPipelineBuilder;
use crate::{
    texture_sampling::{
        create_texture_bind_group, standalone_sampler_descriptor, upload_rgba_texture,
        validate_rgba,
    },
    StandaloneTextureSampling,
};

const HUD_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32x2];

const HUD_SHADER: &str = r#"
@group(0) @binding(0)
var hud_texture: texture_2d<f32>;
@group(0) @binding(1)
var hud_sampler: sampler;

struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tint: vec4<f32>,
    @location(3) local_uv: vec2<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(input.position, 0.0, 1.0);
    out.uv = input.uv;
    out.tint = input.tint;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(hud_texture, hud_sampler, input.uv) * input.tint;
    if texel.a <= 0.01 {
        discard;
    }
    return texel;
}
"#;

const HUD_ITEM_GLINT_SHADER: &str = concat!(
    crate::wgsl::camera_full!(),
    r#"
@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var item_glint_texture: texture_2d<f32>;

@group(0) @binding(2)
var item_glint_sampler: sampler;

@group(1) @binding(0)
var hud_item_mask_texture: texture_2d<f32>;

@group(1) @binding(1)
var hud_item_mask_sampler: sampler;

struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tint: vec4<f32>,
    @location(3) local_uv: vec2<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) mask_uv: vec2<f32>,
    @location(1) glint_uv: vec2<f32>,
    @location(2) mask_alpha: f32,
};

const GLINT_UV_SCALE: f32 = 8.0;
const GLINT_ANGLE: f32 = 0.1745329252;

fn glint_uv(local_uv: vec2<f32>) -> vec2<f32> {
    let scaled = local_uv * GLINT_UV_SCALE;
    let cos_angle = cos(GLINT_ANGLE);
    let sin_angle = sin(GLINT_ANGLE);
    let rotated = vec2<f32>(
        scaled.x * cos_angle - scaled.y * sin_angle,
        scaled.x * sin_angle + scaled.y * cos_angle,
    );
    return rotated + camera.glint_offsets.xy;
}

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(input.position, 0.0, 1.0);
    out.mask_uv = input.uv;
    out.glint_uv = glint_uv(input.local_uv);
    out.mask_alpha = input.tint.a;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let mask = textureSample(hud_item_mask_texture, hud_item_mask_sampler, input.mask_uv);
    if mask.a * input.mask_alpha <= 0.01 {
        discard;
    }
    let color = textureSample(item_glint_texture, item_glint_sampler, fract(input.glint_uv));
    if color.a < 0.1 {
        discard;
    }
    return vec4<f32>(color.rgb * camera.glint_offsets.z, color.a);
}
"#,
);

/// Vanilla `BlendFunction.GLINT`: source is multiplied by its own color and added to the destination;
/// destination alpha is preserved. HUD item icons are already flattened to the swapchain, so this variant
/// keeps the blend shape but has no depth attachment.
const HUD_ITEM_GLINT_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Src,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Zero,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

/// One GPU texture bound for the HUD pipelines; GUI atlas sprites share one.
pub(crate) struct HudTexture {
    pub(crate) texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

/// A drawable HUD image: a whole standalone texture or one GUI atlas sprite.
/// Draw UVs are sprite-local and map into `uv` of the shared texture.
#[derive(Clone)]
pub(crate) struct HudSpriteGpu {
    pub(crate) gpu: Arc<HudTexture>,
    pub(crate) uv: HudUvRect,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) scaling: HudGuiSpriteScaling,
}

pub(crate) const FULL_UV: HudUvRect = HudUvRect {
    min: [0.0, 0.0],
    max: [1.0, 1.0],
};

impl HudSpriteGpu {
    pub(crate) fn solid(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        rgba: [u8; 4],
    ) -> Result<Self> {
        create_hud_sprite_gpu(device, queue, layout, 1, 1, &rgba)
    }

    /// Maps sprite-local UVs into the backing texture.
    pub(crate) fn texture_uv(&self, uv: HudUvRect) -> HudUvRect {
        if self.uv == FULL_UV {
            return uv;
        }
        let span = [
            self.uv.max[0] - self.uv.min[0],
            self.uv.max[1] - self.uv.min[1],
        ];
        HudUvRect {
            min: [
                self.uv.min[0] + span[0] * uv.min[0],
                self.uv.min[1] + span[1] * uv.min[1],
            ],
            max: [
                self.uv.min[0] + span[0] * uv.max[0],
                self.uv.min[1] + span[1] * uv.max[1],
            ],
        }
    }

    pub(crate) fn same_texture(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.gpu, &other.gpu)
    }
}

pub(crate) struct HudGuiPipelines {
    pub(crate) fill: wgpu::RenderPipeline,
    pub(crate) invert: wgpu::RenderPipeline,
    pub(crate) additive: wgpu::RenderPipeline,
    pub(crate) bind_group: wgpu::BindGroup,
    size_uniform_buffer: wgpu::Buffer,
}

impl HudGuiPipelines {
    pub(crate) fn update_gui_extent(&self, queue: &wgpu::Queue, extent: [f32; 2]) {
        queue.write_buffer(
            &self.size_uniform_buffer,
            0,
            bytemuck::cast_slice(&[extent[0], extent[1], 0.0, 0.0]),
        );
    }
}

const HUD_GUI_SHADER: &str = r#"
struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) tint: vec4<f32>,
};
@vertex
fn vs_main(@location(0) position: vec2<f32>, @location(2) tint: vec4<f32>) -> VertexOut {
    return VertexOut(vec4<f32>(position, 0.0, 1.0), tint);
}
@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    if input.tint.a == 0.0 { discard; }
    return input.tint;
}
"#;

pub(crate) fn create_hud_gui_pipelines(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    extent: [f32; 2],
    shaders: Option<&crate::core_shaders::AdaptedShaderPair>,
) -> Result<HudGuiPipelines> {
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-hud-gui-layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(16),
            },
            count: None,
        }],
    });
    let size_uniform_buffer = crate::frame_buffers::create_initialized_buffer(
        device,
        Some("bbb-hud-gui-size"),
        bytemuck::cast_slice(&[extent[0], extent[1], 0.0, 0.0]),
        wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    );
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-hud-gui-size"),
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: size_uniform_buffer.as_entire_binding(),
        }],
    });
    // Vanilla BlendFunction.java:20-22 inverts destination RGB independently of alpha.
    let invert_blend = wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::OneMinusDst,
            dst_factor: wgpu::BlendFactor::OneMinusSrc,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent::REPLACE,
    };
    let selected = shaders
        .map(|shaders| {
            crate::core_shaders::compile_selected_shader_stages(
                device,
                "bbb-selected-hud-gui",
                &shaders.vertex,
                &shaders.fragment,
            )
        })
        .transpose()?;
    let build = |label, blend| {
        if let Some((vertex, fragment)) = &selected {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: vertex,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(hud_vertex_layout())],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: fragment,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        } else {
            RenderPipelineBuilder::new(device, label)
                .shader("bbb-hud-gui-shader", HUD_GUI_SHADER)
                .layout("bbb-hud-gui-layout", &[&bind_group_layout])
                .vertex_buffers(&[hud_vertex_layout()])
                .color_target(format, Some(blend))
                .build()
        }
    };
    Ok(HudGuiPipelines {
        fill: build("bbb-hud-gui-fill", wgpu::BlendState::ALPHA_BLENDING),
        invert: build("bbb-hud-text-invert", invert_blend),
        additive: build("bbb-hud-text-additive", HUD_NAUSEA_OVERLAY_BLEND),
        bind_group,
        size_uniform_buffer,
    })
}

pub(crate) fn create_hud_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-hud-bind-group-layout"),
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
        ],
    })
}

pub(crate) fn create_hud_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedPositionTexColorShaders>,
    blend: wgpu::BlendState,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::core_shaders::create_selected_position_tex_color_pipeline(
            device,
            format,
            bind_group_layout,
            "bbb-selected-hud",
            &shaders.hud,
            &[hud_vertex_layout()],
            Some(blend),
        );
    }
    Ok(RenderPipelineBuilder::new(device, "bbb-hud-pipeline")
        .shader("bbb-hud-shader", HUD_SHADER)
        .layout("bbb-hud-pipeline-layout", &[bind_group_layout])
        .vertex_buffers(&[hud_vertex_layout()])
        .color_target(format, Some(blend))
        .build())
}

// `RenderPipelines.java:688-690`, `BlendFunction.java:20-22`.
pub(crate) const HUD_CROSSHAIR_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::OneMinusDst,
        dst_factor: wgpu::BlendFactor::OneMinusSrc,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent::REPLACE,
};

/// Vanilla `RenderPipelines.GUI_NAUSEA_OVERLAY`
/// (`RenderPipelines.java:676-681`): the ordinary GUI textured pipeline with
/// `BlendFunction.ADDITIVE` (`ONE, ONE` for both colour and alpha).
pub(crate) const HUD_NAUSEA_OVERLAY_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

pub(crate) fn create_hud_nausea_overlay_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedPositionTexColorShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::core_shaders::create_selected_position_tex_color_pipeline(
            device,
            format,
            bind_group_layout,
            "bbb-selected-hud-nausea-overlay",
            &shaders.hud,
            &[hud_vertex_layout()],
            Some(HUD_NAUSEA_OVERLAY_BLEND),
        );
    }
    Ok(
        RenderPipelineBuilder::new(device, "bbb-hud-nausea-overlay-pipeline")
            .shader("bbb-hud-nausea-overlay-shader", HUD_SHADER)
            .layout(
                "bbb-hud-nausea-overlay-pipeline-layout",
                &[bind_group_layout],
            )
            .vertex_buffers(&[hud_vertex_layout()])
            .color_target(format, Some(HUD_NAUSEA_OVERLAY_BLEND))
            .build(),
    )
}

pub(crate) fn create_hud_item_glint_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    glint_bind_group_layout: &wgpu::BindGroupLayout,
    item_mask_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedGlintShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::core_shaders::create_selected_surface_pipeline(
            device,
            format,
            glint_bind_group_layout,
            Some(item_mask_bind_group_layout),
            "bbb-selected-hud-item-glint",
            &shaders.hud_item_vertex,
            &shaders.hud_item_fragment,
            &[],
            &[hud_vertex_layout()],
            Some(HUD_ITEM_GLINT_BLEND),
            wgpu::ColorWrites::ALL,
            None,
            None,
        );
    }
    Ok(
        RenderPipelineBuilder::new(device, "bbb-hud-item-glint-pipeline")
            .shader("bbb-hud-item-glint-shader", HUD_ITEM_GLINT_SHADER)
            .layout(
                "bbb-hud-item-glint-pipeline-layout",
                &[glint_bind_group_layout, item_mask_bind_group_layout],
            )
            .vertex_buffers(&[hud_vertex_layout()])
            .color_target(format, Some(HUD_ITEM_GLINT_BLEND))
            .build(),
    )
}

pub(crate) fn create_hud_sprite_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<HudSpriteGpu> {
    create_hud_sprite_gpu_with_sampling(
        device,
        queue,
        layout,
        width,
        height,
        rgba,
        StandaloneTextureSampling::new(false, true),
    )
}

pub(crate) fn create_standalone_hud_sprite_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    width: u32,
    height: u32,
    rgba: &[u8],
    sampling: StandaloneTextureSampling,
) -> Result<HudSpriteGpu> {
    create_hud_sprite_gpu_with_sampling(device, queue, layout, width, height, rgba, sampling)
}

fn create_hud_sprite_gpu_with_sampling(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    width: u32,
    height: u32,
    rgba: &[u8],
    sampling: StandaloneTextureSampling,
) -> Result<HudSpriteGpu> {
    validate_rgba(width, height, rgba)?;

    let (texture, view) = upload_rgba_texture(
        device,
        queue,
        [width, height],
        rgba,
        "bbb-hud-sprite-texture",
    )?;
    let sampler = device.create_sampler(&standalone_sampler_descriptor(
        "bbb-hud-sprite-sampler",
        sampling,
    ));
    let bind_group =
        create_texture_bind_group(device, layout, "bbb-hud-sprite-bind-group", &view, &sampler);

    Ok(HudSpriteGpu {
        gpu: Arc::new(HudTexture {
            texture,
            view,
            _sampler: sampler,
            bind_group,
            width,
            height,
        }),
        uv: FULL_UV,
        width,
        height,
        scaling: HudGuiSpriteScaling::Stretch,
    })
}

pub(crate) fn create_dynamic_hud_font_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    snapshot: &bbb_render_types::HudDynamicAtlasSnapshot,
) -> Result<HudSpriteGpu> {
    validate_rgba(
        snapshot.static_width,
        snapshot.static_height,
        &snapshot.static_rgba,
    )?;
    let width = snapshot.width.max(1);
    let height = snapshot.height.max(1);
    let max_dim = device.limits().max_texture_dimension_2d;
    if width > max_dim || height > max_dim {
        anyhow::bail!(
            "dynamic HUD font atlas dimensions {width}x{height} exceed device limit {max_dim}"
        );
    }
    let atlas_bytes = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow::anyhow!("dynamic HUD font atlas byte size overflow"))?;
    if atlas_bytes > crate::item_model::font::MAX_DYNAMIC_FONT_ATLAS_BYTES {
        anyhow::bail!(
            "dynamic HUD font atlas size {atlas_bytes} bytes exceeds {} byte budget",
            crate::item_model::font::MAX_DYNAMIC_FONT_ATLAS_BYTES
        );
    }
    if snapshot.static_width > width || snapshot.static_height > height {
        anyhow::bail!(
            "dynamic HUD font atlas static region {}x{} exceeds atlas dimensions {width}x{height}",
            snapshot.static_width,
            snapshot.static_height
        );
    }
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    #[cfg(not(target_arch = "wasm32"))]
    let usage = wgpu::TextureUsages::TEXTURE_BINDING
        | wgpu::TextureUsages::COPY_DST
        | wgpu::TextureUsages::COPY_SRC;
    #[cfg(target_arch = "wasm32")]
    let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-hud-dynamic-font-atlas"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage,
        view_formats: &[],
    });
    if snapshot.static_width > 0 && snapshot.static_height > 0 {
        write_hud_texture_subrect(
            queue,
            &texture,
            0,
            0,
            snapshot.static_width,
            snapshot.static_height,
            &snapshot.static_rgba,
        );
    }
    for glyph in &snapshot.glyphs {
        if glyph.width > 0
            && glyph.height > 0
            && !glyph.rgba.is_empty()
            && glyph.x.saturating_add(glyph.width) <= width
            && glyph.y.saturating_add(glyph.height) <= height
        {
            write_hud_texture_subrect(
                queue,
                &texture,
                glyph.x,
                glyph.y,
                glyph.width,
                glyph.height,
                &glyph.rgba,
            );
        }
    }
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&standalone_sampler_descriptor(
        "bbb-hud-dynamic-font-atlas-sampler",
        StandaloneTextureSampling::new(false, true),
    ));
    let bind_group = create_texture_bind_group(
        device,
        layout,
        "bbb-hud-dynamic-font-atlas-bind-group",
        &view,
        &sampler,
    );
    Ok(HudSpriteGpu {
        gpu: Arc::new(HudTexture {
            texture,
            view,
            _sampler: sampler,
            bind_group,
            width: snapshot.width,
            height: snapshot.height,
        }),
        uv: FULL_UV,
        width: snapshot.width,
        height: snapshot.height,
        scaling: HudGuiSpriteScaling::Stretch,
    })
}

pub(crate) fn write_hud_texture_subrect(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    rgba: &[u8],
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x, y, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::TexelCopyBufferLayout {
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

fn hud_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<HudVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &HUD_VERTEX_ATTRIBUTES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_item_glint_pipeline_state_matches_vanilla_glint_blend_without_depth() {
        assert_eq!(
            HUD_ITEM_GLINT_BLEND.color.src_factor,
            wgpu::BlendFactor::Src
        );
        assert_eq!(
            HUD_ITEM_GLINT_BLEND.color.dst_factor,
            wgpu::BlendFactor::One
        );
        assert_eq!(
            HUD_ITEM_GLINT_BLEND.alpha.src_factor,
            wgpu::BlendFactor::Zero
        );
        assert_eq!(
            HUD_ITEM_GLINT_BLEND.alpha.dst_factor,
            wgpu::BlendFactor::One
        );
    }

    #[test]
    fn hud_item_glint_shader_uses_alpha_mask_and_vanilla_glint_texturing() {
        assert!(HUD_ITEM_GLINT_SHADER.contains("const GLINT_UV_SCALE: f32 = 8.0"));
        assert!(HUD_ITEM_GLINT_SHADER.contains("* camera.glint_offsets.z"));
        assert!(HUD_ITEM_GLINT_SHADER.contains("const GLINT_ANGLE: f32 = 0.1745329252"));
        assert!(HUD_ITEM_GLINT_SHADER.contains("glint_offsets: vec4<f32>"));
        assert!(HUD_ITEM_GLINT_SHADER.contains("rotated + camera.glint_offsets.xy"));
        assert!(HUD_ITEM_GLINT_SHADER.contains("textureSample(hud_item_mask_texture"));
        assert!(HUD_ITEM_GLINT_SHADER.contains("mask.a * input.mask_alpha <= 0.01"));
        assert!(HUD_ITEM_GLINT_SHADER.contains("textureSample(item_glint_texture"));
        assert!(HUD_ITEM_GLINT_SHADER.contains("fract(input.glint_uv)"));
    }

    #[test]
    fn item_atlas_update_validation_requires_the_original_rgba_extent() {
        assert!(validate_rgba(2, 1, &[0; 8]).is_ok());
        assert!(validate_rgba(2, 1, &[0; 4]).is_err());
    }
}
