use super::*;
use crate::input::dialog::DialogFocusState;
use crate::input::dialog::DialogInputValueState;

#[test]
fn boolean_input_renders_toggles_and_binds_dynamic_actions() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(
        &mut input,
        vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::string("none")),
            entry(
                "inputs",
                NbtSummaryValue::List(vec![boolean_input(
                    "enabled", "Enabled", false, "yes", "no",
                )]),
            ),
            entry(
                "action",
                NbtSummaryValue::Compound(vec![
                    entry("label", NbtSummaryValue::string("Send")),
                    entry(
                        "action",
                        NbtSummaryValue::Compound(vec![
                            entry("type", NbtSummaryValue::string("minecraft:dynamic/custom")),
                            entry("id", NbtSummaryValue::string("example:dynamic")),
                            entry(
                                "additions",
                                NbtSummaryValue::Compound(vec![
                                    entry("enabled", NbtSummaryValue::string("old")),
                                    entry("fixed", NbtSummaryValue::Int(7)),
                                ]),
                            ),
                        ]),
                    ),
                ]),
            ),
        ],
    );
    let surface_size = PhysicalSize::new(320, 240);
    let input_layout = {
        let controls = input
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .unwrap();
        dialog_input_layouts(&controls, surface_size).unwrap()[0]
    };
    let input_cursor = (
        input_layout.x.saturating_add(1),
        input_layout.y.saturating_add(1),
    );
    let terrain_textures = TerrainTextureState::default();
    let screen = hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &terrain_textures,
        &DialogHudLocalState {
            cursor_position: Some(input_cursor),
            ..Default::default()
        },
        surface_size,
        0.0,
    )
    .unwrap();
    let scroll_area = screen.scroll_area.as_ref().unwrap();
    assert_eq!(scroll_area.text_labels[0].text, "Enabled");
    assert!(scroll_area.background_layers.iter().any(|layer| {
        layer.texture == HudInventoryBackgroundTexture::Sprite("widget/checkbox_highlighted")
            && (layer.x, layer.y) == (input_layout.x, input_layout.y)
    }));

    let mut counters = NetCounters::default();
    let (tx, mut rx) = mpsc::channel(2);
    let net_commands = Some(tx);
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(input_cursor.0),
            f64::from(input_cursor.1),
        )),
        surface_size,
    ));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::Boolean { selected: true })]
    );
    assert_eq!(input.dialog_focused_input(), Some(0));
    let focused_screen = hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &terrain_textures,
        &DialogHudLocalState::default(),
        surface_size,
        0.0,
    )
    .unwrap();
    assert!(focused_screen
        .scroll_area
        .as_ref()
        .unwrap()
        .background_layers
        .iter()
        .any(|layer| {
            layer.texture
                == HudInventoryBackgroundTexture::Sprite("widget/checkbox_selected_highlighted")
                && (layer.x, layer.y) == (input_layout.x, input_layout.y)
        }));
    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
        false,
        false,
    ));
    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
        false,
        false,
    ));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::Boolean { selected: true })]
    );
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(0.0, 0.0)),
        surface_size,
    ));
    assert_eq!(input.dialog_focused_input(), Some(0));

    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(160.0, 220.0)),
        surface_size,
    ));
    assert!(input.current_dialog().is_some());
    assert_eq!(input.dialog_focused_input(), None);
    assert_eq!(counters.custom_click_action_commands_queued, 1);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::CustomClickAction(CustomClickAction {
            id,
            payload: Some(NbtSummaryValue::Compound(entries)),
        }) if id == "example:dynamic"
            && entries == vec![
                entry("enabled", NbtSummaryValue::Byte(1)),
                entry("fixed", NbtSummaryValue::Int(7)),
            ]
    ));

    let mut command_world = world_with_notice(
        &mut input,
        vec![
            entry(
                "inputs",
                NbtSummaryValue::List(vec![boolean_input("enabled", "Enabled", true, "yes", "no")]),
            ),
            entry(
                "action",
                NbtSummaryValue::Compound(vec![
                    entry("label", NbtSummaryValue::string("Run")),
                    entry(
                        "action",
                        NbtSummaryValue::Compound(vec![
                            entry(
                                "type",
                                NbtSummaryValue::string("minecraft:dynamic/run_command"),
                            ),
                            entry(
                                "template",
                                NbtSummaryValue::string("/say $(enabled)-$(missing)"),
                            ),
                        ]),
                    ),
                ]),
            ),
        ],
    );
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut command_world,
        &net_commands,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(160.0, 220.0)),
        surface_size,
    ));
    assert!(input.command_confirmation().is_some());
    assert!(rx.try_recv().is_err());
    assert!(input.current_dialog().is_some());
    assert_eq!(input.dialog_waiting_ticks(), None);
    input.resolve_command_confirmation(
        true,
        &mut command_world,
        &mut counters,
        &net_commands,
        &crate::input::FallbackTextMeasurer,
    );
    assert!(input.current_dialog().is_none());
    assert_eq!(counters.chat_command_commands_queued, 1);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::ChatCommand(command) if command.command == "say yes-"
    ));
}

