use super::*;

/// Search box glyphs with `i`, `w` and space narrower or wider than the
/// fallback 6-pixel advance.
fn variable_debug_options_search_glyphs() -> bbb_client_render::HudFontGlyphMap {
    let mut glyphs = FallbackTextMeasurer.command_suggestion_glyphs().clone();
    for (ch, advance) in [('i', 2.0), ('w', 7.0), (' ', 4.0)] {
        glyphs.insert_first_wins(ch, bbb_client_render::HudAsciiGlyph::empty(advance));
    }
    glyphs
}

const DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE: i32 = 6;

#[test]
fn debug_options_screen_search_tracks_editbox_cursor_selection_and_word_keys() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen(&mut Default::default(), &None);

    assert!(input.handle_debug_options_screen_text_input("chunk stats"));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_text_input("_"));
    {
        let screen = input.screen.debug_options().unwrap();
        assert_eq!(screen.search.text(), "chunk stat_s");
        assert_eq!(screen.search.cursor(), 11);
        assert_eq!(screen.search.selection(), 11);
    }

    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Released,
    ));
    {
        let screen = input.screen.debug_options().unwrap();
        assert_eq!(screen.search.text(), "chunk stat_s");
        assert_eq!(screen.search.cursor(), 9);
        assert_eq!(screen.search.selection(), 11);
    }

    assert!(input.handle_debug_options_screen_text_input("X"));
    {
        let screen = input.screen.debug_options().unwrap();
        assert_eq!(screen.search.text(), "chunk staXs");
        assert_eq!(screen.search.cursor(), 10);
        assert_eq!(screen.search.selection(), 10);
    }

    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(TEXT_EDIT_SHORTCUT_KEY),
        ElementState::Pressed,
    ));
    assert!(input
        .handle_debug_options_screen_key(PhysicalKey::Code(KeyCode::KeyA), ElementState::Pressed,));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(TEXT_EDIT_SHORTCUT_KEY),
        ElementState::Released,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_text_input("alpha beta gamma"));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    ));
    {
        let screen = input.screen.debug_options().unwrap();
        assert_eq!(screen.search.text(), "alpha beta ");
        assert_eq!(screen.search.cursor(), 11);
        assert_eq!(screen.search.selection(), 11);
    }

    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Released,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    ));
    assert!(input
        .handle_debug_options_screen_key(PhysicalKey::Code(KeyCode::Home), ElementState::Pressed,));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    ));
    assert!(input
        .handle_debug_options_screen_key(PhysicalKey::Code(KeyCode::End), ElementState::Pressed,));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    ));
    let state = input
        .debug_options_screen_hud_state(winit::dpi::PhysicalSize::new(420, 240), false)
        .unwrap();
    assert_eq!(state.search.text(), "lpha");
    assert_eq!(state.search.cursor(), 4);
    assert_eq!(state.search.selection(), 4);
}

#[test]
fn debug_options_screen_search_uses_editbox_filter_and_default_max_length() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen(&mut Default::default(), &None);

    assert!(input.handle_debug_options_screen_text_input(
        "abc\u{a7}\ndefghijklmnopqrstuvwxyz0123456789more"
    ));

    let screen = input.screen.debug_options().unwrap();
    assert_eq!(screen.search.text(), "abcdefghijklmnopqrstuvwxyz012345");
    assert_eq!(screen.search.value().len(), 32);
    assert_eq!(screen.search.cursor(), 32);
    assert_eq!(screen.search.selection(), 32);
}

