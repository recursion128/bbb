//! Sign face text -> world-space glyph quads.
//!
//! Vanilla submits sign text per face through
//! `AbstractSignRenderer.submitSignText`: four centred lines in font-pixel
//! space under the face's text transformation
//! (`StandingSignRenderer.textTransformation` /
//! `HangingSignRenderer.textTransformation`), colored by the face's dye
//! (darkened via `getDarkColor` unless glowing). bbb bakes those glyph quads
//! into world space exactly like the item-frame map label path
//! (`bake_map_text_surface`) and draws them with the same
//! `minecraft:font/default` atlas in the entity translucent feature pass.

use glam::{Mat4, Vec3};

use bbb_render_types::HudStyledTextRun;

use crate::entity_models::{sign_base_transformation, SignModelAttachment};
use crate::{HudAsciiGlyph, HudFontGlyphMap, Renderer};

use super::{ItemModelMesh, ItemModelVertex};

/// Vanilla `SignBlockEntity.TEXT_LINE_HEIGHT` (`10`).
pub const SIGN_TEXT_LINE_HEIGHT: i32 = 10;
/// Vanilla `HangingSignBlockEntity.TEXT_LINE_HEIGHT` (`9`).
pub const HANGING_SIGN_TEXT_LINE_HEIGHT: i32 = 9;
/// Vanilla `SignBlockEntity.MAX_TEXT_LINE_WIDTH` (`90`).
pub const SIGN_MAX_TEXT_LINE_WIDTH: u32 = 90;
/// Vanilla `HangingSignBlockEntity.MAX_TEXT_LINE_WIDTH` (`60`).
pub const HANGING_SIGN_MAX_TEXT_LINE_WIDTH: u32 = 60;

/// Vanilla `StandingSignRenderer.TEXT_OFFSET`
/// (`(0.0F, 0.33333334F, 0.046666667F)`).
const SIGN_TEXT_OFFSET: Vec3 = Vec3::new(0.0, 0.333_333_34, 0.046_666_667);
/// Vanilla `HangingSignRenderer.TEXT_OFFSET` (`(0.0F, -0.32F, 0.073F)`).
const HANGING_SIGN_TEXT_OFFSET: Vec3 = Vec3::new(0.0, -0.32, 0.073);
/// Vanilla `StandingSignRenderer.textTransformation` scale
/// (`0.010416667F` = `RENDER_SCALE (0.6666667) * 0.015625`).
const SIGN_TEXT_RENDER_SCALE: f32 = 0.010_416_667;
/// Vanilla `HangingSignRenderer.textTransformation` scale
/// (`0.0140625F` = `TEXT_RENDER_SCALE (0.9) * 0.015625`).
const HANGING_SIGN_TEXT_RENDER_SCALE: f32 = 0.014_062_5;
/// Vanilla `AbstractSignRenderer.BLACK_TEXT_OUTLINE_COLOR` (`-988212` ==
/// `0xFFF0EBCC`), the cream color a glowing black face darkens to.
const BLACK_GLOWING_SIGN_DARK_RGB: u32 = 0x00F0_EBCC;
/// Vanilla `Font` missing-codepoint degradation (unihex deferred), matching
/// the map-label path's replacement glyph.
const SIGN_TEXT_REPLACEMENT_GLYPH: char = '?';

/// The per-face sign text submit metadata alongside the baked mesh; the
/// glyph quads themselves live in [`SignTextSurface::mesh`].
#[derive(Debug, Clone, PartialEq)]
pub struct SignTextSubmission {
    pub position: [f32; 3],
    pub attachment: SignModelAttachment,
    pub front: bool,
    /// The line base color as `0xRRGGBB` — the glowing face's raw
    /// `DyeColor.getTextColor()` or the non-glowing `getDarkColor` result.
    /// Per-run style colors override it per vanilla
    /// `StringRenderOutput.getTextColor`.
    pub color: u32,
    pub has_glowing_text: bool,
    pub light: [f32; 2],
    pub transform: Mat4,
}

/// One sign face's baked text: world-space glyph quads sampling the
/// `minecraft:font/default` atlas (the map-label font upload).
#[derive(Debug, Clone, PartialEq)]
pub struct SignTextSurface {
    pub submission: SignTextSubmission,
    mesh: ItemModelMesh,
}

impl SignTextSurface {
    pub fn is_empty(&self) -> bool {
        self.mesh.is_empty()
    }

