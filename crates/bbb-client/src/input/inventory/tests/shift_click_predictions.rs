use super::*;

#[test]
fn furnace_mouse_click_queues_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_FURNACE_ID,
        title: bbb_protocol::RichTextComponent::plain("Furnace"),
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
        open_container_slots(&world)[0].item,
        ItemStackSummary::empty()
    );
}

#[test]
fn furnace_shift_click_queues_quick_move_to_input_slot() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
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
        menu_type_id: VANILLA_MENU_TYPE_FURNACE_ID,
        title: bbb_protocol::RichTextComponent::plain("Furnace"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, item_stack(42, 3));
    assert_eq!(slots[3].item, ItemStackSummary::empty());
}

#[test]
fn generic_container_shift_click_queues_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X6_ID,
        title: bbb_protocol::RichTextComponent::plain("Large Chest"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[89].item, item_stack(42, 3));
}

#[test]
fn generic_3x3_shift_click_queues_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Dispenser"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[44].item, item_stack(42, 3));
}

#[test]
fn crafting_table_shift_click_input_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[10].item, item_stack(42, 3));
}

#[test]
fn enchantment_table_shift_click_input_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: bbb_protocol::RichTextComponent::plain("Enchanting Table"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[37].item, item_stack(42, 1));
}

#[test]
fn enchantment_table_shift_click_player_lapis_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_enchantment_lapis_lazuli_item_ids(BTreeSet::from([43]));
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: bbb_protocol::RichTextComponent::plain("Enchanting Table"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[1].item, item_stack(43, 3));
    assert_eq!(slots[37].item, ItemStackSummary::empty());
}

#[test]
fn enchantment_table_shift_click_player_item_queues_predicted_input_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_enchantment_lapis_lazuli_item_ids(BTreeSet::from([43]));
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: bbb_protocol::RichTextComponent::plain("Enchanting Table"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, item_stack(50, 1));
    assert_eq!(slots[37].item, item_stack(50, 2));
}

#[test]
fn non_local_quick_move_with_unhashable_prediction_falls_back_and_accepts_correction() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ItemStackSummary::empty(); 46];
    let mut bundle = bundle_stack(42, 3, 1);
    bundle.component_patch.added_type_ids.push(
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID,
    );
    bundle.component_patch.additional_trade_cost = Some(0);
    items[1] = bundle.clone();
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[1].item, bundle);
    assert_eq!(slots[10].item, ItemStackSummary::empty());

    let corrected = bundle;
    let mut corrected_items = vec![ItemStackSummary::empty(); 46];
    corrected_items[45] = corrected.clone();
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 13,
        items: corrected_items,
        carried_item: ItemStackSummary::empty(),
    });

    let slots = open_container_slots(&world);
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[45].item, corrected);
    assert_eq!(
        world.inventory().open_container.as_ref().unwrap().state_id,
        13
    );
    assert_eq!(world.inventory().cursor_item, ItemStackSummary::empty());
}

#[test]
fn crafting_result_shift_click_queues_single_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_default_item_crafting_remainders(BTreeMap::new());
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 2);
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
    // Remaining grid ingredients defer multi-craft QuickMove to server authority.
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 13,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: Default::default(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, item_stack(90, 1));
    assert_eq!(slots[1].item, item_stack(42, 2));
    assert_eq!(slots[45].item, ItemStackSummary::empty());
}

#[test]
fn crafting_result_left_click_queues_predicted_pickup_with_empty_result() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_default_item_crafting_remainders(BTreeMap::new());
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 2);
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
            input: ContainerInput::Pickup,
            changed_slots: [
                (0, HashedStack::Empty),
                (1, HashedStack::Item(hashed_item(42, 1))),
            ]
            .into(),
            carried_item: HashedStack::Item(hashed_item(90, 1)),
        })
    );
    let inventory = world.inventory();
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, item_stack(42, 1));
    assert_eq!(inventory.cursor_item, item_stack(90, 1));
}

#[test]
fn crafting_result_right_click_queues_full_predicted_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_default_item_crafting_remainders(BTreeMap::new());
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 3);
    items[1] = item_stack(42, 2);
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
        MouseButton::Right,
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
            button_num: 1,
            input: ContainerInput::Pickup,
            changed_slots: [
                (0, HashedStack::Empty),
                (1, HashedStack::Item(hashed_item(42, 1))),
            ]
            .into(),
            carried_item: HashedStack::Item(hashed_item(90, 3)),
        })
    );
    let inventory = world.inventory();
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, item_stack(42, 1));
    assert_eq!(inventory.cursor_item, item_stack(90, 3));
}

