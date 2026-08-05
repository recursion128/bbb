//! Vanilla `ScreenEffectRenderer` first-person overlays: the view-blocking
//! block, underwater and on-fire quads drawn between the first-person items
//! and the camera post effect. `GameRenderer.renderLevel` renders them under
//! the plain 3D-HUD perspective (`hudProjection.setupPerspective(0.05, 100,
//! hudFov, ...)`) with an identity pose stack, so the quads are view-space
//! geometry projected here on the CPU and rasterized by a pass-through
//! pipeline; `RenderPipelines.BLOCK_SCREEN_EFFECT`/`FIRE_SCREEN_EFFECT` are
//! plain `GUI_TEXTURED_SNIPPET` builds: translucent blend, no depth state.

use anyhow::{anyhow, bail, Result};
use glam::{Mat4, Vec4};

use crate::pipeline_builder::RenderPipelineBuilder;
use crate::texture_sampling::standalone_sampler_descriptor;
use bbb_render_types::StandaloneTextureSampling;

/// Vanilla `GameRenderer` 3D-HUD projection depth range
/// (`hudProjection.setupPerspective(0.05F, PROJECTION_3D_HUD_Z_FAR, ...)`).
const SCREEN_EFFECT_Z_NEAR: f32 = 0.05;
const SCREEN_EFFECT_Z_FAR: f32 = 100.0;

/// One vanilla screen-effect quad in view space (camera at the origin looking
/// down -Z), with resolved texture UVs and the vanilla vertex color.
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenEffectQuad {
    /// View-space corners in vanilla submission order (the two triangles are
    /// expanded as 0-1-2 and 0-2-3).
    pub positions: [[f32; 3]; 4],
    pub uvs: [[f32; 2]; 4],
    pub color: [f32; 4],
    pub texture: ScreenEffectTexture,
}

/// Texture source for a screen-effect quad: the block and fire overlays sample
/// the terrain atlas; the underwater overlay samples the standalone
/// `textures/misc/underwater.png`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenEffectTexture {
    TerrainAtlas,
    Underwater,
}

pub(crate) struct ScreenEffectDrawBatch {
    pub(crate) texture: ScreenEffectTexture,
    pub(crate) first_vertex: u32,
    pub(crate) vertex_count: u32,
}

pub(crate) const SCREEN_EFFECT_VERTEX_FLOATS: usize = 10;

pub(crate) const SCREEN_EFFECT_SHADER: &str = r#"
struct VertexInput {
    @location(0) clip_position: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = input.clip_position;
    output.uv = input.uv;
    output.color = input.color;
    return output;
}

@group(0) @binding(0)
var effect_texture: texture_2d<f32>;
@group(0) @binding(1)
var effect_sampler: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(effect_texture, effect_sampler, input.uv) * input.color;
}
"#;

/// Vanilla `BlendFunction.TRANSLUCENT`: `srcAlpha/oneMinusSrcAlpha` color and
/// `one/oneMinusSrcAlpha` alpha.
const SCREEN_EFFECT_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
};

const SCREEN_EFFECT_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] = [
    wgpu::VertexAttribute {
        offset: 0,
        shader_location: 0,
        format: wgpu::VertexFormat::Float32x4,
    },
    wgpu::VertexAttribute {
        offset: 16,
        shader_location: 1,
        format: wgpu::VertexFormat::Float32x2,
    },
    wgpu::VertexAttribute {
        offset: 24,
        shader_location: 2,
        format: wgpu::VertexFormat::Float32x4,
    },
];

fn screen_effect_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: (SCREEN_EFFECT_VERTEX_FLOATS * std::mem::size_of::<f32>())
            as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &SCREEN_EFFECT_VERTEX_ATTRIBUTES,
    }
}

