use super::*;
use crate::input::dialog::DialogBodyFocusPartState;
use crate::input::dialog::DialogFocusState;
use crate::input::dialog::DialogLinkConfirmationState;

#[test]
fn dialog_item_body_tooltip_uses_potion_name_from_show_dialog_components() {
    let mut input = crate::input::ClientInputState::new(true);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("bbb-dialog-potion-{nanos}"));
    crate::item_registry_test_fixture::write_item_registry_fixture(&root, &["potion"]);
    let assets = root
        .join("sources")
        .join(bbb_client_assets::MC_VERSION)
        .join("assets/minecraft");
    for (relative, contents) in [
        (
            "atlases/items.json",
            r#"{"sources":[{"type":"minecraft:directory","prefix":"item/","source":"item"}]}"#,
        ),
        (
            "atlases/blocks.json",
            r#"{"sources":[{"type":"minecraft:directory","prefix":"block/","source":"block"}]}"#,
        ),
        (
            "items/potion.json",
            r#"{"model":{"type":"minecraft:model","model":"minecraft:item/potion"}}"#,
        ),
        (
            "models/item/potion.json",
            r#"{"textures":{"layer0":"minecraft:item/potion"}}"#,
        ),
        (
            "lang/en_us.json",
            r#"{"item.minecraft.potion.effect.swiftness":"Dialog Speed"}"#,
        ),
    ] {
        let path = assets.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }
    let texture = assets.join("textures/item/potion.png");
    std::fs::create_dir_all(texture.parent().unwrap()).unwrap();
    image::save_buffer(texture, &[80, 120, 160, 255], 1, 1, image::ColorType::Rgba8).unwrap();
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap())
            .unwrap()
            .with_vanilla_item_registry_for_test();
    let world = world_with_notice(
        &mut input,
        vec![entry(
            "body",
            NbtSummaryValue::Compound(vec![
                entry("type", NbtSummaryValue::String("minecraft:item".into())),
                entry(
                    "item",
                    NbtSummaryValue::Compound(vec![
                        entry("id", NbtSummaryValue::String("minecraft:potion".into())),
                        entry(
                            "components",
                            NbtSummaryValue::Compound(vec![entry(
                                "minecraft:potion_contents",
                                NbtSummaryValue::Compound(vec![entry(
                                    "potion",
                                    NbtSummaryValue::String("minecraft:long_swiftness".into()),
                                )]),
                            )]),
                        ),
                    ]),
                ),
                entry("width", NbtSummaryValue::Int(32)),
                entry("height", NbtSummaryValue::Int(48)),
            ]),
        )],
    );
    assert!(input.current_dialog().unwrap().semantic_error.is_none());
    let preview = hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        Some(&item_runtime),
        &TerrainTextureState::default(),
        &DialogHudLocalState::default(),
        PhysicalSize::new(320, 240),
        0.0,
    )
    .unwrap();
    assert_eq!(preview.floating_items.len(), 1);
    let item = &preview.floating_items[0];
    let cursor_x = (320 - preview.width) as i32 / 2 + item.x + 8;
    let cursor_y = (240 - preview.height) as i32 / 2 + item.y + 8;
    let screen = hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        Some(&item_runtime),
        &TerrainTextureState::default(),
        &DialogHudLocalState {
            cursor_position: Some((cursor_x, cursor_y)),
            ..Default::default()
        },
        PhysicalSize::new(320, 240),
        0.0,
    )
    .unwrap();
    assert_eq!(screen.tooltip.unwrap().lines[0].text, "Dialog Speed");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn dialog_item_body_mouse_focus_uses_only_the_declared_widget_rect() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(
        &mut input,
        vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::string("none")),
            entry(
                "body",
                NbtSummaryValue::Compound(vec![
                    entry("type", NbtSummaryValue::string("minecraft:item")),
                    entry(
                        "item",
                        NbtSummaryValue::Compound(vec![entry(
                            "id",
                            NbtSummaryValue::string("minecraft:stick"),
                        )]),
                    ),
                    entry(
                        "description",
                        NbtSummaryValue::Compound(vec![
                            entry("contents", NbtSummaryValue::string("Info")),
                            entry("width", NbtSummaryValue::Int(50)),
                        ]),
                    ),
                    entry("width", NbtSummaryValue::Int(32)),
                    entry("height", NbtSummaryValue::Int(48)),
                ]),
            ),
        ],
    );
    let surface_size = PhysicalSize::new(320, 240);
    let dialog = input.current_dialog().unwrap();
    let controls = dialog
        .data
        .as_ref()
        .and_then(|data| dialog_controls(data, world.server_links()))
        .unwrap_or_else(|| panic!("dialog decode failed: {:?}", dialog.semantic_error));
    let item_focus = dialog_focus_node(
        &dialog_focus_graph(&controls, surface_size).unwrap().body,
        DialogFocusState::Body {
            index: 0,
            part: DialogBodyFocusPartState::Primary,
        },
    )
    .unwrap();
    assert_eq!(
        item_focus.rectangle,
        DialogFocusRectangle {
            x: 118,
            y: 63,
            width: 32,
            height: 48,
        }
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
        Some(PhysicalPosition::new(149.0, 110.0)),
        surface_size,
    ));
    assert_eq!(input.dialog_focused_body(), Some(0));

    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(150.0, 64.0)),
        surface_size,
    ));
    assert_eq!(input.dialog_focused_body(), Some(0));

    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(160.0, 220.0)),
        surface_size,
    ));
    assert_eq!(input.dialog_focused_body(), None);
    assert!(input.current_dialog().is_some());
}

