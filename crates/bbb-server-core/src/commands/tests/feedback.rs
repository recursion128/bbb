//! The success, failure and admin-broadcast channels of a run.

use super::*;

#[test]
fn a_refusal_carries_the_send_failure_root_and_a_success_does_not() {
    // `CommandSourceStack.sendFailure` (CommandSourceStack.java:495-499) sends
    // `Component.empty().append(message).withStyle(ChatFormatting.RED)`, so what
    // reaches the client is an empty red root holding the message as its only
    // sibling. `sendSuccess` (:464-477) sends the message untouched.
    // `/teammsg` refuses a source that has an entity but no team, so the source
    // has to be on the player list for this to be the `noteam` branch.
    let players = [user("alice", 1)];
    let refused = ran("teammsg hi", world(&players));
    assert_eq!(kinds(&refused), [CommandFeedbackKind::Failure]);
    assert_eq!(
        refused.feedback[0].message.component(),
        &compound(vec![
            ("text", text("")),
            ("color", text("red")),
            (
                "extra",
                NbtSummaryValue::List(vec![compound(vec![(
                    "translate",
                    text("commands.teammsg.failed.noteam"),
                )])]),
            ),
        ])
    );

    // The root is empty, so the wrapper changes how the refusal looks and not
    // what it says.
    assert_eq!(
        refused.feedback[0].message.text(),
        "commands.teammsg.failed.noteam"
    );

    let answered = ran("list", world(&[]));
    assert_eq!(kinds(&answered), [CommandFeedbackKind::Success]);
    assert_eq!(
        translate_key(answered.feedback[0].message.component()).as_deref(),
        Some("commands.list.players"),
        "a success is the message the command built, with nothing around it"
    );
}

/// `sendSuccess`'s `broadcast` argument (CommandSourceStack.java:464-477) is
/// the command's own rather than the channel's, so the flag is pinned per line
/// against the vanilla owner that passes it.
#[test]
fn every_success_line_carries_the_broadcast_flag_its_vanilla_owner_passes() {
    let players = [user("alice", 1)];
    let informs = |command: &str| -> Vec<bool> {
        let run = ran(command, world(&players));
        run.feedback
            .iter()
            .map(|feedback| {
                assert_eq!(
                    feedback.kind,
                    CommandFeedbackKind::Success,
                    "{command:?}: {run:?}"
                );
                feedback.inform_admins
            })
            .collect()
    };
    // Reports pass `false`: `ListPlayersCommand.java:35`, `SeedCommand.java:15`,
    // `GameRuleCommand.java:45`, `TagCommand.java:112`.
    assert_eq!(informs("list"), [false]);
    assert_eq!(informs("seed"), [false]);
    assert_eq!(informs("gamerule random_tick_speed"), [false]);
    assert_eq!(informs("tag alice list"), [false]);
    // Changes pass `true`: `KillCommand.java:28`, `TagCommand.java:72`,
    // `GameRuleCommand.java:39`.
    assert_eq!(informs("kill alice"), [true]);
    assert_eq!(informs("tag alice add first"), [true]);
    assert_eq!(informs("gamerule random_tick_speed 7"), [true]);
    assert_eq!(informs("setworldspawn"), [true]);
    assert_eq!(informs("attribute alice minecraft:max_health get"), [false]);
    assert_eq!(
        informs("attribute alice minecraft:max_health base get"),
        [false]
    );
    assert_eq!(
        informs("attribute alice minecraft:max_health base set 10"),
        [false]
    );
    assert_eq!(
        informs("attribute alice minecraft:max_health base reset"),
        [false]
    );
    assert_eq!(
        informs("attribute alice minecraft:max_health modifier add foo 1 add_value"),
        [false]
    );
    // The same key can travel either way: `/tick query` reports every line with
    // `false` (TickCommand.java:75-95) while `/tick freeze` writes its status
    // line with `true` (`:123`), and `/save-all` mixes the two in one run
    // (SaveAllCommand.java:24 and `:31`).
    let query = informs("tick query");
    assert!(
        query.len() > 1 && query.iter().all(|inform| !inform),
        "{query:?}"
    );
    assert_eq!(informs("tick freeze"), [true]);
    assert_eq!(informs("save-all"), [false, true]);
    // Neither `sendFailure` (`:495-499`) nor `sendSystemMessage` (`:453-462`)
    // has a broadcast half.
    let refused = ran("teammsg hello", world(&players));
    assert_eq!(kinds(&refused), [CommandFeedbackKind::Failure]);
    assert!(refused
        .feedback
        .iter()
        .all(|feedback| !feedback.inform_admins));
    let version = ran("version", world(&players));
    assert!(!version.feedback.is_empty());
    assert!(kinds(&version)
        .iter()
        .all(|kind| *kind == CommandFeedbackKind::System));
    assert!(version
        .feedback
        .iter()
        .all(|feedback| !feedback.inform_admins));
}

/// `CommandSourceStack.broadcastToAdmins` (CommandSourceStack.java:479-480):
/// `chat.type.admin` over the source's display name and the message, gray and
/// italic.
#[test]
fn the_admin_copy_wraps_the_message_in_the_admin_chat_type_over_the_source() {
    let run = ran("seed", world(&[]));
    let copy = admin_broadcast_copy(
        &source("alice", CommandPermission::Owner),
        &run.feedback[0].message,
        &crate::world::scoreboard::Scoreboard::default(),
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    );
    let NbtSummaryValue::Compound(entries) = copy.component() else {
        panic!("a component is a compound");
    };
    let field = |name: &str| {
        entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| &entry.value)
    };
    assert_eq!(field("translate"), Some(&text("chat.type.admin")));
    assert_eq!(field("color"), Some(&text("gray")));
    assert_eq!(field("italic"), Some(&NbtSummaryValue::Byte(1)));
    assert_eq!(
        field("with"),
        Some(&NbtSummaryValue::List(vec![
            display_name("alice", 1),
            run.feedback[0].message.component().clone(),
        ]))
    );
}

#[test]
fn the_admin_copy_uses_a_stored_entity_sources_display_component() {
    let display = RichTextComponent::plain("Command Cart");
    let source = CommandSource::stored_entity(
        "Command Cart",
        Uuid::from_u128(44),
        19,
        level_key(Java26BuiltinDimension::Nether),
        display.clone(),
        CommandPermission::GameMaster,
    );
    let run = ran_as("seed", &source, world(&[]));
    let copy = admin_broadcast_copy(
        &source,
        &run.feedback[0].message,
        &crate::world::scoreboard::Scoreboard::default(),
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    );
    let NbtSummaryValue::Compound(entries) = copy.component() else {
        panic!("a component is a compound");
    };
    let NbtSummaryValue::List(arguments) = &entries
        .iter()
        .find(|entry| entry.name == "with")
        .expect("the admin translation carries arguments")
        .value
    else {
        panic!("admin arguments are a list");
    };
    assert_eq!(arguments[0], display.component().clone());
}
