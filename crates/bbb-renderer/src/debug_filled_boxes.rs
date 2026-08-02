use serde::{Deserialize, Serialize};

use crate::{
    gpu::DEPTH_FORMAT,
    pipeline_builder::{depth_stencil_state, RenderPipelineBuilder},
};

/// One solid-color world-space cuboid submitted through vanilla's
/// `RenderTypes.debugFilledBox()` surface.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DebugFilledBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub color: [u8; 4],
}

/// One solid-color world-space `RectGizmo` submitted through the same
/// `debugFilledBox` render type as cuboids. Corners are counter-clockwise when
/// viewed from the visible side, matching vanilla `GizmoPrimitives.addQuad`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DebugFilledRect {
    pub corners: [[f32; 3]; 4],
    pub color: [u8; 4],
}

/// Vanilla `Direction` order used by `RectGizmo.fromCuboidFace` consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebugCuboidFace {
    West,
    South,
    East,
    North,
    Down,
    Up,
}

impl DebugFilledBox {
    /// Normalizes reversed bounds and rejects non-finite or zero-volume boxes.
    pub fn sanitized(self) -> Option<Self> {
        if !self
            .min
            .iter()
            .chain(self.max.iter())
            .all(|coordinate| coordinate.is_finite())
        {
            return None;
        }

        let min = std::array::from_fn(|axis| self.min[axis].min(self.max[axis]));
        let max = std::array::from_fn(|axis| self.min[axis].max(self.max[axis]));
        (0..3).all(|axis| min[axis] < max[axis]).then_some(Self {
            min,
            max,
            color: self.color,
        })
    }
}

