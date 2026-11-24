use bbb_client_render::item_model::{ItemModelKeybindContext, NativeItemRuntime};
use bbb_client_render::{
    hud_creative_inventory_background_layers, hud_creative_inventory_scroller_y,
    hud_creative_inventory_slot_layouts, hud_creative_inventory_tab_x,
    HudCreativeInventoryBackground, HudCreativeInventoryChrome,
    HudCreativeInventorySlot as CreativeInventorySlot, HudCreativeInventorySlotLayoutKind,
    HudCreativeInventoryTab, HudCreativeInventoryTabRow, HudEntityPreview, HudInventoryFillLayer,
    HudInventoryFillStage, HudInventoryItem, HudInventoryScreen, HudInventorySlot,
    HudInventoryTextLabel, HudInventoryTooltip, HudInventoryTooltipLine,
    HUD_CREATIVE_INVENTORY_HEIGHT, HUD_CREATIVE_INVENTORY_WIDTH, HUD_CREATIVE_PICKER_COLUMNS,
    HUD_CREATIVE_SEARCH_WIDTH, HUD_CREATIVE_SEARCH_X, HUD_CREATIVE_SEARCH_Y,
};
use bbb_client_world::{
    CreativeInventoryBackground, CreativeInventorySnapshot, CreativeInventoryTabRow,
    CreativeInventoryTabSnapshot, CreativeInventoryTabType, WorldStore,
};
use bbb_protocol::{item_types::VANILLA_ITEM_PAPER_ID, packets::ItemStackSummary};

use crate::{
    input::{
        creative_inventory_matching_item_tags, creative_inventory_max_scroll_row,
        creative_inventory_visible_items, CreativeInventoryHover, CreativeInventoryHudState,
    },
    narrator_runtime::{NarratedElementKind, ScreenNarrationFrame},
    options::key_bindings::{BoundInput, KeyBindings},
};
use bbb_client_render::scene::terrain_textures::TerrainTextureState;

use super::inventory_items::{
    hud_inventory_quick_craft_slot_preview, push_yellow_item_count_label,
    QUICK_CRAFT_SLOT_FILL_COLOR,
};
use super::{
    hud_entity_in_inventory_follows_mouse_preview, hud_item_tooltip_with_inspector,
    localized_gui_text, localized_gui_text_with_args, push_hud_inventory_cursor_item,
    HudItemModelContext, HudItemSlotState, HudItemTooltipContent,
};

const CREATIVE_SEARCH_TEXT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const CREATIVE_TITLE_COLOR: [f32; 4] = [64.0 / 255.0, 64.0 / 255.0, 64.0 / 255.0, 1.0];
const CREATIVE_TAG_COLOR: [f32; 4] = [170.0 / 255.0, 0.0, 170.0 / 255.0, 1.0];
const CREATIVE_TAB_MEMBERSHIP_COLOR: [f32; 4] = [85.0 / 255.0, 85.0 / 255.0, 1.0, 1.0];

#[allow(clippy::too_many_arguments)]
pub(super) fn hud_creative_inventory_screen(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    state: CreativeInventoryHudState,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    advanced_item_tooltips: bool,
    item_inspector: bool,
    surface_size: winit::dpi::PhysicalSize<u32>,
    hud_glyphs: &bbb_client_render::HudFontGlyphMap,
    partial_tick: f32,
) -> Option<HudInventoryScreen> {
    let snapshot = world.creative_inventory_snapshot()?;
    let selected = snapshot.tab(&state.selected_tab)?;
    let hotbar_items = (selected.tab_type == CreativeInventoryTabType::Hotbar).then(|| {
        creative_hotbar_items(
            &state.saved_hotbars,
            &state.hotbar_key_bindings,
            item_runtime,
        )
    });
    let visible_items = hotbar_items.as_ref().map_or_else(
        || creative_inventory_visible_items(world, item_runtime, selected, &state.search.text()),
        |items| items.iter().collect(),
    );
    let max_scroll_row = creative_inventory_max_scroll_row(visible_items.len());
    let scroll_row = state.scroll_row.min(max_scroll_row);

    let mut fill_layers = Vec::new();
    let mut text_labels = creative_inventory_text_labels(item_runtime, selected, &state);
    let slots = creative_inventory_slots(
        world,
        item_runtime,
        terrain_textures,
        selected,
        &visible_items,
        scroll_row,
        state.quick_craft_button_num,
        &state.quick_craft_slots,
        shift_down,
        keybind_context,
        hud_glyphs,
        partial_tick,
        &mut fill_layers,
        &mut text_labels,
    );
    let background_layers = hud_creative_inventory_background_layers(&creative_inventory_chrome(
        snapshot,
        selected,
        scroll_row,
        max_scroll_row,
    ));
    let mut floating_items = creative_inventory_tab_icons(
        world,
        item_runtime,
        terrain_textures,
        snapshot,
        shift_down,
        keybind_context,
        partial_tick,
    );
    push_hud_inventory_cursor_item(
        world,
        item_runtime,
        terrain_textures,
        state.cursor_position,
        state.quick_craft_button_num,
        &state.quick_craft_slots,
        shift_down,
        keybind_context,
        hud_glyphs,
        partial_tick,
        &mut floating_items,
        &mut text_labels,
    );

    let hovered_slot = match state.hovered {
        Some(CreativeInventoryHover::Slot(slot)) => Some(slot),
        _ => None,
    };
    let tooltip = creative_inventory_tooltip(
        world,
        item_runtime,
        snapshot,
        selected,
        &visible_items,
        scroll_row,
        &state,
        advanced_item_tooltips,
        item_inspector,
    );
    let entity_previews = hud_creative_inventory_player_preview(
        world,
        item_runtime,
        selected.tab_type,
        &state,
        partial_tick,
    )
    .into_iter()
    .collect();

    let preedit_overlay = (selected.tab_type == CreativeInventoryTabType::Search
        && state.search_focused)
        .then(|| {
            bbb_client_render::hud_text_field_preedit_overlay(
                &state.search.hud(true, false),
                HUD_CREATIVE_INVENTORY_WIDTH,
                HUD_CREATIVE_INVENTORY_HEIGHT,
                HUD_CREATIVE_SEARCH_X,
                HUD_CREATIVE_SEARCH_Y,
                HUD_CREATIVE_SEARCH_WIDTH,
                10,
                surface_size,
                hud_glyphs,
            )
        })
        .flatten();

    Some(HudInventoryScreen {
        width: HUD_CREATIVE_INVENTORY_WIDTH,
        height: HUD_CREATIVE_INVENTORY_HEIGHT,
        background_layers,
        foreground_layers: Vec::new(),
        fill_layers,
        slots,
        floating_items,
        foreground_items: Vec::new(),
        ghost_items: Vec::new(),
        entity_previews,
        map_previews: Vec::new(),
        banner_pattern_previews: Vec::new(),
        text_labels,
        scroll_area: None,
        preedit_overlay,
        hovered_slot_id: hovered_slot.map(CreativeInventorySlot::hud_id),
        tooltip,
    })
}

