use super::*;
use crate::input::dialog::DialogBodyFocusPartState;
use crate::input::dialog::DialogFocusState;
use crate::input::dialog::DialogInputValueState;

#[test]
fn wrapped_text_preserves_explicit_trailing_empty_line() {
    let text = DialogTextState {
        text: "Line\n".to_string(),
        runs: Vec::new(),
        component: NbtSummaryValue::string("Line\n"),
    };

    let lines = wrapped_text_lines(&text, 200);

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].text, "Line");
    assert_eq!(lines[1].text, "");
}

#[test]
fn inworld_menu_background_covers_surface_outside_centered_dialog_width() {
    let layer = inworld_menu_background_layer(PhysicalSize::new(800, 600), 512, 512);

    assert_eq!(
        layer.texture,
        HudInventoryBackgroundTexture::Texture("textures/gui/inworld_menu_background.png")
    );
    assert_eq!((layer.x, layer.y), (-144, -44));
    assert_eq!((layer.width, layer.height), (800, 600));
    assert_eq!(layer.uv.min, [0.0, 0.0]);
    assert_eq!(layer.uv.max, [25.0, 18.75]);
}

#[test]
fn dialog_warning_button_opens_confirm_screen_and_dispatches_back_or_disconnect() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(&mut input, Vec::new());
    let surface_size = PhysicalSize::new(320, 240);
    let terrain_textures = TerrainTextureState::default();
    let warning_button = DialogWarningButtonLayout::new(
        &input
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .unwrap()
            .common
            .title,
        surface_size,
    )
    .unwrap();
    assert_eq!(
        warning_button,
        DialogWarningButtonLayout {
            title_x: 127,
            title_y: 13,
            x: 173,
            y: 7,
        }
    );
    let screen = hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &terrain_textures,
        &DialogHudLocalState {
            cursor_position: Some((174, 8)),
            ..Default::default()
        },
        surface_size,
        0.0,
    )
    .unwrap();
    assert_eq!(
        (screen.text_labels[0].x, screen.text_labels[0].y),
        (127, 13)
    );
    assert_eq!(
        screen.background_layers[1].texture,
        HudInventoryBackgroundTexture::Sprite("dialog/warning_button_highlighted")
    );
    assert!(screen.tooltip.as_ref().unwrap().lines[0]
        .text
        .starts_with("This is a custom screen"));

    let mut counters = NetCounters::default();
    let (tx, mut rx) = mpsc::channel(1);
    let net_commands = Some(tx);
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(174.0, 8.0)),
        surface_size,
    ));
    assert!(input.dialog_warning_is_open());

    let warning_screen = hud_dialog_warning_screen(None, None, surface_size);
    assert_eq!(warning_screen.text_labels[0].text, DIALOG_WARNING_TITLE);
    assert!(warning_screen
        .text_labels
        .iter()
        .any(|label| label.text == DIALOG_WARNING_DISCONNECT));
    assert!(warning_screen
        .text_labels
        .iter()
        .any(|label| label.text == DIALOG_WARNING_BACK));
    let message_line_count = dialog_warning_message_lines(surface_size).len();
    let warning_layout = DialogWarningScreenLayout::new(message_line_count, surface_size);

    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
        false,
        false,
    ));
    assert_eq!(input.dialog_warning_focused_button(), Some(true));
    assert!(handle_dialog_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        PhysicalKey::Code(KeyCode::ArrowRight),
        ElementState::Pressed,
    ));
    assert_eq!(input.dialog_warning_focused_button(), Some(false));
    let keyboard_focused =
        hud_dialog_warning_screen(input.dialog_warning_focused_button(), None, surface_size);
    assert_eq!(
        keyboard_focused.background_layers[2].texture,
        HudInventoryBackgroundTexture::Sprite("widget/button_highlighted")
    );
    assert!(handle_dialog_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    ));
    assert!(!input.dialog_warning_is_open());
    assert!(input.current_dialog().is_some());

    assert!(input.open_dialog_warning());
    assert!(handle_dialog_key_input_with_modifiers(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
        true,
        false,
    ));
    assert_eq!(input.dialog_warning_focused_button(), Some(false));
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(warning_layout.yes.x + 1),
            f64::from(warning_layout.yes.y + 1),
        )),
        surface_size,
    ));
    assert_eq!(rx.try_recv().unwrap(), NetCommand::Disconnect);
    assert!(input.dialog_warning_is_open());
    assert!(handle_dialog_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    ));
    assert!(!input.dialog_warning_is_open());
}

