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
}
