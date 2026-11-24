//! `ServerLevel.globalLevelEvent` (`ServerLevel.java:1071-1092`) as the wither
//! raises it when its invulnerability runs out (`WitherBoss.java:265-268`).

use super::*;

fn wither_spawn_events(
    global_sound_events: bool,
) -> Vec<(ConnectionId, bbb_protocol::packets::LevelEvent)> {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    join_loaded(&sender, &receiver, &mut world, 3, "carol", 3);
    for command in [
        "tp alice 0 80 0",
        "tp bob 1000 80 0",
        "execute in minecraft:the_nether run tp carol 0 64 0",
    ] {
        chat_command(&sender, &receiver, &mut world, 1, command);
    }
    if !global_sound_events {
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "gamerule global_sound_events false",
        );
    }
    chat_command(&sender, &receiver, &mut world, 1, "difficulty normal");
    for x in 3..=5 {
        for z in -1..=1 {
            assert!(world.set_block(
                Java26BuiltinDimension::Overworld,
                BlockPos { x, y: 79, z },
                block_state_id("minecraft:stone"),
                crate::fluid::SET_BLOCK_UPDATE_ALL,
            ));
        }
    }
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "summon minecraft:wither 4.5 80 0.5",
    );
    let mut ticks = vec![chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "data merge entity @e[type=minecraft:wither,limit=1] {Invul:1}",
    )];
    ticks.push(world.tick(receiver.drain()));
    ticks
        .iter()
        .flat_map(|tick| &tick.events)
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::LevelEvent(event),
            } if event.event_type == 1023 => Some((*connection_id, *event)),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>()
        .into_iter()
        .collect()
}

fn level_event(x: i32, y: i32, z: i32, global: bool) -> bbb_protocol::packets::LevelEvent {
    bbb_protocol::packets::LevelEvent {
        event_type: 1023,
        pos: BlockPos { x, y, z },
        data: 0,
        global,
    }
}

#[test]
fn the_wither_spawn_sound_reaches_every_player_projected_within_32_blocks() {
    assert_eq!(
        wither_spawn_events(true),
        [
            (ConnectionId(1), level_event(4, 80, 0, true)),
            (ConnectionId(2), level_event(968, 80, 0, true)),
            (ConnectionId(3), level_event(0, 64, 0, true)),
        ]
    );
}

#[test]
fn without_global_sound_events_the_wither_spawn_is_an_ordinary_level_event() {
    assert_eq!(
        wither_spawn_events(false),
        [(ConnectionId(1), level_event(4, 80, 0, false))]
    );
}
