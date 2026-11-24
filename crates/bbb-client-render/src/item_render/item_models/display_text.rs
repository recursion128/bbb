//! Vanilla `TextDisplay` layout, geometry, and dedicated render pipelines.
//!
//! `DisplayRenderer.TextDisplayRenderer` wraps a component with `Font.split`,
//! lays the resulting lines out in a 10-pixel grid, and submits an optional
//! untextured background before font glyphs. Normal text uses the
//! `text_polygon_offset` depth state; see-through text disables depth and fog.

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, OnceLock},
};

use anyhow::Result;
use bbb_render_types::HudFontGlyphBook;
use glam::{Mat4, Vec3};

use crate::item_render::pipeline_builder::{depth_stencil_state, RenderPipelineBuilder};
pub use crate::text_mesh::WorldTextRun;
use crate::text_mesh::{
    argb_color, mesh_world_text_line, split_world_text_lines, world_text_argb,
    world_text_visual_order, world_text_width, TextLayoutCache, TextMesh, TextPen, UNTEXTURED_UVS,
};

use super::{item_model_vertex_layout, ItemModelMesh, ItemModelVertex};
use crate::item_render::gpu::DEPTH_FORMAT;

pub const DISPLAY_TEXT_LINE_HEIGHT: i32 = 10;
pub const DISPLAY_TEXT_DEFAULT_BACKGROUND_OPACITY: f32 = 0.25;
pub const DISPLAY_TEXT_FONT_TEXTURE: &str = "minecraft:font/default";