#[test]
fn number_range_renders_drags_and_binds_float_actions() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(
        &mut input,
        vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::string("none")),
            entry(
                "inputs",
                NbtSummaryValue::List(vec![number_range_input(
                    "speed",
                    "A very long speed control",
                    100,
                    0.0,
                    10.0,
                    Some(2.0),
                    Some(2.0),
                )]),
            ),
            entry(
                "action",
                NbtSummaryValue::Compound(vec![
                    entry("label", NbtSummaryValue::string("Send")),
                    entry(
                        "action",
                        NbtSummaryValue::Compound(vec![
                            entry("type", NbtSummaryValue::string("minecraft:dynamic/custom")),
                            entry("id", NbtSummaryValue::string("example:range")),
                        ]),
                    ),
                ]),
            ),
        ],
    );
    let surface_size = PhysicalSize::new(320, 240);
    let input_layout = {
        let dialog = input.current_dialog().unwrap();
        let controls = dialog
            .data
            .as_ref()
            .and_then(|data| dialog_controls(data, world.server_links()))
            .unwrap_or_else(|| panic!("invalid range dialog: {:?}", dialog.semantic_error));
        dialog_input_layouts(&controls, surface_size).unwrap()[0]
    };
    let screen = hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &TerrainTextureState::default(),
        &DialogHudLocalState::default(),
        surface_size,
        0.0,
    )
    .unwrap();
    let scroll_area = screen.scroll_area.as_ref().unwrap();
    let range_label = scroll_area
        .text_labels
        .iter()
        .find(|label| label.text == "A very long speed control: 2")
        .unwrap();
    assert_eq!(
        range_label.scrolling,
        Some(HudInventoryScrollingText {
            x: input_layout.x + 2,
            y: input_layout.y,
            width: 96,
            height: 20,
        })
    );
    assert_eq!(
        scroll_area
            .background_layers
            .iter()
            .filter(|layer| layer.texture == HudInventoryBackgroundTexture::Sprite("widget/slider"))
            .count(),
        3
    );
    assert!(scroll_area.background_layers.iter().any(|layer| {
        layer.texture == HudInventoryBackgroundTexture::Sprite("widget/slider_handle")
            && layer.x == input_layout.x + 18
    }));

    let mut counters = NetCounters::default();
    let (tx, mut rx) = mpsc::channel(2);
    let net_commands = Some(tx);
    let pressed_x = input_layout.x + SLIDER_HANDLE_HALF_WIDTH + 69;
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(pressed_x),
            f64::from(input_layout.y + 1),
        )),
        surface_size,
    ));
    assert_eq!(input.dialog_dragging_number_range_input(), Some(0));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::NumberRange {
            slider_value_bits: 0.75f64.to_bits(),
        })]
    );

    assert!(handle_dialog_cursor_moved(
        &mut input,
        &mut world,
        Some(PhysicalPosition::new(
            f64::from(input_layout.x + SLIDER_HANDLE_HALF_WIDTH + 92),
            f64::from(input_layout.y + 1),
        )),
        surface_size,
    ));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::NumberRange {
            slider_value_bits: 1.0f64.to_bits(),
        })]
    );
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        MouseButton::Left,
        ElementState::Released,
        None,
        surface_size,
    ));
    assert_eq!(input.dialog_dragging_number_range_input(), None);
    assert_eq!(input.dialog_focused_input(), Some(0));
    assert!(input.dialog_number_range_keyboard_can_change(0));
    let focused_screen = hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &TerrainTextureState::default(),
        &DialogHudLocalState::default(),
        surface_size,
        0.0,
    )
    .unwrap();
    assert_eq!(
        focused_screen
            .scroll_area
            .as_ref()
            .unwrap()
            .background_layers
            .iter()
            .filter(|layer| {
                layer.texture
                    == HudInventoryBackgroundTexture::Sprite("widget/slider_handle_highlighted")
            })
            .count(),
        1
    );
    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
        false,
        false,
    ));
    let keyboard_value = 1.0f64 - 1.0 / 92.0;
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::NumberRange {
            slider_value_bits: keyboard_value.to_bits(),
        })]
    );
    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
        false,
        false,
    ));
    assert!(!input.dialog_number_range_keyboard_can_change(0));
    let locked_screen = hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &TerrainTextureState::default(),
        &DialogHudLocalState::default(),
        surface_size,
        0.0,
    )
    .unwrap();
    assert_eq!(
        locked_screen
            .scroll_area
            .as_ref()
            .unwrap()
            .background_layers
            .iter()
            .filter(|layer| {
                layer.texture == HudInventoryBackgroundTexture::Sprite("widget/slider_highlighted")
            })
            .count(),
        3
    );
    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
        false,
        false,
    ));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::NumberRange {
            slider_value_bits: keyboard_value.to_bits(),
        })]
    );

    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(160.0, 220.0)),
        surface_size,
    ));
    assert_eq!(counters.custom_click_action_commands_queued, 1);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::CustomClickAction(CustomClickAction {
            id,
            payload: Some(NbtSummaryValue::Compound(entries)),
        }) if id == "example:range"
            && entries == vec![entry("speed", NbtSummaryValue::Float(10.0f32.to_bits()))]
    ));

    world.apply_show_dialog(bbb_protocol::packets::ShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:notice")),
                entry("title", NbtSummaryValue::string("Range")),
                entry(
                    "inputs",
                    NbtSummaryValue::List(vec![number_range_input(
                        "speed",
                        "Speed",
                        100,
                        0.0,
                        10.0,
                        Some(2.0),
                        Some(2.0),
                    )]),
                ),
                entry(
                    "action",
                    NbtSummaryValue::Compound(vec![
                        entry("label", NbtSummaryValue::string("Run")),
                        entry(
                            "action",
                            NbtSummaryValue::Compound(vec![
                                entry(
                                    "type",
                                    NbtSummaryValue::string("minecraft:dynamic/run_command"),
                                ),
                                entry("template", NbtSummaryValue::string("/say $(speed)")),
                            ]),
                        ),
                    ]),
                ),
            ]),
        },
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(160.0, 220.0)),
        surface_size,
    ));
    assert!(input.command_confirmation().is_some());
    assert!(rx.try_recv().is_err());
    assert!(input.current_dialog().is_some());
    assert_eq!(input.dialog_waiting_ticks(), None);
    input.resolve_command_confirmation(
        true,
        &mut world,
        &mut counters,
        &net_commands,
        &crate::input::FallbackTextMeasurer,
    );
    assert_eq!(counters.chat_command_commands_queued, 1);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::ChatCommand(command) if command.command == "say 2"
    ));
}

