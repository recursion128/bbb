use super::*;

#[test]
fn cartography_table_shift_click_result_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[2] = item_stack(90, 1);
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
        Some(PhysicalPosition::new(705.0, 324.0)),
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
            changed_slots: [
                (0, HashedStack::Empty),
                (1, HashedStack::Empty),
                (2, HashedStack::Empty),
                (38, HashedStack::Item(hashed_item(90, 1))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[2].item, ItemStackSummary::empty());
    assert_eq!(slots[38].item, item_stack(90, 1));
}

#[test]
fn cartography_table_left_click_result_slot_queues_predicted_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[2] = item_stack(90, 1);
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
        Some(PhysicalPosition::new(705.0, 324.0)),
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
            changed_slots: [
                (0, HashedStack::Empty),
                (1, HashedStack::Empty),
                (2, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Item(hashed_item(90, 1)),
        })
    );
    let inventory = world.inventory();
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[2].item, ItemStackSummary::empty());
    assert_eq!(inventory.cursor_item, item_stack(90, 1));
}

#[test]
fn cartography_table_shift_click_input_slots_queue_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[3].item, item_stack(42, 1));
    assert_eq!(slots[4].item, item_stack(43, 3));
}

#[test]
fn cartography_table_shift_click_player_additional_item_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_cartography_additional_items(BTreeMap::from([
        (43, bbb_client_world::CartographyAdditionalItemKind::Paper),
        (
            44,
            bbb_client_world::CartographyAdditionalItemKind::EmptyMap,
        ),
        (
            45,
            bbb_client_world::CartographyAdditionalItemKind::GlassPane,
        ),
    ]));
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[1].item, item_stack(43, 2));
    assert_eq!(slots[38].item, ItemStackSummary::empty());
}

#[test]
fn cartography_table_shift_click_player_map_id_item_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[3] = map_id_item_stack(42, 1, 7);
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
            changed_slots: [
                (0, HashedStack::Item(hashed_map_id_item(42, 1, 7))),
                (3, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, map_id_item_stack(42, 1, 7));
    assert_eq!(slots[3].item, ItemStackSummary::empty());
}

#[test]
fn cartography_table_shift_click_player_map_id_removed_component_item_queues_predicted_quick_move()
{
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    let mut map_stack = map_id_item_stack(42, 1, 7);
    map_stack
        .component_patch
        .removed_type_ids
        .push(VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID);
    items[3] = map_stack.clone();
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
            changed_slots: [
                (
                    0,
                    HashedStack::Item(hashed_map_id_removed_component_item(
                        42,
                        1,
                        7,
                        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID
                    ))
                ),
                (3, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, map_stack);
    assert_eq!(slots[3].item, ItemStackSummary::empty());
}

#[test]
fn loom_shift_click_input_slots_queue_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[2].item, ItemStackSummary::empty());
    assert_eq!(slots[4].item, item_stack(42, 3));
    assert_eq!(slots[5].item, item_stack(43, 2));
    assert_eq!(slots[6].item, item_stack(44, 1));
}

#[test]
fn loom_left_click_result_slot_queues_predicted_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut items = vec![ItemStackSummary::empty(); 40];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[3] = item_stack(90, 1);
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
        Some(PhysicalPosition::new(703.0, 342.0)),
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
            input: ContainerInput::Pickup,
            changed_slots: [
                (0, HashedStack::Empty),
                (1, HashedStack::Empty),
                (3, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Item(hashed_item(90, 1)),
        })
    );
    let inventory = world.inventory();
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[3].item, ItemStackSummary::empty());
    assert_eq!(inventory.cursor_item, item_stack(90, 1));
}

#[test]
fn loom_shift_click_result_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut items = vec![ItemStackSummary::empty(); 40];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[3] = item_stack(90, 1);
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
        Some(PhysicalPosition::new(703.0, 342.0)),
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
                (0, HashedStack::Empty),
                (1, HashedStack::Empty),
                (3, HashedStack::Empty),
                (39, HashedStack::Item(hashed_item(90, 1))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[3].item, ItemStackSummary::empty());
    assert_eq!(slots[39].item, item_stack(90, 1));
}

#[test]
fn hopper_shift_click_queues_bidirectional_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_HOPPER_ID,
        title: bbb_protocol::RichTextComponent::plain("Hopper"),
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
    let slots = open_container_slots(&world);
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, item_stack(42, 3));
    assert_eq!(slots[40].item, ItemStackSummary::empty());
}

#[test]
fn shulker_box_shift_click_queues_bidirectional_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SHULKER_BOX_ID,
        title: bbb_protocol::RichTextComponent::plain("Shulker Box"),
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
    let slots = open_container_slots(&world);
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, item_stack(42, 3));
    assert_eq!(slots[62].item, ItemStackSummary::empty());
}

