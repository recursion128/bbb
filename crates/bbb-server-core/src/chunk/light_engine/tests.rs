use crate::world_creation::default_overworld_level_source;
use bbb_server_types::chunk_data::ChunkGenerationState;
use std::collections::BTreeMap;

use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use sha2::{Digest, Sha256};

use super::*;
use bbb_server_persistence::{
    generation_state_chunk_document_with_light, Java26ChunkContext, Java26ChunkData,
    Java26ChunkStatus,
};
use bbb_server_types::chunk_data::feature_heightmaps::FeatureHeightmaps;
use bbb_server_types::chunk_data::{ChunkColumn, ChunkFeaturesState};

const CENTER: (i32, i32) = (0, 0);

fn state(name: &str, properties: &[(&str, &str)]) -> i32 {
    vanilla_block_state_id_by_name_and_properties(
        name,
        &properties
            .iter()
            .map(|&(name, value)| (name.to_string(), value.to_string()))
            .collect::<BTreeMap<_, _>>(),
    )
    .unwrap()
}

fn air() -> i32 {
    state("minecraft:air", &[])
}

fn empty_column(store: &ChunkStore) -> ChunkColumn {
    ChunkColumn::new(store.min_y(), store.section_count(), air(), 0)
}

fn initialized_window(
    edit: impl FnOnce(&mut BTreeMap<(i32, i32), ChunkColumn>, i32),
) -> ChunkStore {
    initialized_window_with_skylight(true, edit)
}

fn initialized_window_with_skylight(
    has_skylight: bool,
    edit: impl FnOnce(&mut BTreeMap<(i32, i32), ChunkColumn>, i32),
) -> ChunkStore {
    let mut store = ChunkStore::new_with_biome_registry_and_skylight(
        default_overworld_level_source().unwrap(),
        0,
        bbb_server_types::chunk_data::VANILLA_26_1_BIOME_REGISTRY_LEN,
        has_skylight,
    );
    let mut columns = initialize_light_window_coordinates(CENTER)
        .unwrap()
        .into_iter()
        .map(|chunk| (chunk, empty_column(&store)))
        .collect::<BTreeMap<_, _>>();
    edit(&mut columns, store.min_y());
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
    store
}

fn adjacent_initialized_windows(
    edit: impl FnOnce(&mut BTreeMap<(i32, i32), ChunkColumn>, i32),
) -> ChunkStore {
    let mut store = ChunkStore::new(default_overworld_level_source().unwrap(), 0);
    let mut columns = (-1..=2)
        .flat_map(|x| (-1..=1).map(move |z| (x, z)))
        .map(|chunk| (chunk, empty_column(&store)))
        .collect::<BTreeMap<_, _>>();
    edit(&mut columns, store.min_y());
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
    store.initialize_light_window((1, 0)).unwrap();
    store
}

fn packet_mask(mask: &[i64], section_count: usize) -> String {
    (0..section_count)
        .map(|index| {
            let set = mask
                .get(index / 64)
                .is_some_and(|word| (*word as u64) & (1_u64 << (index % 64)) != 0);
            if set {
                '1'
            } else {
                '0'
            }
        })
        .collect()
}

fn packet_updates(mask: &[i64], updates: &[Vec<u8>]) -> Vec<(usize, String)> {
    let mut updates = updates.iter();
    let projected = (0..mask.len() * 64)
        .filter(|index| (mask[index / 64] as u64) & (1_u64 << (index % 64)) != 0)
        .map(|index| {
            let bytes = updates.next().expect("each packet mask bit owns one array");
            (index, format!("{:x}", Sha256::digest(bytes)))
        })
        .collect();
    assert!(updates.next().is_none());
    projected
}

