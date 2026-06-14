use std::{collections::VecDeque, sync::OnceLock};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::{
    rgba_len, rgba_offset, SpriteImage, SpriteMipmapStrategy, SpriteTextureMetadata,
    SpriteTransparency,
};

const ALPHA_CUTOFF: f32 = 0.5;
const STRICT_ALPHA_CUTOFF: f32 = 0.3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteMipLevel {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl SpriteImage {
    /// Generates vanilla-style mip levels for this image's current RGBA frame.
    ///
    /// Animated sprites store their frame data separately; animation upload code
    /// should generate mips from the selected frame rather than treating this as
    /// the full vanilla animation sheet.
    pub fn mip_levels(&self, mip_level: u32) -> Result<Vec<SpriteMipLevel>> {
        generate_sprite_mip_levels(self, mip_level)
    }
}

pub fn generate_sprite_mip_levels(
    image: &SpriteImage,
    mip_level: u32,
) -> Result<Vec<SpriteMipLevel>> {
    generate_sprite_rgba_mip_levels(
        &image.id,
        image.width,
        image.height,
        &image.rgba,
        image.texture_metadata,
        image.transparency,
        mip_level,
    )
}

pub(crate) fn generate_sprite_rgba_mip_levels(
    id: &str,
    width: u32,
    height: u32,
    rgba: &[u8],
    metadata: SpriteTextureMetadata,
    transparency: SpriteTransparency,
    mip_level: u32,
) -> Result<Vec<SpriteMipLevel>> {
    validate_mip_level_request(width, height, rgba, mip_level)?;

    let strategy = resolve_mipmap_strategy(metadata.mipmap_strategy, transparency);
    let mut base = rgba.to_vec();
    if !resource_path(id).starts_with("item/") {
        match strategy {
            SpriteMipmapStrategy::Cutout | SpriteMipmapStrategy::StrictCutout => {
                solidify(&mut base, width, height)?;
            }
            SpriteMipmapStrategy::DarkCutout => {
                fill_empty_areas_with_dark_color(&mut base, width, height)?;
            }
            SpriteMipmapStrategy::Auto | SpriteMipmapStrategy::Mean => {}
        }
    }

    let mut levels = Vec::with_capacity(mip_level as usize + 1);
    levels.push(SpriteMipLevel {
        width,
        height,
        rgba: base,
    });

    let is_cutout_mip = is_cutout_mipmap_strategy(strategy);
    let cutout_ref = if strategy == SpriteMipmapStrategy::StrictCutout {
        STRICT_ALPHA_CUTOFF
    } else {
        ALPHA_CUTOFF
    };
    let original_coverage = if is_cutout_mip {
        alpha_test_coverage(&levels[0], cutout_ref, 1.0)?
    } else {
        0.0
    };

    for level in 1..=mip_level {
        let previous = &levels[(level - 1) as usize];
        let width = previous.width >> 1;
        let height = previous.height >> 1;
        if width == 0 || height == 0 {
            bail!(
                "sprite {} mip level {} would have zero-sized dimensions from {}x{}",
                id,
                level,
                previous.width,
                previous.height
            );
        }

        let mut rgba = vec![0; rgba_len(width, height)?];
        for y in 0..height {
            for x in 0..width {
                let color1 = pixel(previous, x * 2, y * 2)?;
                let color2 = pixel(previous, x * 2 + 1, y * 2)?;
                let color3 = pixel(previous, x * 2, y * 2 + 1)?;
                let color4 = pixel(previous, x * 2 + 1, y * 2 + 1)?;
                let color = if strategy == SpriteMipmapStrategy::DarkCutout {
                    darkened_alpha_blend(color1, color2, color3, color4)
                } else {
                    mean_linear(color1, color2, color3, color4)
                };
                set_pixel(&mut rgba, width, x, y, color)?;
            }
        }

        let mut next = SpriteMipLevel {
            width,
            height,
            rgba,
        };
        if is_cutout_mip {
            scale_alpha_to_coverage(
                &mut next,
                original_coverage,
                cutout_ref,
                metadata.alpha_cutoff_bias,
            )?;
        }
        levels.push(next);
    }

    Ok(levels)
}

fn validate_mip_level_request(width: u32, height: u32, rgba: &[u8], mip_level: u32) -> Result<()> {
    if width == 0 || height == 0 {
        bail!("sprite mipmap dimensions must be non-zero");
    }
    let expected = rgba_len(width, height)?;
    if rgba.len() != expected {
        bail!(
            "sprite mipmap base has {} RGBA bytes, expected {} for {}x{}",
            rgba.len(),
            expected,
            width,
            height
        );
    }
    if width.checked_shr(mip_level).unwrap_or(0) == 0
        || height.checked_shr(mip_level).unwrap_or(0) == 0
    {
        bail!(
            "sprite mip level {} exceeds base dimensions {}x{}",
            mip_level,
            width,
            height
        );
    }
    Ok(())
}

fn resolve_mipmap_strategy(
    strategy: SpriteMipmapStrategy,
    transparency: SpriteTransparency,
) -> SpriteMipmapStrategy {
    if strategy == SpriteMipmapStrategy::Auto {
        if transparency.has_transparent {
            SpriteMipmapStrategy::Cutout
        } else {
            SpriteMipmapStrategy::Mean
        }
    } else {
        strategy
    }
}

fn is_cutout_mipmap_strategy(strategy: SpriteMipmapStrategy) -> bool {
    matches!(
        strategy,
        SpriteMipmapStrategy::Cutout
            | SpriteMipmapStrategy::StrictCutout
            | SpriteMipmapStrategy::DarkCutout
    )
}

fn resource_path(id: &str) -> &str {
    id.split_once(':').map_or(id, |(_, path)| path)
}

fn solidify(rgba: &mut [u8], width: u32, height: u32) -> Result<()> {
    let pixel_count = pixel_count(width, height)?;
    let mut nearest_color = vec![[0, 0, 0, 0]; pixel_count];
    let mut distances = vec![u32::MAX; pixel_count];
    let mut queue = VecDeque::new();

    for x in 0..width {
        for y in 0..height {
            let index = packed_index(width, x, y)?;
            let color = rgba_pixel(rgba, width, x, y)?;
            if color[3] != 0 {
                distances[index] = 0;
                nearest_color[index] = color;
                queue.push_back((x, y));
            }
        }
    }

    while let Some((x, y)) = queue.pop_front() {
        let index = packed_index(width, x, y)?;
        for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
            let Some(neighbor_x) = x.checked_add_signed(dx) else {
                continue;
            };
            let Some(neighbor_y) = y.checked_add_signed(dy) else {
                continue;
            };
            if neighbor_x >= width || neighbor_y >= height {
                continue;
            }
            let neighbor_index = packed_index(width, neighbor_x, neighbor_y)?;
            if distances[neighbor_index] > distances[index].saturating_add(1) {
                distances[neighbor_index] = distances[index] + 1;
                nearest_color[neighbor_index] = nearest_color[index];
                queue.push_back((neighbor_x, neighbor_y));
            }
        }
    }

    for x in 0..width {
        for y in 0..height {
            let offset = rgba_offset(width, x, y)?;
            if rgba[offset + 3] == 0 {
                let mut color = nearest_color[packed_index(width, x, y)?];
                color[3] = 0;
                rgba[offset..offset + 4].copy_from_slice(&color);
            }
        }
    }
    Ok(())
}