#[test]
fn inventory_mouse_click_queues_container_zero_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
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
fn inventory_mouse_click_hashes_integer_component_patch() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: damage_item_stack(42, 1, 7),
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
            carried_item: HashedStack::Item(hashed_int_component_item(
                42,
                1,
                VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                7
            )),
        })
    );
}

#[test]
fn inventory_result_with_default_remainder_queues_predicted_container_zero_click() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_default_item_crafting_remainders(BTreeMap::from([(42, 43)]));
    let mut items = vec![ItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 0,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });
    assert!(world.open_local_inventory());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(714.0, 313.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 12,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: [
                (0, HashedStack::Empty),
                (
                    1,
                    HashedStack::Item(HashedItemStack {
                        item_id: 43,
                        count: 1,
                        components: HashedComponentPatch::default(),
                    }),
                ),
            ]
            .into(),
            carried_item: HashedStack::Item(HashedItemStack {
                item_id: 90,
                count: 1,
                components: HashedComponentPatch::default(),
            }),
        })
    );
    let inventory = world.inventory();
    assert_eq!(
        *inventory.inventory_menu_slot_item(0).unwrap(),
        ItemStackSummary::empty()
    );
    assert_eq!(
        *inventory.inventory_menu_slot_item(1).unwrap(),
        item_stack(43, 1)
    );
    assert_eq!(inventory.cursor_item, item_stack(90, 1));
}

