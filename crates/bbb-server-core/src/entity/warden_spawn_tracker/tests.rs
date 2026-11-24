use super::{WardenSpawnTracker, DECREASE_WARNING_LEVEL_EVERY_INTERVAL};

#[test]
fn the_decay_lowers_the_level_after_twelve_thousand_ticks() {
    let mut tracker = WardenSpawnTracker::new(0, 3, 0);
    for tick in 0..=DECREASE_WARNING_LEVEL_EVERY_INTERVAL {
        assert_eq!(tracker.warning_level(), 3, "tick {tick}");
        tracker.tick();
    }
    assert_eq!(tracker.warning_level(), 2);
    assert_eq!(tracker.ticks_since_last_warning(), 0);

    // And the clamp floor stops the decay at zero.
    let mut tracker = WardenSpawnTracker::new(DECREASE_WARNING_LEVEL_EVERY_INTERVAL, 0, 0);
    tracker.tick();
    assert_eq!(tracker.warning_level(), 0);
}

#[test]
fn a_fresh_tracker_is_zero_and_its_counters_are_readable() {
    let fresh = WardenSpawnTracker::default();
    assert_eq!(fresh, WardenSpawnTracker::new(0, 0, 0));
    assert_eq!(fresh.ticks_since_last_warning(), 0);
    assert_eq!(fresh.warning_level(), 0);
    assert_eq!(fresh.cooldown_ticks(), 0);
    assert!(!fresh.on_cooldown());

    let mut tracker = WardenSpawnTracker::new(7, 3, 9);
    assert_eq!(tracker.ticks_since_last_warning(), 7);
    assert_eq!(tracker.cooldown_ticks(), 9);
    tracker.reset();
    assert_eq!(tracker, WardenSpawnTracker::default());
}
