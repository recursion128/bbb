//! `/clear` against actual player inventories.

use super::*;
use crate::mailbox::{WorldCommandReceiver, WorldCommandSender};
use crate::tests::{console_command, mailbox};
use crate::world::{ServerWorld, ServerWorldConfig, TickResult};

fn holding(
    players: &[(&str, u128, i32)],
) -> (ServerWorld, WorldCommandSender, WorldCommandReceiver) {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    world.set_test_respawn_radius(0);
    for (index, &(name, uuid, count)) in players.iter().enumerate() {
        let connection = (index + 1) as u64;
        crate::tests::join_loaded(&sender, &receiver, &mut world, connection, name, uuid);
        if count > 0 {
            world.stock_menu_slot_for_test(
                crate::connection::ConnectionId(connection),
                36,
                bbb_protocol::packets::ItemStackSummary {
                    item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
                    count,
                    component_patch: Default::default(),
                },
            );
        }
    }
    (world, sender, receiver)
}

fn clear(command: &str, players: &[(&str, u128, i32)]) -> (ServerWorld, TickResult) {
    let (mut world, sender, receiver) = holding(players);
    let tick = console_command(&sender, &receiver, &mut world, 1, command);
    (world, tick)
}

fn report(tick: &TickResult) -> &crate::admin::CommandReport {
    tick.events
        .iter()
        .find_map(|event| match event {
            crate::world::WorldEvent::Admin(outcome) => match &outcome.result {
                Ok(crate::admin::AdminResult::Command(report)) => Some(report),
                _ => None,
            },
            _ => None,
        })
        .unwrap()
}

fn message_body(value: &NbtSummaryValue) -> &NbtSummaryValue {
    if translate_key(value).as_deref().is_some() {
        return value;
    }
    let NbtSummaryValue::Compound(entries) = value else {
        return value;
    };
    match entries
        .iter()
        .find(|entry| entry.name == "extra")
        .map(|entry| &entry.value)
    {
        Some(NbtSummaryValue::List(children)) if children.len() == 1 => &children[0],
        _ => value,
    }
}

fn clear_answer(tick: &TickResult) -> (Option<i32>, Vec<String>) {
    let report = report(tick);
    (
        report.result,
        report
            .messages
            .iter()
            .map(|message| {
                translate_key(message_body(message.component()))
                    .as_deref()
                    .unwrap_or(CURSOR_CONTEXT)
                    .to_owned()
            })
            .collect(),
    )
}

fn feedback_arguments(tick: &TickResult) -> Vec<NbtSummaryValue> {
    let report = report(tick);
    let NbtSummaryValue::Compound(fields) = message_body(report.messages[0].component()) else {
        panic!("translatable feedback");
    };
    match &fields
        .iter()
        .find(|entry| entry.name == "with")
        .unwrap()
        .value
    {
        NbtSummaryValue::List(values) => values.clone(),
        _ => panic!("feedback arguments"),
    }
}

#[test]
fn clear_empties_the_source_player() {
    let (world, tick) = clear("execute as alice run clear", &[("alice", 1, 64)]);
    assert_eq!(
        clear_answer(&tick),
        (Some(64), vec!["commands.clear.success.single".into()])
    );
    assert_eq!(
        feedback_arguments(&tick),
        [NbtSummaryValue::Int(64), display_name("alice", 1)]
    );
    assert_eq!(
        world
            .player_inventory_slot_for_test(crate::connection::ConnectionId(1), 0)
            .count,
        0
    );
}

#[test]
fn clear_empties_every_named_target() {
    let (world, tick) = clear("clear @a", &[("alice", 1, 10), ("bob", 2, 5)]);
    assert_eq!(
        clear_answer(&tick),
        (Some(15), vec!["commands.clear.success.multiple".into()])
    );
    assert_eq!(
        feedback_arguments(&tick),
        [NbtSummaryValue::Int(15), NbtSummaryValue::Int(2)]
    );
    for connection in [1, 2] {
        assert_eq!(
            world
                .player_inventory_slot_for_test(crate::connection::ConnectionId(connection), 0)
                .count,
            0
        );
    }
}

#[test]
fn clear_refuses_an_empty_inventory_and_keeps_its_failure_message() {
    let (_, tick) = clear("clear alice", &[("alice", 1, 0)]);
    assert_eq!(
        clear_answer(&tick),
        (None, vec!["clear.failed.single".into()])
    );
    assert_eq!(
        feedback_arguments(&tick),
        [bbb_protocol::component_literal_nbt("alice")]
    );
}

