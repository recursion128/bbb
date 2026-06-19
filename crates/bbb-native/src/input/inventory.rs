use std::time::{Duration, Instant};

use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::packets::{
    ContainerInput, ItemStackSummary, RenameItem, SelectTradeCommand, SetBeacon,
};
use bbb_world::{
    ContainerClickSlotRequest, MountEquipmentSlotVisibility, MountInventoryKind, WorldStore,
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
        queue_rename_item_command, queue_select_trade_command, queue_set_beacon_command,
    },
    text_edit, AnvilRenameInputSignature, ClientInputState,
};
use crate::item_runtime::NativeItemRuntime;

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
pub(crate) enum InventoryScreenBackground {
    LocalInventory,
    Generic9xRows {
        rows: u8,
    },
    Generic3x3,
    Anvil,
    Beacon,
    BlastFurnace,
    BrewingStand,
    CartographyTable,
    CraftingTable,
    Crafter,
    EnchantmentTable,
    Furnace,
    Grindstone,
    Hopper,
    Mount {
        kind: MountInventoryKind,
        inventory_columns: u8,
    },
    Lectern,
    Loom,
    Merchant,
    ShulkerBox,
    Smithing,
    Smoker,
    Stonecutter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InventoryScreenLayout {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) background: InventoryScreenBackground,
    pub(crate) slots: Vec<InventorySlotLayout>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InventorySlotLayout {
    pub(crate) slot_id: i16,
    pub(crate) x: i32,
    pub(crate) y: i32,
}

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

pub(crate) fn local_inventory_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(46);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 154,
        y: 28,
    });
    for y in 0..2 {
        for x in 0..2 {
            slots.push(InventorySlotLayout {
                slot_id: (1 + x + y * 2) as i16,
                x: 98 + x * 18,
                y: 18 + y * 18,
            });
        }
    }
    for index in 0..4 {
        slots.push(InventorySlotLayout {
            slot_id: (5 + index) as i16,
            x: 8,
            y: 8 + index * 18,
        });
    }
    for y in 0..3 {
        for x in 0..9 {
            slots.push(InventorySlotLayout {
                slot_id: (9 + x + y * 9) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..9 {
        slots.push(InventorySlotLayout {
            slot_id: (36 + x) as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }
    slots.push(InventorySlotLayout {
        slot_id: 45,
        x: 77,
        y: 62,
    });
    slots
}

pub(crate) fn inventory_screen_layout(world: &WorldStore) -> Option<InventoryScreenLayout> {
    if world.local_inventory_is_open() {
        return Some(InventoryScreenLayout {
            width: INVENTORY_SCREEN_WIDTH,
            height: INVENTORY_SCREEN_HEIGHT,
            background: InventoryScreenBackground::LocalInventory,
            slots: local_inventory_slot_layouts(),
        });
    }

    let container = world.inventory().open_container.as_ref()?;
    if let Some(mount) = container.mount {
        let kind = world.open_mount_inventory_kind()?;
        let inventory_columns = match kind {
            MountInventoryKind::Horse => clamped_mount_inventory_columns(mount.inventory_columns),
            MountInventoryKind::Nautilus => 0,
        };
        let equipment_slots = world.open_mount_equipment_slot_visibility()?;
        return Some(InventoryScreenLayout {
            width: MOUNT_SCREEN_WIDTH,
            height: MOUNT_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Mount {
                kind,
                inventory_columns,
            },
            slots: mount_inventory_slot_layouts(inventory_columns, equipment_slots),
        });
    }
    let menu_type_id = container.menu_type_id?;
    if let Some(rows) = generic_container_rows(menu_type_id) {
        return Some(InventoryScreenLayout {
            width: GENERIC_CONTAINER_WIDTH,
            height: GENERIC_CONTAINER_BASE_HEIGHT + i32::from(rows) * GENERIC_CONTAINER_ROW_HEIGHT,
            background: InventoryScreenBackground::Generic9xRows { rows },
            slots: generic_container_slot_layouts(rows),
        });
    }
    if menu_type_id == GENERIC_3X3_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: GENERIC_3X3_SCREEN_WIDTH,
            height: GENERIC_3X3_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Generic3x3,
            slots: generic_3x3_slot_layouts(),
        });
    }
    if menu_type_id == CRAFTER_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: CRAFTER_SCREEN_WIDTH,
            height: CRAFTER_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Crafter,
            slots: crafter_slot_layouts(),
        });
    }
    if menu_type_id == CRAFTING_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: CRAFTING_SCREEN_WIDTH,
            height: CRAFTING_SCREEN_HEIGHT,
            background: InventoryScreenBackground::CraftingTable,
            slots: crafting_table_slot_layouts(),
        });
    }
    if menu_type_id == ENCHANTMENT_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: ENCHANTMENT_SCREEN_WIDTH,
            height: ENCHANTMENT_SCREEN_HEIGHT,
            background: InventoryScreenBackground::EnchantmentTable,
            slots: enchantment_table_slot_layouts(),
        });
    }
    if menu_type_id == ANVIL_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: ANVIL_SCREEN_WIDTH,
            height: ANVIL_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Anvil,
            slots: anvil_slot_layouts(),
        });
    }
    if menu_type_id == BEACON_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: BEACON_SCREEN_WIDTH,
            height: BEACON_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Beacon,
            slots: beacon_slot_layouts(),
        });
    }
    if menu_type_id == BREWING_STAND_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: BREWING_STAND_SCREEN_WIDTH,
            height: BREWING_STAND_SCREEN_HEIGHT,
            background: InventoryScreenBackground::BrewingStand,
            slots: brewing_stand_slot_layouts(),
        });
    }
    if let Some(background) = furnace_screen_background(menu_type_id) {
        return Some(InventoryScreenLayout {
            width: FURNACE_SCREEN_WIDTH,
            height: FURNACE_SCREEN_HEIGHT,
            background,
            slots: furnace_slot_layouts(),
        });
    }
    if menu_type_id == GRINDSTONE_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: GRINDSTONE_SCREEN_WIDTH,
            height: GRINDSTONE_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Grindstone,
            slots: grindstone_slot_layouts(),
        });
    }
    if menu_type_id == HOPPER_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: HOPPER_SCREEN_WIDTH,
            height: HOPPER_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Hopper,
            slots: hopper_slot_layouts(),
        });
    }
    if menu_type_id == LECTERN_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: LECTERN_SCREEN_WIDTH,
            height: LECTERN_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Lectern,
            slots: Vec::new(),
        });
    }
    if menu_type_id == LOOM_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: LOOM_SCREEN_WIDTH,
            height: LOOM_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Loom,
            slots: loom_slot_layouts(),
        });
    }
    if menu_type_id == MERCHANT_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: MERCHANT_SCREEN_WIDTH,
            height: MERCHANT_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Merchant,
            slots: merchant_slot_layouts(),
        });
    }
    if menu_type_id == SHULKER_BOX_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: SHULKER_BOX_SCREEN_WIDTH,
            height: SHULKER_BOX_SCREEN_HEIGHT,
            background: InventoryScreenBackground::ShulkerBox,
            slots: shulker_box_slot_layouts(),
        });
    }
    if menu_type_id == SMITHING_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: SMITHING_SCREEN_WIDTH,
            height: SMITHING_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Smithing,
            slots: smithing_slot_layouts(),
        });
    }
    if menu_type_id == CARTOGRAPHY_TABLE_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: CARTOGRAPHY_TABLE_SCREEN_WIDTH,
            height: CARTOGRAPHY_TABLE_SCREEN_HEIGHT,
            background: InventoryScreenBackground::CartographyTable,
            slots: cartography_table_slot_layouts(),
        });
    }
    if menu_type_id == STONECUTTER_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: STONECUTTER_SCREEN_WIDTH,
            height: STONECUTTER_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Stonecutter,
            slots: stonecutter_slot_layouts(),
        });
    }
    None
}

fn generic_container_rows(menu_type_id: i32) -> Option<u8> {
    (GENERIC_CONTAINER_FIRST_MENU_TYPE_ID..=GENERIC_CONTAINER_LAST_MENU_TYPE_ID)
        .contains(&menu_type_id)
        .then(|| (menu_type_id - GENERIC_CONTAINER_FIRST_MENU_TYPE_ID + 1) as u8)
}

fn furnace_screen_background(menu_type_id: i32) -> Option<InventoryScreenBackground> {
    match menu_type_id {
        BLAST_FURNACE_MENU_TYPE_ID => Some(InventoryScreenBackground::BlastFurnace),
        FURNACE_MENU_TYPE_ID => Some(InventoryScreenBackground::Furnace),
        SMOKER_MENU_TYPE_ID => Some(InventoryScreenBackground::Smoker),
        _ => None,
    }
}

fn generic_container_slot_layouts(rows: u8) -> Vec<InventorySlotLayout> {
    let rows = rows.clamp(1, 6);
    let row_count = i32::from(rows);
    let container_slot_count = i16::from(rows) * GENERIC_CONTAINER_SLOT_COUNT_PER_ROW;
    let inventory_top = 18 + row_count * GENERIC_CONTAINER_ROW_HEIGHT + 13;
    let mut slots = Vec::with_capacity(container_slot_count as usize + 36);

    for y in 0..row_count {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 18 + y * 18,
            });
        }
    }
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: container_slot_count + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: inventory_top + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: container_slot_count + 27 + x as i16,
            x: 8 + x * 18,
            y: inventory_top + 58,
        });
    }

    slots
}

fn generic_3x3_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(GENERIC_3X3_SLOT_COUNT as usize + 36);
    for y in 0..GENERIC_3X3_SLOT_COLUMNS {
        for x in 0..GENERIC_3X3_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: (x + y * GENERIC_3X3_SLOT_COLUMNS) as i16,
                x: 62 + x * 18,
                y: 17 + y * 18,
            });
        }
    }
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: GENERIC_3X3_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: GENERIC_3X3_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn crafter_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(CRAFTER_TOTAL_SLOT_COUNT as usize);
    for y in 0..CRAFTER_GRID_SLOT_COLUMNS {
        for x in 0..CRAFTER_GRID_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: (x + y * CRAFTER_GRID_SLOT_COLUMNS) as i16,
                x: 26 + x * 18,
                y: 17 + y * 18,
            });
        }
    }
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: CRAFTER_GRID_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: CRAFTER_GRID_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }
    slots.push(InventorySlotLayout {
        slot_id: CRAFTER_RESULT_SLOT,
        x: 134,
        y: 35,
    });

    slots
}

fn crafting_table_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(CRAFTING_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 124,
        y: 35,
    });
    for y in 0..CRAFTING_GRID_SLOT_COLUMNS {
        for x in 0..CRAFTING_GRID_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: (1 + x + y * CRAFTING_GRID_SLOT_COLUMNS) as i16,
                x: 30 + x * 18,
                y: 17 + y * 18,
            });
        }
    }
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: CRAFTING_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: CRAFTING_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn anvil_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(ANVIL_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 27,
        y: 47,
    });
    slots.push(InventorySlotLayout {
        slot_id: 1,
        x: 76,
        y: 47,
    });
    slots.push(InventorySlotLayout {
        slot_id: 2,
        x: 134,
        y: 47,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: ANVIL_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: ANVIL_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn enchantment_table_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(ENCHANTMENT_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 15,
        y: 47,
    });
    slots.push(InventorySlotLayout {
        slot_id: 1,
        x: 35,
        y: 47,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: ENCHANTMENT_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: ENCHANTMENT_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn beacon_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(BEACON_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 136,
        y: 110,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: BEACON_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 36 + x * 18,
                y: 137 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: BEACON_SLOT_COUNT + 27 + x as i16,
            x: 36 + x * 18,
            y: 195,
        });
    }

    slots
}

fn brewing_stand_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(BREWING_STAND_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 56,
        y: 51,
    });
    slots.push(InventorySlotLayout {
        slot_id: 1,
        x: 79,
        y: 58,
    });
    slots.push(InventorySlotLayout {
        slot_id: 2,
        x: 102,
        y: 51,
    });
    slots.push(InventorySlotLayout {
        slot_id: 3,
        x: 79,
        y: 17,
    });
    slots.push(InventorySlotLayout {
        slot_id: 4,
        x: 17,
        y: 17,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: BREWING_STAND_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: BREWING_STAND_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn hopper_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(HOPPER_SLOT_COUNT as usize + 36);
    for x in 0..HOPPER_SLOT_COUNT {
        slots.push(InventorySlotLayout {
            slot_id: x,
            x: 44 + i32::from(x) * 18,
            y: 20,
        });
    }
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: HOPPER_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 51 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: HOPPER_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 109,
        });
    }

    slots
}

fn clamped_mount_inventory_columns(inventory_columns: i32) -> u8 {
    inventory_columns.clamp(0, MOUNT_MAX_INVENTORY_COLUMNS) as u8
}

