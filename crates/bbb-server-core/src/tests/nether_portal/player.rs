use super::*;
use bbb_protocol::packets::{ClientCommandAction, PlayerAbilities};

fn end_state() -> i32 {
    vanilla_block_state_id_by_name_and_properties("minecraft:end_portal", &BTreeMap::new()).unwrap()
}

fn sent<'a>(
    events: &'a [WorldEvent],
    connection: u64,
) -> impl Iterator<Item = &'a ClientboundPlayPacket> + 'a {
    events.iter().filter_map(move |event| match event {
        WorldEvent::Send {
            connection_id,
            packet,
        } if *connection_id == ConnectionId(connection) => Some(packet),
        _ => None,
    })
}

fn feet_of(world: &ServerWorld, connection: u64) -> BlockPos {
    let position = world
        .player_transition_state_for_test(ConnectionId(connection))
        .1;
    BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

/// Stands the session on a portal block at `feet` in `dimension`.
fn stand_in(
    world: &mut ServerWorld,
    connection: u64,
    dimension: Java26BuiltinDimension,
    feet: BlockPos,
    state: i32,
) {
    set_portal_blocks(world, dimension, &[(feet, state)]);
    world.set_player_position_for_test(
        ConnectionId(connection),
        Vec3d {
            x: f64::from(feet.x) + 0.5,
            y: f64::from(feet.y),
            z: f64::from(feet.z) + 0.5,
        },
    );
}

fn ticks_until_in(
    world: &mut ServerWorld,
    connection: u64,
    dimension: Java26BuiltinDimension,
    limit: u32,
) -> (u32, Vec<WorldEvent>) {
    for tick in 1..=limit {
        let result = world.tick(Vec::new());
        if world.player_dimension(ConnectionId(connection)) == dimension {
            return (tick, result.events);
        }
    }
    panic!("connection {connection} never reached {dimension:?}");
}

/// `NetherPortalBlock.getPortalTransitionTime` (`NetherPortalBlock.java:119-127`):
/// a survival player waits `players_nether_portal_default_delay` (80) portal
/// ticks, a creative one `players_nether_portal_creative_delay`; the crossing
/// builds the Nether exit and reaches the client as a `KEEP_ALL_DATA` respawn,
/// a position update and the portal travel sound.
#[test]
fn players_cross_a_nether_portal_after_their_game_mode_delay() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    for (connection, game_type) in [(1, GameType::Survival), (2, GameType::Creative)] {
        world.set_player_mode_and_abilities_for_test(
            ConnectionId(connection),
            game_type,
            PlayerAbilities::for_game_type(game_type),
        );
    }
    assert!(world.set_game_rule_for_test(
        "minecraft:players_nether_portal_creative_delay",
        GameRuleValue::Integer(10),
    ));
    let overworld = Java26BuiltinDimension::Overworld;
    let nether = Java26BuiltinDimension::Nether;
    let feet = feet_of(&world, 1);
    stand_in(&mut world, 1, overworld, feet, portal_state("x"));
    stand_in(
        &mut world,
        2,
        overworld,
        BlockPos {
            x: feet.x + 4,
            ..feet
        },
        portal_state("x"),
    );

    let (creative_ticks, _) = ticks_until_in(&mut world, 2, nether, 100);
    assert!((11..=13).contains(&creative_ticks), "{creative_ticks}");
    assert_eq!(world.player_dimension(ConnectionId(1)), overworld);
    let (survival_ticks, events) = ticks_until_in(&mut world, 1, nether, 100);
    let survival_ticks = creative_ticks + survival_ticks;
    assert!((81..=83).contains(&survival_ticks), "{survival_ticks}");

    let packets = sent(&events, 1).collect::<Vec<_>>();
    let ClientboundPlayPacket::Respawn(respawn) = packets[0] else {
        panic!("the crossing opens with a respawn: {:?}", packets[0]);
    };
    assert_eq!(respawn.data_to_keep, 3);
    assert_eq!(respawn.common_spawn_info.dimension, "minecraft:the_nether");
    assert!(packets
        .iter()
        .any(|packet| matches!(packet, ClientboundPlayPacket::PlayerPosition(_))));
    assert!(packets.iter().any(|packet| matches!(
        packet,
        ClientboundPlayPacket::LevelEvent(event) if event.event_type == 1032
    )));
    // Both crossings share the one exit the first of them built.
    assert_eq!(nearby_portal_cell_count(&world, nether), 6);
    let landed = world.player_transition_state_for_test(ConnectionId(1)).1;
    assert!(landed.x.abs() < 20.0 && landed.z.abs() < 20.0, "{landed:?}");
}

