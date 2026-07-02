use std::mem;

use anyhow::{anyhow, bail, Result};
use glam::Vec3;

use crate::{
    gpu::DEPTH_FORMAT,
    pipeline_builder::{depth_stencil_state, RenderPipelineBuilder},
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

#[derive(Debug, Clone, PartialEq)]
pub struct ItemEntityBillboard {
    pub position: [f32; 3],
    /// Multiplier on the base [`ITEM_ENTITY_BILLBOARD_SIZE`] quad, mirroring vanilla's per-renderer
    /// sprite scale (the dropped item and the unit-scale `ThrownItemRenderer` projectiles use `1.0`; the
    /// large fireball uses `3.0`, the small fireball `0.75`).
    pub scale: f32,
    /// Shader-space `[block, sky]` light sampled from vanilla `EntityRenderState.lightCoords`.
    pub light: [f32; 2],
    pub layers: Vec<ItemEntityBillboardLayer>,
}

impl ItemEntityBillboard {
    pub fn single(position: [f32; 3], uv: ItemEntityUvRect) -> Self {
        Self {
            position,
            scale: 1.0,
            light: ITEM_ENTITY_FULL_BRIGHT_LIGHT,
            layers: vec![ItemEntityBillboardLayer::new(uv, ITEM_ENTITY_TINT_WHITE)],
        }
    }
}

pub(crate) struct ItemEntityAtlasGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
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

const ITEM_ENTITY_SHADER: &str = r#"
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

fn sample_lightmap(light: vec2<f32>) -> vec3<f32> {
    let uv = clamp(
        light * (15.0 / 16.0) + vec2<f32>(0.5 / 16.0),
        vec2<f32>(0.5 / 16.0),
        vec2<f32>(15.5 / 16.0),
    );
    return textureSample(lightmap_texture, lightmap_sampler, uv).rgb;
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
    let sampled = textureSample(item_atlas, item_sampler, input.uv);
    if sampled.a < ALPHA_CUTOUT {
        discard;
    }
    let texel = sampled * input.color;
    let light_color = sample_lightmap(input.light);
    return apply_fog(vec4<f32>(texel.rgb * light_color, texel.a), input.spherical_distance, input.cylindrical_distance);
}
"#;

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

pub(crate) fn create_item_entity_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<ItemEntityAtlasGpu> {
    validate_item_entity_atlas_rgba(width, height, rgba)?;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-item-entity-atlas-texture"),
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
        label: Some("bbb-item-entity-atlas-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-item-entity-atlas-bind-group"),
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

    Ok(ItemEntityAtlasGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
    })
}

impl Renderer {
    pub fn upload_item_entity_atlas(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.item_entity_atlas = Some(create_item_entity_atlas_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.camera_buffer,
            width,
            height,
            rgba,
        )?);
        Ok(())
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
            camera_billboard_axes(pose),
            ITEM_ENTITY_BILLBOARD_SIZE,
        )
    }
}

fn validate_item_entity_atlas_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<()> {
    if width == 0 || height == 0 {
        bail!("item entity atlas dimensions must be non-zero");
    }
    let expected_len = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("item entity atlas size overflow"))?;
    if rgba.len() != expected_len {
        bail!(
            "item entity atlas has {} RGBA bytes, expected {} for {}x{}",
            rgba.len(),
            expected_len,
            width,
            height
        );
    }
    Ok(())
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

#[derive(Debug, Clone, Copy)]
struct ItemEntityBillboardAxes {
    right: Vec3,
    up: Vec3,
}

fn camera_billboard_axes(pose: crate::CameraPose) -> ItemEntityBillboardAxes {
    let yaw = pose.y_rot.to_radians();
    let pitch = pose.x_rot.to_radians();
    let cos_pitch = pitch.cos();
    let forward =
        Vec3::new(-yaw.sin() * cos_pitch, -pitch.sin(), yaw.cos() * cos_pitch).normalize_or_zero();
    let forward = if forward.length_squared() > 0.0 {
        forward
    } else {
        Vec3::Z
    };
    let right = Vec3::Y.cross(forward).normalize_or_zero();
    let right = if right.length_squared() > 0.0 {
        right
    } else {
        Vec3::X
    };
    let up = forward.cross(right).normalize_or_zero();
    ItemEntityBillboardAxes {
        right,
        up: if up.length_squared() > 0.0 {
            up
        } else {
            Vec3::Y
        },
    }
}

fn item_entity_billboard_vertices(
    billboards: &[ItemEntityBillboard],
    axes: ItemEntityBillboardAxes,
    base_size: f32,
) -> Vec<ItemEntityVertex> {
    let mut vertices = Vec::new();
    for billboard in billboards {
        let size = base_size * billboard.scale;
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

fn item_entity_layer_vertices(
    position: [f32; 3],
    layer: &ItemEntityBillboardLayer,
    light: [f32; 2],
    axes: ItemEntityBillboardAxes,
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
            ItemEntityBillboardAxes {
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
            light: ITEM_ENTITY_FULL_BRIGHT_LIGHT,
            layers: vec![ItemEntityBillboardLayer::new(
                ItemEntityUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                ITEM_ENTITY_TINT_WHITE,
            )],
        };
        let axes = ItemEntityBillboardAxes {
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
        assert!(ITEM_ENTITY_PIPELINE_DEPTH_WRITE_ENABLED);
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