#[test]
fn plain_message_component_click_requires_a_clickable_glyph() {
    let mut input = crate::input::ClientInputState::new(true);
    let event = ComponentClickEvent::CopyToClipboard {
        value: "copied value".to_string().into(),
    };
    let mut world = world_with_notice(
        &mut input,
        vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::string("none")),
            entry(
                "body",
                plain_message_body(
                    clickable_component(
                        "Copy",
                        "copy_to_clipboard",
                        vec![entry("value", NbtSummaryValue::string("copied value"))],
                    ),
                    80,
                ),
            ),
        ],
    );
    let DialogBodyState::PlainMessage { message } = &input
        .current_dialog()
        .unwrap()
        .data
        .as_ref()
        .and_then(|data| dialog_controls(data, world.server_links()))
        .unwrap()
        .common
        .body_data[0]
    else {
        panic!("expected plain-message body");
    };
    assert_eq!(
        dialog_text_block(message).unwrap().lines[0].click_spans,
        [DialogTextClickSpan {
            x: 0,
            width: 24,
            event,
        }]
    );
    let mut clipboard = MockDialogClipboard::default();
    let mut counters = NetCounters::default();
    let surface_size = PhysicalSize::new(320, 240);

    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        Some(&mut clipboard),
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(121.0, 64.0)),
        surface_size,
    ));
    assert_eq!(clipboard.text, None);
    assert_eq!(
        input.dialog_focus(),
        Some(DialogFocusState::Body {
            index: 0,
            part: DialogBodyFocusPartState::Primary,
        })
    );

    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        Some(&mut clipboard),
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(149.0, 68.0)),
        surface_size,
    ));
    assert_eq!(clipboard.text.as_deref(), Some("copied value"));
    assert!(input.current_dialog().is_some());
}

#[test]
fn item_description_component_click_opens_untrusted_url_prompt() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(
        &mut input,
        vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::string("none")),
            entry(
                "body",
                NbtSummaryValue::Compound(vec![
                    entry("type", NbtSummaryValue::string("minecraft:item")),
                    entry(
                        "item",
                        NbtSummaryValue::Compound(vec![entry(
                            "id",
                            NbtSummaryValue::string("minecraft:stick"),
                        )]),
                    ),
                    entry(
                        "description",
                        NbtSummaryValue::Compound(vec![
                            entry(
                                "contents",
                                clickable_component(
                                    "Info",
                                    "open_url",
                                    vec![entry(
                                        "url",
                                        NbtSummaryValue::string("https://example.invalid/info"),
                                    )],
                                ),
                            ),
                            entry("width", NbtSummaryValue::Int(50)),
                        ]),
                    ),
                    entry("width", NbtSummaryValue::Int(32)),
                    entry("height", NbtSummaryValue::Int(48)),
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
        Some(PhysicalPosition::new(166.0, 84.0)),
        PhysicalSize::new(320, 240),
    ));

    assert_eq!(
        input.dialog_link_confirmation(),
        Some(&DialogLinkConfirmationState {
            focused_button: None,
            url: "https://example.invalid/info".into(),
            after_action: "none".to_string(),
        })
    );
}

#[test]
fn plain_message_run_command_click_applies_parent_after_action() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(
        &mut input,
        vec![
            entry("after_action", NbtSummaryValue::string("wait_for_response")),
            entry(
                "body",
                plain_message_body(
                    clickable_component(
                        "Run",
                        "run_command",
                        vec![entry("command", NbtSummaryValue::string("/say component"))],
                    ),
                    80,
                ),
            ),
        ],
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
        Some(PhysicalPosition::new(152.0, 68.0)),
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
    assert_eq!(counters.chat_command_commands_queued, 1);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::ChatCommand(command) if command.command == "say component"
    ));
    assert_eq!(input.dialog_waiting_ticks(), Some(0));
}

