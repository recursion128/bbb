//! `/gamemode`.

use bbb_protocol::{game_types::GameType, NbtSummaryValue};

use crate::commands::parse::ParseResult;
use crate::commands::{
    component, team_player_display_name_nbt, translatable, CommandCx, CommandPlayerTarget,
    CommandSource,
};

use super::server::game_type_long_display_name;
use super::{
    game_mode_argument, player_states_argument, source_player_state, view_of, CommandRun, Outcome,
};

/// `GameModeCommand.setMode` (GameModeCommand.java:49-59) over `setGameMode`
/// (`:65-72`): the count is the sessions `ServerPlayer.setGameMode` actually
/// moved, and only a moved target reaches `logGamemodeChange`.
pub(super) fn set_game_mode(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    targeted: bool,
) -> Outcome {
    // `EntityArgument.getPlayers` (:30) runs before `GameModeArgument.getGameMode`
    // in the same argument list, so an unresolvable selector is what a
    // `/gamemode` over one reports.
    let players = cx.world.with_entity_reads(|reads| {
        let at = &cx.origin;
        let players = if targeted {
            player_states_argument(parse, "target", source, reads, at)?
        } else {
            // `c.getSource().getPlayerOrException()` (`:27`), whose
            // `ERROR_NOT_PLAYER` (CommandSourceStack.java:405) is
            // `permissions.requires.player`.
            let Some(player) = source_player_state(source, reads) else {
                return Err(translatable("permissions.requires.player", Vec::new()));
            };
            vec![player]
        };
        Ok(players
            .into_iter()
            .map(|player| {
                (
                    player.target(),
                    team_player_display_name_nbt(&view_of(player), reads.scoreboard),
                )
            })
            .collect::<Vec<_>>())
    })?;
    let game_type = game_mode_argument(parse, "gamemode");
    let mut count = 0;
    for (target, name) in players {
        if !cx
            .world
            .set_command_player_game_mode(&target, game_type, cx.events)
        {
            continue;
        }
        count += 1;
        log_gamemode_change(run, source, cx, &target, name, game_type);
    }
    Ok(count)
}

/// `GameModeCommand.logGamemodeChange` (GameModeCommand.java:36-47). The
/// `gameMode.changed` notice is a `sendSystemMessage` behind its own
/// `SEND_COMMAND_FEEDBACK` check (`:41-42`), not a `sendSuccess`.
fn log_gamemode_change(
    run: &mut CommandRun,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    target: &CommandPlayerTarget,
    name: NbtSummaryValue,
    game_type: GameType,
) {
    let mode = game_type_long_display_name(game_type);
    // `source.getEntity() == target` (`:38`); a source with no entity matches
    // no player.
    if source.uuid == target.id {
        run.succeed(
            translatable("commands.gamemode.success.self", vec![mode]),
            true,
        );
        return;
    }
    if cx.world.accepts_command_success() {
        cx.world.send_command_system_message(
            target,
            component(translatable("gameMode.changed", vec![mode.clone()])),
            cx.events,
        );
    }
    run.succeed(
        translatable("commands.gamemode.success.other", vec![name, mode]),
        true,
    );
}