fn fill_empty_areas_with_dark_color(rgba: &mut [u8], width: u32, height: u32) -> Result<()> {
    let mut darkest_color = [255, 255, 255, 255];
    let mut min_brightness = u32::MAX;

    for x in 0..width {
        for y in 0..height {
            let color = rgba_pixel(rgba, width, x, y)?;
            if color[3] != 0 {
                let brightness = u32::from(color[0]) + u32::from(color[1]) + u32::from(color[2]);
                if brightness < min_brightness {
                    min_brightness = brightness;
                    darkest_color = color;
                }
            }
        }
    }

    let darkened_color = [
        (3 * u16::from(darkest_color[0]) / 4) as u8,
        (3 * u16::from(darkest_color[1]) / 4) as u8,
        (3 * u16::from(darkest_color[2]) / 4) as u8,
        0,
    ];
    for x in 0..width {
        for y in 0..height {
            let offset = rgba_offset(width, x, y)?;
            if rgba[offset + 3] == 0 {
                rgba[offset..offset + 4].copy_from_slice(&darkened_color);
            }
        }
    }
    Ok(())
}

fn alpha_test_coverage(image: &SpriteMipLevel, alpha_ref: f32, alpha_scale: f32) -> Result<f32> {
    if image.width <= 1 || image.height <= 1 {
        return Ok(f32::NAN);
    }

    let mut coverage = 0.0;
    for y in 0..image.height - 1 {
        for x in 0..image.width - 1 {
            let alpha00 = scaled_alpha(pixel(image, x, y)?[3], alpha_scale);
            let alpha10 = scaled_alpha(pixel(image, x + 1, y)?[3], alpha_scale);
            let alpha01 = scaled_alpha(pixel(image, x, y + 1)?[3], alpha_scale);
            let alpha11 = scaled_alpha(pixel(image, x + 1, y + 1)?[3], alpha_scale);
            let mut texel_coverage = 0.0;

            for subsample_y in 0..4 {
                let fy = (subsample_y as f32 + 0.5) / 4.0;
                for subsample_x in 0..4 {
                    let fx = (subsample_x as f32 + 0.5) / 4.0;
                    let alpha = alpha00 * (1.0 - fx) * (1.0 - fy)
                        + alpha10 * fx * (1.0 - fy)
                        + alpha01 * (1.0 - fx) * fy
                        + alpha11 * fx * fy;
                    if alpha > alpha_ref {
                        texel_coverage += 1.0;
                    }
                }
            }

            coverage += texel_coverage / 16.0;
        }
    }

    Ok(coverage / ((image.width - 1) * (image.height - 1)) as f32)
}