#[test]
fn plain_message_show_dialog_click_replaces_parent_without_packet_count() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = WorldStore::new();
    input.set_screen_for_test(crate::input::Screen::Advancements(Default::default()));
    apply_notice(
        &mut input,
        &mut world,
        vec![
            entry("after_action", NbtSummaryValue::string("wait_for_response")),
            entry(
                "body",
                plain_message_body(
                    clickable_component(
                        "Show",
                        "show_dialog",
                        vec![entry("dialog", notice_dialog("Inner", None))],
                    ),
                    80,
                ),
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
        Some(PhysicalPosition::new(149.0, 68.0)),
        PhysicalSize::new(320, 240),
    ));

    assert!(matches!(
        input.current_dialog().and_then(|dialog| dialog.data.as_ref()),
        Some(DialogDataState::Notice { common, .. }) if common.title.text == "Inner"
    ));
    assert_eq!(world.counters().show_dialog_packets, 1);
    assert!(input.close_dialog());
    input.apply_screen_requests(&mut world, &mut counters, &None);
    assert!(input.advancements_screen_is_open());
}

#[test]
fn actionless_notice_button_closes_without_counting_clear_packet() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(&mut input, Vec::new());
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

    assert!(input.current_dialog().is_none());
    assert_eq!(world.counters().clear_dialog_packets, 0);
}

#[test]
fn notice_button_honors_none_after_action() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(
        &mut input,
        vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::string("none")),
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

    assert!(input.current_dialog().is_some());
}

#[test]
fn notice_escape_requires_permission_and_forces_close_after_cancel_action() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut default_notice = world_with_notice(&mut input, Vec::new());
    let mut counters = NetCounters::default();
    assert!(handle_dialog_key_input(
        &mut input,
        &mut counters,
        &mut default_notice,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    ));
    assert!(input.current_dialog().is_none());

    let mut locked_notice = world_with_notice(
        &mut input,
        vec![entry("can_close_with_escape", NbtSummaryValue::Byte(0))],
    );
    assert!(handle_dialog_key_input(
        &mut input,
        &mut counters,
        &mut locked_notice,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    ));
    assert!(input.current_dialog().is_some());

    let mut action_notice = world_with_notice(
        &mut input,
        vec![entry(
            "action",
            NbtSummaryValue::Compound(vec![
                entry("label", NbtSummaryValue::string("Send")),
                entry(
                    "action",
                    NbtSummaryValue::Compound(vec![
                        entry("type", NbtSummaryValue::string("minecraft:run_command")),
                        entry("command", NbtSummaryValue::string("say deferred")),
                    ]),
                ),
            ]),
        )],
    );
    assert!(handle_dialog_key_input(
        &mut input,
        &mut counters,
        &mut action_notice,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    ));
    assert!(input.current_dialog().is_some());
    assert!(input.command_confirmation().is_some());
    input.resolve_command_confirmation(
        true,
        &mut action_notice,
        &mut counters,
        &None,
        &crate::input::FallbackTextMeasurer,
    );
    assert!(input.current_dialog().is_none());
    assert_eq!(counters.chat_command_commands_queued, 0);
}

#[test]
fn confirmation_projects_two_buttons_and_escape_runs_no_button_action() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_confirmation(
        &mut input,
        vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::string("none")),
            entry(
                "yes",
                custom_button("Yes", Some("Proceed"), 80, "example:yes", None),
            ),
            entry("no", custom_button("No", None, 100, "example:no", None)),
        ],
    );
    let terrain_textures = TerrainTextureState::default();
    let screen = hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &terrain_textures,
        &DialogHudLocalState {
            cursor_position: Some((100, 220)),
            ..Default::default()
        },
        PhysicalSize::new(320, 240),
        0.0,
    )
    .unwrap();

    assert_eq!(screen.background_layers.len(), 4);
    assert_eq!(screen.background_layers[0].x, 0);
    assert_eq!(screen.background_layers[0].width, 320);
    assert_eq!(
        screen.background_layers[0].texture,
        HudInventoryBackgroundTexture::Texture("textures/gui/inworld_menu_background.png")
    );
    assert_eq!(screen.background_layers[0].uv.max, [10.0, 7.5]);
    assert_eq!(screen.background_layers[1].x, 176);
    assert_eq!(screen.background_layers[1].width, 20);
    assert_eq!(
        screen.background_layers[1].texture,
        HudInventoryBackgroundTexture::Sprite("dialog/warning_button")
    );
    assert_eq!(screen.background_layers[2].x, 66);
    assert_eq!(screen.background_layers[2].width, 80);
    assert_eq!(
        screen.background_layers[2].texture,
        HudInventoryBackgroundTexture::Sprite("widget/button_highlighted")
    );
    assert_eq!(screen.background_layers[3].x, 154);
    assert_eq!(screen.background_layers[3].width, 100);
    assert_eq!(screen.text_labels[1].text, "Yes");
    assert_eq!(screen.text_labels[2].text, "No");
    let tooltip = screen.tooltip.as_ref().unwrap();
    assert_eq!((tooltip.x, tooltip.y), (100, 220));
    assert_eq!(tooltip.lines.len(), 1);
    assert_eq!(tooltip.lines[0].text, "Proceed");

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
        Some(PhysicalPosition::new(100.0, 220.0)),
        PhysicalSize::new(320, 240),
    ));
    assert!(input.current_dialog().is_some());
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::CustomClickAction(CustomClickAction { id, .. }) if id == "example:yes"
    ));

    assert!(handle_dialog_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    ));
    assert!(input.current_dialog().is_none());
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::CustomClickAction(CustomClickAction { id, .. }) if id == "example:no"
    ));
    assert_eq!(counters.custom_click_action_commands_queued, 2);
}

