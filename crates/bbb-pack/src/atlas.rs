use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::{
    mipmap::generate_sprite_rgba_mip_levels, rgba_len, rgba_offset, SpriteAnimation,
    SpriteGuiMetadata, SpriteImage, SpriteMipLevel, SpriteSource, SpriteTextureMetadata,
    SpriteTransparency,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtlasRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtlasSprite {
    pub id: String,
    pub source_width: u32,
    pub source_height: u32,
    pub transparency: SpriteTransparency,
    #[serde(default)]
    pub animation: Option<SpriteAnimation>,
    #[serde(default)]
    pub texture_metadata: SpriteTextureMetadata,
    #[serde(default)]
    pub gui_metadata: SpriteGuiMetadata,
    pub content: AtlasRect,
    pub padded: AtlasRect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtlasLayout {
    pub width: u32,
    pub height: u32,
    pub padding: u32,
    pub sprites: Vec<AtlasSprite>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtlasImage {
    pub layout: AtlasLayout,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtlasMipLevel {
    pub level: u32,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtlasMipImage {
    pub layout: AtlasLayout,
    pub levels: Vec<AtlasMipLevel>,
}

impl AtlasMipImage {
    pub fn base_rgba(&self) -> &[u8] {
        self.levels
            .first()
            .map_or(&[], |level| level.rgba.as_slice())
    }

    pub fn rgba_slices(&self) -> Vec<&[u8]> {
        self.levels
            .iter()
            .map(|level| level.rgba.as_slice())
            .collect()
    }

    pub fn mip_level(&self) -> u32 {
        self.levels.len().saturating_sub(1) as u32
    }
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
                texture_metadata: source.texture_metadata,
                gui_metadata: source.gui_metadata,
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
        self.stitch_with_animation_tick(images, None)
    }

    pub fn stitch_animation_frame(&self, images: &[SpriteImage], tick: u64) -> Result<AtlasImage> {
        self.stitch_with_animation_tick(images, Some(tick))
    }

    pub fn stitch_mips(&self, images: &[SpriteImage], mip_level: u32) -> Result<AtlasMipImage> {
        self.stitch_with_animation_tick_mips(images, None, mip_level)
    }

    pub fn stitch_mips_with_max_level(
        &self,
        images: &[SpriteImage],
        max_mipmap_levels: u32,
    ) -> Result<AtlasMipImage> {
        let sources = images.iter().map(SpriteImage::source).collect::<Vec<_>>();
        let mip_level = vanilla_mip_level(&sources, max_mipmap_levels);
        self.stitch_with_animation_tick_mips(images, None, mip_level)
    }

    pub fn stitch_animation_frame_mips(
        &self,
        images: &[SpriteImage],
        tick: u64,
        mip_level: u32,
    ) -> Result<AtlasMipImage> {
        self.stitch_with_animation_tick_mips(images, Some(tick), mip_level)
    }

    pub fn stitch_animation_frame_mips_with_max_level(
        &self,
        images: &[SpriteImage],
        tick: u64,
        max_mipmap_levels: u32,
    ) -> Result<AtlasMipImage> {
        let sources = images.iter().map(SpriteImage::source).collect::<Vec<_>>();
        let mip_level = vanilla_mip_level(&sources, max_mipmap_levels);
        self.stitch_with_animation_tick_mips(images, Some(tick), mip_level)
    }

    fn stitch_with_animation_tick(
        &self,
        images: &[SpriteImage],
        tick: Option<u64>,
    ) -> Result<AtlasImage> {
        let sources = images.iter().map(SpriteImage::source).collect::<Vec<_>>();
        let mut layout = self.pack(&sources)?;
        for (sprite, image) in layout.sprites.iter_mut().zip(images) {
            sprite.transparency = image.transparency;
            sprite.animation = image.animation.clone();
            sprite.texture_metadata = image.texture_metadata;
            sprite.gui_metadata = image.gui_metadata;
        }
        let mut rgba = vec![0; rgba_len(layout.width, layout.height)?];

        for (sprite, image) in layout.sprites.iter().zip(images) {
            copy_sprite_with_gutter(&mut rgba, layout.width, sprite, image, tick)?;
        }

        Ok(AtlasImage { layout, rgba })
    }

    fn stitch_with_animation_tick_mips(
        &self,
        images: &[SpriteImage],
        tick: Option<u64>,
        mip_level: u32,
    ) -> Result<AtlasMipImage> {
        let sources = images.iter().map(SpriteImage::source).collect::<Vec<_>>();
        let mut layout = self.pack(&sources)?;
        for (sprite, image) in layout.sprites.iter_mut().zip(images) {
            sprite.transparency = image.transparency;
            sprite.animation = image.animation.clone();
            sprite.texture_metadata = image.texture_metadata;
            sprite.gui_metadata = image.gui_metadata;
        }

        let mut sprite_mips = Vec::with_capacity(images.len());
        for image in images {
            let source_rgba = match tick {
                Some(tick) => image.frame_rgba_at_tick(tick)?,
                None => std::borrow::Cow::Borrowed(image.rgba.as_slice()),
            };
            sprite_mips.push(generate_sprite_rgba_mip_levels(
                &image.id,
                image.width,
                image.height,
                source_rgba.as_ref(),
                image.texture_metadata,
                image.transparency,
                mip_level,
            )?);
        }

        let mut levels = Vec::with_capacity(mip_level as usize + 1);
        for level in 0..=mip_level {
            let width = layout.width.checked_shr(level).unwrap_or(0);
            let height = layout.height.checked_shr(level).unwrap_or(0);
            if width == 0 || height == 0 {
                bail!(
                    "atlas mip level {} has zero-sized dimensions from {}x{}",
                    level,
                    layout.width,
                    layout.height
                );
            }
            let mut rgba = vec![0; rgba_len(width, height)?];
            for (sprite, mips) in layout.sprites.iter().zip(&sprite_mips) {
                let mip = &mips[level as usize];
                copy_sprite_mip_with_gutter(&mut rgba, width, sprite, level, mip)?;
            }
            levels.push(AtlasMipLevel {
                level,
                width,
                height,
                rgba,
            });
        }

        Ok(AtlasMipImage { layout, levels })
    }
}

fn copy_sprite_with_gutter(
    atlas: &mut [u8],
    atlas_width: u32,
    sprite: &AtlasSprite,
    image: &SpriteImage,
    tick: Option<u64>,
) -> Result<()> {
    let source_rgba = match tick {
        Some(tick) => image.frame_rgba_at_tick(tick)?,
        None => std::borrow::Cow::Borrowed(image.rgba.as_slice()),
    };
    copy_sprite_rgba_with_gutter(
        atlas,
        atlas_width,
        sprite,
        image.width,
        image.height,
        source_rgba.as_ref(),
    )
}

fn copy_sprite_rgba_with_gutter(
    atlas: &mut [u8],
    atlas_width: u32,
    sprite: &AtlasSprite,
    source_width: u32,
    source_height: u32,
    source_rgba: &[u8],
) -> Result<()> {
    copy_sprite_rect_rgba_with_gutter(
        atlas,
        atlas_width,
        &sprite.id,
        sprite.content,
        sprite.padded,
        source_width,
        source_height,
        source_rgba,
    )
}

fn copy_sprite_mip_with_gutter(
    atlas: &mut [u8],
    atlas_width: u32,
    sprite: &AtlasSprite,
    level: u32,
    mip: &SpriteMipLevel,
) -> Result<()> {
    let content = AtlasRect {
        x: sprite.content.x.checked_shr(level).unwrap_or(0),
        y: sprite.content.y.checked_shr(level).unwrap_or(0),
        width: mip.width,
        height: mip.height,
    };
    let mut padded = mip_rect(sprite.padded, level)?;
    let content_right = content
        .x
        .checked_add(content.width)
        .ok_or_else(|| anyhow::anyhow!("atlas mip content width overflow"))?;
    let content_bottom = content
        .y
        .checked_add(content.height)
        .ok_or_else(|| anyhow::anyhow!("atlas mip content height overflow"))?;
    if content.x < padded.x || content.y < padded.y {
        bail!(
            "sprite {} mip level {} content is outside padding",
            sprite.id,
            level
        );
    }
    let padded_right = padded
        .x
        .checked_add(padded.width)
        .ok_or_else(|| anyhow::anyhow!("atlas mip padded width overflow"))?;
    let padded_bottom = padded
        .y
        .checked_add(padded.height)
        .ok_or_else(|| anyhow::anyhow!("atlas mip padded height overflow"))?;
    if content_right > padded_right {
        padded.width = content_right - padded.x;
    }
    if content_bottom > padded_bottom {
        padded.height = content_bottom - padded.y;
    }
    copy_sprite_rect_rgba_with_gutter(
        atlas,
        atlas_width,
        &sprite.id,
        content,
        padded,
        mip.width,
        mip.height,
        &mip.rgba,
    )
}

fn copy_sprite_rect_rgba_with_gutter(
    atlas: &mut [u8],
    atlas_width: u32,
    sprite_id: &str,
    content: AtlasRect,
    padded: AtlasRect,
    source_width: u32,
    source_height: u32,
    source_rgba: &[u8],
) -> Result<()> {
    let expected_len = rgba_len(source_width, source_height)?;
    if source_rgba.len() != expected_len {
        bail!(
            "sprite {} frame has {} RGBA bytes, expected {}",
            sprite_id,
            source_rgba.len(),
            expected_len
        );
    }
    let content_offset_x = content
        .x
        .checked_sub(padded.x)
        .ok_or_else(|| anyhow::anyhow!("sprite {sprite_id} content starts before padding"))?;
    let content_offset_y = content
        .y
        .checked_sub(padded.y)
        .ok_or_else(|| anyhow::anyhow!("sprite {sprite_id} content starts before padding"))?;
    for local_y in 0..padded.height {
        let source_y = local_y
            .saturating_sub(content_offset_y)
            .min(source_height - 1);
        for local_x in 0..padded.width {
            let source_x = local_x
                .saturating_sub(content_offset_x)
                .min(source_width - 1);
            let source_offset = rgba_offset(source_width, source_x, source_y)?;
            let atlas_offset = rgba_offset(atlas_width, padded.x + local_x, padded.y + local_y)?;
            atlas[atlas_offset..atlas_offset + 4]
                .copy_from_slice(&source_rgba[source_offset..source_offset + 4]);
        }
    }
    Ok(())
}

fn mip_rect(rect: AtlasRect, level: u32) -> Result<AtlasRect> {
    let x = rect.x.checked_shr(level).unwrap_or(0);
    let y = rect.y.checked_shr(level).unwrap_or(0);
    let right = rect
        .x
        .checked_add(rect.width)
        .ok_or_else(|| anyhow::anyhow!("atlas rect width overflow"))?
        .checked_shr(level)
        .unwrap_or(0);
    let bottom = rect
        .y
        .checked_add(rect.height)
        .ok_or_else(|| anyhow::anyhow!("atlas rect height overflow"))?
        .checked_shr(level)
        .unwrap_or(0);
    Ok(AtlasRect {
        x,
        y,
        width: right.saturating_sub(x),
        height: bottom.saturating_sub(y),
    })
}

fn vanilla_mip_level(sources: &[SpriteSource], max_mipmap_levels: u32) -> u32 {
    let Some(first) = sources.first() else {
        return 0;
    };
    let mut min_texel_size = first.width.min(first.height);
    let mut lowest_bit = 1u32.checked_shl(max_mipmap_levels).unwrap_or(u32::MAX);
    for source in sources {
        min_texel_size = min_texel_size.min(source.width.min(source.height));
        let lowest_texture_bit = lowest_one_bit(source.width).min(lowest_one_bit(source.height));
        lowest_bit = lowest_bit.min(lowest_texture_bit);
    }

    let min_size = min_texel_size.min(lowest_bit).max(1);
    floor_log2(min_size).min(max_mipmap_levels)
}

fn lowest_one_bit(value: u32) -> u32 {
    value & value.wrapping_neg()
}

fn floor_log2(value: u32) -> u32 {
    u32::BITS - 1 - value.leading_zeros()
}

#[cfg(test)]
mod tests {
    use super::{AtlasPacker, AtlasRect};
    use crate::{
        SpriteAnimation, SpriteAnimationFrame, SpriteGuiMetadata, SpriteGuiScaling, SpriteImage,
        SpriteMipmapStrategy, SpriteNineSliceBorder, SpriteSource, SpriteTextureMetadata,
        SpriteTransparency,
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
    fn atlas_layout_preserves_sprite_metadata() {
        let animation = SpriteAnimation {
            frame_count: 2,
            default_frame_time: 4,
            interpolate: true,
            frames: vec![
                SpriteAnimationFrame { index: 0, time: 4 },
                SpriteAnimationFrame { index: 1, time: 8 },
            ],
        };
        let texture_metadata = SpriteTextureMetadata {
            blur: false,
            clamp: false,
            mipmap_strategy: SpriteMipmapStrategy::StrictCutout,
            alpha_cutoff_bias: 0.125,
        };
        let gui_metadata = SpriteGuiMetadata {
            scaling: SpriteGuiScaling::NineSlice {
                width: 16,
                height: 16,
                border: SpriteNineSliceBorder::uniform(2),
                stretch_inner: false,
            },
        };
        let source = SpriteSource {
            id: "minecraft:block/water_still".to_string(),
            width: 16,
            height: 16,
            animation: Some(animation.clone()),
            texture_metadata,
            gui_metadata,
        };

        let layout = AtlasPacker::new(64, 1).unwrap().pack(&[source]).unwrap();

        assert_eq!(layout.sprites[0].animation, Some(animation));
        assert_eq!(layout.sprites[0].texture_metadata, texture_metadata);
        assert_eq!(layout.sprites[0].gui_metadata, gui_metadata);
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
    fn atlas_stitcher_preserves_image_metadata() {
        let animation = SpriteAnimation {
            frame_count: 2,
            default_frame_time: 2,
            interpolate: false,
            frames: vec![
                SpriteAnimationFrame { index: 0, time: 2 },
                SpriteAnimationFrame { index: 1, time: 2 },
            ],
        };
        let texture_metadata = SpriteTextureMetadata {
            blur: true,
            clamp: true,
            mipmap_strategy: SpriteMipmapStrategy::Mean,
            alpha_cutoff_bias: 0.0,
        };
        let gui_metadata = SpriteGuiMetadata {
            scaling: SpriteGuiScaling::Tile {
                width: 4,
                height: 4,
            },
        };
        let image = SpriteImage {
            id: "minecraft:block/campfire_fire".to_string(),
            width: 1,
            height: 1,
            transparency: SpriteTransparency::default(),
            animation: Some(animation.clone()),
            texture_metadata,
            gui_metadata,
            animation_frames_rgba: vec![vec![255, 255, 255, 255]; 2],
            rgba: vec![255, 255, 255, 255],
        };

        let atlas = AtlasPacker::new(8, 1).unwrap().stitch(&[image]).unwrap();

        assert_eq!(atlas.layout.sprites[0].animation, Some(animation));
        assert_eq!(atlas.layout.sprites[0].texture_metadata, texture_metadata);
        assert_eq!(atlas.layout.sprites[0].gui_metadata, gui_metadata);
    }

    #[test]
    fn atlas_stitcher_can_render_animation_frame_for_tick() {
        let animation = SpriteAnimation {
            frame_count: 2,
            default_frame_time: 1,
            interpolate: false,
            frames: vec![
                SpriteAnimationFrame { index: 0, time: 2 },
                SpriteAnimationFrame { index: 1, time: 1 },
            ],
        };
        let image = SpriteImage {
            id: "minecraft:block/water_still".to_string(),
            width: 1,
            height: 1,
            transparency: SpriteTransparency::default(),
            animation: Some(animation),
            texture_metadata: SpriteTextureMetadata::default(),
            gui_metadata: SpriteGuiMetadata::default(),
            animation_frames_rgba: vec![vec![10, 0, 0, 255], vec![20, 0, 0, 255]],
            rgba: vec![10, 0, 0, 255],
        };

        let initial = AtlasPacker::new(8, 1)
            .unwrap()
            .stitch(std::slice::from_ref(&image))
            .unwrap();
        let tick_two = AtlasPacker::new(8, 1)
            .unwrap()
            .stitch_animation_frame(std::slice::from_ref(&image), 2)
            .unwrap();
        let tick_three = AtlasPacker::new(8, 1)
            .unwrap()
            .stitch_animation_frame(&[image], 3)
            .unwrap();

        assert_eq!(
            pixel(&initial.rgba, initial.layout.width, 1, 1),
            [10, 0, 0, 255]
        );
        assert_eq!(
            pixel(&tick_two.rgba, tick_two.layout.width, 1, 1),
            [20, 0, 0, 255]
        );
        assert_eq!(
            pixel(&tick_three.rgba, tick_three.layout.width, 1, 1),
            [10, 0, 0, 255]
        );
    }

    #[test]
    fn atlas_stitcher_interpolates_animation_frame_for_tick() {
        let image = SpriteImage {
            id: "minecraft:block/sculk".to_string(),
            width: 1,
            height: 1,
            transparency: SpriteTransparency::default(),
            animation: Some(SpriteAnimation {
                frame_count: 2,
                default_frame_time: 1,
                interpolate: true,
                frames: vec![
                    SpriteAnimationFrame { index: 0, time: 4 },
                    SpriteAnimationFrame { index: 1, time: 4 },
                ],
            }),
            texture_metadata: SpriteTextureMetadata::default(),
            gui_metadata: SpriteGuiMetadata::default(),
            animation_frames_rgba: vec![vec![0, 0, 100, 255], vec![100, 40, 0, 127]],
            rgba: vec![0, 0, 100, 255],
        };

        let tick_two = AtlasPacker::new(8, 1)
            .unwrap()
            .stitch_animation_frame(&[image], 2)
            .unwrap();

        assert_eq!(
            pixel(&tick_two.rgba, tick_two.layout.width, 1, 1),
            [50, 20, 50, 191]
        );
    }

    #[test]
    fn atlas_mips_are_generated_per_sprite_without_cross_sprite_blending() {
        let red = SpriteImage::new(
            "minecraft:block/red",
            2,
            2,
            vec![
                200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255,
            ],
        )
        .unwrap();
        let blue = SpriteImage::new(
            "minecraft:block/blue",
            2,
            2,
            vec![
                0, 0, 200, 255, 0, 0, 200, 255, 0, 0, 200, 255, 0, 0, 200, 255,
            ],
        )
        .unwrap();

        let atlas = AtlasPacker::new(16, 1)
            .unwrap()
            .stitch_mips(&[red, blue], 1)
            .unwrap();

        assert_eq!(atlas.levels.len(), 2);
        assert_eq!((atlas.levels[1].width, atlas.levels[1].height), (4, 2));
        assert_eq!(
            pixel(&atlas.levels[1].rgba, atlas.levels[1].width, 0, 0),
            [200, 0, 0, 255]
        );
        assert_eq!(
            pixel(&atlas.levels[1].rgba, atlas.levels[1].width, 1, 0),
            [200, 0, 0, 255]
        );
        assert_eq!(
            pixel(&atlas.levels[1].rgba, atlas.levels[1].width, 2, 0),
            [0, 0, 200, 255]
        );
        assert_eq!(
            pixel(&atlas.levels[1].rgba, atlas.levels[1].width, 3, 0),
            [0, 0, 200, 255]
        );
    }

    #[test]
    fn atlas_mips_with_max_level_follow_vanilla_lowest_one_bit_limit() {
        let image = SpriteImage::new(
            "minecraft:block/non_power_of_two",
            12,
            12,
            vec![80; 12 * 12 * 4],
        )
        .unwrap();

        let atlas = AtlasPacker::new(32, 1)
            .unwrap()
            .stitch_mips_with_max_level(&[image], 4)
            .unwrap();

        assert_eq!(atlas.mip_level(), 2);
        assert_eq!(
            atlas
                .levels
                .iter()
                .map(|level| (level.width, level.height))
                .collect::<Vec<_>>(),
            vec![(14, 14), (7, 7), (3, 3)]
        );
    }

    fn pixel(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let offset = ((y * width + x) * 4) as usize;
        rgba[offset..offset + 4].try_into().unwrap()
    }
}
