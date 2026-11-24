use super::*;

#[test]
fn f3_f6_records_debug_options_screen_request_without_toggling_overlay() {
    let mut input = ClientInputState::new(true);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F6),
        ElementState::Pressed,
        None,
        None
    ));
    assert_eq!(input.take_debug_options_screen_requests(), 1);
    assert_eq!(input.take_debug_options_screen_requests(), 0);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn debug_options_screen_search_filters_rows() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen(&mut Default::default(), &None);
    let surface = winit::dpi::PhysicalSize::new(420, 240);

    assert!(input.handle_debug_options_screen_text_input("chunk"));
    let filtered = input
        .debug_options_screen_hud_state(surface, false)
        .expect("screen should project after search");
    let labels = filtered
        .rows
        .iter()
        .map(|row| match row {
            DebugOptionsScreenHudRow::Category { label } => label.as_str(),
            DebugOptionsScreenHudRow::Entry { path, .. } => path.as_str(),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec![
            "Debug Screen Text",
            "chunk_generation_stats",
            "chunk_render_stats",
            "chunk_source_stats",
            "Debug Renderers",
            "chunk_borders",
            "chunk_section_octree",
        ]
    );
}

#[test]
fn debug_options_screen_keyboard_focus_starts_at_search_and_skips_inactive_widgets() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen(&mut Default::default(), &None);
    let surface = winit::dpi::PhysicalSize::new(420, 240);

    let initial = input
        .debug_options_screen_hud_state(surface, false)
        .expect("debug options screen");
    assert!(initial.search_focused);
    assert!(!initial.done_focused);

    assert!(input
        .handle_debug_options_screen_key(PhysicalKey::Code(KeyCode::Tab), ElementState::Pressed,));
    let first_focus = input.debug_options_screen_focus().unwrap();
    let DebugOptionsScreenFocus::Entry {
        entry: first_entry,
        status: first_status,
    } = first_focus
    else {
        panic!("Tab should enter the option list");
    };
    assert_ne!(input.debug_screen_entry_status(first_entry), first_status);

    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ArrowRight),
        ElementState::Pressed,
    ));
    let moved = input.debug_options_screen_focus().unwrap();
    assert!(matches!(
        moved,
        DebugOptionsScreenFocus::Entry {
            entry,
            status: DebugScreenEntryStatus::AlwaysOn,
        } if entry == first_entry
    ));

    assert!(
        input.handle_debug_options_screen_key(
            PhysicalKey::Code(KeyCode::Enter),
            ElementState::Pressed,
        )
    );
    assert_eq!(
        input.debug_screen_entry_status(first_entry),
        DebugScreenEntryStatus::AlwaysOn
    );
    assert!(input.handle_debug_options_screen_text_input("ignored"));
    assert_eq!(
        input
            .debug_options_screen_hud_state(surface, false)
            .unwrap()
            .search
            .text(),
        ""
    );

    input.set_debug_options_focus(DebugOptionsScreenFocus::Search, surface);
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    ));
    assert!(input
        .handle_debug_options_screen_key(PhysicalKey::Code(KeyCode::Tab), ElementState::Pressed,));
    assert_eq!(
        input.debug_options_screen_focus(),
        Some(DebugOptionsScreenFocus::Done)
    );
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Released,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    ));
    assert_eq!(
        input.debug_options_screen_focus(),
        Some(DebugOptionsScreenFocus::PerformanceProfile)
    );
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ArrowUp),
        ElementState::Pressed,
    ));
    assert!(matches!(
        input.debug_options_screen_focus(),
        Some(DebugOptionsScreenFocus::Entry {
            status: DebugScreenEntryStatus::Never,
            ..
        })
    ));
}

#[test]
fn debug_options_screen_directional_focus_falls_through_an_inactive_filtered_column() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen(&mut Default::default(), &None);
    assert!(input.handle_debug_options_screen_text_input("biome"));

    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ArrowDown),
        ElementState::Pressed,
    ));
    assert_eq!(
        input.debug_options_screen_focus(),
        Some(DebugOptionsScreenFocus::PerformanceProfile)
    );
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ArrowUp),
        ElementState::Pressed,
    ));
    assert_eq!(
        input.debug_options_screen_focus(),
        Some(DebugOptionsScreenFocus::Search)
    );

    input.set_debug_options_focus(
        DebugOptionsScreenFocus::List,
        winit::dpi::PhysicalSize::new(800, 600),
    );
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ArrowDown),
        ElementState::Pressed,
    ));
    assert_eq!(
        input.debug_options_screen_focus(),
        Some(DebugOptionsScreenFocus::PerformanceProfile)
    );
}

