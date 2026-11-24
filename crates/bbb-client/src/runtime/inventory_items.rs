use super::*;

pub(super) struct HudItemTooltipContent {
    pub(super) style: Option<String>,
    pub(super) lines: Vec<HudInventoryTooltipLine>,
}

pub(super) fn hud_item_tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    item: &ItemStackSummary,
    advanced: bool,
    creative: bool,
) -> Option<HudItemTooltipContent> {
    hud_item_tooltip_with_inspector(world, item_runtime, item, advanced, creative, false)
}

pub(super) fn hud_item_tooltip_with_inspector(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    item: &ItemStackSummary,
    advanced: bool,
    creative: bool,
    item_inspector: bool,
) -> Option<HudItemTooltipContent> {
    let style = world.item_stack_tooltip_style(item).map(str::to_string);
    let effective_item = world.effective_item_stack(item);
    let item = effective_item.as_ref();
    let furnace_burn_ticks = item_inspector
        .then(|| item.item_id.and_then(|id| world.furnace_burn_ticks(id)))
        .flatten();
    let map_data = item
        .component_patch
        .map_id
        .and_then(|map_id| world.map_item(map_id))
        .map(|map| NativeItemMapTooltipData {
            scale: map.scale,
            locked: map.locked,
        });
    let enchantment_keys = world_enchantment_keys(world);
    let trim_material_context = world_trim_material_tooltip_context(world);
    let trim_pattern_context = world_trim_pattern_tooltip_context(world);
    let instrument_context = world_instrument_tooltip_context(world);
    let jukebox_song_context = world_jukebox_song_tooltip_context(world);
    let banner_pattern_context = world_banner_pattern_tooltip_context(world);
    let painting_variant_presentations = world_painting_variant_tooltip_context(world);
    let enchantment_definitions = crate::item_models::world_enchantment_tooltip_definitions(world);
    let enchantment_attribute_modifiers =
        world.enchantment_attribute_modifiers(&item.component_patch.enchantments);
    let attribute_keys = world_attribute_keys(world);
    let attack_damage_id = registry_key_id(
        attribute_keys.as_deref(),
        "minecraft:attack_damage",
        VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
    );
    let attack_speed_id = registry_key_id(
        attribute_keys.as_deref(),
        "minecraft:attack_speed",
        VANILLA_ATTRIBUTE_ATTACK_SPEED_ID,
    );
    let peaceful = world.difficulty().difficulty == "peaceful";
    tooltip_lines_for_stack_with_context(
        item_runtime?,
        item,
        NativeItemTooltipOptions {
            item_profiles: Some(world.item_profiles()),
            advanced,
            creative,
            peaceful,
            map_data,
            trim_material_keys: trim_material_context
                .as_ref()
                .map(|(keys, _)| keys.as_ref()),
            trim_material_presentations: trim_material_context
                .as_ref()
                .map(|(_, presentations)| presentations.as_ref()),
            trim_pattern_keys: trim_pattern_context.as_ref().map(|(keys, _)| keys.as_ref()),
            trim_pattern_presentations: trim_pattern_context
                .as_ref()
                .map(|(_, presentations)| presentations.as_ref()),
            instrument_keys: instrument_context.as_ref().map(|(keys, _)| keys.as_ref()),
            instrument_presentations: instrument_context
                .as_ref()
                .map(|(_, presentations)| presentations.as_ref()),
            jukebox_song_keys: jukebox_song_context.as_ref().map(|(keys, _)| keys.as_ref()),
            jukebox_song_presentations: jukebox_song_context
                .as_ref()
                .map(|(_, presentations)| presentations.as_ref()),
            banner_pattern_keys: banner_pattern_context
                .as_ref()
                .map(|(keys, _)| keys.as_ref()),
            banner_pattern_presentations: banner_pattern_context
                .as_ref()
                .map(|(_, presentations)| presentations.as_ref()),
            painting_variant_presentations: painting_variant_presentations.as_deref(),
            enchantment_keys: enchantment_keys.as_deref(),
            enchantment_definitions: enchantment_definitions.as_deref(),
            enchantment_attribute_modifiers: Some(&enchantment_attribute_modifiers),
            attribute_keys: attribute_keys.as_deref(),
            player_base_attack_damage: Some(
                world
                    .local_player_attribute_base_value(attack_damage_id)
                    .unwrap_or(1.0),
            ),
            player_base_attack_speed: Some(
                world
                    .local_player_attribute_base_value(attack_speed_id)
                    .unwrap_or(4.0),
            ),
            gamemaster_permission: world.local_player_has_gamemaster_permission(),
            item_disabled: !world.is_feature_enabled("minecraft:vanilla"),
            tooltip_tick_rate: Some(world.ticking().tick_rate),
            item_inspector,
            furnace_burn_ticks,
            mod_display_name: None,
        },
    )
    .map(|lines| HudItemTooltipContent {
        style,
        lines: lines
            .into_iter()
            .map(|line| HudInventoryTooltipLine {
                text: line.text,
                tint: line.tint,
                runs: line.runs,
            })
            .collect(),
    })
}

