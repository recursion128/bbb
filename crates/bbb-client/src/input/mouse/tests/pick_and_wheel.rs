use super::*;

#[test]
fn control_middle_mouse_press_on_block_queues_pick_block_with_data() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.control_left_down = true;
    let mut world = world_with_crosshair_block();
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Middle,
        ElementState::Pressed,
    );

    assert_eq!(counters.pick_item_from_block_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PickItemFromBlock(PickItemFromBlock {
            pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
            include_data: true,
        })
    );
}

#[test]
fn control_middle_mouse_press_on_entity_queues_pick_entity_with_data() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.control_left_down = true;
    let mut world = world_with_crosshair_entity(123);
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Middle,
        ElementState::Pressed,
    );

    assert_eq!(counters.pick_item_from_entity_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PickItemFromEntity(PickItemFromEntity {
            entity_id: 123,
            include_data: true,
        })
    );
}

#[test]
fn sprint_middle_mouse_press_does_not_include_pick_data() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    input.forward = true;
    assert!(world.reconcile_local_player_sprinting(input.local_player_input(&world), true));
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Middle,
        ElementState::Pressed,
    );

    assert_eq!(counters.pick_item_from_block_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PickItemFromBlock(PickItemFromBlock {
            pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
            include_data: false,
        })
    );
}

#[test]
fn spectator_middle_mouse_opens_menu_instead_of_picking_target() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_entity(123);
    set_local_spectator(&mut world);
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Middle,
        ElementState::Pressed,
    );

    assert!(input.spectator_menu_is_active());
    assert_eq!(counters.pick_item_from_entity_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn spectator_middle_mouse_does_nothing_while_using_item() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_entity(123);
    set_local_spectator(&mut world);
    world.set_local_using_item(true);
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Middle,
        ElementState::Pressed,
    );

    assert!(!input.spectator_menu_is_active());
    assert_eq!(counters.pick_item_from_entity_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn mouse_wheel_selects_hotbar_slot_updates_world_and_queues_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    assert!(world.set_local_selected_hotbar_slot(4));
    let mut counters = NetCounters::default();

    handle_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, 1.0),
    );

    assert_eq!(world.local_player().selected_hotbar_slot, 3);
    assert_eq!(world.counters().held_slot_packets, 0);
    assert_eq!(counters.held_slot_commands_queued, 1);
    assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(3));
}

#[test]
fn spectator_mouse_wheel_does_not_select_hotbar_or_queue_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    assert!(world.set_local_selected_hotbar_slot(4));
    set_local_spectator(&mut world);
    let mut counters = NetCounters::default();

    handle_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, 1.0),
    );

    assert_eq!(world.local_player().selected_hotbar_slot, 4);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn spectator_mouse_wheel_adjusts_local_flying_speed_without_hotbar_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    assert!(world.set_local_selected_hotbar_slot(4));
    set_local_spectator(&mut world);
    set_flying_abilities(&mut world, 0.05);
    let mut counters = NetCounters::default();

    handle_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, 1.0),
    );

    assert_eq!(world.local_player().selected_hotbar_slot, 4);
    assert_eq!(world.local_player().abilities.unwrap().flying_speed, 0.055);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn spectator_mouse_wheel_selects_active_menu_without_changing_flying_speed() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    set_local_spectator(&mut world);
    set_flying_abilities(&mut world, 0.05);
    let mut counters = NetCounters::default();
    assert!(input.handle_spectator_hotbar_action(&world, &mut counters, &None));

    handle_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &None,
        MouseScrollDelta::LineDelta(0.0, -1.0),
    );

    assert_eq!(input.spectator_menu_selected_slot(), Some(8));
    assert_eq!(world.local_player().abilities.unwrap().flying_speed, 0.05);
}

#[test]
fn spectator_mouse_wheel_clamps_speed_and_ignores_horizontal_steps() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    set_local_spectator(&mut world);
    set_flying_abilities(&mut world, 0.199);
    let mut counters = NetCounters::default();

    handle_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, 1.0),
    );
    assert_eq!(world.local_player().abilities.unwrap().flying_speed, 0.2);

    handle_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(1.0, 0.0),
    );

    assert_eq!(world.local_player().abilities.unwrap().flying_speed, 0.2);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn mouse_wheel_wraps_hotbar_selection_like_vanilla() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    handle_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, 1.0),
    );
    assert_eq!(world.local_player().selected_hotbar_slot, 8);
    assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(8));

    handle_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, -1.0),
    );
    assert_eq!(world.local_player().selected_hotbar_slot, 0);
    assert_eq!(counters.held_slot_commands_queued, 2);
    assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(0));
}

#[test]
fn unfocused_mouse_wheel_does_not_select_or_queue() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(false);
    let mut world = WorldStore::new();
    assert!(world.set_local_selected_hotbar_slot(4));
    let mut counters = NetCounters::default();

    handle_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, 1.0),
    );

    assert_eq!(world.local_player().selected_hotbar_slot, 4);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn sign_editor_mouse_wheel_does_not_select_or_queue() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_open_sign_editor(&mut input);
    assert!(world.set_local_selected_hotbar_slot(4));
    let mut counters = NetCounters::default();

    handle_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, 1.0),
    );

    assert_eq!(world.local_player().selected_hotbar_slot, 4);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn fractional_mouse_wheel_accumulates_before_selecting() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    assert!(world.set_local_selected_hotbar_slot(4));
    let mut counters = NetCounters::default();

    handle_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, 0.5),
    );
    assert_eq!(world.local_player().selected_hotbar_slot, 4);
    assert!(rx.try_recv().is_err());

    handle_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, 0.5),
    );

    assert_eq!(world.local_player().selected_hotbar_slot, 3);
    assert_eq!(counters.held_slot_commands_queued, 1);
    assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(3));
}