#[test]
fn debug_options_screen_search_handles_editbox_clipboard_shortcuts() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen(&mut Default::default(), &None);
    assert!(input.handle_debug_options_screen_text_input("chunk stats"));

    let mut clipboard = MockDebugClipboard::with_text("stale");
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(TEXT_EDIT_SHORTCUT_KEY),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
        Some(&mut clipboard),
    ));
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyC),
        ElementState::Pressed,
        Some(&mut clipboard),
    ));
    assert_eq!(clipboard.text.as_deref(), Some("chunk stats"));
    {
        let screen = input.screen.debug_options().unwrap();
        assert_eq!(screen.search.text(), "chunk stats");
        assert_eq!(screen.search.cursor(), 11);
        assert_eq!(screen.search.selection(), 0);
    }

    clipboard.text = Some("entity_hitboxes".to_string());
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyV),
        ElementState::Pressed,
        Some(&mut clipboard),
    ));
    {
        let screen = input.screen.debug_options().unwrap();
        assert_eq!(screen.search.text(), "entity_hitboxes");
        assert_eq!(screen.search.cursor(), 15);
        assert_eq!(screen.search.selection(), 15);
    }

    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
        Some(&mut clipboard),
    ));
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyX),
        ElementState::Pressed,
        Some(&mut clipboard),
    ));
    assert_eq!(clipboard.text.as_deref(), Some("entity_hitboxes"));
    {
        let screen = input.screen.debug_options().unwrap();
        assert_eq!(screen.search.text(), "");
        assert_eq!(screen.search.cursor(), 0);
        assert_eq!(screen.search.selection(), 0);
    }

    clipboard.text = Some("abc\u{a7}\ndef".to_string());
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyV),
        ElementState::Pressed,
        Some(&mut clipboard),
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(TEXT_EDIT_SHORTCUT_KEY),
        ElementState::Released,
    ));
    let state = input
        .debug_options_screen_hud_state(winit::dpi::PhysicalSize::new(420, 240), false)
        .unwrap();
    assert_eq!(state.search.text(), "abcdef");
    assert_eq!(state.search.cursor(), 6);
    assert_eq!(state.search.selection(), 6);

    let mut rejecting_clipboard = MockDebugClipboard {
        text: None,
        accepts_text: false,
    };
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(TEXT_EDIT_SHORTCUT_KEY),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
        Some(&mut rejecting_clipboard),
    ));
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyX),
        ElementState::Pressed,
        Some(&mut rejecting_clipboard),
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(TEXT_EDIT_SHORTCUT_KEY),
        ElementState::Released,
    ));
    let screen = input.screen.debug_options().unwrap();
    assert_eq!(screen.search.text(), "abcdef");
    assert_eq!(screen.search.cursor(), 6);
    assert_eq!(screen.search.selection(), 0);
}

#[test]
fn debug_options_screen_search_handles_mouse_click_and_drag_selection() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen(&mut Default::default(), &None);
    assert!(input.handle_debug_options_screen_text_input("alpha beta"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let (search_x, search_y, _, _) = debug_options_search_box_rect(surface);
    let text_x = search_x + DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET;

    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x + DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE * 5),
            f64::from(search_y + 2)
        )),
        surface,
        false,
    ));
    {
        let screen = input.screen.debug_options().unwrap();
        assert_eq!(screen.search.cursor(), 5);
        assert_eq!(screen.search.selection(), 5);
    }

    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x + DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE * 9),
            f64::from(search_y + 18)
        )),
        surface,
    ));
    {
        let screen = input.screen.debug_options().unwrap();
        assert_eq!(screen.search.cursor(), 9);
        assert_eq!(screen.search.selection(), 5);
    }

    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Released,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x + DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE * 9),
            f64::from(search_y + 18)
        )),
        surface,
        false,
    ));
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x),
            f64::from(search_y + 2)
        )),
        surface,
    ));
    {
        let screen = input.screen.debug_options().unwrap();
        assert_eq!(screen.search.cursor(), 9);
        assert_eq!(screen.search.selection(), 5);
    }
}