pub(super) fn hud_creative_inventory_player_preview(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    selected_tab_type: CreativeInventoryTabType,
    state: &CreativeInventoryHudState,
    partial_tick: f32,
) -> Option<HudEntityPreview> {
    const X0: i32 = 73;
    const Y0: i32 = 6;
    const X1: i32 = 105;
    const Y1: i32 = 49;
    const SCALE: f32 = 20.0;
    const OFFSET_Y: f32 = 0.0625;

    if selected_tab_type != CreativeInventoryTabType::Inventory {
        return None;
    }
    let local_player_id = world.local_player_id()?;
    hud_entity_in_inventory_follows_mouse_preview(
        world,
        item_runtime,
        local_player_id,
        state.cursor_position,
        partial_tick,
        [X0, Y0, X1, Y1],
        SCALE,
        OFFSET_Y,
    )
}

pub(super) fn creative_inventory_narration_frame(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    state: &CreativeInventoryHudState,
) -> Option<ScreenNarrationFrame> {
    let snapshot = world.creative_inventory_snapshot()?;
    creative_inventory_narration_frame_for_snapshot(snapshot, item_runtime, state)
}

fn creative_inventory_narration_frame_for_snapshot(
    snapshot: &CreativeInventorySnapshot,
    item_runtime: Option<&NativeItemRuntime>,
    state: &CreativeInventoryHudState,
) -> Option<ScreenNarrationFrame> {
    let selected = snapshot.tab(&state.selected_tab)?;
    let mut frame = ScreenNarrationFrame::default();
    frame.add(
        NarratedElementKind::Title,
        0,
        localized_tab_title(item_runtime, selected),
    );
    match state.hovered {
        Some(CreativeInventoryHover::Tab(index)) => {
            let tab = snapshot.tabs.get(index)?;
            frame.add(
                NarratedElementKind::Position,
                1,
                localized_tab_title(item_runtime, tab),
            );
            frame.add(
                NarratedElementKind::Usage,
                1,
                localized_gui_text(
                    item_runtime,
                    "narration.button.usage.hovered",
                    "Left click to activate",
                ),
            );
        }
        Some(CreativeInventoryHover::Search) | None
            if selected.tab_type == CreativeInventoryTabType::Search && state.search_focused =>
        {
            let label = localized_gui_text(item_runtime, "itemGroup.search", "Search Items");
            frame.add(
                NarratedElementKind::Position,
                1,
                localized_gui_text_with_args(
                    item_runtime,
                    "gui.narrate.editBox",
                    "%s edit box: %s",
                    &[&label, &state.search.text()],
                ),
            );
        }
        Some(CreativeInventoryHover::Slot(CreativeInventorySlot::Trash)) => {
            frame.add(
                NarratedElementKind::Position,
                1,
                localized_gui_text(item_runtime, "inventory.binSlot", "Destroy Item"),
            );
        }
        _ => {}
    }
    Some(frame)
}

fn creative_inventory_chrome(
    snapshot: &CreativeInventorySnapshot,
    selected: &CreativeInventoryTabSnapshot,
    scroll_row: usize,
    max_scroll_row: usize,
) -> HudCreativeInventoryChrome {
    HudCreativeInventoryChrome {
        background: match selected.background {
            CreativeInventoryBackground::Items => HudCreativeInventoryBackground::Items,
            CreativeInventoryBackground::ItemSearch => HudCreativeInventoryBackground::ItemSearch,
            CreativeInventoryBackground::Inventory => HudCreativeInventoryBackground::Inventory,
        },
        tabs: snapshot
            .tabs
            .iter()
            .map(|tab| HudCreativeInventoryTab {
                row: match tab.row {
                    CreativeInventoryTabRow::Top => HudCreativeInventoryTabRow::Top,
                    CreativeInventoryTabRow::Bottom => HudCreativeInventoryTabRow::Bottom,
                },
                column: tab.column,
                aligned_right: tab.aligned_right,
                selected: tab.resource_id == selected.resource_id,
            })
            .collect(),
        can_scroll: selected.can_scroll,
        scroll_enabled: max_scroll_row > 0,
        scroller_y: hud_creative_inventory_scroller_y(scroll_row, max_scroll_row),
    }
}

