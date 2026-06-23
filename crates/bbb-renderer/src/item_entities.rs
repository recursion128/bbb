use std::mem;

use anyhow::{anyhow, bail, Result};
use glam::Vec3;

use crate::{gpu::DEPTH_FORMAT, Renderer};

const ITEM_ENTITY_BILLBOARD_SIZE: f32 = 0.5;
const ITEM_ENTITY_TINT_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

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
    pub layers: Vec<ItemEntityBillboardLayer>,
}

impl ItemEntityBillboard {
    pub fn single(position: [f32; 3], uv: ItemEntityUvRect) -> Self {
        Self {
            position,
            scale: 1.0,
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
}

const ITEM_ENTITY_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4];

const ITEM_ENTITY_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var item_atlas: texture_2d<f32>;

@group(0) @binding(2)
var item_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(item_atlas, item_sampler, input.uv) * input.color;
    if texel.a <= 0.01 {
        discard;
    }
    return texel;
}
"#;

pub(crate) fn create_item_entity_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-item-entity-shader"),
        source: wgpu::ShaderSource::Wgsl(ITEM_ENTITY_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-item-entity-pipeline-layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-item-entity-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[item_entity_vertex_layout()],
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
            depth_write_enabled: false,
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
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
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
        item_entity_vertex(bottom_left, [uv.min[0], uv.max[1]], tint),
        item_entity_vertex(bottom_right, [uv.max[0], uv.max[1]], tint),
        item_entity_vertex(top_right, [uv.max[0], uv.min[1]], tint),
        item_entity_vertex(bottom_left, [uv.min[0], uv.max[1]], tint),
        item_entity_vertex(top_right, [uv.max[0], uv.min[1]], tint),
        item_entity_vertex(top_left, [uv.min[0], uv.min[1]], tint),
    ]
}

fn item_entity_vertex(position: Vec3, uv: [f32; 2], color: [f32; 4]) -> ItemEntityVertex {
    ItemEntityVertex {
        position: position.to_array(),
        uv,
        color,
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
        assert_close3_f32(vertices[2].position, [1.2, 2.2, 3.0]);
        assert_eq!(vertices[2].uv, [0.5, 0.125]);
        assert_eq!(vertices[2].color, [0.25, 0.5, 0.75, 0.8]);
        assert_close3_f32(vertices[11].position, [0.8, 2.2, 3.0]);
        assert_eq!(vertices[11].uv, [0.5, 0.5]);
        assert_eq!(vertices[11].color, ITEM_ENTITY_TINT_WHITE);
    }

    #[test]
    fn item_entity_billboard_scale_enlarges_the_quad() {
        // A scale of 3.0 (the large fireball) widens each corner offset 3× relative to the unit-scale
        // sprite, mirroring vanilla's `ThrownItemRenderer` `poseStack.scale(scale)`.
        let make = |scale: f32| ItemEntityBillboard {
            position: [0.0, 0.0, 0.0],
            scale,
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

    fn assert_close3_f32(actual: [f32; 3], expected: [f32; 3]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!(
                (actual - expected).abs() < 1.0e-5,
                "expected {expected}, got {actual}"
            );
        }
    }
}
