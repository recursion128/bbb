use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::{
    gpu::DEPTH_FORMAT,
    pipeline_builder::{depth_stencil_state, RenderPipelineBuilder},
    CameraPose, HudAsciiGlyph, HudFontGlyphMap, HudTextStyle,
};

const DEBUG_TEXT_REPLACEMENT_GLYPH: char = '?';
const DEBUG_TEXT_FULL_BRIGHT_LIGHT: [f32; 2] = [1.0, 1.0];
const DEBUG_TEXT_BLEND: wgpu::BlendState = wgpu::BlendState::ALPHA_BLENDING;
const DEBUG_TEXT_DEPTH_WRITE_ENABLED: bool = true;
const DEBUG_TEXT_DEPTH_COMPARE: wgpu::CompareFunction = wgpu::CompareFunction::LessEqual;
const DEBUG_TEXT_CULL_MODE: Option<wgpu::Face> = Some(wgpu::Face::Back);

/// One world-space `TextGizmo` rendered as camera-facing font quads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugTextBillboard {
    pub position: [f32; 3],
    pub text: String,
    /// RGBA8, corresponding to vanilla `TextGizmo.Style.color` after ARGB
    /// component extraction.
    pub color: [u8; 4],
    /// Vanilla font-pixel scale before the renderer divides by 16.
    pub scale: f32,
    /// `None` centers by `Font.width / 2`; `Some(v)` uses vanilla
    /// `fontX = -v / scale` left adjustment.
    pub adjust_left: Option<f32>,
    /// Render after world transparency composition with a freshly cleared
    /// depth attachment, matching vanilla `TextGizmo.setAlwaysOnTop`.
    pub always_on_top: bool,
}

impl DebugTextBillboard {
    pub const DEFAULT_SCALE: f32 = 0.32;

    pub fn centered(position: [f32; 3], text: impl Into<String>, color: [u8; 4]) -> Self {
        Self {
            position,
            text: text.into(),
            color,
            scale: Self::DEFAULT_SCALE,
            adjust_left: None,
            always_on_top: false,
        }
    }

    fn sanitized(self) -> Option<Self> {
        self.is_valid().then_some(self)
    }

    fn is_valid(&self) -> bool {
        !self.text.is_empty()
            && self
                .position
                .iter()
                .all(|coordinate| coordinate.is_finite())
            && self.scale.is_finite()
            && self.scale > 0.0
            && self.adjust_left.is_none_or(|value| value.is_finite())
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct DebugTextVertex {
    pub(super) position: [f32; 3],
    pub(super) color: [u8; 4],
    pub(super) uv: [f32; 2],
    pub(super) light: [f32; 2],
}

#[derive(Debug, Default, PartialEq)]
pub(super) struct DebugTextMesh {
    pub(super) vertices: Vec<DebugTextVertex>,
    pub(super) indices: Vec<u32>,
}

const DEBUG_TEXT_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Unorm8x4, 2 => Float32x2, 3 => Float32x2];

const DEBUG_TEXT_SHADER: &str = r#"
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
@group(0) @binding(1)
var font_atlas: texture_2d<f32>;
@group(0) @binding(2)
var font_sampler: sampler;
@group(1) @binding(0)
var lightmap_texture: texture_2d<f32>;
@group(1) @binding(1)
var lightmap_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) light: vec2<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) light: vec2<f32>,
    @location(3) spherical_distance: f32,
    @location(4) cylindrical_distance: f32,
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
    out.color = input.color;
    out.uv = input.uv;
    out.light = input.light;
    let fog_pos = input.position - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let color = textureSample(font_atlas, font_sampler, input.uv) * input.color;
    if color.a < 0.1 {
        discard;
    }
    let lit = vec4<f32>(color.rgb * sample_lightmap(input.light), color.a);
    return apply_fog(lit, input.spherical_distance, input.cylindrical_distance);
}
"#;

pub(super) fn sanitize_debug_text_billboards(
    billboards: impl IntoIterator<Item = DebugTextBillboard>,
) -> Vec<DebugTextBillboard> {
    billboards
        .into_iter()
        .filter_map(DebugTextBillboard::sanitized)
        .collect()
}

