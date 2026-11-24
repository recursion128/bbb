use super::*;

#[test]
fn sign_editor_clipboard_copy_cut_and_sanitized_paste_match_text_field_helper() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: 3, y: 64, z: 3 };
    let mut world = world_with_sign_text(sign_pos, ["", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 3, y: 64, z: 3 },
        is_front_text: true,
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "copy");
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
    let mut clipboard = MockDebugClipboard::accepting();
    for code in [KeyCode::KeyC, KeyCode::KeyX] {
        crate::input::key_events::dispatch_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            None,
            PhysicalKey::Code(code),
            ElementState::Pressed,
            PhysicalSize::new(320, 240),
            &FixedCommandSuggestionTextMeasurer,
            Some(&mut clipboard),
            None,
            &mut true,
            &mut crate::simulation::effects::PlatformEffects::default(),
        );
        assert_eq!(clipboard.text.as_deref(), Some("copy"));
    }
    assert_eq!(input.screen.sign_editor().unwrap().lines[0], "");
    clipboard.text = Some("\u{a7}aP\raste\n".to_string());
    crate::input::key_events::dispatch_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        None,
        PhysicalKey::Code(KeyCode::KeyV),
        ElementState::Pressed,
        PhysicalSize::new(320, 240),
        &FixedCommandSuggestionTextMeasurer,
        Some(&mut clipboard),
        None,
        &mut true,
        &mut crate::simulation::effects::PlatformEffects::default(),
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
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate { lines, .. }) if lines[0] == "Paste"
    ));
}

#[test]
fn sign_editor_rejected_wide_paste_keeps_the_selected_line() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: 3, y: 64, z: 3 };
    let mut world = world_with_sign_text(sign_pos, ["", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 3, y: 64, z: 3 },
        is_front_text: true,
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "hello");
    let mut clipboard = MockDebugClipboard::accepting();
    clipboard.text = Some("W".repeat(64));
    let mut press = |input: &mut ClientInputState, world: &mut WorldStore, code, state| {
        crate::input::key_events::dispatch_key_input(
            input,
            &mut counters,
            world,
            &commands,
            None,
            PhysicalKey::Code(code),
            state,
            PhysicalSize::new(320, 240),
            &FixedCommandSuggestionTextMeasurer,
            Some(&mut clipboard),
            None,
            &mut true,
            &mut crate::simulation::effects::PlatformEffects::default(),
        );
    };
    press(
        &mut input,
        &mut world,
        TEXT_EDIT_SHORTCUT_KEY,
        ElementState::Pressed,
    );
    press(&mut input, &mut world, KeyCode::KeyA, ElementState::Pressed);
    press(&mut input, &mut world, KeyCode::KeyV, ElementState::Pressed);
    press(
        &mut input,
        &mut world,
        TEXT_EDIT_SHORTCUT_KEY,
        ElementState::Released,
    );
    assert_eq!(input.screen.sign_editor().unwrap().lines[0], "hello");
    press(
        &mut input,
        &mut world,
        KeyCode::Escape,
        ElementState::Pressed,
    );

    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate { lines, .. }) if lines[0] == "hello"
    ));
}
