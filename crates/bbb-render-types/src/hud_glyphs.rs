use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};

use rustc_hash::FxHashMap;

/// Vanilla text-line reference baseline in font pixels: `GlyphBitmap.getTop()`
/// computes a glyph quad's top edge as `7.0F - getBearingTop()`, where
/// `getBearingTop()` is the bitmap provider's `ascent`
/// (`BitmapProvider.Glyph.bake`). An ascent-7 page (`font/ascii.png`,
/// `font/nonlatin_european.png`) therefore starts exactly at the line top,
/// while `font/accented.png` (ascent 10) rises three pixels above it.
pub const HUD_FONT_BASELINE: i32 = 7;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudUvRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudDigitGlyph {
    /// EmptyGlyph emits no bitmap; zero pixel dimensions alone do not make a glyph empty.
    pub is_empty: bool,
    pub uv: HudUvRect,
    /// Raw bitmap width in pixels.
    pub width: u32,
    /// Raw bitmap height in pixels.
    pub height: u32,
    pub advance: f32,
    /// Vanilla `GlyphBitmap.getBearingTop()`: distance from the reference
    /// baseline up to the glyph cell top. A bitmap provider's constant
    /// `ascent`, and a `ttf` glyph's `bitmap_top / oversample`.
    pub bearing_top: f32,
    /// Vanilla `GlyphBitmap.getBearingLeft()`: horizontal offset of the glyph
    /// cell's left edge from the pen. Zero for bitmap, space and unihex
    /// providers, and `bitmap_left / oversample` for `ttf` glyphs.
    pub bearing_left: f32,
    /// BitmapProvider scale (`1 / GlyphBitmap.getOversample()` for `ttf`
    /// glyphs); GlyphBitmap divides pixel size by its reciprocal.
    pub render_scale: f32,
    /// `GlyphInfo.getBoldOffset()` for this provider.
    pub bold_offset: f32,
    /// `GlyphInfo.getShadowOffset()` for this provider.
    pub shadow_offset: f32,
}

impl HudDigitGlyph {
    pub fn empty(advance: f32) -> Self {
        Self {
            is_empty: true,
            advance,
            ..Self::default()
        }
    }

    /// Vertical offset of the glyph cell top relative to the text line top,
    /// mirroring vanilla `GlyphBitmap.getTop()` = `7.0F - getBearingTop()`.
    /// Zero for ascent-7 pages, negative (above the line top) for taller
    /// pages such as `font/accented.png` (ascent 10 -> -3).
    pub fn baseline_offset(&self) -> f32 {
        HUD_FONT_BASELINE as f32 - self.bearing_top
    }

    pub fn rendered_width(&self) -> f32 {
        0.0 + self.width as f32 / (1.0 / self.render_scale)
    }

    pub fn rendered_height(&self) -> f32 {
        self.height as f32 / (1.0 / self.render_scale)
    }
}

impl Default for HudDigitGlyph {
    fn default() -> Self {
        Self {
            is_empty: false,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            width: 0,
            height: 0,
            advance: 0.0,
            bearing_top: HUD_FONT_BASELINE as f32,
            bearing_left: 0.0,
            render_scale: 1.0,
            bold_offset: HUD_FONT_BOLD_OFFSET,
            shadow_offset: HUD_FONT_SHADOW_OFFSET,
        }
    }
}

pub type HudAsciiGlyph = HudDigitGlyph;

/// Vanilla `GlyphInfo.getBoldOffset()` default (`1.0`): bold redraws each glyph
/// shifted this many font pixels right and widens every advance by the same
/// amount (`GlyphInfo.getAdvance(bold)`).
pub const HUD_FONT_BOLD_OFFSET: f32 = 1.0;
/// Vanilla `GlyphInfo.getShadowOffset()` default (`1.0`): the drop shadow is a
/// second draw of the glyph offset one font pixel down and right.
pub const HUD_FONT_SHADOW_OFFSET: f32 = 1.0;
/// Vanilla `BakedSheetGlyph.extraThickness(true)` (`0.1`): bold expands each
/// glyph quad by this many font pixels on every side.
pub const HUD_FONT_BOLD_EXTRA_THICKNESS: f32 = 0.1;

/// Vanilla text-style flags (`net.minecraft.network.chat.Style`) that change a
/// glyph's advance width and/or draw geometry. All-false is the default
/// (unstyled) text path, so existing HUD callers are unaffected.
///
/// Input end: `bbb_protocol::decode_styled_component_summary` preserves these
/// keys as flattened `StyledTextRun`s, which world/native project into the
/// renderer's styled text runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct HudTextStyle {
    pub bold: bool,
    pub italic: bool,
    pub underlined: bool,
    pub strikethrough: bool,
    pub obfuscated: bool,
}

/// The vanilla `Style` facts a `FormattedCharSequence` part boundary depends on:
/// `SubStringSource.substring` splits a visual run where the per-UTF-16-unit resolved
/// style changes (`SubStringSource.java:36-49`), i.e. on the five decorations, the colour,
/// the shadow, the font, and the click/hover/insertion events. The text a component part
/// happens to carry is not one of them, so two parts that resolve to one style stay one
/// part and keep one local index run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HudSubsequenceKey<'a> {
    pub style: HudTextStyle,
    pub color: Option<u32>,
    pub shadow_color: Option<u32>,
    pub font: Option<&'a str>,
    /// The caller's interaction identity for this glyph, standing in for vanilla's
    /// click, hover and insertion style events; `None` when it carries none.
    pub target: Option<usize>,
}

/// One visual glyph's drawing payload paired with its [`HudSubsequenceKey`].
///
/// [`crate::visual_order_indexed`] compares only the key when it cuts parts, so the same
/// payload stays available to the glyph pass.
#[derive(Debug, Clone)]
pub struct HudSubsequence<'a, T> {
    pub payload: T,
    pub key: HudSubsequenceKey<'a>,
}

impl<T> PartialEq for HudSubsequence<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

/// One styled run of HUD text: contiguous characters sharing a resolved
/// vanilla `Style`. The projection of a flattened chat-component run into the
/// renderer's vocabulary: booleans are resolved (`Style.isBold()` ==
/// `bold == Some(true)`), the colour is the resolved `TextColor` value.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HudStyledTextRun {
    pub text: String,
    pub style: HudTextStyle,
    /// Per-run text colour override as `0xRRGGBB` (vanilla
    /// `Style.getColor().getValue()`); `None` keeps the line's base colour
    /// (vanilla `StringRenderOutput.getTextColor` falls back to the draw
    /// call's colour).
    pub color: Option<u32>,
    pub shadow_color: Option<u32>,
}

impl HudStyledTextRun {
    pub fn resolve_shadow_argb(text_argb: u32, explicit: Option<u32>, draw_shadow: bool) -> u32 {
        // Font.java:428-435: explicit zero overrides the default shadow.
        if let Some(shadow) = explicit {
            let text_alpha = (text_argb >> 24) as f32 / 255.0;
            if text_alpha == 1.0 {
                return shadow;
            }
            let shadow_alpha = (shadow >> 24) as f32 / 255.0;
            let alpha = (text_alpha * shadow_alpha * 255.0).floor() as u32;
            return (shadow & 0x00ff_ffff) | (alpha << 24);
        }
        if draw_shadow {
            (text_argb & 0xff00_0000) | ((text_argb >> 2) & 0x003f_3f3f)
        } else {
            0
        }
    }

    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            shadow_color: None,
            text: text.into(),
            style: HudTextStyle::default(),
            color: None,
        }
    }
}

/// One localized trusted-Component run for HUD surfaces that honor per-style
/// font selection. `visual` owns the resolved text/style/color vocabulary used
/// by the common glyph geometry, while `font` retains the exact Component font
/// id so layout and obfuscation can select the matching resource-pack glyphs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HudComponentTextRun {
    pub visual: HudStyledTextRun,
    pub font: Option<String>,
}

impl HudComponentTextRun {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            visual: HudStyledTextRun::plain(text),
            font: None,
        }
    }
}

impl From<HudStyledTextRun> for HudComponentTextRun {
    fn from(visual: HudStyledTextRun) -> Self {
        Self { visual, font: None }
    }
}

/// One glyph-quad draw pass in font-pixel space, mirroring a single
/// `BakedSheetGlyph.render` call. `corners` are `[top_left, bottom_left,
/// bottom_right, top_right]` (vanilla vertex winding); `shadow` marks the passes
/// vanilla draws in the shadow colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudGlyphQuad {
    pub corners: [[f32; 2]; 4],
    pub uv: HudUvRect,
    pub shadow: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudEffectKind {
    Underline,
    Strikethrough,
}

/// An underline / strikethrough bar in font-pixel space, mirroring the
/// rectangles `Font.StringRenderOutput.accept` feeds to `createEffect`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudEffectRect {
    pub kind: HudEffectKind,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

/// One glyph's clickable area in font-pixel space, mirroring
/// `TextRenderable.Styled` (`activeLeft/Top/Right/Bottom`) for a glyph with a
/// bitmap and `EmptyArea` for an `EmptyGlyph` one. [`Self::right`] is vanilla's
/// `activeRight` override (`x + getAdvance(bold)`), which ignores the shadow,
/// shear and bold overhang the drawn quad keeps; [`Self::bounds_right`] is
/// `TextRenderable.right()`, the extent `PreparedText.bounds()` accumulates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudGlyphArea {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bounds_right: f32,
    pub bottom: f32,
    /// The glyph is an `EmptyGlyph` (`BakedGlyph.createGlyph` returned null), so
    /// vanilla reports an `EmptyArea` instead of a glyph instance.
    pub empty: bool,
}

impl HudDigitGlyph {
    /// `GlyphInfo.getAdvance(bold)`; retain fractional advances until whole-line measurement.
    pub fn styled_advance_f32(&self, style: HudTextStyle) -> f32 {
        self.advance + if style.bold { self.bold_offset } else { 0.0 }
    }

    /// Glyph cell top edge relative to the pen `y`, mirroring
    /// `GlyphBitmap.getTop()` = `7 - ascent` (== [`Self::baseline_offset`]).
    fn cell_up(&self) -> f32 {
        self.baseline_offset()
    }

    /// Glyph cell bottom edge, mirroring `GlyphBitmap.getBottom()` =
    /// `getTop() + pixelHeight`.
    fn cell_down(&self) -> f32 {
        self.baseline_offset() + self.rendered_height()
    }

