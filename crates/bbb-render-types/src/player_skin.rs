use std::io::Cursor;

use anyhow::{bail, ensure, Context, Result};
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};

use crate::StandaloneTextureSampling;

const SKIN_WIDTH: u32 = 64;
const SKIN_HEIGHT: u32 = 64;
const LEGACY_SKIN_HEIGHT: u32 = 32;
const PIXEL_BYTES: usize = 4;
pub const MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES: usize = 1024 * 1024;
pub const MAX_DYNAMIC_PLAYER_TEXTURE_PNG_BYTES: usize = 16 * 1024 * 1024;
const MAX_DYNAMIC_PLAYER_TEXTURE_DIMENSION: u32 = 8_192;
const MAX_DYNAMIC_PLAYER_TEXTURE_PIXELS: u64 = 16 * 1024 * 1024;
const MAX_DYNAMIC_PLAYER_TEXTURE_DECODE_BYTES: u64 = 128 * 1024 * 1024;

/// A player skin after vanilla-compatible size validation and legacy-layout repair.
///
/// The image is always a 64x64 RGBA skin texture ready for a future dynamic texture upload.
/// Downloaded skins have no pack sampling contract; local ResourceTexture body patches retain one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicPlayerSkinImage {
    pub handle: u64,
    pub standalone_sampling: Option<StandaloneTextureSampling>,
    pub rgba: Vec<u8>,
}

impl DynamicPlayerSkinImage {
    pub const SIZE: [u32; 2] = [SKIN_WIDTH, SKIN_HEIGHT];
}

/// A variable-size entity texture decoded or loaded for runtime atlas upload.
///
/// This preserves the decoded dimensions and RGBA pixels unchanged. Player cape/elytra downloads
/// and pack-backed registry variant assets share this renderer boundary. Pack standalone resources
/// retain their independent `.png.mcmeta` sampler contract; atlas-owned and downloaded images leave
/// it absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicEntityTextureImage {
    pub handle: u64,
    pub size: [u32; 2],
    pub standalone_sampling: Option<StandaloneTextureSampling>,
    pub rgba: Vec<u8>,
}

/// Backward-compatible name for downloaded cape/elytra images.
pub type DynamicPlayerTextureImage = DynamicEntityTextureImage;

/// Decode and process a downloaded player skin PNG.
///
/// Mirrors vanilla 26.1 `SkinTextureDownloader.processLegacySkin`: only 64x64 and 64x32 skins are
/// accepted, 64x32 skins are expanded to the current 64x64 layout, and vanilla's opaque-base /
/// legacy-overlay alpha rules are applied.
pub fn decode_dynamic_player_skin_png(handle: u64, png: &[u8]) -> Result<DynamicPlayerSkinImage> {
    let (width, height) = png_ihdr_dimensions(
        png,
        MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES,
        "player skin texture",
        "guess player skin image format",
    )?;
    if width != SKIN_WIDTH || (height != SKIN_HEIGHT && height != LEGACY_SKIN_HEIGHT) {
        bail!("discarding incorrectly sized player skin texture {width}x{height}");
    }
    let image = decode_dynamic_png_rgba(
        png,
        (width, height),
        [SKIN_WIDTH, SKIN_HEIGHT],
        u64::from(SKIN_WIDTH) * u64::from(SKIN_HEIGHT),
        "player skin texture",
    )
    .context("decode player skin png")?;
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
    let dimensions = png_ihdr_dimensions(
        png,
        MAX_DYNAMIC_PLAYER_TEXTURE_PNG_BYTES,
        "player profile texture",
        "guess player profile texture image format",
    )?;
    let image = decode_dynamic_png_rgba(
        png,
        dimensions,
        [
            MAX_DYNAMIC_PLAYER_TEXTURE_DIMENSION,
            MAX_DYNAMIC_PLAYER_TEXTURE_DIMENSION,
        ],
        MAX_DYNAMIC_PLAYER_TEXTURE_PIXELS,
        "player profile texture",
    )
    .context("decode player profile texture png")?;
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
    Ok(DynamicEntityTextureImage {
        handle,
        size: [width, height],
        standalone_sampling: None,
        rgba,
    })
}