    pub fn vertex_count(&self) -> usize {
        self.mesh.vertices.len()
    }

    pub fn index_count(&self) -> usize {
        self.mesh.indices.len()
    }
}

/// Vanilla `SignBlockEntity.getTextLineHeight` dispatch: hanging signs
/// override it to 9.
pub fn sign_text_line_height(attachment: SignModelAttachment) -> i32 {
    if attachment.is_hanging() {
        HANGING_SIGN_TEXT_LINE_HEIGHT
    } else {
        SIGN_TEXT_LINE_HEIGHT
    }
}

/// Vanilla `SignBlockEntity.getMaxTextLineWidth` dispatch: hanging signs
/// override it to 60.
pub fn sign_max_text_line_width(attachment: SignModelAttachment) -> u32 {
    if attachment.is_hanging() {
        HANGING_SIGN_MAX_TEXT_LINE_WIDTH
    } else {
        SIGN_MAX_TEXT_LINE_WIDTH
    }
}

/// Vanilla `ARGB.scaleRGB(color, 0.4F)` over an opaque `0xRRGGBB`: each
/// channel is `(int)(channel * 0.4F)` (truncating), alpha untouched.
pub fn sign_text_scaled_rgb(color: u32) -> u32 {
    let scale = |channel: u32| ((channel & 0xFF) as f32 * 0.4) as u32;
    scale(color >> 16) << 16 | scale(color >> 8) << 8 | scale(color)
}

/// Vanilla `AbstractSignRenderer.getDarkColor`: a glowing black face darkens
/// to `BLACK_TEXT_OUTLINE_COLOR`, everything else to `scaleRGB(color, 0.4F)`.
pub fn sign_text_dark_color(dye_text_color: u32, has_glowing_text: bool) -> u32 {
    if dye_text_color == 0 && has_glowing_text {
        BLACK_GLOWING_SIGN_DARK_RGB
    } else {
        sign_text_scaled_rgb(dye_text_color)
    }
}

/// Vanilla `submitSignText`'s line color pick: a glowing face renders the raw
/// `DyeColor.getTextColor()` (full-bright, outline deferred), a non-glowing
/// face the darkened color.
pub fn sign_text_base_color(dye_text_color: u32, has_glowing_text: bool) -> u32 {
    if has_glowing_text {
        dye_text_color
    } else {
        sign_text_dark_color(dye_text_color, false)
    }
}

/// Vanilla `StandingSignRenderer.textTransformation` /
/// `HangingSignRenderer.textTransformation`: the sign base transformation,
/// a 180° yaw for the back face, then the face's `TEXT_OFFSET` translation
/// and the `scale(s, -s, s)` font-pixel mapping.
pub fn sign_text_transformation(
    position: [f32; 3],
    attachment: SignModelAttachment,
    body_rot_degrees: f32,
    front: bool,
) -> Mat4 {
    let mut transform = sign_base_transformation(position, attachment, body_rot_degrees);
    if !front {
        transform *= Mat4::from_rotation_y(std::f32::consts::PI);
    }
    let (offset, scale) = if attachment.is_hanging() {
        (HANGING_SIGN_TEXT_OFFSET, HANGING_SIGN_TEXT_RENDER_SCALE)
    } else {
        (SIGN_TEXT_OFFSET, SIGN_TEXT_RENDER_SCALE)
    };
    transform * Mat4::from_translation(offset) * Mat4::from_scale(Vec3::new(scale, -scale, scale))
}

/// Vanilla `AbstractSignRenderer.submitSignText`'s per-line
/// `font.split(component, maxTextLineWidth)` first segment: word-wrap at the
/// last space before the width limit (the break space is consumed), falling
/// back to a hard break before the overflowing glyph
/// (`StringSplitter.LineBreakFinder`). Advances are style-aware
/// (`GlyphInfo.getAdvance(bold)`).
pub fn truncate_sign_line_runs_to_width(
    runs: &[HudStyledTextRun],
    max_width: u32,
    glyphs: &HudFontGlyphMap,
) -> Vec<HudStyledTextRun> {
    // Flatten to (run index, char) so the break index can cut mid-run.
    let flattened: Vec<(usize, char)> = runs
        .iter()
        .enumerate()
        .flat_map(|(run_index, run)| run.text.chars().map(move |ch| (run_index, ch)))
        .collect();
    let mut width = 0u32;
    let mut last_space: Option<usize> = None;
    let mut break_at = flattened.len();
    for (index, (run_index, ch)) in flattened.iter().enumerate() {
        if *ch == ' ' {
            last_space = Some(index);
        }
        width = width
            .saturating_add(sign_text_glyph(*ch, glyphs).styled_advance(runs[*run_index].style));
        if width > max_width {
            // The kept slice ends before the break index, so a space break
            // consumes the space itself (vanilla `splitLines` skips it).
            break_at = last_space.unwrap_or(index);
            break;
        }
    }
    let mut out: Vec<HudStyledTextRun> = Vec::new();
    for (run_index, ch) in flattened.into_iter().take(break_at) {
        match out.last_mut() {
            Some(last)
                if last.style == runs[run_index].style && last.color == runs[run_index].color =>
            {
                last.text.push(ch);
            }
            _ => out.push(HudStyledTextRun {
                text: ch.to_string(),
                style: runs[run_index].style,
                color: runs[run_index].color,
            }),
        }
    }
    out
}