#[test]
fn debug_options_screen_mouse_focus_projects_selected_button_and_search_focus() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen(&mut Default::default(), &None);
    assert!(input.handle_debug_options_screen_text_input("entity_hitboxes"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let buttons_start = debug_options_content_x(surface) + DEBUG_OPTIONS_ROW_WIDTH
        - DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 3;
    let always_x = buttons_start + DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 2 + 2;
    let entry_y = DEBUG_OPTIONS_HEADER_HEIGHT + DEBUG_OPTIONS_ROW_HEIGHT + 2;

    assert!(input.handle_debug_options_screen_mouse_input(
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(always_x),
            f64::from(entry_y),
        )),
        surface,
        false,
    ));
    let focused = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert!(!focused.search_focused);
    assert!(focused.rows.iter().any(|row| matches!(
        row,
        DebugOptionsScreenHudRow::Entry {
            entry: DebugScreenEntryId::EntityHitboxes,
            focused_status: Some(DebugScreenEntryStatus::AlwaysOn),
            ..
        }
    )));

    assert!(input.handle_debug_options_screen_mouse_input(
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(debug_options_content_x(surface) + 2),
            f64::from(entry_y),
        )),
        surface,
        false,
    ));
    assert_eq!(
        input.debug_options_screen_focus(),
        Some(DebugOptionsScreenFocus::Entry {
            entry: DebugScreenEntryId::EntityHitboxes,
            status: DebugScreenEntryStatus::AlwaysOn,
        })
    );

    let (search_x, search_y, _, _) = debug_options_search_box_rect(surface);
    assert!(input.handle_debug_options_screen_mouse_input(
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(search_x + 4),
            f64::from(search_y + 4),
        )),
        surface,
        false,
    ));
    assert!(
        input
            .debug_options_screen_hud_state(surface, false)
            .unwrap()
            .search_focused
    );

    assert!(input.handle_debug_options_screen_mouse_input(
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(debug_options_content_x(surface) + 2),
            f64::from(entry_y),
        )),
        surface,
        false,
    ));
    assert_eq!(
        input.debug_options_screen_focus(),
        Some(DebugOptionsScreenFocus::List)
    );

    assert!(input.handle_debug_options_screen_mouse_input(
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(search_x + 4),
            f64::from(search_y + 4),
        )),
        surface,
        false,
    ));
    assert!(input.handle_debug_options_screen_mouse_input(
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(debug_options_content_x(surface) + 2),
            f64::from(DEBUG_OPTIONS_HEADER_HEIGHT + DEBUG_OPTIONS_ROW_HEIGHT * 4 + 2),
        )),
        surface,
        false,
    ));
    assert_eq!(
        input.debug_options_screen_focus(),
        Some(DebugOptionsScreenFocus::Search)
    );
}

#[test]
fn debug_options_screen_buttons_update_status_and_profiles() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen(&mut Default::default(), &None);
    assert!(input.handle_debug_options_screen_text_input("entity_hitboxes"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let content_x = debug_options_content_x(surface);
    let buttons_start = content_x + DEBUG_OPTIONS_ROW_WIDTH - DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 3;
    let always_x = buttons_start + DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 2 + 2;
    let entry_y = DEBUG_OPTIONS_HEADER_HEIGHT + DEBUG_OPTIONS_ROW_HEIGHT + 2;

    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(always_x),
            f64::from(entry_y)
        )),
        surface,
        false,
    ));
    assert_eq!(
        input.debug_screen_entry_status(DebugScreenEntryId::EntityHitboxes),
        DebugScreenEntryStatus::AlwaysOn
    );
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert!(state.default_profile_active);
    assert!(state.performance_profile_active);

    let (_, performance_x, _) = debug_options_footer_button_xs(surface);
    let footer_y = debug_options_footer_button_y(surface) + 2;
    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(performance_x + 2),
            f64::from(footer_y)
        )),
        surface,
        false,
    ));
    assert_eq!(
        input.debug_screen_entry_status(DebugScreenEntryId::Fps),
        DebugScreenEntryStatus::AlwaysOn
    );
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert!(state.default_profile_active);
    assert!(!state.performance_profile_active);
}

