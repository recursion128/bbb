use anyhow::Result;

mod atlas;
mod block_models;
mod colors;
mod roots;
mod sprites;

pub use atlas::{AtlasImage, AtlasLayout, AtlasPacker, AtlasRect, AtlasSprite};
pub use block_models::{
    BlockFaceTextures, BlockModelBox, BlockModelCatalog, BlockModelFace, BlockModelShape,
    BlockRenderModel,
};
pub use colors::{
    BiomeColorCatalog, BiomeColorProfile, ColorMapImage, GrassColorModifier, TerrainColorMaps,
};
pub use roots::{PackRoots, DEFAULT_MC_CODE_ROOT, MC_VERSION};
pub use sprites::{SpriteImage, SpriteSource};

pub(crate) fn rgba_offset(width: u32, x: u32, y: u32) -> Result<usize> {
    let pixel = y
        .checked_mul(width)
        .and_then(|row| row.checked_add(x))
        .ok_or_else(|| anyhow::anyhow!("RGBA offset overflow"))?;
    usize::try_from(pixel)
        .ok()
        .and_then(|pixel| pixel.checked_mul(4))
        .ok_or_else(|| anyhow::anyhow!("RGBA offset overflow"))
}

pub(crate) fn rgba_len(width: u32, height: u32) -> Result<usize> {
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| anyhow::anyhow!("RGBA image size overflow"))?;
    usize::try_from(pixels)
        .ok()
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow::anyhow!("RGBA image size overflow"))
}
