use std::time::{Duration, Instant};

use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::packets::{
    ContainerInput, ItemStackSummary, PlaceRecipeCommand, RecipeBookChangeSettingsCommand,
    RecipeBookType, RenameItem, SelectTradeCommand, SetBeacon,
};
#[cfg(test)]
use bbb_world::ItemEquipmentSlot;
use bbb_world::{
    ContainerClickBuildError, ContainerClickSlotRequest, MountEquipmentSlotVisibility,
    MountInventoryKind, WorldStore,
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
        queue_container_close_command, queue_container_slot_state_changed_command,
        queue_place_recipe_command, queue_recipe_book_change_settings_command,
        queue_rename_item_command, queue_select_trade_command, queue_set_beacon_command,
    },
    text_edit, AnvilRenameInputSignature, ClientInputState,
};
use crate::recipe_book_ui::{
    clamped_recipe_book_page, crafting_recipe_book_collections,
    crafting_recipe_book_visible_tab_indices, furnace_recipe_book_collections,
    furnace_recipe_book_visible_tab_indices, recipe_book_page_count, recipe_book_slot_select_index,
    RecipeBookCraftingGrid, RecipeBookFurnaceFamily, RecipeBookUiCollection,
    RECIPE_BOOK_ITEMS_PER_PAGE,
};
use bbb_item_model::NativeItemRuntime;

mod layout;