#[test]
fn number_range_step_rounding_and_boundary_correction_match_vanilla() {
    let range = DialogNumberRangeState {
        start_bits: 10.0f32.to_bits(),
        end_bits: 0.0f32.to_bits(),
        initial_bits: Some(7.5f32.to_bits()),
        step_bits: Some(2.0f32.to_bits()),
    };

    let rounded_negative = dialog_number_range_scaled_value(&range, 0.6);
    assert_eq!(rounded_negative.to_bits(), 3.5f32.to_bits());
    assert_eq!(dialog_number_range_value_string(rounded_negative), "3.5");
    assert_eq!(
        dialog_number_range_scaled_value(&range, 1.0).to_bits(),
        1.5f32.to_bits()
    );

    let label = DialogTextState {
        text: "Speed".to_string(),
        runs: Vec::new(),
        component: NbtSummaryValue::string("Speed"),
    };
    let runs = dialog_number_range_label_runs("%2$s / %1$s %%", &label, "3.5");
    assert_eq!(
        runs.iter().map(|run| run.text.as_str()).collect::<String>(),
        "3.5 / Speed %"
    );
    assert_eq!(
        dialog_number_range_label_runs("bad %d", &label, "3.5"),
        vec![HudStyledTextRun::plain("bad %d")]
    );
}

#[test]
fn single_option_renders_cycles_and_binds_string_actions() {
    let mut input = crate::input::ClientInputState::new(true);
    let options = vec![
        NbtSummaryValue::string("one"),
        NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::string("two")),
            entry("display", NbtSummaryValue::string("Second")),
            entry("initial", NbtSummaryValue::Byte(1)),
        ]),
        NbtSummaryValue::string("three"),
    ];
    let mut world = world_with_notice(
        &mut input,
        vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::string("none")),
            entry(
                "inputs",
                NbtSummaryValue::List(vec![single_option_input(
                    "mode",
                    "A very long mode label",
                    100,
                    true,
                    options.clone(),
                )]),
            ),
            entry(
                "action",
                NbtSummaryValue::Compound(vec![
                    entry("label", NbtSummaryValue::string("Send")),
                    entry(
                        "action",
                        NbtSummaryValue::Compound(vec![
                            entry("type", NbtSummaryValue::string("minecraft:dynamic/custom")),
                            entry("id", NbtSummaryValue::string("example:option")),
                        ]),
                    ),
                ]),
            ),
        ],
    );
    let surface_size = PhysicalSize::new(320, 240);
    let input_layout = {
        let controls = input
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .unwrap();
        dialog_input_layouts(&controls, surface_size).unwrap()[0]
    };
    let input_cursor = (input_layout.x + 1, input_layout.y + 1);
    let screen = hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &TerrainTextureState::default(),
        &DialogHudLocalState {
            cursor_position: Some(input_cursor),
            ..Default::default()
        },
        surface_size,
        0.0,
    )
    .unwrap();
    let scroll_area = screen.scroll_area.as_ref().unwrap();
    let option_label = scroll_area
        .text_labels
        .iter()
        .find(|label| label.text == "A very long mode label: Second")
        .unwrap();
    assert_eq!(
        option_label.scrolling,
        Some(HudInventoryScrollingText {
            x: input_layout.x + 2,
            y: input_layout.y,
            width: 96,
            height: 20,
        })
    );
    assert_eq!(
        scroll_area
            .background_layers
            .iter()
            .filter(|layer| {
                layer.texture == HudInventoryBackgroundTexture::Sprite("widget/button_highlighted")
                    && layer.y == input_layout.y
            })
            .count(),
        3
    );

    let mut counters = NetCounters::default();
    let (tx, mut rx) = mpsc::channel(2);
    let net_commands = Some(tx);
    let cursor = Some(PhysicalPosition::new(
        f64::from(input_cursor.0),
        f64::from(input_cursor.1),
    ));
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        cursor,
        surface_size,
    ));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::SingleOption {
            selected_index: 2,
        })]
    );
    assert!(handle_dialog_mouse_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        cursor,
        surface_size,
        true,
    ));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::SingleOption {
            selected_index: 1,
        })]
    );
    assert_eq!(input.dialog_focused_input(), Some(0));
    let focused_screen = hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &TerrainTextureState::default(),
        &DialogHudLocalState::default(),
        surface_size,
        0.0,
    )
    .unwrap();
    assert_eq!(
        focused_screen
            .scroll_area
            .as_ref()
            .unwrap()
            .background_layers
            .iter()
            .filter(|layer| {
                layer.texture == HudInventoryBackgroundTexture::Sprite("widget/button_highlighted")
                    && layer.y == input_layout.y
            })
            .count(),
        3
    );
    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
        false,
        false,
    ));
    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        PhysicalKey::Code(KeyCode::NumpadEnter),
        ElementState::Pressed,
        true,
        false,
    ));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::SingleOption {
            selected_index: 1,
        })]
    );
    assert!(handle_dialog_mouse_wheel(
        &mut input,
        &mut world,
        MouseScrollDelta::LineDelta(0.0, -1.0),
        cursor,
        surface_size,
    ));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::SingleOption {
            selected_index: 1,
        })]
    );
    assert!(handle_dialog_mouse_wheel(
        &mut input,
        &mut world,
        MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 1.0)),
        cursor,
        surface_size,
    ));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::SingleOption {
            selected_index: 1,
        })]
    );

    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(160.0, 220.0)),
        surface_size,
    ));
    assert_eq!(counters.custom_click_action_commands_queued, 1);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::CustomClickAction(CustomClickAction {
            id,
            payload: Some(NbtSummaryValue::Compound(entries)),
        }) if id == "example:option"
            && entries == vec![entry(
                "mode",
                NbtSummaryValue::string("two"),
            )]
    ));

    world.apply_show_dialog(bbb_protocol::packets::ShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:notice")),
                entry("title", NbtSummaryValue::string("Options")),
                entry(
                    "inputs",
                    NbtSummaryValue::List(vec![single_option_input(
                        "mode",
                        "Mode",
                        100,
                        false,
                        vec![NbtSummaryValue::Compound(vec![
                            entry("id", NbtSummaryValue::string("alpha")),
                            entry("display", NbtSummaryValue::string("Alpha Display")),
                        ])],
                    )]),
                ),
                entry(
                    "action",
                    NbtSummaryValue::Compound(vec![
                        entry("label", NbtSummaryValue::string("Run")),
                        entry(
                            "action",
                            NbtSummaryValue::Compound(vec![
                                entry(
                                    "type",
                                    NbtSummaryValue::string("minecraft:dynamic/run_command"),
                                ),
                                entry("template", NbtSummaryValue::string("/say $(mode)")),
                            ]),
                        ),
                    ]),
                ),
            ]),
        },
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    let screen = hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &TerrainTextureState::default(),
        &DialogHudLocalState::default(),
        surface_size,
        0.0,
    )
    .unwrap();
    let scroll_area = screen.scroll_area.as_ref().unwrap();
    assert!(scroll_area
        .text_labels
        .iter()
        .any(|label| label.text == "Alpha Display"));
    assert!(!scroll_area
        .text_labels
        .iter()
        .any(|label| label.text.contains("Mode:")));
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(160.0, 220.0)),
        surface_size,
    ));
    assert!(input.command_confirmation().is_some());
    assert!(rx.try_recv().is_err());
    assert!(input.current_dialog().is_some());
    assert_eq!(input.dialog_waiting_ticks(), None);
    input.resolve_command_confirmation(
        true,
        &mut world,
        &mut counters,
        &net_commands,
        &crate::input::FallbackTextMeasurer,
    );
    assert_eq!(counters.chat_command_commands_queued, 1);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::ChatCommand(command) if command.command == "say alpha"
    ));
}

