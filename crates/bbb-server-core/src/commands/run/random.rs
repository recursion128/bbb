//! `/random` (RandomCommand.java:37-194).

use bbb_protocol::{component_literal_nbt, NbtSummaryValue};

use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::{component, source_display_name_nbt, translatable, CommandCx, CommandSource};

use super::{identifier_argument, CommandBroadcast, CommandRun, Outcome};

/// `RandomCommand.randomSample` (RandomCommand.java:133-162). `announce` is
/// the `roll`/`value` split: `roll` broadcasts through
/// `PlayerList.broadcastSystemMessage` (`:153-156`) and `value` reports to the
/// source alone (`:158`).
pub(super) fn random_sample(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    announce: bool,
) -> Outcome {
    let Some(ArgumentValue::IntRange { min, max }) = parse.argument("range") else {
        unreachable!("range is an int_range argument");
    };
    // `:141-142`: an absent bound falls to the integer limits.
    let min = min.unwrap_or(i32::MIN);
    let max = max.unwrap_or(i32::MAX);
    let span = i64::from(max) - i64::from(min);
    if span == 0 {
        return Err(translatable(
            "commands.random.error.range_too_small",
            Vec::new(),
        ));
    }
    if span >= 2147483647 {
        return Err(translatable(
            "commands.random.error.range_too_large",
            Vec::new(),
        ));
    }
    // `Mth.randomBetweenInclusive` (Mth.java:664-666): `nextInt(max - min + 1)
    // + min`, off `getRandomSequence(sequence)` or the source level's own
    // random (`:136-139`).
    let bound = (span + 1) as i32;
    let value = if parse.argument("sequence").is_some() {
        let key = identifier_argument(parse, "sequence");
        cx.world.draw_from_random_sequence(key, bound)
    } else {
        cx.world.draw_from_level_random(&cx.origin.level, bound)
    } + min;
    if announce {
        run.broadcast
            .push(CommandBroadcast::System(component(translatable(
                "commands.random.roll",
                vec![
                    source_display_name_nbt(source, cx.world.scoreboard()),
                    NbtSummaryValue::Int(value),
                    NbtSummaryValue::Int(min),
                    NbtSummaryValue::Int(max),
                ],
            ))));
    } else {
        run.succeed(
            translatable(
                "commands.random.sample.success",
                vec![NbtSummaryValue::Int(value)],
            ),
            false,
        );
    }
    Ok(value)
}

/// `RandomCommand.resetSequence` in both arities (RandomCommand.java:164-178):
/// the explicit seed trio travels when one was typed, and the two boolean
/// arguments default to true by executor (`:80`, `:86`).
pub(super) fn reset_sequence(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let key = identifier_argument(parse, "sequence").to_string();
    cx.world
        .reset_random_sequence(&key, explicit_seed_trio(parse));
    run.succeed(
        translatable(
            "commands.random.reset.success",
            vec![component_literal_nbt(&key)],
        ),
        false,
    );
    Ok(1)
}

/// `RandomCommand.resetAllSequences` and
/// `resetAllSequencesAndSetNewDefaults` (RandomCommand.java:180-194): the
/// defaults move first when a seed was typed, then the map is cleared, and the
/// count reported is what the clear removed.
pub(super) fn reset_all_sequences(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let count = cx
        .world
        .reset_all_random_sequences(explicit_seed_trio(parse));
    let count = i32::try_from(count).unwrap_or(i32::MAX);
    run.succeed(
        translatable(
            "commands.random.reset.all.success",
            vec![NbtSummaryValue::Int(count)],
        ),
        false,
    );
    Ok(count)
}

/// The `(seed, includeWorldSeed, includeSequenceId)` trio of the explicit
/// forms, whose optional booleans default to true (RandomCommand.java:50,
/// `:55`, `:80`, `:86`), or `None` on the bare executors.
fn explicit_seed_trio(parse: &ParseResult) -> Option<(i32, bool, bool)> {
    let Some(ArgumentValue::Integer(seed)) = parse.argument("seed") else {
        return None;
    };
    let boolean = |name: &str| match parse.argument(name) {
        Some(ArgumentValue::Boolean(value)) => *value,
        _ => true,
    };
    Some((
        *seed,
        boolean("includeWorldSeed"),
        boolean("includeSequenceId"),
    ))
}
