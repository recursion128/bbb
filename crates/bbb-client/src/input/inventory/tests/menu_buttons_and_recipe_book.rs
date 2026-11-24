use super::*;

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
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
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
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_recipes(UpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes: (0..6).map(|_| stonecutter_recipe(vec![42])).collect(),
    });
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
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

    assert_eq!(
        world.open_container_data_value(STONECUTTER_SELECTED_RECIPE_DATA_ID),
        Some(5)
    );
    assert_eq!(
        world.take_pending_local_interaction_sound_actions(),
        vec![bbb_client_world::LocalInteractionSoundAction::StonecutterSelectRecipe]
    );
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
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_recipes(UpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes: (0..6).map(|_| stonecutter_recipe(vec![42])).collect(),
    });
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
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
fn stonecutter_recipe_button_click_requires_matching_input_recipe() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
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
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_recipes(UpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes: (0..6).map(|_| stonecutter_recipe(vec![42])).collect(),
    });
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
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
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafter"),
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
    assert_eq!(world.open_container_data_value(0), Some(1));
    assert_eq!(
        world.take_pending_local_interaction_sound_actions(),
        vec![
            bbb_client_world::LocalInteractionSoundAction::CrafterSlotStateChanged {
                enabled: false
            }
        ]
    );
}

#[test]
fn crafter_hotbar_swap_enables_disabled_empty_slot_before_click() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafter"),
    });
    let mut items = vec![ItemStackSummary::empty(); 46];
    items[36] = item_stack(42, 3);
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
    input.container_mut().inventory_hovered_slot = Some(0);

    assert!(handle_inventory_key_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        KeyCode::Digit1,
    ));

    assert_eq!(counters.container_slot_state_changed_commands_queued, 1);
    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerSlotStateChanged(ContainerSlotStateChanged {
            slot_id: 0,
            container_id: 7,
            new_state: true,
        })
    );
    let NetCommand::ContainerClick(click) = rx.try_recv().unwrap() else {
        panic!("expected the ordinary swap click after the slot-state packet");
    };
    assert_eq!(click.container_id, 7);
    assert_eq!(click.state_id, 12);
    assert_eq!(click.slot_num, 0);
    assert_eq!(click.button_num, 0);
    assert_eq!(click.input, ContainerInput::Swap);
    assert_eq!(
        click.changed_slots.keys().copied().collect::<Vec<_>>(),
        vec![0, 36]
    );
    assert_eq!(click.carried_item, HashedStack::Empty);
    assert_eq!(world.open_container_data_value(0), Some(0));
    assert_eq!(
        open_container_slots(&world)
            .into_iter()
            .find(|slot| slot.slot == 0)
            .unwrap()
            .item,
        item_stack(42, 3)
    );
    assert_eq!(
        world.take_pending_local_interaction_sound_actions(),
        vec![
            bbb_client_world::LocalInteractionSoundAction::CrafterSlotStateChanged {
                enabled: true
            }
        ]
    );
}

#[test]
fn crafter_hotbar_swap_keeps_disabled_slot_when_source_is_empty() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafter"),
    });
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![ItemStackSummary::empty(); 46],
        carried_item: ItemStackSummary::empty(),
    });
    world.apply_container_set_data(ContainerSetData {
        container_id: 7,
        id: 0,
        value: 1,
    });
    input.container_mut().inventory_hovered_slot = Some(0);

    assert!(handle_inventory_key_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        KeyCode::Digit1,
    ));

    assert_eq!(counters.container_slot_state_changed_commands_queued, 0);
    assert_eq!(counters.container_click_commands_queued, 1);
    let NetCommand::ContainerClick(click) = rx.try_recv().unwrap() else {
        panic!("empty hotbar swaps still send the ordinary container click");
    };
    assert_eq!(click.input, ContainerInput::Swap);
    assert!(click.changed_slots.is_empty());
    assert_eq!(world.open_container_data_value(0), Some(1));
    assert!(world
        .take_pending_local_interaction_sound_actions()
        .is_empty());
}