#[test]
fn crafting_result_with_default_remainder_queues_predicted_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_default_item_crafting_remainders(BTreeMap::from([(42, 43)]));
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
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
            input: ContainerInput::Pickup,
            changed_slots: [
                (0, HashedStack::Empty),
                (1, HashedStack::Item(hashed_item(43, 1))),
            ]
            .into(),
            carried_item: HashedStack::Item(hashed_item(90, 1)),
        })
    );
    let inventory = world.inventory();
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, item_stack(43, 1));
    assert_eq!(inventory.cursor_item, item_stack(90, 1));
    assert!(rx.try_recv().is_err());
}

#[test]
fn crafting_result_with_recipe_specific_remainder_uses_default_client_prediction() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_default_item_crafting_remainders(BTreeMap::new());
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 2);
    items[5] = item_stack(43, 2);
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
    // Remaining grid ingredients defer multi-craft QuickMove to server authority.
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 13,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: Default::default(),
            carried_item: HashedStack::Empty,
        })
    );
    let inventory = world.inventory();
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, item_stack(90, 1));
    assert_eq!(slots[1].item, item_stack(42, 2));
    assert_eq!(slots[5].item, item_stack(43, 2));
    assert_eq!(slots[45].item, ItemStackSummary::empty());
    assert_eq!(inventory.cursor_item, ItemStackSummary::empty());
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_shift_click_player_item_queues_predicted_input_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, item_stack(42, 3));
    assert_eq!(slots[30].item, ItemStackSummary::empty());
}

#[test]
fn anvil_shift_click_input_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[3].item, item_stack(42, 1));
}

#[test]
fn anvil_shift_click_result_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    items[2] = item_stack(90, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });
    world.apply_container_set_data(ContainerSetData {
        container_id: 7,
        id: 0,
        value: 1,
    });
    world.apply_player_experience(PlayerExperience {
        progress: 0.0,
        level: 1,
        total: 0,
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(694.0, 332.0)),
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
fn anvil_left_click_result_slot_queues_predicted_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    items[2] = item_stack(90, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });
    world.apply_container_set_data(ContainerSetData {
        container_id: 7,
        id: 0,
        value: 1,
    });
    world.apply_player_experience(PlayerExperience {
        progress: 0.0,
        level: 1,
        total: 0,
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(694.0, 332.0)),
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
            changed_slots: [(0, HashedStack::Empty), (2, HashedStack::Empty)].into(),
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
fn smithing_shift_click_input_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[4].item, item_stack(42, 1));
}

#[test]
fn smithing_shift_click_result_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
    });
    let mut items = vec![ItemStackSummary::empty(); 40];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[2] = item_stack(44, 1);
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
        Some(PhysicalPosition::new(658.0, 333.0)),
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
                (2, HashedStack::Empty),
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
    assert_eq!(slots[2].item, ItemStackSummary::empty());
    assert_eq!(slots[3].item, ItemStackSummary::empty());
    assert_eq!(slots[39].item, item_stack(90, 1));
}

#[test]
fn smithing_left_click_result_slot_queues_predicted_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
    });
    let mut items = vec![ItemStackSummary::empty(); 40];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[2] = item_stack(44, 1);
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
        Some(PhysicalPosition::new(658.0, 333.0)),
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
                (2, HashedStack::Empty),
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
    assert_eq!(slots[2].item, ItemStackSummary::empty());
    assert_eq!(slots[3].item, ItemStackSummary::empty());
    assert_eq!(inventory.cursor_item, item_stack(90, 1));
}

#[test]
fn smithing_shift_click_player_template_queues_predicted_input_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
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
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, item_stack(42, 1));
    assert_eq!(slots[31].item, ItemStackSummary::empty());
}

#[test]
fn merchant_shift_click_payment_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[3].item, item_stack(42, 3));
}

