//! Sign face text measured with per-run fonts into world-space glyph quads.

use glam::{Mat4, Vec3};

use bbb_render_types::HudFontGlyphBook;

use crate::render_core::entity_catalog::{sign_base_transformation, SignModelAttachment};
use crate::text_mesh::{
    mesh_world_text_line, split_world_text_lines, world_text_chars, world_text_visual_order,
    world_text_width, PlacedGlyph, TextMesh, TextPen, VisualGlyph, WorldTextRun,
};

use super::ItemModelMesh;

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
/// Vanilla `AbstractSignRenderer.OUTLINE_RENDER_DISTANCE` (`16²`).
pub const SIGN_TEXT_OUTLINE_DISTANCE_SQUARED: f32 = 256.0;
/// The per-face sign text submit metadata alongside the measured glyph slots.
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
    /// Vanilla `getDarkColor(signText)`, used as the glowing outline color.
    pub outline_color: u32,
    pub has_glowing_text: bool,
    pub light: [f32; 2],
    pub transform: Mat4,
}

/// Per-frame camera facts used by Vanilla's sign glowing-outline predicate.
/// The position is the interpolated camera entity position, not a detached
/// third-person camera or eye position.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SignTextOutlineContext {
    pub camera_entity_position: Option<[f32; 3]>,
    pub first_person_scoping: bool,
}

/// One sign face's text. A face without obfuscated text keeps its world-space
/// geometry from the bake; an obfuscated face draws fresh glyphs each frame.
#[derive(Debug, Clone, PartialEq)]
pub struct SignTextSurface {
    pub submission: SignTextSubmission,
    /// The truncated lines, kept to redraw obfuscated glyphs or re-bake after a dynamic font
    /// atlas resize.
    lines: [Vec<WorldTextRun>; 4],
    obfuscated: bool,
    dynamic_atlas_dims: Option<(u32, u32)>,
    baked: SignTextGeometry,
}

impl SignTextSurface {
    /// Where line `line_index` starts: centred at `x = -width / 2` (integer halving of
    /// `font.width`), at `y = i * lineHeight - 4 * lineHeight / 2`.
    fn line_origin(
        &self,
        line_index: usize,
        visual: &[VisualGlyph],
        fonts: &HudFontGlyphBook,
    ) -> (f32, f32) {
        let line_height = sign_text_line_height(self.submission.attachment);
        let width = world_text_width(visual.iter().map(|&(ch, run, _)| (ch, run)), fonts);
        (
            (width.wrapping_neg() / 2) as f32,
            (line_index as i32 * line_height - 4 * line_height / 2) as f32,
        )
    }

    #[cfg(test)]
    fn placed_line(&self, line_index: usize, fonts: &HudFontGlyphBook) -> Vec<PlacedGlyph> {
        let visual = world_text_visual_order(&self.lines[line_index], fonts);
        let (x, y) = self.line_origin(line_index, &visual, fonts);
        let mut pen = TextPen::new(x, y, 0);
        visual
            .iter()
            .map(|&glyph| pen.place_world(glyph, fonts, 0, false))
            .collect()
    }

    pub(crate) fn is_obfuscated(&self) -> bool {
        self.obfuscated
    }

    pub(crate) fn dynamic_atlas_dims(&self) -> Option<(u32, u32)> {
        self.dynamic_atlas_dims
    }

    pub(crate) fn rebake(&mut self, fonts: &HudFontGlyphBook) {
        let mut baked = SignTextGeometry::default();
        let mut dims = fonts.dynamic_atlas().map(|atlas| atlas.atlas_dimensions());
        mesh_sign_text(
            &mut baked,
            self,
            fonts,
            &mut TextPen::unobfuscated(0.0, 0.0),
            self.submission.has_glowing_text,
        );
        let dims_after = fonts.dynamic_atlas().map(|atlas| atlas.atlas_dimensions());
        if dims_after != dims {
            dims = dims_after;
            baked = SignTextGeometry::default();
            mesh_sign_text(
                &mut baked,
                self,
                fonts,
                &mut TextPen::unobfuscated(0.0, 0.0),
                self.submission.has_glowing_text,
            );
        }
        self.dynamic_atlas_dims = dims;
        self.baked = baked;
    }

    pub fn is_empty(&self) -> bool {
        self.vertex_count() == 0
    }

    pub fn vertex_count(&self) -> usize {
        self.baked.primary.vertex_count() + self.baked.effects.vertex_count()
    }

