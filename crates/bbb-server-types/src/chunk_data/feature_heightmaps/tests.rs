use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::heightmap_types::{
    VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
    VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID, VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID,
    VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
};

use super::*;

fn default_state(name: &str) -> i32 {
    vanilla_block_default_state_id(vanilla_block_type_for_resource_id(name).unwrap().id).unwrap()
}

fn column(min_y: i32, section_count: usize) -> ChunkColumn {
    ChunkColumn::new(min_y, section_count, default_state("minecraft:air"), 0)
}

fn height(maps: &FeatureHeightmaps, kind: i32, x: i32, z: i32) -> i32 {
    maps.first_available(kind, x, z).unwrap()
}

fn pack(values: &[i32; 256], min_y: i32, world_height: usize) -> Vec<i64> {
    let bits = usize::BITS as usize - world_height.leading_zeros() as usize;
    let values_per_long = 64 / bits;
    let mut packed = vec![0_i64; 256_usize.div_ceil(values_per_long)];
    for (index, value) in values.iter().enumerate() {
        let relative = u64::try_from(*value - min_y).unwrap();
        packed[index / values_per_long] |= (relative << ((index % values_per_long) * bits)) as i64;
    }
    packed
}

#[test]
fn prime_uses_each_final_heightmap_predicate() {
    let mut column = column(-16, 2);
    let stone = default_state("minecraft:stone");
    let water = default_state("minecraft:water");
    let leaves = default_state("minecraft:oak_leaves");

    column.set_block_state(0, -10, 0, stone).unwrap();
    column.set_block_state(0, -2, 0, water).unwrap();
    column.set_block_state(1, -10, 0, stone).unwrap();
    column.set_block_state(1, -2, 0, leaves).unwrap();
    column.set_block_state(2, 5, 0, stone).unwrap();

    let maps = FeatureHeightmaps::prime(&column);

    assert_eq!(height(&maps, VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, 0, 0), -1);
    assert_eq!(height(&maps, VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID, 0, 0), -9);
    assert_eq!(
        height(&maps, VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, 0, 0),
        -1
    );
    assert_eq!(
        height(&maps, VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID, 0, 0,),
        -1
    );

    assert_eq!(height(&maps, VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, 1, 0), -1);
    assert_eq!(height(&maps, VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID, 1, 0), -1);
    assert_eq!(
        height(&maps, VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, 1, 0),
        -1
    );
    assert_eq!(
        height(&maps, VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID, 1, 0,),
        -9
    );

    for kind in [
        VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
        VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
    ] {
        assert_eq!(height(&maps, kind, 2, 0), 6);
        assert_eq!(height(&maps, kind, 3, 0), -16);
    }
}

#[test]
fn empty_column_primes_every_map_to_negative_min_y() {
    let maps = FeatureHeightmaps::prime(&column(-64, 0));

    for kind in [
        VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
        VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
    ] {
        assert_eq!(height(&maps, kind, 15, 15), -64);
    }
}

#[test]
fn point_height_queries_match_all_maps_after_edits_and_reject_invalid_inputs() {
    let empty = column(-64, 0);
    let mut column = column(-16, 2);
    let kinds = [
        VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
        VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
    ];
    for block in [
        "minecraft:stone",
        "minecraft:water",
        "minecraft:oak_leaves",
        "minecraft:air",
    ] {
        for x in 0..16 {
            for z in 0..16 {
                column
                    .set_block_state(x, -16, z, default_state("minecraft:stone"))
                    .unwrap();
                column
                    .set_block_state(x, x - z, z, default_state(block))
                    .unwrap();
            }
        }
        let maps = FeatureHeightmaps::prime(&column);
        for kind in kinds {
            for x in 0..16 {
                for z in 0..16 {
                    assert_eq!(
                        FeatureHeightmaps::scan_first_available(&column, kind, x, z),
                        maps.first_available(kind, x, z),
                        "{block}: {kind} at ({x}, {z})",
                    );
                }
            }
            for (x, z) in [(-1, 0), (16, 0), (0, -1), (0, 16)] {
                assert_eq!(
                    FeatureHeightmaps::scan_first_available(&column, kind, x, z),
                    None
                );
            }
        }
    }
    assert_eq!(
        FeatureHeightmaps::scan_first_available(&column, VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID, 0, 0),
        None
    );
    for kind in kinds {
        assert_eq!(
            FeatureHeightmaps::scan_first_available(&empty, kind, 15, 15),
            Some(-64)
        );
    }
}