fn png_ihdr_dimensions(
    png: &[u8],
    max_png_bytes: usize,
    label: &str,
    format_context: &str,
) -> Result<(u32, u32)> {
    if png.len() > max_png_bytes {
        bail!(
            "{label} PNG is {} bytes, exceeding the {max_png_bytes} byte limit",
            png.len()
        );
    }
    let format = image::guess_format(png).context(format_context.to_string())?;
    ensure!(format == ImageFormat::Png, "{label} is not a PNG");
    let header = png
        .get(..24)
        .ok_or_else(|| anyhow::anyhow!("{label} has a truncated PNG IHDR"))?;
    ensure!(
        header[8..12] == 13_u32.to_be_bytes() && header[12..16] == *b"IHDR",
        "{label} has an invalid PNG IHDR"
    );
    Ok((
        u32::from_be_bytes(header[16..20].try_into().expect("fixed PNG width slice")),
        u32::from_be_bytes(header[20..24].try_into().expect("fixed PNG height slice")),
    ))
}

fn decode_dynamic_png_rgba(
    png: &[u8],
    header_dimensions: (u32, u32),
    max_dimensions: [u32; 2],
    max_pixels: u64,
    label: &str,
) -> Result<image::RgbaImage> {
    let (header_width, header_height) = header_dimensions;
    if header_width == 0 || header_height == 0 {
        bail!("{label} must not be empty");
    }
    if header_width > max_dimensions[0] || header_height > max_dimensions[1] {
        bail!(
            "{label} is {header_width}x{header_height}, exceeding the {}x{} dimension limit",
            max_dimensions[0],
            max_dimensions[1]
        );
    }
    let pixels = u64::from(header_width)
        .checked_mul(u64::from(header_height))
        .ok_or_else(|| anyhow::anyhow!("{label} pixel count overflow"))?;
    if pixels > max_pixels {
        bail!("{label} has {pixels} pixels, exceeding the {max_pixels} pixel limit");
    }

    let mut reader = ImageReader::with_format(Cursor::new(png), ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(max_dimensions[0]);
    limits.max_image_height = Some(max_dimensions[1]);
    limits.max_alloc = Some(MAX_DYNAMIC_PLAYER_TEXTURE_DECODE_BYTES);
    reader.limits(limits);
    let decoder = reader
        .into_decoder()
        .with_context(|| format!("read {label} PNG header"))?;
    if decoder.dimensions() != (header_width, header_height) {
        bail!("{label} PNG dimensions changed while being read");
    }
    let decoded_bytes = decoder.total_bytes();
    if decoded_bytes > MAX_DYNAMIC_PLAYER_TEXTURE_DECODE_BYTES {
        bail!(
            "{label} requires {decoded_bytes} decoded bytes, exceeding the {} byte limit",
            MAX_DYNAMIC_PLAYER_TEXTURE_DECODE_BYTES
        );
    }
    DynamicImage::from_decoder(decoder)
        .map(DynamicImage::into_rgba8)
        .map_err(Into::into)
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
        standalone_sampling: None,
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
        assert_eq!(skin.standalone_sampling, None);
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
        assert_eq!(texture.standalone_sampling, None);
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

    #[test]
    fn rejects_dynamic_png_byte_budget_before_format_or_decode() {
        let oversized_skin = vec![0_u8; MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES + 1];
        let skin_err = decode_dynamic_player_skin_png(7, &oversized_skin)
            .unwrap_err()
            .to_string();
        assert!(
            skin_err.contains("exceeding the 1048576 byte limit"),
            "{skin_err}"
        );

        let oversized_texture = vec![0_u8; MAX_DYNAMIC_PLAYER_TEXTURE_PNG_BYTES + 1];
        let texture_err = decode_dynamic_player_texture_png(7, &oversized_texture)
            .unwrap_err()
            .to_string();
        assert!(
            texture_err.contains("exceeding the 16777216 byte limit"),
            "{texture_err}"
        );
    }

    #[test]
    fn rejects_profile_texture_pixel_budget_from_ihdr_before_decode() {
        let mut png = rgba_png(1, 1, |_, _| [1, 2, 3, 255]);
        png[16..20].copy_from_slice(&4_096_u32.to_be_bytes());
        png[20..24].copy_from_slice(&4_097_u32.to_be_bytes());

        let err = format!(
            "{:#}",
            decode_dynamic_player_texture_png(7, &png).unwrap_err()
        );
        assert!(err.contains("exceeding the 16777216 pixel limit"), "{err}");
    }

    #[test]
    fn dynamic_texture_decoders_do_not_restore_unbounded_image_helpers() {
        let production = include_str!("player_skin.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("player skin production source");
        assert!(!production.contains("load_from_memory"));
        assert!(!production.contains("image::open"));
        assert!(production.contains("reader.limits(limits)"));
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