fn scale_alpha_to_coverage(
    image: &mut SpriteMipLevel,
    desired_coverage: f32,
    alpha_ref: f32,
    alpha_cutoff_bias: f32,
) -> Result<()> {
    let mut min_alpha_scale = 0.0;
    let mut max_alpha_scale = 4.0;
    let mut alpha_scale = 1.0;
    let mut best_alpha_scale = 1.0;
    let mut best_error = f32::MAX;

    for _ in 0..5 {
        let current_coverage = alpha_test_coverage(image, alpha_ref, alpha_scale)?;
        let error = (current_coverage - desired_coverage).abs();
        if error < best_error {
            best_error = error;
            best_alpha_scale = alpha_scale;
        }

        if current_coverage < desired_coverage {
            min_alpha_scale = alpha_scale;
        } else {
            if !(current_coverage > desired_coverage) {
                break;
            }
            max_alpha_scale = alpha_scale;
        }

        alpha_scale = (min_alpha_scale + max_alpha_scale) * 0.5;
    }

    for pixel in image.rgba.chunks_exact_mut(4) {
        let alpha = (f32::from(pixel[3]) / 255.0) * best_alpha_scale + alpha_cutoff_bias + 0.025;
        pixel[3] = as_8_bit_channel(alpha.clamp(0.0, 1.0));
    }

    Ok(())
}

fn scaled_alpha(alpha: u8, alpha_scale: f32) -> f32 {
    (f32::from(alpha) / 255.0 * alpha_scale).clamp(0.0, 1.0)
}

