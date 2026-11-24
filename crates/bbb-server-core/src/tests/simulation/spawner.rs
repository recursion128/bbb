//! `SpawnerBlock.getTicker` over a real level:
//! `BaseSpawner.serverTick`.

use super::*;

use super::nbt as entry;
use crate::entity::SpawnerSpawnData;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;

/// `SpawnData.CustomSpawnRules` that accept the mob wherever it lands, so the
/// test's subject is the spawner rather than the level's light.
fn permissive_light_rules() -> NbtSummaryValue {
    let full = || NbtSummaryValue::List(vec![NbtSummaryValue::Int(0), NbtSummaryValue::Int(15)]);
    NbtSummaryValue::Compound(vec![
        entry("block_light_limit", full()),
        entry("sky_light_limit", full()),
    ])
}

/// `SpawnData` for a zombie at a fixed `Pos`, which is the one field
/// `BaseSpawner.serverTick` reads before it draws a position of its own
fn zombie_spawn_data(position: [f64; 3]) -> SpawnerSpawnData {
    SpawnerSpawnData::new(
        vec![
            entry("id", NbtSummaryValue::string("minecraft:zombie")),
            entry(
                "Pos",
                NbtSummaryValue::List(position.iter().copied().map(nbt_double).collect()),
            ),
        ],
        Some(permissive_light_rules()),
        None,
    )
}

fn spawner_data(
    world: &mut ServerWorld,
    at: BlockPos,
) -> &crate::block_entity::base_spawner::BaseSpawnerData {
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.spawner())
        .expect("the spawner block entity is live")
        .spawner_data()
}

fn configure_spawner(world: &mut ServerWorld, at: BlockPos, position: [f64; 3]) {
    let spawner = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::spawner_mut)
        .expect("the spawner block entity is live");
    let data = spawner.spawner_data_mut();
    data.set_spawn_delay(0);
    data.set_spawn_parameters(20, 20, 1, 6, 16, 4);
    data.set_next_spawn_data(Some(zombie_spawn_data(position)));
}

fn place_spawner(world: &mut ServerWorld, at: BlockPos) {
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        block_state_id("minecraft:spawner"),
        SET_BLOCK_UPDATE_ALL,
    ));
}

/// A session standing two blocks from the spawner, which is inside its
/// `RequiredPlayerRange`.
fn spawner_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 4.5,
            y: 64.0,
            z: 4.5,
        },
    );
    (sender, receiver, world)
}

