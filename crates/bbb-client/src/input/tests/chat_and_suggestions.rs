use super::*;

#[test]
fn slash_text_opens_command_entry_and_releases_pressed_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/");

    assert_eq!(
        input
            .screen
            .chat_entry()
            .map(|entry| entry.edit.text())
            .as_deref(),
        Some("/")
    );
    assert!(!input.forward);
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/".to_string().into(),
        })
    );
    assert_eq!(
        world.command_suggestions().last_request.as_ref(),
        Some(&bbb_client_world::CommandSuggestionRequestState {
            id: 0,
            command: "/".into(),
        })
    );
}

#[test]
fn command_entry_submits_slash_command_without_leading_slash() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "/time set day",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(counters.chat_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/time set day".to_string().into(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChatCommand(OutboundChatCommand::unsigned("time set day"))
    );
}

#[test]
fn command_entry_with_signable_message_argument_submits_signed_command() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(signable_message_command_tree());
    let _ = world.apply_player_chat(player_chat_with_signature(
        0,
        MessageSignature {
            bytes: vec![12; 256],
        },
    ));

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "/say hello",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(counters.chat_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/say hello".to_string().into(),
        })
    );
    match rx.try_recv().unwrap() {
        NetCommand::ChatCommand(packet) => {
            assert_eq!(packet.command, "say hello");
            let signing = packet.signing.expect("signable command signing context");
            assert!(signing.timestamp_millis > 0);
            assert_ne!(signing.salt, 0);
            assert_eq!(signing.arguments[0].name, "message");
            assert_eq!(signing.arguments[0].value, "hello");
            assert_eq!(signing.last_seen.update.offset, 1);
            // The fixture sender has no PlayerInfo entry, so vanilla presents
            // a validation error and advances only the hidden-message offset.
            assert_eq!(signing.last_seen.update.acknowledged, 0);
            assert_eq!(
                signing.last_seen.update.checksum,
                LastSeenMessagesUpdate::default().checksum
            );
            assert!(signing.last_seen.signatures.is_empty());
        }
        command => panic!("expected chat command request, got {command:?}"),
    }
    assert!(rx.try_recv().is_err());
    assert_eq!(
        world.counters().player_chat_acknowledgement_pending_offset,
        0
    );
}

#[test]
fn chat_key_opens_chat_entry_and_submits_unsigned_message() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    assert_eq!(
        input
            .screen
            .chat_entry()
            .map(|entry| entry.edit.text())
            .as_deref(),
        Some("")
    );
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "hello   world",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert!(!input.forward);
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.chat_message_commands_queued, 1);
    assert_eq!(counters.chat_command_commands_queued, 0);
    assert_eq!(counters.command_suggestion_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    match rx.try_recv().unwrap() {
        NetCommand::ChatMessage(packet) => {
            assert_eq!(packet.message, "hello world");
            assert!(packet.timestamp_millis > 0);
            assert_ne!(packet.salt, 0);
            assert_eq!(packet.last_seen.update, LastSeenMessagesUpdate::default());
            assert!(packet.last_seen.signatures.is_empty());
        }
        command => panic!("expected chat message command, got {command:?}"),
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn chat_visibility_permissions_gate_messages_commands_and_message_arguments() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(local_literal_command_tree(&["time"]));

    let mut config = world.chat_presentation_config().clone();
    config.visibility = bbb_client_world::ChatVisibilityState::System;
    world.set_chat_presentation_config(config.clone());
    open_chat_entry(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        ChatMethod::Message,
        false,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "blocked");
    submit_chat_entry(&mut input, &mut counters, &mut world, &commands);
    assert!(input.chat_entry_is_active());
    assert_eq!(counters.chat_message_commands_queued, 0);

    input.close_chat_screen(ChatScreenExitReason::Intentional);
    open_chat_entry(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        ChatMethod::Command,
        false,
    );
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "time set day",
    );
    submit_chat_entry(&mut input, &mut counters, &mut world, &commands);
    assert_eq!(counters.chat_command_commands_queued, 1);
    assert!(matches!(rx.try_recv(), Ok(NetCommand::ChatCommand(_))));

    world.apply_commands(signable_message_command_tree());
    open_chat_entry(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        ChatMethod::Command,
        false,
    );
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "say hello",
    );
    submit_chat_entry(&mut input, &mut counters, &mut world, &commands);
    assert!(
        input.chat_entry_is_active(),
        "message argument remains blocked"
    );
    assert_eq!(counters.chat_command_commands_queued, 1);

    input.close_chat_screen(ChatScreenExitReason::Intentional);
    config.visibility = bbb_client_world::ChatVisibilityState::Hidden;
    world.set_chat_presentation_config(config);
    open_chat_entry(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        ChatMethod::Command,
        false,
    );
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "time set day",
    );
    submit_chat_entry(&mut input, &mut counters, &mut world, &commands);
    assert!(input.chat_entry_is_active());
    assert_eq!(counters.chat_command_commands_queued, 1);
}

