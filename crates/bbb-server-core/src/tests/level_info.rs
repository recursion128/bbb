//! `PlayerList.sendLevelInfo` (`PlayerList.java:644-657`), the login's level
//! flags (`:161-176`), `MinecraftServer.onGameRuleChanged`
//! (`MinecraftServer.java:2092-2120`) and the hardcore respawn
//! (`ServerGamePacketListenerImpl.java:1879-1882`).

use super::*;

fn sent_to(tick: &TickResult, connection: u64) -> Vec<ClientboundPlayPacket> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet,
            } if *connection_id == ConnectionId(connection) => Some(packet.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn the_join_carries_the_rules_border_and_spawn_and_rule_changes_reach_every_player() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    for rule in [
        "minecraft:reduced_debug_info",
        "minecraft:immediate_respawn",
        "minecraft:limited_crafting",
    ] {
        assert!(world.set_game_rule_for_test(rule, GameRuleValue::Boolean(true)));
    }
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let joined = sent_to(&world.tick(receiver.drain()), 1);
    let Some(ClientboundPlayPacket::Login(login)) = joined.first() else {
        panic!("the join opens with the login: {joined:?}")
    };
    assert!(!login.hardcore);
    assert!(login.reduced_debug_info && !login.show_death_screen && login.do_limited_crafting);
    let index = |matches: fn(&ClientboundPlayPacket) -> bool| {
        joined
            .iter()
            .position(matches)
            .expect("sendLevelInfo sends it")
    };
    let border = index(|packet| matches!(packet, ClientboundPlayPacket::InitializeBorder(_)));
    let time = index(|packet| matches!(packet, ClientboundPlayPacket::SetTime(_)));
    let spawn = index(|packet| matches!(packet, ClientboundPlayPacket::SetDefaultSpawnPosition(_)));
    let load_start = index(|packet| {
        matches!(packet, ClientboundPlayPacket::GameEvent(event)
            if event.event_id == bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID)
    });
    assert!(border < time && time < spawn && spawn < load_start);
    let ClientboundPlayPacket::InitializeBorder(border) = &joined[border] else {
        unreachable!()
    };
    assert_eq!(
        (border.new_size, border.new_absolute_max_size),
        (5.999997E7_f32 as f64, 29_999_984)
    );

    let entity_id = world.publication().players[0].entity_id;
    let changed = sent_to(
        &chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "gamerule reduced_debug_info false",
        ),
        1,
    );
    assert!(changed.iter().any(|packet| matches!(packet,
        ClientboundPlayPacket::EntityEvent(event) if event.entity_id == entity_id && event.event_id == 23)));
    let changed = sent_to(
        &chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "gamerule limited_crafting false",
        ),
        1,
    );
    assert!(changed.iter().any(|packet| matches!(packet,
        ClientboundPlayPacket::GameEvent(event) if event.event_id == 12 && event.param == 0.0)));
    let changed = sent_to(
        &chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "gamerule advance_time false",
        ),
        1,
    );
    assert!(changed.iter().any(|packet| matches!(packet,
        ClientboundPlayPacket::SetTime(time) if time.clock_updates.iter().all(|clock| clock.rate == 0.0))));
}

#[test]
fn a_hardcore_respawn_is_a_spectator() {
    let mut candidate = restore_candidate(0, None, Vec::new());
    super::debug_dimension::set_hardcore(&mut candidate, true);
    let mut world = ServerWorld::restore(operator_config(&[("alice", 1)]), &candidate).unwrap();
    let (sender, receiver) = mailbox();
    let (joined, _) = join_loaded_ticks(&sender, &receiver, &mut world, 1, "alice", 1);
    let joined = sent_to(&joined, 1);
    assert!(matches!(joined.first(), Some(ClientboundPlayPacket::Login(login)) if login.hardcore));
    chat_command(&sender, &receiver, &mut world, 1, "kill alice");
    let respawned = sent_to(
        &play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
        ),
        1,
    );
    let respawn = respawned
        .iter()
        .position(|packet| matches!(packet, ClientboundPlayPacket::Respawn(_)))
        .expect("the dead player respawns");
    let spectator = respawned
        .iter()
        .position(|packet| {
            matches!(packet, ClientboundPlayPacket::GameEvent(event)
                if event.event_id == bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_CHANGE_GAME_MODE_ID
                    && event.param == 3.0)
        })
        .expect("hardcore moves the respawned player to spectator");
    assert!(respawn < spectator);
    assert_eq!(
        world.game_rule_for_test("minecraft:spectators_generate_chunks"),
        Some(GameRuleValue::Boolean(false))
    );
}