#[test]
fn multi_action_projects_scrolled_button_grid_focus_tooltips_and_exit() {
    let mut input = crate::input::ClientInputState::new(true);
    let dynamic_button = NbtSummaryValue::Compound(vec![
        entry("label", NbtSummaryValue::string("One")),
        entry("tooltip", NbtSummaryValue::string("First tip")),
        entry("width", NbtSummaryValue::Int(80)),
        entry(
            "action",
            NbtSummaryValue::Compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:dynamic/custom")),
                entry("id", NbtSummaryValue::string("example:one")),
            ]),
        ),
    ]);
    let mut world = world_with_multi_action(
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
                "actions",
                NbtSummaryValue::List(vec![
                    dynamic_button,
                    custom_button("Two", None, 100, "example:two", None),
                    custom_button("Three", None, 60, "example:three", None),
                    custom_button("Four", Some("Fourth tip"), 60, "example:four", None),
                    custom_button("Five", None, 120, "example:five", None),
                ]),
            ),
            entry("columns", NbtSummaryValue::Int(3)),
            entry(
                "exit_action",
                custom_button("Exit", Some("Exit tip"), 150, "example:exit", None),
            ),
        ],
    );
    let surface_size = PhysicalSize::new(320, 240);
    let controls = input
        .current_dialog()
        .and_then(|dialog| dialog.data.as_ref())
        .and_then(|data| dialog_controls(data, world.server_links()))
        .unwrap();
    assert_eq!(controls.body_action_count, 5);
    assert_eq!(controls.footer_height, FOOTER_HEIGHT);
    let grid = dialog_body_action_grid_layout(&controls).unwrap();
    assert_eq!((grid.width, grid.height), (244, 42));
    assert_eq!(
        grid.buttons,
        vec![
            DialogButtonLayout {
                x: 0,
                y: 0,
                width: 80,
            },
            DialogButtonLayout {
                x: 82,
                y: 0,
                width: 100,
            },
            DialogButtonLayout {
                x: 184,
                y: 0,
                width: 60,
            },
            DialogButtonLayout {
                x: 31,
                y: 22,
                width: 60,
            },
            DialogButtonLayout {
                x: 93,
                y: 22,
                width: 120,
            },
        ]
    );
    let graph = dialog_focus_graph(&controls, surface_size).unwrap();
    assert_eq!(
        graph.tab,
        vec![
            DialogFocusState::WarningButton,
            DialogFocusState::Input { index: 0 },
            DialogFocusState::Action { index: 0 },
            DialogFocusState::Action { index: 1 },
            DialogFocusState::Action { index: 2 },
            DialogFocusState::Action { index: 3 },
            DialogFocusState::Action { index: 4 },
            DialogFocusState::Action { index: 5 },
        ]
    );

    let screen = hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &TerrainTextureState::default(),
        &DialogHudLocalState {
            cursor_position: Some((70, 113)),
            ..Default::default()
        },
        surface_size,
        0.0,
    )
    .unwrap();
    let scroll_area = screen.scroll_area.as_ref().unwrap();
    assert_eq!(
        scroll_area
            .background_layers
            .iter()
            .filter(|layer| matches!(
                layer.texture,
                HudInventoryBackgroundTexture::Sprite("widget/button")
                    | HudInventoryBackgroundTexture::Sprite("widget/button_highlighted")
            ))
            .count(),
        15
    );
    let four = scroll_area
        .text_labels
        .iter()
        .find(|label| label.text == "Four")
        .unwrap();
    assert_eq!(four.y, 118);
    assert_eq!(four.x + i32::try_from(four.width).unwrap() / 2, 99);
    assert_eq!(screen.background_layers[2].x, 85);
    assert_eq!(screen.background_layers[2].y, 213);
    assert_eq!(screen.background_layers[2].width, 150);
    assert_eq!(screen.tooltip.as_ref().unwrap().lines[0].text, "Fourth tip");

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
        Some(PhysicalPosition::new(130.0, 64.0)),
        surface_size,
    ));
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(39.0, 91.0)),
        surface_size,
    ));
    assert_eq!(
        input.dialog_focus(),
        Some(DialogFocusState::Action { index: 0 })
    );
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::CustomClickAction(CustomClickAction {
            id,
            payload: Some(NbtSummaryValue::Compound(entries)),
        }) if id == "example:one"
            && entries == vec![entry("enabled", NbtSummaryValue::Byte(1))]
    ));
    assert!(input.current_dialog().is_some());

    assert!(handle_dialog_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    ));
    assert!(input.current_dialog().is_none());
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::CustomClickAction(CustomClickAction { id, .. }) if id == "example:exit"
    ));
    assert_eq!(counters.custom_click_action_commands_queued, 2);
}