pub(super) fn build_debug_text_mesh_for_layer(
    billboards: &[DebugTextBillboard],
    glyphs: &HudFontGlyphMap,
    camera_pose: CameraPose,
    always_on_top: bool,
) -> DebugTextMesh {
    let axes = camera_pose.billboard_axes();
    let mut mesh = DebugTextMesh::default();
    // DrawableGizmoPrimitives owns separate groups and renders opaque text
    // before translucent text, preserving insertion order inside each group.
    for billboard in billboards
        .iter()
        .filter(|billboard| {
            billboard.is_valid()
                && billboard.color[3] == 255
                && billboard.always_on_top == always_on_top
        })
        .chain(billboards.iter().filter(|billboard| {
            billboard.is_valid()
                && billboard.color[3] < 255
                && billboard.always_on_top == always_on_top
        }))
    {
        let width = billboard.text.chars().fold(0_u32, |width, ch| {
            width.saturating_add(debug_text_glyph(ch, glyphs).advance)
        });
        let mut pen_x = billboard
            .adjust_left
            .map_or(-(width as f32) / 2.0, |adjust| -adjust / billboard.scale);
        let pixel_scale = billboard.scale / 16.0;
        let origin = Vec3::from_array(billboard.position);
        for ch in billboard.text.chars() {
            let glyph = debug_text_glyph(ch, glyphs);
            if glyph.width > 0 && glyph.height > 0 {
                for quad in glyph.styled_quads(pen_x, 0.0, HudTextStyle::default(), false) {
                    let base = u32::try_from(mesh.vertices.len())
                        .expect("debug text vertex count fits in u32");
                    for ([x, y], uv) in quad.corners.into_iter().zip([
                        [quad.uv.min[0], quad.uv.min[1]],
                        [quad.uv.min[0], quad.uv.max[1]],
                        [quad.uv.max[0], quad.uv.max[1]],
                        [quad.uv.max[0], quad.uv.min[1]],
                    ]) {
                        let position =
                            origin + axes.right * (x * pixel_scale) - axes.up * (y * pixel_scale);
                        mesh.vertices.push(DebugTextVertex {
                            position: position.to_array(),
                            color: billboard.color,
                            uv,
                            light: DEBUG_TEXT_FULL_BRIGHT_LIGHT,
                        });
                    }
                    mesh.indices.extend_from_slice(&[
                        base,
                        base + 1,
                        base + 2,
                        base + 2,
                        base + 3,
                        base,
                    ]);
                }
            }
            pen_x += glyph.advance as f32;
        }
    }
    mesh
}

fn debug_text_glyph(ch: char, glyphs: &HudFontGlyphMap) -> HudAsciiGlyph {
    glyphs
        .get(ch)
        .or_else(|| glyphs.get(DEBUG_TEXT_REPLACEMENT_GLYPH))
        .unwrap_or_default()
}

pub(super) fn create_debug_text_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_texture_bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, "bbb-debug-text-pipeline")
        .shader("bbb-debug-text-shader", DEBUG_TEXT_SHADER)
        .layout(
            "bbb-debug-text-pipeline-layout",
            &[camera_texture_bind_group_layout, lightmap_bind_group_layout],
        )
        .vertex_buffers(&[debug_text_vertex_layout()])
        .color_target(format, Some(DEBUG_TEXT_BLEND))
        .depth_stencil(depth_stencil_state(
            DEPTH_FORMAT,
            DEBUG_TEXT_DEPTH_WRITE_ENABLED,
            DEBUG_TEXT_DEPTH_COMPARE,
        ))
        .cull_mode(DEBUG_TEXT_CULL_MODE)
        .build()
}

