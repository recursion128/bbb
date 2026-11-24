use super::*;

#[test]
fn static_run_command_action_trims_optional_slash_before_send() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(
        &mut input,
        vec![entry(
            "action",
            NbtSummaryValue::Compound(vec![
                entry("label", NbtSummaryValue::string("Run")),
                entry(
                    "action",
                    NbtSummaryValue::Compound(vec![
                        entry("type", NbtSummaryValue::string("minecraft:run_command")),
                        entry("command", NbtSummaryValue::string("/say static")),
                    ]),
                ),
            ]),
        )],
    );
    let mut counters = NetCounters::default();
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);

    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(160.0, 220.0)),
        PhysicalSize::new(320, 240),
    ));

    assert!(input.command_confirmation().is_some());
    assert!(rx.try_recv().is_err());
    assert!(input.current_dialog().is_some());
    assert_eq!(input.dialog_waiting_ticks(), None);
    input.resolve_command_confirmation(
        true,
        &mut world,
        &mut counters,
        &commands,
        &crate::input::FallbackTextMeasurer,
    );
    assert!(input.current_dialog().is_none());
    assert_eq!(counters.chat_command_commands_queued, 1);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::ChatCommand(command) if command.command == "say static"
    ));
}

#[test]
fn show_dialog_action_replaces_outer_dialog_and_preserves_previous_screen() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = WorldStore::new();
    input.set_screen_for_test(crate::input::Screen::Advancements(Default::default()));
    apply_notice(
        &mut input,
        &mut world,
        vec![
            entry("after_action", NbtSummaryValue::string("wait_for_response")),
            entry(
                "action",
                NbtSummaryValue::Compound(vec![
                    entry("label", NbtSummaryValue::string("Open")),
                    entry(
                        "action",
                        NbtSummaryValue::Compound(vec![
                            entry("type", NbtSummaryValue::string("minecraft:show_dialog")),
                            entry(
                                "dialog",
                                NbtSummaryValue::Compound(vec![
                                    entry("type", NbtSummaryValue::string("minecraft:notice")),
                                    entry("title", NbtSummaryValue::string("Inner")),
                                ]),
                            ),
                        ]),
                    ),
                ]),
            ),
        ],
    );
    let mut counters = NetCounters::default();
    input.apply_screen_requests(&mut world, &mut counters, &None);

    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(160.0, 220.0)),
        PhysicalSize::new(320, 240),
    ));

    let Some(DialogDataState::Notice { common, .. }) = input
        .current_dialog()
        .and_then(|dialog| dialog.data.as_ref())
    else {
        panic!("expected inner Notice dialog");
    };
    assert_eq!(common.title.text, "Inner");
    assert_eq!(input.dialog_waiting_ticks(), None);
    assert!(!input.advancements_screen_is_open());
    assert_eq!(world.counters().show_dialog_packets, 1);
    assert!(input.close_dialog());
    input.apply_screen_requests(&mut world, &mut counters, &None);
    assert!(input.advancements_screen_is_open());
}

#[test]
fn copy_to_clipboard_action_writes_non_empty_text_before_transition() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(&mut input, copy_action_fields("copied text"));
    let mut counters = NetCounters::default();
    let mut clipboard = MockDialogClipboard::default();

    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        Some(&mut clipboard),
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(160.0, 220.0)),
        PhysicalSize::new(320, 240),
    ));

    assert_eq!(clipboard.text.as_deref(), Some("copied text"));
    assert!(input.current_dialog().is_some());

    let mut empty_world = world_with_notice(&mut input, copy_action_fields(""));
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut empty_world,
        &None,
        Some(&mut clipboard),
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(160.0, 220.0)),
        PhysicalSize::new(320, 240),
    ));
    assert_eq!(clipboard.text.as_deref(), Some("copied text"));
    assert!(input.current_dialog().is_some());

    let mut closing_fields = copy_action_fields("not copied");
    closing_fields[1] = entry("after_action", NbtSummaryValue::string("close"));
    let mut closing_world = world_with_notice(&mut input, closing_fields);
    let mut rejecting_clipboard = MockDialogClipboard {
        text: Some("unchanged".to_string()),
        rejects_write: true,
    };
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut closing_world,
        &None,
        Some(&mut rejecting_clipboard),
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(160.0, 220.0)),
        PhysicalSize::new(320, 240),
    ));
    assert_eq!(rejecting_clipboard.text.as_deref(), Some("unchanged"));
    assert!(input.current_dialog().is_none());
}