#[test]
fn multi_action_without_exit_uses_five_pixel_footer_and_reveals_grid_focus() {
    let mut input = crate::input::ClientInputState::new(true);
    let actions = (0..9)
        .map(|index| {
            custom_button(
                &format!("Action {index}"),
                None,
                100,
                &format!("example:{index}"),
                None,
            )
        })
        .collect::<Vec<_>>();
    let mut world = world_with_multi_action(
        &mut input,
        vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::string("none")),
            entry("actions", NbtSummaryValue::List(actions)),
            entry("columns", NbtSummaryValue::Int(1)),
        ],
    );
    let surface_size = PhysicalSize::new(320, 120);
    let dialog = input.current_dialog().unwrap();
    let controls = dialog
        .data
        .as_ref()
        .and_then(|data| dialog_controls(data, world.server_links()))
        .unwrap_or_else(|| panic!("invalid multi-action dialog: {:?}", dialog.semantic_error));
    assert_eq!(controls.footer_height, 5);
    assert!(controls.footer_actions().is_empty());
    let scroll = dialog_body_scroll_layout(&controls, surface_size, 0.0).unwrap();
    assert_eq!(scroll.rectangle.height, 82);
    assert_eq!(scroll.content_height, 196);
    assert_eq!(scroll.max_scroll, 114);

    assert!(input.focus_dialog_navigation_target(
        &world,
        DialogFocusState::Action { index: 8 },
        true
    ));
    assert!(reveal_dialog_keyboard_focus(
        &mut input,
        &mut world,
        DialogFocusState::Action { index: 8 },
        surface_size,
    ));
    assert_eq!(input.dialog_body_scroll_amount(), 114.0);

    let mut counters = NetCounters::default();
    assert!(handle_dialog_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    ));
    assert!(input.current_dialog().is_none());
}

#[test]
fn multi_action_large_column_count_preserves_partial_row_without_large_allocation() {
    let mut input = crate::input::ClientInputState::new(true);
    let world = world_with_multi_action(
        &mut input,
        vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::string("none")),
            entry(
                "actions",
                NbtSummaryValue::List(vec![
                    custom_button("A", None, 1, "example:a", None),
                    custom_button("B", None, 1, "example:b", None),
                ]),
            ),
            entry("columns", NbtSummaryValue::Int(100)),
        ],
    );
    let controls = input
        .current_dialog()
        .and_then(|dialog| dialog.data.as_ref())
        .and_then(|data| dialog_controls(data, world.server_links()))
        .unwrap();
    assert_eq!(
        dialog_body_action_grid_layout(&controls).unwrap(),
        DialogBodyActionGridLayout {
            width: 198,
            height: 20,
            buttons: vec![
                DialogButtonLayout {
                    x: 97,
                    y: 0,
                    width: 1,
                },
                DialogButtonLayout {
                    x: 100,
                    y: 0,
                    width: 1,
                },
            ],
        }
    );
}

