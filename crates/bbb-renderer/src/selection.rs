use anyhow::Result;
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

/// The render phase selected from Vanilla's baked block-state material flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockSelectionPhase {
    BeforeTranslucent,
    AfterTranslucent,
}

/// Immutable per-frame block-hit outline projection.
///
/// Native control resolves block shape, resource-generation material flags, surface-scaled width,
/// and accessibility style before this crosses into the renderer. The renderer owns only GPU
/// residency and pass placement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockSelectionRenderState {
    pub boxes: Vec<SelectionBox>,
    pub phase: BlockSelectionPhase,
    pub line_width: f32,
    pub high_contrast: bool,
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

/// One world-space line submitted through vanilla's gizmo `lines()` or
/// `linesTranslucent()` surface.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DebugLine {
    pub from: [f32; 3],
    pub to: [f32; 3],
    /// RGBA8 after extracting the components from vanilla's ARGB color.
    pub color: [u8; 4],
    /// Screen-space width in pixels. `GizmoStyle.stroke(...)` defaults to 2.5.
    pub width: f32,
}

impl DebugLine {
    pub const DEFAULT_WIDTH: f32 = 2.5;

    pub fn new(from: [f32; 3], to: [f32; 3], color: [u8; 4], width: f32) -> Self {
        Self {
            from,
            to,
            color,
            width,
        }
    }

    /// Emits `CuboidGizmo`'s twelve stroke edges in vanilla order.
    pub fn cuboid(min: [f32; 3], max: [f32; 3], color: [u8; 4], width: f32) -> [Self; 12] {
        let [x0, y0, z0] = min;
        let [x1, y1, z1] = max;
        [
            Self::new([x0, y0, z0], [x1, y0, z0], color, width),
            Self::new([x0, y0, z0], [x0, y1, z0], color, width),
            Self::new([x0, y0, z0], [x0, y0, z1], color, width),
            Self::new([x1, y0, z0], [x1, y1, z0], color, width),
            Self::new([x1, y1, z0], [x0, y1, z0], color, width),
            Self::new([x0, y1, z0], [x0, y1, z1], color, width),
            Self::new([x0, y1, z1], [x0, y0, z1], color, width),
            Self::new([x0, y0, z1], [x1, y0, z1], color, width),
            Self::new([x1, y0, z1], [x1, y0, z0], color, width),
            Self::new([x0, y1, z1], [x1, y1, z1], color, width),
            Self::new([x1, y0, z1], [x1, y1, z1], color, width),
            Self::new([x1, y1, z0], [x1, y1, z1], color, width),
        ]
    }

    fn sanitized(self) -> Option<Self> {
        let finite_positions = self
            .from
            .iter()
            .chain(self.to.iter())
            .all(|coordinate| coordinate.is_finite());
        let nonzero = self.from != self.to;
        (finite_positions && nonzero && self.width.is_finite() && self.width > 0.0).then_some(self)
    }
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
const HIGH_CONTRAST_SELECTION_BACKGROUND_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const HIGH_CONTRAST_SELECTION_BACKGROUND_WIDTH: f32 = 7.0;
const HIGH_CONTRAST_SELECTION_COLOR: [f32; 4] = [87.0 / 255.0, 1.0, 225.0 / 255.0, 1.0];
const SELECTION_LINES_DEPTH_WRITE: bool = true;
const TRANSLUCENT_DEBUG_LINES_DEPTH_WRITE: bool = false;
const DEBUG_LINE_NEAR_PLANE: f32 = 0.05;
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

pub(super) struct BlockSelectionGpu {
    pub(super) state: BlockSelectionRenderState,
    pub(super) secondary_vertex_buffer: Option<wgpu::Buffer>,
    pub(super) secondary_vertex_count: u32,
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) vertex_count: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct SelectionVertex {
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

pub(super) fn create_block_selection_gpu(
    device: &wgpu::Device,
    state: BlockSelectionRenderState,
) -> BlockSelectionGpu {
    let (secondary_vertices, vertices) = block_selection_vertices(&state);
    let secondary_vertex_buffer = (!secondary_vertices.is_empty()).then(|| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bbb-block-selection-secondary-vertices"),
            contents: bytemuck::cast_slice(&secondary_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        })
    });
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-block-selection-vertices"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    BlockSelectionGpu {
        state,
        secondary_vertex_buffer,
        secondary_vertex_count: secondary_vertices.len() as u32,
        vertex_buffer,
        vertex_count: vertices.len() as u32,
    }
}

