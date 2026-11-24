use super::*;

#[test]
fn escape_key_closes_narrow_recipe_book_before_container() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();

    crate::input::key_events::dispatch_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
        PhysicalSize::new(378, 720),
        &FixedCommandSuggestionTextMeasurer,
        None,
        None,
        &mut true,
        &mut crate::simulation::effects::PlatformEffects::default(),
    );

    assert!(world.inventory().open_container.is_some());
    assert!(!world.recipe_book().settings.crafting.open);
    assert_eq!(counters.container_close_commands_queued, 0);
    assert_eq!(counters.recipe_book_change_settings_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RecipeBookChangeSettings(
            bbb_protocol::packets::RecipeBookChangeSettingsCommand {
                book_type: bbb_protocol::packets::RecipeBookType::Crafting,
                open: false,
                filtering: false,
            }
        )
    );
}

#[test]
fn local_inventory_hovered_drop_key_queues_throw_one_container_click() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.container_mut().inventory_hovered_slot = Some(36);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: ProtocolItemStackSummary {
            item_id: Some(42),
            count: 3,
            component_patch: Default::default(),
        },
    });
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 0,
            slot_num: 36,
            button_num: 0,
            input: ContainerInput::Throw,
            changed_slots: [(
                36,
                HashedStack::Item(HashedItemStack {
                    item_id: 42,
                    count: 2,
                    components: HashedComponentPatch::default(),
                }),
            )]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
}

#[test]
fn local_inventory_hovered_control_drop_key_queues_throw_stack_container_click() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.container_mut().inventory_hovered_slot = Some(36);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: ProtocolItemStackSummary {
            item_id: Some(42),
            count: 3,
            component_patch: Default::default(),
        },
    });
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    assert!(input.control_down());
    assert!(!input.sprint_key_down);
    assert!(rx.try_recv().is_err());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 0,
            slot_num: 36,
            button_num: 1,
            input: ContainerInput::Throw,
            changed_slots: [(36, HashedStack::Empty)].into(),
            carried_item: HashedStack::Empty,
        })
    );
}

#[test]
fn local_inventory_hovered_empty_slot_drop_key_is_consumed_without_packet() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.container_mut().inventory_hovered_slot = Some(36);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn local_inventory_drop_key_without_hovered_slot_does_not_queue_player_drop() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.sprint_key_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());

    input.apply_screen_requests(&mut world, &mut counters, &commands);
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn local_inventory_hovered_number_keys_queue_swap_container_clicks() {
    const HOTBAR_KEYS: [(KeyCode, i8); 9] = [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
        (KeyCode::Digit7, 6),
        (KeyCode::Digit8, 7),
        (KeyCode::Digit9, 8),
    ];

    for (code, button_num) in HOTBAR_KEYS {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::container_for_test();
        input.container_mut().inventory_hovered_slot = Some(9);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: i32::from(button_num),
            item: test_item_stack(42 + i32::from(button_num), 1),
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 9,
            item: test_item_stack(99, 2),
        });
        assert!(world.open_local_inventory());

        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(counters.held_slot_commands_queued, 0);
        assert_eq!(counters.player_action_commands_queued, 0);
        assert_eq!(world.local_player().selected_hotbar_slot, 0);
        match rx.try_recv().unwrap() {
            NetCommand::ContainerClick(click) => {
                assert_eq!(click.container_id, 0);
                assert_eq!(click.state_id, 0);
                assert_eq!(click.slot_num, 9);
                assert_eq!(click.button_num, button_num);
                assert_eq!(click.input, ContainerInput::Swap);
                assert_eq!(
                    click.changed_slots,
                    [
                        (9, test_hashed_item_stack(42 + i32::from(button_num), 1)),
                        (36 + i16::from(button_num), test_hashed_item_stack(99, 2)),
                    ]
                    .into()
                );
                assert_eq!(click.carried_item, HashedStack::Empty);
            }
            command => panic!("expected container click command, got {command:?}"),
        }
        assert_eq!(
            test_player_slot_item(&world, 9),
            test_item_stack(42 + i32::from(button_num), 1)
        );
        assert_eq!(
            test_player_slot_item(&world, i32::from(button_num)),
            test_item_stack(99, 2)
        );
        assert!(rx.try_recv().is_err());
    }
}