    /// Italic shear applied to the cell top edge:
    /// `BakedSheetGlyph.shearTop()` = `1 - 0.25 * up`.
    fn shear_top(&self) -> f32 {
        1.0 - 0.25 * self.cell_up()
    }

    /// Italic shear applied to the cell bottom edge:
    /// `BakedSheetGlyph.shearBottom()` = `1 - 0.25 * down`.
    fn shear_bottom(&self) -> f32 {
        1.0 - 0.25 * self.cell_down()
    }

    /// One glyph quad in font-pixel space, mirroring `BakedSheetGlyph.render`.
    /// `x`/`y` are the pass origin (already shifted for shadow/bold), `italic`
    /// applies the shear, and `bold` applies `extraThickness`.
    fn render_corners(&self, x: f32, y: f32, italic: bool, bold: bool) -> [[f32; 2]; 4] {
        // getLeft() == bearingLeft; getRight() == left + pixelWidth / oversample.
        let x0 = x + self.bearing_left;
        let x1 = x0 + self.rendered_width();
        let y0 = y + self.cell_up();
        let y1 = y + self.cell_down();
        let shear_y0 = if italic { self.shear_top() } else { 0.0 };
        let shear_y1 = if italic { self.shear_bottom() } else { 0.0 };
        let thickness = if bold {
            HUD_FONT_BOLD_EXTRA_THICKNESS
        } else {
            0.0
        };
        [
            [x0 + shear_y0 - thickness, y0 - thickness],
            [x0 + shear_y1 - thickness, y1 + thickness],
            [x1 + shear_y1 + thickness, y1 + thickness],
            [x1 + shear_y0 + thickness, y0 - thickness],
        ]
    }

    /// Ordered glyph quads for a styled glyph at pen (`x`, `y`), mirroring
    /// `BakedSheetGlyph.renderChar`: the shadow pass(es) first (drawn in the
    /// shadow colour at `+shadowOffset`), then the main pass(es). Bold doubles
    /// each pass into a second quad shifted right by `getBoldOffset()`, and the
    /// shadow's first pass carries the bold thickness too (matching vanilla,
    /// which passes the `bold` flag into that `render` call).
    pub fn styled_quads(
        &self,
        x: f32,
        y: f32,
        style: HudTextStyle,
        shadow: bool,
    ) -> Vec<HudGlyphQuad> {
        let mut quads = Vec::new();
        if self.is_empty {
            return quads;
        }
        let HudTextStyle { bold, italic, .. } = style;
        if shadow {
            quads.push(HudGlyphQuad {
                corners: self.render_corners(
                    x + self.shadow_offset,
                    y + self.shadow_offset,
                    italic,
                    bold,
                ),
                uv: self.uv,
                shadow: true,
            });
            if bold {
                quads.push(HudGlyphQuad {
                    corners: self.render_corners(
                        x + self.bold_offset + self.shadow_offset,
                        y + self.shadow_offset,
                        italic,
                        true,
                    ),
                    uv: self.uv,
                    shadow: true,
                });
            }
        }
        quads.push(HudGlyphQuad {
            corners: self.render_corners(x, y, italic, bold),
            uv: self.uv,
            shadow: false,
        });
        if bold {
            quads.push(HudGlyphQuad {
                corners: self.render_corners(x + self.bold_offset, y, italic, true),
                uv: self.uv,
                shadow: false,
            });
        }
        quads
    }

    /// One glyph's clickable area at pen (`x`, `y`), mirroring
    /// `BakedSheetGlyph.GlyphInstance.left/top/right/bottom` and its
    /// `activeRight` override, or `EmptyArea` when the glyph has no bitmap.
    /// `shadow` is whether the glyph is drawn with a shadow pass: vanilla's
    /// click collector always prepares text with `dropShadow = true`
    /// (`ActiveTextCollector.java:159-161`), so the shadow offset extends the
    /// accepted box even on surfaces whose text is drawn without a shadow.
    pub fn styled_glyph_area(
        &self,
        x: f32,
        y: f32,
        style: HudTextStyle,
        shadow: bool,
    ) -> HudGlyphArea {
        let advance = self.styled_advance_f32(style);
        if self.is_empty {
            // `EmptyArea(x, y, advance, 7.0F, 9.0F)`: activeTop is the pen, the
            // box is nine font pixels tall and takes no shadow or shear.
            return HudGlyphArea {
                left: x,
                top: y,
                right: x + advance,
                bounds_right: x + advance,
                bottom: y + 9.0,
                empty: true,
            };
        }
        let thickness = if style.bold {
            HUD_FONT_BOLD_EXTRA_THICKNESS
        } else {
            0.0
        };
        let (shear_left, shear_right) = if style.italic {
            let top = self.shear_top();
            let bottom = self.shear_bottom();
            (top.min(bottom), top.max(bottom))
        } else {
            (0.0, 0.0)
        };
        let shadow_offset = if shadow { self.shadow_offset } else { 0.0 };
        HudGlyphArea {
            left: x + self.bearing_left + shear_left - thickness,
            top: y + self.cell_up() - thickness,
            right: x + advance,
            bounds_right: x
                + self.bearing_left
                + self.rendered_width()
                + shadow_offset
                + shear_right
                + thickness,
            bottom: y + self.cell_down() + shadow_offset + thickness,
            empty: false,
        }
    }