#[test]
fn update_pushes_up_ignores_lower_writes_and_rescans_removed_top() {
    let mut column = column(-16, 2);
    let air = default_state("minecraft:air");
    let stone = default_state("minecraft:stone");
    column.set_block_state(4, -5, 7, stone).unwrap();
    let mut maps = FeatureHeightmaps::prime(&column);

    column.set_block_state(4, -10, 7, stone).unwrap();
    maps.update_after_set(&column, 4, -10, 7, stone);
    for kind in [
        VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
        VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
    ] {
        assert_eq!(height(&maps, kind, 4, 7), -4);
    }

    column.set_block_state(4, 3, 7, stone).unwrap();
    maps.update_after_set(&column, 4, 3, 7, stone);
    for kind in [
        VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
        VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
    ] {
        assert_eq!(height(&maps, kind, 4, 7), 4);
    }

    column.set_block_state(4, 3, 7, air).unwrap();
    maps.update_after_set(&column, 4, 3, 7, air);
    for kind in [
        VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
        VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
    ] {
        assert_eq!(height(&maps, kind, 4, 7), -4);
    }
}

#[test]
fn first_available_rejects_non_final_kinds_and_non_local_coordinates() {
    let maps = FeatureHeightmaps::prime(&column(-16, 1));

    assert_eq!(
        maps.first_available(VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID, 0, 0),
        None
    );
    assert_eq!(
        maps.first_available(VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, -1, 0),
        None
    );
    assert_eq!(
        maps.first_available(VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, 16, 0),
        None
    );
    assert_eq!(
        maps.first_available(VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, 0, -1),
        None
    );
    assert_eq!(
        maps.first_available(VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, 0, 16),
        None
    );
}

#[test]
fn values_exposes_each_final_map_in_vanilla_column_order() {
    let mut column = column(-16, 2);
    let stone = default_state("minecraft:stone");
    column.set_block_state(2, 5, 3, stone).unwrap();
    let maps = FeatureHeightmaps::prime(&column);
    let index = 2 + 3 * 16;

    for kind in [
        VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
        VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
    ] {
        assert_eq!(maps.values(kind).unwrap()[index], 6);
    }
    assert!(maps.values(VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID).is_none());
}

#[test]
fn saved_simple_bit_storage_restores_exact_final_maps_and_rejects_out_of_range_y() {
    let mut column = column(-16, 2);
    column
        .set_block_state(2, 5, 3, default_state("minecraft:stone"))
        .unwrap();
    let expected = FeatureHeightmaps::prime(&column);
    let entries = [
        ("WORLD_SURFACE", VANILLA_HEIGHTMAP_WORLD_SURFACE_ID),
        ("OCEAN_FLOOR", VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID),
        ("MOTION_BLOCKING", VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID),
        (
            "MOTION_BLOCKING_NO_LEAVES",
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
        ),
    ];
    let mut saved = entries
        .into_iter()
        .map(|(key, kind)| {
            (
                key.to_string(),
                pack(expected.values(kind).unwrap(), -16, 32),
            )
        })
        .collect::<BTreeMap<_, _>>();

    assert_eq!(
        FeatureHeightmaps::from_saved(&column, &saved).unwrap(),
        expected
    );

    // Partial or empty Heightmaps primes only the missing kinds.
    let mut partial = saved.clone();
    partial.remove("OCEAN_FLOOR");
    partial.remove("MOTION_BLOCKING_NO_LEAVES");
    assert_eq!(
        FeatureHeightmaps::from_saved(&column, &partial).unwrap(),
        expected
    );
    assert_eq!(
        FeatureHeightmaps::from_saved(&column, &BTreeMap::new()).unwrap(),
        expected
    );

    saved.get_mut("WORLD_SURFACE").unwrap()[0] = 63;
    assert!(FeatureHeightmaps::from_saved(&column, &saved)
        .unwrap_err()
        .to_string()
        .contains("exceeds height 32"));
}

#[test]
fn shared_packet_and_persistence_packing_matches_simple_bit_storage() {
    let mut column = column(-64, 24);
    column
        .set_block_state(2, 319, 3, default_state("minecraft:stone"))
        .unwrap();
    let maps = FeatureHeightmaps::prime(&column);
    let packed = maps
        .packed_values(
            VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
            -64,
            384,
            "test.heightmap",
        )
        .unwrap();
    let index = 2 + 3 * 16;
    let bits = 9;
    let values_per_long = 64 / bits;
    assert_eq!(packed.len(), 256_usize.div_ceil(values_per_long));
    assert_eq!(
        (packed[index / values_per_long] as u64 >> ((index % values_per_long) * bits))
            & ((1_u64 << bits) - 1),
        384
    );
    assert!(maps
        .packed_values(VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID, -64, 384, "test")
        .is_err());
    assert!(pack_heightmap_values(&[-64; 256], -64, 0, "test").is_err());
}