#[test]
fn dialog_list_projects_external_titles_uniform_grid_and_opens_direct_entry() {
    let mut input = crate::input::ClientInputState::new(true);
    let targets = (0..5)
        .map(|index| {
            notice_dialog(
                &format!("Internal {index}"),
                Some(&format!("External {index}")),
            )
        })
        .collect::<Vec<_>>();
    let mut world = world_with_dialog_list(
        &mut input,
        vec![
            entry("dialogs", NbtSummaryValue::List(targets)),
            entry("columns", NbtSummaryValue::Int(3)),
            entry("button_width", NbtSummaryValue::Int(70)),
            entry(
                "exit_action",
                custom_button("Back", Some("Exit tip"), 90, "example:back", None),
            ),
        ],
        false,
    );
    let surface_size = PhysicalSize::new(320, 240);
    let controls = input
        .current_dialog()
        .and_then(|dialog| dialog.data.as_ref())
        .and_then(|data| dialog_controls(data, world.server_links()))
        .unwrap();
    assert_eq!(controls.body_action_count, 5);
    assert_eq!(controls.footer_height, FOOTER_HEIGHT);
    assert!(controls
        .body_actions()
        .iter()
        .all(|action| action.width() == 70 && action.tooltip().is_none()));
    assert_eq!(controls.body_actions()[3].label_text(), "External 3");
    assert_eq!(
        dialog_body_action_grid_layout(&controls).unwrap(),
        DialogBodyActionGridLayout {
            width: 214,
            height: 42,
            buttons: vec![
                DialogButtonLayout {
                    x: 0,
                    y: 0,
                    width: 70,
                },
                DialogButtonLayout {
                    x: 72,
                    y: 0,
                    width: 70,
                },
                DialogButtonLayout {
                    x: 144,
                    y: 0,
                    width: 70,
                },
                DialogButtonLayout {
                    x: 36,
                    y: 22,
                    width: 70,
                },
                DialogButtonLayout {
                    x: 108,
                    y: 22,
                    width: 70,
                },
            ],
        }
    );
    assert_eq!(
        dialog_focus_graph(&controls, surface_size).unwrap().tab,
        vec![
            DialogFocusState::WarningButton,
            DialogFocusState::Action { index: 0 },
            DialogFocusState::Action { index: 1 },
            DialogFocusState::Action { index: 2 },
            DialogFocusState::Action { index: 3 },
            DialogFocusState::Action { index: 4 },
            DialogFocusState::Action { index: 5 },
        ]
    );

    let screen = hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &TerrainTextureState::default(),
        &DialogHudLocalState {
            cursor_position: Some((54, 64)),
            ..Default::default()
        },
        surface_size,
        0.0,
    )
    .unwrap();
    let scroll_area = screen.scroll_area.as_ref().unwrap();
    assert!(scroll_area
        .text_labels
        .iter()
        .any(|label| label.text == "External 0"));
    assert_eq!(
        scroll_area
            .background_layers
            .iter()
            .filter(|layer| matches!(
                layer.texture,
                HudInventoryBackgroundTexture::Sprite("widget/button")
                    | HudInventoryBackgroundTexture::Sprite("widget/button_highlighted")
            ))
            .count(),
        15
    );
    assert_eq!(screen.tooltip, None);

    let mut counters = NetCounters::default();
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(54.0, 64.0)),
        surface_size,
    ));
    let target = input.current_dialog().unwrap();
    assert_eq!(target.holder_kind, "direct");
    assert!(matches!(
        target.data,
        Some(DialogDataState::Notice { ref common, .. }) if common.title.text == "Internal 0"
    ));
    assert_eq!(world.counters().show_dialog_packets, 1);
    assert_eq!(input.dialog_focus(), None);
    assert_eq!(input.dialog_body_scroll_amount(), 0.0);
}

#[test]
fn context_free_dialog_list_without_exit_reveals_and_opens_entry() {
    let mut input = crate::input::ClientInputState::new(true);
    let targets = (0..9)
        .map(|index| notice_dialog(&format!("Target {index}"), None))
        .collect::<Vec<_>>();
    let mut world = world_with_dialog_list(
        &mut input,
        vec![
            entry("dialogs", NbtSummaryValue::List(targets)),
            entry("columns", NbtSummaryValue::Int(1)),
            entry("button_width", NbtSummaryValue::Int(100)),
        ],
        true,
    );
    let surface_size = PhysicalSize::new(320, 120);
    let controls = input
        .current_dialog()
        .and_then(|dialog| dialog.data.as_ref())
        .and_then(|data| dialog_controls(data, world.server_links()))
        .unwrap();
    assert_eq!(controls.footer_height, 5);
    assert!(controls.footer_actions().is_empty());
    let scroll = dialog_body_scroll_layout(&controls, surface_size, 0.0).unwrap();
    assert_eq!(scroll.rectangle.height, 82);
    assert_eq!(scroll.content_height, 196);
    assert_eq!(scroll.max_scroll, 114);
    let graph = dialog_focus_graph(&controls, surface_size).unwrap();
    let DialogTopFocusNode::Body(container) = graph.top[0] else {
        panic!("body container must lead the top-level focus graph");
    };
    assert_eq!(container.rectangle.height, 82);

    let target = DialogFocusState::Action { index: 8 };
    assert!(input.focus_dialog_navigation_target(&world, target, true));
    assert!(reveal_dialog_keyboard_focus(
        &mut input,
        &mut world,
        target,
        surface_size,
    ));
    assert_eq!(input.dialog_body_scroll_amount(), 114.0);
    let mut counters = NetCounters::default();
    assert!(handle_dialog_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    ));
    let target = input.current_dialog().unwrap();
    assert_eq!(target.holder_kind, "context_free");
    assert!(matches!(
        target.data,
        Some(DialogDataState::Notice { ref common, .. }) if common.title.text == "Target 8"
    ));
}