#[test]
fn local_inventory_hovered_offhand_swap_key_queues_swap_container_click() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.container_mut().inventory_hovered_slot = Some(9);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: test_item_stack(42, 1),
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 40,
        item: test_item_stack(77, 1),
    });
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyF),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(counters.player_action_commands_queued, 0);
    match rx.try_recv().unwrap() {
        NetCommand::ContainerClick(click) => {
            assert_eq!(click.container_id, 0);
            assert_eq!(click.state_id, 0);
            assert_eq!(click.slot_num, 9);
            assert_eq!(click.button_num, 40);
            assert_eq!(click.input, ContainerInput::Swap);
            assert_eq!(
                click.changed_slots,
                [
                    (9, test_hashed_item_stack(77, 1)),
                    (45, test_hashed_item_stack(42, 1)),
                ]
                .into()
            );
            assert_eq!(click.carried_item, HashedStack::Empty);
        }
        command => panic!("expected container click command, got {command:?}"),
    }
    assert_eq!(test_player_slot_item(&world, 9), test_item_stack(77, 1));
    assert_eq!(test_player_slot_item(&world, 40), test_item_stack(42, 1));
    assert!(rx.try_recv().is_err());
}

#[test]
fn local_inventory_swap_keys_without_hover_or_with_carried_item_are_no_ops() {
    for code in [KeyCode::Digit5, KeyCode::KeyF] {
        assert_local_inventory_swap_key_noop(code, None, ProtocolItemStackSummary::empty());
        assert_local_inventory_swap_key_noop(
            code,
            Some(36),
            ProtocolItemStackSummary {
                item_id: Some(42),
                count: 1,
                component_patch: Default::default(),
            },
        );
    }
}

fn assert_local_inventory_swap_key_noop(
    code: KeyCode,
    hovered_slot: Option<i16>,
    cursor_item: ProtocolItemStackSummary,
) {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.container_mut().inventory_hovered_slot = hovered_slot;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_cursor_item(ProtocolSetCursorItem { item: cursor_item });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: test_item_stack(42, 1),
    });
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(code),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(world.local_player().selected_hotbar_slot, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn server_opened_container_hovered_drop_key_queues_throw_click() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.container_mut().inventory_hovered_slot = Some(0);
    let mut counters = NetCounters::default();
    let mut world = generic_9x1_container_world(7, 12, Some((0, test_item_stack(42, 3))));

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::Throw,
            changed_slots: [(0, test_hashed_item_stack(42, 2))].into(),
            carried_item: HashedStack::Empty,
        })
    );
    assert_eq!(
        test_open_container_slot_item(&world, 0),
        test_item_stack(42, 2)
    );
}

#[test]
fn server_opened_container_hovered_control_drop_key_queues_throw_stack_click() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.control_left_down = true;
    input.container_mut().inventory_hovered_slot = Some(0);
    let mut counters = NetCounters::default();
    let mut world = generic_9x1_container_world(7, 12, Some((0, test_item_stack(42, 3))));

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 1,
            input: ContainerInput::Throw,
            changed_slots: [(0, HashedStack::Empty)].into(),
            carried_item: HashedStack::Empty,
        })
    );
    assert_eq!(
        test_open_container_slot_item(&world, 0),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn server_opened_container_empty_slot_drop_key_is_consumed_without_packet() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.container_mut().inventory_hovered_slot = Some(0);
    let mut counters = NetCounters::default();
    let mut world = generic_9x1_container_world(7, 12, None);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn server_opened_container_hovered_number_and_offhand_keys_queue_swap_clicks() {
    for (code, button_num) in [(KeyCode::Digit5, 4), (KeyCode::KeyF, 40)] {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::container_for_test();
        input.container_mut().inventory_hovered_slot = Some(0);
        let mut counters = NetCounters::default();
        let mut world = generic_9x1_container_world(7, 12, Some((0, test_item_stack(42, 3))));
        assert_eq!(world.local_player().selected_hotbar_slot, 0);

        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(counters.held_slot_commands_queued, 0);
        assert_eq!(counters.player_action_commands_queued, 0);
        assert_eq!(world.local_player().selected_hotbar_slot, 0);
        let changed_slots = if button_num == 40 {
            [(0, HashedStack::Empty)].into()
        } else {
            [
                (0, HashedStack::Empty),
                (36 + i16::from(button_num), test_hashed_item_stack(42, 3)),
            ]
            .into()
        };
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 0,
                button_num,
                input: ContainerInput::Swap,
                changed_slots,
                carried_item: HashedStack::Empty,
            })
        );
        assert_eq!(
            test_open_container_slot_item(&world, 0),
            ProtocolItemStackSummary::empty()
        );
        if button_num == 40 {
            assert_eq!(test_player_slot_item(&world, 40), test_item_stack(42, 3));
        } else {
            assert_eq!(
                test_open_container_slot_item(&world, 36 + i16::from(button_num)),
                test_item_stack(42, 3)
            );
        }
    }
}