pub(super) const QUICK_CRAFT_SLOT_FILL_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 128.0 / 255.0];
const QUICK_CRAFT_YELLOW_COUNT_COLOR: [f32; 4] = [1.0, 1.0, 85.0 / 255.0, 1.0];

#[allow(clippy::too_many_arguments)]
pub(super) fn hud_inventory_floating_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    background: InventoryScreenBackground,
    main_offset_x: i32,
    stonecutter_recipe_scroll_row: Option<i32>,
    cursor_position: Option<(i32, i32)>,
    quick_craft_button_num: Option<i8>,
    quick_craft_slots: &[i16],
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    hud_glyphs: &bbb_client_render::HudFontGlyphMap,
    text_labels: &mut Vec<HudInventoryTextLabel>,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    let mut items = match background {
        InventoryScreenBackground::Merchant => hud_merchant_trade_items(
            world,
            item_runtime,
            terrain_textures,
            shift_down,
            keybind_context,
            partial_tick,
        ),
        InventoryScreenBackground::Stonecutter => hud_stonecutter_recipe_items(
            world,
            item_runtime,
            terrain_textures,
            stonecutter_recipe_scroll_row.unwrap_or_default(),
            shift_down,
            keybind_context,
            partial_tick,
        ),
        InventoryScreenBackground::Beacon => hud_beacon_example_payment_items(
            world,
            item_runtime,
            terrain_textures,
            keybind_context,
            partial_tick,
        ),
        _ => Vec::new(),
    };
    if background == InventoryScreenBackground::Merchant {
        items.extend(hud_merchant_trade_decorations(
            world,
            item_runtime,
            shift_down,
            keybind_context,
            partial_tick,
        ));
    }
    if main_offset_x != 0 {
        for item in &mut items {
            item.x += main_offset_x;
        }
    }
    push_hud_inventory_cursor_item(
        world,
        item_runtime,
        terrain_textures,
        cursor_position,
        quick_craft_button_num,
        quick_craft_slots,
        shift_down,
        keybind_context,
        hud_glyphs,
        partial_tick,
        &mut items,
        text_labels,
    );
    items
}

