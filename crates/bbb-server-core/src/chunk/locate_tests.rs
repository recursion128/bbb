use std::collections::BTreeSet;

use crate::world_creation::default_overworld_level_source;
use bbb_protocol::packets::BlockPos;

use crate::chunk::noise_generator::overworld_program;
use crate::chunk::ChunkStore;
use bbb_server_types::chunk_data::VANILLA_26_1_BIOME_REGISTRY_LEN;

fn pos(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}

#[test]
fn locate_biome_flat_source_clamps_y_and_rejects_other_biomes() {
    let store = ChunkStore::new(default_overworld_level_source().unwrap(), 7);
    let allowed = BTreeSet::from([7]);
    assert_eq!(
        store.find_closest_biome_3d(pos(-1, -999, -1), &allowed, 6_400, 32, 64),
        Some((pos(-1, -63, -1), 7))
    );
    assert_eq!(
        store.find_closest_biome_3d(pos(4, 999, 8), &allowed, 6_400, 32, 64),
        Some((pos(4, 320, 8), 7))
    );
    assert_eq!(
        store.find_closest_biome_3d(pos(0, 64, 0), &BTreeSet::from([8]), 6_400, 32, 64,),
        None
    );
}

#[test]
fn locate_biome_noise_source_samples_origin_and_cuts_impossible_ids() {
    let program = overworld_program(true, 0);
    let registry_len = i32::try_from(VANILLA_26_1_BIOME_REGISTRY_LEN).unwrap();
    let nether_wastes = (0..registry_len)
        .find(|id| program.biome_name(*id).as_deref() == Some("minecraft:nether_wastes"))
        .unwrap();
    let store = ChunkStore::new_overworld_carvers_with_structure_generation(program.clone(), false)
        .unwrap();
    assert_eq!(
        store.find_closest_biome_3d(
            pos(0, 64, 0),
            &BTreeSet::from([nether_wastes]),
            6_400,
            32,
            64,
        ),
        None
    );

    let all_biomes = (0..registry_len).collect();
    let (found, biome) = store
        .find_closest_biome_3d(pos(0, 64, 0), &all_biomes, 0, 32, 64)
        .unwrap();
    assert_eq!(found, pos(0, 64, 0));
    assert!(program.biome_name(biome).is_some());
}