#[test]
fn debug_options_screen_projects_button_hover_state() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen(&mut Default::default(), &None);
    assert!(input.handle_debug_options_screen_text_input("entity_hitboxes"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let content_x = debug_options_content_x(surface);
    let buttons_start = content_x + DEBUG_OPTIONS_ROW_WIDTH - DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 3;
    let always_x = buttons_start + DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 2 + 2;
    let entry_y = DEBUG_OPTIONS_HEADER_HEIGHT + DEBUG_OPTIONS_ROW_HEIGHT + 2;

    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(always_x),
            f64::from(entry_y)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    let hovered_status = state.rows.iter().find_map(|row| match row {
        DebugOptionsScreenHudRow::Entry {
            entry,
            hovered_status,
            ..
        } if *entry == DebugScreenEntryId::EntityHitboxes => *hovered_status,
        _ => None,
    });
    assert_eq!(hovered_status, Some(DebugScreenEntryStatus::AlwaysOn));
    assert!(!state.default_profile_hovered);
    assert!(!state.performance_profile_hovered);
    assert!(!state.done_hovered);

    let (default_x, performance_x, done_x) = debug_options_footer_button_xs(surface);
    let footer_y = debug_options_footer_button_y(surface) + 2;
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(default_x + 2),
            f64::from(footer_y)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert!(state.default_profile_hovered);
    assert!(!state.performance_profile_hovered);
    assert!(!state.done_hovered);

    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(performance_x + 2),
            f64::from(footer_y)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert!(!state.default_profile_hovered);
    assert!(state.performance_profile_hovered);
    assert!(!state.done_hovered);

    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(done_x + 2),
            f64::from(footer_y)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert!(!state.default_profile_hovered);
    assert!(!state.performance_profile_hovered);
    assert!(state.done_hovered);
}

#[test]
fn debug_options_screen_projects_not_allowed_tooltip_on_entry_name_hover() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen(&mut Default::default(), &None);
    assert!(input.handle_debug_options_screen_text_input("biome"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let content_x = debug_options_content_x(surface);
    let name_x = content_x + 2;
    let entry_y = DEBUG_OPTIONS_HEADER_HEIGHT + DEBUG_OPTIONS_ROW_HEIGHT + 2;

    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(name_x),
            f64::from(entry_y)
        )),
        surface,
    ));
    let reduced = input
        .debug_options_screen_hud_state(surface, true)
        .expect("screen should project under reduced debug info");
    assert_eq!(
        reduced
            .tooltip
            .as_ref()
            .map(|tooltip| tooltip.text.as_str()),
        Some("Not visible when debug info is reduced")
    );
    assert_eq!(
        reduced
            .tooltip
            .as_ref()
            .map(|tooltip| (tooltip.x, tooltip.y)),
        Some((name_x, entry_y))
    );

    let full = input
        .debug_options_screen_hud_state(surface, false)
        .expect("screen should project without reduced debug info");
    assert_eq!(full.tooltip, None);

    let buttons_start = content_x + DEBUG_OPTIONS_ROW_WIDTH - DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 3;
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(buttons_start),
            f64::from(entry_y)
        )),
        surface,
    ));
    let over_button = input
        .debug_options_screen_hud_state(surface, true)
        .expect("screen should project with button hover");
    assert_eq!(over_button.tooltip, None);
}

#[test]
fn debug_options_screen_scrollbar_drag_updates_scroll_row() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen(&mut Default::default(), &None);
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let scrollbar_x = debug_options_scrollbar_x(surface) + 1;
    let start_y = DEBUG_OPTIONS_HEADER_HEIGHT + 29;

    assert_eq!(
        debug_options_scrollbar_drag_scroll_row(0, 47, surface, start_y, start_y + 3),
        1
    );
    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(scrollbar_x),
            f64::from(start_y)
        )),
        surface,
        false,
    ));
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(scrollbar_x),
            f64::from(start_y + 3)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert_eq!(state.scroll_row, 1);

    let below_list_y = DEBUG_OPTIONS_HEADER_HEIGHT + debug_options_list_height(surface) + 1;
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(scrollbar_x),
            f64::from(below_list_y)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert_eq!(state.scroll_row, 40);

    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Released,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(scrollbar_x),
            f64::from(below_list_y)
        )),
        surface,
        false,
    ));
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(scrollbar_x),
            f64::from(DEBUG_OPTIONS_HEADER_HEIGHT)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert_eq!(state.scroll_row, 40);
}