pub(super) fn create_selection_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedLineShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::core_shaders::create_selected_line_pipeline(
            device,
            format,
            camera_bind_group_layout,
            "bbb-selected-selection-outline",
            shaders,
            selection_vertex_layout(),
            SELECTION_LINES_DEPTH_WRITE,
            true,
        );
    }
    Ok(
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
            .build(),
    )
}

pub(super) fn create_translucent_debug_line_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedLineShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::core_shaders::create_selected_line_pipeline(
            device,
            format,
            camera_bind_group_layout,
            "bbb-selected-debug-lines-translucent",
            shaders,
            selection_vertex_layout(),
            TRANSLUCENT_DEBUG_LINES_DEPTH_WRITE,
            true,
        );
    }
    Ok(
        RenderPipelineBuilder::new(device, "bbb-debug-lines-translucent-pipeline")
            .shader("bbb-debug-lines-translucent-shader", SELECTION_SHADER)
            .layout(
                "bbb-debug-lines-translucent-pipeline-layout",
                &[camera_bind_group_layout],
            )
            .vertex_buffers(&[selection_vertex_layout()])
            .color_target(format, Some(wgpu::BlendState::ALPHA_BLENDING))
            .topology(wgpu::PrimitiveTopology::TriangleList)
            .depth_stencil(depth_stencil_state(
                DEPTH_FORMAT,
                TRANSLUCENT_DEBUG_LINES_DEPTH_WRITE,
                wgpu::CompareFunction::LessEqual,
            ))
            .build(),
    )
}

#[derive(Debug, Default)]
pub(super) struct DebugLineVertexBatches {
    pub(super) opaque: Vec<SelectionVertex>,
    pub(super) translucent: Vec<SelectionVertex>,
}

pub(super) fn sanitize_debug_lines(mut lines: Vec<DebugLine>) -> Vec<DebugLine> {
    lines.retain(|line| line.sanitized().is_some());
    lines
}

pub(super) fn debug_line_vertex_batches(
    lines: &[DebugLine],
    camera_pose: crate::CameraPose,
) -> DebugLineVertexBatches {
    let mut batches = DebugLineVertexBatches::default();
    for line in lines.iter().copied().filter_map(DebugLine::sanitized) {
        let Some(line) = clip_debug_line_to_near_plane(line, camera_pose) else {
            continue;
        };
        let color = line.color.map(|component| f32::from(component) / 255.0);
        let vertices = selection_line_vertices_with_width(line.from, line.to, color, line.width);
        if line.color[3] == 255 {
            batches.opaque.extend(vertices);
        } else {
            batches.translucent.extend(vertices);
        }
    }
    batches
}

fn clip_debug_line_to_near_plane(
    mut line: DebugLine,
    camera_pose: crate::CameraPose,
) -> Option<DebugLine> {
    let eye = Vec3::from_array(camera_pose.position) + Vec3::Y * camera_pose.eye_height;
    let forward = camera_pose.billboard_axes().forward;
    let start = Vec3::from_array(line.from);
    let end = Vec3::from_array(line.to);
    let start_depth = (start - eye).dot(forward);
    let end_depth = (end - eye).dot(forward);
    let start_behind = start_depth < DEBUG_LINE_NEAR_PLANE;
    let end_behind = end_depth < DEBUG_LINE_NEAR_PLANE;
    if start_behind && end_behind {
        return None;
    }
    if start_behind || end_behind {
        let denominator = end_depth - start_depth;
        if denominator.abs() < 1.0e-9 {
            return None;
        }
        let t = ((DEBUG_LINE_NEAR_PLANE - start_depth) / denominator).clamp(0.0, 1.0);
        let intersection = start.lerp(end, t).to_array();
        if start_behind {
            line.from = intersection;
        } else {
            line.to = intersection;
        }
    }
    Some(line)
}

