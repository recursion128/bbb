//! `/dialog show` and `/dialog clear`.

use bbb_protocol::packets::{
    decode_play_clientbound, encode_play_clientbound_show_dialog, DialogHolder, PlayClientbound,
    ShowDialog,
};
use bbb_protocol::NbtSummaryValue;

use crate::connection::ClientboundPlayPacket;

use super::*;

fn dialog_world(players: &[NameAndId], dialogs: &[String]) -> CommandTestWorld {
    CommandTestWorld::with_dialog_data(players, dialogs)
}

/// The one packet a run sent, with its fixture connection id.
fn dialog_packet(run: &CommandRun) -> (u64, &ClientboundPlayPacket) {
    match run.sends().as_slice() {
        [sent] => *sent,
        other => panic!("one dialog packet, not {other:?}"),
    }
}

/// `LEVEL_GAMEMASTERS` (`DialogCommand.java:20`); below it the root is gone.
#[test]
fn dialog_needs_gamemaster() {
    let refusal = refused("dialog alice clear", CommandPermission::Moderator);
    assert_eq!(
        translate_key(failure_messages(&refusal)[0]).as_deref(),
        Some("command.unknown.command")
    );
}

/// `clearDialog` (`DialogCommand.java:58-70`).
#[test]
fn dialog_clear_sends_the_unit_packet() {
    let players = [user("alice", 1)];
    let dialogs = [];
    let run = ran("dialog clear alice", dialog_world(&players, &dialogs));
    assert_eq!(run.result, Some(1));
    assert_eq!(
        dialog_packet(&run),
        (1, &ClientboundPlayPacket::ClearDialog)
    );
    assert_eq!(keys(&run), ["commands.dialog.clear.single"]);
    assert_eq!(arguments(&run, 0), vec![display_name("alice", 1)]);
    assert!(run.feedback[0].inform_admins);
}

/// `showDialog` (`DialogCommand.java:44-56`) with a registry name.
#[test]
fn dialog_show_sends_a_reference_holder() {
    let players = [user("alice", 1)];
    let dialogs = ["minecraft:custom_notice".to_string()];
    let run = ran(
        "dialog show alice minecraft:custom_notice",
        dialog_world(&players, &dialogs),
    );
    assert_eq!(run.result, Some(1));
    assert_eq!(
        dialog_packet(&run),
        (
            1,
            &ClientboundPlayPacket::ShowDialog(ShowDialog {
                dialog: DialogHolder::Reference { registry_id: 0 },
            })
        )
    );
    assert_eq!(keys(&run), ["commands.dialog.show.single"]);
    assert_eq!(arguments(&run, 0), vec![display_name("alice", 1)]);
    assert!(run.feedback[0].inform_admins);
}

/// `InlineResult.parse` (`ResourceOrIdArgument.java:150-163`).
#[test]
fn dialog_show_sends_a_direct_holder_for_inline_snbt() {
    let players = [user("alice", 1)];
    let dialogs = [];
    let run = ran(
        "dialog show alice {type:'minecraft:notice',title:'Hello'}",
        dialog_world(&players, &dialogs),
    );
    assert_eq!(run.result, Some(1));
    let (_, packet) = dialog_packet(&run);
    let ClientboundPlayPacket::ShowDialog(ShowDialog {
        dialog: DialogHolder::Direct { dialog },
    }) = packet
    else {
        panic!("direct show dialog, not {packet:?}");
    };
    assert!(matches!(dialog, NbtSummaryValue::Compound(_)));
    assert_eq!(keys(&run), ["commands.dialog.show.single"]);
}

