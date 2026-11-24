//! `/experience` (`/xp`).

use bbb_protocol::NbtSummaryValue;

use crate::commands::parse::ParseResult;
use crate::commands::types::{CommandExperienceKind, CommandPlayer};
use crate::commands::{team_player_display_name_nbt, translatable, CommandCx, CommandSource};
use crate::world::ServerWorld;

use super::{
    integer_argument, player_targets, single_player_argument, view_of, CommandRun, Outcome,
};
use crate::commands::CommandOrigin;

/// `ExperienceCommand.queryExperience` (ExperienceCommand.java:125-129).
///
/// `Type.query` is `Mth.floor(experienceProgress * getXpNeededForNextLevel())`
/// for points (`:183`) and `experienceLevel` for levels (`:187`); the target
/// is `EntityArgument.getPlayer` (`:113`, `:117`), one player and no more.
pub(super) fn query_experience(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
    kind: CommandExperienceKind,
) -> Outcome {
    world.with_entity_reads(|reads| {
        let target = single_player_argument(parse, "target", source, reads, at)?;
        let result = match kind {
            CommandExperienceKind::Points => experience_points_of(target),
            CommandExperienceKind::Levels => target.experience.level(),
        };
        // `:127`, which reports with `false`.
        run.succeed(
            translatable(
                format!("commands.experience.query.{}", kind.name()),
                vec![
                    team_player_display_name_nbt(&view_of(target), reads.scoreboard),
                    NbtSummaryValue::Int(result),
                ],
            ),
            false,
        );
        Ok(result)
    })
}

/// `Type.POINTS`'s query (ExperienceCommand.java:183): the points of the
/// current bar, floored the way `Mth.floor` floors a `float`.
fn experience_points_of(target: CommandPlayer<'_>) -> i32 {
    (target.experience.progress() * target.experience.xp_needed_for_next_level() as f32).floor()
        as i32
}

/// `ExperienceCommand.addExperience` (ExperienceCommand.java:131-147): every
/// target is given the amount (`:134-136`) and one line reports, named after
/// the one target or counting them (`:138-144`), with `true`.
pub(super) fn add_experience(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    kind: CommandExperienceKind,
) -> Outcome {
    let (players, first_name) =
        player_targets(parse, Some("target"), source, cx.world, &cx.origin)?;
    let amount = integer_argument(parse, "amount");
    for player in &players {
        cx.world.give_command_experience(player, amount, kind);
    }
    report_experience_change(run, "add", kind, amount, players.len(), first_name);
    Ok(i32::try_from(players.len()).unwrap_or(i32::MAX))
}

/// `ExperienceCommand.setExperience` (ExperienceCommand.java:149-173).
///
/// `Type.set` is a predicate: for points it refuses an amount the current
/// level cannot hold -- `a >= getXpNeededForNextLevel()` (`:177-179`) -- and
/// sets otherwise; for levels it always sets (`:184-186`). A run in which no
/// target was set is the `commands.experience.set.points.invalid` refusal
/// (`:160-162`), and one in which some were reports over the whole target
/// list (`:164-170`), count included, rather than over the ones that were set.
pub(super) fn set_experience(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    kind: CommandExperienceKind,
) -> Outcome {
    let (players, first_name) =
        player_targets(parse, Some("target"), source, cx.world, &cx.origin)?;
    let amount = integer_argument(parse, "amount");
    let mut success = 0;
    for player in &players {
        if cx.world.set_command_experience(player, amount, kind) {
            success += 1;
        }
    }
    if success == 0 {
        return Err(translatable(
            "commands.experience.set.points.invalid",
            Vec::new(),
        ));
    }
    report_experience_change(run, "set", kind, amount, players.len(), first_name);
    Ok(i32::try_from(players.len()).unwrap_or(i32::MAX))
}

/// The one line `addExperience` (ExperienceCommand.java:138-144) and
/// `setExperience` (`:164-170`) end in: `<verb>.<type>.success.single` over the
/// amount and the first target's display name, or `.multiple` over the amount
/// and the target count.
fn report_experience_change(
    run: &mut CommandRun,
    verb: &str,
    kind: CommandExperienceKind,
    amount: i32,
    count: usize,
    first_name: Option<NbtSummaryValue>,
) {
    let message = if count == 1 {
        translatable(
            format!("commands.experience.{verb}.{}.success.single", kind.name()),
            vec![
                NbtSummaryValue::Int(amount),
                first_name.expect("one target was named"),
            ],
        )
    } else {
        translatable(
            format!(
                "commands.experience.{verb}.{}.success.multiple",
                kind.name()
            ),
            vec![
                NbtSummaryValue::Int(amount),
                NbtSummaryValue::Int(i32::try_from(count).unwrap_or(i32::MAX)),
            ],
        )
    };
    run.succeed(message, true);
}