pub(super) fn create_chunk_border_always_on_top_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedLineShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::core_shaders::create_selected_line_pipeline(
            device,
            format,
            camera_bind_group_layout,
            "bbb-selected-chunk-border-debug",
            shaders,
            selection_vertex_layout(),
            SELECTION_LINES_DEPTH_WRITE,
            CHUNK_BORDER_ALWAYS_ON_TOP_DEPTH_TEST,
        );
    }
    Ok(
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
            .build(),
    )
}

fn selection_outline_vertices(outline: &SelectionOutline) -> Vec<SelectionVertex> {
    selection_outline_vertices_for(outline, false)
}

fn block_selection_vertices(
    state: &BlockSelectionRenderState,
) -> (Vec<SelectionVertex>, Vec<SelectionVertex>) {
    let mut secondary_vertices = Vec::with_capacity(if state.high_contrast {
        state.boxes.len() * 72
    } else {
        0
    });
    if state.high_contrast {
        for outline_box in &state.boxes {
            secondary_vertices.extend(selection_box_vertices_with_width(
                outline_box.min,
                outline_box.max,
                HIGH_CONTRAST_SELECTION_BACKGROUND_COLOR,
                HIGH_CONTRAST_SELECTION_BACKGROUND_WIDTH,
            ));
        }
    }
    let (color, width) = if state.high_contrast {
        (HIGH_CONTRAST_SELECTION_COLOR, state.line_width)
    } else {
        (DEFAULT_SELECTION_OUTLINE_COLOR, state.line_width)
    };
    let mut vertices = Vec::with_capacity(state.boxes.len() * 72);
    for outline_box in &state.boxes {
        vertices.extend(selection_box_vertices_with_width(
            outline_box.min,
            outline_box.max,
            color,
            width,
        ));
    }
    (secondary_vertices, vertices)
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

pub(super) fn selection_lines_vertices(lines: &[SelectionLine]) -> Vec<SelectionVertex> {
    let mut vertices = Vec::with_capacity(lines.len() * 6);
    for line in lines {
        vertices.extend(selection_line_vertices(*line));
    }
    vertices
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
        assert!(!TRANSLUCENT_DEBUG_LINES_DEPTH_WRITE);
    }

    #[test]
    fn debug_line_cuboid_matches_vanilla_edge_order_and_default_width() {
        assert_eq!(DebugLine::DEFAULT_WIDTH, 2.5);
        let lines = DebugLine::cuboid(
            [1.0, 2.0, 3.0],
            [4.0, 5.0, 6.0],
            [7, 8, 9, 255],
            DebugLine::DEFAULT_WIDTH,
        );
        assert_eq!(lines.len(), 12);
        assert_eq!(lines[0].from, [1.0, 2.0, 3.0]);
        assert_eq!(lines[0].to, [4.0, 2.0, 3.0]);
        assert_eq!(lines[1].to, [1.0, 5.0, 3.0]);
        assert_eq!(lines[2].to, [1.0, 2.0, 6.0]);
        assert_eq!(lines[11].from, [4.0, 5.0, 3.0]);
        assert_eq!(lines[11].to, [4.0, 5.0, 6.0]);
        assert!(lines
            .iter()
            .all(|line| line.color == [7, 8, 9, 255] && line.width == 2.5));
    }

    #[test]
    fn debug_line_batches_clip_the_near_plane_and_split_depth_write_groups() {
        let camera = crate::CameraPose {
            position: [0.0; 3],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        };
        let batches = debug_line_vertex_batches(
            &[
                DebugLine::new([0.0, 0.0, -1.0], [0.0, 0.0, 1.0], [255, 255, 255, 255], 2.5),
                DebugLine::new([1.0, 0.0, 2.0], [2.0, 0.0, 2.0], [1, 2, 3, 4], 1.0),
                DebugLine::new([0.0, 0.0, -2.0], [1.0, 0.0, -1.0], [255; 4], 2.5),
            ],
            camera,
        );
        assert_eq!(batches.opaque.len(), 6);
        assert_eq!(batches.translucent.len(), 6);
        assert!((batches.opaque[0].position[2] - DEBUG_LINE_NEAR_PLANE).abs() < 1.0e-6);
        assert_eq!(batches.opaque[0].color, [1.0; 4]);
        assert_eq!(
            batches.translucent[0].color,
            [1.0 / 255.0, 2.0 / 255.0, 3.0 / 255.0, 4.0 / 255.0]
        );
    }

    #[test]
    fn debug_line_sanitize_and_renderer_setter_clear_invalid_or_stale_lines() {
        let valid = DebugLine::new([0.0; 3], [1.0, 0.0, 0.0], [255; 4], 2.5);
        assert_eq!(sanitize_debug_lines(vec![valid]), [valid]);
        assert!(
            sanitize_debug_lines(vec![DebugLine::new([0.0; 3], [0.0; 3], [255; 4], 2.5,)])
                .is_empty()
        );
        assert!(sanitize_debug_lines(vec![DebugLine::new(
            [f32::NAN, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [255; 4],
            2.5,
        )])
        .is_empty());

        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.set_debug_lines(vec![valid]);
        assert_eq!(renderer.debug_lines, [valid]);
        renderer.set_debug_lines(Vec::new());
        assert!(renderer.debug_lines.is_empty());
    }

    #[test]
    fn debug_line_draws_through_item_entity_target_and_transparency_composite() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(64, 64) else {
            return;
        };
        renderer.set_clear_color(crate::ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        renderer.set_camera_pose(Some(crate::CameraPose {
            position: [0.0; 3],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }));
        renderer.set_level_frame_route(crate::LevelFrameRoute::ImprovedTransparency);
        renderer.set_debug_lines(vec![DebugLine::new(
            [-0.5, 0.0, 2.0],
            [0.5, 0.0, 2.0],
            [255; 4],
            2.5,
        )]);

        let pixels = renderer
            .render_offscreen_frame()
            .expect("debug line frame readback");
        let drew_white_line = (24..40).any(|y| {
            (16..48).any(|x| {
                let pixel = pixels.pixel(x, y);
                pixel[0] > 180 && pixel[1] > 180 && pixel[2] > 180
            })
        });
        assert!(drew_white_line, "debug line should reach the framebuffer");
        let corner = pixels.pixel(4, 4);
        assert!(
            corner[2] > 128 && corner[0] < 64,
            "background outside the line stays blue, got {corner:?}"
        );
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
    fn block_selection_vertices_use_surface_width_and_normal_vanilla_color() {
        let state = BlockSelectionRenderState {
            boxes: vec![SelectionBox {
                min: [1.0, 2.0, 3.0],
                max: [2.0, 3.0, 4.0],
            }],
            phase: BlockSelectionPhase::BeforeTranslucent,
            line_width: 3.75,
            high_contrast: false,
        };
        let (secondary, primary) = block_selection_vertices(&state);
        assert!(secondary.is_empty());
        assert_eq!(primary.len(), 72);
        assert_eq!(primary[0].color, DEFAULT_SELECTION_OUTLINE_COLOR);
        assert_eq!(primary[0].line_width, 3.75);
    }

    #[test]
    fn high_contrast_block_selection_emits_secondary_before_cyan_primary() {
        let state = BlockSelectionRenderState {
            boxes: vec![SelectionBox {
                min: [1.0, 2.0, 3.0],
                max: [2.0, 3.0, 4.0],
            }],
            phase: BlockSelectionPhase::AfterTranslucent,
            line_width: 2.5,
            high_contrast: true,
        };
        let (secondary, primary) = block_selection_vertices(&state);
        assert_eq!(secondary.len(), 72);
        assert_eq!(secondary[0].color, HIGH_CONTRAST_SELECTION_BACKGROUND_COLOR);
        assert_eq!(secondary[0].line_width, 7.0);
        assert_eq!(primary.len(), 72);
        assert_eq!(primary[0].color, HIGH_CONTRAST_SELECTION_COLOR);
        assert_eq!(primary[0].line_width, 2.5);
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
