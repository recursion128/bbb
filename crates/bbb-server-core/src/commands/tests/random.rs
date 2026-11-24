//! `/random`.

use super::*;

fn assert_in_range(run: &CommandRun, min: i32, max: i32) -> i32 {
    let value = run.result.expect("a valid draw succeeds");
    assert!((min..=max).contains(&value));
    value
}

/// `span == 0` and `span >= 2147483647` (RandomCommand.java:144-150). A single
/// number is both bounds (`MinMaxBounds.Bounds.fromReader`, `:155-156`).
#[test]
fn random_refuses_a_zero_span_and_a_span_that_does_not_fit_an_int() {
    let too_small = ran("random value 5", CommandTestWorld::default());
    assert_eq!(keys(&too_small), ["commands.random.error.range_too_small"]);
    assert_eq!(too_small.result, None);

    let same_ends = ran("random roll 0..0", CommandTestWorld::default());
    assert_eq!(keys(&same_ends), ["commands.random.error.range_too_small"]);

    // `0..2147483647` is `span == 2147483647`.
    let too_large = ran("random value 0..2147483647", CommandTestWorld::default());
    assert_eq!(keys(&too_large), ["commands.random.error.range_too_large"]);

    let swapped = ran("random value 5..1", CommandTestWorld::default());
    assert_eq!(
        translate_key(failure_messages(&swapped)[0]).as_deref(),
        Some("argument.range.swapped")
    );
}

/// `value` is `sendSuccess(.., false)` (RandomCommand.java:158); `roll` is
/// `broadcastSystemMessage` (`:153-156`) and names the source.
#[test]
fn random_value_reports_to_the_source_and_roll_broadcasts() {
    let mut world = world(&[user("alice", 1)]);
    let value = ran("random value 1..10", &mut world);
    let drawn = assert_in_range(&value, 1, 10);
    assert_eq!(keys(&value), ["commands.random.sample.success"]);
    assert_eq!(arguments(&value, 0), vec![NbtSummaryValue::Int(drawn)]);
    assert!(!value.feedback[0].inform_admins);
    assert!(value.broadcast.is_empty());
    let roll = ran("random roll 1..10", &mut world);
    let drawn = assert_in_range(&roll, 1, 10);
    assert!(roll.feedback.is_empty());
    assert_eq!(roll.broadcast.len(), 1);
    assert_eq!(
        system_broadcast_message(&roll.broadcast[0]),
        &compound(vec![
            ("translate", text("commands.random.roll")),
            (
                "with",
                NbtSummaryValue::List(vec![
                    display_name("alice", 1),
                    NbtSummaryValue::Int(drawn),
                    NbtSummaryValue::Int(1),
                    NbtSummaryValue::Int(10)
                ])
            ),
        ])
    );
}

/// A named sequence draws from `getRandomSequence` (RandomCommand.java:136);
/// no name draws from `source.getLevel().getRandom()` (`:138-139`).
#[test]
fn random_draws_from_a_live_sequence_or_the_sources_level() {
    let mut world = CommandTestWorld::default();
    assert_in_range(&ran("random value 0..5 test", &mut world), 0, 5);
    assert_eq!(world.random_sequence_keys(), ["minecraft:test"]);
    assert_in_range(&ran("random value 0..5 test", &mut world), 0, 5);
    assert_eq!(world.random_sequence_keys(), ["minecraft:test"]);
    let run = ran("execute in the_nether run random value 1..10", &mut world);
    assert_in_range(&run, 1, 10);
    assert_eq!(keys(&run), ["commands.random.sample.success"]);
    assert_eq!(world.random_sequence_keys(), ["minecraft:test"]);
}

/// Bare reset uses the current defaults; a typed seed carries the trio whose
/// missing booleans default to true (RandomCommand.java:164-178, `:80`, `:86`).
#[test]
fn random_reset_of_one_sequence_is_bare_or_an_explicit_seed_trio() {
    let mut world = CommandTestWorld::default();
    for command in [
        "random reset foo",
        "random reset foo 7",
        "random reset foo 7 false",
        "random reset foo 7 false true",
    ] {
        let run = ran(command, &mut world);
        assert_eq!(run.result, Some(1));
        assert_eq!(keys(&run), ["commands.random.reset.success"]);
        assert_eq!(arguments(&run, 0), vec![text("minecraft:foo")]);
        assert!(!run.feedback[0].inform_admins);
        assert_eq!(world.random_sequence_keys(), ["minecraft:foo"]);
    }
}

/// Reset-all reports what `clear` removed (RandomCommand.java:180-184); a
/// typed seed moves the defaults first (`:190-191`).
#[test]
fn random_reset_all_reports_the_cleared_count_and_stores_defaults() {
    let mut world = CommandTestWorld::default();
    world.setup("random value 0..5 a");
    world.setup("random value 0..5 b");
    let bare = ran("random reset *", &mut world);
    assert_eq!(bare.result, Some(2));
    assert_eq!(keys(&bare), ["commands.random.reset.all.success"]);
    assert_eq!(arguments(&bare, 0), vec![NbtSummaryValue::Int(2)]);
    assert!(!bare.feedback[0].inform_admins);
    assert!(world.random_sequence_keys().is_empty());
    world.setup("random value 0..5 a");
    world.setup("random value 0..5 b");
    let seeded = ran("random reset * 9 false", &mut world);
    assert_eq!(seeded.result, Some(2));
    assert_eq!(world.random_sequence_defaults(), (9, false, true));
    assert!(world.random_sequence_keys().is_empty());
    let empty = ran("random reset *", &mut world);
    assert_eq!(empty.result, Some(0));
}

/// `reset` requires `LEVEL_GAMEMASTERS` (RandomCommand.java:44); so does a
/// named sequence on `value`/`roll` (`:121`). The draw itself does not.
#[test]
fn random_reset_and_a_named_sequence_need_gamemaster() {
    let value = ran_as(
        "random value 1..10",
        &source("alice", CommandPermission::All),
        CommandTestWorld::default(),
    );
    assert_in_range(&value, 1, 10);

    let roll = ran_as(
        "random roll 1..10",
        &source("alice", CommandPermission::All),
        CommandTestWorld::default(),
    );
    assert_in_range(&roll, 1, 10);

    let named = refused("random value 1..10 foo", CommandPermission::All);
    assert_eq!(keys(&named), ["command.unknown.argument", CURSOR_CONTEXT]);

    let reset = refused("random reset *", CommandPermission::Moderator);
    assert_eq!(keys(&reset), ["command.unknown.argument", CURSOR_CONTEXT]);

    let allowed = ran_as(
        "random reset *",
        &source("alice", CommandPermission::GameMaster),
        CommandTestWorld::default(),
    );
    assert_eq!(allowed.result, Some(0));
}

/// `suggestRandomSequence` (RandomCommand.java:127-131) offers live keys
/// through `SharedSuggestionProvider.suggest`, not `suggestResource`.
#[test]
fn random_offers_the_live_sequence_keys() {
    let mut world = CommandTestWorld::default();
    world.setup("random value 0..5 alpha");
    world.setup("random value 0..5 beta");
    assert_eq!(
        texts(&completions_for(
            "random reset ",
            CommandPermission::GameMaster,
            &world
        )),
        ["*", "minecraft:alpha", "minecraft:beta"]
    );
    assert!(texts(&completions_for(
        "random value 0..5 alpha",
        CommandPermission::GameMaster,
        &world
    ))
    .is_empty());
    assert_eq!(
        texts(&completions_for(
            "random value 0..5 minecraft:a",
            CommandPermission::GameMaster,
            &world
        )),
        ["minecraft:alpha"]
    );
}
