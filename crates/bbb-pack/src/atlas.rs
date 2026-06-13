use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::{rgba_len, rgba_offset, SpriteImage, SpriteSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtlasRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtlasSprite {
    pub id: String,
    pub source_width: u32,
    pub source_height: u32,
    pub content: AtlasRect,
    pub padded: AtlasRect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtlasLayout {
    pub width: u32,
    pub height: u32,
    pub padding: u32,
    pub sprites: Vec<AtlasSprite>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtlasImage {
    pub layout: AtlasLayout,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasPacker {
    max_width: u32,
    padding: u32,
}

impl AtlasPacker {
    pub fn new(max_width: u32, padding: u32) -> Result<Self> {
        if max_width == 0 {
            bail!("atlas max width must be greater than zero");
        }
        Ok(Self { max_width, padding })
    }

    pub fn pack(&self, sources: &[SpriteSource]) -> Result<AtlasLayout> {
        let mut sprites = Vec::with_capacity(sources.len());
        let mut cursor_x: u32 = 0;
        let mut cursor_y: u32 = 0;
        let mut row_height: u32 = 0;
        let mut used_width: u32 = 0;

        for source in sources {
            if source.width == 0 || source.height == 0 {
                bail!("sprite {} has zero-sized dimensions", source.id);
            }

            let gutter = match self.padding.checked_mul(2) {
                Some(gutter) => gutter,
                None => bail!("atlas padding overflow"),
            };
            let padded_width = match source.width.checked_add(gutter) {
                Some(width) => width,
                None => bail!("sprite {} padded width overflow", source.id),
            };
            let padded_height = match source.height.checked_add(gutter) {
                Some(height) => height,
                None => bail!("sprite {} padded height overflow", source.id),
            };
            if padded_width > self.max_width {
                bail!(
                    "sprite {} padded width {} exceeds atlas max width {}",
                    source.id,
                    padded_width,
                    self.max_width
                );
            }

            let row_width = match cursor_x.checked_add(padded_width) {
                Some(width) => width,
                None => bail!("atlas row width overflow"),
            };
            if cursor_x > 0 && row_width > self.max_width {
                cursor_y = match cursor_y.checked_add(row_height) {
                    Some(y) => y,
                    None => bail!("atlas height overflow"),
                };
                cursor_x = 0;
                row_height = 0;
            }

            let padded = AtlasRect {
                x: cursor_x,
                y: cursor_y,
                width: padded_width,
                height: padded_height,
            };
            let content = AtlasRect {
                x: cursor_x + self.padding,
                y: cursor_y + self.padding,
                width: source.width,
                height: source.height,
            };
            sprites.push(AtlasSprite {
                id: source.id.clone(),
                source_width: source.width,
                source_height: source.height,
                content,
                padded,
            });

            cursor_x = match cursor_x.checked_add(padded_width) {
                Some(x) => x,
                None => bail!("atlas row width overflow"),
            };
            row_height = row_height.max(padded_height);
            used_width = used_width.max(cursor_x);
        }

        Ok(AtlasLayout {
            width: used_width,
            height: match cursor_y.checked_add(row_height) {
                Some(height) => height,
                None => bail!("atlas height overflow"),
            },
            padding: self.padding,
            sprites,
        })
    }

    pub fn stitch(&self, images: &[SpriteImage]) -> Result<AtlasImage> {
        let sources = images.iter().map(SpriteImage::source).collect::<Vec<_>>();
        let layout = self.pack(&sources)?;
        let mut rgba = vec![0; rgba_len(layout.width, layout.height)?];

        for (sprite, image) in layout.sprites.iter().zip(images) {
            copy_sprite_with_gutter(&mut rgba, layout.width, sprite, image)?;
        }

        Ok(AtlasImage { layout, rgba })
    }
}

fn copy_sprite_with_gutter(
    atlas: &mut [u8],
    atlas_width: u32,
    sprite: &AtlasSprite,
    image: &SpriteImage,
) -> Result<()> {
    for local_y in 0..sprite.padded.height {
        let source_y = local_y
            .saturating_sub(sprite.content.y - sprite.padded.y)
            .min(image.height - 1);
        for local_x in 0..sprite.padded.width {
            let source_x = local_x
                .saturating_sub(sprite.content.x - sprite.padded.x)
                .min(image.width - 1);
            let source_offset = rgba_offset(image.width, source_x, source_y)?;
            let atlas_offset = rgba_offset(
                atlas_width,
                sprite.padded.x + local_x,
                sprite.padded.y + local_y,
            )?;
            atlas[atlas_offset..atlas_offset + 4]
                .copy_from_slice(&image.rgba[source_offset..source_offset + 4]);
        }
    }
    Ok(())
}