#[test]
fn a_spawner_block_spawns_its_configured_mob_and_resets_its_delay() {
    let (sender, receiver, mut world) = spawner_world();
    let at = BlockPos { x: 4, y: 64, z: 6 };
    place_spawner(&mut world, at);
    configure_spawner(&mut world, at, [4.5, 65.0, 6.5]);
    let before = world
        .entity_tracking_counts(Java26BuiltinDimension::Overworld)
        .0;

    let spawned = world.tick(receiver.drain());

    assert_eq!(
        world
            .entity_tracking_counts(Java26BuiltinDimension::Overworld)
            .0,
        before + 1
    );
    let zombie = world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_ZOMBIE_ID)
        })
        .expect("the spawner's zombie was published");
    let entity = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, zombie)
        .expect("the zombie is stored");
    assert_eq!(
        entity.position(),
        Vec3d {
            x: 4.5,
            y: 65.0,
            z: 6.5,
        }
    );
    // `BaseSpawner.delay` draws in
    // `[MinSpawnDelay, MaxSpawnDelay)` and queues event 1.
    assert_eq!(spawner_data(&mut world, at).spawn_delay(), 20);
    // `SpawnerBlockEntity`: the redraw re-sends the block entity so
    // the client's cage and delay follow the server.
    assert!(spawned.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::BlockEntityData(update),
            ..
        } if update.pos == at
    )));
    // `BaseSpawner` raises 2004 in the block-entity phase, and
    // `ServerLevel.levelEvent` broadcasts inside
    // the body that raised it, so it leaves in the spawn tick. Event 1 takes the
    // other road: `ServerLevel.blockEvent` only
    // queues it, and that queue drains at the head of the next tick
    // before `tickBlockEntities`.
    assert!(
        spawned.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::LevelEvent(level),
                ..
            } if level.event_type == 2004 && level.pos == at && level.data == 0
        )),
        "{:?}",
        spawned.events
    );
    let published = world.tick(receiver.drain());
    let events = block_events_sent(&published);
    assert!(
        events
            .iter()
            .any(|event| event.pos == at && event.b0 == 1 && event.b1 == 0),
        "{events:?}"
    );
    assert_eq!(
        published
            .events
            .iter()
            .filter(|event| matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::LevelEvent(level),
                    ..
                } if level.event_type == 2004
            ))
            .count(),
        0,
        "the spawn tick already carried 2004"
    );
    // The fixture's walk over the floor to the spawner is the session's own
    // STEP.
    let alice = world.player_entity_id_for_test(ConnectionId(1));
    let (game_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    let game_events: Vec<_> = game_events
        .iter()
        .filter(|event| event.source_entity_id != Some(alice))
        .collect();
    assert_eq!(game_events.len(), 1);
    assert_eq!(game_events[0].game_event, VanillaGameEvent::ENTITY_PLACE);
    assert_eq!(game_events[0].source_entity_id, Some(zombie));

    for (operation, command, result) in [
        (2, "team add blue", 1),
        (3, "team add red", 2),
        (4, "data merge block 4 64 6 {Delay:200s,MinSpawnDelay:200s,MaxSpawnDelay:200s,SpawnCount:1s}", 1),
    ] {
        let tick = console_command(&sender, &receiver, &mut world, operation, command);
        assert_eq!(console_command_answer(&tick).0, Some(result));
    }
    let configured = console_command(
        &sender, &receiver, &mut world, 5,
        "data merge block 4 64 6 {Delay:0s,SpawnData:{entity:{id:'minecraft:cow',Pos:[8.5d,65d,6.5d],NoAI:1b,NoGravity:1b,Team:'blue',Tags:['spawner_team']},custom_spawn_rules:{block_light_limit:[0,15],sky_light_limit:[0,15]}}}",
    );
    assert_eq!(console_command_answer(&configured).0, Some(1));
    let dimension = Java26BuiltinDimension::Overworld;
    let mut events = configured.events;
    let mut cow = None;
    for _ in 0..40 {
        cow = world
            .stored_entity_ids_for_test(dimension)
            .into_iter()
            .find(|id| {
                world
                    .stored_entity_for_test(dimension, *id)
                    .is_some_and(|entity| {
                        entity.entity_type_id()
                            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID
                            && entity.entity_tags().iter().any(|tag| tag == "spawner_team")
                    })
            });
        if cow.is_some() {
            break;
        }
        events.extend(world.tick(receiver.drain()).events);
    }
    let cow = cow.expect("the configured team cow was naturally spawned");
    let entity = world.stored_entity_for_test(dimension, cow).unwrap();
    let uuid = entity.uuid();
    let member = uuid.to_string();
    for (operation, condition, team) in [(6, "if", "blue"), (7, "unless", "red")] {
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            operation,
            &format!("execute {condition} entity @e[tag=spawner_team,team={team}]"),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
        events.extend(tick.events);
    }
    assert!(events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetPlayerTeam(team) }
        if team.name == "blue" && team.players.iter().any(|player| player == &member)
    )));
    let NbtSummaryValue::Compound(record) = world
        .stored_entity_command_data_for_test(dimension, cow)
        .unwrap()
    else {
        panic!("spawned cow record")
    };
    assert!(record
        .iter()
        .any(|entry| entry.name == "UUID" && entry.value == nbt_uuid(uuid)));
    assert!(!record.iter().any(|entry| entry.name == "Team"));
    for name in ["NoAI", "NoGravity"] {
        assert!(record
            .iter()
            .any(|entry| entry.name == name && entry.value == NbtSummaryValue::Byte(1)));
    }
    assert!(matches!(field(&record, "Tags"), NbtSummaryValue::List(tags)
        if tags.contains(&NbtSummaryValue::String("spawner_team".into()))));
    let request = world
        .save_request(1_786_000_005_000, 205, RegionCompression::Zlib)
        .unwrap();
    assert!(request
        .scoreboard
        .teams
        .iter()
        .any(|team| team.name == "blue" && team.players.iter().any(|player| player == &member)));
    assert!(!request
        .scoreboard
        .teams
        .iter()
        .any(|team| team.name == "red" && team.players.iter().any(|player| player == &member)));
}