#[test]
fn locate_structure_generates_single_candidate_and_reuses_restored_start() {
    use bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData;
    use bbb_server_worldgen::noise::structures::NoiseStructurePlacement;

    let program = overworld_program(true, 0);
    let mut store =
        ChunkStore::new_overworld_carvers_with_structure_generation(program.clone(), true).unwrap();
    let source = program
        .find_structure_candidate("minecraft:mineshafts", 256, |_| true)
        .unwrap();
    assert_eq!(
        store
            .find_nearest_map_structure(
                pos(source[0] * 16, 70, source[1] * 16),
                &["minecraft:mineshaft".to_string()],
                0,
                false
            )
            .unwrap(),
        Some((
            pos(source[0] * 16, 0, source[1] * 16),
            "minecraft:mineshaft".to_string()
        ))
    );
    assert_eq!(store.structure_status_counts(), (1, 0));
    let found = store
        .find_nearest_map_structure(
            pos(source[0] * 16, 70, source[1] * 16),
            &["minecraft:mineshaft".to_string()],
            0,
            true,
        )
        .unwrap();
    assert!(found.is_some());
    let starts = store.structure_component(source[0], source[1]).unwrap();
    assert_eq!(starts.mineshaft_starts()[0].references(), 1);
    let document = starts.to_nbt();
    let reopened = program
        .restored_structure_chunk_data(&document, source[0], source[1])
        .unwrap();
    assert_eq!(reopened.mineshaft_starts()[0].references(), 1);
    assert!(store
        .pending_initial_persistence()
        .any(|chunk| chunk == (source[0], source[1])));
    store.mark_persisted(source[0], source[1]);
    // A repeated map lookup skips that start, while /locate still reports it.
    assert!(store
        .find_nearest_map_structure(
            pos(source[0] * 16, 70, source[1] * 16),
            &["minecraft:mineshaft".to_string()],
            0,
            true,
        )
        .unwrap()
        .is_none());
    assert!(store
        .find_nearest_map_structure(
            pos(source[0] * 16, 70, source[1] * 16),
            &["minecraft:mineshaft".to_string()],
            0,
            false,
        )
        .unwrap()
        .is_some());

    let generate = |[x, z]: [i32; 2]| {
        program
            .generate_shipwreck_structure_starts_component(x, z, &[])
            .unwrap()
    };
    let [x, z] = program
        .find_structure_candidate("minecraft:shipwrecks", 128, |chunk| {
            !generate(chunk).is_empty()
        })
        .unwrap();
    let saved_start = generate([x, z]).into_iter().next().unwrap();
    let id = saved_start.structure_id().to_string();
    let plan = program.natural_spawn_structure_plan().unwrap();
    let set = plan
        .sets()
        .iter()
        .find(|set| set.structures.iter().any(|entry| entry.structure == id))
        .unwrap();
    let NoiseStructurePlacement::RandomSpread { common, .. } = &set.placement else {
        panic!("shipwreck placement")
    };
    let data = NoiseStructureChunkData::from_supplied_starts(x, z, [], [])
        .with_shipwreck_starts([saved_start]);
    let document = data.to_nbt();
    let restored = program
        .restored_structure_chunk_data(&document, x, z)
        .unwrap();
    let mut restored_store =
        ChunkStore::new_overworld_carvers_with_structure_generation(program.clone(), false)
            .unwrap();
    restored_store
        .structure_status
        .restore_chunk_component(restored)
        .unwrap();
    assert_eq!(
        restored_store
            .find_nearest_map_structure(
                pos(x * 16, 64, z * 16),
                std::slice::from_ref(&id),
                0,
                false
            )
            .unwrap(),
        Some((
            pos(
                x * 16 + common.locate_offset[0],
                common.locate_offset[1],
                z * 16 + common.locate_offset[2]
            ),
            id
        ))
    );
}

#[test]
fn locate_structure_generates_nearest_ring_start() {
    let program = overworld_program(true, 0);
    let (x, z) = program
        .structure_ring_positions_component("minecraft:strongholds")
        .unwrap()[0];
    let mut store =
        ChunkStore::new_overworld_carvers_with_structure_generation(program, true).unwrap();
    assert_eq!(
        store
            .find_nearest_map_structure(
                pos(x * 16 + 8, 32, z * 16 + 8),
                &["minecraft:stronghold".to_string()],
                0,
                false
            )
            .unwrap(),
        Some((pos(x * 16, 0, z * 16), "minecraft:stronghold".to_string()))
    );
    assert_eq!(store.structure_status_counts(), (1, 0));
}

#[test]
fn locate_structure_does_not_pollute_chunk_store_with_non_matching_starts() {
    let program = overworld_program(true, 0);
    let mut store =
        ChunkStore::new_overworld_carvers_with_structure_generation(program.clone(), true).unwrap();

    // Searching across multiple empty candidate shells that miss must not retain
    // any STRUCTURE_STARTS column or mark any column for persistence.
    let miss = store
        .find_nearest_map_structure(pos(0, 64, 0), &["minecraft:mansion".to_string()], 2, false)
        .unwrap();
    assert!(miss.is_none());
    assert_eq!(store.structure_status_counts(), (0, 0));
    assert_eq!(store.chunks.len(), 0);
    assert!(store.pending_initial_persistence().next().is_none());

    // Even when a search probes multiple candidate chunks before finding a
    // match, only the single winning chunk is retained.
    let source = program
        .find_structure_candidate("minecraft:mineshafts", 256, |_| true)
        .unwrap();
    let mut fresh =
        ChunkStore::new_overworld_carvers_with_structure_generation(program, true).unwrap();
    let hit = fresh
        .find_nearest_map_structure(
            pos((source[0] - 2) * 16, 70, (source[1] - 2) * 16),
            &["minecraft:mineshaft".to_string()],
            4,
            false,
        )
        .unwrap();
    assert!(hit.is_some());
    assert_eq!(fresh.structure_status_counts(), (1, 0));
    assert_eq!(fresh.chunks.len(), 1);
}