#[test]
fn merchant_left_click_result_slot_queues_predicted_trade_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 3);
    items[2] = item_stack(99, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });
    assert!(world.apply_merchant_offers(merchant_offers(7, 1)));

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(730.0, 322.0)),
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
            changed_slots: [(0, HashedStack::Item(hashed_item(42, 2)))].into(),
            carried_item: HashedStack::Item(hashed_item(99, 1)),
        })
    );
    let inventory = world.inventory();
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, item_stack(42, 2));
    assert_eq!(slots[2].item, item_stack(99, 1));
    assert_eq!(inventory.cursor_item, item_stack(99, 1));
}

#[test]
fn merchant_shift_click_result_slot_queues_predicted_trade_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 3);
    items[2] = item_stack(99, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });
    assert!(world.apply_merchant_offers(merchant_offers(7, 1)));

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(730.0, 322.0)),
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
                (2, HashedStack::Empty),
                (38, HashedStack::Item(hashed_item(99, 3))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[2].item, ItemStackSummary::empty());
    assert_eq!(slots[38].item, item_stack(99, 3));
}

#[test]
fn beacon_shift_click_single_payment_item_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    apply_item_tags(
        &mut world,
        vec![("minecraft:beacon_payment_items", vec![42])],
    );
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, item_stack(42, 1));
    assert_eq!(slots[1].item, ItemStackSummary::empty());
}

#[test]
fn brewing_stand_shift_click_potion_item_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    crate::item_registry_test_fixture::install_max_stack_size_default(&mut world, 42, 64);
    world.set_brewing_potion_item_ids(BTreeSet::from([42]));
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BREWING_STAND_ID,
        title: bbb_protocol::RichTextComponent::plain("Brewing Stand"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, item_stack(42, 1));
    assert_eq!(slots[32].item, item_stack(42, 2));
}

#[test]
fn grindstone_shift_click_input_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[3].item, item_stack(42, 1));
}

#[test]
fn grindstone_shift_click_player_to_input_queues_server_authoritative_click() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[3].item, item_stack(42, 3));
}

#[test]
fn grindstone_shift_click_default_damageable_player_item_queues_predicted_input_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    crate::item_registry_test_fixture::install_damageable_item_defaults(&mut world, 42, 100);
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, item_stack(42, 1));
    assert_eq!(slots[3].item, ItemStackSummary::empty());
}

#[test]
fn grindstone_shift_click_player_range_queues_predicted_quick_move_when_inputs_full() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[3].item, ItemStackSummary::empty());
    assert_eq!(slots[31].item, item_stack(42, 3));
}

#[test]
fn grindstone_shift_click_result_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
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
            changed_slots: [
                (0, HashedStack::Empty),
                (2, HashedStack::Empty),
                (38, HashedStack::Item(hashed_item(90, 1))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[2].item, ItemStackSummary::empty());
    assert_eq!(slots[38].item, item_stack(90, 1));
}

#[test]
fn grindstone_left_click_result_slot_queues_predicted_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
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
fn stonecutter_shift_click_input_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[2].item, item_stack(42, 3));
}

#[test]
fn stonecutter_shift_click_valid_recipe_input_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_recipes(UpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes: vec![stonecutter_recipe(vec![42])],
    });
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, item_stack(42, 3));
    assert_eq!(slots[2].item, ItemStackSummary::empty());
}

#[test]
fn stonecutter_left_click_result_slot_queues_predicted_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
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
            input: ContainerInput::Pickup,
            changed_slots: [(0, HashedStack::Empty), (1, HashedStack::Empty)].into(),
            carried_item: HashedStack::Item(hashed_item(90, 1)),
        })
    );
    let inventory = world.inventory();
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(inventory.cursor_item, item_stack(90, 1));
}

#[test]
fn stonecutter_left_click_result_slot_with_remaining_input_queues_server_authoritative_click() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![ItemStackSummary::empty(); 38];
    items[0] = item_stack(42, 2);
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
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::Empty,
        })
    );
    let inventory = world.inventory();
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, item_stack(42, 2));
    assert_eq!(slots[1].item, item_stack(90, 1));
    assert_eq!(inventory.cursor_item, ItemStackSummary::empty());
    assert!(rx.try_recv().is_err());
}

#[test]
fn stonecutter_shift_click_result_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
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
            changed_slots: [
                (0, HashedStack::Empty),
                (1, HashedStack::Empty),
                (37, HashedStack::Item(hashed_item(90, 1))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[37].item, item_stack(90, 1));
}