fn expected_light_updates(projection: &serde_json::Value, key: &str) -> Vec<(usize, String)> {
    projection[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|update| {
            (
                update["section_index"].as_u64().unwrap() as usize,
                update["sha256"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[test]
fn controlled_window_matches_official_clientbound_light_arrays() {
    let glowstone = state("minecraft:glowstone", &[]);
    let mut store = initialized_window(|columns, _| {
        columns
            .get_mut(&CENTER)
            .unwrap()
            .set_block_state(8, 200, 8, glowstone)
            .unwrap();
    });
    store.light_chunk(CENTER).unwrap();

    let fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../scripts/data/noise-overworld-light-26.1.json"
    )))
    .unwrap();
    let min_section_y = fixture["scope"]["min_light_section"].as_i64().unwrap() as i32;
    let section_count = fixture["scope"]["light_sections"].as_u64().unwrap() as usize;
    let packet = store
        .light_storage
        .packet_data(CENTER, min_section_y, section_count)
        .unwrap();
    let expected = &fixture["projection"];
    assert_eq!(
        packet_mask(&packet.sky_y_mask, section_count),
        expected["sky_mask"]
    );
    assert_eq!(
        packet_mask(&packet.block_y_mask, section_count),
        expected["block_mask"]
    );
    assert_eq!(
        packet_mask(&packet.empty_sky_y_mask, section_count),
        expected["empty_sky_mask"]
    );
    assert_eq!(
        packet_mask(&packet.empty_block_y_mask, section_count),
        expected["empty_block_mask"]
    );
    assert_eq!(
        packet_updates(&packet.sky_y_mask, &packet.sky_updates),
        expected_light_updates(expected, "sky_updates")
    );
    assert_eq!(
        packet_updates(&packet.block_y_mask, &packet.block_updates),
        expected_light_updates(expected, "block_updates")
    );
}

#[test]
fn fresh_center_block_source_propagates_in_order_across_chunk_boundary() {
    let glowstone = state("minecraft:glowstone", &[]);
    let store = initialized_window(|columns, min_y| {
        columns
            .get_mut(&CENTER)
            .unwrap()
            .set_block_state(15, min_y + 1, 8, glowstone)
            .unwrap();
    });

    let staged = store
        .stage_light_chunk(CENTER, &ChunkLightStorage::default())
        .unwrap();
    let y = store.min_y() + 1;
    assert_eq!(staged.storage().block_level((15, y, 8)), 15);
    assert_eq!(staged.storage().block_level((14, y, 8)), 14);
    assert_eq!(staged.storage().block_level((16, y, 8)), 14);
    assert_eq!(staged.storage().block_level((17, y, 8)), 13);
    assert_eq!(staged.storage().block_level((29, y, 8)), 1);
    assert_eq!(staged.storage().block_level((30, y, 8)), 0);
    assert_eq!(staged.storage().sky_level((15, y + 2, 8)), 15);
}

#[test]
fn flat_no_skylight_runs_block_light_without_sky_storage_save_or_packet_payload() {
    let glowstone = state("minecraft:glowstone", &[]);
    let mut store = initialized_window_with_skylight(false, |columns, min_y| {
        columns
            .get_mut(&CENTER)
            .unwrap()
            .set_block_state(8, min_y + 1, 8, glowstone)
            .unwrap();
    });
    assert!(!store.has_skylight());

    store.light_chunk(CENTER).unwrap();

    let y = store.min_y() + 1;
    assert_eq!(store.light_storage.block_level((8, y, 8)), 15);
    assert_eq!(store.light_storage.block_level((9, y, 8)), 14);
    assert_eq!(store.sky_light_level(8, y, 8), 0);
    assert_eq!(store.sky_light_level(8, 400, 8), 0);
    let saved = store.saved_light_sections(CENTER);
    assert!(saved.iter().any(|section| section.block_light.is_some()));
    assert!(saved.iter().all(|section| section.sky_light.is_none()));

    let packet = store
        .packet_light_data(
            CENTER,
            store.min_y().div_euclid(16) - 1,
            store.section_count() + 2,
        )
        .unwrap();
    assert!(packet.sky_y_mask.is_empty());
    assert!(packet.empty_sky_y_mask.is_empty());
    assert!(packet.sky_updates.is_empty());
    assert!(!packet.block_y_mask.is_empty());
    assert!(!packet.block_updates.is_empty());
}

#[test]
fn only_the_light_center_contributes_block_emission() {
    let glowstone = state("minecraft:glowstone", &[]);
    let store = initialized_window(|columns, min_y| {
        columns
            .get_mut(&(1, 0))
            .unwrap()
            .set_block_state(0, min_y + 1, 8, glowstone)
            .unwrap();
    });

    let staged = store
        .stage_light_chunk(CENTER, &ChunkLightStorage::default())
        .unwrap();
    assert_eq!(staged.storage().block_level((16, store.min_y() + 1, 8)), 0);
}

#[test]
fn all_air_window_has_implicit_sky_without_serialized_layers() {
    let store = initialized_window(|_, _| {});

    let staged = store
        .stage_light_chunk(CENTER, &ChunkLightStorage::default())
        .unwrap();

    assert_eq!(staged.storage().block_level((8, store.min_y(), 8)), 0);
    assert_eq!(staged.storage().sky_level((8, store.min_y(), 8)), 15);
    assert!(staged.storage().saved_sections(CENTER).is_empty());
}

#[test]
fn sky_sources_descend_through_a_roof_hole_and_spread_below_opaque_blocks() {
    let stone = state("minecraft:stone", &[]);
    let store = initialized_window(|columns, min_y| {
        let roof_y = min_y + 16;
        let center = columns.get_mut(&CENTER).unwrap();
        for z in 0..16 {
            for x in 0..16 {
                if (x, z) != (8, 8) {
                    center.set_block_state(x, roof_y, z, stone).unwrap();
                }
            }
        }
    });

    let staged = store
        .stage_light_chunk(CENTER, &ChunkLightStorage::default())
        .unwrap();
    let below = store.min_y() + 15;
    assert_eq!(staged.storage().sky_level((8, below, 8)), 15);
    assert_eq!(staged.storage().sky_level((9, below, 8)), 14);
    assert_eq!(staged.storage().sky_level((10, below, 8)), 13);
    assert_eq!(staged.storage().sky_level((9, below + 1, 8)), 0);
}

#[test]
fn adjacent_light_centers_reuse_shared_layers_and_add_only_their_own_sources() {
    let glowstone = state("minecraft:glowstone", &[]);
    let mut store = adjacent_initialized_windows(|columns, min_y| {
        columns
            .get_mut(&(1, 0))
            .unwrap()
            .set_block_state(0, min_y + 1, 8, glowstone)
            .unwrap();
    });
    let y = store.min_y() + 1;

    store.light_chunk(CENTER).unwrap();
    assert_eq!(store.light_storage.block_level((16, y, 8)), 0);
    store.light_chunk((1, 0)).unwrap();

    assert_eq!(store.light_storage.block_level((16, y, 8)), 15);
    assert_eq!(store.light_storage.block_level((15, y, 8)), 14);
    assert!(matches!(
        store.generation_state(0, 0),
        Some(ChunkGenerationState::Light(_))
    ));
    assert!(matches!(
        store.generation_state(1, 0),
        Some(ChunkGenerationState::Light(_))
    ));
}

#[test]
fn opaque_target_blocks_the_direct_edge_and_section_shell_is_exact() {
    let glowstone = state("minecraft:glowstone", &[]);
    let stone = state("minecraft:stone", &[]);
    let store = initialized_window(|columns, min_y| {
        let center = columns.get_mut(&CENTER).unwrap();
        center.set_block_state(8, min_y + 1, 8, glowstone).unwrap();
        center.set_block_state(9, min_y + 1, 8, stone).unwrap();
    });

    let staged = store
        .stage_light_chunk(CENTER, &ChunkLightStorage::default())
        .unwrap();
    let y = store.min_y() + 1;
    assert_eq!(staged.storage().block_level((8, y, 8)), 15);
    assert_eq!(staged.storage().block_level((9, y, 8)), 0);
    assert_eq!(staged.storage().block_level((10, y, 8)), 11);
}

#[test]
fn light_chunk_publishes_only_the_center_and_writes_exact_light_payload_shape() {
    let glowstone = state("minecraft:glowstone", &[]);
    let mut store = initialized_window(|columns, min_y| {
        columns
            .get_mut(&CENTER)
            .unwrap()
            .set_block_state(8, min_y + 1, 8, glowstone)
            .unwrap();
    });

    store.light_chunk(CENTER).unwrap();

    let ChunkGenerationState::Light(light) = store.generation_state(0, 0).unwrap() else {
        panic!("LIGHT did not publish the center")
    };
    assert!(light.is_light_correct());
    for chunk in initialize_light_window_coordinates(CENTER).unwrap() {
        if chunk != CENTER {
            assert!(matches!(
                store.generation_state(chunk.0, chunk.1),
                Some(ChunkGenerationState::InitializeLight(_))
            ));
        }
    }

    let saved_light = store.saved_light_sections(CENTER);
    assert!(saved_light
        .iter()
        .any(|section| section.block_light.is_some()));
    assert!(saved_light
        .iter()
        .any(|section| section.sky_light.is_some()));
    let document = generation_state_chunk_document_with_light(
        store.column(0, 0).unwrap(),
        0,
        0,
        &|_| Some("minecraft:plains".to_string()),
        store.generation_state(0, 0).unwrap(),
        None,
        &[],
        &saved_light,
    )
    .unwrap();
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
    assert_eq!(parsed.status(), Java26ChunkStatus::Light);
    assert!(parsed.is_light_on());
    assert!(parsed
        .sections()
        .iter()
        .any(|section| section.block_light.is_some()));
    assert!(parsed
        .sections()
        .iter()
        .any(|section| section.sky_light.is_some()));
}