#[test]
fn single_line_text_renders_edits_and_binds_escaped_and_raw_strings() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(
        &mut input,
        vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::string("none")),
            entry(
                "inputs",
                NbtSummaryValue::List(vec![text_input("name", "Name", 100, true, "ab😀", 5)]),
            ),
        ],
    );
    let surface_size = PhysicalSize::new(320, 240);
    let input_layout = {
        let controls = input
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .unwrap();
        dialog_input_layouts(&controls, surface_size).unwrap()[0]
    };
    let box_layout = dialog_single_line_text_box_layout(input_layout, true);
    let screen = hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &TerrainTextureState::default(),
        &DialogHudLocalState::default(),
        surface_size,
        0.0,
    )
    .unwrap();
    let scroll_area = screen.scroll_area.as_ref().unwrap();
    assert!(scroll_area.text_labels.iter().any(|label| {
        label.text == "Name" && (label.x, label.y) == (input_layout.x, input_layout.y)
    }));
    let text = scroll_area
        .text_labels
        .iter()
        .find(|label| label.text == "ab😀")
        .unwrap();
    assert_eq!(
        (text.x, text.y, text.width),
        (box_layout.x + 4, box_layout.y + 6, 92)
    );
    assert_eq!(text.input.as_ref().unwrap().cursor, 4);
    assert!(!text.input.as_ref().unwrap().cursor_visible);
    assert_eq!(text.tint, TEXT_INPUT_COLOR);
    assert!(text.shadow);
    assert_eq!(
        scroll_area
            .background_layers
            .iter()
            .filter(|layer| {
                layer.texture == HudInventoryBackgroundTexture::Sprite("widget/text_field")
                    && layer.y == box_layout.y
            })
            .count(),
        3
    );

    let mut counters = NetCounters::default();
    let cursor = Some(PhysicalPosition::new(
        f64::from(box_layout.x + TEXT_INPUT_INSET + 6),
        f64::from(box_layout.y + 1),
    ));
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        cursor,
        surface_size,
    ));
    assert_eq!(
        input.dialog_text_edit_state(),
        Some(crate::input::dialog::DialogTextEditState {
            input_index: 0,
            cursor: 1,
            selection: 1,
            scroll_amount: 0,
            ..Default::default()
        })
    );
    assert!(handle_dialog_cursor_moved(
        &mut input,
        &mut world,
        Some(PhysicalPosition::new(
            f64::from(box_layout.x + TEXT_INPUT_INSET + 12),
            f64::from(box_layout.y + 1),
        )),
        surface_size,
    ));
    assert_eq!(
        input.dialog_text_edit_state(),
        Some(crate::input::dialog::DialogTextEditState {
            input_index: 0,
            cursor: 2,
            selection: 1,
            scroll_amount: 0,
            ..Default::default()
        })
    );
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        MouseButton::Left,
        ElementState::Released,
        cursor,
        surface_size,
    ));
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        cursor,
        surface_size,
    ));
    assert!(handle_dialog_text_input(&mut input, &mut world, "§C😀\n"));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::Text {
            value: "aCb😀".into(),
        })]
    );

    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::ArrowRight),
        ElementState::Pressed,
        true,
        false,
    ));
    let mut clipboard = MockDialogClipboard::default();
    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        Some(&mut clipboard),
        PhysicalKey::Code(KeyCode::KeyX),
        ElementState::Pressed,
        false,
        true,
    ));
    assert_eq!(clipboard.text.as_deref(), Some("b"));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::Text {
            value: "aC😀".into(),
        })]
    );
    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        Some(&mut clipboard),
        PhysicalKey::Code(KeyCode::KeyV),
        ElementState::Pressed,
        false,
        true,
    ));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::Text {
            value: "aCb😀".into(),
        })]
    );
    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
        false,
        true,
    ));
    assert!(handle_dialog_text_input(&mut input, &mut world, "😀😀😀"));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::Text {
            value: "😀😀".into(),
        })]
    );
    let screen = hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &TerrainTextureState::default(),
        &DialogHudLocalState::default(),
        surface_size,
        0.0,
    )
    .unwrap();
    assert_eq!(
        screen
            .scroll_area
            .as_ref()
            .unwrap()
            .background_layers
            .iter()
            .filter(|layer| {
                layer.texture
                    == HudInventoryBackgroundTexture::Sprite("widget/text_field_highlighted")
                    && layer.y == box_layout.y
            })
            .count(),
        3
    );
    let edited = screen
        .scroll_area
        .as_ref()
        .unwrap()
        .text_labels
        .iter()
        .find(|label| label.text == "😀😀")
        .unwrap();
    assert_eq!(edited.input.as_ref().unwrap().cursor, 4);

    let mut word_world = world_with_notice(
        &mut input,
        vec![entry(
            "inputs",
            NbtSummaryValue::List(vec![text_input("name", "Name", 100, false, "one two", 32)]),
        )],
    );
    assert!(input.focus_dialog_text_input(0, 7, false));
    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut word_world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
        false,
        true,
    ));
    assert_eq!(input.dialog_text_edit_state().unwrap().cursor, 4);
    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut word_world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Home),
        ElementState::Pressed,
        true,
        false,
    ));
    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut word_world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
        false,
        false,
    ));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::Text {
            value: "two".into(),
        })]
    );
    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut word_world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::End),
        ElementState::Pressed,
        false,
        false,
    ));
    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut word_world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
        false,
        true,
    ));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::Text {
            value: String::new().into(),
        })]
    );

    let escaped_world = world_with_notice(
        &mut input,
        vec![entry(
            "inputs",
            NbtSummaryValue::List(vec![text_input(
                "name",
                "Name",
                100,
                false,
                "a\"'\\\n\u{0001}",
                32,
            )]),
        )],
    );
    let values = bound_dialog_input_values(&input, &escaped_world).unwrap();
    assert_eq!(values["name"].template_substitution, "a\\\"\\'\\\\\\n\\x01");
    assert_eq!(
        instantiate_dialog_command_template(
            &DialogCommandTemplateState {
                raw: "/say $(name)".into(),
                segments: vec!["/say ".into(), bbb_protocol::JavaString::default()],
                variables: vec!["name".to_string()],
            },
            &values,
        ),
        "/say a\\\"\\'\\\\\\n\\x01"
    );
    assert_eq!(
        values["name"].tag,
        NbtSummaryValue::string("a\"'\\\n\u{0001}")
    );
}

