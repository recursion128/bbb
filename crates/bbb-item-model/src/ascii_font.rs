use anyhow::{bail, Context, Result};
use bbb_pack::{PackRoots, ResourceLocation, SpriteImage};
use bbb_render_types::{
    HudAsciiGlyph, HudDigitGlyph, HudUvRect, HUD_ASCII_FIRST_GLYPH, HUD_ASCII_GLYPH_COUNT,
};

const ASCII_FONT_GRID_COLUMNS: u32 = 16;
const ASCII_FONT_GRID_ROWS: u32 = 16;
const ASCII_DIGIT_ROW: u32 = 3;
const HUD_DIGIT_COUNT: u32 = 10;

#[derive(Debug, Clone, PartialEq)]
pub struct HudDigitAtlasImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub glyphs: [HudDigitGlyph; 10],
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudAsciiAtlasImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub glyphs: [HudAsciiGlyph; HUD_ASCII_GLYPH_COUNT],
}

pub fn load_ascii_font_texture(roots: &PackRoots) -> Result<SpriteImage> {
    let location = ResourceLocation::parse("textures/font/ascii.png")?;
    let resource = roots
        .resource_stack()
        .get_resource(&location)
        .context("missing ASCII font texture minecraft:textures/font/ascii.png")?;
    SpriteImage::from_png_file("minecraft:textures/font/ascii", resource.path)
}

pub fn hud_ascii_digit_atlas_from_image(image: &SpriteImage) -> Result<HudDigitAtlasImage> {
    let glyph_width = image.width / ASCII_FONT_GRID_COLUMNS;
    let glyph_height = image.height / ASCII_FONT_GRID_ROWS;
    if glyph_width == 0 || glyph_height == 0 {
        bail!("ascii font texture must contain a non-empty 16x16 glyph grid");
    }

    let width = HUD_DIGIT_COUNT * glyph_width;
    let height = glyph_height;
    let mut rgba = vec![0; rgba_len(width, height)?];
    let mut glyphs = [HudDigitGlyph::default(); 10];

    for digit in 0..HUD_DIGIT_COUNT {
        let src_x = digit * glyph_width;
        let src_y = ASCII_DIGIT_ROW * glyph_height;
        let dst_x = digit * glyph_width;
        copy_ascii_glyph(
            image,
            &mut rgba,
            width,
            dst_x,
            src_x,
            src_y,
            glyph_width,
            glyph_height,
        )?;
        let advance = ascii_glyph_actual_width(image, src_x, src_y, glyph_width, glyph_height) + 1;
        glyphs[digit as usize] = HudDigitGlyph {
            uv: HudUvRect {
                min: [dst_x as f32 / width as f32, 0.0],
                max: [(dst_x + glyph_width) as f32 / width as f32, 1.0],
            },
            width: glyph_width,
            height: glyph_height,
            advance,
        };
    }

    Ok(HudDigitAtlasImage {
        width,
        height,
        rgba,
        glyphs,
    })
}

