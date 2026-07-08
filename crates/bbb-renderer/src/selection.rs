use glam::Vec3;
use serde::{Deserialize, Serialize};
use wgpu::util::DeviceExt;

use crate::gpu::DEPTH_FORMAT;
use crate::pipeline_builder::{depth_stencil_state, RenderPipelineBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SelectionBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SelectionColoredBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SelectionLine {
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub color: [f32; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionOutline {
    pub boxes: Vec<SelectionBox>,
    #[serde(default)]
    pub colored_boxes: Vec<SelectionColoredBox>,
    #[serde(default)]
    pub lines: Vec<SelectionLine>,
}

impl SelectionOutline {
    pub fn from_box(min: [f32; 3], max: [f32; 3]) -> Self {
        Self {
            boxes: vec![SelectionBox { min, max }],
            colored_boxes: Vec::new(),
            lines: Vec::new(),
        }
    }

    pub fn from_boxes(boxes: impl IntoIterator<Item = SelectionBox>) -> Self {
        Self {
            boxes: boxes.into_iter().collect(),
            colored_boxes: Vec::new(),
            lines: Vec::new(),
        }
    }

    pub fn from_colored_boxes_and_lines(
        colored_boxes: impl IntoIterator<Item = SelectionColoredBox>,
        lines: impl IntoIterator<Item = SelectionLine>,
    ) -> Self {
        Self {
            boxes: Vec::new(),
            colored_boxes: colored_boxes.into_iter().collect(),
            lines: lines.into_iter().collect(),
        }
    }
}

const SELECTION_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
#[cfg(test)]
const SELECTION_OUTLINE_ALPHA: f32 = 102.0 / 255.0;
const DEFAULT_SELECTION_OUTLINE_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 102.0 / 255.0];
const SELECTION_LINES_DEPTH_WRITE: bool = true;

const SELECTION_SHADER: &str = r#"
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
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) spherical_distance: f32,
    @location(1) cylindrical_distance: f32,
    @location(2) color: vec4<f32>,
};

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

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj_view_offset_z * vec4<f32>(input.position, 1.0);
    let fog_pos = input.position - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    out.color = input.color;
    return out;
}

const OUTLINE_ALPHA: f32 = 102.0 / 255.0;

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return apply_fog(input.color, input.spherical_distance, input.cylindrical_distance);
}
"#;

pub(super) struct SelectionOutlineGpu {
    pub(super) outline: SelectionOutline,
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) vertex_count: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SelectionVertex {
    position: [f32; 3],
    color: [f32; 4],
}

pub(super) fn create_selection_outline_gpu(
    device: &wgpu::Device,
    outline: SelectionOutline,
) -> SelectionOutlineGpu {
    let vertices = selection_outline_vertices(&outline);
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-selection-outline-vertices"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    SelectionOutlineGpu {
        outline,
        vertex_buffer,
        vertex_count: vertices.len() as u32,
    }
}

pub(super) fn create_selection_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, "bbb-selection-outline-pipeline")
        .shader("bbb-selection-outline-shader", SELECTION_SHADER)
        .layout(
            "bbb-selection-outline-pipeline-layout",
            &[camera_bind_group_layout],
        )
        .vertex_buffers(&[selection_vertex_layout()])
        .color_target(format, Some(wgpu::BlendState::ALPHA_BLENDING))
        .topology(wgpu::PrimitiveTopology::LineList)
        .depth_stencil(depth_stencil_state(
            DEPTH_FORMAT,
            SELECTION_LINES_DEPTH_WRITE,
            wgpu::CompareFunction::LessEqual,
        ))
        .build()
}

fn selection_outline_vertices(outline: &SelectionOutline) -> Vec<SelectionVertex> {
    let mut vertices = Vec::with_capacity(
        outline.boxes.len() * 24 + outline.colored_boxes.len() * 24 + outline.lines.len() * 2,
    );
    for outline_box in &outline.boxes {
        vertices.extend(selection_box_vertices(
            outline_box.min,
            outline_box.max,
            DEFAULT_SELECTION_OUTLINE_COLOR,
        ));
    }
    for outline_box in &outline.colored_boxes {
        vertices.extend(selection_box_vertices(
            outline_box.min,
            outline_box.max,
            outline_box.color,
        ));
    }
    for line in &outline.lines {
        vertices.extend(selection_line_vertices(*line));
    }
    vertices
}