#[cfg(test)]
pub(crate) use layout::local_inventory_slot_layouts;
pub(crate) use layout::{
    inventory_screen_layout, inventory_screen_selected_hotbar_slot_id, recipe_book_button_position,
    recipe_book_main_gui_offset, recipe_book_tab_count_for_background,
    recipe_book_type_for_background, recipe_book_type_settings, InventoryScreenBackground,
    InventoryScreenLayout, InventorySlotLayout, RECIPE_BOOK_BUTTON_HEIGHT,
    RECIPE_BOOK_BUTTON_WIDTH, RECIPE_BOOK_FILTER_BUTTON_HEIGHT, RECIPE_BOOK_FILTER_BUTTON_WIDTH,
    RECIPE_BOOK_FILTER_BUTTON_X, RECIPE_BOOK_FILTER_BUTTON_Y, RECIPE_BOOK_PAGE_BACKWARD_BUTTON_X,
    RECIPE_BOOK_PAGE_BUTTON_HEIGHT, RECIPE_BOOK_PAGE_BUTTON_WIDTH, RECIPE_BOOK_PAGE_BUTTON_Y,
    RECIPE_BOOK_PAGE_FORWARD_BUTTON_X, RECIPE_BOOK_RECIPE_BUTTON_COLUMNS,
    RECIPE_BOOK_RECIPE_BUTTON_SIZE, RECIPE_BOOK_RECIPE_BUTTON_X, RECIPE_BOOK_RECIPE_BUTTON_Y,
    RECIPE_BOOK_SEARCH_BOX_HEIGHT, RECIPE_BOOK_SEARCH_BOX_WIDTH, RECIPE_BOOK_SEARCH_BOX_X,
    RECIPE_BOOK_SEARCH_BOX_Y, RECIPE_BOOK_SEARCH_TEXT_X_OFFSET, RECIPE_BOOK_SEARCH_TEXT_Y_OFFSET,
    RECIPE_BOOK_SELECTED_TAB_X_OFFSET, RECIPE_BOOK_TAB_HEIGHT, RECIPE_BOOK_TAB_STRIDE_Y,
    RECIPE_BOOK_TAB_WIDTH, RECIPE_BOOK_TAB_X, RECIPE_BOOK_TAB_Y,
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
const GENERIC_CONTAINER_FIRST_MENU_TYPE_ID: i32 = 0;
const GENERIC_CONTAINER_LAST_MENU_TYPE_ID: i32 = 5;
const GENERIC_3X3_MENU_TYPE_ID: i32 = 6;
const CRAFTER_MENU_TYPE_ID: i32 = 7;
const ANVIL_MENU_TYPE_ID: i32 = 8;
const BEACON_MENU_TYPE_ID: i32 = 9;
const BLAST_FURNACE_MENU_TYPE_ID: i32 = 10;
const BREWING_STAND_MENU_TYPE_ID: i32 = 11;
const CRAFTING_MENU_TYPE_ID: i32 = 12;
const ENCHANTMENT_MENU_TYPE_ID: i32 = 13;
const FURNACE_MENU_TYPE_ID: i32 = 14;
const GRINDSTONE_MENU_TYPE_ID: i32 = 15;
const HOPPER_MENU_TYPE_ID: i32 = 16;
const LECTERN_MENU_TYPE_ID: i32 = 17;
const LOOM_MENU_TYPE_ID: i32 = 18;
const MERCHANT_MENU_TYPE_ID: i32 = 19;
const SHULKER_BOX_MENU_TYPE_ID: i32 = 20;
const SMITHING_MENU_TYPE_ID: i32 = 21;
const SMOKER_MENU_TYPE_ID: i32 = 22;
const CARTOGRAPHY_TABLE_MENU_TYPE_ID: i32 = 23;
const STONECUTTER_MENU_TYPE_ID: i32 = 24;
const STONECUTTER_SELECTED_RECIPE_DATA_ID: i16 = 0;
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
const ANVIL_RENAME_MAX_LENGTH: usize = 50;
const RECIPE_BOOK_SEARCH_MAX_LENGTH: usize = 50;
const BEACON_SCREEN_WIDTH: i32 = 230;
const BEACON_SCREEN_HEIGHT: i32 = 219;
const BEACON_SLOT_COUNT: i16 = 1;
const BEACON_LEVELS_DATA_ID: i16 = 0;
const BEACON_PRIMARY_EFFECT_DATA_ID: i16 = 1;
const BEACON_SECONDARY_EFFECT_DATA_ID: i16 = 2;
const BEACON_EFFECT_BUTTON_SIZE: i32 = 22;
const BEACON_EFFECT_BUTTON_SPACING: i32 = 24;
const BEACON_PRIMARY_EFFECT_CENTER_X: i32 = 76;
const BEACON_PRIMARY_EFFECT_Y: i32 = 22;
const BEACON_PRIMARY_EFFECT_ROW_SPACING: i32 = 25;
const BEACON_SECONDARY_EFFECT_CENTER_X: i32 = 167;
const BEACON_SECONDARY_EFFECT_Y: i32 = 47;
const BEACON_EFFECT_SPEED_ID: i32 = 0;
const BEACON_EFFECT_HASTE_ID: i32 = 2;
const BEACON_EFFECT_STRENGTH_ID: i32 = 4;
const BEACON_EFFECT_JUMP_BOOST_ID: i32 = 7;
const BEACON_EFFECT_REGENERATION_ID: i32 = 9;
const BEACON_EFFECT_RESISTANCE_ID: i32 = 10;
const BEACON_PRIMARY_EFFECT_ROWS: [&[i32]; 3] = [
    &[BEACON_EFFECT_SPEED_ID, BEACON_EFFECT_HASTE_ID],
    &[BEACON_EFFECT_RESISTANCE_ID, BEACON_EFFECT_JUMP_BOOST_ID],
    &[BEACON_EFFECT_STRENGTH_ID],
];
const BEACON_SECONDARY_EFFECTS: &[i32] = &[BEACON_EFFECT_REGENERATION_ID];
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
const LOOM_SELECTED_PATTERN_DATA_ID: i16 = 0;
const LOOM_PATTERN_BUTTON_X: i32 = 60;
const LOOM_PATTERN_BUTTON_Y: i32 = 13;
const LOOM_PATTERN_BUTTON_COLUMNS: i32 = 4;
const LOOM_PATTERN_BUTTON_ROWS: i32 = 4;
const LOOM_PATTERN_BUTTON_SIZE: i32 = 14;
const LOOM_SCROLLER_X: i32 = 119;
const LOOM_SCROLLER_CLICK_Y: i32 = 9;
const LOOM_SCROLLER_DRAG_Y: i32 = 13;
const LOOM_SCROLLER_WIDTH: i32 = 12;
const LOOM_SCROLLER_HEIGHT: i32 = 15;
const LOOM_SCROLLER_FULL_HEIGHT: i32 = 56;
const LOOM_NO_ITEM_REQUIRED_PATTERN_COUNT: i32 = 32;
const LOOM_PATTERN_ITEM_PATTERN_COUNT: i32 = 1;
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
const STONECUTTER_RECIPE_BUTTON_X: i32 = 52;
const STONECUTTER_RECIPE_BUTTON_Y: i32 = 14;
const STONECUTTER_RECIPE_BUTTON_COLUMNS: i32 = 4;
const STONECUTTER_RECIPE_BUTTON_ROWS: i32 = 3;
const STONECUTTER_RECIPE_BUTTON_WIDTH: i32 = 16;
const STONECUTTER_RECIPE_BUTTON_HEIGHT: i32 = 18;
const STONECUTTER_SCROLLER_X: i32 = 119;
const STONECUTTER_SCROLLER_CLICK_Y: i32 = 9;
const STONECUTTER_SCROLLER_DRAG_Y: i32 = 14;
const STONECUTTER_SCROLLER_WIDTH: i32 = 12;
const STONECUTTER_SCROLLER_HEIGHT: i32 = 15;
const STONECUTTER_SCROLLER_FULL_HEIGHT: i32 = 54;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BeaconEffectButton {
    primary: bool,
    tier: i16,
    effect_id: i32,
    x: i32,
    y: i32,
}

pub(crate) fn handle_inventory_cursor_moved(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    if !input.focused || inventory_screen_layout(world).is_none() {
        return false;
    }
    input.inventory_cursor_position =
        inventory_screen_cursor_position(world, cursor_position, surface_size);

    if input.merchant_trade_scrolling
        && update_merchant_trade_scroll_from_cursor(input, world, cursor_position, surface_size)
    {
        return true;
    }
    if input.stonecutter_recipe_scrolling
        && update_stonecutter_recipe_scroll_from_cursor(input, world, cursor_position, surface_size)
    {
        return true;
    }
    if input.loom_pattern_scrolling
        && update_loom_pattern_scroll_from_cursor(input, world, cursor_position, surface_size)
    {
        return true;
    }
    let hovered = inventory_screen_hovered_slot(world, cursor_position, surface_size);
    if input.inventory_hovered_slot != hovered {
        if let Some(previous) = input.inventory_hovered_slot {
            handle_bundle_slot_hover_end(world, counters, net_commands, i32::from(previous));
        }
        input.inventory_hovered_slot = hovered;
    }
    if let Some(slot) = hovered {
        if world.local_inventory_is_open() {
            local_inventory_add_quick_craft_slot(input, world, slot);
        }
    }
    true
}

#[cfg(test)]
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
    if !input.focused || inventory_screen_layout(world).is_none() {
        return false;
    }
    input.inventory_cursor_position =
        inventory_screen_cursor_position(world, cursor_position, surface_size);
    let button_num = match button {
        MouseButton::Left => 0,
        MouseButton::Right => 1,
        MouseButton::Middle => 2,
        _ => return true,
    };

    if matches!(state, ElementState::Released) {
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
        maybe_queue_inventory_clone_click(
            input,
            world,
            counters,
            net_commands,
            cursor_position,
            surface_size,
        );
        return true;
    }
    if button_num == 0
        && maybe_focus_recipe_book_search(input, world, cursor_position, surface_size)
    {
        input.inventory_last_click_slot = None;
        input.inventory_last_click_button_num = None;
        input.inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if button_num == 0 && maybe_select_recipe_book_tab(input, world, cursor_position, surface_size)
    {
        input.inventory_last_click_slot = None;
        input.inventory_last_click_button_num = None;
        input.inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if button_num == 0
        && maybe_turn_recipe_book_page(input, world, item_runtime, cursor_position, surface_size)
    {
        input.inventory_last_click_slot = None;
        input.inventory_last_click_button_num = None;
        input.inventory_last_click_at = None;
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
        input.inventory_last_click_slot = None;
        input.inventory_last_click_button_num = None;
        input.inventory_last_click_at = None;
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
        input.inventory_last_click_slot = None;
        input.inventory_last_click_button_num = None;
        input.inventory_last_click_at = None;
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
        input.inventory_last_click_slot = None;
        input.inventory_last_click_button_num = None;
        input.inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if button_num == 0
        && maybe_queue_lectern_button_click(
            world,
            counters,
            net_commands,
            cursor_position,
            surface_size,
        )
    {
        input.inventory_last_click_slot = None;
        input.inventory_last_click_button_num = None;
        input.inventory_last_click_at = None;
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
        input.inventory_last_click_slot = None;
        input.inventory_last_click_button_num = None;
        input.inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if button_num == 0
        && maybe_queue_merchant_trade_click(
            world,
            counters,
            net_commands,
            cursor_position,
            surface_size,
        )
    {
        input.inventory_last_click_slot = None;
        input.inventory_last_click_button_num = None;
        input.inventory_last_click_at = None;
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
        input.inventory_last_click_slot = None;
        input.inventory_last_click_button_num = None;
        input.inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if maybe_queue_loom_pattern_click(
        input,
        world,
        counters,
        net_commands,
        cursor_position,
        surface_size,
    ) {
        input.inventory_last_click_slot = None;
        input.inventory_last_click_button_num = None;
        input.inventory_last_click_at = None;
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
        input.inventory_last_click_slot = None;
        input.inventory_last_click_button_num = None;
        input.inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if maybe_start_loom_pattern_scroll_drag(input, world, cursor_position, surface_size) {
        input.inventory_last_click_slot = None;
        input.inventory_last_click_button_num = None;
        input.inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if maybe_start_stonecutter_recipe_scroll_drag(input, world, cursor_position, surface_size) {
        input.inventory_last_click_slot = None;
        input.inventory_last_click_button_num = None;
        input.inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if maybe_start_merchant_trade_scroll_drag(input, world, cursor_position, surface_size) {
        input.inventory_last_click_slot = None;
        input.inventory_last_click_button_num = None;
        input.inventory_last_click_at = None;
        local_inventory_clear_quick_craft(input);
        return true;
    }

    let click_target = inventory_screen_click_target(world, cursor_position, surface_size);
    let now = Instant::now();
    let double_click_slot = match click_target {
        Some(InventoryClickTarget::Slot(slot)) => {
            let is_double_click = local_inventory_is_double_click(input, slot, button_num, now);
            input.inventory_last_click_slot = Some(slot);
            input.inventory_last_click_button_num = Some(button_num);
            input.inventory_last_click_at = Some(now);
            is_double_click.then_some(slot)
        }
        _ => {
            input.inventory_last_click_slot = None;
            input.inventory_last_click_button_num = None;
            input.inventory_last_click_at = None;
            None
        }
    };
    local_inventory_clear_quick_craft(input);
    let (slot_num, click_input) = match click_target {
        Some(InventoryClickTarget::Slot(slot)) => {
            if !world.local_inventory_is_open() {
                if input.shift_down() {
                    (slot, ContainerInput::QuickMove)
                } else {
                    (slot, ContainerInput::Pickup)
                }
            } else if double_click_slot == Some(slot)
                && button_num == 0
                && !input.shift_down()
                && !inventory_cursor_is_empty(world)
            {
                (slot, ContainerInput::PickupAll)
            } else if !inventory_cursor_is_empty(world) {
                input.inventory_quick_craft_button_num = Some(button_num);
                return true;
            } else if input.shift_down() {
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
    maybe_queue_crafter_slot_state_changed(
        world,
        counters,
        net_commands,
        request.slot_num,
        request.input,
    );
    local_inventory_apply_and_queue_click(world, counters, net_commands, request);
    true
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
        input.recipe_book_search_focused = false;
        input.recipe_book_search_suppress_open_key_commit = false;
        return false;
    }
    if !recipe_book_search_box_at_position(world, cursor_position, surface_size) {
        input.recipe_book_search_focused = false;
        input.recipe_book_search_suppress_open_key_commit = false;
        return false;
    }

    input.recipe_book_search_focused = true;
    let cursor = text_edit::char_len(&input.recipe_book_search_text);
    set_recipe_book_search_cursor(input, cursor);
    input.recipe_book_search_suppress_open_key_commit = false;
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
    input.recipe_book_search_focused = false;
    input.recipe_book_search_suppress_open_key_commit = false;
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
    input.recipe_book_search_focused = false;
    input.recipe_book_search_suppress_open_key_commit = false;
    true
}

fn maybe_queue_recipe_book_recipe_click(
    input: &mut ClientInputState,
    world: &WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(recipe_index) = recipe_book_recipe_button_at_position(
        input,
        world,
        item_runtime,
        cursor_position,
        surface_size,
    ) else {
        return false;
    };
    let Some(container_id) = world.open_container_id() else {
        return false;
    };
    queue_place_recipe_command(
        counters,
        net_commands,
        PlaceRecipeCommand {
            container_id,
            recipe_index,
            use_max_items: input.shift_down(),
        },
    );
    input.recipe_book_search_focused = false;
    input.recipe_book_search_suppress_open_key_commit = false;
    true
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
        input.recipe_book_search_focused = false;
        input.recipe_book_search_suppress_open_key_commit = false;
    }
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
            let Some(container_id) = world
                .inventory()
                .open_container
                .as_ref()
                .map(|container| container.container_id)
            else {
                return false;
            };
            queue_container_button_click_command(counters, net_commands, container_id, button_id);
            true
        }
    }
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
                input.beacon_primary_effect = Some(effect_id);
            } else {
                input.beacon_secondary_effect = Some(effect_id);
            }
            input.beacon_effect_selection_dirty = true;
            true
        }
    }
}

fn maybe_queue_merchant_trade_click(
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(item) = merchant_trade_at_position(world, cursor_position, surface_size) else {
        return false;
    };
    world.set_local_merchant_selected_offer(item);
    queue_select_trade_command(counters, net_commands, SelectTradeCommand { item });
    true
}

fn maybe_queue_enchantment_button_click(
    world: &WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(button_id) = enchantment_button_at_position(world, cursor_position, surface_size)
    else {
        return false;
    };
    if world
        .open_container_data_value(button_id as i16)
        .unwrap_or_default()
        <= 0
    {
        return false;
    }
    let Some(container_id) = world
        .inventory()
        .open_container
        .as_ref()
        .map(|container| container.container_id)
    else {
        return false;
    };
    queue_container_button_click_command(counters, net_commands, container_id, button_id);
    true
}

fn maybe_queue_loom_pattern_click(
    input: &mut ClientInputState,
    world: &WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    sync_loom_pattern_state(input, world);
    let start_index = input.loom_pattern_scroll_row * LOOM_PATTERN_BUTTON_COLUMNS;
    let Some(LoomClickTarget::Pattern(button_id)) =
        loom_click_target_at_position(world, start_index, cursor_position, surface_size)
    else {
        return false;
    };
    let Some(container_id) = world
        .inventory()
        .open_container
        .as_ref()
        .map(|container| container.container_id)
    else {
        return false;
    };
    input.loom_selected_pattern_index = Some(button_id);
    input.loom_pattern_selection_dirty = true;
    queue_container_button_click_command(counters, net_commands, container_id, button_id);
    true
}

fn maybe_queue_stonecutter_recipe_click(
    input: &mut ClientInputState,
    world: &WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    sync_stonecutter_recipe_scroll(input, world);
    let start_index = input.stonecutter_recipe_scroll_row * STONECUTTER_RECIPE_BUTTON_COLUMNS;
    let Some(button_id) =
        stonecutter_recipe_button_at_position(world, start_index, cursor_position, surface_size)
    else {
        return false;
    };
    if world.open_container_data_value(STONECUTTER_SELECTED_RECIPE_DATA_ID)
        == Some(button_id as i16)
    {
        return false;
    }
    let Some(container_id) = world
        .inventory()
        .open_container
        .as_ref()
        .map(|container| container.container_id)
    else {
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
    input.merchant_trade_scrolling = false;
    input.stonecutter_recipe_scrolling = false;
    input.loom_pattern_scrolling = false;
    let Some(quick_craft_button_num) = input.inventory_quick_craft_button_num else {
        return true;
    };
    if !world.local_inventory_is_open() {
        local_inventory_clear_quick_craft(input);
        return true;
    }
    if quick_craft_button_num != button_num {
        local_inventory_clear_quick_craft(input);
        return true;
    }

    let quick_craft_slots = std::mem::take(&mut input.inventory_quick_craft_slots);
    input.inventory_quick_craft_button_num = None;
    if !quick_craft_slots.is_empty() {
        local_inventory_queue_quick_craft(
            world,
            counters,
            net_commands,
            quick_craft_button_num,
            quick_craft_slots,
        );
        return true;
    }

    let click_target = inventory_screen_click_target(world, cursor_position, surface_size);
    let Some((slot_num, click_input)) = local_inventory_release_fallback_click(world, click_target)
    else {
        return true;
    };
    local_inventory_apply_and_queue_click(
        world,
        counters,
        net_commands,
        ContainerClickSlotRequest {
            slot_num,
            button_num,
            input: click_input,
        },
    );
    true
}

fn local_inventory_release_fallback_click(
    world: &WorldStore,
    click_target: Option<InventoryClickTarget>,
) -> Option<(i16, ContainerInput)> {
    match click_target {
        Some(InventoryClickTarget::Slot(slot)) => Some((slot, ContainerInput::Pickup)),
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
    if !matches!(button_num, 0 | 1) || inventory_cursor_is_empty(world) {
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

fn maybe_queue_crafter_slot_state_changed(
    world: &WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    slot_num: i16,
    click_input: ContainerInput,
) {
    if click_input != ContainerInput::Pickup || !(0..CRAFTER_GRID_SLOT_COUNT).contains(&slot_num) {
        return;
    }
    let Some(container) = world.inventory().open_container.as_ref() else {
        return;
    };
    if container.menu_type_id != Some(CRAFTER_MENU_TYPE_ID) {
        return;
    }
    let Some(slot) = container.slots.iter().find(|slot| slot.slot == slot_num) else {
        return;
    };
    if !item_stack_is_empty(&slot.item) {
        return;
    }

    let disabled = world
        .open_container_data_value(slot_num)
        .unwrap_or_default()
        == 1;
    let new_state = if disabled {
        true
    } else if inventory_cursor_is_empty(world) {
        false
    } else {
        return;
    };
    queue_container_slot_state_changed_command(
        counters,
        net_commands,
        i32::from(slot_num),
        container.container_id,
        new_state,
    );
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
    let click = if world.local_inventory_is_open()
        || matches!(
            request.input,
            ContainerInput::Pickup | ContainerInput::QuickMove | ContainerInput::Clone
        ) {
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
    } else {
        let Ok(click) = world.build_container_click_slot(request) else {
            return false;
        };
        click
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

    input.inventory_last_click_slot = None;
    input.inventory_last_click_button_num = None;
    input.inventory_last_click_at = None;
    local_inventory_clear_quick_craft(input);
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

fn local_inventory_clear_quick_craft(input: &mut ClientInputState) {
    input.inventory_quick_craft_button_num = None;
    input.inventory_quick_craft_slots.clear();
}

fn local_inventory_add_quick_craft_slot(
    input: &mut ClientInputState,
    world: &WorldStore,
    slot_num: i16,
) {
    if !local_inventory_can_add_quick_craft_slot(input, world, slot_num) {
        return;
    }
    input.inventory_quick_craft_slots.push(slot_num);
}

fn local_inventory_can_add_quick_craft_slot(
    input: &ClientInputState,
    world: &WorldStore,
    slot_num: i16,
) -> bool {
    if input.inventory_quick_craft_slots.contains(&slot_num) || slot_num == 0 {
        return false;
    }
    let Some(button_num) = input.inventory_quick_craft_button_num else {
        return false;
    };
    if !matches!(button_num, 0 | 1) || inventory_cursor_is_empty(world) {
        return false;
    }
    if world.inventory().cursor_item.count
        <= i32::try_from(input.inventory_quick_craft_slots.len()).unwrap_or(i32::MAX)
    {
        return false;
    }
    let Some(slot_item) = local_inventory_slot_item(world, slot_num) else {
        return false;
    };
    if item_stack_is_empty(slot_item) {
        return local_inventory_slot_max_stack_size(slot_num, &world.inventory().cursor_item) > 0;
    }
    if !same_item_same_components(slot_item, &world.inventory().cursor_item) {
        return false;
    }
    slot_item.count < local_inventory_slot_max_stack_size(slot_num, slot_item)
}

fn local_inventory_slot_item(world: &WorldStore, slot_num: i16) -> Option<&ItemStackSummary> {
    world
        .inventory()
        .inventory_menu
        .slots
        .iter()
        .find(|slot| slot.slot == slot_num)
        .map(|slot| &slot.item)
}

fn local_inventory_is_double_click(
    input: &ClientInputState,
    slot: i16,
    button_num: i8,
    now: Instant,
) -> bool {
    input.inventory_last_click_slot == Some(slot)
        && input.inventory_last_click_button_num == Some(button_num)
        && input
            .inventory_last_click_at
            .and_then(|last| now.checked_duration_since(last))
            .is_some_and(|elapsed| elapsed < VANILLA_DOUBLE_CLICK_THRESHOLD)
}

pub(crate) fn anvil_rename_entry_consumes_key(world: &WorldStore, code: KeyCode) -> bool {
    matches!(code, KeyCode::KeyE) && anvil_rename_input_signature(world).is_some()
}

pub(crate) fn recipe_book_search_entry_consumes_key(
    input: &ClientInputState,
    world: &WorldStore,
    code: KeyCode,
) -> bool {
    matches!(code, KeyCode::KeyE) && recipe_book_search_is_active(input, world)
}

pub(crate) fn handle_inventory_text_input(
    input: &mut ClientInputState,
    world: &WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    text: &str,
) -> bool {
    if !input.focused || inventory_screen_layout(world).is_none() {
        return false;
    }

    if handle_recipe_book_search_text_input(input, world, text) {
        return true;
    }

    if !anvil_screen_is_open(world) {
        return false;
    }

    sync_anvil_rename_input(input, world, item_runtime);
    if input.anvil_rename_input.is_none() {
        return true;
    }

    let before = input.anvil_rename_text.clone();
    insert_anvil_rename_text(input, text);
    if input.anvil_rename_text != before {
        queue_anvil_rename(input, counters, net_commands);
    }
    true
}

pub(crate) fn handle_inventory_key_input(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    code: KeyCode,
) -> bool {
    if !input.focused || inventory_screen_layout(world).is_none() {
        return false;
    }

    if handle_recipe_book_search_key_input(input, world, code) {
        return true;
    }

    if handle_anvil_rename_key_input(input, world, counters, net_commands, item_runtime, code) {
        return true;
    }

    if let Some(button_num) = local_inventory_swap_button_num(code) {
        handle_inventory_swap_key(input, world, counters, net_commands, button_num);
        return true;
    }

    if code != KeyCode::KeyQ {
        return true;
    }

    let Some(slot_num) = input.inventory_hovered_slot else {
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
    if world.local_inventory_is_open() {
        let Ok(click) = world.apply_local_container_click_slot(request) else {
            return true;
        };
        if click.changed_slots.is_empty() {
            return true;
        }
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
    if input.recipe_book_search_suppress_open_key_commit {
        input.recipe_book_search_suppress_open_key_commit = false;
        if matches!(text, "t" | "T") {
            return true;
        }
    }
    insert_recipe_book_search_text(input, text);
    true
}

fn handle_recipe_book_search_key_input(
    input: &mut ClientInputState,
    world: &WorldStore,
    code: KeyCode,
) -> bool {
    if !recipe_book_search_box_is_open(world) {
        input.recipe_book_search_focused = false;
        input.recipe_book_search_suppress_open_key_commit = false;
        return false;
    }

    if !input.recipe_book_search_focused {
        if matches!(code, KeyCode::KeyT) {
            input.recipe_book_search_focused = true;
            let cursor = text_edit::char_len(&input.recipe_book_search_text);
            set_recipe_book_search_cursor(input, cursor);
            input.recipe_book_search_suppress_open_key_commit = true;
            return true;
        }
        return false;
    }

    match code {
        KeyCode::Escape => false,
        KeyCode::KeyA if input.control_down() && !input.shift_down() => {
            select_recipe_book_search_text(input);
            true
        }
        KeyCode::ArrowLeft => {
            let cursor = if input.control_down() {
                text_edit::word_position(
                    &input.recipe_book_search_text,
                    input.recipe_book_search_cursor,
                    -1,
                )
            } else {
                input.recipe_book_search_cursor.saturating_sub(1)
            };
            set_recipe_book_search_cursor(input, cursor);
            true
        }
        KeyCode::ArrowRight => {
            let cursor = if input.control_down() {
                text_edit::word_position(
                    &input.recipe_book_search_text,
                    input.recipe_book_search_cursor,
                    1,
                )
            } else {
                (input.recipe_book_search_cursor + 1)
                    .min(text_edit::char_len(&input.recipe_book_search_text))
            };
            set_recipe_book_search_cursor(input, cursor);
            true
        }
        KeyCode::Home => {
            set_recipe_book_search_cursor(input, 0);
            true
        }
        KeyCode::End => {
            let cursor = text_edit::char_len(&input.recipe_book_search_text);
            set_recipe_book_search_cursor(input, cursor);
            true
        }
        KeyCode::Backspace => {
            let deleted_selection = delete_recipe_book_search_selection(input);
            if !deleted_selection && input.control_down() {
                text_edit::remove_word_before_cursor(
                    &mut input.recipe_book_search_text,
                    &mut input.recipe_book_search_cursor,
                );
                input.recipe_book_search_selection = input.recipe_book_search_cursor;
            } else if !deleted_selection {
                remove_recipe_book_search_char_before_cursor(
                    &mut input.recipe_book_search_text,
                    &mut input.recipe_book_search_cursor,
                );
                input.recipe_book_search_selection = input.recipe_book_search_cursor;
            }
            true
        }
        KeyCode::Delete => {
            let deleted_selection = delete_recipe_book_search_selection(input);
            if !deleted_selection && input.control_down() {
                text_edit::remove_word_at_cursor(
                    &mut input.recipe_book_search_text,
                    input.recipe_book_search_cursor,
                );
                input.recipe_book_search_selection = input.recipe_book_search_cursor;
            } else if !deleted_selection {
                remove_recipe_book_search_char_at_cursor(
                    &mut input.recipe_book_search_text,
                    input.recipe_book_search_cursor,
                );
                input.recipe_book_search_selection = input.recipe_book_search_cursor;
            }
            true
        }
        _ => true,
    }
}

fn handle_anvil_rename_key_input(
    input: &mut ClientInputState,
    world: &WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    code: KeyCode,
) -> bool {
    if !anvil_screen_is_open(world) {
        return false;
    }

    match code {
        KeyCode::KeyA if input.control_down() && !input.shift_down() => {
            sync_anvil_rename_input(input, world, item_runtime);
            if input.anvil_rename_input.is_some() {
                select_anvil_rename_text(input);
            }
            true
        }
        KeyCode::ArrowLeft => {
            sync_anvil_rename_input(input, world, item_runtime);
            if input.anvil_rename_input.is_some() {
                let cursor = if input.control_down() {
                    text_edit::word_position(
                        &input.anvil_rename_text,
                        input.anvil_rename_cursor,
                        -1,
                    )
                } else {
                    input.anvil_rename_cursor.saturating_sub(1)
                };
                set_anvil_rename_cursor(input, cursor);
            }
            true
        }
        KeyCode::ArrowRight => {
            sync_anvil_rename_input(input, world, item_runtime);
            if input.anvil_rename_input.is_some() {
                let cursor = if input.control_down() {
                    text_edit::word_position(&input.anvil_rename_text, input.anvil_rename_cursor, 1)
                } else {
                    (input.anvil_rename_cursor + 1)
                        .min(anvil_rename_char_len(&input.anvil_rename_text))
                };
                set_anvil_rename_cursor(input, cursor);
            }
            true
        }
        KeyCode::Home => {
            sync_anvil_rename_input(input, world, item_runtime);
            if input.anvil_rename_input.is_some() {
                set_anvil_rename_cursor(input, 0);
            }
            true
        }
        KeyCode::End => {
            sync_anvil_rename_input(input, world, item_runtime);
            if input.anvil_rename_input.is_some() {
                set_anvil_rename_cursor(input, anvil_rename_char_len(&input.anvil_rename_text));
            }
            true
        }
        KeyCode::Backspace => {
            sync_anvil_rename_input(input, world, item_runtime);
            if input.anvil_rename_input.is_some() {
                let before = input.anvil_rename_text.clone();
                let deleted_selection = delete_anvil_rename_selection(input);
                if !deleted_selection && input.control_down() {
                    text_edit::remove_word_before_cursor(
                        &mut input.anvil_rename_text,
                        &mut input.anvil_rename_cursor,
                    );
                    input.anvil_rename_selection = input.anvil_rename_cursor;
                } else if !deleted_selection {
                    remove_anvil_rename_char_before_cursor(
                        &mut input.anvil_rename_text,
                        &mut input.anvil_rename_cursor,
                    );
                    input.anvil_rename_selection = input.anvil_rename_cursor;
                }
                if input.anvil_rename_text != before {
                    queue_anvil_rename(input, counters, net_commands);
                }
            }
            true
        }
        KeyCode::Delete => {
            sync_anvil_rename_input(input, world, item_runtime);
            if input.anvil_rename_input.is_some() {
                let before = input.anvil_rename_text.clone();
                let deleted_selection = delete_anvil_rename_selection(input);
                if !deleted_selection && input.control_down() {
                    text_edit::remove_word_at_cursor(
                        &mut input.anvil_rename_text,
                        input.anvil_rename_cursor,
                    );
                    input.anvil_rename_selection = input.anvil_rename_cursor;
                } else if !deleted_selection {
                    remove_anvil_rename_char_at_cursor(
                        &mut input.anvil_rename_text,
                        input.anvil_rename_cursor,
                    );
                    input.anvil_rename_selection = input.anvil_rename_cursor;
                }
                if input.anvil_rename_text != before {
                    queue_anvil_rename(input, counters, net_commands);
                }
            }
            true
        }
        _ => false,
    }
}

fn recipe_book_search_is_active(input: &ClientInputState, world: &WorldStore) -> bool {
    input.recipe_book_search_focused && recipe_book_search_box_is_open(world)
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

fn insert_recipe_book_search_text(input: &mut ClientInputState, text: &str) {
    delete_recipe_book_search_selection(input);
    let current = &mut input.recipe_book_search_text;
    input.recipe_book_search_cursor = input
        .recipe_book_search_cursor
        .min(text_edit::char_len(current));
    let mut remaining =
        RECIPE_BOOK_SEARCH_MAX_LENGTH.saturating_sub(recipe_book_search_len(current));
    for ch in text.chars().filter(|ch| is_recipe_book_search_char(*ch)) {
        let len = ch.len_utf16();
        if len > remaining {
            break;
        }
        let insert_at = text_edit::byte_index(current, input.recipe_book_search_cursor);
        current.insert(insert_at, ch);
        input.recipe_book_search_cursor += 1;
        remaining -= len;
    }
    input.recipe_book_search_selection = input.recipe_book_search_cursor;
}

fn set_recipe_book_search_cursor(input: &mut ClientInputState, cursor: usize) {
    input.recipe_book_search_cursor =
        cursor.min(text_edit::char_len(&input.recipe_book_search_text));
    input.recipe_book_search_selection = input.recipe_book_search_cursor;
}

fn select_recipe_book_search_text(input: &mut ClientInputState) {
    input.recipe_book_search_selection = 0;
    input.recipe_book_search_cursor = text_edit::char_len(&input.recipe_book_search_text);
}

fn delete_recipe_book_search_selection(input: &mut ClientInputState) -> bool {
    if input.recipe_book_search_selection == input.recipe_book_search_cursor {
        return false;
    }
    let start = input
        .recipe_book_search_selection
        .min(input.recipe_book_search_cursor);
    let end = input
        .recipe_book_search_selection
        .max(input.recipe_book_search_cursor);
    let start_byte = text_edit::byte_index(&input.recipe_book_search_text, start);
    let end_byte = text_edit::byte_index(&input.recipe_book_search_text, end);
    input
        .recipe_book_search_text
        .replace_range(start_byte..end_byte, "");
    input.recipe_book_search_cursor = start;
    input.recipe_book_search_selection = start;
    true
}

fn is_recipe_book_search_char(ch: char) -> bool {
    ch != '\u{a7}' && ch >= ' ' && ch != '\u{7f}'
}

fn recipe_book_search_len(text: &str) -> usize {
    text.encode_utf16().count()
}

fn remove_recipe_book_search_char_before_cursor(current: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let start = text_edit::byte_index(current, *cursor - 1);
    let end = text_edit::byte_index(current, *cursor);
    current.replace_range(start..end, "");
    *cursor -= 1;
}

fn remove_recipe_book_search_char_at_cursor(current: &mut String, cursor: usize) {
    if cursor >= text_edit::char_len(current) {
        return;
    }
    let start = text_edit::byte_index(current, cursor);
    let end = text_edit::byte_index(current, cursor + 1);
    current.replace_range(start..end, "");
}

fn sync_anvil_rename_input(
    input: &mut ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) {
    let next = anvil_rename_input_signature(world);
    if input.anvil_rename_input != next {
        input.anvil_rename_input = next;
        input.anvil_rename_hover_name = anvil_initial_rename_text(world, item_runtime);
        input
            .anvil_rename_text
            .clone_from(&input.anvil_rename_hover_name);
        input.anvil_rename_cursor = anvil_rename_char_len(&input.anvil_rename_text);
        input.anvil_rename_selection = input.anvil_rename_cursor;
    }
}

fn anvil_initial_rename_text(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> String {
    let Some(input) = anvil_input_slot_item(world) else {
        return String::new();
    };
    item_runtime
        .and_then(|runtime| runtime.tooltip_lines_for_stack(input))
        .and_then(|lines| lines.into_iter().next())
        .map(|line| line.text)
        .or_else(|| anvil_component_hover_name(input))
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
        .map(|book| book.title.as_str())
        .filter(|title| !title.trim().is_empty())
    {
        return Some(title.to_string());
    }
    stack.component_patch.item_name.clone()
}

fn anvil_screen_is_open(world: &WorldStore) -> bool {
    world
        .inventory()
        .open_container
        .as_ref()
        .is_some_and(|container| container.menu_type_id == Some(ANVIL_MENU_TYPE_ID))
}

fn anvil_rename_input_signature(world: &WorldStore) -> Option<AnvilRenameInputSignature> {
    let container = world.inventory().open_container.as_ref()?;
    if container.menu_type_id != Some(ANVIL_MENU_TYPE_ID) {
        return None;
    }
    let item = container
        .slots
        .iter()
        .find(|slot| slot.slot == 0)
        .map(|slot| &slot.item)?;
    if item_stack_is_empty(item) {
        return None;
    }
    Some(AnvilRenameInputSignature {
        container_id: container.container_id,
        item: item.clone(),
    })
}

fn anvil_input_slot_item(world: &WorldStore) -> Option<&ItemStackSummary> {
    let container = world.inventory().open_container.as_ref()?;
    if container.menu_type_id != Some(ANVIL_MENU_TYPE_ID) {
        return None;
    }
    let item = container
        .slots
        .iter()
        .find(|slot| slot.slot == 0)
        .map(|slot| &slot.item)?;
    (!item_stack_is_empty(item)).then_some(item)
}

fn insert_anvil_rename_text(input: &mut ClientInputState, text: &str) {
    delete_anvil_rename_selection(input);
    let current = &mut input.anvil_rename_text;
    input.anvil_rename_cursor = input
        .anvil_rename_cursor
        .min(anvil_rename_char_len(current));
    let mut remaining = ANVIL_RENAME_MAX_LENGTH.saturating_sub(anvil_rename_len(current));
    for ch in text.chars().filter(|ch| is_anvil_rename_char(*ch)) {
        let len = ch.len_utf16();
        if len > remaining {
            break;
        }
        let insert_at = anvil_rename_byte_index(current, input.anvil_rename_cursor);
        current.insert(insert_at, ch);
        input.anvil_rename_cursor += 1;
        remaining -= len;
    }
    input.anvil_rename_selection = input.anvil_rename_cursor;
}

fn set_anvil_rename_cursor(input: &mut ClientInputState, cursor: usize) {
    input.anvil_rename_cursor = cursor.min(anvil_rename_char_len(&input.anvil_rename_text));
    input.anvil_rename_selection = input.anvil_rename_cursor;
}

fn select_anvil_rename_text(input: &mut ClientInputState) {
    input.anvil_rename_selection = 0;
    input.anvil_rename_cursor = anvil_rename_char_len(&input.anvil_rename_text);
}

fn delete_anvil_rename_selection(input: &mut ClientInputState) -> bool {
    if input.anvil_rename_selection == input.anvil_rename_cursor {
        return false;
    }
    let start = input.anvil_rename_selection.min(input.anvil_rename_cursor);
    let end = input.anvil_rename_selection.max(input.anvil_rename_cursor);
    let start_byte = anvil_rename_byte_index(&input.anvil_rename_text, start);
    let end_byte = anvil_rename_byte_index(&input.anvil_rename_text, end);
    input
        .anvil_rename_text
        .replace_range(start_byte..end_byte, "");
    input.anvil_rename_cursor = start;
    input.anvil_rename_selection = start;
    true
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
    if let Some(signature) = &input.anvil_rename_input {
        if signature.item.component_patch.custom_name.is_none()
            && input.anvil_rename_text == input.anvil_rename_hover_name
        {
            return String::new();
        }
    }
    input.anvil_rename_text.clone()
}

fn is_anvil_rename_char(ch: char) -> bool {
    ch != '\u{a7}' && ch >= ' ' && ch != '\u{7f}'
}

fn anvil_rename_len(text: &str) -> usize {
    text.encode_utf16().count()
}

fn anvil_rename_char_len(text: &str) -> usize {
    text.chars().count()
}

fn anvil_rename_byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(index, _)| index)
}

fn remove_anvil_rename_char_before_cursor(current: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let start = anvil_rename_byte_index(current, *cursor - 1);
    let end = anvil_rename_byte_index(current, *cursor);
    current.replace_range(start..end, "");
    *cursor -= 1;
}

fn remove_anvil_rename_char_at_cursor(current: &mut String, cursor: usize) {
    if cursor >= anvil_rename_char_len(current) {
        return;
    }
    let start = anvil_rename_byte_index(current, cursor);
    let end = anvil_rename_byte_index(current, cursor + 1);
    current.replace_range(start..end, "");
}

fn local_inventory_swap_button_num(code: KeyCode) -> Option<i8> {
    if code == KeyCode::KeyF {
        return Some(40);
    }
    hotbar_slot_for_key(code).map(|slot| slot as i8)
}

fn handle_inventory_swap_key(
    input: &ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    button_num: i8,
) {
    if !inventory_cursor_is_empty(world) {
        return;
    }
    let Some(slot_num) = input.inventory_hovered_slot else {
        return;
    };
    let request = ContainerClickSlotRequest {
        slot_num,
        button_num,
        input: ContainerInput::Swap,
    };
    if world.local_inventory_is_open() {
        let Ok(click) = world.apply_local_container_click_slot(request) else {
            return;
        };
        if click.changed_slots.is_empty() {
            return;
        }
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
    if !input.focused || inventory_screen_layout(world).is_none() {
        return false;
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
    if let Some(slot) = inventory_screen_hovered_slot(world, cursor_position, surface_size) {
        handle_bundle_slot_mouse_scroll(
            input,
            world,
            counters,
            net_commands,
            i32::from(slot),
            delta,
        );
    }
    true
}

fn maybe_scroll_stonecutter_recipes(
    input: &mut ClientInputState,
    world: &WorldStore,
    delta: &MouseScrollDelta,
) -> bool {
    sync_stonecutter_recipe_scroll(input, world);
    let Some(max_scroll_row) = stonecutter_recipe_max_scroll_row(world) else {
        return false;
    };
    if max_scroll_row <= 0 {
        return false;
    }
    if let Some(wheel) = inventory_wheel_steps_from_scroll(input, delta) {
        input.stonecutter_recipe_scroll_row =
            (input.stonecutter_recipe_scroll_row - wheel).clamp(0, max_scroll_row);
    }
    true
}

fn maybe_scroll_loom_patterns(
    input: &mut ClientInputState,
    world: &WorldStore,
    delta: &MouseScrollDelta,
) -> bool {
    sync_loom_pattern_scroll(input, world);
    let Some(max_scroll_row) = loom_pattern_max_scroll_row(world) else {
        return false;
    };
    if max_scroll_row <= 0 {
        return false;
    }
    if let Some(wheel) = inventory_wheel_steps_from_scroll(input, delta) {
        input.loom_pattern_scroll_row =
            (input.loom_pattern_scroll_row - wheel).clamp(0, max_scroll_row);
    }
    true
}

fn maybe_start_stonecutter_recipe_scroll_drag(
    input: &mut ClientInputState,
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    sync_stonecutter_recipe_scroll(input, world);
    if stonecutter_recipe_max_scroll_row(world).unwrap_or_default() <= 0 {
        return false;
    }
    if !stonecutter_scroller_at_position(world, cursor_position, surface_size) {
        return false;
    }
    input.stonecutter_recipe_scrolling = true;
    true
}

fn maybe_start_loom_pattern_scroll_drag(
    input: &mut ClientInputState,
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    sync_loom_pattern_scroll(input, world);
    if loom_pattern_max_scroll_row(world).unwrap_or_default() <= 0 {
        return false;
    }
    if !loom_scroller_at_position(world, cursor_position, surface_size) {
        return false;
    }
    input.loom_pattern_scrolling = true;
    true
}

fn maybe_start_merchant_trade_scroll_drag(
    input: &mut ClientInputState,
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    if !merchant_offers_can_scroll(world) {
        return false;
    }
    if !merchant_scroller_track_at_position(world, cursor_position, surface_size) {
        return false;
    }
    input.merchant_trade_scrolling = true;
    true
}

fn maybe_scroll_merchant_trades(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    delta: &MouseScrollDelta,
) -> bool {
    if !matches!(
        inventory_screen_layout(world).map(|layout| layout.background),
        Some(InventoryScreenBackground::Merchant)
    ) {
        return false;
    }
    if !merchant_offers_can_scroll(world) {
        return false;
    }
    if let Some(wheel) = inventory_wheel_steps_from_scroll(input, delta) {
        world.scroll_local_merchant_offers(-wheel);
    }
    true
}

fn merchant_offers_can_scroll(world: &WorldStore) -> bool {
    world
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.merchant_offers.as_ref())
        .is_some_and(|offers| offers.offers.len() > MERCHANT_TRADE_BUTTON_COUNT as usize)
}

fn merchant_offer_count_and_scroll_offset(world: &WorldStore) -> Option<(usize, i32)> {
    world
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.merchant_offers.as_ref())
        .map(|offers| (offers.offers.len(), offers.local_scroll_offset))
}

fn merchant_max_scroll_offset(offer_count: usize) -> i32 {
    i32::try_from(offer_count)
        .unwrap_or_default()
        .saturating_sub(MERCHANT_TRADE_BUTTON_COUNT)
        .max(0)
}

fn inventory_wheel_steps_from_scroll(
    input: &mut ClientInputState,
    delta: &MouseScrollDelta,
) -> Option<i32> {
    let (x, y) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (f64::from(*x), f64::from(*y)),
        MouseScrollDelta::PixelDelta(pos) => (pos.x, pos.y),
    };

    if input.scroll_accumulated_x != 0.0
        && scroll_signum(x) != scroll_signum(input.scroll_accumulated_x)
    {
        input.scroll_accumulated_x = 0.0;
    }
    if input.scroll_accumulated_y != 0.0
        && scroll_signum(y) != scroll_signum(input.scroll_accumulated_y)
    {
        input.scroll_accumulated_y = 0.0;
    }

    input.scroll_accumulated_x += x;
    input.scroll_accumulated_y += y;
    let wheel_x = input.scroll_accumulated_x as i32;
    let wheel_y = input.scroll_accumulated_y as i32;
    if wheel_x == 0 && wheel_y == 0 {
        return None;
    }

    input.scroll_accumulated_x -= f64::from(wheel_x);
    input.scroll_accumulated_y -= f64::from(wheel_y);
    Some(if wheel_y == 0 { -wheel_x } else { wheel_y })
}

fn scroll_signum(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}

fn inventory_screen_hovered_slot(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<i16> {
    match inventory_screen_click_target(world, cursor_position, surface_size) {
        Some(InventoryClickTarget::Slot(slot)) => Some(slot),
        _ => None,
    }
}

fn inventory_screen_cursor_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<(i32, i32)> {
    let layout = inventory_screen_layout(world)?;
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    (x.is_finite() && y.is_finite()).then(|| (x.floor() as i32, y.floor() as i32))
}

fn recipe_book_button_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<RecipeBookType> {
    let layout = inventory_screen_layout(world)?;
    let (button_x, button_y) = recipe_book_button_position(layout.background)?;
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let main_offset_x = recipe_book_main_gui_offset(world, layout.background);
    let x = cursor.x - origin_x - f64::from(main_offset_x + button_x);
    let y = cursor.y - origin_y - f64::from(button_y);
    if x >= 0.0
        && x < f64::from(RECIPE_BOOK_BUTTON_WIDTH)
        && y >= 0.0
        && y < f64::from(RECIPE_BOOK_BUTTON_HEIGHT)
    {
        return recipe_book_type_for_background(layout.background);
    }
    None
}

fn recipe_book_filter_button_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<RecipeBookType> {
    let layout = inventory_screen_layout(world)?;
    let book_type = recipe_book_type_for_background(layout.background)?;
    if recipe_book_main_gui_offset(world, layout.background) == 0 {
        return None;
    }
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x - f64::from(RECIPE_BOOK_FILTER_BUTTON_X);
    let y = cursor.y - origin_y - f64::from(RECIPE_BOOK_FILTER_BUTTON_Y);
    if x >= 0.0
        && x < f64::from(RECIPE_BOOK_FILTER_BUTTON_WIDTH)
        && y >= 0.0
        && y < f64::from(RECIPE_BOOK_FILTER_BUTTON_HEIGHT)
    {
        return Some(book_type);
    }
    None
}

fn recipe_book_search_box_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(layout) = inventory_screen_layout(world) else {
        return false;
    };
    if recipe_book_type_for_background(layout.background).is_none()
        || recipe_book_main_gui_offset(world, layout.background) == 0
    {
        return false;
    }
    let Some(cursor) = cursor_position else {
        return false;
    };
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x - f64::from(RECIPE_BOOK_SEARCH_BOX_X);
    let y = cursor.y - origin_y - f64::from(RECIPE_BOOK_SEARCH_BOX_Y);
    x >= 0.0
        && x < f64::from(RECIPE_BOOK_SEARCH_BOX_WIDTH)
        && y >= 0.0
        && y < f64::from(RECIPE_BOOK_SEARCH_BOX_HEIGHT)
}

fn recipe_book_tab_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<(RecipeBookType, usize)> {
    let layout = inventory_screen_layout(world)?;
    let book_type = recipe_book_type_for_background(layout.background)?;
    if recipe_book_main_gui_offset(world, layout.background) == 0 {
        return None;
    }
    let visible_tabs = recipe_book_visible_tab_indices(world, layout.background);
    if visible_tabs.is_empty() {
        return None;
    }
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    for (visible_index, tab_index) in visible_tabs.into_iter().enumerate() {
        let tab_y = RECIPE_BOOK_TAB_Y + RECIPE_BOOK_TAB_STRIDE_Y * visible_index as i32;
        if x >= f64::from(RECIPE_BOOK_TAB_X)
            && x < f64::from(RECIPE_BOOK_TAB_X + RECIPE_BOOK_TAB_WIDTH)
            && y >= f64::from(tab_y)
            && y < f64::from(tab_y + RECIPE_BOOK_TAB_HEIGHT)
        {
            return Some((book_type, tab_index));
        }
    }
    None
}

fn recipe_book_page_button_at_position(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<(RecipeBookType, RecipeBookPageTurn, usize)> {
    let layout = inventory_screen_layout(world)?;
    let book_type = recipe_book_type_for_background(layout.background)?;
    if recipe_book_main_gui_offset(world, layout.background) == 0 {
        return None;
    }
    let collection_count = recipe_book_collections_for_background(
        input,
        world,
        item_runtime,
        layout.background,
        book_type,
    )?
    .len();
    let page_count = recipe_book_page_count(collection_count);
    if page_count <= 1 {
        return None;
    }
    let current_page = clamped_recipe_book_page(
        selected_recipe_book_page_index(input, book_type),
        collection_count,
    );
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    if current_page + 1 < page_count
        && recipe_book_page_button_contains(
            x,
            y,
            RECIPE_BOOK_PAGE_FORWARD_BUTTON_X,
            RECIPE_BOOK_PAGE_BUTTON_Y,
        )
    {
        return Some((book_type, RecipeBookPageTurn::Next, page_count));
    }
    if current_page > 0
        && recipe_book_page_button_contains(
            x,
            y,
            RECIPE_BOOK_PAGE_BACKWARD_BUTTON_X,
            RECIPE_BOOK_PAGE_BUTTON_Y,
        )
    {
        return Some((book_type, RecipeBookPageTurn::Previous, page_count));
    }
    None
}

fn recipe_book_recipe_button_at_position(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<i32> {
    let layout = inventory_screen_layout(world)?;
    let book_type = recipe_book_type_for_background(layout.background)?;
    if recipe_book_main_gui_offset(world, layout.background) == 0 {
        return None;
    }
    let collections = recipe_book_collections_for_background(
        input,
        world,
        item_runtime,
        layout.background,
        book_type,
    )?;
    let page = clamped_recipe_book_page(
        selected_recipe_book_page_index(input, book_type),
        collections.len(),
    );
    let visible_index =
        recipe_book_recipe_button_index_at_position(cursor_position, surface_size, &layout)?;
    let collection_index = page * RECIPE_BOOK_ITEMS_PER_PAGE + visible_index;
    let slot_select_index = recipe_book_slot_select_index(world, 0.0);
    collections
        .get(collection_index)
        .and_then(|collection| collection.recipe_index_at_slot_select_index(slot_select_index))
}

fn recipe_book_collections_for_background<'a>(
    input: &ClientInputState,
    world: &'a WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    book_type: RecipeBookType,
) -> Option<Vec<RecipeBookUiCollection<'a>>> {
    let tab_count = recipe_book_tab_count_for_background(background)?;
    if tab_count == 0 {
        return None;
    }
    let selected_tab = selected_recipe_book_tab_index(input, book_type).min(tab_count - 1);
    let only_craftable = recipe_book_type_settings(world, book_type).filtering;
    if let Some(grid) = recipe_book_crafting_grid_for_background(background) {
        return Some(crafting_recipe_book_collections(
            world,
            grid,
            selected_tab,
            only_craftable,
            &input.recipe_book_search_text,
            item_runtime,
        ));
    }
    recipe_book_furnace_family_for_background(background).map(|family| {
        furnace_recipe_book_collections(
            world,
            family,
            selected_tab,
            only_craftable,
            &input.recipe_book_search_text,
            item_runtime,
        )
    })
}

fn recipe_book_recipe_button_index_at_position(
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    layout: &InventoryScreenLayout,
) -> Option<usize> {
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, layout);
    let x = cursor.x - origin_x - f64::from(RECIPE_BOOK_RECIPE_BUTTON_X);
    let y = cursor.y - origin_y - f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y);
    if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 {
        return None;
    }
    let column = (x / f64::from(RECIPE_BOOK_RECIPE_BUTTON_SIZE)).floor();
    let row = (y / f64::from(RECIPE_BOOK_RECIPE_BUTTON_SIZE)).floor();
    if column < 0.0
        || column >= RECIPE_BOOK_RECIPE_BUTTON_COLUMNS as f64
        || row < 0.0
        || row >= (RECIPE_BOOK_ITEMS_PER_PAGE / RECIPE_BOOK_RECIPE_BUTTON_COLUMNS) as f64
    {
        return None;
    }
    let local_x = x - column * f64::from(RECIPE_BOOK_RECIPE_BUTTON_SIZE);
    let local_y = y - row * f64::from(RECIPE_BOOK_RECIPE_BUTTON_SIZE);
    if local_x >= f64::from(RECIPE_BOOK_RECIPE_BUTTON_SIZE)
        || local_y >= f64::from(RECIPE_BOOK_RECIPE_BUTTON_SIZE)
    {
        return None;
    }
    let column = column as usize;
    let row = row as usize;
    Some(row * RECIPE_BOOK_RECIPE_BUTTON_COLUMNS + column)
}

fn recipe_book_page_button_contains(x: f64, y: f64, button_x: i32, button_y: i32) -> bool {
    x >= f64::from(button_x)
        && x < f64::from(button_x + RECIPE_BOOK_PAGE_BUTTON_WIDTH)
        && y >= f64::from(button_y)
        && y < f64::from(button_y + RECIPE_BOOK_PAGE_BUTTON_HEIGHT)
}

fn recipe_book_crafting_grid_for_background(
    background: InventoryScreenBackground,
) -> Option<RecipeBookCraftingGrid> {
    match background {
        InventoryScreenBackground::LocalInventory => Some(RecipeBookCraftingGrid {
            width: 2,
            height: 2,
        }),
        InventoryScreenBackground::CraftingTable => Some(RecipeBookCraftingGrid {
            width: 3,
            height: 3,
        }),
        _ => None,
    }
}

fn recipe_book_furnace_family_for_background(
    background: InventoryScreenBackground,
) -> Option<RecipeBookFurnaceFamily> {
    match background {
        InventoryScreenBackground::Furnace => Some(RecipeBookFurnaceFamily::Furnace),
        InventoryScreenBackground::BlastFurnace => Some(RecipeBookFurnaceFamily::BlastFurnace),
        InventoryScreenBackground::Smoker => Some(RecipeBookFurnaceFamily::Smoker),
        _ => None,
    }
}

fn enchantment_button_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<i32> {
    let layout = inventory_screen_layout(world)?;
    if layout.background != InventoryScreenBackground::EnchantmentTable {
        return None;
    }
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    for button_id in 0..ENCHANTMENT_BUTTON_COUNT {
        let button_y = ENCHANTMENT_BUTTON_Y + button_id * ENCHANTMENT_BUTTON_SPACING;
        if x >= f64::from(ENCHANTMENT_BUTTON_X)
            && x < f64::from(ENCHANTMENT_BUTTON_X + ENCHANTMENT_BUTTON_WIDTH)
            && y >= f64::from(button_y)
            && y < f64::from(button_y + ENCHANTMENT_BUTTON_HEIGHT)
        {
            return Some(button_id);
        }
    }
    None
}

fn loom_click_target_at_position(
    world: &WorldStore,
    start_index: i32,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<LoomClickTarget> {
    let selectable_count = loom_selectable_pattern_count(world)?;
    if selectable_count <= 0 {
        return None;
    }
    let layout = inventory_screen_layout(world)?;
    if layout.background != InventoryScreenBackground::Loom {
        return None;
    }
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x - f64::from(LOOM_PATTERN_BUTTON_X);
    let y = cursor.y - origin_y - f64::from(LOOM_PATTERN_BUTTON_Y);
    if x < 0.0 || y < 0.0 {
        return None;
    }
    let column = (x / f64::from(LOOM_PATTERN_BUTTON_SIZE)) as i32;
    let row = (y / f64::from(LOOM_PATTERN_BUTTON_SIZE)) as i32;
    if column >= LOOM_PATTERN_BUTTON_COLUMNS || row >= LOOM_PATTERN_BUTTON_ROWS {
        return None;
    }
    let button_id = start_index + row * LOOM_PATTERN_BUTTON_COLUMNS + column;
    (button_id < selectable_count).then_some(LoomClickTarget::Pattern(button_id))
}

fn merchant_trade_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<i32> {
    let layout = inventory_screen_layout(world)?;
    if layout.background != InventoryScreenBackground::Merchant {
        return None;
    }
    let (offer_count, scroll_offset) = world
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.merchant_offers.as_ref())
        .map(|offers| (offers.offers.len(), offers.local_scroll_offset))
        .unwrap_or_default();
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    if x < f64::from(MERCHANT_TRADE_BUTTON_X)
        || x >= f64::from(MERCHANT_TRADE_BUTTON_X + MERCHANT_TRADE_BUTTON_WIDTH)
    {
        return None;
    }
    for row in 0..MERCHANT_TRADE_BUTTON_COUNT {
        let button_y = MERCHANT_TRADE_BUTTON_Y + row * MERCHANT_TRADE_BUTTON_HEIGHT;
        let item = scroll_offset + row;
        if y >= f64::from(button_y)
            && y < f64::from(button_y + MERCHANT_TRADE_BUTTON_HEIGHT)
            && usize::try_from(item).is_ok_and(|item| item < offer_count)
        {
            return Some(item);
        }
    }
    None
}

fn merchant_scroller_track_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    if !merchant_offers_can_scroll(world) {
        return false;
    }
    let Some(layout) = inventory_screen_layout(world) else {
        return false;
    };
    if layout.background != InventoryScreenBackground::Merchant {
        return false;
    }
    let Some(cursor) = cursor_position else {
        return false;
    };
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    x > f64::from(MERCHANT_SCROLLER_X)
        && x < f64::from(MERCHANT_SCROLLER_X + MERCHANT_SCROLLER_WIDTH)
        && y > f64::from(MERCHANT_SCROLLER_Y)
        && y <= f64::from(MERCHANT_SCROLLER_Y + MERCHANT_SCROLLER_FULL_HEIGHT + 1)
}

fn loom_scroller_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    if loom_pattern_max_scroll_row(world).unwrap_or_default() <= 0 {
        return false;
    }
    let Some(layout) = inventory_screen_layout(world) else {
        return false;
    };
    if layout.background != InventoryScreenBackground::Loom {
        return false;
    }
    let Some(cursor) = cursor_position else {
        return false;
    };
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    x >= f64::from(LOOM_SCROLLER_X)
        && x < f64::from(LOOM_SCROLLER_X + LOOM_SCROLLER_WIDTH)
        && y >= f64::from(LOOM_SCROLLER_CLICK_Y)
        && y < f64::from(LOOM_SCROLLER_CLICK_Y + LOOM_SCROLLER_FULL_HEIGHT)
}

fn update_merchant_trade_scroll_from_cursor(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some((offer_count, current_scroll_offset)) = merchant_offer_count_and_scroll_offset(world)
    else {
        input.merchant_trade_scrolling = false;
        return false;
    };
    let max_scroll_offset = merchant_max_scroll_offset(offer_count);
    if max_scroll_offset <= 0 {
        input.merchant_trade_scrolling = false;
        return false;
    }
    let Some(layout) = inventory_screen_layout(world) else {
        input.merchant_trade_scrolling = false;
        return false;
    };
    if layout.background != InventoryScreenBackground::Merchant {
        input.merchant_trade_scrolling = false;
        return false;
    }
    let Some(cursor) = cursor_position else {
        return false;
    };
    let (_, origin_y) = inventory_screen_origin(surface_size, &layout);
    let y = cursor.y - origin_y;
    let drag_range = f64::from(MERCHANT_SCROLLER_FULL_HEIGHT - MERCHANT_SCROLLER_HEIGHT);
    let scroll_offset =
        (((y - f64::from(MERCHANT_SCROLLER_Y) - f64::from(MERCHANT_SCROLLER_HEIGHT) * 0.5)
            / drag_range)
            * f64::from(max_scroll_offset)
            + 0.5) as i32;
    let scroll_offset = scroll_offset.clamp(0, max_scroll_offset);
    if scroll_offset != current_scroll_offset {
        world.scroll_local_merchant_offers(scroll_offset - current_scroll_offset);
    }
    true
}

fn update_loom_pattern_scroll_from_cursor(
    input: &mut ClientInputState,
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    sync_loom_pattern_scroll(input, world);
    let Some(max_scroll_row) = loom_pattern_max_scroll_row(world) else {
        input.loom_pattern_scrolling = false;
        return false;
    };
    if max_scroll_row <= 0 {
        input.loom_pattern_scrolling = false;
        return false;
    }
    let Some(layout) = inventory_screen_layout(world) else {
        input.loom_pattern_scrolling = false;
        return false;
    };
    if layout.background != InventoryScreenBackground::Loom {
        input.loom_pattern_scrolling = false;
        return false;
    }
    let Some(cursor) = cursor_position else {
        return false;
    };
    let (_, origin_y) = inventory_screen_origin(surface_size, &layout);
    let y = cursor.y - origin_y;
    let drag_range = f64::from(LOOM_SCROLLER_FULL_HEIGHT - LOOM_SCROLLER_HEIGHT);
    let scroll_offs =
        ((y - f64::from(LOOM_SCROLLER_DRAG_Y) - f64::from(LOOM_SCROLLER_HEIGHT) * 0.5)
            / drag_range)
            .clamp(0.0, 1.0);
    input.loom_pattern_scroll_row = (scroll_offs * f64::from(max_scroll_row) + 0.5) as i32;
    true
}

fn lectern_button_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<LecternClickTarget> {
    let layout = inventory_screen_layout(world)?;
    if layout.background != InventoryScreenBackground::Lectern {
        return None;
    }
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    if y >= f64::from(LECTERN_MENU_BUTTON_Y)
        && y < f64::from(LECTERN_MENU_BUTTON_Y + LECTERN_MENU_BUTTON_HEIGHT)
    {
        if x >= f64::from(LECTERN_MENU_DONE_BUTTON_X)
            && x < f64::from(LECTERN_MENU_DONE_BUTTON_X + LECTERN_MENU_BUTTON_WIDTH)
        {
            return Some(LecternClickTarget::Done);
        }
        if x >= f64::from(LECTERN_MENU_TAKE_BOOK_BUTTON_X)
            && x < f64::from(LECTERN_MENU_TAKE_BOOK_BUTTON_X + LECTERN_MENU_BUTTON_WIDTH)
        {
            return Some(LecternClickTarget::MenuButton(LECTERN_BUTTON_TAKE_BOOK));
        }
        return None;
    }
    if y < f64::from(LECTERN_PAGE_BUTTON_Y)
        || y >= f64::from(LECTERN_PAGE_BUTTON_Y + LECTERN_PAGE_BUTTON_HEIGHT)
    {
        return None;
    }
    if x >= f64::from(LECTERN_PAGE_BACK_BUTTON_X)
        && x < f64::from(LECTERN_PAGE_BACK_BUTTON_X + LECTERN_PAGE_BUTTON_WIDTH)
    {
        return Some(LecternClickTarget::MenuButton(LECTERN_BUTTON_PREV_PAGE));
    }
    if x >= f64::from(LECTERN_PAGE_FORWARD_BUTTON_X)
        && x < f64::from(LECTERN_PAGE_FORWARD_BUTTON_X + LECTERN_PAGE_BUTTON_WIDTH)
    {
        return Some(LecternClickTarget::MenuButton(LECTERN_BUTTON_NEXT_PAGE));
    }
    None
}

fn beacon_button_at_position(
    input: &ClientInputState,
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<BeaconClickTarget> {
    let layout = inventory_screen_layout(world)?;
    if layout.background != InventoryScreenBackground::Beacon {
        return None;
    }
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;

    if y >= f64::from(BEACON_ACTION_BUTTON_Y)
        && y < f64::from(BEACON_ACTION_BUTTON_Y + BEACON_ACTION_BUTTON_SIZE)
    {
        if x >= f64::from(BEACON_CONFIRM_BUTTON_X)
            && x < f64::from(BEACON_CONFIRM_BUTTON_X + BEACON_ACTION_BUTTON_SIZE)
        {
            return Some(BeaconClickTarget::Confirm);
        }
        if x >= f64::from(BEACON_CANCEL_BUTTON_X)
            && x < f64::from(BEACON_CANCEL_BUTTON_X + BEACON_ACTION_BUTTON_SIZE)
        {
            return Some(BeaconClickTarget::Cancel);
        }
        return None;
    }

    let levels = beacon_levels(world);
    for button in beacon_effect_buttons(input) {
        if button.tier >= levels {
            continue;
        }
        if x >= f64::from(button.x)
            && x < f64::from(button.x + BEACON_EFFECT_BUTTON_SIZE)
            && y >= f64::from(button.y)
            && y < f64::from(button.y + BEACON_EFFECT_BUTTON_SIZE)
        {
            return Some(BeaconClickTarget::Effect {
                primary: button.primary,
                effect_id: button.effect_id,
            });
        }
    }
    None
}

fn beacon_effect_buttons(input: &ClientInputState) -> Vec<BeaconEffectButton> {
    let mut buttons = Vec::with_capacity(7);
    for (tier, effects) in BEACON_PRIMARY_EFFECT_ROWS.iter().enumerate() {
        let total_width =
            effects.len() as i32 * BEACON_EFFECT_BUTTON_SIZE + (effects.len() as i32 - 1) * 2;
        for (column, effect_id) in effects.iter().enumerate() {
            buttons.push(BeaconEffectButton {
                primary: true,
                tier: i16::try_from(tier).unwrap_or_default(),
                effect_id: *effect_id,
                x: BEACON_PRIMARY_EFFECT_CENTER_X
                    + i32::try_from(column).unwrap_or_default() * BEACON_EFFECT_BUTTON_SPACING
                    - total_width / 2,
                y: BEACON_PRIMARY_EFFECT_Y
                    + i32::try_from(tier).unwrap_or_default() * BEACON_PRIMARY_EFFECT_ROW_SPACING,
            });
        }
    }

    let count = BEACON_SECONDARY_EFFECTS.len() as i32 + 1;
    let total_width = count * BEACON_EFFECT_BUTTON_SIZE + (count - 1) * 2;
    for (column, effect_id) in BEACON_SECONDARY_EFFECTS.iter().enumerate() {
        buttons.push(BeaconEffectButton {
            primary: false,
            tier: 3,
            effect_id: *effect_id,
            x: BEACON_SECONDARY_EFFECT_CENTER_X
                + i32::try_from(column).unwrap_or_default() * BEACON_EFFECT_BUTTON_SPACING
                - total_width / 2,
            y: BEACON_SECONDARY_EFFECT_Y,
        });
    }
    if let Some(effect_id) = input.beacon_primary_effect {
        buttons.push(BeaconEffectButton {
            primary: false,
            tier: 3,
            effect_id,
            x: BEACON_SECONDARY_EFFECT_CENTER_X
                + i32::try_from(BEACON_SECONDARY_EFFECTS.len()).unwrap_or_default()
                    * BEACON_EFFECT_BUTTON_SPACING
                - total_width / 2,
            y: BEACON_SECONDARY_EFFECT_Y,
        });
    }
    buttons
}

fn stonecutter_recipe_button_at_position(
    world: &WorldStore,
    start_index: i32,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<i32> {
    if stonecutter_visible_recipe_count(world)? <= 0 {
        return None;
    }
    let layout = inventory_screen_layout(world)?;
    if layout.background != InventoryScreenBackground::Stonecutter {
        return None;
    }
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x - f64::from(STONECUTTER_RECIPE_BUTTON_X);
    let y = cursor.y - origin_y - f64::from(STONECUTTER_RECIPE_BUTTON_Y);
    if x < 0.0 || y < 0.0 {
        return None;
    }
    let column = (x / f64::from(STONECUTTER_RECIPE_BUTTON_WIDTH)) as i32;
    let row = (y / f64::from(STONECUTTER_RECIPE_BUTTON_HEIGHT)) as i32;
    if column >= STONECUTTER_RECIPE_BUTTON_COLUMNS || row >= STONECUTTER_RECIPE_BUTTON_ROWS {
        return None;
    }
    let button_id = start_index + row * STONECUTTER_RECIPE_BUTTON_COLUMNS + column;
    Some(button_id)
}

fn stonecutter_scroller_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(layout) = inventory_screen_layout(world) else {
        return false;
    };
    if layout.background != InventoryScreenBackground::Stonecutter {
        return false;
    }
    let Some(cursor) = cursor_position else {
        return false;
    };
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    x >= f64::from(STONECUTTER_SCROLLER_X)
        && x < f64::from(STONECUTTER_SCROLLER_X + STONECUTTER_SCROLLER_WIDTH)
        && y >= f64::from(STONECUTTER_SCROLLER_CLICK_Y)
        && y < f64::from(STONECUTTER_SCROLLER_CLICK_Y + STONECUTTER_SCROLLER_FULL_HEIGHT)
}

fn update_stonecutter_recipe_scroll_from_cursor(
    input: &mut ClientInputState,
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    sync_stonecutter_recipe_scroll(input, world);
    let Some(max_scroll_row) = stonecutter_recipe_max_scroll_row(world) else {
        input.stonecutter_recipe_scrolling = false;
        return false;
    };
    if max_scroll_row <= 0 {
        input.stonecutter_recipe_scrolling = false;
        return false;
    }
    let Some(layout) = inventory_screen_layout(world) else {
        input.stonecutter_recipe_scrolling = false;
        return false;
    };
    if layout.background != InventoryScreenBackground::Stonecutter {
        input.stonecutter_recipe_scrolling = false;
        return false;
    }
    let Some(cursor) = cursor_position else {
        return false;
    };
    let (_, origin_y) = inventory_screen_origin(surface_size, &layout);
    let y = cursor.y - origin_y;
    let drag_range = f64::from(STONECUTTER_SCROLLER_FULL_HEIGHT - STONECUTTER_SCROLLER_HEIGHT);
    let scroll_offs = ((y
        - f64::from(STONECUTTER_SCROLLER_DRAG_Y)
        - f64::from(STONECUTTER_SCROLLER_HEIGHT) * 0.5)
        / drag_range)
        .clamp(0.0, 1.0);
    input.stonecutter_recipe_scroll_row = (scroll_offs * f64::from(max_scroll_row) + 0.5) as i32;
    true
}

fn stonecutter_visible_recipe_count(world: &WorldStore) -> Option<i32> {
    let input_item_id = stonecutter_input_item_id(world)?;
    Some(stonecutter_visible_recipe_count_for_input(
        world,
        input_item_id,
    ))
}

fn stonecutter_visible_recipe_count_for_input(world: &WorldStore, input_item_id: i32) -> i32 {
    world
        .recipes()
        .stonecutter_recipes
        .iter()
        .filter(|recipe| recipe.input.item_ids.contains(&input_item_id))
        .count() as i32
}

fn stonecutter_recipe_max_scroll_row(world: &WorldStore) -> Option<i32> {
    let visible_recipes = stonecutter_visible_recipe_count(world)?;
    Some((stonecutter_recipe_row_count(visible_recipes) - STONECUTTER_RECIPE_BUTTON_ROWS).max(0))
}

fn loom_pattern_max_scroll_row(world: &WorldStore) -> Option<i32> {
    let selectable_count = loom_selectable_pattern_count(world)?;
    Some((loom_pattern_row_count(selectable_count) - LOOM_PATTERN_BUTTON_ROWS).max(0))
}

fn loom_selectable_pattern_count(world: &WorldStore) -> Option<i32> {
    let container = world.inventory().open_container.as_ref()?;
    if container.menu_type_id != Some(LOOM_MENU_TYPE_ID) {
        return None;
    }
    if !inventory_slot_has_item(world, 0) || !inventory_slot_has_item(world, 1) {
        return Some(0);
    }
    if inventory_slot_has_item(world, 2) {
        Some(LOOM_PATTERN_ITEM_PATTERN_COUNT)
    } else {
        Some(LOOM_NO_ITEM_REQUIRED_PATTERN_COUNT)
    }
}

fn stonecutter_recipe_row_count(visible_recipes: i32) -> i32 {
    if visible_recipes <= 0 {
        0
    } else {
        (visible_recipes + STONECUTTER_RECIPE_BUTTON_COLUMNS - 1)
            / STONECUTTER_RECIPE_BUTTON_COLUMNS
    }
}

fn loom_pattern_row_count(selectable_count: i32) -> i32 {
    if selectable_count <= 0 {
        0
    } else {
        (selectable_count + LOOM_PATTERN_BUTTON_COLUMNS - 1) / LOOM_PATTERN_BUTTON_COLUMNS
    }
}

fn sync_stonecutter_recipe_scroll(input: &mut ClientInputState, world: &WorldStore) {
    let input_item_id = stonecutter_input_item_id(world);
    if input.stonecutter_recipe_scroll_input_item_id != input_item_id {
        input.stonecutter_recipe_scroll_input_item_id = input_item_id;
        input.stonecutter_recipe_scroll_row = 0;
        input.stonecutter_recipe_scrolling = false;
    }
    let max_scroll_row = stonecutter_recipe_max_scroll_row(world).unwrap_or_default();
    input.stonecutter_recipe_scroll_row =
        input.stonecutter_recipe_scroll_row.clamp(0, max_scroll_row);
}

fn sync_loom_pattern_scroll(input: &mut ClientInputState, world: &WorldStore) {
    let max_scroll_row = loom_pattern_max_scroll_row(world).unwrap_or_default();
    input.loom_pattern_scroll_row = input.loom_pattern_scroll_row.clamp(0, max_scroll_row);
    if max_scroll_row <= 0 {
        input.loom_pattern_scrolling = false;
    }
}

fn sync_loom_pattern_state(input: &mut ClientInputState, world: &WorldStore) {
    let Some(container) = world.inventory().open_container.as_ref() else {
        input.loom_pattern_selection_container_id = None;
        input.loom_pattern_selection_dirty = false;
        input.loom_selected_pattern_index = None;
        input.loom_pattern_scroll_row = 0;
        input.loom_pattern_scrolling = false;
        return;
    };
    if container.menu_type_id != Some(LOOM_MENU_TYPE_ID) {
        input.loom_pattern_selection_container_id = None;
        input.loom_pattern_selection_dirty = false;
        input.loom_selected_pattern_index = None;
        input.loom_pattern_scroll_row = 0;
        input.loom_pattern_scrolling = false;
        return;
    }

    if input.loom_pattern_selection_container_id != Some(container.container_id) {
        input.loom_pattern_selection_container_id = Some(container.container_id);
        input.loom_pattern_selection_dirty = false;
    }
    if !input.loom_pattern_selection_dirty {
        input.loom_selected_pattern_index = world
            .open_container_data_value(LOOM_SELECTED_PATTERN_DATA_ID)
            .and_then(|value| (value >= 0).then_some(i32::from(value)));
    }
    let selectable_count = loom_selectable_pattern_count(world).unwrap_or_default();
    if input
        .loom_selected_pattern_index
        .is_some_and(|index| index < 0 || index >= selectable_count)
    {
        input.loom_selected_pattern_index = None;
        input.loom_pattern_selection_dirty = false;
    }
    sync_loom_pattern_scroll(input, world);
}

pub(crate) fn sync_stonecutter_recipe_scroll_state(
    input: &mut ClientInputState,
    world: &WorldStore,
) {
    sync_stonecutter_recipe_scroll(input, world);
}

pub(crate) fn sync_loom_pattern_state_for_hud(input: &mut ClientInputState, world: &WorldStore) {
    sync_loom_pattern_state(input, world);
}

pub(crate) fn sync_beacon_effect_selection_state(input: &mut ClientInputState, world: &WorldStore) {
    sync_beacon_effect_selection(input, world);
}

fn stonecutter_input_item_id(world: &WorldStore) -> Option<i32> {
    let container = world.inventory().open_container.as_ref()?;
    if container.menu_type_id != Some(STONECUTTER_MENU_TYPE_ID) {
        return None;
    }
    container
        .slots
        .iter()
        .find(|slot| slot.slot == 0)
        .and_then(|slot| slot.item.item_id)
}

fn sync_beacon_effect_selection(input: &mut ClientInputState, world: &WorldStore) {
    let Some(container) = world.inventory().open_container.as_ref() else {
        input.beacon_effect_selection_container_id = None;
        input.beacon_effect_selection_dirty = false;
        input.beacon_primary_effect = None;
        input.beacon_secondary_effect = None;
        return;
    };
    if container.menu_type_id != Some(BEACON_MENU_TYPE_ID) {
        input.beacon_effect_selection_container_id = None;
        input.beacon_effect_selection_dirty = false;
        input.beacon_primary_effect = None;
        input.beacon_secondary_effect = None;
        return;
    }

    if input.beacon_effect_selection_container_id != Some(container.container_id) {
        input.beacon_effect_selection_container_id = Some(container.container_id);
        input.beacon_effect_selection_dirty = false;
    }
    if !input.beacon_effect_selection_dirty {
        input.beacon_primary_effect = beacon_data_effect_id(world, BEACON_PRIMARY_EFFECT_DATA_ID);
        input.beacon_secondary_effect =
            beacon_data_effect_id(world, BEACON_SECONDARY_EFFECT_DATA_ID);
    }
}

fn beacon_set_command(input: &ClientInputState, world: &WorldStore) -> Option<SetBeacon> {
    if !inventory_slot_has_item(world, 0) {
        return None;
    }
    let primary_effect = input.beacon_primary_effect?;
    Some(SetBeacon {
        primary_effect: Some(primary_effect),
        secondary_effect: input.beacon_secondary_effect,
    })
}

fn beacon_levels(world: &WorldStore) -> i16 {
    world
        .open_container_data_value(BEACON_LEVELS_DATA_ID)
        .unwrap_or_default()
}

fn beacon_data_effect_id(world: &WorldStore, data_id: i16) -> Option<i32> {
    let value = world.open_container_data_value(data_id)?;
    (value > 0).then_some(i32::from(value) - 1)
}

fn inventory_screen_click_target(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<InventoryClickTarget> {
    let layout = inventory_screen_layout(world)?;
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    if x < 0.0 || y < 0.0 || x >= f64::from(layout.width) || y >= f64::from(layout.height) {
        return Some(InventoryClickTarget::Outside);
    }
    for slot in layout.slots {
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
        .and_then(|container| container.slots.iter().find(|slot| slot.slot == slot_num))
        .is_some_and(|slot| !item_stack_is_empty(&slot.item))
}

fn item_stack_is_empty(stack: &ItemStackSummary) -> bool {
    stack.item_id.is_none() || stack.count <= 0
}

fn same_item_same_components(left: &ItemStackSummary, right: &ItemStackSummary) -> bool {
    left.item_id == right.item_id && left.component_patch == right.component_patch
}

fn local_inventory_slot_max_stack_size(slot_num: i16, stack: &ItemStackSummary) -> i32 {
    let item_max_stack_size = local_inventory_item_max_stack_size(stack);
    let slot_max_stack_size = match slot_num {
        0 => 0,
        5..=8 => 1,
        _ => 64,
    };
    item_max_stack_size.min(slot_max_stack_size)
}

fn local_inventory_item_max_stack_size(stack: &ItemStackSummary) -> i32 {
    if item_stack_is_empty(stack) {
        return 0;
    }
    if let Some(max_stack_size) = stack.component_patch.max_stack_size {
        return max_stack_size.clamp(1, 99);
    }
    if stack.component_patch.max_damage.is_some() || stack.component_patch.damage.is_some() {
        return 1;
    }
    64
}

#[cfg(test)]
mod tests;
