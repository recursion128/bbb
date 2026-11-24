use super::*;
use crate::block_entity::new_block_entity;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::{VANILLA_BLOCK_SCULK_SENSOR_ID, VANILLA_BLOCK_STONE_ID};
use bbb_protocol::packets::BlockPos;
use bbb_server_persistence::{
    generation_state_chunk_document, Java26ChunkContext, Java26ChunkData,
};
use bbb_server_types::chunk_data::{ChunkGenerationState, VANILLA_26_1_BIOME_REGISTRY_LEN};

#[test]
fn loading_halo_has_exact_typed_stages_and_no_full_columns() {
    let program = crate::chunk::noise_generator::overworld_program(true, 0);
    let mut store =
        ChunkStore::new_overworld_carvers_with_structure_generation(program, true).unwrap();
    let mut targets = Vec::new();
    for x in -11_i32..=11 {
        for z in -11_i32..=11 {
            let status = match x.abs().max(z.abs()) {
                0..=1 => Java26ChunkStatus::InitializeLight,
                2 => Java26ChunkStatus::Carvers,
                3 => Java26ChunkStatus::Biomes,
                _ => Java26ChunkStatus::StructureStarts,
            };
            targets.push(((x, z), status));
        }
    }
    store.ensure_loading_generation(&targets).unwrap();
    for &(chunk, target) in &targets {
        assert_eq!(
            store.generation_status(chunk.0, chunk.1),
            Some(target),
            "{chunk:?}"
        );
    }
    assert!(store.chunks.values().all(|chunk| !matches!(
        chunk.generation,
        Some(ChunkGenerationState::NoiseFull(_) | ChunkGenerationState::Full { .. })
    )));

    store.ensure_loading_generation(&targets).unwrap();
}

#[test]
fn loading_postprocessing_clear_preserves_light_witness_and_writer() {
    let mut store = ChunkStore::new(
        crate::world_creation::default_overworld_level_source().unwrap(),
        0,
    );
    store.try_ensure_served_full_column(0, 0).unwrap();

    store
        .ensure_loading_generation(&[((0, 0), Java26ChunkStatus::InitializeLight)])
        .unwrap();

    let positions = vec![(1, 64, 2)];
    if let ChunkGenerationState::Full {
        postprocessing_positions,
        ..
    } = store.generation_mut((0, 0)).unwrap()
    {
        *postprocessing_positions = positions.clone();
    }

    let light = store.saved_light_sections((0, 0));
    assert_eq!(store.postprocessing_positions(0, 0), positions);
    store.clear_postprocessing_positions(0, 0);
    assert_eq!(store.saved_light_sections((0, 0)), light);
    let document = generation_state_chunk_document(
        &store.loaded((0, 0)).column,
        0,
        0,
        &|_| Some("minecraft:plains".to_string()),
        store.state((0, 0)),
        None,
        &[],
    )
    .unwrap();
    assert_eq!(store.pending_whole_rewrites().count(), 1);
    let intent = store.pending_whole_rewrites().next().unwrap();
    assert!(store.acknowledge_whole_rewrite(intent));
    store.clear_postprocessing_positions(0, 0);
    assert_eq!(store.pending_whole_rewrites().count(), 0);
    let parsed = Java26ChunkData::parse(
        document,
        Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: store.min_y().div_euclid(16),
            section_count: store.section_count(),
        },
    )
    .unwrap();
    assert!(parsed.post_processing().iter().all(Vec::is_empty));
}

