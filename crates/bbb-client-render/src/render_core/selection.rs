use anyhow::Result;
pub use bbb_render_types::{
    BlockSelectionPhase, BlockSelectionRenderState, DebugLine, SelectionBox, SelectionColoredBox,
    SelectionLine, SelectionOutline, SelectionPoint, SelectionTextLabel, SELECTION_OUTLINE_ALPHA,
};
use glam::Vec3;

use crate::frame_buffers::FrameDataBuffer;
use crate::render_core::gpu::DEPTH_FORMAT;
use crate::render_core::pipeline_builder::{depth_stencil_state, RenderPipelineBuilder};

const SELECTION_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32, 3 => Float32, 4 => Float32x4];
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

const SELECTION_SHADER: &str = concat!(
    crate::wgsl::camera_full!(),
    crate::wgsl::linear_fog_value!(),
    crate::wgsl::apply_fog!(),
    r#"
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
"#,
);

pub struct SelectionOutlineGpu {
    outline: Option<SelectionOutline>,
    vertex_buffer: FrameDataBuffer,
    vertex_count: u32,
    always_on_top_vertex_buffer: FrameDataBuffer,
    always_on_top_vertex_count: u32,
}

impl Default for SelectionOutlineGpu {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectionOutlineGpu {
    pub fn new() -> Self {
        Self {
            outline: None,
            vertex_buffer: FrameDataBuffer::vertex("bbb-selection-outline-vertices"),
            vertex_count: 0,
            always_on_top_vertex_buffer: FrameDataBuffer::vertex(
                "bbb-selection-outline-always-on-top-vertices",
            ),
            always_on_top_vertex_count: 0,
        }
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        outline: Option<SelectionOutline>,
    ) {
        if self.outline == outline {
            return;
        }
        if let Some(ref outline) = outline {
            let vertices = selection_outline_vertices(outline);
            let always_on_top_vertices = selection_outline_always_on_top_vertices(outline);
            self.vertex_count =
                if self
                    .vertex_buffer
                    .upload(device, queue, bytemuck::cast_slice(&vertices))
                {
                    vertices.len() as u32
                } else {
                    0
                };
            self.always_on_top_vertex_count = if self.always_on_top_vertex_buffer.upload(
                device,
                queue,
                bytemuck::cast_slice(&always_on_top_vertices),
            ) {
                always_on_top_vertices.len() as u32
            } else {
                0
            };
        } else {
            self.vertex_count = 0;
            self.always_on_top_vertex_count = 0;
        }
        self.outline = outline;
    }

    pub fn has_geometry(&self) -> bool {
        self.vertex_count > 0 || self.always_on_top_vertex_count > 0
    }

    pub fn vertices(&self) -> Option<(&wgpu::Buffer, u32)> {
        (self.vertex_count > 0).then(|| {
            (
                self.vertex_buffer
                    .buffer()
                    .expect("positive selection outline vertex count requires an uploaded buffer"),
                self.vertex_count,
            )
        })
    }

