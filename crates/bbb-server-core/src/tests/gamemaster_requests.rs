//! The gamemaster client requests (`ServerGamePacketListenerImpl.java:787-816`,
//! `:1893-1906`, `:2077-2088`) through real session packets.

use super::*;

use bbb_protocol::packets::GameRuleValues;

/// Alice is an owner, bob holds no operator entry and carol is a moderator:
/// on the operator list, but one level short of the gamemaster permission.
fn gamemaster_request_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let mut config = operator_config(&[("alice", 1), ("carol", 3)]);
    config
        .admin_lists
        .operators
        .get_mut(&Uuid::from_u128(3))
        .expect("carol is on the operator list")
        .level = CommandPermission::Moderator;
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    join_loaded(&sender, &receiver, &mut world, 3, "carol", 3);
    settle_joined_bodies(&mut world);
    (sender, receiver, world)
}

/// The game type `ServerPlayer.setGameMode` announced to the session itself
/// (`ServerPlayer.java:1740`), if it moved.
fn announced_game_type(tick: &TickResult, connection_id: u64) -> Option<i32> {
    tick.events.iter().find_map(|event| {
        match event {
        WorldEvent::Send {
            connection_id: recipient,
            packet: ClientboundPlayPacket::GameEvent(update),
        } if *recipient == ConnectionId(connection_id)
            && update.event_id
                == bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_CHANGE_GAME_MODE_ID =>
        {
            Some(update.param as i32)
        }
        _ => None,
    }
    })
}

fn game_rule_readout(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        connection_id,
        PlayServerbound::ClientCommand(ClientCommandAction::RequestGameRuleValues),
    )
}

fn readout_values(tick: &TickResult) -> BTreeMap<String, String> {
    let [WorldEvent::Send {
        connection_id: ConnectionId(1),
        packet: ClientboundPlayPacket::GameRuleValues(GameRuleValues { values }),
    }] = tick.events.as_slice()
    else {
        panic!("one game rule readout to the asker: {tick:?}");
    };
    values
        .iter()
        .map(|value| (value.rule.clone(), value.value.clone()))
        .collect()
}

#[test]
fn a_gamemaster_request_changes_its_own_game_mode_exactly_as_its_gamemode_command() {
    let (sender, receiver, mut requested) = gamemaster_request_world();
    let (typed_sender, typed_receiver, mut typed) = gamemaster_request_world();

    let request = play(
        &sender,
        &receiver,
        &mut requested,
        1,
        PlayServerbound::ChangeGameMode(ChangeGameModeCommand {
            game_mode: GameType::Survival,
        }),
    );
    let command = chat_command(
        &typed_sender,
        &typed_receiver,
        &mut typed,
        1,
        "gamemode survival",
    );
    assert_eq!(request.events, command.events);
    assert_eq!(
        announced_game_type(&request, 1),
        Some(GameType::Survival.id())
    );
    // `logGamemodeChange`'s self branch (GameModeCommand.java:38-39): the
    // asker's own line, and the `chat.type.admin` copy for carol, who is on
    // the operator list.
    let chats = system_chats(&request);
    assert_eq!(chats.len(), 2, "{chats:?}");
    assert_eq!(chats[0].0, ConnectionId(1));
    assert!(chats[0].1.starts_with("commands.gamemode.success.self"));
    assert_eq!(chats[1].0, ConnectionId(3));

    // The next tick drops the creative ranges (ServerPlayer.java:610-627).
    requested.tick(Vec::new());
    // `ServerPlayer.setGameMode` answers false for the mode the session is
    // already in, and `logGamemodeChange` is never reached
    // (GameModeCommand.java:66-71).
    let again = play(
        &sender,
        &receiver,
        &mut requested,
        1,
        PlayServerbound::ChangeGameMode(ChangeGameModeCommand {
            game_mode: GameType::Survival,
        }),
    );
    assert!(again.events.is_empty(), "{again:?}");
}