#[test]
fn dialog_ime_preedit_projects_from_the_focused_text_cursor() {
    let mut input = crate::input::ClientInputState::new(true);
    let world = world_with_notice(
        &mut input,
        vec![entry(
            "inputs",
            NbtSummaryValue::List(vec![text_input("name", "Name", 100, true, "ab", 32)]),
        )],
    );
    assert!(input.focus_dialog_text_input(0, 2, false));
    let surface_size = PhysicalSize::new(320, 240);
    let input_layout = {
        let controls = input
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .unwrap();
        dialog_input_layouts(&controls, surface_size).unwrap()[0]
    };
    let box_layout = dialog_single_line_text_box_layout(input_layout, true);
    let preedit = DialogImePreeditHudState {
        text: "xy".to_string(),
        caret: Some(1),
        focused_start: 0,
        focused_end: 1,
        screen_revision: input.dialog_screen_revision(),
        caret_visible: true,
    };
    let screen = hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &TerrainTextureState::default(),
        &DialogHudLocalState {
            dialog_ime_preedit: Some(preedit.clone()),
            ..Default::default()
        },
        surface_size,
        0.0,
    )
    .unwrap();
    let overlay = screen.preedit_overlay.unwrap();
    let input_left =
        box_layout.x + TEXT_INPUT_INSET + ApproximateDialogTextMeasurer.width("ab") + 1;
    let input_top = box_layout.y + TEXT_INPUT_Y_OFFSET;
    assert_eq!(overlay.text, "xy");
    assert_eq!((overlay.focused_start, overlay.focused_end), (0, 1));
    assert_eq!(overlay.caret, Some(1));
    assert!(overlay.caret_visible);
    assert_eq!(
        (overlay.text_x, overlay.text_y),
        (input_left, input_top + 14)
    );
    assert_eq!(
        (
            overlay.background_x,
            overlay.background_y,
            overlay.background_width,
            overlay.background_height,
        ),
        (input_left - 5, input_top + 9, 22, 19)
    );
    assert_eq!(
        dialog_ime_cursor_area(
            &input,
            &world,
            Some(&preedit),
            surface_size,
            &ApproximateDialogTextMeasurer,
        ),
        Some((
            PhysicalPosition::new(input_left - 2, input_top - 2),
            PhysicalSize::new(16, 27),
        ))
    );

    let mut stale = preedit.clone();
    stale.screen_revision = stale.screen_revision.saturating_add(1);
    assert_eq!(
        dialog_ime_cursor_area(
            &input,
            &world,
            Some(&stale),
            surface_size,
            &ApproximateDialogTextMeasurer,
        ),
        None
    );

    let mut overflowing = preedit;
    overflowing.text = "x".repeat(60);
    let overflow = dialog_preedit_projection(
        &input,
        &world,
        &overflowing,
        surface_size,
        &ApproximateDialogTextMeasurer,
    )
    .unwrap();
    assert_eq!(
        overflow.overlay.text_x,
        i32::try_from(surface_size.width).unwrap()
            - ApproximateDialogTextMeasurer.width(&overflowing.text)
    );
}

