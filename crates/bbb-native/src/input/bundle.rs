use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::packets::ContainerInput;
#[cfg(test)]
use bbb_protocol::packets::SelectBundleItem;
use bbb_world::WorldStore;
use tokio::sync::mpsc;
use winit::event::MouseScrollDelta;

use super::{commands::queue_select_bundle_item_command, ClientInputState};

pub(crate) fn select_bundle_item(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    slot_id: i32,
    selected_item_index: i32,
) -> bool {
    if !world.apply_local_select_bundle_item(slot_id, selected_item_index) {
        return false;
    }
    queue_select_bundle_item_command(counters, net_commands, slot_id, selected_item_index);
    true
}

pub(crate) fn handle_bundle_slot_mouse_scroll(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    slot_id: i32,
    delta: MouseScrollDelta,
) -> bool {
    if !input.focused {
        return false;
    }
    if bundle_slot_shown_item_count(world, slot_id).is_none_or(|count| count == 0) {
        return false;
    }
    if let Some(wheel) = bundle_wheel_steps_from_scroll(input, delta) {
        select_bundle_item_from_scroll(counters, world, net_commands, slot_id, wheel);
    }
    true
}

pub(crate) fn handle_bundle_slot_hover_end(
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    slot_id: i32,
) -> bool {
    select_bundle_item(counters, world, net_commands, slot_id, -1)
}

pub(crate) fn handle_bundle_slot_click(
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    slot_id: i32,
    input: ContainerInput,
) -> bool {
    if !matches!(input, ContainerInput::QuickMove | ContainerInput::Swap) {
        return false;
    }
    select_bundle_item(counters, world, net_commands, slot_id, -1)
}

fn select_bundle_item_from_scroll(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    slot_id: i32,
    wheel: i32,
) -> bool {
    let Some((selected_item_index, shown_item_count)) =
        bundle_slot_selection(world, slot_id).filter(|(_, count)| *count > 0)
    else {
        return false;
    };
    let Some(updated_selected_item_index) =
        bundle_item_index_for_scroll(wheel, selected_item_index, shown_item_count)
    else {
        return false;
    };
    if updated_selected_item_index == selected_item_index {
        return false;
    }
    select_bundle_item(
        counters,
        world,
        net_commands,
        slot_id,
        updated_selected_item_index,
    )
}

fn bundle_slot_shown_item_count(world: &WorldStore, slot_id: i32) -> Option<usize> {
    bundle_slot_selection(world, slot_id).map(|(_, shown_item_count)| shown_item_count)
}

fn bundle_slot_selection(world: &WorldStore, slot_id: i32) -> Option<(i32, usize)> {
    if let Some(container) = world.inventory().open_container.as_ref().or_else(|| {
        world
            .local_inventory_is_open()
            .then_some(&world.inventory().inventory_menu)
    }) {
        let slot_id = i16::try_from(slot_id).ok()?;
        let slot = container.slots.iter().find(|slot| slot.slot == slot_id)?;
        return bundle_item_selection(
            slot.item.component_patch.bundle_contents_item_count,
            slot.local_selected_bundle_item_index,
        );
    }

    let slot = world
        .inventory()
        .player_slots
        .iter()
        .find(|slot| slot.slot == slot_id)?;
    bundle_item_selection(
        slot.item.component_patch.bundle_contents_item_count,
        slot.local_selected_bundle_item_index,
    )
}

fn bundle_item_selection(
    bundle_contents_item_count: Option<usize>,
    selected_item_index: i32,
) -> Option<(i32, usize)> {
    let item_count = bundle_contents_item_count?;
    Some((
        selected_item_index,
        bundle_number_of_items_to_show(item_count),
    ))
}

fn bundle_number_of_items_to_show(item_count: usize) -> usize {
    let available_items_to_show = if item_count > 12 { 11 } else { 12 };
    let items_on_non_full_row = item_count % 4;
    let empty_space_on_non_full_row = if items_on_non_full_row == 0 {
        0
    } else {
        4 - items_on_non_full_row
    };
    item_count.min(available_items_to_show - empty_space_on_non_full_row)
}