#[test]
fn loading_unload_retires_entities_without_tombstones_and_retains_cold_reads() {
    let mut store = ChunkStore::new(
        crate::world_creation::default_overworld_level_source().unwrap(),
        0,
    );
    store.try_ensure_served_full_column(0, 0).unwrap();
    let pos = BlockPos { x: 1, y: 64, z: 2 };
    let sensor = vanilla_block_default_state_id(VANILLA_BLOCK_SCULK_SENSOR_ID).unwrap();
    let stone = vanilla_block_default_state_id(VANILLA_BLOCK_STONE_ID).unwrap();
    let entity = new_block_entity(pos, sensor).unwrap();
    store.block_entities_mut().add_and_register(entity);
    store.block_entities_mut().set_changed(pos);
    store
        .column_mut(0, 0)
        .unwrap()
        .set_block_state(1, 64, 2, stone);
    store.cache_path_type(pos, 17);
    store.queue_navigation_update(pos);
    let saved = store.unload_saved_column(0, 0).unwrap();
    assert_eq!(saved.get_block_state(1, 64, 2), Some(stone));
    assert!(store.block_entities().get(pos).is_none());
    assert!(store.save_dirty_epoch((0, 0)).is_none());
    assert!(store.block_entities_mut().drain_tickers().is_empty());
    assert_eq!(
        store
            .block_entities()
            .game_event_listeners()
            .listeners()
            .count(),
        0
    );
    assert_eq!(store.cached_path_type(pos), None);
    assert!(store.drain_navigation_updates().is_empty());
    assert_eq!(store.generation_status(0, 0), None);
    assert!(store.saved_light_sections((0, 0)).is_empty());
    assert!(!store.is_loaded(0, 0));
    assert!(store.column(0, 0).is_none());
    assert_eq!(store.get_block_state(1, 64, 2), Some(stone));
    assert_eq!(store.take_cold_column_reads(), BTreeSet::from([(0, 0)]));
    assert!(store.take_cold_column_reads().is_empty());
}

#[test]
fn loading_debug_defers_registry_grid_to_features_and_flat_halo_stays_proto() {
    let mut debug = ChunkStore::new_debug_with_biome_registry_and_skylight(
        DebugLevelSource::new(-64, 384).unwrap(),
        0,
        VANILLA_26_1_BIOME_REGISTRY_LEN,
        true,
    );
    debug
        .ensure_loading_generation(&[
            ((0, 0), Java26ChunkStatus::Carvers),
            ((1, 0), Java26ChunkStatus::Biomes),
            ((2, 0), Java26ChunkStatus::StructureStarts),
        ])
        .unwrap();
    assert_eq!(
        debug.generation_status(0, 0),
        Some(Java26ChunkStatus::Carvers)
    );
    assert_eq!(
        debug.loaded((0, 0)).column.get_block_state(3, 70, 1),
        Some(debug.air_state_id())
    );
    debug
        .ensure_loading_generation(&[((0, 0), Java26ChunkStatus::InitializeLight)])
        .unwrap();
    assert_eq!(
        debug.generation_status(0, 0),
        Some(Java26ChunkStatus::InitializeLight)
    );
    assert_eq!(
        debug.loaded((0, 0)).column.get_block_state(3, 70, 1),
        Some(173)
    );
    debug.try_ensure_served_full_column(0, 0).unwrap();
    assert_eq!(debug.generation_status(0, 0), Some(Java26ChunkStatus::Full));
    assert_eq!(
        debug.loaded((0, 0)).column.get_block_state(3, 70, 1),
        Some(173)
    );

    let mut flat = ChunkStore::new(
        crate::world_creation::default_overworld_level_source().unwrap(),
        0,
    );
    flat.ensure_loading_generation(&[
        ((0, 0), Java26ChunkStatus::InitializeLight),
        ((1, 0), Java26ChunkStatus::Carvers),
        ((2, 0), Java26ChunkStatus::Biomes),
        ((3, 0), Java26ChunkStatus::StructureStarts),
    ])
    .unwrap();
    assert_eq!(
        flat.generation_status(0, 0),
        Some(Java26ChunkStatus::InitializeLight)
    );
    assert_eq!(
        flat.generation_status(1, 0),
        Some(Java26ChunkStatus::Carvers)
    );
    assert_eq!(
        flat.generation_status(2, 0),
        Some(Java26ChunkStatus::Biomes)
    );
    assert_eq!(
        flat.generation_status(3, 0),
        Some(Java26ChunkStatus::StructureStarts)
    );
    flat.try_ensure_served_full_column(0, 0).unwrap();
    assert_eq!(
        flat.generation_status(1, 0),
        Some(Java26ChunkStatus::Carvers)
    );
}