fn mount_inventory_slot_layouts(
    inventory_columns: u8,
    equipment_slots: MountEquipmentSlotVisibility,
) -> Vec<InventorySlotLayout> {
    let inventory_columns = i32::from(inventory_columns);
    let mount_inventory_slot_count = inventory_columns * MOUNT_INVENTORY_ROWS;
    let player_inventory_start =
        MOUNT_EQUIPMENT_SLOT_COUNT + i16::try_from(mount_inventory_slot_count).unwrap_or_default();
    let mut slots = Vec::with_capacity(
        MOUNT_EQUIPMENT_SLOT_COUNT as usize + mount_inventory_slot_count as usize + 36,
    );
    if equipment_slots.saddle {
        slots.push(InventorySlotLayout {
            slot_id: 0,
            x: 8,
            y: 18,
        });
    }
    if equipment_slots.body.is_some() {
        slots.push(InventorySlotLayout {
            slot_id: 1,
            x: 8,
            y: 36,
        });
    }
    for y in 0..MOUNT_INVENTORY_ROWS {
        for x in 0..inventory_columns {
            slots.push(InventorySlotLayout {
                slot_id: MOUNT_EQUIPMENT_SLOT_COUNT
                    + i16::try_from(x + y * inventory_columns).unwrap_or_default(),
                x: 80 + x * 18,
                y: 18 + y * 18,
            });
        }
    }
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: player_inventory_start + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: player_inventory_start + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn furnace_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(FURNACE_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 56,
        y: 17,
    });
    slots.push(InventorySlotLayout {
        slot_id: 1,
        x: 56,
        y: 53,
    });
    slots.push(InventorySlotLayout {
        slot_id: 2,
        x: 116,
        y: 35,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: FURNACE_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: FURNACE_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn grindstone_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(GRINDSTONE_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 49,
        y: 19,
    });
    slots.push(InventorySlotLayout {
        slot_id: 1,
        x: 49,
        y: 40,
    });
    slots.push(InventorySlotLayout {
        slot_id: 2,
        x: 129,
        y: 34,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: GRINDSTONE_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: GRINDSTONE_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn shulker_box_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(SHULKER_BOX_SLOT_COUNT as usize + 36);
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 18 + y * 18,
            });
        }
    }
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: SHULKER_BOX_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: SHULKER_BOX_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn loom_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(LOOM_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 13,
        y: 26,
    });
    slots.push(InventorySlotLayout {
        slot_id: 1,
        x: 33,
        y: 26,
    });
    slots.push(InventorySlotLayout {
        slot_id: 2,
        x: 23,
        y: 45,
    });
    slots.push(InventorySlotLayout {
        slot_id: 3,
        x: 143,
        y: 57,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: LOOM_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: LOOM_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn merchant_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(MERCHANT_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 136,
        y: 37,
    });
    slots.push(InventorySlotLayout {
        slot_id: 1,
        x: 162,
        y: 37,
    });
    slots.push(InventorySlotLayout {
        slot_id: 2,
        x: 220,
        y: 37,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: MERCHANT_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 108 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: MERCHANT_SLOT_COUNT + 27 + x as i16,
            x: 108 + x * 18,
            y: 142,
        });
    }

    slots
}

