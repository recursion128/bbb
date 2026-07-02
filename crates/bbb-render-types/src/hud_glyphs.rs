pub const HUD_ASCII_FIRST_GLYPH: u8 = b' ';
pub const HUD_ASCII_LAST_GLYPH: u8 = b'~';
pub const HUD_ASCII_GLYPH_COUNT: usize =
    (HUD_ASCII_LAST_GLYPH - HUD_ASCII_FIRST_GLYPH + 1) as usize;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudUvRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudDigitGlyph {
    pub uv: HudUvRect,
    pub width: u32,
    pub height: u32,
    pub advance: u32,
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
        }
    }
}

pub type HudAsciiGlyph = HudDigitGlyph;
