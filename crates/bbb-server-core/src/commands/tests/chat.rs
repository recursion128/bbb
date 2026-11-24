//! `/say`, `/me`, `/msg` and `/teammsg`.

use super::*;

#[test]
fn say_broadcasts_the_announcement_decoration_and_keeps_the_message_spacing() {
    let run = ran("say  hello  world ", world(&[]));
    assert!(
        run.feedback.is_empty(),
        "a say answers nobody in particular"
    );
    assert_eq!(run.result, Some(1));
    assert_eq!(run.broadcast.len(), 1);
    assert_eq!(
        chat_broadcast_message(&run.broadcast[0]),
        compound(vec![
            ("translate", text("chat.type.announcement")),
            (
                "with",
                NbtSummaryValue::List(vec![display_name("alice", 1), text(" hello  world "),])
            ),
        ])
    );
}

#[test]
fn me_broadcasts_the_emote_decoration() {
    let run = ran("me waves", world(&[]));
    assert_eq!(run.result, Some(1));
    assert_eq!(run.broadcast.len(), 1);
    assert_eq!(
        chat_broadcast_message(&run.broadcast[0]),
        compound(vec![
            ("translate", text("chat.type.emote")),
            (
                "with",
                NbtSummaryValue::List(vec![display_name("alice", 1), text("waves")])
            ),
        ])
    );
}

/// `MessageArgument.parseText` (MessageArgument.java:127-130): a sender
/// without `COMMANDS_ENTITY_SELECTORS` sends `@e[` as plain text.
#[test]
fn me_from_a_non_op_keeps_selector_text_literal() {
    let run = ran_as(
        "me waves @e[",
        &source("alice", CommandPermission::All),
        world(&[]),
    );
    assert_eq!(run.result, Some(1));
    assert_eq!(
        chat_broadcast_message(&run.broadcast[0]),
        compound(vec![
            ("translate", text("chat.type.emote")),
            (
                "with",
                NbtSummaryValue::List(vec![display_name("alice", 1), text("waves @e[")])
            ),
        ])
    );
}

#[test]
fn msg_answers_the_source_and_carries_the_incoming_line_to_each_target() {
    let players = [user("alice", 1), user("bob", 2)];
    let run = ran("msg bob hello", world(&players));
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.message.display.outgoing"]);
    assert_eq!(kinds(&run), [CommandFeedbackKind::Chat]);
    assert!(!run.feedback[0].inform_admins);
    // `ChatTypeDecoration.outgoingDirectMessage` styles the line gray and
    // italic and takes the target name before the content.
    assert_eq!(
        run.feedback[0].message.component(),
        &compound(vec![
            ("translate", text("commands.message.display.outgoing")),
            (
                "with",
                NbtSummaryValue::List(vec![display_name("bob", 2), text("hello")])
            ),
            ("italic", NbtSummaryValue::Byte(1)),
            ("color", text("gray")),
        ])
    );
    assert_eq!(msg_recipients(&run), [2]);
}

#[test]
fn msg_multiple_targets_preserves_chat_feedback_and_delivery_order() {
    let players = [user("alice", 1), user("bob", 2)];
    let run = ran("msg @a hello", world(&players));
    assert_eq!(run.result, Some(2));
    assert_eq!(kinds(&run), [CommandFeedbackKind::Chat; 2]);
    assert!(run.feedback.iter().all(|line| !line.inform_admins));
    assert_eq!(msg_recipients(&run), [1, 2]);
}

#[test]
fn merging_command_runs_preserves_interleaved_chat_and_system_broadcast_order() {
    let first = ran("say first", world(&[]));
    let second = ran("me second", world(&[]));
    let system = ran("random roll 1..10", world(&[]));
    let expected = [
        first.broadcast.clone(),
        system.broadcast.clone(),
        second.broadcast.clone(),
    ]
    .concat();
    let mut combined = CommandRun::default();
    combined.absorb(first);
    combined.absorb(system);
    combined.absorb(second);
    assert_eq!(combined.broadcast, expected);
    assert!(matches!(
        combined.broadcast.as_slice(),
        [
            CommandBroadcast::Chat(_),
            CommandBroadcast::System(_),
            CommandBroadcast::Chat(_)
        ]
    ));
}

#[test]
fn msg_to_nobody_reports_the_argument_refusal_rather_than_an_empty_ran() {
    let players = [user("alice", 1)];
    let run = ran("msg bob hello", world(&players));
    assert_eq!(keys(&run), ["argument.entity.notfound.player"]);
    assert_eq!(kinds(&run), [CommandFeedbackKind::Failure]);
    assert_eq!(run.result, None);
}

#[test]
fn tell_and_w_reach_the_command_msg_redirects_to() {
    let players = [user("alice", 1), user("bob", 2)];
    for command in ["tell bob hello", "w bob hello"] {
        let run = ran(command, world(&players));
        assert_eq!(kinds(&run), [CommandFeedbackKind::Chat]);
        assert!(!run.feedback[0].inform_admins);
        assert_eq!(msg_recipients(&run), [2], "{command}");
        assert_eq!(
            keys(&run),
            ["commands.message.display.outgoing"],
            "{command}"
        );
    }
}

#[test]
fn teammsg_refuses_a_sender_without_a_team() {
    let players = [user("alice", 1)];
    let run = ran("teammsg hello", world(&players));
    assert_eq!(keys(&run), ["commands.teammsg.failed.noteam"]);
    assert_eq!(kinds(&run), [CommandFeedbackKind::Failure]);
}

#[test]
fn teammsg_asks_for_an_entity_before_it_asks_for_a_team() {
    // `TeamMsgCommand` (TeamMsgCommand.java:36-39) calls
    // `source.getEntityOrException()` first and only then reads `getTeam()`, so
    // a source carrying no entity never reaches `ERROR_NOT_ON_TEAM`: it gets
    // `permissions.requires.entity` from `CommandSourceStack.ERROR_NOT_ENTITY`
    // (CommandSourceStack.java:53, :393-399).
    //
    // The management surface is such a source. It runs at owner level under a
    // principal that is not on the player list, which is what "holds no entity"
    // projects to here.
    let console = source("management", CommandPermission::Owner);
    let offstage = ran_as("teammsg hello", &console, world(&[]));
    assert_eq!(keys(&offstage), ["permissions.requires.entity"]);
    assert_eq!(kinds(&offstage), [CommandFeedbackKind::Failure]);
    assert_eq!(offstage.result, None);
    assert!(offstage.broadcast.is_empty());

    // The same source once it does have an entity: the entity check passes and
    // the team check is what refuses.
    let players = [user("management", 1)];
    let onstage = ran_as("teammsg hello", &console, world(&players));
    assert_eq!(onstage.result, None);
    assert_eq!(keys(&onstage), ["commands.teammsg.failed.noteam"]);

    // `tm` redirects to the same node (TeamMsgCommand.java:57), so the alias
    // takes both branches with it.
    assert_eq!(
        keys(&ran_as("tm hello", &console, world(&[]))),
        ["permissions.requires.entity"]
    );
    assert_eq!(
        keys(&ran_as("tm hello", &console, world(&players))),
        ["commands.teammsg.failed.noteam"]
    );
}
