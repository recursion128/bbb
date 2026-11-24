use super::*;
use crate::input::stats_screen::{
    stats_screen_item_column_at_position, stats_screen_tab_at_position,
};

fn stats(input: &ClientInputState) -> &StatsScreen {
    input.stats_screen().unwrap()
}

fn stats_mut(input: &mut ClientInputState) -> &mut StatsScreen {
    input.stats_screen_mut().unwrap()
}

#[test]
fn stats_screen_done_button_closes_stats_screen() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    open_test_stats_screen(&mut input);

    assert!(
        input.handle_stats_screen_cursor_moved(Some(PhysicalPosition::new(70.0, 223.0)), surface)
    );
    assert_eq!(
        input.stats_screen().and_then(|stats| stats.cursor_position),
        Some((70, 223))
    );
    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(70.0, 223.0)),
        surface,
    ));

    assert!(!input.stats_screen_is_open());
    assert!(!input.stats_screen_is_open());
}

#[test]
fn stats_screen_wheel_uses_half_row_pixels_and_resets_on_close() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    open_test_stats_screen(&mut input);
    stats_mut(&mut input).sync_scroll_metrics(100, 32);

    assert!(input.handle_stats_screen_mouse_wheel(MouseScrollDelta::LineDelta(0.0, -3.0)));
    assert_eq!(stats(&input).scroll_amount, 21.0);
    assert!(
        input.handle_stats_screen_mouse_wheel(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
            0.0, -2.5
        )))
    );
    assert_eq!(stats(&input).scroll_amount, 23.5);
    assert!(input.handle_stats_screen_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0)));
    assert_eq!(stats(&input).scroll_amount, 16.5);

    stats_mut(&mut input).sync_scroll_metrics(10, 32);
    assert_eq!(stats(&input).scroll_amount, 10.0);
    assert!(input.handle_stats_screen_mouse_wheel(MouseScrollDelta::LineDelta(0.0, 10.0)));
    assert_eq!(stats(&input).scroll_amount, 0.0);

    input.close_stats_screen(&mut counters, &mut world, &None);
    assert!(!input.stats_screen_is_open());
    assert!(!input.handle_stats_screen_mouse_wheel(MouseScrollDelta::LineDelta(0.0, -1.0)));
}

#[test]
fn stats_screen_scrollbar_drag_tracks_pixel_scroll_bounds() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    open_test_stats_screen(&mut input);
    stats_mut(&mut input).sync_scroll_metrics(140, 80);
    assert!(
        input.handle_stats_screen_cursor_moved(Some(PhysicalPosition::new(308.0, 60.0)), surface)
    );
    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(308.0, 60.0)),
        surface,
    ));

    assert!(
        input.handle_stats_screen_cursor_moved(Some(PhysicalPosition::new(308.0, 70.0)), surface)
    );
    assert!((stats(&input).scroll_amount - 140.0 * 10.0 / (183.0 - 80.0)).abs() < 1e-6);
    assert!(
        input.handle_stats_screen_cursor_moved(Some(PhysicalPosition::new(308.0, 20.0)), surface)
    );
    assert_eq!(stats(&input).scroll_amount, 0.0);
    assert!(
        input.handle_stats_screen_cursor_moved(Some(PhysicalPosition::new(308.0, 220.0)), surface)
    );
    assert_eq!(stats(&input).scroll_amount, 140.0);

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Released,
        Some(PhysicalPosition::new(308.0, 220.0)),
        surface,
    ));
    assert!(
        input.handle_stats_screen_cursor_moved(Some(PhysicalPosition::new(308.0, 100.0)), surface)
    );
    assert_eq!(stats(&input).scroll_amount, 140.0);
}

