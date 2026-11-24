use super::*;

#[test]
fn drop_key_queues_drop_item_action() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::DropItem,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn control_drop_key_queues_drop_all_items_action() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.control_left_down = true;
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::DropAllItems,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sprint_drop_key_still_queues_drop_item_action() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.sprint_key_down = true;
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::DropItem,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
}

#[test]
fn drop_key_predicts_one_selected_item_and_swings_when_non_empty() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    equip_player_slot(&mut world, 0, 42, 3);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(test_player_slot_item(&world, 0), test_item_stack(42, 2));
    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::DropItem,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
}

#[test]
fn control_drop_key_predicts_selected_stack_and_swings_when_non_empty() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.control_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    equip_player_slot(&mut world, 0, 42, 3);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(
        test_player_slot_item(&world, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::DropAllItems,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
}

#[test]
fn spectator_drop_key_does_not_drop_or_swing() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.control_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    equip_player_slot(&mut world, 0, 42, 3);
    set_local_spectator(&mut world);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(test_player_slot_item(&world, 0), test_item_stack(42, 3));
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn pending_sign_editor_escape_queues_empty_sign_update() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos {
        x: -5,
        y: 70,
        z: 12,
    };
    let mut world = world_with_sign_text(sign_pos, ["", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos {
            x: -5,
            y: 70,
            z: 12,
        },
        is_front_text: false,
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);

    assert!(input.sign_editor_is_open());
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!input.sign_editor_is_open());
    assert_eq!(counters.sign_update_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate {
            pos: ProtocolBlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            is_front_text: false,
            lines: Default::default(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn dialog_stack_submits_suspends_and_restores_the_native_sign_editor() {
    let mut input = ClientInputState::new(true);
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos {
        x: -5,
        y: 70,
        z: 12,
    };
    let mut world = WorldStore::new();
    insert_sign_text(&mut world, sign_pos, ["", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos {
            x: sign_pos.x,
            y: sign_pos.y,
            z: sign_pos.z,
        },
        is_front_text: true,
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    assert!(handle_sign_editor_text(
        &mut input,
        "draft",
        &FixedCommandSuggestionTextMeasurer,
    ));
    let editor = input.screen.sign_editor_mut().unwrap();
    editor.cursor = 2;
    editor.selection = 5;
    let expected = editor.clone();

    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);

    assert!(!input.sign_editor_is_open());
    assert_eq!(input.sign_editor_hud_state(), None);
    assert_eq!(counters.sign_update_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate {
            pos: expected.pos,
            is_front_text: true,
            lines: expected.lines.clone(),
        })
    );

    world.apply_clear_dialog();
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert_eq!(input.screen.sign_editor(), Some(&expected));
    assert_eq!(
        input.sign_editor_hud_state(),
        Some(sign_editor_hud_state_from_editor(&expected, None))
    );

    submit_sign_editor(&mut input, &mut counters, &commands);
    assert_eq!(counters.sign_update_commands_queued, 2);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate { lines, .. }) if lines == expected.lines
    ));
    assert!(rx.try_recv().is_err());
}

#[test]
fn new_sign_editor_replaces_the_one_suspended_below_dialog() {
    let mut input = ClientInputState::new(true);
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut counters = NetCounters::default();
    let old_pos = BlockPos { x: 1, y: 2, z: 3 };
    let new_pos = BlockPos { x: 20, y: 5, z: 6 };
    let mut world = WorldStore::new();
    insert_sign_text(&mut world, old_pos, ["Old", "", "", ""], ["", "", "", ""]);
    insert_sign_text(&mut world, new_pos, ["New", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos {
            x: old_pos.x,
            y: old_pos.y,
            z: old_pos.z,
        },
        is_front_text: true,
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert_eq!(counters.sign_update_commands_queued, 1);

    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos {
            x: new_pos.x,
            y: new_pos.y,
            z: new_pos.z,
        },
        is_front_text: true,
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    input.apply_screen_requests(&mut world, &mut counters, &commands);

    assert!(!input.dialog_screen_is_open());
    assert!(input.sign_editor_is_open());
    assert_eq!(
        input.sign_editor_hud_state().map(|state| state.pos),
        Some(new_pos)
    );
    assert_eq!(input.screen.sign_editor().unwrap().pos.x, new_pos.x);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::SignUpdate(_)));
    assert!(rx.try_recv().is_err());
}

#[test]
fn pending_sign_editor_escape_preserves_existing_sign_text() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: 3, y: 64, z: -9 };
    let mut world = world_with_sign_text(
        sign_pos,
        ["Front 1", "Front 2", "Front 3", "Front 4"],
        ["Back 1", "Back 2", "Back 3", "Back 4"],
    );
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos {
            x: sign_pos.x,
            y: sign_pos.y,
            z: sign_pos.z,
        },
        is_front_text: false,
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!input.sign_editor_is_open());
    assert_eq!(counters.sign_update_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate {
            pos: ProtocolBlockPos {
                x: sign_pos.x,
                y: sign_pos.y,
                z: sign_pos.z,
            },
            is_front_text: false,
            lines: [
                "Back 1".to_string(),
                "Back 2".to_string(),
                "Back 3".to_string(),
                "Back 4".to_string(),
            ],
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn invalid_sign_editor_packet_does_not_reopen_previous_editor() {
    let sign_pos = BlockPos { x: 3, y: 64, z: -9 };
    let mut world = world_with_sign_text(sign_pos, ["Front", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 3, y: 64, z: -9 },
        is_front_text: true,
    });
    let mut input = ClientInputState::new(true);
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    let mut counters = NetCounters::default();
    let editor = input.screen.sign_editor().cloned();

    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos {
            x: 99,
            y: 64,
            z: 99,
        },
        is_front_text: false,
    });
    input.apply_screen_requests(&mut world, &mut counters, &None);

    assert_eq!(input.screen.sign_editor().cloned(), editor);
    assert_eq!(world.counters().open_sign_editor_packets, 2);
    assert_eq!(world.counters().open_sign_editor_updates_applied, 1);
    assert_eq!(world.counters().open_sign_editor_updates_ignored, 1);
}

