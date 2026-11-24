use std::collections::BTreeSet;
use std::sync::Arc;

use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::{
    vanilla_block_type_for_resource_id, VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_STONE_ID,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::ChunkStore;
use crate::chunk::noise_generator::test_support::nether_program;
use crate::chunk::noise_generator::{overworld_program, NoiseFinalDensityProgram};
use bbb_protocol::chunk_light::SavedLightSection;
use bbb_server_persistence::{
    generation_state_chunk_document, generation_state_chunk_document_with_light,
    Java26ChunkContext, Java26ChunkData, Java26ChunkStatus,
};
use bbb_server_types::chunk_data::feature_heightmaps::FeatureHeightmaps;
use bbb_server_types::chunk_data::{
    ChunkColumn, ChunkFeaturesState, ChunkGenerationState, ChunkInitializeLightState,
    ChunkLightState,
};
use bbb_server_types::save::NamedCompound;
use bbb_server_worldgen::noise::mineshaft::MineshaftChestMinecartRecord;
use bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData;

const CHUNK: (i32, i32) = (0, 0);
const MIN_Y: i32 = -64;
const SECTION_COUNT: usize = 24;

fn program() -> Arc<NoiseFinalDensityProgram> {
    overworld_program(true, 0)
}

fn column() -> ChunkColumn {
    let air = vanilla_block_default_state_id(VANILLA_BLOCK_AIR_ID).unwrap();
    let stone = vanilla_block_default_state_id(VANILLA_BLOCK_STONE_ID).unwrap();
    let mut column = ChunkColumn::new(MIN_Y, SECTION_COUNT, air, 0);
    column.set_block_state(0, MIN_Y, 0, stone).unwrap();
    column.set_block_state(15, 17, 15, stone).unwrap();
    column
}

fn features(column: &ChunkColumn) -> ChunkFeaturesState {
    ChunkFeaturesState {
        postprocessing_positions: vec![(1, MIN_Y + 1, 2), (15, 17, 15)],
        carving_mask: vec![7, -3, 11],
        final_heightmaps: FeatureHeightmaps::prime(column),
    }
}

fn proto_minecart() -> Vec<NbtSummaryEntry> {
    MineshaftChestMinecartRecord::new([0.5, 40.5, 0.5], [1, 2, 3, 4], -7)
        .unwrap()
        .entries()
}

fn document(status: Java26ChunkStatus, entities: &[Vec<NbtSummaryEntry>]) -> NamedCompound {
    let column = column();
    let features = features(&column);
    let state = match status {
        Java26ChunkStatus::Carvers => ChunkGenerationState::Carvers {
            postprocessing_positions: features.postprocessing_positions.clone(),
            carving_mask: features.carving_mask.clone(),
        },
        Java26ChunkStatus::Features => ChunkGenerationState::Features(features),
        Java26ChunkStatus::InitializeLight => {
            ChunkGenerationState::InitializeLight(ChunkInitializeLightState {
                features,
                lowest_sky_source_y: [123; 16 * 16],
                non_empty_sections: vec![123],
            })
        }
        _ => panic!("fixture status is not handled"),
    };
    generation_state_chunk_document(
        &column,
        CHUNK.0,
        CHUNK.1,
        &|_| Some("minecraft:plains".to_string()),
        &state,
        Some(&NoiseStructureChunkData::default().to_nbt()),
        entities,
    )
    .unwrap()
}

fn parse(document: NamedCompound) -> Java26ChunkData {
    Java26ChunkData::parse(
        document,
        Java26ChunkContext {
            expected_x: CHUNK.0,
            expected_z: CHUNK.1,
            min_section_y: MIN_Y.div_euclid(16),
            section_count: SECTION_COUNT,
        },
    )
    .unwrap()
}

fn light_document(correct: bool) -> NamedCompound {
    let column = column();
    let state = ChunkGenerationState::Light(ChunkLightState {
        initialized: ChunkInitializeLightState {
            features: features(&column),
            lowest_sky_source_y: [123; 16 * 16],
            non_empty_sections: vec![123],
        },

        is_light_correct: correct,
    });
    generation_state_chunk_document_with_light(
        &column,
        CHUNK.0,
        CHUNK.1,
        &|_| Some("minecraft:plains".to_string()),
        &state,
        Some(&NoiseStructureChunkData::default().to_nbt()),
        &[],
        &[
            SavedLightSection {
                y: -4,
                block_light: Some(vec![0x21; 2048]),
                sky_light: Some(vec![-1; 2048]),
            },
            SavedLightSection {
                y: 1,
                block_light: Some(vec![0; 2048]),
                sky_light: None,
            },
        ],
    )
    .unwrap()
}

fn destination(column: ChunkColumn) -> ChunkStore {
    let mut store =
        ChunkStore::new_overworld_carvers_with_structure_generation(program(), false).unwrap();
    store.try_load_column(CHUNK.0, CHUNK.1).unwrap();
    store.insert_column(CHUNK.0, CHUNK.1, column);
    store
}

fn restore_error(store: &mut ChunkStore, data: &Java26ChunkData) -> String {
    store
        .restore_saved_generation_metadata(data)
        .unwrap_err()
        .to_string()
}

fn field_mut<'a>(document: &'a mut NamedCompound, name: &str) -> &'a mut NbtSummaryValue {
    &mut document
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == name)
        .unwrap()
        .value
}

