//! `/tellraw`.

use super::*;

/// `LEVEL_GAMEMASTERS` (TellRawCommand.java:15); below it the root is gone.
#[test]
fn tellraw_needs_gamemaster() {
    let refusal = refused(
        r#"tellraw alice "hello world""#,
        CommandPermission::Moderator,
    );
    assert_eq!(
        translate_key(failure_messages(&refusal)[0]).as_deref(),
        Some("command.unknown.command")
    );
}

#[test]
fn tellraw_sends_a_literal_to_each_target_and_has_no_success_key() {
    let players = [user("alice", 1)];
    let run = ran(r#"tellraw alice "hello world""#, world(&players));
    assert_eq!(run.result, Some(1));
    assert!(run.feedback.is_empty());
    assert!(keys(&run).is_empty());
    let [message] = run.messages_to(1)[..] else {
        panic!("expected one delivery, got {:?}", run.events);
    };
    assert_eq!(message.component(), &text("hello world"));
}

#[test]
fn tellraw_parses_a_text_compound_and_the_list_example() {
    let players = [user("alice", 1)];
    let compound_form = ran(r#"tellraw @s {text:"hi"}"#, world(&players));
    assert_eq!(compound_form.result, Some(1));
    let [message] = compound_form.messages_to(1)[..] else {
        panic!("expected one delivery, got {:?}", compound_form.events);
    };
    // `ComponentSerialization.STREAM_CODEC` is `fromCodecWithRegistries(CODEC)`
    // (ComponentSerialization.java:37-44), which collapses an unstyled literal.
    assert_eq!(message.component(), &text("hi"));

    let list_form = ran(r#"tellraw alice [""]"#, world(&players));
    assert_eq!(list_form.result, Some(1));
    let [message] = list_form.messages_to(1)[..] else {
        panic!("expected one delivery, got {:?}", list_form.events);
    };
    assert_eq!(message.component(), &text(""));
}

#[test]
fn tellraw_unknown_player_is_the_entity_refusal() {
    let players = [user("alice", 1)];
    let run = ran(r#"tellraw bob "hello""#, world(&players));
    assert_eq!(keys(&run), ["argument.entity.notfound.player"]);
    assert_eq!(kinds(&run), [CommandFeedbackKind::Failure]);
    assert_eq!(run.result, None);
    assert!(run.messages_to(1).is_empty());
}

#[test]
fn tellraw_invalid_snbt_or_component_uses_the_component_refusal() {
    let players = [user("alice", 1)];
    // `withCodec` (CommandArgumentParser.java:38-45) wraps only what the codec
    // rejects: an unreadable SNBT value leaves through the grammar's own
    // failure, which the pinned client-26.1 jar answers for a bare `{`.
    for (command, key) in [
        (r#"tellraw alice {"#, "argument.literal.incorrect"),
        (r#"tellraw alice {foo:1}"#, "argument.component.invalid"),
    ] {
        let run = ran(command, world(&players));
        assert_eq!(keys(&run)[0], key, "{command}");
        assert_eq!(run.result, None, "{command}");
        assert!(run.messages_to(1).is_empty(), "{command}");
    }
}