fn cartography_table_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(CARTOGRAPHY_TABLE_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 15,
        y: 15,
    });
    slots.push(InventorySlotLayout {
        slot_id: 1,
        x: 15,
        y: 52,
    });
    slots.push(InventorySlotLayout {
        slot_id: 2,
        x: 145,
        y: 39,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: CARTOGRAPHY_TABLE_SLOT_COUNT
                    + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: CARTOGRAPHY_TABLE_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn smithing_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(SMITHING_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 8,
        y: 48,
    });
    slots.push(InventorySlotLayout {
        slot_id: 1,
        x: 26,
        y: 48,
    });
    slots.push(InventorySlotLayout {
        slot_id: 2,
        x: 44,
        y: 48,
    });
    slots.push(InventorySlotLayout {
        slot_id: 3,
        x: 98,
        y: 48,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: SMITHING_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: SMITHING_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn stonecutter_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(STONECUTTER_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 20,
        y: 33,
    });
    slots.push(InventorySlotLayout {
        slot_id: 1,
        x: 143,
        y: 33,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: STONECUTTER_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: STONECUTTER_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
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
    if !input.focused || inventory_screen_layout(world).is_none() {
        return false;
    }
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

pub(crate) fn handle_inventory_text_input(
    input: &mut ClientInputState,
    world: &WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    text: &str,
) -> bool {
    if !input.focused || !anvil_screen_is_open(world) {
        return false;
    }

    sync_anvil_rename_input(input, world, item_runtime);
    if input.anvil_rename_input.is_none() {
        return true;
    }

    let before = input.anvil_rename_text.clone();
    insert_anvil_rename_text(
        &mut input.anvil_rename_text,
        &mut input.anvil_rename_cursor,
        text,
    );
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
        KeyCode::ArrowLeft => {
            sync_anvil_rename_input(input, world, item_runtime);
            if input.anvil_rename_input.is_some() {
                input.anvil_rename_cursor = if input.control_down() {
                    text_edit::word_position(
                        &input.anvil_rename_text,
                        input.anvil_rename_cursor,
                        -1,
                    )
                } else {
                    input.anvil_rename_cursor.saturating_sub(1)
                };
            }
            true
        }
        KeyCode::ArrowRight => {
            sync_anvil_rename_input(input, world, item_runtime);
            if input.anvil_rename_input.is_some() {
                input.anvil_rename_cursor = if input.control_down() {
                    text_edit::word_position(&input.anvil_rename_text, input.anvil_rename_cursor, 1)
                } else {
                    (input.anvil_rename_cursor + 1)
                        .min(anvil_rename_char_len(&input.anvil_rename_text))
                };
            }
            true
        }
        KeyCode::Home => {
            sync_anvil_rename_input(input, world, item_runtime);
            if input.anvil_rename_input.is_some() {
                input.anvil_rename_cursor = 0;
            }
            true
        }
        KeyCode::End => {
            sync_anvil_rename_input(input, world, item_runtime);
            if input.anvil_rename_input.is_some() {
                input.anvil_rename_cursor = anvil_rename_char_len(&input.anvil_rename_text);
            }
            true
        }
        KeyCode::Backspace => {
            sync_anvil_rename_input(input, world, item_runtime);
            if input.anvil_rename_input.is_some() {
                let before = input.anvil_rename_text.clone();
                if input.control_down() {
                    text_edit::remove_word_before_cursor(
                        &mut input.anvil_rename_text,
                        &mut input.anvil_rename_cursor,
                    );
                } else {
                    remove_anvil_rename_char_before_cursor(
                        &mut input.anvil_rename_text,
                        &mut input.anvil_rename_cursor,
                    );
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
                if input.control_down() {
                    text_edit::remove_word_at_cursor(
                        &mut input.anvil_rename_text,
                        input.anvil_rename_cursor,
                    );
                } else {
                    remove_anvil_rename_char_at_cursor(
                        &mut input.anvil_rename_text,
                        input.anvil_rename_cursor,
                    );
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

fn insert_anvil_rename_text(current: &mut String, cursor: &mut usize, text: &str) {
    *cursor = (*cursor).min(anvil_rename_char_len(current));
    let mut remaining = ANVIL_RENAME_MAX_LENGTH.saturating_sub(anvil_rename_len(current));
    for ch in text.chars().filter(|ch| is_anvil_rename_char(*ch)) {
        let len = ch.len_utf16();
        if len > remaining {
            break;
        }
        let insert_at = anvil_rename_byte_index(current, *cursor);
        current.insert(insert_at, ch);
        *cursor += 1;
        remaining -= len;
    }
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
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    use bbb_protocol::packets::{
        AddEntity, ContainerButtonClick, ContainerClick, ContainerCloseRequest,
        ContainerSetContent, ContainerSetData, ContainerSlotStateChanged, EntityDataValue,
        EntityDataValueKind, HashedComponentPatch, HashedItemStack, HashedStack, IngredientSummary,
        ItemCostSummary, ItemStackSummary, MerchantOffer, MerchantOffers, MountScreenOpen,
        OpenScreen, PlayerAbilities, RecipePropertySetSummary, RegistryTags, SelectBundleItem,
        SelectTradeCommand, SetBeacon, SetCursorItem, SetEntityData, SetPlayerInventory,
        SlotDisplaySummary, StonecutterSelectableRecipeSummary, TagNetworkPayload, UpdateRecipes,
        UpdateTags, Vec3d,
    };
    use uuid::Uuid;

    const TEST_AGEABLE_MOB_BABY_DATA_ID: u8 = 16;
    const TEST_MOUNT_TAME_FLAGS_DATA_ID: u8 = 18;
    const TEST_ABSTRACT_HORSE_TAME_FLAG: i8 = 2;
    const TEST_TAMABLE_ANIMAL_TAME_FLAG: i8 = 4;

    #[test]
    fn local_inventory_slot_layouts_match_vanilla_inventory_menu() {
        let slots = local_inventory_slot_layouts();
        assert_eq!(slots.len(), 46);
        assert_eq!(
            slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 154,
                y: 28,
            }
        );
        assert_eq!(
            slots[1],
            InventorySlotLayout {
                slot_id: 1,
                x: 98,
                y: 18,
            }
        );
        assert_eq!(
            slots[5],
            InventorySlotLayout {
                slot_id: 5,
                x: 8,
                y: 8,
            }
        );
        assert_eq!(
            slots[9],
            InventorySlotLayout {
                slot_id: 9,
                x: 8,
                y: 84,
            }
        );
        assert_eq!(
            slots[36],
            InventorySlotLayout {
                slot_id: 36,
                x: 8,
                y: 142,
            }
        );
        assert_eq!(
            slots[45],
            InventorySlotLayout {
                slot_id: 45,
                x: 77,
                y: 62,
            }
        );
    }

    #[test]
    fn local_inventory_hit_test_uses_centered_vanilla_screen_and_hover_margin() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        assert!(world.open_local_inventory());
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(560.0, 419.0)), size),
            Some(InventoryClickTarget::Slot(36))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(559.0, 418.0)), size),
            Some(InventoryClickTarget::Slot(36))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(600.0, 300.0)), size),
            Some(InventoryClickTarget::EmptyPanel)
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(551.0, 277.0)), size),
            Some(InventoryClickTarget::Outside)
        );
    }

    #[test]
    fn generic_container_layout_matches_vanilla_chest_menu() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: 5,
            title: "Large Chest".to_string(),
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(layout.width, 176);
        assert_eq!(layout.height, 222);
        assert_eq!(
            layout.background,
            InventoryScreenBackground::Generic9xRows { rows: 6 }
        );
        assert_eq!(layout.slots.len(), 90);
        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 8,
                y: 18,
            }
        );
        assert_eq!(
            layout.slots[53],
            InventorySlotLayout {
                slot_id: 53,
                x: 152,
                y: 108,
            }
        );
        assert_eq!(
            layout.slots[54],
            InventorySlotLayout {
                slot_id: 54,
                x: 8,
                y: 139,
            }
        );
        assert_eq!(
            layout.slots[89],
            InventorySlotLayout {
                slot_id: 89,
                x: 152,
                y: 197,
            }
        );
    }

    #[test]
    fn generic_3x3_layout_matches_vanilla_dispenser_menu() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: 6,
            title: "Dispenser".to_string(),
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(layout.width, 176);
        assert_eq!(layout.height, 166);
        assert_eq!(layout.background, InventoryScreenBackground::Generic3x3);
        assert_eq!(layout.slots.len(), 45);
        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 62,
                y: 17,
            }
        );
        assert_eq!(
            layout.slots[8],
            InventorySlotLayout {
                slot_id: 8,
                x: 98,
                y: 53,
            }
        );
        assert_eq!(
            layout.slots[9],
            InventorySlotLayout {
                slot_id: 9,
                x: 8,
                y: 84,
            }
        );
        assert_eq!(
            layout.slots[44],
            InventorySlotLayout {
                slot_id: 44,
                x: 152,
                y: 142,
            }
        );
    }

    #[test]
    fn crafter_layout_matches_vanilla_menu() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: CRAFTER_MENU_TYPE_ID,
            title: "Crafter".to_string(),
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(layout.width, 176);
        assert_eq!(layout.height, 166);
        assert_eq!(layout.background, InventoryScreenBackground::Crafter);
        assert_eq!(layout.slots.len(), 46);
        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 26,
                y: 17,
            }
        );
        assert_eq!(
            layout.slots[8],
            InventorySlotLayout {
                slot_id: 8,
                x: 62,
                y: 53,
            }
        );
        assert_eq!(
            layout.slots[9],
            InventorySlotLayout {
                slot_id: 9,
                x: 8,
                y: 84,
            }
        );
        assert_eq!(
            layout.slots[44],
            InventorySlotLayout {
                slot_id: 44,
                x: 152,
                y: 142,
            }
        );
        assert_eq!(
            layout.slots[45],
            InventorySlotLayout {
                slot_id: 45,
                x: 134,
                y: 35,
            }
        );
    }

    #[test]
    fn crafting_table_layout_matches_vanilla_crafting_menu() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: CRAFTING_MENU_TYPE_ID,
            title: "Crafting".to_string(),
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(layout.width, 176);
        assert_eq!(layout.height, 166);
        assert_eq!(layout.background, InventoryScreenBackground::CraftingTable);
        assert_eq!(layout.slots.len(), 46);
        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 124,
                y: 35,
            }
        );
        assert_eq!(
            layout.slots[1],
            InventorySlotLayout {
                slot_id: 1,
                x: 30,
                y: 17,
            }
        );
        assert_eq!(
            layout.slots[9],
            InventorySlotLayout {
                slot_id: 9,
                x: 66,
                y: 53,
            }
        );
        assert_eq!(
            layout.slots[10],
            InventorySlotLayout {
                slot_id: 10,
                x: 8,
                y: 84,
            }
        );
        assert_eq!(
            layout.slots[45],
            InventorySlotLayout {
                slot_id: 45,
                x: 152,
                y: 142,
            }
        );
    }

    #[test]
    fn enchantment_table_layout_matches_vanilla_menu() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: ENCHANTMENT_MENU_TYPE_ID,
            title: "Enchanting Table".to_string(),
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(layout.width, 176);
        assert_eq!(layout.height, 166);
        assert_eq!(
            layout.background,
            InventoryScreenBackground::EnchantmentTable
        );
        assert_eq!(layout.slots.len(), 38);
        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 15,
                y: 47,
            }
        );
        assert_eq!(
            layout.slots[1],
            InventorySlotLayout {
                slot_id: 1,
                x: 35,
                y: 47,
            }
        );
        assert_eq!(
            layout.slots[2],
            InventorySlotLayout {
                slot_id: 2,
                x: 8,
                y: 84,
            }
        );
        assert_eq!(
            layout.slots[37],
            InventorySlotLayout {
                slot_id: 37,
                x: 152,
                y: 142,
            }
        );
    }

    #[test]
    fn anvil_layout_matches_vanilla_item_combiner_menu() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: ANVIL_MENU_TYPE_ID,
            title: "Anvil".to_string(),
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(layout.width, 176);
        assert_eq!(layout.height, 166);
        assert_eq!(layout.background, InventoryScreenBackground::Anvil);
        assert_eq!(layout.slots.len(), 39);
        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 27,
                y: 47,
            }
        );
        assert_eq!(
            layout.slots[1],
            InventorySlotLayout {
                slot_id: 1,
                x: 76,
                y: 47,
            }
        );
        assert_eq!(
            layout.slots[2],
            InventorySlotLayout {
                slot_id: 2,
                x: 134,
                y: 47,
            }
        );
        assert_eq!(
            layout.slots[3],
            InventorySlotLayout {
                slot_id: 3,
                x: 8,
                y: 84,
            }
        );
        assert_eq!(
            layout.slots[38],
            InventorySlotLayout {
                slot_id: 38,
                x: 152,
                y: 142,
            }
        );
    }

    #[test]
    fn beacon_layout_matches_vanilla_menu() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: BEACON_MENU_TYPE_ID,
            title: "Beacon".to_string(),
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(layout.width, 230);
        assert_eq!(layout.height, 219);
        assert_eq!(layout.background, InventoryScreenBackground::Beacon);
        assert_eq!(layout.slots.len(), 37);
        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 136,
                y: 110,
            }
        );
        assert_eq!(
            layout.slots[1],
            InventorySlotLayout {
                slot_id: 1,
                x: 36,
                y: 137,
            }
        );
        assert_eq!(
            layout.slots[36],
            InventorySlotLayout {
                slot_id: 36,
                x: 180,
                y: 195,
            }
        );
    }

    #[test]
    fn brewing_stand_layout_matches_vanilla_menu() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: BREWING_STAND_MENU_TYPE_ID,
            title: "Brewing Stand".to_string(),
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(layout.width, 176);
        assert_eq!(layout.height, 166);
        assert_eq!(layout.background, InventoryScreenBackground::BrewingStand);
        assert_eq!(layout.slots.len(), 41);
        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 56,
                y: 51,
            }
        );
        assert_eq!(
            layout.slots[1],
            InventorySlotLayout {
                slot_id: 1,
                x: 79,
                y: 58,
            }
        );
        assert_eq!(
            layout.slots[2],
            InventorySlotLayout {
                slot_id: 2,
                x: 102,
                y: 51,
            }
        );
        assert_eq!(
            layout.slots[3],
            InventorySlotLayout {
                slot_id: 3,
                x: 79,
                y: 17,
            }
        );
        assert_eq!(
            layout.slots[4],
            InventorySlotLayout {
                slot_id: 4,
                x: 17,
                y: 17,
            }
        );
        assert_eq!(
            layout.slots[5],
            InventorySlotLayout {
                slot_id: 5,
                x: 8,
                y: 84,
            }
        );
        assert_eq!(
            layout.slots[40],
            InventorySlotLayout {
                slot_id: 40,
                x: 152,
                y: 142,
            }
        );
    }

    #[test]
    fn furnace_like_layouts_match_vanilla_abstract_furnace_menu() {
        for (menu_type_id, title, background) in [
            (
                BLAST_FURNACE_MENU_TYPE_ID,
                "Blast Furnace",
                InventoryScreenBackground::BlastFurnace,
            ),
            (
                FURNACE_MENU_TYPE_ID,
                "Furnace",
                InventoryScreenBackground::Furnace,
            ),
            (
                SMOKER_MENU_TYPE_ID,
                "Smoker",
                InventoryScreenBackground::Smoker,
            ),
        ] {
            let mut world = WorldStore::new();
            world.apply_open_screen(OpenScreen {
                container_id: 7,
                menu_type_id,
                title: title.to_string(),
            });

            let layout = inventory_screen_layout(&world).unwrap();

            assert_eq!(layout.width, 176);
            assert_eq!(layout.height, 166);
            assert_eq!(layout.background, background);
            assert_eq!(layout.slots.len(), 39);
            assert_eq!(
                layout.slots[0],
                InventorySlotLayout {
                    slot_id: 0,
                    x: 56,
                    y: 17,
                }
            );
            assert_eq!(
                layout.slots[1],
                InventorySlotLayout {
                    slot_id: 1,
                    x: 56,
                    y: 53,
                }
            );
            assert_eq!(
                layout.slots[2],
                InventorySlotLayout {
                    slot_id: 2,
                    x: 116,
                    y: 35,
                }
            );
            assert_eq!(
                layout.slots[3],
                InventorySlotLayout {
                    slot_id: 3,
                    x: 8,
                    y: 84,
                }
            );
            assert_eq!(
                layout.slots[38],
                InventorySlotLayout {
                    slot_id: 38,
                    x: 152,
                    y: 142,
                }
            );
        }
    }

    #[test]
    fn grindstone_layout_matches_vanilla_menu() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: GRINDSTONE_MENU_TYPE_ID,
            title: "Grindstone".to_string(),
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(layout.width, 176);
        assert_eq!(layout.height, 166);
        assert_eq!(layout.background, InventoryScreenBackground::Grindstone);
        assert_eq!(layout.slots.len(), 39);
        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 49,
                y: 19,
            }
        );
        assert_eq!(
            layout.slots[1],
            InventorySlotLayout {
                slot_id: 1,
                x: 49,
                y: 40,
            }
        );
        assert_eq!(
            layout.slots[2],
            InventorySlotLayout {
                slot_id: 2,
                x: 129,
                y: 34,
            }
        );
        assert_eq!(
            layout.slots[3],
            InventorySlotLayout {
                slot_id: 3,
                x: 8,
                y: 84,
            }
        );
        assert_eq!(
            layout.slots[38],
            InventorySlotLayout {
                slot_id: 38,
                x: 152,
                y: 142,
            }
        );
    }

    #[test]
    fn hopper_layout_matches_vanilla_hopper_menu() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: 16,
            title: "Hopper".to_string(),
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(layout.width, 176);
        assert_eq!(layout.height, 133);
        assert_eq!(layout.background, InventoryScreenBackground::Hopper);
        assert_eq!(layout.slots.len(), 41);
        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 44,
                y: 20,
            }
        );
        assert_eq!(
            layout.slots[4],
            InventorySlotLayout {
                slot_id: 4,
                x: 116,
                y: 20,
            }
        );
        assert_eq!(
            layout.slots[5],
            InventorySlotLayout {
                slot_id: 5,
                x: 8,
                y: 51,
            }
        );
        assert_eq!(
            layout.slots[40],
            InventorySlotLayout {
                slot_id: 40,
                x: 152,
                y: 109,
            }
        );
    }

    #[test]
    fn mount_horse_layout_matches_vanilla_horse_inventory_menu() {
        let mut world = WorldStore::new();
        world.apply_add_entity(add_entity_with_type(42, 66));
        world.apply_mount_screen_open(MountScreenOpen {
            container_id: 7,
            inventory_columns: 5,
            entity_id: 42,
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(layout.width, 176);
        assert_eq!(layout.height, 166);
        assert_eq!(
            layout.background,
            InventoryScreenBackground::Mount {
                kind: MountInventoryKind::Horse,
                inventory_columns: 5,
            }
        );
        assert_eq!(layout.slots.len(), 53);
        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 8,
                y: 18,
            }
        );
        assert_eq!(
            layout.slots[1],
            InventorySlotLayout {
                slot_id: 1,
                x: 8,
                y: 36,
            }
        );
        assert_eq!(
            layout.slots[2],
            InventorySlotLayout {
                slot_id: 2,
                x: 80,
                y: 18,
            }
        );
        assert_eq!(
            layout.slots[16],
            InventorySlotLayout {
                slot_id: 16,
                x: 152,
                y: 54,
            }
        );
        assert_eq!(
            layout.slots[17],
            InventorySlotLayout {
                slot_id: 17,
                x: 8,
                y: 84,
            }
        );
        assert_eq!(
            layout.slots[52],
            InventorySlotLayout {
                slot_id: 52,
                x: 152,
                y: 142,
            }
        );
    }

    #[test]
    fn mount_nautilus_layout_uses_equipment_and_player_slots_only() {
        let mut world = WorldStore::new();
        world.apply_add_entity(add_entity_with_type(42, 88));
        world.apply_set_entity_data(SetEntityData {
            id: 42,
            values: vec![byte_entity_data(
                TEST_MOUNT_TAME_FLAGS_DATA_ID,
                TEST_TAMABLE_ANIMAL_TAME_FLAG,
            )],
        });
        world.apply_mount_screen_open(MountScreenOpen {
            container_id: 7,
            inventory_columns: 5,
            entity_id: 42,
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(
            layout.background,
            InventoryScreenBackground::Mount {
                kind: MountInventoryKind::Nautilus,
                inventory_columns: 0,
            }
        );
        assert_eq!(layout.slots.len(), 38);
        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 8,
                y: 18,
            }
        );
        assert_eq!(
            layout.slots[1],
            InventorySlotLayout {
                slot_id: 1,
                x: 8,
                y: 36,
            }
        );
        assert_eq!(
            layout.slots[2],
            InventorySlotLayout {
                slot_id: 2,
                x: 8,
                y: 84,
            }
        );
        assert_eq!(
            layout.slots[37],
            InventorySlotLayout {
                slot_id: 37,
                x: 152,
                y: 142,
            }
        );
    }

    #[test]
    fn mount_donkey_layout_hides_inactive_equipment_slots() {
        let mut world = WorldStore::new();
        world.apply_add_entity(add_entity_with_type(42, 36));
        world.apply_mount_screen_open(MountScreenOpen {
            container_id: 7,
            inventory_columns: 3,
            entity_id: 42,
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert!(!layout.slots.iter().any(|slot| slot.slot_id == 0));
        assert!(!layout.slots.iter().any(|slot| slot.slot_id == 1));
        assert_eq!(layout.slots.len(), 45);
        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 2,
                x: 80,
                y: 18,
            }
        );
        assert_eq!(
            layout.slots[8],
            InventorySlotLayout {
                slot_id: 10,
                x: 116,
                y: 54,
            }
        );
        assert_eq!(
            layout.slots[9],
            InventorySlotLayout {
                slot_id: 11,
                x: 8,
                y: 84,
            }
        );
    }

    #[test]
    fn mount_tamed_donkey_layout_shows_saddle_but_no_body_slot() {
        let mut world = WorldStore::new();
        world.apply_add_entity(add_entity_with_type(42, 36));
        world.apply_set_entity_data(SetEntityData {
            id: 42,
            values: vec![byte_entity_data(
                TEST_MOUNT_TAME_FLAGS_DATA_ID,
                TEST_ABSTRACT_HORSE_TAME_FLAG,
            )],
        });
        world.apply_mount_screen_open(MountScreenOpen {
            container_id: 7,
            inventory_columns: 3,
            entity_id: 42,
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 8,
                y: 18,
            }
        );
        assert!(!layout.slots.iter().any(|slot| slot.slot_id == 1));
        assert_eq!(layout.slots.len(), 46);
        assert_eq!(
            layout.slots[1],
            InventorySlotLayout {
                slot_id: 2,
                x: 80,
                y: 18,
            }
        );
    }

    #[test]
    fn mount_baby_tamed_donkey_layout_hides_equipment_slots() {
        let mut world = WorldStore::new();
        world.apply_add_entity(add_entity_with_type(42, 36));
        world.apply_set_entity_data(SetEntityData {
            id: 42,
            values: vec![
                byte_entity_data(TEST_MOUNT_TAME_FLAGS_DATA_ID, TEST_ABSTRACT_HORSE_TAME_FLAG),
                bool_entity_data(TEST_AGEABLE_MOB_BABY_DATA_ID, true),
            ],
        });
        world.apply_mount_screen_open(MountScreenOpen {
            container_id: 7,
            inventory_columns: 3,
            entity_id: 42,
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert!(!layout.slots.iter().any(|slot| slot.slot_id == 0));
        assert!(!layout.slots.iter().any(|slot| slot.slot_id == 1));
        assert_eq!(layout.slots[0].slot_id, 2);
    }

    #[test]
    fn lectern_layout_matches_vanilla_book_screen() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: LECTERN_MENU_TYPE_ID,
            title: "Lectern".to_string(),
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(layout.width, 192);
        assert_eq!(layout.height, 192);
        assert_eq!(layout.background, InventoryScreenBackground::Lectern);
        assert!(layout.slots.is_empty());
    }

    #[test]
    fn shulker_box_layout_matches_vanilla_menu() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: 20,
            title: "Shulker Box".to_string(),
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(layout.width, 176);
        assert_eq!(layout.height, 167);
        assert_eq!(layout.background, InventoryScreenBackground::ShulkerBox);
        assert_eq!(layout.slots.len(), 63);
        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 8,
                y: 18,
            }
        );
        assert_eq!(
            layout.slots[26],
            InventorySlotLayout {
                slot_id: 26,
                x: 152,
                y: 54,
            }
        );
        assert_eq!(
            layout.slots[27],
            InventorySlotLayout {
                slot_id: 27,
                x: 8,
                y: 84,
            }
        );
        assert_eq!(
            layout.slots[62],
            InventorySlotLayout {
                slot_id: 62,
                x: 152,
                y: 142,
            }
        );
    }

    #[test]
    fn loom_layout_matches_vanilla_menu() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: LOOM_MENU_TYPE_ID,
            title: "Loom".to_string(),
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(layout.width, 176);
        assert_eq!(layout.height, 166);
        assert_eq!(layout.background, InventoryScreenBackground::Loom);
        assert_eq!(layout.slots.len(), 40);
        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 13,
                y: 26,
            }
        );
        assert_eq!(
            layout.slots[1],
            InventorySlotLayout {
                slot_id: 1,
                x: 33,
                y: 26,
            }
        );
        assert_eq!(
            layout.slots[2],
            InventorySlotLayout {
                slot_id: 2,
                x: 23,
                y: 45,
            }
        );
        assert_eq!(
            layout.slots[3],
            InventorySlotLayout {
                slot_id: 3,
                x: 143,
                y: 57,
            }
        );
        assert_eq!(
            layout.slots[4],
            InventorySlotLayout {
                slot_id: 4,
                x: 8,
                y: 84,
            }
        );
        assert_eq!(
            layout.slots[39],
            InventorySlotLayout {
                slot_id: 39,
                x: 152,
                y: 142,
            }
        );
    }

    #[test]
    fn merchant_layout_matches_vanilla_menu() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: MERCHANT_MENU_TYPE_ID,
            title: "Merchant".to_string(),
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(layout.width, 276);
        assert_eq!(layout.height, 166);
        assert_eq!(layout.background, InventoryScreenBackground::Merchant);
        assert_eq!(layout.slots.len(), 39);
        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 136,
                y: 37,
            }
        );
        assert_eq!(
            layout.slots[1],
            InventorySlotLayout {
                slot_id: 1,
                x: 162,
                y: 37,
            }
        );
        assert_eq!(
            layout.slots[2],
            InventorySlotLayout {
                slot_id: 2,
                x: 220,
                y: 37,
            }
        );
        assert_eq!(
            layout.slots[3],
            InventorySlotLayout {
                slot_id: 3,
                x: 108,
                y: 84,
            }
        );
        assert_eq!(
            layout.slots[38],
            InventorySlotLayout {
                slot_id: 38,
                x: 252,
                y: 142,
            }
        );
    }

    #[test]
    fn smithing_layout_matches_vanilla_item_combiner_menu() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: SMITHING_MENU_TYPE_ID,
            title: "Smithing".to_string(),
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(layout.width, 176);
        assert_eq!(layout.height, 166);
        assert_eq!(layout.background, InventoryScreenBackground::Smithing);
        assert_eq!(layout.slots.len(), 40);
        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 8,
                y: 48,
            }
        );
        assert_eq!(
            layout.slots[1],
            InventorySlotLayout {
                slot_id: 1,
                x: 26,
                y: 48,
            }
        );
        assert_eq!(
            layout.slots[2],
            InventorySlotLayout {
                slot_id: 2,
                x: 44,
                y: 48,
            }
        );
        assert_eq!(
            layout.slots[3],
            InventorySlotLayout {
                slot_id: 3,
                x: 98,
                y: 48,
            }
        );
        assert_eq!(
            layout.slots[4],
            InventorySlotLayout {
                slot_id: 4,
                x: 8,
                y: 84,
            }
        );
        assert_eq!(
            layout.slots[39],
            InventorySlotLayout {
                slot_id: 39,
                x: 152,
                y: 142,
            }
        );
    }

    #[test]
    fn cartography_table_layout_matches_vanilla_menu() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: CARTOGRAPHY_TABLE_MENU_TYPE_ID,
            title: "Cartography Table".to_string(),
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(layout.width, 176);
        assert_eq!(layout.height, 166);
        assert_eq!(
            layout.background,
            InventoryScreenBackground::CartographyTable
        );
        assert_eq!(layout.slots.len(), 39);
        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 15,
                y: 15,
            }
        );
        assert_eq!(
            layout.slots[1],
            InventorySlotLayout {
                slot_id: 1,
                x: 15,
                y: 52,
            }
        );
        assert_eq!(
            layout.slots[2],
            InventorySlotLayout {
                slot_id: 2,
                x: 145,
                y: 39,
            }
        );
        assert_eq!(
            layout.slots[3],
            InventorySlotLayout {
                slot_id: 3,
                x: 8,
                y: 84,
            }
        );
        assert_eq!(
            layout.slots[38],
            InventorySlotLayout {
                slot_id: 38,
                x: 152,
                y: 142,
            }
        );
    }

    #[test]
    fn stonecutter_layout_matches_vanilla_menu() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: STONECUTTER_MENU_TYPE_ID,
            title: "Stonecutter".to_string(),
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(layout.width, 176);
        assert_eq!(layout.height, 166);
        assert_eq!(layout.background, InventoryScreenBackground::Stonecutter);
        assert_eq!(layout.slots.len(), 38);
        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 20,
                y: 33,
            }
        );
        assert_eq!(
            layout.slots[1],
            InventorySlotLayout {
                slot_id: 1,
                x: 143,
                y: 33,
            }
        );
        assert_eq!(
            layout.slots[2],
            InventorySlotLayout {
                slot_id: 2,
                x: 8,
                y: 84,
            }
        );
        assert_eq!(
            layout.slots[37],
            InventorySlotLayout {
                slot_id: 37,
                x: 152,
                y: 142,
            }
        );
    }

    #[test]
    fn generic_container_hit_test_uses_vanilla_screen_height() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: 5,
            title: "Large Chest".to_string(),
        });

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(560.0, 267.0)), size),
            Some(InventoryClickTarget::Slot(0))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(704.0, 446.0)), size),
            Some(InventoryClickTarget::Slot(89))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(600.0, 375.0)), size),
            Some(InventoryClickTarget::EmptyPanel)
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(551.0, 249.0)), size),
            Some(InventoryClickTarget::Outside)
        );
    }

    #[test]
    fn generic_3x3_hit_test_uses_vanilla_dispenser_slots() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: 6,
            title: "Dispenser".to_string(),
        });

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(614.0, 294.0)), size),
            Some(InventoryClickTarget::Slot(0))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(650.0, 330.0)), size),
            Some(InventoryClickTarget::Slot(8))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(704.0, 419.0)), size),
            Some(InventoryClickTarget::Slot(44))
        );
    }

    #[test]
    fn crafter_hit_test_uses_vanilla_slots() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: CRAFTER_MENU_TYPE_ID,
            title: "Crafter".to_string(),
        });

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(586.0, 302.0)), size),
            Some(InventoryClickTarget::Slot(0))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(622.0, 338.0)), size),
            Some(InventoryClickTarget::Slot(8))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(694.0, 320.0)), size),
            Some(InventoryClickTarget::Slot(45))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
            Some(InventoryClickTarget::Slot(44))
        );
    }

    #[test]
    fn crafting_table_hit_test_uses_vanilla_slots() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: CRAFTING_MENU_TYPE_ID,
            title: "Crafting".to_string(),
        });

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(684.0, 320.0)), size),
            Some(InventoryClickTarget::Slot(0))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(590.0, 302.0)), size),
            Some(InventoryClickTarget::Slot(1))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(626.0, 338.0)), size),
            Some(InventoryClickTarget::Slot(9))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
            Some(InventoryClickTarget::Slot(45))
        );
    }

    #[test]
    fn enchantment_table_hit_test_uses_vanilla_slots_and_buttons() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: ENCHANTMENT_MENU_TYPE_ID,
            title: "Enchanting Table".to_string(),
        });

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(575.0, 332.0)), size),
            Some(InventoryClickTarget::Slot(0))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(595.0, 332.0)), size),
            Some(InventoryClickTarget::Slot(1))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
            Some(InventoryClickTarget::Slot(37))
        );
        assert_eq!(
            enchantment_button_at_position(&world, Some(PhysicalPosition::new(620.0, 296.0)), size),
            Some(0)
        );
        assert_eq!(
            enchantment_button_at_position(&world, Some(PhysicalPosition::new(620.0, 334.0)), size),
            Some(2)
        );
    }

    #[test]
    fn anvil_hit_test_uses_vanilla_slots() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: ANVIL_MENU_TYPE_ID,
            title: "Anvil".to_string(),
        });

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(587.0, 332.0)), size),
            Some(InventoryClickTarget::Slot(0))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(636.0, 332.0)), size),
            Some(InventoryClickTarget::Slot(1))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(694.0, 332.0)), size),
            Some(InventoryClickTarget::Slot(2))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
            Some(InventoryClickTarget::Slot(38))
        );
    }

    #[test]
    fn beacon_hit_test_uses_vanilla_slots() {
        let size = PhysicalSize::new(1280, 720);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: BEACON_MENU_TYPE_ID,
            title: "Beacon".to_string(),
        });
        world.apply_container_set_data(ContainerSetData {
            container_id: 7,
            id: BEACON_LEVELS_DATA_ID,
            value: 4,
        });
        sync_beacon_effect_selection(&mut input, &world);

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(669.0, 369.0)), size),
            Some(InventoryClickTarget::Slot(0))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(569.0, 396.0)), size),
            Some(InventoryClickTarget::Slot(1))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(713.0, 454.0)), size),
            Some(InventoryClickTarget::Slot(36))
        );
        assert_eq!(
            beacon_button_at_position(
                &input,
                &world,
                Some(PhysicalPosition::new(590.0, 283.0)),
                size
            ),
            Some(BeaconClickTarget::Effect {
                primary: true,
                effect_id: BEACON_EFFECT_SPEED_ID,
            })
        );
        assert_eq!(
            beacon_button_at_position(
                &input,
                &world,
                Some(PhysicalPosition::new(680.0, 308.0)),
                size
            ),
            Some(BeaconClickTarget::Effect {
                primary: false,
                effect_id: BEACON_EFFECT_REGENERATION_ID,
            })
        );
        assert_eq!(
            beacon_button_at_position(
                &input,
                &world,
                Some(PhysicalPosition::new(704.0, 308.0)),
                size
            ),
            None
        );
        assert_eq!(
            beacon_button_at_position(
                &input,
                &world,
                Some(PhysicalPosition::new(700.0, 369.0)),
                size
            ),
            Some(BeaconClickTarget::Confirm)
        );
        assert_eq!(
            beacon_button_at_position(
                &input,
                &world,
                Some(PhysicalPosition::new(726.0, 369.0)),
                size
            ),
            Some(BeaconClickTarget::Cancel)
        );
    }

    #[test]
    fn brewing_stand_hit_test_uses_vanilla_slots() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: BREWING_STAND_MENU_TYPE_ID,
            title: "Brewing Stand".to_string(),
        });

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(577.0, 302.0)), size),
            Some(InventoryClickTarget::Slot(4))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(639.0, 302.0)), size),
            Some(InventoryClickTarget::Slot(3))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(616.0, 336.0)), size),
            Some(InventoryClickTarget::Slot(0))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(662.0, 336.0)), size),
            Some(InventoryClickTarget::Slot(2))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
            Some(InventoryClickTarget::Slot(40))
        );
    }

    #[test]
    fn furnace_hit_test_uses_vanilla_slots() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: FURNACE_MENU_TYPE_ID,
            title: "Furnace".to_string(),
        });

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(616.0, 302.0)), size),
            Some(InventoryClickTarget::Slot(0))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(616.0, 338.0)), size),
            Some(InventoryClickTarget::Slot(1))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(676.0, 320.0)), size),
            Some(InventoryClickTarget::Slot(2))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
            Some(InventoryClickTarget::Slot(38))
        );
    }

    #[test]
    fn grindstone_hit_test_uses_vanilla_slots() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: GRINDSTONE_MENU_TYPE_ID,
            title: "Grindstone".to_string(),
        });

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(609.0, 304.0)), size),
            Some(InventoryClickTarget::Slot(0))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(609.0, 325.0)), size),
            Some(InventoryClickTarget::Slot(1))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(689.0, 319.0)), size),
            Some(InventoryClickTarget::Slot(2))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
            Some(InventoryClickTarget::Slot(38))
        );
    }

    #[test]
    fn hopper_hit_test_uses_vanilla_slots() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: 16,
            title: "Hopper".to_string(),
        });

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(604.0, 314.0)), size),
            Some(InventoryClickTarget::Slot(0))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(676.0, 314.0)), size),
            Some(InventoryClickTarget::Slot(4))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(704.0, 403.0)), size),
            Some(InventoryClickTarget::Slot(40))
        );
    }

    #[test]
    fn mount_horse_hit_test_uses_vanilla_slots() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_add_entity(add_entity_with_type(42, 66));
        world.apply_mount_screen_open(MountScreenOpen {
            container_id: 7,
            inventory_columns: 5,
            entity_id: 42,
        });

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(560.0, 295.0)), size),
            Some(InventoryClickTarget::Slot(0))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(704.0, 331.0)), size),
            Some(InventoryClickTarget::Slot(16))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
            Some(InventoryClickTarget::Slot(52))
        );
    }

    #[test]
    fn mount_donkey_hit_test_ignores_inactive_equipment_slots() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_add_entity(add_entity_with_type(42, 36));
        world.apply_mount_screen_open(MountScreenOpen {
            container_id: 7,
            inventory_columns: 3,
            entity_id: 42,
        });

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(560.0, 295.0)), size),
            Some(InventoryClickTarget::EmptyPanel)
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(560.0, 313.0)), size),
            Some(InventoryClickTarget::EmptyPanel)
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(632.0, 295.0)), size),
            Some(InventoryClickTarget::Slot(2))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(560.0, 361.0)), size),
            Some(InventoryClickTarget::Slot(11))
        );
    }

    #[test]
    fn mount_tamed_donkey_hit_test_uses_active_saddle_slot_only() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_add_entity(add_entity_with_type(42, 36));
        world.apply_set_entity_data(SetEntityData {
            id: 42,
            values: vec![byte_entity_data(
                TEST_MOUNT_TAME_FLAGS_DATA_ID,
                TEST_ABSTRACT_HORSE_TAME_FLAG,
            )],
        });
        world.apply_mount_screen_open(MountScreenOpen {
            container_id: 7,
            inventory_columns: 3,
            entity_id: 42,
        });

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(560.0, 295.0)), size),
            Some(InventoryClickTarget::Slot(0))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(560.0, 313.0)), size),
            Some(InventoryClickTarget::EmptyPanel)
        );
    }

    #[test]
    fn lectern_hit_test_uses_book_screen_and_page_buttons() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: LECTERN_MENU_TYPE_ID,
            title: "Lectern".to_string(),
        });

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(545.0, 265.0)), size),
            Some(InventoryClickTarget::EmptyPanel)
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(543.0, 264.0)), size),
            Some(InventoryClickTarget::Outside)
        );
        assert_eq!(
            lectern_button_at_position(&world, Some(PhysicalPosition::new(588.0, 422.0)), size),
            Some(LecternClickTarget::MenuButton(LECTERN_BUTTON_PREV_PAGE))
        );
        assert_eq!(
            lectern_button_at_position(&world, Some(PhysicalPosition::new(661.0, 422.0)), size),
            Some(LecternClickTarget::MenuButton(LECTERN_BUTTON_NEXT_PAGE))
        );
        assert_eq!(
            lectern_button_at_position(&world, Some(PhysicalPosition::new(560.0, 464.0)), size),
            Some(LecternClickTarget::Done)
        );
        assert_eq!(
            lectern_button_at_position(&world, Some(PhysicalPosition::new(660.0, 464.0)), size),
            Some(LecternClickTarget::MenuButton(LECTERN_BUTTON_TAKE_BOOK))
        );
    }

    #[test]
    fn shulker_box_hit_test_uses_vanilla_slots() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: 20,
            title: "Shulker Box".to_string(),
        });

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(560.0, 303.0)), size),
            Some(InventoryClickTarget::Slot(0))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 339.0)), size),
            Some(InventoryClickTarget::Slot(26))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
            Some(InventoryClickTarget::Slot(62))
        );
    }

    #[test]
    fn loom_hit_test_uses_vanilla_slots() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: LOOM_MENU_TYPE_ID,
            title: "Loom".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 40];
        items[0] = item_stack(42, 1);
        items[1] = item_stack(43, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(573.0, 311.0)), size),
            Some(InventoryClickTarget::Slot(0))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(593.0, 311.0)), size),
            Some(InventoryClickTarget::Slot(1))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(583.0, 330.0)), size),
            Some(InventoryClickTarget::Slot(2))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(703.0, 342.0)), size),
            Some(InventoryClickTarget::Slot(3))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
            Some(InventoryClickTarget::Slot(39))
        );
        assert_eq!(
            loom_click_target_at_position(
                &world,
                0,
                Some(PhysicalPosition::new(620.0, 296.0)),
                size
            ),
            Some(LoomClickTarget::Pattern(0))
        );
        assert_eq!(
            loom_click_target_at_position(
                &world,
                0,
                Some(PhysicalPosition::new(661.0, 339.0)),
                size
            ),
            Some(LoomClickTarget::Pattern(15))
        );
        assert!(loom_scroller_at_position(
            &world,
            Some(PhysicalPosition::new(674.0, 300.0)),
            size
        ));
    }

    #[test]
    fn loom_pattern_click_queues_container_button_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = loom_world_with_banner_and_dye();

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(632.0, 310.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(input.loom_selected_pattern_index(), Some(5));
        assert_eq!(counters.container_button_click_commands_queued, 1);
        assert_eq!(counters.container_click_commands_queued, 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerButtonClick(ContainerButtonClick {
                container_id: 7,
                button_id: 5,
            })
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn loom_pattern_scroll_changes_visible_button_indices() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = loom_world_with_banner_and_dye();

        assert!(handle_inventory_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, -2.0),
            Some(PhysicalPosition::new(620.0, 296.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert_eq!(input.loom_pattern_scroll_row(), 2);

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(620.0, 296.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(input.loom_selected_pattern_index(), Some(8));
        assert_eq!(counters.container_button_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerButtonClick(ContainerButtonClick {
                container_id: 7,
                button_id: 8,
            })
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn loom_scroller_drag_updates_visible_button_indices() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = loom_world_with_banner_and_dye();

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(674.0, 290.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert!(input.loom_pattern_scrolling);
        assert_eq!(input.loom_pattern_scroll_row(), 0);

        assert!(handle_inventory_cursor_moved(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            Some(PhysicalPosition::new(674.0, 328.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert!(input.loom_pattern_scrolling);
        assert_eq!(input.loom_pattern_scroll_row(), 3);

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Released,
            Some(PhysicalPosition::new(674.0, 328.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert!(!input.loom_pattern_scrolling);

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(620.0, 296.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(input.loom_selected_pattern_index(), Some(12));
        assert_eq!(counters.container_button_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerButtonClick(ContainerButtonClick {
                container_id: 7,
                button_id: 12,
            })
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn merchant_hit_test_uses_vanilla_slots() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: MERCHANT_MENU_TYPE_ID,
            title: "Merchant".to_string(),
        });

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(646.0, 322.0)), size),
            Some(InventoryClickTarget::Slot(0))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(672.0, 322.0)), size),
            Some(InventoryClickTarget::Slot(1))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(730.0, 322.0)), size),
            Some(InventoryClickTarget::Slot(2))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(618.0, 369.0)), size),
            Some(InventoryClickTarget::Slot(3))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(762.0, 427.0)), size),
            Some(InventoryClickTarget::Slot(38))
        );
    }

    #[test]
    fn smithing_hit_test_uses_vanilla_slots() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: SMITHING_MENU_TYPE_ID,
            title: "Smithing".to_string(),
        });

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(568.0, 333.0)), size),
            Some(InventoryClickTarget::Slot(0))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(586.0, 333.0)), size),
            Some(InventoryClickTarget::Slot(1))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(604.0, 333.0)), size),
            Some(InventoryClickTarget::Slot(2))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(658.0, 333.0)), size),
            Some(InventoryClickTarget::Slot(3))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
            Some(InventoryClickTarget::Slot(39))
        );
    }

    #[test]
    fn cartography_table_hit_test_uses_vanilla_slots() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: CARTOGRAPHY_TABLE_MENU_TYPE_ID,
            title: "Cartography Table".to_string(),
        });

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(575.0, 300.0)), size),
            Some(InventoryClickTarget::Slot(0))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(575.0, 337.0)), size),
            Some(InventoryClickTarget::Slot(1))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(705.0, 324.0)), size),
            Some(InventoryClickTarget::Slot(2))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
            Some(InventoryClickTarget::Slot(38))
        );
    }

    #[test]
    fn stonecutter_hit_test_uses_vanilla_slots() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: STONECUTTER_MENU_TYPE_ID,
            title: "Stonecutter".to_string(),
        });

        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(580.0, 318.0)), size),
            Some(InventoryClickTarget::Slot(0))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(703.0, 318.0)), size),
            Some(InventoryClickTarget::Slot(1))
        );
        assert_eq!(
            inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
            Some(InventoryClickTarget::Slot(37))
        );
    }

    #[test]
    fn stonecutter_recipe_grid_hit_test_uses_vanilla_first_page_buttons() {
        let size = PhysicalSize::new(1280, 720);
        let mut world = WorldStore::new();
        world.apply_update_recipes(UpdateRecipes {
            property_sets: Vec::new(),
            stonecutter_recipes: vec![stonecutter_recipe(vec![42])],
        });
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: STONECUTTER_MENU_TYPE_ID,
            title: "Stonecutter".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 38];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert_eq!(
            stonecutter_recipe_button_at_position(
                &world,
                0,
                Some(PhysicalPosition::new(612.0, 300.0)),
                size
            ),
            Some(0)
        );
        assert_eq!(
            stonecutter_recipe_button_at_position(
                &world,
                0,
                Some(PhysicalPosition::new(660.0, 336.0)),
                size
            ),
            Some(11)
        );
        assert_eq!(
            stonecutter_recipe_button_at_position(
                &world,
                0,
                Some(PhysicalPosition::new(669.0, 300.0)),
                size
            ),
            None
        );
    }

    #[test]
    fn stonecutter_recipe_button_click_queues_container_button_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_update_recipes(UpdateRecipes {
            property_sets: Vec::new(),
            stonecutter_recipes: (0..6).map(|_| stonecutter_recipe(vec![42])).collect(),
        });
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: STONECUTTER_MENU_TYPE_ID,
            title: "Stonecutter".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 38];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(628.0, 318.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_button_click_commands_queued, 1);
        assert_eq!(counters.container_click_commands_queued, 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerButtonClick(ContainerButtonClick {
                container_id: 7,
                button_id: 5,
            })
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn stonecutter_recipe_button_right_click_matches_vanilla_button_path() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_update_recipes(UpdateRecipes {
            property_sets: Vec::new(),
            stonecutter_recipes: (0..6).map(|_| stonecutter_recipe(vec![42])).collect(),
        });
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: STONECUTTER_MENU_TYPE_ID,
            title: "Stonecutter".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 38];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
            Some(PhysicalPosition::new(628.0, 318.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_button_click_commands_queued, 1);
        assert_eq!(counters.container_click_commands_queued, 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerButtonClick(ContainerButtonClick {
                container_id: 7,
                button_id: 5,
            })
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn stonecutter_mouse_wheel_scrolls_recipe_grid_button_index_by_rows() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_update_recipes(UpdateRecipes {
            property_sets: Vec::new(),
            stonecutter_recipes: (0..25).map(|_| stonecutter_recipe(vec![42])).collect(),
        });
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: STONECUTTER_MENU_TYPE_ID,
            title: "Stonecutter".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 38];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, -2.0),
            Some(PhysicalPosition::new(612.0, 300.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert_eq!(input.stonecutter_recipe_scroll_row, 2);

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(628.0, 336.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_button_click_commands_queued, 1);
        assert_eq!(counters.container_click_commands_queued, 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerButtonClick(ContainerButtonClick {
                container_id: 7,
                button_id: 17,
            })
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn stonecutter_scroller_drag_updates_recipe_grid_button_index() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_update_recipes(UpdateRecipes {
            property_sets: Vec::new(),
            stonecutter_recipes: (0..25).map(|_| stonecutter_recipe(vec![42])).collect(),
        });
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: STONECUTTER_MENU_TYPE_ID,
            title: "Stonecutter".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 38];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(676.0, 300.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert!(input.stonecutter_recipe_scrolling);
        assert_eq!(input.stonecutter_recipe_scroll_row, 0);

        assert!(handle_inventory_cursor_moved(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            Some(PhysicalPosition::new(676.0, 318.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert!(input.stonecutter_recipe_scrolling);
        assert_eq!(input.stonecutter_recipe_scroll_row, 2);
        assert_eq!(counters.container_button_click_commands_queued, 0);
        assert!(rx.try_recv().is_err());

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Released,
            Some(PhysicalPosition::new(676.0, 318.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert!(!input.stonecutter_recipe_scrolling);

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(628.0, 336.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_button_click_commands_queued, 1);
        assert_eq!(counters.container_click_commands_queued, 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerButtonClick(ContainerButtonClick {
                container_id: 7,
                button_id: 17,
            })
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn stonecutter_recipe_scroll_resets_when_input_item_changes() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        let mut stonecutter_recipes = Vec::new();
        stonecutter_recipes.extend((0..25).map(|_| stonecutter_recipe(vec![42])));
        stonecutter_recipes.extend((0..25).map(|_| stonecutter_recipe(vec![99])));
        world.apply_update_recipes(UpdateRecipes {
            property_sets: Vec::new(),
            stonecutter_recipes,
        });
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: STONECUTTER_MENU_TYPE_ID,
            title: "Stonecutter".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 38];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });
        assert!(handle_inventory_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, -2.0),
            Some(PhysicalPosition::new(612.0, 300.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert_eq!(input.stonecutter_recipe_scroll_row, 2);

        let mut replacement_items = vec![ItemStackSummary::empty(); 38];
        replacement_items[0] = item_stack(99, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 13,
            items: replacement_items,
            carried_item: ItemStackSummary::empty(),
        });
        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(612.0, 300.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(input.stonecutter_recipe_scroll_row, 0);
        assert_eq!(counters.container_button_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerButtonClick(ContainerButtonClick {
                container_id: 7,
                button_id: 0,
            })
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn stonecutter_recipe_button_click_requires_matching_input_recipe() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_update_recipes(UpdateRecipes {
            property_sets: Vec::new(),
            stonecutter_recipes: vec![stonecutter_recipe(vec![42])],
        });
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: STONECUTTER_MENU_TYPE_ID,
            title: "Stonecutter".to_string(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(612.0, 300.0)),
            PhysicalSize::new(1280, 720),
        ));

        let mut items = vec![ItemStackSummary::empty(); 38];
        items[0] = item_stack(99, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(612.0, 300.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_button_click_commands_queued, 0);
        assert_eq!(counters.container_click_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn stonecutter_recipe_button_click_ignores_already_selected_recipe() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_update_recipes(UpdateRecipes {
            property_sets: Vec::new(),
            stonecutter_recipes: (0..6).map(|_| stonecutter_recipe(vec![42])).collect(),
        });
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: STONECUTTER_MENU_TYPE_ID,
            title: "Stonecutter".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 38];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });
        world.apply_container_set_data(ContainerSetData {
            container_id: 7,
            id: STONECUTTER_SELECTED_RECIPE_DATA_ID,
            value: 5,
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(628.0, 318.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_button_click_commands_queued, 0);
        assert_eq!(counters.container_click_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn crafter_empty_grid_click_queues_slot_state_change_and_pickup() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: CRAFTER_MENU_TYPE_ID,
            title: "Crafter".to_string(),
        });
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![ItemStackSummary::empty(); 46],
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(586.0, 302.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_slot_state_changed_commands_queued, 1);
        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerSlotStateChanged(ContainerSlotStateChanged {
                slot_id: 0,
                container_id: 7,
                new_state: false,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 0,
                button_num: 0,
                input: ContainerInput::Pickup,
                changed_slots: BTreeMap::new(),
                carried_item: HashedStack::Empty,
            })
        );
        assert_eq!(world.open_container_data_value(0), None);
    }

    #[test]
    fn enchantment_table_option_click_queues_button_command_when_cost_is_available() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: ENCHANTMENT_MENU_TYPE_ID,
            title: "Enchanting Table".to_string(),
        });
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![ItemStackSummary::empty(); 38],
            carried_item: ItemStackSummary::empty(),
        });
        world.apply_container_set_data(ContainerSetData {
            container_id: 7,
            id: 2,
            value: 30,
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(620.0, 334.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_button_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerButtonClick(ContainerButtonClick {
                container_id: 7,
                button_id: 2,
            })
        );
        assert_eq!(counters.container_click_commands_queued, 0);
    }

    #[test]
    fn enchantment_table_option_click_ignores_zero_cost_buttons() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: ENCHANTMENT_MENU_TYPE_ID,
            title: "Enchanting Table".to_string(),
        });
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![ItemStackSummary::empty(); 38],
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(620.0, 296.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_button_click_commands_queued, 0);
        assert_eq!(counters.container_click_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn beacon_confirm_click_queues_set_beacon_then_close_when_active() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: BEACON_MENU_TYPE_ID,
            title: "Beacon".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 37];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });
        world.apply_container_set_data(ContainerSetData {
            container_id: 7,
            id: BEACON_PRIMARY_EFFECT_DATA_ID,
            value: 5,
        });
        world.apply_container_set_data(ContainerSetData {
            container_id: 7,
            id: BEACON_SECONDARY_EFFECT_DATA_ID,
            value: 8,
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(700.0, 369.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.set_beacon_commands_queued, 1);
        assert_eq!(counters.container_close_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SetBeacon(SetBeacon {
                primary_effect: Some(4),
                secondary_effect: Some(7),
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClose(ContainerCloseRequest { container_id: 7 })
        );
        assert!(world.inventory().open_container.is_none());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn beacon_effect_clicks_update_local_selection_and_confirm_submits_it() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: BEACON_MENU_TYPE_ID,
            title: "Beacon".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 37];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });
        world.apply_container_set_data(ContainerSetData {
            container_id: 7,
            id: BEACON_LEVELS_DATA_ID,
            value: 4,
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(600.0, 333.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert_eq!(input.beacon_effect_selection(), (Some(4), None));
        assert_eq!(counters.set_beacon_commands_queued, 0);
        assert!(rx.try_recv().is_err());

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(704.0, 308.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert_eq!(input.beacon_effect_selection(), (Some(4), Some(4)));
        assert_eq!(counters.set_beacon_commands_queued, 0);
        assert!(rx.try_recv().is_err());

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(700.0, 369.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.set_beacon_commands_queued, 1);
        assert_eq!(counters.container_close_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SetBeacon(SetBeacon {
                primary_effect: Some(BEACON_EFFECT_STRENGTH_ID),
                secondary_effect: Some(BEACON_EFFECT_STRENGTH_ID),
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClose(ContainerCloseRequest { container_id: 7 })
        );
        assert!(world.inventory().open_container.is_none());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn beacon_effect_click_ignores_power_buttons_above_current_level() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: BEACON_MENU_TYPE_ID,
            title: "Beacon".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 37];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });
        world.apply_container_set_data(ContainerSetData {
            container_id: 7,
            id: BEACON_LEVELS_DATA_ID,
            value: 1,
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(600.0, 333.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert_eq!(input.beacon_effect_selection(), (None, None));

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(700.0, 369.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.set_beacon_commands_queued, 0);
        assert_eq!(counters.container_close_commands_queued, 0);
        assert!(world.inventory().open_container.is_some());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn beacon_confirm_click_ignores_disabled_button_without_payment() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: BEACON_MENU_TYPE_ID,
            title: "Beacon".to_string(),
        });
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![ItemStackSummary::empty(); 37],
            carried_item: ItemStackSummary::empty(),
        });
        world.apply_container_set_data(ContainerSetData {
            container_id: 7,
            id: BEACON_PRIMARY_EFFECT_DATA_ID,
            value: 5,
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(700.0, 369.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.set_beacon_commands_queued, 0);
        assert_eq!(counters.container_close_commands_queued, 0);
        assert!(world.inventory().open_container.is_some());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn beacon_cancel_click_queues_container_close_request() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: BEACON_MENU_TYPE_ID,
            title: "Beacon".to_string(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(726.0, 369.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_close_commands_queued, 1);
        assert_eq!(counters.set_beacon_commands_queued, 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClose(ContainerCloseRequest { container_id: 7 })
        );
        assert!(world.inventory().open_container.is_none());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn lectern_page_button_click_queues_container_button_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: LECTERN_MENU_TYPE_ID,
            title: "Lectern".to_string(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(661.0, 422.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_button_click_commands_queued, 1);
        assert_eq!(counters.container_click_commands_queued, 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerButtonClick(ContainerButtonClick {
                container_id: 7,
                button_id: LECTERN_BUTTON_NEXT_PAGE,
            })
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn lectern_done_button_queues_container_close_request() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: LECTERN_MENU_TYPE_ID,
            title: "Lectern".to_string(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(560.0, 464.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_close_commands_queued, 1);
        assert_eq!(counters.container_button_click_commands_queued, 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClose(ContainerCloseRequest { container_id: 7 })
        );
        assert!(world.inventory().open_container.is_none());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn lectern_take_book_button_queues_container_button_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: LECTERN_MENU_TYPE_ID,
            title: "Lectern".to_string(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(660.0, 464.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_button_click_commands_queued, 1);
        assert_eq!(counters.container_close_commands_queued, 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerButtonClick(ContainerButtonClick {
                container_id: 7,
                button_id: LECTERN_BUTTON_TAKE_BOOK,
            })
        );
        assert!(world.inventory().open_container.is_some());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn merchant_trade_row_click_queues_select_trade_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: MERCHANT_MENU_TYPE_ID,
            title: "Merchant".to_string(),
        });
        assert!(world.apply_merchant_offers(merchant_offers(7, 4)));

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(545.0, 365.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.select_trade_commands_queued, 1);
        assert_eq!(counters.container_click_commands_queued, 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectTrade(SelectTradeCommand { item: 3 })
        );
        assert_eq!(
            world
                .inventory()
                .open_container
                .as_ref()
                .and_then(|container| container.merchant_offers.as_ref())
                .map(|offers| offers.local_selected_offer_index),
            Some(3)
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn merchant_mouse_wheel_scrolls_visible_trade_window() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: MERCHANT_MENU_TYPE_ID,
            title: "Merchant".to_string(),
        });
        assert!(world.apply_merchant_offers(merchant_offers(7, 8)));

        assert!(handle_inventory_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, -1.0),
            Some(PhysicalPosition::new(545.0, 365.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.select_trade_commands_queued, 0);
        assert_eq!(counters.container_click_commands_queued, 0);
        assert_eq!(
            world
                .inventory()
                .open_container
                .as_ref()
                .and_then(|container| container.merchant_offers.as_ref())
                .map(|offers| offers.local_scroll_offset),
            Some(1)
        );
        assert!(rx.try_recv().is_err());

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(545.0, 365.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.select_trade_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectTrade(SelectTradeCommand { item: 4 })
        );
        assert_eq!(
            world
                .inventory()
                .open_container
                .as_ref()
                .and_then(|container| container.merchant_offers.as_ref())
                .map(|offers| offers.local_selected_offer_index),
            Some(4)
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn merchant_scroller_drag_updates_visible_trade_window() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: MERCHANT_MENU_TYPE_ID,
            title: "Merchant".to_string(),
        });
        assert!(world.apply_merchant_offers(merchant_offers(7, 12)));

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(598.0, 296.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert!(input.merchant_trade_scrolling);

        assert!(handle_inventory_cursor_moved(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            Some(PhysicalPosition::new(598.0, 420.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert!(input.merchant_trade_scrolling);
        assert_eq!(counters.select_trade_commands_queued, 0);
        assert_eq!(counters.container_click_commands_queued, 0);
        assert_eq!(
            world
                .inventory()
                .open_container
                .as_ref()
                .and_then(|container| container.merchant_offers.as_ref())
                .map(|offers| offers.local_scroll_offset),
            Some(5)
        );
        assert!(rx.try_recv().is_err());

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Released,
            Some(PhysicalPosition::new(598.0, 420.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert!(!input.merchant_trade_scrolling);

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(545.0, 300.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.select_trade_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectTrade(SelectTradeCommand { item: 5 })
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn merchant_trade_row_click_ignores_missing_offer() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: MERCHANT_MENU_TYPE_ID,
            title: "Merchant".to_string(),
        });
        assert!(world.apply_merchant_offers(merchant_offers(7, 2)));

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(545.0, 365.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.select_trade_commands_queued, 0);
        assert_eq!(counters.container_click_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn generic_container_mouse_click_queues_pickup() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: 5,
            title: "Large Chest".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 90];
        items[0] = item_stack(42, 3);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(560.0, 267.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 0,
                button_num: 0,
                input: ContainerInput::Pickup,
                changed_slots: [(0, HashedStack::Empty)].into(),
                carried_item: HashedStack::Item(HashedItemStack {
                    item_id: 42,
                    count: 3,
                    components: HashedComponentPatch::default(),
                }),
            })
        );
        assert_eq!(
            world.inventory().open_container.as_ref().unwrap().slots[0].item,
            ItemStackSummary::empty()
        );
    }

    #[test]
    fn mount_horse_mouse_click_queues_pickup() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_add_entity(add_entity_with_type(42, 66));
        world.apply_mount_screen_open(MountScreenOpen {
            container_id: 7,
            inventory_columns: 5,
            entity_id: 42,
        });
        let mut items = vec![ItemStackSummary::empty(); 53];
        items[2] = item_stack(42, 3);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(632.0, 295.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 2,
                button_num: 0,
                input: ContainerInput::Pickup,
                changed_slots: [(2, HashedStack::Empty)].into(),
                carried_item: HashedStack::Item(HashedItemStack {
                    item_id: 42,
                    count: 3,
                    components: HashedComponentPatch::default(),
                }),
            })
        );
        assert_eq!(
            world.inventory().open_container.as_ref().unwrap().slots[2].item,
            ItemStackSummary::empty()
        );
    }

    #[test]
    fn mount_horse_shift_click_queues_server_authoritative_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_add_entity(add_entity_with_type(42, 66));
        world.apply_mount_screen_open(MountScreenOpen {
            container_id: 7,
            inventory_columns: 5,
            entity_id: 42,
        });
        let mut items = vec![ItemStackSummary::empty(); 53];
        items[2] = item_stack(42, 3);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(632.0, 295.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 2,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: BTreeMap::new(),
                carried_item: HashedStack::Empty,
            })
        );
        assert_eq!(
            world.inventory().open_container.as_ref().unwrap().slots[2].item,
            item_stack(42, 3)
        );
    }

    #[test]
    fn furnace_mouse_click_queues_pickup() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: FURNACE_MENU_TYPE_ID,
            title: "Furnace".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 39];
        items[0] = item_stack(42, 3);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(616.0, 302.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 0,
                button_num: 0,
                input: ContainerInput::Pickup,
                changed_slots: [(0, HashedStack::Empty)].into(),
                carried_item: HashedStack::Item(HashedItemStack {
                    item_id: 42,
                    count: 3,
                    components: HashedComponentPatch::default(),
                }),
            })
        );
        assert_eq!(
            world.inventory().open_container.as_ref().unwrap().slots[0].item,
            ItemStackSummary::empty()
        );
    }

    #[test]
    fn furnace_shift_click_queues_quick_move_to_input_slot() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_update_recipes(UpdateRecipes {
            property_sets: vec![RecipePropertySetSummary {
                key: "minecraft:furnace_input".to_string(),
                item_ids: vec![42],
            }],
            stonecutter_recipes: Vec::new(),
        });
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: FURNACE_MENU_TYPE_ID,
            title: "Furnace".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 39];
        items[3] = item_stack(42, 3);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(560.0, 361.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 3,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Item(hashed_item(42, 3))),
                    (3, HashedStack::Empty),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, item_stack(42, 3));
        assert_eq!(slots[3].item, ItemStackSummary::empty());
    }

    #[test]
    fn generic_container_shift_click_queues_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: 5,
            title: "Large Chest".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 90];
        items[0] = item_stack(42, 3);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(560.0, 267.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 0,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Empty),
                    (89, HashedStack::Item(hashed_item(42, 3))),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, ItemStackSummary::empty());
        assert_eq!(slots[89].item, item_stack(42, 3));
    }

    #[test]
    fn generic_3x3_shift_click_queues_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: 6,
            title: "Dispenser".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 45];
        items[0] = item_stack(42, 3);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(614.0, 294.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 0,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Empty),
                    (44, HashedStack::Item(hashed_item(42, 3))),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, ItemStackSummary::empty());
        assert_eq!(slots[44].item, item_stack(42, 3));
    }

    #[test]
    fn crafting_table_shift_click_input_slot_queues_predicted_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: CRAFTING_MENU_TYPE_ID,
            title: "Crafting".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 46];
        items[1] = item_stack(42, 3);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(590.0, 302.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 1,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (1, HashedStack::Empty),
                    (10, HashedStack::Item(hashed_item(42, 3))),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[1].item, ItemStackSummary::empty());
        assert_eq!(slots[10].item, item_stack(42, 3));
    }

    #[test]
    fn enchantment_table_shift_click_input_slot_queues_predicted_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: ENCHANTMENT_MENU_TYPE_ID,
            title: "Enchanting Table".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 38];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(575.0, 332.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 0,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Empty),
                    (37, HashedStack::Item(hashed_item(42, 1))),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, ItemStackSummary::empty());
        assert_eq!(slots[37].item, item_stack(42, 1));
    }

    #[test]
    fn enchantment_table_shift_click_player_lapis_queues_predicted_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.set_enchantment_lapis_lazuli_item_ids(BTreeSet::from([43]));
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: ENCHANTMENT_MENU_TYPE_ID,
            title: "Enchanting Table".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 38];
        items[37] = item_stack(43, 3);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(712.0, 427.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 37,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (1, HashedStack::Item(hashed_item(43, 3))),
                    (37, HashedStack::Empty),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[1].item, item_stack(43, 3));
        assert_eq!(slots[37].item, ItemStackSummary::empty());
    }

    #[test]
    fn enchantment_table_shift_click_player_item_queues_predicted_input_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.set_enchantment_lapis_lazuli_item_ids(BTreeSet::from([43]));
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: ENCHANTMENT_MENU_TYPE_ID,
            title: "Enchanting Table".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 38];
        items[37] = item_stack(50, 3);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(712.0, 427.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 37,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Item(hashed_item(50, 1))),
                    (37, HashedStack::Item(hashed_item(50, 2))),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, item_stack(50, 1));
        assert_eq!(slots[37].item, item_stack(50, 2));
    }

    #[test]
    fn non_local_quick_move_with_unhashable_prediction_falls_back_to_server_click() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: CRAFTING_MENU_TYPE_ID,
            title: "Crafting".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 46];
        items[1] = bundle_stack(42, 3, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(590.0, 302.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.select_bundle_item_commands_queued, 1);
        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectBundleItem(SelectBundleItem {
                slot_id: 1,
                selected_item_index: -1,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 1,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: BTreeMap::new(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[1].item, bundle_stack(42, 3, 1));
        assert_eq!(slots[10].item, ItemStackSummary::empty());
    }

    #[test]
    fn crafting_table_shift_click_result_slot_queues_server_authoritative_click() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: CRAFTING_MENU_TYPE_ID,
            title: "Crafting".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 46];
        items[0] = item_stack(90, 1);
        items[1] = item_stack(42, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(684.0, 320.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 13,
                slot_num: 0,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: BTreeMap::new(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, item_stack(90, 1));
        assert_eq!(slots[1].item, item_stack(42, 1));
    }

    #[test]
    fn anvil_shift_click_player_item_queues_predicted_input_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: ANVIL_MENU_TYPE_ID,
            title: "Anvil".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 39];
        items[30] = item_stack(42, 3);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(560.0, 427.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 30,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Item(hashed_item(42, 3))),
                    (30, HashedStack::Empty),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, item_stack(42, 3));
        assert_eq!(slots[30].item, ItemStackSummary::empty());
    }

    #[test]
    fn anvil_shift_click_input_slot_queues_predicted_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: ANVIL_MENU_TYPE_ID,
            title: "Anvil".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 39];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(587.0, 332.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 0,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Empty),
                    (3, HashedStack::Item(hashed_item(42, 1))),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, ItemStackSummary::empty());
        assert_eq!(slots[3].item, item_stack(42, 1));
    }

    #[test]
    fn smithing_shift_click_input_slot_queues_predicted_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: SMITHING_MENU_TYPE_ID,
            title: "Smithing".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 40];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(568.0, 333.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 0,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Empty),
                    (4, HashedStack::Item(hashed_item(42, 1))),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, ItemStackSummary::empty());
        assert_eq!(slots[4].item, item_stack(42, 1));
    }

    #[test]
    fn smithing_shift_click_player_template_queues_predicted_input_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_update_recipes(UpdateRecipes {
            property_sets: vec![
                RecipePropertySetSummary {
                    key: "minecraft:smithing_template".to_string(),
                    item_ids: vec![42],
                },
                RecipePropertySetSummary {
                    key: "minecraft:smithing_base".to_string(),
                    item_ids: vec![43],
                },
                RecipePropertySetSummary {
                    key: "minecraft:smithing_addition".to_string(),
                    item_ids: vec![44],
                },
            ],
            stonecutter_recipes: Vec::new(),
        });
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: SMITHING_MENU_TYPE_ID,
            title: "Smithing".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 40];
        items[31] = item_stack(42, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(568.0, 427.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 31,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Item(hashed_item(42, 1))),
                    (31, HashedStack::Empty),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, item_stack(42, 1));
        assert_eq!(slots[31].item, ItemStackSummary::empty());
    }

    #[test]
    fn merchant_shift_click_payment_slot_queues_predicted_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: MERCHANT_MENU_TYPE_ID,
            title: "Merchant".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 39];
        items[0] = item_stack(42, 3);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(646.0, 322.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 0,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Empty),
                    (3, HashedStack::Item(hashed_item(42, 3))),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, ItemStackSummary::empty());
        assert_eq!(slots[3].item, item_stack(42, 3));
    }

    #[test]
    fn beacon_shift_click_single_payment_item_queues_predicted_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        apply_item_tags(
            &mut world,
            vec![("minecraft:beacon_payment_items", vec![42])],
        );
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: BEACON_MENU_TYPE_ID,
            title: "Beacon".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 37];
        items[1] = item_stack(42, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(569.0, 396.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 1,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Item(hashed_item(42, 1))),
                    (1, HashedStack::Empty),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, item_stack(42, 1));
        assert_eq!(slots[1].item, ItemStackSummary::empty());
    }

    #[test]
    fn brewing_stand_shift_click_potion_item_queues_predicted_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.set_default_item_max_stack_sizes(BTreeMap::from([(42, 64)]));
        world.set_brewing_potion_item_ids(BTreeSet::from([42]));
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: BREWING_STAND_MENU_TYPE_ID,
            title: "Brewing Stand".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 41];
        items[32] = item_stack(42, 3);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(560.0, 427.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 32,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Item(hashed_item(42, 1))),
                    (32, HashedStack::Item(hashed_item(42, 2))),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, item_stack(42, 1));
        assert_eq!(slots[32].item, item_stack(42, 2));
    }

    #[test]
    fn grindstone_shift_click_input_slot_queues_predicted_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: GRINDSTONE_MENU_TYPE_ID,
            title: "Grindstone".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 39];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(609.0, 304.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 0,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Empty),
                    (3, HashedStack::Item(hashed_item(42, 1))),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, ItemStackSummary::empty());
        assert_eq!(slots[3].item, item_stack(42, 1));
    }

    #[test]
    fn grindstone_shift_click_player_to_input_queues_server_authoritative_click() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: GRINDSTONE_MENU_TYPE_ID,
            title: "Grindstone".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 39];
        items[3] = item_stack(42, 3);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(568.0, 369.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 3,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: BTreeMap::new(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, ItemStackSummary::empty());
        assert_eq!(slots[3].item, item_stack(42, 3));
    }

    #[test]
    fn grindstone_shift_click_default_damageable_player_item_queues_predicted_input_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.set_default_damageable_item_ids(BTreeSet::from([42]));
        world.set_default_item_max_stack_sizes(BTreeMap::from([(42, 1)]));
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: GRINDSTONE_MENU_TYPE_ID,
            title: "Grindstone".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 39];
        items[3] = item_stack(42, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(568.0, 369.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 13,
                slot_num: 3,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Item(hashed_item(42, 1))),
                    (3, HashedStack::Empty),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, item_stack(42, 1));
        assert_eq!(slots[3].item, ItemStackSummary::empty());
    }

    #[test]
    fn grindstone_shift_click_player_range_queues_predicted_quick_move_when_inputs_full() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: GRINDSTONE_MENU_TYPE_ID,
            title: "Grindstone".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 39];
        items[0] = item_stack(10, 1);
        items[1] = item_stack(11, 1);
        items[3] = item_stack(42, 3);
        items[30] = item_stack(43, 4);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(568.0, 369.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 13,
                slot_num: 3,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (3, HashedStack::Empty),
                    (31, HashedStack::Item(hashed_item(42, 3))),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[3].item, ItemStackSummary::empty());
        assert_eq!(slots[31].item, item_stack(42, 3));
    }

    #[test]
    fn grindstone_shift_click_result_slot_queues_server_authoritative_click() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: GRINDSTONE_MENU_TYPE_ID,
            title: "Grindstone".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 39];
        items[0] = item_stack(42, 1);
        items[2] = item_stack(90, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 14,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(689.0, 319.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 14,
                slot_num: 2,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: BTreeMap::new(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, item_stack(42, 1));
        assert_eq!(slots[2].item, item_stack(90, 1));
    }

    #[test]
    fn stonecutter_shift_click_input_slot_queues_predicted_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: STONECUTTER_MENU_TYPE_ID,
            title: "Stonecutter".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 38];
        items[0] = item_stack(42, 3);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(580.0, 318.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 0,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Empty),
                    (2, HashedStack::Item(hashed_item(42, 3))),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, ItemStackSummary::empty());
        assert_eq!(slots[2].item, item_stack(42, 3));
    }

    #[test]
    fn stonecutter_shift_click_valid_recipe_input_queues_predicted_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_update_recipes(UpdateRecipes {
            property_sets: Vec::new(),
            stonecutter_recipes: vec![stonecutter_recipe(vec![42])],
        });
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: STONECUTTER_MENU_TYPE_ID,
            title: "Stonecutter".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 38];
        items[2] = item_stack(42, 3);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(568.0, 369.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 13,
                slot_num: 2,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Item(hashed_item(42, 3))),
                    (2, HashedStack::Empty),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, item_stack(42, 3));
        assert_eq!(slots[2].item, ItemStackSummary::empty());
    }

    #[test]
    fn stonecutter_shift_click_result_slot_queues_server_authoritative_click() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: STONECUTTER_MENU_TYPE_ID,
            title: "Stonecutter".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 38];
        items[0] = item_stack(42, 1);
        items[1] = item_stack(90, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 14,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(703.0, 318.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 14,
                slot_num: 1,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: BTreeMap::new(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, item_stack(42, 1));
        assert_eq!(slots[1].item, item_stack(90, 1));
    }

    #[test]
    fn cartography_table_shift_click_input_slots_queue_predicted_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: CARTOGRAPHY_TABLE_MENU_TYPE_ID,
            title: "Cartography Table".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 39];
        items[0] = item_stack(42, 1);
        items[1] = item_stack(43, 3);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(575.0, 300.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 0,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Empty),
                    (3, HashedStack::Item(hashed_item(42, 1))),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(575.0, 337.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 2);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 1,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (1, HashedStack::Empty),
                    (4, HashedStack::Item(hashed_item(43, 3))),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, ItemStackSummary::empty());
        assert_eq!(slots[1].item, ItemStackSummary::empty());
        assert_eq!(slots[3].item, item_stack(42, 1));
        assert_eq!(slots[4].item, item_stack(43, 3));
    }

    #[test]
    fn cartography_table_shift_click_player_additional_item_queues_predicted_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.set_cartography_additional_item_ids(BTreeSet::from([43, 44, 45]));
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: CARTOGRAPHY_TABLE_MENU_TYPE_ID,
            title: "Cartography Table".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 39];
        items[38] = item_stack(43, 2);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(712.0, 427.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 38,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (1, HashedStack::Item(hashed_item(43, 2))),
                    (38, HashedStack::Empty),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[1].item, item_stack(43, 2));
        assert_eq!(slots[38].item, ItemStackSummary::empty());
    }

    #[test]
    fn loom_shift_click_input_slots_queue_predicted_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: LOOM_MENU_TYPE_ID,
            title: "Loom".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 40];
        items[0] = item_stack(42, 3);
        items[1] = item_stack(43, 2);
        items[2] = item_stack(44, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(573.0, 311.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 0,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Empty),
                    (4, HashedStack::Item(hashed_item(42, 3))),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(593.0, 311.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 2);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 1,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (1, HashedStack::Empty),
                    (5, HashedStack::Item(hashed_item(43, 2))),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(583.0, 330.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 3);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 2,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (2, HashedStack::Empty),
                    (6, HashedStack::Item(hashed_item(44, 1))),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, ItemStackSummary::empty());
        assert_eq!(slots[1].item, ItemStackSummary::empty());
        assert_eq!(slots[2].item, ItemStackSummary::empty());
        assert_eq!(slots[4].item, item_stack(42, 3));
        assert_eq!(slots[5].item, item_stack(43, 2));
        assert_eq!(slots[6].item, item_stack(44, 1));
    }

    #[test]
    fn hopper_shift_click_queues_bidirectional_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: 16,
            title: "Hopper".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 41];
        items[0] = item_stack(42, 3);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(604.0, 314.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 0,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Empty),
                    (40, HashedStack::Item(hashed_item(42, 3))),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, ItemStackSummary::empty());
        assert_eq!(slots[40].item, item_stack(42, 3));

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(704.0, 403.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 2);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 40,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Item(hashed_item(42, 3))),
                    (40, HashedStack::Empty),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, item_stack(42, 3));
        assert_eq!(slots[40].item, ItemStackSummary::empty());
    }

    #[test]
    fn shulker_box_shift_click_queues_bidirectional_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: 20,
            title: "Shulker Box".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 63];
        items[0] = item_stack(42, 3);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(560.0, 303.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 0,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Empty),
                    (62, HashedStack::Item(hashed_item(42, 3))),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, ItemStackSummary::empty());
        assert_eq!(slots[62].item, item_stack(42, 3));

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(712.0, 427.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 2);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 62,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: [
                    (0, HashedStack::Item(hashed_item(42, 3))),
                    (62, HashedStack::Empty),
                ]
                .into(),
                carried_item: HashedStack::Empty,
            })
        );
        let slots = &world.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, item_stack(42, 3));
        assert_eq!(slots[62].item, ItemStackSummary::empty());
    }

    #[test]
    fn inventory_mouse_click_queues_container_zero_pickup() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: item_stack(42, 3),
        });
        assert!(world.open_local_inventory());

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(560.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 0,
                state_id: 0,
                slot_num: 36,
                button_num: 0,
                input: ContainerInput::Pickup,
                changed_slots: [(36, HashedStack::Empty)].into(),
                carried_item: HashedStack::Item(HashedItemStack {
                    item_id: 42,
                    count: 3,
                    components: HashedComponentPatch::default(),
                }),
            })
        );
    }

    #[test]
    fn creative_inventory_middle_click_queues_container_zero_clone() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        apply_instabuild_abilities(&mut world);
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: item_stack(42, 3),
        });
        assert!(world.open_local_inventory());

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Middle,
            ElementState::Pressed,
            Some(PhysicalPosition::new(560.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 0,
                state_id: 0,
                slot_num: 36,
                button_num: 2,
                input: ContainerInput::Clone,
                changed_slots: [].into(),
                carried_item: HashedStack::Item(hashed_item(42, 64)),
            })
        );
        assert_eq!(world.inventory().cursor_item, item_stack(42, 64));
    }

    #[test]
    fn creative_server_opened_container_middle_click_queues_clone() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        apply_instabuild_abilities(&mut world);
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: GENERIC_CONTAINER_FIRST_MENU_TYPE_ID,
            title: "Chest".to_string(),
        });
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![item_stack(42, 3)],
            carried_item: ItemStackSummary::empty(),
        });

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Middle,
            ElementState::Pressed,
            Some(PhysicalPosition::new(568.0, 320.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 0,
                button_num: 2,
                input: ContainerInput::Clone,
                changed_slots: [].into(),
                carried_item: HashedStack::Item(hashed_item(42, 64)),
            })
        );
        assert_eq!(world.inventory().cursor_item, item_stack(42, 64));
    }

    #[test]
    fn inventory_middle_click_without_instabuild_is_consumed_without_packet() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: item_stack(42, 3),
        });
        assert!(world.open_local_inventory());

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Middle,
            ElementState::Pressed,
            Some(PhysicalPosition::new(560.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 0);
        assert_eq!(world.inventory().cursor_item, ItemStackSummary::empty());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn inventory_double_left_click_queues_container_zero_pickup_all() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.inventory_last_click_slot = Some(36);
        input.inventory_last_click_button_num = Some(0);
        input.inventory_last_click_at = Some(Instant::now() - Duration::from_millis(1));
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_set_cursor_item(SetCursorItem {
            item: item_stack(42, 4),
        });
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 1,
            item: item_stack(42, 3),
        });
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 9,
            item: item_stack(42, 5),
        });
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 2,
            item: item_stack(43, 7),
        });
        assert!(world.open_local_inventory());

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(560.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(input.inventory_last_click_slot, Some(36));
        assert_eq!(input.inventory_last_click_button_num, Some(0));
        match rx.try_recv().unwrap() {
            NetCommand::ContainerClick(click) => {
                assert_eq!(click.container_id, 0);
                assert_eq!(click.state_id, 0);
                assert_eq!(click.slot_num, 36);
                assert_eq!(click.button_num, 0);
                assert_eq!(click.input, ContainerInput::PickupAll);
                assert_eq!(
                    click.changed_slots,
                    [(9, HashedStack::Empty), (37, HashedStack::Empty)].into()
                );
                assert_eq!(
                    click.carried_item,
                    HashedStack::Item(HashedItemStack {
                        item_id: 42,
                        count: 12,
                        components: HashedComponentPatch::default(),
                    })
                );
            }
            command => panic!("expected container click command, got {command:?}"),
        }
        assert_eq!(world.inventory().cursor_item, item_stack(42, 12));
        assert_eq!(player_slot_item(&world, 1), ItemStackSummary::empty());
        assert_eq!(player_slot_item(&world, 9), ItemStackSummary::empty());
        assert_eq!(player_slot_item(&world, 2), item_stack(43, 7));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn inventory_double_click_requires_left_button_and_vanilla_threshold() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.inventory_last_click_slot = Some(36);
        input.inventory_last_click_button_num = Some(1);
        input.inventory_last_click_at = Some(Instant::now() - Duration::from_millis(1));
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_set_cursor_item(SetCursorItem {
            item: item_stack(42, 4),
        });
        assert!(world.open_local_inventory());

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
            Some(PhysicalPosition::new(560.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert!(rx.try_recv().is_err());

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Released,
            Some(PhysicalPosition::new(560.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));

        match rx.try_recv().unwrap() {
            NetCommand::ContainerClick(click) => {
                assert_eq!(click.slot_num, 36);
                assert_eq!(click.button_num, 1);
                assert_eq!(click.input, ContainerInput::Pickup);
            }
            command => panic!("expected container click command, got {command:?}"),
        }

        input.inventory_last_click_slot = Some(37);
        input.inventory_last_click_button_num = Some(0);
        input.inventory_last_click_at = Some(Instant::now() - VANILLA_DOUBLE_CLICK_THRESHOLD);

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(580.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert!(rx.try_recv().is_err());

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Released,
            Some(PhysicalPosition::new(580.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));

        match rx.try_recv().unwrap() {
            NetCommand::ContainerClick(click) => {
                assert_eq!(click.slot_num, 37);
                assert_eq!(click.button_num, 0);
                assert_eq!(click.input, ContainerInput::Pickup);
            }
            command => panic!("expected container click command, got {command:?}"),
        }
        assert_eq!(counters.container_click_commands_queued, 2);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn inventory_left_drag_queues_quick_craft_sequence() {
        let (tx, mut rx) = mpsc::channel(8);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_set_cursor_item(SetCursorItem {
            item: item_stack(42, 8),
        });
        assert!(world.open_local_inventory());

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(560.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert!(rx.try_recv().is_err());
        assert!(handle_inventory_cursor_moved(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            Some(PhysicalPosition::new(560.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert!(handle_inventory_cursor_moved(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            Some(PhysicalPosition::new(580.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Released,
            Some(PhysicalPosition::new(580.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 4);
        assert_quick_craft_click(
            &mut rx,
            -999,
            0,
            [].into(),
            HashedStack::Item(hashed_item(42, 8)),
        );
        assert_quick_craft_click(
            &mut rx,
            36,
            1,
            [].into(),
            HashedStack::Item(hashed_item(42, 8)),
        );
        assert_quick_craft_click(
            &mut rx,
            37,
            1,
            [].into(),
            HashedStack::Item(hashed_item(42, 8)),
        );
        assert_quick_craft_click(
            &mut rx,
            -999,
            2,
            [
                (36, HashedStack::Item(hashed_item(42, 4))),
                (37, HashedStack::Item(hashed_item(42, 4))),
            ]
            .into(),
            HashedStack::Empty,
        );
        assert!(rx.try_recv().is_err());
        assert_eq!(world.inventory().cursor_item, ItemStackSummary::empty());
        assert_eq!(player_slot_item(&world, 0), item_stack(42, 4));
        assert_eq!(player_slot_item(&world, 1), item_stack(42, 4));
    }

    #[test]
    fn inventory_right_drag_queues_quick_craft_one_per_slot() {
        let (tx, mut rx) = mpsc::channel(8);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_set_cursor_item(SetCursorItem {
            item: item_stack(42, 8),
        });
        assert!(world.open_local_inventory());

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
            Some(PhysicalPosition::new(560.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert!(handle_inventory_cursor_moved(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            Some(PhysicalPosition::new(560.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert!(handle_inventory_cursor_moved(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            Some(PhysicalPosition::new(580.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Released,
            Some(PhysicalPosition::new(580.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 4);
        assert_quick_craft_click(
            &mut rx,
            -999,
            4,
            [].into(),
            HashedStack::Item(hashed_item(42, 8)),
        );
        assert_quick_craft_click(
            &mut rx,
            36,
            5,
            [].into(),
            HashedStack::Item(hashed_item(42, 8)),
        );
        assert_quick_craft_click(
            &mut rx,
            37,
            5,
            [].into(),
            HashedStack::Item(hashed_item(42, 8)),
        );
        assert_quick_craft_click(
            &mut rx,
            -999,
            6,
            [
                (36, HashedStack::Item(hashed_item(42, 1))),
                (37, HashedStack::Item(hashed_item(42, 1))),
            ]
            .into(),
            HashedStack::Item(hashed_item(42, 6)),
        );
        assert!(rx.try_recv().is_err());
        assert_eq!(world.inventory().cursor_item, item_stack(42, 6));
        assert_eq!(player_slot_item(&world, 0), item_stack(42, 1));
        assert_eq!(player_slot_item(&world, 1), item_stack(42, 1));
    }

    #[test]
    fn inventory_quick_craft_without_slots_falls_back_to_pickup_on_release() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_set_cursor_item(SetCursorItem {
            item: item_stack(42, 3),
        });
        assert!(world.open_local_inventory());

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(560.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert!(rx.try_recv().is_err());

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Released,
            Some(PhysicalPosition::new(560.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        match rx.try_recv().unwrap() {
            NetCommand::ContainerClick(click) => {
                assert_eq!(click.slot_num, 36);
                assert_eq!(click.button_num, 0);
                assert_eq!(click.input, ContainerInput::Pickup);
            }
            command => panic!("expected container click command, got {command:?}"),
        }
        assert_eq!(world.inventory().cursor_item, ItemStackSummary::empty());
        assert_eq!(player_slot_item(&world, 0), item_stack(42, 3));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn inventory_quick_craft_mismatched_release_button_cancels_drag() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_set_cursor_item(SetCursorItem {
            item: item_stack(42, 3),
        });
        assert!(world.open_local_inventory());

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(560.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));
        assert!(handle_inventory_cursor_moved(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            Some(PhysicalPosition::new(560.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Released,
            Some(PhysicalPosition::new(560.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 0);
        assert_eq!(world.inventory().cursor_item, item_stack(42, 3));
        assert_eq!(player_slot_item(&world, 0), ItemStackSummary::empty());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn shift_inventory_slot_click_queues_container_zero_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: item_stack(42, 3),
        });
        assert!(world.open_local_inventory());

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(560.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        match rx.try_recv().unwrap() {
            NetCommand::ContainerClick(click) => {
                assert_eq!(click.container_id, 0);
                assert_eq!(click.state_id, 0);
                assert_eq!(click.slot_num, 36);
                assert_eq!(click.button_num, 0);
                assert_eq!(click.input, ContainerInput::QuickMove);
            }
            command => panic!("expected container click command, got {command:?}"),
        }
    }

    #[test]
    fn shift_server_opened_bundle_slot_click_clears_selection_before_quick_move() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: GENERIC_CONTAINER_FIRST_MENU_TYPE_ID,
            title: "Chest".to_string(),
        });
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bundle_stack(42, 1, 3)],
            carried_item: ItemStackSummary::empty(),
        });
        assert!(world.apply_local_select_bundle_item(0, 1));

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(568.0, 320.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.select_bundle_item_commands_queued, 1);
        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectBundleItem(SelectBundleItem {
                slot_id: 0,
                selected_item_index: -1,
            })
        );
        match rx.try_recv().unwrap() {
            NetCommand::ContainerClick(click) => {
                assert_eq!(click.container_id, 7);
                assert_eq!(click.state_id, 12);
                assert_eq!(click.slot_num, 0);
                assert_eq!(click.button_num, 0);
                assert_eq!(click.input, ContainerInput::QuickMove);
                assert_eq!(click.changed_slots, [].into());
                assert_eq!(click.carried_item, HashedStack::Empty);
            }
            command => panic!("expected container click command, got {command:?}"),
        }
        assert_eq!(open_container_slot_bundle_selection(&world, 0), Some(-1));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn shift_inventory_outside_click_queues_pickup_not_quick_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.shift_left_down = true;
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_set_cursor_item(SetCursorItem {
            item: item_stack(42, 3),
        });
        assert!(world.open_local_inventory());

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(551.0, 277.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 0,
                state_id: 0,
                slot_num: -999,
                button_num: 0,
                input: ContainerInput::Pickup,
                changed_slots: [].into(),
                carried_item: HashedStack::Empty,
            })
        );
    }

    #[test]
    fn inventory_outside_click_with_empty_cursor_is_consumed_without_packet() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        assert!(world.open_local_inventory());

        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(551.0, 277.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.container_click_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn inventory_mouse_wheel_routes_bundle_selection_for_hovered_container_zero_slot() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: bundle_stack(42, 1, 3),
        });
        assert!(world.open_local_inventory());

        assert!(handle_inventory_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, 1.0),
            Some(PhysicalPosition::new(560.0, 419.0)),
            PhysicalSize::new(1280, 720),
        ));

        assert_eq!(counters.select_bundle_item_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectBundleItem(SelectBundleItem {
                slot_id: 36,
                selected_item_index: 2,
            })
        );
    }

    fn stonecutter_recipe(item_ids: Vec<i32>) -> StonecutterSelectableRecipeSummary {
        StonecutterSelectableRecipeSummary {
            input: IngredientSummary {
                tag: None,
                item_ids,
            },
            option_display: SlotDisplaySummary {
                display_type_id: 0,
                raw_payload: Vec::new(),
            },
        }
    }

    fn apply_item_tags(world: &mut WorldStore, tags: Vec<(&str, Vec<i32>)>) {
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: tags
                    .into_iter()
                    .map(|(tag, entries)| TagNetworkPayload {
                        tag: tag.to_string(),
                        entries,
                    })
                    .collect(),
            }],
        });
    }

    fn loom_world_with_banner_and_dye() -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: LOOM_MENU_TYPE_ID,
            title: "Loom".to_string(),
        });
        let mut items = vec![ItemStackSummary::empty(); 40];
        items[0] = item_stack(42, 1);
        items[1] = item_stack(43, 1);
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });
        world
    }

    fn add_entity_with_type(id: i32, entity_type_id: i32) -> AddEntity {
        AddEntity {
            id,
            uuid: Uuid::from_u128(id as u128),
            entity_type_id,
            position: Vec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        }
    }

    fn byte_entity_data(data_id: u8, value: i8) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(value),
        }
    }

    fn bool_entity_data(data_id: u8, value: bool) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: 8,
            value: EntityDataValueKind::Boolean(value),
        }
    }

    fn item_stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }

    fn apply_instabuild_abilities(world: &mut WorldStore) {
        world.apply_player_abilities(PlayerAbilities {
            invulnerable: false,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
    }

    fn merchant_offers(container_id: i32, offer_count: usize) -> MerchantOffers {
        MerchantOffers {
            container_id,
            offers: (0..offer_count)
                .map(|index| MerchantOffer {
                    buy_a: item_cost(42 + index as i32, 3),
                    sell: item_stack(99 + index as i32, 1),
                    buy_b: None,
                    is_out_of_stock: false,
                    uses: 1,
                    max_uses: 12,
                    xp: 8,
                    special_price_diff: -2,
                    price_multiplier: 0.05,
                    demand: 6,
                })
                .collect(),
            villager_level: 3,
            villager_xp: 120,
            show_progress: true,
            can_restock: false,
        }
    }

    fn item_cost(item_id: i32, count: i32) -> ItemCostSummary {
        ItemCostSummary {
            item_id,
            count,
            component_predicate: Default::default(),
        }
    }

    fn bundle_stack(item_id: i32, count: i32, item_count: usize) -> ItemStackSummary {
        let mut stack = item_stack(item_id, count);
        stack.component_patch.bundle_contents_item_count = Some(item_count);
        stack
    }

    fn hashed_item(item_id: i32, count: i32) -> HashedItemStack {
        HashedItemStack {
            item_id,
            count,
            components: HashedComponentPatch::default(),
        }
    }

    fn assert_quick_craft_click(
        rx: &mut mpsc::Receiver<NetCommand>,
        slot_num: i16,
        button_num: i8,
        changed_slots: BTreeMap<i16, HashedStack>,
        carried_item: HashedStack,
    ) {
        match rx.try_recv().unwrap() {
            NetCommand::ContainerClick(click) => {
                assert_eq!(click.container_id, 0);
                assert_eq!(click.state_id, 0);
                assert_eq!(click.slot_num, slot_num);
                assert_eq!(click.button_num, button_num);
                assert_eq!(click.input, ContainerInput::QuickCraft);
                assert_eq!(click.changed_slots, changed_slots);
                assert_eq!(click.carried_item, carried_item);
            }
            command => panic!("expected quick craft container click command, got {command:?}"),
        }
    }

    fn player_slot_item(world: &WorldStore, slot: i32) -> ItemStackSummary {
        world
            .inventory()
            .player_slots
            .iter()
            .find(|state| state.slot == slot)
            .map(|state| state.item.clone())
            .unwrap_or_else(ItemStackSummary::empty)
    }

    fn open_container_slot_bundle_selection(world: &WorldStore, slot: i16) -> Option<i32> {
        world
            .inventory()
            .open_container
            .as_ref()?
            .slots
            .iter()
            .find(|state| state.slot == slot)
            .map(|state| state.local_selected_bundle_item_index)
    }
}
