//! `/title`.

use bbb_protocol::packets::{
    ClearTitles, SetActionBarText, SetSubtitleText, SetTitleText, SetTitlesAnimation,
};

use super::*;
use crate::connection::ClientboundPlayPacket;

/// The one packet a run sent, with its fixture connection id.
fn title_packet(run: &CommandRun) -> (u64, &ClientboundPlayPacket) {
    match run.sends().as_slice() {
        [sent] => *sent,
        other => panic!("one title packet, not {other:?}"),
    }
}

/// `LEVEL_GAMEMASTERS` (TitleCommand.java:29); below it the root is gone.
#[test]
fn title_needs_gamemaster() {
    let refusal = refused("title alice clear", CommandPermission::Moderator);
    assert_eq!(
        translate_key(failure_messages(&refusal)[0]).as_deref(),
        Some("command.unknown.command")
    );
}

/// `clearTitle` (TitleCommand.java:104-118): `ClearTitles(false)`, then
/// `commands.title.cleared.single` with `true`.
#[test]
fn title_clear_sends_clear_titles_false() {
    let players = [user("alice", 1)];
    let run = ran("title alice clear", world(&players));
    assert_eq!(run.result, Some(1));
    assert_eq!(
        title_packet(&run),
        (
            1,
            &ClientboundPlayPacket::ClearTitles(ClearTitles { reset_times: false })
        )
    );
    assert_eq!(keys(&run), ["commands.title.cleared.single"]);
    assert_eq!(arguments(&run, 0), vec![display_name("alice", 1)]);
    assert!(run.feedback[0].inform_admins);
}

/// `resetTitle` (TitleCommand.java:120-134): `ClearTitles(true)`.
#[test]
fn title_reset_sends_clear_titles_true() {
    let players = [user("alice", 1)];
    let run = ran("title alice reset", world(&players));
    assert_eq!(run.result, Some(1));
    assert_eq!(
        title_packet(&run).1,
        &ClientboundPlayPacket::ClearTitles(ClearTitles { reset_times: true })
    );
    assert_eq!(keys(&run), ["commands.title.reset.single"]);
    assert!(run.feedback[0].inform_admins);
}

/// The three `showTitle` factories (TitleCommand.java:38-76, `:136-155`).
#[test]
fn title_show_sends_the_matching_text_packet() {
    let players = [user("alice", 1)];

    let title = ran(r#"title alice title "hello world""#, world(&players));
    let (target, packet) = title_packet(&title);
    assert_eq!(target, 1);
    let ClientboundPlayPacket::SetTitleText(SetTitleText { content }) = packet else {
        panic!("title packet, not {packet:?}");
    };
    assert_eq!(content.component(), &text("hello world"));
    assert_eq!(keys(&title), ["commands.title.show.title.single"]);
    assert_eq!(arguments(&title, 0), vec![display_name("alice", 1)]);
    assert!(title.feedback[0].inform_admins);
    assert_eq!(title.result, Some(1));

    let subtitle = ran(r#"title alice subtitle {text:"hi"}"#, world(&players));
    let ClientboundPlayPacket::SetSubtitleText(SetSubtitleText { content }) =
        title_packet(&subtitle).1
    else {
        panic!("subtitle packet, not {:?}", subtitle.sends());
    };
    // Every title packet's codec is `fromCodec*(CODEC)`
    // (ComponentSerialization.java:37-44), so the typed compound leaves as the
    // bare string `tryCollapseToString` answers.
    assert_eq!(content.component(), &text("hi"));
    assert_eq!(keys(&subtitle), ["commands.title.show.subtitle.single"]);

    let actionbar = ran(r#"title alice actionbar [""]"#, world(&players));
    let ClientboundPlayPacket::SetActionBarText(SetActionBarText { content }) =
        title_packet(&actionbar).1
    else {
        panic!("actionbar packet, not {:?}", actionbar.sends());
    };
    assert_eq!(content.component(), &text(""));
    assert_eq!(keys(&actionbar), ["commands.title.show.actionbar.single"]);
}

/// `setTimes` (TitleCommand.java:157-171): fadeIn/stay/fadeOut as ticks.
#[test]
fn title_times_sends_the_animation_packet() {
    let players = [user("alice", 1)];
    for command in [
        "title alice times 10t 70t 20t",
        "title alice times 10 70 20",
    ] {
        let run = ran(command, world(&players));
        assert_eq!(run.result, Some(1), "{command}");
        assert_eq!(
            title_packet(&run).1,
            &ClientboundPlayPacket::SetTitlesAnimation(SetTitlesAnimation {
                fade_in: 10,
                stay: 70,
                fade_out: 20,
            }),
            "{command}"
        );
        assert_eq!(keys(&run), ["commands.title.times.single"], "{command}");
        assert!(run.feedback[0].inform_admins, "{command}");
    }
}

/// Two targets word the `.multiple` key over `targets.size()`.
#[test]
fn title_multiple_targets_report_the_count() {
    let players = [user("alice", 1), user("bob", 2)];
    let run = ran("title @a clear", world(&players));
    assert_eq!(run.result, Some(2));
    assert_eq!(run.sends().len(), 2);
    assert_eq!(keys(&run), ["commands.title.cleared.multiple"]);
    assert_eq!(arguments(&run, 0), vec![NbtSummaryValue::Int(2)]);
}

#[test]
fn title_unknown_player_is_the_entity_refusal() {
    let players = [user("alice", 1)];
    let run = ran("title bob clear", world(&players));
    assert_eq!(keys(&run), ["argument.entity.notfound.player"]);
    assert_eq!(kinds(&run), [CommandFeedbackKind::Failure]);
    assert_eq!(run.result, None);
    assert!(run.sends().is_empty());
}

#[test]
fn title_invalid_snbt_or_component_uses_the_component_refusal() {
    let players = [user("alice", 1)];
    for (command, key) in [
        (r#"title alice title {"#, "argument.literal.incorrect"),
        (r#"title alice title {foo:1}"#, "argument.component.invalid"),
    ] {
        let run = ran(command, world(&players));
        assert_eq!(keys(&run)[0], key, "{command}");
        assert_eq!(run.result, None, "{command}");
        assert!(run.sends().is_empty(), "{command}");
    }
}
