//! `/stopwatch` (StopwatchCommand.java:29-114).

use bbb_protocol::{component_literal_nbt, NbtSummaryValue};

use crate::commands::parse::ParseResult;
use crate::commands::{translatable, CommandCx};
use crate::world::idle_timeout::monotonic_millis;
use crate::world::ServerWorld;

use super::{identifier_argument, CommandRun, Outcome};

fn stopwatch_id(parse: &ParseResult) -> String {
    identifier_argument(parse, "id").to_string()
}

fn does_not_exist(id: &str) -> NbtSummaryValue {
    translatable(
        "commands.stopwatch.does_not_exist",
        vec![component_literal_nbt(id)],
    )
}

fn already_exists(id: &str) -> NbtSummaryValue {
    translatable(
        "commands.stopwatch.already_exists",
        vec![component_literal_nbt(id)],
    )
}

/// `StopwatchCommand.createStopwatch` (StopwatchCommand.java:68-78).
pub(super) fn create_stopwatch(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let id = stopwatch_id(parse);
    if !cx.world.create_stopwatch(id.clone()) {
        return Err(already_exists(&id));
    }
    run.succeed(
        translatable(
            "commands.stopwatch.create.success",
            vec![component_literal_nbt(&id)],
        ),
        true,
    );
    Ok(1)
}

/// `StopwatchCommand.queryStopwatch` (StopwatchCommand.java:80-92).
pub(super) fn query_stopwatch(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &ServerWorld,
    scale: f64,
) -> Outcome {
    let id = stopwatch_id(parse);
    let Some(stopwatch) = world.stopwatch(&id) else {
        return Err(does_not_exist(&id));
    };
    let elapsed_seconds = stopwatch.elapsed_seconds(monotonic_millis());
    run.succeed(
        translatable(
            "commands.stopwatch.query",
            vec![
                component_literal_nbt(&id),
                NbtSummaryValue::Double(elapsed_seconds.to_bits()),
            ],
        ),
        true,
    );
    Ok((elapsed_seconds * scale) as i32)
}

/// `StopwatchCommand.restartStopwatch` (StopwatchCommand.java:94-103).
pub(super) fn restart_stopwatch(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let id = stopwatch_id(parse);
    if !cx.world.restart_stopwatch(&id) {
        return Err(does_not_exist(&id));
    }
    run.succeed(
        translatable(
            "commands.stopwatch.restart.success",
            vec![component_literal_nbt(&id)],
        ),
        true,
    );
    Ok(1)
}

/// `StopwatchCommand.removeStopwatch` (StopwatchCommand.java:105-114).
pub(super) fn remove_stopwatch(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let id = stopwatch_id(parse);
    if !cx.world.remove_stopwatch(&id) {
        return Err(does_not_exist(&id));
    }
    run.succeed(
        translatable(
            "commands.stopwatch.remove.success",
            vec![component_literal_nbt(&id)],
        ),
        true,
    );
    Ok(1)
}