#[test]
fn sign_editor_text_input_changes_lines_and_queues_sign_update() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: 3, y: 64, z: -9 };
    let mut world = world_with_sign_text(sign_pos, ["", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 3, y: 64, z: -9 },
        is_front_text: true,
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "Front");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "Back");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowUp),
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
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!input.sign_editor_is_open());
    assert_eq!(counters.sign_update_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate {
            pos: ProtocolBlockPos { x: 3, y: 64, z: -9 },
            is_front_text: true,
            lines: [
                "Fron".to_string(),
                "Back".to_string(),
                String::new(),
                String::new(),
            ],
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sign_editor_text_input_filters_invalid_chars_and_measured_width() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: 0, y: 1, z: 2 };
    let mut world = world_with_sign_text(sign_pos, ["", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 2 },
        is_front_text: true,
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        &"a".repeat(15),
    );
    // The 16th six-pixel glyph exceeds a standing sign's 90-pixel width and
    // the whole candidate is rejected. Invalid chat characters insert nothing.
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "b");
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "\u{a7}\n");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    let expected = "a".repeat(15);
    assert_eq!(counters.sign_update_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate {
            pos: ProtocolBlockPos { x: 0, y: 1, z: 2 },
            is_front_text: true,
            lines: [expected, String::new(), String::new(), String::new(),],
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sign_editor_hanging_block_uses_60_pixel_candidate_limit() {
    let sign_pos = BlockPos { x: 0, y: 1, z: 2 };
    let mut world = WorldStore::new();
    insert_empty_chunk_for_block(&mut world, sign_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 2 },
        block_state_id: vanilla_block_state_id(
            "minecraft:bamboo_hanging_sign",
            [
                ("attached", "true"),
                ("rotation", "0"),
                ("waterlogged", "false"),
            ],
        ),
    }));
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 2 },
        is_front_text: true,
    });
    let mut input = ClientInputState::new(true);
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    assert_eq!(
        input.screen.sign_editor().unwrap().max_line_width,
        HANGING_SIGN_MAX_LINE_WIDTH
    );

    for text in ["a".repeat(10), "b".to_string()] {
        assert!(handle_sign_editor_text(
            &mut input,
            &text,
            &FixedCommandSuggestionTextMeasurer,
        ));
    }
    assert_eq!(input.screen.sign_editor().unwrap().lines[0], "a".repeat(10));
}