#[test]
fn change_page_dialog_action_only_applies_after_action() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(
        &mut input,
        vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::string("wait_for_response")),
            entry(
                "action",
                NbtSummaryValue::Compound(vec![
                    entry("label", NbtSummaryValue::string("Page")),
                    entry(
                        "action",
                        NbtSummaryValue::Compound(vec![
                            entry("type", NbtSummaryValue::string("minecraft:change_page")),
                            entry("page", NbtSummaryValue::Int(7)),
                        ]),
                    ),
                ]),
            ),
        ],
    );
    let mut counters = NetCounters::default();

    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(160.0, 220.0)),
        PhysicalSize::new(320, 240),
    ));

    assert_eq!(input.current_dialog(), None);
    assert_eq!(input.dialog_waiting_ticks(), Some(0));
}

#[test]
fn suggest_command_dialog_action_only_applies_after_action() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(
        &mut input,
        vec![
            entry("after_action", NbtSummaryValue::string("wait_for_response")),
            entry(
                "action",
                NbtSummaryValue::Compound(vec![
                    entry("label", NbtSummaryValue::string("Suggest")),
                    entry(
                        "action",
                        NbtSummaryValue::Compound(vec![
                            entry("type", NbtSummaryValue::string("minecraft:suggest_command")),
                            entry("command", NbtSummaryValue::string("/say unchanged")),
                        ]),
                    ),
                ]),
            ),
        ],
    );
    let mut counters = NetCounters::default();

    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(160.0, 220.0)),
        PhysicalSize::new(320, 240),
    ));

    assert_eq!(input.current_dialog(), None);
    assert_eq!(input.dialog_waiting_ticks(), Some(0));
    assert_eq!(counters.chat_command_commands_queued, 0);
}

#[test]
fn static_custom_notice_action_queues_exact_payload_then_closes() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(&mut input, custom_action_fields());
    let mut counters = NetCounters::default();
    let (tx, mut rx) = mpsc::channel(1);

    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &Some(tx),
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(160.0, 220.0)),
        PhysicalSize::new(320, 240),
    ));

    assert!(input.current_dialog().is_none());
    assert_eq!(counters.custom_click_action_commands_queued, 1);
    let command = rx.try_recv().unwrap();
    assert!(matches!(
        command,
        NetCommand::CustomClickAction(CustomClickAction {
            id,
            payload: Some(NbtSummaryValue::Int(7)),
        }) if id == "example:test"
    ));
}

#[test]
fn static_custom_notice_cancel_queues_action_before_forced_close() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(&mut input, custom_action_fields());
    let mut counters = NetCounters::default();
    let (tx, mut rx) = mpsc::channel(1);

    assert!(handle_dialog_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &Some(tx),
        None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    ));

    assert!(input.current_dialog().is_none());
    assert_eq!(counters.custom_click_action_commands_queued, 1);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::CustomClickAction(_)
    ));
}

#[test]
fn wait_for_response_countdown_enables_back_after_five_seconds() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(
        &mut input,
        vec![entry(
            "after_action",
            NbtSummaryValue::string("wait_for_response"),
        )],
    );
    let mut counters = NetCounters::default();

    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(160.0, 220.0)),
        PhysicalSize::new(320, 240),
    ));
    assert_eq!(input.dialog_waiting_ticks(), Some(0));

    let hidden = hud_waiting_response_screen(20, None, PhysicalSize::new(320, 240));
    assert_eq!(hidden.background_layers.len(), 1);
    assert_eq!(
        hidden.background_layers[0].texture,
        HudInventoryBackgroundTexture::Texture("textures/gui/inworld_menu_background.png")
    );
    assert_eq!(hidden.text_labels.len(), 1);

    let first_countdown = hud_waiting_response_screen(21, None, PhysicalSize::new(320, 240));
    assert_eq!(first_countdown.text_labels[1].text, "Back (4s)");
    assert_eq!(
        first_countdown.background_layers[1].texture,
        HudInventoryBackgroundTexture::Sprite("widget/button_disabled")
    );
    let last_countdown = hud_waiting_response_screen(100, None, PhysicalSize::new(320, 240));
    assert_eq!(last_countdown.text_labels[1].text, "Back (1s)");

    input.advance_dialog_waiting_ticks(100);
    assert!(handle_dialog_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    ));
    assert_eq!(input.dialog_waiting_ticks(), Some(100));

    input.advance_dialog_waiting_ticks(1);
    let active = hud_waiting_response_screen(101, Some((160, 64)), PhysicalSize::new(320, 240));
    assert_eq!(active.text_labels[1].text, "Back");
    assert_eq!(
        active.background_layers[1].texture,
        HudInventoryBackgroundTexture::Sprite("widget/button_highlighted")
    );
    assert!(handle_dialog_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    ));
    assert!(!input.dialog_screen_is_open());

    let mut mouse_world = world_with_notice(
        &mut input,
        vec![entry(
            "after_action",
            NbtSummaryValue::string("wait_for_response"),
        )],
    );
    assert!(input.wait_for_dialog_response());
    input.advance_dialog_waiting_ticks(101);
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut mouse_world,
        &None,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(160.0, 64.0)),
        PhysicalSize::new(320, 240),
    ));
    assert!(!input.dialog_screen_is_open());
}
