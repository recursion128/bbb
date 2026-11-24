//! `/clear` (`ClearInventoryCommands.java:27-93`).

use bbb_protocol::{component_literal_nbt, NbtSummaryValue};

use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::{
    team_player_display_name_nbt, translatable, CommandCx, CommandPlayer, CommandSource,
};

use super::{
    integer_argument, player_states_argument, source_player_state, view_of, CommandRun, Outcome,
};

/// `ClearInventoryCommands.clearInventory` (`ClearInventoryCommands.java:61-93`).
pub(super) fn clear_items(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    targeted: bool,
) -> Outcome {
    let (players, single_name) = cx.world.with_entity_reads(|reads| {
        let players = if targeted {
            player_states_argument(parse, "targets", source, reads, &cx.origin)?
        } else {
            // `c.getSource().getPlayerOrException()` (`ClearInventoryCommands.java:31`),
            // whose `ERROR_NOT_PLAYER` (CommandSourceStack.java:405) is
            // `permissions.requires.player`.
            let Some(player) = source_player_state(source, reads) else {
                return Err(translatable("permissions.requires.player", Vec::new()));
            };
            vec![player]
        };
        let single_name = (players.len() == 1)
            .then(|| team_player_display_name_nbt(&view_of(players[0]), reads.scoreboard));
        let players = players
            .iter()
            .map(CommandPlayer::target)
            .collect::<Vec<_>>();
        Ok::<_, NbtSummaryValue>((players, single_name))
    })?;
    let predicate = match parse.argument("item") {
        Some(ArgumentValue::ItemPredicate(predicate)) => Some(predicate),
        None => None,
        other => unreachable!("item is an item predicate, not {other:?}"),
    };
    let max_count = parse
        .argument("maxCount")
        .map_or(-1, |_| integer_argument(parse, "maxCount"));
    let resolver = cx.world.item_stack_resolver();
    let mut matches = |stack: &bbb_server_types::item_stack::RuntimeItemStack| {
        let Some(predicate) = predicate else {
            return Ok(true);
        };
        let tool = crate::loot_runtime::LootTool {
            item_id: stack.item_id(),
            count: stack.count(),
            stack: stack.clone(),
            enchantments: Default::default(),
        };
        predicate.matches(&tool, &resolver).map_err(|error| {
            bbb_server_types::ServerWorldError::InvalidConfiguration(error.to_string())
        })
    };
    let mut count = 0_i32;
    for player in &players {
        count = count.saturating_add(cx.world.clear_command_items(
            player,
            max_count,
            &mut matches,
            cx.events,
        )?);
    }
    let targets = i32::try_from(players.len()).unwrap_or(i32::MAX);
    if count == 0 {
        return Err(if players.len() == 1 {
            translatable(
                "clear.failed.single",
                vec![component_literal_nbt(&players[0].name)],
            )
        } else {
            translatable("clear.failed.multiple", vec![NbtSummaryValue::Int(targets)])
        });
    }
    run.succeed(
        match single_name {
            Some(name) => translatable(
                if max_count == 0 {
                    "commands.clear.test.single"
                } else {
                    "commands.clear.success.single"
                },
                vec![NbtSummaryValue::Int(count), name],
            ),
            None => translatable(
                if max_count == 0 {
                    "commands.clear.test.multiple"
                } else {
                    "commands.clear.success.multiple"
                },
                vec![NbtSummaryValue::Int(count), NbtSummaryValue::Int(targets)],
            ),
        },
        true,
    );
    Ok(count)
}
