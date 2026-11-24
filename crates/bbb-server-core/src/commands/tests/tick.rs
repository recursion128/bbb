//! `/tick`.

use super::*;

#[test]
fn tick_query_reports_the_status_the_rate_and_the_three_percentiles() {
    let samples = [1_000_000; crate::world::TICK_STATS_SPAN];
    let run = ran(
        "tick query",
        CommandTestWorld::default().with_tick_samples(&samples),
    );
    assert_eq!(
        keys(&run),
        [
            "commands.tick.status.running",
            "commands.tick.query.rate.running",
            "commands.tick.query.percentiles"
        ]
    );
    assert_eq!(
        arguments(&run, 1),
        vec![text("20.0"), text("1.0"), text("50.0")]
    );
    assert_eq!(
        arguments(&run, 2),
        vec![
            text("1.0"),
            text("1.0"),
            text("1.0"),
            NbtSummaryValue::Int(100)
        ]
    );
}

#[test]
fn a_frozen_server_says_so_and_a_sprinting_one_reports_a_different_rate_line() {
    let mut world = CommandTestWorld::default();
    world.setup("tick freeze");
    let frozen = ran("tick query", &mut world);
    assert_eq!(keys(&frozen)[0], "commands.tick.status.frozen");
    let lagging = ran(
        "tick query",
        CommandTestWorld::default().with_tick_samples(&[60_000_000; crate::world::TICK_STATS_SPAN]),
    );
    assert_eq!(keys(&lagging)[0], "commands.tick.status.lagging");
    world.setup("tick sprint 5t");
    let sprinting = ran("tick query", &mut world);
    assert_eq!(
        keys(&sprinting),
        [
            "commands.tick.status.sprinting",
            "commands.tick.query.rate.sprinting",
            "commands.tick.query.percentiles"
        ]
    );
    assert_eq!(arguments(&sprinting, 1), vec![text("20.0"), text("0.0")]);
}

#[test]
fn the_millisecond_strings_are_rounded_in_single_precision_like_vanilla() {
    let run = ran(
        "tick query",
        CommandTestWorld::default().with_tick_samples(&[25_050_000; crate::world::TICK_STATS_SPAN]),
    );
    assert_eq!(arguments(&run, 1)[1], text("25.0"));
}

#[test]
fn step_and_step_stop_refuse_from_the_state_that_makes_them_impossible() {
    let mut world = CommandTestWorld::default();
    let running = ran("tick step 5t", &mut world);
    assert_eq!(keys(&running), ["commands.tick.step.fail"]);
    assert_eq!(kinds(&running), [CommandFeedbackKind::Failure]);
    let nothing_to_stop = ran("tick step stop", &mut world);
    assert_eq!(keys(&nothing_to_stop), ["commands.tick.step.stop.fail"]);
    world.setup("tick freeze");
    let frozen = ran("tick step 5t", &mut world);
    assert_eq!(keys(&frozen), ["commands.tick.step.success"]);
    assert_eq!(arguments(&frozen, 0), vec![NbtSummaryValue::Int(5)]);
    assert!(world.world().tick_rate().is_stepping_forward());
    let stepping = ran("tick step stop", &mut world);
    assert_eq!(keys(&stepping), ["commands.tick.step.stop.success"]);
    assert!(!world.world().tick_rate().is_stepping_forward());
}

#[test]
fn sprint_stop_refuses_unless_a_sprint_is_still_running() {
    let mut world = CommandTestWorld::default();
    let idle = ran("tick sprint stop", &mut world);
    assert_eq!(keys(&idle), ["commands.tick.sprint.stop.fail"]);
    world.setup("tick sprint 5t");
    let running = ran("tick sprint stop", &mut world);
    assert_eq!(keys(&running), ["commands.tick.sprint.stop.success"]);
    assert!(!world.world().tick_rate().has_remaining_sprint_ticks());
}

#[test]
fn a_sprint_that_interrupts_another_one_says_so_first() {
    let mut world = CommandTestWorld::default();
    let fresh = ran("tick sprint 1t", &mut world);
    assert_eq!(keys(&fresh), ["commands.tick.status.sprinting"]);
    let interrupting = ran("tick sprint 1t", &mut world);
    assert_eq!(
        keys(&interrupting),
        [
            "commands.tick.sprint.stop.success",
            "commands.tick.status.sprinting"
        ]
    );
    assert!(world.world().tick_rate().has_remaining_sprint_ticks());
}

#[test]
fn the_rate_is_reported_with_one_decimal_and_the_bounds_are_the_published_ones() {
    let run = ran("tick rate 40", CommandTestWorld::default());
    assert_eq!(keys(&run), ["commands.tick.rate.success"]);
    // `String.format(Locale.ROOT, "%.1f", rate)` (TickCommand.java:64).
    assert_eq!(arguments(&run, 0), vec![text("40.0")]);
    // `FloatArgumentType.floatArg(1.0F, 10000.0F)` (TickCommand.java:28) is the
    // published template, so the parser refuses either side of it.
    let too_low = refused("tick rate 0.5", CommandPermission::Owner);
    assert_eq!(
        failure_messages(&too_low)
            .into_iter()
            .filter_map(translate_key)
            .collect::<Vec<_>>(),
        ["argument.float.low"]
    );
    let too_high = refused("tick rate 10001", CommandPermission::Owner);
    assert_eq!(
        failure_messages(&too_high)
            .into_iter()
            .filter_map(translate_key)
            .collect::<Vec<_>>(),
        ["argument.float.big"]
    );
}