#[test]
fn debug_options_screen_search_mouse_hit_testing_uses_variable_text_measurer() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen(&mut Default::default(), &None);
    assert!(input.handle_debug_options_screen_text_input("iwx"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let (search_x, search_y, _, _) = debug_options_search_box_rect(surface);
    let text_x = search_x + DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET;
    let measurer = variable_debug_options_search_glyphs();

    assert!(
        input.handle_debug_options_screen_mouse_input_with_text_measurer(
            winit::event::MouseButton::Left,
            ElementState::Pressed,
            Some(winit::dpi::PhysicalPosition::new(
                f64::from(text_x + 2),
                f64::from(search_y + 2)
            )),
            surface,
            false,
            &measurer,
        )
    );
    {
        let screen = input.screen.debug_options().unwrap();
        assert_eq!(screen.search.cursor(), 1);
        assert_eq!(screen.search.selection(), 1);
    }

    assert!(
        input.handle_debug_options_screen_cursor_moved_with_text_measurer(
            Some(winit::dpi::PhysicalPosition::new(
                f64::from(text_x + 9),
                f64::from(search_y + 18)
            )),
            surface,
            &measurer,
        )
    );
    let screen = input.screen.debug_options().unwrap();
    assert_eq!(screen.search.cursor(), 2);
    assert_eq!(screen.search.selection(), 1);
}

#[test]
fn debug_options_screen_search_mouse_hit_testing_uses_display_start() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen(&mut Default::default(), &None);
    assert!(input.handle_debug_options_screen_text_input("aaaaaaaaaaaaaaaaaaaa"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let (search_x, search_y, _, _) = debug_options_search_box_rect(surface);
    let text_x = search_x + DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET;
    let measurer = variable_debug_options_search_glyphs();

    assert!(
        input.handle_debug_options_screen_mouse_input_with_text_measurer(
            winit::event::MouseButton::Left,
            ElementState::Pressed,
            Some(winit::dpi::PhysicalPosition::new(
                f64::from(text_x),
                f64::from(search_y + 2)
            )),
            surface,
            false,
            &measurer,
        )
    );
    let screen = input.screen.debug_options().unwrap();
    assert_eq!(screen.search.cursor(), 2);
    assert_eq!(screen.search.selection(), 2);
}

#[test]
fn debug_options_screen_search_double_click_selects_word() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen(&mut Default::default(), &None);
    assert!(input.handle_debug_options_screen_text_input("alpha beta"));
    input.screen.debug_options_mut().unwrap().last_left_click_at = Some(Instant::now());
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let (search_x, search_y, _, _) = debug_options_search_box_rect(surface);
    let text_x = search_x + DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET;

    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x + DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE * 8),
            f64::from(search_y + 2)
        )),
        surface,
        false,
    ));
    let screen = input.screen.debug_options().unwrap();
    assert_eq!(screen.search.selection(), 6);
    assert_eq!(screen.search.cursor(), 10);
}

#[test]
fn debug_options_screen_search_double_click_requires_vanilla_threshold() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen(&mut Default::default(), &None);
    assert!(input.handle_debug_options_screen_text_input("alpha beta"));
    input.screen.debug_options_mut().unwrap().last_left_click_at =
        Some(Instant::now() - DEBUG_OPTIONS_DOUBLE_CLICK_THRESHOLD);
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let (search_x, search_y, _, _) = debug_options_search_box_rect(surface);
    let text_x = search_x + DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET;

    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x + DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE * 8),
            f64::from(search_y + 2)
        )),
        surface,
        false,
    ));
    let screen = input.screen.debug_options().unwrap();
    assert_eq!(screen.search.cursor(), 8);
    assert_eq!(screen.search.selection(), 8);
}

#[test]
fn debug_options_screen_search_shift_click_extends_selection() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen(&mut Default::default(), &None);
    assert!(input.handle_debug_options_screen_text_input("chunk stats"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let (search_x, search_y, _, _) = debug_options_search_box_rect(surface);
    let text_x = search_x + DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET;

    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x + DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE * 5),
            f64::from(search_y + 2)
        )),
        surface,
        false,
    ));

    let screen = input.screen.debug_options().unwrap();
    assert_eq!(screen.search.cursor(), 5);
    assert_eq!(screen.search.selection(), 11);
}
