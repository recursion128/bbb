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
    #[serde(
        default = "default_selection_line_width",
        skip_serializing_if = "is_default_selection_line_width"
    )]
    pub line_width: f32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub always_on_top: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SelectionLine {
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub color: [f32; 4],
    #[serde(
        default = "default_selection_line_width",
        skip_serializing_if = "is_default_selection_line_width"
    )]
    pub width: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SelectionPoint {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub size: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionTextLabel {
    pub position: [f32; 3],
    pub text: String,
    pub color: [f32; 4],
    #[serde(default, skip_serializing_if = "is_false")]
    pub centered: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionOutline {
    pub boxes: Vec<SelectionBox>,
    #[serde(default)]
    pub colored_boxes: Vec<SelectionColoredBox>,
    #[serde(default)]
    pub lines: Vec<SelectionLine>,
    #[serde(default)]
    pub points: Vec<SelectionPoint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub text_labels: Vec<SelectionTextLabel>,
}

impl SelectionOutline {
    pub fn from_box(min: [f32; 3], max: [f32; 3]) -> Self {
        Self {
            boxes: vec![SelectionBox { min, max }],
            colored_boxes: Vec::new(),
            lines: Vec::new(),
            points: Vec::new(),
            text_labels: Vec::new(),
        }
    }

    pub fn from_boxes(boxes: impl IntoIterator<Item = SelectionBox>) -> Self {
        Self {
            boxes: boxes.into_iter().collect(),
            colored_boxes: Vec::new(),
            lines: Vec::new(),
            points: Vec::new(),
            text_labels: Vec::new(),
        }
    }

    pub fn from_colored_boxes_and_lines(
        colored_boxes: impl IntoIterator<Item = SelectionColoredBox>,
        lines: impl IntoIterator<Item = SelectionLine>,
    ) -> Self {
        Self::from_colored_boxes_lines_and_points(
            colored_boxes,
            lines,
            std::iter::empty::<SelectionPoint>(),
        )
    }

    pub fn from_colored_boxes_lines_and_points(
        colored_boxes: impl IntoIterator<Item = SelectionColoredBox>,
        lines: impl IntoIterator<Item = SelectionLine>,
        points: impl IntoIterator<Item = SelectionPoint>,
    ) -> Self {
        Self::from_colored_boxes_lines_points_and_labels(
            colored_boxes,
            lines,
            points,
            std::iter::empty::<SelectionTextLabel>(),
        )
    }

    pub fn from_colored_boxes_lines_points_and_labels(
        colored_boxes: impl IntoIterator<Item = SelectionColoredBox>,
        lines: impl IntoIterator<Item = SelectionLine>,
        points: impl IntoIterator<Item = SelectionPoint>,
        text_labels: impl IntoIterator<Item = SelectionTextLabel>,
    ) -> Self {
        Self {
            boxes: Vec::new(),
            colored_boxes: colored_boxes.into_iter().collect(),
            lines: lines.into_iter().collect(),
            points: points.into_iter().collect(),
            text_labels: text_labels.into_iter().collect(),
        }
    }
}

const SELECTION_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32, 3 => Float32, 4 => Float32x4];
#[cfg(test)]
const SELECTION_OUTLINE_ALPHA: f32 = 102.0 / 255.0;
const DEFAULT_SELECTION_OUTLINE_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 102.0 / 255.0];
const DEFAULT_SELECTION_LINE_WIDTH: f32 = 1.0;
const SELECTION_LINES_DEPTH_WRITE: bool = true;
const CHUNK_BORDER_ALWAYS_ON_TOP_DEPTH_TEST: bool = false;
const SELECTION_POINT_PROXY_WORLD_SCALE: f32 = 0.01;

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
    @location(1) line_direction: vec3<f32>,
    @location(2) line_width: f32,
    @location(3) line_side: f32,
    @location(4) color: vec4<f32>,
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
    let line_pos_start = camera.view_proj_view_offset_z * vec4<f32>(input.position, 1.0);
    let line_pos_end = camera.view_proj_view_offset_z * vec4<f32>(input.position + input.line_direction, 1.0);
    let ndc_start = line_pos_start.xyz / line_pos_start.w;
    let ndc_end = line_pos_end.xyz / line_pos_end.w;
    let screen_size = max(camera.viewport_size.xy, vec2<f32>(1.0, 1.0));
    let screen_direction = (ndc_end.xy - ndc_start.xy) * screen_size;
    var line_offset = vec2<f32>(0.0, 0.0);
    if (length(screen_direction) > 0.000001) {
        let line_screen_direction = normalize(screen_direction);
        line_offset = vec2<f32>(-line_screen_direction.y, line_screen_direction.x) * input.line_width / screen_size;
        if (line_offset.x < 0.0) {
            line_offset = -line_offset;
        }
    }
    out.position = vec4<f32>(
        (ndc_start + vec3<f32>(line_offset * input.line_side, 0.0)) * line_pos_start.w,
        line_pos_start.w
    );
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
    pub(super) always_on_top_vertex_buffer: Option<wgpu::Buffer>,
    pub(super) always_on_top_vertex_count: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SelectionVertex {
    position: [f32; 3],
    line_direction: [f32; 3],
    line_width: f32,
    line_side: f32,
    color: [f32; 4],
}

