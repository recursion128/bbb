//! `/dialog`.

use bbb_protocol::NbtSummaryValue;

use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::{translatable, CommandCx, CommandSource};
use crate::connection::ClientboundPlayPacket;

use super::{player_targets, CommandRun, Outcome};

/// `DialogCommand.showDialog` (`DialogCommand.java:44-56`): `openDialog` per
/// target.
pub(super) fn show_dialog(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let dialog = match parse.argument("dialog") {
        Some(ArgumentValue::Dialog(dialog)) => dialog.clone(),
        other => unreachable!("dialog is a dialog argument, not {other:?}"),
    };
    send_dialog(
        run,
        parse,
        source,
        cx,
        ClientboundPlayPacket::ShowDialog(dialog),
        "commands.dialog.show",
    )
}

/// `DialogCommand.clearDialog` (`DialogCommand.java:58-70`):
/// `ClientboundClearDialogPacket.INSTANCE` per target.
pub(super) fn clear_dialog(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    send_dialog(
        run,
        parse,
        source,
        cx,
        ClientboundPlayPacket::ClearDialog,
        "commands.dialog.clear",
    )
}

fn send_dialog(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    packet: ClientboundPlayPacket,
    stem: &str,
) -> Outcome {
    let (targets, first_name) =
        player_targets(parse, Some("targets"), source, cx.world, &cx.origin)?;
    for target in &targets {
        cx.world
            .send_to_command_player(target, packet.clone(), cx.events);
    }
    let count = i32::try_from(targets.len()).unwrap_or(i32::MAX);
    if targets.len() == 1 {
        run.succeed(
            translatable(
                format!("{stem}.single"),
                vec![first_name.expect("one target was named")],
            ),
            true,
        );
    } else {
        run.succeed(
            translatable(
                format!("{stem}.multiple"),
                vec![NbtSummaryValue::Int(count)],
            ),
            true,
        );
    }
    Ok(count)
}