    /// Underline / strikethrough bars for a styled glyph, mirroring
    /// `Font.StringRenderOutput.accept`: strikethrough spans `y+3.5..y+4.5`,
    /// underline spans `y+8.0..y+9.0`, both from `effectX0` to `x + advance`
    /// where advance is bold-aware. `subsequence_start` is vanilla's
    /// `position == 0` for this glyph — the first glyph of its own
    /// `FormattedCharSequence` part — and extends the bar one pixel left
    /// (`Font.java:358`). Empty when the style sets neither.
    pub fn styled_effect_rects(
        &self,
        x: f32,
        y: f32,
        style: HudTextStyle,
        subsequence_start: bool,
    ) -> Vec<HudEffectRect> {
        let mut rects = Vec::new();
        let advance = self.styled_advance_f32(style);
        let effect_x0 = if subsequence_start { x - 1.0 } else { x };
        let x1 = x + advance;
        if style.strikethrough {
            rects.push(HudEffectRect {
                kind: HudEffectKind::Strikethrough,
                x0: effect_x0,
                y0: y + 4.5 - 1.0,
                x1,
                y1: y + 4.5,
            });
        }
        if style.underlined {
            rects.push(HudEffectRect {
                kind: HudEffectKind::Underline,
                x0: effect_x0,
                y0: y + 9.0 - 1.0,
                x1,
                y1: y + 9.0,
            });
        }
        rects
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphMetricProfile {
    height: u32,
    bearing_top_bits: u32,
    bearing_left_bits: u32,
    render_scale_bits: u32,
    bold_offset_bits: u32,
    shadow_offset_bits: u32,
    is_empty: bool,
}

impl GlyphMetricProfile {
    #[inline]
    fn from_glyph(glyph: HudAsciiGlyph) -> Self {
        Self {
            height: glyph.height,
            bearing_top_bits: glyph.bearing_top.to_bits(),
            bearing_left_bits: glyph.bearing_left.to_bits(),
            render_scale_bits: glyph.render_scale.to_bits(),
            bold_offset_bits: glyph.bold_offset.to_bits(),
            shadow_offset_bits: glyph.shadow_offset.to_bits(),
            is_empty: glyph.is_empty,
        }
    }
}

/// Pending rectangular RGBA8 upload produced by a lazy font atlas.
#[derive(Debug, Clone)]
pub struct HudDynamicGlyphUpload {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Full snapshot of a dynamic font atlas used on initial upload, resize, or debug dump.
#[derive(Debug, Clone)]
pub struct HudDynamicAtlasSnapshot {
    pub width: u32,
    pub height: u32,
    pub static_width: u32,
    pub static_height: u32,
    pub static_rgba: Arc<[u8]>,
    pub glyphs: Vec<HudDynamicGlyphUpload>,
    pub layout_generation: u64,
    pub upload_revision: u64,
}

/// Result of polling a dynamic font atlas for GPU updates since the last synced revisions.
#[derive(Debug)]
pub enum HudDynamicAtlasUpdate {
    Unchanged,
    Uploads {
        upload_revision: u64,
        uploads: Vec<HudDynamicGlyphUpload>,
    },
    Resized(HudDynamicAtlasSnapshot),
}

/// On-demand glyph rasterizer backing lazy font providers (Unihex / TrueType).
pub trait HudDynamicFontAtlas: std::fmt::Debug + Send + Sync {
    fn atlas_dimensions(&self) -> (u32, u32);
    fn static_uv_scale(&self) -> [f32; 2];
    fn layout_generation(&self) -> u64;
    fn upload_revision(&self) -> u64;
    fn ensure_glyph_uv(&self, lazy_id: u32) -> HudUvRect;
    fn snapshot(&self) -> HudDynamicAtlasSnapshot;
    fn poll_updates(&self, synced_layout_gen: u64, synced_upload_rev: u64)
        -> HudDynamicAtlasUpdate;
    fn pixel_rgba(&self, x: u32, y: u32) -> [u8; 4];
}

#[inline]
fn scale_uv_rect(uv: HudUvRect, scale: [f32; 2]) -> HudUvRect {
    if scale == [1.0, 1.0] {
        return uv;
    }
    HudUvRect {
        min: [uv.min[0] * scale[0], uv.min[1] * scale[1]],
        max: [uv.max[0] * scale[0], uv.max[1] * scale[1]],
    }
}

#[derive(Debug, Clone, Copy)]
struct CompactGlyphEntry {
    codepoint: u32,
    uv: HudUvRect,
    advance: f32,
    width: u32,
    profile_index: u32,
    /// `0` for static bitmap/empty glyphs; `n > 0` addresses lazy glyph `n - 1`
    /// inside `HudFontGlyphTable::dynamic_atlas`.
    lazy_key: u32,
}

#[derive(Debug, Clone, Default)]
struct HudFontGlyphTable {
    entries: Vec<CompactGlyphEntry>,
    profiles: Vec<GlyphMetricProfile>,
    profile_index: FxHashMap<GlyphMetricProfile, u32>,
    obfuscation_advances: Vec<(u32, f32)>,
    dynamic_atlas: Option<Arc<dyn HudDynamicFontAtlas>>,
}

impl HudFontGlyphTable {
    #[inline]
    fn intern_profile(&mut self, profile: GlyphMetricProfile) -> u32 {
        if self.profile_index.is_empty() && !self.profiles.is_empty() {
            self.profile_index.reserve(self.profiles.len());
            for (index, &existing) in self.profiles.iter().enumerate() {
                self.profile_index.insert(existing, index as u32);
            }
        }
        if let Some(&index) = self.profile_index.get(&profile) {
            return index;
        }
        let index = self.profiles.len() as u32;
        self.profiles.push(profile);
        self.profile_index.insert(profile, index);
        index
    }

    #[inline]
    fn decode_metrics_only(&self, entry: CompactGlyphEntry) -> HudAsciiGlyph {
        let profile = self.profiles[entry.profile_index as usize];
        HudAsciiGlyph {
            is_empty: profile.is_empty,
            uv: entry.uv,
            width: entry.width,
            height: profile.height,
            advance: entry.advance,
            bearing_top: f32::from_bits(profile.bearing_top_bits),
            bearing_left: f32::from_bits(profile.bearing_left_bits),
            render_scale: f32::from_bits(profile.render_scale_bits),
            bold_offset: f32::from_bits(profile.bold_offset_bits),
            shadow_offset: f32::from_bits(profile.shadow_offset_bits),
        }
    }

    #[inline]
    fn decode_entry(&self, entry: CompactGlyphEntry) -> HudAsciiGlyph {
        let mut glyph = self.decode_metrics_only(entry);
        if let Some(dynamic) = &self.dynamic_atlas {
            if entry.lazy_key != 0 {
                glyph.uv = dynamic.ensure_glyph_uv(entry.lazy_key - 1);
            } else if !glyph.is_empty {
                glyph.uv = scale_uv_rect(glyph.uv, dynamic.static_uv_scale());
            }
        }
        glyph
    }
}

fn insert_obfuscation_advance(advances: &mut Vec<(u32, f32)>, codepoint: u32, advance: f32) {
    if advances.last().is_none_or(|&(last, _)| last < codepoint) {
        advances.push((codepoint, advance));
        return;
    }
    let index = advances
        .binary_search_by_key(&codepoint, |(key, _)| *key)
        .unwrap_err();
    advances.insert(index, (codepoint, advance));
}

fn empty_glyph_table() -> Arc<HudFontGlyphTable> {
    static EMPTY: OnceLock<Arc<HudFontGlyphTable>> = OnceLock::new();
    Arc::clone(EMPTY.get_or_init(|| Arc::new(HudFontGlyphTable::default())))
}

/// Codepoint-keyed glyph table resolved from the vanilla `font/default.json`
/// bitmap provider chain. Lookup priority mirrors vanilla
/// `FontSet.computeGlyphInfo`: providers are walked in flattened `providers`
/// order and the first one supplying a codepoint wins, so inserts are
/// first-wins.
#[derive(Debug, Clone)]
pub struct HudFontGlyphMap {
    /// Sorted by codepoint for binary-search lookup; each codepoint appears
    /// at most once (the first inserted provider glyph).
    table: Arc<HudFontGlyphTable>,
    non_fishy: Option<Box<HudFontGlyphMap>>,
    missing_glyph: Option<HudAsciiGlyph>,
    /// Selected-language paragraph direction default. Logical measurement/edit
    /// helpers intentionally ignore this; render-time completed lines query it
    /// before applying visual order.
    default_right_to_left: bool,
}

impl PartialEq for HudFontGlyphMap {
    fn eq(&self, other: &Self) -> bool {
        fn dynamic_atlas_eq(
            left: &Option<Arc<dyn HudDynamicFontAtlas>>,
            right: &Option<Arc<dyn HudDynamicFontAtlas>>,
        ) -> bool {
            match (left, right) {
                (None, None) => true,
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                _ => false,
            }
        }
        fn table_eq(left: &HudFontGlyphMap, right: &HudFontGlyphMap) -> bool {
            left.missing_glyph == right.missing_glyph
                && left.default_right_to_left == right.default_right_to_left
                && dynamic_atlas_eq(&left.table.dynamic_atlas, &right.table.dynamic_atlas)
                && (Arc::ptr_eq(&left.table, &right.table)
                    || (left.len() == right.len()
                        && left.table.obfuscation_advances == right.table.obfuscation_advances
                        && left
                            .table
                            .entries
                            .iter()
                            .zip(right.table.entries.iter())
                            .all(|(&a, &b)| {
                                a.codepoint == b.codepoint
                                    && a.lazy_key == b.lazy_key
                                    && left.table.decode_metrics_only(a)
                                        == right.table.decode_metrics_only(b)
                            })))
        }
        table_eq(self, other)
            && match (&self.non_fishy, &other.non_fishy) {
                (None, None) => true,
                _ => table_eq(self.non_fishy(), other.non_fishy()),
            }
    }
}

impl Default for HudFontGlyphMap {
    fn default() -> Self {
        Self {
            table: empty_glyph_table(),
            non_fishy: None,
            missing_glyph: None,
            default_right_to_left: false,
        }
    }
}

/// Font maps and lazily built obfuscation pools sharing one resource-generation atlas.
#[derive(Debug, Clone, Default)]
pub struct HudFontGlyphBook {
    /// Changes whenever the glyphs change, so text layouts can be cached per book.
    generation: u64,
    default: HudFontGlyphMap,
    fonts: BTreeMap<String, HudFontGlyphMap>,
    default_obfuscated: Arc<OnceLock<HudObfuscatedGlyphPool>>,
    missing: HudFontGlyphMap,
    missing_obfuscated: Arc<OnceLock<HudObfuscatedGlyphPool>>,
    obfuscated: BTreeMap<String, Arc<OnceLock<HudObfuscatedGlyphPool>>>,
}

impl PartialEq for HudFontGlyphBook {
    fn eq(&self, other: &Self) -> bool {
        self.default == other.default && self.fonts == other.fonts && self.missing == other.missing
    }
}

fn next_font_book_generation() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl HudFontGlyphBook {
    pub fn from_fonts(default_font_id: &str, fonts: BTreeMap<String, HudFontGlyphMap>) -> Self {
        let default = fonts.get(default_font_id).cloned().unwrap_or_default();
        let obfuscated: BTreeMap<String, Arc<OnceLock<HudObfuscatedGlyphPool>>> = fonts
            .keys()
            .map(|font_id| (font_id.clone(), Arc::new(OnceLock::new())))
            .collect();
        let default_obfuscated = obfuscated
            .get(default_font_id)
            .cloned()
            .unwrap_or_else(|| Arc::new(OnceLock::new()));
        let missing = if let Some(glyph) = default.raw_missing_glyph() {
            let mut map = HudFontGlyphMap::new();
            map.set_missing_glyph(glyph);
            if let Some(dynamic) = default.dynamic_atlas() {
                map.set_dynamic_atlas(Arc::clone(dynamic));
            }
            map
        } else {
            default.clone()
        };
        let missing_obfuscated = Arc::new(OnceLock::new());
        Self {
            generation: next_font_book_generation(),
            default,
            fonts,
            default_obfuscated,
            missing,
            missing_obfuscated,
            obfuscated,
        }
    }

    pub fn glyphs(&self, font_id: Option<&str>) -> &HudFontGlyphMap {
        match font_id {
            None => &self.default,
            Some(id) => self.fonts.get(id).unwrap_or(&self.missing),
        }
    }

    pub fn obfuscated_pool(&self, font_id: Option<&str>) -> &HudObfuscatedGlyphPool {
        match font_id {
            None => self
                .default_obfuscated
                .get_or_init(|| HudObfuscatedGlyphPool::from_glyph_map(&self.default)),
            Some(id) => match (self.fonts.get(id), self.obfuscated.get(id)) {
                (Some(glyphs), Some(cell)) => {
                    cell.get_or_init(|| HudObfuscatedGlyphPool::from_glyph_map(glyphs))
                }
                _ => self
                    .missing_obfuscated
                    .get_or_init(|| HudObfuscatedGlyphPool::from_glyph_map(&self.missing)),
            },
        }
    }

    pub fn non_fishy_glyphs(&self, font_id: Option<&str>) -> &HudFontGlyphMap {
        self.glyphs(font_id).non_fishy()
    }

    pub fn contains_font(&self, font_id: &str) -> bool {
        self.fonts.contains_key(font_id)
    }

    pub fn font_count(&self) -> usize {
        self.fonts.len()
    }

    pub fn default_right_to_left(&self) -> bool {
        self.default.default_right_to_left()
    }

    pub fn generation(&self) -> u64 {
        let layout_gen = self
            .dynamic_atlas()
            .map_or(0, |atlas| atlas.layout_generation());
        self.generation
            .wrapping_add(layout_gen.wrapping_mul(0x9e37_79b9_7f4a_7c15))
    }

    pub fn dynamic_atlas(&self) -> Option<&Arc<dyn HudDynamicFontAtlas>> {
        self.default.dynamic_atlas()
    }

    pub fn set_default_right_to_left(&mut self, default_right_to_left: bool) {
        self.generation = next_font_book_generation();
        self.default
            .set_default_right_to_left(default_right_to_left);
        self.missing
            .set_default_right_to_left(default_right_to_left);
        for glyphs in self.fonts.values_mut() {
            glyphs.set_default_right_to_left(default_right_to_left);
        }
    }
}

impl From<HudFontGlyphMap> for HudFontGlyphBook {
    fn from(glyphs: HudFontGlyphMap) -> Self {
        Self::from_fonts(
            "minecraft:default",
            BTreeMap::from([("minecraft:default".to_string(), glyphs)]),
        )
    }
}

impl HudFontGlyphMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        if capacity == 0 {
            return Self::default();
        }
        Self {
            table: Arc::new(HudFontGlyphTable {
                entries: Vec::with_capacity(capacity),
                profiles: Vec::with_capacity(8),
                profile_index: FxHashMap::default(),
                obfuscation_advances: Vec::new(),
                dynamic_atlas: None,
            }),
            ..Self::default()
        }
    }

    pub fn set_dynamic_atlas(&mut self, atlas: Arc<dyn HudDynamicFontAtlas>) {
        Arc::make_mut(&mut self.table).dynamic_atlas = Some(Arc::clone(&atlas));
        if let Some(filtered) = &mut self.non_fishy {
            filtered.set_dynamic_atlas(atlas);
        }
    }

    pub fn dynamic_atlas(&self) -> Option<&Arc<dyn HudDynamicFontAtlas>> {
        self.table.dynamic_atlas.as_ref()
    }

    pub fn shrink_to_fit(&mut self) {
        if self.table.entries.capacity() > self.table.entries.len()
            || self.table.profiles.capacity() > self.table.profiles.len()
            || self.table.profile_index.capacity() > 0
            || self.table.obfuscation_advances.capacity() > self.table.obfuscation_advances.len()
        {
            let table = Arc::make_mut(&mut self.table);
            table.entries.shrink_to_fit();
            table.profiles.shrink_to_fit();
            // Drop the build-time dedup index after loading; `intern_profile` rebuilds it if mutated again.
            table.profile_index.clear();
            table.profile_index.shrink_to_fit();
            table.obfuscation_advances.shrink_to_fit();
        }
        if let Some(filtered) = &mut self.non_fishy {
            filtered.shrink_to_fit();
        }
    }

    /// Inserts a glyph unless the codepoint is already mapped, returning
    /// whether the glyph was inserted. First-wins matches the vanilla
    /// provider order (`FontSet.computeGlyphInfo`).
    pub fn insert_first_wins(&mut self, codepoint: char, glyph: HudAsciiGlyph) -> bool {
        self.insert_codepoint_first_wins(u32::from(codepoint), glyph)
    }

    // Surrogate catalog entries can be selected by obfuscation without a Rust char.
    pub fn insert_codepoint_first_wins(&mut self, codepoint: u32, glyph: HudAsciiGlyph) -> bool {
        if !self.insert_entry(codepoint, glyph, 0) {
            return false;
        }
        if let Some(filtered) = &mut self.non_fishy {
            filtered.insert_entry(codepoint, glyph, 0);
        }
        true
    }

    fn insert_entry(&mut self, codepoint: u32, glyph: HudAsciiGlyph, lazy_key: u32) -> bool {
        if codepoint > 0x10ffff {
            return false;
        }
        let index = if self
            .table
            .entries
            .last()
            .is_some_and(|last| last.codepoint < codepoint)
        {
            self.table.entries.len()
        } else {
            match self
                .table
                .entries
                .binary_search_by_key(&codepoint, |existing| existing.codepoint)
            {
                Ok(_) => return false,
                Err(index) => index,
            }
        };
        let table = Arc::make_mut(&mut self.table);
        let profile_index = table.intern_profile(GlyphMetricProfile::from_glyph(glyph));
        let entry = CompactGlyphEntry {
            codepoint,
            uv: glyph.uv,
            advance: glyph.advance,
            width: glyph.width,
            profile_index,
            lazy_key,
        };
        if index == table.entries.len() {
            table.entries.push(entry);
        } else {
            table.entries.insert(index, entry);
        }
        true
    }

    fn contains_codepoint(&self, codepoint: u32) -> bool {
        if self
            .table
            .entries
            .last()
            .is_some_and(|last| last.codepoint < codepoint)
        {
            return false;
        }
        self.table
            .entries
            .binary_search_by_key(&codepoint, |existing| existing.codepoint)
            .is_ok()
    }

    pub fn get(&self, codepoint: char) -> Option<HudAsciiGlyph> {
        self.get_codepoint(u32::from(codepoint))
    }

    fn get_codepoint(&self, codepoint: u32) -> Option<HudAsciiGlyph> {
        self.table
            .entries
            .binary_search_by_key(&codepoint, |existing| existing.codepoint)
            .ok()
            .map(|index| self.table.decode_entry(self.table.entries[index]))
    }

    fn get_metrics_codepoint(&self, codepoint: u32) -> Option<HudAsciiGlyph> {
        self.table
            .entries
            .binary_search_by_key(&codepoint, |existing| existing.codepoint)
            .ok()
            .map(|index| self.table.decode_metrics_only(self.table.entries[index]))
    }

    pub fn set_missing_glyph(&mut self, glyph: HudAsciiGlyph) {
        self.missing_glyph = Some(glyph);
        if let Some(filtered) = &mut self.non_fishy {
            filtered.set_missing_glyph(glyph);
        }
    }

    fn raw_missing_glyph(&self) -> Option<HudAsciiGlyph> {
        self.missing_glyph
    }

    // Selection and bucket width precede missing-glyph substitution in FontSet.
    pub fn insert_selected_codepoint_first_wins(
        &mut self,
        codepoint: u32,
        any: HudAsciiGlyph,
        non_fishy: HudAsciiGlyph,
        unbaked_advance: f32,
    ) -> bool {
        self.insert_selected_codepoint_with_lazy_keys(
            codepoint,
            any,
            0,
            non_fishy,
            0,
            unbaked_advance,
        )
    }

    pub fn insert_selected_codepoint_with_lazy_keys(
        &mut self,
        codepoint: u32,
        any: HudAsciiGlyph,
        any_lazy_key: u32,
        non_fishy: HudAsciiGlyph,
        non_fishy_lazy_key: u32,
        unbaked_advance: f32,
    ) -> bool {
        if codepoint > 0x10ffff || self.contains_codepoint(codepoint) {
            return false;
        }
        if self.non_fishy.is_none() && (any != non_fishy || any_lazy_key != non_fishy_lazy_key) {
            self.non_fishy = Some(Box::new(self.clone()));
        }
        self.insert_entry(codepoint, any, any_lazy_key);
        if unbaked_advance.to_bits() != any.advance.to_bits() {
            let table = Arc::make_mut(&mut self.table);
            insert_obfuscation_advance(&mut table.obfuscation_advances, codepoint, unbaked_advance);
        }
        if let Some(filtered) = &mut self.non_fishy {
            filtered.insert_entry(codepoint, non_fishy, non_fishy_lazy_key);
            if unbaked_advance.to_bits() != non_fishy.advance.to_bits() {
                let table = Arc::make_mut(&mut filtered.table);
                insert_obfuscation_advance(
                    &mut table.obfuscation_advances,
                    codepoint,
                    unbaked_advance,
                );
            }
        }
        true
    }

    pub fn non_fishy(&self) -> &Self {
        self.non_fishy.as_deref().unwrap_or(self)
    }

    pub fn missing_glyph(&self) -> Option<HudAsciiGlyph> {
        let mut glyph = self.missing_glyph?;
        if let Some(dynamic) = &self.table.dynamic_atlas {
            if !glyph.is_empty {
                glyph.uv = scale_uv_rect(glyph.uv, dynamic.static_uv_scale());
            }
        }
        Some(glyph)
    }

    pub fn resolve(&self, codepoint: char) -> HudAsciiGlyph {
        self.get(codepoint)
            .or_else(|| self.missing_glyph())
            .or_else(|| self.get('?'))
            .unwrap_or_else(|| HudAsciiGlyph::empty(0.0))
    }

    pub fn is_available(&self) -> bool {
        self.missing_glyph.is_some() || !self.table.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.table.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.table.entries.is_empty()
    }

    /// Logical bytes retained by this immutable glyph projection.
    pub fn retained_logical_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            .saturating_add(
                self.table
                    .entries
                    .capacity()
                    .saturating_mul(std::mem::size_of::<(u32, HudAsciiGlyph)>()),
            )
            .saturating_add(
                self.table
                    .obfuscation_advances
                    .capacity()
                    .saturating_mul(std::mem::size_of::<(u32, f32)>()),
            )
            .saturating_add(
                self.non_fishy
                    .as_ref()
                    .map_or(0, |map| map.retained_logical_bytes()),
            )
    }