#[test]
fn debug_options_screen_consumes_keys_scrolls_and_allows_f3_global_keymap() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen(&mut Default::default(), &None);
    let surface = winit::dpi::PhysicalSize::new(420, 161);

    assert!(input.handle_debug_options_screen_mouse_wheel(
        winit::event::MouseScrollDelta::LineDelta(0.0, -3.0),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert_eq!(state.visible_rows, 3);
    assert_eq!(state.scroll_row, 3);

    assert!(!input
        .handle_debug_options_screen_key(PhysicalKey::Code(KeyCode::F3), ElementState::Pressed));
    assert!(input.debug_options_screen_is_open());
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed
    ));
    assert!(!input.debug_options_screen_is_open());
}

#[test]
fn f3_escape_records_pause_without_menu_request_without_toggling_overlay() {
    let mut input = ClientInputState::new(true);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
        None,
        None
    ));
    assert_eq!(input.take_debug_pause_screen_requests(), 1);
    assert_eq!(input.take_debug_pause_screen_requests(), 0);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn releasing_active_input_preserves_f3_escape_debug_action_state() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
        None,
        None
    ));
    assert_eq!(input.take_debug_pause_screen_requests(), 1);

    release_active_input(&mut input, &mut world, &mut counters, &None);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn debug_pause_screen_consumes_gameplay_keys_and_escape_closes() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    input.open_debug_pause_screen_with_menu(&WorldStore::new(), &mut Default::default(), &None);

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert!(input.debug_pause_screen_is_open());
    assert!(!input.forward);
    assert!(!input.pressed_keys.contains(&KeyCode::KeyW));

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!input.debug_pause_screen_is_open());
    assert!(!input.pressed_keys.contains(&KeyCode::Escape));
}

#[test]
fn escape_opens_pause_screen_with_menu() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert_eq!(
        input.debug_pause_screen(),
        Some(&pause_screen::DebugPauseScreenState {
            show_pause_menu: true,
            cursor_position: None,
            ..pause_screen::DebugPauseScreenState::default()
        })
    );
    assert!(!input.pressed_keys.contains(&KeyCode::Escape));
}

#[test]
fn debug_pause_screen_return_to_game_button_closes_menu_screen() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu(&world, &mut Default::default(), &None);

    assert!(input.handle_debug_pause_screen_cursor_moved(Some(PhysicalPosition::new(68.0, 78.0))));
    assert_eq!(
        input.debug_pause_screen(),
        Some(&pause_screen::DebugPauseScreenState {
            show_pause_menu: true,
            cursor_position: Some((68, 78)),
            ..pause_screen::DebugPauseScreenState::default()
        })
    );
    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(68.0, 78.0)),
        surface,
    ));

    assert!(!input.debug_pause_screen_is_open());
}

#[test]
fn debug_pause_screen_advancements_button_opens_advancements_screen() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![input_displayed_advancement("minecraft:story/root", None)],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });
    input.open_debug_pause_screen_with_menu(&world, &mut Default::default(), &None);

    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(68.0, 102.0)),
        surface,
    ));

    assert!(input.advancements_screen_is_open());
    assert!(matches!(input.screen.parent(), Some(Screen::Pause(_))));
    assert_eq!(
        world.selected_advancements_tab(),
        Some("minecraft:story/root")
    );
    assert_eq!(counters.advancements_seen_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SeenAdvancements(SeenAdvancements::OpenedTab {
            tab: "minecraft:story/root".to_string()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn debug_pause_screen_stats_button_opens_stats_screen_and_requests_stats() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu(&world, &mut Default::default(), &None);

    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(170.0, 102.0)),
        surface,
    ));

    assert!(input.stats_screen_is_open());
    assert!(matches!(input.screen.parent(), Some(Screen::Pause(_))));
    assert_eq!(counters.request_stats_commands_queued, 1);
    assert_eq!(rx.try_recv().unwrap(), NetCommand::RequestStats);
    assert!(rx.try_recv().is_err());
}