const DISPLAY_TEXT_FLAG_SHADOW: u8 = 1;
const DISPLAY_TEXT_FLAG_SEE_THROUGH: u8 = 2;
const DISPLAY_TEXT_FLAG_DEFAULT_BACKGROUND: u8 = 4;
const DISPLAY_TEXT_FLAG_ALIGN_LEFT: u8 = 8;
const DISPLAY_TEXT_FLAG_ALIGN_RIGHT: u8 = 16;
const TEXT_POLYGON_OFFSET_DEPTH_BIAS: wgpu::DepthBiasState = wgpu::DepthBiasState {
    constant: -10,
    slope_scale: -1.0,
    clamp: 0.0,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayTextAlignment {
    Center,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayTextRenderType {
    Background,
    BackgroundSeeThrough,
    PolygonOffset,
    SeeThrough,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayTextLayerSubmission {
    pub texture: Option<&'static str>,
    pub render_type: DisplayTextRenderType,
    pub tint: [f32; 4],
    pub light: [f32; 2],
    pub overlay: [f32; 2],
    pub outline_color: Option<u32>,
    pub order: u32,
    pub submit_sequence: u64,
}

impl DisplayTextAlignment {
    fn from_flags(flags: u8) -> Self {
        if flags & DISPLAY_TEXT_FLAG_ALIGN_LEFT != 0 {
            Self::Left
        } else if flags & DISPLAY_TEXT_FLAG_ALIGN_RIGHT != 0 {
            Self::Right
        } else {
            Self::Center
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayTextLine {
    pub runs: Vec<WorldTextRun>,
    pub width: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayTextSubmission {
    pub entity_id: i32,
    pub line_width: u32,
    pub content_width: i32,
    pub content_height: i32,
    pub alignment: DisplayTextAlignment,
    pub shadow: bool,
    pub see_through: bool,
    pub background_argb: u32,
    pub text_alpha: u8,
    pub light: [f32; 2],
    pub transform: Mat4,
    pub lines: Vec<DisplayTextLine>,
    pub background_layer: Option<DisplayTextLayerSubmission>,
    pub text_layer: DisplayTextLayerSubmission,
}

#[derive(Debug, Clone)]
pub struct DisplayTextSurface {
    pub submission: DisplayTextSubmission,
    dynamic_atlas_dims: Option<(u32, u32)>,
    background: Arc<TextMesh>,
    text: Arc<TextMesh>,
    world_background: OnceLock<ItemModelMesh>,
    world_text: OnceLock<ItemModelMesh>,
}

impl PartialEq for DisplayTextSurface {
    fn eq(&self, other: &Self) -> bool {
        self.submission == other.submission
            && self.dynamic_atlas_dims == other.dynamic_atlas_dims
            && self.background == other.background
            && self.text == other.text
    }
}

impl DisplayTextSurface {
    pub fn is_empty(&self) -> bool {
        self.background.0.is_empty() && self.text.0.is_empty()
    }

    pub(crate) fn rescale_for_atlas_dims(&mut self, new_dims: (u32, u32)) -> bool {
        let Some((old_w, old_h)) = self.dynamic_atlas_dims else {
            return false;
        };
        if (old_w, old_h) == new_dims || new_dims.0 == 0 || new_dims.1 == 0 {
            return false;
        }
        let scale_x = old_w as f32 / new_dims.0 as f32;
        let scale_y = old_h as f32 / new_dims.1 as f32;
        self.text = Arc::new(self.text.rescaled_for_atlas(scale_x, scale_y));
        self.world_text = OnceLock::new();
        self.dynamic_atlas_dims = Some(new_dims);
        true
    }

    pub fn background_vertex_count(&self) -> usize {
        self.background.0.vertex_count()
    }

    pub fn text_vertex_count(&self) -> usize {
        self.text.0.vertex_count()
    }

    pub(crate) fn same_draw(&self, other: &Self) -> bool {
        // Font reloads and fresh obfuscated glyphs replace the shared font-space meshes.
        self.submission == other.submission
            && Arc::ptr_eq(&self.background, &other.background)
            && Arc::ptr_eq(&self.text, &other.text)
    }

    pub(crate) fn retain_world_meshes(&mut self, previous: &mut Self) {
        if self.submission.transform == previous.submission.transform {
            if Arc::ptr_eq(&self.background, &previous.background) {
                self.world_background = std::mem::take(&mut previous.world_background);
            }
            if Arc::ptr_eq(&self.text, &previous.text) {
                self.world_text = std::mem::take(&mut previous.world_text);
            }
        }
    }

    fn background_mesh(&self) -> &ItemModelMesh {
        self.world_background
            .get_or_init(|| self.background.transformed(self.submission.transform))
    }

    fn text_mesh(&self) -> &ItemModelMesh {
        self.world_text
            .get_or_init(|| self.text.transformed(self.submission.transform))
    }
}

/// `StringSplitter.splitLines` of the display's component, each line with its width.
pub fn split_display_text_lines(
    runs: &[WorldTextRun],
    max_width: i32,
    fonts: &HudFontGlyphBook,
) -> Vec<DisplayTextLine> {
    split_world_text_lines(runs, max_width, fonts)
        .into_iter()
        .map(|runs| DisplayTextLine {
            width: world_text_width(crate::text_mesh::world_text_chars(&runs), fonts),
            runs,
        })
        .collect()
}

/// The font-space layout of one display, independent of where it is drawn.
struct DisplayTextLayout {
    lines: Vec<DisplayTextLine>,
    content_width: i32,
    content_height: i32,
    background: Arc<TextMesh>,
    text: Arc<TextMesh>,
}

#[derive(PartialEq, Eq, Hash)]
struct DisplayTextLayoutKey {
    fonts: u64,
    runs: Vec<WorldTextRun>,
    line_width: i32,
    background_argb: u32,
    text_opacity: i8,
    flags: u8,
    light: [u32; 2],
}

thread_local! {
    static DISPLAY_TEXT_LAYOUTS: RefCell<TextLayoutCache<DisplayTextLayoutKey, DisplayTextLayout>> =
        RefCell::new(TextLayoutCache::default());
}

pub(crate) fn next_display_text_layout_frame() {
    DISPLAY_TEXT_LAYOUTS.with(|cache| cache.borrow_mut().next_frame());
}

/// Bakes one `TextDisplay` under its already-resolved common Display transform.
#[allow(clippy::too_many_arguments)]
pub fn bake_display_text_surface(
    entity_id: i32,
    submit_sequence: u64,
    outline_color: Option<u32>,
    display_transform: Mat4,
    runs: &[WorldTextRun],
    line_width: i32,
    background_color: i32,
    text_opacity: i8,
    flags: i8,
    light: [f32; 2],
    fonts: &HudFontGlyphBook,
    obfuscated_seed: u64,
) -> Option<DisplayTextSurface> {
    let flags = flags as u8;
    let background_argb = if flags & DISPLAY_TEXT_FLAG_DEFAULT_BACKGROUND != 0 {
        ((DISPLAY_TEXT_DEFAULT_BACKGROUND_OPACITY * 255.0) as u32) << 24
    } else {
        background_color as u32
    };
    let text_alpha = text_opacity as u8;
    let alignment = DisplayTextAlignment::from_flags(flags);
    let shadow = flags & DISPLAY_TEXT_FLAG_SHADOW != 0;
    let build = || {
        layout_display_text(
            runs,
            line_width,
            background_argb,
            text_alpha,
            alignment,
            shadow,
            light,
            fonts,
            obfuscated_seed,
        )
    };
    let build_consistent = || {
        let dims_before = fonts.dynamic_atlas().map(|atlas| atlas.atlas_dimensions());
        let built = build();
        let dims_after = fonts.dynamic_atlas().map(|atlas| atlas.atlas_dimensions());
        if dims_after != dims_before {
            (build(), dims_after)
        } else {
            (built, dims_after)
        }
    };
    let (layout, dynamic_atlas_dims) = if runs.iter().any(|run| run.style.obfuscated) {
        let (built, dims) = build_consistent();
        (Rc::new(built), dims)
    } else {
        let make_key = |fonts_gen| DisplayTextLayoutKey {
            fonts: fonts_gen,
            runs: runs.to_vec(),
            line_width,
            background_argb,
            text_opacity,
            flags,
            light: light.map(f32::to_bits),
        };
        let gen_before = fonts.generation();
        let cached = DISPLAY_TEXT_LAYOUTS.with(|cache| {
            cache
                .borrow_mut()
                .get_or_build(make_key(gen_before), &build)
        });
        let gen_after = fonts.generation();
        let dims_after = fonts.dynamic_atlas().map(|atlas| atlas.atlas_dimensions());
        if gen_after != gen_before {
            let rebuilt = DISPLAY_TEXT_LAYOUTS
                .with(|cache| cache.borrow_mut().get_or_build(make_key(gen_after), &build));
            (rebuilt, dims_after)
        } else {
            (cached, dims_after)
        }
    };
    if layout.lines.is_empty() {
        return None;
    }
    let see_through = flags & DISPLAY_TEXT_FLAG_SEE_THROUGH != 0;
    let transform = display_transform
        * Mat4::from_rotation_y(std::f32::consts::PI)
        * Mat4::from_scale(Vec3::splat(-0.025))
        * Mat4::from_translation(Vec3::new(
            1.0 - layout.content_width as f32 / 2.0,
            -(layout.content_height as f32),
            0.0,
        ));
    let surface = DisplayTextSurface {
        submission: DisplayTextSubmission {
            entity_id,
            line_width: line_width.max(1) as u32,
            content_width: layout.content_width,
            content_height: layout.content_height,
            alignment,
            shadow,
            see_through,
            background_argb,
            text_alpha,
            light,
            transform,
            lines: layout.lines.clone(),
            background_layer: (background_argb != 0).then(|| DisplayTextLayerSubmission {
                texture: None,
                render_type: if see_through {
                    DisplayTextRenderType::BackgroundSeeThrough
                } else {
                    DisplayTextRenderType::Background
                },
                tint: argb_color(background_argb),
                light,
                overlay: super::ITEM_MODEL_NO_OVERLAY,
                outline_color,
                order: 0,
                submit_sequence,
            }),
            text_layer: DisplayTextLayerSubmission {
                texture: Some(DISPLAY_TEXT_FONT_TEXTURE),
                render_type: if see_through {
                    DisplayTextRenderType::SeeThrough
                } else {
                    DisplayTextRenderType::PolygonOffset
                },
                tint: argb_color(world_text_argb(None, text_alpha)),
                light,
                overlay: super::ITEM_MODEL_NO_OVERLAY,
                outline_color,
                order: u32::from(background_argb != 0),
                submit_sequence,
            },
        },
        dynamic_atlas_dims,
        background: Arc::clone(&layout.background),
        text: Arc::clone(&layout.text),
        world_background: OnceLock::new(),
        world_text: OnceLock::new(),
    };
    (!surface.is_empty()).then_some(surface)
}

#[allow(clippy::too_many_arguments)]
fn layout_display_text(
    runs: &[WorldTextRun],
    line_width: i32,
    background_argb: u32,
    text_alpha: u8,
    alignment: DisplayTextAlignment,
    shadow: bool,
    light: [f32; 2],
    fonts: &HudFontGlyphBook,
    obfuscated_seed: u64,
) -> DisplayTextLayout {
    let lines = split_world_text_lines(runs, line_width, fonts);
    let visual_lines = lines
        .iter()
        .map(|line| world_text_visual_order(line, fonts))
        .collect::<Vec<_>>();
    let widths = visual_lines
        .iter()
        .map(|visual| world_text_width(visual.iter().map(|&(ch, run, _)| (ch, run)), fonts))
        .collect::<Vec<_>>();
    let content_width = widths.iter().copied().fold(0, i32::max);
    let content_height = lines.len() as i32 * DISPLAY_TEXT_LINE_HEIGHT - 1;
    let mut background = TextMesh::default();
    if background_argb != 0 && !lines.is_empty() {
        background.push_rect(
            [-1.0, -1.0, content_width as f32, content_height as f32],
            0.0,
            UNTEXTURED_UVS,
            background_argb,
            light,
        );
    }
    let mut text = TextMesh::default();
    let mut pen = TextPen::new(0.0, 0.0, obfuscated_seed);
    for (line_index, (visual, &width)) in visual_lines.iter().zip(&widths).enumerate() {
        pen.x = match alignment {
            DisplayTextAlignment::Left => 0.0,
            DisplayTextAlignment::Right => content_width.wrapping_sub(width) as f32,
            DisplayTextAlignment::Center => content_width as f32 / 2.0 - width as f32 / 2.0,
        };
        pen.y = (line_index as i32 * DISPLAY_TEXT_LINE_HEIGHT) as f32;
        mesh_world_text_line(
            &mut text,
            None,
            visual,
            &mut pen,
            fonts,
            |run| world_text_argb(run.color, text_alpha),
            shadow,
            light,
            UNTEXTURED_UVS,
        );
    }
    DisplayTextLayout {
        lines: visual_lines
            .iter()
            .zip(widths)
            .map(|(visual, width)| {
                let mut runs = Vec::new();
                for &(ch, run, _) in visual {
                    crate::text_mesh::push_world_text(&mut runs, run, ch.encode_utf8(&mut [0; 4]));
                }
                DisplayTextLine { runs, width }
            })
            .collect(),
        content_width,
        content_height,
        background: Arc::new(background),
        text: Arc::new(text),
    }
}

#[derive(Default)]
pub struct DisplayTextGeometry {
    pub normal_background: (Vec<ItemModelVertex>, Vec<u32>),
    pub order_zero_normal_text: (Vec<ItemModelVertex>, Vec<u32>),
    pub order_one_normal_text: (Vec<ItemModelVertex>, Vec<u32>),
    pub see_through_background: (Vec<ItemModelVertex>, Vec<u32>),
    pub order_zero_see_through_text: (Vec<ItemModelVertex>, Vec<u32>),
    pub order_one_see_through_text: (Vec<ItemModelVertex>, Vec<u32>),
}

pub(crate) struct WorldTextBuffers {
    pub(crate) vertices: crate::render_core::frame_buffers::FrameDataBuffer,
    pub(crate) indices: crate::render_core::frame_buffers::FrameDataBuffer,
    pub(crate) index_count: u32,
}

impl Default for WorldTextBuffers {
    fn default() -> Self {
        Self {
            vertices: crate::render_core::frame_buffers::FrameDataBuffer::vertex(
                "bbb-world-text-vertices",
            ),
            indices: crate::render_core::frame_buffers::FrameDataBuffer::index(
                "bbb-world-text-indices",
            ),
            index_count: 0,
        }
    }
}

impl WorldTextBuffers {
    pub(crate) fn collect_retire(mut self, buffers: &mut Vec<wgpu::Buffer>) {
        if let Some(buffer) = self.vertices.take_buffer() {
            buffers.push(buffer);
        }
        if let Some(buffer) = self.indices.take_buffer() {
            buffers.push(buffer);
        }
    }
}

impl WorldTextBuffers {
    pub(crate) fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[ItemModelVertex],
        indices: &[u32],
    ) {
        self.vertices
            .upload(device, queue, bytemuck::cast_slice(vertices));
        self.upload_indices(device, queue, indices);
    }

    pub(crate) fn upload_changed_vertices(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[ItemModelVertex],
        previous: &[ItemModelVertex],
        indices: &[u32],
    ) {
        self.vertices.upload_changed(
            device,
            queue,
            bytemuck::cast_slice(vertices),
            bytemuck::cast_slice(previous),
        );
        self.upload_indices(device, queue, indices);
    }

    pub(crate) fn upload_indices(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        indices: &[u32],
    ) {
        self.indices
            .upload(device, queue, bytemuck::cast_slice(indices));
        self.index_count = indices.len() as u32;
    }
}

pub(crate) struct DisplayTextRenderState {
    pub(crate) dirty: bool,
    pub(crate) camera: Option<[f32; 3]>,
    pub(crate) geometry: DisplayTextGeometry,
    pub(crate) buffers: [WorldTextBuffers; 6],
}

impl Default for DisplayTextRenderState {
    fn default() -> Self {
        Self {
            dirty: true,
            camera: None,
            geometry: DisplayTextGeometry::default(),
            buffers: std::array::from_fn(|_| WorldTextBuffers::default()),
        }
    }
}

pub fn append_mesh(target: &mut (Vec<ItemModelVertex>, Vec<u32>), mesh: &ItemModelMesh) {
    let base = u32::try_from(target.0.len()).expect("display text vertex count fits in u32");
    target.0.extend(mesh.vertices.iter().copied());
    target
        .1
        .extend(mesh.indices.iter().map(|index| index + base));
}

fn sort_text_mesh_indices(mesh: &mut (Vec<ItemModelVertex>, Vec<u32>), camera: Option<[f32; 3]>) {
    let camera = camera.map(Vec3::from_array);
    let mut quads = mesh
        .0
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .map(|(quad, vertices)| {
            let center = vertices
                .iter()
                .map(|vertex| Vec3::from_array(vertex.position))
                .sum::<Vec3>()
                * 0.25;
            (
                quad as u32 * 4,
                camera.map_or(0.0, |camera| center.distance_squared(camera)),
            )
        })
        .collect::<Vec<_>>();
    if camera.is_some() {
        quads.sort_by(|left, right| right.1.total_cmp(&left.1));
    }
    mesh.1.clear();
    for (base, _) in quads {
        mesh.1
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

pub(crate) fn sort_display_text_geometry(
    geometry: &mut DisplayTextGeometry,
    camera: Option<[f32; 3]>,
) {
    for mesh in [
        &mut geometry.normal_background,
        &mut geometry.order_zero_normal_text,
        &mut geometry.order_one_normal_text,
        &mut geometry.see_through_background,
    ] {
        sort_text_mesh_indices(mesh, camera);
    }
}

impl DisplayTextGeometry {
    pub(crate) fn meshes(&self) -> [&(Vec<ItemModelVertex>, Vec<u32>); 6] {
        [
            &self.order_zero_normal_text,
            &self.order_zero_see_through_text,
            &self.normal_background,
            &self.see_through_background,
            &self.order_one_normal_text,
            &self.order_one_see_through_text,
        ]
    }
}

pub fn merge_display_text_surfaces(
    surfaces: &[DisplayTextSurface],
    camera: Option<[f32; 3]>,
) -> DisplayTextGeometry {
    let mut geometry = DisplayTextGeometry::default();
    for surface in surfaces {
        let (background, order_zero_text, order_one_text) = if surface.submission.see_through {
            (
                &mut geometry.see_through_background,
                &mut geometry.order_zero_see_through_text,
                &mut geometry.order_one_see_through_text,
            )
        } else {
            (
                &mut geometry.normal_background,
                &mut geometry.order_zero_normal_text,
                &mut geometry.order_one_normal_text,
            )
        };
        append_mesh(background, surface.background_mesh());
        let text = if surface.submission.text_layer.order == 0 {
            order_zero_text
        } else {
            order_one_text
        };
        append_mesh(text, surface.text_mesh());
    }
    // Background and polygon-offset lanes sort on upload; see-through glyphs retain
    // submission order.
    if let Some(camera) = camera {
        sort_display_text_geometry(&mut geometry, Some(camera));
    }
    geometry
}

const DISPLAY_TEXT_SHADER: &str = concat!(
    crate::wgsl::camera!(),
    crate::wgsl::linear_fog_value!(),
    crate::wgsl::apply_fog!(),
    crate::wgsl::sample_lightmap!(),
    r#"
@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var font_atlas: texture_2d<f32>;
@group(0) @binding(2) var font_sampler: sampler;
@group(1) @binding(0) var lightmap_texture: texture_2d<f32>;
@group(1) @binding(1) var lightmap_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) light: vec2<f32>,
    @location(4) overlay: vec2<f32>,
    @location(5) normal_diffuse: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) light: vec2<f32>,
    @location(3) spherical_distance: f32,
    @location(4) cylindrical_distance: f32,
};

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
    let textured = input.uv.x >= 0.0;
    let sampled = select(vec4<f32>(1.0), textureSample(font_atlas, font_sampler, input.uv), textured);
    let color = sampled * input.color * vec4<f32>(sample_lightmap(input.light), 1.0);
    if color.a < 0.1 { discard; }
    return apply_fog(color, input.spherical_distance, input.cylindrical_distance);
}
"#,
);

const DISPLAY_TEXT_SEE_THROUGH_SHADER: &str = concat!(
    crate::wgsl::camera!(),
    r#"
@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var font_atlas: texture_2d<f32>;
@group(0) @binding(2) var font_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) light: vec2<f32>,
    @location(4) overlay: vec2<f32>,
    @location(5) normal_diffuse: vec4<f32>,
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
    let textured = input.uv.x >= 0.0;
    let sampled = select(vec4<f32>(1.0), textureSample(font_atlas, font_sampler, input.uv), textured);
    let color = sampled * input.color;
    if color.a < 0.1 { discard; }
    return color;
}
"#,
);

fn display_text_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    atlas_layout: &wgpu::BindGroupLayout,
    lightmap_layout: &wgpu::BindGroupLayout,
    label: &'static str,
    shader: &'static str,
    depth: Option<wgpu::DepthStencilState>,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, label)
        .shader("bbb-display-text-shader", shader)
        .layout(
            "bbb-display-text-pipeline-layout",
            &[atlas_layout, lightmap_layout],
        )
        .vertex_buffers(&[item_model_vertex_layout()])
        .color_target(format, Some(wgpu::BlendState::ALPHA_BLENDING))
        .cull_mode(None)
        .depth_stencil(depth)
        .build()
}