#[test]
fn dialog_ime_preedit_uses_the_multiline_cursor_row() {
    let mut input = crate::input::ClientInputState::new(true);
    let world = world_with_notice(
        &mut input,
        vec![entry(
            "inputs",
            NbtSummaryValue::List(vec![multiline_text_input(
                "notes",
                "Notes",
                100,
                false,
                "a\nb",
                32,
                None,
                Some(30),
            )]),
        )],
    );
    assert!(input.focus_dialog_text_input(0, 3, false));
    let surface_size = PhysicalSize::new(320, 240);
    let box_layout = {
        let controls = input
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .unwrap();
        let input_layout = dialog_input_layouts(&controls, surface_size).unwrap()[0];
        dialog_multiline_text_box_layout(input_layout, false)
    };
    let preedit = DialogImePreeditHudState {
        text: "候选".to_string(),
        caret: None,
        focused_start: 0,
        focused_end: 2,
        screen_revision: input.dialog_screen_revision(),
        caret_visible: true,
    };
    let projection = dialog_preedit_projection(
        &input,
        &world,
        &preedit,
        surface_size,
        &ApproximateDialogTextMeasurer,
    )
    .unwrap();

    assert_eq!(
        projection.overlay.text_x,
        box_layout.x + TEXT_INPUT_INSET + ApproximateDialogTextMeasurer.width("b")
    );
    assert_eq!(
        projection.overlay.text_y,
        box_layout.y + TEXT_INPUT_INSET + TEXT_LINE_HEIGHT + 14
    );
}

#[test]
fn multiline_text_renders_wraps_edits_scrolls_and_enforces_line_limit() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(
        &mut input,
        vec![entry(
            "inputs",
            NbtSummaryValue::List(vec![multiline_text_input(
                "notes",
                "Notes",
                44,
                true,
                "ab cd\nx",
                64,
                Some(3),
                Some(17),
            )]),
        )],
    );
    let surface_size = PhysicalSize::new(320, 240);
    let (input_layout, box_layout) = {
        let controls = input
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .unwrap();
        let input_layout = dialog_input_layouts(&controls, surface_size).unwrap()[0];
        (
            input_layout,
            dialog_multiline_text_box_layout(input_layout, true),
        )
    };
    let graph = dialog_focus_graph(
        &input
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .unwrap(),
        surface_size,
    )
    .unwrap();
    assert!(graph.tab.contains(&DialogFocusState::Input { index: 0 }));

    let screen = hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &TerrainTextureState::default(),
        &DialogHudLocalState::default(),
        surface_size,
        0.0,
    )
    .unwrap();
    let scroll_area = screen.scroll_area.as_ref().unwrap();
    assert!(scroll_area.text_labels.iter().any(|label| {
        label.text == "Notes" && (label.x, label.y) == (input_layout.x, input_layout.y)
    }));
    assert_eq!(scroll_area.text_areas.len(), 1);
    let area = &scroll_area.text_areas[0];
    assert_eq!(
        (area.x, area.y, area.width, area.height, area.text.as_str()),
        (box_layout.x, box_layout.y, 44, 17, "ab cd\nx")
    );
    assert_eq!(area.scroll_amount, i32::MAX);
    assert_eq!(area.max_length, 64);
    assert_eq!(
        scroll_area
            .background_layers
            .iter()
            .filter(|layer| {
                layer.texture == HudInventoryBackgroundTexture::Sprite("widget/text_field")
                    && layer.x >= box_layout.x
                    && layer.y >= box_layout.y
            })
            .count(),
        9
    );

    assert!(input.focus_dialog_navigation_target(
        &world,
        DialogFocusState::Input { index: 0 },
        true
    ));
    let mut counters = NetCounters::default();
    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Home),
        ElementState::Pressed,
        false,
        false,
    ));
    assert_eq!(input.dialog_text_edit_state().unwrap().cursor, 6);
    assert!(handle_dialog_text_input(&mut input, &mut world, "§Y"));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::Text {
            value: "ab cd\nYx".into(),
        })]
    );
    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
        false,
        false,
    ));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::Text {
            value: "ab cd\nY\nx".into(),
        })]
    );
    assert!(input.dialog_text_edit_state().unwrap().scroll_amount > 0);

    let before = input.current_dialog_input_values().to_vec();
    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
        false,
        false,
    ));
    assert_eq!(input.current_dialog_input_values(), before);
    let scrollbar_cursor = Some(PhysicalPosition::new(
        f64::from(box_layout.x + box_layout.width + 2),
        f64::from(box_layout.y + 4),
    ));
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        scrollbar_cursor,
        surface_size,
    ));
    assert_eq!(input.dialog_dragging_multiline_scroll_input(), Some(0));
    assert!(handle_dialog_cursor_moved(
        &mut input,
        &mut world,
        Some(PhysicalPosition::new(
            f64::from(box_layout.x + box_layout.width + 2),
            f64::from(box_layout.y + 14),
        )),
        surface_size,
    ));
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        MouseButton::Left,
        ElementState::Released,
        scrollbar_cursor,
        surface_size,
    ));
    assert_eq!(input.dialog_dragging_multiline_scroll_input(), None);
    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
        false,
        false,
    ));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::Text {
            value: "ab cd\nYx".into(),
        })]
    );
}

