use std::collections::{BTreeMap, BTreeSet, HashMap};

use anyhow::{bail, Context, Result};
use bbb_pack::{PackRoots, PanoramaCatalog, ResourceLocation, SpriteGuiScaling, SpriteImage};
use bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_TYPES;
use bbb_renderer::{
    HudAdvancementBackgroundTexture, HudAdvancementHoverBoxSprite, HudAdvancementTabSprite,
    HudAdvancementWidgetFrameSprite, HudBossBarColor, HudBossBarOverlay, HudGuiSpriteScaling,
    HudHeartKind, HudLocatorStyle, HudNineSliceScaling, HudPlayerListPingSprite,
    HudStatsScrollbarSprite, HudStatsSprite, HudTutorialToastIcon, StandaloneTextureSampling,
    VIGNETTE_TEXTURE_PATH,
};
use bbb_world::vanilla_sign_wood_kinds;

use bbb_item_model::font::{
    hud_ascii_digit_atlas_from_image, load_ascii_font_texture, load_hud_font_atlas,
};

use crate::sign_scene::sign_model_wood;

const SPYGLASS_SCOPE_TEXTURE_PATH: &str = "textures/misc/spyglass_scope.png";
/// The `Gui.extractTextureOverlay` textures a vanilla server can reach: the
/// powder-snow freeze outline and the only vanilla item `Equippable`
/// `cameraOverlay` (`carved_pumpkin`).
const CAMERA_OVERLAY_TEXTURE_IDS: &[&str] = &[
    crate::runtime::VANILLA_POWDER_SNOW_OUTLINE_TEXTURE,
    "minecraft:textures/misc/pumpkinblur.png",
];

macro_rules! upload_standalone_hud {
    ($renderer:expr, $method:ident, $image:expr) => {
        $renderer.$method(
            $image.width,
            $image.height,
            &$image.rgba,
            standalone_sampling($image),
        )
    };
    ($renderer:expr, $method:ident, $key:expr, $image:expr) => {
        $renderer.$method(
            $key,
            $image.width,
            $image.height,
            &$image.rgba,
            standalone_sampling($image),
        )
    };
}

pub(crate) fn load_hud_textures(renderer: &mut bbb_renderer::Renderer, roots: Option<&PackRoots>) {
    let Some(roots) = roots else {
        tracing::warn!("continuing without vanilla HUD sprites because pack roots are unavailable");
        return;
    };
    if let Err(err) = try_load_hud_textures(renderer, roots) {
        tracing::warn!(?err, "continuing without vanilla HUD sprites");
    }
}

pub(crate) fn load_panorama(renderer: &mut bbb_renderer::Renderer, roots: Option<&PackRoots>) {
    let Some(roots) = roots else {
        tracing::warn!("continuing without panorama because pack roots are unavailable");
        return;
    };
    let result = roots
        .load_panorama_catalog()
        .context("prepare startup panorama resources")
        .and_then(|catalog| upload_panorama(renderer, &catalog));
    if let Err(error) = result {
        tracing::warn!(%error, "continuing without panorama; menu background remains active");
    }
}

pub(crate) fn upload_panorama(
    renderer: &mut bbb_renderer::Renderer,
    catalog: &PanoramaCatalog,
) -> Result<()> {
    let faces = catalog.faces();
    let face_rgba = std::array::from_fn(|index| faces[index].rgba.as_slice());
    let overlay = catalog.overlay();
    renderer
        .upload_hud_panorama(
            faces[0].width,
            faces[0].height,
            face_rgba,
            overlay.width,
            overlay.height,
            &overlay.rgba,
            standalone_sampling(overlay),
        )
        .context("upload panorama GPU resources")
}

