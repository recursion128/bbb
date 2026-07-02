use anyhow::{bail, ensure, Context, Result};

const SKIN_WIDTH: u32 = 64;
const SKIN_HEIGHT: u32 = 64;
const LEGACY_SKIN_HEIGHT: u32 = 32;
const PIXEL_BYTES: usize = 4;

/// A downloaded player skin after vanilla-compatible size validation and legacy-layout repair.
///
/// The image is always a 64x64 RGBA skin texture ready for a future dynamic texture upload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicPlayerSkinImage {
    pub handle: u64,
    pub rgba: Vec<u8>,
}

impl DynamicPlayerSkinImage {
    pub const SIZE: [u32; 2] = [SKIN_WIDTH, SKIN_HEIGHT];
}

/// A downloaded non-skin player-profile texture, such as a cape or elytra texture.
///
/// Unlike body skins, vanilla 26.1 registers cape/elytra textures without legacy skin repair, so this
/// type preserves the decoded PNG dimensions and RGBA pixels unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicPlayerTextureImage {
    pub handle: u64,
    pub size: [u32; 2],
    pub rgba: Vec<u8>,
}

/// Decode and process a downloaded player skin PNG.
///
/// Mirrors vanilla 26.1 `SkinTextureDownloader.processLegacySkin`: only 64x64 and 64x32 skins are
/// accepted, 64x32 skins are expanded to the current 64x64 layout, and vanilla's opaque-base /
/// legacy-overlay alpha rules are applied.
pub fn decode_dynamic_player_skin_png(handle: u64, png: &[u8]) -> Result<DynamicPlayerSkinImage> {
    let format = image::guess_format(png).context("guess player skin image format")?;
    ensure!(
        format == image::ImageFormat::Png,
        "player skin texture is not a PNG"
    );
    let image = image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .context("decode player skin png")?
        .to_rgba8();
    process_dynamic_player_skin_rgba(handle, image.width(), image.height(), image.into_raw())
}

/// Decode a downloaded cape/elytra profile texture PNG.
///
/// Vanilla `SkinManager.TextureCache` only asks `SkinTextureDownloader` to process legacy layout when
/// loading `Type.SKIN`; cape and elytra textures are normal PNG-backed client assets.
pub fn decode_dynamic_player_texture_png(
    handle: u64,
    png: &[u8],
) -> Result<DynamicPlayerTextureImage> {
    let format = image::guess_format(png).context("guess player profile texture image format")?;
    ensure!(
        format == image::ImageFormat::Png,
        "player profile texture is not a PNG"
    );
    let image = image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .context("decode player profile texture png")?
        .to_rgba8();
    let width = image.width();
    let height = image.height();
    let rgba = image.into_raw();
    let expected_len = rgba_len(width, height, "player profile texture")?;
    if rgba.len() != expected_len {
        bail!(
            "player profile texture has {} RGBA bytes, expected {} for {}x{}",
            rgba.len(),
            expected_len,
            width,
            height
        );
    }
    Ok(DynamicPlayerTextureImage {
        handle,
        size: [width, height],
        rgba,
    })
}

fn process_dynamic_player_skin_rgba(
    handle: u64,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
) -> Result<DynamicPlayerSkinImage> {
    if width != SKIN_WIDTH || (height != SKIN_HEIGHT && height != LEGACY_SKIN_HEIGHT) {
        bail!("discarding incorrectly sized player skin texture {width}x{height}");
    }
    let expected_len = rgba_len(width, height, "player skin texture")?;
    if rgba.len() != expected_len {
        bail!(
            "player skin texture has {} RGBA bytes, expected {} for {}x{}",
            rgba.len(),
            expected_len,
            width,
            height
        );
    }

    let legacy = height == LEGACY_SKIN_HEIGHT;
    let mut image = if legacy {
        let mut converted = vec![0u8; rgba_len(SKIN_WIDTH, SKIN_HEIGHT, "converted player skin")?];
        copy_region(
            &rgba,
            width,
            &mut converted,
            SKIN_WIDTH,
            RectCopy {
                source_x: 0,
                source_y: 0,
                target_x: 0,
                target_y: 0,
                width,
                height,
                swap_x: false,
                swap_y: false,
            },
        );
        copy_legacy_skin_regions(&mut converted);
        converted
    } else {
        rgba
    };

    set_no_alpha(&mut image, 0, 0, 32, 16);
    if legacy {
        apply_legacy_overlay_alpha(&mut image);
    }
    set_no_alpha(&mut image, 0, 16, 64, 32);
    set_no_alpha(&mut image, 16, 48, 48, 64);

    Ok(DynamicPlayerSkinImage {
        handle,
        rgba: image,
    })
}