    pub fn default_right_to_left(&self) -> bool {
        self.default_right_to_left
    }

    pub fn set_default_right_to_left(&mut self, default_right_to_left: bool) {
        self.default_right_to_left = default_right_to_left;
        if let Some(filtered) = &mut self.non_fishy {
            filtered.set_default_right_to_left(default_right_to_left);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, HudAsciiGlyph)> + '_ {
        let table = &self.table;
        table
            .entries
            .iter()
            .map(move |&entry| (entry.codepoint, table.decode_entry(entry)))
    }

    /// `Font.java:185-194`: round the complete floating-point sum once.
    pub fn plain_text_width(&self, text: &str) -> i32 {
        self.plain_text_width_f32(text).ceil() as i32
    }

    pub fn plain_text_width_f32(&self, text: &str) -> f32 {
        text.chars()
            .map(|ch| self.styled_char_advance(ch, HudTextStyle::default()))
            .sum()
    }

    /// Measures one character with the active fallback glyph and resolved
    /// style. Native presentation owners use this to perform Vanilla-width
    /// wrapping before publishing immutable renderer values.
    pub fn styled_char_advance(&self, ch: char, style: HudTextStyle) -> f32 {
        self.plain_glyph(ch).styled_advance_f32(style)
    }

    /// Returns the character cursor reached by consuming at most `width`
    /// pixels from `display_start`, or `None` before a font projection loads.
    pub fn plain_text_cursor_for_width_from(
        &self,
        text: &str,
        display_start: usize,
        width: i32,
    ) -> Option<usize> {
        if !self.is_available() {
            return None;
        }
        let text_len = text.chars().count();
        let display_start = display_start.min(text_len);
        let mut remaining = width as f32;
        let mut display_len = 0usize;
        for ch in text.chars().skip(display_start) {
            remaining -= self
                .plain_glyph(ch)
                .styled_advance_f32(HudTextStyle::default());
            if remaining < 0.0 {
                break;
            }
            display_len += 1;
        }
        Some(display_start.saturating_add(display_len))
    }

    /// Returns the first character that keeps `scroll_to` visible in the
    /// supplied width, matching the renderer's text-input layout.
    pub fn plain_text_display_start_for_width(
        &self,
        text: &str,
        scroll_to: usize,
        width: i32,
    ) -> Option<usize> {
        if !self.is_available() {
            return None;
        }
        let scroll_to = scroll_to.min(text.chars().count());
        let prefix_width = text.chars().take(scroll_to).fold(0.0_f32, |used, ch| {
            used + self
                .plain_glyph(ch)
                .styled_advance_f32(HudTextStyle::default())
        });
        if prefix_width <= width as f32 {
            return Some(0);
        }

        let chars = text.chars().take(scroll_to).collect::<Vec<_>>();
        let mut start = chars.len();
        let mut used_width = 0.0_f32;
        while start > 0 {
            let advance = self
                .plain_glyph(chars[start - 1])
                .styled_advance_f32(HudTextStyle::default());
            if (used_width + advance) > width as f32 {
                break;
            }
            used_width += advance;
            start -= 1;
        }
        Some(start)
    }