    pub fn index_count(&self) -> usize {
        self.vertex_count() / 4 * 6
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SignTextGeometry {
    pub outline: ItemModelMesh,
    pub primary: ItemModelMesh,
    pub effects: ItemModelMesh,
}

pub(crate) struct SignTextRenderState {
    pub(crate) dirty: bool,
    pub(crate) obfuscated: bool,
    pub(crate) buffers: [super::display_text::WorldTextBuffers; 3],
}

impl Default for SignTextRenderState {
    fn default() -> Self {
        Self {
            dirty: true,
            obfuscated: false,
            buffers: std::array::from_fn(|_| super::display_text::WorldTextBuffers::default()),
        }
    }
}

impl SignTextGeometry {
    fn extend(&mut self, other: &Self, outline: bool) {
        if outline {
            extend_mesh(&mut self.outline, &other.outline);
        }
        extend_mesh(&mut self.primary, &other.primary);
        extend_mesh(&mut self.effects, &other.effects);
    }
}

fn extend_mesh(target: &mut ItemModelMesh, mesh: &ItemModelMesh) {
    let base = u32::try_from(target.vertices.len()).expect("sign text vertex count fits in u32");
    target.vertices.extend_from_slice(&mesh.vertices);
    target
        .indices
        .extend(mesh.indices.iter().map(|index| index + base));
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
/// `font.split(component, maxTextLineWidth)` first segment
pub fn truncate_sign_line_runs_to_width(
    runs: &[WorldTextRun],
    max_width: u32,
    glyphs: &HudFontGlyphBook,
) -> Vec<WorldTextRun> {
    split_world_text_lines(runs, max_width.min(i32::MAX as u32) as i32, glyphs)
        .into_iter()
        .next()
        .unwrap_or_default()
}

/// The centring width of one sign line (`Font.width`).
pub fn sign_line_runs_width(runs: &[WorldTextRun], glyphs: &HudFontGlyphBook) -> i32 {
    world_text_width(world_text_chars(runs), glyphs)
}

/// Bakes one sign face's text, transcribing `AbstractSignRenderer.submitSignText`:
/// - each line is width-truncated to `getMaxTextLineWidth()` and centred at
///   `x = -width / 2` (vanilla integer halving of `font.width`);
/// - line `i` sits at `y = i * lineHeight - 4 * lineHeight / 2`;
/// - the line color is the glowing raw dye color or the `getDarkColor`
///   darkened one; per-run style colors override it
///   (`StringRenderOutput.getTextColor`).
#[allow(clippy::too_many_arguments)]
pub fn bake_sign_text_surface(
    position: [f32; 3],
    attachment: SignModelAttachment,
    body_rot_degrees: f32,
    front: bool,
    lines: &[Vec<WorldTextRun>; 4],
    dye_text_color: u32,
    has_glowing_text: bool,
    light: [f32; 2],
    glyphs: &HudFontGlyphBook,
) -> Option<SignTextSurface> {
    let max_width = sign_max_text_line_width(attachment);
    let lines = lines
        .each_ref()
        .map(|line| truncate_sign_line_runs_to_width(line, max_width, glyphs));
    let mut surface = SignTextSurface {
        submission: SignTextSubmission {
            position,
            attachment,
            front,
            color: sign_text_base_color(dye_text_color, has_glowing_text),
            outline_color: sign_text_dark_color(dye_text_color, has_glowing_text),
            has_glowing_text,
            light,
            transform: sign_text_transformation(position, attachment, body_rot_degrees, front),
        },
        obfuscated: lines.iter().flatten().any(|run| run.style.obfuscated),
        lines,
        dynamic_atlas_dims: None,
        baked: SignTextGeometry::default(),
    };
    surface.rebake(glyphs);
    (!surface.is_empty()).then_some(surface)
}

/// Draws every line of one face: the eight outline passes when `outline`, then the glyphs, then
/// the effect bars. All obfuscated lookups draw from `pen`'s random stream in that order.
fn mesh_sign_text(
    geometry: &mut SignTextGeometry,
    surface: &SignTextSurface,
    fonts: &HudFontGlyphBook,
    pen: &mut TextPen,
    outline: bool,
) {
    let submission = &surface.submission;
    let color = 0xFF00_0000 | submission.color;
    let outline_color = 0xFF00_0000 | submission.outline_color;
    let mut outline_mesh = TextMesh::default();
    let mut primary = TextMesh::default();
    let mut effects = TextMesh::default();
    for (line_index, line) in surface.lines.iter().enumerate() {
        let visual = world_text_visual_order(line, fonts);
        let (x, y) = surface.line_origin(line_index, &visual, fonts);
        if outline {
            // `Font.drawInBatch8xOutline`: nested x/y order, excluding the
            // centre. Each pass performs its own obfuscated glyph lookup and
            // keeps the runs' shadow overrides.
            for xo in -1..=1 {
                for yo in -1..=1 {
                    if xo == 0 && yo == 0 {
                        continue;
                    }
                    pen.x = x;
                    pen.y = y;
                    for &glyph in &visual {
                        let placed = pen.place_world(glyph, fonts, outline_color, false);
                        let shifted = PlacedGlyph {
                            x: placed.x + xo as f32 * placed.glyph.shadow_offset,
                            y: placed.y + yo as f32 * placed.glyph.shadow_offset,
                            ..placed
                        };
                        for (quad, z, argb) in shifted.glyph_quads() {
                            outline_mesh.push_glyph(&quad, z, argb, submission.light);
                        }
                    }
                }
            }
        }
        pen.x = x;
        pen.y = y;
        mesh_world_text_line(
            &mut primary,
            Some(&mut effects),
            &visual,
            pen,
            fonts,
            |run| run.color.map_or(color, |rgb| 0xFF00_0000 | rgb),
            false,
            submission.light,
            SIGN_EFFECT_UVS,
        );
    }
    geometry.extend(
        &SignTextGeometry {
            outline: outline_mesh.transformed(submission.transform),
            primary: primary.transformed(submission.transform),
            effects: effects.transformed(submission.transform),
        },
        true,
    );
}

/// The white effect material's corners.
const SIGN_EFFECT_UVS: [[f32; 2]; 4] = [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];

pub fn sign_text_draws_outline(surface: &SignTextSurface, context: SignTextOutlineContext) -> bool {
    if !surface.submission.has_glowing_text {
        return false;
    }
    // Vanilla always outlines glowing black text, independent of camera.
    if surface.submission.color == 0 || context.first_person_scoping {
        return true;
    }
    let Some(camera) = context
        .camera_entity_position
        .filter(|position| position.iter().all(|coordinate| coordinate.is_finite()))
    else {
        return false;
    };
    let block_center = surface
        .submission
        .position
        .map(|coordinate| coordinate.floor() + 0.5);
    camera
        .into_iter()
        .zip(block_center)
        .map(|(camera, center)| (camera - center).powi(2))
        .sum::<f32>()
        < SIGN_TEXT_OUTLINE_DISTANCE_SQUARED
}

pub fn merge_sign_text_surfaces(
    surfaces: &[SignTextSurface],
    visible_positions: &[[i32; 3]],
    fonts: &HudFontGlyphBook,
    obfuscated_seed: u64,
    outline_context: SignTextOutlineContext,
) -> SignTextGeometry {
    let mut geometry = SignTextGeometry::default();
    let mut pen = TextPen::new(0.0, 0.0, obfuscated_seed);
    for surface in surfaces.iter().filter(|surface| {
        visible_positions
            .binary_search(&sign_text_block_position(surface))
            .is_ok()
    }) {
        let outline = sign_text_draws_outline(surface, outline_context);
        if surface.obfuscated {
            mesh_sign_text(&mut geometry, surface, fonts, &mut pen, outline);
        } else {
            geometry.extend(&surface.baked, outline);
        }
    }
    geometry
}

pub fn sign_text_block_position(surface: &SignTextSurface) -> [i32; 3] {
    surface
        .submission
        .position
        .map(|coordinate| coordinate.floor() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_render_types::{HudAsciiGlyph, HudTextStyle, HudUvRect};
    use bbb_render_types::{HudFontGlyphMap, HudObfuscatedGlyphPool, HudObfuscatedRandom};

    /// A minimal ascent-7 glyph table: advance 6 (space 4), 5×8 cells.
    fn test_glyphs() -> HudFontGlyphMap {
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
        glyphs
    }

    fn bidirectional_glyphs() -> HudFontGlyphMap {
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
        glyphs.set_default_right_to_left(true);
        glyphs
    }

    fn plain_runs(text: &str) -> Vec<WorldTextRun> {
        vec![WorldTextRun::plain(text)]
    }

    #[test]
    fn sign_text_uses_missing_glyph_with_no_registered_characters() {
        let mut glyphs = HudFontGlyphMap::new();
        let missing = HudAsciiGlyph {
            width: 5,
            height: 8,
            advance: 6.0,
            uv: HudUvRect {
                min: [0.2, 0.4],
                max: [0.3, 0.6],
            },
            ..Default::default()
        };
        glyphs.set_missing_glyph(missing);
        let lines = [plain_runs("缺"), vec![], vec![], vec![]];
        let surface = bake_sign_text_surface(
            [0.0; 3],
            SignModelAttachment::Standing,
            0.0,
            true,
            &lines,
            0xffffff,
            false,
            [0.0, 1.0],
            &HudFontGlyphBook::from(glyphs.clone()),
        )
        .unwrap();
        assert_eq!(surface.vertex_count(), 4);
        let geometry = surface_geometry(&surface, &HudFontGlyphBook::from(glyphs.clone()), 0);
        assert_eq!(geometry.primary.vertices[0].uv, missing.uv.min);
        glyphs.insert_first_wins(
            '?',
            HudAsciiGlyph {
                advance: 99.0,
                ..Default::default()
            },
        );
        assert_eq!(
            HudFontGlyphBook::from(glyphs.clone())
                .glyphs(None)
                .resolve('缺'),
            missing
        );
    }

    fn assert_vec3_close(actual: glam::Vec3, expected: glam::Vec3) {
        assert!(
            (actual - expected).length() < 1e-5,
            "expected {actual:?} ~= {expected:?}"
        );
    }

    fn surface_geometry(
        surface: &SignTextSurface,
        glyphs: &HudFontGlyphBook,
        seed: u64,
    ) -> SignTextGeometry {
        surface_geometry_with_context(surface, glyphs, seed, SignTextOutlineContext::default())
    }

    fn surface_geometry_with_context(
        surface: &SignTextSurface,
        glyphs: &HudFontGlyphBook,
        seed: u64,
        context: SignTextOutlineContext,
    ) -> SignTextGeometry {
        merge_sign_text_surfaces(
            std::slice::from_ref(surface),
            &[sign_text_block_position(surface)],
            glyphs,
            seed,
            context,
        )
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
            Vec3::new(0.5, 0.833_333_3, 0.5 - 0.046_666_667),
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
        let fits = truncate_sign_line_runs_to_width(
            &plain_runs(&"a".repeat(15)),
            90,
            &HudFontGlyphBook::from(glyphs.clone()),
        );
        assert_eq!(fits[0].text.len(), 15);
        // Hard break with no space: the 16th glyph pushes 96 > 90, so the
        // first line keeps 15 chars (the overflowing char is excluded).
        let hard = truncate_sign_line_runs_to_width(
            &plain_runs(&"a".repeat(20)),
            90,
            &HudFontGlyphBook::from(glyphs.clone()),
        );
        assert_eq!(hard[0].text.len(), 15);
        // Word wrap: the accumulated width passes 90 inside the second word,
        // so the line breaks at the space and the space is consumed.
        let text = format!("{} {}", "a".repeat(8), "b".repeat(10)); // 48 + 4 + 60
        let wrapped = truncate_sign_line_runs_to_width(
            &plain_runs(&text),
            90,
            &HudFontGlyphBook::from(glyphs.clone()),
        );
        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0].text, "a".repeat(8));
        // Bold advances are 7px (GlyphInfo.getAdvance(bold)); 13 bold glyphs
        // reach 91 > 90 so only 12 stay.
        let bold_runs = vec![WorldTextRun {
            font: None,
            shadow_color: None,
            text: "a".repeat(20),
            style: HudTextStyle {
                bold: true,
                ..Default::default()
            },
            color: None,
            event_id: None,
        }];
        let bold = truncate_sign_line_runs_to_width(
            &bold_runs,
            90,
            &HudFontGlyphBook::from(glyphs.clone()),
        );
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
            &HudFontGlyphBook::from(glyphs.clone()),
        )
        .unwrap();
        // 3 glyphs, one quad each (no bold, no shadow).
        assert_eq!(surface.vertex_count(), 12);
        assert_eq!(surface.index_count(), 18);
        // Line 0 ("ab", width 12): x starts at -12/2 = -6; line 0 sits at
        // y = 0*10 - 20 = -20 font px. World TL corner of the first glyph:
        // (0.5 - 6s, 0.83333334 + 20s, 0.5 + 0.046666667), s = 0.010416667.
        let geometry = surface_geometry(&surface, &HudFontGlyphBook::from(glyphs.clone()), 0);
        let vertices = &geometry.primary.vertices;
        let expected = Vec3::new(
            0.5 - 6.0 * 0.010_416_667,
            0.833_333_3 + 20.0 * 0.010_416_667,
            0.546_666_7,
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
            Vec3::new(0.5 - 3.0 * 0.010_416_667, 0.833_333_3, 0.546_666_7),
        );
    }

    #[test]
    fn sign_text_truncates_logically_then_draws_the_completed_line_in_visual_order() {
        let glyphs = bidirectional_glyphs();
        let logical = plain_runs("123-");
        assert_eq!(
            truncate_sign_line_runs_to_width(
                &logical,
                SIGN_MAX_TEXT_LINE_WIDTH,
                &HudFontGlyphBook::from(glyphs.clone())
            )[0]
            .text,
            "123-"
        );

        let surface = bake_sign_text_surface(
            [0.0; 3],
            SignModelAttachment::Standing,
            0.0,
            true,
            &[logical, Vec::new(), Vec::new(), Vec::new()],
            0,
            false,
            [0.0, 1.0],
            &HudFontGlyphBook::from(glyphs.clone()),
        )
        .unwrap();
        let geometry = surface_geometry(&surface, &HudFontGlyphBook::from(glyphs.clone()), 0);
        let glyph_uvs = geometry
            .primary
            .vertices
            .as_chunks::<4>()
            .0
            .iter()
            .map(|vertices| vertices[0].uv[0])
            .collect::<Vec<_>>();
        assert_eq!(glyph_uvs, [0.4, 0.1, 0.2, 0.3]);
    }

    #[test]
    fn bake_applies_run_color_overrides_and_bold_double_draw() {
        let glyphs = test_glyphs();
        let lines = [
            vec![WorldTextRun {
                font: None,
                shadow_color: None,
                text: "a".to_string(),
                style: HudTextStyle {
                    bold: true,
                    ..Default::default()
                },
                color: Some(0x11_22_33),
                event_id: None,
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
            &HudFontGlyphBook::from(glyphs.clone()),
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
        let geometry = surface_geometry(&surface, &HudFontGlyphBook::from(glyphs.clone()), 0);
        assert!(geometry
            .primary
            .vertices
            .iter()
            .all(|vertex| vertex.color == expected));
    }

    #[test]
    fn obfuscated_sign_text_cycles_equal_advance_glyphs_without_moving_layout() {
        let mut glyphs = HudFontGlyphMap::new();
        for (ch, min_x) in [('a', 0.1), ('b', 0.2), ('c', 0.3), ('?', 0.4)] {
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
        let lines = [
            vec![WorldTextRun {
                font: None,
                shadow_color: None,
                text: "aa".to_string(),
                style: HudTextStyle {
                    obfuscated: true,
                    ..Default::default()
                },
                color: None,
                event_id: None,
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
            0xFF_FF_FF,
            false,
            [0.0, 1.0],
            &HudFontGlyphBook::from(glyphs.clone()),
        )
        .unwrap();
        let first = surface_geometry(&surface, &HudFontGlyphBook::from(glyphs.clone()), 0);
        let first_uvs = first
            .primary
            .vertices
            .as_chunks::<4>()
            .0
            .iter()
            .map(|vertices| vertices[0].uv[0])
            .collect::<Vec<_>>();
        let (changed_seed, changed) = (1..100)
            .map(|seed| {
                (
                    seed,
                    surface_geometry(&surface, &HudFontGlyphBook::from(glyphs.clone()), seed),
                )
            })
            .find(|(_, geometry)| {
                geometry
                    .primary
                    .vertices
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|vertices| vertices[0].uv[0])
                    .collect::<Vec<_>>()
                    != first_uvs
            })
            .expect("a later deterministic frame seed changes the selected glyphs");
        assert!(changed_seed > 0);
        assert_eq!(first.primary.indices, changed.primary.indices);
        assert_eq!(
            first
                .primary
                .vertices
                .as_chunks::<4>()
                .0
                .iter()
                .map(|vertices| vertices[0].position)
                .collect::<Vec<_>>(),
            changed
                .primary
                .vertices
                .as_chunks::<4>()
                .0
                .iter()
                .map(|vertices| vertices[0].position)
                .collect::<Vec<_>>(),
            "equal-advance substitutes must not shift either glyph pen"
        );
        assert!(first_uvs.iter().all(|uv| [0.1, 0.2, 0.3, 0.4].contains(uv)));
    }

    #[test]
    fn underline_and_strikethrough_emit_original_advance_white_material_quads() {
        let glyphs = test_glyphs();
        let style = HudTextStyle {
            underlined: true,
            strikethrough: true,
            ..Default::default()
        };
        let surface = bake_sign_text_surface(
            [0.0; 3],
            SignModelAttachment::Standing,
            0.0,
            true,
            &[
                vec![WorldTextRun {
                    font: None,
                    shadow_color: None,
                    text: "a ".to_string(),
                    style,
                    color: Some(0x12_34_56),
                    event_id: None,
                }],
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ],
            0,
            false,
            [0.25, 0.75],
            &HudFontGlyphBook::from(glyphs.clone()),
        )
        .unwrap();
        let geometry = surface_geometry(&surface, &HudFontGlyphBook::from(glyphs.clone()), 0);
        assert_eq!(geometry.primary.vertex_count(), 4);
        assert_eq!(geometry.effects.vertex_count(), 16);
        assert_eq!(geometry.effects.index_count(), 24);
        let inverse = surface.submission.transform.inverse();
        let effect_font_corners: Vec<[[f32; 3]; 4]> = geometry
            .effects
            .vertices
            .as_chunks::<4>()
            .0
            .iter()
            .map(|vertices| {
                std::array::from_fn(|index| {
                    inverse
                        .transform_point3(Vec3::from_array(vertices[index].position))
                        .to_array()
                })
            })
            .collect::<Vec<_>>();
        let expected = [
            [
                [-6.0, -16.5, 0.01],
                [-6.0, -15.5, 0.01],
                [1.0, -15.5, 0.01],
                [1.0, -16.5, 0.01],
            ],
            [
                [-6.0, -12.0, 0.01],
                [-6.0, -11.0, 0.01],
                [1.0, -11.0, 0.01],
                [1.0, -12.0, 0.01],
            ],
            [
                [1.0, -16.5, 0.01],
                [1.0, -15.5, 0.01],
                [5.0, -15.5, 0.01],
                [5.0, -16.5, 0.01],
            ],
            [
                [1.0, -12.0, 0.01],
                [1.0, -11.0, 0.01],
                [5.0, -11.0, 0.01],
                [5.0, -12.0, 0.01],
            ],
        ];
        for (actual, expected) in effect_font_corners.into_iter().zip(expected) {
            for (actual, expected) in actual.into_iter().zip(expected) {
                assert_vec3_close(Vec3::from_array(actual), Vec3::from_array(expected));
            }
        }
        assert!(geometry.effects.vertices.iter().all(|vertex| {
            vertex.color
                == [
                    0x12 as f32 / 255.0,
                    0x34 as f32 / 255.0,
                    0x56 as f32 / 255.0,
                    1.0,
                ]
                && vertex.light == [0.25, 0.75]
        }));
    }

    #[test]
    fn glowing_outline_uses_strict_camera_range_scoping_and_black_override() {
        let glyphs = test_glyphs();
        let lines = [plain_runs("a"), Vec::new(), Vec::new(), Vec::new()];
        let red = bake_sign_text_surface(
            [0.0; 3],
            SignModelAttachment::Standing,
            0.0,
            true,
            &lines,
            0xFF_00_00,
            true,
            [1.0, 1.0],
            &HudFontGlyphBook::from(glyphs.clone()),
        )
        .unwrap();
        let boundary = surface_geometry_with_context(
            &red,
            &HudFontGlyphBook::from(glyphs.clone()),
            0,
            SignTextOutlineContext {
                camera_entity_position: Some([16.5, 0.5, 0.5]),
                first_person_scoping: false,
            },
        );
        assert!(
            boundary.outline.is_empty(),
            "distance is strictly below 16²"
        );
        let near = surface_geometry_with_context(
            &red,
            &HudFontGlyphBook::from(glyphs.clone()),
            0,
            SignTextOutlineContext {
                camera_entity_position: Some([16.49, 0.5, 0.5]),
                first_person_scoping: false,
            },
        );
        assert_eq!(near.outline.vertex_count(), 8 * 4);
        assert!(near.outline.vertices.iter().all(|vertex| {
            vertex.color == [0x66 as f32 / 255.0, 0.0, 0.0, 1.0] && vertex.light == [1.0, 1.0]
        }));
        let scoped = surface_geometry_with_context(
            &red,
            &HudFontGlyphBook::from(glyphs.clone()),
            0,
            SignTextOutlineContext {
                camera_entity_position: None,
                first_person_scoping: true,
            },
        );
        assert_eq!(scoped.outline.vertex_count(), 8 * 4);

        let black = bake_sign_text_surface(
            [0.0; 3],
            SignModelAttachment::Standing,
            0.0,
            true,
            &lines,
            0,
            true,
            [1.0, 1.0],
            &HudFontGlyphBook::from(glyphs.clone()),
        )
        .unwrap();
        let black = surface_geometry(&black, &HudFontGlyphBook::from(glyphs.clone()), 0);
        assert_eq!(black.outline.vertex_count(), 8 * 4);
        assert!(black.outline.vertices.iter().all(|vertex| {
            vertex.color
                == [
                    0xF0 as f32 / 255.0,
                    0xEB as f32 / 255.0,
                    0xCC as f32 / 255.0,
                    1.0,
                ]
        }));

        let non_glowing = bake_sign_text_surface(
            [0.0; 3],
            SignModelAttachment::Standing,
            0.0,
            true,
            &lines,
            0,
            false,
            [0.0, 1.0],
            &HudFontGlyphBook::from(glyphs.clone()),
        )
        .unwrap();
        let non_glowing = surface_geometry_with_context(
            &non_glowing,
            &HudFontGlyphBook::from(glyphs.clone()),
            0,
            SignTextOutlineContext {
                camera_entity_position: Some([0.5, 0.5, 0.5]),
                first_person_scoping: true,
            },
        );
        assert!(non_glowing.outline.is_empty());
    }

    #[test]
    fn glowing_outline_offsets_and_obfuscated_lookups_follow_vanilla_order() {
        let mut glyphs = HudFontGlyphMap::new();
        for (ch, min_x) in [('a', 0.1), ('b', 0.2), ('c', 0.3), ('?', 0.4)] {
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
        let surface = bake_sign_text_surface(
            [0.0; 3],
            SignModelAttachment::Standing,
            0.0,
            true,
            &[
                vec![WorldTextRun {
                    font: None,
                    shadow_color: None,
                    text: "a".to_string(),
                    style: HudTextStyle {
                        obfuscated: true,
                        ..Default::default()
                    },
                    color: None,
                    event_id: None,
                }],
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ],
            0,
            true,
            [1.0, 1.0],
            &HudFontGlyphBook::from(glyphs.clone()),
        )
        .unwrap();
        let seed = 7;
        let geometry = surface_geometry(&surface, &HudFontGlyphBook::from(glyphs.clone()), seed);
        let inverse = surface.submission.transform.inverse();
        let outline_tops = geometry
            .outline
            .vertices
            .as_chunks::<4>()
            .0
            .iter()
            .map(|vertices| {
                inverse
                    .transform_point3(Vec3::from_array(vertices[0].position))
                    .to_array()
            })
            .collect::<Vec<_>>();
        let expected_tops = [
            [-4.0, -21.0, 0.0],
            [-4.0, -20.0, 0.0],
            [-4.0, -19.0, 0.0],
            [-3.0, -21.0, 0.0],
            [-3.0, -19.0, 0.0],
            [-2.0, -21.0, 0.0],
            [-2.0, -20.0, 0.0],
            [-2.0, -19.0, 0.0],
        ];
        for (actual, expected) in outline_tops.into_iter().zip(expected_tops) {
            assert_vec3_close(Vec3::from_array(actual), Vec3::from_array(expected));
        }

        let pool = HudObfuscatedGlyphPool::from_glyph_map(&glyphs);
        let mut random = HudObfuscatedRandom::with_seed(seed);
        let expected_uvs = (0..9)
            .map(|_| pool.random_glyph(6.0, &mut random).unwrap().uv.min[0])
            .collect::<Vec<_>>();
        let actual_uvs = geometry
            .outline
            .vertices
            .as_chunks::<4>()
            .0
            .iter()
            .chain(geometry.primary.vertices.as_chunks::<4>().0)
            .map(|vertices| vertices[0].uv[0])
            .collect::<Vec<_>>();
        assert_eq!(actual_uvs, expected_uvs);
    }

    #[test]
    fn sign_text_submits_zero_bitmap_dimensions_but_empty_only_advances() {
        for (width, height, render_scale) in [(0, 8, 1.0), (5, 0, f32::INFINITY)] {
            let mut glyphs = test_glyphs();
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
            let bake = |text| {
                bake_sign_text_surface(
                    [0.0; 3],
                    SignModelAttachment::Standing,
                    0.0,
                    true,
                    &[plain_runs(text), vec![], vec![], vec![]],
                    0xffffff,
                    false,
                    [0.0, 1.0],
                    &HudFontGlyphBook::from(glyphs.clone()),
                )
            };
            let bitmap_only = bake("x").unwrap();
            assert_eq!(bitmap_only.vertex_count(), 4);
            assert!(bake(" ").is_none());

            let bitmap = bake("xa").unwrap();
            let empty = bake(" a").unwrap();
            assert_eq!(bitmap.vertex_count(), 8);
            assert_eq!(empty.vertex_count(), 4);
            let fonts = HudFontGlyphBook::from(glyphs.clone());
            assert_eq!(bitmap.placed_line(0, &fonts)[1].x, -1.0);
            assert_eq!(empty.placed_line(0, &fonts)[1].x, -1.0);
            let bitmap = surface_geometry(&bitmap, &HudFontGlyphBook::from(glyphs.clone()), 0);
            let empty = surface_geometry(&empty, &HudFontGlyphBook::from(glyphs.clone()), 0);
            assert_eq!(bitmap.primary.vertices.len(), 8);
            assert_eq!(bitmap.primary.indices.len(), 12);
            assert_eq!(empty.primary.vertices.len(), 4);
            for (actual, expected) in bitmap.primary.vertices[4..]
                .iter()
                .zip(&empty.primary.vertices)
            {
                assert_eq!(actual.position, expected.position);
                assert_eq!(actual.uv, expected.uv);
            }
            if height == 0 {
                assert!(bitmap.primary.vertices[..4]
                    .iter()
                    .any(|vertex| vertex.position.iter().any(|value| value.is_nan())));
            }
        }
    }

    #[test]
    fn bake_returns_none_for_empty_or_spaces_only_faces() {
        let glyphs = test_glyphs();
        let empty: [Vec<WorldTextRun>; 4] = Default::default();
        assert!(bake_sign_text_surface(
            [0.0; 3],
            SignModelAttachment::Standing,
            0.0,
            true,
            &empty,
            0,
            false,
            [0.0, 1.0],
            &HudFontGlyphBook::from(glyphs.clone()),
        )
        .is_none());
        // EmptyGlyph retains advance without a bitmap.
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
            &HudFontGlyphBook::from(glyphs.clone()),
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
            &HudFontGlyphBook::from(glyphs.clone()),
        )
        .unwrap();
        // 12 glyphs * 6 = 72 > 60: hard break keeps 10 glyphs (60 is not > 60).
        assert_eq!(surface.vertex_count(), 40);
        // Line 0 of a hanging sign: y = 0*9 - 18 = -18 font px at scale
        // 0.0140625, from the hanging text origin y = 0.2955.
        let geometry = surface_geometry(&surface, &HudFontGlyphBook::from(glyphs.clone()), 0);
        let tl = Vec3::from_array(geometry.primary.vertices[0].position);
        let expected_y = (0.9375 - 0.3125 - 0.32) + 18.0 * 0.014_062_5;
        assert!((tl.y - expected_y).abs() < 1e-5);
    }

    #[test]
    fn merged_sign_text_geometry_keeps_only_visible_block_positions() {
        let glyphs = test_glyphs();
        let lines = [plain_runs("a"), Vec::new(), Vec::new(), Vec::new()];
        let hidden = bake_sign_text_surface(
            [1.0, 2.0, 3.0],
            SignModelAttachment::Standing,
            0.0,
            true,
            &lines,
            0,
            false,
            [0.0, 1.0],
            &HudFontGlyphBook::from(glyphs.clone()),
        )
        .unwrap();
        let visible = bake_sign_text_surface(
            [4.0, 5.0, 6.0],
            SignModelAttachment::Standing,
            0.0,
            true,
            &lines,
            0,
            false,
            [0.0, 1.0],
            &HudFontGlyphBook::from(glyphs.clone()),
        )
        .unwrap();

        let geometry = merge_sign_text_surfaces(
            std::slice::from_ref(&hidden),
            &[],
            &HudFontGlyphBook::from(glyphs.clone()),
            0,
            SignTextOutlineContext::default(),
        );
        assert!(geometry.primary.vertices.is_empty());
        assert!(geometry.primary.indices.is_empty());

        let geometry = merge_sign_text_surfaces(
            &[hidden, visible],
            &[[4, 5, 6]],
            &HudFontGlyphBook::from(glyphs.clone()),
            0,
            SignTextOutlineContext::default(),
        );
        assert_eq!(geometry.primary.indices.len(), 6);
        assert!(geometry
            .primary
            .vertices
            .iter()
            .all(|vertex| (4.0..5.0).contains(&vertex.position[0])));
    }
    #[test]
    fn fractional_sign_metrics_keep_negative_and_zero_width_visible_lines() {
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
        for (text, bold, expected_width, positions) in [
            ("aa", false, 1, vec![0.0, 0.5]),
            ("ba", false, -2, vec![1.0, -2.0]),
            ("aaaaaab", false, 0, vec![0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0]),
            ("aa", true, 2, vec![-1.0, 0.0]),
        ] {
            let mut runs = plain_runs(text);
            runs[0].style.bold = bold;
            assert_eq!(
                sign_line_runs_width(&runs, &HudFontGlyphBook::from(glyphs.clone())),
                expected_width
            );
            let surface = bake_sign_text_surface(
                [0.0; 3],
                SignModelAttachment::Standing,
                0.0,
                true,
                &[runs, Vec::new(), Vec::new(), Vec::new()],
                0,
                false,
                [0.0, 1.0],
                &HudFontGlyphBook::from(glyphs.clone()),
            )
            .unwrap();
            assert_eq!(
                surface
                    .placed_line(0, &HudFontGlyphBook::from(glyphs.clone()))
                    .iter()
                    .map(|glyph| glyph.x)
                    .collect::<Vec<_>>(),
                positions
            );
        }
    }

    #[test]
    fn sign_run_that_only_changes_its_event_restarts_the_effect_expansion() {
        // `SubStringSource.substring` cuts a part where the resolved style
        // changes, and vanilla `Style` equality counts the click, hover and
        // insertion events, so two runs drawing
        // the same glyphs in the same visible style are still two parts.
        let glyphs = test_glyphs();
        let underlined = |text: &str, event_id| WorldTextRun {
            text: text.into(),
            style: HudTextStyle {
                underlined: true,
                ..Default::default()
            },
            event_id,
            ..Default::default()
        };
        let starts = |runs: Vec<WorldTextRun>| {
            bake_sign_text_surface(
                [0.0; 3],
                SignModelAttachment::Standing,
                0.0,
                true,
                &[runs, vec![], vec![], vec![]],
                0,
                false,
                [1.0; 2],
                &HudFontGlyphBook::from(glyphs.clone()),
            )
            .unwrap()
            .placed_line(0, &HudFontGlyphBook::from(glyphs.clone()))
            .iter()
            .map(|glyph| glyph.subsequence_start)
            .collect::<Vec<_>>()
        };
        // One part: `position` keeps counting, so only the first glyph expands.
        assert_eq!(
            starts(vec![underlined("ab", None), underlined("ab", None)]),
            [true, false, false, false]
        );
        // The second run carries a different event triple, so its own first
        // glyph is at `position == 0` and expands one pixel left
        assert_eq!(
            starts(vec![underlined("ab", None), underlined("ab", Some(0))]),
            [true, false, true, false]
        );
    }
}

#[cfg(test)]
#[path = "sign_text/shadow_tests.rs"]
mod shadow_tests;

#[cfg(test)]
mod line_break_tests {
    use super::*;
    use bbb_render_types::{HudAsciiGlyph, HudFontGlyphMap};

    #[test]
    fn obfuscated_face_is_kept_by_its_original_glyphs() {
        let mut glyphs = HudFontGlyphMap::new();
        glyphs.insert_first_wins(
            'A',
            HudAsciiGlyph {
                width: 5,
                height: 8,
                advance: 6.0,
                ..Default::default()
            },
        );
        // An equal-advance bucket of mostly empty glyphs.
        for ch in 'a'..='z' {
            glyphs.insert_first_wins(ch, HudAsciiGlyph::empty(6.0));
        }
        let fonts = HudFontGlyphBook::from(glyphs);
        let mut run = WorldTextRun::plain("A");
        run.style.obfuscated = true;
        let surface = bake_sign_text_surface(
            [0.0; 3],
            SignModelAttachment::Standing,
            0.0,
            true,
            &[vec![run], vec![], vec![], vec![]],
            0,
            false,
            [1.0; 2],
            &fonts,
        );
        assert_eq!(surface.map(|surface| surface.vertex_count()), Some(4));
    }

    #[test]
    fn sign_first_line_stops_at_newline_and_keeps_first_nonzero_wide_glyph() {
        let mut glyphs = HudFontGlyphMap::new();
        for (ch, advance) in [('a', 6.0), ('b', 6.0), ('z', 0.0), ('w', 120.0), (' ', 1.0)] {
            glyphs.insert_first_wins(ch, HudAsciiGlyph::empty(advance));
        }
        let first = |text: &str, width| {
            truncate_sign_line_runs_to_width(
                &[WorldTextRun::plain(text)],
                width,
                &HudFontGlyphBook::from(glyphs.clone()),
            )
            .into_iter()
            .map(|run| run.text)
            .collect::<String>()
        };
        assert_eq!(first("a\nb", 90), "a");
        assert_eq!(first("zzwa", 90), "zzw");
        assert_eq!(first("  ", 0), " ");
        assert_eq!(first("a b", 7), "a");
        assert_eq!(first("\na", 90), "");
    }
}

#[cfg(test)]
#[path = "sign_text/font_tests.rs"]
mod font_tests;