/// `EndPortalBlock` (`EndPortalBlock.java:74-122`): into the End the player
/// lands on the obsidian platform; the End's own portal shows the credits
/// first (`ServerPlayer.showEndCredits`), and `PERFORM_RESPAWN` out of them
/// (`ServerGamePacketListenerImpl.java:1865-1870`) puts the player back at
/// spawn with `seenCredits` saved. Afterwards the exit portal no longer shows
/// the poem.
#[test]
fn end_portal_trip_shows_credits_once_and_saves_seen_credits() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let overworld = Java26BuiltinDimension::Overworld;
    let end = Java26BuiltinDimension::End;
    let feet = feet_of(&world, 1);
    stand_in(&mut world, 1, overworld, feet, end_state());

    let (_, events) = ticks_until_in(&mut world, 1, end, 5);
    let landed = world.player_transition_state_for_test(ConnectionId(1)).1;
    assert_eq!((landed.x, landed.y, landed.z), (100.5, 49.0, 0.5));
    assert_eq!(
        world.block_state(
            end,
            BlockPos {
                x: 100,
                y: 48,
                z: 0
            }
        ),
        vanilla_block_default_state_id(VANILLA_BLOCK_OBSIDIAN_ID)
    );
    assert!(sent(&events, 1).any(|packet| matches!(
        packet,
        ClientboundPlayPacket::LevelEvent(event) if event.event_type == 1032
    )));

    stand_in(
        &mut world,
        1,
        end,
        BlockPos {
            x: 100,
            y: 49,
            z: 3,
        },
        end_state(),
    );
    let tick = world.tick(Vec::new());
    let win_game = |events: &[WorldEvent]| {
        sent(events, 1).any(|packet| {
            matches!(
                packet,
                ClientboundPlayPacket::GameEvent(event) if event.event_id
                    == bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_WIN_GAME_ID
            )
        })
    };
    assert!(win_game(&tick.events));
    assert_eq!(world.player_dimension(ConnectionId(1)), end);

    let tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
    );
    assert_eq!(world.player_dimension(ConnectionId(1)), overworld);
    let respawn = sent(&tick.events, 1)
        .find_map(|packet| match packet {
            ClientboundPlayPacket::Respawn(respawn) => Some(respawn),
            _ => None,
        })
        .expect("the credits end in a respawn");
    assert_eq!(respawn.data_to_keep, 1);
    assert_eq!(respawn.common_spawn_info.dimension, "minecraft:overworld");

    let request = world
        .save_request(0, 1, bbb_server_persistence::RegionCompression::Zlib)
        .unwrap();
    let patch = &request.player_patches[0];
    assert!(patch.created.seen_credits);
    let document = bbb_server_persistence::created_player_document(
        patch,
        request.item_stack_resolver.as_dyn(),
    )
    .unwrap();
    let parsed = bbb_server_persistence::Java26PlayerData::parse(document, patch.uuid).unwrap();
    assert!(parsed.seen_credits());
    let mut restored = ServerWorld::restore(
        ServerWorldConfig::default(),
        &restore_candidate(0, None, vec![(patch.uuid, parsed)]),
    )
    .unwrap();
    let (restored_sender, restored_receiver) = mailbox();
    join_loaded(
        &restored_sender,
        &restored_receiver,
        &mut restored,
        1,
        "alice",
        1,
    );
    let request = restored
        .save_request(0, 1, bbb_server_persistence::RegionCompression::Zlib)
        .unwrap();
    assert!(request.player_patches[0].created.seen_credits);

    // Back in the End, the exit portal now takes the portal path to spawn.
    let mut events = Vec::new();
    assert!(world.teleport_player(
        ConnectionId(1),
        crate::world::TeleportTransition {
            new_level: end.into(),
            position: Vec3d {
                x: 100.5,
                y: 49.0,
                z: 0.5,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut events,
    ));
    stand_in(
        &mut world,
        1,
        end,
        BlockPos {
            x: 100,
            y: 49,
            z: 0,
        },
        end_state(),
    );
    let mut saw_win_game = false;
    for _ in 0..5 {
        let tick = world.tick(Vec::new());
        saw_win_game |= win_game(&tick.events);
        if world.player_dimension(ConnectionId(1)) == overworld {
            break;
        }
    }
    assert_eq!(world.player_dimension(ConnectionId(1)), overworld);
    assert!(!saw_win_game);
}