    fn plain_glyph(&self, codepoint: char) -> HudAsciiGlyph {
        self.get_metrics_codepoint(u32::from(codepoint))
            .or(self.missing_glyph)
            .or_else(|| self.get_metrics_codepoint(u32::from('?')))
            .unwrap_or_else(|| HudAsciiGlyph::empty(0.0))
    }

    // StringSplitter.java:388-405 and StringDecomposer.java:12-40 retain UTF-16 positions.
    pub fn plain_utf16_head_len(&self, text: &[u16], width: i32) -> Option<usize> {
        if !self.is_available() {
            return None;
        }
        let mut remaining = width as f32;
        let mut end = 0;
        for decoded in char::decode_utf16(text.iter().copied()) {
            let ch = decoded.unwrap_or(char::REPLACEMENT_CHARACTER);
            remaining -= self.plain_glyph(ch).advance;
            if remaining < 0.0 || remaining.is_nan() {
                break;
            }
            end += ch.len_utf16();
        }
        Some(end)
    }

    // StringSplitter.java:66-78 accumulates tail width instead of subtracting it.
    pub fn plain_utf16_tail_len(&self, text: &[u16], width: i32) -> Option<usize> {
        if !self.is_available() {
            return None;
        }
        let mut start = text.len();
        let mut used = 0.0_f32;
        while start > 0 {
            let last = text[start - 1];
            let (ch, units) = if (0xdc00..=0xdfff).contains(&last)
                && start >= 2
                && (0xd800..=0xdbff).contains(&text[start - 2])
            {
                let cp = 0x10000 + ((u32::from(text[start - 2]) - 0xd800) << 10) + u32::from(last)
                    - 0xdc00;
                (char::from_u32(cp).unwrap(), 2)
            } else {
                (
                    char::from_u32(u32::from(last)).unwrap_or(char::REPLACEMENT_CHARACTER),
                    1,
                )
            };
            used += self.plain_glyph(ch).advance;
            if used > width as f32 {
                break;
            }
            start -= units;
        }
        Some(text.len() - start)
    }

    // EditBox.java:576-592 can put displayPos inside a surrogate pair.
    pub fn plain_utf16_scroll_to(
        &self,
        text: &[u16],
        display_start: usize,
        target: usize,
        width: i32,
    ) -> Option<usize> {
        let start = display_start.min(text.len());
        let target = target.min(text.len());
        let last = start + self.plain_utf16_head_len(&text[start..], width)?;
        let mut display = start as i128;
        if target == start {
            display -= self.plain_utf16_tail_len(text, width)? as i128;
        }
        if target > last {
            display += (target - last) as i128;
        } else if target as i128 <= display {
            display = target as i128;
        }
        Some(display.clamp(0, text.len() as i128) as usize)
    }
}

/// Vanilla `LegacyRandomSource` (`RandomSource.create()`) 48-bit LCG constants.
/// `Font.random` advances this generator once per obfuscated glyph
/// (`FontSet.getRandomGlyph`). Vanilla seeds it from a wall-clock
/// `RandomSupport.generateUniqueSeed()`; bbb seeds it from the injected clock
/// (`text_obfuscation_seed`) so the same virtual time draws the same glyphs.
const OBFUSCATED_RANDOM_MULTIPLIER: u64 = 25_214_903_917;
const OBFUSCATED_RANDOM_INCREMENT: u64 = 11;
const OBFUSCATED_RANDOM_MASK: u64 = (1_u64 << 48) - 1;

/// Deterministic clone of vanilla `LegacyRandomSource`, driving the per-frame
/// obfuscated (`§k`) glyph substitution. Only `nextInt(bound)` is needed
/// (`FontSet.getRandomGlyph` picks `chars.getInt(random.nextInt(size))`).
#[derive(Debug, Clone)]
pub struct HudObfuscatedRandom {
    seed: u64,
}

impl HudObfuscatedRandom {
    /// Seeds the generator, mirroring `LegacyRandomSource.setSeed`
    /// (`(seed ^ multiplier) & mask`).
    pub fn with_seed(seed: u64) -> Self {
        Self {
            seed: (seed ^ OBFUSCATED_RANDOM_MULTIPLIER) & OBFUSCATED_RANDOM_MASK,
        }
    }

    /// Vanilla `RandomSource.nextInt(bound)` for a positive `bound`: the
    /// power-of-two fast path plus the rejection loop that removes modulo bias.
    pub fn next_int_bound(&mut self, bound: u32) -> u32 {
        debug_assert!(bound > 0, "bound must be positive");
        let bound = bound as i32;
        if (bound & (bound - 1)) == 0 {
            return ((i64::from(bound) * i64::from(self.next_bits(31))) >> 31) as u32;
        }
        loop {
            let sample = self.next_bits(31) as i32;
            let modulo = sample % bound;
            if sample.wrapping_sub(modulo).wrapping_add(bound - 1) >= 0 {
                return modulo as u32;
            }
        }
    }

    fn next_bits(&mut self, bits: u32) -> u32 {
        self.seed = self
            .seed
            .wrapping_mul(OBFUSCATED_RANDOM_MULTIPLIER)
            .wrapping_add(OBFUSCATED_RANDOM_INCREMENT)
            & OBFUSCATED_RANDOM_MASK;
        (self.seed >> (48 - bits)) as u32
    }
}

#[derive(Debug, PartialEq, Default)]
struct HudObfuscatedGlyphBuckets {
    /// Sorted by advance for binary-search lookup; each bucket lists every
    /// codepoint with that advance in codepoint order (the table is sorted),
    /// so pool ordering is deterministic without eagerly rasterizing lazy glyphs.
    buckets: Vec<(i32, Vec<u32>)>,
}

/// Obfuscated glyph buckets keyed by `ceil(advance)`; substitutions can
/// have different fractional advances within one bucket.
#[derive(Debug, Clone, Default)]
pub struct HudObfuscatedGlyphPool {
    map: Option<HudFontGlyphMap>,
    built: Arc<OnceLock<HudObfuscatedGlyphBuckets>>,
}

impl PartialEq for HudObfuscatedGlyphPool {
    fn eq(&self, other: &Self) -> bool {
        self.buckets() == other.buckets() && self.map == other.map
    }
}

impl HudObfuscatedGlyphPool {
    pub fn from_glyph_map(map: &HudFontGlyphMap) -> Self {
        Self {
            map: Some(map.clone()),
            built: Arc::new(OnceLock::new()),
        }
    }

    fn buckets(&self) -> &HudObfuscatedGlyphBuckets {
        self.built.get_or_init(|| {
            let Some(map) = &self.map else {
                return HudObfuscatedGlyphBuckets::default();
            };
            let entries = map.table.entries.as_slice();
            if entries.is_empty() {
                return HudObfuscatedGlyphBuckets::default();
            }
            // Vanilla `FontSet.glyphsByWidth` (`FontSet.java:58, 109`) buckets glyphs by
            // `ceil(advance)` in O(1); sorting `(width, codepoint)` once avoids O(N^2)
            // sorted-Vec insertion while preserving deterministic ascending width and
            // codepoint order without hash-collision risk.
            let overrides = map.table.obfuscation_advances.as_slice();
            let mut override_idx = 0usize;
            let mut pairs = Vec::with_capacity(entries.len());
            for entry in entries {
                let codepoint = entry.codepoint;
                while override_idx < overrides.len() && overrides[override_idx].0 < codepoint {
                    override_idx += 1;
                }
                let advance = if overrides
                    .get(override_idx)
                    .is_some_and(|&(key, _)| key == codepoint)
                {
                    let value = overrides[override_idx].1;
                    override_idx += 1;
                    value
                } else {
                    entry.advance
                };
                pairs.push((advance.ceil() as i32, codepoint));
            }
            pairs.sort_unstable();
            let bucket_count = pairs.chunk_by(|a, b| a.0 == b.0).count();
            let mut buckets = Vec::with_capacity(bucket_count);
            for group in pairs.chunk_by(|a, b| a.0 == b.0) {
                let width = group[0].0;
                let mut codepoints = Vec::with_capacity(group.len());
                codepoints.extend(group.iter().map(|&(_, codepoint)| codepoint));
                buckets.push((width, codepoints));
            }
            HudObfuscatedGlyphBuckets { buckets }
        })
    }

    pub fn random_glyph(
        &self,
        advance: f32,
        random: &mut HudObfuscatedRandom,
    ) -> Option<HudAsciiGlyph> {
        let map = self.map.as_ref()?;
        let built = self.buckets();
        let index = match built
            .buckets
            .binary_search_by_key(&(advance.ceil() as i32), |(bucket_advance, _)| {
                *bucket_advance
            }) {
            Ok(index) => index,
            Err(_) => return map.missing_glyph(),
        };
        let bucket = &built.buckets[index].1;
        if bucket.is_empty() {
            return map.missing_glyph();
        }
        let pick = random.next_int_bound(bucket.len() as u32) as usize;
        let codepoint = bucket[pick];
        Some(
            map.non_fishy()
                .get_codepoint(codepoint)
                .or_else(|| map.missing_glyph())
                .or_else(|| map.get_codepoint(codepoint))
                .unwrap_or_else(|| HudAsciiGlyph::empty(0.0)),
        )
    }

