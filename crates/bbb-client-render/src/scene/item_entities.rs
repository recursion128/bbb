use std::mem;

use anyhow::Result;
use bbb_render_types::AtlasMipPatch;
use glam::Vec3;

use crate::{
    gpu::DEPTH_FORMAT,
    pipeline_builder::{depth_stencil_state, RenderPipelineBuilder},
    texture_sampling::{create_camera_texture_bind_group, create_nearest_clamp_sampler},
    Renderer,
};

const ITEM_ENTITY_BILLBOARD_SIZE: f32 = 0.5;
const ITEM_ENTITY_TINT_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const ITEM_ENTITY_FULL_BRIGHT_LIGHT: [f32; 2] = [1.0, 1.0];
const ITEM_ENTITY_PIPELINE_BLEND: wgpu::BlendState = wgpu::BlendState::ALPHA_BLENDING;
const ITEM_ENTITY_PIPELINE_CULL_MODE: Option<wgpu::Face> = Some(wgpu::Face::Back);
const ITEM_ENTITY_PIPELINE_DEPTH_WRITE_ENABLED: bool = true;
const ITEM_ENTITY_PIPELINE_DEPTH_COMPARE: wgpu::CompareFunction = wgpu::CompareFunction::LessEqual;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemEntityUvRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemEntityBillboardLayer {
    pub uv: ItemEntityUvRect,
    pub tint: [f32; 4],
}

impl ItemEntityBillboardLayer {
    pub fn new(uv: ItemEntityUvRect, tint: [f32; 4]) -> Self {
        Self { uv, tint }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ItemEntityBillboardOrientation {
    #[default]
    CameraFacing,
    FireworkShotAtAngle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemEntityBillboard {
    pub position: [f32; 3],
    /// Multiplier on the base [`ITEM_ENTITY_BILLBOARD_SIZE`] quad, mirroring vanilla's per-renderer
    /// sprite scale (the dropped item and the unit-scale `ThrownItemRenderer` projectiles use `1.0`; the
    /// large fireball uses `3.0`, the small fireball `0.75`).
    pub scale: f32,
    /// Local item-model pose after the camera-facing transform. Vanilla firework rockets apply
    /// `Z+180`, `Y+180`, `X+90` when `FireworkRocketRenderState.isShotAtAngle` is true.
    pub orientation: ItemEntityBillboardOrientation,
    /// Shader-space `[block, sky]` light sampled from vanilla `EntityRenderState.lightCoords`.
    pub light: [f32; 2],
    pub layers: Vec<ItemEntityBillboardLayer>,
}

impl ItemEntityBillboard {
    pub fn single(position: [f32; 3], uv: ItemEntityUvRect) -> Self {
        Self {
            position,
            scale: 1.0,
            orientation: ItemEntityBillboardOrientation::CameraFacing,
            light: ITEM_ENTITY_FULL_BRIGHT_LIGHT,
            layers: vec![ItemEntityBillboardLayer::new(uv, ITEM_ENTITY_TINT_WHITE)],
        }
    }
}

pub(crate) struct ItemEntityAtlasGpu {
    pub(crate) hud: crate::hud::HudSpriteGpu,
    sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) hud3d_bind_group: wgpu::BindGroup,
    pub(crate) gui_bind_group: wgpu::BindGroup,
}

impl ItemEntityAtlasGpu {
    pub(crate) fn view_and_sampler(&self) -> (&wgpu::TextureView, &wgpu::Sampler) {
        (&self.hud.gpu.view, &self.sampler)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ItemEntityVertex {
    pub(crate) position: [f32; 3],
    pub(crate) uv: [f32; 2],
    pub(crate) color: [f32; 4],
    pub(crate) light: [f32; 2],
}

const ITEM_ENTITY_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4, 3 => Float32x2];

const ITEM_ENTITY_SHADER: &str = concat!(
    crate::wgsl::camera!(),
    crate::wgsl::linear_fog_value!(),
    crate::wgsl::apply_fog!(),
    crate::wgsl::sample_lightmap!(),
    r#"
@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var item_atlas: texture_2d<f32>;

@group(0) @binding(2)
var item_sampler: sampler;

@group(1) @binding(0)
var lightmap_texture: texture_2d<f32>;

@group(1) @binding(1)
var lightmap_sampler: sampler;

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

const ALPHA_CUTOUT: f32 = 0.1;

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
    let sampled = textureSample(item_atlas, item_sampler, input.uv);
    if sampled.a < ALPHA_CUTOUT {
        discard;
    }
    let texel = sampled * input.color;
    let light_color = sample_lightmap(input.light);
    return apply_fog(vec4<f32>(texel.rgb * light_color, texel.a), input.spherical_distance, input.cylindrical_distance);
}
"#,
);

pub(crate) fn create_item_entity_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, "bbb-item-entity-pipeline")
        .shader("bbb-item-entity-shader", ITEM_ENTITY_SHADER)
        .layout(
            "bbb-item-entity-pipeline-layout",
            &[bind_group_layout, lightmap_bind_group_layout],
        )
        .vertex_buffers(&[item_entity_vertex_layout()])
        .color_target(format, Some(ITEM_ENTITY_PIPELINE_BLEND))
        .cull_mode(ITEM_ENTITY_PIPELINE_CULL_MODE)
        .depth_stencil(depth_stencil_state(
            DEPTH_FORMAT,
            ITEM_ENTITY_PIPELINE_DEPTH_WRITE_ENABLED,
            ITEM_ENTITY_PIPELINE_DEPTH_COMPARE,
        ))
        .build()
}

