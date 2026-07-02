/// Per-pixel alpha coverage of one sprite frame, row-major (`width * height`), `true` where the pixel is
/// opaque enough to contribute geometry. The native layer derives this from the sprite's atlas pixels
/// (vanilla `SpriteContents.isTransparent`: a pixel is transparent when its alpha is below the cutoff).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteAlphaMask {
    width: u32,
    height: u32,
    opaque: Vec<bool>,
}

impl SpriteAlphaMask {
    /// `opaque` is row-major, `width * height` booleans (`true` = opaque). Panics if the length mismatches.
    pub fn new(width: u32, height: u32, opaque: Vec<bool>) -> Self {
        assert_eq!(
            opaque.len(),
            (width as usize) * (height as usize),
            "sprite alpha mask length must be width * height"
        );
        Self {
            width,
            height,
            opaque,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Vanilla `ItemModelGenerator.isTransparent`: out-of-bounds counts as transparent.
    pub fn is_transparent(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return true;
        }
        !self.opaque[(y as u32 * self.width + x as u32) as usize]
    }
}

/// The atlas sub-rectangle (absolute UVs) a sprite occupies: its `min`/`max` corners. A sprite-local UV
/// in `0..=1` maps linearly into this rect (vanilla `TextureAtlasSprite.getU`/`getV`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemSpriteRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

impl ItemSpriteRect {
    pub fn map(&self, u: f32, v: f32) -> [f32; 2] {
        [
            self.min[0] + (self.max[0] - self.min[0]) * u,
            self.min[1] + (self.max[1] - self.min[1]) * v,
        ]
    }
}