    /// Number of distinct advance buckets (test/introspection aid).
    pub fn bucket_count(&self) -> usize {
        self.buckets().buckets.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_box_utf16_viewport_preserves_split_surrogates_and_scroll_history() {
        let mut map = HudFontGlyphMap::new();
        for ch in ['A', 'B', char::REPLACEMENT_CHARACTER] {
            map.insert_first_wins(ch, HudAsciiGlyph::empty(6.0));
        }
        map.insert_first_wins('😀', HudAsciiGlyph::empty(12.0));
        let text = "😀AB".encode_utf16().collect::<Vec<_>>();
        assert_eq!(map.plain_utf16_head_len(&text, 18), Some(3));
        assert_eq!(map.plain_utf16_tail_len(&text, 18), Some(2));
        assert_eq!(map.plain_utf16_scroll_to(&text, 0, 4, 18), Some(1));
        assert_eq!(map.plain_utf16_scroll_to(&text, 1, 4, 18), Some(1));
        assert_eq!(map.plain_utf16_head_len(&text[1..], 18), Some(3));
        assert_eq!(map.plain_utf16_head_len(&text[1..], 0), Some(0));
        assert_eq!(map.plain_utf16_scroll_to(&text, 1, 1, 18), Some(0));
        assert_eq!(map.plain_utf16_head_len(&text[..1], 6), Some(1));
        assert_eq!(map.plain_utf16_tail_len(&text[..1], 6), Some(1));
        let unavailable = HudFontGlyphMap::default();
        assert_eq!(unavailable.plain_utf16_scroll_to(&text, 0, 4, 18), None);
    }

    #[test]
    fn edit_box_utf16_head_and_tail_keep_distinct_nan_comparisons() {
        let mut map = HudFontGlyphMap::new();
        map.insert_first_wins('A', HudAsciiGlyph::empty(f32::NAN));
        assert_eq!(map.plain_utf16_head_len(&[65], 0), Some(0));
        assert_eq!(map.plain_utf16_tail_len(&[65], 0), Some(1));
    }

    #[test]
    fn non_fishy_selection_retains_raw_buckets_clone_and_projection_budget() {
        let mut map = HudFontGlyphMap::new();
        let missing = HudAsciiGlyph {
            advance: 6.0,
            ..Default::default()
        };
        let any = HudAsciiGlyph {
            advance: -3.0,
            ..Default::default()
        };
        let filtered = HudAsciiGlyph {
            advance: 8.25,
            ..Default::default()
        };
        map.set_missing_glyph(missing);
        map.insert_first_wins('Z', HudAsciiGlyph::empty(0.0));
        assert!(map.insert_selected_codepoint_first_wins(u32::from('A'), any, filtered, -3.0));
        assert!(map.insert_selected_codepoint_first_wins(0xd800, missing, filtered, 300.0));
        assert!(!map.insert_selected_codepoint_first_wins(u32::from('A'), missing, missing, 0.0));
        assert!(!map.insert_selected_codepoint_first_wins(0x110000, any, filtered, -3.0));
        map.insert_first_wins('B', HudAsciiGlyph::empty(2.0));
        map.set_default_right_to_left(true);
        let cloned = map.clone();
        assert_eq!(cloned.plain_text_width_f32("A"), -3.0);
        assert_eq!(cloned.non_fishy().plain_text_width_f32("A"), 8.25);
        assert_eq!(cloned.non_fishy().get('B'), Some(HudAsciiGlyph::empty(2.0)));
        assert!(cloned.non_fishy().default_right_to_left());
        assert_eq!(cloned.non_fishy().get('Z'), Some(HudAsciiGlyph::empty(0.0)));
        for projection in [&cloned, cloned.non_fishy()] {
            let pool = HudObfuscatedGlyphPool::from_glyph_map(projection);
            for width in [-3.0, 300.0] {
                assert_eq!(
                    pool.random_glyph(width, &mut HudObfuscatedRandom::with_seed(1)),
                    Some(filtered)
                );
            }
            assert_eq!(
                pool.random_glyph(8.25, &mut HudObfuscatedRandom::with_seed(1)),
                Some(missing)
            );
        }
        assert!(
            cloned.retained_logical_bytes()
                >= std::mem::size_of::<HudFontGlyphMap>()
                    + 4 * std::mem::size_of::<(u32, HudAsciiGlyph)>()
                    + 2 * std::mem::size_of::<(u32, f32)>()
                    + cloned.non_fishy().retained_logical_bytes()
        );
        let book = HudFontGlyphBook::from(cloned);
        assert_eq!(book.non_fishy_glyphs(None).get('A'), Some(filtered));
        assert_eq!(book.glyphs(None).get('A'), Some(any));
        assert_eq!(book.non_fishy_glyphs(Some("absent")).resolve('A'), missing);
    }

    #[test]
    fn surrogate_catalog_is_distinct_from_scalar_text_and_available_to_obfuscation() {
        let glyph = HudAsciiGlyph {
            advance: 6.0,
            width: 10,
            height: 16,
            ..Default::default()
        };
        let missing = HudAsciiGlyph {
            advance: 6.0,
            width: 5,
            height: 8,
            ..Default::default()
        };
        let mut map = HudFontGlyphMap::new();
        map.set_missing_glyph(missing);
        assert!(map.insert_codepoint_first_wins(0xd800, glyph));
        assert!(!map.insert_codepoint_first_wins(0xd800, missing));
        assert!(!map.insert_codepoint_first_wins(0x110000, glyph));
        assert_eq!(map.iter().collect::<Vec<_>>(), [(0xd800, glyph)]);
        for ch in ['A', '\u{fffd}', '\u{10000}'] {
            assert_eq!(map.get(ch), None);
            assert_eq!(map.resolve(ch), missing);
        }
        let pool = HudObfuscatedGlyphPool::from_glyph_map(&map);
        assert_eq!(
            pool.random_glyph(6.0, &mut HudObfuscatedRandom::with_seed(0)),
            Some(glyph)
        );
        assert!(
            map.retained_logical_bytes()
                >= std::mem::size_of::<HudFontGlyphMap>()
                    + std::mem::size_of::<(u32, HudAsciiGlyph)>()
        );
    }

    #[test]
    fn component_shadow_argb_preserves_overrides_and_quantizes_alpha() {
        for (text, explicit, draw, expected) in [
            (0x817f80ff, None, true, 0x811f203f),
            (0x817f80ff, None, false, 0),
            (0xffffffff, Some(0), true, 0),
            (0xffffffff, Some(0x7faabbcc), false, 0x7faabbcc),
            (0x80abcdef, Some(0x7fff4400), false, 0x3fff4400),
            (0x01000000, Some(0xfeabcdef), true, 0x00abcdef),
            (0xfe000000, Some(0xffaabbcc), false, 0xfeaabbcc),
            (0x00ffffff, Some(0x80aabbcc), false, 0x00aabbcc),
        ] {
            assert_eq!(
                HudStyledTextRun::resolve_shadow_argb(text, explicit, draw),
                expected
            );
        }
    }

    #[test]
    fn missing_glyph_is_available_without_registered_codepoints() {
        let missing = HudAsciiGlyph {
            advance: 6.0,
            width: 5,
            height: 8,
            ..Default::default()
        };
        let mut map = HudFontGlyphMap::new();
        map.set_missing_glyph(missing);
        assert!(map.is_available());
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
        assert!(map.get('x').is_none());
        assert_eq!(map.resolve('x'), missing);
        assert_eq!(map.plain_text_width("x"), 6);
        assert_eq!(map.plain_text_cursor_for_width_from("xy", 0, 6), Some(1));
        map.insert_first_wins(
            '?',
            HudAsciiGlyph {
                advance: 3.0,
                ..Default::default()
            },
        );
        assert_eq!(map.resolve('?').advance, 3.0);
        assert_eq!(map.resolve('x'), missing);
        let book = HudFontGlyphBook::from(map);
        assert_eq!(book.glyphs(Some("unknown:font")).resolve('?'), missing);
        let mut random = HudObfuscatedRandom::with_seed(0);
        assert_eq!(
            book.obfuscated_pool(None).random_glyph(100.0, &mut random),
            Some(missing)
        );
    }

    #[test]
    fn fractional_signed_advances_round_only_the_complete_line() {
        let mut map = HudFontGlyphMap::new();
        for (ch, advance) in [('a', 0.5), ('b', 6.0), ('c', -2.25), ('n', -4.0)] {
            map.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    advance,
                    ..Default::default()
                },
            );
        }
        assert_eq!(map.plain_text_width("aa"), 1);
        assert_eq!(map.plain_text_width_f32("bca"), 4.25);
        assert_eq!(map.plain_text_width("bca"), 5);
        assert_eq!(map.plain_text_width("c"), -2);
        assert_eq!(map.plain_text_cursor_for_width_from("bnb", 0, 3), Some(0));
        assert_eq!(map.plain_text_cursor_for_width_from("bnb", 0, 6), Some(2));
        assert_eq!(map.plain_text_cursor_for_width_from("ca", 0, -1), Some(2));
        assert_eq!(map.plain_text_display_start_for_width("caa", 3, 0), Some(0));
    }

    #[test]
    fn obfuscated_buckets_use_signed_ceiled_advance() {
        let mut map = HudFontGlyphMap::new();
        for (ch, advance) in [('a', 4.1), ('b', 4.9), ('c', -2.1), ('d', -2.9)] {
            map.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    advance,
                    ..Default::default()
                },
            );
        }
        let pool = HudObfuscatedGlyphPool::from_glyph_map(&map);
        assert_eq!(pool.bucket_count(), 2);
        let mut random = HudObfuscatedRandom::with_seed(1);
        assert!(matches!(
            pool.random_glyph(4.5, &mut random).unwrap().advance,
            4.1 | 4.9
        ));
        assert!(matches!(
            pool.random_glyph(-2.5, &mut random).unwrap().advance,
            -2.1 | -2.9
        ));
    }

    #[test]
    fn glyph_map_lookup_is_first_wins() {
        let mut map = HudFontGlyphMap::new();
        let first = HudAsciiGlyph {
            advance: 7.0,
            bearing_top: 7.0,
            ..HudAsciiGlyph::default()
        };
        let second = HudAsciiGlyph {
            advance: 9.0,
            bearing_top: 10.0,
            ..HudAsciiGlyph::default()
        };

        assert!(map.insert_first_wins('é', first));
        assert!(!map.insert_first_wins('é', second));

        assert_eq!(map.get('é'), Some(first));
        assert_eq!(map.get('e'), None);
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn glyph_map_retained_bytes_charge_entry_capacity() {
        let mut map = HudFontGlyphMap::new();
        let empty_bytes = map.retained_logical_bytes();
        assert_eq!(empty_bytes, std::mem::size_of::<HudFontGlyphMap>());

        map.insert_first_wins('a', HudAsciiGlyph::default());
        assert_eq!(
            map.retained_logical_bytes(),
            std::mem::size_of::<HudFontGlyphMap>()
                + map.table.entries.capacity() * std::mem::size_of::<(char, HudAsciiGlyph)>()
        );
        assert!(map.retained_logical_bytes() > empty_bytes);
    }

    #[test]
    fn text_direction_flag_does_not_change_logical_measurement_or_edit_cursors() {
        let mut map = HudFontGlyphMap::new();
        for ch in ['1', '2', '3', '-'] {
            map.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    advance: 6.0,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        let logical_width = map.plain_text_width("123-");
        let logical_cursor = map.plain_text_cursor_for_width_from("123-", 0, 12);
        map.set_default_right_to_left(true);

        assert!(map.default_right_to_left());
        assert_eq!(map.plain_text_width("123-"), logical_width);
        assert_eq!(
            map.plain_text_cursor_for_width_from("123-", 0, 12),
            logical_cursor
        );
    }

    #[test]
    fn font_book_selects_custom_maps_and_matching_obfuscation_pools() {
        let glyph = |advance, min_x| HudAsciiGlyph {
            uv: HudUvRect {
                min: [min_x, 0.0],
                max: [min_x + 0.1, 1.0],
            },
            width: advance,
            height: 8,
            advance: advance as f32,
            bearing_top: 7.0,
            ..HudAsciiGlyph::default()
        };
        let mut default = HudFontGlyphMap::new();
        default.insert_first_wins('A', glyph(5, 0.0));
        let mut custom = HudFontGlyphMap::new();
        custom.insert_first_wins('A', glyph(7, 0.5));
        let book = HudFontGlyphBook::from_fonts(
            "minecraft:default",
            BTreeMap::from([
                ("example:custom".to_string(), custom),
                ("minecraft:default".to_string(), default),
            ]),
        );

        assert_eq!(book.font_count(), 2);
        assert!(book.contains_font("example:custom"));
        assert_eq!(
            book.glyphs(Some("example:custom"))
                .get('A')
                .unwrap()
                .advance,
            7.0
        );
        assert_eq!(
            book.glyphs(Some("example:missing"))
                .get('A')
                .unwrap()
                .advance,
            5.0
        );
        assert_eq!(
            book.obfuscated_pool(Some("example:custom")).bucket_count(),
            1
        );
        assert!(book
            .obfuscated_pool(Some("example:custom"))
            .random_glyph(7.0, &mut HudObfuscatedRandom::with_seed(1))
            .is_some());
        assert!(book
            .obfuscated_pool(Some("example:missing"))
            .random_glyph(5.0, &mut HudObfuscatedRandom::with_seed(1))
            .is_some());
    }

    #[test]
    fn immutable_plain_text_projection_matches_loaded_advances_and_scrolling() {
        let mut map = HudFontGlyphMap::new();
        for (ch, advance) in [('i', 2), ('w', 7), ('?', 5)] {
            map.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    advance: advance as f32,
                    width: advance,
                    height: 8,
                    ..HudAsciiGlyph::default()
                },
            );
        }

        assert_eq!(map.plain_text_width("iwx"), 14);
        assert_eq!(map.plain_text_cursor_for_width_from("iwx", 0, 8), Some(1));
        assert_eq!(map.plain_text_cursor_for_width_from("iwx", 1, 12), Some(3));
        assert_eq!(
            map.plain_text_display_start_for_width("iiwww", 5, 14),
            Some(3)
        );

        let empty = HudFontGlyphMap::new();
        assert_eq!(empty.plain_text_width("iwx"), 0);
        assert_eq!(empty.plain_text_cursor_for_width_from("iwx", 0, 8), None);
        assert_eq!(empty.plain_text_display_start_for_width("iwx", 3, 8), None);
    }

    fn styled_glyph() -> HudAsciiGlyph {
        // width 5, height 8, ascent 7 -> cell up 0, cell down 8.
        HudAsciiGlyph {
            uv: HudUvRect {
                min: [0.1, 0.2],
                max: [0.3, 0.4],
            },
            width: 5,
            height: 8,
            advance: 6.0,
            bearing_top: 7.0,
            ..HudAsciiGlyph::default()
        }
    }

    fn assert_close(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-4, "expected {a} ~= {b}");
    }

    #[test]
    fn styled_advance_only_bold_widens_the_pen() {
        let glyph = styled_glyph();
        // Vanilla GlyphInfo.getAdvance(bold) = advance + (bold ? 1 : 0).
        assert_eq!(glyph.styled_advance_f32(HudTextStyle::default()), 6.0);
        assert_eq!(
            glyph.styled_advance_f32(HudTextStyle {
                bold: true,
                ..Default::default()
            }),
            7.0
        );
        // italic / underline / strikethrough / obfuscated leave the advance
        // untouched; obfuscated substitutes an equal-advance glyph.
        for style in [
            HudTextStyle {
                italic: true,
                ..Default::default()
            },
            HudTextStyle {
                underlined: true,
                ..Default::default()
            },
            HudTextStyle {
                strikethrough: true,
                ..Default::default()
            },
            HudTextStyle {
                obfuscated: true,
                ..Default::default()
            },
        ] {
            assert_eq!(glyph.styled_advance_f32(style), 6.0);
        }
    }

    #[test]
    fn unihex_metrics_preserve_two_times_oversampling_and_half_pixel_offsets() {
        let glyph = HudAsciiGlyph {
            width: 16,
            height: 16,
            advance: 9.0,
            render_scale: 0.5,
            bold_offset: 0.5,
            shadow_offset: 0.5,
            ..HudAsciiGlyph::default()
        };
        let bold = HudTextStyle {
            bold: true,
            ..HudTextStyle::default()
        };
        assert_eq!(glyph.rendered_width(), 8.0);
        assert_eq!(glyph.rendered_height(), 8.0);
        assert_eq!(glyph.styled_advance_f32(bold), 9.5);
        assert_eq!((glyph.styled_advance_f32(bold) * 2.0).ceil() as i32, 19);

        let quads = glyph.styled_quads(0.0, 0.0, bold, true);
        assert_eq!(quads.len(), 4);
        assert_close(quads[0].corners[0][0], 0.4);
        assert_close(quads[0].corners[0][1], 0.4);
        assert_close(quads[1].corners[0][0], 0.9);
        assert_close(quads[2].corners[2][0], 8.1);
    }

    #[test]
    fn default_style_no_shadow_is_a_single_plain_quad() {
        let glyph = styled_glyph();
        let quads = glyph.styled_quads(2.0, 3.0, HudTextStyle::default(), false);
        assert_eq!(quads.len(), 1);
        assert!(!quads[0].shadow);
        // No shear, no thickness: an axis-aligned cell [x, x+width] x [y+up, y+down].
        assert_eq!(
            quads[0].corners,
            [[2.0, 3.0], [2.0, 11.0], [7.0, 11.0], [7.0, 3.0]]
        );
        assert_eq!(quads[0].uv, glyph.uv);
    }

    #[test]
    fn shadow_pass_is_drawn_first_offset_by_one_one() {
        let glyph = styled_glyph();
        let quads = glyph.styled_quads(2.0, 3.0, HudTextStyle::default(), true);
        assert_eq!(quads.len(), 2);
        // Shadow first, then the main pass.
        assert!(quads[0].shadow);
        assert!(!quads[1].shadow);
        // Vanilla shadow = the same glyph at (x + shadowOffset, y + shadowOffset).
        for (shadow_corner, main_corner) in quads[0].corners.iter().zip(quads[1].corners.iter()) {
            assert_close(shadow_corner[0], main_corner[0] + HUD_FONT_SHADOW_OFFSET);
            assert_close(shadow_corner[1], main_corner[1] + HUD_FONT_SHADOW_OFFSET);
        }
    }

    #[test]
    fn bold_adds_a_second_quad_shifted_by_the_bold_offset() {
        let glyph = styled_glyph();
        let style = HudTextStyle {
            bold: true,
            ..Default::default()
        };
        let quads = glyph.styled_quads(2.0, 3.0, style, false);
        // Bold, no shadow -> main pass + bold pass.
        assert_eq!(quads.len(), 2);
        assert!(quads.iter().all(|quad| !quad.shadow));
        // Both quads carry the 0.1 extraThickness; the second is shifted right
        // by exactly boldOffset (the thickness cancels in the delta).
        for (bold_corner, main_corner) in quads[1].corners.iter().zip(quads[0].corners.iter()) {
            assert_close(bold_corner[0] - main_corner[0], HUD_FONT_BOLD_OFFSET);
            assert_close(bold_corner[1], main_corner[1]);
        }
        // Thickness expands the main cell outward on every side.
        assert_close(quads[0].corners[0][0], 2.0 - HUD_FONT_BOLD_EXTRA_THICKNESS);
        assert_close(quads[0].corners[0][1], 3.0 - HUD_FONT_BOLD_EXTRA_THICKNESS);
        assert_close(quads[0].corners[2][0], 7.0 + HUD_FONT_BOLD_EXTRA_THICKNESS);
        assert_close(quads[0].corners[2][1], 11.0 + HUD_FONT_BOLD_EXTRA_THICKNESS);
    }

    #[test]
    fn bold_with_shadow_emits_four_passes() {
        let glyph = styled_glyph();
        let style = HudTextStyle {
            bold: true,
            ..Default::default()
        };
        let quads = glyph.styled_quads(2.0, 3.0, style, true);
        // Vanilla renderChar with shadow+bold: shadow, shadow-bold, main, bold.
        assert_eq!(quads.len(), 4);
        assert_eq!(
            quads.iter().map(|quad| quad.shadow).collect::<Vec<_>>(),
            [true, true, false, false]
        );
    }

    #[test]
    fn italic_shears_top_and_bottom_edges() {
        let glyph = styled_glyph();
        let plain = glyph.styled_quads(2.0, 3.0, HudTextStyle::default(), false)[0].corners;
        let italic = glyph.styled_quads(
            2.0,
            3.0,
            HudTextStyle {
                italic: true,
                ..Default::default()
            },
            false,
        )[0]
        .corners;
        // shearTop = 1 - 0.25*up = 1.0 ; shearBottom = 1 - 0.25*down = -1.0.
        let shear_top = 1.0 - 0.25 * 0.0;
        let shear_bottom = 1.0 - 0.25 * 8.0;
        // Corners are [top_left, bottom_left, bottom_right, top_right].
        assert_close(italic[0][0] - plain[0][0], shear_top); // top-left
        assert_close(italic[3][0] - plain[3][0], shear_top); // top-right
        assert_close(italic[1][0] - plain[1][0], shear_bottom); // bottom-left
        assert_close(italic[2][0] - plain[2][0], shear_bottom); // bottom-right
                                                                // Shear is purely horizontal.
        for (sheared, straight) in italic.iter().zip(plain.iter()) {
            assert_close(sheared[1], straight[1]);
        }
    }

    #[test]
    fn effect_rects_match_vanilla_y_ranges_and_span() {
        let glyph = styled_glyph();
        let style = HudTextStyle {
            underlined: true,
            strikethrough: true,
            ..Default::default()
        };
        // Mid-line glyph: effectX0 == x, span to x + advance.
        let rects = glyph.styled_effect_rects(2.0, 3.0, style, false);
        assert_eq!(rects.len(), 2);
        // Strikethrough emitted before underline (vanilla accept order).
        assert_eq!(rects[0].kind, HudEffectKind::Strikethrough);
        assert_eq!(rects[1].kind, HudEffectKind::Underline);
        // Strikethrough bar: y in [3.5, 4.5] relative to y=3.
        assert_close(rects[0].y0, 3.0 + 3.5);
        assert_close(rects[0].y1, 3.0 + 4.5);
        // Underline bar: y in [8.0, 9.0].
        assert_close(rects[1].y0, 3.0 + 8.0);
        assert_close(rects[1].y1, 3.0 + 9.0);
        for rect in &rects {
            assert_close(rect.x0, 2.0);
            assert_close(rect.x1, 2.0 + 6.0); // x + advance (no bold)
        }
    }

    #[test]
    fn effect_subsequence_start_extends_left_and_bold_widens_span() {
        let glyph = styled_glyph();
        let style = HudTextStyle {
            underlined: true,
            bold: true,
            ..Default::default()
        };
        let rects = glyph.styled_effect_rects(4.0, 0.0, style, true);
        assert_eq!(rects.len(), 1);
        // Vanilla `position == 0` of this glyph's own part: effectX0 = x - 1.
        assert_close(rects[0].x0, 4.0 - 1.0);
        // Bold-aware advance widens the bar: x + (advance + boldOffset).
        assert_close(rects[0].x1, 4.0 + 7.0);
    }

    #[test]
    fn baseline_offset_follows_vanilla_glyph_bitmap_top() {
        // GlyphBitmap.getTop(): 7.0F - ascent.
        let ascii = HudAsciiGlyph {
            bearing_top: 7.0,
            ..HudAsciiGlyph::default()
        };
        let accented = HudAsciiGlyph {
            bearing_top: 10.0,
            ..HudAsciiGlyph::default()
        };

        assert_eq!(ascii.baseline_offset(), 0.0);
        assert_eq!(accented.baseline_offset(), -3.0);
        assert_eq!(HudAsciiGlyph::default().baseline_offset(), 0.0);
    }

    fn advance_glyph(advance: u32, uv_min: f32) -> HudAsciiGlyph {
        // Distinct uv per glyph so a substitution is observable; width/height
        // are non-zero so the drawn quad survives the width>0 gate.
        HudAsciiGlyph {
            uv: HudUvRect {
                min: [uv_min, 0.0],
                max: [uv_min + 0.1, 0.1],
            },
            width: 5,
            height: 8,
            advance: advance as f32,
            bearing_top: 7.0,
            ..HudAsciiGlyph::default()
        }
    }

    #[test]
    fn obfuscated_random_is_deterministic_and_seed_sensitive() {
        // Same seed -> same stream; the LCG never touches wall-clock state.
        let mut a = HudObfuscatedRandom::with_seed(7);
        let mut b = HudObfuscatedRandom::with_seed(7);
        let seq_a: Vec<u32> = (0..8).map(|_| a.next_int_bound(10)).collect();
        let seq_b: Vec<u32> = (0..8).map(|_| b.next_int_bound(10)).collect();
        assert_eq!(seq_a, seq_b);
        assert!(seq_a.iter().all(|value| *value < 10));
        // A different seed diverges.
        let mut c = HudObfuscatedRandom::with_seed(8);
        let seq_c: Vec<u32> = (0..8).map(|_| c.next_int_bound(10)).collect();
        assert_ne!(seq_a, seq_c);
    }

    #[test]
    fn obfuscated_pool_groups_by_advance_and_picks_equal_advance() {
        let mut map = HudFontGlyphMap::new();
        // Two advance-6 glyphs and one advance-4 glyph.
        map.insert_first_wins('a', advance_glyph(6, 0.1));
        map.insert_first_wins('b', advance_glyph(6, 0.2));
        map.insert_first_wins('c', advance_glyph(4, 0.3));
        let pool = HudObfuscatedGlyphPool::from_glyph_map(&map);
        assert_eq!(pool.bucket_count(), 2);

        // Every draw from the advance-6 bucket keeps advance 6 and lands on one
        // of the two pooled uvs.
        let mut random = HudObfuscatedRandom::with_seed(1);
        for _ in 0..32 {
            let glyph = pool
                .random_glyph(6.0, &mut random)
                .expect("advance-6 bucket");
            assert_eq!(glyph.advance, 6.0);
            assert!(glyph.uv.min[0] == 0.1 || glyph.uv.min[0] == 0.2);
        }
        // The advance-4 bucket only yields its single member.
        let mut random = HudObfuscatedRandom::with_seed(1);
        let glyph = pool
            .random_glyph(4.0, &mut random)
            .expect("advance-4 bucket");
        assert_eq!(glyph.advance, 4.0);
        assert_eq!(glyph.uv.min[0], 0.3);
        // No bucket for an unseen advance.
        assert!(pool
            .random_glyph(9.0, &mut HudObfuscatedRandom::with_seed(1))
            .is_none());
        assert!(HudObfuscatedGlyphPool::default()
            .random_glyph(6.0, &mut HudObfuscatedRandom::with_seed(1))
            .is_none());
    }

    #[test]
    fn obfuscated_buckets_and_profile_interning_scale_with_descending_widths() {
        const GLYPH_COUNT: u32 = 8_192;
        let mut map = HudFontGlyphMap::with_capacity(GLYPH_COUNT as usize + 2);
        for index in 0..GLYPH_COUNT {
            // Strictly descending widths (worst case for sorted-Vec insertion) combined with
            // distinct per-glyph metric profiles (worst case for linear profile scans).
            let glyph = HudAsciiGlyph {
                width: 8,
                height: 8 + (index % 17),
                advance: (GLYPH_COUNT - index) as f32,
                bearing_top: 7.0 + (index / 17) as f32 * 0.25,
                bearing_left: (index % 5) as f32 * 0.5,
                ..HudAsciiGlyph::default()
            };
            assert!(map.insert_codepoint_first_wins(index, glyph));
        }
        let initial_profiles = map.table.profiles.len();
        assert_eq!(initial_profiles, GLYPH_COUNT as usize);

        // `shrink_to_fit` drops the build-time profile index; subsequent inserts lazily rebuild it
        // and continue deduplicating existing profiles without losing accuracy.
        map.shrink_to_fit();
        assert_eq!(map.table.profile_index.capacity(), 0);
        let duplicate_profile_glyph = HudAsciiGlyph {
            width: 8,
            height: 8,
            advance: 1.0,
            bearing_top: 7.0,
            bearing_left: 0.0,
            ..HudAsciiGlyph::default()
        };
        let fresh_profile_glyph = HudAsciiGlyph {
            width: 8,
            height: 99,
            advance: 1.0,
            bearing_top: 12.5,
            bearing_left: 1.25,
            ..HudAsciiGlyph::default()
        };
        assert!(map.insert_codepoint_first_wins(GLYPH_COUNT, duplicate_profile_glyph));
        assert!(map.insert_codepoint_first_wins(GLYPH_COUNT + 1, fresh_profile_glyph));
        assert_eq!(map.table.profiles.len(), initial_profiles + 1);
        assert_eq!(
            map.get_codepoint(GLYPH_COUNT),
            Some(duplicate_profile_glyph)
        );
        assert_eq!(
            map.get_codepoint(GLYPH_COUNT + 1),
            Some(fresh_profile_glyph)
        );

        let pool = HudObfuscatedGlyphPool::from_glyph_map(&map);
        assert_eq!(pool.bucket_count(), GLYPH_COUNT as usize);

        let buckets = &pool.buckets().buckets;
        assert!(buckets.windows(2).all(|pair| pair[0].0 < pair[1].0));
        // Width 1 contains the last descending glyph (codepoint GLYPH_COUNT - 1) plus the two
        // appended width-1 glyphs in strictly ascending codepoint order.
        assert_eq!(
            buckets[0],
            (1, vec![GLYPH_COUNT - 1, GLYPH_COUNT, GLYPH_COUNT + 1])
        );
        assert_eq!(
            buckets[GLYPH_COUNT as usize - 1],
            (GLYPH_COUNT as i32, vec![0])
        );

        let mut random = HudObfuscatedRandom::with_seed(42);
        assert_eq!(
            pool.random_glyph(GLYPH_COUNT as f32, &mut random),
            map.get_codepoint(0)
        );
        let picked_width_1 = pool.random_glyph(1.0, &mut random).unwrap();
        assert_eq!(picked_width_1.advance, 1.0);
    }
}

