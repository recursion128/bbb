//! Vanilla `TextDisplay` layout, geometry, and dedicated render pipelines.
//!
//! `DisplayRenderer.TextDisplayRenderer` wraps a component with `Font.split`,
//! lays the resulting lines out in a 10-pixel grid, and submits an optional
//! untextured background before font glyphs. Normal text uses the
//! `text_polygon_offset` depth state; see-through text disables depth and fog.

use glam::{Mat4, Vec3};

use crate::pipeline_builder::{depth_stencil_state, RenderPipelineBuilder};
use crate::{HudAsciiGlyph, HudFontGlyphMap, HudStyledTextRun, Renderer};

use super::{item_model_vertex_layout, ItemModelMesh, ItemModelVertex};
use crate::gpu::DEPTH_FORMAT;

pub const DISPLAY_TEXT_LINE_HEIGHT: i32 = 10;
pub const DISPLAY_TEXT_DEFAULT_BACKGROUND_OPACITY: f32 = 0.25;
pub const DISPLAY_TEXT_FONT_TEXTURE: &str = "minecraft:font/default";

const DISPLAY_TEXT_FLAG_SHADOW: u8 = 1;
const DISPLAY_TEXT_FLAG_SEE_THROUGH: u8 = 2;
const DISPLAY_TEXT_FLAG_DEFAULT_BACKGROUND: u8 = 4;
const DISPLAY_TEXT_FLAG_ALIGN_LEFT: u8 = 8;
const DISPLAY_TEXT_FLAG_ALIGN_RIGHT: u8 = 16;
const DISPLAY_TEXT_REPLACEMENT_GLYPH: char = '?';
const DISPLAY_TEXT_UNTEXTURED_UV: [f32; 2] = [-1.0, -1.0];

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
    pub runs: Vec<HudStyledTextRun>,
    pub width: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayTextSubmission {
    pub entity_id: i32,
    pub line_width: u32,
    pub content_width: u32,
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

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayTextSurface {
    pub submission: DisplayTextSubmission,
    background: ItemModelMesh,
    text: ItemModelMesh,
}

impl DisplayTextSurface {
    pub fn is_empty(&self) -> bool {
        self.background.is_empty() && self.text.is_empty()
    }

    pub fn background_vertex_count(&self) -> usize {
        self.background.vertex_count()
    }

    pub fn text_vertex_count(&self) -> usize {
        self.text.vertex_count()
    }
}

pub(super) fn display_text_glyph(ch: char, glyphs: &HudFontGlyphMap) -> HudAsciiGlyph {
    glyphs
        .get(ch)
        .or_else(|| glyphs.get(DISPLAY_TEXT_REPLACEMENT_GLYPH))
        .unwrap_or_default()
}

pub(super) fn display_text_runs_width(runs: &[HudStyledTextRun], glyphs: &HudFontGlyphMap) -> u32 {
    runs.iter()
        .map(|run| {
            run.text
                .chars()
                .map(|ch| display_text_glyph(ch, glyphs).styled_advance(run.style))
                .sum::<u32>()
        })
        .sum()
}

fn push_styled_char(output: &mut Vec<HudStyledTextRun>, source: &HudStyledTextRun, ch: char) {
    match output.last_mut() {
        Some(last) if last.style == source.style && last.color == source.color => {
            last.text.push(ch);
        }
        _ => output.push(HudStyledTextRun {
            text: ch.to_string(),
            style: source.style,
            color: source.color,
        }),
    }
}

/// Style-preserving transcription of `StringSplitter.splitLines` for the
/// already flattened component runs used by the native renderer. The width is
/// clamped to one pixel, spaces/newlines at a break are consumed, the first
/// non-zero-width glyph may exceed the limit, and a terminal newline produces
/// the final empty line just like vanilla.
pub fn split_display_text_lines(
    runs: &[HudStyledTextRun],
    max_width: i32,
    glyphs: &HudFontGlyphMap,
) -> Vec<DisplayTextLine> {
    let chars: Vec<(usize, char)> = runs
        .iter()
        .enumerate()
        .flat_map(|(run_index, run)| run.text.chars().map(move |ch| (run_index, ch)))
        .collect();
    if chars.is_empty() {
        return Vec::new();
    }

    let max_width = max_width.max(1) as u32;
    let mut lines = Vec::new();
    let mut start = 0usize;
    while start < chars.len() {
        let mut width = 0u32;
        let mut had_non_zero_width = false;
        let mut last_space = None;
        let mut split = None;
        let mut index = start;
        while index < chars.len() {
            let (run_index, ch) = chars[index];
            if ch == '\n' {
                split = Some((index, 1usize, true));
                break;
            }
            if ch == ' ' {
                last_space = Some(index);
            }
            let advance = display_text_glyph(ch, glyphs).styled_advance(runs[run_index].style);
            width = width.saturating_add(advance);
            if had_non_zero_width && width > max_width {
                let split_at = last_space.unwrap_or(index);
                let skip = usize::from(chars[split_at].1 == ' ');
                split = Some((split_at, skip, false));
                break;
            }
            had_non_zero_width |= advance != 0;
            index += 1;
        }

        let (end, skip, newline) = split.unwrap_or((chars.len(), 0, false));
        let mut line_runs = Vec::new();
        for &(run_index, ch) in &chars[start..end] {
            push_styled_char(&mut line_runs, &runs[run_index], ch);
        }
        let line_width = display_text_runs_width(&line_runs, glyphs);
        lines.push(DisplayTextLine {
            runs: line_runs,
            width: line_width,
        });
        if end == chars.len() {
            break;
        }
        start = end + skip;
        if newline && start == chars.len() {
            lines.push(DisplayTextLine {
                runs: Vec::new(),
                width: 0,
            });
        }
    }
    lines
}

pub(super) fn argb_to_color(argb: u32) -> [f32; 4] {
    [
        ((argb >> 16) & 0xFF) as f32 / 255.0,
        ((argb >> 8) & 0xFF) as f32 / 255.0,
        (argb & 0xFF) as f32 / 255.0,
        ((argb >> 24) & 0xFF) as f32 / 255.0,
    ]
}

pub(super) fn text_argb(rgb: Option<u32>, alpha: u8) -> u32 {
    u32::from(alpha) << 24 | rgb.unwrap_or(0xFF_FF_FF) & 0xFF_FF_FF
}

fn shadow_argb(argb: u32) -> u32 {
    let scale = |shift: u32| (((argb >> shift) & 0xFFu32) as f32 * 0.25) as u32;
    argb & 0xFF00_0000 | scale(16) << 16 | scale(8) << 8 | scale(0)
}

pub(super) fn append_quad(
    mesh: &mut ItemModelMesh,
    transform: Mat4,
    corners: [[f32; 3]; 4],
    uvs: [[f32; 2]; 4],
    color: [f32; 4],
    light: [f32; 2],
) {
    mesh.append_raw_textured_quad(
        corners.map(|corner| {
            transform
                .transform_point3(Vec3::from_array(corner))
                .to_array()
        }),
        uvs,
        color,
        light,
    );
}

pub(super) fn append_untextured_rect(
    mesh: &mut ItemModelMesh,
    transform: Mat4,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    z: f32,
    color: [f32; 4],
    light: [f32; 2],
) {
    append_quad(
        mesh,
        transform,
        [[x0, y0, z], [x0, y1, z], [x1, y1, z], [x1, y0, z]],
        [DISPLAY_TEXT_UNTEXTURED_UV; 4],
        color,
        light,
    );
}

/// Bakes one `TextDisplay` under its already-resolved common Display transform.
#[allow(clippy::too_many_arguments)]
pub fn bake_display_text_surface(
    entity_id: i32,
    submit_sequence: u64,
    outline_color: Option<u32>,
    display_transform: Mat4,
    runs: &[HudStyledTextRun],
    line_width: i32,
    background_color: i32,
    text_opacity: i8,
    flags: i8,
    light: [f32; 2],
    glyphs: &HudFontGlyphMap,
) -> Option<DisplayTextSurface> {
    let lines = split_display_text_lines(runs, line_width, glyphs);
    if lines.is_empty() {
        return None;
    }
    let flags = flags as u8;
    let alignment = DisplayTextAlignment::from_flags(flags);
    let shadow = flags & DISPLAY_TEXT_FLAG_SHADOW != 0;
    let see_through = flags & DISPLAY_TEXT_FLAG_SEE_THROUGH != 0;
    let background_argb = if flags & DISPLAY_TEXT_FLAG_DEFAULT_BACKGROUND != 0 {
        ((DISPLAY_TEXT_DEFAULT_BACKGROUND_OPACITY * 255.0) as u32) << 24
    } else {
        background_color as u32
    };
    let text_alpha = text_opacity as u8;
    let content_width = lines.iter().map(|line| line.width).max().unwrap_or(0);
    let content_height = lines.len() as i32 * DISPLAY_TEXT_LINE_HEIGHT - 1;
    let transform = display_transform
        * Mat4::from_rotation_y(std::f32::consts::PI)
        * Mat4::from_scale(Vec3::splat(-0.025))
        * Mat4::from_translation(Vec3::new(
            1.0 - content_width as f32 / 2.0,
            -(content_height as f32),
            0.0,
        ));

    let mut background = ItemModelMesh::new();
    if background_argb != 0 {
        append_untextured_rect(
            &mut background,
            transform,
            -1.0,
            -1.0,
            content_width as f32,
            content_height as f32,
            0.0,
            argb_to_color(background_argb),
            light,
        );
    }

    let mut text = ItemModelMesh::new();
    for (line_index, line) in lines.iter().enumerate() {
        let mut pen_x = match alignment {
            DisplayTextAlignment::Left => 0.0,
            DisplayTextAlignment::Right => (content_width - line.width) as f32,
            DisplayTextAlignment::Center => content_width as f32 / 2.0 - line.width as f32 / 2.0,
        };
        let y = (line_index as i32 * DISPLAY_TEXT_LINE_HEIGHT) as f32;
        let mut first_in_line = true;
        let mut effects = Vec::new();
        for run in &line.runs {
            let argb = text_argb(run.color, text_alpha);
            let main_color = argb_to_color(argb);
            let shadow_color = argb_to_color(shadow_argb(argb));
            for ch in run.text.chars() {
                let glyph = display_text_glyph(ch, glyphs);
                if glyph.width > 0 && glyph.height > 0 {
                    for quad in glyph.styled_quads(pen_x, y, run.style, shadow) {
                        let color = if quad.shadow {
                            shadow_color
                        } else {
                            main_color
                        };
                        append_quad(
                            &mut text,
                            transform,
                            quad.corners.map(|[x, y]| [x, y, 0.0]),
                            [
                                [quad.uv.min[0], quad.uv.min[1]],
                                [quad.uv.min[0], quad.uv.max[1]],
                                [quad.uv.max[0], quad.uv.max[1]],
                                [quad.uv.max[0], quad.uv.min[1]],
                            ],
                            color,
                            light,
                        );
                    }
                }
                for rect in glyph.styled_effect_rects(pen_x, y, run.style, first_in_line) {
                    effects.push((rect, main_color, shadow_color));
                }
                pen_x += glyph.styled_advance(run.style) as f32;
                first_in_line = false;
            }
        }
        // `PreparedText.visit`: glyphs precede underline/strikethrough effect
        // sprites, with the effect z lifted by 0.01 font pixel.
        for (rect, main_color, shadow_color) in effects {
            if shadow {
                append_untextured_rect(
                    &mut text,
                    transform,
                    rect.x0 + 1.0,
                    rect.y0 + 1.0,
                    rect.x1 + 1.0,
                    rect.y1 + 1.0,
                    0.01,
                    shadow_color,
                    light,
                );
            }
            append_untextured_rect(
                &mut text, transform, rect.x0, rect.y0, rect.x1, rect.y1, 0.01, main_color, light,
            );
        }
    }

    let surface = DisplayTextSurface {
        submission: DisplayTextSubmission {
            entity_id,
            line_width: line_width.max(1) as u32,
            content_width,
            content_height,
            alignment,
            shadow,
            see_through,
            background_argb,
            text_alpha,
            light,
            transform,
            lines,
            background_layer: (background_argb != 0).then(|| DisplayTextLayerSubmission {
                texture: None,
                render_type: if see_through {
                    DisplayTextRenderType::BackgroundSeeThrough
                } else {
                    DisplayTextRenderType::Background
                },
                tint: argb_to_color(background_argb),
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
                tint: argb_to_color(text_argb(None, text_alpha)),
                light,
                overlay: super::ITEM_MODEL_NO_OVERLAY,
                outline_color,
                order: u32::from(background_argb != 0),
                submit_sequence,
            },
        },
        background,
        text,
    };
    (!surface.is_empty()).then_some(surface)
}

#[derive(Default)]
pub(crate) struct DisplayTextGeometry {
    pub(crate) normal_background: (Vec<ItemModelVertex>, Vec<u32>),
    pub(crate) normal_text: (Vec<ItemModelVertex>, Vec<u32>),
    pub(crate) see_through_background: (Vec<ItemModelVertex>, Vec<u32>),
    pub(crate) see_through_text: (Vec<ItemModelVertex>, Vec<u32>),
}

pub(super) fn append_mesh(target: &mut (Vec<ItemModelVertex>, Vec<u32>), mesh: &ItemModelMesh) {
    let base = u32::try_from(target.0.len()).expect("display text vertex count fits in u32");
    target.0.extend(mesh.vertices.iter().copied());
    target
        .1
        .extend(mesh.indices.iter().map(|index| index + base));
}

fn append_sorted_meshes<'a>(
    target: &mut (Vec<ItemModelVertex>, Vec<u32>),
    meshes: impl IntoIterator<Item = &'a ItemModelMesh>,
    camera: Option<[f32; 3]>,
) {
    let Some(camera) = camera.map(Vec3::from_array) else {
        for mesh in meshes {
            append_mesh(target, mesh);
        }
        return;
    };
    let mut quads: Vec<[ItemModelVertex; 4]> = meshes
        .into_iter()
        .flat_map(|mesh| {
            debug_assert_eq!(mesh.vertices.len() % 4, 0);
            debug_assert_eq!(mesh.indices.len() / 6, mesh.vertices.len() / 4);
            mesh.vertices
                .chunks_exact(4)
                .map(|quad| <[ItemModelVertex; 4]>::try_from(quad).expect("four-vertex quad"))
        })
        .collect();
    quads.sort_by(|left, right| {
        let distance = |quad: &[ItemModelVertex; 4]| {
            let center = quad
                .iter()
                .map(|vertex| Vec3::from_array(vertex.position))
                .sum::<Vec3>()
                * 0.25;
            center.distance_squared(camera)
        };
        distance(right).total_cmp(&distance(left))
    });
    for quad in quads {
        let base = u32::try_from(target.0.len()).expect("display text vertex count fits in u32");
        target.0.extend(quad);
        target
            .1
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

pub(crate) fn merge_display_text_surfaces(
    surfaces: &[DisplayTextSurface],
    camera: Option<[f32; 3]>,
) -> DisplayTextGeometry {
    let mut geometry = DisplayTextGeometry::default();
    let mut normal_background = Vec::new();
    let mut normal_text = Vec::new();
    let mut see_through_background = Vec::new();
    let mut see_through_text = Vec::new();
    for surface in surfaces {
        let (background, text) = if surface.submission.see_through {
            (&mut see_through_background, &mut see_through_text)
        } else {
            (&mut normal_background, &mut normal_text)
        };
        background.push(&surface.background);
        text.push(&surface.text);
    }
    // `textBackground`, `textPolygonOffset`, and
    // `textBackgroundSeeThrough` all set `sortOnUpload`; `textSeeThrough`
    // deliberately preserves collector submission order.
    append_sorted_meshes(&mut geometry.normal_background, normal_background, camera);
    append_sorted_meshes(&mut geometry.normal_text, normal_text, camera);
    append_sorted_meshes(
        &mut geometry.see_through_background,
        see_through_background,
        camera,
    );
    for mesh in see_through_text {
        append_mesh(&mut geometry.see_through_text, mesh);
    }
    geometry
}

impl Renderer {
    pub fn set_display_text_surfaces(&mut self, surfaces: Vec<DisplayTextSurface>) {
        self.display_text_surfaces = if self.item_frame_map_text_font_atlas.is_some() {
            surfaces
                .into_iter()
                .filter(|surface| !surface.is_empty())
                .collect()
        } else {
            Vec::new()
        };
    }

    pub(crate) fn collect_display_text_geometry(&self) -> DisplayTextGeometry {
        if self.item_frame_map_text_font_atlas.is_none() {
            return DisplayTextGeometry::default();
        }
        merge_display_text_surfaces(&self.display_text_surfaces, self.camera_sort_position())
    }
}

const DISPLAY_TEXT_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    lightmap_factors: vec4<f32>,
    lightmap_effects: vec4<f32>,
    camera_position: vec4<f32>,
    fog_color: vec4<f32>,
    fog_distances: vec4<f32>,
    minecraft_light0: vec4<f32>,
    minecraft_light1: vec4<f32>,
};

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

fn sample_lightmap(light: vec2<f32>) -> vec3<f32> {
    let uv = clamp(
        light * (15.0 / 16.0) + vec2<f32>(0.5 / 16.0),
        vec2<f32>(0.5 / 16.0),
        vec2<f32>(15.5 / 16.0),
    );
    return textureSample(lightmap_texture, lightmap_sampler, uv).rgb;
}

fn linear_fog_value(vertex_distance: f32, fog_start: f32, fog_end: f32) -> f32 {
    if (vertex_distance <= fog_start) { return 0.0; }
    if (vertex_distance >= fog_end) { return 1.0; }
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
"#;

const DISPLAY_TEXT_SEE_THROUGH_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    lightmap_factors: vec4<f32>,
    lightmap_effects: vec4<f32>,
    camera_position: vec4<f32>,
    fog_color: vec4<f32>,
    fog_distances: vec4<f32>,
    minecraft_light0: vec4<f32>,
    minecraft_light1: vec4<f32>,
};

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
"#;

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

pub(crate) fn create_display_text_background_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    atlas_layout: &wgpu::BindGroupLayout,
    lightmap_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    display_text_pipeline(
        device,
        format,
        atlas_layout,
        lightmap_layout,
        "bbb-display-text-background-pipeline",
        DISPLAY_TEXT_SHADER,
        Some(depth_stencil_state(
            DEPTH_FORMAT,
            true,
            wgpu::CompareFunction::LessEqual,
        )),
    )
}

pub(crate) fn create_display_text_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    atlas_layout: &wgpu::BindGroupLayout,
    lightmap_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let mut depth = depth_stencil_state(DEPTH_FORMAT, true, wgpu::CompareFunction::LessEqual);
    depth.bias = wgpu::DepthBiasState {
        constant: -10,
        slope_scale: -1.0,
        clamp: 0.0,
    };
    display_text_pipeline(
        device,
        format,
        atlas_layout,
        lightmap_layout,
        "bbb-display-text-polygon-offset-pipeline",
        DISPLAY_TEXT_SHADER,
        Some(depth),
    )
}

