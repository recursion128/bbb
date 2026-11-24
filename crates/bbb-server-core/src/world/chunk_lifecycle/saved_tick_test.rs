use crate::world::{SavedChunkTicket, ServerWorld, ServerWorldConfig};
use crate::world_creation::{created_save, default_overworld_level_source};
use bbb_protocol::packets::BlockPos;
use bbb_server_persistence::{
    Java26ChunkContext, Java26ChunkData, Java26WorldCandidate, Java26WorldCreation,
    Java26WorldStorage, RegionCompression,
};
use bbb_server_types::chunk_ticks::{BlockTickType, Java26SavedTick};
use bbb_server_types::save::{Java26BuiltinDimension, WorldSaveLimits, WorldSavePaths};

#[test]
fn saved_no_op_ticks_retire_after_restore_and_after_new_saves() {
    let root = std::env::temp_dir().join(format!("bbb-cold-saved-tick-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let storage = Java26WorldStorage::open(
        WorldSavePaths::new(&root).unwrap(),
        WorldSaveLimits::default(),
    )
    .unwrap();
    let candidate = create_column_with_saved_tick(&storage);
    let dimension = Java26BuiltinDimension::Overworld.into();
    let position = BlockPos { x: 8, y: -64, z: 8 };
    let stone =
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:stone").unwrap();
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(stone.id).unwrap();
    let mut world = crate::tests::restore_stored(ServerWorldConfig::default(), &candidate);
    assert_eq!(
        world
            .level_by_id(&dimension)
            .chunks
            .get_block_state(8, -64, 8),
        Some(state)
    );
    assert!(!world
        .level_by_id(&dimension)
        .chunks
        .tick_container::<BlockTickType>((0, 0))
        .unwrap()
        .is_empty());
    for _ in 0..2 {
        world
            .level_mut_by_id(&dimension)
            .chunk_lifecycle
            .purge_unknown();
    }
    world.refresh_loading_chunks(&dimension).unwrap();
    let unload = world
        .runtime_unload_save_request(2, RegionCompression::Zlib)
        .unwrap();
    storage.save_runtime_unloads(&candidate, &unload).unwrap();
    let saved = storage
        .read_runtime_column(&candidate, &world.level_key(dimension).clone(), 0, 0)
        .unwrap();
    assert_eq!(saved.chunk.as_ref().unwrap().data.block_ticks().len(), 1);
    world.retain_runtime_unload_columns(&world.level_key(dimension).clone(), vec![((0, 0), saved)]);
    world.acknowledge_save_success(&unload);
    assert!(!world.level_by_id(&dimension).chunks.is_loaded(0, 0));
    assert!(!world
        .level_by_id(&dimension)
        .saved_tick_chunks
        .contains(&(0, 0)));

    assert_eq!(world.block_state_by_key(&dimension, position), Some(state));
    assert!(world.level_by_id(&dimension).chunks.is_loaded(0, 0));
    assert!(!world
        .level_by_id(&dimension)
        .chunks
        .has_saved_column_input((0, 0)));
    assert!(!world
        .level_by_id(&dimension)
        .chunks
        .tick_container::<BlockTickType>((0, 0))
        .unwrap()
        .is_empty());
    world
        .level_mut_by_id(&dimension)
        .saved_chunk_tickets
        .push(SavedChunkTicket {
            chunk: (0, 0),
            level: 32,
            ticks_left: None,
        });
    world.refresh_simulation_chunk_levels_by_key(&dimension);
    world.refresh_loading_chunks(&dimension).unwrap();
    world
        .complete_runtime_entity_chunk(
            &world.level_key(dimension).clone(),
            (0, 0),
            storage
                .read_runtime_entity_chunk(&candidate, &world.level_key(dimension).clone(), 0, 0)
                .unwrap(),
        )
        .unwrap();
    assert!(world
        .block_ticking_chunks_by_key(&dimension)
        .contains(&(0, 0)));
    world.tick(Vec::new());
    assert!(world
        .level_by_id(&dimension)
        .chunks
        .tick_container::<BlockTickType>((0, 0))
        .unwrap()
        .is_empty());
    assert_eq!(
        world
            .level_by_id(&dimension)
            .chunks
            .get_block_state(8, -64, 8),
        Some(state)
    );

    // Finish the restored lighting batch so derived initialization cannot mask tick debt.
    let level = world.level_mut_by_id(&dimension);
    level.chunks.begin_ordinary_light_restore();
    level.chunks.finish_ordinary_light_restore().unwrap();
    assert!(level.chunks.save_dirty_epoch((0, 0)).is_none());
    assert!(!level
        .chunks
        .pending_initial_persistence()
        .any(|chunk| chunk == (0, 0)));
    assert!(!level
        .chunks
        .ordinary_derived_dirty_chunks()
        .any(|chunk| chunk == (0, 0)));
    assert!(!level
        .chunks
        .pending_whole_rewrites()
        .any(|intent| intent.chunk == (0, 0)));

    let request = world.save_request(3, 3, RegionCompression::Zlib).unwrap();
    let write = request
        .chunk_writes
        .iter()
        .find(|write| {
            write.dimension == world.level_key(dimension).clone()
                && (write.chunk_x, write.chunk_z) == (0, 0)
        })
        .expect("consuming a saved tick must leave a save debt even when its body does nothing");
    let parsed_write = Java26ChunkData::parse(
        &write.document,
        Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: -4,
            section_count: 24,
        },
    )
    .unwrap();
    assert!(parsed_write.block_ticks().is_empty());
    assert!(parsed_write.fluid_ticks().is_empty());
    assert!(parsed_write.block_entities().is_empty());
    storage.save_snapshot(&candidate, &request).unwrap();
    world.acknowledge_save_success(&request);
    let reparsed = storage
        .read_runtime_column(&candidate, &world.level_key(dimension).clone(), 0, 0)
        .unwrap();
    assert!(reparsed.chunk.unwrap().data.block_ticks().is_empty());

    world.test_schedule_block_tick(
        Java26BuiltinDimension::Overworld,
        position,
        state,
        world.game_time,
    );
    let pending = world.save_request(4, 4, RegionCompression::Zlib).unwrap();
    storage.save_snapshot(&candidate, &pending).unwrap();
    world.acknowledge_detached_save(&pending);
    world.tick(Vec::new());
    let consumed = world.save_request(5, 5, RegionCompression::Zlib).unwrap();
    let write = consumed
        .chunk_writes
        .iter()
        .find(|write| (write.chunk_x, write.chunk_z) == (0, 0))
        .expect("a newly saved tick must also be removed from disk after consumption");
    let parsed_consumed = Java26ChunkData::parse(
        &write.document,
        Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: -4,
            section_count: 24,
        },
    )
    .unwrap();
    assert!(parsed_consumed.block_ticks().is_empty());
    storage.save_snapshot(&candidate, &consumed).unwrap();
    world.acknowledge_save_success(&consumed);
    let reparsed = storage
        .read_runtime_column(&candidate, &world.level_key(dimension).clone(), 0, 0)
        .unwrap();
    assert!(reparsed.chunk.unwrap().data.block_ticks().is_empty());
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

fn create_column_with_saved_tick(storage: &Java26WorldStorage) -> Java26WorldCandidate {
    let fixture = bbb_server_types::fixture::FlatFixture::new().unwrap();
    let column = default_overworld_level_source()
        .unwrap()
        .generate_chunk(0, 0, fixture.biome_id())
        .into_column();
    let mut created = created_save(
        &Java26WorldCreation {
            level_name: "cold-saved-tick".into(),
            game_type: ServerWorld::fixture_game_type_id(),
            difficulty: "normal".into(),
            spawn_position: [0, 64, 0],
            last_played: 1,
        },
        &fixture,
        &[(0, 0, &column)],
    )
    .unwrap();
    bbb_server_persistence::write_chunk_ticks(
        &mut created.chunks[0].document,
        0,
        &[Java26SavedTick {
            id: "minecraft:stone".into(),
            x: 8,
            y: -64,
            z: 8,
            delay: 0,
            priority: 0,
        }],
        &[],
    );
    storage
        .create_world(&created, RegionCompression::Zlib, 1)
        .unwrap();
    storage.load_candidate().unwrap()
}