#[test]
fn dialog_show_inline_item_projects_potion_hidden_but_keeps_generic_hidden() {
    fn field<'a>(value: &'a NbtSummaryValue, name: &str) -> &'a NbtSummaryValue {
        let NbtSummaryValue::Compound(entries) = value else {
            panic!("compound {name}");
        };
        &entries
            .iter()
            .find(|entry| entry.name == name)
            .unwrap()
            .value
    }
    fn first(value: &NbtSummaryValue) -> &NbtSummaryValue {
        let NbtSummaryValue::List(values) = value else {
            panic!("list");
        };
        values.first().unwrap()
    }
    fn wire_dialog(packet: &ClientboundPlayPacket) -> NbtSummaryValue {
        let ClientboundPlayPacket::ShowDialog(show) = packet else {
            panic!("show dialog");
        };
        let (id, payload) = encode_play_clientbound_show_dialog(show).unwrap();
        let PlayClientbound::ShowDialog(ShowDialog {
            dialog: DialogHolder::Direct { dialog },
        }) = decode_play_clientbound(id, &payload).unwrap()
        else {
            panic!("direct ShowDialog packet");
        };
        dialog
    }
    let players = [user("alice", 1)];
    let run = ran(
        r#"dialog show alice {type:'notice',title:'P',body:{type:'item',item:{id:'potion',components:{potion_contents:{custom_name:"\uD800",custom_effects:[{id:'haste',hidden_effect:{duration:40}}]}}}}}"#,
        dialog_world(&players, &[]),
    );
    assert_eq!(run.result, Some(1));
    let (_, packet) = dialog_packet(&run);
    let dialog = wire_dialog(packet);
    let components = field(field(field(&dialog, "body"), "item"), "components");
    let potion_effect = first(field(
        field(components, "minecraft:potion_contents"),
        "custom_effects",
    ));
    assert!(
        matches!(potion_effect, NbtSummaryValue::Compound(entries) if !entries.iter().any(|entry| entry.name == "hidden_effect"))
    );
    let run = ran(
        "dialog show alice {type:'notice',title:'G',body:{type:'item',item:{id:'potion',components:{consumable:{on_consume_effects:[{type:'apply_effects',effects:[{id:'haste',hidden_effect:{duration:40}}]}]}}}}}",
        dialog_world(&players, &[]),
    );
    assert_eq!(run.result, Some(1));
    let (_, packet) = dialog_packet(&run);
    let dialog = wire_dialog(packet);
    let components = field(field(field(&dialog, "body"), "item"), "components");
    let consumable_effect = first(field(
        field(components, "minecraft:consumable"),
        "on_consume_effects",
    ));
    let generic_effect = first(field(consumable_effect, "effects"));
    assert!(matches!(
        field(generic_effect, "hidden_effect"),
        NbtSummaryValue::Compound(_)
    ));
}

/// `ReferenceResult.parse` (`ResourceOrIdArgument.java:184-196`).
#[test]
fn dialog_show_unknown_id_is_no_such_element() {
    let players = [user("alice", 1)];
    let dialogs = ["minecraft:custom_notice".to_string()];
    let run = ran(
        "dialog show alice minecraft:missing",
        dialog_world(&players, &dialogs),
    );
    assert_eq!(
        keys(&run),
        ["argument.resource_or_id.no_such_element", CURSOR_CONTEXT]
    );
    assert_eq!(run.result, None);
    assert!(run.sends().is_empty());
}

#[test]
fn dialog_show_invalid_inline_is_failed_to_parse() {
    let players = [user("alice", 1)];
    let dialogs = [];
    let run = ran(
        "dialog show alice {type:'minecraft:missing'}",
        dialog_world(&players, &dialogs),
    );
    assert_eq!(
        keys(&run),
        ["argument.resource_or_id.failed_to_parse", CURSOR_CONTEXT]
    );
    assert_eq!(run.result, None);
    assert!(run.sends().is_empty());
}

#[test]
fn dialog_multiple_targets_report_the_count() {
    let players = [user("alice", 1), user("bob", 2)];
    let dialogs = [];
    let run = ran("dialog clear @a", dialog_world(&players, &dialogs));
    assert_eq!(run.result, Some(2));
    assert_eq!(run.sends().len(), 2);
    assert_eq!(keys(&run), ["commands.dialog.clear.multiple"]);
    assert_eq!(arguments(&run, 0), vec![NbtSummaryValue::Int(2)]);
}

#[test]
fn dialog_unknown_player_is_the_entity_refusal() {
    let players = [user("alice", 1)];
    let dialogs = [];
    let run = ran("dialog clear bob", dialog_world(&players, &dialogs));
    assert_eq!(keys(&run), ["argument.entity.notfound.player"]);
    assert_eq!(kinds(&run), [CommandFeedbackKind::Failure]);
    assert_eq!(run.result, None);
    assert!(run.sends().is_empty());
}