#[test]
fn text_double_click_uses_vanilla_single_and_multiline_word_selection() {
    let mut input = crate::input::ClientInputState::new(true);
    let surface_size = PhysicalSize::new(320, 240);
    let mut counters = NetCounters::default();
    let mut single = world_with_notice(
        &mut input,
        vec![entry(
            "inputs",
            NbtSummaryValue::List(vec![text_input(
                "name",
                "Name",
                120,
                false,
                "one two three",
                32,
            )]),
        )],
    );
    let single_layout = {
        let controls = input
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, single.server_links()))
            .unwrap();
        let layout = dialog_input_layouts(&controls, surface_size).unwrap()[0];
        dialog_single_line_text_box_layout(layout, false)
    };
    let single_first = Some(PhysicalPosition::new(
        f64::from(
            single_layout.x + TEXT_INPUT_INSET + ApproximateDialogTextMeasurer.width("one t"),
        ),
        f64::from(single_layout.y + TEXT_INPUT_INSET),
    ));
    let single_second = Some(PhysicalPosition::new(
        f64::from(
            single_layout.x + TEXT_INPUT_INSET + ApproximateDialogTextMeasurer.width("one two th"),
        ),
        f64::from(single_layout.y + TEXT_INPUT_INSET),
    ));
    assert!(handle_dialog_mouse_input_with_modifiers_and_measurer(
        &mut input,
        &mut counters,
        &mut single,
        &None,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        single_first,
        surface_size,
        false,
        false,
        &ApproximateDialogTextMeasurer,
    ));
    assert_eq!(input.dialog_text_edit_state().unwrap().cursor, 5);
    assert!(handle_dialog_mouse_input_with_modifiers_and_measurer(
        &mut input,
        &mut counters,
        &mut single,
        &None,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        single_second,
        surface_size,
        true,
        true,
        &ApproximateDialogTextMeasurer,
    ));
    assert_eq!(
        input.dialog_text_edit_state(),
        Some(crate::input::dialog::DialogTextEditState {
            input_index: 0,
            cursor: 13,
            selection: 8,
            scroll_amount: 0,
            ..Default::default()
        })
    );
    assert_eq!(input.dialog_dragging_text_input(), Some(0));

    let mut multiline = world_with_notice(
        &mut input,
        vec![entry(
            "inputs",
            NbtSummaryValue::List(vec![multiline_text_input(
                "notes",
                "Notes",
                120,
                false,
                "one two\nthree",
                64,
                None,
                Some(30),
            )]),
        )],
    );
    let multiline_layout = {
        let controls = input
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, multiline.server_links()))
            .unwrap();
        let layout = dialog_input_layouts(&controls, surface_size).unwrap()[0];
        dialog_multiline_text_box_layout(layout, false)
    };
    let multiline_first = Some(PhysicalPosition::new(
        f64::from(
            multiline_layout.x + TEXT_INPUT_INSET + ApproximateDialogTextMeasurer.width("one t"),
        ),
        f64::from(multiline_layout.y + TEXT_INPUT_INSET),
    ));
    let multiline_second = Some(PhysicalPosition::new(
        f64::from(
            multiline_layout.x + TEXT_INPUT_INSET + ApproximateDialogTextMeasurer.width("thr"),
        ),
        f64::from(multiline_layout.y + TEXT_INPUT_INSET + TEXT_LINE_HEIGHT),
    ));
    assert!(handle_dialog_mouse_input_with_modifiers_and_measurer(
        &mut input,
        &mut counters,
        &mut multiline,
        &None,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        multiline_first,
        surface_size,
        false,
        false,
        &ApproximateDialogTextMeasurer,
    ));
    assert_eq!(input.dialog_text_edit_state().unwrap().cursor, 5);
    assert!(handle_dialog_mouse_input_with_modifiers_and_measurer(
        &mut input,
        &mut counters,
        &mut multiline,
        &None,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        multiline_second,
        surface_size,
        true,
        true,
        &ApproximateDialogTextMeasurer,
    ));
    assert_eq!(
        input.dialog_text_edit_state(),
        Some(crate::input::dialog::DialogTextEditState {
            input_index: 0,
            cursor: 7,
            selection: 4,
            scroll_amount: 0,
            ..Default::default()
        })
    );
    assert_eq!(input.dialog_dragging_text_input(), Some(0));
}

#[test]
fn dialog_multiline_wrap_matches_renderer_with_zero_negative_and_fractional_advances() {
    use bbb_client_render::{HudAsciiGlyph, HudFontGlyphMap};
    let mut glyphs = HudFontGlyphMap::new();
    for (ch, advance) in [('z', 0.0), ('w', 120.0), ('a', 57.0), ('n', -0.5)] {
        glyphs.insert_first_wins(ch, HudAsciiGlyph::empty(advance));
    }
    let lines = dialog_multiline_lines("zzwa\n", 114, &glyphs);
    assert_eq!(
        lines
            .iter()
            .map(|line| (line.begin, line.end))
            .collect::<Vec<_>>(),
        [(0, 3), (3, 4), (5, 5)]
    );
    assert_eq!(dialog_multiline_lines("naa", 114, &glyphs).len(), 1);
    assert_eq!(dialog_multiline_lines("", 0, &glyphs).len(), 1);
}

