use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use bbb_pack::{PackRoots, ResourceLocation, SpriteImage};
use bbb_renderer::{HudDigitGlyph, HudUvRect};

const ASCII_FONT_GRID_COLUMNS: u32 = 16;
const ASCII_FONT_GRID_ROWS: u32 = 16;
const ASCII_DIGIT_ROW: u32 = 3;
const HUD_DIGIT_COUNT: u32 = 10;

pub(crate) fn load_hud_textures(renderer: &mut bbb_renderer::Renderer, roots: Option<&PackRoots>) {
    let Some(roots) = roots else {
        tracing::warn!("continuing without vanilla HUD sprites because pack roots are unavailable");
        return;
    };
    if let Err(err) = try_load_hud_textures(renderer, roots) {
        tracing::warn!(?err, "continuing without vanilla HUD sprites");
    }
}

fn try_load_hud_textures(renderer: &mut bbb_renderer::Renderer, roots: &PackRoots) -> Result<()> {
    let sprites = load_gui_sprites(roots)?;
    let crosshair = hud_sprite(&sprites, "hud/crosshair")?;
    renderer.upload_hud_crosshair(crosshair.width, crosshair.height, &crosshair.rgba)?;
    let hotbar = hud_sprite(&sprites, "hud/hotbar")?;
    renderer.upload_hud_hotbar(hotbar.width, hotbar.height, &hotbar.rgba)?;
    let hotbar_selection = hud_sprite(&sprites, "hud/hotbar_selection")?;
    renderer.upload_hud_hotbar_selection(
        hotbar_selection.width,
        hotbar_selection.height,
        &hotbar_selection.rgba,
    )?;
    let inventory = gui_texture(
        roots,
        "textures/gui/container/inventory.png",
        "minecraft:textures/gui/container/inventory",
    )?;
    renderer.upload_hud_inventory_background(inventory.width, inventory.height, &inventory.rgba)?;
    let generic_container = gui_texture(
        roots,
        "textures/gui/container/generic_54.png",
        "minecraft:textures/gui/container/generic_54",
    )?;
    renderer.upload_hud_generic_container_background(
        generic_container.width,
        generic_container.height,
        &generic_container.rgba,
    )?;
    let dispenser = gui_texture(
        roots,
        "textures/gui/container/dispenser.png",
        "minecraft:textures/gui/container/dispenser",
    )?;
    renderer.upload_hud_dispenser_background(dispenser.width, dispenser.height, &dispenser.rgba)?;
    let crafting_table = gui_texture(
        roots,
        "textures/gui/container/crafting_table.png",
        "minecraft:textures/gui/container/crafting_table",
    )?;
    renderer.upload_hud_crafting_table_background(
        crafting_table.width,
        crafting_table.height,
        &crafting_table.rgba,
    )?;
    let furnace = gui_texture(
        roots,
        "textures/gui/container/furnace.png",
        "minecraft:textures/gui/container/furnace",
    )?;
    renderer.upload_hud_furnace_background(furnace.width, furnace.height, &furnace.rgba)?;
    let furnace_lit_progress = hud_sprite(&sprites, "container/furnace/lit_progress")?;
    renderer.upload_hud_furnace_lit_progress(
        furnace_lit_progress.width,
        furnace_lit_progress.height,
        &furnace_lit_progress.rgba,
    )?;
    let furnace_burn_progress = hud_sprite(&sprites, "container/furnace/burn_progress")?;
    renderer.upload_hud_furnace_burn_progress(
        furnace_burn_progress.width,
        furnace_burn_progress.height,
        &furnace_burn_progress.rgba,
    )?;
    let blast_furnace = gui_texture(
        roots,
        "textures/gui/container/blast_furnace.png",
        "minecraft:textures/gui/container/blast_furnace",
    )?;
    renderer.upload_hud_blast_furnace_background(
        blast_furnace.width,
        blast_furnace.height,
        &blast_furnace.rgba,
    )?;
    let blast_furnace_lit_progress = hud_sprite(&sprites, "container/blast_furnace/lit_progress")?;
    renderer.upload_hud_blast_furnace_lit_progress(
        blast_furnace_lit_progress.width,
        blast_furnace_lit_progress.height,
        &blast_furnace_lit_progress.rgba,
    )?;
    let blast_furnace_burn_progress =
        hud_sprite(&sprites, "container/blast_furnace/burn_progress")?;
    renderer.upload_hud_blast_furnace_burn_progress(
        blast_furnace_burn_progress.width,
        blast_furnace_burn_progress.height,
        &blast_furnace_burn_progress.rgba,
    )?;
    let smoker = gui_texture(
        roots,
        "textures/gui/container/smoker.png",
        "minecraft:textures/gui/container/smoker",
    )?;
    renderer.upload_hud_smoker_background(smoker.width, smoker.height, &smoker.rgba)?;
    let smoker_lit_progress = hud_sprite(&sprites, "container/smoker/lit_progress")?;
    renderer.upload_hud_smoker_lit_progress(
        smoker_lit_progress.width,
        smoker_lit_progress.height,
        &smoker_lit_progress.rgba,
    )?;
    let smoker_burn_progress = hud_sprite(&sprites, "container/smoker/burn_progress")?;
    renderer.upload_hud_smoker_burn_progress(
        smoker_burn_progress.width,
        smoker_burn_progress.height,
        &smoker_burn_progress.rgba,
    )?;
    let hopper = gui_texture(
        roots,
        "textures/gui/container/hopper.png",
        "minecraft:textures/gui/container/hopper",
    )?;
    renderer.upload_hud_hopper_background(hopper.width, hopper.height, &hopper.rgba)?;
    let shulker_box = gui_texture(
        roots,
        "textures/gui/container/shulker_box.png",
        "minecraft:textures/gui/container/shulker_box",
    )?;
    renderer.upload_hud_shulker_box_background(
        shulker_box.width,
        shulker_box.height,
        &shulker_box.rgba,
    )?;
    let slot_highlight_back = hud_sprite(&sprites, "container/slot_highlight_back")?;
    renderer.upload_hud_slot_highlight_back(
        slot_highlight_back.width,
        slot_highlight_back.height,
        &slot_highlight_back.rgba,
    )?;
    let slot_highlight_front = hud_sprite(&sprites, "container/slot_highlight_front")?;
    renderer.upload_hud_slot_highlight_front(
        slot_highlight_front.width,
        slot_highlight_front.height,
        &slot_highlight_front.rgba,
    )?;
    let experience_background = hud_sprite(&sprites, "hud/experience_bar_background")?;
    renderer.upload_hud_experience_background(
        experience_background.width,
        experience_background.height,
        &experience_background.rgba,
    )?;
    let experience_progress = hud_sprite(&sprites, "hud/experience_bar_progress")?;
    renderer.upload_hud_experience_progress(
        experience_progress.width,
        experience_progress.height,
        &experience_progress.rgba,
    )?;
    let heart_container = hud_sprite(&sprites, "hud/heart/container")?;
    renderer.upload_hud_heart_container(
        heart_container.width,
        heart_container.height,
        &heart_container.rgba,
    )?;
    let heart_full = hud_sprite(&sprites, "hud/heart/full")?;
    renderer.upload_hud_heart_full(heart_full.width, heart_full.height, &heart_full.rgba)?;
    let heart_half = hud_sprite(&sprites, "hud/heart/half")?;
    renderer.upload_hud_heart_half(heart_half.width, heart_half.height, &heart_half.rgba)?;
    let food_empty = hud_sprite(&sprites, "hud/food_empty")?;
    renderer.upload_hud_food_empty(food_empty.width, food_empty.height, &food_empty.rgba)?;
    let food_full = hud_sprite(&sprites, "hud/food_full")?;
    renderer.upload_hud_food_full(food_full.width, food_full.height, &food_full.rgba)?;
    let food_half = hud_sprite(&sprites, "hud/food_half")?;
    renderer.upload_hud_food_half(food_half.width, food_half.height, &food_half.rgba)?;
    let digit_atlas = hud_ascii_digit_atlas(roots)?;
    renderer.upload_hud_digit_atlas(
        digit_atlas.width,
        digit_atlas.height,
        &digit_atlas.rgba,
        digit_atlas.glyphs,
    )?;
    tracing::info!(
        crosshair = ?(crosshair.width, crosshair.height),
        hotbar = ?(hotbar.width, hotbar.height),
        inventory = ?(inventory.width, inventory.height),
        generic_container = ?(generic_container.width, generic_container.height),
        furnace = ?(furnace.width, furnace.height),
        blast_furnace = ?(blast_furnace.width, blast_furnace.height),
        smoker = ?(smoker.width, smoker.height),
        experience = ?(experience_background.width, experience_background.height),
        heart = ?(heart_full.width, heart_full.height),
        food = ?(food_full.width, food_full.height),
        digits = ?(digit_atlas.width, digit_atlas.height),
        "loaded vanilla HUD sprites"
    );
    Ok(())
}