fn hud_beacon_example_payment_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    let item_models = HudItemModelContext::new(world, item_runtime);
    let Some(item_runtime) = item_runtime else {
        return Vec::new();
    };
    let mut items = Vec::with_capacity(BEACON_EXAMPLE_PAYMENT_ITEMS.len());
    for (vanilla_item_id, x, y) in BEACON_EXAMPLE_PAYMENT_ITEMS {
        let Some(resource_id) =
            vanilla_item_type(vanilla_item_id).map(|item_type| item_type.resource_id)
        else {
            continue;
        };
        let Some(item_id) = item_runtime.item_protocol_id(resource_id) else {
            continue;
        };
        let stack = ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: Default::default(),
        };
        let projection = item_models.project(
            &stack,
            HudItemSlotState {
                keybind: keybind_context,
                ..HudItemSlotState::default()
            },
            None,
        );
        let Some(icon) = projection
            .as_ref()
            .and_then(|projection| projection.icon(partial_tick))
        else {
            continue;
        };
        items.push(HudInventoryItem {
            x,
            y,
            scale: 1.0,
            scale_y: 1.0,
            icon,
            scissor: None,
            draw_decorations: false,
            block_model: projection
                .as_ref()
                .and_then(|projection| projection.block_model(terrain_textures)),
        });
    }
    items
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_inventory_cursor_item(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    cursor_position: Option<(i32, i32)>,
    quick_craft_button_num: Option<i8>,
    quick_craft_slots: &[i16],
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    hud_glyphs: &bbb_client_render::HudFontGlyphMap,
    partial_tick: f32,
    items: &mut Vec<HudInventoryItem>,
    text_labels: &mut Vec<HudInventoryTextLabel>,
) {
    let item_models = HudItemModelContext::new(world, item_runtime);
    let Some((cursor_x, cursor_y)) = cursor_position else {
        return;
    };
    let mut item = world.inventory().cursor_item.clone();
    if item_stack_is_empty(&item) {
        return;
    }
    let mut show_yellow_zero = false;
    if let Some(count) =
        hud_inventory_quick_craft_cursor_count(world, quick_craft_button_num, quick_craft_slots)
    {
        if count == 0 {
            show_yellow_zero = true;
            item.count = 1;
        } else {
            item.count = count;
        }
    }
    let projection = item_models.project(
        &item,
        HudItemSlotState {
            carried_item: true,
            shift_down,
            keybind: keybind_context,
            ..HudItemSlotState::default()
        },
        world.local_player_team_color_rgb(),
    );
    let item_x = cursor_x - 8;
    let item_y = cursor_y - 8;
    if show_yellow_zero {
        push_yellow_item_count_label(text_labels, item_x, item_y, 0, hud_glyphs);
    }
    let Some(mut icon) = projection
        .as_ref()
        .and_then(|projection| projection.icon(partial_tick))
    else {
        return;
    };
    if show_yellow_zero {
        icon.count_label = None;
    }
    let block_model = projection
        .as_ref()
        .and_then(|projection| projection.block_model(terrain_textures));
    items.push(HudInventoryItem {
        x: item_x,
        y: item_y,
        scale: 1.0,
        scale_y: 1.0,
        icon,
        scissor: None,
        draw_decorations: true,
        block_model,
    });
}

pub(super) fn push_yellow_item_count_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    item_x: i32,
    item_y: i32,
    count: i32,
    hud_glyphs: &bbb_client_render::HudFontGlyphMap,
) {
    let text = count.to_string();
    let width = hud_glyphs.plain_text_width(&text).max(1);
    // Vanilla `GuiGraphicsExtractor.itemCount`: `(x + 19 - 2 - font.width(text), y + 6 + 3)`.
    labels.push(HudInventoryTextLabel {
        component_runs: Vec::new(),
        x: item_x + 17 - width,
        y: item_y + 9,
        width: u32::try_from(width.max(1)).unwrap_or(1),
        text,
        tint: QUICK_CRAFT_YELLOW_COUNT_COLOR,
        background: None,
        input: None,
        scrolling: None,
        shadow: true,
        font: None,
        runs: Vec::new(),
    });
}