fn dialog_text_key(
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    code: KeyCode,
) -> bool {
    handle_focused_dialog_text_key(
        input,
        world,
        &mut None,
        PhysicalKey::Code(code),
        false,
        false,
        &ApproximateDialogTextMeasurer,
    )
}

#[test]
fn raw_initial_text_edits_units_without_reconstructing_display() {
    let mut input = crate::input::ClientInputState::new(true);
    for units in [
        vec![0xd800],
        vec![0xdc00],
        vec![0xd83d, 0xde00],
        vec![0xd800, 65, 0xdc00],
    ] {
        for multiline in [false, true] {
            let raw = bbb_protocol::JavaString::from(units.clone());
            let definition = if multiline {
                multiline_text_input("text", "Text", 100, false, raw.clone(), 32, None, None)
            } else {
                text_input("text", "Text", 100, false, raw.clone(), 32)
            };
            let mut world = world_with_notice(
                &mut input,
                vec![entry("inputs", NbtSummaryValue::List(vec![definition]))],
            );
            assert_eq!(input.current_dialog().unwrap().semantic_error, None);
            let bound = bound_dialog_input_values(&input, &world).unwrap();
            assert_eq!(bound["text"].tag, raw.to_nbt());
            assert_eq!(bound["text"].template_substitution, raw);
            assert!(input.focus_dialog_text_input(0, units.len(), false));
            assert!(handle_dialog_text_input(&mut input, &mut world, "Z"));
            let mut expected = units.clone();
            expected.push(90);
            assert_eq!(
                bound_dialog_input_values(&input, &world).unwrap()["text"].tag,
                bbb_protocol::JavaString::from(expected.clone()).to_nbt()
            );
            assert!(dialog_text_key(&mut input, &mut world, KeyCode::Backspace));
            assert_eq!(
                bound_dialog_input_values(&input, &world).unwrap()["text"].tag,
                raw.to_nbt()
            );
            assert!(input.set_dialog_text_input_raw_value_and_selection(0, raw.clone(), 0, 0));
            assert!(handle_dialog_text_input(&mut input, &mut world, "A"));
            expected = vec![65];
            expected.extend_from_slice(&units);
            assert_eq!(
                bound_dialog_input_values(&input, &world).unwrap()["text"].tag,
                bbb_protocol::JavaString::from(expected).to_nbt()
            );
            assert!(input.set_dialog_text_input_raw_value_and_selection(0, raw.clone(), 1, 0));
            assert!(dialog_text_key(&mut input, &mut world, KeyCode::Backspace));
            assert_eq!(
                bound_dialog_input_values(&input, &world).unwrap()["text"].tag,
                bbb_protocol::JavaString::from(units[1..].to_vec()).to_nbt()
            );
        }
    }
}

#[test]
fn single_and_multiline_follow_native_pair_steps_and_insertion_limits() {
    let mut input = crate::input::ClientInputState::new(true);
    let pair = bbb_protocol::JavaString::from(vec![0xd83d, 0xde00]);
    for multiline in [false, true] {
        let definition = if multiline {
            multiline_text_input("text", "Text", 100, false, pair.clone(), 3, None, None)
        } else {
            text_input("text", "Text", 100, false, pair.clone(), 3)
        };
        let mut world = world_with_notice(
            &mut input,
            vec![entry("inputs", NbtSummaryValue::List(vec![definition]))],
        );
        assert!(input.focus_dialog_text_input(0, 2, false));
        assert!(dialog_text_key(&mut input, &mut world, KeyCode::ArrowLeft));
        assert_eq!(
            input.dialog_text_edit_state().unwrap().cursor,
            if multiline { 1 } else { 0 }
        );
        assert!(dialog_text_key(&mut input, &mut world, KeyCode::ArrowRight));
        assert_eq!(input.dialog_text_edit_state().unwrap().cursor, 2);
        assert!(dialog_text_key(&mut input, &mut world, KeyCode::Backspace));
        assert_eq!(
            bound_dialog_input_values(&input, &world).unwrap()["text"].tag,
            bbb_protocol::JavaString::from(if multiline { vec![0xd83d] } else { vec![] }).to_nbt()
        );
        assert!(input.set_dialog_text_input_raw_value_and_selection(0, "AB".into(), 2, 0));
        assert!(handle_dialog_text_input(&mut input, &mut world, "😀"));
        assert_eq!(
            bound_dialog_input_values(&input, &world).unwrap()["text"].tag,
            bbb_protocol::JavaString::from(if multiline {
                vec![0xd83d]
            } else {
                pair.units().to_vec()
            })
            .to_nbt()
        );
        assert!(input.set_dialog_text_input_raw_value_and_selection(0, "ABC".into(), 3, 0));
        assert!(handle_dialog_text_input(&mut input, &mut world, "Z"));
        assert_eq!(
            bound_dialog_input_values(&input, &world).unwrap()["text"].tag,
            bbb_protocol::JavaString::from(if multiline { "" } else { "Z" }).to_nbt()
        );
    }
    let raw = bbb_protocol::JavaString::from(vec![0xd800, 0x61, 0xd83d, 0xde00, 0xdc00]);
    let projection = DialogTextRaw::new(&raw);
    assert_eq!(projection.display, "�a😀�");
    assert_eq!(projection.unit_cursor(3), 4);
    assert_eq!(
        projection.cursor_for_width(0, 18, &ApproximateDialogTextMeasurer),
        4
    );
    assert_eq!(dialog_char_slice(&raw, 2, 3), "�");
}
