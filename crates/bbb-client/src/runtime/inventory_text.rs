use super::*;

pub(super) const CONTAINER_LABEL_TEXT_COLOR: [f32; 4] = rgba32(-12_566_464);
pub(super) const CONTAINER_LABEL_WIDTH: u32 = 512;
pub(super) const CONTAINER_CENTERED_LABEL_WIDTH: u32 = 512;
const CONTAINER_TEXT_LINE_HEIGHT: u32 = 9;
pub(super) fn hud_inventory_text_labels(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    screen_height: i32,
    local_state: &InventoryHudLocalState,
    enchantment_options: Option<&[EnchantmentOptionHudState; ENCHANTMENT_OPTION_COUNT]>,
) -> Vec<HudInventoryTextLabel> {
    let mut labels = if background == InventoryScreenBackground::Beacon {
        hud_beacon_text_labels(item_runtime)
    } else {
        hud_server_opened_container_labels(world, item_runtime, background, screen_height)
    };
    match background {
        InventoryScreenBackground::Anvil => {
            if let Some(text) = local_state
                .anvil_rename_text
                .as_ref()
                .filter(|text| !text.is_empty())
            {
                labels.push(HudInventoryTextLabel {
                    component_runs: Vec::new(),
                    x: 62,
                    y: 24,
                    width: 103,
                    text: text.clone(),
                    tint: ANVIL_RENAME_TEXT_COLOR,
                    background: None,
                    input: None,
                    scrolling: None,
                    shadow: false,
                    font: None,
                    runs: Vec::new(),
                });
            }
            if let Some(label) = anvil_cost_text_label(world) {
                labels.push(label);
            }
        }
        InventoryScreenBackground::EnchantmentTable => {
            labels.extend(enchanting_table_text_labels(
                enchantment_options.map_or(&[], |options| options.as_slice()),
            ));
        }
        InventoryScreenBackground::Lectern => {}
        _ => {}
    }
    labels
}

fn hud_beacon_text_labels(item_runtime: Option<&NativeItemRuntime>) -> Vec<HudInventoryTextLabel> {
    let mut labels = Vec::with_capacity(2);
    for (translation_key, fallback, center_x) in [
        (
            "block.minecraft.beacon.primary",
            "Primary Power",
            BEACON_PRIMARY_LABEL_CENTER_X,
        ),
        (
            "block.minecraft.beacon.secondary",
            "Secondary Power",
            BEACON_SECONDARY_LABEL_CENTER_X,
        ),
    ] {
        let text = localized_gui_text(item_runtime, translation_key, fallback);
        let bounds_x =
            center_x - i32::try_from(CONTAINER_CENTERED_LABEL_WIDTH).unwrap_or(i32::MAX) / 2;
        labels.push(HudInventoryTextLabel {
            component_runs: Vec::new(),
            x: bounds_x,
            y: BEACON_LABEL_Y,
            width: CONTAINER_LABEL_WIDTH,
            text: text.clone(),
            tint: BEACON_LABEL_TEXT_COLOR,
            background: None,
            input: None,
            scrolling: Some(HudInventoryScrollingText {
                x: bounds_x,
                y: BEACON_LABEL_Y - 1,
                width: CONTAINER_CENTERED_LABEL_WIDTH,
                height: CONTAINER_TEXT_LINE_HEIGHT,
            }),
            shadow: false,
            font: None,
            runs: Vec::new(),
        });
    }
    labels
}