pub(super) fn hud_inventory_quick_craft_slot_preview(
    world: &WorldStore,
    container: &bbb_client_world::ContainerState,
    slot_num: i16,
    button_num: Option<i8>,
    slots: &[i16],
) -> Option<(ItemStackSummary, bool)> {
    if slots.len() <= 1 || !slots.contains(&slot_num) {
        return None;
    }
    let button_num = button_num?;
    let instabuild = world
        .local_player()
        .abilities
        .is_some_and(|abilities| abilities.instabuild);
    let valid_button = matches!(button_num, 0 | 1) || (button_num == 2 && instabuild);
    if !valid_button {
        return None;
    }
    let source = &world.inventory().cursor_item;
    if item_stack_is_empty(source) || !world.active_container_slot_can_quick_craft(slot_num) {
        return None;
    }
    let slot_item = world
        .inventory()
        .container_slot(container, slot_num)
        .map(|slot| slot.item)?;
    let max_size = world.active_container_slot_max_stack_size(slot_num, source)?;
    if max_size <= 0
        || (!item_stack_is_empty(slot_item)
            && !item_stacks_match_by_item_and_components(slot_item, source))
    {
        return None;
    }
    let slot_count = i32::try_from(slots.len()).ok()?;
    let item_max = world.item_profiles().item_stack_max_stack_size(source);
    let place_count = match button_num {
        0 => source.count / slot_count,
        1 => 1,
        2 => item_max,
        _ => return None,
    };
    let carry = if item_stack_is_empty(slot_item) {
        0
    } else {
        slot_item.count
    };
    let uncapped = place_count + carry;
    let capped = uncapped > max_size;
    let new_count = uncapped.min(max_size);
    let mut preview = source.clone();
    preview.count = new_count;
    Some((preview, capped))
}

fn hud_inventory_quick_craft_cursor_count(
    world: &WorldStore,
    button_num: Option<i8>,
    slots: &[i16],
) -> Option<i32> {
    if slots.len() <= 1 {
        return None;
    }
    let button_num = button_num?;
    let instabuild = world
        .local_player()
        .abilities
        .is_some_and(|abilities| abilities.instabuild);
    let valid_button = matches!(button_num, 0 | 1) || (button_num == 2 && instabuild);
    if !valid_button {
        return None;
    }
    let source = &world.inventory().cursor_item;
    if item_stack_is_empty(source) {
        return None;
    }
    if button_num == 2 {
        // Vanilla `AbstractContainerScreen.recalculateQuickCraftRemaining` (`AbstractContainerScreen.java:283-284`).
        return Some(world.item_profiles().item_stack_max_stack_size(source));
    }

    let container = if world.local_inventory_is_open() {
        &world.inventory().inventory_menu
    } else {
        world.inventory().open_container.as_ref()?
    };
    let slot_count = i32::try_from(slots.len()).ok()?;
    let place_count = match button_num {
        0 => source.count / slot_count,
        1 => 1,
        _ => return None,
    };
    let mut remaining = source.count;
    for slot_num in slots {
        let Some(slot) = world.inventory().container_slot(container, *slot_num) else {
            continue;
        };
        if !world.active_container_slot_can_quick_craft(*slot_num) {
            continue;
        }
        let Some(max_size) = world.active_container_slot_max_stack_size(*slot_num, source) else {
            continue;
        };
        if max_size <= 0
            || (!item_stack_is_empty(slot.item)
                && !item_stacks_match_by_item_and_components(slot.item, source))
        {
            continue;
        }
        let carry = if item_stack_is_empty(slot.item) {
            0
        } else {
            slot.item.count
        };
        let new_count = (place_count + carry).min(max_size);
        remaining -= new_count - carry;
    }

    Some(remaining.max(0))
}

pub(super) fn item_stacks_match_by_item_and_components(
    left: &ItemStackSummary,
    right: &ItemStackSummary,
) -> bool {
    left.item_id == right.item_id && left.component_patch == right.component_patch
}

fn hud_merchant_trade_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    let Some(offers) = merchant_offers_state(world) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    let scroll_offset = merchant_scroll_offset(offers);
    for (row, offer) in offers
        .offers
        .iter()
        .skip(scroll_offset)
        .take(MERCHANT_VISIBLE_OFFER_COUNT)
        .enumerate()
    {
        let row_y = MERCHANT_TRADE_ITEM_Y + row as i32 * MERCHANT_TRADE_ROW_HEIGHT;
        let cost_a = merchant_offer_cost_a_stack(world, offer);
        let base_cost_a = merchant_offer_base_cost_a_stack(offer);
        let cost_a_is_modified = cost_a.count != base_cost_a.count;
        push_merchant_trade_item(
            world,
            item_runtime,
            terrain_textures,
            shift_down,
            keybind_context,
            partial_tick,
            &mut items,
            MERCHANT_TRADE_COST_A_X,
            row_y,
            cost_a,
            !cost_a_is_modified,
        );
        if let Some(cost_b) = offer.buy_b.as_ref() {
            push_merchant_trade_item(
                world,
                item_runtime,
                terrain_textures,
                shift_down,
                keybind_context,
                partial_tick,
                &mut items,
                MERCHANT_TRADE_COST_B_X,
                row_y,
                item_cost_stack(cost_b, cost_b.count),
                true,
            );
        }
        push_merchant_trade_item(
            world,
            item_runtime,
            terrain_textures,
            shift_down,
            keybind_context,
            partial_tick,
            &mut items,
            MERCHANT_TRADE_RESULT_X,
            row_y,
            offer.sell.clone(),
            true,
        );
    }
    items
}