pub(super) fn create_selection_outline_gpu(
    device: &wgpu::Device,
    outline: SelectionOutline,
) -> SelectionOutlineGpu {
    let vertices = selection_outline_vertices(&outline);
    let always_on_top_vertices = selection_outline_always_on_top_vertices(&outline);
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-selection-outline-vertices"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let always_on_top_vertex_buffer = (!always_on_top_vertices.is_empty()).then(|| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bbb-selection-outline-always-on-top-vertices"),
            contents: bytemuck::cast_slice(&always_on_top_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        })
    });

    SelectionOutlineGpu {
        outline,
        vertex_buffer,
        vertex_count: vertices.len() as u32,
        always_on_top_vertex_buffer,
        always_on_top_vertex_count: always_on_top_vertices.len() as u32,
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
        .topology(wgpu::PrimitiveTopology::TriangleList)
        .depth_stencil(depth_stencil_state(
            DEPTH_FORMAT,
            SELECTION_LINES_DEPTH_WRITE,
            wgpu::CompareFunction::LessEqual,
        ))
        .build()
}

pub(super) fn create_chunk_border_always_on_top_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, "bbb-chunk-border-debug-pipeline")
        .shader("bbb-chunk-border-debug-shader", SELECTION_SHADER)
        .layout(
            "bbb-chunk-border-debug-pipeline-layout",
            &[camera_bind_group_layout],
        )
        .vertex_buffers(&[selection_vertex_layout()])
        .color_target(format, Some(wgpu::BlendState::ALPHA_BLENDING))
        .topology(wgpu::PrimitiveTopology::TriangleList)
        .depth_stencil(if CHUNK_BORDER_ALWAYS_ON_TOP_DEPTH_TEST {
            Some(depth_stencil_state(
                DEPTH_FORMAT,
                SELECTION_LINES_DEPTH_WRITE,
                wgpu::CompareFunction::LessEqual,
            ))
        } else {
            None
        })
        .build()
}

fn selection_outline_vertices(outline: &SelectionOutline) -> Vec<SelectionVertex> {
    selection_outline_vertices_for(outline, false)
}

fn selection_outline_always_on_top_vertices(outline: &SelectionOutline) -> Vec<SelectionVertex> {
    selection_outline_vertices_for(outline, true)
}

fn selection_outline_vertices_for(
    outline: &SelectionOutline,
    always_on_top: bool,
) -> Vec<SelectionVertex> {
    let mut vertices = Vec::with_capacity(
        outline.boxes.len() * 72
            + outline.colored_boxes.len() * 72
            + outline.lines.len() * 6
            + outline.points.len() * 18,
    );
    if !always_on_top {
        for outline_box in &outline.boxes {
            vertices.extend(selection_box_vertices_with_width(
                outline_box.min,
                outline_box.max,
                DEFAULT_SELECTION_OUTLINE_COLOR,
                DEFAULT_SELECTION_LINE_WIDTH,
            ));
        }
    }
    for outline_box in &outline.colored_boxes {
        if outline_box.always_on_top != always_on_top {
            continue;
        }
        vertices.extend(selection_box_vertices_with_width(
            outline_box.min,
            outline_box.max,
            outline_box.color,
            sanitize_line_width(outline_box.line_width),
        ));
    }
    if !always_on_top {
        for line in &outline.lines {
            vertices.extend(selection_line_vertices(*line));
        }
        for point in &outline.points {
            vertices.extend(selection_point_vertices(*point));
        }
    }
    vertices
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn default_selection_line_width() -> f32 {
    DEFAULT_SELECTION_LINE_WIDTH
}

fn is_default_selection_line_width(value: &f32) -> bool {
    (*value - DEFAULT_SELECTION_LINE_WIDTH).abs() <= f32::EPSILON
}

fn sanitize_line_width(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        DEFAULT_SELECTION_LINE_WIDTH
    }
}