const HOTBAR_KEY_MAPPING_NAMES: [&str; 9] = [
    "key.hotbar.1",
    "key.hotbar.2",
    "key.hotbar.3",
    "key.hotbar.4",
    "key.hotbar.5",
    "key.hotbar.6",
    "key.hotbar.7",
    "key.hotbar.8",
    "key.hotbar.9",
];

fn bound_input_display_key_and_default(input: BoundInput) -> (&'static str, String) {
    use winit::{event::MouseButton, keyboard::KeyCode};
    match input {
        BoundInput::Unbound => ("key.keyboard.unknown", "Not Bound".to_string()),
        BoundInput::Mouse(MouseButton::Left) => ("key.mouse.left", "Left Button".to_string()),
        BoundInput::Mouse(MouseButton::Right) => ("key.mouse.right", "Right Button".to_string()),
        BoundInput::Mouse(MouseButton::Middle) => ("key.mouse.middle", "Middle Button".to_string()),
        BoundInput::Mouse(MouseButton::Back) => ("key.mouse.4", "Button 4".to_string()),
        BoundInput::Mouse(MouseButton::Forward) => ("key.mouse.5", "Button 5".to_string()),
        BoundInput::Mouse(MouseButton::Other(button)) => {
            ("key.mouse.unknown", format!("Button {}", button + 1))
        }
        BoundInput::Key(code) => match code {
            KeyCode::KeyA => ("key.keyboard.a", "A".to_string()),
            KeyCode::KeyB => ("key.keyboard.b", "B".to_string()),
            KeyCode::KeyC => ("key.keyboard.c", "C".to_string()),
            KeyCode::KeyD => ("key.keyboard.d", "D".to_string()),
            KeyCode::KeyE => ("key.keyboard.e", "E".to_string()),
            KeyCode::KeyF => ("key.keyboard.f", "F".to_string()),
            KeyCode::KeyG => ("key.keyboard.g", "G".to_string()),
            KeyCode::KeyH => ("key.keyboard.h", "H".to_string()),
            KeyCode::KeyI => ("key.keyboard.i", "I".to_string()),
            KeyCode::KeyJ => ("key.keyboard.j", "J".to_string()),
            KeyCode::KeyK => ("key.keyboard.k", "K".to_string()),
            KeyCode::KeyL => ("key.keyboard.l", "L".to_string()),
            KeyCode::KeyM => ("key.keyboard.m", "M".to_string()),
            KeyCode::KeyN => ("key.keyboard.n", "N".to_string()),
            KeyCode::KeyO => ("key.keyboard.o", "O".to_string()),
            KeyCode::KeyP => ("key.keyboard.p", "P".to_string()),
            KeyCode::KeyQ => ("key.keyboard.q", "Q".to_string()),
            KeyCode::KeyR => ("key.keyboard.r", "R".to_string()),
            KeyCode::KeyS => ("key.keyboard.s", "S".to_string()),
            KeyCode::KeyT => ("key.keyboard.t", "T".to_string()),
            KeyCode::KeyU => ("key.keyboard.u", "U".to_string()),
            KeyCode::KeyV => ("key.keyboard.v", "V".to_string()),
            KeyCode::KeyW => ("key.keyboard.w", "W".to_string()),
            KeyCode::KeyX => ("key.keyboard.x", "X".to_string()),
            KeyCode::KeyY => ("key.keyboard.y", "Y".to_string()),
            KeyCode::KeyZ => ("key.keyboard.z", "Z".to_string()),
            KeyCode::Digit0 => ("key.keyboard.0", "0".to_string()),
            KeyCode::Digit1 => ("key.keyboard.1", "1".to_string()),
            KeyCode::Digit2 => ("key.keyboard.2", "2".to_string()),
            KeyCode::Digit3 => ("key.keyboard.3", "3".to_string()),
            KeyCode::Digit4 => ("key.keyboard.4", "4".to_string()),
            KeyCode::Digit5 => ("key.keyboard.5", "5".to_string()),
            KeyCode::Digit6 => ("key.keyboard.6", "6".to_string()),
            KeyCode::Digit7 => ("key.keyboard.7", "7".to_string()),
            KeyCode::Digit8 => ("key.keyboard.8", "8".to_string()),
            KeyCode::Digit9 => ("key.keyboard.9", "9".to_string()),
            other => ("key.keyboard.unknown", format!("{other:?}")),
        },
    }
}

fn localized_bound_key_label(
    key_bindings: &KeyBindings,
    item_runtime: Option<&NativeItemRuntime>,
    mapping_name: &str,
) -> String {
    let (key_id, default_label) =
        bound_input_display_key_and_default(key_bindings.input(mapping_name));
    let display_key = if key_id == "key.keyboard.unknown" {
        crate::options::key_bindings::bound_key_display_key(mapping_name).unwrap_or(key_id)
    } else {
        key_id
    };
    localized_gui_text(item_runtime, display_key, &default_label)
}