fn below_zero_retrogen_document(
    status: Java26ChunkStatus,
    missing_bedrock: Vec<i64>,
) -> NamedCompound {
    let mut document = document(Java26ChunkStatus::Carvers, &[]);
    *field_mut(&mut document, "Status") = NbtSummaryValue::string(status.as_str());
    document
        .entries_mut()
        .retain(|entry| entry.name != "carving_mask");
    let heightmaps = field_mut(&mut document, "Heightmaps");
    *heightmaps = if matches!(
        status,
        Java26ChunkStatus::Noise | Java26ChunkStatus::Surface
    ) {
        NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "OCEAN_FLOOR_WG".into(),
                value: NbtSummaryValue::LongArray(vec![0; 37]),
            },
            NbtSummaryEntry {
                name: "WORLD_SURFACE_WG".into(),
                value: NbtSummaryValue::LongArray(vec![0; 37]),
            },
        ])
    } else {
        NbtSummaryValue::Compound(Vec::new())
    };
    document.entries_mut().push(NbtSummaryEntry {
        name: "below_zero_retrogen".into(),
        value: NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "target_status".into(),
                value: NbtSummaryValue::string("minecraft:carvers"),
            },
            NbtSummaryEntry {
                name: "missing_bedrock".into(),
                value: NbtSummaryValue::LongArray(missing_bedrock),
            },
        ]),
    });
    document
}

#[test]
fn below_zero_retrogen_resume_runs_pre_carvers_statuses_atomically() {
    let bedrock = vanilla_block_default_state_id(
        vanilla_block_type_for_resource_id("minecraft:bedrock")
            .unwrap()
            .id,
    )
    .unwrap();
    let deepslate = vanilla_block_default_state_id(
        vanilla_block_type_for_resource_id("minecraft:deepslate")
            .unwrap()
            .id,
    )
    .unwrap();
    let air = vanilla_block_default_state_id(VANILLA_BLOCK_AIR_ID).unwrap();
    for status in [
        Java26ChunkStatus::Empty,
        Java26ChunkStatus::Biomes,
        Java26ChunkStatus::Noise,
        Java26ChunkStatus::Surface,
    ] {
        let missing = (status == Java26ChunkStatus::Empty)
            .then_some(vec![1])
            .unwrap_or_default();
        let parsed = parse(below_zero_retrogen_document(status, missing));
        let mut saved = column();
        saved.set_block_state(1, 0, 1, bedrock).unwrap();
        let mut store = destination(saved.clone());
        store.restore_saved_generation_metadata(&parsed).unwrap();
        assert!(store.generation_state(CHUNK.0, CHUNK.1).is_none());

        store
            .finish_restored_generation_batch(&BTreeSet::from([CHUNK]))
            .unwrap();
        assert!(matches!(
            store.generation_state(CHUNK.0, CHUNK.1),
            Some(ChunkGenerationState::Carvers { .. })
        ));
        assert!(store
            .pending_legacy_generation_work(CHUNK.0, CHUNK.1)
            .is_none());
        assert_eq!(
            store
                .column(CHUNK.0, CHUNK.1)
                .unwrap()
                .get_block_state(1, 0, 1),
            Some(if status < Java26ChunkStatus::Noise {
                deepslate
            } else {
                bedrock
            })
        );
        if status == Java26ChunkStatus::Empty {
            assert_eq!(
                store
                    .column(CHUNK.0, CHUNK.1)
                    .unwrap()
                    .get_block_state(0, -64, 0),
                Some(air)
            );
        }
    }
}