pub(crate) fn create_screen_effect_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    texture_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedPositionTexColorShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::core_shaders::create_selected_position_tex_color_pipeline(
            device,
            format,
            texture_bind_group_layout,
            "bbb-selected-screen-effect",
            &shaders.screen_effect,
            &[screen_effect_vertex_layout()],
            Some(SCREEN_EFFECT_BLEND),
        );
    }
    Ok(RenderPipelineBuilder::new(device, "bbb-screen-effect-pipeline")
        .shader("bbb-screen-effect-shader", SCREEN_EFFECT_SHADER)
        .layout("bbb-screen-effect-pipeline-layout", &[texture_bind_group_layout])
        .vertex_buffers(&[screen_effect_vertex_layout()])
        .color_target(format, Some(SCREEN_EFFECT_BLEND))
        .cull_mode(None)
        .build())
}

/// Projects the view-space quads through the vanilla 3D-HUD perspective and
/// expands them into triangle-list vertices, batching consecutive quads that
/// share a texture so the pass preserves the vanilla block -> water -> fire
/// draw order with one draw per texture run.
pub(crate) fn screen_effect_draw_data(
    quads: &[ScreenEffectQuad],
    hud_fov_degrees: f32,
    aspect: f32,
) -> (Vec<f32>, Vec<ScreenEffectDrawBatch>) {
    let projection = Mat4::perspective_rh(
        hud_fov_degrees.clamp(0.1, 179.0).to_radians(),
        aspect.max(0.1),
        SCREEN_EFFECT_Z_NEAR,
        SCREEN_EFFECT_Z_FAR,
    );
    let mut vertices = Vec::with_capacity(quads.len() * 6 * SCREEN_EFFECT_VERTEX_FLOATS);
    let mut batches: Vec<ScreenEffectDrawBatch> = Vec::new();
    for quad in quads {
        let clip: Vec<Vec4> = quad
            .positions
            .iter()
            .map(|position| {
                projection
                    * Vec4::new(position[0], position[1], position[2], 1.0)
            })
            .collect();
        let mut push_corner = |corner: usize| {
            vertices.extend_from_slice(&clip[corner].to_array());
            vertices.extend_from_slice(&quad.uvs[corner]);
            vertices.extend_from_slice(&quad.color);
        };
        for corner in [0, 1, 2, 0, 2, 3] {
            push_corner(corner);
        }
        match batches.last_mut() {
            Some(batch) if batch.texture == quad.texture => batch.vertex_count += 6,
            _ => {
                let first_vertex = (vertices.len() / SCREEN_EFFECT_VERTEX_FLOATS - 6) as u32;
                batches.push(ScreenEffectDrawBatch {
                    texture: quad.texture,
                    first_vertex,
                    vertex_count: 6,
                });
            }
        }
    }
    (vertices, batches)
}

pub(crate) struct ScreenEffectUnderwaterGpu {
    pub(crate) bind_group: wgpu::BindGroup,
}