fn hud_server_opened_container_labels(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    screen_height: i32,
) -> Vec<HudInventoryTextLabel> {
    let Some(container) = world.inventory().open_container.as_ref() else {
        return Vec::new();
    };
    let Some(title) = container.title.as_ref() else {
        return Vec::new();
    };
    if matches!(
        background,
        InventoryScreenBackground::LocalInventory
            | InventoryScreenBackground::Mount { .. }
            | InventoryScreenBackground::Lectern
    ) {
        return Vec::new();
    }

    let title_runs = localized_component_runs(title, &ComponentStyle::default(), item_runtime);
    let mut labels = Vec::with_capacity(if background == InventoryScreenBackground::Merchant {
        3
    } else {
        2
    });
    if background == InventoryScreenBackground::Merchant {
        let title_runs = hud_merchant_title_runs(world, item_runtime, title_runs);
        push_centered_container_label(&mut labels, title_runs, 187, 6);
        push_plain_container_label(
            &mut labels,
            localized_gui_text(item_runtime, "container.inventory", "Inventory"),
            107,
            screen_height - 94,
        );
        push_centered_container_label(
            &mut labels,
            vec![StyledTextRun {
                text: localized_gui_text(item_runtime, "merchant.trades", "Trades"),
                style: ComponentStyle::default(),
            }],
            53,
            6,
        );
        return labels;
    }

    match background {
        InventoryScreenBackground::Generic3x3
        | InventoryScreenBackground::Crafter
        | InventoryScreenBackground::BlastFurnace
        | InventoryScreenBackground::BrewingStand
        | InventoryScreenBackground::Furnace
        | InventoryScreenBackground::Smoker => {
            push_centered_container_label(&mut labels, title_runs, 88, 6);
        }
        InventoryScreenBackground::Anvil => {
            push_component_container_label(&mut labels, title_runs, 60, 6);
        }
        InventoryScreenBackground::CraftingTable => {
            push_component_container_label(&mut labels, title_runs, 29, 6);
        }
        InventoryScreenBackground::Loom | InventoryScreenBackground::CartographyTable => {
            push_component_container_label(&mut labels, title_runs, 8, 4);
        }
        InventoryScreenBackground::Smithing => {
            push_component_container_label(&mut labels, title_runs, 44, 15);
        }
        InventoryScreenBackground::Stonecutter => {
            push_component_container_label(&mut labels, title_runs, 8, 5);
        }
        InventoryScreenBackground::Generic9xRows { .. }
        | InventoryScreenBackground::Beacon
        | InventoryScreenBackground::EnchantmentTable
        | InventoryScreenBackground::Grindstone
        | InventoryScreenBackground::Hopper
        | InventoryScreenBackground::ShulkerBox => {
            push_component_container_label(&mut labels, title_runs, 8, 6);
        }
        InventoryScreenBackground::LocalInventory
        | InventoryScreenBackground::Mount { .. }
        | InventoryScreenBackground::Lectern
        | InventoryScreenBackground::Merchant => {
            unreachable!("non-standard container labels returned above")
        }
    }
    push_plain_container_label(
        &mut labels,
        localized_gui_text(item_runtime, "container.inventory", "Inventory"),
        8,
        screen_height - 94,
    );
    labels
}

fn hud_merchant_title_runs(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    title_runs: Vec<StyledTextRun>,
) -> Vec<StyledTextRun> {
    let Some(offers) = merchant_offers_state(world)
        .filter(|offers| offers.show_progress && (1..=5).contains(&offers.villager_level))
    else {
        return title_runs;
    };
    let level_key = format!("merchant.level.{}", offers.villager_level);
    let level_fallback = match offers.villager_level {
        1 => "Novice",
        2 => "Apprentice",
        3 => "Journeyman",
        4 => "Expert",
        5 => "Master",
        _ => unreachable!("merchant level was range checked"),
    };
    let level_runs = vec![StyledTextRun {
        text: localized_gui_text(item_runtime, &level_key, level_fallback),
        style: ComponentStyle::default(),
    }];
    let template = localized_gui_text(item_runtime, "merchant.title", "%s - %s");
    let arguments = [&title_runs, &level_runs];
    let mut runs = Vec::new();
    visit_localized_template(&template, arguments.len(), |part| match part {
        LocalizedTemplatePart::Literal(text) => {
            push_component_text_run(&mut runs, text, &ComponentStyle::default());
        }
        LocalizedTemplatePart::Argument { index, raw } => {
            if let Some(argument) = arguments.get(index) {
                for run in *argument {
                    push_component_text_run(&mut runs, &run.text, &run.style);
                }
            } else {
                push_component_text_run(&mut runs, raw, &ComponentStyle::default());
            }
        }
    });
    runs
}

fn push_plain_container_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    text: String,
    x: i32,
    y: i32,
) {
    push_component_container_label(
        labels,
        vec![StyledTextRun {
            text,
            style: ComponentStyle::default(),
        }],
        x,
        y,
    );
}

pub(super) fn push_component_container_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    runs: Vec<StyledTextRun>,
    x: i32,
    y: i32,
) {
    let text = runs.iter().map(|run| run.text.as_str()).collect::<String>();
    if text.is_empty() {
        return;
    }
    labels.push(HudInventoryTextLabel {
        component_runs: Vec::new(),
        x,
        y,
        width: CONTAINER_LABEL_WIDTH,
        text,
        tint: CONTAINER_LABEL_TEXT_COLOR,
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
        runs: hud_styled_runs_from_component_runs(&runs),
    });
}

