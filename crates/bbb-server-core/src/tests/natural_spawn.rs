use super::*;
use bbb_protocol::difficulties::Difficulty;

fn natural_tick_world(
    spawn_mobs: bool,
    difficulty: Difficulty,
    clock_ticks: i64,
) -> (ServerWorld, WorldCommandReceiver) {
    let (sender, receiver) = mailbox();
    let mut candidate = restore_candidate_without_tickets(396, None, Vec::new());
    candidate.game_rules.insert(
        "minecraft:spawn_mobs".to_owned(),
        bbb_protocol::game_rules::GameRuleValue::Boolean(spawn_mobs),
    );
    candidate.world_clocks = bbb_server_persistence::Java26WorldClocks::new(
        candidate
            .world_clocks
            .entries()
            .iter()
            .cloned()
            .map(|mut entry| {
                if entry.resource_id == "minecraft:overworld" {
                    entry.state.total_ticks = clock_ticks;
                }
                entry
            })
            .collect(),
    );
    let mut world = spawn_pinned_restore(
        ServerWorldConfig {
            datapacks: Some(startup_generation()),
            difficulty,
            chunk_radius: 5,
            ..ServerWorldConfig::default()
        },
        &candidate,
    );
    world.set_test_default_spawn(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::packets::Vec3d {
            x: 0.5,
            y: 96.0,
            z: 0.5,
        },
    );
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(1, "spawn-fixture", AdminScope::Command),
            request: AdminRequest::ExecuteCommand {
                command: "teleport alice 0.5 96 0.5".to_owned(),
            },
        },
    )
    .unwrap();
    // A five-chunk view tracks every chunk the two-chunk simulation distance
    // spawns into, and acknowledging each batch sends the whole view, so a
    // group is paired with the session wherever it lands.
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::ClientInformation(ClientInformation {
                view_distance: 5,
                ..Default::default()
            }),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    assert_eq!(world.game_time(), 399);
    // Off the creature cadence while the view is sent.
    world.set_game_time_for_test(1);
    for _ in 0..40 {
        if world
            .chunk_delivery_state_for_test(ConnectionId(1))
            .0
            .contains("pending_chunks: {}")
        {
            break;
        }
        enqueue(
            &sender,
            WorldCommand::Play {
                connection_id: ConnectionId(1),
                packet: PlayServerbound::ChunkBatchReceived {
                    desired_chunks_per_tick: 64.0,
                },
            },
        )
        .unwrap();
        world.tick(receiver.drain());
    }
    world.set_game_time_for_test(399);
    let dimension = Java26BuiltinDimension::Overworld;
    let (_, chunks) =
        world.natural_spawn_chunks_by_key(&world.require_level_id(&level_key(dimension)));
    let grass = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:grass_block",
        &BTreeMap::from([("snowy".to_owned(), "false".to_owned())]),
    )
    .unwrap();
    for (chunk_x, chunk_z) in chunks {
        for x in chunk_x * 16..chunk_x * 16 + 16 {
            for z in chunk_z * 16..chunk_z * 16 + 16 {
                world.set_block(
                    dimension,
                    BlockPos { x, y: 63, z },
                    grass,
                    SET_BLOCK_UPDATE_CLIENTS,
                );
            }
        }
    }
    (world, receiver)
}

/// Ticks the creature cadence (game time 400) up to 30 times until `done`
/// holds: a cadence tick lands no group with odds near 0.1 on this grass.
fn cadence_ticks(
    world: &mut ServerWorld,
    receiver: &WorldCommandReceiver,
    mut done: impl FnMut(&ServerWorld, &TickResult) -> bool,
) -> TickResult {
    let mut tick = world.tick(receiver.drain());
    for _ in 0..30 {
        assert_eq!(world.game_time(), 400);
        if done(world, &tick) {
            break;
        }
        world.set_game_time_for_test(399);
        tick = world.tick(receiver.drain());
    }
    tick
}

#[test]
fn natural_spawn_real_tick_publishes_creatures_and_obeys_the_live_rule() {
    let dimension = Java26BuiltinDimension::Overworld;
    let (mut world, receiver) = natural_tick_world(true, Difficulty::Peaceful, 6_000);
    // Groups landing in a chunk the session has not received yet stay unpaired.
    let reached = |world: &ServerWorld, tick: &TickResult| {
        world
            .stored_entity_ids_for_test(dimension)
            .iter()
            .any(|&id| {
                tick.events.iter().any(|event| {
                    matches!(
                        event,
                        WorldEvent::Send {
                            connection_id: ConnectionId(1),
                            packet: ClientboundPlayPacket::AddEntity(add),
                        } if add.id == id
                    )
                })
            })
    };
    let tick = cadence_ticks(&mut world, &receiver, reached);
    assert!(
        reached(&world, &tick),
        "no stored natural entity reached the loaded session"
    );
    let ids = world.stored_entity_ids_for_test(dimension);
    world.tick(receiver.drain());
    assert_eq!(world.stored_entity_ids_for_test(dimension), ids);

    let (mut disabled, receiver) = natural_tick_world(false, Difficulty::Peaceful, 6_000);
    disabled.tick(receiver.drain());
    assert_eq!(disabled.game_time(), 400);
    assert!(disabled.stored_entity_ids_for_test(dimension).is_empty());
}

#[test]
fn natural_spawn_real_tick_uses_the_restored_clock_for_night_monsters() {
    let monsters = |_: &ServerWorld, tick: &TickResult| {
        tick.events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::AddEntity(add),
                    ..
                } => bbb_protocol::entity_types::vanilla_entity_type(add.entity_type_id),
                _ => None,
            })
            .any(|entity| {
                entity.mob_category == bbb_protocol::entity_types::VanillaEntityMobCategory::Monster
            })
    };
    for (clock_ticks, expected_monsters) in [(6_000, false), (18_000, true)] {
        let (mut world, receiver) = natural_tick_world(true, Difficulty::Normal, clock_ticks);
        let mut seen = false;
        cadence_ticks(&mut world, &receiver, |world, tick| {
            seen |= monsters(world, tick);
            seen
        });
        assert_eq!(seen, expected_monsters, "restored clock {clock_ticks}");
    }
}