impl DebugFilledRect {
    /// Exact `RectGizmo.fromCuboidFace` corner winding.
    pub fn cuboid_face(
        min: [f32; 3],
        max: [f32; 3],
        face: DebugCuboidFace,
        color: [u8; 4],
    ) -> Self {
        let [x0, y0, z0] = min;
        let [x1, y1, z1] = max;
        let corners = match face {
            DebugCuboidFace::West => [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]],
            DebugCuboidFace::South => [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
            DebugCuboidFace::East => [[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]],
            DebugCuboidFace::North => [[x0, y0, z0], [x0, y1, z0], [x1, y1, z0], [x1, y0, z0]],
            DebugCuboidFace::Down => [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
            DebugCuboidFace::Up => [[x0, y1, z0], [x0, y1, z1], [x1, y1, z1], [x1, y1, z0]],
        };
        Self { corners, color }
    }

    pub fn sanitized(self) -> Option<Self> {
        if !self.corners.iter().flatten().all(|value| value.is_finite()) {
            return None;
        }
        let a = glam::Vec3::from_array(self.corners[0]);
        let b = glam::Vec3::from_array(self.corners[1]);
        let c = glam::Vec3::from_array(self.corners[2]);
        ((b - a).cross(c - a).length_squared() > f32::EPSILON).then_some(self)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct DebugFilledBoxVertex {
    pub(super) position: [f32; 3],
    /// Vanilla `DefaultVertexFormat.POSITION_COLOR` stores color as normalized
    /// unsigned bytes. `Unorm8x4` exposes the exact `component / 255.0` value
    /// to WGSL while retaining the original public color without round trips.
    pub(super) color: [u8; 4],
}

#[derive(Debug, Default, PartialEq)]
pub(super) struct DebugFilledBoxMesh {
    pub(super) vertices: Vec<DebugFilledBoxVertex>,
    pub(super) indices: Vec<u32>,
}

const DEBUG_FILLED_BOX_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Unorm8x4];
const DEBUG_FILLED_BOX_BLEND: wgpu::BlendState = wgpu::BlendState::ALPHA_BLENDING;
const DEBUG_FILLED_BOX_DEPTH_WRITE_ENABLED: bool = false;
const DEBUG_FILLED_BOX_DEPTH_COMPARE: wgpu::CompareFunction = wgpu::CompareFunction::LessEqual;
const DEBUG_FILLED_BOX_CULL_MODE: Option<wgpu::Face> = Some(wgpu::Face::Back);
const DEBUG_FILLED_BOX_SORT_ON_UPLOAD: bool = true;

const DEBUG_FILLED_BOX_SHADER: &str = r#"
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
    minecraft_light0: vec4<f32>,
    minecraft_light1: vec4<f32>,
    glint_offsets: vec4<f32>,
    view_proj_view_offset_z: mat4x4<f32>,
    view_proj_view_offset_z_forward: mat4x4<f32>,
    projection: mat4x4<f32>,
    sky_model_view: mat4x4<f32>,
    shader_game_time: vec4<f32>,
    viewport_size: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj_view_offset_z * vec4<f32>(input.position, 1.0);
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

pub(super) fn sanitize_debug_filled_boxes(mut boxes: Vec<DebugFilledBox>) -> Vec<DebugFilledBox> {
    boxes.retain_mut(|value| match value.sanitized() {
        Some(sanitized) => {
            *value = sanitized;
            true
        }
        None => false,
    });
    boxes
}

pub(super) fn sanitize_debug_filled_rects(mut rects: Vec<DebugFilledRect>) -> Vec<DebugFilledRect> {
    rects.retain_mut(|value| match value.sanitized() {
        Some(sanitized) => {
            *value = sanitized;
            true
        }
        None => false,
    });
    rects
}

/// Builds vanilla `CuboidGizmo`'s six outward-facing quads. When a camera is
/// available, indices are sorted far-to-near by quad center, mirroring
/// `RenderType.sortOnUpload()` / `MeshData.sortQuads`.
pub(super) fn build_debug_filled_mesh<'a>(
    boxes: impl Clone + Iterator<Item = &'a DebugFilledBox>,
    rects: &[DebugFilledRect],
    camera_position: Option<[f32; 3]>,
) -> DebugFilledBoxMesh {
    let box_count = boxes.clone().count();
    let mut vertices = Vec::with_capacity(
        box_count
            .saturating_mul(24)
            .saturating_add(rects.len().saturating_mul(4)),
    );
    let mut quad_centers =
        Vec::with_capacity(box_count.saturating_mul(6).saturating_add(rects.len()));

    for debug_box in boxes.copied().filter_map(DebugFilledBox::sanitized) {
        let [x0, y0, z0] = debug_box.min;
        let [x1, y1, z1] = debug_box.max;
        // Vertex order matches vanilla 26.1 `CuboidGizmo.emit`: EAST, WEST,
        // NORTH, SOUTH, UP, DOWN, each counter-clockwise from outside.
        for quad in [
            [[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]],
            [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]],
            [[x0, y0, z0], [x0, y1, z0], [x1, y1, z0], [x1, y0, z0]],
            [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
            [[x0, y1, z0], [x0, y1, z1], [x1, y1, z1], [x1, y1, z0]],
            [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
        ] {
            let center = std::array::from_fn(|axis| (quad[0][axis] + quad[2][axis]) * 0.5);
            quad_centers.push(center);
            vertices.extend(quad.map(|position| DebugFilledBoxVertex {
                position,
                color: debug_box.color,
            }));
        }
    }
    for rect in rects.iter().copied().filter_map(DebugFilledRect::sanitized) {
        let center = std::array::from_fn(|axis| {
            rect.corners.iter().map(|corner| corner[axis]).sum::<f32>() * 0.25
        });
        quad_centers.push(center);
        vertices.extend(rect.corners.map(|position| DebugFilledBoxVertex {
            position,
            color: rect.color,
        }));
    }

    let mut quad_order: Vec<usize> = (0..quad_centers.len()).collect();
    if DEBUG_FILLED_BOX_SORT_ON_UPLOAD {
        if let Some(camera) =
            camera_position.filter(|position| position.iter().all(|v| v.is_finite()))
        {
            quad_order.sort_by(|&left, &right| {
                let distance_squared = |center: [f32; 3]| {
                    let delta = [
                        center[0] - camera[0],
                        center[1] - camera[1],
                        center[2] - camera[2],
                    ];
                    delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]
                };
                distance_squared(quad_centers[right])
                    .total_cmp(&distance_squared(quad_centers[left]))
                    .then_with(|| left.cmp(&right))
            });
        }
    }

    let mut indices = Vec::with_capacity(quad_order.len().saturating_mul(6));
    for quad_index in quad_order {
        let base = (quad_index * 4) as u32;
        indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
    }

    DebugFilledBoxMesh { vertices, indices }
}

#[cfg(test)]
fn build_debug_filled_box_mesh(
    boxes: &[DebugFilledBox],
    camera_position: Option<[f32; 3]>,
) -> DebugFilledBoxMesh {
    build_debug_filled_mesh(boxes.iter(), &[], camera_position)
}

pub(super) fn create_debug_filled_box_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, "bbb-debug-filled-box-pipeline")
        .shader("bbb-debug-filled-box-shader", DEBUG_FILLED_BOX_SHADER)
        .layout(
            "bbb-debug-filled-box-pipeline-layout",
            &[camera_bind_group_layout],
        )
        .vertex_buffers(&[debug_filled_box_vertex_layout()])
        .color_target(format, Some(DEBUG_FILLED_BOX_BLEND))
        .depth_stencil(depth_stencil_state(
            DEPTH_FORMAT,
            DEBUG_FILLED_BOX_DEPTH_WRITE_ENABLED,
            DEBUG_FILLED_BOX_DEPTH_COMPARE,
        ))
        .cull_mode(DEBUG_FILLED_BOX_CULL_MODE)
        .build()
}