#[test]
fn saved_noise_surface_resumes_without_refilling_terrain_or_biomes() {
    let program = program();
    let generated = program
        .generate_fresh_surface_chunk(CHUNK.0, CHUNK.1)
        .unwrap();
    let mut saved = generated.column().clone();
    let gold = vanilla_block_default_state_id(
        vanilla_block_type_for_resource_id("minecraft:gold_block")
            .unwrap()
            .id,
    )
    .unwrap();
    saved.set_block_state(5, 250, 7, gold).unwrap();
    let desert = crate::chunk::noise_generator::overworld_startup(true)
        .registries
        .registry("minecraft:worldgen/biome")
        .unwrap()
        .elements
        .iter()
        .position(|biome| biome.id == "minecraft:desert")
        .unwrap() as i32;
    saved.set_noise_biome(1, 0, 1, desert).unwrap();
    let state = bbb_server_types::chunk_data::ChunkPreCarversState::new(
        bbb_server_types::chunk_data::ChunkPreCarversStatus::Surface,
        generated.postprocessing_positions().to_vec(),
    );
    let data = parse(
        bbb_server_persistence::pre_carvers_chunk_document(
            &saved,
            CHUNK.0,
            CHUNK.1,
            &|id| program.biome_name(id),
            &state,
            None,
            &[],
        )
        .unwrap(),
    );
    let mut store = destination(saved);
    store.restore_saved_generation_metadata(&data).unwrap();
    store
        .finish_restored_generation_batch(&BTreeSet::from([CHUNK]))
        .unwrap();
    assert_eq!(
        store
            .pre_carvers_generation_state(CHUNK.0, CHUNK.1)
            .unwrap()
            .status(),
        &bbb_server_types::chunk_data::ChunkPreCarversStatus::Surface
    );
    store.try_load_column(CHUNK.0, CHUNK.1).unwrap();
    let column = store.column(CHUNK.0, CHUNK.1).unwrap();
    assert_eq!(column.get_block_state(5, 250, 7), Some(gold));
    assert_eq!(column.get_noise_biome(1, 0, 1), Some(desert));
    assert!(matches!(
        store.generation_state(CHUNK.0, CHUNK.1),
        Some(ChunkGenerationState::Carvers { .. })
    ));
    assert!(store
        .pre_carvers_generation_state(CHUNK.0, CHUNK.1)
        .is_none());
    assert!(store.has_whole_rewrite(CHUNK));
}

