use crate::world_creation::default_overworld_level_source;
use std::collections::BTreeMap;
use std::sync::Arc;

use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_id_by_name_and_properties,
};
use bbb_protocol::block_types::{VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_STONE_ID};
use bbb_protocol::NbtSummaryValue;

use super::ChunkStore;
use crate::chunk::noise_generator::{overworld_program, NoiseFinalDensityProgram};
use bbb_protocol::chunk_light::SavedLightSection;
use bbb_server_persistence::{
    generation_state_chunk_document_with_light, Java26ChunkContext, Java26ChunkData,
    Java26ChunkStatus,
};
use bbb_server_types::chunk_data::feature_heightmaps::FeatureHeightmaps;
use bbb_server_types::chunk_data::full::ChunkNoiseFullState;
use bbb_server_types::chunk_data::spawn::ChunkSpawnState;
use bbb_server_types::chunk_data::{
    ChunkColumn, ChunkFeaturesState, ChunkGenerationState, ChunkInitializeLightState,
    ChunkLightState,
};
use bbb_server_types::save::NamedCompound;
use bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData;

const CENTER: (i32, i32) = (0, 0);
const MIN_Y: i32 = -64;
const SECTION_COUNT: usize = 24;

fn air() -> i32 {
    vanilla_block_default_state_id(VANILLA_BLOCK_AIR_ID).unwrap()
}

fn glowstone() -> i32 {
    vanilla_block_state_id_by_name_and_properties("minecraft:glowstone", &BTreeMap::new()).unwrap()
}

fn empty_column(store: &ChunkStore) -> ChunkColumn {
    ChunkColumn::new(store.min_y(), store.section_count(), air(), 0)
}

fn initialized_window(emit_block_light: bool) -> ChunkStore {
    let mut store = ChunkStore::new(default_overworld_level_source().unwrap(), 0);
    let mut columns = (-1..=1)
        .flat_map(|x| (-1..=1).map(move |z| (x, z)))
        .map(|chunk| (chunk, empty_column(&store)))
        .collect::<BTreeMap<_, _>>();
    if emit_block_light {
        columns
            .get_mut(&CENTER)
            .unwrap()
            .set_block_state(8, store.min_y() + 1, 8, glowstone())
            .unwrap();
    }
    for (chunk, column) in columns {
        let final_heightmaps = FeatureHeightmaps::prime(&column);
        store.insert_column(chunk.0, chunk.1, column);
        store.set_generation(
            chunk,
            ChunkGenerationState::Features(ChunkFeaturesState {
                postprocessing_positions: Vec::new(),
                carving_mask: Vec::new(),
                final_heightmaps,
            }),
        );
        store.loaded_mut(chunk).pending_initial_persistence = true;
    }
    store.initialize_light_window(CENTER).unwrap();
    store.light_chunk(CENTER).unwrap();
    store
}

fn wrap_spawn(store: &mut ChunkStore) {
    let ChunkGenerationState::Light(light) = store.loaded_mut(CENTER).generation.take().unwrap()
    else {
        panic!("window center must be LIGHT before SPAWN wrap")
    };
    store.set_generation(
        CENTER,
        ChunkGenerationState::Spawn(ChunkSpawnState::from_light(light, false, false)),
    );
}

fn wrap_full(store: &mut ChunkStore) {
    let ChunkGenerationState::Light(light) = store.loaded_mut(CENTER).generation.take().unwrap()
    else {
        panic!("window center must be LIGHT before FULL wrap")
    };
    store.set_generation(
        CENTER,
        ChunkGenerationState::NoiseFull(ChunkNoiseFullState::from_spawn(
            ChunkSpawnState::from_light(light, false, false),
        )),
    );
}

fn write_center(store: &ChunkStore) -> NamedCompound {
    generation_state_chunk_document_with_light(
        store.column(CENTER.0, CENTER.1).unwrap(),
        CENTER.0,
        CENTER.1,
        &|_| Some("minecraft:plains".to_string()),
        store.generation_state(CENTER.0, CENTER.1).unwrap(),
        None,
        &[],
        &store.saved_light_sections(CENTER),
    )
    .unwrap()
}

fn parse_center(store: &ChunkStore, document: NamedCompound) -> Java26ChunkData {
    Java26ChunkData::parse(
        document,
        Java26ChunkContext {
            expected_x: CENTER.0,
            expected_z: CENTER.1,
            min_section_y: store.min_y().div_euclid(16),
            section_count: store.section_count(),
        },
    )
    .unwrap()
}