#[test]
fn sign_editor_preload_uses_filtered_open_snapshot() {
    let sign_pos = BlockPos { x: 1, y: 64, z: 1 };
    let mut world = WorldStore::new();
    insert_sign_text_state(
        &mut world,
        sign_pos,
        sign_text_side_with_filtered(["raw", "", "", ""], ["filtered", "", "", ""]),
        sign_text_side(["", "", "", ""]),
    );
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 1, y: 64, z: 1 },
        is_front_text: true,
    });
    let mut input = ClientInputState::new(true);
    input.set_client_text_filtering_enabled(true);
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);

    assert_eq!(input.sign_editor_hud_state().unwrap().lines[0], "filtered");
    assert_eq!(input.screen.sign_editor().unwrap().lines[0], "filtered");
}

#[test]
fn sign_editor_shift_selection_replaces_the_selected_range() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: 2, y: 64, z: 2 };
    let mut world = world_with_sign_text(sign_pos, ["", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 2, y: 64, z: 2 },
        is_front_text: true,
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "abcd");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    );
    for _ in 0..2 {
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::ArrowLeft),
            ElementState::Pressed,
        );
    }
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Released,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "X");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate { lines, .. }) if lines[0] == "abX"
    ));
}

#[test]
fn sign_editor_tick_range_guard_submits_once() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: 4, y: 64, z: 4 };
    let mut world = world_with_local_player_id(42);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 4.5,
            y: 63.0,
            z: 4.5,
        },
        ..Default::default()
    });
    insert_sign_text(
        &mut world,
        sign_pos,
        ["draft", "", "", ""],
        ["", "", "", ""],
    );
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 4, y: 64, z: 4 },
        is_front_text: true,
    });
    let mut input = ClientInputState::new(true);
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    assert!(!input.sync_sign_editor(&mut world, &mut counters, &commands));
    assert!(input.sign_editor_is_open());

    let mut pose = world.local_player_pose().unwrap();
    pose.position.x = 100.0;
    world.set_local_player_pose(pose);
    assert!(input.sync_sign_editor(&mut world, &mut counters, &commands));
    assert!(!input.sync_sign_editor(&mut world, &mut counters, &commands));
    assert!(!input.sign_editor_is_open());
    assert_eq!(counters.sign_update_commands_queued, 1);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::SignUpdate(_)));
    assert!(rx.try_recv().is_err());
}

#[test]
fn sign_editor_tick_removed_sign_submits_once() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: 4, y: 64, z: 4 };
    let mut world = world_with_local_player_id(42);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 4.5,
            y: 63.0,
            z: 4.5,
        },
        ..Default::default()
    });
    insert_sign_text(
        &mut world,
        sign_pos,
        ["draft", "", "", ""],
        ["", "", "", ""],
    );
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 4, y: 64, z: 4 },
        is_front_text: true,
    });
    let mut input = ClientInputState::new(true);
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    assert!(!input.sync_sign_editor(&mut world, &mut counters, &commands));

    assert!(world.forget_chunk(ChunkPos { x: 0, z: 0 }));
    assert!(input.sync_sign_editor(&mut world, &mut counters, &commands));
    assert!(!input.sync_sign_editor(&mut world, &mut counters, &commands));
    assert!(!input.sign_editor_is_open());
    assert_eq!(counters.sign_update_commands_queued, 1);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::SignUpdate(_)));
    assert!(rx.try_recv().is_err());
}

#[test]
fn sign_editor_done_mouse_and_ime_preedit_share_one_submit_path() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: 5, y: 64, z: 5 };
    let mut world = world_with_sign_text(sign_pos, ["", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 5, y: 64, z: 5 },
        is_front_text: true,
    });
    let mut input = ClientInputState::new(true);
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    assert!(input.handle_sign_editor_ime_preedit("候補", Some((0, 3))));
    let state = input.sign_editor_hud_state().unwrap();
    assert_eq!(state.lines[0], "");
    assert_eq!(state.preedit.as_ref().unwrap().text, "候補");
    assert_eq!(state.preedit.as_ref().unwrap().focused_end, 1);

    input.clear_sign_editor_ime_preedit();
    assert!(handle_sign_editor_text(
        &mut input,
        "候",
        &FixedCommandSuggestionTextMeasurer,
    ));
    let surface = PhysicalSize::new(320, 240);
    let done = bbb_client_render::hud_sign_editor_done_button_rect(surface);
    assert!(input.handle_sign_editor_mouse_input(
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(done.x + 1),
            f64::from(done.y + 1),
        )),
        surface,
    ));
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate { lines, .. }) if lines[0] == "候"
    ));
    assert!(rx.try_recv().is_err());
}