#[test]
fn legacy_generation_metadata_restores_and_completes_typed_pending_work() {
    let fixtures = [
        (
            "UpgradeData",
            NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "Sides".into(),
                    value: NbtSummaryValue::Byte(1),
                },
                NbtSummaryEntry {
                    name: "Indices".into(),
                    value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "0".into(),
                        value: NbtSummaryValue::IntArray(vec![0x123]),
                    }]),
                },
            ]),
        ),
        (
            "below_zero_retrogen",
            NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "target_status".into(),
                    value: NbtSummaryValue::string("minecraft:full"),
                },
                NbtSummaryEntry {
                    name: "missing_bedrock".into(),
                    value: NbtSummaryValue::LongArray(vec![1]),
                },
            ]),
        ),
    ];

    for (field, value) in fixtures {
        let mut raw = document(Java26ChunkStatus::InitializeLight, &[]);
        raw.entries_mut().push(NbtSummaryEntry {
            name: field.into(),
            value,
        });
        let parsed = parse(raw);
        let metadata = parsed.legacy_generation_metadata();
        assert_eq!(metadata.has_upgrade_data(), field == "UpgradeData");
        assert!(!metadata.has_blending_data());
        assert_eq!(
            metadata.has_below_zero_retrogen(),
            field == "below_zero_retrogen"
        );
        assert_eq!(metadata.field_names().collect::<Vec<_>>(), vec![field]);

        let mut store = destination(column());
        store.restore_saved_generation_metadata(&parsed).unwrap();
        let pending = store
            .pending_legacy_generation_work(CHUNK.0, CHUNK.1)
            .unwrap();
        assert!(!store.is_ticking_full(CHUNK.0, CHUNK.1));
        assert_eq!(pending.upgrade_data().is_some(), field == "UpgradeData");
        assert_eq!(
            pending.below_zero_retrogen().is_some(),
            field == "below_zero_retrogen"
        );
        if let Some(upgrade) = pending.upgrade_data() {
            assert_eq!(upgrade.indices().unwrap()[&0], [0x123]);
            assert_eq!(upgrade.sides(), 1);
            store
                .try_ensure_noise_full_column(CHUNK.0, CHUNK.1)
                .unwrap();
            assert!(store
                .pending_legacy_generation_work(CHUNK.0, CHUNK.1)
                .is_none());
        } else {
            let retrogen = pending.below_zero_retrogen().unwrap();
            assert_eq!(retrogen.target_status(), Java26ChunkStatus::Full);
            assert!(retrogen.has_bedrock_hole(0, 0));
            store
                .try_ensure_noise_full_column(CHUNK.0, CHUNK.1)
                .unwrap();
            assert!(matches!(
                store.generation_state(CHUNK.0, CHUNK.1),
                Some(ChunkGenerationState::NoiseFull(_))
            ));
            assert!(store
                .pending_legacy_generation_work(CHUNK.0, CHUNK.1)
                .is_none());
        }
    }

    assert!(parse(document(Java26ChunkStatus::InitializeLight, &[]))
        .legacy_generation_metadata()
        .is_empty());
}

#[test]
fn blending_border_ticks_restore_installs_typed_witness_and_refuses_unknown_neighbors_atomically() {
    let mut raw = document(Java26ChunkStatus::InitializeLight, &[]);
    raw.entries_mut().push(NbtSummaryEntry {
        name: "blending_data".into(),
        value: NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "min_section".into(),
                value: NbtSummaryValue::Int(-4),
            },
            NbtSummaryEntry {
                name: "max_section".into(),
                value: NbtSummaryValue::Int(20),
            },
        ]),
    });
    let parsed = parse(raw.clone());
    let expected = parsed.blending_data().cloned().unwrap();
    let mut store = destination(column());
    store.restore_saved_generation_metadata(&parsed).unwrap();
    assert_eq!(
        store.blending_witness(CHUNK).and_then(Option::as_ref),
        Some(&expected)
    );
    assert!(store.blending_border_tick_witness(CHUNK).is_err());
    for (index, chunk) in [(0, -1), (1, 0), (0, 1), (-1, 0)]
        .iter()
        .copied()
        .enumerate()
    {
        let loaded = store.put_column(chunk, column());
        loaded.generation = Some(ChunkGenerationState::Carvers {
            postprocessing_positions: Vec::new(),
            carving_mask: Vec::new(),
        });
        loaded.blending = Some(matches!(index, 1 | 2).then(|| expected.clone()));
    }
    let witness = store.blending_border_tick_witness(CHUNK).unwrap().unwrap();
    assert_eq!((witness.old_min_y, witness.old_max_y), (-64, 319));
    assert_eq!(witness.neighbor_old, [false, true, true, false]);

    raw.entries_mut()
        .retain(|entry| entry.name != "carving_mask");
    let malformed_status = parse(raw);
    let mut failed = destination(column());
    assert!(restore_error(&mut failed, &malformed_status)
        .contains("saved INITIALIZE_LIGHT state has no carving mask"));
    assert!(failed.blending_witness(CHUNK).is_none());
}