#[test]
fn a_spawner_block_out_of_player_range_stays_dormant() {
    // No session at all, so `Level.hasNearbyAlivePlayer` answers false and
    // `BaseSpawner.serverTick` returns before it touches the delay.
    let (_, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    let at = BlockPos { x: 4, y: 64, z: 6 };
    place_spawner(&mut world, at);
    configure_spawner(&mut world, at, [4.5, 65.0, 6.5]);
    let before = world
        .entity_tracking_counts(Java26BuiltinDimension::Overworld)
        .0;

    let ticked = world.tick(receiver.drain());

    assert_eq!(
        world
            .entity_tracking_counts(Java26BuiltinDimension::Overworld)
            .0,
        before
    );
    assert_eq!(spawner_data(&mut world, at).spawn_delay(), 0);
    assert!(block_events_sent(&ticked).is_empty());
}

#[test]
fn a_spawner_block_whose_spawn_position_is_blocked_keeps_its_delay() {
    let (_, receiver, mut world) = spawner_world();
    let at = BlockPos { x: 4, y: 64, z: 6 };
    place_spawner(&mut world, at);
    configure_spawner(&mut world, at, [4.5, 65.0, 6.5]);
    // `BaseSpawner.serverTick`'s `level.noCollision` gate (`:110-111`) refuses
    // every ordinal, and a cycle that spawned nothing does not call `delay`.
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos { x: 4, y: 65, z: 6 },
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    ));
    let before = world
        .entity_tracking_counts(Java26BuiltinDimension::Overworld)
        .0;

    let ticked = world.tick(receiver.drain());

    assert_eq!(
        world
            .entity_tracking_counts(Java26BuiltinDimension::Overworld)
            .0,
        before
    );
    assert_eq!(spawner_data(&mut world, at).spawn_delay(), 0);
    assert!(block_events_sent(&ticked).is_empty());
}

#[test]
fn a_spawner_blocks_cycle_reaches_the_save_patch() {
    let (sender, receiver) = mailbox();
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = spawn_pinned_restore(datapack_config(), &candidate);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 4.5,
            y: 64.0,
            z: 4.5,
        },
    );
    let at = BlockPos { x: 4, y: 64, z: 6 };
    place_spawner(&mut world, at);
    configure_spawner(&mut world, at, [4.5, 65.0, 6.5]);
    world.tick(receiver.drain());

    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();

    let created = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .expect("the spawner's column is written");
    let parsed = Java26ChunkData::parse(
        created.document.clone(),
        Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: -4,
            section_count: 24,
        },
    )
    .expect("the runtime's own document is a valid Java 26.1 chunk");
    let record = parsed
        .block_entities()
        .iter()
        .find(|record| {
            field(record, "x") == &NbtSummaryValue::Int(at.x)
                && field(record, "y") == &NbtSummaryValue::Int(at.y)
                && field(record, "z") == &NbtSummaryValue::Int(at.z)
        })
        .unwrap_or_else(|| {
            panic!(
                "the spawner's record is written: {:?}",
                parsed.block_entities()
            )
        });
    assert_eq!(
        field(record, "id"),
        &NbtSummaryValue::string("minecraft:mob_spawner")
    );
    assert_eq!(field(record, "Delay"), &NbtSummaryValue::Short(20));
    assert_eq!(field(record, "MinSpawnDelay"), &NbtSummaryValue::Short(20));
    assert_eq!(field(record, "MaxSpawnDelay"), &NbtSummaryValue::Short(20));
    assert_eq!(field(record, "SpawnCount"), &NbtSummaryValue::Short(1));
    assert_eq!(
        field(record, "RequiredPlayerRange"),
        &NbtSummaryValue::Short(16)
    );
    assert_eq!(field(record, "SpawnRange"), &NbtSummaryValue::Short(4));
    let NbtSummaryValue::Compound(spawn_data) = field(record, "SpawnData") else {
        panic!("SpawnData is a compound: {record:?}");
    };
    let NbtSummaryValue::Compound(entity) = field(spawn_data, "entity") else {
        panic!("the spawn data carries an entity compound: {spawn_data:?}");
    };
    assert_eq!(
        field(entity, "id"),
        &NbtSummaryValue::string("minecraft:zombie")
    );
}

/// `SpawnData.CustomSpawnRules.isValidPosition`
/// reads the effective sky brightness: an open-sky spawner that demands a
/// full sky refuses at midnight and spawns at noon.
#[test]
fn a_custom_sky_light_limit_reads_the_darkened_sky() {
    let (sender, receiver, mut world) = spawner_world();
    let at = BlockPos { x: 4, y: 64, z: 6 };
    place_spawner(&mut world, at);
    let zombies = |world: &ServerWorld| {
        world
            .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
            .into_iter()
            .filter(|id| {
                world
                    .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                    .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_ZOMBIE_ID)
            })
            .count()
    };
    for (operation, time, spawns) in [(1, "midnight", false), (3, "noon", true)] {
        let before = zombies(&world);
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            operation,
            &format!("time set {time}"),
        );
        assert!(console_command_answer(&tick).0.is_some());
        world.tick(receiver.drain());
        let configured = console_command(
            &sender,
            &receiver,
            &mut world,
            operation + 1,
            "data merge block 4 64 6 {Delay:0s,SpawnData:{entity:{id:'minecraft:zombie',\
             Pos:[4.5d,65d,6.5d]},custom_spawn_rules:{sky_light_limit:[15,15]}}}",
        );
        assert_eq!(console_command_answer(&configured).0, Some(1));
        for _ in 0..3 {
            world.tick(receiver.drain());
        }
        assert_eq!(zombies(&world) > before, spawns, "time set {time}");
    }
}
