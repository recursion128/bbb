use super::*;
use bbb_server_persistence::{
    generation_state_chunk_document_with_light, Java26ChunkContext, Java26ChunkData,
};
use bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData;

fn populated_fixture(centers: &[(i32, i32)]) -> ChunkStore {
    let mut store =
        ChunkStore::new_overworld_carvers_with_structure_generation(program(), false).unwrap();
    store
        .try_ensure_served_full_columns_with_population(centers, &mut |_, _| Ok(vec![]))
        .unwrap();
    for &center in centers {
        let ChunkGenerationState::NoiseFull(full) = store.state(center).clone() else {
            panic!("FULL fixture")
        };
        store.set_generation(center, ChunkGenerationState::Light(full.light().clone()));
    }
    store
}

fn population_record() -> Vec<NbtSummaryEntry> {
    bbb_server_worldgen::noise::mineshaft::MineshaftChestMinecartRecord::new(
        [0.5, 80.0, 0.5],
        [1, 2, 3, 4],
        -7,
    )
    .unwrap()
    .entries()
}

fn full_record(store: &ChunkStore, center: (i32, i32)) {
    let Some(ChunkGenerationState::NoiseFull(full)) = store.generation_state(center.0, center.1)
    else {
        panic!("FULL")
    };
    assert!(!full.spawn().original_mobs_refused());
    assert!(full.light().is_light_correct());
    assert_eq!(
        store.published_worldgen_entity_records(center.0, center.1),
        [population_record()]
    );
    assert!(store.proto_entity_records(center.0, center.1).is_empty());
}

#[test]
fn original_population_sees_light_publishes_once_and_relit_full_does_not_repeat() {
    let mut store = populated_fixture(&[CENTER]);
    let mut calls = 0;
    store
        .try_ensure_served_full_columns_with_population(&[CENTER], &mut |view, center| {
            calls += 1;
            let Some(ChunkGenerationState::Light(light)) =
                view.generation_state(center.0, center.1)
            else {
                panic!("callback must see LIGHT")
            };
            assert!(light.is_light_correct());
            assert!(!view.saved_light_sections(center).is_empty());
            Ok(vec![population_record()])
        })
        .unwrap();
    full_record(&store, CENTER);
    store
        .try_ensure_served_full_columns_with_population(&[CENTER], &mut |_, _| {
            calls += 1;
            Ok(vec![])
        })
        .unwrap();
    assert_eq!(
        store.take_published_worldgen_entity_records(0, 0),
        vec![population_record()]
    );
    let glow =
        vanilla_block_state_id_by_name_and_properties("minecraft:glowstone", &BTreeMap::new())
            .unwrap();
    store.set_live_block_state(8, 80, 8, glow).unwrap();
    store.flush_live_relights().unwrap();
    store
        .try_ensure_served_full_columns_with_population(&[CENTER], &mut |_, _| {
            calls += 1;
            Ok(vec![])
        })
        .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(store.block_light_level(8, 80, 8), 15);
    assert!(store.published_worldgen_entity_records(0, 0).is_empty());
}

fn raw_document(store: &ChunkStore) -> bbb_server_types::save::NamedCompound {
    generation_state_chunk_document_with_light(
        store.column(0, 0).unwrap(),
        0,
        0,
        &|_| Some("minecraft:plains".into()),
        store.generation_state(0, 0).unwrap(),
        Some(&NoiseStructureChunkData::default().to_nbt()),
        store.proto_entity_records(0, 0),
        &store.saved_light_sections(CENTER),
    )
    .unwrap()
}

fn parsed(store: &ChunkStore) -> Java26ChunkData {
    let document = raw_document(store);
    Java26ChunkData::parse(
        document,
        Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: store.min_y() / 16,
            section_count: store.section_count(),
        },
    )
    .unwrap()
}