pub fn create_display_text_background_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    atlas_layout: &wgpu::BindGroupLayout,
    lightmap_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::item_render::core_shaders::AdaptedTextBackgroundShaders>,
) -> Result<wgpu::RenderPipeline> {
    let depth = depth_stencil_state(DEPTH_FORMAT, true, wgpu::CompareFunction::LessEqual);
    if let Some(shaders) = shaders {
        return crate::item_render::core_shaders::create_selected_text_pipeline(
            device,
            format,
            &[atlas_layout, lightmap_layout],
            "bbb-selected-display-text-background",
            &shaders.vertex,
            &shaders.fragment,
            item_model_vertex_layout(),
            Some(depth),
            None,
        );
    }
    Ok(display_text_pipeline(
        device,
        format,
        atlas_layout,
        lightmap_layout,
        "bbb-display-text-background-pipeline",
        DISPLAY_TEXT_SHADER,
        Some(depth),
    ))
}

pub fn create_display_text_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    atlas_layout: &wgpu::BindGroupLayout,
    lightmap_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::item_render::core_shaders::AdaptedTextShaders>,
) -> Result<wgpu::RenderPipeline> {
    let mut depth = depth_stencil_state(DEPTH_FORMAT, true, wgpu::CompareFunction::LessEqual);
    depth.bias = TEXT_POLYGON_OFFSET_DEPTH_BIAS;
    if let Some(shaders) = shaders {
        return crate::item_render::core_shaders::create_selected_text_pipeline(
            device,
            format,
            &[atlas_layout, lightmap_layout],
            "bbb-selected-display-text-polygon-offset",
            &shaders.display_vertex,
            &shaders.fragment,
            item_model_vertex_layout(),
            Some(depth),
            None,
        );
    }
    Ok(display_text_pipeline(
        device,
        format,
        atlas_layout,
        lightmap_layout,
        "bbb-display-text-polygon-offset-pipeline",
        DISPLAY_TEXT_SHADER,
        Some(depth),
    ))
}