#[test]
fn stats_screen_tab_hitboxes_match_renderer_column_widths() {
    let surface = PhysicalSize::new(320, 240);
    assert_eq!(
        stats_screen_tab_at_position(Some((111, 12)), surface),
        Some(0)
    );
    assert_eq!(
        stats_screen_tab_at_position(Some((112, 12)), surface),
        Some(1)
    );
    assert_eq!(
        stats_screen_tab_at_position(Some((209, 12)), surface),
        Some(1)
    );
    assert_eq!(
        stats_screen_tab_at_position(Some((210, 12)), surface),
        Some(2)
    );
    assert_eq!(stats_screen_tab_at_position(Some((20, 42)), surface), None);
    let wide = PhysicalSize::new(500, 350);
    assert_eq!(stats_screen_tab_at_position(Some((63, 12)), wide), None);
    assert_eq!(stats_screen_tab_at_position(Some((64, 12)), wide), Some(0));
    assert_eq!(stats_screen_tab_at_position(Some((188, 12)), wide), Some(1));
    assert_eq!(stats_screen_tab_at_position(Some((312, 12)), wide), Some(2));
    assert_eq!(
        stats_screen_item_column_at_position(Some((77, 27)), surface, 0),
        Some(0)
    );
    assert_eq!(
        stats_screen_item_column_at_position(Some((94, 44)), surface, 0),
        Some(0)
    );
    assert_eq!(
        stats_screen_item_column_at_position(Some((95, 28)), surface, 0),
        None
    );
    assert_eq!(
        stats_screen_item_column_at_position(Some((157, 28)), surface, 0),
        Some(2)
    );
    assert_eq!(
        stats_screen_item_column_at_position(Some((157, 28)), surface, 10),
        Some(2)
    );
    assert_eq!(
        stats_screen_item_column_at_position(Some((157, 28)), surface, 22),
        None
    );
}

#[test]
fn stats_screen_control_tab_and_digits_skip_disabled_tabs() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    open_test_stats_screen(&mut input);
    world.apply_award_stats(AwardStats {
        stats: vec![
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_ITEM_CRAFTED_ID,
                value_id: bbb_protocol::item_types::vanilla_item_type_for_resource_id(
                    "minecraft:apple",
                )
                .unwrap()
                .id,
                amount: 1,
            },
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_ENTITY_KILLED_ID,
                value_id: VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                amount: 1,
            },
        ],
    });
    input.apply_screen_requests(&mut world, &mut counters, &None);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Digit2),
        ElementState::Pressed,
    );
    assert_eq!(stats(&input).tab, 1);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    assert_eq!(stats(&input).tab, 2);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Numpad1),
        ElementState::Pressed,
    );
    assert_eq!(stats(&input).tab, 0);
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Digit9),
        ElementState::Pressed,
    );
    assert_eq!(stats(&input).tab, 2);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    assert_eq!(stats(&input).tab, 1);

    let mut empty_world = WorldStore::new();
    empty_world.apply_award_stats(AwardStats { stats: Vec::new() });
    stats_mut(&mut input).tab = 0;
    handle_key_input(
        &mut input,
        &mut counters,
        &mut empty_world,
        &None,
        PhysicalKey::Code(KeyCode::Digit3),
        ElementState::Pressed,
    );
    assert_eq!(stats(&input).tab, 0);
    handle_key_input(
        &mut input,
        &mut counters,
        &mut empty_world,
        &None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    assert_eq!(stats(&input).tab, 0);
}

