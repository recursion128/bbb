use super::*;

#[test]
fn command_block_editor_waits_for_authoritative_data_and_done_sends_exact_state_once() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_command_block(pos, "say initial");
    let command_block_state = world.block_state_id_at(pos).unwrap();
    for block_state_id in [0, command_block_state] {
        assert!(world.apply_block_update(ProtocolBlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_state_id,
        }));
    }
    assert!(world.command_block_data_at(pos).is_none());
    assert!(world.try_open_command_block_editor(pos));
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(input
        .command_block_editor_hud_state()
        .is_some_and(|state| state.block.is_some()));
    assert!(
        !input
            .command_block_editor_hud_state()
            .unwrap()
            .block
            .unwrap()
            .controls_enabled
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );
    assert!(rx.try_recv().is_err());
    assert!(input
        .command_block_editor_hud_state()
        .is_some_and(|state| state.block.is_some()));

    let authoritative_command = bbb_protocol::JavaString::from(
        "say authoritative"
            .encode_utf16()
            .chain([0xd800])
            .collect::<Vec<_>>(),
    );
    assert!(world
        .apply_block_entity_data(BlockEntityData {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_entity_type_id: VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID,
            raw_nbt: bbb_protocol::encode_nbt_summary(
                &bbb_protocol::NbtSummaryValue::Compound(vec![
                    bbb_protocol::NbtSummaryEntry {
                        name: "Command".into(),
                        value: authoritative_command.to_nbt(),
                    },
                    bbb_protocol::NbtSummaryEntry {
                        name: "TrackOutput".into(),
                        value: bbb_protocol::NbtSummaryValue::Byte(1),
                    },
                    bbb_protocol::NbtSummaryEntry {
                        name: "LastOutput".into(),
                        value: bbb_protocol::NbtSummaryValue::string("previous"),
                    },
                    bbb_protocol::NbtSummaryEntry {
                        name: "auto".into(),
                        value: bbb_protocol::NbtSummaryValue::Byte(0),
                    },
                ]),
                1024,
            )
            .unwrap(),
        })
        .unwrap());
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    let state = input.command_block_editor_hud_state().unwrap();
    assert!(state.block.unwrap().controls_enabled);
    assert!(state.track_output);
    assert_eq!(
        input.screen.chat_entry().unwrap().edit.value(),
        authoritative_command.units()
    );
    let saved_command = bbb_protocol::JavaString::from(
        "say saved"
            .encode_utf16()
            .chain([0xd801])
            .collect::<Vec<_>>(),
    );
    assert!(input.replace_chat_entry_java_text_with_measurer(
        saved_command.clone(),
        &FixedCommandSuggestionTextMeasurer,
    ));

    let surface = PhysicalSize::new(320, 240);
    let layout = hud_command_block_editor_layout(surface);
    assert!(input.handle_command_block_editor_mouse_input(
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(layout.command_input.x + 4 + 18),
            f64::from(layout.command_input.y + 6),
        )),
        surface,
        &FixedCommandSuggestionTextMeasurer,
    ));
    assert_eq!(input.screen.chat_entry().unwrap().edit.cursor(), 3);
    for rect in [
        layout.mode,
        layout.conditional,
        layout.automatic,
        layout.track_output,
    ] {
        assert!(input.handle_command_block_editor_mouse_input(
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(
                f64::from(rect.x + 1),
                f64::from(rect.y + 1),
            )),
            surface,
            &FixedCommandSuggestionTextMeasurer,
        ));
    }
    let rotated = bbb_client_world::vanilla_block_state_id(
        "minecraft:command_block",
        &BTreeMap::from([
            ("conditional".into(), "false".into()),
            ("facing".into(), "south".into()),
        ]),
    )
    .unwrap();
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
        block_state_id: rotated,
    }));
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert_eq!(
        input
            .command_block_editor_hud_state()
            .unwrap()
            .block
            .unwrap()
            .mode,
        CommandBlockMode::Sequence
    );
    assert!(input.handle_command_block_editor_mouse_input(
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(layout.done.x + 1),
            f64::from(layout.done.y + 1),
        )),
        surface,
        &FixedCommandSuggestionTextMeasurer,
    ));

    assert_eq!(counters.set_command_block_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SetCommandBlock(SetCommandBlock {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            command: saved_command,
            mode: CommandBlockMode::Sequence,
            track_output: false,
            conditional: true,
            automatic: true,
        })
    );
    assert!(rx.try_recv().is_err());
    assert!(!input
        .command_block_editor_hud_state()
        .is_some_and(|state| state.block.is_some()));

    let data = world.command_block_data_at(pos).unwrap();
    assert!(!data.track_output);
    assert_eq!(data.last_output, None);
}