#[allow(clippy::too_many_arguments)]
fn push_merchant_trade_item(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
    items: &mut Vec<HudInventoryItem>,
    x: i32,
    y: i32,
    item: ItemStackSummary,
    draw_decorations: bool,
) {
    let item_models = HudItemModelContext::new(world, item_runtime);
    if let Some((projection, icon)) = item_models
        .project(
            &item,
            HudItemSlotState {
                shift_down,
                keybind: keybind_context,
                ..HudItemSlotState::default()
            },
            None,
        )
        .and_then(|projection| {
            let icon = projection.icon(partial_tick)?;
            Some((projection, icon))
        })
    {
        let block_model = projection.block_model(terrain_textures);
        items.push(HudInventoryItem {
            x,
            y,
            scale: 1.0,
            scale_y: 1.0,
            icon,
            scissor: None,
            draw_decorations,
            block_model,
        });
    }
}

fn hud_merchant_trade_decorations(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    let Some(offers) = merchant_offers_state(world) else {
        return Vec::new();
    };
    let mut decorations = Vec::new();
    for (row, offer) in offers
        .offers
        .iter()
        .skip(merchant_scroll_offset(offers))
        .take(MERCHANT_VISIBLE_OFFER_COUNT)
        .enumerate()
    {
        let base_cost = merchant_offer_base_cost_a_stack(offer);
        let modified_cost = merchant_offer_cost_a_stack(world, offer);
        if base_cost.count == modified_cost.count {
            continue;
        }
        let y = MERCHANT_TRADE_ITEM_Y + row as i32 * MERCHANT_TRADE_ROW_HEIGHT;
        push_merchant_trade_decoration(
            world,
            item_runtime,
            shift_down,
            keybind_context,
            partial_tick,
            &mut decorations,
            MERCHANT_TRADE_COST_A_X,
            y,
            &base_cost,
        );
        push_merchant_trade_decoration(
            world,
            item_runtime,
            shift_down,
            keybind_context,
            partial_tick,
            &mut decorations,
            MERCHANT_TRADE_COST_A_X + 14,
            y,
            &modified_cost,
        );
    }
    decorations
}

#[allow(clippy::too_many_arguments)]
fn push_merchant_trade_decoration(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
    decorations: &mut Vec<HudInventoryItem>,
    x: i32,
    y: i32,
    item: &ItemStackSummary,
) {
    let item_models = HudItemModelContext::new(world, item_runtime);
    let Some(mut icon) = hud_item_icon(
        &item_models,
        item,
        HudItemSlotState {
            shift_down,
            keybind: keybind_context,
            ..HudItemSlotState::default()
        },
        None,
        partial_tick,
    ) else {
        return;
    };
    icon.layers.clear();
    icon.foil = HudItemFoil::None;
    if item.count == 1 {
        icon.count_label = Some(HudItemCountLabel::new("1"));
    }
    decorations.push(HudInventoryItem {
        x,
        y,
        scale: 1.0,
        scale_y: 1.0,
        icon,
        scissor: None,
        draw_decorations: true,
        block_model: None,
    });
}

