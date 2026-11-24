//! `/stopwatch` and `/execute if|unless stopwatch`.

use bbb_protocol::component_literal_nbt;
use bbb_protocol::NbtSummaryValue;

use crate::world::idle_timeout::{advance_clock_for_test, set_clock_for_test};

use super::*;

fn create_foo() -> CommandTestWorld {
    set_clock_for_test(1);
    let mut world = CommandTestWorld::default();
    let created = ran("stopwatch create foo", &mut world);
    assert_eq!(keys(&created), ["commands.stopwatch.create.success"]);
    assert_eq!(created.result, Some(1));
    world
}

#[test]
fn create_then_query_is_zero_at_the_same_clock() {
    let mut world = create_foo();
    let queried = ran("stopwatch query foo", &mut world);
    assert_eq!(keys(&queried), ["commands.stopwatch.query"]);
    assert_eq!(queried.result, Some(0));
    assert_eq!(
        arguments(&queried, 0),
        vec![
            component_literal_nbt("minecraft:foo"),
            NbtSummaryValue::Double(0.0_f64.to_bits()),
        ]
    );
}

#[test]
fn create_duplicate_refuses_already_exists() {
    let mut world = create_foo();
    let duplicate = ran("stopwatch create foo", &mut world);
    assert_eq!(keys(&duplicate), ["commands.stopwatch.already_exists"]);
    assert_eq!(duplicate.result, None);
}

#[test]
fn query_restart_remove_missing_refuse_does_not_exist() {
    for command in [
        "stopwatch query missing",
        "stopwatch restart missing",
        "stopwatch remove missing",
    ] {
        let run = ran(command, CommandTestWorld::default());
        assert_eq!(
            keys(&run),
            ["commands.stopwatch.does_not_exist"],
            "{command}"
        );
        assert_eq!(run.result, None, "{command}");
        assert_eq!(
            arguments(&run, 0),
            vec![component_literal_nbt("minecraft:missing")],
            "{command}"
        );
    }
}

#[test]
fn restart_zeroes_elapsed() {
    let mut world = create_foo();
    advance_clock_for_test(5_000);
    let before = ran("stopwatch query foo", &mut world);
    assert_eq!(before.result, Some(5));
    let restarted = ran("stopwatch restart foo", &mut world);
    assert_eq!(keys(&restarted), ["commands.stopwatch.restart.success"]);
    assert_eq!(restarted.result, Some(1));
    let after = ran("stopwatch query foo", &mut world);
    assert_eq!(after.result, Some(0));
    assert_eq!(
        arguments(&after, 0)[1],
        NbtSummaryValue::Double(0.0_f64.to_bits())
    );
}

#[test]
fn remove_then_query_missing() {
    let mut world = create_foo();
    let removed = ran("stopwatch remove foo", &mut world);
    assert_eq!(keys(&removed), ["commands.stopwatch.remove.success"]);
    assert_eq!(removed.result, Some(1));
    let queried = ran("stopwatch query foo", &mut world);
    assert_eq!(keys(&queried), ["commands.stopwatch.does_not_exist"]);
}

#[test]
fn query_scale_ten_returns_int_of_seconds_times_ten() {
    let mut world = create_foo();
    advance_clock_for_test(1_500);
    let queried = ran("stopwatch query foo 10", &mut world);
    assert_eq!(keys(&queried), ["commands.stopwatch.query"]);
    assert_eq!(queried.result, Some(15));
}

#[test]
fn stopwatch_is_hidden_from_a_moderator_and_allowed_for_a_gamemaster() {
    let refusal = refused("stopwatch create foo", CommandPermission::Moderator);
    assert_eq!(keys(&refusal), ["command.unknown.command", CURSOR_CONTEXT]);
    let allowed = ran_as(
        "stopwatch create foo",
        &source("alice", CommandPermission::GameMaster),
        CommandTestWorld::default(),
    );
    assert_eq!(keys(&allowed), ["commands.stopwatch.create.success"]);
    assert_eq!(allowed.result, Some(1));
}

#[test]
fn execute_if_stopwatch_missing_throws_does_not_exist() {
    for command in [
        "execute if stopwatch missing 0..",
        "execute unless stopwatch missing 0..",
    ] {
        let run = ran(command, CommandTestWorld::default());
        assert_eq!(
            keys(&run),
            ["commands.stopwatch.does_not_exist"],
            "{command}"
        );
        assert_eq!(run.result, None, "{command}");
        assert!(
            !keys(&run).contains(&"commands.execute.conditional.fail".to_string()),
            "{command} must not fail-closed as a false condition"
        );
    }
}

#[test]
fn execute_if_stopwatch_passes_after_create_and_unless_is_inverted() {
    let mut world = create_foo();
    let passed = ran("execute if stopwatch foo 0..", &mut world);
    assert_eq!(keys(&passed), ["commands.execute.conditional.pass"]);
    assert_eq!(passed.result, Some(1));
    let inverted = ran("execute unless stopwatch foo 0..", &mut world);
    assert_eq!(keys(&inverted), ["commands.execute.conditional.fail"]);
    assert_eq!(inverted.result, None);
    let too_high = ran("execute if stopwatch foo 1.5..", &mut world);
    assert_eq!(keys(&too_high), ["commands.execute.conditional.fail"]);
}

#[test]
fn float_range_reads_open_and_closed_ends_and_refuses_swapped() {
    assert_eq!(
        read_float_range(&("1.5..").encode_utf16().collect::<Vec<_>>(), 0).unwrap(),
        (
            5,
            ArgumentValue::FloatRange {
                min: Some(1.5),
                max: None
            }
        )
    );
    assert_eq!(
        read_float_range(&("..0").encode_utf16().collect::<Vec<_>>(), 0).unwrap(),
        (
            3,
            ArgumentValue::FloatRange {
                min: None,
                max: Some(0.0)
            }
        )
    );
    assert_eq!(
        reader_failure(read_float_range(
            &("2..1").encode_utf16().collect::<Vec<_>>(),
            0
        ))
        .as_deref(),
        Some("argument.range.swapped")
    );
    let swapped = ran("execute if stopwatch foo 2..1", CommandTestWorld::default());
    assert_eq!(keys(&swapped)[0], "argument.range.swapped");
}

#[test]
fn query_restart_remove_and_execute_suggest_current_ids() {
    let mut world = CommandTestWorld::default();
    world.setup("stopwatch create alpha");
    world.setup("stopwatch create beta");
    for command in [
        "stopwatch query ",
        "stopwatch restart ",
        "stopwatch remove ",
        "execute if stopwatch ",
        "execute unless stopwatch ",
    ] {
        assert_eq!(
            texts(&completions_for(command, CommandPermission::Owner, &world)),
            ["minecraft:alpha", "minecraft:beta"],
            "{command}"
        );
    }
    assert!(texts(&completions_for(
        "stopwatch create ",
        CommandPermission::Owner,
        &world
    ))
    .is_empty());
}