fn copy_legacy_skin_regions(image: &mut [u8]) {
    for copy in [
        RectCopy::offset(4, 16, 16, 32, 4, 4, true, false),
        RectCopy::offset(8, 16, 16, 32, 4, 4, true, false),
        RectCopy::offset(0, 20, 24, 32, 4, 12, true, false),
        RectCopy::offset(4, 20, 16, 32, 4, 12, true, false),
        RectCopy::offset(8, 20, 8, 32, 4, 12, true, false),
        RectCopy::offset(12, 20, 16, 32, 4, 12, true, false),
        RectCopy::offset(44, 16, -8, 32, 4, 4, true, false),
        RectCopy::offset(48, 16, -8, 32, 4, 4, true, false),
        RectCopy::offset(40, 20, 0, 32, 4, 12, true, false),
        RectCopy::offset(44, 20, -8, 32, 4, 12, true, false),
        RectCopy::offset(48, 20, -16, 32, 4, 12, true, false),
        RectCopy::offset(52, 20, -8, 32, 4, 12, true, false),
    ] {
        copy_region_in_place(image, SKIN_WIDTH, copy);
    }
}

#[derive(Debug, Clone, Copy)]
struct RectCopy {
    source_x: u32,
    source_y: u32,
    target_x: u32,
    target_y: u32,
    width: u32,
    height: u32,
    swap_x: bool,
    swap_y: bool,
}

impl RectCopy {
    fn offset(
        source_x: u32,
        source_y: u32,
        offset_x: i32,
        offset_y: i32,
        width: u32,
        height: u32,
        swap_x: bool,
        swap_y: bool,
    ) -> Self {
        Self {
            source_x,
            source_y,
            target_x: add_offset(source_x, offset_x),
            target_y: add_offset(source_y, offset_y),
            width,
            height,
            swap_x,
            swap_y,
        }
    }
}

fn add_offset(value: u32, offset: i32) -> u32 {
    if offset >= 0 {
        value + offset as u32
    } else {
        value - offset.unsigned_abs()
    }
}

fn copy_region(
    source: &[u8],
    source_width: u32,
    target: &mut [u8],
    target_width: u32,
    copy: RectCopy,
) {
    for y in 0..copy.height {
        for x in 0..copy.width {
            let source_offset = rgba_offset(source_width, copy.source_x + x, copy.source_y + y);
            let target_x = if copy.swap_x { copy.width - 1 - x } else { x };
            let target_y = if copy.swap_y { copy.height - 1 - y } else { y };
            let target_offset = rgba_offset(
                target_width,
                copy.target_x + target_x,
                copy.target_y + target_y,
            );
            target[target_offset..target_offset + PIXEL_BYTES]
                .copy_from_slice(&source[source_offset..source_offset + PIXEL_BYTES]);
        }
    }
}

fn copy_region_in_place(image: &mut [u8], width: u32, copy: RectCopy) {
    let mut scratch = Vec::with_capacity(usize::try_from(copy.width * copy.height * 4).unwrap());
    for y in 0..copy.height {
        for x in 0..copy.width {
            let source_offset = rgba_offset(width, copy.source_x + x, copy.source_y + y);
            scratch.extend_from_slice(&image[source_offset..source_offset + PIXEL_BYTES]);
        }
    }
    for y in 0..copy.height {
        for x in 0..copy.width {
            let source_offset = usize::try_from((x + y * copy.width) * PIXEL_BYTES as u32).unwrap();
            let target_x = if copy.swap_x { copy.width - 1 - x } else { x };
            let target_y = if copy.swap_y { copy.height - 1 - y } else { y };
            let target_offset =
                rgba_offset(width, copy.target_x + target_x, copy.target_y + target_y);
            image[target_offset..target_offset + PIXEL_BYTES]
                .copy_from_slice(&scratch[source_offset..source_offset + PIXEL_BYTES]);
        }
    }
}