#[test]
fn recent_chat_is_bounded_deduplicated_and_history_restores_the_edit_buffer() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    for index in 0..=RECENT_CHAT_LIMIT {
        add_recent_chat(&mut input, format!("message {index}"));
    }
    add_recent_chat(&mut input, "message 100".to_string());
    assert_eq!(input.recent_chat.len(), RECENT_CHAT_LIMIT);
    assert_eq!(
        input
            .recent_chat
            .front()
            .map(|value| String::from_utf16_lossy(value))
            .as_deref(),
        Some("message 1")
    );
    assert_eq!(
        input
            .recent_chat
            .back()
            .map(|value| String::from_utf16_lossy(value))
            .as_deref(),
        Some("message 100")
    );

    open_chat_entry(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        ChatMethod::Message,
        false,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "working");
    move_in_chat_history(
        &mut input,
        -1,
        &text_field::TextFieldLayout {
            glyphs: FallbackTextMeasurer.chat_input_glyphs(),
            inner_width: 1276,
        },
    );
    assert_eq!(
        input
            .screen
            .chat_entry()
            .map(|entry| entry.edit.text())
            .as_deref(),
        Some("message 100")
    );
    move_in_chat_history(
        &mut input,
        1,
        &text_field::TextFieldLayout {
            glyphs: FallbackTextMeasurer.chat_input_glyphs(),
            inner_width: 1276,
        },
    );
    assert_eq!(
        input
            .screen
            .chat_entry()
            .map(|entry| entry.edit.text())
            .as_deref(),
        Some("working")
    );
}

#[test]
fn chat_draft_exit_reasons_and_focus_loss_follow_vanilla_lifecycle() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    open_chat_entry(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        ChatMethod::Message,
        false,
    );
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "draft text",
    );
    release_active_input(&mut input, &mut world, &mut counters, &commands);
    assert!(
        input.chat_entry_is_active(),
        "focus release preserves ChatScreen"
    );
    assert!(input.interrupt_chat_screen());
    assert_eq!(
        input
            .latest_chat_draft
            .as_ref()
            .map(|draft| String::from_utf16_lossy(&draft.text))
            .as_deref(),
        Some("draft text")
    );

    open_chat_entry(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        ChatMethod::Command,
        false,
    );
    assert_eq!(
        input
            .screen
            .chat_entry()
            .map(|entry| entry.edit.text())
            .as_deref(),
        Some("/")
    );
    input.close_chat_screen(ChatScreenExitReason::Intentional);
    assert!(input.latest_chat_draft.is_none());

    input.set_save_chat_drafts(true);
    open_chat_entry(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        ChatMethod::Message,
        false,
    );
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "saved intentionally",
    );
    input.close_chat_screen(ChatScreenExitReason::Intentional);
    assert_eq!(
        input
            .latest_chat_draft
            .as_ref()
            .map(|draft| String::from_utf16_lossy(&draft.text))
            .as_deref(),
        Some("saved intentionally")
    );
    open_chat_entry(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        ChatMethod::Message,
        false,
    );
    handle_chat_entry_key(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::Backspace,
        true,
        None,
        &FallbackTextMeasurer,
    );
    assert_eq!(
        input
            .screen
            .chat_entry()
            .map(|entry| entry.edit.text())
            .as_deref(),
        Some("")
    );
}

