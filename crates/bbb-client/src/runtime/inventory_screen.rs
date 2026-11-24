use super::inventory_items::{
    apply_stonecutter_recipe_hover_highlight, hud_inventory_quick_craft_slot_preview,
    push_yellow_item_count_label, QUICK_CRAFT_SLOT_FILL_COLOR,
};
use super::*;
use crate::input::InventorySlotLayout;

const CRAFTER_GRID_SLOT_COUNT: i16 = 9;

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct InventoryHudLocalState {
    pub(super) stonecutter_recipe_scroll_row: Option<i32>,
    pub(super) beacon_effect_selection: Option<(Option<i32>, Option<i32>)>,
    pub(super) loom_pattern_scroll_row: Option<i32>,
    pub(super) loom_selected_pattern_index: Option<i32>,
    pub(super) anvil_rename_text: Option<String>,
    pub(super) anvil_rename_field: Option<crate::input::TextField>,
    pub(super) recipe_book_search: RecipeBookSearchHudState,
    pub(super) recipe_book_tabs: RecipeBookTabSelectionHudState,
    pub(super) recipe_book_pages: RecipeBookPageHudState,
    pub(super) recipe_book_presentation: RecipeBookPresentationHudState,
    pub(super) recipe_book_overlay: Option<RecipeBookOverlayHudState>,
    pub(super) cursor_position: Option<(i32, i32)>,
    pub(super) merchant_focused_trade_row: Option<i32>,
    pub(super) quick_craft_button_num: Option<i8>,
    pub(super) quick_craft_slots: Vec<i16>,
    pub(super) shift_down: bool,
    pub(super) keybind_context: ItemModelKeybindContext,
    pub(super) advanced_item_tooltips: bool,
    pub(super) item_inspector: bool,
    pub(super) smithing: smithing_screen::SmithingScreenFrame,
}

/// Test helper: the inventory screen with default local state and no resident terrain atlas (so block
/// items resolve no 3D model — the tests assert the 2D icon / layout path).
#[cfg(test)]
pub(super) fn hud_inventory_screen(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    hovered_slot_id: Option<i16>,
    partial_tick: f32,
) -> Option<HudInventoryScreen> {
    hud_inventory_screen_with_local_state(
        world,
        item_runtime,
        &TerrainTextureState::default(),
        hovered_slot_id,
        InventoryHudLocalState::default(),
        partial_tick,
    )
}

#[cfg(test)]
pub(super) fn hud_inventory_screen_with_local_state(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    hovered_slot_id: Option<i16>,
    local_state: InventoryHudLocalState,
    partial_tick: f32,
) -> Option<HudInventoryScreen> {
    hud_inventory_screen_with_local_state_for_surface(
        world,
        item_runtime,
        terrain_textures,
        hovered_slot_id,
        local_state,
        winit::dpi::PhysicalSize::new(1280, 720),
        partial_tick,
    )
}

pub(super) fn update_advancement_hover_fade(
    screen: &mut AdvancementsScreen,
    world: &WorldStore,
    cursor_position: Option<(i32, i32)>,
    surface_size: winit::dpi::PhysicalSize<u32>,
) {
    let widgets = world.selected_advancement_widgets();
    let scroll_delta = screen.scroll_delta(world.selected_advancements_tab());
    let Some((scroll_x, scroll_y)) = advancements_widget_scroll(&widgets, scroll_delta) else {
        screen.update_hover_fade(false);
        return;
    };
    let (window_x, window_y) = advancements_window_origin_for_surface(surface_size);
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;
    let hovering = advancement_hovered_widget(
        &widgets,
        scroll_x,
        scroll_y,
        inside_x,
        inside_y,
        cursor_position,
    )
    .is_some();
    screen.update_hover_fade(hovering);
}

#[cfg(test)]
pub(super) fn hud_inventory_screen_with_local_state_for_surface(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    hovered_slot_id: Option<i16>,
    local_state: InventoryHudLocalState,
    surface_size: winit::dpi::PhysicalSize<u32>,
    partial_tick: f32,
) -> Option<HudInventoryScreen> {
    let hud_fonts = approximate_inventory_hud_fonts();
    hud_inventory_screen_for_surface(
        world,
        item_runtime,
        terrain_textures,
        hovered_slot_id,
        local_state,
        surface_size,
        &hud_fonts,
        None,
        None,
        partial_tick,
    )
}

