//! Vanilla `MultifaceGrowthFeature` (`Feature.MULTIFACE_GROWTH`,
//! `Feature.java:93-95`): glow lichen / sculk vein growth searched out from the
//! origin along the configured faces.

use bbb_protocol::packets::{BlockPos, Direction};
use bbb_registry_codec::random_source::RandomSource;
use bbb_server_datapack::worldgen::MultifaceGrowthConfig;

use super::multiface::{
    spread_from_face_toward_random_direction, state_for_placement, SpreadConfig,
    DEFAULT_SPREAD_ORDER,
};
use super::{
    block_in_holder_set, block_is, direction_opposite, is_empty_block, offset_block_pos,
    util_shuffle, WorldGenLevel, FEATURE_SET_BLOCK,
};

/// `MultifaceSpreadeableBlock.getSpreader` for the two 26.1 owners: glow
/// lichen (default config) and sculk vein (`veinSpreader`).
fn spreader(config: &MultifaceGrowthConfig) -> SpreadConfig {
    if config.block == "minecraft:sculk_vein" {
        SpreadConfig::SculkVein {
            spread_types: &DEFAULT_SPREAD_ORDER,
        }
    } else {
        SpreadConfig::Default {
            block: config.block,
        }
    }
}

/// `getShuffledDirectionsExcept`: `Util.toShuffledList` of the filtered
/// stream.
fn shuffled_directions_except(
    config: &MultifaceGrowthConfig,
    random: &mut dyn RandomSource,
    exclude_direction: Direction,
) -> Vec<Direction> {
    let mut result: Vec<Direction> = config
        .valid_directions
        .iter()
        .copied()
        .filter(|direction| *direction != exclude_direction)
        .collect();
    util_shuffle(&mut result, random);
    result
}

/// Vanilla `MultifaceGrowthFeature.place` (`MultifaceGrowthFeature.java:19-53`).
pub fn place_multiface_growth<L: WorldGenLevel>(
    config: &MultifaceGrowthConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    if !is_air_or_water(level.get_block(origin)) {
        return Some(false);
    }
    let mut search_directions = config.valid_directions.clone();
    util_shuffle(&mut search_directions, random);
    let origin_state = level.get_block(origin);
    if place_growth_if_possible(
        level,
        origin,
        origin_state,
        config,
        random,
        &search_directions,
    )? {
        return Some(true);
    }
    for search_direction in search_directions {
        let placement_directions =
            shuffled_directions_except(config, random, direction_opposite(search_direction));
        for _ in 0..config.search_range {
            // `pos.setWithOffset(origin, searchDirection)` re-anchors on the
            // origin every step, so the search never leaves the first offset.
            let pos = offset_block_pos(origin, search_direction);
            let state = level.get_block(pos);
            if !is_air_or_water(state) && !block_is(state, config.block) {
                break;
            }
            if place_growth_if_possible(level, pos, state, config, random, &placement_directions)? {
                return Some(true);
            }
        }
    }
    Some(false)
}

/// Vanilla `MultifaceGrowthFeature.placeGrowthIfPossible`
/// (`MultifaceGrowthFeature.java:55-84`).
fn place_growth_if_possible<L: WorldGenLevel>(
    level: &mut L,
    pos: BlockPos,
    old_state: i32,
    config: &MultifaceGrowthConfig,
    random: &mut dyn RandomSource,
    placement_directions: &[Direction],
) -> Option<bool> {
    for placement_direction in placement_directions {
        let neighbour_state = level.get_block(offset_block_pos(pos, *placement_direction));
        if block_in_holder_set(level, neighbour_state, &config.can_be_placed_on) {
            let Some(new_state) =
                state_for_placement(config.block, level, old_state, pos, *placement_direction)
            else {
                return Some(false);
            };
            let _ = level.set_block(pos, new_state, FEATURE_SET_BLOCK);
            level.mark_pos_for_postprocessing(pos);
            if random.next_float() < config.chance_of_spreading {
                spread_from_face_toward_random_direction(
                    spreader(config),
                    new_state,
                    level,
                    pos,
                    *placement_direction,
                    random,
                    true,
                );
            }
            return Some(true);
        }
    }
    Some(false)
}