fn bundle_item_index_for_scroll(
    wheel: i32,
    current_selected: i32,
    item_count: usize,
) -> Option<i32> {
    if item_count == 0 {
        return None;
    }
    let step = wheel.signum();
    if step == 0 {
        return None;
    }

    let limit = i32::try_from(item_count).ok()?;
    let mut selected = (current_selected - step).max(-1);
    while selected < 0 {
        selected += limit;
    }
    while selected >= limit {
        selected -= limit;
    }
    Some(selected)
}

fn bundle_wheel_steps_from_scroll(
    input: &mut ClientInputState,
    delta: MouseScrollDelta,
) -> Option<i32> {
    let (x, y) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (f64::from(x), f64::from(y)),
        MouseScrollDelta::PixelDelta(pos) => (pos.x, pos.y),
    };

    if input.bundle_scroll_accumulated_x != 0.0
        && scroll_signum(x) != scroll_signum(input.bundle_scroll_accumulated_x)
    {
        input.bundle_scroll_accumulated_x = 0.0;
    }
    if input.bundle_scroll_accumulated_y != 0.0
        && scroll_signum(y) != scroll_signum(input.bundle_scroll_accumulated_y)
    {
        input.bundle_scroll_accumulated_y = 0.0;
    }

    input.bundle_scroll_accumulated_x += x;
    input.bundle_scroll_accumulated_y += y;
    let wheel_x = input.bundle_scroll_accumulated_x as i32;
    let wheel_y = input.bundle_scroll_accumulated_y as i32;
    if wheel_x == 0 && wheel_y == 0 {
        return None;
    }

    input.bundle_scroll_accumulated_x -= f64::from(wheel_x);
    input.bundle_scroll_accumulated_y -= f64::from(wheel_y);
    let wheel = if wheel_y == 0 { -wheel_x } else { wheel_y };
    (wheel != 0).then_some(wheel)
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

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        ContainerSetContent, DataComponentPatchSummary, ItemStackSummary, SetPlayerInventory,
    };

    #[test]
    fn bundle_number_of_items_to_show_matches_vanilla_grid() {
        assert_eq!(bundle_number_of_items_to_show(0), 0);
        assert_eq!(bundle_number_of_items_to_show(1), 1);
        assert_eq!(bundle_number_of_items_to_show(4), 4);
        assert_eq!(bundle_number_of_items_to_show(10), 10);
        assert_eq!(bundle_number_of_items_to_show(11), 11);
        assert_eq!(bundle_number_of_items_to_show(12), 12);
        assert_eq!(bundle_number_of_items_to_show(13), 8);
        assert_eq!(bundle_number_of_items_to_show(14), 9);
        assert_eq!(bundle_number_of_items_to_show(15), 10);
        assert_eq!(bundle_number_of_items_to_show(16), 11);
        assert_eq!(bundle_number_of_items_to_show(17), 8);
    }

    #[test]
    fn bundle_item_index_for_scroll_matches_vanilla_selection_wrap() {
        assert_eq!(bundle_item_index_for_scroll(-1, -1, 3), Some(0));
        assert_eq!(bundle_item_index_for_scroll(1, -1, 3), Some(2));
        assert_eq!(bundle_item_index_for_scroll(1, 0, 3), Some(2));
        assert_eq!(bundle_item_index_for_scroll(-1, 2, 3), Some(0));
        assert_eq!(bundle_item_index_for_scroll(2, 1, 3), Some(0));
        assert_eq!(bundle_item_index_for_scroll(-3, 1, 10), Some(2));
        assert_eq!(bundle_item_index_for_scroll(3, 1, 10), Some(0));
        assert_eq!(bundle_item_index_for_scroll(0, 1, 3), None);
        assert_eq!(bundle_item_index_for_scroll(1, 0, 0), None);
    }

    #[test]
    fn select_bundle_item_from_scroll_updates_world_and_queues_command() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 4,
            item: bundle_stack(42, 1, 3),
        });

        assert!(select_bundle_item_from_scroll(
            &mut counters,
            &mut world,
            &commands,
            4,
            -1,
        ));
        assert_eq!(player_slot_selection(&world, 4), Some(0));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectBundleItem(SelectBundleItem {
                slot_id: 4,
                selected_item_index: 0,
            })
        );

        assert!(select_bundle_item_from_scroll(
            &mut counters,
            &mut world,
            &commands,
            4,
            1,
        ));
        assert_eq!(player_slot_selection(&world, 4), Some(2));
        assert_eq!(counters.select_bundle_item_commands_queued, 2);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectBundleItem(SelectBundleItem {
                slot_id: 4,
                selected_item_index: 2,
            })
        );
    }

    #[test]
    fn select_bundle_item_from_scroll_uses_shown_item_count_not_total_contents() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 4,
            item: bundle_stack(42, 1, 13),
        });

        assert!(select_bundle_item_from_scroll(
            &mut counters,
            &mut world,
            &commands,
            4,
            1,
        ));

        assert_eq!(player_slot_selection(&world, 4), Some(7));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectBundleItem(SelectBundleItem {
                slot_id: 4,
                selected_item_index: 7,
            })
        );
    }

    #[test]
    fn select_bundle_item_from_scroll_uses_open_container_slot_when_present() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 1,
            item: bundle_stack(42, 1, 3),
        });
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 1,
            items: vec![item_stack(1, 1), bundle_stack(43, 1, 2)],
            carried_item: ItemStackSummary::empty(),
        });

        assert!(select_bundle_item_from_scroll(
            &mut counters,
            &mut world,
            &commands,
            1,
            1,
        ));

        assert_eq!(player_slot_selection(&world, 1), Some(-1));
        assert_eq!(container_slot_selection(&world, 1), Some(1));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectBundleItem(SelectBundleItem {
                slot_id: 1,
                selected_item_index: 1,
            })
        );
    }

    #[test]
    fn select_bundle_item_from_scroll_ignores_empty_non_bundle_and_single_noop() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 4,
            item: item_stack(42, 1),
        });
        assert!(!select_bundle_item_from_scroll(
            &mut counters,
            &mut world,
            &commands,
            4,
            1,
        ));

        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 4,
            item: bundle_stack(43, 1, 1),
        });
        assert!(select_bundle_item(
            &mut counters,
            &mut world,
            &commands,
            4,
            0,
        ));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectBundleItem(SelectBundleItem {
                slot_id: 4,
                selected_item_index: 0,
            })
        );
        assert!(!select_bundle_item_from_scroll(
            &mut counters,
            &mut world,
            &commands,
            4,
            1,
        ));

        assert_eq!(counters.select_bundle_item_commands_queued, 1);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn bundle_slot_mouse_scroll_consumes_fractional_scroll_before_selecting() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 4,
            item: bundle_stack(42, 1, 3),
        });
        let mut counters = NetCounters::default();

        assert!(handle_bundle_slot_mouse_scroll(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            4,
            MouseScrollDelta::LineDelta(0.0, 0.5),
        ));
        assert_eq!(player_slot_selection(&world, 4), Some(-1));
        assert!(rx.try_recv().is_err());

        assert!(handle_bundle_slot_mouse_scroll(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            4,
            MouseScrollDelta::LineDelta(0.0, 0.5),
        ));
        assert_eq!(player_slot_selection(&world, 4), Some(2));
        assert_eq!(counters.select_bundle_item_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectBundleItem(SelectBundleItem {
                slot_id: 4,
                selected_item_index: 2,
            })
        );
    }

    #[test]
    fn bundle_slot_mouse_scroll_uses_vertical_before_horizontal() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 4,
            item: bundle_stack(42, 1, 3),
        });
        let mut counters = NetCounters::default();

        assert!(handle_bundle_slot_mouse_scroll(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            4,
            MouseScrollDelta::LineDelta(-1.0, -1.0),
        ));

        assert_eq!(player_slot_selection(&world, 4), Some(0));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectBundleItem(SelectBundleItem {
                slot_id: 4,
                selected_item_index: 0,
            })
        );
    }

    #[test]
    fn bundle_slot_mouse_scroll_negates_horizontal_wheel() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 4,
            item: bundle_stack(42, 1, 3),
        });
        let mut counters = NetCounters::default();

        assert!(handle_bundle_slot_mouse_scroll(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            4,
            MouseScrollDelta::LineDelta(1.0, 0.0),
        ));

        assert_eq!(player_slot_selection(&world, 4), Some(0));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectBundleItem(SelectBundleItem {
                slot_id: 4,
                selected_item_index: 0,
            })
        );
    }

    #[test]
    fn bundle_slot_hover_end_and_quick_move_click_unselect() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 4,
            item: bundle_stack(42, 1, 3),
        });
        let mut counters = NetCounters::default();

        assert!(select_bundle_item(
            &mut counters,
            &mut world,
            &commands,
            4,
            1,
        ));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectBundleItem(SelectBundleItem {
                slot_id: 4,
                selected_item_index: 1,
            })
        );

        assert!(handle_bundle_slot_hover_end(
            &mut world,
            &mut counters,
            &commands,
            4,
        ));
        assert_eq!(player_slot_selection(&world, 4), Some(-1));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectBundleItem(SelectBundleItem {
                slot_id: 4,
                selected_item_index: -1,
            })
        );

        assert!(select_bundle_item(
            &mut counters,
            &mut world,
            &commands,
            4,
            2,
        ));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectBundleItem(SelectBundleItem {
                slot_id: 4,
                selected_item_index: 2,
            })
        );
        assert!(!handle_bundle_slot_click(
            &mut world,
            &mut counters,
            &commands,
            4,
            ContainerInput::Pickup,
        ));
        assert!(handle_bundle_slot_click(
            &mut world,
            &mut counters,
            &commands,
            4,
            ContainerInput::QuickMove,
        ));
        assert_eq!(player_slot_selection(&world, 4), Some(-1));
        assert_eq!(counters.select_bundle_item_commands_queued, 4);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectBundleItem(SelectBundleItem {
                slot_id: 4,
                selected_item_index: -1,
            })
        );
    }

    #[test]
    fn bundle_slot_mouse_scroll_ignores_empty_or_unfocused_slots() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 4,
            item: item_stack(42, 1),
        });
        let mut counters = NetCounters::default();

        assert!(!handle_bundle_slot_mouse_scroll(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            4,
            MouseScrollDelta::LineDelta(0.0, 1.0),
        ));

        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 4,
            item: bundle_stack(43, 1, 0),
        });
        assert!(!handle_bundle_slot_mouse_scroll(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            4,
            MouseScrollDelta::LineDelta(0.0, 1.0),
        ));

        input.focused = false;
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 4,
            item: bundle_stack(44, 1, 3),
        });
        assert!(!handle_bundle_slot_mouse_scroll(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            4,
            MouseScrollDelta::LineDelta(0.0, 1.0),
        ));

        assert_eq!(counters.select_bundle_item_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    fn item_stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: DataComponentPatchSummary::default(),
        }
    }

    fn bundle_stack(
        item_id: i32,
        count: i32,
        bundle_contents_item_count: usize,
    ) -> ItemStackSummary {
        let mut stack = item_stack(item_id, count);
        stack.component_patch.bundle_contents_item_count = Some(bundle_contents_item_count);
        stack
    }

    fn player_slot_selection(world: &WorldStore, slot_id: i32) -> Option<i32> {
        world
            .inventory()
            .player_slots
            .iter()
            .find(|slot| slot.slot == slot_id)
            .map(|slot| slot.local_selected_bundle_item_index)
    }

    fn container_slot_selection(world: &WorldStore, slot_id: i16) -> Option<i32> {
        world
            .inventory()
            .open_container
            .as_ref()?
            .slots
            .iter()
            .find(|slot| slot.slot == slot_id)
            .map(|slot| slot.local_selected_bundle_item_index)
    }
}
