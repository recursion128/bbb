use std::collections::HashMap;

use anyhow::{Context, Result};
use bbb_pack::{PackRoots, ResourceLocation, SpriteImage};

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
    tracing::info!(
        crosshair = ?(crosshair.width, crosshair.height),
        hotbar = ?(hotbar.width, hotbar.height),
        inventory = ?(inventory.width, inventory.height),
        experience = ?(experience_background.width, experience_background.height),
        heart = ?(heart_full.width, heart_full.height),
        food = ?(food_full.width, food_full.height),
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
}