fn hud_stonecutter_recipe_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    scroll_row: i32,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    let item_models = HudItemModelContext::new(world, item_runtime);
    let mut items = Vec::new();
    for option in stonecutter_visible_recipe_option_stacks(world, scroll_row) {
        if let Some((projection, icon)) = item_models
            .project(
                &option.stack,
                HudItemSlotState {
                    shift_down,
                    keybind: keybind_context,
                    ..HudItemSlotState::default()
                },
                world.local_player_team_color_rgb(),
            )
            .and_then(|projection| {
                let icon = projection.icon(partial_tick)?;
                Some((projection, icon))
            })
        {
            let block_model = projection.block_model(terrain_textures);
            items.push(HudInventoryItem {
                x: option.x,
                y: option.y,
                scale: 1.0,
                scale_y: 1.0,
                icon,
                scissor: None,
                draw_decorations: true,
                block_model,
            });
        }
    }
    items
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StonecutterRecipeOptionStack {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) stack: ItemStackSummary,
}

pub(super) fn stonecutter_visible_recipe_option_stacks(
    world: &WorldStore,
    scroll_row: i32,
) -> Vec<StonecutterRecipeOptionStack> {
    let recipes = stonecutter_visible_recipes(world);
    let start_index = stonecutter_recipe_start_index(recipes.len(), scroll_row).unwrap_or_default();
    recipes
        .into_iter()
        .skip(start_index)
        .take(STONECUTTER_VISIBLE_RECIPE_BUTTON_COUNT)
        .enumerate()
        .filter_map(|(position, recipe)| {
            let stack = world
                .resolve_slot_display_first_stack(&recipe.option_display)
                .ok()
                .flatten()?;
            let column = position as i32 % STONECUTTER_RECIPE_BUTTON_COLUMNS;
            let row = position as i32 / STONECUTTER_RECIPE_BUTTON_COLUMNS;
            Some(StonecutterRecipeOptionStack {
                x: STONECUTTER_RECIPE_BUTTON_X + column * STONECUTTER_RECIPE_BUTTON_WIDTH,
                y: STONECUTTER_RECIPE_ITEM_Y + row * STONECUTTER_RECIPE_BUTTON_HEIGHT,
                stack,
            })
        })
        .collect()
}

pub(super) fn apply_stonecutter_recipe_hover_highlight(
    layers: &mut [HudInventoryBackgroundLayer],
    cursor_position: Option<(i32, i32)>,
) {
    let Some((cursor_x, cursor_y)) = cursor_position else {
        return;
    };
    // Vanilla `StonecutterScreen.extractButtons` (`StonecutterScreen.java:101-112`):
    // unselected recipe buttons use `recipe_highlighted` when cursor is in
    // `[posX, posX + 16) x [posY, posY + 18)`, where `textureY = posY - 1`.
    for layer in layers {
        if layer.texture != HudInventoryBackgroundTexture::Sprite("container/stonecutter/recipe") {
            continue;
        }
        let pos_y = layer.y + 1;
        if cursor_x >= layer.x
            && cursor_x < layer.x + STONECUTTER_RECIPE_BUTTON_WIDTH
            && cursor_y >= pos_y
            && cursor_y < pos_y + STONECUTTER_RECIPE_BUTTON_HEIGHT
        {
            layer.texture =
                HudInventoryBackgroundTexture::Sprite("container/stonecutter/recipe_highlighted");
        }
    }
}