#[test]
fn chat_entry_copies_cuts_and_pastes_through_the_clipboard() {
    let (tx, _rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(9);
    let mut clipboard = MockDebugClipboard::with_text("pasted");

    open_chat_entry(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        ChatMethod::Message,
        false,
    );
    input.replace_chat_entry_text("hello".to_string());
    set_text_edit_shortcut_down(&mut input, true);

    let mut press = |input: &mut ClientInputState, code, clipboard: &mut MockDebugClipboard| {
        handle_chat_entry_key(
            input,
            &mut counters,
            &mut world,
            &commands,
            code,
            true,
            Some(clipboard),
            &FallbackTextMeasurer,
        );
    };

    press(&mut input, KeyCode::KeyA, &mut clipboard);
    press(&mut input, KeyCode::KeyC, &mut clipboard);
    assert_eq!(clipboard.text.as_deref(), Some("hello"));

    press(&mut input, KeyCode::KeyX, &mut clipboard);
    assert_eq!(
        input
            .screen
            .chat_entry()
            .map(|entry| entry.edit.text())
            .as_deref(),
        Some("")
    );

    clipboard.text = Some("pasted".to_string());
    press(&mut input, KeyCode::KeyV, &mut clipboard);
    assert_eq!(
        input
            .screen
            .chat_entry()
            .map(|entry| entry.edit.text())
            .as_deref(),
        Some("pasted")
    );

    input.shift_left_down = true;
    clipboard.text = Some("kept".to_string());
    press(&mut input, KeyCode::KeyC, &mut clipboard);
    assert_eq!(clipboard.text.as_deref(), Some("kept"));
}

#[test]
fn chat_restrictions_screen_preserves_previous_screen_and_interrupts_as_one_lifecycle() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    open_chat_entry(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        ChatMethod::Message,
        false,
    );
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "preserved",
    );
    let previous_revision = input.chat_screen_revision();
    assert!(input.open_chat_restrictions_screen());
    assert!(!input.chat_entry_is_active());
    assert!(input.chat_restrictions_screen_is_open());
    assert_ne!(input.chat_screen_revision(), previous_revision);
    assert!(input.close_chat_restrictions_screen());
    assert_eq!(input.chat_screen_revision(), previous_revision);
    assert_eq!(
        input
            .screen
            .chat_entry()
            .map(|entry| entry.edit.text())
            .as_deref(),
        Some("preserved")
    );

    assert!(input.open_chat_restrictions_screen());
    assert!(input.interrupt_chat_screen());
    assert!(!input.chat_restrictions_screen_is_open());
    assert_eq!(
        input
            .latest_chat_draft
            .as_ref()
            .map(|draft| String::from_utf16_lossy(&draft.text))
            .as_deref(),
        Some("preserved")
    );
}

#[test]
fn dialog_stack_suspends_and_restores_the_native_chat_screen() {
    let mut input = ClientInputState::new(true);
    let commands = None;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    open_chat_entry(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        ChatMethod::Message,
        false,
    );
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "draft command",
    );
    let entry = input.screen.chat_entry_mut().unwrap();
    let layout = text_field::TextFieldLayout {
        glyphs: FallbackTextMeasurer.chat_input_glyphs(),
        inner_width: 1276,
    };
    entry.edit.move_cursor(5, false, Some(&layout));
    entry.edit.set_selection(11, Some(&layout));
    let expected = entry.clone();

    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(!input.chat_entry_is_active());

    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 12 },
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(!input.chat_entry_is_active());

    world.apply_clear_dialog();
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert_eq!(input.screen.chat_entry(), Some(&expected));
    assert_eq!(world.counters().show_dialog_packets, 2);
    assert_eq!(world.counters().clear_dialog_packets, 1);
}

#[test]
fn native_chat_screen_is_not_restored_after_another_screen_replaces_dialog() {
    let mut input = ClientInputState::new(true);
    let commands = None;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    open_chat_entry(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        ChatMethod::Message,
        false,
    );
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "discarded draft",
    );
    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(!input.chat_entry_is_active());

    open_test_book_screen(&mut input, &mut world, vec!["Replacement"]);
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(!input.chat_entry_is_active());
    assert!(input.book_screen().is_some());

    assert!(input.screen.close().is_some());
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(!input.chat_entry_is_active());
}