fn selection_box_vertices_with_width(
    box_min: [f32; 3],
    box_max: [f32; 3],
    color: [f32; 4],
    line_width: f32,
) -> Vec<SelectionVertex> {
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

    let mut vertices = Vec::with_capacity(72);
    for (from, to) in [
        (p000, p100),
        (p100, p101),
        (p101, p001),
        (p001, p000),
        (p010, p110),
        (p110, p111),
        (p111, p011),
        (p011, p010),
        (p000, p010),
        (p100, p110),
        (p101, p111),
        (p001, p011),
    ] {
        vertices.extend(selection_line_vertices_with_width(
            from, to, color, line_width,
        ));
    }
    vertices
}

fn selection_line_vertices(line: SelectionLine) -> [SelectionVertex; 6] {
    selection_line_vertices_with_width(line.from, line.to, line.color, line.width)
}

fn selection_line_vertices_with_width(
    from: [f32; 3],
    to: [f32; 3],
    color: [f32; 4],
    line_width: f32,
) -> [SelectionVertex; 6] {
    let line_width = sanitize_line_width(line_width);
    let direction = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    [
        selection_vertex(from, direction, line_width, 1.0, color),
        selection_vertex(from, direction, line_width, -1.0, color),
        selection_vertex(to, direction, line_width, 1.0, color),
        selection_vertex(to, direction, line_width, 1.0, color),
        selection_vertex(from, direction, line_width, -1.0, color),
        selection_vertex(to, direction, line_width, -1.0, color),
    ]
}

fn selection_point_vertices(point: SelectionPoint) -> Vec<SelectionVertex> {
    let half_extent = if point.size.is_finite() {
        point.size.max(0.0) * SELECTION_POINT_PROXY_WORLD_SCALE
    } else {
        0.0
    };
    let center = Vec3::from_array(point.position);
    let x = Vec3::X * half_extent;
    let y = Vec3::Y * half_extent;
    let z = Vec3::Z * half_extent;
    let width = sanitize_line_width(point.size);
    let mut vertices = Vec::with_capacity(18);
    vertices.extend(selection_line_vertices_with_width(
        (center - x).to_array(),
        (center + x).to_array(),
        point.color,
        width,
    ));
    vertices.extend(selection_line_vertices_with_width(
        (center - y).to_array(),
        (center + y).to_array(),
        point.color,
        width,
    ));
    vertices.extend(selection_line_vertices_with_width(
        (center - z).to_array(),
        (center + z).to_array(),
        point.color,
        width,
    ));
    vertices
}