#[test]
fn dialog_focus_graph_matches_warning_body_input_and_footer_hierarchy() {
    let mut input = crate::input::ClientInputState::new(true);
    assert_eq!(dialog_content_y(240, 205, FOOTER_HEIGHT), 33);
    let plain = NbtSummaryValue::Compound(vec![
        entry("type", NbtSummaryValue::string("minecraft:plain_message")),
        entry("contents", NbtSummaryValue::string("Plain")),
        entry("width", NbtSummaryValue::Int(80)),
    ]);
    let item = NbtSummaryValue::Compound(vec![
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
        entry("height", NbtSummaryValue::Int(20)),
    ]);
    let options = vec![
        NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::string("a")),
            entry("initial", NbtSummaryValue::Byte(1)),
        ]),
        NbtSummaryValue::Compound(vec![entry("id", NbtSummaryValue::string("b"))]),
    ];
    let mut world = world_with_notice(
        &mut input,
        vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::string("none")),
            entry("body", NbtSummaryValue::List(vec![plain, item])),
            entry(
                "inputs",
                NbtSummaryValue::List(vec![
                    boolean_input("enabled", "Enabled", false, "yes", "no"),
                    number_range_input("amount", "Amount", 100, 0.0, 10.0, None, None),
                    single_option_input("mode", "Mode", 100, true, options),
                    text_input("name", "Name", 100, false, "ab", 16),
                ]),
            ),
            entry(
                "action",
                NbtSummaryValue::Compound(vec![
                    entry("label", NbtSummaryValue::string("Done")),
                    entry("tooltip", NbtSummaryValue::string("Finish dialog")),
                    entry("width", NbtSummaryValue::Int(150)),
                ]),
            ),
        ],
    );
    let surface_size = PhysicalSize::new(320, 240);
    let controls = input
        .current_dialog()
        .and_then(|dialog| dialog.data.as_ref())
        .and_then(|data| dialog_controls(data, world.server_links()))
        .unwrap();
    let graph = dialog_focus_graph(&controls, surface_size).unwrap();
    let expected = vec![
        DialogFocusState::WarningButton,
        DialogFocusState::Body {
            index: 0,
            part: DialogBodyFocusPartState::Primary,
        },
        DialogFocusState::Body {
            index: 1,
            part: DialogBodyFocusPartState::Primary,
        },
        DialogFocusState::Body {
            index: 1,
            part: DialogBodyFocusPartState::Description,
        },
        DialogFocusState::Input { index: 0 },
        DialogFocusState::Input { index: 1 },
        DialogFocusState::Input { index: 2 },
        DialogFocusState::Input { index: 3 },
        DialogFocusState::Action { index: 0 },
    ];
    assert_eq!(graph.tab, expected);
    assert_eq!(
        next_dialog_focus_in_direction(
            &graph,
            Some(DialogFocusState::WarningButton),
            DialogFocusDirection::Down,
        ),
        Some(expected[1])
    );
    assert_eq!(
        next_dialog_focus_in_direction(&graph, Some(expected[1]), DialogFocusDirection::Down,),
        Some(expected[2])
    );
    assert_eq!(
        next_dialog_focus_in_direction(&graph, Some(expected[2]), DialogFocusDirection::Right,),
        Some(expected[3])
    );
    assert_eq!(
        next_dialog_focus_in_direction(&graph, Some(expected[7]), DialogFocusDirection::Down,),
        Some(expected[8])
    );
    assert_eq!(
        next_dialog_focus_in_direction(&graph, Some(expected[8]), DialogFocusDirection::Up,),
        Some(expected[7])
    );

    let mut counters = NetCounters::default();
    for target in &expected {
        assert!(handle_dialog_key_input_with_modifiers_for_surface(
            &mut input,
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Tab),
            ElementState::Pressed,
            false,
            false,
            surface_size,
        ));
        assert_eq!(input.dialog_focus(), Some(*target));
        if *target == (DialogFocusState::Input { index: 1 }) {
            assert!(input.dialog_number_range_keyboard_can_change(1));
        }
    }
    assert!(handle_dialog_key_input_with_modifiers_for_surface(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
        false,
        false,
        surface_size,
    ));
    assert_eq!(input.dialog_focus(), Some(expected[0]));

    let focused_warning = hud_dialog_screen(
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
        focused_warning.background_layers[1].texture,
        HudInventoryBackgroundTexture::Sprite("dialog/warning_button_highlighted")
    );
    assert!(focused_warning.tooltip.is_some());
    assert!(handle_dialog_key_input_with_modifiers_for_surface(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
        false,
        false,
        surface_size,
    ));
    assert!(input.dialog_warning_is_open());
    assert_eq!(input.dialog_focus(), Some(expected[0]));
    assert!(handle_dialog_key_input_with_modifiers_for_surface(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
        false,
        false,
        surface_size,
    ));
    assert!(!input.dialog_warning_is_open());

    assert!(input.focus_dialog_navigation_target(&world, expected[1], false));
    let focused_plain = hud_dialog_screen(
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
        focused_plain
            .scroll_area
            .as_ref()
            .unwrap()
            .fill_layers
            .len(),
        4
    );

    let description = dialog_focus_node(&graph.body, expected[3]).unwrap();
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(description.rectangle.x + 1),
            f64::from(description.rectangle.y + 1),
        )),
        surface_size,
    ));
    assert_eq!(input.dialog_focus(), Some(expected[3]));

    assert!(input.focus_dialog_navigation_target(&world, expected[7], false));
    assert_eq!(input.dialog_text_edit_state().unwrap().cursor, 2);
    assert!(handle_dialog_key_input_with_modifiers_for_surface(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
        false,
        false,
        surface_size,
    ));
    assert_eq!(input.dialog_text_edit_state().unwrap().cursor, 1);
    assert!(handle_dialog_key_input_with_modifiers_for_surface(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
        false,
        false,
        surface_size,
    ));
    assert_eq!(input.dialog_focus(), Some(expected[8]));
    assert_eq!(input.dialog_text_edit_state(), None);
    let focused_action = hud_dialog_screen(
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
        focused_action.background_layers.last().unwrap().texture,
        HudInventoryBackgroundTexture::Sprite("widget/button_highlighted")
    );
    assert_eq!(
        focused_action.tooltip.unwrap().lines[0].text,
        "Finish dialog"
    );
    assert!(handle_dialog_key_input_with_modifiers_for_surface(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
        false,
        false,
        surface_size,
    ));
    assert!(input.current_dialog().is_some());
    assert_eq!(input.dialog_focus(), Some(expected[8]));
    assert!(handle_dialog_key_input_with_modifiers_for_surface(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
        true,
        false,
        surface_size,
    ));
    assert_eq!(input.dialog_focus(), Some(expected[7]));
    assert_eq!(input.dialog_text_edit_state().unwrap().cursor, 1);
}