fn mean_linear(color1: [u8; 4], color2: [u8; 4], color3: [u8; 4], color4: [u8; 4]) -> [u8; 4] {
    [
        linear_channel_mean(color1[0], color2[0], color3[0], color4[0]),
        linear_channel_mean(color1[1], color2[1], color3[1], color4[1]),
        linear_channel_mean(color1[2], color2[2], color3[2], color4[2]),
        ((u32::from(color1[3])
            + u32::from(color2[3])
            + u32::from(color3[3])
            + u32::from(color4[3]))
            / 4) as u8,
    ]
}

fn linear_channel_mean(c1: u8, c2: u8, c3: u8, c4: u8) -> u8 {
    let srgb_to_linear = srgb_to_linear_lookup();
    let linear_to_srgb = linear_to_srgb_lookup();
    let linear = (u32::from(srgb_to_linear[c1 as usize])
        + u32::from(srgb_to_linear[c2 as usize])
        + u32::from(srgb_to_linear[c3 as usize])
        + u32::from(srgb_to_linear[c4 as usize]))
        / 4;
    linear_to_srgb[linear as usize]
}

fn darkened_alpha_blend(
    color1: [u8; 4],
    color2: [u8; 4],
    color3: [u8; 4],
    color4: [u8; 4],
) -> [u8; 4] {
    let mut alpha_total = 0.0;
    let mut red_total = 0.0;
    let mut green_total = 0.0;
    let mut blue_total = 0.0;
    for color in [color1, color2, color3, color4] {
        if color[3] != 0 {
            alpha_total += srgb_to_linear_channel(color[3]);
            red_total += srgb_to_linear_channel(color[0]);
            green_total += srgb_to_linear_channel(color[1]);
            blue_total += srgb_to_linear_channel(color[2]);
        }
    }

    [
        linear_to_srgb_channel(red_total / 4.0),
        linear_to_srgb_channel(green_total / 4.0),
        linear_to_srgb_channel(blue_total / 4.0),
        linear_to_srgb_channel(alpha_total / 4.0),
    ]
}

fn srgb_to_linear_channel(srgb: u8) -> f32 {
    f32::from(srgb_to_linear_lookup()[srgb as usize]) / 1023.0
}

fn linear_to_srgb_channel(linear: f32) -> u8 {
    let index = (linear * 1023.0).floor().clamp(0.0, 1023.0) as usize;
    linear_to_srgb_lookup()[index]
}

fn srgb_to_linear_lookup() -> &'static [u16; 256] {
    static LOOKUP: OnceLock<[u16; 256]> = OnceLock::new();
    LOOKUP.get_or_init(|| {
        let mut lookup = [0; 256];
        for (i, slot) in lookup.iter_mut().enumerate() {
            let channel = i as f32 / 255.0;
            *slot = (compute_srgb_to_linear(channel) * 1023.0).round() as u16;
        }
        lookup
    })
}

fn linear_to_srgb_lookup() -> &'static [u8; 1024] {
    static LOOKUP: OnceLock<[u8; 1024]> = OnceLock::new();
    LOOKUP.get_or_init(|| {
        let mut lookup = [0; 1024];
        for (i, slot) in lookup.iter_mut().enumerate() {
            let channel = i as f32 / 1023.0;
            *slot = (compute_linear_to_srgb(channel) * 255.0).round() as u8;
        }
        lookup
    })
}

fn compute_srgb_to_linear(x: f32) -> f32 {
    if x >= 0.04045 {
        ((x + 0.055) / 1.055).powf(2.4)
    } else {
        x / 12.92
    }
}

fn compute_linear_to_srgb(x: f32) -> f32 {
    if x >= 0.003_130_8 {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    } else {
        12.92 * x
    }
}

fn as_8_bit_channel(value: f32) -> u8 {
    (value * 255.0).floor() as u8
}

fn pixel(image: &SpriteMipLevel, x: u32, y: u32) -> Result<[u8; 4]> {
    rgba_pixel(&image.rgba, image.width, x, y)
}

