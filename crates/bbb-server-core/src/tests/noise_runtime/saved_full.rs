use super::*;
use crate::tests::entities::{console_command, console_command_answer};

#[test]
fn saved_overworld_noise_full_batch_restarts_and_serves_without_regeneration() {
    const SPAWN: [i32; 3] = [-32, 69, 0];
    const CENTER: (i32, i32) = (-2, 0);
    const LIVE_X: i32 = -31;
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-noise-full-resume-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let paths = WorldSavePaths::new(&root).unwrap();
    let candidate = noise_candidate();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    storage
        .create_world(
            &Java26CreatedSave {
                level: crate::world_creation::created_level_document(&Java26WorldCreation {
                    level_name: "bbb noise resume".to_string(),
                    game_type: ServerWorld::fixture_game_type_id(),
                    difficulty: Difficulty::Easy.serialized_name().to_string(),
                    spawn_position: SPAWN,
                    last_played: 1_786_000_000_000,
                })
                .unwrap(),
                world_gen_settings: candidate.world_gen_settings.document.clone(),
                chunks: Vec::new(),
            },
            RegionCompression::Zlib,
            99,
        )
        .unwrap();
    let baseline = storage.load_candidate().unwrap();
    assert!(baseline
        .dimensions
        .values()
        .all(|level| level.chunks.is_empty()));

    let mut world = noise_world(&baseline);
    let (sender, receiver) = mailbox();
    let first = join_noise_player(&sender, &receiver, &mut world, 97);
    let spawned = world.publication().players[0].position;
    assert_eq!((spawned.x, spawned.z), (-41.5, -9.5));
    assert!((60.0..100.0).contains(&spawned.y), "{spawned:?}");
    let initial = chunk_packets(&first, 97);
    assert_eq!(initial.len(), 9, "{:#?}", closed_reasons(&first));
    let surface_y = match world
        .chunk_store_for_test(Java26BuiltinDimension::Overworld)
        .generation_state(CENTER.0, CENTER.1)
        .unwrap()
    {
        bbb_server_types::chunk_data::ChunkGenerationState::NoiseFull(full) => full
            .light()
            .initialized()
            .features()
            .final_heightmaps()
            .first_available(
                bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
                1,
                0,
            )
            .unwrap(),
        state => panic!("expected FULL, found {state:?}"),
    };
    let stone = block_state_id("minecraft:stone");
    let note_block = crate::block_behavior::note_block::state_for_placement(
        block_state_id("minecraft:air"),
        stone,
    );
    for (y, block_state_id) in [(surface_y, stone), (surface_y + 1, note_block)] {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            BlockPos { x: LIVE_X, y, z: 0 },
            block_state_id,
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
    }
    let peer = join_noise_player(&sender, &receiver, &mut world, 98);
    assert!(
        closed_reasons(&peer).is_empty(),
        "a peer spawn search must flush another session's live relight: {:#?}",
        closed_reasons(&peer)
    );
    let second = play(
        &sender,
        &receiver,
        &mut world,
        97,
        PlayServerbound::ChunkBatchReceived {
            desired_chunks_per_tick: 1.0,
        },
    );
    assert!(
        !second
            .events
            .iter()
            .any(|event| matches!(event, WorldEvent::Close { .. })),
        "a pending live relight must commit before the next batch grows its dependency graph"
    );
    assert_eq!(chunk_packets(&second, 97).len(), 1);
    for _ in 0..crate::chunk_sender::MAX_UNACKNOWLEDGED_BATCHES {
        world.tick(receiver.drain());
    }
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos {
            x: LIVE_X + 1,
            y: 319,
            z: 0
        },
        block_state_id("minecraft:glowstone"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let relit = world.tick(receiver.drain());
    assert!(!relit.snapshot.stopping, "{:?}", relit.events);
    assert_eq!(
        world
            .chunk_store_for_test(Java26BuiltinDimension::Overworld)
            .block_light_level(LIVE_X + 1, 319, 0),
        15
    );
    ServerWorld::encode_chunk_for_test(
        world.chunk_store_for_test(Java26BuiltinDimension::Overworld),
        CENTER,
    )
    .expect("ordinary ticks commit live light before the save barrier");
    let request = world
        .save_request(1_786_000_000_000, 100, RegionCompression::Zlib)
        .unwrap();
    let saved_full = request
        .chunk_writes
        .iter()
        .filter(|write| {
            Java26ChunkData::parse(
                write.document.clone(),
                Java26ChunkContext {
                    expected_x: write.chunk_x,
                    expected_z: write.chunk_z,
                    min_section_y: -4,
                    section_count: 24,
                },
            )
            .unwrap()
            .status()
                == Java26ChunkStatus::Full
        })
        .count();
    // PLAYER_LOADING level 31 spans view radius two; FULL extends two dependency rings.
    assert_eq!(saved_full, 81);
    storage.save_snapshot(&baseline, &request).unwrap();

    world.acknowledge_save_success(&request);
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos {
            x: LIVE_X,
            y: surface_y + 1,
            z: 0,
        },
        block_state_id("minecraft:air"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let acknowledged_fresh_rewrite = world
        .save_request(1_786_000_000_500, 101, RegionCompression::Zlib)
        .unwrap();
    assert!(acknowledged_fresh_rewrite
        .chunk_writes
        .iter()
        .any(|entry| (entry.chunk_x, entry.chunk_z) == CENTER));
    storage
        .save_snapshot(&baseline, &acknowledged_fresh_rewrite)
        .unwrap();
    world.acknowledge_save_success(&acknowledged_fresh_rewrite);
    let expected = initial
        .iter()
        .map(|chunk| {
            ServerWorld::encode_chunk_for_test(
                world.chunk_store_for_test(Java26BuiltinDimension::Overworld),
                (chunk.x, chunk.z),
            )
            .map(|packet| (*packet).clone())
        })
        .collect::<SaveResult<Vec<_>>>()
        .unwrap();

    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let saved = crate::tests::read_stored_candidate(storage.load_candidate().unwrap());
    assert_eq!(
        saved.dimensions[&Java26LevelKey::from(Java26BuiltinDimension::Overworld)]
            .chunks
            .values()
            .filter(|chunk| chunk.data.status() == Java26ChunkStatus::Full)
            .count(),
        saved_full
    );
    let mut restarted = noise_world(&saved);
    let (sender, receiver) = mailbox();
    let resumed = join_noise_player(&sender, &receiver, &mut restarted, 99);
    assert!(
        !resumed
            .events
            .iter()
            .any(|event| matches!(event, WorldEvent::Close { .. })),
        "a storage-restored FULL batch must be serveable"
    );
    let resumed = chunk_packets(&resumed, 99);
    assert_eq!(resumed.len(), expected.len());
    let original_chunks = world.chunk_store_for_test(Java26BuiltinDimension::Overworld);
    let restored_chunks = restarted.chunk_store_for_test(Java26BuiltinDimension::Overworld);
    for (resumed, expected) in resumed.iter().zip(&expected) {
        assert_eq!((resumed.x, resumed.z), (expected.x, expected.z));
        assert_eq!(
            resumed.chunk_data.heightmaps, expected.chunk_data.heightmaps,
            "restored heightmaps differ at [{},{}]",
            resumed.x, resumed.z
        );
        assert_same_column_cells(
            original_chunks.column(expected.x, expected.z).unwrap(),
            restored_chunks.column(resumed.x, resumed.z).unwrap(),
            (resumed.x, resumed.z),
        );
        assert_eq!(
            resumed.chunk_data.block_entities, expected.chunk_data.block_entities,
            "restored block entities differ at [{},{}]",
            resumed.x, resumed.z
        );
        assert_eq!(
            resumed.light_data, expected.light_data,
            "restored light differs at [{},{}]",
            resumed.x, resumed.z
        );
    }

    let dimension = Java26LevelKey::from(Java26BuiltinDimension::Overworld);
    let border = saved.dimensions[&dimension]
        .chunks
        .iter()
        .filter(|(_, column)| column.data.status() != Java26ChunkStatus::Full)
        .min_by_key(|(&(x, z), _)| (x - CENTER.0).abs().max((z - CENTER.1).abs()))
        .map(|(&chunk, _)| chunk)
        .expect("saved loading halo contains a proto border");
    restarted.prepare_full_for_test(Java26BuiltinDimension::Overworld, &[border]);
    assert!(restarted.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos {
            x: border.0 * 16 + 8,
            y: 319,
            z: border.1 * 16 + 8
        },
        stone,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let continuation = restarted
        .save_request(1_786_000_000_750, 102, RegionCompression::Zlib)
        .unwrap();
    assert!(!continuation.chunk_writes.is_empty());
    assert!(continuation.chunk_writes.iter().any(|write| {
        saved.dimensions[&write.dimension]
            .chunks
            .get(&(write.chunk_x, write.chunk_z))
            .is_some_and(|chunk| chunk.data.status() != Java26ChunkStatus::Full)
    }));
    storage.save_snapshot(&saved, &continuation).unwrap();
    restarted.acknowledge_save_success(&continuation);
    assert_eq!(
        restarted
            .chunk_store_for_test(Java26BuiltinDimension::Overworld)
            .pending_whole_rewrites()
            .count(),
        0
    );

    assert!(restarted.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos {
            x: LIVE_X,
            y: surface_y + 1,
            z: 0,
        },
        note_block,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let rewrite = restarted
        .save_request(1_786_000_001_000, 103, RegionCompression::Zlib)
        .unwrap();
    assert!(rewrite
        .chunk_writes
        .iter()
        .any(|entry| (entry.chunk_x, entry.chunk_z) == CENTER));
    let expected_rewrites = rewrite
        .chunk_writes
        .iter()
        .map(|entry| {
            let data = Java26ChunkData::parse(
                entry.document.clone(),
                Java26ChunkContext {
                    expected_x: entry.chunk_x,
                    expected_z: entry.chunk_z,
                    min_section_y: -4,
                    section_count: 24,
                },
            )
            .unwrap();
            assert_eq!(data.heightmaps().len(), 4);
            ((entry.chunk_x, entry.chunk_z), data)
        })
        .collect::<BTreeMap<_, _>>();
    let rewritten_expected = resumed
        .iter()
        .map(|chunk| {
            ServerWorld::encode_chunk_for_test(
                restarted.chunk_store_for_test(Java26BuiltinDimension::Overworld),
                (chunk.x, chunk.z),
            )
            .map(|packet| (*packet).clone())
        })
        .collect::<SaveResult<Vec<_>>>()
        .unwrap();
    storage.save_snapshot(&saved, &rewrite).unwrap();
    restarted.acknowledge_save_success(&rewrite);
    assert_eq!(
        restarted
            .chunk_store_for_test(Java26BuiltinDimension::Overworld)
            .pending_whole_rewrites()
            .count(),
        0
    );

    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let rewritten = crate::tests::read_stored_candidate(storage.load_candidate().unwrap());
    for (chunk, expected) in &expected_rewrites {
        let actual = &rewritten.dimensions
            [&Java26LevelKey::from(Java26BuiltinDimension::Overworld)]
            .chunks[chunk]
            .data;
        assert_eq!(actual.sections(), expected.sections());
        assert_eq!(actual.heightmaps(), expected.heightmaps());
        assert_eq!(actual.block_entities(), expected.block_entities());
        assert!(actual.is_light_on());
    }
    let mut rewritten_world = noise_world(&rewritten);
    let (sender, receiver) = mailbox();
    let rewritten_packets = chunk_packets(
        &join_noise_player(&sender, &receiver, &mut rewritten_world, 100),
        100,
    );
    assert_eq!(rewritten_packets.len(), rewritten_expected.len());
    let before_restart = restarted.chunk_store_for_test(Java26BuiltinDimension::Overworld);
    let after_restart = rewritten_world.chunk_store_for_test(Java26BuiltinDimension::Overworld);
    for (actual, expected) in rewritten_packets.iter().zip(&rewritten_expected) {
        assert_eq!((actual.x, actual.z), (expected.x, expected.z));
        assert_eq!(actual.chunk_data.heightmaps, expected.chunk_data.heightmaps);
        assert_same_column_cells(
            before_restart.column(expected.x, expected.z).unwrap(),
            after_restart.column(actual.x, actual.z).unwrap(),
            (actual.x, actual.z),
        );
        assert_eq!(
            actual.chunk_data.block_entities,
            expected.chunk_data.block_entities
        );
        if actual.light_data != expected.light_data {
            let first_array_difference = actual
                .light_data
                .sky_updates
                .iter()
                .chain(&actual.light_data.block_updates)
                .zip(
                    expected
                        .light_data
                        .sky_updates
                        .iter()
                        .chain(&expected.light_data.block_updates),
                )
                .enumerate()
                .find_map(|(array, (actual, expected))| {
                    actual
                        .iter()
                        .zip(expected)
                        .position(|(actual, expected)| actual != expected)
                        .map(|byte| (array, byte, actual[byte], expected[byte]))
                });
            panic!(
                "rewritten light differs at [{},{}]: first {first_array_difference:?}, saved sky y=4 byte15 {:?}, continuation rewrite {}, live rewrite {}",
                actual.x,
                actual.z,
                rewritten.dimensions[&Java26LevelKey::from(
                    Java26BuiltinDimension::Overworld,
                )]
                    .chunks[&(actual.x, actual.z)]
                    .data
                    .sections()
                    .iter()
                    .find(|section| section.y == 4)
                    .and_then(|section| section.sky_light.as_ref())
                    .map(|bytes| bytes[15]),
                continuation
                    .chunk_writes
                    .iter()
                    .any(|write| (write.chunk_x, write.chunk_z) == (actual.x, actual.z)),
                rewrite
                    .chunk_writes
                    .iter()
                    .any(|write| (write.chunk_x, write.chunk_z) == (actual.x, actual.z)),
            );
        }
    }
    let crafter =
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:crafter").unwrap();
    let crafter =
        bbb_protocol::block_state_types::vanilla_block_default_state_id(crafter.id).unwrap();
    let crafter_pos = BlockPos {
        x: LIVE_X,
        y: 317,
        z: 1,
    };
    assert!(rewritten_world.set_block(
        Java26BuiltinDimension::Overworld,
        crafter_pos,
        crafter,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(100),
        },
    )
    .unwrap();
    rewritten_world.tick(receiver.drain());
    rewritten_world.tick(receiver.drain());
    for (level, (x, z)) in rewritten_world.pending_runtime_entity_chunks() {
        let record = storage
            .read_runtime_entity_chunk(&rewritten, &level, x, z)
            .unwrap();
        rewritten_world
            .complete_runtime_entity_chunk(&level, (x, z), record)
            .unwrap();
    }
    assert!(rewritten_world
        .runtime_unload_columns()
        .contains(&(dimension.clone(), CENTER)));
    let request = rewritten_world
        .save_request(1_786_000_001_500, 104, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&rewritten, &request).unwrap();
    let input = storage
        .read_runtime_column(&rewritten, &dimension, CENTER.0, CENTER.1)
        .unwrap();
    rewritten_world.retain_runtime_unload_columns(&dimension, vec![(CENTER, input)]);
    rewritten_world.acknowledge_save_success(&request);
    assert!(!rewritten_world
        .chunk_store_for_test(Java26BuiltinDimension::Overworld)
        .is_loaded(CENTER.0, CENTER.1));
    assert_eq!(
        rewritten_world
            .chunk_store_for_test(Java26BuiltinDimension::Overworld)
            .get_block_state(crafter_pos.x, crafter_pos.y, crafter_pos.z),
        Some(crafter)
    );
    assert_eq!(
        rewritten_world.block_state(Java26BuiltinDimension::Overworld, crafter_pos),
        Some(crafter)
    );
    ServerWorld::encode_chunk_for_test(
        rewritten_world.chunk_store_for_test(Java26BuiltinDimension::Overworld),
        CENTER,
    )
    .unwrap();
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn noise_runtime_reload_serves_an_unloaded_full_column_beside_loaded_chunks() {
    const RELOADED: (i32, i32) = (0, 1);
    // One column beyond the second ticket's level-44 reach, so RELOADED unloads alone.
    const KEPT: (i32, i32) = (14, 1);
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-noise-runtime-reload-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let candidate = noise_candidate();
    let storage = Java26WorldStorage::open(
        WorldSavePaths::new(&root).unwrap(),
        WorldSaveLimits::default(),
    )
    .unwrap();
    storage
        .create_world(
            &Java26CreatedSave {
                level: crate::world_creation::created_level_document(&Java26WorldCreation {
                    level_name: "bbb noise runtime reload".to_string(),
                    game_type: ServerWorld::fixture_game_type_id(),
                    difficulty: Difficulty::Easy.serialized_name().to_string(),
                    spawn_position: [0, 64, 16],
                    last_played: 1_786_000_000_000,
                })
                .unwrap(),
                world_gen_settings: candidate.world_gen_settings.document.clone(),
                chunks: Vec::new(),
            },
            RegionCompression::Zlib,
            99,
        )
        .unwrap();
    let mut baseline = storage.load_candidate().unwrap();
    // The reads below arrive the way the runtime's IO thread delivers them.
    baseline.column_reader = None;
    let mut world = noise_world(&baseline);
    let overworld = crate::world::LevelId::from(Java26BuiltinDimension::Overworld);
    let key = Java26LevelKey::from(Java26BuiltinDimension::Overworld);
    let at = |(x, z): (i32, i32)| bbb_protocol::packets::ChunkPos { x, z };
    let probe = BlockPos { x: 0, y: 64, z: 16 };
    let marker = BlockPos {
        x: 1,
        y: 300,
        z: 17,
    };
    let glowstone = block_state_id("minecraft:glowstone");

    world.set_chunks_forced(overworld, true, &[at(RELOADED), at(KEPT)]);
    world.tick(Vec::new());
    assert!(world.is_execute_chunk_loaded(&key, probe));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        marker,
        glowstone,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));

    let (sender, receiver) = mailbox();
    for (operation, command, result) in [(1, "team add red", 1), (2, "team add blue", 2)] {
        let tick = console_command(&sender, &receiver, &mut world, operation, command);
        assert_eq!(console_command_answer(&tick).0, Some(result));
    }
    let summoned = console_command(
        &sender,
        &receiver,
        &mut world,
        3,
        "summon minecraft:cow 0.5 300 16.5 {NoAI:1b,NoGravity:1b,Silent:1b,Tags:['runtime_team_probe'],Team:'red'}",
    );
    assert_eq!(console_command_answer(&summoned).0, Some(1));
    let dimension = Java26BuiltinDimension::Overworld;
    let cow = world
        .stored_entity_ids_for_test(dimension)
        .into_iter()
        .find(|id| {
            let entity = world.stored_entity_for_test(dimension, *id).unwrap();
            entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID
                && entity
                    .entity_tags()
                    .iter()
                    .any(|tag| tag == "runtime_team_probe")
        })
        .unwrap();
    let entity = world.stored_entity_for_test(dimension, cow).unwrap();
    let cow_uuid = entity.uuid();
    let cow_position = entity.position();
    let member = cow_uuid.to_string();
    let summoned = console_command(
        &sender,
        &receiver,
        &mut world,
        6,
        "summon minecraft:oak_chest_boat 4.5 300 16.5 {NoGravity:1b,Tags:['runtime_team_boat'],Passengers:[{id:'minecraft:cow',NoAI:1b,NoGravity:1b,Silent:1b,Tags:['runtime_team_child'],Team:'red'}]}",
    );
    assert_eq!(console_command_answer(&summoned).0, Some(1));
    let boat = world
        .stored_entity_ids_for_test(dimension)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(dimension, *id)
                .unwrap()
                .entity_tags()
                .iter()
                .any(|tag| tag == "runtime_team_boat")
        })
        .unwrap();
    let boat_body = world.stored_entity_for_test(dimension, boat).unwrap();
    assert_eq!(
        boat_body.data().entity_resource_id(),
        "minecraft:oak_chest_boat"
    );
    let boat_uuid = boat_body.uuid();
    let child = *world
        .stored_passengers_for_test(dimension, boat)
        .first()
        .unwrap();
    assert_eq!(world.stored_passengers_for_test(dimension, boat), &[child]);
    let child_body = world.stored_entity_for_test(dimension, child).unwrap();
    assert_eq!(
        child_body.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID
    );
    assert!(child_body
        .entity_tags()
        .iter()
        .any(|tag| tag == "runtime_team_child"));
    let child_uuid = child_body.uuid();
    let child_member = child_uuid.to_string();
    let summoned = console_command(
        &sender, &receiver, &mut world, 9,
        "summon minecraft:armor_stand 8.5 300 16.5 {Team:'red',NoGravity:1b,Silent:1b,ShowArms:1b,Tags:['runtime_stand_team']}",
    );
    assert_eq!(console_command_answer(&summoned).0, Some(1));
    let stand = world
        .stored_entity_ids_for_test(dimension)
        .into_iter()
        .find(|id| {
            let body = world.stored_entity_for_test(dimension, *id).unwrap();
            body.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
                && body
                    .entity_tags()
                    .iter()
                    .any(|tag| tag == "runtime_stand_team")
        })
        .unwrap();
    let stand_uuid = world
        .stored_entity_for_test(dimension, stand)
        .unwrap()
        .uuid();
    let stand_member = stand_uuid.to_string();

    for (operation, command) in [
        (20, "summon minecraft:oak_chest_boat 12.5 300 16.5 {NoGravity:1b,Tags:['runtime_stand_boat']}"),
        (21, "summon minecraft:armor_stand 12.5 302 16.5 {Team:'red',NoGravity:1b,Silent:1b,ShowArms:1b,Tags:['runtime_stand_child']}"),
    ] {
        let tick = console_command(&sender, &receiver, &mut world, operation, command);
        assert_eq!(console_command_answer(&tick).0, Some(1));
    }
    let stand_boat = world
        .stored_entity_ids_for_test(dimension)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(dimension, *id)
                .unwrap()
                .entity_tags()
                .iter()
                .any(|tag| tag == "runtime_stand_boat")
        })
        .unwrap();
    let stand_child = world
        .stored_entity_ids_for_test(dimension)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(dimension, *id)
                .unwrap()
                .entity_tags()
                .iter()
                .any(|tag| tag == "runtime_stand_child")
        })
        .unwrap();
    let stand_boat_body = world.stored_entity_for_test(dimension, stand_boat).unwrap();
    assert_eq!(
        stand_boat_body.data().entity_resource_id(),
        "minecraft:oak_chest_boat"
    );
    let stand_boat_uuid = stand_boat_body.uuid();
    let stand_child_body = world
        .stored_entity_for_test(dimension, stand_child)
        .unwrap();
    assert_eq!(
        stand_child_body.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
    );
    let stand_child_uuid = stand_child_body.uuid();
    let stand_child_member = stand_child_uuid.to_string();
    let mounted = console_command(
        &sender,
        &receiver,
        &mut world,
        22,
        &format!("ride {stand_child_uuid} mount {stand_boat_uuid}"),
    );
    assert_eq!(console_command_answer(&mounted).0, Some(1));
    assert_eq!(
        world.stored_passengers_for_test(dimension, stand_boat),
        &[stand_child]
    );

    world.set_chunks_forced(overworld, false, &[at(RELOADED)]);
    world.tick(Vec::new());
    assert!(world.needs_runtime_unload_save());
    let mut request = world
        .runtime_unload_save_request(100, RegionCompression::Zlib)
        .unwrap();
    assert!(request
        .scoreboard
        .teams
        .iter()
        .any(|team| { team.name == "red" && team.players.iter().any(|player| player == &member) }));
    assert!(!request.scoreboard.teams.iter().any(|team| {
        team.name == "blue" && team.players.iter().any(|player| player == &member)
    }));
    assert!(request.scoreboard.teams.iter().any(
        |team| team.name == "red" && team.players.iter().any(|player| player == &child_member)
    ));
    assert!(!request.scoreboard.teams.iter().any(
        |team| team.name == "blue" && team.players.iter().any(|player| player == &child_member)
    ));
    assert!(request.scoreboard.teams.iter().any(
        |team| team.name == "red" && team.players.iter().any(|player| player == &stand_member)
    ));
    assert!(!request.scoreboard.teams.iter().any(
        |team| team.name == "blue" && team.players.iter().any(|player| player == &stand_member)
    ));
    let mut injected = false;
    let mut injected_child = false;
    let mut injected_stand = false;
    assert!(request
        .scoreboard
        .teams
        .iter()
        .any(|team| team.name == "red"
            && team
                .players
                .iter()
                .any(|player| player == &stand_child_member)));
    assert!(!request
        .scoreboard
        .teams
        .iter()
        .any(|team| team.name == "blue"
            && team
                .players
                .iter()
                .any(|player| player == &stand_child_member)));
    let mut injected_stand_child = false;
    for write in &mut request.entity_writes {
        let NbtSummaryValue::List(records) = &mut write
            .document
            .entries_mut()
            .iter_mut()
            .find(|entry| entry.name == "Entities")
            .unwrap()
            .value
        else {
            panic!("saved entity list")
        };
        for record in records {
            let NbtSummaryValue::Compound(fields) = record else {
                panic!("saved entity record")
            };
            if fields
                .iter()
                .any(|entry| entry.name == "UUID" && entry.value == nbt_uuid(cow_uuid))
            {
                assert!(!injected);
                assert!(!fields.iter().any(|entry| entry.name == "Team"));
                fields.push(nbt("Team", NbtSummaryValue::string("blue")));
                injected = true;
            }
            if fields
                .iter()
                .any(|entry| entry.name == "UUID" && entry.value == nbt_uuid(stand_uuid))
            {
                assert!(!injected_stand);
                assert!(!fields.iter().any(|entry| entry.name == "Team"));
                fields.push(nbt("Team", NbtSummaryValue::string("blue")));
                injected_stand = true;
            }
            if fields
                .iter()
                .any(|entry| entry.name == "UUID" && entry.value == nbt_uuid(boat_uuid))
            {
                let NbtSummaryValue::List(passengers) = &mut fields
                    .iter_mut()
                    .find(|entry| entry.name == "Passengers")
                    .unwrap()
                    .value
                else {
                    panic!("saved passengers")
                };
                let child = passengers
                    .iter_mut()
                    .find_map(|value| match value {
                        NbtSummaryValue::Compound(fields)
                            if fields.iter().any(|entry| {
                                entry.name == "UUID" && entry.value == nbt_uuid(child_uuid)
                            }) =>
                        {
                            Some(fields)
                        }
                        _ => None,
                    })
                    .unwrap();
                assert!(!injected_child);
                assert!(!child.iter().any(|entry| entry.name == "Team"));
                child.push(nbt("Team", NbtSummaryValue::string("blue")));
                injected_child = true;
            }
            if fields
                .iter()
                .any(|entry| entry.name == "UUID" && entry.value == nbt_uuid(stand_boat_uuid))
            {
                let NbtSummaryValue::List(passengers) = &mut fields
                    .iter_mut()
                    .find(|entry| entry.name == "Passengers")
                    .unwrap()
                    .value
                else {
                    panic!("saved stand passengers")
                };
                let child = passengers
                    .iter_mut()
                    .find_map(|value| match value {
                        NbtSummaryValue::Compound(fields)
                            if fields.iter().any(|entry| {
                                entry.name == "UUID" && entry.value == nbt_uuid(stand_child_uuid)
                            }) =>
                        {
                            Some(fields)
                        }
                        _ => None,
                    })
                    .unwrap();
                assert!(!injected_stand_child);
                assert!(!child.iter().any(|entry| entry.name == "Team"));
                child.push(nbt("Team", NbtSummaryValue::string("blue")));
                injected_stand_child = true;
            }
        }
    }
    assert!(injected);
    assert!(injected_child);
    assert!(injected_stand);
    assert!(injected_stand_child);
    storage.save_runtime_unloads(&baseline, &request).unwrap();
    let unloading = world
        .runtime_unload_columns()
        .into_iter()
        .map(|(_, (x, z))| {
            let record = storage.read_runtime_column(&baseline, &key, x, z).unwrap();
            ((x, z), record)
        })
        .collect();
    world.retain_runtime_unload_columns(&key, unloading);
    world.acknowledge_save_success(&request);
    assert!(world
        .live_entity_id_for_uuid_for_test(dimension, cow_uuid)
        .is_none());
    assert!(world
        .live_entity_id_for_uuid_for_test(dimension, stand_uuid)
        .is_none());
    for uuid in [boat_uuid, child_uuid] {
        assert!(world
            .live_entity_id_for_uuid_for_test(dimension, uuid)
            .is_none());
    }
    for uuid in [stand_boat_uuid, stand_child_uuid] {
        assert!(world
            .live_entity_id_for_uuid_for_test(dimension, uuid)
            .is_none());
    }
    let chunks = world.chunk_store_for_test(Java26BuiltinDimension::Overworld);
    assert!(!chunks.is_loaded(RELOADED.0, RELOADED.1));
    assert!(chunks.is_loaded(KEPT.0, KEPT.1));
    assert!(!world.is_execute_chunk_loaded(&key, probe));

    let lectern = BlockPos {
        x: KEPT.0 * 16 + 1,
        y: 300,
        z: KEPT.1 * 16 + 1,
    };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        lectern,
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_LECTERN_ID,
        )
        .unwrap(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.with_level_for_test(Java26BuiltinDimension::Overworld, |level| {
        assert!(level
            .poi_mut()
            .acquire_ticket([lectern.x, lectern.y, lectern.z]));
    });

    world.set_chunks_forced(overworld, true, &[at(RELOADED)]);
    world.tick(Vec::new());
    let pending = world.pending_runtime_columns();
    assert!(!pending.is_empty());
    // A column whose record cannot be read is generated afresh instead of
    // holding back the rest of the level.
    let unreadable = pending[0].1;
    let records = pending
        .into_iter()
        .map(|(_, (x, z))| {
            let record = if (x, z) == unreadable {
                Err(bbb_server_types::save::SaveError::InvalidNbt {
                    path: None,
                    detail: "unreadable test record".into(),
                })
            } else {
                Ok(storage.read_runtime_column(&baseline, &key, x, z).unwrap())
            };
            ((x, z), record)
        })
        .collect();
    let failures = world.complete_runtime_columns(&key, records);
    assert_eq!(
        failures.iter().map(|(chunk, _)| *chunk).collect::<Vec<_>>(),
        vec![unreadable]
    );
    assert!(world.pending_runtime_columns().is_empty());
    for _ in 0..4 {
        world.tick(Vec::new());
    }
    let mut read_cow = false;
    let mut read_child = false;
    let mut read_stand = false;
    let mut read_stand_child = false;
    for _ in 0..20 {
        for (level, chunk) in world.pending_runtime_entity_chunks() {
            let record = storage
                .read_runtime_entity_chunk(&baseline, &level, chunk.0, chunk.1)
                .unwrap();
            if level == key && chunk == RELOADED {
                assert!(!world.is_execute_chunk_loaded(&key, probe));
                let cow = record
                    .as_ref()
                    .unwrap()
                    .data
                    .entities()
                    .find(|record| record.uuid() == cow_uuid)
                    .unwrap();
                assert!(cow.raw().iter().any(|entry| {
                    entry.name == "Team" && entry.value == NbtSummaryValue::string("blue")
                }));
                read_cow = true;
                let boat = record
                    .as_ref()
                    .unwrap()
                    .data
                    .entities()
                    .find(|record| record.uuid() == boat_uuid)
                    .unwrap();
                let child = boat
                    .passengers()
                    .find(|record| record.uuid() == child_uuid)
                    .unwrap();
                assert!(child
                    .raw()
                    .iter()
                    .any(|entry| entry.name == "Team"
                        && entry.value == NbtSummaryValue::string("blue")));
                read_child = true;
                let stand = record
                    .as_ref()
                    .unwrap()
                    .data
                    .entities()
                    .find(|record| record.uuid() == stand_uuid)
                    .unwrap();
                assert_eq!(stand.id(), "minecraft:armor_stand");
                assert!(stand
                    .raw()
                    .iter()
                    .any(|entry| entry.name == "Team"
                        && entry.value == NbtSummaryValue::string("blue")));
                read_stand = true;
                let boat = record
                    .as_ref()
                    .unwrap()
                    .data
                    .entities()
                    .find(|record| record.uuid() == stand_boat_uuid)
                    .unwrap();
                let child = boat
                    .passengers()
                    .find(|record| record.uuid() == stand_child_uuid)
                    .unwrap();
                assert_eq!(child.id(), "minecraft:armor_stand");
                assert!(child
                    .raw()
                    .iter()
                    .any(|entry| entry.name == "Team"
                        && entry.value == NbtSummaryValue::string("blue")));
                read_stand_child = true;
            }
            world
                .complete_runtime_entity_chunk(&level, chunk, record)
                .unwrap();
        }
        world.tick(Vec::new());
        if world
            .live_entity_id_for_uuid_for_test(dimension, cow_uuid)
            .is_some()
        {
            break;
        }
    }
    assert!(read_cow);
    assert!(read_child);
    assert!(read_stand);
    assert!(read_stand_child);
    let stand_boat = world
        .live_entity_id_for_uuid_for_test(dimension, stand_boat_uuid)
        .unwrap();
    let stand_child = world
        .live_entity_id_for_uuid_for_test(dimension, stand_child_uuid)
        .unwrap();
    assert_eq!(
        world.stored_passengers_for_test(dimension, stand_boat),
        &[stand_child]
    );
    assert_eq!(
        world
            .stored_entity_for_test(dimension, stand_boat)
            .unwrap()
            .uuid(),
        stand_boat_uuid
    );
    let body = world
        .stored_entity_for_test(dimension, stand_child)
        .unwrap();
    assert_eq!(body.uuid(), stand_child_uuid);
    assert_eq!(
        body.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
    );
    for (operation, condition, team) in [(23, "if", "blue"), (24, "unless", "red")] {
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            operation,
            &format!("execute {condition} entity @e[tag=runtime_stand_child,team={team}]"),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
    }
    let NbtSummaryValue::Compound(stand_child_record) = world
        .stored_entity_command_data_for_test(dimension, stand_child)
        .unwrap()
    else {
        panic!("reloaded stand child")
    };
    assert!(!stand_child_record.iter().any(|entry| entry.name == "Team"));
    for name in ["Silent", "NoGravity", "ShowArms"] {
        assert!(stand_child_record
            .iter()
            .any(|entry| entry.name == name && entry.value == NbtSummaryValue::Byte(1)));
    }
    assert!(
        matches!(field(&stand_child_record, "Tags"), NbtSummaryValue::List(tags)
        if tags.contains(&NbtSummaryValue::String("runtime_stand_child".into())))
    );
    let (joined, loaded) = join_loaded_ticks(&sender, &receiver, &mut world, 1, "watcher", 0xbeef);
    assert!(joined.events.iter().chain(&loaded.events).any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetPlayerTeam(team) }
        if team.name == "blue" && team.players.iter().any(|player| player == &stand_child_member)
    )));
    let stand = world
        .live_entity_id_for_uuid_for_test(dimension, stand_uuid)
        .unwrap();
    let body = world.stored_entity_for_test(dimension, stand).unwrap();
    assert_eq!(body.uuid(), stand_uuid);
    assert_eq!(
        body.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
    );
    for (operation, condition, team) in [(10, "if", "blue"), (11, "unless", "red")] {
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            operation,
            &format!("execute {condition} entity @e[tag=runtime_stand_team,team={team}]"),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
    }
    let NbtSummaryValue::Compound(stand_record) = world
        .stored_entity_command_data_for_test(dimension, stand)
        .unwrap()
    else {
        panic!("reloaded stand")
    };
    assert!(!stand_record.iter().any(|entry| entry.name == "Team"));
    for name in ["Silent", "NoGravity", "ShowArms"] {
        assert!(stand_record
            .iter()
            .any(|entry| entry.name == name && entry.value == NbtSummaryValue::Byte(1)));
    }
    assert!(
        matches!(field(&stand_record, "Tags"), NbtSummaryValue::List(tags)
        if tags.contains(&NbtSummaryValue::String("runtime_stand_team".into())))
    );
    let boat = world
        .live_entity_id_for_uuid_for_test(dimension, boat_uuid)
        .unwrap();
    let child = world
        .live_entity_id_for_uuid_for_test(dimension, child_uuid)
        .unwrap();
    assert_eq!(world.stored_passengers_for_test(dimension, boat), &[child]);
    assert_eq!(
        world
            .stored_entity_for_test(dimension, boat)
            .unwrap()
            .uuid(),
        boat_uuid
    );
    assert_eq!(
        world
            .stored_entity_for_test(dimension, child)
            .unwrap()
            .uuid(),
        child_uuid
    );
    assert_eq!(
        world
            .stored_entity_for_test(dimension, boat)
            .unwrap()
            .data()
            .entity_resource_id(),
        "minecraft:oak_chest_boat"
    );
    assert_eq!(
        world
            .stored_entity_for_test(dimension, child)
            .unwrap()
            .entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID
    );
    for (operation, condition, team) in [(7, "if", "blue"), (8, "unless", "red")] {
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            operation,
            &format!("execute {condition} entity @e[tag=runtime_team_child,team={team}]"),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
    }
    let NbtSummaryValue::Compound(child_record) = world
        .stored_entity_command_data_for_test(dimension, child)
        .unwrap()
    else {
        panic!("reloaded passenger")
    };
    assert!(!child_record.iter().any(|entry| entry.name == "Team"));
    assert!(child_record
        .iter()
        .any(|entry| entry.name == "Silent" && entry.value == NbtSummaryValue::Byte(1)));
    let cow = world
        .live_entity_id_for_uuid_for_test(dimension, cow_uuid)
        .unwrap();
    let entity = world.stored_entity_for_test(dimension, cow).unwrap();
    assert_eq!(
        entity.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID
    );
    assert_eq!(entity.uuid(), cow_uuid);
    assert_eq!(entity.position(), cow_position);
    for (operation, condition, team) in [(4, "if", "blue"), (5, "unless", "red")] {
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            operation,
            &format!("execute {condition} entity @e[tag=runtime_team_probe,team={team}]"),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
    }
    let NbtSummaryValue::Compound(record) = world
        .stored_entity_command_data_for_test(dimension, cow)
        .unwrap()
    else {
        panic!("reloaded cow")
    };
    assert!(!record.iter().any(|entry| entry.name == "Team"));
    assert!(record
        .iter()
        .any(|entry| { entry.name == "Silent" && entry.value == NbtSummaryValue::Byte(1) }));
    assert!(world.is_execute_chunk_loaded(&key, probe));
    let chunks = world.chunk_store_for_test(Java26BuiltinDimension::Overworld);
    assert!(matches!(
        chunks.generation_state(RELOADED.0, RELOADED.1),
        Some(bbb_server_types::chunk_data::ChunkGenerationState::NoiseFull(_))
    ));
    assert_eq!(
        chunks.get_block_state(marker.x, marker.y, marker.z),
        Some(glowstone)
    );
    assert!(chunks
        .generation_status(unreadable.0, unreadable.1)
        .is_some());
    let pending_save = world
        .save_request(1_786_000_000_100, 101, RegionCompression::Zlib)
        .unwrap();
    assert!(pending_save.scoreboard.teams.iter().any(|team| {
        team.name == "blue" && team.players.iter().any(|player| player == &member)
    }));
    assert!(!pending_save
        .scoreboard
        .teams
        .iter()
        .any(|team| { team.name == "red" && team.players.iter().any(|player| player == &member) }));
    assert!(pending_save.scoreboard.teams.iter().any(
        |team| team.name == "blue" && team.players.iter().any(|player| player == &stand_member)
    ));
    assert!(!pending_save.scoreboard.teams.iter().any(
        |team| team.name == "red" && team.players.iter().any(|player| player == &stand_member)
    ));
    let stand_record = saved_entities(&pending_save)
        .into_iter()
        .find(|fields| {
            fields
                .iter()
                .any(|entry| entry.name == "UUID" && entry.value == nbt_uuid(stand_uuid))
        })
        .unwrap();
    assert!(!stand_record.iter().any(|entry| entry.name == "Team"));
    for name in ["Silent", "NoGravity", "ShowArms"] {
        assert!(stand_record
            .iter()
            .any(|entry| entry.name == name && entry.value == NbtSummaryValue::Byte(1)));
    }
    assert!(
        matches!(field(&stand_record, "Tags"), NbtSummaryValue::List(tags)
        if tags.contains(&NbtSummaryValue::String("runtime_stand_team".into())))
    );
    let cow_record = saved_entities(&pending_save)
        .into_iter()
        .find(|fields| {
            fields
                .iter()
                .any(|entry| entry.name == "UUID" && entry.value == nbt_uuid(cow_uuid))
        })
        .unwrap();
    assert!(!cow_record.iter().any(|entry| entry.name == "Team"));
    assert!(pending_save.scoreboard.teams.iter().any(
        |team| team.name == "blue" && team.players.iter().any(|player| player == &child_member)
    ));
    assert!(!pending_save.scoreboard.teams.iter().any(
        |team| team.name == "red" && team.players.iter().any(|player| player == &child_member)
    ));
    let boat_record = saved_entities(&pending_save)
        .into_iter()
        .find(|fields| {
            fields
                .iter()
                .any(|entry| entry.name == "UUID" && entry.value == nbt_uuid(boat_uuid))
        })
        .unwrap();
    let NbtSummaryValue::List(passengers) = &boat_record
        .iter()
        .find(|entry| entry.name == "Passengers")
        .unwrap()
        .value
    else {
        panic!("resaved passengers")
    };
    let child_record = passengers
        .iter()
        .find_map(|value| match value {
            NbtSummaryValue::Compound(fields)
                if fields
                    .iter()
                    .any(|entry| entry.name == "UUID" && entry.value == nbt_uuid(child_uuid)) =>
            {
                Some(fields)
            }
            _ => None,
        })
        .unwrap();
    assert!(!child_record.iter().any(|entry| entry.name == "Team"));
    assert!(pending_save
        .scoreboard
        .teams
        .iter()
        .any(|team| team.name == "blue"
            && team
                .players
                .iter()
                .any(|player| player == &stand_child_member)));
    assert!(!pending_save
        .scoreboard
        .teams
        .iter()
        .any(|team| team.name == "red"
            && team
                .players
                .iter()
                .any(|player| player == &stand_child_member)));
    let boat_record = saved_entities(&pending_save)
        .into_iter()
        .find(|fields| {
            fields
                .iter()
                .any(|entry| entry.name == "UUID" && entry.value == nbt_uuid(stand_boat_uuid))
        })
        .unwrap();
    let NbtSummaryValue::List(passengers) = field(&boat_record, "Passengers") else {
        panic!("resaved stand passengers")
    };
    let child_record = passengers
        .iter()
        .find_map(|value| match value {
            NbtSummaryValue::Compound(fields)
                if fields.iter().any(|entry| {
                    entry.name == "UUID" && entry.value == nbt_uuid(stand_child_uuid)
                }) =>
            {
                Some(fields)
            }
            _ => None,
        })
        .unwrap();
    assert!(!child_record.iter().any(|entry| entry.name == "Team"));
    for name in ["Silent", "NoGravity", "ShowArms"] {
        assert!(child_record
            .iter()
            .any(|entry| entry.name == name && entry.value == NbtSummaryValue::Byte(1)));
    }
    assert!(
        matches!(field(child_record, "Tags"), NbtSummaryValue::List(tags)
        if tags.contains(&NbtSummaryValue::String("runtime_stand_child".into())))
    );
    assert!(
        pending_save.poi_edits.iter().any(|edit| {
            (edit.chunk_x, edit.chunk_z) == KEPT
                && edit
                    .sections
                    .iter()
                    .flat_map(|section| &section.records)
                    .any(|record| {
                        record.pos == [lectern.x, lectern.y, lectern.z]
                            && record.type_id == "minecraft:librarian"
                            && record.free_tickets == 0
                    })
        }),
        "restoring another column must preserve the unsaved workstation"
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