fn load_gui_sprites(roots: &PackRoots) -> Result<HashMap<String, SpriteImage>> {
    Ok(roots
        .load_atlas_texture_images("gui")?
        .into_iter()
        .map(|image| (image.id.clone(), image))
        .collect())
}

fn hud_sprite<'a>(
    sprites: &'a HashMap<String, SpriteImage>,
    path: &str,
) -> Result<&'a SpriteImage> {
    let id = format!("minecraft:{path}");
    sprites
        .get(&id)
        .with_context(|| format!("missing HUD sprite {id} in vanilla GUI atlas"))
}

fn gui_texture(roots: &PackRoots, path: &str, id: &str) -> Result<SpriteImage> {
    let location = ResourceLocation::parse(path)?;
    let resource = roots
        .resource_stack()
        .get_resource(&location)
        .with_context(|| format!("missing GUI texture minecraft:{path}"))?;
    SpriteImage::from_png_file(id, resource.path)
}

#[derive(Debug, Clone, PartialEq)]
struct HudDigitAtlasImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    glyphs: [HudDigitGlyph; 10],
}

fn hud_ascii_digit_atlas(roots: &PackRoots) -> Result<HudDigitAtlasImage> {
    let ascii = gui_texture(
        roots,
        "textures/font/ascii.png",
        "minecraft:textures/font/ascii",
    )?;
    hud_ascii_digit_atlas_from_image(&ascii)
}