fn selection_box_vertices(
    box_min: [f32; 3],
    box_max: [f32; 3],
    color: [f32; 4],
) -> [SelectionVertex; 24] {
    let min = Vec3::from_array(box_min).min(Vec3::from_array(box_max)) - Vec3::splat(0.002);
    let max = Vec3::from_array(box_min).max(Vec3::from_array(box_max)) + Vec3::splat(0.002);
    let p000 = [min.x, min.y, min.z];
    let p100 = [max.x, min.y, min.z];
    let p010 = [min.x, max.y, min.z];
    let p110 = [max.x, max.y, min.z];
    let p001 = [min.x, min.y, max.z];
    let p101 = [max.x, min.y, max.z];
    let p011 = [min.x, max.y, max.z];
    let p111 = [max.x, max.y, max.z];

    [
        selection_vertex(p000, color),
        selection_vertex(p100, color),
        selection_vertex(p100, color),
        selection_vertex(p101, color),
        selection_vertex(p101, color),
        selection_vertex(p001, color),
        selection_vertex(p001, color),
        selection_vertex(p000, color),
        selection_vertex(p010, color),
        selection_vertex(p110, color),
        selection_vertex(p110, color),
        selection_vertex(p111, color),
        selection_vertex(p111, color),
        selection_vertex(p011, color),
        selection_vertex(p011, color),
        selection_vertex(p010, color),
        selection_vertex(p000, color),
        selection_vertex(p010, color),
        selection_vertex(p100, color),
        selection_vertex(p110, color),
        selection_vertex(p101, color),
        selection_vertex(p111, color),
        selection_vertex(p001, color),
        selection_vertex(p011, color),
    ]
}

fn selection_line_vertices(line: SelectionLine) -> [SelectionVertex; 2] {
    [
        selection_vertex(line.from, line.color),
        selection_vertex(line.to, line.color),
    ]
}

fn selection_vertex(position: [f32; 3], color: [f32; 4]) -> SelectionVertex {
    SelectionVertex { position, color }
}

fn selection_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<SelectionVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &SELECTION_VERTEX_ATTRIBUTES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_lines_shader_uses_vanilla_view_offset_layering_and_alpha() {
        // Vanilla `RenderTypes.lines()` applies `VIEW_OFFSET_Z_LAYERING` and
        // the normal block hit outline uses `ARGB.black(102)`.
        assert!(SELECTION_SHADER.contains("view_proj_view_offset_z: mat4x4<f32>"));
        assert!(SELECTION_SHADER
            .contains("camera.view_proj_view_offset_z * vec4<f32>(input.position, 1.0)"));
        assert!(SELECTION_SHADER.contains("const OUTLINE_ALPHA: f32 = 102.0 / 255.0"));
        assert!(!SELECTION_SHADER.contains("0.65"));
        assert!((SELECTION_OUTLINE_ALPHA - (102.0 / 255.0)).abs() < f32::EPSILON);
        assert_eq!(
            DEFAULT_SELECTION_OUTLINE_COLOR,
            [0.0, 0.0, 0.0, SELECTION_OUTLINE_ALPHA]
        );
    }

    #[test]
    fn selection_lines_pipeline_keeps_vanilla_depth_write_state() {
        // Vanilla `RenderPipelines.LINES` inherits `DepthStencilState.DEFAULT`:
        // LESS_EQUAL with depth writes enabled.
        assert!(SELECTION_LINES_DEPTH_WRITE);
    }

    #[test]
    fn selection_outline_vertices_emit_expanded_box_edges() {
        let vertices = selection_outline_vertices(&SelectionOutline::from_box(
            [1.0, 2.0, 3.0],
            [2.0, 3.0, 4.0],
        ));
        assert_eq!(vertices.len(), 24);
        assert_eq!(vertices[0].position, [0.998, 1.998, 2.998]);
        assert_eq!(vertices[0].color, DEFAULT_SELECTION_OUTLINE_COLOR);
        assert_eq!(vertices[1].position, [2.002, 1.998, 2.998]);
        assert_eq!(vertices[22].position, [0.998, 1.998, 4.002]);
        assert_eq!(vertices[23].position, [0.998, 3.002, 4.002]);
    }

    #[test]
    fn selection_outline_vertices_emit_each_box_edges() {
        let vertices = selection_outline_vertices(&SelectionOutline::from_boxes([
            SelectionBox {
                min: [0.0, 0.0, 0.0],
                max: [1.0, 1.0, 1.0],
            },
            SelectionBox {
                min: [2.0, 0.0, 0.0],
                max: [3.0, 1.0, 1.0],
            },
        ]));

        assert_eq!(vertices.len(), 48);
        assert_eq!(vertices[0].position, [-0.002, -0.002, -0.002]);
        assert_eq!(vertices[24].position, [1.998, -0.002, -0.002]);
        assert_eq!(vertices[25].position, [3.002, -0.002, -0.002]);
    }

    #[test]
    fn selection_outline_vertices_emit_colored_boxes_and_lines() {
        let vertices = selection_outline_vertices(&SelectionOutline::from_colored_boxes_and_lines(
            [SelectionColoredBox {
                min: [0.0, 0.0, 0.0],
                max: [1.0, 1.0, 1.0],
                color: [1.0, 0.0, 0.0, 1.0],
            }],
            [SelectionLine {
                from: [2.0, 0.0, 0.0],
                to: [3.0, 1.0, 0.0],
                color: [0.0, 0.0, 1.0, 1.0],
            }],
        ));

        assert_eq!(vertices.len(), 26);
        assert_eq!(vertices[0].position, [-0.002, -0.002, -0.002]);
        assert_eq!(vertices[0].color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(vertices[24].position, [2.0, 0.0, 0.0]);
        assert_eq!(vertices[24].color, [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(vertices[25].position, [3.0, 1.0, 0.0]);
        assert_eq!(vertices[25].color, [0.0, 0.0, 1.0, 1.0]);
    }
}