#[test]
fn server_links_dialog_projects_canonical_links_and_generated_open_url_actions() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_server_links_dialog(
        &mut input,
        vec![
            ServerLinkEntry {
                link_type: ServerLinkType::Known(ServerLinkKnownType::Support),
                url: "https://example.invalid/support".to_string(),
            },
            ServerLinkEntry {
                link_type: ServerLinkType::Known(ServerLinkKnownType::BugReport),
                url: "https://example.invalid/bugs".to_string(),
            },
            ServerLinkEntry {
                link_type: ServerLinkType::Custom {
                    label: "Rules".to_string(),
                },
                url: "http://example.invalid/rules".to_string(),
            },
            ServerLinkEntry {
                link_type: ServerLinkType::Known(ServerLinkKnownType::Website),
                url: "ftp://example.invalid/rejected".to_string(),
            },
        ],
        vec![
            entry("columns", NbtSummaryValue::Int(2)),
            entry("button_width", NbtSummaryValue::Int(90)),
            entry(
                "exit_action",
                custom_button("Back", Some("Exit tip"), 100, "example:back", None),
            ),
        ],
    );
    assert_eq!(world.counters().server_link_invalid_entries, 1);
    let surface_size = PhysicalSize::new(320, 240);
    let controls = input
        .current_dialog()
        .and_then(|dialog| dialog.data.as_ref())
        .and_then(|data| dialog_controls(data, world.server_links()))
        .unwrap();
    assert_eq!(controls.body_action_count, 3);
    assert_eq!(controls.footer_height, FOOTER_HEIGHT);
    assert_eq!(
        controls
            .body_actions()
            .iter()
            .map(|action| action.label_text())
            .collect::<Vec<_>>(),
        vec!["Support", "Report Server Bug", "Rules"]
    );
    assert!(controls
        .body_actions()
        .iter()
        .all(|action| action.width() == 90 && action.tooltip().is_none()));
    assert!(matches!(
        controls.body_actions()[0].activation(),
        DialogControlActivation::Action(Some(DialogActionState::OpenUrl { url }))
            if url == "https://example.invalid/support"
    ));
    assert_eq!(
        dialog_body_action_grid_layout(&controls).unwrap(),
        DialogBodyActionGridLayout {
            width: 182,
            height: 42,
            buttons: vec![
                DialogButtonLayout {
                    x: 0,
                    y: 0,
                    width: 90,
                },
                DialogButtonLayout {
                    x: 92,
                    y: 0,
                    width: 90,
                },
                DialogButtonLayout {
                    x: 46,
                    y: 22,
                    width: 90,
                },
            ],
        }
    );
    assert_eq!(
        dialog_focus_graph(&controls, surface_size).unwrap().tab,
        vec![
            DialogFocusState::WarningButton,
            DialogFocusState::Action { index: 0 },
            DialogFocusState::Action { index: 1 },
            DialogFocusState::Action { index: 2 },
            DialogFocusState::Action { index: 3 },
        ]
    );

    let screen = hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &TerrainTextureState::default(),
        &DialogHudLocalState {
            cursor_position: Some((70, 64)),
            ..Default::default()
        },
        surface_size,
        0.0,
    )
    .unwrap();
    assert!(screen
        .scroll_area
        .as_ref()
        .unwrap()
        .text_labels
        .iter()
        .any(|label| label.text == "Report Server Bug"));
    assert_eq!(screen.tooltip, None);

    let mut counters = NetCounters::default();
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(70.0, 64.0)),
        surface_size,
    ));
    assert_eq!(
        input.dialog_focus(),
        Some(DialogFocusState::Action { index: 0 })
    );
    assert!(input.current_dialog().is_some());
    assert_eq!(
        input.dialog_link_confirmation(),
        Some(&DialogLinkConfirmationState {
            focused_button: None,
            url: "https://example.invalid/support".into(),
            after_action: "close".to_string(),
        })
    );
}