#[test]
fn crafter_hotbar_swap_does_not_apply_partial_batch_under_backpressure() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafter"),
    });
    let mut items = vec![ItemStackSummary::empty(); 46];
    items[36] = item_stack(42, 3);
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
    input.container_mut().inventory_hovered_slot = Some(0);

    assert!(handle_inventory_key_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        KeyCode::Digit1,
    ));

    assert_eq!(counters.container_slot_state_changed_commands_queued, 0);
    assert_eq!(counters.container_click_commands_queued, 0);
    assert!(rx.try_recv().is_err());
    assert_eq!(world.open_container_data_value(0), Some(1));
    assert!(world
        .inventory()
        .container_slot(world.inventory().open_container.as_ref().unwrap(), 0)
        .is_some_and(|slot| item_stack_is_empty(slot.item)));
    assert!(world
        .take_pending_local_interaction_sound_actions()
        .is_empty());
}

#[test]
fn enchantment_table_option_click_queues_button_command_when_cost_is_available() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: bbb_protocol::RichTextComponent::plain("Enchanting Table"),
    });
    let mut items = vec![ItemStackSummary::empty(); 38];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 3);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: items.clone(),
        carried_item: ItemStackSummary::empty(),
    });
    world.apply_container_set_data(ContainerSetData {
        container_id: 7,
        id: 2,
        value: 30,
    });
    world.apply_player_experience(PlayerExperience {
        progress: 0.5,
        level: 30,
        total: 200,
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
    assert_eq!(open_container_slots(&world)[0].item, items[0]);
    assert_eq!(world.local_player().experience.unwrap().level, 30);
}

#[test]
fn enchantment_table_option_click_ignores_zero_cost_buttons() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: bbb_protocol::RichTextComponent::plain("Enchanting Table"),
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
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
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
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
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
            primary_effect: Some(VANILLA_MOB_EFFECT_STRENGTH_ID),
            secondary_effect: Some(VANILLA_MOB_EFFECT_STRENGTH_ID),
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
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
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
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
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
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
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
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LECTERN_ID,
        title: bbb_protocol::RichTextComponent::plain("Lectern"),
    });
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![lectern_writable_book(3)],
        carried_item: ItemStackSummary::empty(),
    });
    world.apply_container_set_data(ContainerSetData {
        container_id: 7,
        id: 0,
        value: 1,
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
    assert_eq!(counters.container_button_click_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(661.0, 160.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_button_click_commands_queued, 1);
    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(world.open_container_data_value(0), Some(1));
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
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LECTERN_ID,
        title: bbb_protocol::RichTextComponent::plain("Lectern"),
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(560.0, 197.0)),
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
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LECTERN_ID,
        title: bbb_protocol::RichTextComponent::plain("Lectern"),
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(660.0, 197.0)),
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
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    assert!(world.apply_merchant_offers(merchant_offers(7, 4)));
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[3] = item_stack(45, 5);
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
        Some(PhysicalPosition::new(545.0, 365.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.select_trade_commands_queued, 1);
    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(input.merchant_focused_trade_row(), Some(3));
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
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, item_stack(45, 5));
    assert_eq!(slots[3].item, ItemStackSummary::empty());
    assert!(rx.try_recv().is_err());
}

#[test]
fn merchant_mouse_wheel_scrolls_visible_trade_window() {
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
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
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
    assert!(input.container_mut().merchant_trade_scrolling);

    assert!(handle_inventory_cursor_moved(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(PhysicalPosition::new(598.0, 420.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(input.container_mut().merchant_trade_scrolling);
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
    assert!(!input.container_mut().merchant_trade_scrolling);

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
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
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
    assert_eq!(input.merchant_focused_trade_row(), None);
    assert!(rx.try_recv().is_err());
}

#[test]
fn generic_container_mouse_click_queues_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
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
fn generic_container_double_click_predicts_pickup_all_before_queueing() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.container_mut().inventory_last_click_slot = Some(0);
    input.container_mut().inventory_last_click_button_num = Some(0);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X1_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    let mut items = vec![ItemStackSummary::empty(); 45];
    items[1] = item_stack(42, 3);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: item_stack(42, 5),
    });

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
        Some(PhysicalPosition::new(568.0, 320.0)),
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
            button_num: 0,
            input: ContainerInput::PickupAll,
            changed_slots: [(1, HashedStack::Empty)].into(),
            carried_item: HashedStack::Item(hashed_item(42, 8)),
        })
    );
    assert_eq!(
        open_container_slots(&world)[1].item,
        ItemStackSummary::empty()
    );
    assert_eq!(world.inventory().cursor_item, item_stack(42, 8));
}

