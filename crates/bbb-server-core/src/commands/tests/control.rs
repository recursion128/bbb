//! `/return` and the execution context every command now runs on.

use bbb_protocol::game_rules::GameRuleValue;

use super::*;

fn gamemaster() -> CommandSource {
    source("alice", CommandPermission::GameMaster)
}

#[test]
fn the_context_opens_on_the_two_quota_game_rules() {
    let defaults = CommandTestWorld::default();
    assert_eq!(
        ExecutionContext::open(defaults.world()).quotas(),
        (65536, 65536)
    );
    let world = CommandTestWorld::default().with_rules(&[
        (
            "minecraft:max_command_sequence_length",
            GameRuleValue::Integer(7),
        ),
        ("minecraft:max_command_forks", GameRuleValue::Integer(3)),
    ]);
    assert_eq!(ExecutionContext::open(world.world()).quotas(), (7, 3));
    let world = CommandTestWorld::default().with_rules(&[(
        "minecraft:max_command_sequence_length",
        GameRuleValue::Integer(0),
    )]);
    assert_eq!(ExecutionContext::open(world.world()).quotas().0, 1);
}

#[test]
fn return_needs_gamemaster_and_is_otherwise_three_nodes() {
    // `ReturnCommand.register` (ReturnCommand.java:20-30): the root requires
    // `LEVEL_GAMEMASTERS`, and `requires` removes the whole subtree from
    // parsing for anyone below it.
    for command in ["return 3", "return fail", "return run list"] {
        assert_eq!(
            keys(&refused(command, CommandPermission::Moderator)),
            ["command.unknown.command", CURSOR_CONTEXT],
            "{command}"
        );
    }
    for command in ["return 3", "return fail"] {
        let run = ran_as(command, &gamemaster(), CommandTestWorld::default());
        assert!(run.feedback.is_empty(), "{command}: {run:?}");
    }
}

#[test]
fn return_at_the_top_frame_reports_nothing_and_returns_nothing() {
    // Both executors set the running frame's return value and discard it
    // (ReturnCommand.java:32-39, `:62-70`). At the frame
    // `queueInitialCommandExecution` opened, the consumer is
    // `CommandResultCallback.EMPTY` and the control is `commandQueue::clear`
    // (ExecutionContext.java:38-41), so neither is observable.
    //
    // A custom executor also never reaches `ContextChain.runExecutable`
    // (BuildContexts.java:102-107), which is why the run carries no result.
    for command in ["return 3", "return fail"] {
        let run = ran_as(command, &gamemaster(), CommandTestWorld::default());
        assert_eq!(run.result, None, "{command}");
        assert_eq!(run.successful_contexts, 0, "{command}");
    }
}

#[test]
fn return_run_runs_the_command_after_it() {
    // `ReturnFromCommandCustomModifier.apply` (ReturnCommand.java:41-59)
    // discards the frame it is running in and queues the rest of the chain as a
    // `BuildContexts.Continuation` with `setReturn`, so the inner command runs
    // exactly as it would have on its own.
    let players = [user("alice", 1)];
    let run = ran_as("return run list", &gamemaster(), world(&players));
    assert_eq!(keys(&run), ["commands.list.players"]);
    assert_eq!(run.result, Some(1));

    let run = ran_as("return run say hi", &gamemaster(), world(&players));
    assert_eq!(run.broadcast.len(), 1, "{run:?}");
}

#[test]
fn return_run_carries_the_dimension_the_stage_before_it_set() {
    let mut world = CommandTestWorld::default()
        .with_border(
            Java26BuiltinDimension::Overworld.into(),
            super::worldborder::static_border(100.0),
        )
        .with_border(
            Java26BuiltinDimension::Nether.into(),
            super::worldborder::static_border(50.0),
        );
    let run = ran_as(
        "execute in the_nether run return run worldborder get",
        &gamemaster(),
        &mut world,
    );
    assert_eq!(keys(&run), ["commands.worldborder.get"]);
    assert_eq!(run.result, Some(50));
}