/// Vanilla `CreativeModeInventoryScreen.selectTab` (`CreativeModeInventoryScreen.java:536-558`):
/// populate each of the 9 saved-hotbar rows with its saved items when non-empty,
/// or a diagonal locked-paper placeholder citing the configured save + hotbar keys.
fn creative_hotbar_items(
    saved_hotbars: &[Vec<ItemStackSummary>],
    key_bindings: &KeyBindings,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<ItemStackSummary> {
    let activator_key =
        localized_bound_key_label(key_bindings, item_runtime, "key.saveToolbarActivator");
    let mut items = Vec::with_capacity(81);
    for (hotbar, hotbar_mapping) in HOTBAR_KEY_MAPPING_NAMES.iter().enumerate() {
        if let Some(saved_row) = saved_hotbars
            .get(hotbar)
            .filter(|row| row.iter().any(|stack| !super::item_stack_is_empty(stack)))
        {
            for column in 0..9 {
                items.push(
                    saved_row
                        .get(column)
                        .cloned()
                        .unwrap_or_else(ItemStackSummary::empty),
                );
            }
            continue;
        }
        let hotbar_key = localized_bound_key_label(key_bindings, item_runtime, hotbar_mapping);
        for column in 0..9 {
            if column == hotbar {
                let mut placeholder = ItemStackSummary {
                    item_id: Some(VANILLA_ITEM_PAPER_ID),
                    count: 1,
                    component_patch: Default::default(),
                };
                placeholder.component_patch.creative_slot_lock = true;
                placeholder.component_patch.item_name = Some(localized_gui_text_with_args(
                    item_runtime,
                    "inventory.hotbarInfo",
                    "Save hotbar with %s + %s",
                    &[&activator_key, &hotbar_key],
                ));
                items.push(placeholder);
            } else {
                items.push(ItemStackSummary::empty());
            }
        }
    }
    items
}

#[allow(clippy::too_many_arguments)]
fn creative_inventory_slots(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    selected: &CreativeInventoryTabSnapshot,
    visible_items: &[&ItemStackSummary],
    scroll_row: usize,
    quick_craft_button_num: Option<i8>,
    quick_craft_slots: &[i16],
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    hud_glyphs: &bbb_client_render::HudFontGlyphMap,
    partial_tick: f32,
    fill_layers: &mut Vec<HudInventoryFillLayer>,
    text_labels: &mut Vec<HudInventoryTextLabel>,
) -> Vec<HudInventorySlot> {
    let item_models = HudItemModelContext::new(world, item_runtime);
    let selected_hotbar_slot = super::inventory_screen_selected_hotbar_slot_id(world);
    let inventory_menu = &world.inventory().inventory_menu;
    hud_creative_inventory_slot_layouts(
        if selected.tab_type == CreativeInventoryTabType::Inventory {
            HudCreativeInventorySlotLayoutKind::Inventory
        } else {
            HudCreativeInventorySlotLayoutKind::Picker
        },
    )
    .map(|layout| {
        let menu_slot_id = match layout.slot {
            CreativeInventorySlot::Hotbar(index) => i16::try_from(36 + index).ok(),
            CreativeInventorySlot::Inventory(slot) => Some(slot),
            CreativeInventorySlot::Picker(_) | CreativeInventorySlot::Trash => None,
        };
        let quick_craft_preview = menu_slot_id.and_then(|slot_id| {
            hud_inventory_quick_craft_slot_preview(
                world,
                inventory_menu,
                slot_id,
                quick_craft_button_num,
                quick_craft_slots,
            )
        });
        let selected_item = menu_slot_id.is_some() && menu_slot_id == selected_hotbar_slot;
        let (projection, capped_count) = if let Some((preview_item, capped)) = &quick_craft_preview
        {
            fill_layers.push(HudInventoryFillLayer {
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
                    selected_item,
                    shift_down,
                    keybind: keybind_context,
                    ..HudItemSlotState::default()
                },
                world.local_player_team_color_rgb(),
            );
            (projection, (*capped).then_some(preview_item.count))
        } else {
            let item = creative_inventory_slot_item(world, visible_items, scroll_row, layout.slot);
            let bundle_selected_item_index = menu_slot_id
                .and_then(|slot_id| world.inventory().container_slot(inventory_menu, slot_id))
                .and_then(|slot| {
                    (slot.local_selected_bundle_item_index >= 0)
                        .then_some(slot.local_selected_bundle_item_index)
                });
            let projection = item.and_then(|item| {
                item_models.project(
                    item,
                    HudItemSlotState {
                        bundle_selected_item_index,
                        selected_item,
                        shift_down,
                        keybind: keybind_context,
                        ..HudItemSlotState::default()
                    },
                    world.local_player_team_color_rgb(),
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
            push_yellow_item_count_label(text_labels, layout.x, layout.y, capped_count, hud_glyphs);
        }
        HudInventorySlot {
            slot_id: layout.slot.hud_id(),
            x: layout.x,
            y: layout.y,
            icon,
            block_model: projection
                .as_ref()
                .and_then(|projection| projection.block_model(terrain_textures)),
        }
    })
    .collect()
}

fn creative_inventory_slot_item<'a>(
    world: &'a WorldStore,
    visible_items: &'a [&'a ItemStackSummary],
    scroll_row: usize,
    slot: CreativeInventorySlot,
) -> Option<&'a ItemStackSummary> {
    match slot {
        CreativeInventorySlot::Picker(index) => visible_items
            .get(scroll_row * HUD_CREATIVE_PICKER_COLUMNS + index)
            .copied(),
        CreativeInventorySlot::Hotbar(index) => {
            inventory_menu_item(world, i16::try_from(36 + index).ok()?)
        }
        CreativeInventorySlot::Inventory(slot) => inventory_menu_item(world, slot),
        CreativeInventorySlot::Trash => None,
    }
}

fn inventory_menu_item(world: &WorldStore, slot: i16) -> Option<&ItemStackSummary> {
    world
        .inventory()
        .inventory_menu_slot_item(slot)
        .filter(|item| !super::item_stack_is_empty(item))
}

#[allow(clippy::too_many_arguments)]
fn creative_inventory_tab_icons(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    snapshot: &CreativeInventorySnapshot,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    let item_models = HudItemModelContext::new(world, item_runtime);
    snapshot
        .tabs
        .iter()
        .filter_map(|tab| {
            let projection = item_models.project(
                &tab.icon,
                HudItemSlotState {
                    shift_down,
                    keybind: keybind_context,
                    ..HudItemSlotState::default()
                },
                world.local_player_team_color_rgb(),
            );
            let icon = projection
                .as_ref()
                .and_then(|projection| projection.icon(partial_tick))?;
            let x = hud_creative_inventory_tab_x(tab.column, tab.aligned_right) + 5;
            let y = match tab.row {
                CreativeInventoryTabRow::Top => -19,
                CreativeInventoryTabRow::Bottom => 139,
            };
            Some(HudInventoryItem {
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
            })
        })
        .collect()
}

fn creative_inventory_text_labels(
    item_runtime: Option<&NativeItemRuntime>,
    selected: &CreativeInventoryTabSnapshot,
    state: &CreativeInventoryHudState,
) -> Vec<HudInventoryTextLabel> {
    let mut labels = Vec::with_capacity(2);
    if selected.show_title {
        labels.push(HudInventoryTextLabel {
            component_runs: Vec::new(),
            x: 8,
            y: 6,
            width: 170,
            text: localized_tab_title(item_runtime, selected),
            tint: CREATIVE_TITLE_COLOR,
            background: None,
            input: None,
            scrolling: None,
            shadow: false,
            font: None,
            runs: Vec::new(),
        });
    }
    if selected.tab_type == CreativeInventoryTabType::Search {
        labels.push(HudInventoryTextLabel {
            component_runs: Vec::new(),
            x: HUD_CREATIVE_SEARCH_X,
            y: HUD_CREATIVE_SEARCH_Y,
            width: HUD_CREATIVE_SEARCH_WIDTH,
            text: state.search.text(),
            tint: CREATIVE_SEARCH_TEXT_COLOR,
            background: None,
            input: Some(
                state
                    .search
                    .hud(state.search_focused, false)
                    .decoration(CREATIVE_SEARCH_TEXT_COLOR),
            ),
            scrolling: None,
            shadow: false,
            font: None,
            runs: Vec::new(),
        });
    }
    labels
}

#[allow(clippy::too_many_arguments)]
fn creative_inventory_tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    snapshot: &CreativeInventorySnapshot,
    selected: &CreativeInventoryTabSnapshot,
    visible_items: &[&ItemStackSummary],
    scroll_row: usize,
    state: &CreativeInventoryHudState,
    advanced: bool,
    item_inspector: bool,
) -> Option<HudInventoryTooltip> {
    let (cursor_x, cursor_y) = state.cursor_position?;
    match state.hovered? {
        CreativeInventoryHover::Tab(index) => {
            let tab = snapshot.tabs.get(index)?;
            plain_tooltip(
                u16::MAX,
                cursor_x,
                cursor_y,
                localized_tab_title(item_runtime, tab),
            )
        }
        CreativeInventoryHover::Search | CreativeInventoryHover::Scroller => None,
        CreativeInventoryHover::Slot(CreativeInventorySlot::Trash) => plain_tooltip(
            CreativeInventorySlot::Trash.hud_id(),
            cursor_x,
            cursor_y,
            localized_gui_text(item_runtime, "inventory.binSlot", "Destroy Item"),
        ),
        CreativeInventoryHover::Slot(slot) => {
            let item = creative_inventory_slot_item(world, visible_items, scroll_row, slot)?;
            let creative_slot = matches!(slot, CreativeInventorySlot::Picker(_));
            let mut content = hud_item_tooltip_with_inspector(
                world,
                item_runtime,
                item,
                advanced,
                creative_slot,
                item_inspector,
            )?;
            augment_creative_item_tooltip(
                world,
                item_runtime,
                snapshot,
                selected,
                item,
                creative_slot,
                &state.search.text(),
                &mut content,
            );
            Some(HudInventoryTooltip {
                slot_id: slot.hud_id(),
                x: cursor_x,
                y: cursor_y,
                style: content.style,
                lines: content.lines,
            })
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn augment_creative_item_tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    snapshot: &CreativeInventorySnapshot,
    selected: &CreativeInventoryTabSnapshot,
    item: &ItemStackSummary,
    creative_slot: bool,
    search_text: &str,
    content: &mut HudItemTooltipContent,
) {
    if content.lines.is_empty()
        || (selected.tab_type == CreativeInventoryTabType::Category && creative_slot)
    {
        return;
    }
    let mut matching_tags = Vec::new();
    if selected.tab_type == CreativeInventoryTabType::Search
        && creative_slot
        && search_text.starts_with('#')
    {
        for tag in creative_inventory_matching_item_tags(world, item, search_text) {
            matching_tags.push(HudInventoryTooltipLine {
                text: format!("#{tag}"),
                tint: CREATIVE_TAG_COLOR,
                runs: Vec::new(),
            });
        }
    }
    let mut insert_at = 1;
    for tab in snapshot.tabs.iter().filter(|tab| {
        tab.tab_type != CreativeInventoryTabType::Search
            && tab
                .items
                .iter()
                .any(|row| super::item_stacks_match_by_item_and_components(row, item))
    }) {
        content.lines.insert(
            insert_at,
            HudInventoryTooltipLine {
                text: localized_tab_title(item_runtime, tab),
                tint: CREATIVE_TAB_MEMBERSHIP_COLOR,
                runs: Vec::new(),
            },
        );
        insert_at += 1;
    }
    for tag in matching_tags.into_iter().rev() {
        content.lines.insert(insert_at, tag);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn hud_creative_inventory_bundle_tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    state: &CreativeInventoryHudState,
    tooltip: Option<&HudInventoryTooltip>,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Option<bbb_client_render::HudBundleTooltip> {
    let tooltip = tooltip?;
    let CreativeInventoryHover::Slot(slot) = state.hovered? else {
        return None;
    };
    if slot.hud_id() != tooltip.slot_id {
        return None;
    }
    let snapshot = world.creative_inventory_snapshot()?;
    let selected = snapshot.tab(&state.selected_tab)?;
    let hotbar_items = (selected.tab_type == CreativeInventoryTabType::Hotbar).then(|| {
        creative_hotbar_items(
            &state.saved_hotbars,
            &state.hotbar_key_bindings,
            item_runtime,
        )
    });
    let visible_items = hotbar_items.as_ref().map_or_else(
        || creative_inventory_visible_items(world, item_runtime, selected, &state.search.text()),
        |items| items.iter().collect(),
    );
    let max_scroll_row = creative_inventory_max_scroll_row(visible_items.len());
    let scroll_row = state.scroll_row.min(max_scroll_row);
    let (item, selected_item_index) = match slot {
        CreativeInventorySlot::Hotbar(index) => {
            let slot_id = i16::try_from(36 + index).ok()?;
            let s = world
                .inventory()
                .container_slot(&world.inventory().inventory_menu, slot_id)?;
            (s.item, s.local_selected_bundle_item_index)
        }
        CreativeInventorySlot::Inventory(slot_id) => {
            let s = world
                .inventory()
                .container_slot(&world.inventory().inventory_menu, slot_id)?;
            (s.item, s.local_selected_bundle_item_index)
        }
        CreativeInventorySlot::Picker(_) => (
            creative_inventory_slot_item(world, &visible_items, scroll_row, slot)?,
            -1,
        ),
        CreativeInventorySlot::Trash => return None,
    };
    super::hud_bundle_tooltip_for_stack(
        world,
        item_runtime,
        item,
        selected_item_index,
        shift_down,
        keybind_context,
        partial_tick,
    )
}

fn plain_tooltip(slot_id: u16, x: i32, y: i32, text: String) -> Option<HudInventoryTooltip> {
    (!text.is_empty()).then_some(HudInventoryTooltip {
        slot_id,
        x,
        y,
        style: None,
        lines: vec![HudInventoryTooltipLine {
            text,
            tint: [1.0, 1.0, 1.0, 1.0],
            runs: Vec::new(),
        }],
    })
}

fn localized_tab_title(
    item_runtime: Option<&NativeItemRuntime>,
    tab: &CreativeInventoryTabSnapshot,
) -> String {
    let fallback = tab
        .resource_id
        .split_once(':')
        .map_or(tab.resource_id.as_str(), |(_, path)| path)
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    localized_gui_text(item_runtime, &tab.title_key, &fallback)
}

#[cfg(test)]
mod tests {
    use bbb_client_world::{CreativeInventoryMaterializationRevision, RegistryAccessRevision};
    use bbb_protocol::packets::{RegistryTags, TagNetworkPayload, UpdateTags};

    use super::*;

    fn stack(id: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(id),
            count: 1,
            component_patch: Default::default(),
        }
    }

    fn tab(
        id: &str,
        tab_type: CreativeInventoryTabType,
        items: Vec<ItemStackSummary>,
    ) -> CreativeInventoryTabSnapshot {
        CreativeInventoryTabSnapshot {
            resource_id: id.to_string(),
            row: CreativeInventoryTabRow::Top,
            column: 0,
            title_key: format!("itemGroup.{id}"),
            icon: stack(1),
            tab_type,
            background: CreativeInventoryBackground::Items,
            aligned_right: false,
            show_title: true,
            can_scroll: true,
            items,
        }
    }

    fn snapshot(tabs: Vec<CreativeInventoryTabSnapshot>) -> CreativeInventorySnapshot {
        CreativeInventorySnapshot {
            generation: 1,
            revision: CreativeInventoryMaterializationRevision {
                registry: RegistryAccessRevision::default(),
                enabled_features: vec!["minecraft:vanilla".to_string()],
                has_permissions: false,
            },
            tabs,
        }
    }

    #[test]
    fn chrome_projects_all_tab_types_and_selected_layer_last() {
        for (tab_type, background, expected) in [
            (
                CreativeInventoryTabType::Category,
                CreativeInventoryBackground::Items,
                HudCreativeInventoryBackground::Items,
            ),
            (
                CreativeInventoryTabType::Hotbar,
                CreativeInventoryBackground::Items,
                HudCreativeInventoryBackground::Items,
            ),
            (
                CreativeInventoryTabType::Search,
                CreativeInventoryBackground::ItemSearch,
                HudCreativeInventoryBackground::ItemSearch,
            ),
            (
                CreativeInventoryTabType::Inventory,
                CreativeInventoryBackground::Inventory,
                HudCreativeInventoryBackground::Inventory,
            ),
        ] {
            let mut selected = tab("minecraft:selected", tab_type, Vec::new());
            selected.background = background;
            selected.can_scroll = tab_type != CreativeInventoryTabType::Inventory;
            let snapshot = snapshot(vec![
                selected.clone(),
                tab(
                    "minecraft:other",
                    CreativeInventoryTabType::Category,
                    Vec::new(),
                ),
            ]);
            let chrome = creative_inventory_chrome(&snapshot, &selected, 0, 0);
            assert_eq!(chrome.background, expected);
            let layers = hud_creative_inventory_background_layers(&chrome);
            assert!(matches!(
                layers.last().unwrap().texture,
                bbb_client_render::HudInventoryBackgroundTexture::Sprite(id)
                    if id.contains("_selected_")
            ));
        }
    }

    #[test]
    fn picker_projection_applies_scroll_row_before_slot_index() {
        let items = (0..60).map(stack).collect::<Vec<_>>();
        let refs = items.iter().collect::<Vec<_>>();
        let world = WorldStore::default();
        assert_eq!(
            creative_inventory_slot_item(&world, &refs, 1, CreativeInventorySlot::Picker(0))
                .and_then(|item| item.item_id),
            Some(9)
        );
        assert_eq!(
            creative_inventory_slot_item(&world, &refs, 2, CreativeInventorySlot::Picker(44))
                .and_then(|item| item.item_id),
            None
        );
    }

    #[test]
    fn search_edit_box_uses_borderless_vanilla_text_geometry() {
        let mut search = tab(
            "minecraft:search",
            CreativeInventoryTabType::Search,
            Vec::new(),
        );
        search.show_title = false;
        let labels = creative_inventory_text_labels(
            None,
            &search,
            &CreativeInventoryHudState {
                selected_tab: search.resource_id.clone(),
                search: crate::input::text_field::TextField::with_text("stone", 50),
                search_focused: true,
                scroll_row: 0,
                hovered: None,
                cursor_position: None,
                saved_hotbars: Vec::new(),
                hotbar_key_bindings: KeyBindings::default(),
                quick_craft_button_num: None,
                quick_craft_slots: Vec::new(),
            },
        );
        assert_eq!(labels.len(), 1);
        assert_eq!((labels[0].x, labels[0].y, labels[0].width), (82, 6, 80));
        assert_eq!(
            labels[0]
                .input
                .as_ref()
                .map(|input| (input.cursor, input.selection, input.max_length)),
            Some((5, 5, 50))
        );
    }

    #[test]
    fn category_creative_slots_keep_original_tooltip_but_search_inserts_memberships() {
        let item = stack(7);
        let category = tab(
            "minecraft:building_blocks",
            CreativeInventoryTabType::Category,
            vec![item.clone()],
        );
        let search = tab(
            "minecraft:search",
            CreativeInventoryTabType::Search,
            vec![item.clone()],
        );
        let catalog_snapshot = snapshot(vec![category.clone(), search.clone()]);
        let mut content = HudItemTooltipContent {
            style: None,
            lines: vec![HudInventoryTooltipLine {
                text: "Stone".to_string(),
                tint: [1.0; 4],
                runs: Vec::new(),
            }],
        };
        augment_creative_item_tooltip(
            &WorldStore::default(),
            None,
            &catalog_snapshot,
            &category,
            &item,
            true,
            "",
            &mut content,
        );
        assert_eq!(content.lines.len(), 1);
        augment_creative_item_tooltip(
            &WorldStore::default(),
            None,
            &catalog_snapshot,
            &search,
            &item,
            true,
            "",
            &mut content,
        );
        assert_eq!(content.lines.len(), 2);
        assert_eq!(content.lines[1].text, "Building Blocks");
        assert_eq!(content.lines[1].tint, CREATIVE_TAB_MEMBERSHIP_COLOR);

        let inventory = tab(
            "minecraft:inventory",
            CreativeInventoryTabType::Inventory,
            Vec::new(),
        );
        let catalog_snapshot = snapshot(vec![category, search, inventory.clone()]);
        let mut inventory_item = item;
        inventory_item.count = 64;
        let mut content = HudItemTooltipContent {
            style: None,
            lines: vec![HudInventoryTooltipLine {
                text: "Stone".to_string(),
                tint: [1.0; 4],
                runs: Vec::new(),
            }],
        };
        augment_creative_item_tooltip(
            &WorldStore::default(),
            None,
            &catalog_snapshot,
            &inventory,
            &inventory_item,
            false,
            "",
            &mut content,
        );
        assert_eq!(content.lines[1].text, "Building Blocks");
    }

    #[test]
    fn tag_search_tooltip_inserts_tab_memberships_before_matching_tags() {
        let item = stack(7);
        let category = tab(
            "minecraft:building_blocks",
            CreativeInventoryTabType::Category,
            vec![item.clone()],
        );
        let search = tab(
            "minecraft:search",
            CreativeInventoryTabType::Search,
            vec![item.clone()],
        );
        let snapshot = snapshot(vec![category, search.clone()]);
        let mut world = WorldStore::default();
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:stone_family".to_string(),
                    entries: vec![7],
                }],
            }],
        });
        let mut content = HudItemTooltipContent {
            style: None,
            lines: vec![HudInventoryTooltipLine {
                text: "Stone".to_string(),
                tint: [1.0; 4],
                runs: Vec::new(),
            }],
        };

        augment_creative_item_tooltip(
            &world,
            None,
            &snapshot,
            &search,
            &item,
            true,
            "#stone",
            &mut content,
        );

        assert_eq!(
            content
                .lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>(),
            vec!["Stone", "Building Blocks", "#minecraft:stone_family"]
        );
        assert_eq!(content.lines[1].tint, CREATIVE_TAB_MEMBERSHIP_COLOR);
        assert_eq!(content.lines[2].tint, CREATIVE_TAG_COLOR);
    }

    #[test]
    fn narration_prioritizes_tab_search_and_trash_context() {
        let category = tab(
            "minecraft:building_blocks",
            CreativeInventoryTabType::Category,
            Vec::new(),
        );
        let search = tab(
            "minecraft:search",
            CreativeInventoryTabType::Search,
            Vec::new(),
        );
        let inventory = tab(
            "minecraft:inventory",
            CreativeInventoryTabType::Inventory,
            Vec::new(),
        );
        let snapshot = snapshot(vec![category, search, inventory]);

        let search_frame = creative_inventory_narration_frame_for_snapshot(
            &snapshot,
            None,
            &CreativeInventoryHudState {
                selected_tab: "minecraft:search".to_string(),
                search: crate::input::text_field::TextField::with_text("stone", 50),
                search_focused: true,
                scroll_row: 0,
                hovered: Some(CreativeInventoryHover::Search),
                cursor_position: Some((90, 8)),
                saved_hotbars: Vec::new(),
                hotbar_key_bindings: KeyBindings::default(),
                quick_craft_button_num: None,
                quick_craft_slots: Vec::new(),
            },
        )
        .unwrap();
        assert_eq!(
            search_frame.all_text_for_test(),
            "Search. Search Items edit box: stone"
        );

        let tab_frame = creative_inventory_narration_frame_for_snapshot(
            &snapshot,
            None,
            &CreativeInventoryHudState {
                selected_tab: "minecraft:building_blocks".to_string(),
                search: crate::input::text_field::TextField::new(50),
                search_focused: false,
                scroll_row: 0,
                hovered: Some(CreativeInventoryHover::Tab(2)),
                cursor_position: Some((170, 140)),
                saved_hotbars: Vec::new(),
                hotbar_key_bindings: KeyBindings::default(),
                quick_craft_button_num: None,
                quick_craft_slots: Vec::new(),
            },
        )
        .unwrap();
        assert_eq!(
            tab_frame.all_text_for_test(),
            "Building Blocks. Inventory. Left click to activate"
        );

        let trash_frame = creative_inventory_narration_frame_for_snapshot(
            &snapshot,
            None,
            &CreativeInventoryHudState {
                selected_tab: "minecraft:inventory".to_string(),
                search: crate::input::text_field::TextField::new(50),
                search_focused: false,
                scroll_row: 0,
                hovered: Some(CreativeInventoryHover::Slot(CreativeInventorySlot::Trash)),
                cursor_position: Some((175, 114)),
                saved_hotbars: Vec::new(),
                hotbar_key_bindings: KeyBindings::default(),
                quick_craft_button_num: None,
                quick_craft_slots: Vec::new(),
            },
        )
        .unwrap();
        assert_eq!(trash_frame.all_text_for_test(), "Inventory. Destroy Item");
    }

    #[test]
    fn empty_hotbar_presets_project_nine_locked_paper_placeholders() {
        let items = creative_hotbar_items(&[], &KeyBindings::default(), None);
        assert_eq!(items.len(), 81);
        let nonempty = items
            .iter()
            .enumerate()
            .filter(|(_, stack)| !super::super::item_stack_is_empty(stack))
            .collect::<Vec<_>>();
        assert_eq!(
            nonempty.iter().map(|(index, _)| *index).collect::<Vec<_>>(),
            vec![0, 10, 20, 30, 40, 50, 60, 70, 80]
        );
        assert!(nonempty.iter().all(|(_, stack)| {
            stack.item_id == Some(VANILLA_ITEM_PAPER_ID) && stack.component_patch.creative_slot_lock
        }));
        assert_eq!(
            nonempty[0].1.component_patch.item_name.as_deref(),
            Some("Save hotbar with C + 1")
        );
    }
}