#[test]
fn clear_refuses_empty_targets_and_keeps_its_failure_message() {
    let (_, tick) = clear("clear @a", &[("alice", 1, 0), ("bob", 2, 0)]);
    assert_eq!(
        clear_answer(&tick),
        (None, vec!["clear.failed.multiple".into()])
    );
    assert_eq!(feedback_arguments(&tick), [NbtSummaryValue::Int(2)]);
}

#[test]
fn clear_from_the_console_needs_a_player() {
    let (_, tick) = clear("clear", &[]);
    assert_eq!(
        clear_answer(&tick),
        (None, vec!["permissions.requires.player".into()])
    );
}

#[test]
fn clear_matches_real_component_data_and_counting_keeps_stacks() {
    let (mut world, sender, receiver) = holding(&[("alice", 1, 0)]);
    for command in [
        "item replace entity alice hotbar.0 with stone[custom_data={marker:{value:2}}] 7",
        "item replace entity alice hotbar.1 with stone[custom_data={marker:{value:3}}] 5",
    ] {
        assert!(
            report(&console_command(&sender, &receiver, &mut world, 1, command))
                .result
                .is_some()
        );
    }
    let tick = console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "clear alice *[custom_data~{marker:{value:2}}] 0",
    );
    assert_eq!(
        clear_answer(&tick),
        (Some(7), vec!["commands.clear.test.single".into()])
    );
    assert_eq!(
        world
            .player_inventory_slot_for_test(crate::connection::ConnectionId(1), 0)
            .count,
        7
    );
    let tick = console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "clear alice stone[custom_data~{marker:{value:2}}]",
    );
    assert_eq!(clear_answer(&tick).0, Some(7));
    assert_eq!(
        world
            .player_inventory_slot_for_test(crate::connection::ConnectionId(1), 0)
            .count,
        0
    );
    assert_eq!(
        world
            .player_inventory_slot_for_test(crate::connection::ConnectionId(1), 1)
            .count,
        5
    );
}

#[test]
fn clear_shares_each_players_limit_across_inventory_crafting_and_cursor() {
    let (mut world, sender, receiver) = holding(&[("alice", 1, 2), ("bob", 2, 2)]);
    for name in ["alice", "bob"] {
        for (slot, count) in [
            ("hotbar.1", 3),
            ("player.crafting.2", 4),
            ("player.cursor", 6),
        ] {
            let command = format!("item replace entity {name} {slot} with stone {count}");
            assert!(report(&console_command(
                &sender, &receiver, &mut world, 1, &command
            ))
            .result
            .is_some());
        }
    }
    let tick = console_command(&sender, &receiver, &mut world, 1, "clear @a stone 0");
    assert_eq!(
        clear_answer(&tick),
        (Some(30), vec!["commands.clear.test.multiple".into()])
    );
    let tick = console_command(&sender, &receiver, &mut world, 1, "clear @a stone 11");
    assert_eq!(
        clear_answer(&tick),
        (Some(22), vec!["commands.clear.success.multiple".into()])
    );
    for connection in [1, 2] {
        for slot in [0, 1] {
            assert_eq!(
                world
                    .player_inventory_slot_for_test(
                        crate::connection::ConnectionId(connection),
                        slot
                    )
                    .count,
                0
            );
        }
    }
    for name in ["alice", "bob"] {
        let command = format!("execute if items entity {name} player.crafting.* stone");
        assert_eq!(
            report(&console_command(
                &sender, &receiver, &mut world, 1, &command
            ))
            .result,
            None
        );
        let command = format!("execute if items entity {name} player.cursor stone");
        assert_eq!(
            report(&console_command(
                &sender, &receiver, &mut world, 1, &command
            ))
            .result,
            Some(4)
        );
    }
    let tick = console_command(&sender, &receiver, &mut world, 2, "clear @a stone 0");
    assert_eq!(clear_answer(&tick).0, Some(8));
}

#[test]
fn clear_is_hidden_from_a_moderator() {
    let refusal = refused("clear", CommandPermission::Moderator);
    assert_eq!(keys(&refusal), ["command.unknown.command", CURSOR_CONTEXT]);
}