#[test]
fn debug_pause_screen_send_feedback_button_records_link_request() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu(&world, &mut Default::default(), &None);

    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(68.0, 126.0)),
        surface,
    ));

    assert!(input.debug_pause_screen_is_open());
    assert_eq!(
        input.take_pause_screen_link_requests(),
        vec![PauseScreenLinkRequest::SendFeedback {
            url: "https://aka.ms/javafeedback?ref=game"
        }]
    );
}

#[test]
fn debug_pause_screen_report_bugs_button_records_link_request_when_enabled() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu(&world, &mut Default::default(), &None);

    assert!(pause_screen_report_bugs_button_enabled());
    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(170.0, 126.0)),
        surface,
    ));

    assert!(input.debug_pause_screen_is_open());
    assert_eq!(
        input.take_pause_screen_link_requests(),
        vec![PauseScreenLinkRequest::ReportBugs {
            url: "https://aka.ms/snapshotbugs?ref=game"
        }]
    );
}

#[test]
fn debug_pause_screen_disconnect_button_records_disconnect_request() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    let (x, y, width, height) = bbb_client_render::hud_pause_disconnect_button_rect(surface);
    let position =
        winit::dpi::PhysicalPosition::new(f64::from(x + width / 2), f64::from(y + height / 2));
    input.open_debug_pause_screen_with_menu(&world, &mut Default::default(), &None);

    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(position),
        surface,
    ));

    assert!(input.debug_pause_screen_is_open());
    assert!(input.pause_screen_disconnect_requested());
    assert!(input.take_pause_screen_link_requests().is_empty());
}

#[test]
fn debug_pause_screen_still_allows_global_f3_overlay_toggle() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    input.open_debug_pause_screen_with_menu(&WorldStore::new(), &mut Default::default(), &None);

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert!(input.debug_pause_screen_is_open());
    assert!(input.debug_overlay_visible());
}

#[test]
fn f3_c_copies_location_tp_command_to_clipboard_and_reports_feedback() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 10.25,
            y: 64.0,
            z: -5.75,
        },
        y_rot: 90.5,
        x_rot: -15.25,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyC),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some("/execute in minecraft:overworld run tp @s 10.25 64.00 -5.75 90.50 -15.25")
    );
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].kind, ChatMessageKind::ClientSystem);
    assert_eq!(messages[0].content, "[Debug]: Copied location to clipboard");
    assert_eq!(world.counters().chat_messages_tracked, 1);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_c_reduced_debug_blocks_location_copy_but_still_consumes_crash_modifier() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(true);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 10.25,
            y: 64.0,
            z: -5.75,
        },
        y_rot: 90.5,
        x_rot: -15.25,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyC),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert_eq!(clipboard.text, None);
    assert!(world.client_chat().messages.is_empty());

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_c_hold_reports_manual_crash_warning_countdown_without_toggling_overlay() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);
    let start = Instant::now();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    input.set_key_down(KeyCode::KeyC, true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyC),
        ElementState::Pressed,
        Some(&mut world),
        None,
    ));

    advance_player_input(&mut input, &mut world, &mut counters, &commands, start);
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_millis(999),
    );
    assert!(world.client_chat().messages.is_empty());

    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_secs(1),
    );
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_secs(2),
    );
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 2);
    assert_eq!(
        messages[0].content,
        "[Debug]: F3 + C is held down. This will crash the game unless released."
    );
    assert_eq!(messages[1].content, "[Debug]: Crashing in 8...");
    let first_prefix = &messages[0].styled_content[0];
    let countdown_prefix = &messages[1].styled_content[0];
    assert_eq!(first_prefix.style.color, Some(0xFFFF55));
    assert_eq!(countdown_prefix.style.color, Some(0xFF5555));
    assert_eq!(countdown_prefix.style.bold, Some(true));

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyC),
        ElementState::Released,
    );
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_secs(3),
    );
    assert_eq!(world.client_chat().messages.len(), 2);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(!input.debug_overlay_visible());
    assert_eq!(counters.player_input_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
#[should_panic(expected = "Manually triggered debug crash")]
fn f3_c_hold_panics_after_vanilla_manual_crash_delay() {
    let (tx, _rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);
    let start = Instant::now();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    input.set_key_down(KeyCode::KeyC, true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyC),
        ElementState::Pressed,
        Some(&mut world),
        None,
    ));

    advance_player_input(&mut input, &mut world, &mut counters, &commands, start);
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_millis(10_001),
    );
}