#[test]
fn server_opened_bundle_slot_swap_key_clears_selection_before_click() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.container_mut().inventory_hovered_slot = Some(0);
    let mut counters = NetCounters::default();
    let mut world = generic_9x1_container_world(7, 12, Some((0, test_bundle_stack(42, 1, 3))));
    assert!(world.apply_local_select_bundle_item(0, 1));

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Digit5),
        ElementState::Pressed,
    );

    assert_eq!(counters.select_bundle_item_commands_queued, 1);
    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SelectBundleItem(SelectBundleItem {
            slot_id: 0,
            selected_item_index: -1,
        })
    );
    let NetCommand::ContainerClick(click) = rx.try_recv().unwrap() else {
        panic!("expected container click command");
    };
    assert_eq!(click.container_id, 7);
    assert_eq!(click.state_id, 12);
    assert_eq!(click.slot_num, 0);
    assert_eq!(click.button_num, 4);
    assert_eq!(click.input, ContainerInput::Swap);
    assert_eq!(
        click.changed_slots.keys().copied().collect::<Vec<_>>(),
        vec![0, 40]
    );
    assert_eq!(click.changed_slots[&0], HashedStack::Empty);
    assert!(matches!(click.changed_slots[&40], HashedStack::Item(_)));
    assert_eq!(click.carried_item, HashedStack::Empty);
    assert_eq!(open_container_slot_bundle_selection(&world, 0), Some(-1));
    assert!(rx.try_recv().is_err());
}