#[test]
fn initialize_light_restore_rebuilds_metadata_and_publishes_complete_proto_state() {
    let entities = vec![proto_minecart()];
    let parsed = parse(document(Java26ChunkStatus::InitializeLight, &entities));
    assert_eq!(parsed.status(), Java26ChunkStatus::InitializeLight);

    let restored_column = column();
    let expected_features = features(&restored_column);
    let mut store = destination(restored_column.clone());
    let expected = store
        .stage_initialize_light_from_features(CHUNK, expected_features.clone())
        .unwrap();
    assert_eq!(expected.lowest_sky_source_y[0], MIN_Y + 1);
    assert_eq!(expected.lowest_sky_source_y[15 + 15 * 16], 18);
    assert_eq!(expected.non_empty_sections, vec![-4, 1]);

    store.restore_saved_generation_metadata(&parsed).unwrap();

    let ChunkGenerationState::InitializeLight(initialized) =
        store.generation_state(CHUNK.0, CHUNK.1).unwrap()
    else {
        panic!("INITIALIZE_LIGHT restore did not publish its typed state")
    };
    assert_eq!(initialized, &expected);
    assert_eq!(initialized.features, expected_features);
    assert_eq!(store.column(CHUNK.0, CHUNK.1), Some(&restored_column));
    assert_eq!(store.proto_entity_records(CHUNK.0, CHUNK.1), entities);
    assert_eq!(
        store.structure_component(CHUNK.0, CHUNK.1),
        Some(&NoiseStructureChunkData::default())
    );
    assert!(!store.chunk_flag(CHUNK, |chunk| chunk.pending_initial_persistence));

    assert!(store.initialize_light(CHUNK).is_err());
}

#[test]
fn restored_carvers_and_features_behavior_is_unchanged() {
    let parsed = parse(document(Java26ChunkStatus::Carvers, &[]));
    let mut carvers = destination(column());
    carvers.restore_saved_generation_metadata(&parsed).unwrap();
    assert!(matches!(
        carvers.generation_state(CHUNK.0, CHUNK.1),
        Some(ChunkGenerationState::Carvers { .. })
    ));
    assert!(!carvers.chunk_flag(CHUNK, |chunk| chunk.pending_initial_persistence));

    let parsed = parse(document(Java26ChunkStatus::Features, &[]));
    let mut features = destination(column());
    features.restore_saved_generation_metadata(&parsed).unwrap();
    assert!(matches!(
        features.generation_state(CHUNK.0, CHUNK.1),
        Some(ChunkGenerationState::Features(_))
    ));
    assert!(!features.chunk_flag(CHUNK, |chunk| chunk.pending_initial_persistence));
}