fn push_centered_container_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    runs: Vec<StyledTextRun>,
    center_x: i32,
    y: i32,
) {
    let text = runs.iter().map(|run| run.text.as_str()).collect::<String>();
    if text.is_empty() {
        return;
    }
    let bounds_x = center_x - i32::try_from(CONTAINER_CENTERED_LABEL_WIDTH).unwrap_or(i32::MAX) / 2;
    labels.push(HudInventoryTextLabel {
        component_runs: Vec::new(),
        x: bounds_x,
        y,
        width: CONTAINER_LABEL_WIDTH,
        text,
        tint: CONTAINER_LABEL_TEXT_COLOR,
        background: None,
        input: None,
        scrolling: Some(HudInventoryScrollingText {
            x: bounds_x,
            y: y - 1,
            width: CONTAINER_CENTERED_LABEL_WIDTH,
            height: CONTAINER_TEXT_LINE_HEIGHT,
        }),
        shadow: false,
        font: None,
        runs: hud_styled_runs_from_component_runs(&runs),
    });
}

fn push_component_text_run(runs: &mut Vec<StyledTextRun>, text: &str, style: &ComponentStyle) {
    if text.is_empty() {
        return;
    }
    if let Some(previous) = runs.last_mut().filter(|previous| previous.style == *style) {
        previous.text.push_str(text);
    } else {
        runs.push(StyledTextRun {
            text: text.to_string(),
            style: style.clone(),
        });
    }
}

fn enchanting_table_text_labels(
    options: &[EnchantmentOptionHudState],
) -> Vec<HudInventoryTextLabel> {
    let mut labels = Vec::with_capacity(options.len() * 2);
    for option in options {
        if option.cost <= 0 {
            continue;
        }
        let row = i32::try_from(option.index).expect("three enchanting rows fit i32");
        labels.push(HudInventoryTextLabel {
            component_runs: Vec::new(),
            x: ENCHANTING_TABLE_OPTION_X + 20,
            y: ENCHANTING_TABLE_OPTION_Y + 2 + row * ENCHANTING_TABLE_OPTION_SPACING,
            width: option.name_max_width.max(0) as u32,
            text: option.name.clone(),
            tint: if !option.enabled {
                ENCHANTING_TABLE_NAME_TEXT_DISABLED_COLOR
            } else if option.highlighted {
                ENCHANTING_TABLE_NAME_TEXT_HIGHLIGHTED_COLOR
            } else {
                ENCHANTING_TABLE_NAME_TEXT_COLOR
            },
            background: None,
            input: None,
            scrolling: None,
            shadow: false,
            font: Some(ENCHANTMENT_ALT_FONT_ID.to_string()),
            runs: Vec::new(),
        });

        let text = option.cost.to_string();
        let text_width =
            (ENCHANTING_TABLE_OPTION_TEXT_WIDTH as i32).saturating_sub(option.name_max_width);
        labels.push(HudInventoryTextLabel {
            component_runs: Vec::new(),
            x: ENCHANTING_TABLE_COST_TEXT_X_RIGHT.saturating_sub(text_width),
            y: ENCHANTING_TABLE_COST_TEXT_Y_OFFSET + row * ENCHANTING_TABLE_OPTION_SPACING,
            width: text_width.max(0) as u32,
            text,
            tint: if option.enabled {
                ENCHANTING_TABLE_COST_TEXT_ENABLED_COLOR
            } else {
                ENCHANTING_TABLE_COST_TEXT_DISABLED_COLOR
            },
            background: None,
            input: None,
            scrolling: None,
            shadow: false,
            font: None,
            runs: Vec::new(),
        });
    }
    labels
}

fn anvil_cost_text_label(world: &WorldStore) -> Option<HudInventoryTextLabel> {
    let cost = world.open_container_data_value(ANVIL_COST_DATA_ID)?;
    if cost <= 0 {
        return None;
    }

    let creative = world.gameplay().game_type == GameType::Creative.id();
    let (text, tint) = if cost >= ANVIL_TOO_EXPENSIVE_LEVEL_COST && !creative {
        ("Too Expensive!".to_string(), ANVIL_COST_ERROR_TEXT_COLOR)
    } else {
        if !open_container_slot_has_item(world, ANVIL_RESULT_SLOT) {
            return None;
        }
        let may_pickup = creative
            || world
                .local_player()
                .experience
                .is_some_and(|experience| experience.level >= i32::from(cost));
        (
            format!("Cost: {cost}"),
            if may_pickup {
                ANVIL_COST_TEXT_COLOR
            } else {
                ANVIL_COST_ERROR_TEXT_COLOR
            },
        )
    };

    let text_width = hud_ascii_approx_text_width(&text)?;
    let x = ANVIL_COST_LABEL_RIGHT - 2 - i32::try_from(text_width).ok()?;
    Some(HudInventoryTextLabel {
        component_runs: Vec::new(),
        x,
        y: ANVIL_COST_LABEL_Y,
        width: text_width,
        text,
        tint,
        background: Some(HudInventoryTextBackground {
            x: x - 2,
            y: ANVIL_COST_BACKGROUND_Y,
            width: text_width.saturating_add(4),
            height: ANVIL_COST_BACKGROUND_HEIGHT,
            tint: ANVIL_COST_BACKGROUND_TINT,
        }),
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
        runs: Vec::new(),
    })
}