fn hud_ascii_digit_atlas_from_image(image: &SpriteImage) -> Result<HudDigitAtlasImage> {
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
        .context("ascii digit atlas destination pixel is outside image")?;
    dst_rgba[dst_offset..dst_end].copy_from_slice(&src_rgba[src_offset..src_end]);
    Ok(())
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
        .context("RGBA image size overflow")
}

fn rgba_offset(width: u32, x: u32, y: u32) -> Result<usize> {
    if x >= width {
        bail!("RGBA x coordinate is outside image width");
    }
    let row = y.checked_mul(width).context("RGBA row offset overflow")?;
    let pixel = row.checked_add(x).context("RGBA pixel offset overflow")?;
    usize::try_from(pixel)
        .ok()
        .and_then(|pixel| pixel.checked_mul(4))
        .context("RGBA byte offset overflow")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_sprite_uses_vanilla_gui_atlas_ids() {
        let image =
            SpriteImage::new("minecraft:hud/crosshair", 1, 1, vec![255, 255, 255, 255]).unwrap();
        let mut sprites = HashMap::new();
        sprites.insert(image.id.clone(), image);

        let crosshair = hud_sprite(&sprites, "hud/crosshair").unwrap();
        assert_eq!(crosshair.id, "minecraft:hud/crosshair");

        let err = hud_sprite(&sprites, "gui/sprites/hud/crosshair").unwrap_err();
        assert!(err
            .to_string()
            .contains("minecraft:gui/sprites/hud/crosshair"));
    }

    #[test]
    fn hud_ascii_digit_atlas_extracts_digits_and_vanilla_advances() {
        let mut rgba = vec![0; rgba_len(128, 128).unwrap()];
        set_pixel(&mut rgba, 128, 3, 24, [255, 255, 255, 255]);
        set_pixel(&mut rgba, 128, 4 * 8 + 5, 24 + 7, [10, 20, 30, 255]);
        let image = SpriteImage::new("minecraft:textures/font/ascii", 128, 128, rgba).unwrap();

        let atlas = hud_ascii_digit_atlas_from_image(&image).unwrap();

        assert_eq!(atlas.width, 80);
        assert_eq!(atlas.height, 8);
        assert_eq!(atlas.glyphs[0].width, 8);
        assert_eq!(atlas.glyphs[0].height, 8);
        assert_eq!(atlas.glyphs[0].advance, 5);
        assert_eq!(atlas.glyphs[4].advance, 7);
        assert_eq!(atlas.glyphs[4].uv.min, [0.4, 0.0]);
        assert_eq!(atlas.glyphs[4].uv.max, [0.5, 1.0]);
        assert_eq!(
            atlas.rgba[rgba_offset(80, 3, 0).unwrap()..rgba_offset(80, 3, 0).unwrap() + 4],
            [255, 255, 255, 255]
        );
        assert_eq!(
            atlas.rgba[rgba_offset(80, 4 * 8 + 5, 7).unwrap()
                ..rgba_offset(80, 4 * 8 + 5, 7).unwrap() + 4],
            [10, 20, 30, 255]
        );
    }

    fn set_pixel(rgba: &mut [u8], width: u32, x: u32, y: u32, pixel: [u8; 4]) {
        let offset = rgba_offset(width, x, y).unwrap();
        rgba[offset..offset + 4].copy_from_slice(&pixel);
    }
}