/// Uploads `textures/misc/underwater.png`. The texture carries no `.mcmeta`
/// sampling metadata, so the vanilla defaults apply: nearest filtering with
/// repeat addressing (the water overlay tiles its UVs across `0..4`).
pub(crate) fn create_screen_effect_underwater_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<ScreenEffectUnderwaterGpu> {
    if width == 0 || height == 0 {
        bail!("underwater texture must have non-zero dimensions");
    }
    let expected_len = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("underwater texture size overflow"))?;
    if rgba.len() != expected_len {
        bail!(
            "underwater texture rgba length {} does not match {}x{}",
            rgba.len(),
            width,
            height
        );
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-screen-effect-underwater-texture"),
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
    let sampler = device.create_sampler(&standalone_sampler_descriptor(
        "bbb-screen-effect-underwater-sampler",
        StandaloneTextureSampling::new(false, false),
    ));
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-screen-effect-underwater-bind-group"),
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
        ],
    });
    Ok(ScreenEffectUnderwaterGpu { bind_group })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad(texture: ScreenEffectTexture) -> ScreenEffectQuad {
        ScreenEffectQuad {
            positions: [
                [-1.0, -1.0, -0.5],
                [1.0, -1.0, -0.5],
                [1.0, 1.0, -0.5],
                [-1.0, 1.0, -0.5],
            ],
            uvs: [[1.0, 1.0], [0.0, 1.0], [0.0, 0.0], [1.0, 0.0]],
            color: [0.1, 0.1, 0.1, 1.0],
            texture,
        }
    }

    #[test]
    fn full_screen_overlay_quads_cover_the_whole_viewport_after_projection() {
        // Vanilla's block/water overlays span x,y in [-1, 1] at view z = -0.5.
        // Under any sane hud fov the half-extent of the frustum at that depth
        // (0.5 * tan(fov/2) * aspect) stays below 1, so every projected corner
        // must land outside the NDC unit square: the quad always covers the
        // full screen exactly like vanilla.
        let (vertices, batches) =
            screen_effect_draw_data(&[quad(ScreenEffectTexture::TerrainAtlas)], 70.0, 16.0 / 9.0);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].vertex_count, 6);
        for vertex in vertices.chunks(SCREEN_EFFECT_VERTEX_FLOATS) {
            let (x, y, w) = (vertex[0], vertex[1], vertex[3]);
            assert!(w > 0.0, "quad must sit inside the near/far range");
            assert!(
                (x / w).abs() >= 1.0 && (y / w).abs() >= 1.0,
                "overlay corner {x},{y},{w} must project outside the NDC unit square"
            );
        }
    }

    #[test]
    fn draw_batches_split_only_at_texture_changes_and_keep_vanilla_order() {
        let quads = [
            quad(ScreenEffectTexture::TerrainAtlas),
            quad(ScreenEffectTexture::Underwater),
            quad(ScreenEffectTexture::TerrainAtlas),
            quad(ScreenEffectTexture::TerrainAtlas),
        ];
        let (vertices, batches) = screen_effect_draw_data(&quads, 70.0, 16.0 / 9.0);
        assert_eq!(vertices.len(), 4 * 6 * SCREEN_EFFECT_VERTEX_FLOATS);
        assert_eq!(batches.len(), 3);
        assert_eq!(
            (batches[0].texture, batches[0].first_vertex, batches[0].vertex_count),
            (ScreenEffectTexture::TerrainAtlas, 0, 6)
        );
        assert_eq!(
            (batches[1].texture, batches[1].first_vertex, batches[1].vertex_count),
            (ScreenEffectTexture::Underwater, 6, 6)
        );
        assert_eq!(
            (batches[2].texture, batches[2].first_vertex, batches[2].vertex_count),
            (ScreenEffectTexture::TerrainAtlas, 12, 12)
        );
    }

    #[test]
    fn screen_effect_pipeline_states_match_the_vanilla_gui_textured_snippet() {
        // `BLOCK_SCREEN_EFFECT`/`FIRE_SCREEN_EFFECT` build `GUI_TEXTURED_SNIPPET`
        // unchanged: translucent blend and no depth state. The shader multiplies
        // the sampled texel by the vanilla vertex color.
        assert_eq!(
            SCREEN_EFFECT_BLEND.color.src_factor,
            wgpu::BlendFactor::SrcAlpha
        );
        assert_eq!(
            SCREEN_EFFECT_BLEND.color.dst_factor,
            wgpu::BlendFactor::OneMinusSrcAlpha
        );
        assert_eq!(SCREEN_EFFECT_BLEND.alpha.src_factor, wgpu::BlendFactor::One);
        assert!(SCREEN_EFFECT_SHADER
            .contains("textureSample(effect_texture, effect_sampler, input.uv) * input.color"));
        let source = include_str!("screen_effects.rs");
        let production = &source[..source.find("#[cfg(test)]").expect("test module marker")];
        assert!(
            !production.contains("depth_stencil("),
            "screen effect pipeline must not attach a depth state"
        );
    }
}