#[test]
fn dialog_warning_restores_the_native_debug_options_screen_after_back() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let commands = None;
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(420, 161);
    input.open_debug_options_screen(&mut Default::default(), &None);
    assert!(input.handle_debug_options_screen_text_input("chunk"));
    assert!(input
        .handle_debug_options_screen_mouse_wheel(MouseScrollDelta::LineDelta(0.0, -2.0), surface));
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(PhysicalPosition::new(210.0, 80.0)),
        surface,
    ));
    let expected = input.screen.debug_options().cloned().unwrap();

    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(!input.debug_options_screen_is_open());

    assert!(input.open_dialog_warning());
    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 12 },
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    world.apply_clear_dialog();
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(input.dialog_warning_is_open());
    assert!(!input.debug_options_screen_is_open());

    assert!(input.close_dialog_warning());
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert_eq!(input.screen.debug_options(), Some(&expected));
    assert_eq!(
        input
            .debug_options_screen_hud_state(surface, false)
            .unwrap()
            .search
            .text(),
        "chunk"
    );
}

#[test]
fn dialog_clear_restores_the_native_pause_screen_state() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let commands = None;
    let mut world = WorldStore::new();
    input.open_debug_pause_screen_with_menu(&world, &mut Default::default(), &None);
    assert!(input.handle_debug_pause_screen_cursor_moved(Some(PhysicalPosition::new(68.0, 198.0,))));
    input.screen.pause_mut().unwrap().disconnect_requested = true;
    let expected = input.debug_pause_screen().cloned().unwrap();

    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(!input.debug_pause_screen_is_open());
    assert!(!input.pause_screen_disconnect_requested());

    world.apply_clear_dialog();
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert_eq!(input.debug_pause_screen(), Some(&expected));
    assert!(input.pause_screen_disconnect_requested());
}

#[test]
fn chat_entry_limits_message_text_to_vanilla_length() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        &"a".repeat(CHAT_ENTRY_MAX_LENGTH + 20),
    );

    let entry = input.screen.chat_entry().unwrap();
    assert_eq!(entry.edit.text().chars().count(), CHAT_ENTRY_MAX_LENGTH);
    assert_eq!(entry.edit.cursor(), CHAT_ENTRY_MAX_LENGTH);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    match rx.try_recv().unwrap() {
        NetCommand::ChatMessage(packet) => {
            assert_eq!(packet.message.units().len(), CHAT_ENTRY_MAX_LENGTH);
            assert_eq!(packet.message, "a".repeat(CHAT_ENTRY_MAX_LENGTH));
        }
        command => panic!("expected chat message command, got {command:?}"),
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn chat_entry_mid_cursor_insert_only_uses_remaining_vanilla_length() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        &"b".repeat(CHAT_ENTRY_MAX_LENGTH - 1),
    );
    let entry = input.screen.chat_entry_mut().unwrap();
    entry.edit.move_cursor(
        10,
        false,
        Some(&text_field::TextFieldLayout {
            glyphs: FallbackTextMeasurer.chat_input_glyphs(),
            inner_width: 1276,
        }),
    );

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "XYZ");

    let entry = input.screen.chat_entry().unwrap();
    let expected = format!(
        "{}{}{}",
        "b".repeat(10),
        "X",
        "b".repeat(CHAT_ENTRY_MAX_LENGTH - 11)
    );
    assert_eq!(entry.edit.text().chars().count(), CHAT_ENTRY_MAX_LENGTH);
    assert_eq!(entry.edit.text(), expected);
    assert_eq!(entry.edit.cursor(), 11);
    assert!(rx.try_recv().is_err());
}