fn debug_filled_box_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<DebugFilledBoxVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &DEBUG_FILLED_BOX_VERTEX_ATTRIBUTES,
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;
    use crate::{ClearColor, Renderer};

    fn box_from(min: [f32; 3], max: [f32; 3], color: [u8; 4]) -> DebugFilledBox {
        DebugFilledBox { min, max, color }
    }

    #[test]
    fn debug_filled_box_sanitize_normalizes_reversed_bounds_and_drops_invalid_boxes() {
        let color = [1, 2, 3, 4];
        assert_eq!(
            box_from([3.0, 4.0, 5.0], [1.0, 2.0, 3.0], color).sanitized(),
            Some(box_from([1.0, 2.0, 3.0], [3.0, 4.0, 5.0], color))
        );
        assert_eq!(
            box_from([0.0, 0.0, 0.0], [1.0, 0.0, 1.0], color).sanitized(),
            None,
            "zero extent on any axis is rejected"
        );
        assert_eq!(
            box_from([f32::NAN, 0.0, 0.0], [1.0, 1.0, 1.0], color).sanitized(),
            None
        );
        assert_eq!(
            box_from([0.0, 0.0, 0.0], [1.0, f32::INFINITY, 1.0], color).sanitized(),
            None
        );
    }

    #[test]
    fn debug_filled_rect_sanitize_and_mesh_preserve_vanilla_quad_winding() {
        let rect = DebugFilledRect::cuboid_face(
            [0.0, 0.0, -1.0],
            [1.0, 1.0, 0.0],
            DebugCuboidFace::South,
            [1, 2, 3, 4],
        );
        assert_eq!(
            rect.corners,
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ]
        );
        assert_eq!(rect.sanitized(), Some(rect));
        assert_eq!(
            DebugFilledRect {
                corners: [[0.0; 3]; 4],
                color: rect.color,
            }
            .sanitized(),
            None
        );
        let mesh = build_debug_filled_mesh([].iter(), &[rect], None);
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices, [0, 1, 2, 2, 3, 0]);
        let a = Vec3::from_array(mesh.vertices[0].position);
        let b = Vec3::from_array(mesh.vertices[1].position);
        let c = Vec3::from_array(mesh.vertices[2].position);
        assert!((b - a).cross(c - a).normalize().dot(Vec3::Z) > 0.999);
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.color == rect.color));
    }

    #[test]
    fn debug_filled_box_mesh_has_six_ccw_faces_and_vanilla_topology() {
        let mesh = build_debug_filled_box_mesh(
            &[box_from([1.0, 2.0, 3.0], [4.0, 6.0, 8.0], [7, 8, 9, 10])],
            None,
        );
        assert_eq!(mesh.vertices.len(), 24);
        assert_eq!(mesh.indices.len(), 36);

        let expected_normals = [
            Vec3::X,
            Vec3::NEG_X,
            Vec3::NEG_Z,
            Vec3::Z,
            Vec3::Y,
            Vec3::NEG_Y,
        ];
        for (face, expected_normal) in expected_normals.into_iter().enumerate() {
            let base = (face * 4) as u32;
            assert_eq!(
                &mesh.indices[face * 6..face * 6 + 6],
                &[base, base + 1, base + 2, base + 2, base + 3, base]
            );
            let a = Vec3::from_array(mesh.vertices[base as usize].position);
            let b = Vec3::from_array(mesh.vertices[base as usize + 1].position);
            let c = Vec3::from_array(mesh.vertices[base as usize + 2].position);
            assert!(
                (b - a).cross(c - a).normalize().dot(expected_normal) > 0.999,
                "face {face} must be CCW when viewed from outside"
            );
        }
    }

    #[test]
    fn debug_filled_box_vertex_color_uses_exact_unorm8_normalization() {
        let color = [0, 1, 127, 255];
        let mesh = build_debug_filled_box_mesh(&[box_from([0.0; 3], [1.0; 3], color)], None);
        assert!(mesh.vertices.iter().all(|vertex| vertex.color == color));
        assert_eq!(
            DEBUG_FILLED_BOX_VERTEX_ATTRIBUTES[1].format,
            wgpu::VertexFormat::Unorm8x4
        );
        assert_eq!(
            color.map(|component| component as f32 / 255.0),
            [0.0, 1.0 / 255.0, 127.0 / 255.0, 1.0]
        );
    }

    #[test]
    fn debug_filled_box_mesh_sorts_quads_back_to_front_on_upload() {
        let mesh = build_debug_filled_box_mesh(
            &[
                box_from([1.0, 0.0, 0.0], [2.0, 1.0, 1.0], [255; 4]),
                box_from([10.0, 0.0, 0.0], [11.0, 1.0, 1.0], [255; 4]),
            ],
            Some([0.0; 3]),
        );
        assert!(
            mesh.indices[..36].iter().all(|index| *index >= 24),
            "all six faces of the farther box sort before the nearer box"
        );

        let boxes = [
            box_from([-11.0, 0.0, 0.0], [-10.0, 1.0, 1.0], [255; 4]),
            box_from([10.0, 0.0, 0.0], [11.0, 1.0, 1.0], [255; 4]),
        ];
        let from_left = build_debug_filled_box_mesh(&boxes, Some([-5.0, 0.0, 0.0]));
        let from_right = build_debug_filled_box_mesh(&boxes, Some([5.0, 0.0, 0.0]));
        assert!(from_left.indices[0] >= 24);
        assert!(from_right.indices[0] < 24);
    }

    #[test]
    fn debug_filled_mesh_chains_dynamic_and_game_test_lanes_before_sorting() {
        let dynamic = [box_from([1.0, 0.0, 0.0], [2.0, 1.0, 1.0], [255, 0, 0, 255])];
        let game_test = [box_from(
            [10.0, 0.0, 0.0],
            [11.0, 1.0, 1.0],
            [0, 255, 0, 255],
        )];
        let mesh =
            build_debug_filled_mesh(dynamic.iter().chain(game_test.iter()), &[], Some([0.0; 3]));

        assert_eq!(mesh.vertices.len(), 48);
        assert!(mesh.vertices[..24]
            .iter()
            .all(|vertex| vertex.color == [255, 0, 0, 255]));
        assert!(mesh.vertices[24..]
            .iter()
            .all(|vertex| vertex.color == [0, 255, 0, 255]));
        assert!(
            mesh.indices[..36].iter().all(|index| *index >= 24),
            "the farther stable lane must sort ahead of the nearer dynamic lane in one mesh"
        );
    }

    #[test]
    fn debug_filled_sanitizer_preserves_the_input_allocation() {
        let mut boxes = Vec::with_capacity(16);
        boxes.push(box_from([1.0; 3], [0.0; 3], [1, 2, 3, 4]));
        boxes.push(box_from([0.0; 3], [0.0; 3], [5, 6, 7, 8]));
        let boxes = sanitize_debug_filled_boxes(boxes);
        assert_eq!(boxes.len(), 1);
        assert_eq!(boxes[0].min, [0.0; 3]);
        assert_eq!(boxes[0].max, [1.0; 3]);
        assert!(boxes.capacity() >= 16);
    }

    #[test]
    fn debug_filled_box_pipeline_matches_vanilla_render_type_state() {
        assert_eq!(DEBUG_FILLED_BOX_CULL_MODE, Some(wgpu::Face::Back));
        assert!(!DEBUG_FILLED_BOX_DEPTH_WRITE_ENABLED);
        assert_eq!(
            DEBUG_FILLED_BOX_DEPTH_COMPARE,
            wgpu::CompareFunction::LessEqual
        );
        assert_eq!(
            DEBUG_FILLED_BOX_BLEND.color,
            wgpu::BlendState::ALPHA_BLENDING.color
        );
        assert_eq!(
            DEBUG_FILLED_BOX_BLEND.alpha,
            wgpu::BlendState::ALPHA_BLENDING.alpha
        );
        assert!(DEBUG_FILLED_BOX_SORT_ON_UPLOAD);
        assert!(DEBUG_FILLED_BOX_SHADER.contains("view_proj_view_offset_z: mat4x4<f32>"));
        assert!(DEBUG_FILLED_BOX_SHADER
            .contains("camera.view_proj_view_offset_z * vec4<f32>(input.position, 1.0)"));
        assert!(DEBUG_FILLED_BOX_SHADER.contains("return input.color;"));
        assert!(!DEBUG_FILLED_BOX_SHADER.contains("fn apply_fog"));
        assert!(!DEBUG_FILLED_BOX_SHADER.contains("linear_fog"));
    }

    #[test]
    fn debug_filled_box_setter_empty_clears_stale_frame_geometry() {
        let Some(mut renderer) = Renderer::new_offscreen(64, 64) else {
            return;
        };
        renderer.set_clear_color(ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        renderer.set_debug_filled_boxes(vec![box_from(
            [-0.5, -0.5, 0.25],
            [0.5, 0.5, 0.75],
            [255, 0, 0, 255],
        )]);
        assert_eq!(renderer.debug_filled_boxes.len(), 1);
        let with_box = renderer
            .render_offscreen_frame()
            .expect("debug box frame readback");
        let with_box_draw_calls = renderer.counters.draw_calls;
        let center = with_box.pixel(32, 32);
        assert!(
            center[0] > 128 && center[2] < 128,
            "first frame center should contain the red cuboid, got {center:?}"
        );

        renderer.set_debug_filled_boxes(Vec::new());
        assert!(renderer.debug_filled_boxes.is_empty());
        // Keep the unrelated tick-owned lightmap pass equal across both frames;
        // this assertion is specifically about the removed debug-box draw.
        renderer.lightmap_dirty = true;
        let without_box = renderer
            .render_offscreen_frame()
            .expect("cleared debug box frame readback");
        let center = without_box.pixel(32, 32);
        assert!(
            center[2] > 128 && center[0] < 128,
            "empty setter must not redraw the previous frame's cuboid, got {center:?}"
        );
        assert_eq!(
            with_box_draw_calls,
            renderer.counters.draw_calls + 1,
            "the cleared frame omits the debug-filled-box draw"
        );

        renderer.set_game_test_highlights(
            vec![box_from([2.0, 2.0, 2.0], [3.0, 3.0, 3.0], [0, 255, 0, 255])],
            Vec::new(),
        );
        renderer.set_debug_filled_boxes(vec![box_from(
            [-0.5, -0.5, 0.25],
            [0.5, 0.5, 0.75],
            [255, 0, 0, 255],
        )]);
        renderer.set_debug_filled_boxes(Vec::new());
        assert!(renderer.debug_filled_boxes.is_empty());
        assert_eq!(
            renderer.game_test_highlight_boxes.len(),
            1,
            "clearing dynamic geometry must retain the stable GameTest lane"
        );
    }
}