#[test]
fn loading_next_to_saved_full_relights_without_regenerating_the_full_body() {
    let program = crate::chunk::noise_generator::overworld_program(true, 0);
    let mut original =
        ChunkStore::new_overworld_carvers_with_structure_generation(program.clone(), false)
            .unwrap();
    original.try_ensure_noise_full_column(0, 0).unwrap();
    let column = original.loaded((0, 0)).column.clone();
    let state = original.state((0, 0)).clone();
    let document = bbb_server_persistence::generation_state_chunk_document_with_light(
        &column,
        0,
        0,
        &|id| program.biome_name(id),
        &state,
        original
            .structure_component(0, 0)
            .map(|s| s.to_nbt())
            .as_ref(),
        &[],
        &original.saved_light_sections((0, 0)),
    )
    .unwrap();
    let data = Java26ChunkData::parse(
        document,
        Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: original.min_y().div_euclid(16),
            section_count: original.section_count(),
        },
    )
    .unwrap();
    let mut store =
        ChunkStore::new_overworld_carvers_with_structure_generation(program, false).unwrap();
    store.insert_restored_column(0, 0, column.clone(), state);
    store.restore_saved_generation_metadata(&data).unwrap();
    store
        .finish_restored_generation_batch(&BTreeSet::from([(0, 0)]))
        .unwrap();
    let entities = store
        .block_entities()
        .chunk((0, 0))
        .cloned()
        .collect::<Vec<_>>();
    let population_flags = match store.generation((0, 0)).unwrap() {
        ChunkGenerationState::NoiseFull(full) => (
            full.spawn().disable_mob_generation(),
            full.spawn().original_mobs_refused(),
        ),
        _ => unreachable!(),
    };
    let targets = (-1..=1)
        .flat_map(|x| (-1..=1).map(move |z| ((x, z), Java26ChunkStatus::InitializeLight)))
        .collect::<Vec<_>>();
    store.ensure_loading_generation(&targets).unwrap();
    assert_eq!(store.generation_status(0, 0), Some(Java26ChunkStatus::Full));
    assert_eq!(
        store.generation_status(1, 0),
        Some(Java26ChunkStatus::InitializeLight)
    );
    assert_eq!(store.loaded((0, 0)).column, column);
    assert_eq!(
        store
            .block_entities()
            .chunk((0, 0))
            .cloned()
            .collect::<Vec<_>>(),
        entities
    );
    assert!(store.proto_entity_records(0, 0).is_empty());
    assert!(store.chunks.values().all(|chunk| !chunk.relighting_full));
    store.validate_served_full_column(0, 0).unwrap();
    let ChunkGenerationState::NoiseFull(full) = store.generation((0, 0)).unwrap() else {
        unreachable!()
    };
    assert_eq!(
        (
            full.spawn().disable_mob_generation(),
            full.spawn().original_mobs_refused()
        ),
        population_flags
    );
}

#[test]
fn loading_cold_columns_reject_generation_and_writes_until_saved_publication() {
    let mut store = ChunkStore::new(
        crate::world_creation::default_overworld_level_source().unwrap(),
        0,
    );
    store.try_ensure_served_full_column(0, 0).unwrap();
    let stone = vanilla_block_default_state_id(VANILLA_BLOCK_STONE_ID).unwrap();
    store.set_live_block_state(1, 200, 2, stone).unwrap();
    store.mark_persisted(0, 0);
    let state = store.generation_state(0, 0).unwrap().clone();
    let saved = store.unload_saved_column(0, 0).unwrap();

    let error = store.try_load_column(0, 0).unwrap_err().to_string();
    assert!(
        error.contains("waiting for saved-column hydration"),
        "{error}"
    );
    assert!(store.try_load_column_unlit(0, 0).is_err());
    assert!(store.publish_structure_starts_column([0, 0]).is_err());
    assert!(store.try_ensure_served_full_column(0, 0).is_err());
    assert!(store
        .ensure_loading_generation(&[
            ((1, 0), Java26ChunkStatus::InitializeLight),
            ((0, 0), Java26ChunkStatus::StructureStarts),
        ])
        .is_err());
    assert!(store
        .set_live_block_state(1, 200, 2, store.air_state_id())
        .is_err());
    assert!(store
        .set_block_state(1, 200, 2, store.air_state_id())
        .is_none());
    assert_ne!(store.generated_block_state(1, 200, 2), Some(stone));
    assert_eq!(store.get_block_state(1, 200, 2), Some(stone));

    let mut restored = store;
    restored.hydrate_saved_generation_column(0, 0, Java26ChunkStatus::Full);
    assert!(restored.require_generation_admission((0, 0)).is_ok());
    restored.insert_restored_column(0, 0, saved, state);
    restored.mark_persisted(0, 0);
    assert!(restored.cold_column(0, 0).is_none());
    assert!(restored.take_cold_column_reads().is_empty());
    assert!(restored.is_ticking_full(0, 0));
    assert_eq!(restored.get_block_state(1, 200, 2), Some(stone));
    assert!(!restored.chunk_flag((0, 0), |chunk| chunk.pending_initial_persistence));
    assert_eq!(
        restored.generation_status(0, 0),
        Some(Java26ChunkStatus::Full)
    );
}