#[test]
fn chat_entry_filters_vanilla_disallowed_formatting_character() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        &format!("he{}llo", '\u{a7}'),
    );

    assert_eq!(
        input
            .screen
            .chat_entry()
            .map(|entry| entry.edit.text())
            .as_deref(),
        Some("hello")
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    match rx.try_recv().unwrap() {
        NetCommand::ChatMessage(packet) => assert_eq!(packet.message, "hello"),
        command => panic!("expected chat message command, got {command:?}"),
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_filters_vanilla_disallowed_formatting_character() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        &format!("/s{}eed", '\u{a7}'),
    );

    assert_eq!(
        input
            .screen
            .chat_entry()
            .map(|entry| entry.edit.text())
            .as_deref(),
        Some("/seed")
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/seed".to_string().into(),
        })
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChatCommand(OutboundChatCommand::unsigned("seed"))
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn chat_key_submits_message_with_pending_last_seen_update() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let _ = world.apply_player_chat(player_chat_with_signature(
        0,
        MessageSignature {
            bytes: vec![12; 256],
        },
    ));

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "reply");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    match rx.try_recv().unwrap() {
        NetCommand::ChatMessage(packet) => {
            assert_eq!(packet.message, "reply");
            assert_eq!(packet.last_seen.update.offset, 1);
            // Unknown-sender validation errors are processed as not shown:
            // their offset is acknowledged but no signature enters last-seen.
            assert_eq!(packet.last_seen.update.acknowledged, 0);
            assert_eq!(
                packet.last_seen.update.checksum,
                LastSeenMessagesUpdate::default().checksum
            );
            assert!(packet.last_seen.signatures.is_empty());
        }
        command => panic!("expected chat message command, got {command:?}"),
    }
    assert_eq!(
        world.counters().player_chat_acknowledgement_pending_offset,
        0
    );
}