#[cfg(test)]
pub(super) fn approximate_inventory_hud_fonts() -> HudFontGlyphBook {
    let mut glyphs = bbb_client_render::HudFontGlyphMap::new();
    for ch in ' '..='~' {
        let advance = hud_ascii_approx_char_width(ch);
        glyphs.insert_first_wins(
            ch,
            bbb_client_render::HudAsciiGlyph {
                width: advance.saturating_sub(1),
                height: 8,
                advance: advance as f32,
                ..bbb_client_render::HudAsciiGlyph::default()
            },
        );
    }
    HudFontGlyphBook::from_fonts(
        "minecraft:default",
        BTreeMap::from([
            ("minecraft:default".to_string(), glyphs.clone()),
            (ENCHANTMENT_ALT_FONT_ID.to_string(), glyphs),
        ]),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn hud_inventory_screen_for_surface(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    hovered_slot_id: Option<i16>,
    local_state: InventoryHudLocalState,
    surface_size: winit::dpi::PhysicalSize<u32>,
    hud_fonts: &HudFontGlyphBook,
    entity_variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    enchantment_book: Option<EnchantmentBookHudState>,
    partial_tick: f32,
) -> Option<HudInventoryScreen> {
    let item_models = HudItemModelContext::new(world, item_runtime);
    let layout = inventory_screen_layout_for_surface(world, surface_size)?;
    if layout.background == InventoryScreenBackground::Lectern {
        return None;
    }
    let container = if world.local_inventory_is_open() {
        &world.inventory().inventory_menu
    } else {
        world.inventory().open_container.as_ref()?
    };
    let enchantment_options =
        enchantment_option_hud_states(world, local_state.cursor_position, hud_fonts);
    let main_offset_x = layout
        .recipe_book
        .map(|recipe_book| recipe_book.main_gui_x_offset)
        .unwrap_or_default();
    let recipe_book_x = layout
        .recipe_book
        .map(|recipe_book| recipe_book.x)
        .unwrap_or_default();
    let recipe_book_cursor_position = local_state
        .cursor_position
        .map(|(x, y)| (x - recipe_book_x, y));

    let selected_hotbar_slot_id = inventory_screen_selected_hotbar_slot_id(world);
    let background = layout.background;
    let loom_presentation = (background == InventoryScreenBackground::Loom).then(|| {
        loom_screen::loom_table_presentation(
            world,
            item_runtime,
            entity_variant_textures,
            local_state.loom_pattern_scroll_row.unwrap_or_default(),
            local_state.loom_selected_pattern_index,
            local_state.cursor_position,
        )
    });
    let hud_glyphs = hud_fonts.glyphs(None);
    let mut quick_craft_fill_layers = Vec::new();
    let mut quick_craft_count_labels = Vec::new();
    let slots = layout
        .slots
        .iter()
        .copied()
        .map(|layout| {
            let inventory_slot = world.inventory().container_slot(container, layout.slot_id);
            let owner_team_color_rgb =
                if inventory_screen_slot_uses_fake_item(background, layout.slot_id) {
                    None
                } else {
                    world.local_player_team_color_rgb()
                };
            let quick_craft_preview = hud_inventory_quick_craft_slot_preview(
                world,
                container,
                layout.slot_id,
                local_state.quick_craft_button_num,
                &local_state.quick_craft_slots,
            );
            let (projection, capped_count) =
                if let Some((preview_item, capped)) = &quick_craft_preview {
                    quick_craft_fill_layers.push(HudInventoryFillLayer {
                        x: layout.x,
                        y: layout.y,
                        width: 16,
                        height: 16,
                        tint: QUICK_CRAFT_SLOT_FILL_COLOR,
                        stage: HudInventoryFillStage::BeforeGhostItem,
                    });
                    let projection = item_models.project(
                        preview_item,
                        HudItemSlotState {
                            selected_item: selected_hotbar_slot_id == Some(layout.slot_id),
                            shift_down: local_state.shift_down,
                            keybind: local_state.keybind_context,
                            ..HudItemSlotState::default()
                        },
                        owner_team_color_rgb,
                    );
                    (projection, (*capped).then_some(preview_item.count))
                } else {
                    let projection = inventory_slot.and_then(|slot| {
                        item_models.project(
                            slot.item,
                            HudItemSlotState {
                                bundle_selected_item_index: (slot.local_selected_bundle_item_index
                                    >= 0)
                                    .then_some(slot.local_selected_bundle_item_index),
                                selected_item: selected_hotbar_slot_id == Some(layout.slot_id),
                                shift_down: local_state.shift_down,
                                keybind: local_state.keybind_context,
                                ..HudItemSlotState::default()
                            },
                            owner_team_color_rgb,
                        )
                    });
                    (projection, None)
                };
            let mut icon = projection
                .as_ref()
                .and_then(|projection| projection.icon(partial_tick));
            if let Some(capped_count) = capped_count {
                if let Some(icon) = icon.as_mut() {
                    icon.count_label = None;
                }
                push_yellow_item_count_label(
                    &mut quick_craft_count_labels,
                    layout.x,
                    layout.y,
                    capped_count,
                    hud_glyphs,
                );
            }
            HudInventorySlot {
                slot_id: u16::try_from(layout.slot_id).unwrap_or_default(),
                x: layout.x,
                y: layout.y,
                icon,
                block_model: projection
                    .as_ref()
                    .and_then(|projection| projection.block_model(terrain_textures)),
            }
        })
        .collect();

    let mut background_layers = hud_inventory_background_layers(
        world,
        item_runtime,
        layout.background,
        &local_state,
        enchantment_options.as_ref(),
    );
    if background == InventoryScreenBackground::Stonecutter {
        apply_stonecutter_recipe_hover_highlight(
            &mut background_layers,
            local_state.cursor_position,
        );
    }
    if background == InventoryScreenBackground::Smithing {
        background_layers.extend(local_state.smithing.background_layers.iter().copied());
    }
    if let Some(presentation) = &loom_presentation {
        background_layers.extend(presentation.background_layers.iter().copied());
    }
    if main_offset_x != 0 {
        for layer in &mut background_layers {
            layer.x += main_offset_x;
        }
    }
    if let Some(recipe_book) = layout.recipe_book {
        let mut layer = hud_recipe_book_background_layer();
        layer.x += recipe_book.x;
        if recipe_book.narrow {
            background_layers.push(layer);
        } else {
            background_layers.insert(0, layer);
        }
    }
    let mut recipe_book_tab_layers = hud_recipe_book_tab_layers(
        world,
        layout.background,
        &local_state.recipe_book_tabs,
        &local_state.recipe_book_presentation,
        partial_tick,
    );
    offset_hud_inventory_background_layers(&mut recipe_book_tab_layers, recipe_book_x);
    background_layers.extend(recipe_book_tab_layers);
    if let Some(layer) =
        hud_recipe_book_search_box_layer(world, layout.background, &local_state.recipe_book_search)
    {
        let mut layers = vec![layer];
        offset_hud_inventory_background_layers(&mut layers, recipe_book_x);
        background_layers.extend(layers);
    }
    let mut recipe_book_button_layers = hud_recipe_book_recipe_button_layers(
        world,
        item_runtime,
        layout.background,
        &local_state.recipe_book_tabs,
        &local_state.recipe_book_pages,
        &local_state.recipe_book_search,
        &local_state.recipe_book_presentation,
        partial_tick,
    );
    offset_hud_inventory_background_layers(&mut recipe_book_button_layers, recipe_book_x);
    background_layers.extend(recipe_book_button_layers);
    let mut recipe_book_page_layers = hud_recipe_book_page_button_layers(
        world,
        item_runtime,
        layout.background,
        &local_state.recipe_book_tabs,
        &local_state.recipe_book_pages,
        &local_state.recipe_book_search,
        recipe_book_cursor_position,
    );
    offset_hud_inventory_background_layers(&mut recipe_book_page_layers, recipe_book_x);
    background_layers.extend(recipe_book_page_layers);
    if let Some(layer) = hud_recipe_book_button_layer(
        layout.background,
        main_offset_x,
        local_state.cursor_position,
    ) {
        background_layers.push(layer);
    }
    if let Some(layer) =
        hud_recipe_book_filter_button_layer(world, layout.background, recipe_book_cursor_position)
    {
        let mut layers = vec![layer];
        offset_hud_inventory_background_layers(&mut layers, recipe_book_x);
        background_layers.extend(layers);
    }
    let mut recipe_book_overlay_layers = hud_recipe_book_overlay_layers(
        world,
        item_runtime,
        layout.background,
        &local_state.recipe_book_tabs,
        &local_state.recipe_book_pages,
        &local_state.recipe_book_search,
        local_state.recipe_book_overlay,
        recipe_book_cursor_position,
        partial_tick,
    );
    offset_hud_inventory_background_layers(&mut recipe_book_overlay_layers, recipe_book_x);
    background_layers.extend(recipe_book_overlay_layers);
    let mut floating_items = hud_inventory_floating_items(
        world,
        item_runtime,
        terrain_textures,
        layout.background,
        main_offset_x,
        local_state.stonecutter_recipe_scroll_row,
        local_state.cursor_position,
        local_state.quick_craft_button_num,
        &local_state.quick_craft_slots,
        local_state.shift_down,
        local_state.keybind_context,
        hud_glyphs,
        &mut quick_craft_count_labels,
        partial_tick,
    );
    let mut recipe_book_tab_icon_items = hud_recipe_book_tab_icon_items(
        world,
        item_runtime,
        terrain_textures,
        layout.background,
        &local_state.recipe_book_tabs,
        &local_state.recipe_book_presentation,
        local_state.keybind_context,
        partial_tick,
    );
    offset_hud_inventory_items(&mut recipe_book_tab_icon_items, recipe_book_x);
    floating_items.extend(recipe_book_tab_icon_items);
    let mut recipe_book_recipe_button_icon_items = hud_recipe_book_recipe_button_icon_items(
        world,
        item_runtime,
        terrain_textures,
        layout.background,
        &local_state.recipe_book_tabs,
        &local_state.recipe_book_pages,
        &local_state.recipe_book_search,
        &local_state.recipe_book_presentation,
        local_state.keybind_context,
        partial_tick,
    );
    offset_hud_inventory_items(&mut recipe_book_recipe_button_icon_items, recipe_book_x);
    floating_items.extend(recipe_book_recipe_button_icon_items);
    let mut recipe_book_overlay_icon_items = hud_recipe_book_overlay_icon_items(
        world,
        item_runtime,
        terrain_textures,
        layout.background,
        &local_state.recipe_book_tabs,
        &local_state.recipe_book_pages,
        &local_state.recipe_book_search,
        local_state.recipe_book_overlay,
        local_state.keybind_context,
        partial_tick,
    );
    offset_hud_inventory_items(&mut recipe_book_overlay_icon_items, recipe_book_x);
    floating_items.extend(recipe_book_overlay_icon_items);
    let (mut fill_layers, ghost_items) = hud_recipe_book_ghost_layers_and_items(
        world,
        item_runtime,
        layout.background,
        &layout.slots,
        local_state.keybind_context,
        partial_tick,
    );
    fill_layers.extend(quick_craft_fill_layers);
    let mut entity_previews = hud_inventory_entity_previews(
        world,
        item_runtime,
        layout.background,
        &local_state,
        enchantment_book,
        partial_tick,
    );
    if let Some(result) = loom_presentation
        .as_ref()
        .and_then(|presentation| presentation.result.as_ref())
    {
        entity_previews.push(HudEntityPreview {
            entity: EntityModelInstance::banner_flag(-1, result.base_color, result.layers)
                .with(|s| s.light_coords = ENTITY_FULL_BRIGHT_LIGHT_COORDS),
            lighting: GuiItemLightingEntry::ItemsFlat,
            rect: HudEntityPreviewRect {
                x: 141,
                y: 8,
                width: 20,
                height: 40,
            },
            scissor: None,
            translation: [0.0, 0.25, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            override_camera_rotation: None,
            scale: 16.0,
            depth_isolated: true,
            after_background_layer: Some(result.after_background_layer),
            item_layers: Vec::new(),
        });
    }
    offset_hud_entity_previews(&mut entity_previews, main_offset_x);
    let mut map_previews = if layout.background == InventoryScreenBackground::CartographyTable {
        cartography_table_presentation(world, item_runtime).map_previews
    } else {
        Vec::new()
    };
    offset_hud_inventory_map_previews(&mut map_previews, main_offset_x);
    let mut banner_pattern_previews = loom_presentation
        .as_ref()
        .map(|presentation| presentation.pattern_previews.clone())
        .unwrap_or_default();
    for preview in &mut banner_pattern_previews {
        preview.x += main_offset_x;
    }
    let mut text_labels = hud_inventory_text_labels(
        world,
        item_runtime,
        layout.background,
        layout.height,
        &local_state,
        enchantment_options.as_ref(),
    );
    offset_hud_inventory_text_labels(&mut text_labels, main_offset_x);
    let mut recipe_book_text_labels = Vec::new();
    if let Some(label) = hud_recipe_book_search_text_label(
        world,
        item_runtime,
        layout.background,
        &local_state.recipe_book_search,
    ) {
        recipe_book_text_labels.push(label);
    }
    if let Some(label) = hud_recipe_book_page_text_label(
        world,
        item_runtime,
        layout.background,
        &local_state.recipe_book_tabs,
        &local_state.recipe_book_pages,
        &local_state.recipe_book_search,
    ) {
        recipe_book_text_labels.push(label);
    }
    offset_hud_inventory_text_labels(&mut recipe_book_text_labels, recipe_book_x);
    text_labels.extend(recipe_book_text_labels);
    text_labels.extend(quick_craft_count_labels);
    let foreground_layers = if layout.background == InventoryScreenBackground::Merchant {
        hud_merchant_discount_layers(world)
    } else {
        Vec::new()
    };
    let creative = world.gameplay().game_type == GameType::Creative.id();
    let inventory_tooltip = hud_recipe_book_tooltip(
        world,
        item_runtime,
        layout.background,
        &local_state.recipe_book_tabs,
        &local_state.recipe_book_pages,
        &local_state.recipe_book_search,
        local_state.recipe_book_overlay,
        recipe_book_cursor_position,
        recipe_book_x,
        local_state.advanced_item_tooltips,
        creative,
        partial_tick,
    )
    .or_else(|| {
        hud_recipe_book_ghost_tooltip(
            world,
            item_runtime,
            layout.background,
            hovered_slot_id,
            &layout.slots,
            local_state.advanced_item_tooltips,
            creative,
            partial_tick,
        )
    })
    .or_else(|| {
        hud_inventory_tooltip_with_inspector(
            world,
            item_runtime,
            hovered_slot_id,
            &layout.slots,
            container,
            local_state.advanced_item_tooltips,
            creative,
            local_state.item_inspector,
        )
    });
    let tooltip = match layout.background {
        InventoryScreenBackground::EnchantmentTable => hud_enchantment_table_tooltip(
            world,
            item_runtime,
            local_state.cursor_position,
            enchantment_options.as_ref(),
        )
        .or(inventory_tooltip),
        InventoryScreenBackground::Merchant => hud_merchant_trade_tooltip_with_inspector(
            world,
            item_runtime,
            local_state.cursor_position,
            local_state.advanced_item_tooltips,
            creative,
            local_state.item_inspector,
        )
        .or(inventory_tooltip),
        InventoryScreenBackground::Stonecutter => hud_stonecutter_recipe_tooltip_with_inspector(
            world,
            item_runtime,
            local_state.cursor_position,
            local_state
                .stonecutter_recipe_scroll_row
                .unwrap_or_default(),
            local_state.advanced_item_tooltips,
            creative,
            local_state.item_inspector,
        )
        .or(inventory_tooltip),
        InventoryScreenBackground::Loom => loom_presentation
            .as_ref()
            .and_then(|presentation| presentation.tooltip.clone())
            .or(inventory_tooltip),
        InventoryScreenBackground::Smithing => smithing_screen::tooltip(
            world,
            item_runtime,
            hovered_slot_id,
            local_state.cursor_position,
            hud_fonts,
        )
        .or(inventory_tooltip),
        InventoryScreenBackground::Crafter => hud_crafter_togglable_slot_tooltip(
            world,
            item_runtime,
            hovered_slot_id,
            local_state.cursor_position,
            &layout.slots,
        )
        .or(inventory_tooltip),
        InventoryScreenBackground::Beacon => hud_beacon_tooltip(
            item_runtime,
            local_state.cursor_position,
            local_state
                .beacon_effect_selection
                .unwrap_or_else(|| beacon_effect_selection_from_world(world)),
        )
        .or(inventory_tooltip),
        _ => inventory_tooltip,
    };

    let screen_width = u32::try_from(layout.width).unwrap_or_default();
    let screen_height = u32::try_from(layout.height).unwrap_or_default();
    let preedit_overlay = if layout.background == InventoryScreenBackground::Anvil {
        local_state.anvil_rename_field.as_ref().and_then(|field| {
            bbb_client_render::hud_text_field_preedit_overlay(
                &field.hud(true, false),
                screen_width,
                screen_height,
                main_offset_x + 62,
                24,
                103,
                10,
                surface_size,
                hud_fonts.glyphs(None),
            )
        })
    } else if local_state.recipe_book_search.focused
        && recipe_book_type_for_background(layout.background).is_some()
        && recipe_book_main_gui_offset(world, layout.background) != 0
    {
        bbb_client_render::hud_text_field_preedit_overlay(
            &local_state.recipe_book_search.edit.hud(true, false),
            screen_width,
            screen_height,
            recipe_book_x + RECIPE_BOOK_SEARCH_BOX_X + RECIPE_BOOK_SEARCH_TEXT_X_OFFSET,
            RECIPE_BOOK_SEARCH_BOX_Y + RECIPE_BOOK_SEARCH_TEXT_Y_OFFSET,
            u32::try_from(RECIPE_BOOK_SEARCH_BOX_WIDTH - 8).unwrap_or_default(),
            10,
            surface_size,
            hud_fonts.glyphs(None),
        )
    } else {
        None
    };

    Some(HudInventoryScreen {
        width: screen_width,
        height: screen_height,
        background_layers,
        foreground_layers,
        fill_layers,
        slots,
        floating_items,
        foreground_items: Vec::new(),
        ghost_items,
        entity_previews,
        map_previews,
        banner_pattern_previews,
        text_labels,
        scroll_area: None,
        preedit_overlay,
        hovered_slot_id: hovered_slot_id.and_then(|slot| u16::try_from(slot).ok()),
        tooltip,
    })
}

/// Vanilla `CrafterScreen.extractRenderState` (`CrafterScreen.java:100-106`):
/// empty enabled Crafter grid slots show `gui.togglable_slot` when cursor is empty
/// and the player is not in Spectator mode.
fn hud_crafter_togglable_slot_tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    hovered_slot_id: Option<i16>,
    cursor_position: Option<(i32, i32)>,
    slots: &[InventorySlotLayout],
) -> Option<HudInventoryTooltip> {
    let slot_id = hovered_slot_id?;
    if !(0..CRAFTER_GRID_SLOT_COUNT).contains(&slot_id)
        || world.gameplay().game_type == GameType::Spectator.id()
        || !item_stack_is_empty(&world.inventory().cursor_item)
        || world.open_container_data_value(slot_id).unwrap_or_default() == 1
        || open_container_slot_has_item(world, slot_id)
    {
        return None;
    }
    let (x, y) = cursor_position.or_else(|| {
        slots
            .iter()
            .find(|slot| slot.slot_id == slot_id)
            .map(|slot| (slot.x, slot.y))
    })?;
    Some(HudInventoryTooltip {
        slot_id: u16::try_from(slot_id).ok()?,
        x,
        y,
        style: None,
        lines: vec![HudInventoryTooltipLine {
            text: localized_gui_text(item_runtime, "gui.togglable_slot", "Click to toggle slot"),
            tint: [1.0, 1.0, 1.0, 1.0],
            runs: Vec::new(),
        }],
    })
}

pub(super) fn inventory_screen_slot_uses_fake_item(
    background: InventoryScreenBackground,
    slot_id: i16,
) -> bool {
    matches!(
        (background, slot_id),
        (
            InventoryScreenBackground::LocalInventory | InventoryScreenBackground::CraftingTable,
            0
        ) | (InventoryScreenBackground::Crafter, 45)
    )
}

fn offset_hud_entity_previews(previews: &mut [HudEntityPreview], x_offset: i32) {
    if x_offset == 0 {
        return;
    }
    for preview in previews {
        preview.rect.x += x_offset;
        if let Some(scissor) = &mut preview.scissor {
            scissor.x += x_offset;
        }
    }
}

fn offset_hud_inventory_map_previews(previews: &mut [HudInventoryMapPreview], x_offset: i32) {
    if x_offset == 0 {
        return;
    }
    for preview in previews {
        preview.x += x_offset;
    }
}

fn offset_hud_inventory_background_layers(
    layers: &mut [HudInventoryBackgroundLayer],
    x_offset: i32,
) {
    if x_offset == 0 {
        return;
    }
    for layer in layers {
        layer.x += x_offset;
    }
}

fn offset_hud_inventory_items(items: &mut [HudInventoryItem], x_offset: i32) {
    if x_offset == 0 {
        return;
    }
    for item in items {
        item.x += x_offset;
    }
}

fn offset_hud_inventory_text_labels(labels: &mut [HudInventoryTextLabel], x_offset: i32) {
    if x_offset == 0 {
        return;
    }
    for label in labels {
        label.x += x_offset;
        if let Some(background) = &mut label.background {
            background.x += x_offset;
        }
        if let Some(scrolling) = &mut label.scrolling {
            scrolling.x += x_offset;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::menu_types::{
        VANILLA_MENU_TYPE_CRAFTER_3X3_ID, VANILLA_MENU_TYPE_GENERIC_9X1_ID,
        VANILLA_MENU_TYPE_STONECUTTER_ID,
    };
    use bbb_protocol::packets::{
        ContainerSetContent, ContainerSetData, OpenScreen, RecipePropertySetSummary,
        SlotDisplaySummary, StonecutterSelectableRecipeSummary, UpdateRecipes,
    };
    use bbb_protocol::recipe_wire_types::VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID;

    #[test]
    fn quick_craft_projects_slot_fill_and_yellow_zero_remainder_in_generic_container() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X1_ID,
            title: bbb_protocol::RichTextComponent::plain("Chest"),
        });
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 1,
            items: vec![ItemStackSummary::empty(); 45],
            carried_item: ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
                count: 2,
                component_patch: bbb_protocol::packets::DataComponentPatchSummary::default().into(),
            },
        });
        let terrain = TerrainTextureState::default();
        let screen = hud_inventory_screen_with_local_state(
            &world,
            None,
            &terrain,
            None,
            InventoryHudLocalState {
                cursor_position: Some((600, 360)),
                quick_craft_button_num: Some(0),
                quick_craft_slots: vec![0, 1],
                ..InventoryHudLocalState::default()
            },
            0.0,
        )
        .unwrap();

        assert_eq!(screen.fill_layers.len(), 2);
        assert!(screen
            .fill_layers
            .iter()
            .all(|layer| layer.stage == HudInventoryFillStage::BeforeGhostItem));
        assert!(screen
            .text_labels
            .iter()
            .any(|label| label.text == "0" && label.tint == [1.0, 1.0, 85.0 / 255.0, 1.0]));
    }

    #[test]
    fn crafter_empty_enabled_slot_shows_togglable_slot_tooltip() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 8,
            menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
            title: bbb_protocol::RichTextComponent::plain("Crafter"),
        });
        world.apply_container_set_content(ContainerSetContent {
            container_id: 8,
            state_id: 1,
            items: vec![ItemStackSummary::empty(); 46],
            carried_item: ItemStackSummary::empty(),
        });
        let layout = crate::input::inventory_screen_layout(&world).unwrap();
        let slot0 = layout.slots.iter().find(|s| s.slot_id == 0).unwrap();
        let terrain = TerrainTextureState::default();
        let screen = hud_inventory_screen_with_local_state(
            &world,
            None,
            &terrain,
            Some(0),
            InventoryHudLocalState {
                cursor_position: Some((slot0.x + 8, slot0.y + 8)),
                ..InventoryHudLocalState::default()
            },
            0.0,
        )
        .unwrap();

        assert_eq!(
            screen
                .tooltip
                .as_ref()
                .and_then(|tooltip| tooltip.lines.first())
                .map(|line| line.text.as_str()),
            Some("Click to toggle slot")
        );
    }

    #[test]
    fn stonecutter_hovered_unselected_recipe_uses_highlighted_sprite() {
        let mut world = WorldStore::new();
        let mut raw_payload = Vec::new();
        bbb_protocol::codec::write_var_i32_to(&mut raw_payload, VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID);
        bbb_protocol::codec::write_var_i32_to(
            &mut raw_payload,
            bbb_protocol::item_types::VANILLA_ITEM_STONE_ID,
        );
        world.apply_update_recipes(UpdateRecipes {
            property_sets: Vec::<RecipePropertySetSummary>::new(),
            stonecutter_recipes: vec![StonecutterSelectableRecipeSummary {
                input: bbb_protocol::packets::IngredientSummary {
                    tag: None,
                    item_ids: vec![bbb_protocol::item_types::VANILLA_ITEM_STONE_ID],
                },
                option_display: SlotDisplaySummary {
                    display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
                    item_stack: Some(ItemStackSummary {
                        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
                        count: 1,
                        component_patch: bbb_protocol::packets::DataComponentPatchSummary::default(
                        )
                        .into(),
                    }),
                    tag: None,
                    raw_payload,
                },
            }],
        });
        world.apply_open_screen(OpenScreen {
            container_id: 9,
            menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
            title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
        });
        let mut items = vec![ItemStackSummary::empty(); 38];
        items[0] = ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            count: 1,
            component_patch: bbb_protocol::packets::DataComponentPatchSummary::default().into(),
        };
        world.apply_container_set_content(ContainerSetContent {
            container_id: 9,
            state_id: 1,
            items,
            carried_item: ItemStackSummary::empty(),
        });
        world.apply_container_set_data(ContainerSetData {
            container_id: 9,
            id: 0,
            value: -1,
        });
        let terrain = TerrainTextureState::default();
        let screen = hud_inventory_screen_with_local_state(
            &world,
            None,
            &terrain,
            None,
            InventoryHudLocalState {
                cursor_position: Some((52 + 4, 14 + 4)),
                ..InventoryHudLocalState::default()
            },
            0.0,
        )
        .unwrap();

        assert!(screen.background_layers.iter().any(|layer| {
            layer.texture
                == HudInventoryBackgroundTexture::Sprite("container/stonecutter/recipe_highlighted")
        }));
    }

    #[test]
    fn bundle_capacity_bar_and_visual_tooltip_match_vanilla_rules() {
        use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID;
        use bbb_protocol::item_types::{
            VANILLA_ITEM_BUNDLE_ID, VANILLA_ITEM_DIAMOND_ID, VANILLA_ITEM_STICK_ID,
        };
        use bbb_protocol::packets::ItemStackTemplateSummary;

        let world = WorldStore::new();
        let mut bundle = ItemStackSummary {
            item_id: Some(VANILLA_ITEM_BUNDLE_ID),
            count: 1,
            component_patch: Default::default(),
        };
        assert_eq!(
            super::super::hud_item_durability_bar_for_stack(&world, &bundle),
            None,
            "empty bundle shows no capacity bar"
        );
        let empty_tooltip = super::super::hud_bundle_tooltip_for_stack(
            &world,
            None,
            &bundle,
            -1,
            false,
            ItemModelKeybindContext::default(),
            0.0,
        )
        .expect("empty bundle produces visual tooltip");
        assert!(empty_tooltip.items.is_empty());
        assert_eq!(empty_tooltip.progress_label.as_deref(), Some("Empty"));

        bundle
            .component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID);
        bundle.component_patch.bundle_contents_items = (0..13)
            .map(|idx| ItemStackTemplateSummary {
                item_id: if idx == 0 {
                    VANILLA_ITEM_DIAMOND_ID
                } else {
                    VANILLA_ITEM_STICK_ID
                },
                count: 2,
                component_patch: Default::default(),
            })
            .collect();

        let bar = super::super::hud_item_durability_bar_for_stack(&world, &bundle)
            .expect("non-empty bundle shows capacity bar");
        // 26 / 64 weight => 1 + floor(26 * 12 / 64) = 5, not full => light blue.
        assert_eq!(bar.width, 5);
        assert_eq!(bar.color, [0.44, 0.53, 1.0]);

        let tooltip = super::super::hud_bundle_tooltip_for_stack(
            &world,
            None,
            &bundle,
            0,
            false,
            ItemModelKeybindContext::default(),
            0.0,
        )
        .expect("populated bundle tooltip");
        // 13 stacks => 8 shown (11 - 3 empty slots on row), 5 hidden stacks * count 2 = 10.
        assert_eq!(tooltip.items.len(), 8);
        assert_eq!(tooltip.hidden_item_count, 10);
        assert_eq!(tooltip.selected_index, Some(0));
        assert!(tooltip.selected_item_name.is_some());
        assert_eq!(tooltip.progress_label, None);

        // Full bundle switches to red full color & "Full" label.
        bundle.component_patch.bundle_contents_items = vec![ItemStackTemplateSummary {
            item_id: VANILLA_ITEM_STICK_ID,
            count: 64,
            component_patch: Default::default(),
        }];
        let full_bar = super::super::hud_item_durability_bar_for_stack(&world, &bundle).unwrap();
        assert_eq!(full_bar.width, 13);
        assert_eq!(full_bar.color, [1.0, 0.33, 0.33]);
        let full_tooltip = super::super::hud_bundle_tooltip_for_stack(
            &world,
            None,
            &bundle,
            -1,
            false,
            ItemModelKeybindContext::default(),
            0.0,
        )
        .unwrap();
        assert_eq!(full_tooltip.progress_label.as_deref(), Some("Full"));

        // TOOLTIP_DISPLAY hiding bundle_contents suppresses the visual tooltip.
        bundle
            .component_patch
            .tooltip_hidden_component_type_ids
            .push(VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID);
        assert!(super::super::hud_bundle_tooltip_for_stack(
            &world,
            None,
            &bundle,
            -1,
            false,
            ItemModelKeybindContext::default(),
            0.0,
        )
        .is_none());
    }
}
