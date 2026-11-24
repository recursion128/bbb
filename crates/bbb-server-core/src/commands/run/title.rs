//! `/title`.

use bbb_protocol::{
    packets::{ClearTitles, SetActionBarText, SetSubtitleText, SetTitleText, SetTitlesAnimation},
    NbtSummaryValue,
};

use crate::commands::parse::ParseResult;
use crate::commands::{translatable, CommandCx, CommandPlayerTarget, CommandSource};
use crate::connection::ClientboundPlayPacket;

use super::{component_argument, player_targets, time_argument, CommandRun, Outcome};

/// `TitleCommand.clearTitle` (TitleCommand.java:104-118):
/// `ClientboundClearTitlesPacket(false)` to every target.
pub(super) fn clear_title(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    send_title(
        run,
        parse,
        source,
        cx,
        ClientboundPlayPacket::ClearTitles(ClearTitles { reset_times: false }),
        "commands.title.cleared",
    )
}

/// `TitleCommand.resetTitle` (TitleCommand.java:120-134):
/// `ClientboundClearTitlesPacket(true)` to every target.
pub(super) fn reset_title(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    send_title(
        run,
        parse,
        source,
        cx,
        ClientboundPlayPacket::ClearTitles(ClearTitles { reset_times: true }),
        "commands.title.reset",
    )
}

/// `TitleCommand.showTitle` (TitleCommand.java:136-155).
pub(super) fn show_title(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    kind: &str,
) -> Outcome {
    let (targets, first_name) =
        player_targets(parse, Some("targets"), source, cx.world, &cx.origin)?;
    for target in &targets {
        let resolved = cx.world.with_entity_reads(|reads| {
            let at = &cx.origin;
            super::text::resolve(
                component_argument(parse, "title"),
                source,
                at,
                reads,
                reads.players().find(|player| player.id == target.id),
            )
        })?;
        let title = crate::commands::component_with_registry(
            resolved,
            cx.world.plugins().content_registry(),
        );
        let packet = match kind {
            "title" => ClientboundPlayPacket::SetTitleText(SetTitleText { content: title }),
            "subtitle" => {
                ClientboundPlayPacket::SetSubtitleText(SetSubtitleText { content: title })
            }
            "actionbar" => {
                ClientboundPlayPacket::SetActionBarText(SetActionBarText { content: title })
            }
            other => unreachable!("{other} is a title show literal of the derived tree"),
        };
        cx.world.send_to_command_player(target, packet, cx.events);
    }
    report_title(
        run,
        &targets,
        first_name,
        &format!("commands.title.show.{kind}"),
    );
    Ok(i32::try_from(targets.len()).unwrap_or(i32::MAX))
}

/// `TitleCommand.setTimes` (TitleCommand.java:157-171).
pub(super) fn set_title_times(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    send_title(
        run,
        parse,
        source,
        cx,
        ClientboundPlayPacket::SetTitlesAnimation(SetTitlesAnimation {
            fade_in: time_argument(parse, "fadeIn"),
            stay: time_argument(parse, "stay"),
            fade_out: time_argument(parse, "fadeOut"),
        }),
        "commands.title.times",
    )
}

fn send_title(
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
    report_title(run, &targets, first_name, stem);
    Ok(i32::try_from(targets.len()).unwrap_or(i32::MAX))
}

fn report_title(
    run: &mut CommandRun,
    targets: &[CommandPlayerTarget],
    first_name: Option<NbtSummaryValue>,
    stem: &str,
) {
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
                vec![NbtSummaryValue::Int(
                    i32::try_from(targets.len()).unwrap_or(i32::MAX),
                )],
            ),
            true,
        );
    }
}