fn field<'a>(document: &'a NamedCompound, name: &str) -> Option<&'a NbtSummaryValue> {
    document
        .entries()
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn sections_have_light(document: &NamedCompound) -> (bool, bool) {
    let Some(NbtSummaryValue::List(sections)) = field(document, "sections") else {
        panic!("sections must be a list")
    };
    let mut block = false;
    let mut sky = false;
    for section in sections {
        let NbtSummaryValue::Compound(entries) = section else {
            panic!("section must be a compound")
        };
        block |= entries.iter().any(|entry| entry.name == "BlockLight");
        sky |= entries.iter().any(|entry| entry.name == "SkyLight");
    }
    (block, sky)
}

fn program() -> Arc<NoiseFinalDensityProgram> {
    overworld_program(true, 0)
}

fn restore_column() -> ChunkColumn {
    let stone = vanilla_block_default_state_id(VANILLA_BLOCK_STONE_ID).unwrap();
    let mut column = ChunkColumn::new(MIN_Y, SECTION_COUNT, air(), 0);
    column.set_block_state(0, MIN_Y, 0, stone).unwrap();
    column.set_block_state(15, 17, 15, stone).unwrap();
    column
}

fn restore_features(column: &ChunkColumn) -> ChunkFeaturesState {
    ChunkFeaturesState {
        postprocessing_positions: vec![(1, MIN_Y + 1, 2)],
        carving_mask: vec![7, -3],
        final_heightmaps: FeatureHeightmaps::prime(column),
    }
}

fn constructed_light(column: &ChunkColumn, correct: bool) -> ChunkLightState {
    ChunkLightState {
        initialized: ChunkInitializeLightState {
            features: restore_features(column),
            lowest_sky_source_y: [123; 16 * 16],
            non_empty_sections: vec![123],
        },

        is_light_correct: correct,
    }
}

