use std::collections::BTreeMap;
use std::sync::Arc;

use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::lighting::initialize_light_window_coordinates;
use super::ChunkStore;
use crate::chunk::noise_generator::{overworld_program, NoiseFinalDensityProgram};
use bbb_server_types::chunk_data::feature_heightmaps::FeatureHeightmaps;
use bbb_server_types::chunk_data::{ChunkColumn, ChunkFeaturesState, ChunkGenerationState};

const CENTER: (i32, i32) = (0, 0);

fn air() -> i32 {
    vanilla_block_state_id_by_name_and_properties("minecraft:air", &BTreeMap::new()).unwrap()
}

fn empty_column(store: &ChunkStore) -> ChunkColumn {
    ChunkColumn::new(store.min_y(), store.section_count(), air(), 0)
}

fn program() -> Arc<NoiseFinalDensityProgram> {
    overworld_program(true, 0)
}

fn install_features_window(store: &mut ChunkStore) {
    for chunk in initialize_light_window_coordinates(CENTER).unwrap() {
        let column = empty_column(store);
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
}

fn lighted_overworld_store() -> ChunkStore {
    let mut store =
        ChunkStore::new_overworld_carvers_with_structure_generation(program(), false).unwrap();
    install_features_window(&mut store);
    store.initialize_light_window(CENTER).unwrap();
    store.light_chunk(CENTER).unwrap();
    store
}

fn proto_pig() -> Vec<NbtSummaryEntry> {
    vec![NbtSummaryEntry {
        name: "id".into(),
        value: NbtSummaryValue::string("minecraft:pig"),
    }]
}

#[test]
fn overworld_light_publishes_spawn_and_counts_the_natural_spawner_gap() {
    let mut store = lighted_overworld_store();
    let before_neighbors: BTreeMap<_, _> = initialize_light_window_coordinates(CENTER)
        .unwrap()
        .into_iter()
        .filter(|&chunk| chunk != CENTER)
        .map(|chunk| (chunk, store.state(chunk).clone()))
        .collect();

    store.spawn_chunk(CENTER).unwrap();

    let Some(ChunkGenerationState::Spawn(spawn)) = store.generation_state(CENTER.0, CENTER.1)
    else {
        panic!("SPAWN did not publish the Overworld LIGHT center")
    };
    assert!(!spawn.disable_mob_generation());
    assert!(spawn.original_mobs_refused());
    assert!(spawn.light().is_light_correct());
    assert!(store.proto_entity_records(CENTER.0, CENTER.1).is_empty());
    for (chunk, expected) in before_neighbors {
        assert_eq!(*store.state(chunk), expected);
    }
}

#[test]
fn spawn_to_noise_full_publishes_when_proto_entities_are_empty() {
    let mut store = lighted_overworld_store();
    store.spawn_chunk(CENTER).unwrap();
    let Some(ChunkGenerationState::Spawn(spawn)) = store.generation_state(CENTER.0, CENTER.1)
    else {
        panic!("SPAWN is required before FULL")
    };
    let expected = spawn.clone();

    store.full_noise_chunk(CENTER).unwrap();

    let Some(ChunkGenerationState::NoiseFull(full)) = store.generation_state(CENTER.0, CENTER.1)
    else {
        panic!("FULL did not publish the empty-entity SPAWN center")
    };
    assert_eq!(full.spawn(), &expected);
    assert!(full.light().is_light_correct());
    assert!(!full.spawn().disable_mob_generation());
    assert!(full.spawn().original_mobs_refused());
}

#[test]
fn nonempty_proto_entities_publish_at_full_atomically() {
    let mut store = lighted_overworld_store();
    store.spawn_chunk(CENTER).unwrap();
    let pig = proto_pig();
    store.loaded_mut(CENTER).proto_entities = vec![pig.clone()];

    store.full_noise_chunk(CENTER).unwrap();
    assert!(matches!(
        store.generation_state(CENTER.0, CENTER.1),
        Some(ChunkGenerationState::NoiseFull(_))
    ));
    assert!(store.proto_entity_records(CENTER.0, CENTER.1).is_empty());
    assert_eq!(
        store.published_worldgen_entity_records(CENTER.0, CENTER.1),
        [pig]
    );
}

mod original_population;
