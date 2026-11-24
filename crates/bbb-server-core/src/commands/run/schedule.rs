//! `/schedule` (ScheduleCommand.java:38-129).

use bbb_protocol::{component_literal_nbt, NbtSummaryValue};

use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::types::CommandScheduledCallback;
use crate::commands::{translatable, CommandCx};

use super::{time_argument, CommandRun, Outcome};

/// `Math.floorMod(tickTime, Integer.MAX_VALUE)` (ScheduleCommand.java:118).
fn floor_mod_max(tick_time: i64) -> i32 {
    tick_time.rem_euclid(i64::from(i32::MAX)) as i32
}

fn function_argument(parse: &ParseResult) -> (bool, &str) {
    match parse.argument("function") {
        Some(ArgumentValue::Function { tag, id }) => (*tag, id.as_str()),
        other => unreachable!("the schedule function argument parsed as {other:?}"),
    }
}

fn clear_id(parse: &ParseResult) -> &bbb_protocol::JavaString {
    match parse.argument("function") {
        Some(ArgumentValue::Text(id)) => id,
        other => unreachable!("the schedule clear argument parsed as {other:?}"),
    }
}

/// `ScheduleCommand.schedule` (ScheduleCommand.java:82-118).
pub(super) fn schedule_function(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
    replace: bool,
) -> Outcome {
    let time = time_argument(parse, "time");
    if time == 0 {
        return Err(translatable("commands.schedule.same_tick", Vec::new()));
    }
    let (tag, id) = function_argument(parse);
    let functions = cx.world.command_functions();
    if tag {
        if functions.tag(id).is_none() {
            return Err(translatable(
                "arguments.function.tag.unknown",
                vec![component_literal_nbt(id)],
            ));
        }
    } else {
        let Some(function) = functions.definition(id) else {
            return Err(translatable(
                "arguments.function.unknown",
                vec![component_literal_nbt(id)],
            ));
        };
        if function.is_macro() {
            return Err(translatable("commands.schedule.macro", Vec::new()));
        }
    }
    let trigger_time = cx.world.game_time() + i64::from(time);
    let schedule_id = if tag {
        format!("#{id}")
    } else {
        id.to_string()
    };
    cx.world.schedule_event(
        schedule_id,
        trigger_time,
        if tag {
            CommandScheduledCallback::FunctionTag(id.to_string())
        } else {
            CommandScheduledCallback::Function(id.to_string())
        },
        replace,
    );
    let key = if tag {
        "commands.schedule.created.tag"
    } else {
        "commands.schedule.created.function"
    };
    run.succeed(
        translatable(
            key,
            vec![
                component_literal_nbt(id),
                NbtSummaryValue::Int(time),
                NbtSummaryValue::Long(trigger_time),
            ],
        ),
        true,
    );
    Ok(floor_mod_max(trigger_time))
}

/// `ScheduleCommand.remove` (ScheduleCommand.java:121-129).
pub(super) fn clear_schedule(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let id = clear_id(parse);
    let count = cx.world.clear_scheduled_events(id);
    if count == 0 {
        return Err(translatable(
            "commands.schedule.cleared.failure",
            vec![component_literal_nbt(id)],
        ));
    }
    run.succeed(
        translatable(
            "commands.schedule.cleared.success",
            vec![NbtSummaryValue::Int(count), component_literal_nbt(id)],
        ),
        true,
    );
    Ok(count)
}