#[test]
fn command_block_editor_cancel_and_escape_close_without_sending() {
    let mut input = ClientInputState::new(true);
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut counters = NetCounters::default();
    let mut world = world_with_command_block(pos, "say initial");

    assert!(world.try_open_command_block_editor(pos));
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    assert!(!input
        .command_block_editor_hud_state()
        .is_some_and(|state| state.block.is_some()));

    assert!(world.try_open_command_block_editor(pos));
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    let surface = PhysicalSize::new(320, 240);
    let cancel = hud_command_block_editor_layout(surface).cancel;
    assert!(input.handle_command_block_editor_mouse_input(
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(cancel.x + 1),
            f64::from(cancel.y + 1),
        )),
        surface,
        &FixedCommandSuggestionTextMeasurer,
    ));
    assert!(!input
        .command_block_editor_hud_state()
        .is_some_and(|state| state.block.is_some()));

    assert_eq!(counters.set_command_block_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    assert!(world.try_open_command_block_editor(pos));
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(!input.command_entry_is_active());

    world.apply_clear_dialog();
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(input
        .command_block_editor_hud_state()
        .is_some_and(|state| state.block.is_some()));

    assert!(world.open_local_inventory());
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(!input
        .command_block_editor_hud_state()
        .is_some_and(|state| state.block.is_some()));
}

#[test]
fn command_block_editor_uses_commands_only_seven_row_suggestions_and_utf16_limit() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let commands = None;
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_command_block(pos, "c");
    let literals = (0..12)
        .map(|index| format!("cmd{index:02}"))
        .collect::<Vec<_>>();
    world.apply_commands(local_literal_command_tree(
        &literals.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert!(world.try_open_command_block_editor(pos));
    input.apply_screen_requests(&mut world, &mut counters, &commands);

    let hud = input
        .command_suggestion_hud_state(
            &world,
            &FixedCommandSuggestionTextMeasurer,
            PhysicalSize::new(320, 240),
        )
        .unwrap();
    assert!(hud.command_block_layout);
    assert_eq!(hud.input_text, "c");
    assert_eq!(hud.rect.y, 71);
    assert_eq!(hud.rect.height, 84);
    assert_eq!(hud.entries.len(), 7);
    assert!(!hud.has_previous);
    assert!(hud.has_next);

    assert!(input.replace_chat_entry_text("😀".repeat(16_250)));
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "a");
    assert_eq!(
        input
            .screen
            .chat_entry()
            .unwrap()
            .edit
            .text()
            .encode_utf16()
            .count(),
        MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS
    );
    assert!(!input
        .screen
        .chat_entry()
        .unwrap()
        .edit
        .text()
        .ends_with('a'));
    assert!(input.replace_chat_entry_text("😀".repeat(16_249)));
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "ab");
    assert_eq!(
        input
            .screen
            .chat_entry()
            .unwrap()
            .edit
            .text()
            .encode_utf16()
            .count(),
        MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS
    );
    assert!(input
        .screen
        .chat_entry()
        .unwrap()
        .edit
        .text()
        .ends_with("ab"));
}