pub(super) fn hud_ascii_approx_text_width(text: &str) -> Option<u32> {
    let mut width = 0u32;
    for ch in text.chars() {
        width = width.checked_add(hud_ascii_approx_char_width(ch))?;
    }
    (width > 0).then_some(width)
}

pub(super) fn hud_ascii_approx_char_width(ch: char) -> u32 {
    if ch == ' ' {
        4
    } else {
        6
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn hud_inventory_tooltip_with_inspector(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    hovered_slot_id: Option<i16>,
    layout_slots: &[crate::input::InventorySlotLayout],
    container: &ContainerState,
    advanced: bool,
    creative: bool,
    item_inspector: bool,
) -> Option<HudInventoryTooltip> {
    let slot_id = hovered_slot_id?;
    let layout = layout_slots
        .iter()
        .find(|layout| layout.slot_id == slot_id)?;
    let slot = world.inventory().container_slot(container, slot_id)?;
    let tooltip = hud_item_tooltip_with_inspector(
        world,
        item_runtime,
        slot.item,
        advanced,
        creative,
        item_inspector,
    )?;
    Some(HudInventoryTooltip {
        slot_id: u16::try_from(slot_id).ok()?,
        x: layout.x,
        y: layout.y,
        style: tooltip.style,
        lines: tooltip.lines,
    })
}

/// Vanilla `BundleContents.getNumberOfItemsToShow` (`BundleContents.java:83-89`).
fn bundle_number_of_items_to_show(item_count: usize) -> usize {
    let available: usize = if item_count > 12 { 11 } else { 12 };
    let on_non_full_row = item_count % 4;
    let empty_space = if on_non_full_row == 0 {
        0
    } else {
        4 - on_non_full_row
    };
    item_count.min(available.saturating_sub(empty_space))
}

/// Vanilla `BundleItem.getTooltipImage` (`BundleItem.java:241-246`) &
/// `ClientBundleTooltip` (`ClientBundleTooltip.java:83-222`).
#[allow(clippy::too_many_arguments)]
pub(super) fn hud_bundle_tooltip_for_stack(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    stack: &bbb_protocol::packets::ItemStackSummary,
    selected_item_index: i32,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Option<bbb_client_render::HudBundleTooltip> {
    if !is_bundle_item_stack(stack) {
        return None;
    }
    let effective = world.effective_item_stack(stack);
    if effective.component_patch.tooltip_hide_tooltip
        || effective
            .component_patch
            .tooltip_hidden_component_type_ids
            .contains(
                &bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
            )
    {
        return None;
    }
    let contents = effective_bundle_contents(effective.as_ref())?;
    let (weight_num, weight_den) = bundle_contents_weight_fraction(world, contents)?;
    if weight_den <= 0 {
        return None;
    }
    let shown_count = bundle_number_of_items_to_show(contents.len());
    let hidden_item_count = contents.iter().skip(shown_count).fold(0u32, |acc, item| {
        acc.saturating_add(u32::try_from(item.count.max(0)).unwrap_or(u32::MAX))
    });
    let item_models = HudItemModelContext::new(world, item_runtime);
    let team_color = world.local_player_team_color_rgb();
    let items = contents
        .iter()
        .take(shown_count)
        .enumerate()
        .map(|(index, entry)| {
            let entry_stack = bbb_protocol::packets::ItemStackSummary {
                item_id: Some(entry.item_id),
                count: entry.count,
                component_patch: entry.component_patch.clone(),
            };
            hud_item_icon(
                &item_models,
                &entry_stack,
                HudItemSlotState {
                    shift_down,
                    keybind: keybind_context,
                    seed: i32::try_from(index + 1).unwrap_or(0),
                    ..HudItemSlotState::default()
                },
                team_color,
                partial_tick,
            )
            .unwrap_or_else(|| HudItemIcon {
                lighting: GuiItemLightingEntry::ItemsFlat,
                layers: Vec::new(),
                foil: HudItemFoil::None,
                count_label: hud_item_count_label_for_stack(&entry_stack),
                durability_bar: hud_item_durability_bar_for_stack(world, &entry_stack),
                cooldown_progress: hud_item_cooldown_progress_for_stack(
                    world,
                    item_runtime,
                    &entry_stack,
                    partial_tick,
                ),
                pop: 0.0,
            })
        })
        .collect();
    let selected_index = usize::try_from(selected_item_index)
        .ok()
        .filter(|&idx| idx < shown_count);
    let (selected_item_name, selected_item_tooltip_style) = if let Some(idx) = selected_index {
        let entry = &contents[idx];
        let entry_stack = bbb_protocol::packets::ItemStackSummary {
            item_id: Some(entry.item_id),
            count: entry.count,
            component_patch: entry.component_patch.clone(),
        };
        let entry_effective = world.effective_item_stack(&entry_stack);
        let style = entry_effective.component_patch.tooltip_style.clone();
        let name = item_runtime
            .and_then(|rt| styled_hover_name(rt, entry_effective.as_ref()))
            .map(|hover| HudInventoryTooltipLine {
                text: hover.text,
                tint: [1.0, 1.0, 1.0, 1.0],
                runs: hover.runs,
            })
            .or_else(|| {
                hud_item_tooltip(world, item_runtime, &entry_stack, false, false)
                    .and_then(|tooltip| tooltip.lines.into_iter().next())
            })
            .or_else(|| {
                vanilla_item_type(entry.item_id).map(|descriptor| {
                    let path = descriptor
                        .resource_id
                        .strip_prefix("minecraft:")
                        .unwrap_or(descriptor.resource_id);
                    HudInventoryTooltipLine {
                        text: humanize_resource_path(path),
                        tint: [1.0, 1.0, 1.0, 1.0],
                        runs: Vec::new(),
                    }
                })
            });
        (name, style)
    } else {
        (None, None)
    };
    let occupancy = (weight_num as f64 / weight_den as f64).clamp(0.0, 1.0) as f32;
    let empty_description = localized_gui_text(
        item_runtime,
        "item.minecraft.bundle.empty.description",
        "Can hold a mixed stack of items",
    );
    let progress_label = if weight_num <= 0 {
        Some(localized_gui_text(
            item_runtime,
            "item.minecraft.bundle.empty",
            "Empty",
        ))
    } else if weight_num >= weight_den {
        Some(localized_gui_text(
            item_runtime,
            "item.minecraft.bundle.full",
            "Full",
        ))
    } else {
        None
    };
    Some(bbb_client_render::HudBundleTooltip {
        items,
        hidden_item_count,
        selected_index,
        selected_item_name,
        selected_item_tooltip_style,
        occupancy,
        empty_description,
        progress_label,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn hud_inventory_bundle_tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    hovered_slot_id: Option<i16>,
    tooltip: Option<&HudInventoryTooltip>,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Option<bbb_client_render::HudBundleTooltip> {
    let slot_id = hovered_slot_id?;
    let tooltip = tooltip?;
    if u16::try_from(slot_id).ok() != Some(tooltip.slot_id) {
        return None;
    }
    let container = world.inventory().open_container.as_ref().or_else(|| {
        world
            .local_inventory_is_open()
            .then_some(&world.inventory().inventory_menu)
    })?;
    let slot = world.inventory().container_slot(container, slot_id)?;
    hud_bundle_tooltip_for_stack(
        world,
        item_runtime,
        slot.item,
        slot.local_selected_bundle_item_index,
        shift_down,
        keybind_context,
        partial_tick,
    )
}

pub(super) fn hud_beacon_tooltip(
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<(i32, i32)>,
    selection: (Option<i32>, Option<i32>),
) -> Option<HudInventoryTooltip> {
    let (cursor_x, cursor_y) = cursor_position?;
    for button in beacon_effect_buttons(
        selection
            .0
            .filter(|effect_id| beacon_primary_effect(*effect_id)),
    ) {
        if !beacon_button_contains(cursor_position, button.x, button.y) {
            continue;
        }
        let descriptor = vanilla_mob_effect_type(button.effect_id)?;
        let resource_path = descriptor
            .resource_id
            .strip_prefix("minecraft:")
            .unwrap_or(descriptor.resource_id);
        let translation_key = format!("effect.minecraft.{resource_path}");
        let fallback = humanize_resource_path(resource_path);
        let mut text = localized_gui_text(item_runtime, &translation_key, &fallback);
        if button.upgrade {
            text.push_str(" II");
        }
        return Some(hud_beacon_control_tooltip(text, cursor_x, cursor_y));
    }

    let action = if beacon_button_contains(
        cursor_position,
        BEACON_CONFIRM_BUTTON_X,
        BEACON_ACTION_BUTTON_Y,
    ) {
        Some(("gui.done", "Done"))
    } else if beacon_button_contains(
        cursor_position,
        BEACON_CANCEL_BUTTON_X,
        BEACON_ACTION_BUTTON_Y,
    ) {
        Some(("gui.cancel", "Cancel"))
    } else {
        None
    };
    let (translation_key, fallback) = action?;
    Some(hud_beacon_control_tooltip(
        localized_gui_text(item_runtime, translation_key, fallback),
        cursor_x,
        cursor_y,
    ))
}

fn hud_beacon_control_tooltip(text: String, x: i32, y: i32) -> HudInventoryTooltip {
    HudInventoryTooltip {
        slot_id: u16::MAX,
        x,
        y,
        style: None,
        lines: vec![HudInventoryTooltipLine {
            text,
            tint: [1.0, 1.0, 1.0, 1.0],
            runs: Vec::new(),
        }],
    }
}

pub(super) fn hud_merchant_trade_tooltip_with_inspector(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<(i32, i32)>,
    advanced: bool,
    creative: bool,
    item_inspector: bool,
) -> Option<HudInventoryTooltip> {
    let (cursor_x, cursor_y) = cursor_position?;
    let offers = merchant_offers_state(world)?;
    if offers.can_restock
        && merchant_selected_offer(offers).is_some_and(|offer| offer.is_out_of_stock)
        && (MERCHANT_DEPRECATED_TOOLTIP_X
            ..MERCHANT_DEPRECATED_TOOLTIP_X + MERCHANT_DEPRECATED_TOOLTIP_WIDTH)
            .contains(&cursor_x)
        && (MERCHANT_DEPRECATED_TOOLTIP_Y
            ..MERCHANT_DEPRECATED_TOOLTIP_Y + MERCHANT_DEPRECATED_TOOLTIP_HEIGHT)
            .contains(&cursor_y)
    {
        return Some(HudInventoryTooltip {
            slot_id: u16::MAX,
            x: cursor_x,
            y: cursor_y,
            style: None,
            lines: vec![HudInventoryTooltipLine {
                text: localized_gui_text(
                    item_runtime,
                    "merchant.deprecated",
                    "Villagers restock up to two times per day.",
                ),
                tint: [1.0, 1.0, 1.0, 1.0],
                runs: Vec::new(),
            }],
        });
    }

    if cursor_x < MERCHANT_TRADE_BUTTON_X
        || cursor_x >= MERCHANT_TRADE_BUTTON_X + MERCHANT_TRADE_BUTTON_WIDTH as i32
        || cursor_y < MERCHANT_TRADE_BUTTON_Y
    {
        return None;
    }
    let row = (cursor_y - MERCHANT_TRADE_BUTTON_Y) / MERCHANT_TRADE_ROW_HEIGHT;
    if !(0..MERCHANT_VISIBLE_OFFER_COUNT as i32).contains(&row)
        || cursor_y
            >= MERCHANT_TRADE_BUTTON_Y
                + row * MERCHANT_TRADE_ROW_HEIGHT
                + MERCHANT_TRADE_BUTTON_HEIGHT as i32
    {
        return None;
    }
    let offer_index = merchant_scroll_offset(offers).checked_add(usize::try_from(row).ok()?)?;
    let offer = offers.offers.get(offer_index)?;
    let item = if cursor_x < MERCHANT_TRADE_BUTTON_X + 20 {
        merchant_offer_cost_a_stack(world, offer)
    } else if cursor_x > MERCHANT_TRADE_BUTTON_X + 30 && cursor_x < MERCHANT_TRADE_BUTTON_X + 50 {
        let cost_b = offer.buy_b.as_ref()?;
        item_cost_stack(cost_b, cost_b.count)
    } else if cursor_x > MERCHANT_TRADE_BUTTON_X + 65 {
        offer.sell.clone()
    } else {
        return None;
    };
    let tooltip = hud_item_tooltip_with_inspector(
        world,
        item_runtime,
        &item,
        advanced,
        creative,
        item_inspector,
    )?;
    Some(HudInventoryTooltip {
        slot_id: u16::MAX,
        x: cursor_x,
        y: cursor_y,
        style: tooltip.style,
        lines: tooltip.lines,
    })
}

pub(super) fn hud_stonecutter_recipe_tooltip_with_inspector(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<(i32, i32)>,
    scroll_row: i32,
    advanced: bool,
    creative: bool,
    item_inspector: bool,
) -> Option<HudInventoryTooltip> {
    let (cursor_x, cursor_y) = cursor_position?;
    let option = stonecutter_visible_recipe_option_stacks(world, scroll_row)
        .into_iter()
        .find(|option| {
            cursor_x >= option.x
                && cursor_x < option.x + STONECUTTER_RECIPE_BUTTON_WIDTH
                && cursor_y >= option.y
                && cursor_y < option.y + STONECUTTER_RECIPE_BUTTON_HEIGHT
        })?;
    let tooltip = hud_item_tooltip_with_inspector(
        world,
        item_runtime,
        &option.stack,
        advanced,
        creative,
        item_inspector,
    )?;
    Some(HudInventoryTooltip {
        slot_id: u16::MAX,
        x: cursor_x,
        y: cursor_y,
        style: tooltip.style,
        lines: tooltip.lines,
    })
}

pub(super) fn hud_enchantment_table_tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<(i32, i32)>,
    options: Option<&[EnchantmentOptionHudState; ENCHANTMENT_OPTION_COUNT]>,
) -> Option<HudInventoryTooltip> {
    const ENCHANTMENT_REGISTRY: &str = "minecraft:enchantment";
    const TOOLTIP_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
    const TOOLTIP_GRAY: [f32; 4] = [170.0 / 255.0, 170.0 / 255.0, 170.0 / 255.0, 1.0];
    const TOOLTIP_RED: [f32; 4] = [1.0, 85.0 / 255.0, 85.0 / 255.0, 1.0];

    let (cursor_x, cursor_y) = cursor_position?;
    let option = options?
        .iter()
        .find(|option| option.tooltip_hovered && option.cost > 0)?;
    let holder_id = option.clue_holder_id?;
    let enchantment_level = option.clue_level?;
    let resource_id = world.registry_resource_id_by_holder_id(ENCHANTMENT_REGISTRY, holder_id)?;
    let definitions = world.enchantment_tooltip_definitions_in_holder_order();
    let definition = usize::try_from(holder_id)
        .ok()
        .and_then(|holder_id| definitions.as_ref()?.get(holder_id))
        .and_then(Option::as_ref);
    if definition.is_none() && VanillaEnchantmentIdentity::for_resource_id(resource_id).is_none() {
        return None;
    }

    let enchantment_line = item_runtime
        .map(|runtime| {
            let line =
                enchantment_name_tooltip_line(runtime, resource_id, enchantment_level, definition);
            HudInventoryTooltipLine {
                text: line.text,
                tint: line.tint,
                runs: line.runs,
            }
        })
        .unwrap_or_else(|| {
            fallback_enchantment_name_tooltip_line(resource_id, enchantment_level, definition)
        });
    let mut lines = vec![enchantment_clue_tooltip_line(
        item_runtime,
        enchantment_line,
    )];

    let infinite_materials = world
        .local_player()
        .abilities
        .is_some_and(|abilities| abilities.instabuild);
    if !infinite_materials {
        lines.push(HudInventoryTooltipLine {
            text: String::new(),
            tint: TOOLTIP_WHITE,
            runs: Vec::new(),
        });
        let required_lapis =
            i32::try_from(option.index + 1).expect("three enchanting rows fit i32");
        let experience_level = world
            .local_player()
            .experience
            .map(|experience| experience.level)
            .unwrap_or_default();
        if experience_level < i32::from(option.cost) {
            let cost = option.cost.to_string();
            lines.push(HudInventoryTooltipLine {
                text: localized_gui_text_with_args(
                    item_runtime,
                    "container.enchant.level.requirement",
                    "Level Requirement: %s",
                    &[&cost],
                ),
                tint: TOOLTIP_RED,
                runs: Vec::new(),
            });
        } else {
            let lapis_count = open_container_slot_item(world, 1)
                .filter(|stack| !item_stack_is_empty(stack))
                .map(|stack| stack.count)
                .unwrap_or_default();
            let resource_cost = required_lapis.to_string();
            let (lapis_key, lapis_fallback) = if required_lapis == 1 {
                ("container.enchant.lapis.one", "1 Lapis Lazuli")
            } else {
                ("container.enchant.lapis.many", "%s Lapis Lazuli")
            };
            lines.push(HudInventoryTooltipLine {
                text: localized_gui_text_with_args(
                    item_runtime,
                    lapis_key,
                    lapis_fallback,
                    &[&resource_cost],
                ),
                tint: if lapis_count >= required_lapis {
                    TOOLTIP_GRAY
                } else {
                    TOOLTIP_RED
                },
                runs: Vec::new(),
            });
            let (level_key, level_fallback) = if required_lapis == 1 {
                ("container.enchant.level.one", "1 Enchantment Level")
            } else {
                ("container.enchant.level.many", "%s Enchantment Levels")
            };
            lines.push(HudInventoryTooltipLine {
                text: localized_gui_text_with_args(
                    item_runtime,
                    level_key,
                    level_fallback,
                    &[&resource_cost],
                ),
                tint: TOOLTIP_GRAY,
                runs: Vec::new(),
            });
        }
    }

    Some(HudInventoryTooltip {
        slot_id: u16::MAX,
        x: cursor_x,
        y: cursor_y,
        style: None,
        lines,
    })
}

fn enchantment_clue_tooltip_line(
    item_runtime: Option<&NativeItemRuntime>,
    enchantment: HudInventoryTooltipLine,
) -> HudInventoryTooltipLine {
    let template = localized_gui_text(item_runtime, "container.enchant.clue", "%s . . . ?");
    let mut text = String::new();
    let mut runs = Vec::new();
    visit_localized_template(&template, 1, |part| match part {
        LocalizedTemplatePart::Literal(literal) => {
            text.push_str(literal);
            runs.push(HudComponentTextRun::plain(literal));
        }
        LocalizedTemplatePart::Argument { index: 0, .. } => {
            text.push_str(&enchantment.text);
            runs.extend(enchantment.runs.clone());
        }
        LocalizedTemplatePart::Argument { raw, .. } => {
            text.push_str(raw);
            runs.push(HudComponentTextRun::plain(raw));
        }
    });
    HudInventoryTooltipLine {
        text,
        tint: [1.0, 1.0, 1.0, 1.0],
        runs,
    }
}

fn fallback_enchantment_name_tooltip_line(
    resource_id: &str,
    level: i32,
    definition: Option<&EnchantmentTooltipDefinition>,
) -> HudInventoryTooltipLine {
    let (mut text, mut runs) = if let Some(definition) = definition {
        let mut text = definition.description.clone();
        let mut styled = definition.description_styled.clone();
        if let Some(translation_key) = definition.description_translation_key.as_deref() {
            let fallback = definition
                .description_translation_fallback
                .as_deref()
                .unwrap_or(translation_key);
            let encoded_text = definition
                .description_translation_fallback
                .as_deref()
                .filter(|fallback| text.contains(fallback))
                .unwrap_or(translation_key);
            if let Some(run) = styled.iter_mut().find(|run| run.text == encoded_text) {
                run.text = fallback.to_string();
                text = styled.iter().map(|run| run.text.as_str()).collect();
            } else if text.contains(encoded_text) {
                text = text.replacen(encoded_text, fallback, 1);
            }
        }
        (
            text,
            styled
                .into_iter()
                .map(component_hover::hud_component_run)
                .collect::<Vec<_>>(),
        )
    } else {
        let path = resource_id
            .split_once(':')
            .map(|(_, path)| path)
            .unwrap_or(resource_id);
        (humanize_resource_path(path), Vec::new())
    };
    if runs.is_empty() && !text.is_empty() {
        runs.push(HudComponentTextRun::plain(text.clone()));
    }
    let max_level = definition
        .map(|definition| definition.max_level)
        .or_else(|| {
            VanillaEnchantmentIdentity::for_resource_id(resource_id)
                .map(VanillaEnchantmentIdentity::max_level)
        })
        .unwrap_or(1);
    if level != 1 || max_level != 1 {
        let level_text = match level {
            1 => "I".to_string(),
            2..=10 => mob_effect_level_fallback(level).to_string(),
            _ => level.to_string(),
        };
        text.push(' ');
        text.push_str(&level_text);
        let mut suffix = runs
            .first()
            .cloned()
            .unwrap_or_else(|| HudComponentTextRun::plain(""));
        suffix.visual.text = format!(" {level_text}");
        runs.push(suffix);
    }
    let curse = matches!(
        resource_id,
        "minecraft:binding_curse" | "minecraft:vanishing_curse"
    );
    HudInventoryTooltipLine {
        text,
        tint: if curse {
            [1.0, 85.0 / 255.0, 85.0 / 255.0, 1.0]
        } else {
            [170.0 / 255.0, 170.0 / 255.0, 170.0 / 255.0, 1.0]
        },
        runs,
    }
}