#[test]
fn mount_horse_mouse_click_queues_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
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
        open_container_slots(&world)[2].item,
        ItemStackSummary::empty()
    );
}

#[test]
fn mount_horse_shift_click_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
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
            changed_slots: [
                (2, HashedStack::Empty),
                (52, HashedStack::Item(hashed_item(42, 3))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = open_container_slots(&world);
    assert_eq!(slots[2].item, ItemStackSummary::empty());
    assert_eq!(slots[52].item, item_stack(42, 3));
}

#[test]
fn mount_horse_shift_click_queues_predicted_saddle_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    crate::item_registry_test_fixture::install_equippable_item_default(&mut world, 90, "saddle");
    world.apply_add_entity(add_entity_with_type(42, 66));
    world.apply_mount_screen_open(MountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });
    let mut items = vec![ItemStackSummary::empty(); 53];
    items[17] = item_stack(90, 1);
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
            slot_num: 17,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Item(hashed_item(90, 1))),
                (17, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = open_container_slots(&world);
    assert_eq!(slots[0].item, item_stack(90, 1));
    assert_eq!(slots[17].item, ItemStackSummary::empty());
}

#[test]
fn recipe_book_button_click_toggles_local_setting_and_queues_packet() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    world.apply_recipe_book_settings(RecipeBookSettings {
        crafting: RecipeBookTypeSettings {
            open: false,
            filtering: true,
        },
        furnace: RecipeBookTypeSettings::default(),
        blast_furnace: RecipeBookTypeSettings::default(),
        smoker: RecipeBookTypeSettings::default(),
    });
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(558.0, 312.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert!(world.recipe_book().settings.crafting.open);
    assert_eq!(counters.recipe_book_change_settings_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RecipeBookChangeSettings(RecipeBookChangeSettingsCommand {
            book_type: RecipeBookType::Crafting,
            open: true,
            filtering: true,
        })
    );

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(635.0, 312.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert!(!world.recipe_book().settings.crafting.open);
    assert_eq!(counters.recipe_book_change_settings_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RecipeBookChangeSettings(RecipeBookChangeSettingsCommand {
            book_type: RecipeBookType::Crafting,
            open: false,
            filtering: true,
        })
    );
}

#[test]
fn scaled_local_inventory_recipe_button_click_uses_the_logical_gui_surface() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());
    let surface = PhysicalSize::new(427, 240);
    let cursor = Some(PhysicalPosition::new(239.18671875, 107.33333333333333));

    assert_eq!(
        recipe_book_button_at_position(&world, cursor, surface),
        Some(RecipeBookType::Crafting)
    );
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        cursor,
        surface,
    ));

    assert!(world.recipe_book().settings.crafting.open);
    assert_eq!(counters.recipe_book_change_settings_commands_queued, 1);
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::RecipeBookChangeSettings(
            RecipeBookChangeSettingsCommand {
                book_type: RecipeBookType::Crafting,
                open: true,
                filtering: false,
            }
        ))
    ));
}