fn rgba_pixel(rgba: &[u8], width: u32, x: u32, y: u32) -> Result<[u8; 4]> {
    let offset = rgba_offset(width, x, y)?;
    Ok([
        rgba[offset],
        rgba[offset + 1],
        rgba[offset + 2],
        rgba[offset + 3],
    ])
}

fn set_pixel(rgba: &mut [u8], width: u32, x: u32, y: u32, color: [u8; 4]) -> Result<()> {
    let offset = rgba_offset(width, x, y)?;
    rgba[offset..offset + 4].copy_from_slice(&color);
    Ok(())
}

fn pixel_count(width: u32, height: u32) -> Result<usize> {
    width
        .checked_mul(height)
        .and_then(|pixels| usize::try_from(pixels).ok())
        .ok_or_else(|| anyhow::anyhow!("sprite mipmap pixel count overflow"))
}

fn packed_index(width: u32, x: u32, y: u32) -> Result<usize> {
    y.checked_mul(width)
        .and_then(|row| row.checked_add(x))
        .and_then(|index| usize::try_from(index).ok())
        .ok_or_else(|| anyhow::anyhow!("sprite mipmap pixel index overflow"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SpriteMipmapStrategy, SpriteTextureMetadata};

    #[test]
    fn sprite_mip_levels_use_vanilla_mean_linear_filtering() {
        let mut image = SpriteImage::new(
            "minecraft:block/mean",
            2,
            2,
            vec![
                0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            ],
        )
        .unwrap();
        image.texture_metadata.mipmap_strategy = SpriteMipmapStrategy::Mean;

        let levels = image.mip_levels(1).unwrap();

        assert_eq!(levels.len(), 2);
        assert_eq!((levels[1].width, levels[1].height), (1, 1));
        assert_eq!(levels[1].rgba, vec![225, 225, 225, 255]);
    }

    #[test]
    fn sprite_mip_levels_auto_cutout_solidifies_non_item_base() {
        let image = SpriteImage::new(
            "minecraft:block/cutout",
            2,
            2,
            vec![10, 20, 30, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        )
        .unwrap();

        let levels = image.mip_levels(0).unwrap();

        assert_eq!(
            levels[0].rgba,
            vec![10, 20, 30, 255, 10, 20, 30, 0, 10, 20, 30, 0, 10, 20, 30, 0]
        );
    }

    #[test]
    fn sprite_mip_levels_keep_item_cutout_base_unmodified() {
        let image = SpriteImage::new(
            "minecraft:item/cutout",
            2,
            2,
            vec![10, 20, 30, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        )
        .unwrap();

        let levels = image.mip_levels(0).unwrap();

        assert_eq!(levels[0].rgba, image.rgba);
    }

    #[test]
    fn sprite_mip_levels_dark_cutout_fills_empty_base_with_darkest_color() {
        let mut image = SpriteImage::new(
            "minecraft:block/dark_cutout",
            2,
            2,
            vec![100, 80, 60, 255, 30, 30, 30, 255, 0, 0, 0, 0, 0, 0, 0, 0],
        )
        .unwrap();
        image.texture_metadata = SpriteTextureMetadata {
            mipmap_strategy: SpriteMipmapStrategy::DarkCutout,
            ..SpriteTextureMetadata::default()
        };

        let levels = image.mip_levels(0).unwrap();

        assert_eq!(
            levels[0].rgba,
            vec![100, 80, 60, 255, 30, 30, 30, 255, 22, 22, 22, 0, 22, 22, 22, 0]
        );
    }

    #[test]
    fn sprite_mip_levels_reject_zero_sized_requested_level() {
        let image =
            SpriteImage::new("minecraft:block/tiny", 1, 1, vec![255, 255, 255, 255]).unwrap();

        let err = image.mip_levels(1).unwrap_err();

        assert!(err.to_string().contains("exceeds base dimensions 1x1"));
    }
}
