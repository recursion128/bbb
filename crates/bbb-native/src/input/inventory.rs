use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::packets::ContainerInput;
use bbb_world::{ContainerClickSlotRequest, WorldStore};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta},
};

use super::{
    bundle::{handle_bundle_slot_hover_end, handle_bundle_slot_mouse_scroll},
    commands::queue_container_click_command,
    ClientInputState,
};

const INVENTORY_SCREEN_WIDTH: f64 = 176.0;
const INVENTORY_SCREEN_HEIGHT: f64 = 166.0;
const SLOT_SIZE: f64 = 16.0;
const SLOT_HOVER_MARGIN: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LocalInventorySlotLayout {
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

pub(crate) fn local_inventory_slot_layouts() -> Vec<LocalInventorySlotLayout> {
    let mut slots = Vec::with_capacity(46);
    slots.push(LocalInventorySlotLayout {
        slot_id: 0,
        x: 154,
        y: 28,
    });
    for y in 0..2 {
        for x in 0..2 {
            slots.push(LocalInventorySlotLayout {
                slot_id: (1 + x + y * 2) as i16,
                x: 98 + x * 18,
                y: 18 + y * 18,
            });
        }
    }
    for index in 0..4 {
        slots.push(LocalInventorySlotLayout {
            slot_id: (5 + index) as i16,
            x: 8,
            y: 8 + index * 18,
        });
    }
    for y in 0..3 {
        for x in 0..9 {
            slots.push(LocalInventorySlotLayout {
                slot_id: (9 + x + y * 9) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..9 {
        slots.push(LocalInventorySlotLayout {
            slot_id: (36 + x) as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }
    slots.push(LocalInventorySlotLayout {
        slot_id: 45,
        x: 77,
        y: 62,
    });
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
    if !input.focused || !world.local_inventory_is_open() {
        return false;
    }

    let hovered = local_inventory_hovered_slot(cursor_position, surface_size);
    if input.inventory_hovered_slot != hovered {
        if let Some(previous) = input.inventory_hovered_slot {
            handle_bundle_slot_hover_end(world, counters, net_commands, i32::from(previous));
        }
        input.inventory_hovered_slot = hovered;
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
    if !input.focused || !world.local_inventory_is_open() {
        return false;
    }
    let pressed = matches!(state, ElementState::Pressed);
    let button_num = match button {
        MouseButton::Left if pressed => 0,
        MouseButton::Right if pressed => 1,
        _ => return true,
    };

    let slot_num = match local_inventory_click_target(cursor_position, surface_size) {
        Some(InventoryClickTarget::Slot(slot)) => slot,
        Some(InventoryClickTarget::Outside) => {
            if inventory_cursor_is_empty(world) {
                return true;
            }
            -999
        }
        Some(InventoryClickTarget::EmptyPanel) | None => return true,
    };
    let Ok(click) = world.apply_local_container_click_slot(ContainerClickSlotRequest {
        slot_num,
        button_num,
        input: ContainerInput::Pickup,
    }) else {
        return true;
    };
    queue_container_click_command(counters, net_commands, click);
    true
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
    if !input.focused || !world.local_inventory_is_open() {
        return false;
    }
    if let Some(slot) = local_inventory_hovered_slot(cursor_position, surface_size) {
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

fn local_inventory_hovered_slot(
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<i16> {
    match local_inventory_click_target(cursor_position, surface_size) {
        Some(InventoryClickTarget::Slot(slot)) => Some(slot),
        _ => None,
    }
}

fn local_inventory_click_target(
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<InventoryClickTarget> {
    let cursor = cursor_position?;
    let (origin_x, origin_y) = local_inventory_screen_origin(surface_size);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    if x < 0.0 || y < 0.0 || x >= INVENTORY_SCREEN_WIDTH || y >= INVENTORY_SCREEN_HEIGHT {
        return Some(InventoryClickTarget::Outside);
    }
    for slot in local_inventory_slot_layouts() {
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

fn local_inventory_screen_origin(surface_size: PhysicalSize<u32>) -> (f64, f64) {
    (
        (f64::from(surface_size.width.max(1)) - INVENTORY_SCREEN_WIDTH) * 0.5,
        (f64::from(surface_size.height.max(1)) - INVENTORY_SCREEN_HEIGHT) * 0.5,
    )
}

fn inventory_cursor_is_empty(world: &WorldStore) -> bool {
    let cursor = &world.inventory().cursor_item;
    cursor.item_id.is_none() || cursor.count <= 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        ContainerClick, HashedComponentPatch, HashedItemStack, HashedStack, ItemStackSummary,
        SelectBundleItem, SetPlayerInventory,
    };

    #[test]
    fn local_inventory_slot_layouts_match_vanilla_inventory_menu() {
        let slots = local_inventory_slot_layouts();
        assert_eq!(slots.len(), 46);
        assert_eq!(
            slots[0],
            LocalInventorySlotLayout {
                slot_id: 0,
                x: 154,
                y: 28,
            }
        );
        assert_eq!(
            slots[1],
            LocalInventorySlotLayout {
                slot_id: 1,
                x: 98,
                y: 18,
            }
        );
        assert_eq!(
            slots[5],
            LocalInventorySlotLayout {
                slot_id: 5,
                x: 8,
                y: 8,
            }
        );
        assert_eq!(
            slots[9],
            LocalInventorySlotLayout {
                slot_id: 9,
                x: 8,
                y: 84,
            }
        );
        assert_eq!(
            slots[36],
            LocalInventorySlotLayout {
                slot_id: 36,
                x: 8,
                y: 142,
            }
        );
        assert_eq!(
            slots[45],
            LocalInventorySlotLayout {
                slot_id: 45,
                x: 77,
                y: 62,
            }
        );
    }

    #[test]
    fn local_inventory_hit_test_uses_centered_vanilla_screen_and_hover_margin() {
        let size = PhysicalSize::new(1280, 720);
        assert_eq!(
            local_inventory_click_target(Some(PhysicalPosition::new(560.0, 419.0)), size),
            Some(InventoryClickTarget::Slot(36))
        );
        assert_eq!(
            local_inventory_click_target(Some(PhysicalPosition::new(559.0, 418.0)), size),
            Some(InventoryClickTarget::Slot(36))
        );
        assert_eq!(
            local_inventory_click_target(Some(PhysicalPosition::new(600.0, 300.0)), size),
            Some(InventoryClickTarget::EmptyPanel)
        );
        assert_eq!(
            local_inventory_click_target(Some(PhysicalPosition::new(551.0, 277.0)), size),
            Some(InventoryClickTarget::Outside)
        );
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
}