#[test]
fn overflowing_dialog_body_scrolls_clips_and_reveals_keyboard_focus() {
    let mut input = crate::input::ClientInputState::new(true);
    let body = (0..10)
        .map(|index| {
            NbtSummaryValue::Compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:plain_message")),
                entry("contents", NbtSummaryValue::string(format!("Line {index}"))),
                entry("width", NbtSummaryValue::Int(80)),
            ])
        })
        .collect::<Vec<_>>();
    let options = vec![
        NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::string("a")),
            entry("initial", NbtSummaryValue::Byte(1)),
        ]),
        NbtSummaryValue::Compound(vec![entry("id", NbtSummaryValue::string("b"))]),
    ];
    let mut world = world_with_notice(
        &mut input,
        vec![
            entry("body", NbtSummaryValue::List(body)),
            entry(
                "inputs",
                NbtSummaryValue::List(vec![single_option_input(
                    "mode", "Mode", 100, true, options,
                )]),
            ),
        ],
    );
    let surface_size = PhysicalSize::new(320, 240);
    let controls = input
        .current_dialog()
        .and_then(|dialog| dialog.data.as_ref())
        .and_then(|data| dialog_controls(data, world.server_links()))
        .unwrap();
    let layout = dialog_body_scroll_layout(&controls, surface_size, 0.0).unwrap();
    assert_eq!(
        layout.rectangle,
        DialogFocusRectangle {
            x: 100,
            y: 33,
            width: 120,
            height: 174,
        }
    );
    assert_eq!((layout.content_height, layout.max_scroll), (290, 116));
    assert_eq!(
        dialog_scrollbar_layout(layout),
        Some(DialogScrollbarLayout {
            x: 214,
            y: 33,
            width: 6,
            height: 174,
            thumb_y: 33,
            thumb_height: 104,
        })
    );
    let _ = controls;

    assert!(input.focus_dialog_navigation_target(
        &world,
        DialogFocusState::Input { index: 0 },
        true,
    ));
    assert!(reveal_dialog_keyboard_focus(
        &mut input,
        &mut world,
        DialogFocusState::Input { index: 0 },
        surface_size,
    ));
    assert_eq!(input.dialog_body_scroll_amount(), 116.0);
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
    assert_eq!(
        scroll_area.scissor,
        HudInventoryItemScissor {
            x: 100,
            y: 33,
            width: 120,
            height: 174,
        }
    );
    assert_eq!(scroll_area.scrollbar.unwrap().thumb_y, 103);
    assert!(scroll_area
        .text_labels
        .iter()
        .any(|label| label.text == "Mode: a" && label.y == 193));

    let input_cursor = Some(PhysicalPosition::new(160.0, 190.0));
    assert!(handle_dialog_mouse_wheel(
        &mut input,
        &mut world,
        MouseScrollDelta::LineDelta(0.0, 1.0),
        input_cursor,
        surface_size,
    ));
    assert_eq!(input.dialog_body_scroll_amount(), 106.0);
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::SingleOption {
            selected_index: 0,
        })]
    );

    input.set_dialog_body_scroll_amount(0.0, 116.0);
    let mut counters = NetCounters::default();
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(215.0, 40.0)),
        surface_size,
    ));
    assert_eq!(input.dialog_body_scroll_drag_y(), Some(40));
    assert!(handle_dialog_cursor_moved(
        &mut input,
        &mut world,
        Some(PhysicalPosition::new(215.0, 220.0)),
        surface_size,
    ));
    assert_eq!(input.dialog_body_scroll_amount(), 116.0);
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        MouseButton::Left,
        ElementState::Released,
        None,
        surface_size,
    ));
    assert_eq!(input.dialog_body_scroll_drag_y(), None);
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(220.0, 40.0)),
        surface_size,
    ));
    assert_eq!(input.dialog_body_scroll_drag_y(), Some(40));
    assert!(handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        None,
        MouseButton::Left,
        ElementState::Released,
        None,
        surface_size,
    ));
    assert_eq!(input.dialog_body_scroll_drag_y(), None);
}

