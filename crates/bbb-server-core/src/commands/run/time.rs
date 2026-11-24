//! `/time` (`TimeCommand.java:46-62`).

use bbb_protocol::NbtSummaryValue;

use super::super::parse::ParseResult;
use super::super::types::CommandClockChange;
use super::super::{CommandCx, CommandOrigin};
use super::{
    float_argument, identifier_argument, resource_argument, time_argument, translatable,
    CommandRun, Outcome,
};
use crate::world::ServerWorld;

/// `TimeCommand.wrapTime` (`TimeCommand.java:203-205`).
fn wrap_time(ticks: i64) -> i32 {
    (ticks % 2_147_483_647) as i32
}

fn clock_name(clock: &str) -> NbtSummaryValue {
    NbtSummaryValue::string(clock)
}

/// `TimeCommand.ClockGetter` (`TimeCommand.java:212-214`): the `of <clock>`
/// argument the `addClockNodes` caller bound, or `getDefaultClock`
/// (`:207-210`), which refuses a dimension type declaring none.
fn clock_of(
    parse: &ParseResult,
    world: &ServerWorld,
    at: &CommandOrigin,
    explicit: bool,
) -> Result<String, NbtSummaryValue> {
    if explicit {
        return Ok(resource_argument(parse, "clock").to_string());
    }
    let (dimension_type, facts) = world.command_dimension_type(&at.level);
    facts.default_clock.ok_or_else(|| {
        translatable(
            "commands.time.no_default_clock",
            vec![NbtSummaryValue::string(dimension_type)],
        )
    })
}

/// `TimeCommand.queryGameTime` (`TimeCommand.java:131-135`).
pub(super) fn query_game_time(run: &mut CommandRun, world: &ServerWorld) -> i32 {
    let game_time = world.game_time();
    run.succeed(
        translatable(
            "commands.time.query.gametime",
            vec![NbtSummaryValue::Long(game_time)],
        ),
        false,
    );
    wrap_time(game_time)
}

/// `TimeCommand.queryTime` (`TimeCommand.java:137-142`).
pub(super) fn query_clock_time(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &ServerWorld,
    at: &CommandOrigin,
    explicit: bool,
) -> Outcome {
    let clock = clock_of(parse, world, at, explicit)?;
    let total_ticks = total_ticks(world, &clock);
    run.succeed(
        translatable(
            "commands.time.query.absolute",
            vec![clock_name(&clock), NbtSummaryValue::Long(total_ticks)],
        ),
        false,
    );
    Ok(wrap_time(total_ticks))
}

/// `TimeCommand.queryTimelineTicks` (`TimeCommand.java:144-153`) and
/// `queryTimelineRepetitions` (`:155-164`), which share the clock check and
/// differ only in the reader they call and the key they report.
pub(super) fn query_timeline(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &ServerWorld,
    at: &CommandOrigin,
    explicit: bool,
    repetitions: bool,
) -> Outcome {
    let clock = clock_of(parse, world, at, explicit)?;
    let id = resource_argument(parse, "timeline");
    let timeline = world
        .clock_catalog()
        .timeline(id)
        .expect("the timeline argument resolved against this catalog");
    if timeline.clock != clock {
        return Err(translatable(
            "commands.time.wrong_timeline_for_clock",
            vec![clock_name(id), clock_name(&clock)],
        ));
    }
    let total_ticks = total_ticks(world, &clock);
    let (key, value) = if repetitions {
        (
            "commands.time.query.timeline.repetitions",
            i64::from(timeline.period_count(total_ticks)),
        )
    } else {
        (
            "commands.time.query.timeline",
            timeline.current_ticks(total_ticks),
        )
    };
    run.succeed(
        translatable(key, vec![clock_name(id), NbtSummaryValue::Long(value)]),
        false,
    );
    Ok(wrap_time(value))
}

/// `TimeCommand.setTotalTicks` (`TimeCommand.java:166-171`), whose answer is
/// the argument rather than the wrapped clock reading.
pub(super) fn set_clock_ticks(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
    explicit: bool,
) -> Outcome {
    let clock = clock_of(parse, cx.world, &cx.origin, explicit)?;
    let total_ticks = time_argument(parse, "time");
    change(
        cx,
        &clock,
        CommandClockChange::SetTotalTicks(i64::from(total_ticks)),
    );
    run.succeed(
        translatable(
            "commands.time.set.absolute",
            vec![clock_name(&clock), NbtSummaryValue::Int(total_ticks)],
        ),
        true,
    );
    Ok(total_ticks)
}