fn selection_vertex(
    position: [f32; 3],
    line_direction: [f32; 3],
    line_width: f32,
    line_side: f32,
    color: [f32; 4],
) -> SelectionVertex {
    SelectionVertex {
        position,
        line_direction,
        line_width,
        line_side,
        color,
    }
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
    fn chunk_border_pipeline_models_vanilla_always_on_top() {
        // Vanilla `ChunkBorderRenderer.emitGizmos` marks the current-section
        // cuboid with `setAlwaysOnTop()`. The dedicated bbb debug pipeline
        // mirrors that by omitting depth testing for that split primitive set.
        assert!(!CHUNK_BORDER_ALWAYS_ON_TOP_DEPTH_TEST);
    }

    #[test]
    fn selection_outline_vertices_emit_expanded_box_edges() {
        let vertices = selection_outline_vertices(&SelectionOutline::from_box(
            [1.0, 2.0, 3.0],
            [2.0, 3.0, 4.0],
        ));
        assert_eq!(vertices.len(), 72);
        assert_eq!(vertices[0].position, [0.998, 1.998, 2.998]);
        assert_eq!(vertices[0].color, DEFAULT_SELECTION_OUTLINE_COLOR);
        assert_eq!(vertices[0].line_width, DEFAULT_SELECTION_LINE_WIDTH);
        assert_eq!(vertices[2].position, [2.002, 1.998, 2.998]);
        assert_eq!(vertices[66].position, [0.998, 1.998, 4.002]);
        assert_eq!(vertices[71].position, [0.998, 3.002, 4.002]);
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

        assert_eq!(vertices.len(), 144);
        assert_eq!(vertices[0].position, [-0.002, -0.002, -0.002]);
        assert_eq!(vertices[72].position, [1.998, -0.002, -0.002]);
        assert_eq!(vertices[74].position, [3.002, -0.002, -0.002]);
    }

    #[test]
    fn selection_outline_vertices_emit_colored_boxes_and_lines() {
        let vertices = selection_outline_vertices(&SelectionOutline::from_colored_boxes_and_lines(
            [SelectionColoredBox {
                min: [0.0, 0.0, 0.0],
                max: [1.0, 1.0, 1.0],
                color: [1.0, 0.0, 0.0, 1.0],
                line_width: 2.5,
                always_on_top: false,
            }],
            [SelectionLine {
                from: [2.0, 0.0, 0.0],
                to: [3.0, 1.0, 0.0],
                color: [0.0, 0.0, 1.0, 1.0],
                width: 4.0,
            }],
        ));

        assert_eq!(vertices.len(), 78);
        assert_eq!(vertices[0].position, [-0.002, -0.002, -0.002]);
        assert_eq!(vertices[0].color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(vertices[0].line_width, 2.5);
        assert_eq!(vertices[72].position, [2.0, 0.0, 0.0]);
        assert_eq!(vertices[72].color, [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(vertices[72].line_width, 4.0);
        assert_eq!(vertices[74].position, [3.0, 1.0, 0.0]);
        assert_eq!(vertices[74].color, [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn selection_outline_splits_always_on_top_colored_boxes() {
        let outline = SelectionOutline::from_colored_boxes_and_lines(
            [
                SelectionColoredBox {
                    min: [0.0, 0.0, 0.0],
                    max: [1.0, 1.0, 1.0],
                    color: [1.0, 0.0, 0.0, 1.0],
                    line_width: 1.0,
                    always_on_top: false,
                },
                SelectionColoredBox {
                    min: [2.0, 0.0, 0.0],
                    max: [3.0, 1.0, 1.0],
                    color: [0.0, 1.0, 0.0, 1.0],
                    line_width: 1.0,
                    always_on_top: true,
                },
            ],
            [SelectionLine {
                from: [4.0, 0.0, 0.0],
                to: [5.0, 0.0, 0.0],
                color: [0.0, 0.0, 1.0, 1.0],
                width: 1.0,
            }],
        );

        let vertices = selection_outline_vertices(&outline);
        let always_on_top_vertices = selection_outline_always_on_top_vertices(&outline);
        assert_eq!(vertices.len(), 78);
        assert_eq!(vertices[0].position, [-0.002, -0.002, -0.002]);
        assert_eq!(vertices[72].position, [4.0, 0.0, 0.0]);
        assert_eq!(always_on_top_vertices.len(), 72);
        assert_eq!(always_on_top_vertices[0].position, [1.998, -0.002, -0.002]);
    }

    #[test]
    fn selection_outline_vertices_emit_point_proxies() {
        let vertices =
            selection_outline_vertices(&SelectionOutline::from_colored_boxes_lines_and_points(
                std::iter::empty::<SelectionColoredBox>(),
                std::iter::empty::<SelectionLine>(),
                [SelectionPoint {
                    position: [1.0, 2.0, 3.0],
                    color: [1.0, 1.0, 1.0, 1.0],
                    size: 2.0,
                }],
            ));

        assert_eq!(vertices.len(), 18);
        assert_eq!(vertices[0].position, [0.98, 2.0, 3.0]);
        assert_eq!(vertices[2].position, [1.02, 2.0, 3.0]);
        assert_eq!(vertices[6].position, [1.0, 1.98, 3.0]);
        assert_eq!(vertices[8].position, [1.0, 2.02, 3.0]);
        assert_eq!(vertices[12].position, [1.0, 2.0, 2.98]);
        assert_eq!(vertices[14].position, [1.0, 2.0, 3.02]);
        assert_eq!(vertices[0].color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(vertices[0].line_width, 2.0);
    }
}