pub(super) fn push_stonecutter_recipe_layers(
    world: &WorldStore,
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    scroll_row: i32,
) {
    let visible_recipe_count = stonecutter_visible_recipes(world).len();
    let can_scroll = visible_recipe_count > STONECUTTER_VISIBLE_RECIPE_BUTTON_COUNT;
    layers.push(hud_inventory_background_layer(
        if can_scroll {
            HudInventoryBackgroundTexture::Sprite("container/stonecutter/scroller")
        } else {
            HudInventoryBackgroundTexture::Sprite("container/stonecutter/scroller_disabled")
        },
        STONECUTTER_SCROLLER_X,
        STONECUTTER_SCROLLER_Y
            + if can_scroll {
                stonecutter_scroller_offset(visible_recipe_count, scroll_row)
            } else {
                0
            },
        u32::try_from(STONECUTTER_SCROLLER_WIDTH).unwrap_or_default(),
        u32::try_from(STONECUTTER_SCROLLER_HEIGHT).unwrap_or_default(),
        [0.0, 0.0],
        [1.0, 1.0],
    ));

    let Some(start_index) = stonecutter_recipe_start_index(visible_recipe_count, scroll_row) else {
        return;
    };
    let selected_recipe = world
        .open_container_data_value(STONECUTTER_SELECTED_RECIPE_DATA_ID)
        .map(i32::from);
    for position in 0..STONECUTTER_VISIBLE_RECIPE_BUTTON_COUNT {
        let recipe_index = start_index + position;
        if recipe_index >= visible_recipe_count {
            break;
        }
        let column = position as i32 % STONECUTTER_RECIPE_BUTTON_COLUMNS;
        let row = position as i32 / STONECUTTER_RECIPE_BUTTON_COLUMNS;
        layers.push(hud_inventory_background_layer(
            if selected_recipe == Some(recipe_index as i32) {
                HudInventoryBackgroundTexture::Sprite("container/stonecutter/recipe_selected")
            } else {
                HudInventoryBackgroundTexture::Sprite("container/stonecutter/recipe")
            },
            STONECUTTER_RECIPE_BUTTON_X + column * STONECUTTER_RECIPE_BUTTON_WIDTH,
            STONECUTTER_RECIPE_BUTTON_SPRITE_Y + row * STONECUTTER_RECIPE_BUTTON_HEIGHT,
            u32::try_from(STONECUTTER_RECIPE_BUTTON_WIDTH).unwrap_or_default(),
            u32::try_from(STONECUTTER_RECIPE_BUTTON_HEIGHT).unwrap_or_default(),
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }
}

fn stonecutter_visible_recipes(
    world: &WorldStore,
) -> Vec<&bbb_protocol::packets::StonecutterSelectableRecipeSummary> {
    let Some(input_item_id) = stonecutter_input_item_id(world) else {
        return Vec::new();
    };
    world
        .recipes()
        .stonecutter_recipes
        .iter()
        .filter(|recipe| recipe.input.item_ids.contains(&input_item_id))
        .collect()
}

fn stonecutter_input_item_id(world: &WorldStore) -> Option<i32> {
    let inventory = world.inventory();
    let item = inventory
        .container_slot(inventory.open_container.as_ref()?, 0)?
        .item;
    if item_stack_is_empty(item) {
        return None;
    }
    item.item_id
}

fn stonecutter_recipe_start_index(visible_recipe_count: usize, scroll_row: i32) -> Option<usize> {
    if visible_recipe_count == 0 {
        return None;
    }
    let row = stonecutter_clamped_recipe_scroll_row(visible_recipe_count, scroll_row);
    usize::try_from(row * STONECUTTER_RECIPE_BUTTON_COLUMNS).ok()
}

fn stonecutter_clamped_recipe_scroll_row(visible_recipe_count: usize, scroll_row: i32) -> i32 {
    scroll_row.clamp(0, stonecutter_recipe_max_scroll_row(visible_recipe_count))
}

fn stonecutter_recipe_max_scroll_row(visible_recipe_count: usize) -> i32 {
    (stonecutter_recipe_row_count(visible_recipe_count) - STONECUTTER_RECIPE_BUTTON_ROWS).max(0)
}

fn stonecutter_recipe_row_count(visible_recipe_count: usize) -> i32 {
    if visible_recipe_count == 0 {
        0
    } else {
        ((visible_recipe_count as i32) + STONECUTTER_RECIPE_BUTTON_COLUMNS - 1)
            / STONECUTTER_RECIPE_BUTTON_COLUMNS
    }
}

fn stonecutter_scroller_offset(visible_recipe_count: usize, scroll_row: i32) -> i32 {
    let max_scroll_row = stonecutter_recipe_max_scroll_row(visible_recipe_count);
    if max_scroll_row <= 0 {
        return 0;
    }
    let scroll_offs = stonecutter_clamped_recipe_scroll_row(visible_recipe_count, scroll_row)
        as f32
        / max_scroll_row as f32;
    (STONECUTTER_SCROLLER_MAX_OFFSET as f32 * scroll_offs) as i32
}
