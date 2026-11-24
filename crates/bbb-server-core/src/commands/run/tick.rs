//! `/tick`: query, rate, freeze, step and sprint.

use bbb_protocol::{component_literal_nbt, NbtSummaryValue, RichTextComponent};

use crate::tick_rate::java_fixed_decimals;

use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::{component, translatable, CommandCx};
use crate::world::ServerWorld;

use super::CommandRun;

/// The `<time>` argument of `/tick step` and `/tick sprint`, already in ticks
/// (`TimeArgument.parse`, TimeArgument.java:44-58).
pub(super) fn tick_time_argument(parse: &ParseResult) -> i32 {
    match parse.argument("time") {
        Some(ArgumentValue::Time(ticks)) => *ticks,
        other => unreachable!("the tick time argument parsed as {other:?}"),
    }
}

/// `TickCommand.tickQuery` (TickCommand.java:69-97).
///
/// Every number here is a measurement of how fast this process happened to run,
/// so no two servers can be expected to agree on any of them; only the status
/// line and the tick rate are properties of the world rather than of the
/// machine.
pub(super) fn tick_query(run: &mut CommandRun, world: &ServerWorld) -> i32 {
    let rate = world.tick_rate();
    let average_tick_time_nanos = world.average_tick_time_nanos();
    let busy_time = nanos_to_millis_string(average_tick_time_nanos);
    let tick_rate_string = java_fixed_decimals(f64::from(rate.tickrate()), 1);
    if rate.is_sprinting() {
        run.succeed(
            translatable("commands.tick.status.sprinting", Vec::new()),
            false,
        );
        run.succeed(
            translatable(
                "commands.tick.query.rate.sprinting",
                vec![
                    component_literal_nbt(&tick_rate_string),
                    component_literal_nbt(&busy_time),
                ],
            ),
            false,
        );
    } else {
        if rate.is_frozen() {
            run.succeed(
                translatable("commands.tick.status.frozen", Vec::new()),
                false,
            );
        } else if rate.nanoseconds_per_tick() < average_tick_time_nanos {
            run.succeed(
                translatable("commands.tick.status.lagging", Vec::new()),
                false,
            );
        } else {
            run.succeed(
                translatable("commands.tick.status.running", Vec::new()),
                false,
            );
        }
        run.succeed(
            translatable(
                "commands.tick.query.rate.running",
                vec![
                    component_literal_nbt(&tick_rate_string),
                    component_literal_nbt(&busy_time),
                    component_literal_nbt(nanos_to_millis_string(rate.nanoseconds_per_tick())),
                ],
            ),
            false,
        );
    }
    // `Arrays.copyOf` then `Arrays.sort` (TickCommand.java:90-91): the ring is
    // sorted by duration, so the three percentiles index the sorted copy rather
    // than the ring's own tick order.
    let mut samples = world.tick_times_nanos().to_vec();
    samples.sort_unstable();
    let count = samples.len();
    let percentile = |fraction: f64| {
        samples
            .get((count as f64 * fraction) as usize)
            .copied()
            .unwrap_or_default()
    };
    let median = samples.get(count / 2).copied().unwrap_or_default();
    run.succeed(
        translatable(
            "commands.tick.query.percentiles",
            vec![
                component_literal_nbt(nanos_to_millis_string(median)),
                component_literal_nbt(nanos_to_millis_string(percentile(0.95))),
                component_literal_nbt(nanos_to_millis_string(percentile(0.99))),
                NbtSummaryValue::Int(i32::try_from(count).unwrap_or(i32::MAX)),
            ],
        ),
        false,
    );
    // `return (int)tickRate` (TickCommand.java:96).
    rate.tickrate() as i32
}

/// `TickCommand.nanosToMilisString` (TickCommand.java:57-59).
///
/// Both operands are narrowed to `float` before the division, as the two Java
/// casts do, and only the single-precision quotient is widened for `%.1f`.
/// Dividing in double instead would print a different tenth wherever the exact
/// value sits within about 2e-6 ms of a `.x5` boundary: 25_050_000 ns is
/// 25.049999237060547 as a `float` and prints `25.0`, while the double quotient
/// is 25.05 exactly and would print `25.1`.
fn nanos_to_millis_string(nanos: i64) -> String {
    let milliseconds = nanos as f32 / NANOSECONDS_PER_MILLISECOND as f32;
    java_fixed_decimals(f64::from(milliseconds), 1)
}