#[test]
fn stats_screen_focus_matches_loading_replacement_and_widget_tab_order() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    input.open_stats_screen(&mut counters, &mut world, &None);

    stats_mut(&mut input).sync_interaction(true, 0, 0);
    assert_eq!(stats(&input).focus, StatsScreenFocus::None);
    stats_mut(&mut input).sync_interaction(false, 0, 3);
    assert_eq!(stats(&input).focus, StatsScreenFocus::Tabs);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    assert_eq!(stats(&input).focus, StatsScreenFocus::GeneralRow(0));
    stats_mut(&mut input).move_tab_focus(true);
    assert_eq!(stats(&input).focus, StatsScreenFocus::Done);
    stats_mut(&mut input).move_tab_focus(true);
    assert_eq!(stats(&input).focus, StatsScreenFocus::Tabs);

    stats_mut(&mut input).tab = 1;
    stats_mut(&mut input).sync_interaction(false, 1, 2);
    stats_mut(&mut input).focus = StatsScreenFocus::Tabs;
    stats_mut(&mut input).move_tab_focus(true);
    assert_eq!(stats(&input).focus, StatsScreenFocus::ItemHeader(0));
    for column in 1..6 {
        stats_mut(&mut input).move_tab_focus(true);
        assert_eq!(stats(&input).focus, StatsScreenFocus::ItemHeader(column));
    }
    stats_mut(&mut input).move_tab_focus(true);
    assert_eq!(stats(&input).focus, StatsScreenFocus::ItemRow(0));
    stats_mut(&mut input).move_tab_focus(true);
    assert_eq!(stats(&input).focus, StatsScreenFocus::ItemRow(1));
    stats_mut(&mut input).move_tab_focus(true);
    assert_eq!(stats(&input).focus, StatsScreenFocus::Done);
    stats_mut(&mut input).move_tab_focus(false);
    assert_eq!(stats(&input).focus, StatsScreenFocus::ItemRow(1));
}

#[test]
fn stats_screen_focus_arrow_scroll_and_activation_follow_widget_paths() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    input.open_stats_screen(&mut counters, &mut world, &None);
    stats_mut(&mut input).was_loading = false;
    stats_mut(&mut input).tab = 1;
    stats_mut(&mut input).sync_interaction(false, 1, 12);
    stats_mut(&mut input).focus = StatsScreenFocus::ItemHeader(0);

    stats_mut(&mut input).move_arrow_focus(&world, KeyCode::ArrowRight);
    assert_eq!(stats(&input).focus, StatsScreenFocus::ItemHeader(1));
    assert!(!stats_mut(&mut input).activate_focus());
    assert_eq!(stats(&input).item_sort(), (Some(1), -1));
    assert!(!stats_mut(&mut input).activate_focus());
    assert_eq!(stats(&input).item_sort(), (Some(1), 1));
    assert!(!stats_mut(&mut input).activate_focus());
    assert_eq!(stats(&input).item_sort(), (None, 0));

    stats_mut(&mut input).move_arrow_focus(&world, KeyCode::ArrowDown);
    assert_eq!(stats(&input).focus, StatsScreenFocus::ItemRow(0));
    for _ in 0..11 {
        stats_mut(&mut input).move_arrow_focus(&world, KeyCode::ArrowDown);
    }
    assert_eq!(stats(&input).focus, StatsScreenFocus::ItemRow(11));
    stats_mut(&mut input).sync_scroll_metrics(200, 40);
    stats_mut(&mut input).keep_focus_visible(66, 22);
    assert_eq!(stats(&input).scroll_amount, 222.0_f64.min(200.0));
    stats_mut(&mut input).move_arrow_focus(&world, KeyCode::ArrowDown);
    assert_eq!(stats(&input).focus, StatsScreenFocus::Done);
    assert!(stats_mut(&mut input).activate_focus());
}

#[test]
fn stats_screen_focus_mouse_clicks_rows_headers_and_tabs() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    open_test_stats_screen(&mut input);
    world.apply_award_stats(AwardStats { stats: Vec::new() });
    input.apply_screen_requests(&mut world, &mut counters, &None);
    stats_mut(&mut input).tab = 0;
    stats_mut(&mut input).sync_interaction(false, 0, 20);

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(160.0, 42.0)),
        surface,
    ));
    assert_eq!(stats(&input).focus, StatsScreenFocus::GeneralRow(1));

    stats_mut(&mut input).tab = 1;
    stats_mut(&mut input).sync_interaction(false, 1, 3);
    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(117.0, 28.0)),
        surface,
    ));
    assert_eq!(stats(&input).focus, StatsScreenFocus::ItemHeader(1));

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(70.0, 12.0)),
        surface,
    ));
    assert_eq!(stats(&input).focus, StatsScreenFocus::Tabs);
}

#[test]
fn escape_key_closes_stats_screen_without_requesting_stats_again() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    open_test_stats_screen(&mut input);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!input.stats_screen_is_open());
    assert_eq!(counters.request_stats_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}