#[test]
fn server_opened_container_swap_key_with_carried_item_is_consumed_without_packet() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.container_mut().inventory_hovered_slot = Some(0);
    let mut counters = NetCounters::default();
    let mut world = generic_9x1_container_world(7, 12, Some((0, test_item_stack(42, 3))));
    world.apply_set_cursor_item(ProtocolSetCursorItem {
        item: test_item_stack(99, 1),
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Digit5),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_text_input_queues_rename_item_command() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "Sharp Pick",
    );

    assert_eq!(input.anvil_rename_text(), "Sharp Pick");
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Sharp Pick".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_text_input_starts_from_default_hover_name_when_item_runtime_is_available() {
    let root = unique_input_temp_dir("anvil-rename-default");
    write_input_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(0, 1)));

    handle_text_input_with_item_runtime(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        Some(&item_runtime),
        "!",
        &FixedCommandSuggestionTextMeasurer,
    );

    assert_eq!(input.anvil_rename_text(), "Test Combo!");
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Test Combo!".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn anvil_text_input_starts_from_decoded_custom_name_without_item_runtime() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut stack = test_item_stack(42, 1);
    stack.component_patch.custom_name = Some("Custom Pick".to_string());
    stack.component_patch.item_name = Some("Ignored Item Name".to_string());
    let mut world = anvil_container_world(7, 12, Some(stack));

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "!");

    assert_eq!(input.anvil_rename_text(), "Custom Pick!");
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Custom Pick!".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_text_input_starts_from_written_book_title_without_item_runtime() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut stack = test_item_stack(42, 1);
    stack.component_patch.written_book = Some(WrittenBookContentSummary {
        title: "Book Title".into(),
        title_filter: None,
        author: "Author".into(),
        generation: 0,
        pages: Vec::new(),
        page_filters: Vec::new(),
        resolved: true,
    });
    stack.component_patch.item_name = Some("Ignored Item Name".to_string());
    let mut world = anvil_container_world(7, 12, Some(stack));

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "!");

    assert_eq!(input.anvil_rename_text(), "Book Title!");
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Book Title!".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_text_input_starts_from_decoded_item_name_without_item_runtime() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut stack = test_item_stack(42, 1);
    stack.component_patch.written_book = Some(WrittenBookContentSummary {
        title: "   ".into(),
        title_filter: None,
        author: "Author".into(),
        generation: 0,
        pages: Vec::new(),
        page_filters: Vec::new(),
        resolved: true,
    });
    stack.component_patch.item_name = Some("Component Item".to_string());
    let mut world = anvil_container_world(7, 12, Some(stack));

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "!");

    assert_eq!(input.anvil_rename_text(), "Component Item!");
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Component Item!".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_default_hover_name_is_sent_as_empty_rename() {
    let root = unique_input_temp_dir("anvil-rename-default-empty");
    write_input_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(0, 1)));

    handle_text_input_with_item_runtime(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        Some(&item_runtime),
        "!",
        &FixedCommandSuggestionTextMeasurer,
    );
    crate::input::key_events::dispatch_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        Some(&item_runtime),
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
        PhysicalSize::new(1280, 720),
        &FixedCommandSuggestionTextMeasurer,
        None,
        None,
        &mut true,
        &mut crate::simulation::effects::PlatformEffects::default(),
    );

    assert_eq!(input.anvil_rename_text(), "Test Combo");
    assert_eq!(counters.rename_item_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Test Combo!".to_string(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: String::new(),
        })
    );
    assert!(rx.try_recv().is_err());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn anvil_custom_name_matching_hover_name_is_sent_verbatim() {
    let root = unique_input_temp_dir("anvil-rename-custom-name");
    write_input_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut stack = test_item_stack(0, 1);
    stack.component_patch.custom_name = Some("Custom Combo".to_string());
    let mut world = anvil_container_world(7, 12, Some(stack));

    handle_text_input_with_item_runtime(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        Some(&item_runtime),
        "!",
        &FixedCommandSuggestionTextMeasurer,
    );
    crate::input::key_events::dispatch_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        Some(&item_runtime),
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
        PhysicalSize::new(1280, 720),
        &FixedCommandSuggestionTextMeasurer,
        None,
        None,
        &mut true,
        &mut crate::simulation::effects::PlatformEffects::default(),
    );

    assert_eq!(input.anvil_rename_text(), "Custom Combo");
    assert_eq!(counters.rename_item_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Custom Combo!".to_string(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Custom Combo".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn anvil_backspace_queues_updated_rename_item_command() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "Axe");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    );

    assert_eq!(input.anvil_rename_text(), "Ax");
    assert_eq!(counters.rename_item_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Axe".to_string(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Ax".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_cursor_keys_edit_rename_text_inside_line() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "abcd");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "X");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Home),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, ">");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::End),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "<");

    assert_eq!(input.anvil_rename_text(), ">abd<");
    assert_eq!(counters.rename_item_commands_queued, 6);
    for name in ["abcd", "abXcd", "abXd", "abd", ">abd", ">abd<"] {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::RenameItem(RenameItem {
                name: name.to_string(),
            })
        );
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_control_word_keys_edit_rename_text() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "alpha beta gamma",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowRight),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );

    assert_eq!(input.anvil_rename_text(), "alpha ");
    assert_eq!(counters.rename_item_commands_queued, 3);
    for name in ["alpha beta gamma", "alpha gamma", "alpha "] {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::RenameItem(RenameItem {
                name: name.to_string(),
            })
        );
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_control_a_selection_replaces_and_deletes_rename_text() {
    let (tx, mut rx) = mpsc::channel(6);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "replace me",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(TEXT_EDIT_SHORTCUT_KEY),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(TEXT_EDIT_SHORTCUT_KEY),
        ElementState::Released,
    );
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "Diamond Pick",
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(TEXT_EDIT_SHORTCUT_KEY),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(TEXT_EDIT_SHORTCUT_KEY),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    );

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "delete me",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(TEXT_EDIT_SHORTCUT_KEY),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(TEXT_EDIT_SHORTCUT_KEY),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );

    assert_eq!(input.anvil_rename_text(), "");
    assert_eq!(counters.rename_item_commands_queued, 5);
    for name in ["replace me", "Diamond Pick", "", "delete me", ""] {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::RenameItem(RenameItem {
                name: name.to_string(),
            })
        );
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_delete_at_end_is_consumed_without_rename_command() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "Axe");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );

    assert_eq!(input.anvil_rename_text(), "Axe");
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Axe".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_rename_field_consumes_inventory_key_when_editable() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyE),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "e");

    assert!(world.inventory().open_container.is_some());
    assert_eq!(counters.container_close_commands_queued, 0);
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "e".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_text_input_filters_vanilla_invalid_chars_and_max_length() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));
    let text = format!("{}{}\u{a7}\n", "a".repeat(49), "🙂");

    handle_text_input(&mut input, &mut counters, &mut world, &commands, &text);

    let expected = "a".repeat(49);
    assert_eq!(input.anvil_rename_text(), expected);
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem { name: expected })
    );
    assert!(rx.try_recv().is_err());
}