fn debug_text_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<DebugTextVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &DEBUG_TEXT_VERTEX_ATTRIBUTES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClearColor, HudUvRect, Renderer};

    fn test_glyphs() -> HudFontGlyphMap {
        let mut glyphs = HudFontGlyphMap::new();
        glyphs.insert_first_wins(
            '8',
            HudAsciiGlyph {
                uv: HudUvRect {
                    min: [0.25, 0.5],
                    max: [0.5, 1.0],
                },
                width: 5,
                height: 8,
                advance: 6,
                ascent: 7,
            },
        );
        glyphs
    }

    fn camera() -> CameraPose {
        CameraPose {
            position: [0.0; 3],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }
    }

    #[test]
    fn debug_text_sanitize_rejects_invalid_or_empty_submissions() {
        let valid = DebugTextBillboard::centered([1.0, 2.0, 3.0], "8", [1, 2, 3, 4]);
        assert_eq!(valid.clone().sanitized(), Some(valid.clone()));
        assert_eq!(
            DebugTextBillboard::centered([0.0; 3], "", [0; 4]).sanitized(),
            None
        );
        assert_eq!(
            DebugTextBillboard {
                position: [f32::NAN, 0.0, 0.0],
                ..valid.clone()
            }
            .sanitized(),
            None
        );
        assert_eq!(
            DebugTextBillboard {
                scale: 0.0,
                ..valid.clone()
            }
            .sanitized(),
            None
        );
        assert_eq!(
            DebugTextBillboard {
                adjust_left: Some(f32::INFINITY),
                ..valid
            }
            .sanitized(),
            None
        );
    }

    #[test]
    fn debug_text_mesh_centers_font_pixels_and_uses_full_bright_rgba() {
        let mesh = build_debug_text_mesh_for_layer(
            &[DebugTextBillboard::centered(
                [1.0, 2.0, 3.0],
                "8",
                [10, 20, 30, 40],
            )],
            &test_glyphs(),
            camera(),
            false,
        );
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices, [0, 1, 2, 2, 3, 0]);
        // width=6 => fontX=-3; default 0.32/16=0.02 world units per pixel.
        // At vanilla yaw 0, camera-local +X points toward world -X, so the
        // negative font-space left edge lies on world +X.
        assert_eq!(mesh.vertices[0].position, [1.06, 2.0, 3.0]);
        assert_eq!(mesh.vertices[1].position, [1.06, 1.84, 3.0]);
        assert_eq!(mesh.vertices[2].position, [0.96, 1.84, 3.0]);
        assert_eq!(mesh.vertices[0].uv, [0.25, 0.5]);
        assert_eq!(mesh.vertices[2].uv, [0.5, 1.0]);
        assert!(mesh.vertices.iter().all(|vertex| {
            vertex.color == [10, 20, 30, 40] && vertex.light == DEBUG_TEXT_FULL_BRIGHT_LIGHT
        }));
    }

    #[test]
    fn debug_text_left_adjustment_matches_text_gizmo_formula() {
        let mut billboard = DebugTextBillboard::centered([0.0; 3], "8", [255; 4]);
        billboard.adjust_left = Some(0.5);
        let mesh = build_debug_text_mesh_for_layer(&[billboard], &test_glyphs(), camera(), false);
        // fontX=-0.5/0.32; multiplied by 0.32/16 => -0.5/16.
        // Vanilla yaw 0 maps camera-local +X to world -X.
        assert_eq!(mesh.vertices[0].position[0], 0.5 / 16.0);
    }

    #[test]
    fn debug_text_mesh_groups_opaque_before_translucent_stably() {
        let translucent = DebugTextBillboard::centered([1.0, 0.0, 0.0], "8", [1, 2, 3, 4]);
        let opaque = DebugTextBillboard::centered([2.0, 0.0, 0.0], "8", [5, 6, 7, 255]);
        let translucent_later = DebugTextBillboard::centered([3.0, 0.0, 0.0], "8", [8, 9, 10, 11]);
        let mesh = build_debug_text_mesh_for_layer(
            &[translucent, opaque, translucent_later],
            &test_glyphs(),
            camera(),
            false,
        );
        assert_eq!(mesh.vertices.len(), 12);
        assert!(mesh.vertices[0..4]
            .iter()
            .all(|vertex| vertex.color == [5, 6, 7, 255]));
        assert!(mesh.vertices[4..8]
            .iter()
            .all(|vertex| vertex.color == [1, 2, 3, 4]));
        assert!(mesh.vertices[8..12]
            .iter()
            .all(|vertex| vertex.color == [8, 9, 10, 11]));
    }

    #[test]
    fn debug_text_mesh_separates_standard_and_always_on_top_layers() {
        let standard = DebugTextBillboard::centered([1.0, 0.0, 0.0], "8", [1, 2, 3, 255]);
        let mut always_on_top = DebugTextBillboard::centered([2.0, 0.0, 0.0], "8", [4, 5, 6, 255]);
        always_on_top.always_on_top = true;
        let billboards = [standard, always_on_top];

        let standard_mesh =
            build_debug_text_mesh_for_layer(&billboards, &test_glyphs(), camera(), false);
        let always_on_top_mesh =
            build_debug_text_mesh_for_layer(&billboards, &test_glyphs(), camera(), true);

        assert_eq!(standard_mesh.vertices.len(), 4);
        assert!(standard_mesh
            .vertices
            .iter()
            .all(|vertex| vertex.color == [1, 2, 3, 255]));
        assert_eq!(always_on_top_mesh.vertices.len(), 4);
        assert!(always_on_top_mesh
            .vertices
            .iter()
            .all(|vertex| vertex.color == [4, 5, 6, 255]));
    }

    #[test]
    fn debug_text_pipeline_matches_vanilla_render_types_text_state() {
        assert_eq!(DEBUG_TEXT_CULL_MODE, Some(wgpu::Face::Back));
        assert!(DEBUG_TEXT_DEPTH_WRITE_ENABLED);
        assert_eq!(DEBUG_TEXT_DEPTH_COMPARE, wgpu::CompareFunction::LessEqual);
        assert_eq!(DEBUG_TEXT_BLEND, wgpu::BlendState::ALPHA_BLENDING);
        assert!(DEBUG_TEXT_SHADER.contains("if color.a < 0.1"));
        assert!(DEBUG_TEXT_SHADER.contains("sample_lightmap(input.light)"));
        assert!(DEBUG_TEXT_SHADER.contains("return apply_fog("));
    }

    #[test]
    fn debug_text_setter_sanitizes_and_clears_stale_submissions() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.set_debug_text_billboards(vec![
            DebugTextBillboard::centered([0.0; 3], "8", [255; 4]),
            DebugTextBillboard::centered([0.0; 3], "", [255; 4]),
        ]);
        assert_eq!(renderer.debug_text_billboards.len(), 1);
        renderer.set_debug_text_billboards(Vec::new());
        assert!(renderer.debug_text_billboards.is_empty());
    }

    #[test]
    fn debug_text_billboard_draws_through_full_frame_readback() {
        let Some(mut renderer) = Renderer::new_offscreen(64, 64) else {
            return;
        };
        renderer.set_clear_color(ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        let mut glyphs = HudFontGlyphMap::new();
        glyphs.insert_first_wins(
            '8',
            HudAsciiGlyph {
                uv: HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                width: 16,
                height: 16,
                advance: 16,
                ascent: 7,
            },
        );
        renderer
            .upload_hud_font_atlas(1, 1, &[255, 255, 255, 255], glyphs)
            .expect("upload debug glyph metrics");
        renderer.upload_item_frame_map_text_font(1, 1, &[255, 255, 255, 255]);
        renderer.set_camera_pose(Some(CameraPose {
            position: [0.0; 3],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }));
        let mut billboard = DebugTextBillboard::centered([0.0, 0.18, 2.0], "8", [0, 0, 0, 255]);
        renderer.set_debug_text_billboards(vec![billboard.clone()]);

        let pixels = renderer
            .render_offscreen_frame()
            .expect("debug text frame readback");
        let drew_black_glyph = (20..44).any(|y| {
            (20..44).any(|x| {
                let pixel = pixels.pixel(x, y);
                pixel[0] < 64 && pixel[1] < 64 && pixel[2] < 64
            })
        });
        assert!(
            drew_black_glyph,
            "camera-facing glyph should reach the framebuffer"
        );
        let corner = pixels.pixel(4, 4);
        assert!(
            corner[2] > 128 && corner[0] < 64,
            "background outside the glyph stays blue, got {corner:?}"
        );

        billboard.always_on_top = true;
        renderer.set_debug_text_billboards(vec![billboard]);
        let pixels = renderer
            .render_offscreen_frame()
            .expect("always-on-top debug text frame readback");
        assert!(
            (20..44).any(|y| {
                (20..44).any(|x| {
                    let pixel = pixels.pixel(x, y);
                    pixel[0] < 64 && pixel[1] < 64 && pixel[2] < 64
                })
            }),
            "late camera-facing glyph should reach the framebuffer"
        );
    }
}