#[test]
fn chat_entry_starting_with_slash_requests_suggestions_and_submits_command() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/seed");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(counters.chat_command_commands_queued, 1);
    assert_eq!(counters.chat_message_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/seed".to_string().into(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChatCommand(OutboundChatCommand::unsigned("seed"))
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_blocks_movement_keys_and_backspace_edits_text() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/givw");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "e");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert_eq!(
        input
            .screen
            .chat_entry()
            .map(|entry| entry.edit.text())
            .as_deref(),
        Some("/give")
    );
    assert!(!input.forward);
    assert_eq!(counters.player_input_commands_queued, 0);
    assert_eq!(counters.command_suggestion_commands_queued, 3);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/givw".to_string().into(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 1,
            command: "/giv".to_string().into(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 2,
            command: "/give".to_string().into(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_cursor_editing_updates_slash_suggestions() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/givve");
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
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );

    let entry = input
        .screen
        .chat_entry()
        .expect("command entry should stay open");
    assert_eq!(entry.edit.text(), "/give");
    assert_eq!(entry.edit.cursor(), 4);
    assert_eq!(counters.command_suggestion_commands_queued, 2);
    for (id, command) in [(0, "/givve"), (1, "/giv")] {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
                id,
                command: command.to_string().into(),
            })
        );
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_control_word_keys_edit_text() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "/say alpha beta gamma",
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

    let entry = input
        .screen
        .chat_entry()
        .expect("command entry should stay open");
    assert_eq!(entry.edit.text(), "/say alpha ");
    assert_eq!(entry.edit.cursor(), 11);
    assert_eq!(counters.command_suggestion_commands_queued, 3);
    for (id, command) in [
        (0, "/say alpha beta gamma"),
        (1, "/say alpha "),
        (2, "/say alpha "),
    ] {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
                id,
                command: command.to_string().into(),
            })
        );
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn chat_entry_cursor_editing_submits_corrected_message() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "helo");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "l");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.command_suggestion_commands_queued, 0);
    assert_eq!(counters.chat_message_commands_queued, 1);
    match rx.try_recv().unwrap() {
        NetCommand::ChatMessage(packet) => assert_eq!(packet.message, "hello"),
        command => panic!("expected chat message command, got {command:?}"),
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_escape_cancels_without_queuing_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/seed");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(counters.chat_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/seed".to_string().into(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn slash_only_enter_requests_suggestions_and_submits_empty_command() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(counters.chat_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/".to_string().into(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChatCommand(OutboundChatCommand::unsigned(""))
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_multi_character_commit_requests_suggestions_once() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/seed");

    assert_eq!(
        input
            .screen
            .chat_entry()
            .map(|entry| entry.edit.text())
            .as_deref(),
        Some("/seed")
    );
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/seed".to_string().into(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_tab_applies_latest_suggestion_range() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(local_literal_command_tree(&["give"]));

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/giv");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );

    assert_eq!(
        input
            .screen
            .chat_entry()
            .map(|entry| entry.edit.text())
            .as_deref(),
        Some("/give")
    );
    assert_eq!(
        input
            .screen
            .chat_entry()
            .and_then(|entry| entry.last_suggestion_request_text.as_ref()),
        Some(&bbb_protocol::JavaString::from("/give"))
    );
    assert_eq!(counters.command_suggestion_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChatCommand(OutboundChatCommand::unsigned("give"))
    );
}

#[test]
fn command_suggestion_list_cycles_applies_suffix_and_preserves_narration_state() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/ga");
    world.apply_command_suggestions(CommandSuggestions {
        id: 0,
        start: 1,
        length: 2,
        suggestions: vec![
            CommandSuggestion {
                text: "gamemode".into(),
                tooltip: None,
                tooltip_styled: None,
                tooltip_component: None,
            },
            CommandSuggestion {
                text: "gamerule".into(),
                tooltip: Some("Change a game rule".to_string()),
                tooltip_styled: Some(vec![StyledTextRun {
                    text: "Change a game rule".into(),
                    style: ComponentStyle {
                        bold: Some(true),
                        color: Some(0x55_AA_FF),
                        ..Default::default()
                    },
                }]),
                tooltip_component: None,
            },
            CommandSuggestion {
                text: "give".into(),
                tooltip: None,
                tooltip_styled: None,
                tooltip_component: None,
            },
        ],
    });

    let hud = input
        .command_suggestion_hud_state(
            &world,
            &FixedCommandSuggestionTextMeasurer,
            PhysicalSize::new(320, 240),
        )
        .expect("open command entry");
    assert_eq!(hud.suffix.as_deref(), Some("memode"));
    assert_eq!(hud.entries.len(), 3);
    assert!(hud.entries[0].selected);
    assert_eq!(input.command_suggestion_narration(), None);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowDown),
        ElementState::Pressed,
    );
    assert_eq!(
        input.command_suggestion_narration(),
        Some(&CommandSuggestionNarrationState {
            position: 2,
            total: 3,
            text: "gamerule".into(),
            tooltip: Some("Change a game rule".to_string()),
            tooltip_translation_key: None,
        })
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    assert_eq!(input.screen.chat_entry().unwrap().edit.text(), "/gamerule");
    assert_eq!(input.screen.chat_entry().unwrap().edit.cursor(), 9);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    assert_eq!(input.screen.chat_entry().unwrap().edit.text(), "/give");

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    assert_eq!(input.screen.chat_entry().unwrap().edit.text(), "/gamerule");
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/ga".to_string().into(),
        })
    );
}