/// `TimeCommand.addTime` (`TimeCommand.java:173-179`), which reports the
/// clock reading `addTicks` left behind (`ServerClockManager.java:100-102`).
pub(super) fn add_clock_ticks(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
    explicit: bool,
) -> Outcome {
    let clock = clock_of(parse, cx.world, &cx.origin, explicit)?;
    let before = total_ticks(cx.world, &clock);
    let ticks = time_argument(parse, "time");
    let total_ticks = before.wrapping_add(i64::from(ticks)).max(0);
    change(cx, &clock, CommandClockChange::AddTicks(ticks));
    run.succeed(
        translatable(
            "commands.time.set.absolute",
            vec![clock_name(&clock), NbtSummaryValue::Long(total_ticks)],
        ),
        true,
    );
    Ok(wrap_time(total_ticks))
}

/// `TimeCommand.setTimeToTimeMarker` (`TimeCommand.java:181-189`).
///
/// `moveToTimeMarker` (`ServerClockManager.java:87-98`) answers false for a
/// marker the clock does not carry, and every marker a timeline declared is
/// registered whether or not it shows in commands (`Timeline.java:138-143`).
pub(super) fn set_clock_time_marker(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
    explicit: bool,
) -> Outcome {
    let id = identifier_argument(parse, "timemarker");
    let clock = clock_of(parse, cx.world, &cx.origin, explicit)?;
    let total_ticks = cx
        .world
        .clock_catalog()
        .clock(&clock)
        .and_then(|entry| entry.time_marker(id))
        .map(|marker| marker.resolve_time_to_move_to(total_ticks(cx.world, &clock)));
    let Some(total_ticks) = total_ticks else {
        return Err(translatable(
            "commands.time.no_time_marker_found",
            vec![
                // `ResourceKey.toString` (`ResourceKey.java:43-45`), which is
                // what `translatableEscape` makes of the key
                // (`Component.java:147-156`).
                NbtSummaryValue::string(format!("ResourceKey[minecraft:clock_time_marker / {id}]")),
                clock_name(&clock),
            ],
        ));
    };
    change(cx, &clock, CommandClockChange::SetTotalTicks(total_ticks));
    run.succeed(
        translatable(
            "commands.time.set.time_marker",
            vec![clock_name(&clock), clock_name(id)],
        ),
        true,
    );
    Ok(wrap_time(total_ticks))
}

/// `TimeCommand.setPaused` (`TimeCommand.java:191-195`).
pub(super) fn set_clock_paused(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
    explicit: bool,
    paused: bool,
) -> Outcome {
    let clock = clock_of(parse, cx.world, &cx.origin, explicit)?;
    change(cx, &clock, CommandClockChange::SetPaused(paused));
    run.succeed(
        translatable(
            if paused {
                "commands.time.pause"
            } else {
                "commands.time.resume"
            },
            vec![clock_name(&clock)],
        ),
        true,
    );
    Ok(1)
}

/// `TimeCommand.setRate` (`TimeCommand.java:197-201`).
pub(super) fn set_clock_rate(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
    explicit: bool,
) -> Outcome {
    let clock = clock_of(parse, cx.world, &cx.origin, explicit)?;
    let rate = float_argument(parse, "rate");
    change(cx, &clock, CommandClockChange::SetRate(rate));
    run.succeed(
        translatable(
            "commands.time.rate",
            vec![clock_name(&clock), NbtSummaryValue::Float(rate.to_bits())],
        ),
        true,
    );
    Ok(1)
}

/// `ServerClockManager.getTotalTicks` (`ServerClockManager.java:125-127`).
fn total_ticks(world: &ServerWorld, clock: &str) -> i64 {
    world
        .clock_total_ticks(clock)
        .expect("the clock argument resolved against this catalog")
}

fn change(cx: &mut CommandCx<'_>, clock: &str, change: CommandClockChange) {
    cx.world.apply_clock_change(clock, change, cx.events);
}