fn constructed_document(status: Java26ChunkStatus, correct: bool) -> NamedCompound {
    let column = restore_column();
    let light = constructed_light(&column, correct);
    let state = match status {
        Java26ChunkStatus::Light => ChunkGenerationState::Light(light),
        Java26ChunkStatus::Spawn => {
            ChunkGenerationState::Spawn(ChunkSpawnState::from_light(light, false, false))
        }
        Java26ChunkStatus::Full => ChunkGenerationState::NoiseFull(
            ChunkNoiseFullState::from_spawn(ChunkSpawnState::from_light(light, false, false)),
        ),
        _ => panic!("fixture status is not LIGHT, SPAWN or FULL"),
    };
    generation_state_chunk_document_with_light(
        &column,
        CENTER.0,
        CENTER.1,
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

fn parse_restore(document: NamedCompound) -> Java26ChunkData {
    Java26ChunkData::parse(
        document,
        Java26ChunkContext {
            expected_x: CENTER.0,
            expected_z: CENTER.1,
            min_section_y: MIN_Y.div_euclid(16),
            section_count: SECTION_COUNT,
        },
    )
    .unwrap()
}

fn destination(column: ChunkColumn) -> ChunkStore {
    let mut store =
        ChunkStore::new_overworld_carvers_with_structure_generation(program(), false).unwrap();
    store.try_load_column(CENTER.0, CENTER.1).unwrap();
    store.insert_column(CENTER.0, CENTER.1, column);
    store
}

#[test]
fn spawn_writer_reuses_light_payload_status_and_arrays() {
    let mut store = initialized_window(true);
    wrap_spawn(&mut store);
    let document = write_center(&store);
    // `ChunkStatus.java:31`; `ChunkPyramid.java:38,52`.
    assert_eq!(
        field(&document, "Status"),
        Some(&NbtSummaryValue::string("minecraft:spawn"))
    );
    assert_eq!(
        field(&document, "isLightOn"),
        Some(&NbtSummaryValue::Byte(1))
    );
    assert_eq!(
        field(&document, "entities"),
        Some(&NbtSummaryValue::List(Vec::new()))
    );
    assert!(field(&document, "carving_mask").is_some());
    let (block, sky) = sections_have_light(&document);
    assert!(block);
    assert!(sky);

    let parsed = parse_center(&store, document);
    assert_eq!(parsed.status(), Java26ChunkStatus::Spawn);
    assert!(parsed.is_light_on());
    assert!(parsed
        .sections()
        .iter()
        .any(|section| section.block_light.is_some()));
    assert!(parsed
        .sections()
        .iter()
        .any(|section| section.sky_light.is_some()));
    assert_eq!(parsed.entities(), Some([].as_slice()));
}

#[test]
fn spawn_writer_omits_light_arrays_when_the_light_column_has_none() {
    let mut store = initialized_window(false);
    wrap_spawn(&mut store);
    let document = write_center(&store);
    assert_eq!(
        field(&document, "Status"),
        Some(&NbtSummaryValue::string("minecraft:spawn"))
    );
    assert_eq!(
        field(&document, "isLightOn"),
        Some(&NbtSummaryValue::Byte(1))
    );
    let (block, sky) = sections_have_light(&document);
    assert!(!block);
    assert!(!sky);
}

#[test]
fn full_writer_reuses_light_payload_and_omits_proto_entities() {
    let mut store = initialized_window(true);
    wrap_full(&mut store);
    let document = write_center(&store);
    // `ChunkStatus.java:32`; `SerializableChunkData.java:366-375,461-467`.
    assert_eq!(
        field(&document, "Status"),
        Some(&NbtSummaryValue::string("minecraft:full"))
    );
    assert_eq!(
        field(&document, "isLightOn"),
        Some(&NbtSummaryValue::Byte(1))
    );
    assert!(field(&document, "entities").is_none());
    assert!(field(&document, "carving_mask").is_none());
    let (block, sky) = sections_have_light(&document);
    assert!(block);
    assert!(sky);

    let parsed = parse_center(&store, document);
    assert_eq!(parsed.status(), Java26ChunkStatus::Full);
    assert!(parsed.is_light_on());
    assert!(parsed.entities().is_none());
    assert!(parsed.carving_mask().is_none());
    assert!(parsed
        .sections()
        .iter()
        .any(|section| section.block_light.is_some()));
}

#[test]
fn spawn_and_full_restore_round_trip_status_and_light_correctness() {
    for correct in [false, true] {
        let parsed = parse_restore(constructed_document(Java26ChunkStatus::Spawn, correct));
        assert_eq!(parsed.status(), Java26ChunkStatus::Spawn);
        assert_eq!(parsed.is_light_on(), correct);
        let mut store = destination(restore_column());
        store.restore_saved_generation_metadata(&parsed).unwrap();
        let Some(ChunkGenerationState::Spawn(spawn)) = store.generation_state(CENTER.0, CENTER.1)
        else {
            panic!("SPAWN restore did not publish its typed state")
        };
        assert_eq!(spawn.light().is_light_correct(), correct);
        assert!(!spawn.disable_mob_generation());
        assert!(!spawn.original_mobs_refused());
        assert!(!store.chunk_flag(CENTER, |chunk| chunk.pending_initial_persistence));

        let parsed = parse_restore(constructed_document(Java26ChunkStatus::Full, correct));
        assert_eq!(parsed.status(), Java26ChunkStatus::Full);
        assert_eq!(parsed.is_light_on(), correct);
        assert!(parsed.entities().is_none());
        let mut store = destination(restore_column());
        store.restore_saved_generation_metadata(&parsed).unwrap();
        let Some(ChunkGenerationState::NoiseFull(full)) =
            store.generation_state(CENTER.0, CENTER.1)
        else {
            panic!("FULL restore did not publish its typed state")
        };
        assert_eq!(full.light().is_light_correct(), correct);
        assert!(store.proto_entity_records(CENTER.0, CENTER.1).is_empty());
    }
}

#[test]
fn restored_light_and_spawn_continue_to_rewritten_full() {
    for status in [Java26ChunkStatus::Light, Java26ChunkStatus::Spawn] {
        let parsed = parse_restore(constructed_document(status, true));
        let mut store = destination(restore_column());
        store.restore_saved_generation_metadata(&parsed).unwrap();
        for x in -2..=2 {
            for z in -2..=2 {
                store.try_load_column(x, z).unwrap();
            }
        }

        store
            .try_ensure_noise_full_column(CENTER.0, CENTER.1)
            .unwrap();
        assert!(matches!(
            store.generation_state(CENTER.0, CENTER.1),
            Some(ChunkGenerationState::NoiseFull(_))
        ));
        assert!(store
            .pending_whole_rewrites()
            .any(|intent| intent.chunk == CENTER));
    }
}