#[test]
fn creative_inventory_middle_click_queues_container_zero_clone() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
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
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    apply_instabuild_abilities(&mut world);
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X1_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
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
    let mut input = ClientInputState::container_for_test();
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
    let mut input = ClientInputState::container_for_test();
    input.container_mut().inventory_last_click_slot = Some(36);
    input.container_mut().inventory_last_click_button_num = Some(0);
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

    // `handle_inventory_mouse_input` measures this against the 250 ms
    // `VANILLA_DOUBLE_CLICK_THRESHOLD` with its own `Instant::now()`, so the
    // stamp is taken here rather than before the fixture the scheduler may
    // spend that budget building.
    input.container_mut().inventory_last_click_at = Some(Instant::now() - Duration::from_millis(1));
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
    assert_eq!(input.container_mut().inventory_last_click_slot, Some(36));
    assert_eq!(
        input.container_mut().inventory_last_click_button_num,
        Some(0)
    );
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
    let mut input = ClientInputState::container_for_test();
    input.container_mut().inventory_last_click_slot = Some(36);
    input.container_mut().inventory_last_click_button_num = Some(1);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_cursor_item(SetCursorItem {
        item: item_stack(42, 4),
    });
    assert!(world.open_local_inventory());

    // `handle_inventory_mouse_input` measures this against the 250 ms
    // `VANILLA_DOUBLE_CLICK_THRESHOLD` with its own `Instant::now()`, so the
    // stamp is taken here rather than before the fixture the scheduler may
    // spend that budget building.
    input.container_mut().inventory_last_click_at = Some(Instant::now() - Duration::from_millis(1));
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

    input.container_mut().inventory_last_click_slot = Some(37);
    input.container_mut().inventory_last_click_button_num = Some(0);
    input.container_mut().inventory_last_click_at =
        Some(Instant::now() - VANILLA_DOUBLE_CLICK_THRESHOLD);

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
    let mut input = ClientInputState::container_for_test();
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
fn generic_container_left_drag_queues_predicted_quick_craft_sequence() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X1_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![ItemStackSummary::empty(); 45],
        carried_item: item_stack(42, 8),
    });

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
    assert!(rx.try_recv().is_err());
    assert!(handle_inventory_cursor_moved(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(PhysicalPosition::new(568.0, 320.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(handle_inventory_cursor_moved(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(PhysicalPosition::new(586.0, 320.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Released,
        Some(PhysicalPosition::new(586.0, 320.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 4);
    assert_quick_craft_click_for_container(
        &mut rx,
        7,
        12,
        -999,
        0,
        [].into(),
        HashedStack::Item(hashed_item(42, 8)),
    );
    assert_quick_craft_click_for_container(
        &mut rx,
        7,
        12,
        0,
        1,
        [].into(),
        HashedStack::Item(hashed_item(42, 8)),
    );
    assert_quick_craft_click_for_container(
        &mut rx,
        7,
        12,
        1,
        1,
        [].into(),
        HashedStack::Item(hashed_item(42, 8)),
    );
    assert_quick_craft_click_for_container(
        &mut rx,
        7,
        12,
        -999,
        2,
        [
            (0, HashedStack::Item(hashed_item(42, 4))),
            (1, HashedStack::Item(hashed_item(42, 4))),
        ]
        .into(),
        HashedStack::Empty,
    );
    assert!(rx.try_recv().is_err());
    assert_eq!(world.inventory().cursor_item, ItemStackSummary::empty());
    assert_eq!(open_container_slots(&world)[0].item, item_stack(42, 4));
    assert_eq!(open_container_slots(&world)[1].item, item_stack(42, 4));
}

#[test]
fn specialized_container_drag_selection_uses_world_slot_rules() {
    let mut input = ClientInputState::container_for_test();
    input.container_mut().inventory_quick_craft_button_num = Some(0);
    let mut world = WorldStore::new();
    world.set_enchantment_lapis_lazuli_item_ids(BTreeSet::from([43]));
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: bbb_protocol::RichTextComponent::plain("Enchanting Table"),
    });
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![ItemStackSummary::empty(); 38],
        carried_item: item_stack(42, 3),
    });
    let slots = inventory_screen_layout(&world).unwrap().slots;
    let enchantment_input_slot = slots[0].slot_id;
    let enchantment_lapis_slot = slots[1].slot_id;

    assert!(local_inventory_can_add_quick_craft_slot(
        &input,
        &world,
        enchantment_input_slot,
    ));
    assert!(!local_inventory_can_add_quick_craft_slot(
        &input,
        &world,
        enchantment_lapis_slot,
    ));

    world.apply_set_cursor_item(SetCursorItem {
        item: item_stack(43, 3),
    });
    assert!(local_inventory_can_add_quick_craft_slot(
        &input,
        &world,
        enchantment_lapis_slot,
    ));
}

#[test]
fn mount_drag_selection_uses_equipment_and_storage_slot_rules() {
    let mut input = ClientInputState::container_for_test();
    input.container_mut().inventory_quick_craft_button_num = Some(0);
    let mut world = WorldStore::new();
    crate::item_registry_test_fixture::install_equippable_item_default(&mut world, 90, "saddle");
    world.apply_add_entity(add_entity_with_type(42, 66));
    world.apply_mount_screen_open(MountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![ItemStackSummary::empty(); 53],
        carried_item: item_stack(90, 3),
    });
    let slots = inventory_screen_layout(&world).unwrap().slots;

    assert!(local_inventory_can_add_quick_craft_slot(
        &input,
        &world,
        slots[0].slot_id,
    ));
    assert!(!local_inventory_can_add_quick_craft_slot(
        &input,
        &world,
        slots[1].slot_id,
    ));
    assert!(local_inventory_can_add_quick_craft_slot(
        &input,
        &world,
        slots[2].slot_id,
    ));

    world.apply_set_cursor_item(SetCursorItem {
        item: item_stack(43, 3),
    });
    assert!(!local_inventory_can_add_quick_craft_slot(
        &input,
        &world,
        slots[0].slot_id,
    ));
    assert!(local_inventory_can_add_quick_craft_slot(
        &input,
        &world,
        slots[2].slot_id,
    ));
}

#[test]
fn inventory_right_drag_queues_quick_craft_one_per_slot() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
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
    let mut input = ClientInputState::container_for_test();
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
    let mut input = ClientInputState::container_for_test();
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
    let mut input = ClientInputState::container_for_test();
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
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X1_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
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
    let mut input = ClientInputState::container_for_test();
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
    let mut input = ClientInputState::container_for_test();
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
    let mut input = ClientInputState::container_for_test();
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

#[test]
fn server_opened_container_mouse_wheel_routes_bundle_selection_for_hovered_slot() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X1_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bundle_stack(42, 1, 3)],
        carried_item: ItemStackSummary::empty(),
    });

    assert!(handle_inventory_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, 1.0),
        Some(PhysicalPosition::new(568.0, 320.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.select_bundle_item_commands_queued, 1);
    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(open_container_slot_bundle_selection(&world, 0), Some(2));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SelectBundleItem(SelectBundleItem {
            slot_id: 0,
            selected_item_index: 2,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn local_inventory_swap_key_clears_selected_bundle_index_and_bundle_wheel_precedes_stonecutter_scroll(
) {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: bundle_stack(42, 1, 3),
    });
    assert!(world.open_local_inventory());
    assert!(world.apply_local_select_bundle_item(36, 1));

    // Swap key clears local bundle selection and sends SelectBundleItem(-1) before Swap.
    input.container_mut().inventory_hovered_slot = Some(36);
    handle_inventory_swap_key(&mut input, &mut world, &mut counters, &commands, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SelectBundleItem(SelectBundleItem {
            slot_id: 36,
            selected_item_index: -1,
        })
    );
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            slot_num: 36,
            button_num: 1,
            input: ContainerInput::Swap,
            ..
        })
    ));

    // Bundle wheel scroll takes priority over Stonecutter recipe list scrolling.
    world.apply_open_screen(OpenScreen {
        container_id: 11,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![ItemStackSummary::empty(); 38];
    items[0] = bundle_stack(42, 1, 3);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 11,
        state_id: 3,
        items,
        carried_item: ItemStackSummary::empty(),
    });
    let layout = inventory_screen_layout(&world).unwrap();
    let slot0 = layout.slots.iter().find(|s| s.slot_id == 0).unwrap();
    let origin_x = (1280 - layout.width) / 2;
    let origin_y = (720 - layout.height) / 2;
    let cursor = PhysicalPosition::new(
        f64::from(origin_x + slot0.x + 8),
        f64::from(origin_y + slot0.y + 8),
    );
    assert!(handle_inventory_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, 1.0),
        Some(cursor),
        PhysicalSize::new(1280, 720),
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SelectBundleItem(SelectBundleItem {
            slot_id: 0,
            selected_item_index: 2,
        })
    );
}