impl Renderer {
    pub fn upload_item_atlas(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        let hud = crate::hud::create_hud_sprite_gpu(
            &self.device,
            &self.queue,
            &self.hud_bind_group_layout,
            width,
            height,
            rgba,
        )?;
        let sampler = create_nearest_clamp_sampler(&self.device, "bbb-item-entity-atlas-sampler");
        let bind = |label, camera| {
            create_camera_texture_bind_group(
                &self.device,
                &self.terrain_bind_group_layout,
                label,
                &camera,
                &hud.gpu.view,
                &sampler,
            )
        };
        self.pack.item_entity_atlas = Some(ItemEntityAtlasGpu {
            bind_group: bind(
                "bbb-item-entity-atlas-bind-group",
                self.camera_buffers.world(),
            ),
            hud3d_bind_group: bind(
                "bbb-item-entity-atlas-hud3d-bind-group",
                self.camera_buffers.hud3d(),
            ),
            gui_bind_group: bind(
                "bbb-item-entity-atlas-gui-bind-group",
                self.camera_buffers.gui(),
            ),
            hud,
            sampler,
        });
        self.invalidate_item_model_atlas(crate::item_models::ItemModelAtlasKind::Items);
        Ok(())
    }

    pub fn update_item_atlas_patches(&mut self, patches: &[AtlasMipPatch]) -> Result<()> {
        let Some(atlas) = &self.pack.item_entity_atlas else {
            return Ok(());
        };
        let texture = &atlas.hud.gpu;
        crate::gpu::write_texture_atlas_mip_patches_gpu(
            &self.queue,
            &texture.texture,
            texture.width,
            texture.height,
            1,
            patches,
        )
    }

    pub fn set_item_entity_billboards(&mut self, billboards: Vec<ItemEntityBillboard>) {
        let billboards = billboards
            .into_iter()
            .filter_map(sanitize_item_entity_billboard)
            .collect::<Vec<_>>();
        self.counters.item_entity_billboards = billboards.len();
        self.item_entity_billboards = billboards;
    }

    pub(crate) fn collect_item_entity_vertices(&self) -> Vec<ItemEntityVertex> {
        let Some(pose) = self.camera_pose else {
            return Vec::new();
        };
        item_entity_billboard_vertices(
            &self.item_entity_billboards,
            pose.billboard_axes(),
            ITEM_ENTITY_BILLBOARD_SIZE,
        )
    }
}

fn item_entity_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<ItemEntityVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ITEM_ENTITY_VERTEX_ATTRIBUTES,
    }
}