#[test]
fn light_restore_preserves_correctness_and_packed_layers_through_the_shared_owner() {
    for correct in [false, true] {
        let parsed = parse(light_document(correct));
        assert_eq!(parsed.status(), Java26ChunkStatus::Light);
        assert_eq!(parsed.is_light_on(), correct);
        let mut store = destination(column());

        store.restore_saved_generation_metadata(&parsed).unwrap();

        let Some(ChunkGenerationState::Light(light)) = store.generation_state(CHUNK.0, CHUNK.1)
        else {
            panic!("LIGHT restore did not publish its typed state")
        };
        assert_eq!(light.is_light_correct(), correct);
        let sections = store.saved_light_sections(CHUNK);
        assert_eq!(sections.len(), if correct { 4 } else { 3 });
        let lower_shell = sections.iter().find(|section| section.y == -5).unwrap();
        assert_eq!(lower_shell.sky_light.as_deref(), Some(&vec![-1; 2048][..]));
        let lower = sections.iter().find(|section| section.y == -4).unwrap();
        assert_eq!(lower.block_light.as_deref(), Some(&vec![0x21; 2048][..]));
        assert_eq!(lower.sky_light.as_deref(), Some(&vec![-1; 2048][..]));
        let upper = sections.iter().find(|section| section.y == 1).unwrap();
        assert_eq!(upper.block_light.as_deref(), Some(&vec![0; 2048][..]));
        if correct {
            let upper_shell = sections.iter().find(|section| section.y == 2).unwrap();
            assert_eq!(upper_shell.sky_light.as_deref(), Some(&vec![-1; 2048][..]));
        }
    }
}

#[test]
fn no_skylight_restore_drops_saved_sky_keeps_block_and_does_not_enable_sky() {
    let program = nether_program(0);
    let mut source =
        ChunkStore::new_nether_carvers_with_structure_generation(Arc::clone(&program), false)
            .unwrap();
    assert!(!source.has_skylight());
    let column = source.generated.clone();
    source.insert_column(CHUNK.0, CHUNK.1, column.clone());
    let initialized = source
        .stage_initialize_light_from_features(
            CHUNK,
            ChunkFeaturesState {
                postprocessing_positions: Vec::new(),
                carving_mask: Vec::new(),
                final_heightmaps: FeatureHeightmaps::prime(&column),
            },
        )
        .unwrap();
    let state = ChunkGenerationState::Light(ChunkLightState {
        initialized,

        is_light_correct: true,
    });
    let document = generation_state_chunk_document_with_light(
        &column,
        CHUNK.0,
        CHUNK.1,
        &|id| program.biome_name(id),
        &state,
        Some(&NoiseStructureChunkData::default().to_nbt()),
        &[],
        &[
            SavedLightSection {
                y: 0,
                block_light: Some(vec![0x21; 2048]),
                sky_light: Some(vec![-1; 2048]),
            },
            SavedLightSection {
                y: 16,
                block_light: None,
                sky_light: Some(vec![-1; 2048]),
            },
        ],
    )
    .unwrap();
    let parsed = Java26ChunkData::parse(
        document,
        Java26ChunkContext {
            expected_x: CHUNK.0,
            expected_z: CHUNK.1,
            min_section_y: 0,
            section_count: 16,
        },
    )
    .unwrap();
    assert!(parsed
        .sections()
        .iter()
        .any(|section| section.sky_light.is_some()));

    let mut restored =
        ChunkStore::new_nether_carvers_with_structure_generation(program, false).unwrap();
    restored.insert_column(CHUNK.0, CHUNK.1, column);
    restored.restore_saved_generation_metadata(&parsed).unwrap();

    assert_eq!(restored.light_storage.block_level((0, 0, 0)), 1);
    assert_eq!(restored.light_storage.block_level((1, 0, 0)), 2);
    assert_eq!(restored.sky_light_level(0, 0, 0), 0);
    assert_eq!(restored.sky_light_level(0, 300, 0), 0);
    let saved = restored.saved_light_sections(CHUNK);
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].y, 0);
    assert_eq!(saved[0].block_light.as_deref(), Some(&vec![0x21; 2048][..]));
    assert!(saved[0].sky_light.is_none());
    let packet = restored.packet_light_data(CHUNK, -1, 18).unwrap();
    assert!(packet.sky_y_mask.is_empty());
    assert!(packet.empty_sky_y_mask.is_empty());
    assert!(packet.sky_updates.is_empty());
    assert!(!packet.block_y_mask.is_empty());
    assert!(!packet.block_updates.is_empty());
}