#[test]
fn f3_game_mode_keys_report_no_permission_without_gameplay_commands() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyN),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert!(!input.debug_overlay_visible());
    let feedback: Vec<_> = world
        .client_chat()
        .messages
        .iter()
        .map(|message| message.content.as_str())
        .collect();
    assert_eq!(
        feedback,
        vec![
            "[Debug]: Unable to switch game mode; no permission",
            "[Debug]: Unable to open game mode switcher; no permission",
        ]
    );
    assert!(world
        .client_chat()
        .messages
        .iter()
        .all(|message| message.kind == ChatMessageKind::ClientSystem));
    assert_eq!(counters.player_input_commands_queued, 0);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(counters.change_game_mode_commands_queued, 0);
    assert_eq!(input.debug_game_mode_switcher_selected(), None);
    assert!(rx.try_recv().is_err());
}

#[test]
fn f3_n_queues_spectator_game_mode_with_permission() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyN),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert!(!input.debug_overlay_visible());
    assert!(world.client_chat().messages.is_empty());
    assert_eq!(counters.change_game_mode_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChangeGameMode(bbb_protocol::packets::ChangeGameModeCommand {
            game_mode: bbb_protocol::packets::GameType::Spectator,
        })
    );
}

#[test]
fn f3_n_queues_previous_or_creative_game_mode_when_already_spectator() {
    for (previous_game_type, expected_game_mode) in [
        (-1, bbb_protocol::packets::GameType::Creative),
        (0, bbb_protocol::packets::GameType::Survival),
        (2, bbb_protocol::packets::GameType::Adventure),
    ] {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = world_with_debug_player_in_game_mode(false, 3, previous_game_type);
        grant_debug_recreate_nbt_permission(&mut world);
        assert!(world.local_player_is_spectator());

        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::F3),
            ElementState::Pressed,
        );
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::KeyN),
            ElementState::Pressed,
        );

        assert_eq!(counters.change_game_mode_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ChangeGameMode(bbb_protocol::packets::ChangeGameModeCommand {
                game_mode: expected_game_mode,
            })
        );
    }
}

#[test]
fn f3_f4_game_mode_switcher_queues_default_selection_on_f3_release() {
    for (game_type, previous_game_type, expected_game_mode) in [
        (0, -1, bbb_protocol::packets::GameType::Creative),
        (1, -1, bbb_protocol::packets::GameType::Survival),
        (3, 0, bbb_protocol::packets::GameType::Survival),
    ] {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = world_with_debug_player_in_game_mode(false, game_type, previous_game_type);
        grant_debug_recreate_nbt_permission(&mut world);
        input.synchronize_camera_post_effect(1, 1, Some("minecraft:spider"));

        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::F3),
            ElementState::Pressed,
        );
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::F4),
            ElementState::Pressed,
        );

        assert_eq!(
            input.debug_game_mode_switcher_selected(),
            Some(expected_game_mode)
        );
        assert!(
            input.camera_post_effect_active(),
            "the consumed F3+F4 chord must not toggle the spectator post effect"
        );
        assert_eq!(counters.change_game_mode_commands_queued, 0);
        assert!(rx.try_recv().is_err());

        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::F3),
            ElementState::Released,
        );

        assert_eq!(input.debug_game_mode_switcher_selected(), None);
        assert!(!input.debug_overlay_visible());
        assert_eq!(counters.change_game_mode_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ChangeGameMode(bbb_protocol::packets::ChangeGameModeCommand {
                game_mode: expected_game_mode,
            })
        );
    }
}

#[test]
fn camera_post_effect_f3_f4_chord_preserves_active_state() {
    let (tx, _rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player_in_game_mode(false, 1, -1);
    grant_debug_recreate_nbt_permission(&mut world);
    input.synchronize_camera_post_effect(1, 1, Some("minecraft:spider"));

    for code in [KeyCode::F3, KeyCode::F4] {
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );
    }

    assert!(
        input.camera_post_effect_active(),
        "the consumed F3+F4 chord must not toggle the spectator post effect"
    );
}

#[test]
fn f3_f4_game_mode_switcher_cycles_with_additional_f4_presses() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    for _ in 0..3 {
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::F4),
            ElementState::Pressed,
        );
    }
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Adventure)
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert_eq!(counters.change_game_mode_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChangeGameMode(bbb_protocol::packets::ChangeGameModeCommand {
            game_mode: bbb_protocol::packets::GameType::Adventure,
        })
    );
}