#[test]
fn a_gamemaster_request_writes_each_known_game_rule_and_tells_the_operators() {
    let (sender, receiver, mut world) = gamemaster_request_world();
    let entry = |rule: &str, value: &str| SetGameRuleEntry {
        rule: rule.to_string(),
        value: value.to_string(),
    };
    let tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetGameRule(SetGameRule {
            entries: vec![
                entry("minecraft:keep_inventory", "true"),
                entry("minecraft:not_a_rule", "true"),
                entry("random_tick_speed", "abc"),
                entry("minecraft:max_snow_accumulation_height", "9"),
                entry("minecraft:random_tick_speed", "7 "),
                entry("random_tick_speed", "7"),
                // Outside the level's feature set: `GameRules.set` discards the
                // write, and the broadcast still goes out (:807-808).
                entry("minecraft:max_minecart_speed", "10"),
            ],
        }),
    );
    // `broadcastGameRuleChangeToOperators` (:812-816) reaches everyone on the
    // operator list, carol included, and nobody else.
    assert_eq!(
        system_chats(&tick),
        [
            (ConnectionId(1), "commands.gamerule.set keep_inventorytrue"),
            (ConnectionId(3), "commands.gamerule.set keep_inventorytrue"),
            (ConnectionId(1), "commands.gamerule.set random_tick_speed7"),
            (ConnectionId(3), "commands.gamerule.set random_tick_speed7"),
            (
                ConnectionId(1),
                "commands.gamerule.set max_minecart_speed10"
            ),
            (
                ConnectionId(3),
                "commands.gamerule.set max_minecart_speed10"
            ),
        ]
        .map(|(connection_id, text)| (connection_id, text.to_string()))
    );

    let values = readout_values(&game_rule_readout(&sender, &receiver, &mut world, 1));
    assert_eq!(values["minecraft:keep_inventory"], "true");
    assert_eq!(values["minecraft:random_tick_speed"], "7");
    assert_eq!(values["minecraft:max_snow_accumulation_height"], "1");
    assert!(!values.contains_key("minecraft:max_minecart_speed"));
}

#[test]
fn a_gamemaster_game_rule_readout_answers_every_available_rule() {
    let (sender, receiver, mut world) = gamemaster_request_world();
    let values = readout_values(&game_rule_readout(&sender, &receiver, &mut world, 1));
    let mut available = bbb_protocol::game_rules::VANILLA_GAME_RULES
        .iter()
        .filter(|rule| rule.required_features.is_empty())
        .map(|rule| (rule.resource_id.to_string(), rule.default_value.serialize()))
        .collect::<BTreeMap<_, _>>();
    // `spawn_pinned_world` writes this one rule.
    available.insert("minecraft:respawn_radius".to_string(), "0".to_string());
    assert_eq!(values, available);
}

#[test]
fn gamemaster_requests_below_the_gamemaster_permission_change_and_answer_nothing() {
    let (sender, receiver, mut world) = gamemaster_request_world();
    for connection_id in [2, 3] {
        for packet in [
            PlayServerbound::ChangeGameMode(ChangeGameModeCommand {
                game_mode: GameType::Survival,
            }),
            PlayServerbound::SetGameRule(SetGameRule {
                entries: vec![SetGameRuleEntry {
                    rule: "minecraft:keep_inventory".to_string(),
                    value: "true".to_string(),
                }],
            }),
            PlayServerbound::ClientCommand(ClientCommandAction::RequestGameRuleValues),
        ] {
            let tick = play(&sender, &receiver, &mut world, connection_id, packet);
            assert!(
                tick.events.is_empty(),
                "connection {connection_id}: {tick:?}"
            );
        }
    }
    let values = readout_values(&game_rule_readout(&sender, &receiver, &mut world, 1));
    assert_eq!(values["minecraft:keep_inventory"], "false");
}