fn sanitize_item_entity_billboard(billboard: ItemEntityBillboard) -> Option<ItemEntityBillboard> {
    if !billboard
        .position
        .iter()
        .all(|component| component.is_finite())
    {
        return None;
    }
    // A non-finite or non-positive scale would collapse or explode the quad; fall back to the base size.
    let scale = if billboard.scale.is_finite() && billboard.scale > 0.0 {
        billboard.scale
    } else {
        1.0
    };
    let layers = billboard
        .layers
        .into_iter()
        .filter_map(sanitize_item_entity_layer)
        .collect::<Vec<_>>();
    (!layers.is_empty()).then_some(ItemEntityBillboard {
        position: billboard.position,
        scale,
        orientation: billboard.orientation,
        light: sanitize_item_entity_light(billboard.light),
        layers,
    })
}

fn sanitize_item_entity_layer(layer: ItemEntityBillboardLayer) -> Option<ItemEntityBillboardLayer> {
    if !layer.tint.iter().all(|component| component.is_finite()) {
        return None;
    }
    Some(ItemEntityBillboardLayer {
        uv: sanitize_item_entity_uv_rect(layer.uv)?,
        tint: layer.tint.map(|component| component.clamp(0.0, 1.0)),
    })
}

fn sanitize_item_entity_uv_rect(rect: ItemEntityUvRect) -> Option<ItemEntityUvRect> {
    let components = [rect.min[0], rect.min[1], rect.max[0], rect.max[1]];
    if !components.iter().all(|component| component.is_finite()) {
        return None;
    }

    let min_x = rect.min[0].clamp(0.0, 1.0);
    let min_y = rect.min[1].clamp(0.0, 1.0);
    let max_x = rect.max[0].clamp(0.0, 1.0);
    let max_y = rect.max[1].clamp(0.0, 1.0);
    Some(ItemEntityUvRect {
        min: [min_x.min(max_x), min_y.min(max_y)],
        max: [min_x.max(max_x), min_y.max(max_y)],
    })
}

fn sanitize_item_entity_light(light: [f32; 2]) -> [f32; 2] {
    [
        if light[0].is_finite() {
            light[0].clamp(0.0, 1.0)
        } else {
            ITEM_ENTITY_FULL_BRIGHT_LIGHT[0]
        },
        if light[1].is_finite() {
            light[1].clamp(0.0, 1.0)
        } else {
            ITEM_ENTITY_FULL_BRIGHT_LIGHT[1]
        },
    ]
}

fn item_entity_billboard_vertices(
    billboards: &[ItemEntityBillboard],
    axes: crate::camera::CameraBillboardAxes,
    base_size: f32,
) -> Vec<ItemEntityVertex> {
    let mut vertices = Vec::new();
    for billboard in billboards {
        let size = base_size * billboard.scale;
        let axes = item_entity_layer_axes(axes, billboard.orientation);
        for layer in &billboard.layers {
            vertices.extend(item_entity_layer_vertices(
                billboard.position,
                layer,
                billboard.light,
                axes,
                size,
            ));
        }
    }
    vertices
}

fn item_entity_layer_axes(
    axes: crate::camera::CameraBillboardAxes,
    orientation: ItemEntityBillboardOrientation,
) -> crate::camera::CameraBillboardAxes {
    match orientation {
        ItemEntityBillboardOrientation::CameraFacing => axes,
        ItemEntityBillboardOrientation::FireworkShotAtAngle => crate::camera::CameraBillboardAxes {
            forward: axes.up,
            right: axes.right,
            up: -axes.forward,
        },
    }
}

fn item_entity_layer_vertices(
    position: [f32; 3],
    layer: &ItemEntityBillboardLayer,
    light: [f32; 2],
    axes: crate::camera::CameraBillboardAxes,
    size: f32,
) -> [ItemEntityVertex; 6] {
    let center = Vec3::from_array(position);
    let half_size = size * 0.5;
    let right = axes.right * half_size;
    let up = axes.up * half_size;
    let bottom_left = center - right - up;
    let bottom_right = center + right - up;
    let top_right = center + right + up;
    let top_left = center - right + up;
    let uv = layer.uv;
    let tint = layer.tint;

    [
        item_entity_vertex(bottom_left, [uv.min[0], uv.max[1]], tint, light),
        item_entity_vertex(bottom_right, [uv.max[0], uv.max[1]], tint, light),
        item_entity_vertex(top_right, [uv.max[0], uv.min[1]], tint, light),
        item_entity_vertex(bottom_left, [uv.min[0], uv.max[1]], tint, light),
        item_entity_vertex(top_right, [uv.max[0], uv.min[1]], tint, light),
        item_entity_vertex(top_left, [uv.min[0], uv.min[1]], tint, light),
    ]
}

