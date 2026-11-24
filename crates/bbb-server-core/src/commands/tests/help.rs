//! `/help` smart usage over the implemented dispatcher.

use bbb_protocol::NbtSummaryValue;

use super::*;

fn lines(run: &CommandRun) -> Vec<String> {
    run.feedback
        .iter()
        .map(|feedback| {
            assert_eq!(feedback.kind, CommandFeedbackKind::Success);
            assert!(!feedback.inform_admins);
            let NbtSummaryValue::String(line) = body(feedback) else {
                panic!("help sends literal components: {feedback:?}");
            };
            line.try_to_string().expect("fixture line is valid Unicode")
        })
        .collect()
}

fn assert_no_admin_broadcast(run: &CommandRun) {
    assert!(run.broadcast.is_empty());
    assert!(run.feedback.iter().all(|feedback| !feedback.inform_admins));
}

#[test]
fn bare_help_preserves_root_order_and_filters_by_permission() {
    let run = ran_as_default("help", &source("alice", CommandPermission::All));
    assert_eq!(run.result, Some(10));
    assert_eq!(
        lines(&run),
        [
            "/me <action>",
            "/help [<command>]",
            "/list [uuids]",
            "/msg <targets> <message>",
            "/tell ->msg",
            "/w ->msg",
            "/random (value|roll)",
            "/teammsg <message>",
            "/tm ->teammsg",
            "/trigger <objective> [add|set]",
        ]
    );
    assert_no_admin_broadcast(&run);

    let owner = ran_default("help");
    assert_eq!(owner.result, Some(87));
    assert!(lines(&owner)
        .iter()
        .any(|line| line == "/difficulty [peaceful|easy|normal|hard]"));
    assert_no_admin_broadcast(&owner);
}

#[test]
fn command_help_prints_each_child_on_its_own_line() {
    let run = ran_default("help difficulty");
    assert_eq!(run.result, Some(4));
    assert_eq!(
        lines(&run),
        [
            "/difficulty [peaceful]",
            "/difficulty [easy]",
            "/difficulty [normal]",
            "/difficulty [hard]",
        ]
    );
    assert_no_admin_broadcast(&run);
}

#[test]
fn command_help_prints_a_root_redirect_as_ellipsis() {
    let run = ran_default("help return");
    assert_eq!(run.result, Some(3));
    assert_eq!(
        lines(&run),
        ["/return <value>", "/return fail", "/return run ..."]
    );
    assert_no_admin_broadcast(&run);
}

#[test]
fn command_help_fails_when_the_nested_parse_reads_no_node() {
    let run = ran_default("help no_such_command");
    assert_eq!(keys(&run), ["commands.help.failed"]);
    assert_eq!(kinds(&run), [CommandFeedbackKind::Failure]);
    assert_no_admin_broadcast(&run);

    let hidden = ran_as_default("help difficulty", &source("alice", CommandPermission::All));
    assert_eq!(keys(&hidden), ["commands.help.failed"]);
    assert_eq!(kinds(&hidden), [CommandFeedbackKind::Failure]);
    assert_no_admin_broadcast(&hidden);
}

#[test]
fn command_help_uses_the_last_node_before_trailing_invalid_text() {
    let run = ran_default("help difficulty junk");
    assert_eq!(run.result, Some(4));
    assert_eq!(
        lines(&run),
        [
            "/difficulty junk [peaceful]",
            "/difficulty junk [easy]",
            "/difficulty junk [normal]",
            "/difficulty junk [hard]",
        ]
    );
    assert_no_admin_broadcast(&run);
}

#[test]
fn command_help_on_a_leaf_returns_zero_without_feedback() {
    let run = ran_default("help difficulty peaceful");
    assert_eq!(run.result, Some(0));
    assert!(run.feedback.is_empty());
    assert_no_admin_broadcast(&run);
}