pub fn create_display_text_see_through_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    atlas_layout: &wgpu::BindGroupLayout,
    lightmap_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::item_render::core_shaders::AdaptedTextShaders>,
) -> Result<wgpu::RenderPipeline> {
    // `RenderPipelines.TEXT_SEE_THROUGH` has no depth state; it still draws inside
    // passes with a depth attachment, so the format is declared and left untouched.
    let depth = depth_stencil_state(DEPTH_FORMAT, false, wgpu::CompareFunction::Always);
    if let Some(shaders) = shaders {
        return crate::item_render::core_shaders::create_selected_text_pipeline(
            device,
            format,
            &[atlas_layout, lightmap_layout],
            "bbb-selected-display-text-see-through",
            &shaders.see_through_vertex,
            &shaders.see_through_fragment,
            item_model_vertex_layout(),
            Some(depth),
            None,
        );
    }
    Ok(display_text_pipeline(
        device,
        format,
        atlas_layout,
        lightmap_layout,
        "bbb-display-text-see-through-pipeline",
        DISPLAY_TEXT_SEE_THROUGH_SHADER,
        Some(depth),
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use bbb_render_types::{HudAsciiGlyph, HudFontGlyphMap, HudTextStyle, HudUvRect};

    fn fonts() -> HudFontGlyphBook {
        let mut glyphs = HudFontGlyphMap::new();
        for ch in ['a', 'b', 'c', '?', ' '] {
            glyphs.insert_first_wins(
                ch,
                if ch == ' ' {
                    HudAsciiGlyph::empty(4.0)
                } else {
                    HudAsciiGlyph {
                        uv: HudUvRect {
                            min: [0.25, 0.5],
                            max: [0.3, 0.6],
                        },
                        width: 5,
                        height: 8,
                        advance: 6.0,
                        bearing_top: 7.0,
                        ..HudAsciiGlyph::default()
                    }
                },
            );
        }
        HudFontGlyphBook::from_fonts(
            "minecraft:default",
            BTreeMap::from([("minecraft:default".to_string(), glyphs)]),
        )
    }

    fn bidirectional_fonts() -> HudFontGlyphBook {
        let mut glyphs = HudFontGlyphMap::new();
        for (ch, min_x) in [('1', 0.1), ('2', 0.2), ('3', 0.3), ('-', 0.4), ('?', 0.5)] {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    uv: HudUvRect {
                        min: [min_x, 0.0],
                        max: [min_x + 0.05, 0.5],
                    },
                    width: 5,
                    height: 8,
                    advance: 6.0,
                    bearing_top: 7.0,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        let mut fonts = HudFontGlyphBook::from_fonts(
            "minecraft:default",
            BTreeMap::from([("minecraft:default".to_string(), glyphs)]),
        );
        fonts.set_default_right_to_left(true);
        fonts
    }

    #[test]
    fn display_text_submits_zero_bitmap_dimensions_but_empty_only_advances() {
        for (width, height, render_scale) in [(0, 8, 1.0), (5, 0, f32::INFINITY)] {
            let mut glyphs = HudFontGlyphMap::new();
            glyphs.insert_first_wins(' ', HudAsciiGlyph::empty(4.0));
            glyphs.insert_first_wins(
                'x',
                HudAsciiGlyph {
                    width,
                    height,
                    render_scale,
                    advance: 4.0,
                    ..Default::default()
                },
            );
            glyphs.insert_first_wins(
                'a',
                HudAsciiGlyph {
                    width: 5,
                    height: 8,
                    advance: 6.0,
                    ..Default::default()
                },
            );
            let fonts = HudFontGlyphBook::from_fonts(
                "minecraft:default",
                BTreeMap::from([("minecraft:default".to_string(), glyphs)]),
            );
            let bake = |text| {
                bake_display_text_surface(
                    1,
                    0,
                    None,
                    Mat4::IDENTITY,
                    &[WorldTextRun::plain(text)],
                    200,
                    0,
                    -1,
                    0,
                    [0.0, 1.0],
                    &fonts,
                    0,
                )
            };
            assert_eq!(bake("x").unwrap().text_mesh().vertices.len(), 4);
            assert!(bake(" ").is_none());
            let bitmap = bake("xa").unwrap();
            let empty = bake(" a").unwrap();
            assert_eq!(bitmap.submission.content_width, 10);
            assert_eq!(empty.submission.content_width, 10);
            assert_eq!(bitmap.text_mesh().vertices.len(), 8);
            assert_eq!(bitmap.text_mesh().indices.len(), 12);
            assert_eq!(empty.text_mesh().vertices.len(), 4);
            for (actual, expected) in bitmap.text_mesh().vertices[4..]
                .iter()
                .zip(&empty.text_mesh().vertices)
            {
                assert_eq!(actual.position, expected.position);
                assert_eq!(actual.uv, expected.uv);
            }
            let expected = empty
                .submission
                .transform
                .transform_point3(Vec3::new(4.0, 0.0, 0.0));
            assert!(
                (Vec3::from_array(empty.text_mesh().vertices[0].position) - expected).length()
                    < 1e-5
            );
            if height == 0 {
                assert!(bitmap.text_mesh().vertices[..4]
                    .iter()
                    .any(|vertex| vertex.position.iter().any(|value| value.is_nan())));
            }
        }
    }

    #[test]
    fn display_text_wraps_spaces_hard_breaks_and_terminal_newlines() {
        let fonts = fonts();
        let lines = split_display_text_lines(&[WorldTextRun::plain("aa bb")], 14, &fonts);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].runs[0].text, "aa");
        assert_eq!(lines[1].runs[0].text, "bb");

        let lines = split_display_text_lines(&[WorldTextRun::plain("abc")], 6, &fonts);
        assert_eq!(
            lines.iter().map(|line| line.width).collect::<Vec<_>>(),
            [6, 6, 6]
        );

        let lines = split_display_text_lines(&[WorldTextRun::plain("a\n")], 100, &fonts);
        assert_eq!(lines.len(), 2);
        assert!(lines[1].runs.is_empty());
    }

    #[test]
    fn display_text_wraps_logically_then_measures_and_draws_visual_lines() {
        let fonts = bidirectional_fonts();
        let logical = [WorldTextRun::plain("123-")];
        let split = split_display_text_lines(&logical, 200, &fonts);
        assert_eq!(split[0].runs[0].text, "123-");

        let surface = bake_display_text_surface(
            1,
            0,
            None,
            Mat4::IDENTITY,
            &logical,
            200,
            0,
            -1,
            0,
            [0.0, 1.0],
            &fonts,
            0,
        )
        .unwrap();
        assert_eq!(surface.submission.lines[0].runs[0].text, "-123");
        assert_eq!(surface.submission.lines[0].width, 24);
        let glyph_uvs = surface
            .text_mesh()
            .vertices
            .as_chunks::<4>()
            .0
            .iter()
            .map(|vertices| vertices[0].uv[0])
            .collect::<Vec<_>>();
        assert_eq!(glyph_uvs, [0.4, 0.1, 0.2, 0.3]);
    }

    #[test]
    fn display_text_bakes_vanilla_transform_background_alignment_and_opacity() {
        let surface = bake_display_text_surface(
            42,
            3,
            Some(0xFF11_2233),
            Mat4::from_translation(Vec3::new(2.0, 3.0, 4.0)),
            &[WorldTextRun::plain("ab\na")],
            200,
            0,
            -2,
            (DISPLAY_TEXT_FLAG_DEFAULT_BACKGROUND | DISPLAY_TEXT_FLAG_ALIGN_RIGHT) as i8,
            [0.25, 1.0],
            &fonts(),
            0,
        )
        .unwrap();
        assert_eq!(surface.submission.content_width, 12);
        assert_eq!(surface.submission.content_height, 19);
        assert_eq!(surface.submission.alignment, DisplayTextAlignment::Right);
        assert_eq!(surface.submission.text_alpha, 254);
        assert_eq!(surface.submission.background_argb, 0x3F00_0000);
        let background = surface.submission.background_layer.as_ref().unwrap();
        assert_eq!(background.render_type, DisplayTextRenderType::Background);
        assert_eq!(background.texture, None);
        assert_eq!(background.order, 0);
        assert_eq!(background.submit_sequence, 3);
        assert_eq!(background.outline_color, Some(0xFF11_2233));
        assert_eq!(
            surface.submission.text_layer.render_type,
            DisplayTextRenderType::PolygonOffset
        );
        assert_eq!(
            surface.submission.text_layer.texture,
            Some(DISPLAY_TEXT_FONT_TEXTURE)
        );
        assert_eq!(surface.submission.text_layer.order, 1);
        assert_eq!(surface.background_vertex_count(), 4);
        assert_eq!(surface.text_vertex_count(), 12);
        let expected_origin = Mat4::from_translation(Vec3::new(2.0, 3.0, 4.0))
            * Mat4::from_rotation_y(std::f32::consts::PI)
            * Mat4::from_scale(Vec3::splat(-0.025))
            * Mat4::from_translation(Vec3::new(-5.0, -19.0, 0.0));
        assert_eq!(surface.submission.transform, expected_origin);
        assert!(surface
            .text_mesh()
            .vertices
            .iter()
            .all(|vertex| (vertex.color[3] - 254.0 / 255.0).abs() < 1e-6));
    }

    #[test]
    fn display_text_bakes_shadow_bold_and_effect_geometry() {
        let runs = [WorldTextRun {
            shadow_color: None,
            text: "a".to_string(),
            style: HudTextStyle {
                bold: true,
                underlined: true,
                ..Default::default()
            },
            color: Some(0x80_40_20),
            font: None,
            event_id: None,
        }];
        let surface = bake_display_text_surface(
            7,
            0,
            None,
            Mat4::IDENTITY,
            &runs,
            200,
            0,
            -1,
            DISPLAY_TEXT_FLAG_SHADOW as i8,
            [0.0, 1.0],
            &fonts(),
            0,
        )
        .unwrap();
        // Bold glyph: shadow/main x two quads; underline: shadow/main quads.
        assert_eq!(surface.text_vertex_count(), 24);
        assert_eq!(surface.submission.lines[0].width, 7);
        assert_eq!(
            surface.text_mesh().vertices[0].color,
            [
                0x20 as f32 / 255.0,
                0x10 as f32 / 255.0,
                0x08 as f32 / 255.0,
                1.0,
            ]
        );
    }

    #[test]
    fn display_text_run_that_only_changes_its_event_restarts_the_local_index() {
        // `SubStringSource.substring` cuts a part where the resolved style
        // changes and vanilla `Style` equality counts the click, hover and
        // insertion events, so the local index
        // `bake_display_text_surface` feeds `styled_effect_rects` as vanilla's
        // `position` restarts at the event boundary.
        let underlined = |text: &str, event_id| WorldTextRun {
            text: text.to_string(),
            style: HudTextStyle {
                underlined: true,
                ..Default::default()
            },
            event_id,
            ..Default::default()
        };
        let order = |runs: [WorldTextRun; 2]| {
            crate::text_mesh::world_text_visual_order(&runs, &fonts())
                .iter()
                .map(|glyph| glyph.2)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            order([underlined("ab", None), underlined("ab", None)]),
            [0, 1, 2, 3]
        );
        assert_eq!(
            order([underlined("ab", None), underlined("ab", Some(0))]),
            [0, 1, 0, 1]
        );
    }

    #[test]
    fn display_text_uses_custom_font_metrics_and_frame_seeded_obfuscation() {
        let make_glyph = |advance, min_x| HudAsciiGlyph {
            uv: HudUvRect {
                min: [min_x, 0.0],
                max: [min_x + 0.1, 0.5],
            },
            width: 5,
            height: 8,
            advance,
            bearing_top: 7.0,
            ..HudAsciiGlyph::default()
        };
        let mut default = HudFontGlyphMap::new();
        default.insert_first_wins('a', make_glyph(6.0, 0.0));
        default.insert_first_wins('?', make_glyph(6.0, 0.1));
        let mut custom = HudFontGlyphMap::new();
        for (ch, min_x) in [('a', 0.4), ('b', 0.5), ('c', 0.6)] {
            custom.insert_first_wins(ch, make_glyph(8.0, min_x));
        }
        let missing_glyph = make_glyph(6.0, 0.8);
        default.set_missing_glyph(missing_glyph);
        custom.set_missing_glyph(missing_glyph);
        let fonts = HudFontGlyphBook::from_fonts(
            "minecraft:default",
            BTreeMap::from([
                ("example:custom".to_string(), custom),
                ("minecraft:default".to_string(), default),
            ]),
        );
        let runs = [WorldTextRun {
            shadow_color: None,
            text: "a".to_string(),
            style: HudTextStyle {
                obfuscated: true,
                ..Default::default()
            },
            color: None,
            font: Some("example:custom".to_string()),
            event_id: None,
        }];
        let mut seen_uvs = Vec::new();
        for seed in 0..32 {
            let surface = bake_display_text_surface(
                1,
                0,
                None,
                Mat4::IDENTITY,
                &runs,
                200,
                0,
                -1,
                0,
                [0.0, 1.0],
                &fonts,
                seed,
            )
            .unwrap();
            assert_eq!(surface.submission.content_width, 8);
            let uv = surface.text_mesh().vertices[0].uv;
            assert!((0.4..=0.6).contains(&uv[0]));
            if !seen_uvs.contains(&uv) {
                seen_uvs.push(uv);
            }
        }
        assert!(
            seen_uvs.len() > 1,
            "frame seeds should vary the equal-width glyph"
        );

        let missing = [WorldTextRun {
            text: "z".to_string(),
            font: Some("example:custom".to_string()),
            ..WorldTextRun::default()
        }];
        assert_eq!(
            crate::text_mesh::world_text_width(
                crate::text_mesh::world_text_chars(&missing),
                &fonts
            ),
            6
        );
        for font in [Some("example:custom"), Some("example:unknown")] {
            let mut missing_runs = missing.clone();
            missing_runs[0].font = font.map(str::to_owned);
            let surface = bake_display_text_surface(
                1,
                0,
                None,
                Mat4::IDENTITY,
                &missing_runs,
                200,
                0,
                -1,
                0,
                [0.0, 1.0],
                &fonts,
                0,
            )
            .unwrap();
            assert_eq!(surface.submission.content_width, 6);
            assert_eq!(surface.text_mesh().vertices[0].uv, missing_glyph.uv.min);
            assert_eq!(
                fonts.glyphs(Some("example:unknown")).resolve('a'),
                missing_glyph
            );
        }
    }

    #[test]
    fn display_text_empty_component_has_no_surface_but_newline_can_have_background() {
        assert!(bake_display_text_surface(
            1,
            0,
            None,
            Mat4::IDENTITY,
            &[],
            200,
            0x4000_0000,
            -1,
            0,
            [0.0, 1.0],
            &fonts(),
            0,
        )
        .is_none());
        let surface = bake_display_text_surface(
            1,
            0,
            None,
            Mat4::IDENTITY,
            &[WorldTextRun::plain("\n")],
            200,
            0x4000_0000,
            -1,
            0,
            [0.0, 1.0],
            &fonts(),
            0,
        )
        .unwrap();
        assert_eq!(surface.submission.lines.len(), 2);
        assert_eq!(surface.background_vertex_count(), 4);
        assert_eq!(surface.text_vertex_count(), 0);
    }

    #[test]
    fn display_text_pipelines_encode_vanilla_light_fog_and_depth_modes() {
        assert!(DISPLAY_TEXT_SHADER.contains("sample_lightmap(input.light)"));
        assert!(DISPLAY_TEXT_SHADER.contains("textureSample(lightmap_texture"));
        assert!(DISPLAY_TEXT_SHADER.contains("return apply_fog(color"));
        assert!(!DISPLAY_TEXT_SEE_THROUGH_SHADER.contains("sample_lightmap"));
        assert!(!DISPLAY_TEXT_SEE_THROUGH_SHADER.contains("apply_fog"));
        assert!(DISPLAY_TEXT_SHADER.contains("if color.a < 0.1 { discard; }"));
        assert!(DISPLAY_TEXT_SEE_THROUGH_SHADER.contains("if color.a < 0.1 { discard; }"));
        assert_eq!(TEXT_POLYGON_OFFSET_DEPTH_BIAS.constant, -10);
        assert_eq!(TEXT_POLYGON_OFFSET_DEPTH_BIAS.slope_scale, -1.0);
        assert_eq!(TEXT_POLYGON_OFFSET_DEPTH_BIAS.clamp, 0.0);
    }

    #[test]
    fn display_text_unknown_nonzero_order_falls_forward_to_the_later_lane() {
        let mut surface = bake_display_text_surface(
            1,
            0,
            None,
            Mat4::IDENTITY,
            &[WorldTextRun::plain("a")],
            200,
            0,
            -1,
            0,
            [0.0, 1.0],
            &fonts(),
            0,
        )
        .unwrap();
        surface.submission.text_layer.order = u32::MAX;

        let geometry = merge_display_text_surfaces(&[surface], None);
        assert!(geometry.order_zero_normal_text.0.is_empty());
        assert_eq!(geometry.order_one_normal_text.0.len(), 4);
    }

    #[test]
    fn display_text_upload_sorting_matches_each_vanilla_render_type() {
        let first_drawn_z =
            |mesh: &(Vec<ItemModelVertex>, Vec<u32>)| mesh.0[mesh.1[0] as usize].position[2];
        let make = |z: f32, flags: u8, sequence: u64, background_color: i32| {
            bake_display_text_surface(
                sequence as i32,
                sequence,
                None,
                Mat4::from_translation(Vec3::new(0.0, 0.0, z)),
                &[WorldTextRun::plain("a")],
                200,
                background_color,
                -1,
                flags as i8,
                [0.0, 1.0],
                &fonts(),
                0,
            )
            .unwrap()
        };
        let normal_order_one_near = make(1.0, 0, 0, 0x4000_0000);
        let normal_order_one_far = make(10.0, 0, 1, 0x4000_0000);
        let normal_order_zero_near = make(2.0, 0, 2, 0);
        let normal_order_zero_far = make(20.0, 0, 3, 0);
        assert_eq!(normal_order_one_near.submission.text_layer.order, 1);
        assert_eq!(normal_order_zero_near.submission.text_layer.order, 0);
        assert!(normal_order_zero_near.submission.background_layer.is_none());
        let geometry = merge_display_text_surfaces(
            &[
                normal_order_one_near,
                normal_order_one_far,
                normal_order_zero_near,
                normal_order_zero_far,
            ],
            Some([0.0, 0.0, 0.0]),
        );
        assert!((first_drawn_z(&geometry.normal_background) - 10.0).abs() < 1e-5);
        assert_eq!(geometry.normal_background.0.len(), 8);
        assert_eq!(geometry.order_one_normal_text.0.len(), 8);
        assert_eq!(geometry.order_zero_normal_text.0.len(), 8);
        assert!((first_drawn_z(&geometry.order_one_normal_text) - 10.0).abs() < 1e-5);
        assert!((first_drawn_z(&geometry.order_zero_normal_text) - 20.0).abs() < 1e-5);

        let see_order_one_near = make(1.0, DISPLAY_TEXT_FLAG_SEE_THROUGH, 0, 0x4000_0000);
        let see_order_one_far = make(10.0, DISPLAY_TEXT_FLAG_SEE_THROUGH, 1, 0x4000_0000);
        let see_order_zero_near = make(2.0, DISPLAY_TEXT_FLAG_SEE_THROUGH, 2, 0);
        let see_order_zero_far = make(20.0, DISPLAY_TEXT_FLAG_SEE_THROUGH, 3, 0);
        assert_eq!(see_order_one_near.submission.text_layer.order, 1);
        assert_eq!(see_order_zero_near.submission.text_layer.order, 0);
        let geometry = merge_display_text_surfaces(
            &[
                see_order_one_near,
                see_order_one_far,
                see_order_zero_near,
                see_order_zero_far,
            ],
            Some([0.0, 0.0, 0.0]),
        );
        // BackgroundSeeThrough sorts on upload, TextSeeThrough does not.
        assert!((first_drawn_z(&geometry.see_through_background) - 10.0).abs() < 1e-5);
        assert_eq!(geometry.see_through_background.0.len(), 8);
        assert_eq!(geometry.order_one_see_through_text.0.len(), 8);
        assert_eq!(geometry.order_zero_see_through_text.0.len(), 8);
        assert!((first_drawn_z(&geometry.order_one_see_through_text) - 1.0).abs() < 1e-5);
        assert!((first_drawn_z(&geometry.order_zero_see_through_text) - 2.0).abs() < 1e-5);
    }
    #[test]
    fn fractional_display_metrics_preserve_signed_lines_and_geometry() {
        let mut glyphs = HudFontGlyphMap::new();
        for (ch, advance) in [('a', 0.5), ('b', -3.0)] {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    advance,
                    width: 1,
                    height: 8,
                    bold_offset: 0.5,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        let fonts = HudFontGlyphBook::from(glyphs);
        for (text, bold, expected_width, expected_delta) in [
            ("aa", false, 1, 0.5),
            ("ba", false, -2, -3.0),
            ("aa", true, 2, 1.0),
        ] {
            let mut run = WorldTextRun::plain(text);
            run.style.bold = bold;
            let lines = split_display_text_lines(&[run.clone()], 100, &fonts);
            assert_eq!(lines[0].width, expected_width);
            let surface = bake_display_text_surface(
                1,
                0,
                None,
                Mat4::IDENTITY,
                &[run],
                100,
                0,
                -1,
                0,
                [0.0, 1.0],
                &fonts,
                0,
            )
            .unwrap();
            assert_eq!(surface.submission.content_width, expected_width.max(0));
            let first = surface.text_mesh().vertices[0].position[0];
            let second = surface.text_mesh().vertices[if bold { 8 } else { 4 }].position[0];
            assert!((second - first - expected_delta * 0.025).abs() < 0.00001);
        }
    }

    #[test]
    fn display_text_shaders_use_current_camera_uniform_layout() {
        for shader in [DISPLAY_TEXT_SHADER, DISPLAY_TEXT_SEE_THROUGH_SHADER] {
            for field in [
                "block_light_tint: vec4<f32>,",
                "sky_light_color: vec4<f32>,",
                "ambient_color: vec4<f32>,",
                "night_vision_color: vec4<f32>,",
                "camera_position: vec4<f32>,",
                "fog_color: vec4<f32>,",
                "fog_distances: vec4<f32>,",
            ] {
                assert!(
                    shader.contains(field),
                    "missing {field} in display text shader"
                );
            }
        }
    }
}

#[cfg(test)]
#[path = "display_text/shadow_tests.rs"]
mod shadow_tests;
