//! `/tellraw`.

use bbb_protocol::NbtSummaryValue;

use crate::commands::parse::ParseResult;
use crate::commands::{component_with_registry, CommandCx, CommandSource};

use super::{component_argument, player_states_argument, Outcome};

/// `TellRawCommand` (TellRawCommand.java:18-26): `sendSystemMessage` per
/// target and no `sendSuccess`.
pub(super) fn tellraw(
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let message = component_argument(parse, "message");
    let targets = cx.world.with_entity_reads(|reads| {
        let at = &cx.origin;
        Ok::<_, NbtSummaryValue>(
            player_states_argument(parse, "targets", source, reads, at)?
                .into_iter()
                .map(|player| player.target())
                .collect::<Vec<_>>(),
        )
    })?;
    for target in &targets {
        let text = cx.world.with_entity_reads(|reads| {
            super::text::resolve(
                message,
                source,
                &cx.origin,
                reads,
                reads.players().find(|player| player.id == target.id),
            )
        })?;
        let text = component_with_registry(text, cx.world.plugins().content_registry());
        cx.world
            .send_command_system_message(target, text, cx.events);
    }
    Ok(i32::try_from(targets.len()).unwrap_or(i32::MAX))
}
