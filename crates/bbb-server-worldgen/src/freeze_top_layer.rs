//! Vanilla `SnowAndFreezeFeature` (`Feature.FREEZE_TOP_LAYER`,
//! `Feature.java:81-83`): the surface ice-and-snow pass over one chunk.

use bbb_protocol::packets::BlockPos;

use super::{catalog_default_state, default_block_state, WorldGenLevel, UPDATE_CLIENTS};

/// Vanilla `SnowAndFreezeFeature.place` (`SnowAndFreezeFeature.java:20-49`).
///
/// Every column of the 16x16 chunk square at `origin` reads its
/// `MOTION_BLOCKING` height, then the biome sampled at the top position
/// decides ice below (`:33-36`) and snow on top with the `SNOWY` flag on the
/// block underneath (`:38-44`). The feature ignores its random source.
pub fn place_freeze_top_layer<L: WorldGenLevel>(level: &mut L, origin: BlockPos) -> Option<bool> {
    let ice = default_block_state("minecraft:ice");
    let snow = catalog_default_state("minecraft:snow");
    for dx in 0..16 {
        for dz in 0..16 {
            let x = origin.x + dx;
            let z = origin.z + dz;
            let y = level.get_motion_blocking_height(x, z);
            let top_pos = BlockPos { x, y, z };
            let below_pos = BlockPos { x, y: y - 1, z };
            if level.biome_at_should_freeze(top_pos, below_pos) {
                let _ = level.set_block(below_pos, ice, UPDATE_CLIENTS);
            }
            if level.biome_should_snow(top_pos) {
                let _ = level.set_block(top_pos, snow, UPDATE_CLIENTS);
                let below_state = level.get_block(below_pos);
                if let Some(snowy) = bbb_facts::block_state_types::vanilla_block_state_with_property(
                    below_state,
                    "snowy",
                    "true",
                ) {
                    let _ = level.set_block(below_pos, snowy, UPDATE_CLIENTS);
                }
            }
        }
    }
    Some(true)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
    use bbb_protocol::packets::BlockPos;

    use crate::test_support::SeamLevel;
    use crate::tests::{default_state, origin, place_feature, retained_from_json};
    use crate::UPDATE_CLIENTS;

    fn retained() -> bbb_server_datapack::worldgen::ConfiguredFeature {
        retained_from_json(br#"{"type":"minecraft:freeze_top_layer","config":{}}"#)
    }

    fn state_with(name: &str, properties: &[(&str, &str)]) -> i32 {
        let properties: BTreeMap<String, String> = properties
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect();
        vanilla_block_state_id_by_name_and_properties(name, &properties).unwrap()
    }

    #[test]
    fn freeze_top_layer_places_ice_snow_and_snowy_flag_per_column() {
        let mut level = SeamLevel::filled(default_state("minecraft:stone"));
        let origin = origin();
        // Column (0, 0): water top at y=70 freezes; column (3, 5): grass gets snow.
        let water_column = (origin.x, origin.z);
        let grass_column = (origin.x + 3, origin.z + 5);
        level.motion_blocking_heights.insert(water_column, 70);
        level.motion_blocking_heights.insert(grass_column, 80);
        let grass = state_with("minecraft:grass_block", &[("snowy", "false")]);
        level.set_cell(
            BlockPos {
                x: grass_column.0,
                y: 79,
                z: grass_column.1,
            },
            grass,
        );
        level
            .freeze_at
            .insert((water_column.0, 69, water_column.1), true);
        level
            .snow_at
            .insert((grass_column.0, 80, grass_column.1), true);

        assert!(place_feature(&retained(), &mut level, origin));

        let snowy_grass = state_with("minecraft:grass_block", &[("snowy", "true")]);
        let ice = default_state("minecraft:ice");
        let snow = state_with("minecraft:snow", &[("layers", "1")]);
        assert_eq!(
            *level.inner.sets.borrow(),
            vec![
                (
                    BlockPos {
                        x: water_column.0,
                        y: 69,
                        z: water_column.1
                    },
                    ice,
                    UPDATE_CLIENTS
                ),
                (
                    BlockPos {
                        x: grass_column.0,
                        y: 80,
                        z: grass_column.1
                    },
                    snow,
                    UPDATE_CLIENTS
                ),
                (
                    BlockPos {
                        x: grass_column.0,
                        y: 79,
                        z: grass_column.1
                    },
                    snowy_grass,
                    UPDATE_CLIENTS
                ),
            ]
        );
        // Every one of the 256 columns asked the biome at the top position
        // about the block below it, in dx-major then dz order.
        let queries = level.freeze_queries.borrow();
        assert_eq!(queries.len(), 256);
        assert_eq!(
            queries[0],
            (
                BlockPos {
                    x: origin.x,
                    y: 70,
                    z: origin.z
                },
                BlockPos {
                    x: origin.x,
                    y: 69,
                    z: origin.z
                }
            )
        );
        assert_eq!(
            queries[1],
            (
                BlockPos {
                    x: origin.x,
                    y: level.inner.max_y,
                    z: origin.z + 1
                },
                BlockPos {
                    x: origin.x,
                    y: level.inner.max_y - 1,
                    z: origin.z + 1
                }
            )
        );
    }
}