#[test]
fn command_suggestion_list_limits_rows_scrolls_clicks_and_shows_rich_tooltip() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/c");
    world.apply_command_suggestions(CommandSuggestions {
        id: 0,
        start: 1,
        length: 1,
        suggestions: (0..12)
            .map(|index| CommandSuggestion {
                text: format!("cmd{index:02}").into(),
                tooltip: (index == 2).then(|| "Rich help".to_string()),
                tooltip_styled: (index == 2).then(|| {
                    vec![StyledTextRun {
                        text: "Rich help".into(),
                        style: ComponentStyle {
                            italic: Some(true),
                            underlined: Some(true),
                            color: Some(0x12_34_56),
                            ..Default::default()
                        },
                    }]
                }),
                tooltip_component: None,
            })
            .collect(),
    });
    let surface = PhysicalSize::new(320, 240);
    let hud = input
        .command_suggestion_hud_state(&world, &FixedCommandSuggestionTextMeasurer, surface)
        .expect("open command entry");
    assert_eq!(hud.rect.x, 9);
    assert_eq!(hud.rect.y, 105);
    assert_eq!(hud.rect.width, 31);
    assert_eq!(hud.rect.height, 120);
    assert_eq!(hud.entries.len(), 10);
    assert!(!hud.has_previous);
    assert!(hud.has_next);

    assert!(input
        .handle_command_suggestion_cursor_moved(&world, Some(PhysicalPosition::new(11.0, 131.0))));
    let hovered = input
        .command_suggestion_hud_state(&world, &FixedCommandSuggestionTextMeasurer, surface)
        .unwrap();
    let tooltip = hovered.tooltip.expect("selected rich tooltip");
    assert_eq!(tooltip.text, "Rich help");
    assert_eq!(tooltip.runs[0].style.italic, Some(true));
    assert_eq!(tooltip.runs[0].style.underlined, Some(true));
    assert_eq!(tooltip.runs[0].style.color, Some(0x12_34_56));
    assert_eq!(
        input.command_suggestion_narration(),
        Some(&CommandSuggestionNarrationState {
            position: 3,
            total: 12,
            text: "cmd02".into(),
            tooltip: Some("Rich help".to_string()),
            tooltip_translation_key: None,
        })
    );

    assert!(input.handle_command_suggestion_mouse_wheel(MouseScrollDelta::LineDelta(0.0, -1.0)));
    let scrolled = input
        .command_suggestion_hud_state(&world, &FixedCommandSuggestionTextMeasurer, surface)
        .unwrap();
    assert!(scrolled.has_previous);
    assert!(scrolled.has_next);
    assert_eq!(
        scrolled.entries[0].runs,
        vec![bbb_client_render::HudStyledTextRun::plain("cmd01")]
    );

    assert!(!input.handle_command_suggestion_mouse_input(
        &world,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(500.0, 500.0)),
        &FixedCommandSuggestionTextMeasurer,
    ));
    assert!(input.handle_command_suggestion_mouse_input(
        &world,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(11.0, 155.0)),
        &FixedCommandSuggestionTextMeasurer,
    ));
    assert_eq!(input.screen.chat_entry().unwrap().edit.text(), "/cmd05");
    assert_eq!(input.screen.chat_entry().unwrap().edit.cursor(), 6);
    assert!(input.replace_chat_entry_text("/seed".to_string()));
    assert_eq!(input.screen.chat_entry().unwrap().edit.text(), "/seed");
    assert_eq!(input.screen.chat_entry().unwrap().edit.cursor(), 5);
    assert!(input.screen.chat_entry().unwrap().suggestion_list.is_none());
}