#[test]
fn minecart_command_block_editor_preserves_unsent_command_and_done_sends_exact_state_once() {
    let entity_id = 300;
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_command_minecart(entity_id, "say initial", "first output");
    assert!(world.try_open_minecart_command_block_editor(entity_id));
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(input
        .command_block_editor_hud_state()
        .is_some_and(|state| state.block.is_none()));
    let hud = input.command_block_editor_hud_state().unwrap();
    assert!(hud.track_output);
    assert_eq!(hud.previous_output, "first output");

    assert!(input.replace_chat_entry_text("say unsent".to_string()));
    apply_command_minecart_metadata(&mut world, entity_id, "say server", "new output");
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert_eq!(input.screen.chat_entry().unwrap().edit.text(), "say unsent");
    assert_eq!(
        input
            .command_block_editor_hud_state()
            .unwrap()
            .previous_output,
        "new output"
    );

    let surface = PhysicalSize::new(320, 240);
    let layout = bbb_client_render::hud_minecart_command_block_editor_layout(surface);
    assert!(input.handle_command_block_editor_mouse_input(
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(layout.track_output.x + 1),
            f64::from(layout.track_output.y + 1),
        )),
        surface,
        &FixedCommandSuggestionTextMeasurer,
    ));
    assert!(input.handle_command_block_editor_mouse_input(
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(layout.done.x + 1),
            f64::from(layout.done.y + 1),
        )),
        surface,
        &FixedCommandSuggestionTextMeasurer,
    ));

    assert_eq!(counters.set_command_minecart_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SetCommandMinecart(SetCommandMinecart {
            entity_id,
            command: "say unsent".into(),
            track_output: false,
        })
    );
    assert!(rx.try_recv().is_err());
    assert!(!input
        .command_block_editor_hud_state()
        .is_some_and(|state| state.block.is_none()));

    let data = world.minecart_command_block_data(entity_id).unwrap();
    assert_eq!(data.command, "say server");
    assert!(!data.track_output);
    assert!(data.last_output.is_none());
}

