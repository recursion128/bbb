use std::time::{Duration, Instant};

use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::packets::{ContainerInput, ItemStackSummary};
use bbb_world::{ContainerClickBuildError, ContainerClickSlotRequest, WorldStore};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::KeyCode,
};

use super::{
    bundle::{handle_bundle_slot_hover_end, handle_bundle_slot_mouse_scroll},
    commands::{hotbar_slot_for_key, queue_container_click_command},
    ClientInputState,
};

const INVENTORY_SCREEN_WIDTH: i32 = 176;
const INVENTORY_SCREEN_HEIGHT: i32 = 166;
const GENERIC_CONTAINER_WIDTH: i32 = 176;
const GENERIC_CONTAINER_BASE_HEIGHT: i32 = 114;
const GENERIC_CONTAINER_ROW_HEIGHT: i32 = 18;
const GENERIC_CONTAINER_FIRST_MENU_TYPE_ID: i32 = 0;
const GENERIC_CONTAINER_LAST_MENU_TYPE_ID: i32 = 5;
const GENERIC_3X3_MENU_TYPE_ID: i32 = 6;
const BLAST_FURNACE_MENU_TYPE_ID: i32 = 10;
const CRAFTING_MENU_TYPE_ID: i32 = 12;
const FURNACE_MENU_TYPE_ID: i32 = 14;
const HOPPER_MENU_TYPE_ID: i32 = 16;
const SHULKER_BOX_MENU_TYPE_ID: i32 = 20;
const SMOKER_MENU_TYPE_ID: i32 = 22;
const GENERIC_CONTAINER_SLOT_COLUMNS: i32 = 9;
const GENERIC_CONTAINER_SLOT_COUNT_PER_ROW: i16 = 9;
const GENERIC_3X3_SCREEN_WIDTH: i32 = 176;
const GENERIC_3X3_SCREEN_HEIGHT: i32 = 166;
const GENERIC_3X3_SLOT_COLUMNS: i32 = 3;
const GENERIC_3X3_SLOT_COUNT: i16 = 9;
const CRAFTING_SCREEN_WIDTH: i32 = 176;
const CRAFTING_SCREEN_HEIGHT: i32 = 166;
const CRAFTING_GRID_SLOT_COLUMNS: i32 = 3;
const CRAFTING_SLOT_COUNT: i16 = 10;
const FURNACE_SCREEN_WIDTH: i32 = 176;
const FURNACE_SCREEN_HEIGHT: i32 = 166;
const FURNACE_SLOT_COUNT: i16 = 3;
const HOPPER_SCREEN_WIDTH: i32 = 176;
const HOPPER_SCREEN_HEIGHT: i32 = 133;
const HOPPER_SLOT_COUNT: i16 = 5;
const SHULKER_BOX_SCREEN_WIDTH: i32 = 176;
const SHULKER_BOX_SCREEN_HEIGHT: i32 = 167;
const SHULKER_BOX_SLOT_COUNT: i16 = 27;
const SLOT_SIZE: f64 = 16.0;
const SLOT_HOVER_MARGIN: f64 = 1.0;
const VANILLA_DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InventoryScreenBackground {
    LocalInventory,
    Generic9xRows { rows: u8 },
    Generic3x3,
    BlastFurnace,
    CraftingTable,
    Furnace,
    Hopper,
    ShulkerBox,
    Smoker,
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
    if menu_type_id == CRAFTING_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: CRAFTING_SCREEN_WIDTH,
            height: CRAFTING_SCREEN_HEIGHT,
            background: InventoryScreenBackground::CraftingTable,
            slots: crafting_table_slot_layouts(),
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
    if menu_type_id == HOPPER_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: HOPPER_SCREEN_WIDTH,
            height: HOPPER_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Hopper,
            slots: hopper_slot_layouts(),
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
    local_inventory_apply_and_queue_click(world, counters, net_commands, request);
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

fn local_inventory_apply_and_queue_click(
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    request: ContainerClickSlotRequest,
) -> bool {
    let click = if world.local_inventory_is_open()
        || matches!(
            request.input,
            ContainerInput::Pickup | ContainerInput::QuickMove
        ) {
        match world.apply_local_container_click_slot(request) {
            Ok(click) => click,
            Err(ContainerClickBuildError::UnsupportedLocalClickInput(_))
                if !world.local_inventory_is_open() =>
            {
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

pub(crate) fn handle_inventory_key_input(
    input: &ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    code: KeyCode,
) -> bool {
    if !input.focused || inventory_screen_layout(world).is_none() {
        return false;
    }

    if !world.local_inventory_is_open() {
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
    if !local_inventory_slot_has_item(world, slot_num) {
        return true;
    }
    let request = ContainerClickSlotRequest {
        slot_num,
        button_num: if input.control_down() { 1 } else { 0 },
        input: ContainerInput::Throw,
    };
    let Ok(click) = world.apply_local_container_click_slot(request) else {
        return true;
    };
    if click.changed_slots.is_empty() {
        return true;
    }
    queue_container_click_command(counters, net_commands, click);
    true
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
    let Ok(click) = world.apply_local_container_click_slot(request) else {
        return;
    };
    if click.changed_slots.is_empty() {
        return;
    }
    queue_container_click_command(counters, net_commands, click);
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
    use std::collections::BTreeMap;

    use bbb_protocol::packets::{
        ContainerClick, ContainerSetContent, HashedComponentPatch, HashedItemStack, HashedStack,
        ItemStackSummary, OpenScreen, RecipePropertySetSummary, SelectBundleItem, SetCursorItem,
        SetPlayerInventory, UpdateRecipes,
    };

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
    fn hopper_shift_click_queues_quick_move() {
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
    }

    #[test]
    fn shulker_box_shift_click_queues_quick_move() {
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

    fn item_stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
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
}