pub fn hud_ascii_atlas_from_image(image: &SpriteImage) -> Result<HudAsciiAtlasImage> {
    let glyph_width = image.width / ASCII_FONT_GRID_COLUMNS;
    let glyph_height = image.height / ASCII_FONT_GRID_ROWS;
    if glyph_width == 0 || glyph_height == 0 {
        bail!("ascii font texture must contain a non-empty 16x16 glyph grid");
    }

    let glyph_count = u32::try_from(HUD_ASCII_GLYPH_COUNT).context("ASCII glyph count overflow")?;
    let width = glyph_count
        .checked_mul(glyph_width)
        .context("ASCII atlas width overflow")?;
    let height = glyph_height;
    let mut rgba = vec![0; rgba_len(width, height)?];
    let mut glyphs = [HudAsciiGlyph::default(); HUD_ASCII_GLYPH_COUNT];

    for (index, glyph) in glyphs.iter_mut().enumerate() {
        let byte = HUD_ASCII_FIRST_GLYPH
            .checked_add(u8::try_from(index).context("ASCII glyph index overflow")?)
            .context("ASCII glyph byte overflow")?;
        let src_index = u32::from(byte);
        let src_x = (src_index % ASCII_FONT_GRID_COLUMNS) * glyph_width;
        let src_y = (src_index / ASCII_FONT_GRID_COLUMNS) * glyph_height;
        let dst_x = u32::try_from(index)
            .context("ASCII glyph destination index overflow")?
            .checked_mul(glyph_width)
            .context("ASCII glyph destination x overflow")?;
        copy_ascii_glyph(
            image,
            &mut rgba,
            width,
            dst_x,
            src_x,
            src_y,
            glyph_width,
            glyph_height,
        )?;
        let advance = ascii_glyph_advance(image, src_x, src_y, glyph_width, glyph_height, byte);
        *glyph = HudAsciiGlyph {
            uv: HudUvRect {
                min: [dst_x as f32 / width as f32, 0.0],
                max: [(dst_x + glyph_width) as f32 / width as f32, 1.0],
            },
            width: glyph_width,
            height: glyph_height,
            advance,
        };
    }

    Ok(HudAsciiAtlasImage {
        width,
        height,
        rgba,
        glyphs,
    })
}

fn copy_ascii_glyph(
    image: &SpriteImage,
    dst: &mut [u8],
    dst_width: u32,
    dst_x: u32,
    src_x: u32,
    src_y: u32,
    glyph_width: u32,
    glyph_height: u32,
) -> Result<()> {
    for y in 0..glyph_height {
        for x in 0..glyph_width {
            let src_offset = rgba_offset(image.width, src_x + x, src_y + y)?;
            let dst_offset = rgba_offset(dst_width, dst_x + x, y)?;
            copy_rgba_pixel(&image.rgba, src_offset, dst, dst_offset)?;
        }
    }
    Ok(())
}

fn copy_rgba_pixel(
    src_rgba: &[u8],
    src_offset: usize,
    dst_rgba: &mut [u8],
    dst_offset: usize,
) -> Result<()> {
    let src_end = src_offset
        .checked_add(4)
        .filter(|end| *end <= src_rgba.len())
        .context("ascii font source pixel is outside image")?;
    let dst_end = dst_offset
        .checked_add(4)
        .filter(|end| *end <= dst_rgba.len())
        .context("ascii atlas destination pixel is outside image")?;
    dst_rgba[dst_offset..dst_end].copy_from_slice(&src_rgba[src_offset..src_end]);
    Ok(())
}

fn ascii_glyph_advance(
    image: &SpriteImage,
    src_x: u32,
    src_y: u32,
    glyph_width: u32,
    glyph_height: u32,
    byte: u8,
) -> u32 {
    if byte == b' ' {
        return 4u32.min(glyph_width).max(1);
    }
    ascii_glyph_actual_width(image, src_x, src_y, glyph_width, glyph_height) + 1
}

fn ascii_glyph_actual_width(
    image: &SpriteImage,
    src_x: u32,
    src_y: u32,
    glyph_width: u32,
    glyph_height: u32,
) -> u32 {
    for x in (0..glyph_width).rev() {
        for y in 0..glyph_height {
            if ascii_font_pixel_visible(image, src_x + x, src_y + y) {
                return x + 1;
            }
        }
    }
    0
}

fn ascii_font_pixel_visible(image: &SpriteImage, x: u32, y: u32) -> bool {
    rgba_offset(image.width, x, y)
        .ok()
        .and_then(|offset| image.rgba.get(offset..offset + 4))
        .is_some_and(|pixel| pixel[3] != 0 || pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
}

fn rgba_len(width: u32, height: u32) -> Result<usize> {
    usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .context("atlas RGBA size overflow")
}

fn rgba_offset(width: u32, x: u32, y: u32) -> Result<usize> {
    y.checked_mul(width)
        .and_then(|row| row.checked_add(x))
        .and_then(|pixel| pixel.checked_mul(4))
        .and_then(|offset| usize::try_from(offset).ok())
        .context("RGBA pixel offset overflow")
}
