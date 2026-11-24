use super::*;
use crate::chunk::noise_generator::overworld_program;
use crate::chunk::ChunkStore;
use crate::world::ServerWorldConfig;
use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_server_persistence::{Java26ChunkContext, Java26ChunkData};
use bbb_server_types::save::{Java26BuiltinDimension, Java26LevelKey};

#[test]
fn save_success_acknowledges_only_whole_writes_from_that_request() {
    let program = overworld_program(true, 0);
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let level = world.level_mut(Java26BuiltinDimension::Overworld);
    level.chunks =
        ChunkStore::new_overworld_carvers_with_structure_generation(program, false).unwrap();
    for chunk in [(-8, 8), (3, -5)] {
        level.chunks.try_load_column(chunk.0, chunk.1).unwrap();
    }
    let request = world
        .save_request(1_786_000_009_999, 300, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(
        request
            .chunk_writes
            .iter()
            .map(|write| (write.chunk_x, write.chunk_z))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([(-8, 8), (3, -5)])
    );
    assert!(world
        .level(Java26BuiltinDimension::Overworld)
        .saved_chunks
        .is_empty());

    let mut first_success = request.clone();
    first_success.chunk_writes.truncate(1);
    let acknowledged = (
        first_success.chunk_writes[0].chunk_x,
        first_success.chunk_writes[0].chunk_z,
    );
    world.acknowledge_save_success(&first_success);
    assert_eq!(
        world
            .level(Java26BuiltinDimension::Overworld)
            .chunks
            .pending_initial_persistence()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([if acknowledged == (-8, 8) {
            (3, -5)
        } else {
            (-8, 8)
        }])
    );

    world.acknowledge_save_success(&request);
    assert_eq!(
        world
            .level(Java26BuiltinDimension::Overworld)
            .chunks
            .pending_initial_persistence()
            .count(),
        0
    );
    assert!(world
        .save_request(1_786_000_010_000, 301, RegionCompression::Zlib)
        .unwrap()
        .chunk_writes
        .is_empty());
}

#[test]
fn a_detached_save_retires_unchanged_created_chunks_and_keeps_advanced_ones() {
    let program = overworld_program(true, 0);
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let level = world.level_mut(Java26BuiltinDimension::Overworld);
    level.chunks =
        ChunkStore::new_overworld_carvers_with_structure_generation(program, false).unwrap();
    level.chunks.try_load_column(3, -5).unwrap();
    let request = world
        .save_request(1_786_000_009_999, 300, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(request.chunk_writes.len(), 1);

    // An unchanged newly generated chunk retires its first-write intent once
    // the detached autosave finishes, so subsequent autosaves do not rewrite it.
    world.acknowledge_detached_save(&request);
    assert!(world
        .save_request(1_786_000_010_000, 301, RegionCompression::Zlib)
        .unwrap()
        .chunk_writes
        .is_empty());

    // A newly generated chunk that advances generation status while a detached
    // save is in flight stays pending for the next save.
    let level = world.level_mut(Java26BuiltinDimension::Overworld);
    level
        .chunks
        .ensure_loading_generation(&[((4, -5), bbb_server_persistence::Java26ChunkStatus::Biomes)])
        .unwrap();
    let in_flight = world
        .save_request(1_786_000_010_001, 302, RegionCompression::Zlib)
        .unwrap();
    assert!(in_flight
        .chunk_writes
        .iter()
        .any(|write| (write.chunk_x, write.chunk_z) == (4, -5)));
    world
        .level_mut(Java26BuiltinDimension::Overworld)
        .chunks
        .try_load_column(4, -5)
        .unwrap();
    world.acknowledge_detached_save(&in_flight);
    let follow_up = world
        .save_request(1_786_000_010_002, 303, RegionCompression::Zlib)
        .unwrap();
    assert!(follow_up
        .chunk_writes
        .iter()
        .any(|write| (write.chunk_x, write.chunk_z) == (4, -5)));
    world.acknowledge_detached_save(&follow_up);
    assert!(world
        .save_request(1_786_000_010_003, 304, RegionCompression::Zlib)
        .unwrap()
        .chunk_writes
        .is_empty());
}

#[test]
fn ordinary_neighbor_save_dirty_survives_light_publication_and_repeated_save() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let dimension = Java26BuiltinDimension::Overworld;
    let west = (0, 0);
    let east = (1, 0);
    let glowstone =
        vanilla_block_state_id_by_name_and_properties("minecraft:glowstone", &BTreeMap::new())
            .unwrap();
    let expected_light = {
        let level = world.level_mut(dimension);
        level.chunks.begin_ordinary_light_restore();
        for chunk in [west, east] {
            level.chunks.try_load_column(chunk.0, chunk.1).unwrap();
        }
        level.chunks.finish_ordinary_light_restore().unwrap();
        level.saved_chunks.extend([west, east]);
        assert!(level.chunks.take_live_light_updates().unwrap().is_empty());
        assert_eq!(level.chunks.block_light_level(16, 200, 8), 0);

        level
            .chunks
            .set_live_block_state(15, 200, 8, glowstone)
            .unwrap();
        level.chunks.flush_live_relights().unwrap();
        assert!(level.chunks.block_light_level(16, 200, 8) > 0);
        let updates = level.chunks.take_live_light_updates().unwrap();
        assert!(updates
            .iter()
            .any(|update| (update.chunk_x, update.chunk_z) == east));
        assert!(level.chunks.take_live_light_updates().unwrap().is_empty());
        level.chunks.saved_light_sections(east)
    };
    assert!(!expected_light.is_empty());

    for timestamp in [1, 2] {
        let request = world
            .save_request(timestamp, timestamp as u32, RegionCompression::Zlib)
            .unwrap();
        let east_write = request
            .chunk_writes
            .iter()
            .find(|write| {
                write.dimension == Java26LevelKey::from(dimension)
                    && (write.chunk_x, write.chunk_z) == east
            })
            .expect("the light-only neighbor requires a whole-column write");
        let parsed = Java26ChunkData::parse(
            &east_write.document,
            Java26ChunkContext {
                expected_x: east.0,
                expected_z: east.1,
                min_section_y: -4,
                section_count: 24,
            },
        )
        .unwrap();
        assert!(parsed.is_light_on());
        assert!(parsed
            .sections()
            .iter()
            .any(|section| section.block_light.is_some()));
    }
}

#[test]
fn single_path_chunk_write_preserves_legacy_fields_and_unmodeled_block_entities() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let dimension = Java26BuiltinDimension::Overworld;
    let furnace_state = vanilla_block_state_id_by_name_and_properties(
        "minecraft:furnace",
        &BTreeMap::from([
            ("facing".to_string(), "north".to_string()),
            ("lit".to_string(), "false".to_string()),
        ]),
    )
    .unwrap();
    let stone_state = world.cold.fixture.stone_state_id();
    let air_state = world.cold.fixture.air_state_id();
    let entry = |name: &str, value: NbtSummaryValue| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let unmodeled_be = vec![
        entry(
            "id",
            NbtSummaryValue::string("minecraft:custom_unmodeled_be"),
        ),
        entry("x", NbtSummaryValue::Int(2)),
        entry("y", NbtSummaryValue::Int(-64)),
        entry("z", NbtSummaryValue::Int(3)),
        entry("CustomPayload", NbtSummaryValue::Int(777)),
    ];
    {
        let level = world.level_mut(dimension);
        level.chunks.begin_ordinary_light_restore();
        level.chunks.try_load_column(0, 0).unwrap();
        level.chunks.finish_ordinary_light_restore().unwrap();
        level.saved_chunks.insert((0, 0));
        level
            .chunks
            .set_live_block_state(2, -64, 3, furnace_state)
            .unwrap();
        level.chunks.flush_live_relights().unwrap();
    }
    let initial = world.save_request(1, 1, RegionCompression::Zlib).unwrap();
    let mut raw = initial
        .chunk_writes
        .iter()
        .find(|w| (w.chunk_x, w.chunk_z) == (0, 0))
        .unwrap()
        .document
        .clone();
    bbb_server_persistence::replace_block_entities(&mut raw, vec![unmodeled_be.clone()]);
    raw.entries_mut().push(entry(
        "UpgradeData",
        NbtSummaryValue::Compound(vec![
            entry("Sides", NbtSummaryValue::Byte(0x15)),
            entry(
                "Indices",
                NbtSummaryValue::Compound(vec![entry("3", NbtSummaryValue::IntArray(vec![7, 42]))]),
            ),
        ]),
    ));
    let expected_data = Java26ChunkData::parse(
        raw.clone(),
        Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: -4,
            section_count: 24,
        },
    )
    .unwrap();
    assert!(expected_data.upgrade_data().is_some());
    let loaded_chunk = bbb_server_persistence::Java26LoadedChunk {
        timestamp: 1,
        data: expected_data.clone(),
    };
    let mut saved_dim = bbb_server_persistence::Java26LoadedDimension::new(
        Java26BuiltinDimension::Overworld.into(),
    );
    saved_dim.chunks.insert((0, 0), loaded_chunk);
    let registries = world.registries.clone();
    let resolver = world.item_stack_resolver();
    {
        let level = world.level_mut(dimension);
        level.chunks = ChunkStore::new(
            crate::world_creation::default_overworld_level_source().unwrap(),
            0,
        );
        level
            .chunks
            .restore_saved_columns(Some(&saved_dim), &registries, &resolver, true)
            .unwrap();
        level.saved_chunks.insert((0, 0));
        assert_eq!(
            level
                .chunks
                .set_live_block_state(5, -64, 5, air_state)
                .unwrap(),
            Some(stone_state)
        );
        level.chunks.flush_live_relights().unwrap();
    }
    let resaved = world.save_request(2, 2, RegionCompression::Zlib).unwrap();
    let out = resaved
        .chunk_writes
        .iter()
        .find(|w| (w.chunk_x, w.chunk_z) == (0, 0))
        .expect("restored chunk should have a write");
    let reparsed = Java26ChunkData::parse(
        &out.document,
        Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: -4,
            section_count: 24,
        },
    )
    .unwrap();
    assert_eq!(reparsed.upgrade_data(), expected_data.upgrade_data());
    assert_eq!(reparsed.block_entities(), &[unmodeled_be]);
}

