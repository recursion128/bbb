use bbb_client_clock::Instant;
use std::time::Duration;

use crate::control::NetCounters;
use bbb_client_net::NetCommand;
use bbb_client_world::{
    ContainerClickBuildError, ContainerClickSlotRequest, MountEquipmentSlotVisibility,
    MountInventoryKind, WorldStore,
};
#[cfg(test)]
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID;
use bbb_protocol::menu_types::{
    VANILLA_MENU_TYPE_ANVIL_ID, VANILLA_MENU_TYPE_BEACON_ID, VANILLA_MENU_TYPE_BLAST_FURNACE_ID,
    VANILLA_MENU_TYPE_BREWING_STAND_ID, VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
    VANILLA_MENU_TYPE_CRAFTER_3X3_ID, VANILLA_MENU_TYPE_CRAFTING_ID,
    VANILLA_MENU_TYPE_ENCHANTMENT_ID, VANILLA_MENU_TYPE_FURNACE_ID,
    VANILLA_MENU_TYPE_GENERIC_3X3_ID, VANILLA_MENU_TYPE_GENERIC_9X1_ID,
    VANILLA_MENU_TYPE_GENERIC_9X6_ID, VANILLA_MENU_TYPE_GRINDSTONE_ID, VANILLA_MENU_TYPE_HOPPER_ID,
    VANILLA_MENU_TYPE_LECTERN_ID, VANILLA_MENU_TYPE_LOOM_ID, VANILLA_MENU_TYPE_MERCHANT_ID,
    VANILLA_MENU_TYPE_SHULKER_BOX_ID, VANILLA_MENU_TYPE_SMITHING_ID, VANILLA_MENU_TYPE_SMOKER_ID,
    VANILLA_MENU_TYPE_STONECUTTER_ID,
};
use bbb_protocol::packets::{
    ContainerInput, ContainerSlotStateChanged, ItemStackSummary, PlaceRecipeCommand,
    RecipeBookChangeSettingsCommand, RecipeBookType, RenameItem, SelectTradeCommand, SetBeacon,
};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::KeyCode,
};

use super::{
    bundle::{
        handle_bundle_slot_click, handle_bundle_slot_hover_end, handle_bundle_slot_mouse_scroll,
    },
    commands::{
        hotbar_slot_for_key, queue_container_button_click_command, queue_container_click_command,
        queue_container_close_command, queue_container_slot_state_click_commands_atomically,
        queue_place_recipe_command, queue_recipe_book_change_settings_command,
        queue_recipe_book_seen_recipe_command, queue_rename_item_command,
        queue_select_trade_command, queue_set_beacon_command,
    },
    text_field, AnvilRenameInputSignature, ClientInputState, DebugClipboard,
    RecipeBookOverlayHudState,
};
use crate::item_models::tooltip::{tooltip_lines_for_stack_with_context, NativeItemTooltipOptions};
use crate::recipe_book_ui::{
    clamped_recipe_book_page, crafting_recipe_book_collections,
    crafting_recipe_book_visible_tab_indices, furnace_recipe_book_collections,
    furnace_recipe_book_visible_tab_indices, recipe_book_overlay_entry_position,
    recipe_book_overlay_origin, recipe_book_page_count, recipe_book_slot_select_index,
    RecipeBookCraftingGrid, RecipeBookFurnaceFamily, RecipeBookUiCollection,
    RECIPE_BOOK_ITEMS_PER_PAGE, RECIPE_BOOK_OVERLAY_BUTTON_DRAW_SIZE,
};
use bbb_client_render::item_model::NativeItemRuntime;

pub(super) mod creative;
mod layout;
mod menu_screens_26_1;
mod menu_widgets;
mod recipe_book_lifecycle;
mod screen;
use menu_widgets::*;
use screen::{
    sync_beacon_effect_selection, sync_loom_pattern_scroll, sync_loom_pattern_state,
    sync_stonecutter_recipe_scroll,
};
pub(crate) use screen::{
    sync_beacon_effect_selection_state, sync_loom_pattern_state_for_hud,
    sync_stonecutter_recipe_scroll_state, ContainerInputState,
};

#[cfg(test)]
pub(crate) use layout::local_inventory_slot_layouts;
pub(crate) use layout::{
    beacon_effect_buttons, inventory_screen_layout, inventory_screen_layout_for_surface,
    inventory_screen_selected_hotbar_slot_id, loom_pattern_row_count, recipe_book_button_position,
    recipe_book_main_gui_offset, recipe_book_recipe_button_position,
    recipe_book_tab_count_for_background, recipe_book_type_for_background,
    recipe_book_type_settings, InventoryScreenBackground, InventoryScreenLayout,
    InventorySlotLayout, BEACON_EFFECT_BUTTON_SIZE, LOOM_PATTERN_BUTTON_COLUMNS,
    LOOM_PATTERN_BUTTON_ROWS, LOOM_PATTERN_BUTTON_SIZE, LOOM_PATTERN_BUTTON_X,
    LOOM_PATTERN_BUTTON_Y, LOOM_SCROLLER_CLICK_Y, LOOM_SCROLLER_DRAG_Y, LOOM_SCROLLER_FULL_HEIGHT,
    LOOM_SCROLLER_HEIGHT, LOOM_SCROLLER_MAX_OFFSET, LOOM_SCROLLER_WIDTH, LOOM_SCROLLER_X,
    LOOM_SCROLLER_Y, RECIPE_BOOK_BUTTON_HEIGHT, RECIPE_BOOK_BUTTON_WIDTH,
    RECIPE_BOOK_FILTER_BUTTON_HEIGHT, RECIPE_BOOK_FILTER_BUTTON_WIDTH, RECIPE_BOOK_FILTER_BUTTON_X,
    RECIPE_BOOK_FILTER_BUTTON_Y, RECIPE_BOOK_PAGE_BACKWARD_BUTTON_X,
    RECIPE_BOOK_PAGE_BUTTON_HEIGHT, RECIPE_BOOK_PAGE_BUTTON_WIDTH, RECIPE_BOOK_PAGE_BUTTON_Y,
    RECIPE_BOOK_PAGE_FORWARD_BUTTON_X, RECIPE_BOOK_RECIPE_BUTTON_COLUMNS,
    RECIPE_BOOK_RECIPE_BUTTON_SIZE, RECIPE_BOOK_RECIPE_BUTTON_X, RECIPE_BOOK_RECIPE_BUTTON_Y,
    RECIPE_BOOK_SEARCH_BOX_HEIGHT, RECIPE_BOOK_SEARCH_BOX_WIDTH, RECIPE_BOOK_SEARCH_BOX_X,
    RECIPE_BOOK_SEARCH_BOX_Y, RECIPE_BOOK_SEARCH_TEXT_X_OFFSET, RECIPE_BOOK_SEARCH_TEXT_Y_OFFSET,
    RECIPE_BOOK_SELECTED_TAB_X_OFFSET, RECIPE_BOOK_TAB_HEIGHT, RECIPE_BOOK_TAB_STRIDE_Y,
    RECIPE_BOOK_TAB_WIDTH, RECIPE_BOOK_TAB_X, RECIPE_BOOK_TAB_Y, STONECUTTER_RECIPE_BUTTON_COLUMNS,
    STONECUTTER_RECIPE_BUTTON_HEIGHT, STONECUTTER_RECIPE_BUTTON_ROWS,
    STONECUTTER_RECIPE_BUTTON_SPRITE_Y, STONECUTTER_RECIPE_BUTTON_WIDTH,
    STONECUTTER_RECIPE_BUTTON_X, STONECUTTER_RECIPE_BUTTON_Y, STONECUTTER_RECIPE_ITEM_Y,
    STONECUTTER_SCROLLER_CLICK_Y, STONECUTTER_SCROLLER_DRAG_Y, STONECUTTER_SCROLLER_FULL_HEIGHT,
    STONECUTTER_SCROLLER_HEIGHT, STONECUTTER_SCROLLER_MAX_OFFSET, STONECUTTER_SCROLLER_WIDTH,
    STONECUTTER_SCROLLER_X, STONECUTTER_SCROLLER_Y, STONECUTTER_VISIBLE_RECIPE_BUTTON_COUNT,
};
pub(crate) use recipe_book_lifecycle::{
    sync_recipe_book_presentation, RecipeBookPresentationHudState, RecipeBookPresentationState,
};

pub(crate) fn recipe_book_visible_tab_indices(
    world: &WorldStore,
    background: InventoryScreenBackground,
) -> Vec<usize> {
    let Some(tab_count) = recipe_book_tab_count_for_background(background) else {
        return Vec::new();
    };
    if let Some(grid) = recipe_book_crafting_grid_for_background(background) {
        return crafting_recipe_book_visible_tab_indices(world, grid, tab_count);
    }
    if let Some(family) = recipe_book_furnace_family_for_background(background) {
        return furnace_recipe_book_visible_tab_indices(world, family, tab_count);
    }
    (0..tab_count).collect()
}