#[test]
fn loading_cold_structure_sources_block_neighbor_metadata_regeneration() {
    let program = crate::chunk::noise_generator::overworld_program(true, 0);
    let mut store =
        ChunkStore::new_overworld_carvers_with_structure_generation(program, true).unwrap();
    store
        .ensure_loading_generation(&[((0, 0), Java26ChunkStatus::StructureStarts)])
        .unwrap();
    store.unload_saved_column(0, 0).unwrap();
    assert!(store.try_load_column(1, 0).is_err());
    assert!(store
        .ensure_loading_generation(&[((1, 0), Java26ChunkStatus::Biomes)])
        .is_err());
    assert_eq!(
        store.structure_status.status(0, 0),
        crate::chunk::structure_status::NoiseStructureTaskStatus::Empty
    );
}

#[test]
fn restored_flat_features_column_advances_to_full_and_is_rewritten_whole() {
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
    use bbb_server_datapack::flat_generator::{FlatLevelGeneratorSettings, FlatLevelSource};

    let startup = crate::chunk::noise_generator::test_support::vanilla_startup();
    let shape = bbb_server_datapack::dimension_type::dimension_type_shape(
        &startup.registries,
        "minecraft:overworld",
    )
    .unwrap();
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let layers = [("minecraft:stone", 264), ("minecraft:water", 1)]
        .into_iter()
        .map(|(block, height)| {
            NbtSummaryValue::Compound(vec![
                entry("block", NbtSummaryValue::string(block)),
                entry("height", NbtSummaryValue::Int(height)),
            ])
        })
        .collect();
    let settings = FlatLevelGeneratorSettings::parse(
        &NbtSummaryValue::Compound(vec![
            entry("layers", NbtSummaryValue::List(layers)),
            entry("lakes", NbtSummaryValue::Byte(1)),
        ]),
        "test.flat",
    )
    .unwrap();
    let source = FlatLevelSource::new(settings, shape.min_y, shape.height).unwrap();
    let flat_store = || {
        let mut store = ChunkStore::new(source.clone(), 0);
        store
            .configure_ordinary_structure_generation(Arc::clone(&startup), 0, shape, false, None)
            .unwrap();
        store
    };
    let document = |store: &ChunkStore, column: &ChunkColumn| {
        let document = generation_state_chunk_document(
            column,
            0,
            0,
            &|_| Some("minecraft:plains".to_string()),
            store.state((0, 0)),
            store.structure_component(0, 0).map(|s| s.to_nbt()).as_ref(),
            &[],
        )
        .unwrap();
        Java26ChunkData::parse(
            document,
            Java26ChunkContext {
                expected_x: 0,
                expected_z: 0,
                min_section_y: store.min_y().div_euclid(16),
                section_count: store.section_count(),
            },
        )
        .unwrap()
    };
    let stone = vanilla_block_default_state_id(VANILLA_BLOCK_STONE_ID).unwrap();
    let water =
        vanilla_block_default_state_id(bbb_protocol::block_types::VANILLA_BLOCK_WATER_ID).unwrap();
    let mut original = flat_store();
    original
        .ensure_loading_generation(&[((0, 0), Java26ChunkStatus::Features)])
        .unwrap();
    assert_eq!(original.get_block_state(0, 199, 0), Some(stone));
    assert_eq!(original.get_block_state(0, 200, 0), Some(water));
    let mut column = original.loaded((0, 0)).column.clone();
    column.set_block_state(1, 200, 2, stone).unwrap();
    let saved = document(&original, &column);
    assert_eq!(saved.status(), Java26ChunkStatus::Features);

    let mut store = flat_store();
    store.insert_restored_column(0, 0, column, original.state((0, 0)).clone());
    store.restore_saved_generation_metadata(&saved).unwrap();
    store
        .ensure_loading_generation(&[((0, 0), Java26ChunkStatus::InitializeLight)])
        .unwrap();
    store.try_ensure_served_full_column(0, 0).unwrap();
    assert!(store
        .pending_whole_rewrites()
        .any(|intent| intent.chunk == (0, 0)));
    let resaved = document(&store, &store.loaded((0, 0)).column);
    assert_eq!(resaved.status(), Java26ChunkStatus::Full);
    assert_eq!(store.get_block_state(1, 200, 2), Some(stone));
    assert_eq!(store.get_block_state(0, 200, 0), Some(water));
}