#[cfg(test)]
mod empty_glyph_tests {
    use super::*;

    #[test]
    fn empty_glyph_keeps_advance_and_decorations_without_bitmap_quads() {
        let glyph = HudDigitGlyph::empty(-0.5);
        let style = HudTextStyle {
            bold: true,
            underlined: true,
            ..Default::default()
        };
        assert_eq!(glyph.styled_advance_f32(style), 0.5);
        assert!(glyph.styled_quads(1.0, 2.0, style, true).is_empty());
        let effect = glyph.styled_effect_rects(1.0, 2.0, style, false);
        assert_eq!(effect.len(), 1);
        assert_eq!((effect[0].x0, effect[0].x1), (1.0, 1.5));
        assert!(HudFontGlyphMap::new().resolve('X').is_empty);
    }

    #[test]
    fn zero_bitmap_keeps_styled_submission_even_with_nonfinite_geometry() {
        for (width, height, render_scale) in [(0, 8, 1.0), (8, 0, f32::INFINITY), (0, 0, f32::NAN)]
        {
            let glyph = HudDigitGlyph {
                width,
                height,
                render_scale,
                advance: 1.0,
                ..Default::default()
            };
            let quads = glyph.styled_quads(
                0.0,
                0.0,
                HudTextStyle {
                    bold: true,
                    ..Default::default()
                },
                true,
            );
            assert_eq!(quads.len(), 4);
            if height == 0 {
                assert!(quads[0]
                    .corners
                    .iter()
                    .flatten()
                    .any(|coordinate| !coordinate.is_finite()));
            }
        }
    }
}