/// Vanilla `MultifaceGrowthFeature.isAirOrWater` (`MultifaceGrowthFeature.java:86-88`).
fn is_air_or_water(state: i32) -> bool {
    is_empty_block(state) || block_is(state, "minecraft:water")
}

#[cfg(test)]
mod tests {
    use bbb_protocol::packets::Direction;

    use crate::random::WorldgenRandom;

    use crate::multiface::{all_shuffled, axis, face_property};

    use crate::tests::{default_state, origin, place_feature, retained_from_json, RecordingLevel};
    use crate::{
        block_set_bool, catalog_default_state, util_shuffle, FEATURE_SET_BLOCK, UPDATE_CLIENTS,
    };

    fn lichen_retained(
        chance_of_spreading: &str,
    ) -> bbb_server_datapack::worldgen::ConfiguredFeature {
        let json = format!(
            r#"{{"type":"minecraft:multiface_growth","config":{{"block":"minecraft:glow_lichen","search_range":1,"can_place_on_floor":true,"can_place_on_ceiling":true,"can_place_on_wall":true,"chance_of_spreading":{chance_of_spreading},"can_be_placed_on":["minecraft:stone"]}}}}"#
        );
        retained_from_json(json.as_bytes())
    }

    /// `validDirections` for floor+ceiling+wall: UP, DOWN, then the
    /// horizontal plane.
    const VALID_DIRECTIONS: [Direction; 6] = [
        Direction::Up,
        Direction::Down,
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];

    fn pocket_level() -> RecordingLevel {
        let level = RecordingLevel::filled(default_state("minecraft:stone"));
        let origin = origin();
        level.cells.borrow_mut().insert(
            (origin.x, origin.y, origin.z),
            default_state("minecraft:air"),
        );
        level
    }

    #[test]
    fn multiface_growth_places_lichen_on_the_first_shuffled_face_of_an_air_pocket() {
        let mut level = pocket_level();
        assert!(place_feature(&lichen_retained("0.0"), &mut level, origin()));

        let mut random = WorldgenRandom::with_seed(1);
        let mut directions = VALID_DIRECTIONS;
        util_shuffle(&mut directions, &mut random);
        let lichen = block_set_bool(
            catalog_default_state("minecraft:glow_lichen"),
            face_property(directions[0]),
            true,
        );
        assert_eq!(
            *level.sets.borrow(),
            vec![(origin(), lichen, FEATURE_SET_BLOCK)]
        );
        assert_eq!(*level.postprocess.borrow(), vec![origin()]);
    }

    #[test]
    fn multiface_growth_spreads_once_toward_the_first_off_axis_shuffled_direction() {
        let mut level = pocket_level();
        assert!(place_feature(&lichen_retained("1.0"), &mut level, origin()));

        let mut random = WorldgenRandom::with_seed(1);
        let mut directions = VALID_DIRECTIONS;
        util_shuffle(&mut directions, &mut random);
        let first_face = directions[0];
        let _chance = random.next_float();
        let spread_direction = all_shuffled(&mut random)
            .into_iter()
            .find(|direction| axis(*direction) != axis(first_face))
            .unwrap();
        let lichen = catalog_default_state("minecraft:glow_lichen");
        let first = block_set_bool(lichen, face_property(first_face), true);
        let second = block_set_bool(first, face_property(spread_direction), true);
        assert_eq!(
            *level.sets.borrow(),
            vec![
                (origin(), first, FEATURE_SET_BLOCK),
                (origin(), second, UPDATE_CLIENTS),
            ]
        );
        assert_eq!(*level.postprocess.borrow(), vec![origin(), origin()]);
    }

    #[test]
    fn multiface_growth_refuses_a_solid_origin() {
        let mut level = RecordingLevel::filled(default_state("minecraft:stone"));
        assert!(!place_feature(
            &lichen_retained("0.0"),
            &mut level,
            origin()
        ));
        assert!(level.sets.borrow().is_empty());
    }
}