fn set_no_alpha(image: &mut [u8], x0: u32, y0: u32, x1: u32, y1: u32) {
    for y in y0..y1 {
        for x in x0..x1 {
            image[rgba_offset(SKIN_WIDTH, x, y) + 3] = 255;
        }
    }
}

fn apply_legacy_overlay_alpha(image: &mut [u8]) {
    for y in 0..32 {
        for x in 32..64 {
            if image[rgba_offset(SKIN_WIDTH, x, y) + 3] < 128 {
                return;
            }
        }
    }

    for y in 0..32 {
        for x in 32..64 {
            image[rgba_offset(SKIN_WIDTH, x, y) + 3] = 0;
        }
    }
}

fn rgba_len(width: u32, height: u32, label: &str) -> Result<usize> {
    usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(PIXEL_BYTES))
        .ok_or_else(|| anyhow::anyhow!("{label} RGBA size overflow"))
}

fn rgba_offset(width: u32, x: u32, y: u32) -> usize {
    usize::try_from((x + y * width) * PIXEL_BYTES as u32).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn rejects_non_png_and_incorrect_skin_sizes() {
        let err = decode_dynamic_player_skin_png(7, b"not a png").unwrap_err();
        assert!(err.to_string().contains("guess player skin image format"));

        let png = rgba_png(32, 32, |x, y| [x as u8, y as u8, 0, 255]);
        let err = decode_dynamic_player_skin_png(7, &png).unwrap_err();
        assert!(err
            .to_string()
            .contains("discarding incorrectly sized player skin texture 32x32"));
    }

    #[test]
    fn accepts_64_by_64_skin_and_applies_vanilla_opaque_regions() {
        let mut raw = patterned_rgba(64, 64);
        raw[rgba_offset(64, 2, 2) + 3] = 1;
        raw[rgba_offset(64, 40, 8) + 3] = 2;
        raw[rgba_offset(64, 8, 24) + 3] = 3;
        raw[rgba_offset(64, 20, 50) + 3] = 4;
        raw[rgba_offset(64, 2, 40) + 3] = 5;

        let skin = process_dynamic_player_skin_rgba(99, 64, 64, raw).unwrap();

        assert_eq!(skin.handle, 99);
        assert_eq!(skin.rgba.len(), 64 * 64 * 4);
        assert_eq!(alpha(&skin.rgba, 2, 2), 255);
        assert_eq!(alpha(&skin.rgba, 40, 8), 2);
        assert_eq!(alpha(&skin.rgba, 8, 24), 255);
        assert_eq!(alpha(&skin.rgba, 20, 50), 255);
        assert_eq!(alpha(&skin.rgba, 2, 40), 5);
    }

    #[test]
    fn expands_legacy_64_by_32_skin_with_vanilla_copy_rects() {
        let raw = patterned_rgba(64, 32);
        let skin = process_dynamic_player_skin_rgba(123, 64, 32, raw.clone()).unwrap();

        assert_eq!(skin.rgba.len(), 64 * 64 * 4);
        assert_eq!(rgb(&skin.rgba, 23, 48), rgb(&raw, 4, 16));
        assert_eq!(rgb(&skin.rgba, 20, 48), rgb(&raw, 7, 16));
        assert_eq!(rgb(&skin.rgba, 39, 48), rgb(&raw, 44, 16));
        assert_eq!(rgb(&skin.rgba, 36, 48), rgb(&raw, 47, 16));
        assert_eq!(rgb(&skin.rgba, 40, 52), rgb(&raw, 43, 20));
        assert_eq!(rgb(&skin.rgba, 47, 52), rgb(&raw, 52, 20));
        assert_eq!(alpha(&skin.rgba, 23, 48), 255);
        assert_eq!(alpha(&skin.rgba, 47, 52), 255);
    }

    #[test]
    fn legacy_skin_applies_notch_transparency_hack_before_base_alpha_regions() {
        let mut raw = patterned_rgba(64, 32);
        for y in 0..32 {
            for x in 32..64 {
                raw[rgba_offset(64, x, y) + 3] = 255;
            }
        }

        let skin = process_dynamic_player_skin_rgba(1, 64, 32, raw).unwrap();

        assert_eq!(alpha(&skin.rgba, 40, 8), 0);
        assert_eq!(alpha(&skin.rgba, 40, 24), 255);
        assert_eq!(alpha(&skin.rgba, 8, 8), 255);
        assert_eq!(alpha(&skin.rgba, 20, 50), 255);
    }

    #[test]
    fn legacy_skin_keeps_overlay_alpha_when_any_overlay_pixel_is_transparent() {
        let mut raw = patterned_rgba(64, 32);
        for y in 0..32 {
            for x in 32..64 {
                raw[rgba_offset(64, x, y) + 3] = 255;
            }
        }
        raw[rgba_offset(64, 40, 8) + 3] = 12;

        let skin = process_dynamic_player_skin_rgba(1, 64, 32, raw).unwrap();

        assert_eq!(alpha(&skin.rgba, 40, 8), 12);
    }

    #[test]
    fn decodes_png_and_returns_converted_skin() {
        let png = rgba_png(64, 32, |x, y| [x as u8, y as u8, 99, 255]);

        let skin = decode_dynamic_player_skin_png(77, &png).unwrap();

        assert_eq!(skin.handle, 77);
        assert_eq!(skin.rgba.len(), 64 * 64 * 4);
        assert_eq!(pixel(&skin.rgba, 0, 0), [0, 0, 99, 255]);
    }

    #[test]
    fn decodes_profile_texture_png_without_skin_post_processing() {
        let png = rgba_png(64, 32, |x, y| {
            [x as u8, y as u8, 44, x.wrapping_add(y) as u8]
        });

        let texture = decode_dynamic_player_texture_png(55, &png).unwrap();

        assert_eq!(texture.handle, 55);
        assert_eq!(texture.size, [64, 32]);
        assert_eq!(texture.rgba.len(), 64 * 32 * 4);
        assert_eq!(pixel_with_width(&texture.rgba, 64, 2, 2), [2, 2, 44, 4]);
    }

    #[test]
    fn rejects_non_png_profile_texture() {
        let err = decode_dynamic_player_texture_png(7, b"not a png").unwrap_err();
        assert!(err
            .to_string()
            .contains("guess player profile texture image format"));
    }

    fn patterned_rgba(width: u32, height: u32) -> Vec<u8> {
        let mut rgba = vec![0; usize::try_from(width * height * 4).unwrap()];
        for y in 0..height {
            for x in 0..width {
                let offset = rgba_offset(width, x, y);
                rgba[offset..offset + 4].copy_from_slice(&[
                    x as u8,
                    y as u8,
                    x.wrapping_add(y) as u8,
                    200,
                ]);
            }
        }
        rgba
    }

    fn rgba_png(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
        let mut image = image::RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                image.put_pixel(x, y, image::Rgba(pixel(x, y)));
            }
        }
        let mut cursor = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .unwrap();
        cursor.into_inner()
    }

    fn pixel(rgba: &[u8], x: u32, y: u32) -> [u8; 4] {
        pixel_with_width(rgba, 64, x, y)
    }

    fn pixel_with_width(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let offset = rgba_offset(width, x, y);
        rgba[offset..offset + 4].try_into().unwrap()
    }

    fn rgb(rgba: &[u8], x: u32, y: u32) -> [u8; 3] {
        let offset = rgba_offset(64, x, y);
        rgba[offset..offset + 3].try_into().unwrap()
    }

    fn alpha(rgba: &[u8], x: u32, y: u32) -> u8 {
        rgba[rgba_offset(64, x, y) + 3]
    }
}