    pub fn always_on_top_vertices(&self) -> Option<(&wgpu::Buffer, u32)> {
        (self.always_on_top_vertex_count > 0).then(|| {
            (
                self.always_on_top_vertex_buffer.buffer().expect(
                    "positive always-on-top selection outline vertex count requires an uploaded buffer",
                ),
                self.always_on_top_vertex_count,
            )
        })
    }
}

pub struct BlockSelectionGpu {
    state: Option<BlockSelectionRenderState>,
    secondary_vertex_buffer: FrameDataBuffer,
    secondary_vertex_count: u32,
    vertex_buffer: FrameDataBuffer,
    vertex_count: u32,
}

impl Default for BlockSelectionGpu {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockSelectionGpu {
    pub fn new() -> Self {
        Self {
            state: None,
            secondary_vertex_buffer: FrameDataBuffer::vertex(
                "bbb-block-selection-secondary-vertices",
            ),
            secondary_vertex_count: 0,
            vertex_buffer: FrameDataBuffer::vertex("bbb-block-selection-vertices"),
            vertex_count: 0,
        }
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        state: Option<BlockSelectionRenderState>,
    ) {
        if self.state == state {
            return;
        }
        if let Some(ref state) = state {
            let (secondary_vertices, vertices) = block_selection_vertices(state);
            self.secondary_vertex_count = if self.secondary_vertex_buffer.upload(
                device,
                queue,
                bytemuck::cast_slice(&secondary_vertices),
            ) {
                secondary_vertices.len() as u32
            } else {
                0
            };
            self.vertex_count =
                if self
                    .vertex_buffer
                    .upload(device, queue, bytemuck::cast_slice(&vertices))
                {
                    vertices.len() as u32
                } else {
                    0
                };
        } else {
            self.secondary_vertex_count = 0;
            self.vertex_count = 0;
        }
        self.state = state;
    }

    pub fn is_active(&self) -> bool {
        self.state.is_some() && (self.vertex_count > 0 || self.secondary_vertex_count > 0)
    }

    pub fn has_phase(&self, phase: BlockSelectionPhase) -> bool {
        self.is_active()
            && self
                .state
                .as_ref()
                .is_some_and(|state| state.phase == phase)
    }

    pub fn secondary_vertices(&self) -> Option<(&wgpu::Buffer, u32)> {
        (self.secondary_vertex_count > 0).then(|| {
            (
                self.secondary_vertex_buffer.buffer().expect(
                    "positive secondary block selection vertex count requires an uploaded buffer",
                ),
                self.secondary_vertex_count,
            )
        })
    }

    pub fn vertices(&self) -> Option<(&wgpu::Buffer, u32)> {
        (self.vertex_count > 0).then(|| {
            (
                self.vertex_buffer
                    .buffer()
                    .expect("positive block selection vertex count requires an uploaded buffer"),
                self.vertex_count,
            )
        })
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SelectionVertex {
    position: [f32; 3],
    line_direction: [f32; 3],
    line_width: f32,
    line_side: f32,
    color: [f32; 4],
}

pub fn create_selection_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::render_core::core_shaders::AdaptedLineShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::render_core::core_shaders::create_selected_line_pipeline(
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

pub fn create_translucent_debug_line_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::render_core::core_shaders::AdaptedLineShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::render_core::core_shaders::create_selected_line_pipeline(
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
pub struct DebugLineVertexBatches {
    pub opaque: Vec<SelectionVertex>,
    pub translucent: Vec<SelectionVertex>,
}

pub fn sanitize_debug_lines(mut lines: Vec<DebugLine>) -> Vec<DebugLine> {
    lines.retain(|line| line.sanitized().is_some());
    lines
}

pub fn debug_line_vertex_batches(
    lines: &[DebugLine],
    camera_pose: crate::render_core::CameraPose,
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
    camera_pose: crate::render_core::CameraPose,
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

pub fn create_chunk_border_always_on_top_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::render_core::core_shaders::AdaptedLineShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::render_core::core_shaders::create_selected_line_pipeline(
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

pub fn selection_lines_vertices(lines: &[SelectionLine]) -> Vec<SelectionVertex> {
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
        const {
            assert!(SELECTION_LINES_DEPTH_WRITE);
            assert!(!TRANSLUCENT_DEBUG_LINES_DEPTH_WRITE);
        }
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
        let camera = crate::render_core::CameraPose {
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
    fn chunk_border_pipeline_models_vanilla_always_on_top() {
        // Vanilla `ChunkBorderRenderer.emitGizmos` marks the current-section
        // cuboid with `setAlwaysOnTop()`. The dedicated bbb debug pipeline
        // mirrors that by omitting depth testing for that split primitive set.
        const {
            assert!(!CHUNK_BORDER_ALWAYS_ON_TOP_DEPTH_TEST);
        }
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
