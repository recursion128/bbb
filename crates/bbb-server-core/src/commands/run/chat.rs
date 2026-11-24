//! `/say`, `/me`, `/msg` and `/teammsg`.

use bbb_protocol::{ComponentClickEvent, ComponentHoverEvent, ComponentStyle, NbtSummaryValue};
use uuid::Uuid;

use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::{
    component, source_display_name_nbt, team_player_display_name_nbt, translatable, CommandCx,
    CommandOrigin, CommandPlayerTarget, CommandPlayerView, CommandSource,
};
use crate::world::{ChatKind, EntityReads, OutgoingChat};

use super::{
    players_argument, source_entity_state, CommandBroadcast, CommandFeedback, CommandFeedbackKind,
    CommandRun, Outcome,
};

/// `MessageArgument.resolveChatMessage` (MessageArgument.java:42-52): the
/// signed argument when the chain carries one, `PlayerChatMessage.system`
/// otherwise, bound to `kind` over the source's display name.
fn chat_message(
    parse: &ParseResult,
    argument: &str,
    source: &CommandSource,
    at: &CommandOrigin,
    reads: &EntityReads<'_>,
    signer: Option<Uuid>,
    kind: ChatKind,
) -> Result<OutgoingChat, NbtSummaryValue> {
    let formatted = super::text::message(parse, argument, source, at, reads)?;
    let Some(ArgumentValue::Message(message)) = parse.argument(argument) else {
        unreachable!("message argument");
    };
    Ok(OutgoingChat::new_with_registry(
        signer,
        message.text.to_jdk_string(),
        formatted,
        kind,
        source_display_name_nbt(source, reads.scoreboard),
        reads.content_registry_arc(),
    ))
}

/// `SayCommand.register` (SayCommand.java:15-22) and
/// `EmoteCommands.register` (EmoteCommands.java:12-19).
pub(super) fn broadcast(
    run: &mut CommandRun,
    parse: &ParseResult,
    argument: &str,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    kind: ChatKind,
) -> Outcome {
    let (signer, at) = (cx.chat_signer, &cx.origin);
    let message = cx.world.with_entity_reads(|reads| {
        chat_message(parse, argument, source, at, reads, signer, kind)
    })?;
    run.broadcast.push(CommandBroadcast::Chat(message));
    Ok(1)
}

/// `MsgCommand.sendMessage` (MsgCommand.java:33-49): the source sees one
/// outgoing line per target and each target sees the incoming line.
pub(super) fn message_players(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let (signer, at) = (cx.chat_signer, &cx.origin);
    let targets = players_argument(parse, "targets", source, cx.world, at)?;
    let incoming = cx.world.with_entity_reads(|reads| {
        chat_message(
            parse,
            "message",
            source,
            at,
            reads,
            signer,
            ChatKind::MsgCommandIncoming,
        )
    })?;
    for target in &targets {
        let view = CommandPlayerView {
            name: target.name.clone(),
            uuid: target.id,
        };
        let outgoing = incoming.clone().with_target(
            ChatKind::MsgCommandOutgoing,
            team_player_display_name_nbt(&view, cx.world.scoreboard()),
        );
        run.feedback.push(CommandFeedback {
            message: outgoing.decorated(),
            kind: CommandFeedbackKind::Chat,
            // `sendChatMessage` (CommandSourceStack.java:442-451) has no
            // broadcast half.
            inform_admins: false,
            chat: Some(Box::new(outgoing)),
        });
        run.broadcast.push(CommandBroadcast::Whisper {
            player: target.clone(),
            message: incoming.clone(),
        });
    }
    Ok(i32::try_from(targets.len()).unwrap_or(i32::MAX))
}

// TeamMsgCommand.java:36-53, :60-79.
pub(super) fn team_message(
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let messages = team_messages(parse, source, cx)?;
    let receivers = i32::try_from(messages.len()).unwrap_or(i32::MAX);
    for (player, message) in messages {
        cx.world
            .send_command_chat_message(&player, &message, cx.events);
    }
    Ok(receivers)
}

fn team_messages(
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Result<Vec<(CommandPlayerTarget, OutgoingChat)>, NbtSummaryValue> {
    let (signer, at) = (cx.chat_signer, &cx.origin);
    cx.world.with_entity_reads(|reads| {
    let entity = source_entity_state(source, reads)
        .ok_or_else(|| translatable("permissions.requires.entity", Vec::new()))?;
    let team = super::selector::team_name(entity, reads)
        .and_then(|name| reads.scoreboard.team(name))
        .ok_or_else(|| translatable("commands.teammsg.failed.noteam", Vec::new()))?;
    let hover = component(translatable("chat.type.team.hover", Vec::new()));
    let team_name = super::scoreboard::with_style(
        super::team::formatted_team_name(team),
        ComponentStyle {
            hover_event: Some(ComponentHoverEvent::ShowText {
                value: hover.styled().to_vec(),
                component: Some(hover.component().clone()),
            }),
            click_event: Some(ComponentClickEvent::SuggestCommand {
                command: "/teammsg ".into(),
            }),
            ..ComponentStyle::default()
        },
    );
    let message = chat_message(
        parse,
        "message",
        source,
        at,
        reads,
        signer,
        ChatKind::TeamMsgCommandIncoming,
    )?;
    let mut messages = Vec::new();
    for player in reads.players() {
        let outgoing = matches!(entity, crate::commands::types::CommandDamageEntityRef::Player(sender) if sender.id == player.id);
        if !outgoing
            && reads.scoreboard
                .players_team(player.name)
                .map(|team| &team.name)
                != Some(&team.name)
        {
            continue;
        }
        let kind = if outgoing {
            ChatKind::TeamMsgCommandOutgoing
        } else {
            ChatKind::TeamMsgCommandIncoming
        };
        messages.push((
            player.target(),
            message.clone().with_target(kind, team_name.clone()),
        ));
    }
    Ok(messages)
    })
}
