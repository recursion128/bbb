use super::*;

fn pause_if_inactive(input: &mut ClientInputState, now: Instant, world: &mut WorldStore) {
    input.pause_if_inactive(now, world, &mut NetCounters::default(), &None);
}

#[test]
fn pause_if_inactive_opens_pause_screen_after_500ms_without_a_screen() {
    let mut input = ClientInputState::new(false);
    assert!(input.debug_pause_on_lost_focus());
    let mut world = WorldStore::new();
    // Pin the baseline the constructor read: comparing against a clock the
    // test never stated makes the delay the scheduler's, not the test's.
    let now = Instant::now();
    input.set_last_active_at_for_test(now);

    pause_if_inactive(&mut input, now + Duration::from_millis(501), &mut world);

    assert!(input.debug_pause_screen_is_open());
    assert_eq!(
        input
            .debug_pause_screen()
            .map(|screen| screen.show_pause_menu),
        Some(true)
    );
}

#[test]
fn pause_if_inactive_waits_out_the_500ms_delay_and_refreshes_while_active() {
    let mut input = ClientInputState::new(false);
    let mut world = WorldStore::new();
    let now = Instant::now();
    input.set_last_active_at_for_test(now);

    pause_if_inactive(&mut input, now + Duration::from_millis(400), &mut world);
    assert!(!input.debug_pause_screen_is_open());

    pause_if_inactive(&mut input, now + Duration::from_millis(600), &mut world);
    assert!(input.debug_pause_screen_is_open());
}

#[test]
fn pause_if_inactive_refreshes_the_delay_while_focused_and_respects_open_screen() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let now = Instant::now();
    // Focused frames refresh `lastActiveTime` (vanilla's else branch).
    pause_if_inactive(&mut input, now, &mut world);
    input.focused = false;
    pause_if_inactive(&mut input, now + Duration::from_millis(200), &mut world);
    assert!(!input.debug_pause_screen_is_open());

    pause_if_inactive(&mut input, now + Duration::from_millis(600), &mut world);
    assert!(input.debug_pause_screen_is_open());

    // An already-open screen suppresses re-opening (vanilla `pauseGame` acts
    // only when `screen == null`).
    let mut input = ClientInputState::new(false);
    input.open_debug_pause_screen_with_menu(&world, &mut Default::default(), &None);
    pause_if_inactive(&mut input, now + Duration::from_millis(600), &mut world);
    assert!(input.debug_pause_screen_is_open());
}

#[test]
fn focus_regained_command_in_the_same_step_prevents_the_pause_screen() {
    // Regression for `Minecraft.pauseIfInactive` ordering
    // the window was unfocused for >500 ms, and a
    // focus-regained command arrives in the same step. The command applies
    // first (`apply_focus` -> `input.focused = true`), then `pauseIfInactive`
    // observes the updated focus and must not open the pause screen.
    let mut input = ClientInputState::new(false);
    let mut world = WorldStore::new();
    let now = Instant::now();
    // The client has been unfocused since `now`, so by `now + 600 ms` it is
    // past the strict >500 ms delay.
    input.last_active_at = Some(now);

    handle_focus_change(
        &mut input,
        &mut world,
        &mut NetCounters::default(),
        &None,
        true,
    );
    pause_if_inactive(&mut input, now + Duration::from_millis(600), &mut world);

    assert!(!input.debug_pause_screen_is_open());
}

#[test]
fn pause_if_inactive_respects_chat_entry_as_an_open_screen() {
    // `pauseGame` suppresses on any current screen, not
    // only the runtime's narrow subset: an active chat/command entry must
    // survive lost focus without being replaced by the pause screen.
    let mut input = ClientInputState::new(false);
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let now = Instant::now();
    input.last_active_at = Some(now);
    open_chat_entry(
        &mut input,
        &mut world,
        &mut counters,
        &None,
        ChatMethod::Message,
        false,
    );
    assert!(input.command_entry_is_active());

    pause_if_inactive(&mut input, now + Duration::from_millis(600), &mut world);

    assert!(!input.debug_pause_screen_is_open());
    assert!(input.command_entry_is_active());
}