pub(crate) fn create_display_text_see_through_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    atlas_layout: &wgpu::BindGroupLayout,
    lightmap_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    display_text_pipeline(
        device,
        format,
        atlas_layout,
        lightmap_layout,
        "bbb-display-text-see-through-pipeline",
        DISPLAY_TEXT_SEE_THROUGH_SHADER,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HudTextStyle, HudUvRect};

    fn glyphs() -> HudFontGlyphMap {
        let mut glyphs = HudFontGlyphMap::new();
        for ch in ['a', 'b', 'c', '?', ' '] {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    uv: HudUvRect {
                        min: [0.25, 0.5],
                        max: [0.3, 0.6],
                    },
                    width: if ch == ' ' { 0 } else { 5 },
                    height: if ch == ' ' { 0 } else { 8 },
                    advance: if ch == ' ' { 4 } else { 6 },
                    ascent: 7,
                },
            );
        }
        glyphs
    }

    #[test]
    fn display_text_wraps_spaces_hard_breaks_and_terminal_newlines() {
        let glyphs = glyphs();
        let lines = split_display_text_lines(&[HudStyledTextRun::plain("aa bb")], 14, &glyphs);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].runs[0].text, "aa");
        assert_eq!(lines[1].runs[0].text, "bb");

        let lines = split_display_text_lines(&[HudStyledTextRun::plain("abc")], 6, &glyphs);
        assert_eq!(
            lines.iter().map(|line| line.width).collect::<Vec<_>>(),
            [6, 6, 6]
        );

        let lines = split_display_text_lines(&[HudStyledTextRun::plain("a\n")], 100, &glyphs);
        assert_eq!(lines.len(), 2);
        assert!(lines[1].runs.is_empty());
    }

    #[test]
    fn display_text_bakes_vanilla_transform_background_alignment_and_opacity() {
        let surface = bake_display_text_surface(
            42,
            3,
            Some(0xFF11_2233),
            Mat4::from_translation(Vec3::new(2.0, 3.0, 4.0)),
            &[HudStyledTextRun::plain("ab\na")],
            200,
            0,
            -2,
            (DISPLAY_TEXT_FLAG_DEFAULT_BACKGROUND | DISPLAY_TEXT_FLAG_ALIGN_RIGHT) as i8,
            [0.25, 1.0],
            &glyphs(),
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
            .text
            .vertices
            .iter()
            .all(|vertex| (vertex.color[3] - 254.0 / 255.0).abs() < 1e-6));
    }

    #[test]
    fn display_text_bakes_shadow_bold_and_effect_geometry() {
        let runs = [HudStyledTextRun {
            text: "a".to_string(),
            style: HudTextStyle {
                bold: true,
                underlined: true,
                ..Default::default()
            },
            color: Some(0x80_40_20),
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
            &glyphs(),
        )
        .unwrap();
        // Bold glyph: shadow/main x two quads; underline: shadow/main quads.
        assert_eq!(surface.text_vertex_count(), 24);
        assert_eq!(surface.submission.lines[0].width, 7);
        assert_eq!(
            surface.text.vertices[0].color,
            [
                0x20 as f32 / 255.0,
                0x10 as f32 / 255.0,
                0x08 as f32 / 255.0,
                1.0,
            ]
        );
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
            &glyphs(),
        )
        .is_none());
        let surface = bake_display_text_surface(
            1,
            0,
            None,
            Mat4::IDENTITY,
            &[HudStyledTextRun::plain("\n")],
            200,
            0x4000_0000,
            -1,
            0,
            [0.0, 1.0],
            &glyphs(),
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
    }

    #[test]
    fn display_text_upload_sorting_matches_each_vanilla_render_type() {
        let make = |z: f32, flags: u8, sequence: u64| {
            bake_display_text_surface(
                sequence as i32,
                sequence,
                None,
                Mat4::from_translation(Vec3::new(0.0, 0.0, z)),
                &[HudStyledTextRun::plain("a")],
                200,
                0x4000_0000,
                -1,
                flags as i8,
                [0.0, 1.0],
                &glyphs(),
            )
            .unwrap()
        };
        let normal_near = make(1.0, 0, 0);
        let normal_far = make(10.0, 0, 1);
        let geometry =
            merge_display_text_surfaces(&[normal_near, normal_far], Some([0.0, 0.0, 0.0]));
        assert!((geometry.normal_background.0[0].position[2] - 10.0).abs() < 1e-5);
        assert!((geometry.normal_text.0[0].position[2] - 10.0).abs() < 1e-5);

        let see_near = make(1.0, DISPLAY_TEXT_FLAG_SEE_THROUGH, 0);
        let see_far = make(10.0, DISPLAY_TEXT_FLAG_SEE_THROUGH, 1);
        let geometry = merge_display_text_surfaces(&[see_near, see_far], Some([0.0, 0.0, 0.0]));
        // BackgroundSeeThrough sorts on upload, TextSeeThrough does not.
        assert!((geometry.see_through_background.0[0].position[2] - 10.0).abs() < 1e-5);
        assert!((geometry.see_through_text.0[0].position[2] - 1.0).abs() < 1e-5);
    }
}