#[test]
fn command_suggestion_escape_hides_list_before_closing_command_entry() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/se");
    world.apply_command_suggestions(CommandSuggestions {
        id: 0,
        start: 1,
        length: 2,
        suggestions: vec![CommandSuggestion {
            text: "seed".into(),
            tooltip: None,
            tooltip_styled: None,
            tooltip_component: None,
        }],
    });
    input.command_suggestion_hud_state(
        &world,
        &FixedCommandSuggestionTextMeasurer,
        PhysicalSize::new(320, 240),
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    assert!(input.command_entry_is_active());
    assert!(input
        .command_suggestion_hud_state(
            &world,
            &FixedCommandSuggestionTextMeasurer,
            PhysicalSize::new(320, 240),
        )
        .unwrap()
        .entries
        .is_empty());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    assert_eq!(input.screen.chat_entry().unwrap().edit.text(), "/se");
    assert_eq!(
        input.command_suggestion_narration(),
        Some(&CommandSuggestionNarrationState {
            position: 1,
            total: 1,
            text: "seed".into(),
            tooltip: None,
            tooltip_translation_key: None,
        })
    );
    assert_eq!(
        input
            .command_suggestion_hud_state(
                &world,
                &FixedCommandSuggestionTextMeasurer,
                PhysicalSize::new(320, 240),
            )
            .unwrap()
            .entries
            .len(),
        1
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    assert!(input.command_entry_is_active());
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    assert!(!input.command_entry_is_active());
}

#[test]
fn chat_scroll_lines_clamp_to_the_rendered_page_top_at_scroll_time() {
    let mut input = ClientInputState::new(true);
    input.scroll_chat_lines(7);
    assert_eq!(input.chat_scroll().position, 0);
    input.set_chat_scroll_page(bbb_client_render::HudChatScrollPage {
        lines: 30,
        lines_per_page: 20,
    });
    input.scroll_chat_lines(7);
    input.scroll_chat_lines(7);
    assert_eq!(input.chat_scroll().position, 10);
    input.scroll_chat_lines(-7);
    assert_eq!(input.chat_scroll().position, 3);
    input.scroll_chat_lines(i32::MIN);
    assert_eq!(input.chat_scroll().position, 0);
}

#[test]
fn chat_entry_tab_applies_server_custom_completion_without_command_request() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_custom_chat_completions(CustomChatCompletions {
        action: CustomChatCompletionsAction::Set,
        entries: vec!["helper".to_string(), "hello".to_string()],
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "say hel");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );

    let entry = input.screen.chat_entry().unwrap();
    assert_eq!(entry.edit.text(), "say hello");
    assert_eq!(entry.edit.cursor(), 9);
    assert_eq!(entry.last_suggestion_request_text, None);
    assert_eq!(counters.command_suggestion_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn custom_chat_completion_candidates_merge_online_names_and_match_vanilla_boundaries() {
    let mut world = WorldStore::new();
    let profile_id = Uuid::from_u128(0x12345678123456781234567812345678);
    world.apply_player_info_update(PlayerInfoUpdate {
        actions: vec![PlayerInfoAction::AddPlayer],
        entries: vec![PlayerInfoEntry {
            profile_id,
            profile: Some(GameProfile {
                uuid: profile_id,
                name: "Alex".to_string(),
                properties: Vec::new(),
            }),
            listed: false,
            latency: 0,
            game_mode: Default::default(),
            display_name: None,
            show_hat: true,
            list_order: 0,
            chat_session: None,
        }],
    });
    world.apply_custom_chat_completions(CustomChatCompletions {
        action: CustomChatCompletionsAction::Set,
        entries: vec![
            "Alice".to_string(),
            "minecraft:hello_world".to_string(),
            "worldwide".to_string(),
        ],
    });

    assert_eq!(
        latest_custom_chat_completion_text(&world, "tell ali later", 8),
        Some(("tell Alice later".to_string(), 10))
    );
    assert_eq!(
        latest_custom_chat_completion_text(&world, "tell ALE", 8),
        Some(("tell Alex".to_string(), 9))
    );
    assert_eq!(
        latest_custom_chat_completion_text(&world, "say world", 9),
        Some(("say worldwide".to_string(), 13))
    );
    assert!(matches_suggestion_sub_str(
        &"world".encode_utf16().collect::<Vec<u16>>(),
        &"minecraft:hello_world".encode_utf16().collect::<Vec<u16>>()
    ));
    assert_eq!(latest_custom_chat_completion_text(&world, "   ", 3), None);
}

#[test]
fn command_entry_tab_ignores_stale_suggestion_response() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/giv");
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "e");
    world.apply_command_suggestions(CommandSuggestions {
        id: 0,
        start: 1,
        length: 3,
        suggestions: vec![CommandSuggestion {
            text: "gamemode".into(),
            tooltip: None,
            tooltip_styled: None,
            tooltip_component: None,
        }],
    });
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );

    assert_eq!(
        input
            .screen
            .chat_entry()
            .map(|entry| entry.edit.text())
            .as_deref(),
        Some("/give")
    );
    assert_eq!(counters.command_suggestion_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/giv".to_string().into(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 1,
            command: "/give".to_string().into(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_tab_ignores_invalid_suggestion_range() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/give");
    world.apply_command_suggestions(CommandSuggestions {
        id: 0,
        start: 10,
        length: 1,
        suggestions: vec![CommandSuggestion {
            text: "gamemode".into(),
            tooltip: None,
            tooltip_styled: None,
            tooltip_component: None,
        }],
    });
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );

    assert_eq!(
        input
            .screen
            .chat_entry()
            .map(|entry| entry.edit.text())
            .as_deref(),
        Some("/give")
    );
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/give".to_string().into(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn text_input_requires_focus_and_leading_slash() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(false);
    let mut counters = NetCounters::default();

    handle_text_input_without_world(&mut input, &mut counters, &commands, "/seed");
    assert!(!input.chat_entry_is_active());

    input.focused = true;
    handle_text_input_without_world(&mut input, &mut counters, &commands, "seed");

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.chat_command_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}