#[test]
fn f3_f4_game_mode_switcher_does_not_open_over_existing_screen() {
    #[derive(Debug, Clone, Copy)]
    enum ExistingScreen {
        Chat,
        LocalInventory,
        Book,
        Dialog,
        Advancements,
        SignEditor,
    }

    for existing_screen in [
        ExistingScreen::Chat,
        ExistingScreen::LocalInventory,
        ExistingScreen::Book,
        ExistingScreen::Dialog,
        ExistingScreen::Advancements,
        ExistingScreen::SignEditor,
    ] {
        let mut input = ClientInputState::new(true);
        let commands = None;
        let mut counters = NetCounters::default();
        let mut world = world_with_debug_player(false);
        grant_debug_recreate_nbt_permission(&mut world);
        input.apply_screen_requests(&mut world, &mut counters, &commands);

        match existing_screen {
            ExistingScreen::Chat => handle_key_input(
                &mut input,
                &mut counters,
                &mut world,
                &commands,
                PhysicalKey::Code(KeyCode::KeyT),
                ElementState::Pressed,
            ),
            ExistingScreen::LocalInventory => {
                assert!(world.open_local_inventory());
            }
            ExistingScreen::Book => open_test_book_screen(&mut input, &mut world, vec!["First"]),
            ExistingScreen::Dialog => world.apply_show_dialog(ProtocolShowDialog {
                dialog: DialogHolder::Reference { registry_id: 11 },
            }),
            ExistingScreen::Advancements => {
                open_test_advancements_screen(&mut input);
            }
            ExistingScreen::SignEditor => {
                let pos = BlockPos { x: 1, y: 2, z: 3 };
                insert_sign_text(&mut world, pos, ["", "", "", ""], ["", "", "", ""]);
                world.apply_open_sign_editor(OpenSignEditor {
                    pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
                    is_front_text: true,
                });
            }
        }
        input.apply_screen_requests(&mut world, &mut counters, &commands);
        assert!(
            input.current_screen().is_some(),
            "fixture must open {existing_screen:?}"
        );

        assert!(input.handle_debug_overlay_key(
            PhysicalKey::Code(KeyCode::F3),
            ElementState::Pressed,
            Some(&mut world),
            None
        ));
        assert!(
            !input.handle_debug_overlay_key(
                PhysicalKey::Code(KeyCode::F4),
                ElementState::Pressed,
                Some(&mut world),
                None
            ),
            "F3+F4 should not be a debug action over {existing_screen:?}"
        );
        assert_eq!(input.debug_game_mode_switcher_selected(), None);
        assert!(world.client_chat().messages.is_empty());
        assert!(input.handle_debug_overlay_key(
            PhysicalKey::Code(KeyCode::F3),
            ElementState::Released,
            Some(&mut world),
            None
        ));
        assert!(
            input.debug_overlay_visible(),
            "blocked F4 should leave F3 release as an ordinary overlay toggle over {existing_screen:?}"
        );
    }
}

#[test]
fn f3_f4_game_mode_switcher_hover_uses_first_mouse_suppression() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let surface_size = PhysicalSize::new(320, 240);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Pressed,
    );
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Creative)
    );

    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(176.0, 102.0)),
        surface_size
    ));
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Creative),
        "first mouse position only arms hover detection"
    );
    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(176.0, 102.0)),
        surface_size
    ));
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Creative),
        "same as first mouse position stays suppressed"
    );
    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(207.0, 102.0)),
        surface_size
    ));
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Spectator)
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert_eq!(counters.change_game_mode_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChangeGameMode(bbb_protocol::packets::ChangeGameModeCommand {
            game_mode: bbb_protocol::packets::GameType::Spectator,
        })
    );
}

#[test]
fn f3_f4_game_mode_switcher_f4_cycle_resets_first_mouse_suppression() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let surface_size = PhysicalSize::new(320, 240);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Pressed,
    );
    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(145.0, 102.0)),
        surface_size
    ));
    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(176.0, 102.0)),
        surface_size
    ));
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Adventure)
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Pressed,
    );
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Spectator)
    );
    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(114.0, 102.0)),
        surface_size
    ));
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Spectator),
        "cursor hover is suppressed again after F4 cycling"
    );
    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(145.0, 102.0)),
        surface_size
    ));
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Survival)
    );
}