pub(crate) fn try_load_hud_textures(
    renderer: &mut bbb_renderer::Renderer,
    roots: &PackRoots,
) -> Result<()> {
    let sprites = load_gui_sprites(roots)?;
    let vignette = load_vignette_image(roots)?;
    renderer.upload_hud_vignette_texture_with_sampling(
        vignette.width,
        vignette.height,
        &vignette.rgba,
        standalone_sampling(&vignette),
    )?;
    let underwater = gui_texture(
        roots,
        "textures/misc/underwater.png",
        "minecraft:textures/misc/underwater",
    )?;
    renderer.upload_screen_effect_underwater_texture(
        underwater.width,
        underwater.height,
        &underwater.rgba,
    )?;
    let spyglass_scope = load_spyglass_scope_image(roots)?;
    upload_standalone_hud!(
        renderer,
        upload_hud_spyglass_scope_with_sampling,
        &spyglass_scope
    )?;
    let nausea = gui_texture(
        roots,
        "textures/misc/nausea.png",
        "minecraft:textures/misc/nausea",
    )?;
    upload_standalone_hud!(renderer, upload_hud_nausea_overlay_with_sampling, &nausea)?;
    for texture_id in CAMERA_OVERLAY_TEXTURE_IDS {
        let path = texture_id.strip_prefix("minecraft:").unwrap_or(texture_id);
        let image = gui_texture(roots, path, texture_id)?;
        renderer.upload_hud_camera_overlay_texture_with_sampling(
            texture_id,
            image.width,
            image.height,
            &image.rgba,
            standalone_sampling(&image),
        )?;
    }
    let win_logo = gui_texture(
        roots,
        "textures/gui/title/minecraft.png",
        "minecraft:textures/gui/title/minecraft",
    )?;
    upload_standalone_hud!(renderer, upload_hud_win_logo_with_sampling, &win_logo)?;
    let win_easter_egg_logo = gui_texture(
        roots,
        "textures/gui/title/minceraft.png",
        "minecraft:textures/gui/title/minceraft",
    )?;
    upload_standalone_hud!(
        renderer,
        upload_hud_win_easter_egg_logo_with_sampling,
        &win_easter_egg_logo
    )?;
    let win_edition = gui_texture(
        roots,
        "textures/gui/title/edition.png",
        "minecraft:textures/gui/title/edition",
    )?;
    upload_standalone_hud!(renderer, upload_hud_win_edition_with_sampling, &win_edition)?;
    let credits_vignette = gui_texture(
        roots,
        "textures/misc/credits_vignette.png",
        "minecraft:textures/misc/credits_vignette",
    )?;
    renderer.upload_hud_win_credits_vignette_with_sampling(
        credits_vignette.width,
        credits_vignette.height,
        &credits_vignette.rgba,
        standalone_sampling(&credits_vignette),
    )?;
    let crosshair = hud_sprite(&sprites, "hud/crosshair")?;
    renderer.upload_hud_crosshair(crosshair.width, crosshair.height, &crosshair.rgba)?;
    let crosshair_attack_full = hud_sprite(&sprites, "hud/crosshair_attack_indicator_full")?;
    renderer.upload_hud_crosshair_attack_indicator_full(
        crosshair_attack_full.width,
        crosshair_attack_full.height,
        &crosshair_attack_full.rgba,
    )?;
    let crosshair_attack_background =
        hud_sprite(&sprites, "hud/crosshair_attack_indicator_background")?;
    renderer.upload_hud_crosshair_attack_indicator_background(
        crosshair_attack_background.width,
        crosshair_attack_background.height,
        &crosshair_attack_background.rgba,
    )?;
    let crosshair_attack_progress =
        hud_sprite(&sprites, "hud/crosshair_attack_indicator_progress")?;
    renderer.upload_hud_crosshair_attack_indicator_progress(
        crosshair_attack_progress.width,
        crosshair_attack_progress.height,
        &crosshair_attack_progress.rgba,
    )?;
    let hotbar = hud_sprite(&sprites, "hud/hotbar")?;
    renderer.upload_hud_hotbar(hotbar.width, hotbar.height, &hotbar.rgba)?;
    let hotbar_selection = hud_sprite(&sprites, "hud/hotbar_selection")?;
    renderer.upload_hud_hotbar_selection(
        hotbar_selection.width,
        hotbar_selection.height,
        &hotbar_selection.rgba,
    )?;
    let hotbar_offhand_left = hud_sprite(&sprites, "hud/hotbar_offhand_left")?;
    renderer.upload_hud_hotbar_offhand_left(
        hotbar_offhand_left.width,
        hotbar_offhand_left.height,
        &hotbar_offhand_left.rgba,
    )?;
    let hotbar_offhand_right = hud_sprite(&sprites, "hud/hotbar_offhand_right")?;
    renderer.upload_hud_hotbar_offhand_right(
        hotbar_offhand_right.width,
        hotbar_offhand_right.height,
        &hotbar_offhand_right.rgba,
    )?;
    let hotbar_attack_background = hud_sprite(&sprites, "hud/hotbar_attack_indicator_background")?;
    renderer.upload_hud_hotbar_attack_indicator_background(
        hotbar_attack_background.width,
        hotbar_attack_background.height,
        &hotbar_attack_background.rgba,
    )?;
    let hotbar_attack_progress = hud_sprite(&sprites, "hud/hotbar_attack_indicator_progress")?;
    renderer.upload_hud_hotbar_attack_indicator_progress(
        hotbar_attack_progress.width,
        hotbar_attack_progress.height,
        &hotbar_attack_progress.rgba,
    )?;
    let spectator_close = hud_sprite(&sprites, "spectator/close")?;
    renderer.upload_hud_spectator_close(
        spectator_close.width,
        spectator_close.height,
        &spectator_close.rgba,
    )?;
    let spectator_scroll_left = hud_sprite(&sprites, "spectator/scroll_left")?;
    renderer.upload_hud_spectator_scroll_left(
        spectator_scroll_left.width,
        spectator_scroll_left.height,
        &spectator_scroll_left.rgba,
    )?;
    let spectator_scroll_right = hud_sprite(&sprites, "spectator/scroll_right")?;
    renderer.upload_hud_spectator_scroll_right(
        spectator_scroll_right.width,
        spectator_scroll_right.height,
        &spectator_scroll_right.rgba,
    )?;
    let spectator_player = hud_sprite(&sprites, "spectator/teleport_to_player")?;
    renderer.upload_hud_spectator_teleport_to_player(
        spectator_player.width,
        spectator_player.height,
        &spectator_player.rgba,
    )?;
    let spectator_team = hud_sprite(&sprites, "spectator/teleport_to_team")?;
    renderer.upload_hud_spectator_teleport_to_team(
        spectator_team.width,
        spectator_team.height,
        &spectator_team.rgba,
    )?;
    let chat_modified = hud_sprite(&sprites, "icon/chat_modified")?;
    renderer.upload_hud_chat_modified_icon(
        chat_modified.width,
        chat_modified.height,
        &chat_modified.rgba,
    )?;
    let draft_report = hud_sprite(&sprites, "icon/draft_report")?;
    renderer.upload_hud_draft_report_icon(
        draft_report.width,
        draft_report.height,
        &draft_report.rgba,
    )?;
    for kind in HudPlayerListPingSprite::ALL {
        let ping = hud_sprite(&sprites, kind.sprite_path())?;
        renderer.upload_hud_player_list_ping_sprite(kind, ping.width, ping.height, &ping.rgba)?;
    }
    let demo_popup_background = hud_sprite(&sprites, "popup/background")?;
    let demo_popup_scaling = hud_nine_slice_scaling(demo_popup_background)?;
    renderer.upload_hud_demo_popup_background(
        demo_popup_background.width,
        demo_popup_background.height,
        &demo_popup_background.rgba,
        demo_popup_scaling,
    )?;
    let inventory = gui_texture(
        roots,
        "textures/gui/container/inventory.png",
        "minecraft:textures/gui/container/inventory",
    )?;
    upload_standalone_hud!(
        renderer,
        upload_hud_inventory_background_with_sampling,
        &inventory
    )?;
    let creative_items = gui_texture(
        roots,
        "textures/gui/container/creative_inventory/tab_items.png",
        "minecraft:textures/gui/container/creative_inventory/tab_items",
    )?;
    upload_standalone_hud!(
        renderer,
        upload_hud_creative_items_background_with_sampling,
        &creative_items
    )?;
    let creative_item_search = gui_texture(
        roots,
        "textures/gui/container/creative_inventory/tab_item_search.png",
        "minecraft:textures/gui/container/creative_inventory/tab_item_search",
    )?;
    upload_standalone_hud!(
        renderer,
        upload_hud_creative_item_search_background_with_sampling,
        &creative_item_search
    )?;
    let creative_inventory = gui_texture(
        roots,
        "textures/gui/container/creative_inventory/tab_inventory.png",
        "minecraft:textures/gui/container/creative_inventory/tab_inventory",
    )?;
    upload_standalone_hud!(
        renderer,
        upload_hud_creative_inventory_background_with_sampling,
        &creative_inventory
    )?;
    for disabled in [false, true] {
        let suffix = if disabled { "_disabled" } else { "" };
        let scroller = hud_sprite(
            &sprites,
            &format!("container/creative_inventory/scroller{suffix}"),
        )?;
        renderer.upload_hud_creative_scroller(
            disabled,
            scroller.width,
            scroller.height,
            &scroller.rgba,
        )?;
    }
    for (row_name, row) in [
        ("top", bbb_renderer::HudCreativeInventoryTabRow::Top),
        ("bottom", bbb_renderer::HudCreativeInventoryTabRow::Bottom),
    ] {
        for selected in [false, true] {
            let selection = if selected { "selected" } else { "unselected" };
            for column in 0_u8..7 {
                let tab = hud_sprite(
                    &sprites,
                    &format!(
                        "container/creative_inventory/tab_{row_name}_{selection}_{}",
                        column + 1
                    ),
                )?;
                renderer.upload_hud_creative_tab(
                    row, selected, column, tab.width, tab.height, &tab.rgba,
                )?;
            }
        }
    }
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
    for (style, (background_id, frame_id)) in tooltip_style_sprite_pairs(&sprites) {
        let background = sprites
            .get(&background_id)
            .with_context(|| format!("missing discovered tooltip background {background_id}"))?;
        let frame = sprites
            .get(&frame_id)
            .with_context(|| format!("missing discovered tooltip frame {frame_id}"))?;
        renderer.upload_hud_tooltip_style(
            style,
            background.width,
            background.height,
            &background.rgba,
            hud_gui_sprite_scaling(background),
            frame.width,
            frame.height,
            &frame.rgba,
            hud_gui_sprite_scaling(frame),
        )?;
    }
    let system_toast = hud_sprite(&sprites, "toast/system")?;
    let system_toast_scaling = hud_nine_slice_scaling(system_toast)?;
    renderer.upload_hud_system_toast_background(
        system_toast.width,
        system_toast.height,
        &system_toast.rgba,
        system_toast_scaling,
    )?;
    let advancement_toast = hud_sprite(&sprites, "toast/advancement")?;
    renderer.upload_hud_advancement_toast_background(
        advancement_toast.width,
        advancement_toast.height,
        &advancement_toast.rgba,
    )?;
    let recipe_toast = hud_sprite(&sprites, "toast/recipe")?;
    renderer.upload_hud_recipe_toast_background(
        recipe_toast.width,
        recipe_toast.height,
        &recipe_toast.rgba,
    )?;
    let tutorial_toast = hud_sprite(&sprites, "toast/tutorial")?;
    renderer.upload_hud_tutorial_toast_background(
        tutorial_toast.width,
        tutorial_toast.height,
        &tutorial_toast.rgba,
        hud_nine_slice_scaling(tutorial_toast)?,
    )?;
    for (icon, sprite_name) in [
        (HudTutorialToastIcon::MovementKeys, "toast/movement_keys"),
        (HudTutorialToastIcon::Mouse, "toast/mouse"),
        (HudTutorialToastIcon::Tree, "toast/tree"),
        (HudTutorialToastIcon::RecipeBook, "toast/recipe_book"),
        (HudTutorialToastIcon::WoodenPlanks, "toast/wooden_planks"),
    ] {
        let sprite = hud_sprite(&sprites, sprite_name)?;
        renderer.upload_hud_tutorial_toast_icon(icon, sprite.width, sprite.height, &sprite.rgba)?;
    }
    let generic_container = gui_texture(
        roots,
        "textures/gui/container/generic_54.png",
        "minecraft:textures/gui/container/generic_54",
    )?;
    upload_standalone_hud!(
        renderer,
        upload_hud_generic_container_background_with_sampling,
        &generic_container
    )?;
    let dispenser = gui_texture(
        roots,
        "textures/gui/container/dispenser.png",
        "minecraft:textures/gui/container/dispenser",
    )?;
    upload_standalone_hud!(
        renderer,
        upload_hud_dispenser_background_with_sampling,
        &dispenser
    )?;
    let crafting_table = gui_texture(
        roots,
        "textures/gui/container/crafting_table.png",
        "minecraft:textures/gui/container/crafting_table",
    )?;
    upload_standalone_hud!(
        renderer,
        upload_hud_crafting_table_background_with_sampling,
        &crafting_table
    )?;
    let advancements_window = gui_texture(
        roots,
        "textures/gui/advancements/window.png",
        "minecraft:textures/gui/advancements/window",
    )?;
    upload_standalone_hud!(
        renderer,
        upload_hud_advancements_window_with_sampling,
        &advancements_window
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
        upload_standalone_hud!(
            renderer,
            upload_hud_advancement_background_with_sampling,
            background,
            &texture
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
    for hover_box_sprite in HudAdvancementHoverBoxSprite::ALL {
        let sprite = hud_sprite(&sprites, hover_box_sprite.sprite_path())?;
        renderer.upload_hud_advancement_hover_box(
            hover_box_sprite,
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
    upload_standalone_hud!(
        renderer,
        upload_hud_recipe_book_background_with_sampling,
        &recipe_book
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
    let widget_preedit = hud_sprite(&sprites, "widget/preedit")?;
    let widget_preedit_scaling = hud_nine_slice_scaling(widget_preedit)?;
    renderer.upload_hud_widget_preedit(
        widget_preedit.width,
        widget_preedit.height,
        &widget_preedit.rgba,
        widget_preedit_scaling,
    )?;
    let widget_button = hud_sprite(&sprites, "widget/button")?;
    renderer.upload_hud_widget_button(
        widget_button.width,
        widget_button.height,
        &widget_button.rgba,
    )?;
    let widget_button_disabled = hud_sprite(&sprites, "widget/button_disabled")?;
    renderer.upload_hud_widget_button_disabled(
        widget_button_disabled.width,
        widget_button_disabled.height,
        &widget_button_disabled.rgba,
    )?;
    let widget_button_highlighted = hud_sprite(&sprites, "widget/button_highlighted")?;
    renderer.upload_hud_widget_button_highlighted(
        widget_button_highlighted.width,
        widget_button_highlighted.height,
        &widget_button_highlighted.rgba,
    )?;
    let social_background = hud_sprite(&sprites, "social_interactions/background")?;
    let social_background_scaling = hud_nine_slice_scaling(social_background)?;
    renderer.upload_hud_social_interactions_background(
        social_background.width,
        social_background.height,
        &social_background.rgba,
        social_background_scaling,
    )?;
    let social_search = hud_sprite(&sprites, "icon/search")?;
    renderer.upload_hud_social_interactions_search(
        social_search.width,
        social_search.height,
        &social_search.rgba,
    )?;
    let social_mute = hud_sprite(&sprites, "social_interactions/mute_button")?;
    renderer.upload_hud_social_interactions_mute(
        social_mute.width,
        social_mute.height,
        &social_mute.rgba,
    )?;
    let social_mute_highlighted =
        hud_sprite(&sprites, "social_interactions/mute_button_highlighted")?;
    renderer.upload_hud_social_interactions_mute_highlighted(
        social_mute_highlighted.width,
        social_mute_highlighted.height,
        &social_mute_highlighted.rgba,
    )?;
    let social_unmute = hud_sprite(&sprites, "social_interactions/unmute_button")?;
    renderer.upload_hud_social_interactions_unmute(
        social_unmute.width,
        social_unmute.height,
        &social_unmute.rgba,
    )?;
    let social_unmute_highlighted =
        hud_sprite(&sprites, "social_interactions/unmute_button_highlighted")?;
    renderer.upload_hud_social_interactions_unmute_highlighted(
        social_unmute_highlighted.width,
        social_unmute_highlighted.height,
        &social_unmute_highlighted.rgba,
    )?;
    let dialog_warning_button = hud_sprite(&sprites, "dialog/warning_button")?;
    renderer.upload_hud_dialog_warning_button(
        dialog_warning_button.width,
        dialog_warning_button.height,
        &dialog_warning_button.rgba,
    )?;
    let dialog_warning_button_disabled = hud_sprite(&sprites, "dialog/warning_button_disabled")?;
    renderer.upload_hud_dialog_warning_button_disabled(
        dialog_warning_button_disabled.width,
        dialog_warning_button_disabled.height,
        &dialog_warning_button_disabled.rgba,
    )?;
    let dialog_warning_button_highlighted =
        hud_sprite(&sprites, "dialog/warning_button_highlighted")?;
    renderer.upload_hud_dialog_warning_button_highlighted(
        dialog_warning_button_highlighted.width,
        dialog_warning_button_highlighted.height,
        &dialog_warning_button_highlighted.rgba,
    )?;
    let widget_checkbox = hud_sprite(&sprites, "widget/checkbox")?;
    renderer.upload_hud_widget_checkbox(
        widget_checkbox.width,
        widget_checkbox.height,
        &widget_checkbox.rgba,
    )?;
    let widget_checkbox_highlighted = hud_sprite(&sprites, "widget/checkbox_highlighted")?;
    renderer.upload_hud_widget_checkbox_highlighted(
        widget_checkbox_highlighted.width,
        widget_checkbox_highlighted.height,
        &widget_checkbox_highlighted.rgba,
    )?;
    let widget_checkbox_selected = hud_sprite(&sprites, "widget/checkbox_selected")?;
    renderer.upload_hud_widget_checkbox_selected(
        widget_checkbox_selected.width,
        widget_checkbox_selected.height,
        &widget_checkbox_selected.rgba,
    )?;
    let widget_checkbox_selected_highlighted =
        hud_sprite(&sprites, "widget/checkbox_selected_highlighted")?;
    renderer.upload_hud_widget_checkbox_selected_highlighted(
        widget_checkbox_selected_highlighted.width,
        widget_checkbox_selected_highlighted.height,
        &widget_checkbox_selected_highlighted.rgba,
    )?;
    let widget_slider = hud_sprite(&sprites, "widget/slider")?;
    renderer.upload_hud_widget_slider(
        widget_slider.width,
        widget_slider.height,
        &widget_slider.rgba,
    )?;
    let widget_slider_highlighted = hud_sprite(&sprites, "widget/slider_highlighted")?;
    renderer.upload_hud_widget_slider_highlighted(
        widget_slider_highlighted.width,
        widget_slider_highlighted.height,
        &widget_slider_highlighted.rgba,
    )?;
    let widget_slider_handle = hud_sprite(&sprites, "widget/slider_handle")?;
    renderer.upload_hud_widget_slider_handle(
        widget_slider_handle.width,
        widget_slider_handle.height,
        &widget_slider_handle.rgba,
    )?;
    let widget_slider_handle_highlighted =
        hud_sprite(&sprites, "widget/slider_handle_highlighted")?;
    renderer.upload_hud_widget_slider_handle_highlighted(
        widget_slider_handle_highlighted.width,
        widget_slider_handle_highlighted.height,
        &widget_slider_handle_highlighted.rgba,
    )?;
    let inworld_menu_background = gui_texture(
        roots,
        "textures/gui/inworld_menu_background.png",
        "minecraft:textures/gui/inworld_menu_background",
    )?;
    upload_standalone_hud!(
        renderer,
        upload_hud_inworld_menu_background_with_sampling,
        &inworld_menu_background
    )?;
    let menu_background = gui_texture(
        roots,
        "textures/gui/menu_background.png",
        "minecraft:textures/gui/menu_background",
    )?;
    upload_standalone_hud!(
        renderer,
        upload_hud_menu_background_with_sampling,
        &menu_background
    )?;
    for stats_sprite in HudStatsSprite::ALL {
        let sprite = hud_sprite(&sprites, stats_sprite.sprite_path())?;
        renderer.upload_hud_stats_sprite(
            stats_sprite,
            sprite.width,
            sprite.height,
            &sprite.rgba,
        )?;
    }
    for scrollbar_sprite in HudStatsScrollbarSprite::ALL {
        let sprite = hud_sprite(&sprites, scrollbar_sprite.sprite_path())?;
        let scaling = hud_nine_slice_scaling(sprite)?;
        renderer.upload_hud_stats_scrollbar_sprite(
            scrollbar_sprite,
            sprite.width,
            sprite.height,
            &sprite.rgba,
            scaling,
        )?;
    }
    let game_mode_switcher = gui_texture(
        roots,
        "textures/gui/container/gamemode_switcher.png",
        "minecraft:textures/gui/container/gamemode_switcher",
    )?;
    upload_standalone_hud!(
        renderer,
        upload_hud_debug_game_mode_switcher_background_with_sampling,
        &game_mode_switcher
    )?;
    let game_mode_switcher_slot = hud_sprite(&sprites, "gamemode_switcher/slot")?;
    renderer.upload_hud_debug_game_mode_switcher_slot(
        game_mode_switcher_slot.width,
        game_mode_switcher_slot.height,
        &game_mode_switcher_slot.rgba,
    )?;
    let game_mode_switcher_selection = hud_sprite(&sprites, "gamemode_switcher/selection")?;
    renderer.upload_hud_debug_game_mode_switcher_selection(
        game_mode_switcher_selection.width,
        game_mode_switcher_selection.height,
        &game_mode_switcher_selection.rgba,
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
    upload_standalone_hud!(
        renderer,
        upload_hud_cartography_table_background_with_sampling,
        &cartography_table
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
    upload_standalone_hud!(renderer, upload_hud_loom_background_with_sampling, &loom)?;
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
    upload_standalone_hud!(
        renderer,
        upload_hud_crafter_background_with_sampling,
        &crafter
    )?;
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
    upload_standalone_hud!(renderer, upload_hud_anvil_background_with_sampling, &anvil)?;
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
    upload_standalone_hud!(
        renderer,
        upload_hud_enchanting_table_background_with_sampling,
        &enchanting_table
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
    upload_standalone_hud!(
        renderer,
        upload_hud_beacon_background_with_sampling,
        &beacon
    )?;
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
    let effect_background = hud_sprite(&sprites, "hud/effect_background")?;
    let effect_background_ambient = hud_sprite(&sprites, "hud/effect_background_ambient")?;
    let inventory_effect_background =
        hud_sprite(&sprites, "container/inventory/effect_background")?;
    let inventory_effect_background_scaling = hud_nine_slice_scaling(inventory_effect_background)?;
    let inventory_effect_background_ambient =
        hud_sprite(&sprites, "container/inventory/effect_background_ambient")?;
    let inventory_effect_background_ambient_scaling =
        hud_nine_slice_scaling(inventory_effect_background_ambient)?;
    let mob_effect_sprites = VANILLA_MOB_EFFECT_TYPES
        .iter()
        .map(|effect| {
            let path = effect
                .resource_id
                .strip_prefix("minecraft:")
                .unwrap_or(effect.resource_id);
            Ok((
                effect.id,
                hud_sprite(&sprites, &format!("mob_effect/{path}"))?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    renderer.upload_hud_effect_background(
        effect_background.width,
        effect_background.height,
        &effect_background.rgba,
    )?;
    renderer.upload_hud_effect_background_ambient(
        effect_background_ambient.width,
        effect_background_ambient.height,
        &effect_background_ambient.rgba,
    )?;
    renderer.upload_hud_inventory_effect_background(
        inventory_effect_background.width,
        inventory_effect_background.height,
        &inventory_effect_background.rgba,
        inventory_effect_background_scaling,
    )?;
    renderer.upload_hud_inventory_effect_background_ambient(
        inventory_effect_background_ambient.width,
        inventory_effect_background_ambient.height,
        &inventory_effect_background_ambient.rgba,
        inventory_effect_background_ambient_scaling,
    )?;
    let mob_effect_uploads = mob_effect_sprites
        .iter()
        .map(|(effect_id, sprite)| {
            (
                *effect_id,
                sprite.width,
                sprite.height,
                sprite.rgba.as_slice(),
            )
        })
        .collect::<Vec<_>>();
    renderer.upload_hud_mob_effect_sprites(&mob_effect_uploads)?;
    let furnace = gui_texture(
        roots,
        "textures/gui/container/furnace.png",
        "minecraft:textures/gui/container/furnace",
    )?;
    upload_standalone_hud!(
        renderer,
        upload_hud_furnace_background_with_sampling,
        &furnace
    )?;
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
    upload_standalone_hud!(
        renderer,
        upload_hud_blast_furnace_background_with_sampling,
        &blast_furnace
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
    upload_standalone_hud!(
        renderer,
        upload_hud_smoker_background_with_sampling,
        &smoker
    )?;
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
    upload_standalone_hud!(
        renderer,
        upload_hud_smithing_background_with_sampling,
        &smithing
    )?;
    let smithing_error = hud_sprite(&sprites, "container/smithing/error")?;
    renderer.upload_hud_smithing_error(
        smithing_error.width,
        smithing_error.height,
        &smithing_error.rgba,
    )?;
    for icon in bbb_renderer::HudSmithingSlotIcon::ALL {
        let sprite = hud_sprite(&sprites, icon.sprite_path())?;
        renderer.upload_hud_smithing_slot_icon(icon, sprite.width, sprite.height, &sprite.rgba)?;
    }
    let brewing_stand = gui_texture(
        roots,
        "textures/gui/container/brewing_stand.png",
        "minecraft:textures/gui/container/brewing_stand",
    )?;
    upload_standalone_hud!(
        renderer,
        upload_hud_brewing_stand_background_with_sampling,
        &brewing_stand
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
    upload_standalone_hud!(
        renderer,
        upload_hud_grindstone_background_with_sampling,
        &grindstone
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
    upload_standalone_hud!(
        renderer,
        upload_hud_hopper_background_with_sampling,
        &hopper
    )?;
    let horse = gui_texture(
        roots,
        "textures/gui/container/horse.png",
        "minecraft:textures/gui/container/horse",
    )?;
    upload_standalone_hud!(renderer, upload_hud_horse_background_with_sampling, &horse)?;
    let nautilus = gui_texture(
        roots,
        "textures/gui/container/nautilus.png",
        "minecraft:textures/gui/container/nautilus",
    )?;
    upload_standalone_hud!(
        renderer,
        upload_hud_nautilus_background_with_sampling,
        &nautilus
    )?;
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
    upload_standalone_hud!(renderer, upload_hud_book_background_with_sampling, &book)?;
    let page_backward = hud_sprite(&sprites, "widget/page_backward")?;
    renderer.upload_hud_page_backward(
        page_backward.width,
        page_backward.height,
        &page_backward.rgba,
    )?;
    let page_backward_highlighted = hud_sprite(&sprites, "widget/page_backward_highlighted")?;
    renderer.upload_hud_page_backward_highlighted(
        page_backward_highlighted.width,
        page_backward_highlighted.height,
        &page_backward_highlighted.rgba,
    )?;
    let page_forward = hud_sprite(&sprites, "widget/page_forward")?;
    renderer.upload_hud_page_forward(
        page_forward.width,
        page_forward.height,
        &page_forward.rgba,
    )?;
    let page_forward_highlighted = hud_sprite(&sprites, "widget/page_forward_highlighted")?;
    renderer.upload_hud_page_forward_highlighted(
        page_forward_highlighted.width,
        page_forward_highlighted.height,
        &page_forward_highlighted.rgba,
    )?;
    load_hanging_sign_backgrounds(renderer, roots)?;
    let shulker_box = gui_texture(
        roots,
        "textures/gui/container/shulker_box.png",
        "minecraft:textures/gui/container/shulker_box",
    )?;
    upload_standalone_hud!(
        renderer,
        upload_hud_shulker_box_background_with_sampling,
        &shulker_box
    )?;
    let stonecutter = gui_texture(
        roots,
        "textures/gui/container/stonecutter.png",
        "minecraft:textures/gui/container/stonecutter",
    )?;
    upload_standalone_hud!(
        renderer,
        upload_hud_stonecutter_background_with_sampling,
        &stonecutter
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
    upload_standalone_hud!(
        renderer,
        upload_hud_villager_background_with_sampling,
        &villager
    )?;
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
    let locator_background = hud_sprite(&sprites, "hud/locator_bar_background")?;
    renderer.upload_hud_locator_background(
        locator_background.width,
        locator_background.height,
        &locator_background.rgba,
    )?;
    let locator_arrow_up = hud_sprite(&sprites, "hud/locator_bar_arrow_up")?;
    renderer.upload_hud_locator_arrow_up(
        locator_arrow_up.width,
        locator_arrow_up.height,
        &locator_arrow_up.rgba,
    )?;
    let locator_arrow_down = hud_sprite(&sprites, "hud/locator_bar_arrow_down")?;
    renderer.upload_hud_locator_arrow_down(
        locator_arrow_down.width,
        locator_arrow_down.height,
        &locator_arrow_down.rgba,
    )?;

    let waypoint_styles = roots.load_waypoint_style_catalog()?;
    let mut locator_styles = BTreeMap::new();
    let mut locator_sprite_ids = BTreeSet::new();
    for (style_id, style) in waypoint_styles.styles() {
        locator_sprite_ids.extend(style.sprite_locations.iter().cloned());
        locator_styles.insert(
            style_id.clone(),
            HudLocatorStyle {
                near_distance: style.near_distance,
                far_distance: style.far_distance,
                sprite_ids: style.sprite_locations.clone(),
            },
        );
    }
    if let Some(missing) = sprites.get("minecraft:missingno") {
        renderer.upload_hud_locator_marker_sprite(
            missing.id.clone(),
            missing.width,
            missing.height,
            &missing.rgba,
        )?;
    } else {
        renderer.upload_hud_locator_marker_sprite(
            "minecraft:missingno".to_string(),
            16,
            16,
            &missing_texture_rgba(16, 16),
        )?;
    }
    for sprite_id in locator_sprite_ids {
        let Some(sprite) = sprites.get(&sprite_id) else {
            continue;
        };
        renderer.upload_hud_locator_marker_sprite(
            sprite_id,
            sprite.width,
            sprite.height,
            &sprite.rgba,
        )?;
    }
    renderer.set_hud_locator_styles(locator_styles);
    // Player heart sprites: every `HudHeartKind` × hardcore × half × blink
    // combination (vanilla `hud/heart/*`, Gui.java:1334-1393).
    // `Container` has no distinct half sprite, so its half slot is not loaded
    // (the renderer normalizes container half -> full).
    let heart_full = hud_sprite(&sprites, "hud/heart/full")?;
    for kind in HudHeartKind::ALL {
        for hardcore in [false, true] {
            for half in [false, true] {
                if matches!(kind, HudHeartKind::Container) && half {
                    continue;
                }
                for blinking in [false, true] {
                    let name = kind.sprite_name(hardcore, half, blinking);
                    let sprite = hud_sprite(&sprites, &format!("hud/heart/{name}"))?;
                    renderer.upload_hud_heart_sprite(
                        kind,
                        hardcore,
                        half,
                        blinking,
                        sprite.width,
                        sprite.height,
                        &sprite.rgba,
                    )?;
                }
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
        font_atlas.fonts,
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
        vignette = ?(vignette.width, vignette.height),
        "loaded vanilla HUD sprites"
    );
    Ok(())
}

fn load_hanging_sign_backgrounds(
    renderer: &mut bbb_renderer::Renderer,
    roots: &PackRoots,
) -> Result<()> {
    for wood in vanilla_sign_wood_kinds() {
        let name = wood.name();
        let path = format!("textures/gui/hanging_signs/{name}.png");
        let id = format!("minecraft:textures/gui/hanging_signs/{name}");
        let texture = gui_texture(roots, &path, &id)?;
        upload_standalone_hud!(
            renderer,
            upload_hud_hanging_sign_background_with_sampling,
            sign_model_wood(wood),
            &texture
        )?;
    }
    Ok(())
}

fn load_gui_sprites(roots: &PackRoots) -> Result<HashMap<String, SpriteImage>> {
    Ok(roots
        .load_atlas_texture_images("gui")?
        .into_iter()
        .map(|image| (image.id.clone(), image))
        .collect())
}

fn load_vignette_image(roots: &PackRoots) -> Result<SpriteImage> {
    gui_texture(
        roots,
        VIGNETTE_TEXTURE_PATH,
        "minecraft:textures/misc/vignette",
    )
}

fn load_spyglass_scope_image(roots: &PackRoots) -> Result<SpriteImage> {
    gui_texture(
        roots,
        SPYGLASS_SCOPE_TEXTURE_PATH,
        "minecraft:textures/misc/spyglass_scope",
    )
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

fn tooltip_style_sprite_pairs(
    sprites: &HashMap<String, SpriteImage>,
) -> BTreeMap<String, (String, String)> {
    let mut pairs = BTreeMap::new();
    for background_id in sprites.keys() {
        let Ok(background) = ResourceLocation::parse(background_id) else {
            continue;
        };
        let Some(style_path) = background
            .path()
            .strip_prefix("tooltip/")
            .and_then(|path| path.strip_suffix("_background"))
            .filter(|path| !path.is_empty())
        else {
            continue;
        };
        let Ok(style) = ResourceLocation::new(background.namespace(), style_path) else {
            continue;
        };
        let Ok(frame) = ResourceLocation::new(
            background.namespace(),
            format!("tooltip/{style_path}_frame"),
        ) else {
            continue;
        };
        let frame_id = frame.id();
        if sprites.contains_key(&frame_id) {
            pairs.insert(style.id(), (background_id.clone(), frame_id));
        }
    }
    pairs
}

/// Extracts a HUD sprite's nine-slice scaling from its `gui.scaling` mcmeta, failing loudly if the
/// sprite is not declared `nine_slice` (the tooltip, system-toast, and tutorial-toast sprites used
/// here are).
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

fn hud_gui_sprite_scaling(sprite: &SpriteImage) -> HudGuiSpriteScaling {
    match sprite.gui_metadata.scaling {
        SpriteGuiScaling::Stretch => HudGuiSpriteScaling::Stretch,
        SpriteGuiScaling::Tile { width, height } => HudGuiSpriteScaling::Tile { width, height },
        SpriteGuiScaling::NineSlice {
            width,
            height,
            border,
            stretch_inner,
        } => HudGuiSpriteScaling::NineSlice(HudNineSliceScaling {
            sprite_width: width,
            sprite_height: height,
            border_left: border.left,
            border_top: border.top,
            border_right: border.right,
            border_bottom: border.bottom,
            stretch_inner,
        }),
    }
}

fn gui_texture(roots: &PackRoots, path: &str, id: &str) -> Result<SpriteImage> {
    let location = ResourceLocation::parse(path)?;
    let resources = roots.resource_stack();
    let resource = resources
        .get_resource(&location)
        .with_context(|| format!("missing GUI texture minecraft:{path}"))?;
    SpriteImage::from_png_file_with_resource_stack(id, resource.path, &resources)
}

fn standalone_sampling(image: &SpriteImage) -> StandaloneTextureSampling {
    StandaloneTextureSampling::new(image.texture_metadata.blur, image.texture_metadata.clamp)
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

        for (path, width, height) in [
            ("hud/crosshair_attack_indicator_full", 16, 16),
            ("hud/crosshair_attack_indicator_background", 16, 4),
            ("hud/crosshair_attack_indicator_progress", 16, 4),
        ] {
            let image = SpriteImage::new(
                format!("minecraft:{path}"),
                width,
                height,
                vec![255; (width * height * 4) as usize],
            )
            .unwrap();
            sprites.insert(image.id.clone(), image);
            let sprite = hud_sprite(&sprites, path).unwrap();
            assert_eq!((sprite.width, sprite.height), (width, height));
        }

        let err = hud_sprite(&sprites, "gui/sprites/hud/crosshair").unwrap_err();
        assert!(err
            .to_string()
            .contains("minecraft:gui/sprites/hud/crosshair"));

        let toast =
            SpriteImage::new("minecraft:toast/system", 160, 64, vec![255; 160 * 64 * 4]).unwrap();
        sprites.insert(toast.id.clone(), toast);
        assert_eq!(
            hud_sprite(&sprites, "toast/system").unwrap().id,
            "minecraft:toast/system"
        );

        let chat_modified =
            SpriteImage::new("minecraft:icon/chat_modified", 9, 9, vec![255; 9 * 9 * 4]).unwrap();
        sprites.insert(chat_modified.id.clone(), chat_modified);
        let chat_modified = hud_sprite(&sprites, "icon/chat_modified").unwrap();
        assert_eq!(chat_modified.id, "minecraft:icon/chat_modified");
        assert_eq!((chat_modified.width, chat_modified.height), (9, 9));
        let draft_report = SpriteImage::new(
            "minecraft:icon/draft_report",
            15,
            15,
            vec![255; 15 * 15 * 4],
        )
        .unwrap();
        sprites.insert(draft_report.id.clone(), draft_report);
        let draft_report = hud_sprite(&sprites, "icon/draft_report").unwrap();
        assert_eq!(draft_report.id, "minecraft:icon/draft_report");
        assert_eq!((draft_report.width, draft_report.height), (15, 15));

        for kind in HudPlayerListPingSprite::ALL {
            let id = format!("minecraft:{}", kind.sprite_path());
            let image = SpriteImage::new(id.clone(), 10, 8, vec![255; 10 * 8 * 4]).unwrap();
            sprites.insert(image.id.clone(), image);
            let ping = hud_sprite(&sprites, kind.sprite_path()).unwrap();
            assert_eq!(ping.id, id);
            assert_eq!((ping.width, ping.height), (10, 8));
        }

        for id in [
            "dialog/warning_button",
            "dialog/warning_button_disabled",
            "dialog/warning_button_highlighted",
        ] {
            let image = SpriteImage::new(format!("minecraft:{id}"), 20, 20, vec![255; 20 * 20 * 4])
                .unwrap();
            sprites.insert(image.id.clone(), image);
            let warning = hud_sprite(&sprites, id).unwrap();
            assert_eq!(warning.id, format!("minecraft:{id}"));
            assert_eq!((warning.width, warning.height), (20, 20));
        }

        let mut preedit =
            SpriteImage::new("minecraft:widget/preedit", 200, 20, vec![255; 200 * 20 * 4]).unwrap();
        preedit.gui_metadata.scaling = SpriteGuiScaling::NineSlice {
            width: 200,
            height: 20,
            border: bbb_pack::SpriteNineSliceBorder::uniform(1),
            stretch_inner: false,
        };
        sprites.insert(preedit.id.clone(), preedit);
        let preedit = hud_sprite(&sprites, "widget/preedit").unwrap();
        assert_eq!((preedit.width, preedit.height), (200, 20));
        assert_eq!(
            hud_nine_slice_scaling(preedit).unwrap(),
            HudNineSliceScaling {
                sprite_width: 200,
                sprite_height: 20,
                border_left: 1,
                border_top: 1,
                border_right: 1,
                border_bottom: 1,
                stretch_inner: false,
            }
        );
    }

    #[test]
    fn tooltip_style_sprite_pairs_preserve_namespace_and_require_both_layers() {
        let mut sprites = HashMap::new();
        for id in [
            "minecraft:tooltip/background",
            "minecraft:tooltip/frame",
            "example:tooltip/rare_background",
            "example:tooltip/rare_frame",
            "example:tooltip/nested/epic_background",
            "example:tooltip/nested/epic_frame",
            "example:tooltip/incomplete_background",
            "other:tooltip/rare_frame",
        ] {
            let image = SpriteImage::new(id, 1, 1, vec![255, 255, 255, 255]).unwrap();
            sprites.insert(id.to_string(), image);
        }

        assert_eq!(
            tooltip_style_sprite_pairs(&sprites),
            BTreeMap::from([
                (
                    "example:nested/epic".to_string(),
                    (
                        "example:tooltip/nested/epic_background".to_string(),
                        "example:tooltip/nested/epic_frame".to_string(),
                    ),
                ),
                (
                    "example:rare".to_string(),
                    (
                        "example:tooltip/rare_background".to_string(),
                        "example:tooltip/rare_frame".to_string(),
                    ),
                ),
            ])
        );
    }

    #[test]
    fn tooltip_style_scaling_preserves_all_vanilla_gui_sprite_variants() {
        let stretch =
            SpriteImage::new("example:tooltip/stretch_background", 1, 1, vec![255; 4]).unwrap();
        assert_eq!(
            hud_gui_sprite_scaling(&stretch),
            HudGuiSpriteScaling::Stretch
        );

        let mut tile = SpriteImage::new(
            "example:tooltip/tile_background",
            2,
            3,
            vec![255; 2 * 3 * 4],
        )
        .unwrap();
        tile.gui_metadata.scaling = SpriteGuiScaling::Tile {
            width: 2,
            height: 3,
        };
        assert_eq!(
            hud_gui_sprite_scaling(&tile),
            HudGuiSpriteScaling::Tile {
                width: 2,
                height: 3,
            }
        );

        let mut nine_slice =
            SpriteImage::new("example:tooltip/panel_frame", 7, 9, vec![255; 7 * 9 * 4]).unwrap();
        nine_slice.gui_metadata.scaling = SpriteGuiScaling::NineSlice {
            width: 7,
            height: 9,
            border: bbb_pack::SpriteNineSliceBorder {
                left: 1,
                top: 2,
                right: 3,
                bottom: 4,
            },
            stretch_inner: true,
        };
        assert_eq!(
            hud_gui_sprite_scaling(&nine_slice),
            HudGuiSpriteScaling::NineSlice(HudNineSliceScaling {
                sprite_width: 7,
                sprite_height: 9,
                border_left: 1,
                border_top: 2,
                border_right: 3,
                border_bottom: 4,
                stretch_inner: true,
            })
        );
    }

    #[test]
    fn spectator_sprite_catalog_resolves_all_five_vanilla_icons() {
        let mut sprites = HashMap::new();
        for id in [
            "spectator/close",
            "spectator/scroll_left",
            "spectator/scroll_right",
            "spectator/teleport_to_player",
            "spectator/teleport_to_team",
        ] {
            let image = SpriteImage::new(format!("minecraft:{id}"), 16, 16, vec![255; 16 * 16 * 4])
                .unwrap();
            sprites.insert(image.id.clone(), image);
        }

        for id in [
            "spectator/close",
            "spectator/scroll_left",
            "spectator/scroll_right",
            "spectator/teleport_to_player",
            "spectator/teleport_to_team",
        ] {
            let sprite = hud_sprite(&sprites, id).unwrap();
            assert_eq!(sprite.id, format!("minecraft:{id}"));
            assert_eq!((sprite.width, sprite.height), (16, 16));
        }
    }

    #[test]
    fn hud_mob_effect_sprite_catalog_resolves_every_generated_effect_id() {
        let mut sprites = HashMap::new();
        for effect in VANILLA_MOB_EFFECT_TYPES {
            let path = effect
                .resource_id
                .strip_prefix("minecraft:")
                .unwrap_or(effect.resource_id);
            let id = format!("minecraft:mob_effect/{path}");
            let image = SpriteImage::new(id.clone(), 18, 18, vec![255; 18 * 18 * 4]).unwrap();
            assert!(sprites.insert(id, image).is_none());
        }

        let resolved = VANILLA_MOB_EFFECT_TYPES
            .iter()
            .map(|effect| {
                let path = effect
                    .resource_id
                    .strip_prefix("minecraft:")
                    .unwrap_or(effect.resource_id);
                (
                    effect.id,
                    hud_sprite(&sprites, &format!("mob_effect/{path}"))
                        .expect("generated mob-effect icon"),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(resolved.len(), VANILLA_MOB_EFFECT_TYPES.len());
        assert_eq!(resolved.len(), 40);
        assert!(resolved
            .iter()
            .all(|(_, sprite)| (sprite.width, sprite.height) == (18, 18)));
    }

    #[test]
    fn hud_heart_sprite_catalog_resolves_blinking_variants() {
        let mut sprites = HashMap::new();
        for kind in HudHeartKind::ALL {
            for hardcore in [false, true] {
                for half in [false, true] {
                    if matches!(kind, HudHeartKind::Container) && half {
                        continue;
                    }
                    for blinking in [false, true] {
                        let name = kind.sprite_name(hardcore, half, blinking);
                        let id = format!("minecraft:hud/heart/{name}");
                        let image =
                            SpriteImage::new(id.clone(), 9, 9, vec![255; 9 * 9 * 4]).unwrap();
                        assert!(sprites.insert(id, image).is_none());
                    }
                }
            }
        }

        let mut resolved = 0;
        for kind in HudHeartKind::ALL {
            for hardcore in [false, true] {
                for half in [false, true] {
                    if matches!(kind, HudHeartKind::Container) && half {
                        continue;
                    }
                    for blinking in [false, true] {
                        let name = kind.sprite_name(hardcore, half, blinking);
                        let sprite = hud_sprite(&sprites, &format!("hud/heart/{name}")).unwrap();
                        assert_eq!((sprite.width, sprite.height), (9, 9));
                        resolved += 1;
                    }
                }
            }
        }
        assert_eq!(resolved, 44);
    }

    #[test]
    fn vignette_texture_loads_from_the_selected_resource_stack() {
        let temp = unique_temp_dir("bbb-vignette-texture");
        let sources = temp.join("sources").join("26.1");
        let texture_path = sources
            .join("assets")
            .join("minecraft")
            .join(VIGNETTE_TEXTURE_PATH);
        std::fs::create_dir_all(texture_path.parent().unwrap()).unwrap();
        image::RgbaImage::from_pixel(8, 4, image::Rgba([7, 11, 13, 255]))
            .save(&texture_path)
            .unwrap();
        std::fs::write(
            texture_path.with_extension("png.mcmeta"),
            r#"{"texture":{"blur":true,"clamp":false}}"#,
        )
        .unwrap();
        let roots = PackRoots::from_paths(temp.clone(), sources, temp.join("unused-assets"));

        let image = load_vignette_image(&roots).unwrap();

        assert_eq!(image.id, "minecraft:textures/misc/vignette");
        assert_eq!((image.width, image.height), (8, 4));
        assert_eq!(image.rgba.len(), 8 * 4 * 4);
        assert!(image.texture_metadata.blur);
        assert!(!image.texture_metadata.clamp);
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn spyglass_scope_texture_loads_from_the_selected_resource_stack() {
        let temp = unique_temp_dir("bbb-spyglass-scope-texture");
        let sources = temp.join("sources").join("26.1");
        let texture_path = sources
            .join("assets")
            .join("minecraft")
            .join(SPYGLASS_SCOPE_TEXTURE_PATH);
        std::fs::create_dir_all(texture_path.parent().unwrap()).unwrap();
        image::RgbaImage::from_pixel(16, 8, image::Rgba([17, 19, 23, 255]))
            .save(&texture_path)
            .unwrap();
        std::fs::write(
            texture_path.with_extension("png.mcmeta"),
            r#"{"texture":{"blur":true,"clamp":true}}"#,
        )
        .unwrap();
        let roots = PackRoots::from_paths(temp.clone(), sources, temp.join("unused-assets"));

        let image = load_spyglass_scope_image(&roots).unwrap();

        assert_eq!(image.id, "minecraft:textures/misc/spyglass_scope");
        assert_eq!((image.width, image.height), (16, 8));
        assert_eq!(
            standalone_sampling(&image),
            StandaloneTextureSampling::new(true, true)
        );
        assert_eq!(&image.rgba[..4], &[17, 19, 23, 255]);
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn stats_sprite_catalog_resolves_through_gui_atlas_ids() {
        let mut sprites = HashMap::new();
        for sprite in HudStatsSprite::ALL {
            let image = SpriteImage::new(
                format!("minecraft:{}", sprite.sprite_path()),
                18,
                18,
                vec![255; 18 * 18 * 4],
            )
            .unwrap();
            sprites.insert(image.id.clone(), image);
        }

        for sprite in HudStatsSprite::ALL {
            let image = hud_sprite(&sprites, sprite.sprite_path()).unwrap();
            assert_eq!(image.id, format!("minecraft:{}", sprite.sprite_path()));
            assert_eq!((image.width, image.height), (18, 18));
        }
        for sprite in HudStatsScrollbarSprite::ALL {
            let image = SpriteImage::new(
                format!("minecraft:{}", sprite.sprite_path()),
                6,
                32,
                vec![255; 6 * 32 * 4],
            )
            .unwrap();
            sprites.insert(image.id.clone(), image);
        }
        for sprite in HudStatsScrollbarSprite::ALL {
            let image = hud_sprite(&sprites, sprite.sprite_path()).unwrap();
            assert_eq!(image.id, format!("minecraft:{}", sprite.sprite_path()));
            assert_eq!((image.width, image.height), (6, 32));
        }
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

    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
    }
}
