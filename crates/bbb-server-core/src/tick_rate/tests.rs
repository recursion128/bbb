use super::*;

#[test]
fn a_fresh_manager_runs_every_tick() {
    let mut manager = TickRateManager::default();
    assert!(!manager.is_frozen());
    assert!(manager.runs_normally());
    for _ in 0..5 {
        manager.tick();
        assert!(manager.runs_normally());
    }
    assert_eq!(manager.tickrate(), DEFAULT_TICKRATE);
    assert_eq!(manager.nanoseconds_per_tick(), 50_000_000);
}

#[test]
fn freezing_stops_the_world_on_the_next_tick_not_this_one() {
    let mut manager = TickRateManager::default();
    manager.set_frozen(true);
    // `runGameElements` is only recomputed by `tick`
    // (`TickRateManager.java:56-57`), so the tick that is already under way
    // still counts as running.
    assert!(manager.runs_normally());
    manager.tick();
    assert!(!manager.runs_normally());
}

#[test]
fn a_step_runs_exactly_the_ticks_it_asked_for() {
    let mut manager = TickRateManager::default();
    manager.set_frozen(true);
    manager.tick();
    assert!(!manager.runs_normally());

    assert!(manager.step_game_if_paused(3));
    assert!(manager.is_stepping_forward());
    assert_eq!(manager.frozen_ticks_to_run(), 3);

    let mut ran = 0;
    for _ in 0..6 {
        manager.tick();
        if manager.runs_normally() {
            ran += 1;
        }
    }
    assert_eq!(ran, 3);
    assert!(!manager.is_stepping_forward());
}

#[test]
fn stepping_is_refused_unless_the_game_is_frozen() {
    let mut manager = TickRateManager::default();
    assert!(!manager.step_game_if_paused(10));
    assert_eq!(manager.frozen_ticks_to_run(), 0);
}

#[test]
fn stopping_a_step_only_succeeds_while_one_is_running() {
    let mut manager = TickRateManager::default();
    assert!(!manager.stop_stepping());
    manager.set_frozen(true);
    manager.tick();
    assert!(manager.step_game_if_paused(5));
    assert!(manager.stop_stepping());
    assert_eq!(manager.frozen_ticks_to_run(), 0);
    assert!(!manager.stop_stepping());
}

#[test]
fn the_tick_rate_never_falls_below_the_minimum() {
    let mut manager = TickRateManager::default();
    manager.set_tick_rate(0.5);
    assert_eq!(manager.tickrate(), MIN_TICKRATE);
    assert_eq!(manager.nanoseconds_per_tick(), 1_000_000_000);
    manager.set_tick_rate(40.0);
    assert_eq!(manager.tickrate(), 40.0);
    assert_eq!(manager.nanoseconds_per_tick(), 25_000_000);
    assert_eq!(manager.milliseconds_per_tick(), 25.0);
}

#[test]
fn a_sprint_unfreezes_for_its_duration_and_freezes_back_afterwards() {
    let mut manager = TickRateManager::default();
    manager.set_frozen(true);
    manager.tick();
    assert!(!manager.runs_normally());

    assert!(!manager.request_game_to_sprint(2));
    assert!(manager.is_sprinting());
    assert!(!manager.is_frozen());

    let mut now = 0;
    let mut ran = 0;
    let mut report = None;
    for _ in 0..4 {
        manager.tick();
        if !manager.is_sprinting() {
            continue;
        }
        let (sprints, finished) = manager.check_should_sprint_this_tick(now);
        if sprints {
            ran += 1;
            now += 5_000_000;
            manager.end_tick_work(now);
        }
        if finished.is_some() {
            report = finished;
        }
    }
    assert_eq!(ran, 2);
    let report = report.expect("the sprint finishes on the tick after its last one");
    assert_eq!(report.ticks_per_second, 200);
    assert_eq!(report.milliseconds_per_tick, "5.00");
    // `previousIsFrozen` was captured before the sprint unfroze the game
    // (`ServerTickRateManager.java:75-76`), so the freeze comes back.
    assert!(manager.is_frozen());
}

#[test]
fn interrupting_a_sprint_says_so_and_reports_what_ran() {
    let mut manager = TickRateManager::default();
    assert!(!manager.request_game_to_sprint(100));
    manager.tick();
    let (sprints, finished) = manager.check_should_sprint_this_tick(0);
    assert!(sprints);
    assert!(finished.is_none());
    manager.end_tick_work(2_000_000);

    // A second request while one is running is the interrupted case
    // (`ServerTickRateManager.java:71`).
    assert!(manager.request_game_to_sprint(10));
    let report = manager.stop_sprinting().expect("a running sprint reports");
    assert_eq!(report.ticks_per_second, 0);
    assert!(manager.stop_sprinting().is_none());
}

#[test]
fn a_sprint_that_never_ran_a_tick_reports_the_configured_rate() {
    let mut manager = TickRateManager::default();
    manager.set_tick_rate(20.0);
    assert!(!manager.request_game_to_sprint(5));
    let report = manager
        .stop_sprinting()
        .expect("a scheduled sprint reports");
    assert_eq!(report.ticks_per_second, 0);
    // `completedTicks == 0` falls back to `millisecondsPerTick()`
    // (`ServerTickRateManager.java:84-86`).
    assert_eq!(report.milliseconds_per_tick, "50.00");
}

/// `String.format(Locale.ROOT, "%.0f", 64.0)` is `64`, not `64.0`, and
/// `commands.worldborder.get` (`WorldBorderCommand.java:169-173`) reports its
/// width through exactly that call.
#[test]
fn zero_decimals_writes_no_radix_point() {
    assert_eq!(java_fixed_decimals(64.0, 0), "64");
    assert_eq!(java_fixed_decimals(0.0, 0), "0");
    assert_eq!(java_fixed_decimals(-1.5, 0), "-2");
    assert_eq!(java_fixed_decimals(1.5, 0), "2");
    assert_eq!(java_fixed_decimals(20.0, 1), "20.0");
    assert_eq!(java_fixed_decimals(1.25, 2), "1.25");
}