const INVENTORY_SCREEN_WIDTH: i32 = 176;
const INVENTORY_SCREEN_HEIGHT: i32 = 166;
const GENERIC_CONTAINER_WIDTH: i32 = 176;
const GENERIC_CONTAINER_BASE_HEIGHT: i32 = 114;
const GENERIC_CONTAINER_ROW_HEIGHT: i32 = 18;
pub(crate) const STONECUTTER_SELECTED_RECIPE_DATA_ID: i16 = 0;
const GENERIC_CONTAINER_SLOT_COLUMNS: i32 = 9;
const GENERIC_CONTAINER_SLOT_COUNT_PER_ROW: i16 = 9;
const GENERIC_3X3_SCREEN_WIDTH: i32 = 176;
const GENERIC_3X3_SCREEN_HEIGHT: i32 = 166;
const GENERIC_3X3_SLOT_COLUMNS: i32 = 3;
const GENERIC_3X3_SLOT_COUNT: i16 = 9;
const CRAFTER_SCREEN_WIDTH: i32 = 176;
const CRAFTER_SCREEN_HEIGHT: i32 = 166;
const CRAFTER_GRID_SLOT_COLUMNS: i32 = 3;
const CRAFTER_GRID_SLOT_COUNT: i16 = 9;
const CRAFTER_RESULT_SLOT: i16 = 45;
const CRAFTER_TOTAL_SLOT_COUNT: i16 = 46;
const ANVIL_SCREEN_WIDTH: i32 = 176;
const ANVIL_SCREEN_HEIGHT: i32 = 166;
const ANVIL_SLOT_COUNT: i16 = 3;
pub(super) const ANVIL_RENAME_MAX_LENGTH: usize = 50;
pub(super) const RECIPE_BOOK_SEARCH_MAX_LENGTH: usize = 50;
const BEACON_SCREEN_WIDTH: i32 = 230;
const BEACON_SCREEN_HEIGHT: i32 = 219;
const BEACON_SLOT_COUNT: i16 = 1;
const BEACON_LEVELS_DATA_ID: i16 = 0;
const BEACON_PRIMARY_EFFECT_DATA_ID: i16 = 1;
const BEACON_SECONDARY_EFFECT_DATA_ID: i16 = 2;
const BEACON_CONFIRM_BUTTON_X: i32 = 164;
const BEACON_CANCEL_BUTTON_X: i32 = 190;
const BEACON_ACTION_BUTTON_Y: i32 = 107;
const BEACON_ACTION_BUTTON_SIZE: i32 = 22;
const CARTOGRAPHY_TABLE_SCREEN_WIDTH: i32 = 176;
const CARTOGRAPHY_TABLE_SCREEN_HEIGHT: i32 = 166;
const CARTOGRAPHY_TABLE_SLOT_COUNT: i16 = 3;
const BREWING_STAND_SCREEN_WIDTH: i32 = 176;
const BREWING_STAND_SCREEN_HEIGHT: i32 = 166;
const BREWING_STAND_SLOT_COUNT: i16 = 5;
const CRAFTING_SCREEN_WIDTH: i32 = 176;
const CRAFTING_SCREEN_HEIGHT: i32 = 166;
const CRAFTING_GRID_SLOT_COLUMNS: i32 = 3;
const CRAFTING_SLOT_COUNT: i16 = 10;
const ENCHANTMENT_SCREEN_WIDTH: i32 = 176;
const ENCHANTMENT_SCREEN_HEIGHT: i32 = 166;
const ENCHANTMENT_SLOT_COUNT: i16 = 2;
const ENCHANTMENT_BUTTON_X: i32 = 60;
const ENCHANTMENT_BUTTON_Y: i32 = 14;
const ENCHANTMENT_BUTTON_WIDTH: i32 = 108;
const ENCHANTMENT_BUTTON_HEIGHT: i32 = 19;
const ENCHANTMENT_BUTTON_SPACING: i32 = 19;
const ENCHANTMENT_BUTTON_COUNT: i32 = 3;
const FURNACE_SCREEN_WIDTH: i32 = 176;
const FURNACE_SCREEN_HEIGHT: i32 = 166;
const FURNACE_SLOT_COUNT: i16 = 3;
const GRINDSTONE_SCREEN_WIDTH: i32 = 176;
const GRINDSTONE_SCREEN_HEIGHT: i32 = 166;
const GRINDSTONE_SLOT_COUNT: i16 = 3;
const HOPPER_SCREEN_WIDTH: i32 = 176;
const HOPPER_SCREEN_HEIGHT: i32 = 133;
const HOPPER_SLOT_COUNT: i16 = 5;
const MOUNT_SCREEN_WIDTH: i32 = 176;
const MOUNT_SCREEN_HEIGHT: i32 = 166;
const MOUNT_EQUIPMENT_SLOT_COUNT: i16 = 2;
const MOUNT_INVENTORY_ROWS: i32 = 3;
const MOUNT_MAX_INVENTORY_COLUMNS: i32 = 5;
const LECTERN_SCREEN_WIDTH: i32 = 192;
const LECTERN_SCREEN_HEIGHT: i32 = 192;
const LECTERN_BUTTON_PREV_PAGE: i32 = 1;
const LECTERN_BUTTON_NEXT_PAGE: i32 = 2;
const LECTERN_BUTTON_TAKE_BOOK: i32 = 3;
const LECTERN_PAGE_BUTTON_Y: i32 = 157;
const LECTERN_PAGE_BACK_BUTTON_X: i32 = 43;
const LECTERN_PAGE_FORWARD_BUTTON_X: i32 = 116;
const LECTERN_PAGE_BUTTON_WIDTH: i32 = 23;
const LECTERN_PAGE_BUTTON_HEIGHT: i32 = 13;
const LECTERN_MENU_BUTTON_Y: i32 = 194;
const LECTERN_MENU_DONE_BUTTON_X: i32 = -4;
const LECTERN_MENU_TAKE_BOOK_BUTTON_X: i32 = 98;
const LECTERN_MENU_BUTTON_WIDTH: i32 = 98;
const LECTERN_MENU_BUTTON_HEIGHT: i32 = 20;
const LOOM_SCREEN_WIDTH: i32 = 176;
const LOOM_SCREEN_HEIGHT: i32 = 166;
const LOOM_SLOT_COUNT: i16 = 4;
pub(crate) const LOOM_SELECTED_PATTERN_DATA_ID: i16 = 0;
const MERCHANT_SCREEN_WIDTH: i32 = 276;
const MERCHANT_SCREEN_HEIGHT: i32 = 166;
const MERCHANT_SLOT_COUNT: i16 = 3;
const MERCHANT_TRADE_BUTTON_X: i32 = 5;
const MERCHANT_TRADE_BUTTON_Y: i32 = 18;
const MERCHANT_TRADE_BUTTON_WIDTH: i32 = 88;
const MERCHANT_TRADE_BUTTON_HEIGHT: i32 = 20;
const MERCHANT_TRADE_BUTTON_COUNT: i32 = 7;
const MERCHANT_SCROLLER_X: i32 = 94;
const MERCHANT_SCROLLER_Y: i32 = 18;
const MERCHANT_SCROLLER_WIDTH: i32 = 6;
const MERCHANT_SCROLLER_HEIGHT: i32 = 27;
const MERCHANT_SCROLLER_FULL_HEIGHT: i32 = 139;
const SHULKER_BOX_SCREEN_WIDTH: i32 = 176;
const SHULKER_BOX_SCREEN_HEIGHT: i32 = 167;
const SHULKER_BOX_SLOT_COUNT: i16 = 27;
const SMITHING_SCREEN_WIDTH: i32 = 176;
const SMITHING_SCREEN_HEIGHT: i32 = 166;
const SMITHING_SLOT_COUNT: i16 = 4;
const STONECUTTER_SCREEN_WIDTH: i32 = 176;
const STONECUTTER_SCREEN_HEIGHT: i32 = 166;
const STONECUTTER_SLOT_COUNT: i16 = 2;
const SLOT_SIZE: f64 = 16.0;
const SLOT_HOVER_MARGIN: f64 = 1.0;
const VANILLA_DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InventoryClickTarget {
    Slot(i16),
    EmptyPanel,
    Outside,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LecternClickTarget {
    Done,
    MenuButton(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecipeBookPageTurn {
    Previous,
    Next,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BeaconClickTarget {
    Confirm,
    Cancel,
    Effect { primary: bool, effect_id: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoomClickTarget {
    Pattern(i32),
}

pub(crate) fn handle_inventory_cursor_moved(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    if input.screen.creative().is_some() {
        if !input.focused {
            return false;
        }
        let Some(snapshot) = world.creative_inventory_snapshot() else {
            return true;
        };
        let visible_item_count = input.screen.creative_mut().unwrap().visible_item_count();
        let creative = input.screen.creative_mut().unwrap();
        creative.set_cursor_position(cursor_position, surface_size, snapshot, visible_item_count);
        creative.update_quick_craft_drag(world, snapshot);
        input.inventory_cursor_position = creative.cursor_position();
        return true;
    }
    if !input.focused
        || input.screen.container().is_none()
        || inventory_screen_layout_for_surface(world, surface_size).is_none()
    {
        return false;
    }
    input.inventory_cursor_position =
        inventory_screen_cursor_position(world, cursor_position, surface_size);

    if input.container_mut().merchant_trade_scrolling
        && update_merchant_trade_scroll_from_cursor(input, world, cursor_position, surface_size)
    {
        return true;
    }
    if input.container_mut().stonecutter_recipe_scrolling
        && update_stonecutter_recipe_scroll_from_cursor(input, world, cursor_position, surface_size)
    {
        return true;
    }
    if input.container_mut().loom_pattern_scrolling
        && update_loom_pattern_scroll_from_cursor(input, world, cursor_position, surface_size)
    {
        return true;
    }
    let hovered = if narrow_recipe_book_consumes_container_click(world, surface_size) {
        None
    } else {
        inventory_screen_hovered_slot(world, cursor_position, surface_size)
    };
    if input.container_mut().inventory_hovered_slot != hovered {
        if let Some(previous) = input.container_mut().inventory_hovered_slot {
            handle_bundle_slot_hover_end(world, counters, net_commands, i32::from(previous));
        }
        input.container_mut().inventory_hovered_slot = hovered;
    }
    if let Some(slot) = hovered {
        local_inventory_add_quick_craft_slot(input, world, slot);
    }
    true
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_inventory_mouse_input(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    button: MouseButton,
    state: ElementState,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    handle_inventory_mouse_input_with_item_runtime(
        input,
        world,
        counters,
        net_commands,
        None,
        button,
        state,
        cursor_position,
        surface_size,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_inventory_mouse_input_with_item_runtime(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    button: MouseButton,
    state: ElementState,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    if input.screen.creative().is_some() {
        if !input.focused {
            return false;
        }
        let Some(snapshot) = world.creative_inventory_snapshot_shared() else {
            return true;
        };
        let visible_item_count = input.screen.creative_mut().unwrap().visible_item_count();
        input.screen.creative_mut().unwrap().set_cursor_position(
            cursor_position,
            surface_size,
            &snapshot,
            visible_item_count,
        );
        input.inventory_cursor_position = input.screen.creative_mut().unwrap().cursor_position();
        let button_num = match button {
            MouseButton::Left => 0,
            MouseButton::Right => 1,
            MouseButton::Middle => 2,
            _ => return true,
        };
        match (button, state) {
            (MouseButton::Left, ElementState::Pressed) => {
                input
                    .screen
                    .creative_mut()
                    .unwrap()
                    .handle_primary_press(&snapshot, visible_item_count);
            }
            (MouseButton::Left, ElementState::Released) => {
                input.screen.creative_mut().unwrap().handle_action_release(
                    world,
                    counters,
                    net_commands,
                    item_runtime,
                    &snapshot,
                    button_num,
                );
                input
                    .screen
                    .creative_mut()
                    .unwrap()
                    .handle_primary_release(&snapshot);
            }
            (_, ElementState::Released) => {
                input.screen.creative_mut().unwrap().handle_action_release(
                    world,
                    counters,
                    net_commands,
                    item_runtime,
                    &snapshot,
                    button_num,
                );
            }
            _ => {}
        }
        if state == ElementState::Pressed {
            let shift_down = input.shift_down();
            input.screen.creative_mut().unwrap().handle_action_press(
                world,
                counters,
                net_commands,
                item_runtime,
                &snapshot,
                button_num,
                shift_down,
            );
        }
        input.refresh_creative_visible_items(world, item_runtime);
        return true;
    }
    if !input.focused || input.screen.container().is_none() {
        return false;
    }
    let Some(layout) = inventory_screen_layout_for_surface(world, surface_size) else {
        return false;
    };
    input.inventory_cursor_position =
        inventory_screen_cursor_position(world, cursor_position, surface_size);
    let button_num = match button {
        MouseButton::Left => 0,
        MouseButton::Right => 1,
        MouseButton::Middle => 2,
        _ => return true,
    };

    if layout.background == InventoryScreenBackground::Lectern {
        if matches!(state, ElementState::Pressed) && button_num == 0 {
            maybe_queue_lectern_button_click(
                world,
                counters,
                net_commands,
                cursor_position,
                surface_size,
            );
        }
        input.container_mut().inventory_last_click_slot = None;
        input.container_mut().inventory_last_click_button_num = None;
        input.container_mut().inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }

    if matches!(state, ElementState::Released) {
        if narrow_recipe_book_consumes_container_click(world, surface_size) {
            local_inventory_clear_quick_craft(input);
            return true;
        }
        return handle_inventory_mouse_released(
            input,
            world,
            counters,
            net_commands,
            button_num,
            cursor_position,
            surface_size,
        );
    }
    if button_num == 2 {
        let instabuild = world
            .local_player()
            .abilities
            .is_some_and(|abilities| abilities.instabuild);
        if !instabuild {
            return true;
        }
        if inventory_cursor_is_empty(world) {
            if maybe_queue_inventory_clone_click(
                input,
                world,
                counters,
                net_commands,
                cursor_position,
                surface_size,
            ) {
                input.container_mut().inventory_skip_next_release = true;
            }
            return true;
        }
        let click_target = inventory_screen_click_target(world, cursor_position, surface_size);
        reset_container_click_state(input, false);
        if matches!(click_target, Some(InventoryClickTarget::Slot(_))) {
            // Vanilla `AbstractContainerScreen.mouseClicked` (`AbstractContainerScreen.java:369-378`):
            // middle-clicking while carrying an item in Creative starts `QUICKCRAFT_TYPE_CLONE`.
            input.container_mut().inventory_quick_craft_button_num = Some(2);
        }
        return true;
    }
    if maybe_handle_recipe_book_overlay_click(
        input,
        world,
        counters,
        net_commands,
        item_runtime,
        button_num,
        cursor_position,
        surface_size,
    ) {
        input.container_mut().inventory_last_click_slot = None;
        input.container_mut().inventory_last_click_button_num = None;
        input.container_mut().inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if button_num == 0
        && maybe_focus_recipe_book_search(input, world, cursor_position, surface_size)
    {
        input.container_mut().inventory_last_click_slot = None;
        input.container_mut().inventory_last_click_button_num = None;
        input.container_mut().inventory_last_click_at = None;
        input.container_mut().recipe_book_overlay = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if button_num == 0 && maybe_select_recipe_book_tab(input, world, cursor_position, surface_size)
    {
        input.container_mut().inventory_last_click_slot = None;
        input.container_mut().inventory_last_click_button_num = None;
        input.container_mut().inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if button_num == 0
        && maybe_turn_recipe_book_page(input, world, item_runtime, cursor_position, surface_size)
    {
        input.container_mut().inventory_last_click_slot = None;
        input.container_mut().inventory_last_click_button_num = None;
        input.container_mut().inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if button_num == 0
        && maybe_queue_recipe_book_recipe_click(
            input,
            world,
            counters,
            net_commands,
            item_runtime,
            cursor_position,
            surface_size,
        )
    {
        input.container_mut().inventory_last_click_slot = None;
        input.container_mut().inventory_last_click_button_num = None;
        input.container_mut().inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if button_num == 1
        && maybe_open_recipe_book_overlay(input, world, item_runtime, cursor_position, surface_size)
    {
        input.container_mut().inventory_last_click_slot = None;
        input.container_mut().inventory_last_click_button_num = None;
        input.container_mut().inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if button_num == 0
        && maybe_queue_recipe_book_filter_click(
            world,
            counters,
            net_commands,
            cursor_position,
            surface_size,
        )
    {
        input.container_mut().inventory_last_click_slot = None;
        input.container_mut().inventory_last_click_button_num = None;
        input.container_mut().inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if narrow_recipe_book_consumes_container_click(world, surface_size) {
        input.container_mut().inventory_last_click_slot = None;
        input.container_mut().inventory_last_click_button_num = None;
        input.container_mut().inventory_last_click_at = None;
        input.container_mut().recipe_book_overlay = None;
        input.container_mut().recipe_book_search_focused = false;
        input
            .container_mut()
            .recipe_book_search_suppress_open_key_commit = false;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if button_num == 0
        && maybe_queue_recipe_book_toggle_click(
            input,
            world,
            counters,
            net_commands,
            cursor_position,
            surface_size,
        )
    {
        input.container_mut().inventory_last_click_slot = None;
        input.container_mut().inventory_last_click_button_num = None;
        input.container_mut().inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if button_num == 0
        && maybe_queue_beacon_button_click(
            input,
            world,
            counters,
            net_commands,
            cursor_position,
            surface_size,
        )
    {
        input.container_mut().inventory_last_click_slot = None;
        input.container_mut().inventory_last_click_button_num = None;
        input.container_mut().inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if button_num == 0
        && maybe_queue_merchant_trade_click(
            input,
            world,
            counters,
            net_commands,
            cursor_position,
            surface_size,
        )
    {
        input.container_mut().inventory_last_click_slot = None;
        input.container_mut().inventory_last_click_button_num = None;
        input.container_mut().inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if button_num == 0
        && maybe_queue_enchantment_button_click(
            world,
            counters,
            net_commands,
            cursor_position,
            surface_size,
        )
    {
        input.container_mut().inventory_last_click_slot = None;
        input.container_mut().inventory_last_click_button_num = None;
        input.container_mut().inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    // Vanilla 26.1 `LoomScreen.mouseClicked` (`LoomScreen.java:186-203`) and
    // `StonecutterScreen.mouseClicked` (`StonecutterScreen.java:134-149`)
    // accept any mouse button on pattern/recipe buttons.
    if maybe_queue_loom_pattern_click(
        input,
        world,
        counters,
        net_commands,
        cursor_position,
        surface_size,
    ) {
        input.container_mut().inventory_last_click_slot = None;
        input.container_mut().inventory_last_click_button_num = None;
        input.container_mut().inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if maybe_queue_stonecutter_recipe_click(
        input,
        world,
        counters,
        net_commands,
        cursor_position,
        surface_size,
    ) {
        input.container_mut().inventory_last_click_slot = None;
        input.container_mut().inventory_last_click_button_num = None;
        input.container_mut().inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if button_num == 0
        && maybe_start_loom_pattern_scroll_drag(input, world, cursor_position, surface_size)
    {
        input.container_mut().inventory_last_click_slot = None;
        input.container_mut().inventory_last_click_button_num = None;
        input.container_mut().inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if button_num == 0
        && maybe_start_stonecutter_recipe_scroll_drag(input, world, cursor_position, surface_size)
    {
        input.container_mut().inventory_last_click_slot = None;
        input.container_mut().inventory_last_click_button_num = None;
        input.container_mut().inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if button_num == 0
        && maybe_start_merchant_trade_scroll_drag(input, world, cursor_position, surface_size)
    {
        input.container_mut().inventory_last_click_slot = None;
        input.container_mut().inventory_last_click_button_num = None;
        input.container_mut().inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }

    let click_target = inventory_screen_click_target(world, cursor_position, surface_size);
    let now = bbb_client_clock::now();
    let double_click = match click_target {
        Some(InventoryClickTarget::Slot(slot)) => {
            let is_double_click = local_inventory_is_double_click(input, slot, button_num, now);
            input.container_mut().inventory_last_click_slot = Some(slot);
            input.container_mut().inventory_last_click_button_num = Some(button_num);
            input.container_mut().inventory_last_click_at = Some(now);
            is_double_click
        }
        _ => {
            input.container_mut().inventory_last_click_slot = None;
            input.container_mut().inventory_last_click_button_num = None;
            input.container_mut().inventory_last_click_at = None;
            false
        }
    };
    input.container_mut().inventory_double_click = double_click;
    input.container_mut().inventory_skip_next_release = false;
    local_inventory_clear_quick_craft(input);
    let (slot_num, click_input) = match click_target {
        Some(InventoryClickTarget::Slot(slot)) => {
            if !inventory_cursor_is_empty(world) {
                input.container_mut().inventory_quick_craft_button_num = Some(button_num);
                return true;
            } else if input.shift_down() {
                input.container_mut().inventory_last_quick_moved =
                    local_inventory_slot_item(world, slot)
                        .filter(|item| !item_stack_is_empty(item))
                        .cloned();
                (slot, ContainerInput::QuickMove)
            } else {
                (slot, ContainerInput::Pickup)
            }
        }
        Some(InventoryClickTarget::Outside) => {
            if inventory_cursor_is_empty(world) {
                return true;
            }
            (-999, ContainerInput::Pickup)
        }
        Some(InventoryClickTarget::EmptyPanel) | None => return true,
    };
    let request = ContainerClickSlotRequest {
        slot_num,
        button_num,
        input: click_input,
    };
    input.container_mut().inventory_skip_next_release = true;
    clear_recipe_book_last_placed_if_crafting_slot(input, world, slot_num);
    if !maybe_apply_and_queue_crafter_slot_state_changed(world, counters, net_commands, request) {
        local_inventory_apply_and_queue_click(world, counters, net_commands, request);
    }
    true
}

fn narrow_recipe_book_consumes_container_click(
    world: &WorldStore,
    surface_size: PhysicalSize<u32>,
) -> bool {
    inventory_screen_layout_for_surface(world, surface_size)
        .and_then(|layout| layout.recipe_book)
        .is_some_and(|recipe_book| recipe_book.narrow)
}

fn maybe_queue_recipe_book_filter_click(
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(book_type) =
        recipe_book_filter_button_at_position(world, cursor_position, surface_size)
    else {
        return false;
    };
    let mut settings = recipe_book_type_settings(world, book_type);
    settings.filtering = !settings.filtering;
    world.set_local_recipe_book_type_settings(book_type, settings);
    queue_recipe_book_change_settings_command(
        counters,
        net_commands,
        RecipeBookChangeSettingsCommand {
            book_type,
            open: settings.open,
            filtering: settings.filtering,
        },
    );
    true
}

fn maybe_focus_recipe_book_search(
    input: &mut ClientInputState,
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    if !recipe_book_search_box_is_open(world) {
        input.container_mut().recipe_book_search_focused = false;
        input
            .container_mut()
            .recipe_book_search_suppress_open_key_commit = false;
        return false;
    }
    if !recipe_book_search_box_at_position(world, cursor_position, surface_size) {
        input.container_mut().recipe_book_search_focused = false;
        input
            .container_mut()
            .recipe_book_search_suppress_open_key_commit = false;
        return false;
    }

    input.container_mut().recipe_book_search_focused = true;
    input.container_mut().recipe_book_search.focus();
    input.container_mut().recipe_book_overlay = None;
    let end = input.container_mut().recipe_book_search.value().len();
    input
        .container_mut()
        .recipe_book_search
        .move_cursor(end, false, None);
    input
        .container_mut()
        .recipe_book_search_suppress_open_key_commit = false;
    true
}

fn maybe_select_recipe_book_tab(
    input: &mut ClientInputState,
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some((book_type, index)) =
        recipe_book_tab_at_position(world, cursor_position, surface_size)
    else {
        return false;
    };
    set_recipe_book_selected_tab_index(input, book_type, index);
    input.container_mut().recipe_book_overlay = None;
    input.container_mut().recipe_book_search_focused = false;
    input
        .container_mut()
        .recipe_book_search_suppress_open_key_commit = false;
    true
}

fn maybe_turn_recipe_book_page(
    input: &mut ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some((book_type, turn, page_count)) = recipe_book_page_button_at_position(
        input,
        world,
        item_runtime,
        cursor_position,
        surface_size,
    ) else {
        return false;
    };
    let current_page =
        selected_recipe_book_page_index(input, book_type).min(page_count.saturating_sub(1));
    let next_page = match turn {
        RecipeBookPageTurn::Previous => current_page.saturating_sub(1),
        RecipeBookPageTurn::Next => (current_page + 1).min(page_count.saturating_sub(1)),
    };
    set_recipe_book_page_index(input, book_type, next_page);
    input.container_mut().recipe_book_overlay = None;
    input.container_mut().recipe_book_search_focused = false;
    input
        .container_mut()
        .recipe_book_search_suppress_open_key_commit = false;
    true
}

#[allow(clippy::too_many_arguments)]
fn maybe_handle_recipe_book_overlay_click(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    button_num: i8,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    if input.container_mut().recipe_book_overlay.is_none() {
        return false;
    }
    if button_num == 0 {
        if let Some((recipe_index, craftable)) = recipe_book_overlay_recipe_at_position(
            input,
            world,
            item_runtime,
            cursor_position,
            surface_size,
        ) {
            maybe_queue_recipe_book_place_recipe(
                input,
                world,
                counters,
                net_commands,
                recipe_index,
                craftable,
                surface_size,
            );
        }
    }
    input.container_mut().recipe_book_overlay = None;
    input.container_mut().recipe_book_search_focused = false;
    input
        .container_mut()
        .recipe_book_search_suppress_open_key_commit = false;
    true
}

fn maybe_open_recipe_book_overlay(
    input: &mut ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some((book_type, page, visible_index, collection)) = recipe_book_collection_at_position(
        input,
        world,
        item_runtime,
        cursor_position,
        surface_size,
    ) else {
        return false;
    };
    let item_tag_entries = world.registry_tags("minecraft:item").map(|tags| &tags.tags);
    let entry_count = collection
        .overlay_entries(item_tag_entries, recipe_book_slot_select_index(world, 0.0))
        .len();
    if entry_count <= 1 {
        return false;
    }
    let (button_x, button_y) = recipe_book_recipe_button_position(visible_index);
    let (x, y) = recipe_book_overlay_origin(button_x, button_y, entry_count);
    input.container_mut().recipe_book_overlay = Some(RecipeBookOverlayHudState {
        book_type,
        tab_index: selected_recipe_book_tab_index(input, book_type),
        page_index: page,
        button_index: visible_index,
        x,
        y,
    });
    input.container_mut().recipe_book_search_focused = false;
    input
        .container_mut()
        .recipe_book_search_suppress_open_key_commit = false;
    true
}

fn maybe_queue_recipe_book_recipe_click(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some((recipe_index, craftable)) = recipe_book_recipe_button_at_position(
        input,
        world,
        item_runtime,
        cursor_position,
        surface_size,
    ) else {
        return false;
    };
    if !maybe_queue_recipe_book_place_recipe(
        input,
        world,
        counters,
        net_commands,
        recipe_index,
        craftable,
        surface_size,
    ) {
        return false;
    }
    input.container_mut().recipe_book_search_focused = false;
    input
        .container_mut()
        .recipe_book_search_suppress_open_key_commit = false;
    true
}

fn maybe_queue_recipe_book_place_recipe(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    recipe_index: i32,
    craftable: bool,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(container_id) = world.open_container_id() else {
        return false;
    };
    if !craftable
        && input.container_mut().recipe_book_last_placed_recipe
            == Some((container_id, recipe_index))
    {
        return false;
    }
    input.container_mut().recipe_book_last_placed_recipe = Some((container_id, recipe_index));
    world.clear_ghost_recipe();
    queue_place_recipe_command(
        counters,
        net_commands,
        PlaceRecipeCommand {
            container_id,
            recipe_index,
            use_max_items: input.shift_down(),
        },
    );
    close_narrow_recipe_book_after_place(input, world, counters, net_commands, surface_size);
    true
}

fn close_narrow_recipe_book_after_place(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    surface_size: PhysicalSize<u32>,
) {
    let Some(layout) = inventory_screen_layout_for_surface(world, surface_size) else {
        return;
    };
    let Some(recipe_book) = layout.recipe_book else {
        return;
    };
    if !recipe_book.narrow {
        return;
    }
    let Some(book_type) = recipe_book_type_for_background(layout.background) else {
        return;
    };
    close_recipe_book(input, world, counters, net_commands, book_type);
}

pub(crate) fn maybe_close_narrow_recipe_book(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(layout) = inventory_screen_layout_for_surface(world, surface_size) else {
        return false;
    };
    let Some(recipe_book) = layout.recipe_book else {
        return false;
    };
    if !recipe_book.narrow {
        return false;
    }
    let Some(book_type) = recipe_book_type_for_background(layout.background) else {
        return false;
    };
    close_recipe_book(input, world, counters, net_commands, book_type);
    true
}

fn close_recipe_book(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    book_type: RecipeBookType,
) {
    let mut settings = recipe_book_type_settings(world, book_type);
    if !settings.open {
        return;
    }
    settings.open = false;
    input.container_mut().recipe_book_overlay = None;
    input.container_mut().recipe_book_search_focused = false;
    input
        .container_mut()
        .recipe_book_search_suppress_open_key_commit = false;
    world.set_local_recipe_book_type_settings(book_type, settings);
    queue_recipe_book_change_settings_command(
        counters,
        net_commands,
        RecipeBookChangeSettingsCommand {
            book_type,
            open: false,
            filtering: settings.filtering,
        },
    );
}

fn maybe_queue_recipe_book_toggle_click(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(book_type) = recipe_book_button_at_position(world, cursor_position, surface_size)
    else {
        return false;
    };
    let mut settings = recipe_book_type_settings(world, book_type);
    settings.open = !settings.open;
    if !settings.open {
        input.container_mut().recipe_book_search_focused = false;
        input
            .container_mut()
            .recipe_book_search_suppress_open_key_commit = false;
    }
    input.container_mut().recipe_book_overlay = None;
    world.set_local_recipe_book_type_settings(book_type, settings);
    queue_recipe_book_change_settings_command(
        counters,
        net_commands,
        RecipeBookChangeSettingsCommand {
            book_type,
            open: settings.open,
            filtering: settings.filtering,
        },
    );
    true
}

fn maybe_queue_lectern_button_click(
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(target) = lectern_button_at_position(world, cursor_position, surface_size) else {
        return false;
    };
    match target {
        LecternClickTarget::Done => queue_container_close_command(counters, world, net_commands),
        LecternClickTarget::MenuButton(button_id) => {
            queue_lectern_button_click(world, counters, net_commands, button_id)
        }
    }
}

pub(crate) fn queue_lectern_button_click(
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    button_id: i32,
) -> bool {
    let Some(container_id) = world.apply_local_container_button_click(button_id) else {
        return false;
    };
    queue_container_button_click_command(counters, net_commands, container_id, button_id);
    true
}

fn maybe_queue_beacon_button_click(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    sync_beacon_effect_selection(input, world);
    let Some(target) = beacon_button_at_position(input, world, cursor_position, surface_size)
    else {
        return false;
    };
    match target {
        BeaconClickTarget::Cancel => queue_container_close_command(counters, world, net_commands),
        BeaconClickTarget::Confirm => {
            let Some(command) = beacon_set_command(input, world) else {
                return false;
            };
            let Some(primary_effect) = command.primary_effect else {
                return false;
            };
            if !world.apply_local_beacon_confirm_effects(primary_effect, command.secondary_effect) {
                return false;
            }
            queue_set_beacon_command(counters, net_commands, command);
            queue_container_close_command(counters, world, net_commands)
        }
        BeaconClickTarget::Effect { primary, effect_id } => {
            if primary {
                input.container_mut().beacon_primary_effect = Some(effect_id);
            } else {
                input.container_mut().beacon_secondary_effect = Some(effect_id);
            }
            input.container_mut().beacon_effect_selection_dirty = true;
            true
        }
    }
}

fn maybe_queue_merchant_trade_click(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(item) = merchant_trade_at_position(world, cursor_position, surface_size) else {
        return false;
    };
    input.container_mut().merchant_focused_trade_row = world
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.merchant_offers.as_ref())
        .map(|offers| item - offers.local_scroll_offset);
    world.set_local_merchant_selected_offer(item);
    queue_select_trade_command(counters, net_commands, SelectTradeCommand { item });
    true
}

fn maybe_queue_enchantment_button_click(
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(button_id) = enchantment_button_at_position(world, cursor_position, surface_size)
    else {
        return false;
    };
    let Some(container_id) = world.apply_local_container_button_click(button_id) else {
        return false;
    };
    queue_container_button_click_command(counters, net_commands, container_id, button_id);
    true
}

fn maybe_queue_loom_pattern_click(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    sync_loom_pattern_state(input, world);
    let start_index = input.container_mut().loom_pattern_scroll_row * LOOM_PATTERN_BUTTON_COLUMNS;
    let Some(LoomClickTarget::Pattern(button_id)) =
        loom_click_target_at_position(world, start_index, cursor_position, surface_size)
    else {
        return false;
    };
    let Some(container_id) = world.apply_local_container_button_click(button_id) else {
        return false;
    };
    input.container_mut().loom_selected_pattern_index = Some(button_id);
    input.container_mut().loom_pattern_selection_dirty = true;
    queue_container_button_click_command(counters, net_commands, container_id, button_id);
    true
}

fn maybe_queue_stonecutter_recipe_click(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    sync_stonecutter_recipe_scroll(input, world);
    let start_index =
        input.container_mut().stonecutter_recipe_scroll_row * STONECUTTER_RECIPE_BUTTON_COLUMNS;
    let Some(button_id) =
        stonecutter_recipe_button_at_position(world, start_index, cursor_position, surface_size)
    else {
        return false;
    };
    let Some(container_id) = world.apply_local_container_button_click(button_id) else {
        return false;
    };
    queue_container_button_click_command(counters, net_commands, container_id, button_id);
    true
}

fn handle_inventory_mouse_released(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    button_num: i8,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    input.container_mut().merchant_trade_scrolling = false;
    input.container_mut().stonecutter_recipe_scrolling = false;
    input.container_mut().loom_pattern_scrolling = false;
    let click_target = inventory_screen_click_target(world, cursor_position, surface_size);
    // Vanilla `AbstractContainerScreen.mouseReleased` (`AbstractContainerScreen.java:458-476`):
    // handle Shift+Double-Click and `PickupAll` on mouse release before `skipNextRelease` or quick-craft.
    if input.container().inventory_double_click && button_num == 0 {
        if let Some(InventoryClickTarget::Slot(slot)) = click_target {
            if world.active_container_slot_can_take_for_pick_all(slot) {
                if input.shift_down() {
                    if let Some(last_quick_moved) =
                        input.container().inventory_last_quick_moved.clone()
                    {
                        let slot_ids = world.active_container_slot_ids();
                        let max_stack_size = world
                            .item_profiles()
                            .item_stack_max_stack_size(&last_quick_moved);
                        for target_slot in slot_ids {
                            if !world.active_container_slot_may_pickup(target_slot)
                                || !world.active_container_slots_share_container(target_slot, slot)
                            {
                                continue;
                            }
                            let matches = local_inventory_slot_item(world, target_slot)
                                .is_some_and(|item| {
                                    !item_stack_is_empty(item)
                                        && item.item_id == last_quick_moved.item_id
                                        && item.component_patch == last_quick_moved.component_patch
                                        && item.count <= max_stack_size
                                });
                            if !matches {
                                continue;
                            }
                            let request = ContainerClickSlotRequest {
                                slot_num: target_slot,
                                button_num,
                                input: ContainerInput::QuickMove,
                            };
                            clear_recipe_book_last_placed_if_crafting_slot(
                                input,
                                world,
                                target_slot,
                            );
                            if !maybe_apply_and_queue_crafter_slot_state_changed(
                                world,
                                counters,
                                net_commands,
                                request,
                            ) {
                                local_inventory_apply_and_queue_click(
                                    world,
                                    counters,
                                    net_commands,
                                    request,
                                );
                            }
                        }
                    }
                } else {
                    let request = ContainerClickSlotRequest {
                        slot_num: slot,
                        button_num,
                        input: ContainerInput::PickupAll,
                    };
                    clear_recipe_book_last_placed_if_crafting_slot(input, world, slot);
                    local_inventory_apply_and_queue_click(world, counters, net_commands, request);
                }
                input.container_mut().inventory_double_click = false;
                input.container_mut().inventory_skip_next_release = false;
                local_inventory_clear_quick_craft(input);
                return true;
            }
        }
    }
    input.container_mut().inventory_double_click = false;

    if let Some(quick_craft_button_num) = input.container().inventory_quick_craft_button_num {
        if quick_craft_button_num != button_num {
            local_inventory_clear_quick_craft(input);
            input.container_mut().inventory_skip_next_release = true;
            return true;
        }
    }
    if input.container().inventory_skip_next_release {
        input.container_mut().inventory_skip_next_release = false;
        local_inventory_clear_quick_craft(input);
        return true;
    }

    let Some(quick_craft_button_num) = input.container_mut().inventory_quick_craft_button_num
    else {
        return true;
    };
    let quick_craft_slots = std::mem::take(&mut input.container_mut().inventory_quick_craft_slots);
    input.container_mut().inventory_quick_craft_button_num = None;
    if !quick_craft_slots.is_empty() {
        for &slot_num in &quick_craft_slots {
            clear_recipe_book_last_placed_if_crafting_slot(input, world, slot_num);
        }
        local_inventory_queue_quick_craft(
            world,
            counters,
            net_commands,
            quick_craft_button_num,
            quick_craft_slots,
        );
        return true;
    }

    if button_num == 2 {
        return true;
    }
    let Some((slot_num, click_input)) =
        local_inventory_release_fallback_click(world, click_target, input.shift_down())
    else {
        return true;
    };
    if click_input == ContainerInput::QuickMove {
        input.container_mut().inventory_last_quick_moved =
            local_inventory_slot_item(world, slot_num)
                .filter(|item| !item_stack_is_empty(item))
                .cloned();
    }
    let request = ContainerClickSlotRequest {
        slot_num,
        button_num,
        input: click_input,
    };
    clear_recipe_book_last_placed_if_crafting_slot(input, world, slot_num);
    if !maybe_apply_and_queue_crafter_slot_state_changed(world, counters, net_commands, request) {
        local_inventory_apply_and_queue_click(world, counters, net_commands, request);
    }
    true
}

fn local_inventory_release_fallback_click(
    world: &WorldStore,
    click_target: Option<InventoryClickTarget>,
    shift_down: bool,
) -> Option<(i16, ContainerInput)> {
    match click_target {
        Some(InventoryClickTarget::Slot(slot)) => Some((
            slot,
            if shift_down {
                ContainerInput::QuickMove
            } else {
                ContainerInput::Pickup
            },
        )),
        Some(InventoryClickTarget::Outside) if !inventory_cursor_is_empty(world) => {
            Some((-999, ContainerInput::Pickup))
        }
        Some(InventoryClickTarget::Outside | InventoryClickTarget::EmptyPanel) | None => None,
    }
}

fn local_inventory_queue_quick_craft(
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    button_num: i8,
    slots: Vec<i16>,
) {
    let instabuild = world
        .local_player()
        .abilities
        .is_some_and(|abilities| abilities.instabuild);
    let valid_button = matches!(button_num, 0 | 1) || (button_num == 2 && instabuild);
    if !valid_button || inventory_cursor_is_empty(world) {
        return;
    }
    let start = ContainerClickSlotRequest {
        slot_num: -999,
        button_num: local_inventory_quick_craft_mask(0, button_num),
        input: ContainerInput::QuickCraft,
    };
    local_inventory_apply_and_queue_click(world, counters, net_commands, start);
    for slot_num in slots {
        let add_slot = ContainerClickSlotRequest {
            slot_num,
            button_num: local_inventory_quick_craft_mask(1, button_num),
            input: ContainerInput::QuickCraft,
        };
        local_inventory_apply_and_queue_click(world, counters, net_commands, add_slot);
    }
    let finish = ContainerClickSlotRequest {
        slot_num: -999,
        button_num: local_inventory_quick_craft_mask(2, button_num),
        input: ContainerInput::QuickCraft,
    };
    local_inventory_apply_and_queue_click(world, counters, net_commands, finish);
}

fn local_inventory_quick_craft_mask(header: i8, quick_craft_type: i8) -> i8 {
    (header & 3) | ((quick_craft_type & 3) << 2)
}

fn maybe_apply_and_queue_crafter_slot_state_changed(
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    request: ContainerClickSlotRequest,
) -> bool {
    let Some(change) = world.plan_local_crafter_slot_state_change(
        request.slot_num,
        request.input,
        request.button_num,
    ) else {
        return false;
    };
    let Ok(fallback_click) = world.build_container_click_slot(request) else {
        return true;
    };
    let state = ContainerSlotStateChanged {
        slot_id: i32::from(change.slot_id),
        container_id: change.container_id,
        new_state: change.enabled,
    };
    let _ = queue_container_slot_state_click_commands_atomically(counters, net_commands, || {
        if !world.apply_local_crafter_slot_state_change(change) {
            return None;
        }
        let click = world
            .apply_local_container_click_slot(request)
            .unwrap_or(fallback_click);
        Some((state, click))
    });
    true
}

fn local_inventory_apply_and_queue_click(
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    request: ContainerClickSlotRequest,
) -> bool {
    handle_bundle_slot_click(
        world,
        counters,
        net_commands,
        i32::from(request.slot_num),
        request.input,
    );
    let click = {
        match world.apply_local_container_click_slot(request) {
            Ok(click) => click,
            Err(ContainerClickBuildError::UnsupportedLocalClickInput(_))
                if world.local_inventory_is_open() =>
            {
                let Ok(click) = world.build_container_click_slot(request) else {
                    return false;
                };
                click
            }
            Err(_) if !world.local_inventory_is_open() => {
                let Ok(click) = world.build_container_click_slot(request) else {
                    return false;
                };
                click
            }
            Err(_) => return false,
        }
    };
    queue_container_click_command(counters, net_commands, click);
    true
}

fn maybe_queue_inventory_clone_click(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    if !world
        .local_player()
        .abilities
        .is_some_and(|abilities| abilities.instabuild)
    {
        return false;
    }
    if !inventory_cursor_is_empty(world) {
        return false;
    }
    let Some(InventoryClickTarget::Slot(slot_num)) =
        inventory_screen_click_target(world, cursor_position, surface_size)
    else {
        return false;
    };
    if !inventory_slot_has_item(world, slot_num) {
        return false;
    }

    input.container_mut().inventory_last_click_slot = None;
    input.container_mut().inventory_last_click_button_num = None;
    input.container_mut().inventory_last_click_at = None;
    input.container_mut().inventory_double_click = false;
    local_inventory_clear_quick_craft(input);
    clear_recipe_book_last_placed_if_crafting_slot(input, world, slot_num);
    local_inventory_apply_and_queue_click(
        world,
        counters,
        net_commands,
        ContainerClickSlotRequest {
            slot_num,
            button_num: 2,
            input: ContainerInput::Clone,
        },
    )
}

fn reset_container_click_state(input: &mut ClientInputState, skip_next_release: bool) {
    input.container_mut().inventory_last_click_slot = None;
    input.container_mut().inventory_last_click_button_num = None;
    input.container_mut().inventory_last_click_at = None;
    input.container_mut().inventory_double_click = false;
    input.container_mut().inventory_skip_next_release = skip_next_release;
    local_inventory_clear_quick_craft(input);
}

fn clear_recipe_book_last_placed_if_crafting_slot(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    slot_num: i16,
) {
    let Some(background) = inventory_screen_layout(world).map(|layout| layout.background) else {
        return;
    };
    let is_crafting_slot = match background {
        InventoryScreenBackground::LocalInventory => (0..=4).contains(&slot_num),
        InventoryScreenBackground::CraftingTable => (0..=9).contains(&slot_num),
        InventoryScreenBackground::Furnace
        | InventoryScreenBackground::BlastFurnace
        | InventoryScreenBackground::Smoker => (0..=2).contains(&slot_num),
        _ => false,
    };
    if is_crafting_slot {
        input.container_mut().recipe_book_last_placed_recipe = None;
        world.clear_ghost_recipe();
    }
}

fn local_inventory_clear_quick_craft(input: &mut ClientInputState) {
    input.container_mut().inventory_quick_craft_button_num = None;
    input.container_mut().inventory_quick_craft_slots.clear();
}

fn local_inventory_add_quick_craft_slot(
    input: &mut ClientInputState,
    world: &WorldStore,
    slot_num: i16,
) {
    if !local_inventory_can_add_quick_craft_slot(input, world, slot_num) {
        return;
    }
    input
        .container_mut()
        .inventory_quick_craft_slots
        .push(slot_num);
}

fn local_inventory_can_add_quick_craft_slot(
    input: &ClientInputState,
    world: &WorldStore,
    slot_num: i16,
) -> bool {
    if input
        .container()
        .inventory_quick_craft_slots
        .contains(&slot_num)
        || (world.local_inventory_is_open() && slot_num == 0)
    {
        return false;
    }
    let Some(button_num) = input.container().inventory_quick_craft_button_num else {
        return false;
    };
    let instabuild = world
        .local_player()
        .abilities
        .is_some_and(|abilities| abilities.instabuild);
    let valid_button = matches!(button_num, 0 | 1) || (button_num == 2 && instabuild);
    if !valid_button || inventory_cursor_is_empty(world) {
        return false;
    }
    if button_num != 2
        && world.inventory().cursor_item.count
            <= i32::try_from(input.container().inventory_quick_craft_slots.len())
                .unwrap_or(i32::MAX)
    {
        return false;
    }
    world.active_container_slot_can_quick_craft(slot_num)
}

fn local_inventory_slot_item(world: &WorldStore, slot_num: i16) -> Option<&ItemStackSummary> {
    let inventory = world.inventory();
    let container = inventory
        .open_container
        .as_ref()
        .unwrap_or(&inventory.inventory_menu);
    inventory
        .container_slot(container, slot_num)
        .map(|slot| slot.item)
}

fn local_inventory_is_double_click(
    input: &ClientInputState,
    slot: i16,
    button_num: i8,
    now: Instant,
) -> bool {
    input.container().inventory_last_click_slot == Some(slot)
        && input.container().inventory_last_click_button_num == Some(button_num)
        && input
            .container()
            .inventory_last_click_at
            .and_then(|last| now.checked_duration_since(last))
            .is_some_and(|elapsed| elapsed < VANILLA_DOUBLE_CLICK_THRESHOLD)
}

pub(crate) fn anvil_rename_entry_consumes_key(world: &WorldStore, _code: KeyCode) -> bool {
    anvil_rename_input_signature(world).is_some()
}

pub(crate) fn recipe_book_search_entry_consumes_key(
    input: &ClientInputState,
    world: &WorldStore,
    _code: KeyCode,
) -> bool {
    recipe_book_search_is_active(input, world)
}

pub(crate) fn creative_inventory_search_entry_consumes_key(
    input: &ClientInputState,
    world: &WorldStore,
    code: KeyCode,
) -> bool {
    let Some(snapshot) = world.creative_inventory_snapshot() else {
        return false;
    };
    input.screen.creative().is_some()
        && input
            .screen
            .creative()
            .unwrap()
            .search_entry_consumes_key(snapshot, code)
}

pub(crate) fn handle_inventory_text_input(
    input: &mut ClientInputState,
    world: &WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    text: &str,
) -> bool {
    if input.screen.creative().is_some() {
        if !input.focused {
            return false;
        }
        let Some(snapshot) = world.creative_inventory_snapshot() else {
            return true;
        };
        let handled = input
            .screen
            .creative_mut()
            .unwrap()
            .handle_text(snapshot, text);
        input.refresh_creative_visible_items(world, item_runtime);
        return handled;
    }
    if !input.focused
        || input.screen.container().is_none()
        || inventory_screen_layout(world).is_none()
    {
        return false;
    }

    if handle_recipe_book_search_text_input(input, world, text) {
        return true;
    }

    if !anvil_screen_is_open(world) {
        return false;
    }

    sync_anvil_rename_input(input, world, item_runtime);
    if input.container_mut().anvil_rename_input.is_none() {
        return true;
    }

    let before = input.container_mut().anvil_rename.value().to_vec();
    input.container_mut().anvil_rename.type_text(text, None);
    if input.container_mut().anvil_rename.value() != before {
        queue_anvil_rename(input, counters, net_commands);
    }
    true
}

#[cfg(test)]
pub(crate) fn handle_inventory_key_input(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    code: KeyCode,
) -> bool {
    handle_inventory_key_input_with_clipboard(
        input,
        world,
        counters,
        net_commands,
        item_runtime,
        code,
        &mut None,
    )
}

pub(crate) fn handle_inventory_key_input_with_clipboard(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    code: KeyCode,
    clipboard: &mut Option<&mut dyn DebugClipboard>,
) -> bool {
    if input.screen.creative().is_some() {
        if !input.focused {
            return false;
        }
        let control_down = input.control_down();
        let modifiers = input.text_field_modifiers();
        if input.binding_matches("key.drop", code)
            || input.binding_matches("key.swapOffhand", code)
            || hotbar_slot_for_key(input.key_bindings(), code).is_some()
        {
            let Some(snapshot) = world.creative_inventory_snapshot_shared() else {
                return true;
            };
            let bindings = input.key_bindings().clone();
            if input.screen.creative_mut().unwrap().handle_action_key(
                world,
                counters,
                net_commands,
                item_runtime,
                &snapshot,
                &bindings,
                code,
                control_down,
            ) {
                return true;
            }
            input
                .screen
                .creative_mut()
                .unwrap()
                .handle_key(&snapshot, &bindings, code, modifiers, clipboard);
            input.refresh_creative_visible_items(world, item_runtime);
            return true;
        }
        let Some(snapshot) = world.creative_inventory_snapshot() else {
            return true;
        };
        let bindings = input.key_bindings().clone();
        input
            .screen
            .creative_mut()
            .unwrap()
            .handle_key(snapshot, &bindings, code, modifiers, clipboard);
        input.refresh_creative_visible_items(world, item_runtime);
        return true;
    }
    if !input.focused
        || input.screen.container().is_none()
        || inventory_screen_layout(world).is_none()
    {
        return false;
    }

    if handle_recipe_book_search_key_input(input, world, code, clipboard) {
        return true;
    }

    if handle_anvil_rename_key_input(
        input,
        world,
        counters,
        net_commands,
        item_runtime,
        code,
        clipboard,
    ) {
        return true;
    }

    if let Some(button_num) = local_inventory_swap_button_num(input, code) {
        handle_inventory_swap_key(input, world, counters, net_commands, button_num);
        return true;
    }

    if !input.binding_matches("key.drop", code) {
        return true;
    }

    let Some(slot_num) = input.container_mut().inventory_hovered_slot else {
        return true;
    };
    if !inventory_slot_has_item(world, slot_num) {
        return true;
    }
    let request = ContainerClickSlotRequest {
        slot_num,
        button_num: if input.control_down() { 1 } else { 0 },
        input: ContainerInput::Throw,
    };
    clear_recipe_book_last_placed_if_crafting_slot(input, world, slot_num);
    if world.local_inventory_is_open() {
        let click = match world.apply_local_container_click_slot(request) {
            Ok(click) => {
                if click.changed_slots.is_empty() {
                    return true;
                }
                click
            }
            Err(bbb_client_world::ContainerClickBuildError::UnsupportedLocalClickInput(_)) => {
                let Ok(click) = world.build_container_click_slot(request) else {
                    return true;
                };
                click
            }
            Err(_) => return true,
        };
        queue_container_click_command(counters, net_commands, click);
        return true;
    }
    local_inventory_apply_and_queue_click(world, counters, net_commands, request);
    true
}

fn handle_recipe_book_search_text_input(
    input: &mut ClientInputState,
    world: &WorldStore,
    text: &str,
) -> bool {
    if !recipe_book_search_is_active(input, world) {
        return false;
    }
    if input
        .container_mut()
        .recipe_book_search_suppress_open_key_commit
    {
        input
            .container_mut()
            .recipe_book_search_suppress_open_key_commit = false;
        let matches_open = match input.key_bindings().input("key.chat") {
            crate::options::key_bindings::BoundInput::Key(code) => {
                crate::input::key_code_matches_opening_text(code, text)
            }
            _ => matches!(text, "t" | "T"),
        };
        if matches_open {
            return true;
        }
    }
    if !text.chars().any(text_field::is_allowed_chat_character) {
        return false;
    }
    input
        .container_mut()
        .recipe_book_search
        .type_text(text, None);
    maybe_request_recipe_book_language(input);
    true
}

fn handle_recipe_book_search_key_input(
    input: &mut ClientInputState,
    world: &WorldStore,
    code: KeyCode,
    clipboard: &mut Option<&mut dyn DebugClipboard>,
) -> bool {
    if !recipe_book_search_box_is_open(world) {
        input.container_mut().recipe_book_search_focused = false;
        input
            .container_mut()
            .recipe_book_search_suppress_open_key_commit = false;
        return false;
    }

    if !input.container_mut().recipe_book_search_focused {
        if input.binding_matches("key.chat", code) {
            input.container_mut().recipe_book_search_focused = true;
            input.container_mut().recipe_book_search.focus();
            let end = input.container_mut().recipe_book_search.value().len();
            input
                .container_mut()
                .recipe_book_search
                .move_cursor(end, false, None);
            input
                .container_mut()
                .recipe_book_search_suppress_open_key_commit = true;
            return true;
        }
        return false;
    }

    if code == KeyCode::Escape {
        return false;
    }
    let modifiers = input.text_field_modifiers();
    let search_box_handled = input
        .container_mut()
        .recipe_book_search
        .key(code, modifiers, clipboard, None)
        .is_some();
    if search_box_handled {
        maybe_request_recipe_book_language(input);
    }
    // `RecipeBookComponent.keyPressed` consumes every non-Escape key while
    // its visible search box is focused, but calls `checkSearchStringUpdate`
    // only when `EditBox.keyPressed` returned true.
    true
}

/// `RecipeBookComponent.checkSearchStringUpdate` lowercases with
/// `Locale.ROOT` and compares with the fixed `excitedze` token after both
/// `keyPressed` and `charTyped`. The language owner decides whether `en_pt`
/// exists and whether it is already selected.
fn maybe_request_recipe_book_language(input: &mut ClientInputState) {
    if input
        .container_mut()
        .recipe_book_search
        .text()
        .eq_ignore_ascii_case("excitedze")
    {
        input.recipe_book_language_requests = input.recipe_book_language_requests.saturating_add(1);
    }
}

fn handle_anvil_rename_key_input(
    input: &mut ClientInputState,
    world: &WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    code: KeyCode,
    clipboard: &mut Option<&mut dyn DebugClipboard>,
) -> bool {
    if !anvil_screen_is_open(world) {
        return false;
    }
    sync_anvil_rename_input(input, world, item_runtime);
    // AnvilScreen.slotChanged: the name box is editable only with an input item.
    if input.container_mut().anvil_rename_input.is_none() {
        return false;
    }
    let modifiers = input.text_field_modifiers();
    let before = input.container_mut().anvil_rename.value().to_vec();
    if input
        .container_mut()
        .anvil_rename
        .key(code, modifiers, clipboard, None)
        .is_none()
    {
        return false;
    }
    if input.container_mut().anvil_rename.value() != before {
        queue_anvil_rename(input, counters, net_commands);
    }
    true
}

pub(crate) fn recipe_book_search_is_active(input: &ClientInputState, world: &WorldStore) -> bool {
    input
        .screen
        .container()
        .is_some_and(|state| state.recipe_book_search_focused)
        && recipe_book_search_box_is_open(world)
}

fn set_recipe_book_selected_tab_index(
    input: &mut ClientInputState,
    book_type: RecipeBookType,
    index: usize,
) {
    if selected_recipe_book_tab_index(input, book_type) != index {
        set_recipe_book_page_index(input, book_type, 0);
    }
    match book_type {
        RecipeBookType::Crafting => input.recipe_book_crafting_tab_index = index,
        RecipeBookType::Furnace => input.recipe_book_furnace_tab_index = index,
        RecipeBookType::BlastFurnace => input.recipe_book_blast_furnace_tab_index = index,
        RecipeBookType::Smoker => input.recipe_book_smoker_tab_index = index,
    }
}

fn selected_recipe_book_tab_index(input: &ClientInputState, book_type: RecipeBookType) -> usize {
    match book_type {
        RecipeBookType::Crafting => input.recipe_book_crafting_tab_index,
        RecipeBookType::Furnace => input.recipe_book_furnace_tab_index,
        RecipeBookType::BlastFurnace => input.recipe_book_blast_furnace_tab_index,
        RecipeBookType::Smoker => input.recipe_book_smoker_tab_index,
    }
}

fn selected_recipe_book_page_index(input: &ClientInputState, book_type: RecipeBookType) -> usize {
    match book_type {
        RecipeBookType::Crafting => input.recipe_book_crafting_page,
        RecipeBookType::Furnace => input.recipe_book_furnace_page,
        RecipeBookType::BlastFurnace => input.recipe_book_blast_furnace_page,
        RecipeBookType::Smoker => input.recipe_book_smoker_page,
    }
}

fn set_recipe_book_page_index(
    input: &mut ClientInputState,
    book_type: RecipeBookType,
    page: usize,
) {
    match book_type {
        RecipeBookType::Crafting => input.recipe_book_crafting_page = page,
        RecipeBookType::Furnace => input.recipe_book_furnace_page = page,
        RecipeBookType::BlastFurnace => input.recipe_book_blast_furnace_page = page,
        RecipeBookType::Smoker => input.recipe_book_smoker_page = page,
    }
}

fn recipe_book_search_box_is_open(world: &WorldStore) -> bool {
    let Some(layout) = inventory_screen_layout(world) else {
        return false;
    };
    recipe_book_type_for_background(layout.background).is_some()
        && recipe_book_main_gui_offset(world, layout.background) != 0
}

fn sync_anvil_rename_input(
    input: &mut ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) {
    let next = anvil_rename_input_signature(world);
    if input.container_mut().anvil_rename_input != next {
        input.container_mut().anvil_rename_input = next;
        input.container_mut().anvil_rename_hover_name =
            anvil_initial_rename_text(world, item_runtime);
        let state = input.container_mut();
        state
            .anvil_rename
            .set_text(&state.anvil_rename_hover_name, None);
    }
}

fn anvil_initial_rename_text(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> String {
    let Some(input) = anvil_input_slot_item(world) else {
        return String::new();
    };
    let effective = world.effective_item_stack(input);
    item_runtime
        .and_then(|runtime| {
            tooltip_lines_for_stack_with_context(
                runtime,
                effective.as_ref(),
                NativeItemTooltipOptions {
                    item_profiles: Some(world.item_profiles()),
                    ..NativeItemTooltipOptions::default()
                },
            )
        })
        .and_then(|lines| lines.into_iter().next())
        .map(|line| line.text)
        .or_else(|| anvil_component_hover_name(effective.as_ref()))
        .unwrap_or_default()
}

fn anvil_component_hover_name(stack: &ItemStackSummary) -> Option<String> {
    if let Some(name) = &stack.component_patch.custom_name {
        return Some(name.clone());
    }
    if let Some(title) = stack
        .component_patch
        .written_book
        .as_ref()
        .map(|book| &book.title)
        .filter(|title| !title.is_minecraft_blank())
    {
        return Some(title.to_display_string());
    }
    stack.component_patch.item_name.clone()
}

pub(crate) fn anvil_screen_is_open(world: &WorldStore) -> bool {
    world
        .inventory()
        .open_container
        .as_ref()
        .is_some_and(|container| container.menu_type_id == Some(VANILLA_MENU_TYPE_ANVIL_ID))
}

fn anvil_rename_input_signature(world: &WorldStore) -> Option<AnvilRenameInputSignature> {
    let container = world.inventory().open_container.as_ref()?;
    if container.menu_type_id != Some(VANILLA_MENU_TYPE_ANVIL_ID) {
        return None;
    }
    let item = world.inventory().container_slot(container, 0)?.item;
    if item_stack_is_empty(item) {
        return None;
    }
    Some(AnvilRenameInputSignature { item: item.clone() })
}

fn anvil_input_slot_item(world: &WorldStore) -> Option<&ItemStackSummary> {
    let container = world.inventory().open_container.as_ref()?;
    if container.menu_type_id != Some(VANILLA_MENU_TYPE_ANVIL_ID) {
        return None;
    }
    let item = world.inventory().container_slot(container, 0)?.item;
    (!item_stack_is_empty(item)).then_some(item)
}

fn queue_anvil_rename(
    input: &ClientInputState,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    let name = anvil_rename_wire_name(input);
    queue_rename_item_command(counters, net_commands, RenameItem { name });
}

fn anvil_rename_wire_name(input: &ClientInputState) -> String {
    if let Some(signature) = &input.container().anvil_rename_input {
        if signature.item.component_patch.custom_name.is_none()
            && input.container().anvil_rename.text() == input.container().anvil_rename_hover_name
        {
            return String::new();
        }
    }
    input.container().anvil_rename.text()
}

fn local_inventory_swap_button_num(input: &ClientInputState, code: KeyCode) -> Option<i8> {
    if input.binding_matches("key.swapOffhand", code) {
        return Some(40);
    }
    hotbar_slot_for_key(input.key_bindings(), code).map(|slot| slot as i8)
}

fn handle_inventory_swap_key(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    button_num: i8,
) {
    if !inventory_cursor_is_empty(world) {
        return;
    }
    let Some(slot_num) = input.container().inventory_hovered_slot else {
        return;
    };
    let request = ContainerClickSlotRequest {
        slot_num,
        button_num,
        input: ContainerInput::Swap,
    };
    clear_recipe_book_last_placed_if_crafting_slot(input, world, slot_num);
    if maybe_apply_and_queue_crafter_slot_state_changed(world, counters, net_commands, request) {
        return;
    }
    if world.local_inventory_is_open() {
        handle_bundle_slot_click(
            world,
            counters,
            net_commands,
            i32::from(slot_num),
            ContainerInput::Swap,
        );
        let click = match world.apply_local_container_click_slot(request) {
            Ok(click) => {
                if click.changed_slots.is_empty() {
                    return;
                }
                click
            }
            Err(bbb_client_world::ContainerClickBuildError::UnsupportedLocalClickInput(_)) => {
                let Ok(click) = world.build_container_click_slot(request) else {
                    return;
                };
                click
            }
            Err(_) => return,
        };
        queue_container_click_command(counters, net_commands, click);
        return;
    }
    local_inventory_apply_and_queue_click(world, counters, net_commands, request);
}

pub(crate) fn handle_inventory_mouse_wheel(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    delta: MouseScrollDelta,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    if input.screen.creative().is_some() {
        if !input.focused {
            return false;
        }
        let bundle_slot = input
            .screen
            .creative()
            .and_then(|creative| creative.hovered_slot())
            .and_then(|slot| match slot {
                bbb_client_render::HudCreativeInventorySlot::Hotbar(index) => {
                    i32::try_from(36 + index).ok()
                }
                bbb_client_render::HudCreativeInventorySlot::Inventory(slot_num) => {
                    Some(i32::from(slot_num))
                }
                bbb_client_render::HudCreativeInventorySlot::Picker(_)
                | bbb_client_render::HudCreativeInventorySlot::Trash => None,
            });
        if let Some(slot_id) = bundle_slot {
            if handle_bundle_slot_mouse_scroll(input, world, counters, net_commands, slot_id, delta)
            {
                return true;
            }
        }
        let visible_item_count = input.screen.creative_mut().unwrap().visible_item_count();
        input
            .screen
            .creative_mut()
            .unwrap()
            .handle_wheel(delta, visible_item_count);
        return true;
    }
    if !input.focused
        || input.screen.container().is_none()
        || inventory_screen_layout_for_surface(world, surface_size).is_none()
    {
        return false;
    }
    // Vanilla `AbstractContainerScreen.mouseScrolled` (`AbstractContainerScreen.java:170-181`)
    // runs `ItemSlotMouseAction` (bundles) before subclass list scrolling.
    if let Some(slot) = inventory_screen_hovered_slot(world, cursor_position, surface_size) {
        if handle_bundle_slot_mouse_scroll(
            input,
            world,
            counters,
            net_commands,
            i32::from(slot),
            delta,
        ) {
            return true;
        }
    }
    if maybe_scroll_stonecutter_recipes(input, world, &delta) {
        return true;
    }
    if maybe_scroll_loom_patterns(input, world, &delta) {
        return true;
    }
    if maybe_scroll_merchant_trades(input, world, &delta) {
        return true;
    }
    true
}

pub(crate) fn advance_creative_inventory_commands(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    ticks: u32,
) {
    let mut creative_open = false;
    for screen in input.screen.iter_mut() {
        if let super::Screen::Creative(state) = screen {
            state.advance_commands(world, counters, net_commands, ticks);
            creative_open = true;
        }
    }
    if !creative_open {
        input.creative_drop_spam_count = input
            .creative_drop_spam_count
            .saturating_sub(u16::try_from(ticks).unwrap_or(u16::MAX));
    }
}

pub(crate) use menu_widgets::recipe_book_button_at_position;

fn inventory_screen_click_target(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<InventoryClickTarget> {
    let layout = inventory_screen_layout_for_surface(world, surface_size)?;
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    let inside_panel = if let Some(recipe_book) = layout.recipe_book {
        if recipe_book.narrow {
            (x >= 0.0 && y >= 0.0 && x < f64::from(layout.width) && y < f64::from(layout.height))
                || recipe_book_tab_at_position(world, cursor_position, surface_size).is_some()
        } else {
            // Vanilla `AbstractRecipeBookScreen.hasClickedOutside` (`AbstractRecipeBookScreen.java:111-114`)
            // and `RecipeBookComponent.hasClickedOutside` (`RecipeBookComponent.java:416-424`):
            // the recipe book + 176px main GUI span `[recipe_book.x, main_gui_x_offset + 176)` (325px),
            // and visible recipe-book tabs count as inside for every mouse button.
            (x >= f64::from(recipe_book.x)
                && x < f64::from(recipe_book.main_gui_x_offset + 176)
                && y >= 0.0
                && y < f64::from(layout.height))
                || recipe_book_tab_at_position(world, cursor_position, surface_size).is_some()
        }
    } else {
        x >= 0.0 && y >= 0.0 && x < f64::from(layout.width) && y < f64::from(layout.height)
    };
    if !inside_panel {
        return Some(InventoryClickTarget::Outside);
    }
    for slot in layout.slots.iter() {
        if x >= f64::from(slot.x) - SLOT_HOVER_MARGIN
            && x < f64::from(slot.x) + SLOT_SIZE + SLOT_HOVER_MARGIN
            && y >= f64::from(slot.y) - SLOT_HOVER_MARGIN
            && y < f64::from(slot.y) + SLOT_SIZE + SLOT_HOVER_MARGIN
        {
            return Some(InventoryClickTarget::Slot(slot.slot_id));
        }
    }
    Some(InventoryClickTarget::EmptyPanel)
}

fn inventory_screen_origin(
    surface_size: PhysicalSize<u32>,
    layout: &InventoryScreenLayout,
) -> (f64, f64) {
    (
        (f64::from(surface_size.width.max(1)) - f64::from(layout.width)) * 0.5,
        (f64::from(surface_size.height.max(1)) - f64::from(layout.height)) * 0.5,
    )
}

fn inventory_cursor_is_empty(world: &WorldStore) -> bool {
    item_stack_is_empty(&world.inventory().cursor_item)
}

fn local_inventory_slot_has_item(world: &WorldStore, slot_num: i16) -> bool {
    local_inventory_slot_item(world, slot_num).is_some_and(|item| !item_stack_is_empty(item))
}

fn inventory_slot_has_item(world: &WorldStore, slot_num: i16) -> bool {
    if world.local_inventory_is_open() {
        return local_inventory_slot_has_item(world, slot_num);
    }
    world
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| world.inventory().container_slot(container, slot_num))
        .is_some_and(|slot| !item_stack_is_empty(slot.item))
}

fn item_stack_is_empty(stack: &ItemStackSummary) -> bool {
    stack.item_id.is_none() || stack.count <= 0
}

#[cfg(test)]
mod tests;
