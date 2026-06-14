use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::{
    rgba_len, rgba_offset, SpriteAnimation, SpriteImage, SpriteSource, SpriteTransparency,
};

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
    pub transparency: SpriteTransparency,
    #[serde(default)]
    pub animation: Option<SpriteAnimation>,
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
                transparency: SpriteTransparency::default(),
                animation: source.animation.clone(),
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
        let mut layout = self.pack(&sources)?;
        for (sprite, image) in layout.sprites.iter_mut().zip(images) {
            sprite.transparency = image.transparency;
            sprite.animation = image.animation.clone();
        }
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

#[cfg(test)]
mod tests {
    use super::{AtlasPacker, AtlasRect};
    use crate::{
        SpriteAnimation, SpriteAnimationFrame, SpriteImage, SpriteSource, SpriteTransparency,
    };

    #[test]
    fn atlas_rects_preserve_content_dimensions_inside_padding() {
        let layout = AtlasPacker::new(128, 2)
            .unwrap()
            .pack(&[
                SpriteSource::new("minecraft:block/stone", 16, 16),
                SpriteSource::new("pack:block/hd_overlay", 64, 32),
            ])
            .unwrap();

        assert_eq!(layout.width, 88);
        assert_eq!(layout.height, 36);
        assert_eq!(layout.padding, 2);

        let stone = &layout.sprites[0];
        assert_eq!(stone.source_width, 16);
        assert_eq!(stone.source_height, 16);
        assert_eq!(
            stone.padded,
            AtlasRect {
                x: 0,
                y: 0,
                width: 20,
                height: 20
            }
        );
        assert_eq!(
            stone.content,
            AtlasRect {
                x: 2,
                y: 2,
                width: 16,
                height: 16
            }
        );

        let overlay = &layout.sprites[1];
        assert_eq!(
            overlay.padded,
            AtlasRect {
                x: 20,
                y: 0,
                width: 68,
                height: 36
            }
        );
        assert_eq!(
            overlay.content,
            AtlasRect {
                x: 22,
                y: 2,
                width: 64,
                height: 32
            }
        );
    }

    #[test]
    fn atlas_packer_wraps_rows_for_mixed_resolution_sprites() {
        let layout = AtlasPacker::new(300, 1)
            .unwrap()
            .pack(&[
                SpriteSource::new("pack:block/large", 256, 256),
                SpriteSource::new("pack:block/medium", 64, 64),
                SpriteSource::new("minecraft:block/small", 16, 16),
            ])
            .unwrap();

        assert_eq!(layout.width, 258);
        assert_eq!(layout.height, 324);
        assert_eq!(
            layout.sprites[0].content,
            AtlasRect {
                x: 1,
                y: 1,
                width: 256,
                height: 256
            }
        );
        assert_eq!(
            layout.sprites[1].content,
            AtlasRect {
                x: 1,
                y: 259,
                width: 64,
                height: 64
            }
        );
        assert_eq!(
            layout.sprites[2].content,
            AtlasRect {
                x: 67,
                y: 259,
                width: 16,
                height: 16
            }
        );
    }

    #[test]
    fn atlas_packer_rejects_invalid_sprite_dimensions() {
        let zero = AtlasPacker::new(128, 1)
            .unwrap()
            .pack(&[SpriteSource::new("bad", 0, 16)]);
        assert!(zero.is_err());

        let too_wide = AtlasPacker::new(16, 1)
            .unwrap()
            .pack(&[SpriteSource::new("wide", 16, 16)]);
        assert!(too_wide.is_err());
    }

    #[test]
    fn atlas_layout_preserves_sprite_animation_metadata() {
        let animation = SpriteAnimation {
            frame_count: 2,
            default_frame_time: 4,
            interpolate: true,
            frames: vec![
                SpriteAnimationFrame { index: 0, time: 4 },
                SpriteAnimationFrame { index: 1, time: 8 },
            ],
        };
        let source = SpriteSource {
            id: "minecraft:block/water_still".to_string(),
            width: 16,
            height: 16,
            animation: Some(animation.clone()),
        };

        let layout = AtlasPacker::new(64, 1).unwrap().pack(&[source]).unwrap();

        assert_eq!(layout.sprites[0].animation, Some(animation));
    }

    #[test]
    fn atlas_stitcher_extends_sprite_edges_into_padding() {
        let image = SpriteImage::new(
            "test:quad",
            2,
            2,
            vec![10, 0, 0, 255, 20, 0, 0, 0, 30, 0, 0, 255, 40, 0, 0, 255],
        )
        .unwrap();
        let atlas = AtlasPacker::new(8, 1).unwrap().stitch(&[image]).unwrap();

        assert_eq!(atlas.layout.width, 4);
        assert_eq!(atlas.layout.height, 4);
        assert!(atlas.layout.sprites[0].transparency.has_transparent);
        assert!(!atlas.layout.sprites[0].transparency.has_translucent);
        assert_eq!(
            pixel(&atlas.rgba, atlas.layout.width, 0, 0),
            [10, 0, 0, 255]
        );
        assert_eq!(pixel(&atlas.rgba, atlas.layout.width, 3, 0), [20, 0, 0, 0]);
        assert_eq!(
            pixel(&atlas.rgba, atlas.layout.width, 0, 3),
            [30, 0, 0, 255]
        );
        assert_eq!(
            pixel(&atlas.rgba, atlas.layout.width, 3, 3),
            [40, 0, 0, 255]
        );
        assert_eq!(
            pixel(&atlas.rgba, atlas.layout.width, 1, 1),
            [10, 0, 0, 255]
        );
        assert_eq!(
            pixel(&atlas.rgba, atlas.layout.width, 2, 2),
            [40, 0, 0, 255]
        );
    }

    #[test]
    fn atlas_stitcher_preserves_image_animation_metadata() {
        let animation = SpriteAnimation {
            frame_count: 2,
            default_frame_time: 2,
            interpolate: false,
            frames: vec![
                SpriteAnimationFrame { index: 0, time: 2 },
                SpriteAnimationFrame { index: 1, time: 2 },
            ],
        };
        let image = SpriteImage {
            id: "minecraft:block/campfire_fire".to_string(),
            width: 1,
            height: 1,
            transparency: SpriteTransparency::default(),
            animation: Some(animation.clone()),
            rgba: vec![255, 255, 255, 255],
        };

        let atlas = AtlasPacker::new(8, 1).unwrap().stitch(&[image]).unwrap();

        assert_eq!(atlas.layout.sprites[0].animation, Some(animation));
    }

    fn pixel(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let offset = ((y * width + x) * 4) as usize;
        rgba[offset..offset + 4].try_into().unwrap()
    }
}
