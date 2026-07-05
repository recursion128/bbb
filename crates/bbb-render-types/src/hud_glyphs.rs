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
    pub uv: HudUvRect,
    /// Rendered cell width in font pixels (`glyphWidth * pixelScale`; vanilla
    /// draws the full bitmap cell, `GlyphBitmap.getRight`).
    pub width: u32,
    /// Rendered cell height in font pixels (`glyphHeight * pixelScale`, i.e.
    /// the bitmap provider's `height` field, `GlyphBitmap.getBottom`).
    pub height: u32,
    pub advance: u32,
    /// Bitmap provider `ascent` (vanilla `GlyphBitmap.getBearingTop()`):
    /// distance from the reference baseline up to the glyph cell top.
    pub ascent: i32,
}

impl HudDigitGlyph {
    /// Vertical offset of the glyph cell top relative to the text line top,
    /// mirroring vanilla `GlyphBitmap.getTop()` = `7.0F - getBearingTop()`.
    /// Zero for ascent-7 pages, negative (above the line top) for taller
    /// pages such as `font/accented.png` (ascent 10 -> -3).
    pub fn baseline_offset(&self) -> f32 {
        (HUD_FONT_BASELINE - self.ascent) as f32
    }
}

impl Default for HudDigitGlyph {
    fn default() -> Self {
        Self {
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            width: 0,
            height: 0,
            advance: 0,
            ascent: HUD_FONT_BASELINE,
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
/// keys as flattened `StyledTextRun`s, which world/native project into
/// [`HudStyledTextRun`]s consumed by the HUD text draw loops. The `italic`
/// shear and per-tick `obfuscated` glyph substitution are still pending draw
/// primitives; both flags are carried (widths already correct) but drawn as
/// upright/original glyphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HudTextStyle {
    pub bold: bool,
    pub italic: bool,
    pub underlined: bool,
    pub strikethrough: bool,
    pub obfuscated: bool,
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
}

impl HudStyledTextRun {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: HudTextStyle::default(),
            color: None,
        }
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

impl HudDigitGlyph {
    /// Vanilla `GlyphInfo.getAdvance(bold)`: bold widens the pen advance by
    /// `getBoldOffset()` (1) font pixel. Every other style flag leaves the
    /// advance unchanged — obfuscated substitutes an equal-advance glyph, and
    /// italic/underline/strikethrough only affect geometry.
    pub fn styled_advance(&self, style: HudTextStyle) -> u32 {
        self.advance + u32::from(style.bold)
    }

    /// Glyph cell top edge relative to the pen `y`, mirroring
    /// `GlyphBitmap.getTop()` = `7 - ascent` (== [`Self::baseline_offset`]).
    fn cell_up(&self) -> f32 {
        self.baseline_offset()
    }

    /// Glyph cell bottom edge, mirroring `GlyphBitmap.getBottom()` =
    /// `getTop() + pixelHeight`.
    fn cell_down(&self) -> f32 {
        self.baseline_offset() + self.height as f32
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
        let x0 = x; // getLeft() == bearingLeft == 0
        let x1 = x + self.width as f32; // getRight() == left + pixelWidth
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
        let HudTextStyle { bold, italic, .. } = style;
        if shadow {
            quads.push(HudGlyphQuad {
                corners: self.render_corners(
                    x + HUD_FONT_SHADOW_OFFSET,
                    y + HUD_FONT_SHADOW_OFFSET,
                    italic,
                    bold,
                ),
                uv: self.uv,
                shadow: true,
            });
            if bold {
                quads.push(HudGlyphQuad {
                    corners: self.render_corners(
                        x + HUD_FONT_BOLD_OFFSET + HUD_FONT_SHADOW_OFFSET,
                        y + HUD_FONT_SHADOW_OFFSET,
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
                corners: self.render_corners(x + HUD_FONT_BOLD_OFFSET, y, italic, true),
                uv: self.uv,
                shadow: false,
            });
        }
        quads
    }

    /// Underline / strikethrough bars for a styled glyph, mirroring
    /// `Font.StringRenderOutput.accept`: strikethrough spans `y+3.5..y+4.5`,
    /// underline spans `y+8.0..y+9.0`, both from `effectX0` to `x + advance`
    /// where advance is bold-aware. `first_in_line` extends the bar one pixel
    /// left (vanilla `position == 0`). Empty when the style sets neither.
    pub fn styled_effect_rects(
        &self,
        x: f32,
        y: f32,
        style: HudTextStyle,
        first_in_line: bool,
    ) -> Vec<HudEffectRect> {
        let mut rects = Vec::new();
        let advance = self.styled_advance(style) as f32;
        let effect_x0 = if first_in_line { x - 1.0 } else { x };
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

/// Codepoint-keyed glyph table resolved from the vanilla `font/default.json`
/// bitmap provider chain. Lookup priority mirrors vanilla
/// `FontSet.computeGlyphInfo`: providers are walked in flattened `providers`
/// order and the first one supplying a codepoint wins, so inserts are
/// first-wins.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HudFontGlyphMap {
    /// Sorted by codepoint for binary-search lookup; each codepoint appears
    /// at most once (the first inserted provider glyph).
    entries: Vec<(char, HudAsciiGlyph)>,
}

impl HudFontGlyphMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a glyph unless the codepoint is already mapped, returning
    /// whether the glyph was inserted. First-wins matches the vanilla
    /// provider fallback order (`FontSet.computeGlyphInfo`).
    pub fn insert_first_wins(&mut self, codepoint: char, glyph: HudAsciiGlyph) -> bool {
        match self
            .entries
            .binary_search_by_key(&codepoint, |(existing, _)| *existing)
        {
            Ok(_) => false,
            Err(index) => {
                self.entries.insert(index, (codepoint, glyph));
                true
            }
        }
    }

    pub fn get(&self, codepoint: char) -> Option<HudAsciiGlyph> {
        self.entries
            .binary_search_by_key(&codepoint, |(existing, _)| *existing)
            .ok()
            .map(|index| self.entries[index].1)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (char, HudAsciiGlyph)> + '_ {
        self.entries.iter().copied()
    }
}

/// Vanilla `LegacyRandomSource` (`RandomSource.create()`) 48-bit LCG constants.
/// `Font.random` advances this generator once per obfuscated glyph
/// (`FontSet.getRandomGlyph`). Vanilla seeds it from a wall-clock
/// `RandomSupport.generateUniqueSeed()`; bbb substitutes a caller-supplied
/// deterministic seed (the render frame counter) so the obfuscated jitter stays
/// reproducible — same frame -> same glyph sequence — never wall-clock random.
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

/// Same-advance glyph pool for obfuscated (`§k`) text, mirroring vanilla
/// `FontSet.glyphsByWidth` (`Int2ObjectMap<IntList>` keyed by
/// `Mth.ceil(getAdvance(false))`). Built once from a resolved
/// [`HudFontGlyphMap`] and cached by the renderer, so obfuscated draws never
/// rescan the full table per frame. `random_glyph` substitutes an equal-advance
/// glyph, keeping the pen width fixed while the drawn bitmap jitters.
///
/// Advances are already integer here (`u32`), so vanilla's `Mth.ceil` over
/// fractional TTF advances is a no-op and the bucket key is the advance itself.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HudObfuscatedGlyphPool {
    /// Sorted by advance for binary-search lookup; each bucket lists every
    /// glyph with that advance in codepoint order (the map iterates sorted),
    /// so the pool ordering is deterministic.
    buckets: Vec<(u32, Vec<HudAsciiGlyph>)>,
}

impl HudObfuscatedGlyphPool {
    /// Groups every glyph in the map by advance, mirroring vanilla's
    /// `glyphsByWidth.computeIfAbsent(...).add(codepoint)` sweep over supported
    /// glyphs. The map already holds only resolved provider glyphs in
    /// first-wins order, matching the non-missing filter vanilla applies.
    pub fn from_glyph_map(map: &HudFontGlyphMap) -> Self {
        let mut buckets: Vec<(u32, Vec<HudAsciiGlyph>)> = Vec::new();
        for (_codepoint, glyph) in map.iter() {
            match buckets.binary_search_by_key(&glyph.advance, |(advance, _)| *advance) {
                Ok(index) => buckets[index].1.push(glyph),
                Err(index) => buckets.insert(index, (glyph.advance, vec![glyph])),
            }
        }
        Self { buckets }
    }

    /// Uniformly picks a glyph sharing `advance` (vanilla
    /// `FontSet.getRandomGlyph`), advancing `random` exactly once. Returns
    /// `None` when no glyph has that advance, so the caller keeps the original
    /// glyph (vanilla falls back to the invisible missing glyph).
    pub fn random_glyph(
        &self,
        advance: u32,
        random: &mut HudObfuscatedRandom,
    ) -> Option<HudAsciiGlyph> {
        let index = self
            .buckets
            .binary_search_by_key(&advance, |(bucket_advance, _)| *bucket_advance)
            .ok()?;
        let bucket = &self.buckets[index].1;
        if bucket.is_empty() {
            return None;
        }
        let pick = random.next_int_bound(bucket.len() as u32) as usize;
        Some(bucket[pick])
    }

    /// Number of distinct advance buckets (test/introspection aid).
    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_map_lookup_is_first_wins() {
        let mut map = HudFontGlyphMap::new();
        let first = HudAsciiGlyph {
            advance: 7,
            ascent: 7,
            ..HudAsciiGlyph::default()
        };
        let second = HudAsciiGlyph {
            advance: 9,
            ascent: 10,
            ..HudAsciiGlyph::default()
        };

        assert!(map.insert_first_wins('é', first));
        assert!(!map.insert_first_wins('é', second));

        assert_eq!(map.get('é'), Some(first));
        assert_eq!(map.get('e'), None);
        assert_eq!(map.len(), 1);
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
            advance: 6,
            ascent: 7,
        }
    }

    fn assert_close(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-4, "expected {a} ~= {b}");
    }

    #[test]
    fn styled_advance_only_bold_widens_the_pen() {
        let glyph = styled_glyph();
        // Vanilla GlyphInfo.getAdvance(bold) = advance + (bold ? 1 : 0).
        assert_eq!(glyph.styled_advance(HudTextStyle::default()), 6);
        assert_eq!(
            glyph.styled_advance(HudTextStyle {
                bold: true,
                ..Default::default()
            }),
            7
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
            assert_eq!(glyph.styled_advance(style), 6);
        }
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
    fn effect_first_in_line_extends_left_and_bold_widens_span() {
        let glyph = styled_glyph();
        let style = HudTextStyle {
            underlined: true,
            bold: true,
            ..Default::default()
        };
        let rects = glyph.styled_effect_rects(4.0, 0.0, style, true);
        assert_eq!(rects.len(), 1);
        // Vanilla position==0: effectX0 = x - 1.
        assert_close(rects[0].x0, 4.0 - 1.0);
        // Bold-aware advance widens the bar: x + (advance + boldOffset).
        assert_close(rects[0].x1, 4.0 + 7.0);
    }

    #[test]
    fn baseline_offset_follows_vanilla_glyph_bitmap_top() {
        // GlyphBitmap.getTop(): 7.0F - ascent.
        let ascii = HudAsciiGlyph {
            ascent: 7,
            ..HudAsciiGlyph::default()
        };
        let accented = HudAsciiGlyph {
            ascent: 10,
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
            advance,
            ascent: 7,
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
            let glyph = pool.random_glyph(6, &mut random).expect("advance-6 bucket");
            assert_eq!(glyph.advance, 6);
            assert!(glyph.uv.min[0] == 0.1 || glyph.uv.min[0] == 0.2);
        }
        // The advance-4 bucket only yields its single member.
        let mut random = HudObfuscatedRandom::with_seed(1);
        let glyph = pool.random_glyph(4, &mut random).expect("advance-4 bucket");
        assert_eq!(glyph.advance, 4);
        assert_eq!(glyph.uv.min[0], 0.3);
        // No bucket for an unseen advance.
        assert!(pool
            .random_glyph(9, &mut HudObfuscatedRandom::with_seed(1))
            .is_none());
        assert!(HudObfuscatedGlyphPool::default()
            .random_glyph(6, &mut HudObfuscatedRandom::with_seed(1))
            .is_none());
    }
}