#[test]
fn original_population_saved_spawn_and_full_restore_do_not_repopulate() {
    for full in [false, true] {
        let mut source = populated_fixture(&[CENTER]);
        source
            .spawn_chunk_with_population(CENTER, Some(&mut |_, _| Ok(vec![population_record()])))
            .unwrap();
        if full {
            source.full_noise_chunk(CENTER).unwrap();
        }
        let data = parsed(&source);
        let mut restored = populated_fixture(&[CENTER]);
        restored.restore_saved_generation_metadata(&data).unwrap();
        let mut calls = 0;
        restored
            .try_ensure_served_full_columns_with_population(&[CENTER], &mut |_, _| {
                calls += 1;
                Ok(vec![population_record()])
            })
            .unwrap();
        assert_eq!(calls, 0);
        assert!(matches!(
            restored.generation_state(0, 0),
            Some(ChunkGenerationState::NoiseFull(_))
        ));
        if !full {
            assert_eq!(
                restored.published_worldgen_entity_records(0, 0),
                [population_record()]
            );
        }
    }
}

#[test]
fn original_population_disabled_settings_skip_callback() {
    let mut startup = (*crate::chunk::noise_generator::overworld_startup(true)).clone();
    let element = startup
        .registries
        .registries
        .iter_mut()
        .find(|r| r.registry_id == "minecraft:worldgen/noise_settings")
        .unwrap()
        .elements
        .iter_mut()
        .find(|e| e.id == "minecraft:overworld")
        .unwrap();
    let mut settings: serde_json::Value = serde_json::from_slice(&element.source_json).unwrap();
    settings["disable_mob_generation"] = serde_json::json!(true);
    element.source_json = serde_json::to_vec(&settings).unwrap().into();
    element.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(settings)).unwrap())
            .unwrap(),
    );
    let element = element.clone();
    crate::chunk::noise_generator::test_support::publish_worldgen_element(
        &mut startup.registries,
        "minecraft:worldgen/noise_settings",
        element,
    );
    let program = Arc::new(
        crate::chunk::noise_generator::compile_noise_final_density(
            &crate::chunk::noise_generator::overworld_noise_stem(),
            Arc::new(startup),
            0,
            "test.original_population",
        )
        .unwrap(),
    );
    assert!(program.disable_mob_generation());
    let mut store =
        ChunkStore::new_overworld_carvers_with_structure_generation(program, false).unwrap();
    install_features_window(&mut store);
    store.initialize_light_window(CENTER).unwrap();
    store.light_chunk(CENTER).unwrap();
    store
        .spawn_chunk_with_population(CENTER, Some(&mut |_, _| panic!("disabled population")))
        .unwrap();
    let Some(ChunkGenerationState::Spawn(spawn)) = store.generation_state(0, 0) else {
        panic!("SPAWN")
    };
    assert!(spawn.disable_mob_generation());
    assert!(!spawn.original_mobs_refused());
}

#[test]
fn original_population_active_below_zero_retrogen_skips_callback() {
    let mut store = lighted_overworld_store();
    let mut document = raw_document(&store);
    document.entries_mut().push(NbtSummaryEntry {
        name: "below_zero_retrogen".into(),
        value: NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "target_status".into(),
                value: NbtSummaryValue::String("minecraft:full".into()),
            },
            NbtSummaryEntry {
                name: "missing_bedrock".into(),
                value: NbtSummaryValue::LongArray(vec![]),
            },
        ]),
    });
    let data = Java26ChunkData::parse(
        document,
        Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: store.min_y() / 16,
            section_count: store.section_count(),
        },
    )
    .unwrap();
    store.restore_saved_generation_metadata(&data).unwrap();
    assert!(store
        .pending_legacy_generation_work(0, 0)
        .unwrap()
        .below_zero_retrogen()
        .is_some());
    store
        .spawn_chunk_with_population(CENTER, Some(&mut |_, _| panic!("retrogen population")))
        .unwrap();
    let Some(ChunkGenerationState::Spawn(spawn)) = store.generation_state(0, 0) else {
        panic!("SPAWN")
    };
    assert!(!spawn.original_mobs_refused());
    assert!(store.proto_entity_records(0, 0).is_empty());
}