#[test]
fn recipe_book_filter_click_toggles_filtering_and_queues_packet() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    world.apply_recipe_book_settings(RecipeBookSettings {
        crafting: RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: RecipeBookTypeSettings::default(),
        blast_furnace: RecipeBookTypeSettings::default(),
        smoker: RecipeBookTypeSettings::default(),
    });
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(591.0, 290.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert!(world.recipe_book().settings.crafting.open);
    assert!(world.recipe_book().settings.crafting.filtering);
    assert_eq!(counters.recipe_book_change_settings_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RecipeBookChangeSettings(RecipeBookChangeSettingsCommand {
            book_type: RecipeBookType::Crafting,
            open: true,
            filtering: true,
        })
    );

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(591.0, 290.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert!(world.recipe_book().settings.crafting.open);
    assert!(!world.recipe_book().settings.crafting.filtering);
    assert_eq!(counters.recipe_book_change_settings_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RecipeBookChangeSettings(RecipeBookChangeSettingsCommand {
            book_type: RecipeBookType::Crafting,
            open: true,
            filtering: false,
        })
    );
}

#[test]
fn narrow_recipe_book_consumes_underlying_container_click_and_hover() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ItemStackSummary::empty(); 46];
    items[45] = item_stack(90, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });
    world.apply_recipe_book_settings(RecipeBookSettings {
        crafting: RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: RecipeBookTypeSettings::default(),
        blast_furnace: RecipeBookTypeSettings::default(),
        smoker: RecipeBookTypeSettings::default(),
    });
    let surface_size = PhysicalSize::new(378, 720);
    let origin_x = (378.0 - 176.0) / 2.0;
    let origin_y = (720.0 - 166.0) / 2.0;
    let hotbar_slot = Some(PhysicalPosition::new(
        origin_x + 152.0 + 8.0,
        origin_y + 142.0 + 8.0,
    ));

    assert!(handle_inventory_cursor_moved(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        hotbar_slot,
        surface_size,
    ));
    assert_eq!(input.inventory_hovered_slot(), None);
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        hotbar_slot,
        surface_size,
    ));
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Released,
        hotbar_slot,
        surface_size,
    ));

    assert!(world.recipe_book().settings.crafting.open);
    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(counters.recipe_book_change_settings_commands_queued, 0);
    assert_eq!(
        open_container_slots(&world)
            .into_iter()
            .find(|slot| slot.slot == 45)
            .unwrap()
            .item,
        item_stack(90, 1)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn recipe_book_search_click_focuses_and_edits_local_text() {
    let commands = None;
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    world.apply_recipe_book_settings(RecipeBookSettings {
        crafting: RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: RecipeBookTypeSettings::default(),
        blast_furnace: RecipeBookTypeSettings::default(),
        smoker: RecipeBookTypeSettings::default(),
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(506.0, 291.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(input.container_mut().recipe_book_search_focused);

    assert!(handle_inventory_text_input(
        &mut input,
        &world,
        &mut counters,
        &commands,
        None,
        "axe",
    ));
    assert_eq!(input.container_mut().recipe_book_search.text(), "axe");

    assert!(handle_inventory_key_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        KeyCode::Backspace,
    ));
    assert_eq!(input.container_mut().recipe_book_search.text(), "ax");
    assert_eq!(counters.recipe_book_change_settings_commands_queued, 0);
}

#[test]
fn recipe_book_excitedze_search_requests_the_en_pt_language_decision() {
    let commands = None;
    let mut input = ClientInputState::container_for_test();
    input.container_mut().recipe_book_search_focused = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    world.apply_recipe_book_settings(RecipeBookSettings {
        crafting: RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: RecipeBookTypeSettings::default(),
        blast_furnace: RecipeBookTypeSettings::default(),
        smoker: RecipeBookTypeSettings::default(),
    });

    assert!(handle_inventory_text_input(
        &mut input,
        &world,
        &mut counters,
        &commands,
        None,
        "EXCITEDZE",
    ));
    assert_eq!(input.container_mut().recipe_book_search.text(), "EXCITEDZE");
    assert_eq!(input.take_recipe_book_language_requests(), 1);

    // Vanilla checks after EditBox-accepted keys too, including cursor-only
    // edits, but not after the focused-field guard consumes an unrelated key.
    assert!(handle_inventory_key_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        KeyCode::ArrowLeft,
    ));
    assert_eq!(input.take_recipe_book_language_requests(), 1);
    assert!(handle_inventory_key_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        KeyCode::F3,
    ));
    assert_eq!(input.take_recipe_book_language_requests(), 0);

    // A rejected character likewise does not reach checkSearchStringUpdate.
    assert!(!handle_inventory_text_input(
        &mut input,
        &world,
        &mut counters,
        &commands,
        None,
        "\n",
    ));
    assert_eq!(input.take_recipe_book_language_requests(), 0);

    assert!(handle_inventory_key_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        KeyCode::Backspace,
    ));
    assert_eq!(input.take_recipe_book_language_requests(), 0);
}