#[test]
fn dialog_layout_uses_full_surface_dimensions_beyond_512_pixels() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(&mut input, Vec::new());
    let surface_size = PhysicalSize::new(960, 540);
    let terrain_textures = TerrainTextureState::default();

    let screen = hud_dialog_screen(
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
    assert_eq!((screen.width, screen.height), (960, 540));

    let footer_button = screen
        .background_layers
        .iter()
        .find(|layer| layer.texture == HudInventoryBackgroundTexture::Sprite("widget/button"))
        .expect("footer action button");
    assert_eq!(footer_button.x, (960 - footer_button.width as i32) / 2);
    assert_eq!(
        footer_button.y,
        540 - FOOTER_HEIGHT + (FOOTER_HEIGHT - 20) / 2
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
        Some(PhysicalPosition::new(
            f64::from(footer_button.x + 10),
            f64::from(footer_button.y + 10),
        )),
        surface_size,
    ));
    assert!(input.current_dialog().is_none());
}

#[test]
fn dialog_text_wraps_at_word_boundaries_using_measurer_glyph_metrics() {
    let mut glyphs = bbb_client_render::HudFontGlyphMap::new();
    glyphs.set_missing_glyph(bbb_client_render::HudAsciiGlyph::empty(10.0));
    glyphs.insert_first_wins(' ', bbb_client_render::HudAsciiGlyph::empty(4.0));
    let text_ctx = DialogTextContext {
        item_runtime: None,
        glyphs: &glyphs,
    };

    let message = DialogPlainMessageState {
        contents: DialogTextState {
            text: "hi world".to_string(),
            runs: Vec::new(),
            component: NbtSummaryValue::Compound(vec![
                entry("text", NbtSummaryValue::string("hi world")),
                entry(
                    "click_event",
                    NbtSummaryValue::Compound(vec![
                        entry("action", NbtSummaryValue::string("copy_to_clipboard")),
                        entry("value", NbtSummaryValue::string("word")),
                    ]),
                ),
            ]),
        },
        width: 68, // inner width = 60: "hi" (20) + ' ' (4) + "world" (50) = 74 > 60
    };
    let block = dialog_text_block_with(&message, text_ctx).unwrap();
    assert_eq!(
        block
            .lines
            .iter()
            .map(|line| (line.text.as_str(), line.width))
            .collect::<Vec<_>>(),
        vec![("hi", 20), ("world", 50)]
    );
    assert_eq!(block.lines[0].click_spans[0].width, 20);
    assert_eq!(block.lines[1].click_spans[0].width, 50);

    // Click hit-test uses the same custom glyph advances and full surface coordinates.
    let rect = DialogFocusRectangle {
        x: 100,
        y: 50,
        width: block.width as i32,
        height: block.height,
    };
    assert_eq!(
        dialog_text_click_event_at_with(
            &message,
            rect,
            Some(PhysicalPosition::new(110.0, 50.0 + 4.0 + 10.0)),
            PhysicalSize::new(960, 540),
            text_ctx,
        ),
        Some(ComponentClickEvent::CopyToClipboard {
            value: "word".to_string().into(),
        })
    );
}