#[test]
fn detached_unload_batch_unloads_clean_columns_and_skips_mutated_or_in_flight() {
    let overworld = Java26BuiltinDimension::Overworld;
    let dim_key: Java26LevelKey = overworld.into();
    let root = std::env::temp_dir().join(format!(
        "bbb-detached-unload-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let storage = bbb_server_persistence::Java26WorldStorage::open(
        bbb_server_types::save::WorldSavePaths::new(&root).unwrap(),
        bbb_server_types::save::WorldSaveLimits::default(),
    )
    .unwrap();
    let fixture = bbb_server_types::fixture::FlatFixture::new().unwrap();
    let source = crate::world_creation::default_overworld_level_source().unwrap();
    let c0 = source
        .generate_chunk(0, 0, fixture.biome_id())
        .into_column();
    let c1 = source
        .generate_chunk(1, 0, fixture.biome_id())
        .into_column();
    let created = crate::world_creation::created_save(
        &bbb_server_persistence::Java26WorldCreation {
            level_name: "detached-unload".into(),
            game_type: 0,
            difficulty: "normal".into(),
            spawn_position: [0, -60, 0],
            last_played: 1,
        },
        &fixture,
        &[(0, 0, &c0), (1, 0, &c1)],
    )
    .unwrap();
    storage
        .create_world(&created, RegionCompression::Zlib, 1)
        .unwrap();
    let candidate = storage.load_candidate().unwrap();
    let mut world = crate::tests::restore_stored(ServerWorldConfig::default(), &candidate);
    let empty_props = BTreeMap::new();
    let stone_state =
        vanilla_block_state_id_by_name_and_properties("minecraft:stone", &empty_props).unwrap();
    let dirt_state =
        vanilla_block_state_id_by_name_and_properties("minecraft:dirt", &empty_props).unwrap();
    {
        let level = world.level_mut(overworld);
        level
            .chunks
            .set_live_block_state(1, -60, 1, stone_state)
            .unwrap();
        level
            .chunks
            .set_live_block_state(17, -60, 1, stone_state)
            .unwrap();
        level.chunks.flush_live_relights().unwrap();
        level.chunk_lifecycle.column_unloads.insert((0, 0));
        level.chunk_lifecycle.column_unloads.insert((1, 0));
    }

    // (0, 0) has an in-flight read so only (1, 0) is frozen for batch 1.
    let in_flight = BTreeSet::from([(dim_key.clone(), (0, 0))]);
    let (request_1, batch_1) = world
        .runtime_unload_save_request_batch(2, RegionCompression::Zlib, 4, &in_flight)
        .unwrap()
        .expect("eligible unload column");
    assert_eq!(
        batch_1.columns.iter().map(|c| c.chunk).collect::<Vec<_>>(),
        vec![(1, 0)]
    );
    assert!(world
        .level(overworld)
        .chunk_lifecycle
        .column_unloads
        .contains(&(0, 0)));

    // Mutate (1, 0) while its unload write runs off-thread.
    {
        let level = world.level_mut(overworld);
        level
            .chunks
            .set_live_block_state(18, -60, 2, dirt_state)
            .unwrap();
        level.chunks.flush_live_relights().unwrap();
    }
    world.acknowledge_detached_unload(&request_1, &batch_1, &BTreeSet::new());
    assert!(world.level(overworld).chunks.is_loaded(1, 0));
    assert!(world
        .level(overworld)
        .chunks
        .save_dirty_epoch((1, 0))
        .is_some());
    assert!(world
        .level(overworld)
        .chunk_lifecycle
        .column_unloads
        .contains(&(1, 0)));

    // If a read for (0, 0) begins mid-flight, ack still refuses to unload (0, 0).
    let (request_2, batch_2) = world
        .runtime_unload_save_request_batch(3, RegionCompression::Zlib, 1, &BTreeSet::new())
        .unwrap()
        .expect("batch limited to 1");
    assert_eq!(batch_2.columns.len(), 1);
    assert_eq!(batch_2.columns[0].chunk, (0, 0));
    assert!(world
        .level(overworld)
        .chunk_lifecycle
        .column_unloads
        .contains(&(1, 0)));

    world.acknowledge_detached_unload(
        &request_2,
        &batch_2,
        &BTreeSet::from([(dim_key.clone(), (0, 0))]),
    );
    assert!(world.level(overworld).chunks.is_loaded(0, 0));
    assert!(world
        .level(overworld)
        .chunk_lifecycle
        .column_unloads
        .contains(&(0, 0)));

    // With no read in flight and no further edits, (0, 0) unloads cleanly.
    world.acknowledge_detached_unload(&request_2, &batch_2, &BTreeSet::new());
    assert!(!world.level(overworld).chunks.is_loaded(0, 0));
    assert!(!world
        .level(overworld)
        .chunk_lifecycle
        .column_unloads
        .contains(&(0, 0)));
    assert!(world.level(overworld).chunks.is_loaded(1, 0));
    let _ = storage.close();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn amortized_autosave_snapshots_bounded_columns_per_tick_and_produces_full_request() {
    let overworld = Java26BuiltinDimension::Overworld;
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let diamond_state =
        vanilla_block_state_id_by_name_and_properties("minecraft:diamond_block", &BTreeMap::new())
            .unwrap();
    let dirty_chunks = [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)];
    {
        let level = world.level_mut(overworld);
        level.chunks.begin_ordinary_light_restore();
        for &(cx, cz) in &dirty_chunks {
            level.chunks.try_load_column(cx, cz).unwrap();
        }
        level.chunks.finish_ordinary_light_restore().unwrap();
        for &(cx, cz) in &dirty_chunks {
            level.saved_chunks.insert((cx, cz));
            assert_ne!(
                level
                    .chunks
                    .set_live_block_state(cx * 16 + 1, 200, cz * 16 + 1, diamond_state)
                    .unwrap(),
                Some(diamond_state)
            );
        }
    }

    world.begin_autosave_snapshot().unwrap();
    assert!(world.has_pending_autosave_snapshot());

    // Step 1: budget 2 snapshots 2 columns, leaving 3 pending.
    assert!(world
        .step_autosave_snapshot(100, 10, RegionCompression::Zlib, 2, true)
        .unwrap()
        .is_none());
    assert!(world.has_pending_autosave_snapshot());

    // Step 2: budget 2 snapshots 2 more columns, leaving 1 pending.
    assert!(world
        .step_autosave_snapshot(101, 11, RegionCompression::Zlib, 2, true)
        .unwrap()
        .is_none());
    assert!(world.has_pending_autosave_snapshot());

    // Step 3: finalize = false snaps the last column without retiring staging yet.
    assert!(world
        .step_autosave_snapshot(102, 12, RegionCompression::Zlib, 2, false)
        .unwrap()
        .is_none());
    assert!(world.has_pending_autosave_snapshot());

    // Step 4: finalize = true returns the complete request covering all 5 chunks.
    let request = world
        .step_autosave_snapshot(103, 13, RegionCompression::Zlib, 2, true)
        .unwrap()
        .expect("final autosave step produces complete request");
    assert!(!world.has_pending_autosave_snapshot());
    let written: BTreeSet<_> = request
        .chunk_writes
        .iter()
        .map(|w| (w.chunk_x, w.chunk_z))
        .collect();
    assert_eq!(written, BTreeSet::from(dirty_chunks));

    world.acknowledge_detached_save(&request);
    for &(cx, cz) in &dirty_chunks {
        assert!(world
            .level(overworld)
            .chunks
            .save_dirty_epoch((cx, cz))
            .is_none());
    }
}

#[test]
fn detached_entity_unload_uses_dirty_epoch_to_unload_unchanged_and_keep_mutated_chunks() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MARKER_ID;
    use bbb_protocol::packets::Vec3d;

    let overworld = Java26BuiltinDimension::Overworld;
    let root = std::env::temp_dir().join(format!(
        "bbb-detached-entity-epoch-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let storage = bbb_server_persistence::Java26WorldStorage::open(
        bbb_server_types::save::WorldSavePaths::new(&root).unwrap(),
        bbb_server_types::save::WorldSaveLimits::default(),
    )
    .unwrap();
    let fixture = bbb_server_types::fixture::FlatFixture::new().unwrap();
    let source = crate::world_creation::default_overworld_level_source().unwrap();
    let c0 = source
        .generate_chunk(0, 0, fixture.biome_id())
        .into_column();
    let c1 = source
        .generate_chunk(1, 0, fixture.biome_id())
        .into_column();
    let created = crate::world_creation::created_save(
        &bbb_server_persistence::Java26WorldCreation {
            level_name: "detached-entity-epoch".into(),
            game_type: 0,
            difficulty: "normal".into(),
            spawn_position: [0, -60, 0],
            last_played: 1,
        },
        &fixture,
        &[(0, 0, &c0), (1, 0, &c1)],
    )
    .unwrap();
    storage
        .create_world(&created, RegionCompression::Zlib, 1)
        .unwrap();
    let candidate = storage.load_candidate().unwrap();
    let mut world = crate::tests::restore_stored(ServerWorldConfig::default(), &candidate);

    let (e0, e1, epoch_0, epoch_1) = {
        let level = world.level_mut(overworld);
        let e0 = level.entities.spawn(
            VANILLA_ENTITY_TYPE_MARKER_ID,
            Vec3d {
                x: 4.5,
                y: -60.0,
                z: 4.5,
            },
            crate::entity::EntityData::Marker(crate::entity::marker::MarkerData::default()),
        );
        let e1 = level.entities.spawn(
            VANILLA_ENTITY_TYPE_MARKER_ID,
            Vec3d {
                x: 20.5,
                y: -60.0,
                z: 4.5,
            },
            crate::entity::EntityData::Marker(crate::entity::marker::MarkerData::default()),
        );
        level
            .chunk_lifecycle
            .entity_loads
            .insert((0, 0), EntityChunkLoadStatus::Loaded);
        level
            .chunk_lifecycle
            .entity_loads
            .insert((1, 0), EntityChunkLoadStatus::Loaded);
        level.chunk_lifecycle.entity_unloads.insert((0, 0));
        level.chunk_lifecycle.entity_unloads.insert((1, 0));
        (
            e0,
            e1,
            level.entities.chunk_dirty_epoch((0, 0)),
            level.entities.chunk_dirty_epoch((1, 0)),
        )
    };
    assert!(epoch_0 > 0);
    assert!(epoch_1 > 0);

    let (request, batch) = world
        .runtime_unload_save_request_batch(2, RegionCompression::Zlib, 4, &BTreeSet::new())
        .unwrap()
        .expect("both entity chunks are eligible for unload");
    assert_eq!(batch.entities.len(), 2);

    // Mutate only the entity in (1, 0) while the detached save is in flight.
    {
        let level = world.level_mut(overworld);
        let entity = level.entities.get_mut(e1).expect("e1 is still loaded");
        entity.set_custom_name(Some(bbb_protocol::RichTextComponent::plain("mutated")));
        assert_eq!(level.entities.chunk_dirty_epoch((0, 0)), epoch_0);
        assert!(level.entities.chunk_dirty_epoch((1, 0)) > epoch_1);
    }

    world.acknowledge_detached_unload(&request, &batch, &BTreeSet::new());
    {
        let level = world.level(overworld);
        assert!(level.entities.get(e0).is_none());
        assert!(!level.entities.is_chunk_occupied((0, 0)));
        assert!(!level.chunk_lifecycle.entity_unloads.contains(&(0, 0)));
        assert!(level.chunk_lifecycle.stored_entities.contains(&(0, 0)));

        assert!(level.entities.get(e1).is_some());
        assert!(level.entities.is_chunk_occupied((1, 0)));
        assert!(level.chunk_lifecycle.entity_unloads.contains(&(1, 0)));
    }

    // Without further mutation, the follow-up unload retires (1, 0) cleanly.
    let (request_2, batch_2) = world
        .runtime_unload_save_request_batch(3, RegionCompression::Zlib, 4, &BTreeSet::new())
        .unwrap()
        .expect("mutated chunk retries unload");
    world.acknowledge_detached_unload(&request_2, &batch_2, &BTreeSet::new());
    assert!(world.level(overworld).entities.get(e1).is_none());
    assert!(!world
        .level(overworld)
        .chunk_lifecycle
        .entity_unloads
        .contains(&(1, 0)));

    let _ = storage.close();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn deferred_chunk_save_preserves_registered_custom_mod_block() {
    let mut builder = crate::plugin::ServerPluginRegistryBuilder::new();
    builder.register_block(bbb_protocol::content_registry::ModBlockRegistration {
        resource_id: "modded:crystal_block".to_string(),
        state_count: 1,
        light_emission: 12,
        destroy_speed: 1.5,
        full_cube_collision: true,
        dynamic_light_rgb: None,
    });
    let plugins = Arc::new(builder.freeze().unwrap());
    let content_registry = Arc::clone(plugins.content_registry());
    let custom_state = content_registry
        .block_by_resource_id("modded:crystal_block")
        .unwrap()
        .first_state_id;

    let mut world = ServerWorld::new(ServerWorldConfig {
        plugins,
        ..ServerWorldConfig::default()
    })
    .unwrap();
    let overworld = Java26BuiltinDimension::Overworld;
    {
        let level = world.level_mut(overworld);
        level.chunks.try_load_column(0, 0).unwrap();
        level.saved_chunks.insert((0, 0));
        level
            .chunks
            .set_live_block_state(2, 200, 3, custom_state)
            .unwrap();
    }

    let (_, deferred) = world
        .save_request_deferred(1, 1, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(
        world.level(overworld).chunks.block_light_level(2, 200, 3),
        12
    );
    assert_eq!(
        world.level(overworld).chunks.block_light_level(3, 200, 3),
        11
    );
    assert_eq!(world.level(overworld).chunks.sky_light_level(2, 200, 3), 0);
    let chunks = materialize_deferred_chunks(deferred).unwrap();
    let write = chunks
        .iter()
        .find(|w| (w.chunk_x, w.chunk_z) == (0, 0))
        .expect("custom block chunk writes cleanly");
    let ctx = Java26ChunkContext {
        expected_x: 0,
        expected_z: 0,
        min_section_y: -4,
        section_count: 24,
    };
    let parsed =
        Java26ChunkData::parse_with_content_registry(&write.document, ctx, &content_registry)
            .unwrap();
    let section_y = (200_i32 >> 4) as i8;
    let section = parsed
        .sections()
        .iter()
        .find(|s| s.y == section_y)
        .expect("section containing custom block exists");
    let block_states = section.block_states.as_ref().expect("section block states");
    let entry_index = (((200_i32 & 15) as usize) << 8) | (3_usize << 4) | 2_usize;
    let palette_index = block_states
        .palette_index(entry_index)
        .expect("custom block palette index");
    assert_eq!(
        block_states.palette[palette_index].name,
        "modded:crystal_block"
    );
}