#[test]
fn recipe_book_tab_click_selects_local_tab_without_packet() {
    let commands = None;
    let mut input = ClientInputState::container_for_test();
    input.container_mut().recipe_book_search_focused = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    world.apply_recipe_book_settings(RecipeBookSettings {
        crafting: RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: RecipeBookTypeSettings::default(),
        blast_furnace: RecipeBookTypeSettings::default(),
        smoker: RecipeBookTypeSettings::default(),
    });
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![crafting_recipe_book_entry(20, 2, 120)],
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(452.0, 309.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(input.recipe_book_crafting_tab_index, 1);
    assert!(!input.container_mut().recipe_book_search_focused);
    assert_eq!(counters.recipe_book_change_settings_commands_queued, 0);
}

#[test]
fn recipe_book_recipe_click_uses_craftable_filter_selection() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ItemStackSummary::empty(); 46];
    items[10] = item_stack(50, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });
    world.apply_recipe_book_settings(RecipeBookSettings {
        crafting: RecipeBookTypeSettings {
            open: true,
            filtering: true,
        },
        furnace: RecipeBookTypeSettings::default(),
        blast_furnace: RecipeBookTypeSettings::default(),
        smoker: RecipeBookTypeSettings::default(),
    });
    apply_item_tags(&mut world, vec![("minecraft:planks", vec![50])]);
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            crafting_recipe_book_entry_with_requirements(20, 2, 120, vec![vec![51]]),
            crafting_recipe_book_entry_with_requirement_summaries(
                21,
                2,
                121,
                vec![IngredientSummary {
                    tag: Some("minecraft:planks".to_string()),
                    item_ids: Vec::new(),
                }],
            ),
        ],
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(496.0, 311.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 21,
            use_max_items: false,
        })
    );
}

#[test]
fn recipe_book_bounds_persistence_and_crafting_slot_clear_match_vanilla() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ItemStackSummary::empty(); 46];
    items[10] = item_stack(42, 4);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 2,
        items,
        carried_item: item_stack(42, 2),
    });
    world.apply_recipe_book_settings(RecipeBookSettings {
        crafting: RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: RecipeBookTypeSettings::default(),
        blast_furnace: RecipeBookTypeSettings::default(),
        smoker: RecipeBookTypeSettings::default(),
    });
    let layout = inventory_screen_layout(&world).unwrap();
    let origin_x = (1280 - layout.width) / 2;
    let origin_y = (720 - layout.height) / 2;
    // Rightmost 5px of main GUI (149 + 176 = 325 total width) must NOT count as outside drop.
    let right_edge_pos = PhysicalPosition::new(
        f64::from(origin_x + layout.recipe_book.as_ref().unwrap().main_gui_x_offset + 174),
        f64::from(origin_y + 10),
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(right_edge_pos), PhysicalSize::new(1280, 720)),
        Some(InventoryClickTarget::EmptyPanel)
    );
    // Recipe Book tab bounds must also be treated as inside (no outside drop on right-click).
    let tab_pos = PhysicalPosition::new(
        f64::from(origin_x + layout.recipe_book.as_ref().unwrap().x - 10),
        f64::from(origin_y + 10),
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(tab_pos), PhysicalSize::new(1280, 720)),
        Some(InventoryClickTarget::EmptyPanel)
    );

    // Clicking a crafting grid slot clears recipe_book_last_placed_recipe.
    input.container_mut().recipe_book_last_placed_recipe = Some((7, 21));
    let grid_slot = layout.slots.iter().find(|s| s.slot_id == 1).unwrap();
    let grid_pos = PhysicalPosition::new(
        f64::from(origin_x + grid_slot.x + 8),
        f64::from(origin_y + grid_slot.y + 8),
    );
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(grid_pos),
        PhysicalSize::new(1280, 720),
    ));
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Released,
        Some(grid_pos),
        PhysicalSize::new(1280, 720),
    ));
    assert_eq!(input.container().recipe_book_last_placed_recipe, None);
    let _ = rx.try_recv();

    // Closing the screen persists only search text, clearing focus/overlay/last_placed.
    input
        .container_mut()
        .recipe_book_search
        .set_text("planks", None);
    input.container_mut().recipe_book_search_focused = true;
    input.container_mut().recipe_book_last_placed_recipe = Some((7, 21));
    input.clear_pressed();
    assert_eq!(input.container().recipe_book_search.text(), "planks");
    assert!(!input.container().recipe_book_search_focused);
    assert_eq!(input.container().recipe_book_last_placed_recipe, None);
}