#[test]
fn minecart_command_block_editor_cancel_escape_and_removal_close_without_sending() {
    let entity_id = 300;
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_command_minecart(entity_id, "say initial", "output");

    assert!(world.try_open_minecart_command_block_editor(entity_id));
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    let surface = PhysicalSize::new(320, 240);
    let layout = bbb_client_render::hud_minecart_command_block_editor_layout(surface);
    assert!(input.handle_command_block_editor_mouse_input(
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(layout.track_output.x + 1),
            f64::from(layout.track_output.y + 1),
        )),
        surface,
        &FixedCommandSuggestionTextMeasurer,
    ));
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    assert!(!input
        .command_block_editor_hud_state()
        .is_some_and(|state| state.block.is_none()));

    assert!(world.try_open_minecart_command_block_editor(entity_id));
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(!input.command_block_editor_hud_state().unwrap().track_output);
    let cancel = layout.cancel;
    assert!(input.handle_command_block_editor_mouse_input(
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(cancel.x + 1),
            f64::from(cancel.y + 1),
        )),
        surface,
        &FixedCommandSuggestionTextMeasurer,
    ));
    assert!(!input
        .command_block_editor_hud_state()
        .is_some_and(|state| state.block.is_none()));

    assert!(world.try_open_minecart_command_block_editor(entity_id));
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert_eq!(
        world.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![entity_id],
        }),
        1
    );
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(!input
        .command_block_editor_hud_state()
        .is_some_and(|state| state.block.is_none()));

    let mut replacement_world = world_with_command_minecart(entity_id, "say initial", "output");
    let mut replacement_input = ClientInputState::new(true);
    assert!(replacement_world.try_open_minecart_command_block_editor(entity_id));
    replacement_input.apply_screen_requests(&mut replacement_world, &mut counters, &commands);
    assert!(replacement_world.open_local_inventory());
    replacement_input.apply_screen_requests(&mut replacement_world, &mut counters, &commands);
    assert!(!replacement_input
        .command_block_editor_hud_state()
        .is_some_and(|state| state.block.is_none()));

    assert_eq!(counters.set_command_minecart_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn minecart_command_block_editor_enter_sends_once() {
    let entity_id = 300;
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_command_minecart(entity_id, "say initial", "output");
    assert!(world.try_open_minecart_command_block_editor(entity_id));
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(input.replace_chat_entry_text("say enter".to_string()));

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert_eq!(counters.set_command_minecart_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SetCommandMinecart(SetCommandMinecart {
            entity_id,
            command: "say enter".into(),
            track_output: true,
        })
    );
    assert!(rx.try_recv().is_err());
    assert!(!input
        .command_block_editor_hud_state()
        .is_some_and(|state| state.block.is_none()));

    assert_eq!(
        world
            .minecart_command_block_data(entity_id)
            .unwrap()
            .command,
        "say initial"
    );
}

#[test]
fn minecart_command_block_editor_uses_commands_only_suggestions_and_utf16_limit() {
    let entity_id = 300;
    let commands = None;
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_command_minecart(entity_id, "c", "");
    let literals = (0..12)
        .map(|index| format!("cmd{index:02}"))
        .collect::<Vec<_>>();
    world.apply_commands(local_literal_command_tree(
        &literals.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert!(world.try_open_minecart_command_block_editor(entity_id));
    input.apply_screen_requests(&mut world, &mut counters, &commands);

    let hud = input
        .command_suggestion_hud_state(
            &world,
            &FixedCommandSuggestionTextMeasurer,
            PhysicalSize::new(320, 240),
        )
        .unwrap();
    assert!(hud.command_block_layout);
    assert_eq!(hud.rect.y, 71);
    assert_eq!(hud.rect.height, 84);
    assert_eq!(hud.entries.len(), 7);
    assert!(hud.has_next);

    assert!(input.replace_chat_entry_text("😀".repeat(16_250)));
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "a");
    assert_eq!(
        input
            .screen
            .chat_entry()
            .unwrap()
            .edit
            .text()
            .encode_utf16()
            .count(),
        MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS
    );
    assert!(!input
        .screen
        .chat_entry()
        .unwrap()
        .edit
        .text()
        .ends_with('a'));
}

#[test]
fn command_block_editor_authoritative_updates_and_chunk_close_reach_parked_screen() {
    let mut input = ClientInputState::new(true);
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut counters = NetCounters::default();
    let mut world = world_with_command_block(pos, "say initial");
    world.apply_commands(ask_server_command_tree());
    assert!(world.try_open_command_block_editor(pos));
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(
        matches!(rx.try_recv().unwrap(), NetCommand::CommandSuggestionRequest(request) if request.command == "say initial")
    );
    assert!(input.replace_chat_entry_text("say unsent".into()));
    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(world
        .apply_block_entity_data(BlockEntityData {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_entity_type_id: VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID,
            raw_nbt: nbt_compound(vec![nbt_string("Command", "say authoritative")]),
        })
        .unwrap());
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(rx.try_recv().is_err());
    world.apply_clear_dialog();
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert_eq!(
        input.screen.chat_entry().unwrap().edit.text(),
        "say authoritative"
    );
    assert!(
        input
            .command_block_editor_hud_state()
            .unwrap()
            .block
            .unwrap()
            .controls_enabled
    );
    assert!(
        matches!(rx.try_recv().unwrap(), NetCommand::CommandSuggestionRequest(request) if request.command == "say authoritative")
    );
    assert!(rx.try_recv().is_err());

    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(world.forget_chunk(pos.chunk()));
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    world.apply_clear_dialog();
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(!input.command_entry_is_active());
    assert_eq!(counters.set_command_block_commands_queued, 0);
}
