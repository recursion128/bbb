//! The primed heightmaps an ordinary column keeps between packets.

use super::ChunkStore;
use bbb_protocol::heightmap_types::{
    VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
    VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID, VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
};
use bbb_server_types::chunk_data::feature_heightmaps::FeatureHeightmaps;

const BIOME: i32 = 3;

fn flat_store() -> ChunkStore {
    ChunkStore::new(
        crate::world_creation::default_overworld_level_source().unwrap(),
        BIOME,
    )
}

#[test]
fn ordinary_packet_heightmaps_follow_column_edits() {
    let mut store = flat_store();
    store.try_ensure_served_full_column(0, 0).unwrap();
    store.flush_live_relights().unwrap();
    let expected = FeatureHeightmaps::prime(store.column(0, 0).unwrap());
    let (min_y, height) = {
        let column = store.column(0, 0).unwrap();
        (column.min_y(), column.section_count() * 16)
    };
    let packed = |maps: &FeatureHeightmaps, kind_id: i32| {
        maps.packed_values(kind_id, min_y, height, "test").unwrap()
    };
    let first = store.full_chunk_packet_data(0, 0).unwrap();
    for map in &first.heightmaps {
        assert_eq!(map.data, packed(&expected, map.kind_id));
    }

    assert_lookups_match_scans(&store);
    assert_eq!(store.full_chunk_packet_data(0, 0).unwrap(), first);

    // Raising the surface of one column moves its heightmap on the next packet.
    let column = store.column(0, 0).unwrap();
    let surface =
        FeatureHeightmaps::scan_first_available(column, VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, 3, 5)
            .unwrap();
    let top_state = column.get_block_state(3, surface - 1, 5).unwrap();
    assert!(store
        .set_block_state(3, surface + 2, 5, top_state)
        .is_some());
    store.flush_live_relights().unwrap();
    let raised = FeatureHeightmaps::prime(store.column(0, 0).unwrap());
    assert_ne!(raised, expected);
    let second = store.full_chunk_packet_data(0, 0).unwrap();
    for map in &second.heightmaps {
        assert_eq!(map.data, packed(&raised, map.kind_id));
    }
    assert_lookups_match_scans(&store);
}

fn assert_lookups_match_scans(store: &ChunkStore) {
    let column = store.column(0, 0).unwrap();
    for kind_id in [
        VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
        VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
    ] {
        for x in 0..16 {
            for z in 0..16 {
                assert_eq!(
                    store.final_heightmap_first_available(0, 0, kind_id, x, z),
                    FeatureHeightmaps::scan_first_available(column, kind_id, x, z),
                    "kind {kind_id} at [{x},{z}]"
                );
            }
        }
    }
}