/// Style-aware line width in font pixels — vanilla `Font.width` over the
/// baked glyph advances (`GlyphInfo.getAdvance(bold)`).
pub fn sign_line_runs_width(runs: &[HudStyledTextRun], glyphs: &HudFontGlyphMap) -> u32 {
    runs.iter()
        .map(|run| {
            run.text
                .chars()
                .map(|ch| sign_text_glyph(ch, glyphs).styled_advance(run.style))
                .sum::<u32>()
        })
        .sum()
}

fn sign_text_glyph(ch: char, glyphs: &HudFontGlyphMap) -> HudAsciiGlyph {
    glyphs
        .get(ch)
        .or_else(|| glyphs.get(SIGN_TEXT_REPLACEMENT_GLYPH))
        .unwrap_or_default()
}

/// Bakes one sign face's text into world-space glyph quads, transcribing
/// `AbstractSignRenderer.submitSignText`:
/// - each line is width-truncated to `getMaxTextLineWidth()` and centred at
///   `x = -width / 2` (vanilla integer halving of `font.width`);
/// - line `i` sits at `y = i * lineHeight - 4 * lineHeight / 2`;
/// - the line color is the glowing raw dye color or the `getDarkColor`
///   darkened one; per-run style colors override it
///   (`StringRenderOutput.getTextColor`);
/// - glyph quads (bold double-draw + italic shear via `styled_quads`, no
///   drop shadow — vanilla submits with `dropShadow = false`) are transformed
///   by the face's text transformation into world space.
///
/// The glowing outline 8-way redraw and the underline/strikethrough effect
/// bars (drawn from a white sprite vanilla-side, unavailable in the font
/// atlas draw) are deferred; obfuscated runs draw their original glyphs.
#[allow(clippy::too_many_arguments)]
pub fn bake_sign_text_surface(
    position: [f32; 3],
    attachment: SignModelAttachment,
    body_rot_degrees: f32,
    front: bool,
    lines: &[Vec<HudStyledTextRun>; 4],
    dye_text_color: u32,
    has_glowing_text: bool,
    light: [f32; 2],
    glyphs: &HudFontGlyphMap,
) -> Option<SignTextSurface> {
    let transform = sign_text_transformation(position, attachment, body_rot_degrees, front);
    let line_height = sign_text_line_height(attachment);
    let max_width = sign_max_text_line_width(attachment);
    // Vanilla `submitSignText`: `signMidpoint = 4 * textLineHeight / 2`.
    let sign_midpoint = 4 * line_height / 2;
    let base_color = sign_text_base_color(dye_text_color, has_glowing_text);

    let mut mesh = ItemModelMesh::new();
    for (line_index, line) in lines.iter().enumerate() {
        let runs = truncate_sign_line_runs_to_width(line, max_width, glyphs);
        let width = sign_line_runs_width(&runs, glyphs);
        if width == 0 {
            continue;
        }
        // Vanilla: `float x1 = -this.font.width(actualLine) / 2;` (integer
        // division before the float widening).
        let mut pen_x = (-(width as i32) / 2) as f32;
        let y = (line_index as i32 * line_height - sign_midpoint) as f32;
        for run in &runs {
            let color = rgb_to_color(run.color.unwrap_or(base_color));
            for ch in run.text.chars() {
                let glyph = sign_text_glyph(ch, glyphs);
                if glyph.width > 0 && glyph.height > 0 {
                    for quad in glyph.styled_quads(pen_x, y, run.style, false) {
                        // `styled_quads` corners are [TL, BL, BR, TR].
                        let corners = quad.corners.map(|[x, y]| {
                            transform.transform_point3(Vec3::new(x, y, 0.0)).to_array()
                        });
                        mesh.append_raw_textured_quad(
                            corners,
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
                pen_x += glyph.styled_advance(run.style) as f32;
            }
        }
    }
    if mesh.is_empty() {
        return None;
    }
    Some(SignTextSurface {
        submission: SignTextSubmission {
            position,
            attachment,
            front,
            color: base_color,
            has_glowing_text,
            light,
            transform,
        },
        mesh,
    })
}

fn rgb_to_color(rgb: u32) -> [f32; 4] {
    [
        ((rgb >> 16) & 0xFF) as f32 / 255.0,
        ((rgb >> 8) & 0xFF) as f32 / 255.0,
        (rgb & 0xFF) as f32 / 255.0,
        1.0,
    ]
}

pub(crate) fn merge_sign_text_surfaces(
    surfaces: &[SignTextSurface],
) -> (Vec<ItemModelVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for surface in surfaces {
        let base = u32::try_from(vertices.len()).expect("sign text vertex count fits in u32");
        vertices.extend(surface.mesh.vertices.iter().copied());
        indices.extend(surface.mesh.indices.iter().map(|index| index + base));
    }
    (vertices, indices)
}

impl Renderer {
    /// Stores the frame's sign text surfaces. Like the map labels, they draw
    /// with the `minecraft:font/default` atlas uploaded by
    /// [`Renderer::upload_item_frame_map_text_font`]; without it the surfaces
    /// are dropped.
    pub fn set_sign_text_surfaces(&mut self, surfaces: Vec<SignTextSurface>) {
        self.sign_text_surfaces = if self.item_frame_map_text_font_atlas.is_some() {
            surfaces
                .into_iter()
                .filter(|surface| !surface.is_empty())
                .collect()
        } else {
            Vec::new()
        };
    }

    pub(crate) fn collect_sign_text_geometry(&self) -> (Vec<ItemModelVertex>, Vec<u32>) {
        if self.item_frame_map_text_font_atlas.is_none() {
            return (Vec::new(), Vec::new());
        }
        merge_sign_text_surfaces(&self.sign_text_surfaces)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_render_types::{HudAsciiGlyph, HudTextStyle, HudUvRect};

    /// A minimal ascent-7 glyph table: advance 6 (space 4), 5×8 cells.
    fn test_glyphs() -> HudFontGlyphMap {
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

    fn plain_runs(text: &str) -> Vec<HudStyledTextRun> {
        vec![HudStyledTextRun::plain(text)]
    }

    fn assert_vec3_close(actual: glam::Vec3, expected: glam::Vec3) {
        assert!(
            (actual - expected).length() < 1e-5,
            "expected {actual:?} ~= {expected:?}"
        );
    }

    #[test]
    fn sign_text_transformation_matches_vanilla_offsets_and_scale() {
        // Standing front, angle 0 at the origin block: the font origin lands
        // at TEXT_OFFSET above/in front of the block centre; +x maps to
        // +x * 0.010416667, +y (font-down) to -y world.
        let front = sign_text_transformation([0.0; 3], SignModelAttachment::Standing, 0.0, true);
        assert_vec3_close(
            front.transform_point3(Vec3::ZERO),
            Vec3::new(0.5, 0.5 + 0.333_333_34, 0.5 + 0.046_666_667),
        );
        assert_vec3_close(
            front.transform_vector3(Vec3::X),
            Vec3::new(0.010_416_667, 0.0, 0.0),
        );
        assert_vec3_close(
            front.transform_vector3(Vec3::Y),
            Vec3::new(0.0, -0.010_416_667, 0.0),
        );
        // The back face rotates 180° about the block centre: the offset sits
        // on the other side and +x mirrors.
        let back = sign_text_transformation([0.0; 3], SignModelAttachment::Standing, 0.0, false);
        assert_vec3_close(
            back.transform_point3(Vec3::ZERO),
            Vec3::new(0.5, 0.833_333_34, 0.5 - 0.046_666_667),
        );
        assert_vec3_close(
            back.transform_vector3(Vec3::X),
            Vec3::new(-0.010_416_667, 0.0, 0.0),
        );
        // Hanging front: translation(0.5, 0.9375, 0.5) · translate(0, -0.3125, 0)
        // · translate(0, -0.32, 0.073), scale 0.0140625.
        let hanging =
            sign_text_transformation([0.0; 3], SignModelAttachment::HangingCeiling, 0.0, true);
        assert_vec3_close(
            hanging.transform_point3(Vec3::ZERO),
            Vec3::new(0.5, 0.9375 - 0.3125 - 0.32, 0.5 + 0.073),
        );
        assert_vec3_close(
            hanging.transform_vector3(Vec3::Y),
            Vec3::new(0.0, -0.014_062_5, 0.0),
        );
    }

    #[test]
    fn sign_dark_color_matches_vanilla_formula() {
        // ARGB.scaleRGB(color, 0.4F): truncating per-channel scale.
        assert_eq!(sign_text_scaled_rgb(0xFF_FF_FF), 0x66_66_66);
        assert_eq!(sign_text_scaled_rgb(0xBF_FF_00), 0x4C_66_00);
        assert_eq!(sign_text_scaled_rgb(0x00_00_00), 0x00_00_00);
        // getDarkColor: black + glowing -> the -988212 cream constant.
        assert_eq!(sign_text_dark_color(0x00_00_00, true), 0xF0_EB_CC);
        assert_eq!(sign_text_dark_color(0x00_00_00, false), 0x00_00_00);
        assert_eq!(sign_text_dark_color(0xFF_00_00, true), 0x66_00_00);
        // Line base color: glowing renders the raw dye color, otherwise the
        // darkened one.
        assert_eq!(sign_text_base_color(0xFF_00_00, true), 0xFF_00_00);
        assert_eq!(sign_text_base_color(0xFF_00_00, false), 0x66_00_00);
    }

    #[test]
    fn line_height_and_width_limits_match_vanilla_block_entities() {
        assert_eq!(sign_text_line_height(SignModelAttachment::Standing), 10);
        assert_eq!(sign_text_line_height(SignModelAttachment::Wall), 10);
        assert_eq!(
            sign_text_line_height(SignModelAttachment::HangingCeiling),
            9
        );
        assert_eq!(sign_max_text_line_width(SignModelAttachment::Standing), 90);
        assert_eq!(
            sign_max_text_line_width(SignModelAttachment::HangingWall),
            60
        );
    }

    #[test]
    fn truncation_word_wraps_at_the_last_space_and_hard_breaks_without_one() {
        let glyphs = test_glyphs();
        // Fits: untouched (15 * 6 = 90 is not > 90).
        let fits = truncate_sign_line_runs_to_width(&plain_runs(&"a".repeat(15)), 90, &glyphs);
        assert_eq!(fits[0].text.len(), 15);
        // Hard break with no space: the 16th glyph pushes 96 > 90, so the
        // first line keeps 15 chars (the overflowing char is excluded).
        let hard = truncate_sign_line_runs_to_width(&plain_runs(&"a".repeat(20)), 90, &glyphs);
        assert_eq!(hard[0].text.len(), 15);
        // Word wrap: the accumulated width passes 90 inside the second word,
        // so the line breaks at the space and the space is consumed.
        let text = format!("{} {}", "a".repeat(8), "b".repeat(10)); // 48 + 4 + 60
        let wrapped = truncate_sign_line_runs_to_width(&plain_runs(&text), 90, &glyphs);
        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0].text, "a".repeat(8));
        // Bold advances are 7px (GlyphInfo.getAdvance(bold)); 13 bold glyphs
        // reach 91 > 90 so only 12 stay.
        let bold_runs = vec![HudStyledTextRun {
            text: "a".repeat(20),
            style: HudTextStyle {
                bold: true,
                ..Default::default()
            },
            color: None,
        }];
        let bold = truncate_sign_line_runs_to_width(&bold_runs, 90, &glyphs);
        assert_eq!(bold[0].text.len(), 12);
        // The kept slice preserves per-run styles across a mid-run break.
        assert!(bold[0].style.bold);
    }

    #[test]
    fn bake_centers_lines_and_positions_glyphs_like_vanilla() {
        let glyphs = test_glyphs();
        let lines = [plain_runs("ab"), Vec::new(), plain_runs("a"), Vec::new()];
        let surface = bake_sign_text_surface(
            [0.0; 3],
            SignModelAttachment::Standing,
            0.0,
            true,
            &lines,
            0x00_00_00,
            false,
            [0.25, 1.0],
            &glyphs,
        )
        .unwrap();
        // 3 glyphs, one quad each (no bold, no shadow).
        assert_eq!(surface.vertex_count(), 12);
        assert_eq!(surface.index_count(), 18);
        // Line 0 ("ab", width 12): x starts at -12/2 = -6; line 0 sits at
        // y = 0*10 - 20 = -20 font px. World TL corner of the first glyph:
        // (0.5 - 6s, 0.83333334 + 20s, 0.5 + 0.046666667), s = 0.010416667.
        let vertices = &surface.mesh.vertices;
        let expected = Vec3::new(
            0.5 - 6.0 * 0.010_416_667,
            0.833_333_34 + 20.0 * 0.010_416_667,
            0.546_666_67,
        );
        assert_vec3_close(Vec3::from_array(vertices[0].position), expected);
        // Non-glowing black dye: darkened base color stays black.
        assert_eq!(vertices[0].color, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(vertices[0].light, [0.25, 1.0]);
        assert_eq!(vertices[0].uv, [0.25, 0.5]);
        // Line 2 ("a", width 6): vanilla integer halving -> x = -(6/2) = -3,
        // y = 2*10 - 20 = 0.
        let line2_tl = Vec3::from_array(vertices[8].position);
        assert_vec3_close(
            line2_tl,
            Vec3::new(0.5 - 3.0 * 0.010_416_667, 0.833_333_34, 0.546_666_67),
        );
    }

    #[test]
    fn bake_applies_run_color_overrides_and_bold_double_draw() {
        let glyphs = test_glyphs();
        let lines = [
            vec![HudStyledTextRun {
                text: "a".to_string(),
                style: HudTextStyle {
                    bold: true,
                    ..Default::default()
                },
                color: Some(0x11_22_33),
            }],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ];
        let surface = bake_sign_text_surface(
            [0.0; 3],
            SignModelAttachment::Standing,
            0.0,
            true,
            &lines,
            0xFF_00_00,
            false,
            [0.0, 1.0],
            &glyphs,
        )
        .unwrap();
        // Bold renders the glyph twice (main + boldOffset pass).
        assert_eq!(surface.vertex_count(), 8);
        // The run's own color wins over the darkened face color
        // (StringRenderOutput.getTextColor).
        let expected = [
            0x11 as f32 / 255.0,
            0x22 as f32 / 255.0,
            0x33 as f32 / 255.0,
            1.0,
        ];
        assert!(surface
            .mesh
            .vertices
            .iter()
            .all(|vertex| vertex.color == expected));
    }

    #[test]
    fn bake_returns_none_for_empty_or_spaces_only_faces() {
        let glyphs = test_glyphs();
        let empty: [Vec<HudStyledTextRun>; 4] = Default::default();
        assert!(bake_sign_text_surface(
            [0.0; 3],
            SignModelAttachment::Standing,
            0.0,
            true,
            &empty,
            0,
            false,
            [0.0, 1.0],
            &glyphs,
        )
        .is_none());
        // A zero-width-glyph-only line lays out nothing either (advance-only).
        let spaces = [plain_runs("  "), Vec::new(), Vec::new(), Vec::new()];
        assert!(bake_sign_text_surface(
            [0.0; 3],
            SignModelAttachment::Standing,
            0.0,
            true,
            &spaces,
            0,
            false,
            [0.0, 1.0],
            &glyphs,
        )
        .is_none());
    }

    #[test]
    fn hanging_sign_lines_use_nine_px_height_and_sixty_px_limit() {
        let glyphs = test_glyphs();
        let lines = [
            plain_runs(&"a".repeat(12)),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ];
        let surface = bake_sign_text_surface(
            [0.0; 3],
            SignModelAttachment::HangingCeiling,
            0.0,
            true,
            &lines,
            0x00_00_00,
            false,
            [0.0, 1.0],
            &glyphs,
        )
        .unwrap();
        // 12 glyphs * 6 = 72 > 60: hard break keeps 10 glyphs (60 is not > 60).
        assert_eq!(surface.vertex_count(), 40);
        // Line 0 of a hanging sign: y = 0*9 - 18 = -18 font px at scale
        // 0.0140625, from the hanging text origin y = 0.2955.
        let tl = Vec3::from_array(surface.mesh.vertices[0].position);
        let expected_y = (0.9375 - 0.3125 - 0.32) + 18.0 * 0.014_062_5;
        assert!((tl.y - expected_y).abs() < 1e-5);
    }
}
