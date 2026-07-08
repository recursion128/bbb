use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use bbb_pack::{PackRoots, ResourceLocation, SpriteGuiScaling, SpriteImage};
use bbb_renderer::{
    HudAdvancementBackgroundTexture, HudAdvancementTabSprite, HudAdvancementWidgetFrameSprite,
    HudBossBarColor, HudBossBarOverlay, HudHeartKind, HudNineSliceScaling, SignModelWood,
};

use bbb_item_model::font::{
    hud_ascii_digit_atlas_from_image, load_ascii_font_texture, load_hud_font_atlas,
};

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
    let tooltip_background = hud_sprite(&sprites, "tooltip/background")?;
    let tooltip_background_scaling = hud_nine_slice_scaling(tooltip_background)?;
    renderer.upload_hud_tooltip_background(
        tooltip_background.width,
        tooltip_background.height,
        &tooltip_background.rgba,
        tooltip_background_scaling,
    )?;
    let tooltip_frame = hud_sprite(&sprites, "tooltip/frame")?;
    let tooltip_frame_scaling = hud_nine_slice_scaling(tooltip_frame)?;
    renderer.upload_hud_tooltip_frame(
        tooltip_frame.width,
        tooltip_frame.height,
        &tooltip_frame.rgba,
        tooltip_frame_scaling,
    )?;
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
    let advancements_window = gui_texture(
        roots,
        "textures/gui/advancements/window.png",
        "minecraft:textures/gui/advancements/window",
    )?;
    renderer.upload_hud_advancements_window(
        advancements_window.width,
        advancements_window.height,
        &advancements_window.rgba,
    )?;
    for tab_sprite in HudAdvancementTabSprite::ALL {
        let sprite = hud_sprite(&sprites, tab_sprite.sprite_path())?;
        renderer.upload_hud_advancement_tab(
            tab_sprite,
            sprite.width,
            sprite.height,
            &sprite.rgba,
        )?;
    }
    for background in HudAdvancementBackgroundTexture::VANILLA {
        let texture = gui_texture(
            roots,
            background
                .texture_path()
                .expect("vanilla advancement background has a texture path"),
            background
                .texture_resource_id()
                .expect("vanilla advancement background has a resource id"),
        )?;
        renderer.upload_hud_advancement_background(
            background,
            texture.width,
            texture.height,
            &texture.rgba,
        )?;
    }
    let missing_background = missing_texture_rgba(16, 16);
    renderer.upload_hud_advancement_background(
        HudAdvancementBackgroundTexture::Missing,
        16,
        16,
        &missing_background,
    )?;
    for frame_sprite in HudAdvancementWidgetFrameSprite::ALL {
        let sprite = hud_sprite(&sprites, frame_sprite.sprite_path())?;
        renderer.upload_hud_advancement_widget_frame(
            frame_sprite,
            sprite.width,
            sprite.height,
            &sprite.rgba,
        )?;
    }
    let recipe_book = gui_texture(
        roots,
        "textures/gui/recipe_book.png",
        "minecraft:textures/gui/recipe_book",
    )?;
    renderer.upload_hud_recipe_book_background(
        recipe_book.width,
        recipe_book.height,
        &recipe_book.rgba,
    )?;
    let widget_text_field = hud_sprite(&sprites, "widget/text_field")?;
    renderer.upload_hud_widget_text_field(
        widget_text_field.width,
        widget_text_field.height,
        &widget_text_field.rgba,
    )?;
    let widget_text_field_highlighted = hud_sprite(&sprites, "widget/text_field_highlighted")?;
    renderer.upload_hud_widget_text_field_highlighted(
        widget_text_field_highlighted.width,
        widget_text_field_highlighted.height,
        &widget_text_field_highlighted.rgba,
    )?;
    let widget_button = hud_sprite(&sprites, "widget/button")?;
    renderer.upload_hud_widget_button(
        widget_button.width,
        widget_button.height,
        &widget_button.rgba,
    )?;
    let widget_button_highlighted = hud_sprite(&sprites, "widget/button_highlighted")?;
    renderer.upload_hud_widget_button_highlighted(
        widget_button_highlighted.width,
        widget_button_highlighted.height,
        &widget_button_highlighted.rgba,
    )?;
    let recipe_book_tab = hud_sprite(&sprites, "recipe_book/tab")?;
    renderer.upload_hud_recipe_book_tab(
        recipe_book_tab.width,
        recipe_book_tab.height,
        &recipe_book_tab.rgba,
    )?;
    let recipe_book_tab_selected = hud_sprite(&sprites, "recipe_book/tab_selected")?;
    renderer.upload_hud_recipe_book_tab_selected(
        recipe_book_tab_selected.width,
        recipe_book_tab_selected.height,
        &recipe_book_tab_selected.rgba,
    )?;
    let recipe_book_button = hud_sprite(&sprites, "recipe_book/button")?;
    renderer.upload_hud_recipe_book_button(
        recipe_book_button.width,
        recipe_book_button.height,
        &recipe_book_button.rgba,
    )?;
    let recipe_book_button_highlighted = hud_sprite(&sprites, "recipe_book/button_highlighted")?;
    renderer.upload_hud_recipe_book_button_highlighted(
        recipe_book_button_highlighted.width,
        recipe_book_button_highlighted.height,
        &recipe_book_button_highlighted.rgba,
    )?;
    let recipe_book_filter_enabled = hud_sprite(&sprites, "recipe_book/filter_enabled")?;
    renderer.upload_hud_recipe_book_filter_enabled(
        recipe_book_filter_enabled.width,
        recipe_book_filter_enabled.height,
        &recipe_book_filter_enabled.rgba,
    )?;
    let recipe_book_filter_disabled = hud_sprite(&sprites, "recipe_book/filter_disabled")?;
    renderer.upload_hud_recipe_book_filter_disabled(
        recipe_book_filter_disabled.width,
        recipe_book_filter_disabled.height,
        &recipe_book_filter_disabled.rgba,
    )?;
    let recipe_book_filter_enabled_highlighted =
        hud_sprite(&sprites, "recipe_book/filter_enabled_highlighted")?;
    renderer.upload_hud_recipe_book_filter_enabled_highlighted(
        recipe_book_filter_enabled_highlighted.width,
        recipe_book_filter_enabled_highlighted.height,
        &recipe_book_filter_enabled_highlighted.rgba,
    )?;
    let recipe_book_filter_disabled_highlighted =
        hud_sprite(&sprites, "recipe_book/filter_disabled_highlighted")?;
    renderer.upload_hud_recipe_book_filter_disabled_highlighted(
        recipe_book_filter_disabled_highlighted.width,
        recipe_book_filter_disabled_highlighted.height,
        &recipe_book_filter_disabled_highlighted.rgba,
    )?;
    let recipe_book_furnace_filter_enabled =
        hud_sprite(&sprites, "recipe_book/furnace_filter_enabled")?;
    renderer.upload_hud_recipe_book_furnace_filter_enabled(
        recipe_book_furnace_filter_enabled.width,
        recipe_book_furnace_filter_enabled.height,
        &recipe_book_furnace_filter_enabled.rgba,
    )?;
    let recipe_book_furnace_filter_disabled =
        hud_sprite(&sprites, "recipe_book/furnace_filter_disabled")?;
    renderer.upload_hud_recipe_book_furnace_filter_disabled(
        recipe_book_furnace_filter_disabled.width,
        recipe_book_furnace_filter_disabled.height,
        &recipe_book_furnace_filter_disabled.rgba,
    )?;
    let recipe_book_furnace_filter_enabled_highlighted =
        hud_sprite(&sprites, "recipe_book/furnace_filter_enabled_highlighted")?;
    renderer.upload_hud_recipe_book_furnace_filter_enabled_highlighted(
        recipe_book_furnace_filter_enabled_highlighted.width,
        recipe_book_furnace_filter_enabled_highlighted.height,
        &recipe_book_furnace_filter_enabled_highlighted.rgba,
    )?;
    let recipe_book_furnace_filter_disabled_highlighted =
        hud_sprite(&sprites, "recipe_book/furnace_filter_disabled_highlighted")?;
    renderer.upload_hud_recipe_book_furnace_filter_disabled_highlighted(
        recipe_book_furnace_filter_disabled_highlighted.width,
        recipe_book_furnace_filter_disabled_highlighted.height,
        &recipe_book_furnace_filter_disabled_highlighted.rgba,
    )?;
    let recipe_book_slot_craftable = hud_sprite(&sprites, "recipe_book/slot_craftable")?;
    renderer.upload_hud_recipe_book_slot_craftable(
        recipe_book_slot_craftable.width,
        recipe_book_slot_craftable.height,
        &recipe_book_slot_craftable.rgba,
    )?;
    let recipe_book_slot_uncraftable = hud_sprite(&sprites, "recipe_book/slot_uncraftable")?;
    renderer.upload_hud_recipe_book_slot_uncraftable(
        recipe_book_slot_uncraftable.width,
        recipe_book_slot_uncraftable.height,
        &recipe_book_slot_uncraftable.rgba,
    )?;
    let recipe_book_slot_many_craftable = hud_sprite(&sprites, "recipe_book/slot_many_craftable")?;
    renderer.upload_hud_recipe_book_slot_many_craftable(
        recipe_book_slot_many_craftable.width,
        recipe_book_slot_many_craftable.height,
        &recipe_book_slot_many_craftable.rgba,
    )?;
    let recipe_book_slot_many_uncraftable =
        hud_sprite(&sprites, "recipe_book/slot_many_uncraftable")?;
    renderer.upload_hud_recipe_book_slot_many_uncraftable(
        recipe_book_slot_many_uncraftable.width,
        recipe_book_slot_many_uncraftable.height,
        &recipe_book_slot_many_uncraftable.rgba,
    )?;
    let recipe_book_page_forward = hud_sprite(&sprites, "recipe_book/page_forward")?;
    renderer.upload_hud_recipe_book_page_forward(
        recipe_book_page_forward.width,
        recipe_book_page_forward.height,
        &recipe_book_page_forward.rgba,
    )?;
    let recipe_book_page_forward_highlighted =
        hud_sprite(&sprites, "recipe_book/page_forward_highlighted")?;
    renderer.upload_hud_recipe_book_page_forward_highlighted(
        recipe_book_page_forward_highlighted.width,
        recipe_book_page_forward_highlighted.height,
        &recipe_book_page_forward_highlighted.rgba,
    )?;
    let recipe_book_page_backward = hud_sprite(&sprites, "recipe_book/page_backward")?;
    renderer.upload_hud_recipe_book_page_backward(
        recipe_book_page_backward.width,
        recipe_book_page_backward.height,
        &recipe_book_page_backward.rgba,
    )?;
    let recipe_book_page_backward_highlighted =
        hud_sprite(&sprites, "recipe_book/page_backward_highlighted")?;
    renderer.upload_hud_recipe_book_page_backward_highlighted(
        recipe_book_page_backward_highlighted.width,
        recipe_book_page_backward_highlighted.height,
        &recipe_book_page_backward_highlighted.rgba,
    )?;
    let recipe_book_overlay_recipe = hud_sprite(&sprites, "recipe_book/overlay_recipe")?;
    renderer.upload_hud_recipe_book_overlay_recipe(
        recipe_book_overlay_recipe.width,
        recipe_book_overlay_recipe.height,
        &recipe_book_overlay_recipe.rgba,
    )?;
    let recipe_book_crafting_overlay = hud_sprite(&sprites, "recipe_book/crafting_overlay")?;
    renderer.upload_hud_recipe_book_crafting_overlay(
        recipe_book_crafting_overlay.width,
        recipe_book_crafting_overlay.height,
        &recipe_book_crafting_overlay.rgba,
    )?;
    let recipe_book_crafting_overlay_highlighted =
        hud_sprite(&sprites, "recipe_book/crafting_overlay_highlighted")?;
    renderer.upload_hud_recipe_book_crafting_overlay_highlighted(
        recipe_book_crafting_overlay_highlighted.width,
        recipe_book_crafting_overlay_highlighted.height,
        &recipe_book_crafting_overlay_highlighted.rgba,
    )?;
    let recipe_book_crafting_overlay_disabled =
        hud_sprite(&sprites, "recipe_book/crafting_overlay_disabled")?;
    renderer.upload_hud_recipe_book_crafting_overlay_disabled(
        recipe_book_crafting_overlay_disabled.width,
        recipe_book_crafting_overlay_disabled.height,
        &recipe_book_crafting_overlay_disabled.rgba,
    )?;
    let recipe_book_crafting_overlay_disabled_highlighted = hud_sprite(
        &sprites,
        "recipe_book/crafting_overlay_disabled_highlighted",
    )?;
    renderer.upload_hud_recipe_book_crafting_overlay_disabled_highlighted(
        recipe_book_crafting_overlay_disabled_highlighted.width,
        recipe_book_crafting_overlay_disabled_highlighted.height,
        &recipe_book_crafting_overlay_disabled_highlighted.rgba,
    )?;
    let recipe_book_furnace_overlay = hud_sprite(&sprites, "recipe_book/furnace_overlay")?;
    renderer.upload_hud_recipe_book_furnace_overlay(
        recipe_book_furnace_overlay.width,
        recipe_book_furnace_overlay.height,
        &recipe_book_furnace_overlay.rgba,
    )?;
    let recipe_book_furnace_overlay_highlighted =
        hud_sprite(&sprites, "recipe_book/furnace_overlay_highlighted")?;
    renderer.upload_hud_recipe_book_furnace_overlay_highlighted(
        recipe_book_furnace_overlay_highlighted.width,
        recipe_book_furnace_overlay_highlighted.height,
        &recipe_book_furnace_overlay_highlighted.rgba,
    )?;
    let recipe_book_furnace_overlay_disabled =
        hud_sprite(&sprites, "recipe_book/furnace_overlay_disabled")?;
    renderer.upload_hud_recipe_book_furnace_overlay_disabled(
        recipe_book_furnace_overlay_disabled.width,
        recipe_book_furnace_overlay_disabled.height,
        &recipe_book_furnace_overlay_disabled.rgba,
    )?;
    let recipe_book_furnace_overlay_disabled_highlighted =
        hud_sprite(&sprites, "recipe_book/furnace_overlay_disabled_highlighted")?;
    renderer.upload_hud_recipe_book_furnace_overlay_disabled_highlighted(
        recipe_book_furnace_overlay_disabled_highlighted.width,
        recipe_book_furnace_overlay_disabled_highlighted.height,
        &recipe_book_furnace_overlay_disabled_highlighted.rgba,
    )?;
    let cartography_table = gui_texture(
        roots,
        "textures/gui/container/cartography_table.png",
        "minecraft:textures/gui/container/cartography_table",
    )?;
    renderer.upload_hud_cartography_table_background(
        cartography_table.width,
        cartography_table.height,
        &cartography_table.rgba,
    )?;
    let cartography_table_error = hud_sprite(&sprites, "container/cartography_table/error")?;
    renderer.upload_hud_cartography_table_error(
        cartography_table_error.width,
        cartography_table_error.height,
        &cartography_table_error.rgba,
    )?;
    let cartography_table_scaled_map =
        hud_sprite(&sprites, "container/cartography_table/scaled_map")?;
    renderer.upload_hud_cartography_table_scaled_map(
        cartography_table_scaled_map.width,
        cartography_table_scaled_map.height,
        &cartography_table_scaled_map.rgba,
    )?;
    let cartography_table_duplicated_map =
        hud_sprite(&sprites, "container/cartography_table/duplicated_map")?;
    renderer.upload_hud_cartography_table_duplicated_map(
        cartography_table_duplicated_map.width,
        cartography_table_duplicated_map.height,
        &cartography_table_duplicated_map.rgba,
    )?;
    let cartography_table_map = hud_sprite(&sprites, "container/cartography_table/map")?;
    renderer.upload_hud_cartography_table_map(
        cartography_table_map.width,
        cartography_table_map.height,
        &cartography_table_map.rgba,
    )?;
    let cartography_table_locked = hud_sprite(&sprites, "container/cartography_table/locked")?;
    renderer.upload_hud_cartography_table_locked(
        cartography_table_locked.width,
        cartography_table_locked.height,
        &cartography_table_locked.rgba,
    )?;
    let loom = gui_texture(
        roots,
        "textures/gui/container/loom.png",
        "minecraft:textures/gui/container/loom",
    )?;
    renderer.upload_hud_loom_background(loom.width, loom.height, &loom.rgba)?;
    let loom_banner_slot = hud_sprite(&sprites, "container/slot/banner")?;
    renderer.upload_hud_loom_banner_slot(
        loom_banner_slot.width,
        loom_banner_slot.height,
        &loom_banner_slot.rgba,
    )?;
    let loom_dye_slot = hud_sprite(&sprites, "container/slot/dye")?;
    renderer.upload_hud_loom_dye_slot(
        loom_dye_slot.width,
        loom_dye_slot.height,
        &loom_dye_slot.rgba,
    )?;
    let loom_pattern_slot = hud_sprite(&sprites, "container/slot/banner_pattern")?;
    renderer.upload_hud_loom_pattern_slot(
        loom_pattern_slot.width,
        loom_pattern_slot.height,
        &loom_pattern_slot.rgba,
    )?;
    let loom_scroller = hud_sprite(&sprites, "container/loom/scroller")?;
    renderer.upload_hud_loom_scroller(
        loom_scroller.width,
        loom_scroller.height,
        &loom_scroller.rgba,
    )?;
    let loom_scroller_disabled = hud_sprite(&sprites, "container/loom/scroller_disabled")?;
    renderer.upload_hud_loom_scroller_disabled(
        loom_scroller_disabled.width,
        loom_scroller_disabled.height,
        &loom_scroller_disabled.rgba,
    )?;
    let loom_pattern_selected = hud_sprite(&sprites, "container/loom/pattern_selected")?;
    renderer.upload_hud_loom_pattern_selected(
        loom_pattern_selected.width,
        loom_pattern_selected.height,
        &loom_pattern_selected.rgba,
    )?;
    let loom_pattern_highlighted = hud_sprite(&sprites, "container/loom/pattern_highlighted")?;
    renderer.upload_hud_loom_pattern_highlighted(
        loom_pattern_highlighted.width,
        loom_pattern_highlighted.height,
        &loom_pattern_highlighted.rgba,
    )?;
    let loom_pattern = hud_sprite(&sprites, "container/loom/pattern")?;
    renderer.upload_hud_loom_pattern(
        loom_pattern.width,
        loom_pattern.height,
        &loom_pattern.rgba,
    )?;
    let loom_error = hud_sprite(&sprites, "container/loom/error")?;
    renderer.upload_hud_loom_error(loom_error.width, loom_error.height, &loom_error.rgba)?;
    let crafter = gui_texture(
        roots,
        "textures/gui/container/crafter.png",
        "minecraft:textures/gui/container/crafter",
    )?;
    renderer.upload_hud_crafter_background(crafter.width, crafter.height, &crafter.rgba)?;
    let crafter_disabled_slot = hud_sprite(&sprites, "container/crafter/disabled_slot")?;
    renderer.upload_hud_crafter_disabled_slot(
        crafter_disabled_slot.width,
        crafter_disabled_slot.height,
        &crafter_disabled_slot.rgba,
    )?;
    let crafter_powered_redstone = hud_sprite(&sprites, "container/crafter/powered_redstone")?;
    renderer.upload_hud_crafter_powered_redstone(
        crafter_powered_redstone.width,
        crafter_powered_redstone.height,
        &crafter_powered_redstone.rgba,
    )?;
    let crafter_unpowered_redstone = hud_sprite(&sprites, "container/crafter/unpowered_redstone")?;
    renderer.upload_hud_crafter_unpowered_redstone(
        crafter_unpowered_redstone.width,
        crafter_unpowered_redstone.height,
        &crafter_unpowered_redstone.rgba,
    )?;
    let anvil = gui_texture(
        roots,
        "textures/gui/container/anvil.png",
        "minecraft:textures/gui/container/anvil",
    )?;
    renderer.upload_hud_anvil_background(anvil.width, anvil.height, &anvil.rgba)?;
    let anvil_text_field = hud_sprite(&sprites, "container/anvil/text_field")?;
    renderer.upload_hud_anvil_text_field(
        anvil_text_field.width,
        anvil_text_field.height,
        &anvil_text_field.rgba,
    )?;
    let anvil_text_field_disabled = hud_sprite(&sprites, "container/anvil/text_field_disabled")?;
    renderer.upload_hud_anvil_text_field_disabled(
        anvil_text_field_disabled.width,
        anvil_text_field_disabled.height,
        &anvil_text_field_disabled.rgba,
    )?;
    let anvil_error = hud_sprite(&sprites, "container/anvil/error")?;
    renderer.upload_hud_anvil_error(anvil_error.width, anvil_error.height, &anvil_error.rgba)?;
    let enchanting_table = gui_texture(
        roots,
        "textures/gui/container/enchanting_table.png",
        "minecraft:textures/gui/container/enchanting_table",
    )?;
    renderer.upload_hud_enchanting_table_background(
        enchanting_table.width,
        enchanting_table.height,
        &enchanting_table.rgba,
    )?;
    let enchanting_table_lapis_slot = hud_sprite(&sprites, "container/slot/lapis_lazuli")?;
    renderer.upload_hud_enchanting_table_lapis_slot(
        enchanting_table_lapis_slot.width,
        enchanting_table_lapis_slot.height,
        &enchanting_table_lapis_slot.rgba,
    )?;
    let enchanting_table_enchantment_slot_disabled = hud_sprite(
        &sprites,
        "container/enchanting_table/enchantment_slot_disabled",
    )?;
    renderer.upload_hud_enchanting_table_enchantment_slot_disabled(
        enchanting_table_enchantment_slot_disabled.width,
        enchanting_table_enchantment_slot_disabled.height,
        &enchanting_table_enchantment_slot_disabled.rgba,
    )?;
    let enchanting_table_enchantment_slot_highlighted = hud_sprite(
        &sprites,
        "container/enchanting_table/enchantment_slot_highlighted",
    )?;
    renderer.upload_hud_enchanting_table_enchantment_slot_highlighted(
        enchanting_table_enchantment_slot_highlighted.width,
        enchanting_table_enchantment_slot_highlighted.height,
        &enchanting_table_enchantment_slot_highlighted.rgba,
    )?;
    let enchanting_table_enchantment_slot =
        hud_sprite(&sprites, "container/enchanting_table/enchantment_slot")?;
    renderer.upload_hud_enchanting_table_enchantment_slot(
        enchanting_table_enchantment_slot.width,
        enchanting_table_enchantment_slot.height,
        &enchanting_table_enchantment_slot.rgba,
    )?;
    let enchanting_table_level_1 = hud_sprite(&sprites, "container/enchanting_table/level_1")?;
    renderer.upload_hud_enchanting_table_level_1(
        enchanting_table_level_1.width,
        enchanting_table_level_1.height,
        &enchanting_table_level_1.rgba,
    )?;
    let enchanting_table_level_2 = hud_sprite(&sprites, "container/enchanting_table/level_2")?;
    renderer.upload_hud_enchanting_table_level_2(
        enchanting_table_level_2.width,
        enchanting_table_level_2.height,
        &enchanting_table_level_2.rgba,
    )?;
    let enchanting_table_level_3 = hud_sprite(&sprites, "container/enchanting_table/level_3")?;
    renderer.upload_hud_enchanting_table_level_3(
        enchanting_table_level_3.width,
        enchanting_table_level_3.height,
        &enchanting_table_level_3.rgba,
    )?;
    let enchanting_table_level_1_disabled =
        hud_sprite(&sprites, "container/enchanting_table/level_1_disabled")?;
    renderer.upload_hud_enchanting_table_level_1_disabled(
        enchanting_table_level_1_disabled.width,
        enchanting_table_level_1_disabled.height,
        &enchanting_table_level_1_disabled.rgba,
    )?;
    let enchanting_table_level_2_disabled =
        hud_sprite(&sprites, "container/enchanting_table/level_2_disabled")?;
    renderer.upload_hud_enchanting_table_level_2_disabled(
        enchanting_table_level_2_disabled.width,
        enchanting_table_level_2_disabled.height,
        &enchanting_table_level_2_disabled.rgba,
    )?;
    let enchanting_table_level_3_disabled =
        hud_sprite(&sprites, "container/enchanting_table/level_3_disabled")?;
    renderer.upload_hud_enchanting_table_level_3_disabled(
        enchanting_table_level_3_disabled.width,
        enchanting_table_level_3_disabled.height,
        &enchanting_table_level_3_disabled.rgba,
    )?;
    let beacon = gui_texture(
        roots,
        "textures/gui/container/beacon.png",
        "minecraft:textures/gui/container/beacon",
    )?;
    renderer.upload_hud_beacon_background(beacon.width, beacon.height, &beacon.rgba)?;
    let beacon_button_disabled = hud_sprite(&sprites, "container/beacon/button_disabled")?;
    renderer.upload_hud_beacon_button_disabled(
        beacon_button_disabled.width,
        beacon_button_disabled.height,
        &beacon_button_disabled.rgba,
    )?;
    let beacon_button_selected = hud_sprite(&sprites, "container/beacon/button_selected")?;
    renderer.upload_hud_beacon_button_selected(
        beacon_button_selected.width,
        beacon_button_selected.height,
        &beacon_button_selected.rgba,
    )?;
    let beacon_button_highlighted = hud_sprite(&sprites, "container/beacon/button_highlighted")?;
    renderer.upload_hud_beacon_button_highlighted(
        beacon_button_highlighted.width,
        beacon_button_highlighted.height,
        &beacon_button_highlighted.rgba,
    )?;
    let beacon_button = hud_sprite(&sprites, "container/beacon/button")?;
    renderer.upload_hud_beacon_button(
        beacon_button.width,
        beacon_button.height,
        &beacon_button.rgba,
    )?;
    let beacon_confirm = hud_sprite(&sprites, "container/beacon/confirm")?;
    renderer.upload_hud_beacon_confirm(
        beacon_confirm.width,
        beacon_confirm.height,
        &beacon_confirm.rgba,
    )?;
    let beacon_cancel = hud_sprite(&sprites, "container/beacon/cancel")?;
    renderer.upload_hud_beacon_cancel(
        beacon_cancel.width,
        beacon_cancel.height,
        &beacon_cancel.rgba,
    )?;
    let beacon_effect_speed = hud_sprite(&sprites, "mob_effect/speed")?;
    renderer.upload_hud_beacon_effect_speed(
        beacon_effect_speed.width,
        beacon_effect_speed.height,
        &beacon_effect_speed.rgba,
    )?;
    let beacon_effect_haste = hud_sprite(&sprites, "mob_effect/haste")?;
    renderer.upload_hud_beacon_effect_haste(
        beacon_effect_haste.width,
        beacon_effect_haste.height,
        &beacon_effect_haste.rgba,
    )?;
    let beacon_effect_resistance = hud_sprite(&sprites, "mob_effect/resistance")?;
    renderer.upload_hud_beacon_effect_resistance(
        beacon_effect_resistance.width,
        beacon_effect_resistance.height,
        &beacon_effect_resistance.rgba,
    )?;
    let beacon_effect_jump_boost = hud_sprite(&sprites, "mob_effect/jump_boost")?;
    renderer.upload_hud_beacon_effect_jump_boost(
        beacon_effect_jump_boost.width,
        beacon_effect_jump_boost.height,
        &beacon_effect_jump_boost.rgba,
    )?;
    let beacon_effect_strength = hud_sprite(&sprites, "mob_effect/strength")?;
    renderer.upload_hud_beacon_effect_strength(
        beacon_effect_strength.width,
        beacon_effect_strength.height,
        &beacon_effect_strength.rgba,
    )?;
    let beacon_effect_regeneration = hud_sprite(&sprites, "mob_effect/regeneration")?;
    renderer.upload_hud_beacon_effect_regeneration(
        beacon_effect_regeneration.width,
        beacon_effect_regeneration.height,
        &beacon_effect_regeneration.rgba,
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
    let smithing = gui_texture(
        roots,
        "textures/gui/container/smithing.png",
        "minecraft:textures/gui/container/smithing",
    )?;
    renderer.upload_hud_smithing_background(smithing.width, smithing.height, &smithing.rgba)?;
    let smithing_error = hud_sprite(&sprites, "container/smithing/error")?;
    renderer.upload_hud_smithing_error(
        smithing_error.width,
        smithing_error.height,
        &smithing_error.rgba,
    )?;
    let brewing_stand = gui_texture(
        roots,
        "textures/gui/container/brewing_stand.png",
        "minecraft:textures/gui/container/brewing_stand",
    )?;
    renderer.upload_hud_brewing_stand_background(
        brewing_stand.width,
        brewing_stand.height,
        &brewing_stand.rgba,
    )?;
    let brewing_stand_fuel_length = hud_sprite(&sprites, "container/brewing_stand/fuel_length")?;
    renderer.upload_hud_brewing_stand_fuel_length(
        brewing_stand_fuel_length.width,
        brewing_stand_fuel_length.height,
        &brewing_stand_fuel_length.rgba,
    )?;
    let brewing_stand_brew_progress =
        hud_sprite(&sprites, "container/brewing_stand/brew_progress")?;
    renderer.upload_hud_brewing_stand_brew_progress(
        brewing_stand_brew_progress.width,
        brewing_stand_brew_progress.height,
        &brewing_stand_brew_progress.rgba,
    )?;
    let brewing_stand_bubbles = hud_sprite(&sprites, "container/brewing_stand/bubbles")?;
    renderer.upload_hud_brewing_stand_bubbles(
        brewing_stand_bubbles.width,
        brewing_stand_bubbles.height,
        &brewing_stand_bubbles.rgba,
    )?;
    let grindstone = gui_texture(
        roots,
        "textures/gui/container/grindstone.png",
        "minecraft:textures/gui/container/grindstone",
    )?;
    renderer.upload_hud_grindstone_background(
        grindstone.width,
        grindstone.height,
        &grindstone.rgba,
    )?;
    let grindstone_error = hud_sprite(&sprites, "container/grindstone/error")?;
    renderer.upload_hud_grindstone_error(
        grindstone_error.width,
        grindstone_error.height,
        &grindstone_error.rgba,
    )?;
    let hopper = gui_texture(
        roots,
        "textures/gui/container/hopper.png",
        "minecraft:textures/gui/container/hopper",
    )?;
    renderer.upload_hud_hopper_background(hopper.width, hopper.height, &hopper.rgba)?;
    let horse = gui_texture(
        roots,
        "textures/gui/container/horse.png",
        "minecraft:textures/gui/container/horse",
    )?;
    renderer.upload_hud_horse_background(horse.width, horse.height, &horse.rgba)?;
    let nautilus = gui_texture(
        roots,
        "textures/gui/container/nautilus.png",
        "minecraft:textures/gui/container/nautilus",
    )?;
    renderer.upload_hud_nautilus_background(nautilus.width, nautilus.height, &nautilus.rgba)?;
    let mount_slot = hud_sprite(&sprites, "container/slot")?;
    renderer.upload_hud_mount_slot(mount_slot.width, mount_slot.height, &mount_slot.rgba)?;
    let mount_saddle_slot = hud_sprite(&sprites, "container/slot/saddle")?;
    renderer.upload_hud_mount_saddle_slot(
        mount_saddle_slot.width,
        mount_saddle_slot.height,
        &mount_saddle_slot.rgba,
    )?;
    let mount_horse_armor_slot = hud_sprite(&sprites, "container/slot/horse_armor")?;
    renderer.upload_hud_mount_horse_armor_slot(
        mount_horse_armor_slot.width,
        mount_horse_armor_slot.height,
        &mount_horse_armor_slot.rgba,
    )?;
    let mount_llama_armor_slot = hud_sprite(&sprites, "container/slot/llama_armor")?;
    renderer.upload_hud_mount_llama_armor_slot(
        mount_llama_armor_slot.width,
        mount_llama_armor_slot.height,
        &mount_llama_armor_slot.rgba,
    )?;
    let mount_nautilus_armor_slot =
        hud_sprite(&sprites, "container/slot/nautilus_armor_inventory")?;
    renderer.upload_hud_mount_nautilus_armor_slot(
        mount_nautilus_armor_slot.width,
        mount_nautilus_armor_slot.height,
        &mount_nautilus_armor_slot.rgba,
    )?;
    let mount_chest_slots = hud_sprite(&sprites, "container/horse/chest_slots")?;
    renderer.upload_hud_mount_chest_slots(
        mount_chest_slots.width,
        mount_chest_slots.height,
        &mount_chest_slots.rgba,
    )?;
    let book = gui_texture(
        roots,
        "textures/gui/book.png",
        "minecraft:textures/gui/book",
    )?;
    renderer.upload_hud_book_background(book.width, book.height, &book.rgba)?;
    let page_backward = hud_sprite(&sprites, "widget/page_backward")?;
    renderer.upload_hud_page_backward(
        page_backward.width,
        page_backward.height,
        &page_backward.rgba,
    )?;
    let page_forward = hud_sprite(&sprites, "widget/page_forward")?;
    renderer.upload_hud_page_forward(
        page_forward.width,
        page_forward.height,
        &page_forward.rgba,
    )?;
    load_hanging_sign_backgrounds(renderer, roots)?;
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
    let stonecutter = gui_texture(
        roots,
        "textures/gui/container/stonecutter.png",
        "minecraft:textures/gui/container/stonecutter",
    )?;
    renderer.upload_hud_stonecutter_background(
        stonecutter.width,
        stonecutter.height,
        &stonecutter.rgba,
    )?;
    let stonecutter_scroller = hud_sprite(&sprites, "container/stonecutter/scroller")?;
    renderer.upload_hud_stonecutter_scroller(
        stonecutter_scroller.width,
        stonecutter_scroller.height,
        &stonecutter_scroller.rgba,
    )?;
    let stonecutter_scroller_disabled =
        hud_sprite(&sprites, "container/stonecutter/scroller_disabled")?;
    renderer.upload_hud_stonecutter_scroller_disabled(
        stonecutter_scroller_disabled.width,
        stonecutter_scroller_disabled.height,
        &stonecutter_scroller_disabled.rgba,
    )?;
    let stonecutter_recipe_selected =
        hud_sprite(&sprites, "container/stonecutter/recipe_selected")?;
    renderer.upload_hud_stonecutter_recipe_selected(
        stonecutter_recipe_selected.width,
        stonecutter_recipe_selected.height,
        &stonecutter_recipe_selected.rgba,
    )?;
    let stonecutter_recipe_highlighted =
        hud_sprite(&sprites, "container/stonecutter/recipe_highlighted")?;
    renderer.upload_hud_stonecutter_recipe_highlighted(
        stonecutter_recipe_highlighted.width,
        stonecutter_recipe_highlighted.height,
        &stonecutter_recipe_highlighted.rgba,
    )?;
    let stonecutter_recipe = hud_sprite(&sprites, "container/stonecutter/recipe")?;
    renderer.upload_hud_stonecutter_recipe(
        stonecutter_recipe.width,
        stonecutter_recipe.height,
        &stonecutter_recipe.rgba,
    )?;
    let villager = gui_texture(
        roots,
        "textures/gui/container/villager.png",
        "minecraft:textures/gui/container/villager",
    )?;
    renderer.upload_hud_villager_background(villager.width, villager.height, &villager.rgba)?;
    let villager_out_of_stock = hud_sprite(&sprites, "container/villager/out_of_stock")?;
    renderer.upload_hud_villager_out_of_stock(
        villager_out_of_stock.width,
        villager_out_of_stock.height,
        &villager_out_of_stock.rgba,
    )?;
    let villager_experience_bar_background =
        hud_sprite(&sprites, "container/villager/experience_bar_background")?;
    renderer.upload_hud_villager_experience_bar_background(
        villager_experience_bar_background.width,
        villager_experience_bar_background.height,
        &villager_experience_bar_background.rgba,
    )?;
    let villager_experience_bar_current =
        hud_sprite(&sprites, "container/villager/experience_bar_current")?;
    renderer.upload_hud_villager_experience_bar_current(
        villager_experience_bar_current.width,
        villager_experience_bar_current.height,
        &villager_experience_bar_current.rgba,
    )?;
    let villager_experience_bar_result =
        hud_sprite(&sprites, "container/villager/experience_bar_result")?;
    renderer.upload_hud_villager_experience_bar_result(
        villager_experience_bar_result.width,
        villager_experience_bar_result.height,
        &villager_experience_bar_result.rgba,
    )?;
    let villager_scroller = hud_sprite(&sprites, "container/villager/scroller")?;
    renderer.upload_hud_villager_scroller(
        villager_scroller.width,
        villager_scroller.height,
        &villager_scroller.rgba,
    )?;
    let villager_scroller_disabled = hud_sprite(&sprites, "container/villager/scroller_disabled")?;
    renderer.upload_hud_villager_scroller_disabled(
        villager_scroller_disabled.width,
        villager_scroller_disabled.height,
        &villager_scroller_disabled.rgba,
    )?;
    let villager_trade_arrow = hud_sprite(&sprites, "container/villager/trade_arrow")?;
    renderer.upload_hud_villager_trade_arrow(
        villager_trade_arrow.width,
        villager_trade_arrow.height,
        &villager_trade_arrow.rgba,
    )?;
    let villager_trade_arrow_out_of_stock =
        hud_sprite(&sprites, "container/villager/trade_arrow_out_of_stock")?;
    renderer.upload_hud_villager_trade_arrow_out_of_stock(
        villager_trade_arrow_out_of_stock.width,
        villager_trade_arrow_out_of_stock.height,
        &villager_trade_arrow_out_of_stock.rgba,
    )?;
    let villager_discount_strikethrough =
        hud_sprite(&sprites, "container/villager/discount_strikethrough")?;
    renderer.upload_hud_villager_discount_strikethrough(
        villager_discount_strikethrough.width,
        villager_discount_strikethrough.height,
        &villager_discount_strikethrough.rgba,
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
    let jump_bar_background = hud_sprite(&sprites, "hud/jump_bar_background")?;
    renderer.upload_hud_jump_bar_background(
        jump_bar_background.width,
        jump_bar_background.height,
        &jump_bar_background.rgba,
    )?;
    let jump_bar_progress = hud_sprite(&sprites, "hud/jump_bar_progress")?;
    renderer.upload_hud_jump_bar_progress(
        jump_bar_progress.width,
        jump_bar_progress.height,
        &jump_bar_progress.rgba,
    )?;
    let jump_bar_cooldown = hud_sprite(&sprites, "hud/jump_bar_cooldown")?;
    renderer.upload_hud_jump_bar_cooldown(
        jump_bar_cooldown.width,
        jump_bar_cooldown.height,
        &jump_bar_cooldown.rgba,
    )?;
    // Player heart sprites: every `HudHeartKind` × hardcore × half combination
    // (vanilla `hud/heart/*`, Gui.java:1334-1393). Blink variants are skipped
    // (blink is deferred). `Container` has no distinct half sprite, so its half
    // slot is not loaded (the renderer normalizes container half -> full).
    let heart_full = hud_sprite(&sprites, "hud/heart/full")?;
    for kind in HudHeartKind::ALL {
        for hardcore in [false, true] {
            for half in [false, true] {
                if matches!(kind, HudHeartKind::Container) && half {
                    continue;
                }
                let name = kind.sprite_name(hardcore, half, false);
                let sprite = hud_sprite(&sprites, &format!("hud/heart/{name}"))?;
                renderer.upload_hud_heart_sprite(
                    kind,
                    hardcore,
                    half,
                    sprite.width,
                    sprite.height,
                    &sprite.rgba,
                )?;
            }
        }
    }
    let food_empty = hud_sprite(&sprites, "hud/food_empty")?;
    renderer.upload_hud_food_empty(food_empty.width, food_empty.height, &food_empty.rgba)?;
    let food_full = hud_sprite(&sprites, "hud/food_full")?;
    renderer.upload_hud_food_full(food_full.width, food_full.height, &food_full.rgba)?;
    let food_half = hud_sprite(&sprites, "hud/food_half")?;
    renderer.upload_hud_food_half(food_half.width, food_half.height, &food_half.rgba)?;
    // Armor bar icons (vanilla `ARMOR_*_SPRITE`, Gui.java:94-96).
    let armor_empty = hud_sprite(&sprites, "hud/armor_empty")?;
    renderer.upload_hud_armor_empty(armor_empty.width, armor_empty.height, &armor_empty.rgba)?;
    let armor_half = hud_sprite(&sprites, "hud/armor_half")?;
    renderer.upload_hud_armor_half(armor_half.width, armor_half.height, &armor_half.rgba)?;
    let armor_full = hud_sprite(&sprites, "hud/armor_full")?;
    renderer.upload_hud_armor_full(armor_full.width, armor_full.height, &armor_full.rgba)?;
    // Air bubble icons (vanilla `AIR_SPRITE` / `AIR_POPPING_SPRITE` /
    // `AIR_EMPTY_SPRITE`, Gui.java:103-105).
    let air_bubble = hud_sprite(&sprites, "hud/air")?;
    renderer.upload_hud_air_bubble(air_bubble.width, air_bubble.height, &air_bubble.rgba)?;
    let air_bubble_bursting = hud_sprite(&sprites, "hud/air_bursting")?;
    renderer.upload_hud_air_bubble_bursting(
        air_bubble_bursting.width,
        air_bubble_bursting.height,
        &air_bubble_bursting.rgba,
    )?;
    let air_bubble_empty = hud_sprite(&sprites, "hud/air_empty")?;
    renderer.upload_hud_air_bubble_empty(
        air_bubble_empty.width,
        air_bubble_empty.height,
        &air_bubble_empty.rgba,
    )?;
    // Vehicle heart icons (vanilla `HEART_VEHICLE_*_SPRITE`, Gui.java:106-108).
    let heart_vehicle_container = hud_sprite(&sprites, "hud/heart/vehicle_container")?;
    renderer.upload_hud_heart_vehicle_container(
        heart_vehicle_container.width,
        heart_vehicle_container.height,
        &heart_vehicle_container.rgba,
    )?;
    let heart_vehicle_full = hud_sprite(&sprites, "hud/heart/vehicle_full")?;
    renderer.upload_hud_heart_vehicle_full(
        heart_vehicle_full.width,
        heart_vehicle_full.height,
        &heart_vehicle_full.rgba,
    )?;
    let heart_vehicle_half = hud_sprite(&sprites, "hud/heart/vehicle_half")?;
    renderer.upload_hud_heart_vehicle_half(
        heart_vehicle_half.width,
        heart_vehicle_half.height,
        &heart_vehicle_half.rgba,
    )?;
    // Hunger-effect food variants (vanilla `FOOD_*_HUNGER_SPRITE`, Gui.java:97-99).
    let food_empty_hunger = hud_sprite(&sprites, "hud/food_empty_hunger")?;
    renderer.upload_hud_food_empty_hunger(
        food_empty_hunger.width,
        food_empty_hunger.height,
        &food_empty_hunger.rgba,
    )?;
    let food_full_hunger = hud_sprite(&sprites, "hud/food_full_hunger")?;
    renderer.upload_hud_food_full_hunger(
        food_full_hunger.width,
        food_full_hunger.height,
        &food_full_hunger.rgba,
    )?;
    let food_half_hunger = hud_sprite(&sprites, "hud/food_half_hunger")?;
    renderer.upload_hud_food_half_hunger(
        food_half_hunger.width,
        food_half_hunger.height,
        &food_half_hunger.rgba,
    )?;
    // The 22 vanilla boss-bar sheets (`BossHealthOverlay`'s sprite arrays):
    // a background/progress pair per `BossBarColor` plus one per notched
    // `BossBarOverlay`, all plain 182x5 GUI-atlas sprites.
    for color in HudBossBarColor::ALL {
        let background = hud_sprite(&sprites, &format!("boss_bar/{}_background", color.name()))?;
        renderer.upload_hud_boss_bar_background(
            color,
            background.width,
            background.height,
            &background.rgba,
        )?;
        let progress = hud_sprite(&sprites, &format!("boss_bar/{}_progress", color.name()))?;
        renderer.upload_hud_boss_bar_progress(
            color,
            progress.width,
            progress.height,
            &progress.rgba,
        )?;
    }
    for overlay in HudBossBarOverlay::NOTCHED {
        let background = hud_sprite(&sprites, &format!("boss_bar/{}_background", overlay.name()))?;
        renderer.upload_hud_boss_bar_notched_background(
            overlay,
            background.width,
            background.height,
            &background.rgba,
        )?;
        let progress = hud_sprite(&sprites, &format!("boss_bar/{}_progress", overlay.name()))?;
        renderer.upload_hud_boss_bar_notched_progress(
            overlay,
            progress.width,
            progress.height,
            &progress.rgba,
        )?;
    }
    let ascii_font = load_ascii_font_texture(roots)?;
    let digit_atlas = hud_ascii_digit_atlas_from_image(&ascii_font)?;
    renderer.upload_hud_digit_atlas(
        digit_atlas.width,
        digit_atlas.height,
        &digit_atlas.rgba,
        digit_atlas.glyphs,
    )?;
    // The `font/default.json` bitmap provider chain baked into one multi-page
    // glyph atlas; HUD text and map decoration labels share the same texture.
    let font_atlas = load_hud_font_atlas(roots)?;
    renderer.upload_hud_font_atlas(
        font_atlas.width,
        font_atlas.height,
        &font_atlas.rgba,
        font_atlas.glyphs,
    )?;
    renderer.upload_item_frame_map_text_font(font_atlas.width, font_atlas.height, &font_atlas.rgba);
    tracing::info!(
        crosshair = ?(crosshair.width, crosshair.height),
        hotbar = ?(hotbar.width, hotbar.height),
        inventory = ?(inventory.width, inventory.height),
        generic_container = ?(generic_container.width, generic_container.height),
        cartography_table = ?(cartography_table.width, cartography_table.height),
        loom = ?(loom.width, loom.height),
        crafter = ?(crafter.width, crafter.height),
        anvil = ?(anvil.width, anvil.height),
        enchanting_table = ?(enchanting_table.width, enchanting_table.height),
        beacon = ?(beacon.width, beacon.height),
        furnace = ?(furnace.width, furnace.height),
        blast_furnace = ?(blast_furnace.width, blast_furnace.height),
        smoker = ?(smoker.width, smoker.height),
        brewing_stand = ?(brewing_stand.width, brewing_stand.height),
        grindstone = ?(grindstone.width, grindstone.height),
        stonecutter = ?(stonecutter.width, stonecutter.height),
        villager = ?(villager.width, villager.height),
        experience = ?(experience_background.width, experience_background.height),
        heart = ?(heart_full.width, heart_full.height),
        food = ?(food_full.width, food_full.height),
        digits = ?(digit_atlas.width, digit_atlas.height),
        font = ?(font_atlas.width, font_atlas.height),
        "loaded vanilla HUD sprites"
    );
    Ok(())
}

fn load_hanging_sign_backgrounds(
    renderer: &mut bbb_renderer::Renderer,
    roots: &PackRoots,
) -> Result<()> {
    for (wood, name) in HANGING_SIGN_BACKGROUNDS {
        let path = format!("textures/gui/hanging_signs/{name}.png");
        let id = format!("minecraft:textures/gui/hanging_signs/{name}");
        let texture = gui_texture(roots, &path, &id)?;
        renderer.upload_hud_hanging_sign_background(
            *wood,
            texture.width,
            texture.height,
            &texture.rgba,
        )?;
    }
    Ok(())
}

const HANGING_SIGN_BACKGROUNDS: &[(SignModelWood, &str)] = &[
    (SignModelWood::Oak, "oak"),
    (SignModelWood::Spruce, "spruce"),
    (SignModelWood::Birch, "birch"),
    (SignModelWood::Acacia, "acacia"),
    (SignModelWood::Cherry, "cherry"),
    (SignModelWood::Jungle, "jungle"),
    (SignModelWood::DarkOak, "dark_oak"),
    (SignModelWood::PaleOak, "pale_oak"),
    (SignModelWood::Crimson, "crimson"),
    (SignModelWood::Warped, "warped"),
    (SignModelWood::Mangrove, "mangrove"),
    (SignModelWood::Bamboo, "bamboo"),
];

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

/// Extracts a HUD sprite's nine-slice scaling from its `gui.scaling` mcmeta, failing loudly if the
/// sprite is not declared `nine_slice` (the tooltip background/frame sprites always are).
fn hud_nine_slice_scaling(sprite: &SpriteImage) -> Result<HudNineSliceScaling> {
    match sprite.gui_metadata.scaling {
        SpriteGuiScaling::NineSlice {
            width,
            height,
            border,
            stretch_inner,
        } => Ok(HudNineSliceScaling {
            sprite_width: width,
            sprite_height: height,
            border_left: border.left,
            border_top: border.top,
            border_right: border.right,
            border_bottom: border.bottom,
            stretch_inner,
        }),
        other => bail!(
            "HUD sprite {} has gui scaling {:?}, expected nine_slice",
            sprite.id,
            other
        ),
    }
}

fn gui_texture(roots: &PackRoots, path: &str, id: &str) -> Result<SpriteImage> {
    let location = ResourceLocation::parse(path)?;
    let resource = roots
        .resource_stack()
        .get_resource(&location)
        .with_context(|| format!("missing GUI texture minecraft:{path}"))?;
    SpriteImage::from_png_file(id, resource.path)
}

fn missing_texture_rgba(width: u32, height: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((width as usize) * (height as usize) * 4);
    for y in 0..height {
        for x in 0..width {
            if (y < height / 2) ^ (x < width / 2) {
                rgba.extend_from_slice(&[0xf8, 0x00, 0xf8, 0xff]);
            } else {
                rgba.extend_from_slice(&[0x00, 0x00, 0x00, 0xff]);
            }
        }
    }
    rgba
}

#[cfg(test)]
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

#[cfg(test)]
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