fn item_entity_vertex(
    position: Vec3,
    uv: [f32; 2],
    color: [f32; 4],
    light: [f32; 2],
) -> ItemEntityVertex {
    ItemEntityVertex {
        position: position.to_array(),
        uv,
        color,
        light,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_atlas_update_validation_requires_the_original_rgba_extent() {
        assert!(crate::texture_sampling::validate_rgba(2, 1, &[0; 8]).is_ok());
        assert!(crate::texture_sampling::validate_rgba(2, 1, &[0; 4]).is_err());
    }

    #[test]
    fn sanitize_item_entity_billboard_discards_invalid_position() {
        let billboard = ItemEntityBillboard::single(
            [0.0, f32::NAN, 0.0],
            ItemEntityUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        );

        assert_eq!(sanitize_item_entity_billboard(billboard), None);
    }

    #[test]
    fn sanitize_item_entity_billboard_keeps_valid_layers_and_clamps_values() {
        let billboard = ItemEntityBillboard {
            position: [1.0, 2.0, 3.0],
            scale: 3.0,
            orientation: ItemEntityBillboardOrientation::CameraFacing,
            light: [1.25, f32::NAN],
            layers: vec![
                ItemEntityBillboardLayer::new(
                    ItemEntityUvRect {
                        min: [0.8, -0.5],
                        max: [0.2, 1.5],
                    },
                    [1.2, 0.5, -0.2, 0.75],
                ),
                ItemEntityBillboardLayer::new(
                    ItemEntityUvRect {
                        min: [0.0, f32::NAN],
                        max: [1.0, 1.0],
                    },
                    ITEM_ENTITY_TINT_WHITE,
                ),
            ],
        };

        assert_eq!(
            sanitize_item_entity_billboard(billboard),
            Some(ItemEntityBillboard {
                position: [1.0, 2.0, 3.0],
                scale: 3.0,
                orientation: ItemEntityBillboardOrientation::CameraFacing,
                light: [1.0, 1.0],
                layers: vec![ItemEntityBillboardLayer::new(
                    ItemEntityUvRect {
                        min: [0.2, 0.0],
                        max: [0.8, 1.0],
                    },
                    [1.0, 0.5, 0.0, 0.75],
                )],
            })
        );
    }

    #[test]
    fn sanitize_item_entity_billboard_falls_back_on_invalid_scale() {
        for bad_scale in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            let billboard = ItemEntityBillboard {
                position: [0.0, 0.0, 0.0],
                scale: bad_scale,
                orientation: ItemEntityBillboardOrientation::CameraFacing,
                light: ITEM_ENTITY_FULL_BRIGHT_LIGHT,
                layers: vec![ItemEntityBillboardLayer::new(
                    ItemEntityUvRect {
                        min: [0.0, 0.0],
                        max: [1.0, 1.0],
                    },
                    ITEM_ENTITY_TINT_WHITE,
                )],
            };
            assert_eq!(
                sanitize_item_entity_billboard(billboard).map(|b| b.scale),
                Some(1.0),
                "scale {bad_scale} should fall back to 1.0",
            );
        }
    }

    #[test]
    fn item_entity_billboard_vertices_emit_layered_camera_facing_quads() {
        let billboards = [ItemEntityBillboard {
            position: [1.0, 2.0, 3.0],
            scale: 1.0,
            orientation: ItemEntityBillboardOrientation::CameraFacing,
            light: [0.4, 0.8],
            layers: vec![
                ItemEntityBillboardLayer::new(
                    ItemEntityUvRect {
                        min: [0.25, 0.125],
                        max: [0.5, 0.375],
                    },
                    [0.25, 0.5, 0.75, 0.8],
                ),
                ItemEntityBillboardLayer::new(
                    ItemEntityUvRect {
                        min: [0.5, 0.5],
                        max: [0.75, 0.75],
                    },
                    ITEM_ENTITY_TINT_WHITE,
                ),
            ],
        }];

        let vertices = item_entity_billboard_vertices(
            &billboards,
            crate::camera::CameraBillboardAxes {
                forward: Vec3::Z,
                right: Vec3::X,
                up: Vec3::Y,
            },
            0.4,
        );

        assert_eq!(vertices.len(), 12);
        assert_close3_f32(vertices[0].position, [0.8, 1.8, 3.0]);
        assert_eq!(vertices[0].uv, [0.25, 0.375]);
        assert_eq!(vertices[0].color, [0.25, 0.5, 0.75, 0.8]);
        assert_eq!(vertices[0].light, [0.4, 0.8]);
        assert_close3_f32(vertices[2].position, [1.2, 2.2, 3.0]);
        assert_eq!(vertices[2].uv, [0.5, 0.125]);
        assert_eq!(vertices[2].color, [0.25, 0.5, 0.75, 0.8]);
        assert_eq!(vertices[2].light, [0.4, 0.8]);
        assert_close3_f32(vertices[11].position, [0.8, 2.2, 3.0]);
        assert_eq!(vertices[11].uv, [0.5, 0.5]);
        assert_eq!(vertices[11].color, ITEM_ENTITY_TINT_WHITE);
        assert_eq!(vertices[11].light, [0.4, 0.8]);
    }

    #[test]
    fn item_entity_billboard_scale_enlarges_the_quad() {
        // A scale of 3.0 (the large fireball) widens each corner offset 3× relative to the unit-scale
        // sprite, mirroring vanilla's `ThrownItemRenderer` `poseStack.scale(scale)`.
        let make = |scale: f32| ItemEntityBillboard {
            position: [0.0, 0.0, 0.0],
            scale,
            orientation: ItemEntityBillboardOrientation::CameraFacing,
            light: ITEM_ENTITY_FULL_BRIGHT_LIGHT,
            layers: vec![ItemEntityBillboardLayer::new(
                ItemEntityUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                ITEM_ENTITY_TINT_WHITE,
            )],
        };
        let axes = crate::camera::CameraBillboardAxes {
            forward: Vec3::Z,
            right: Vec3::X,
            up: Vec3::Y,
        };
        let unit = item_entity_billboard_vertices(&[make(1.0)], axes, 0.5);
        let triple = item_entity_billboard_vertices(&[make(3.0)], axes, 0.5);

        // Bottom-left corner: half-size 0.25 at unit scale, 0.75 at ×3.
        assert_close3_f32(unit[0].position, [-0.25, -0.25, 0.0]);
        assert_close3_f32(triple[0].position, [-0.75, -0.75, 0.0]);
    }

    #[test]
    fn item_entity_billboard_firework_shot_at_angle_applies_vanilla_post_camera_pose() {
        let billboards = [ItemEntityBillboard {
            position: [0.0, 0.0, 0.0],
            scale: 1.0,
            orientation: ItemEntityBillboardOrientation::FireworkShotAtAngle,
            light: ITEM_ENTITY_FULL_BRIGHT_LIGHT,
            layers: vec![ItemEntityBillboardLayer::new(
                ItemEntityUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                ITEM_ENTITY_TINT_WHITE,
            )],
        }];

        let vertices = item_entity_billboard_vertices(
            &billboards,
            crate::camera::CameraBillboardAxes {
                forward: Vec3::Z,
                right: Vec3::X,
                up: Vec3::Y,
            },
            0.5,
        );

        // FireworkEntityRenderer applies camera orientation and then Z+180, Y+180, X+90. For the local
        // item quad this leaves X on camera-right and rotates local Y onto negative camera-forward.
        assert_close3_f32(vertices[0].position, [-0.25, 0.0, 0.25]);
        assert_close3_f32(vertices[1].position, [0.25, 0.0, 0.25]);
        assert_close3_f32(vertices[2].position, [0.25, 0.0, -0.25]);
        assert_close3_f32(vertices[5].position, [-0.25, 0.0, -0.25]);
    }

    #[test]
    fn item_entity_shader_samples_dynamic_lightmap_texture() {
        assert_eq!(ITEM_ENTITY_VERTEX_ATTRIBUTES.len(), 4);
        assert_eq!(ITEM_ENTITY_VERTEX_ATTRIBUTES[3].shader_location, 3);
        assert_eq!(
            ITEM_ENTITY_VERTEX_ATTRIBUTES[3].format,
            wgpu::VertexFormat::Float32x2
        );
        assert!(ITEM_ENTITY_SHADER.contains("@location(3) light: vec2<f32>"));
        assert!(ITEM_ENTITY_SHADER.contains("@group(1) @binding(0)"));
        assert!(ITEM_ENTITY_SHADER.contains("var lightmap_texture: texture_2d<f32>"));
        assert!(ITEM_ENTITY_SHADER.contains("@group(1) @binding(1)"));
        assert!(ITEM_ENTITY_SHADER.contains("var lightmap_sampler: sampler"));
        assert!(ITEM_ENTITY_SHADER.contains("fn sample_lightmap(light: vec2<f32>) -> vec3<f32>"));
        assert!(ITEM_ENTITY_SHADER.contains("light * (15.0 / 16.0) + vec2<f32>(0.5 / 16.0)"));
        assert!(ITEM_ENTITY_SHADER
            .contains("textureSample(lightmap_texture, lightmap_sampler, uv).rgb"));
        assert!(ITEM_ENTITY_SHADER.contains("let light_color = sample_lightmap(input.light)"));
        assert!(ITEM_ENTITY_SHADER.contains("texel.rgb * light_color"));
        assert!(!ITEM_ENTITY_SHADER.contains("fn lightmap_brightness"));
        assert!(!ITEM_ENTITY_SHADER.contains("camera.lightmap_factors.y"));
    }

    #[test]
    fn item_entity_shader_uses_vanilla_item_alpha_cutout() {
        // Vanilla thrown-item renderers call `ItemStackRenderState.submit`, whose
        // item render types define `ALPHA_CUTOUT` as 0.1F and discard before tint.
        assert!(ITEM_ENTITY_SHADER.contains("const ALPHA_CUTOUT: f32 = 0.1;"));
        assert!(ITEM_ENTITY_SHADER
            .contains("let sampled = textureSample(item_atlas, item_sampler, input.uv);"));
        assert!(ITEM_ENTITY_SHADER.contains("if sampled.a < ALPHA_CUTOUT {"));
        assert!(ITEM_ENTITY_SHADER.contains("let texel = sampled * input.color;"));
        assert!(!ITEM_ENTITY_SHADER.contains("texel.a <= 0.01"));
        assert!(!ITEM_ENTITY_SHADER
            .contains("textureSample(item_atlas, item_sampler, input.uv) * input.color"));
    }

    #[test]
    fn item_entity_pipeline_state_matches_vanilla_item_translucent() {
        assert_eq!(ITEM_ENTITY_PIPELINE_CULL_MODE, Some(wgpu::Face::Back));
        const {
            assert!(ITEM_ENTITY_PIPELINE_DEPTH_WRITE_ENABLED);
        }
        assert_eq!(
            ITEM_ENTITY_PIPELINE_DEPTH_COMPARE,
            wgpu::CompareFunction::LessEqual
        );
        assert_eq!(
            ITEM_ENTITY_PIPELINE_BLEND.color.src_factor,
            wgpu::BlendFactor::SrcAlpha
        );
        assert_eq!(
            ITEM_ENTITY_PIPELINE_BLEND.color.dst_factor,
            wgpu::BlendFactor::OneMinusSrcAlpha
        );
        assert_eq!(
            ITEM_ENTITY_PIPELINE_BLEND.alpha.src_factor,
            wgpu::BlendFactor::One
        );
        assert_eq!(
            ITEM_ENTITY_PIPELINE_BLEND.alpha.dst_factor,
            wgpu::BlendFactor::OneMinusSrcAlpha
        );
    }

    fn assert_close3_f32(actual: [f32; 3], expected: [f32; 3]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!(
                (actual - expected).abs() < 1.0e-5,
                "expected {expected}, got {actual}"
            );
        }
    }
}