#[test]
fn dialog_non_button_text_uses_unified_localized_component_owner() {
    let root = std::env::temp_dir().join(format!(
        "bbb-dialog-l10n-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    crate::item_registry_test_fixture::write_item_registry_fixture(&root, &["stone"]);
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
            "lang/en_us.json",
            r#"{"dialog.test.title":"Localized Title","dialog.test.body":"Localized Body"}"#,
        ),
    ] {
        let path = assets.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap())
            .unwrap()
            .with_vanilla_item_registry_for_test();
    std::fs::remove_dir_all(&root).unwrap();

    let text_ctx = DialogTextContext::approximate(Some(&item_runtime));
    let title = DialogTextState {
        text: "dialog.test.title".to_string(),
        runs: Vec::new(),
        component: NbtSummaryValue::Compound(vec![entry(
            "translate",
            NbtSummaryValue::string("dialog.test.title"),
        )]),
    };
    let body = DialogTextState {
        text: "dialog.test.body".to_string(),
        runs: Vec::new(),
        component: NbtSummaryValue::Compound(vec![entry(
            "translate",
            NbtSummaryValue::string("dialog.test.body"),
        )]),
    };

    let mut labels = Vec::new();
    push_text_label(&mut labels, &title, text_ctx, 10, 10);
    assert_eq!(labels[0].text, "Localized Title");
    assert_eq!(
        labels[0]
            .runs
            .iter()
            .map(|r| r.text.as_str())
            .collect::<String>(),
        "Localized Title"
    );

    let wrapped = wrapped_text_lines_with(&body, 200, text_ctx);
    assert_eq!(wrapped.len(), 1);
    assert_eq!(wrapped[0].text, "Localized Body");
}