#[test]
fn shift_double_click_moves_matching_stacks_in_same_container_and_middle_drag_clones_in_creative() {
    let (tx, mut rx) = mpsc::channel(16);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X1_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    let mut items = vec![ItemStackSummary::empty(); 45];
    items[0] = item_stack(42, 3);
    items[1] = item_stack(42, 4);
    items[2] = item_stack(43, 2);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 5,
        items,
        // Vanilla Shift+Double-Click requires carrying an item on the cursor so the first
        // release records `lastQuickMoved` without the second press overwriting it.
        carried_item: item_stack(44, 1),
    });
    let layout = inventory_screen_layout(&world).unwrap();
    let origin_x = (1280 - layout.width) / 2;
    let origin_y = (720 - layout.height) / 2;
    let slot0 = layout.slots.iter().find(|s| s.slot_id == 0).unwrap();
    let pos0 = PhysicalPosition::new(
        f64::from(origin_x + slot0.x + 8),
        f64::from(origin_y + slot0.y + 8),
    );

    // Hold Shift and double-click slot 0: first click moves slot 0, second release moves slot 1.
    input.shift_left_down = true;
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(pos0),
        PhysicalSize::new(1280, 720),
    ));
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Released,
        Some(pos0),
        PhysicalSize::new(1280, 720),
    ));
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(pos0),
        PhysicalSize::new(1280, 720),
    ));
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Released,
        Some(pos0),
        PhysicalSize::new(1280, 720),
    ));
    assert_eq!(
        open_container_slots(&world)[0].item,
        ItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slots(&world)[1].item,
        ItemStackSummary::empty()
    );
    assert_eq!(open_container_slots(&world)[2].item, item_stack(43, 2));
    while rx.try_recv().is_ok() {}

    // Middle-click drag in Creative fills each dragged slot via QUICKCRAFT_TYPE_CLONE.
    input.shift_left_down = false;
    world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: false,
        flying: false,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    world.apply_set_cursor_item(SetCursorItem {
        item: item_stack(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID, 1),
    });
    let slot1 = layout.slots.iter().find(|s| s.slot_id == 1).unwrap();
    let pos1 = PhysicalPosition::new(
        f64::from(origin_x + slot1.x + 8),
        f64::from(origin_y + slot1.y + 8),
    );
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Middle,
        ElementState::Pressed,
        Some(pos0),
        PhysicalSize::new(1280, 720),
    ));
    handle_inventory_cursor_moved(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(pos0),
        PhysicalSize::new(1280, 720),
    );
    handle_inventory_cursor_moved(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(pos1),
        PhysicalSize::new(1280, 720),
    );
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Middle,
        ElementState::Released,
        Some(pos1),
        PhysicalSize::new(1280, 720),
    ));
    assert_eq!(
        open_container_slots(&world)[0].item,
        item_stack(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID, 64)
    );
    assert_eq!(
        open_container_slots(&world)[1].item,
        item_stack(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID, 64)
    );
}

#[test]
fn crafter_disabled_slot_release_with_carried_item_toggles_before_pickup() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 14,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafter"),
    });
    world.apply_container_set_content(ContainerSetContent {
        container_id: 14,
        state_id: 2,
        items: vec![ItemStackSummary::empty(); 46],
        carried_item: item_stack(42, 3),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 14,
        id: 0,
        value: 1,
    });
    let layout = inventory_screen_layout(&world).unwrap();
    let origin_x = (1280 - layout.width) / 2;
    let origin_y = (720 - layout.height) / 2;
    let slot0 = layout.slots.iter().find(|s| s.slot_id == 0).unwrap();
    let pos0 = PhysicalPosition::new(
        f64::from(origin_x + slot0.x + 8),
        f64::from(origin_y + slot0.y + 8),
    );

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(pos0),
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
        Some(pos0),
        PhysicalSize::new(1280, 720),
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerSlotStateChanged(bbb_protocol::packets::ContainerSlotStateChanged {
            slot_id: 0,
            container_id: 14,
            new_state: true,
        })
    );
    assert_eq!(world.open_container_data_value(0), Some(0));
}