#[test]
fn swap_offhand_key_queues_swap_action() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyF),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::SwapItemWithOffhand,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
}

#[test]
fn spectator_swap_offhand_key_does_not_queue_action() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_local_spectator(&mut world);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyF),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn death_respawn_key_queues_perform_respawn() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_player_health(PlayerHealth {
        health: 0.0,
        food: 0,
        saturation: 0.0,
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert_eq!(counters.perform_respawn_commands_queued, 1);
    assert_eq!(rx.try_recv().unwrap(), NetCommand::PerformRespawn);
}

#[test]
fn death_state_consumes_gameplay_keys_without_queueing_actions() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_player_health(PlayerHealth {
        health: 0.0,
        food: 0,
        saturation: 0.0,
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.perform_respawn_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn sleeping_escape_key_queues_stop_sleeping_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_sleeping_local_player(77);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StopSleeping,
            data: 0,
        })
    );
}

#[test]
fn sleeping_allows_chat_key_and_escape_from_bed_chat_wakes_player() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_sleeping_local_player(77);

    // Pressing chat key while sleeping opens chat instead of being swallowed.
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    assert!(input.chat_entry_is_active());

    // Pressing Escape in bed chat closes chat and sends StopSleeping.
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    assert!(!input.chat_entry_is_active());
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StopSleeping,
            data: 0,
        })
    );
}

#[test]
fn sleeping_state_consumes_gameplay_keys_without_queueing_actions() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_sleeping_local_player(77);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn inventory_key_opens_local_inventory_without_player_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    world.set_tutorial_step(bbb_client_world::TutorialStep::OpenInventory);
    world.advance_tutorial_ticks(1);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyE),
        ElementState::Pressed,
    );

    assert!(world.local_inventory_is_open());
    assert_eq!(world.open_container_id(), Some(0));
    assert_eq!(counters.player_command_commands_queued, 0);
    assert!(rx.try_recv().is_err());
    assert_eq!(
        world.tutorial_step(),
        bbb_client_world::TutorialStep::CraftPlanks
    );
}

#[test]
fn inventory_key_on_server_controlled_mount_queues_open_inventory_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_vehicle(77, 10, VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID);
    world.set_tutorial_step(bbb_client_world::TutorialStep::OpenInventory);
    world.advance_tutorial_ticks(1);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyE),
        ElementState::Pressed,
    );

    assert!(!world.local_inventory_is_open());
    assert_eq!(world.open_container_id(), None);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::OpenInventory,
            data: 0,
        })
    );
    assert_eq!(
        world.tutorial_step(),
        bbb_client_world::TutorialStep::OpenInventory
    );
}

#[test]
fn escape_key_closes_local_inventory_and_queues_container_zero() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());

    input.apply_screen_requests(&mut world, &mut counters, &commands);
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!world.local_inventory_is_open());
    assert_eq!(world.open_container_id(), None);
    assert_eq!(counters.container_close_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClose(ContainerCloseRequest { container_id: 0 })
    );
}

#[test]
fn inventory_key_closes_local_inventory_and_queues_container_zero() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());

    input.apply_screen_requests(&mut world, &mut counters, &commands);
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyE),
        ElementState::Pressed,
    );

    assert!(!world.local_inventory_is_open());
    assert_eq!(world.open_container_id(), None);
    assert_eq!(counters.container_close_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClose(ContainerCloseRequest { container_id: 0 })
    );
}

#[test]
fn escape_key_closes_open_container_and_queues_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });

    input.apply_screen_requests(&mut world, &mut counters, &commands);
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(world.inventory().open_container.is_none());
    assert_eq!(world.counters().container_close_updates_received, 0);
    assert_eq!(counters.container_close_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClose(ContainerCloseRequest { container_id: 7 })
    );
}

#[test]
fn inventory_key_closes_open_container_before_open_inventory_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 8,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });

    input.apply_screen_requests(&mut world, &mut counters, &commands);
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyE),
        ElementState::Pressed,
    );

    assert!(world.inventory().open_container.is_none());
    assert_eq!(counters.container_close_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClose(ContainerCloseRequest { container_id: 8 })
    );
}

#[test]
fn inventory_key_is_consumed_by_focused_recipe_book_search() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.container_mut().recipe_book_search_focused = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 8,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyE),
        ElementState::Pressed,
    );

    assert_eq!(world.open_container_id(), Some(8));
    assert_eq!(counters.container_close_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}