/// `TimeUtil.NANOSECONDS_PER_MILLISECOND` (TimeUtil.java:8), as the `float`
/// `nanosToMilisString` divides by.
const NANOSECONDS_PER_MILLISECOND: u32 = 1_000_000;

/// `TickCommand.setTickingRate` (TickCommand.java:61-66).
pub(super) fn set_ticking_rate(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
) -> i32 {
    let Some(ArgumentValue::Float(rate)) = parse.argument("rate") else {
        unreachable!("the tick rate argument parsed as something other than a float");
    };
    let rate = *rate;
    cx.world.set_tick_rate(rate, cx.events);
    run.succeed(
        translatable(
            "commands.tick.rate.success",
            vec![component_literal_nbt(java_fixed_decimals(
                f64::from(rate),
                1,
            ))],
        ),
        true,
    );
    // `return (int)rate` (TickCommand.java:66).
    rate as i32
}

/// `TickCommand.setFreeze` (TickCommand.java:109-129): freezing stops a
/// sprint and a step first.
pub(super) fn set_freeze(run: &mut CommandRun, cx: &mut CommandCx<'_>, freeze: bool) -> i32 {
    if freeze {
        if cx.world.tick_rate().is_sprinting() {
            cx.world.stop_sprinting(cx.events);
        }
        if cx.world.tick_rate().is_stepping_forward() {
            cx.world.stop_stepping(cx.events);
        }
    }
    cx.world.set_tick_frozen(freeze, cx.events);
    run.succeed(
        translatable(
            if freeze {
                "commands.tick.status.frozen"
            } else {
                "commands.tick.status.running"
            },
            Vec::new(),
        ),
        true,
    );
    // `return freeze ? 1 : 0` (TickCommand.java:128).
    i32::from(freeze)
}

/// `TickCommand.step` (TickCommand.java:131-141): `stepGameIfPaused` only
/// arms the counter while the game is frozen, and the refusal is the whole
/// answer when it is not.
pub(super) fn step(run: &mut CommandRun, cx: &mut CommandCx<'_>, advance: i32) -> i32 {
    if cx.world.step_ticks(advance, cx.events) {
        run.succeed(
            translatable(
                "commands.tick.step.success",
                vec![NbtSummaryValue::Int(advance)],
            ),
            true,
        );
    } else {
        run.fail(translatable("commands.tick.step.fail", Vec::new()));
    }
    // `return 1` either way (TickCommand.java:140), because `sendFailure` is
    // not a `CommandSyntaxException`.
    1
}

/// `TickCommand.stopStepping` (TickCommand.java:143-153).
pub(super) fn stop_stepping(run: &mut CommandRun, cx: &mut CommandCx<'_>) -> i32 {
    if cx.world.stop_stepping(cx.events) {
        run.succeed(
            translatable("commands.tick.step.stop.success", Vec::new()),
            true,
        );
        1
    } else {
        run.fail(translatable("commands.tick.step.stop.fail", Vec::new()));
        0
    }
}

/// `TickCommand.sprint` (TickCommand.java:99-107): the interrupted arm reports
/// the sprint it replaced.
pub(super) fn sprint(run: &mut CommandRun, cx: &mut CommandCx<'_>, time: i32) -> i32 {
    if cx.world.sprint_ticks(time, cx.events) {
        run.succeed(
            translatable("commands.tick.sprint.stop.success", Vec::new()),
            true,
        );
    }
    run.succeed(
        translatable("commands.tick.status.sprinting", Vec::new()),
        true,
    );
    1
}

/// `TickCommand.stopSprinting` (TickCommand.java:155-164).
pub(super) fn stop_sprinting(run: &mut CommandRun, cx: &mut CommandCx<'_>) -> i32 {
    if cx.world.stop_sprinting(cx.events) {
        run.succeed(
            translatable("commands.tick.sprint.stop.success", Vec::new()),
            true,
        );
        1
    } else {
        run.fail(translatable("commands.tick.sprint.stop.fail", Vec::new()));
        0
    }
}

/// `commands.tick.sprint.report` (ServerTickRateManager.java:89-91), which the
/// manager sends when a sprint ends rather than when a command asks for one.
pub(crate) fn sprint_report(
    ticks_per_second: i32,
    milliseconds_per_tick: &str,
) -> RichTextComponent {
    component(translatable(
        "commands.tick.sprint.report",
        vec![
            NbtSummaryValue::Int(ticks_per_second),
            component_literal_nbt(milliseconds_per_tick),
        ],
    ))
}