#[test]
fn confirm_link_screen_projects_vanilla_text_layout_and_three_buttons() {
    let mut input = crate::input::ClientInputState::new(true);
    world_with_notice(
        &mut input,
        vec![entry("after_action", NbtSummaryValue::string("none"))],
    );
    assert!(input.open_dialog_url("https://example.invalid/docs".into(), "none".to_string(),));
    assert!(input.focus_dialog_link_button(1));
    let surface_size = PhysicalSize::new(320, 240);

    let screen = hud_dialog_confirm_link_screen(
        input.dialog_link_confirmation().unwrap(),
        input.dialog_link_focused_button(),
        None,
        surface_size,
    );

    assert_eq!((screen.width, screen.height), (320, 240));
    assert!(screen.text_labels.iter().any(|label| {
        label.text == CONFIRM_LINK_TITLE && label.y == 76 && label.tint == TEXT_COLOR
    }));
    assert!(screen
        .text_labels
        .iter()
        .any(|label| { label.text == "https://example.invalid/docs" && label.y == 93 }));
    assert!(screen.text_labels.iter().any(|label| {
        label.text == CONFIRM_LINK_WARNING
            && label.y == 110
            && label.tint == CONFIRM_LINK_WARNING_COLOR
    }));
    assert_eq!(
        screen
            .text_labels
            .iter()
            .filter(|label| matches!(label.text.as_str(), "Yes" | "Copy to Clipboard" | "No"))
            .map(|label| (label.text.as_str(), label.y))
            .collect::<Vec<_>>(),
        [("Yes", 149), ("Copy to Clipboard", 149), ("No", 149)]
    );
    assert_eq!(
        screen.background_layers[1..]
            .iter()
            .map(|layer| (layer.x, layer.y, layer.width, layer.texture))
            .collect::<Vec<_>>(),
        [
            (
                6,
                143,
                100,
                HudInventoryBackgroundTexture::Sprite("widget/button")
            ),
            (
                110,
                143,
                100,
                HudInventoryBackgroundTexture::Sprite("widget/button_highlighted"),
            ),
            (
                214,
                143,
                100,
                HudInventoryBackgroundTexture::Sprite("widget/button")
            ),
        ]
    );
}

#[test]
fn confirm_link_keyboard_copies_or_cancels_without_opening_uri() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(
        &mut input,
        vec![entry("after_action", NbtSummaryValue::string("none"))],
    );
    let mut counters = NetCounters::default();
    assert!(run_dialog_action(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        Some(&DialogActionState::OpenUrl {
            url: "https://example.invalid/copy".into(),
        }),
        "none",
    ));
    assert!(handle_dialog_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    ));
    assert_eq!(input.dialog_link_focused_button(), Some(0));
    assert!(handle_dialog_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    ));
    assert_eq!(input.dialog_link_focused_button(), Some(1));
    let mut clipboard = MockDialogClipboard::default();
    assert!(handle_dialog_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        Some(&mut clipboard),
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    ));
    assert_eq!(
        clipboard.text.as_deref(),
        Some("https://example.invalid/copy")
    );
    assert_eq!(input.dialog_link_confirmation(), None);
    assert!(input.current_dialog().is_some());
    assert!(input.take_dialog_open_url_requests().is_empty());

    assert!(input.open_dialog_url("https://example.invalid/cancel".into(), "none".to_string(),));
    assert!(handle_dialog_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    ));
    assert_eq!(input.dialog_link_confirmation(), None);
    assert!(input.current_dialog().is_some());
    assert!(input.take_dialog_open_url_requests().is_empty());
}

#[test]
fn confirm_link_mouse_yes_queues_uri_and_applies_close_action() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(&mut input, Vec::new());
    assert!(input.open_dialog_url("https://example.invalid/open".into(), "close".to_string(),));
    let mut counters = NetCounters::default();

    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(7.0, 144.0)),
        PhysicalSize::new(320, 240),
    ));

    assert_eq!(input.dialog_link_confirmation(), None);
    assert!(input.current_dialog().is_none());
    assert_eq!(
        input.take_dialog_open_url_requests(),
        ["https://example.invalid/open"]
    );
}

#[test]
fn empty_server_links_dialog_uses_five_pixel_footer_and_escape_close() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_server_links_dialog(&mut input, Vec::new(), Vec::new());
    let surface_size = PhysicalSize::new(320, 120);
    let controls = input
        .current_dialog()
        .and_then(|dialog| dialog.data.as_ref())
        .and_then(|data| dialog_controls(data, world.server_links()))
        .unwrap();
    assert_eq!(controls.body_action_count, 0);
    assert_eq!(controls.footer_height, 5);
    assert!(controls.actions.is_empty());
    assert_eq!(
        dialog_focus_graph(&controls, surface_size).unwrap().tab,
        vec![DialogFocusState::WarningButton]
    );
    let mut counters = NetCounters::default();
    assert!(handle_dialog_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    ));
    assert!(input.current_dialog().is_none());
}
