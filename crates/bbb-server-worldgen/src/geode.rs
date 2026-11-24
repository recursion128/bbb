//! Vanilla `GeodeFeature` (`Feature.GEODE`, `Feature.java:144`): the layered
//! amethyst geode shell with its crack and crystal buds.

use crate::random::WorldgenRandom;
use bbb_protocol::packets::BlockPos;
use bbb_registry_codec::random_source::RandomSource;

use bbb_server_datapack::worldgen::GeodeConfig;
use bbb_server_types::fluid::block_state_fluid_state;

use super::multiface::DIRECTIONS;
use super::noise::NormalNoise;
use super::state_providers::BlockStateProviderExt;
use super::value_providers::sample_int;
use super::{
    between_closed, block_has_property, block_is, block_set_bool, block_set_property,
    default_block_state, dist_to_low_corner_sqr, is_empty_block, offset_block_pos, WorldGenLevel,
    UPDATE_CLIENTS,
};

/// Vanilla `GeodeFeature.place` (`GeodeFeature.java:35-170`).
pub fn place_geode<L: WorldGenLevel>(
    config: &GeodeConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let min_gen_offset = config.min_gen_offset;
    let max_gen_offset = config.max_gen_offset;
    let mut points: Vec<(BlockPos, i32)> = Vec::new();
    let num_points = sample_int(&config.distribution_points, random);
    let seed = level.get_seed();
    let mut level_random = WorldgenRandom::named(seed, "minecraft:geode");
    let noise = NormalNoise::create(&mut level_random, -4, &[1.0])?;
    let mut crack_points: Vec<BlockPos> = Vec::new();
    let crack_size_adjustment =
        f64::from(num_points) / f64::from(config.outer_wall_distance.max_inclusive());
    let inner_air = 1.0 / config.filling.sqrt();
    let innermost_block_layer = 1.0 / (config.inner_layer + crack_size_adjustment).sqrt();
    let inner_crust = 1.0 / (config.middle_layer + crack_size_adjustment).sqrt();
    let outer_crust = 1.0 / (config.outer_layer + crack_size_adjustment).sqrt();
    let crack_size = 1.0
        / (config.base_crack_size
            + random.next_double() / 2.0
            + if num_points > 3 {
                crack_size_adjustment
            } else {
                0.0
            })
        .sqrt();
    let should_generate_crack = f64::from(random.next_float()) < config.generate_crack_chance;
    let mut num_invalid_points = 0;
    for _ in 0..num_points {
        let x = sample_int(&config.outer_wall_distance, random);
        let y = sample_int(&config.outer_wall_distance, random);
        let z = sample_int(&config.outer_wall_distance, random);
        let pos = BlockPos {
            x: origin.x + x,
            y: origin.y + y,
            z: origin.z + z,
        };
        let state = level.get_block(pos);
        if is_empty_block(state) || level.block_state_in_tag(state, &config.invalid_blocks) {
            num_invalid_points += 1;
            if num_invalid_points > config.invalid_blocks_threshold {
                return Some(false);
            }
        }
        points.push((pos, sample_int(&config.point_offset, random)));
    }
    if should_generate_crack {
        let offset_index = random.next_int(4);
        let crack_offset = num_points * 2 + 1;
        let (dx, dz) = match offset_index {
            0 => (crack_offset, 0),
            1 => (0, crack_offset),
            2 => (crack_offset, crack_offset),
            _ => (0, 0),
        };
        for dy in [7, 5, 1] {
            crack_points.push(BlockPos {
                x: origin.x + dx,
                y: origin.y + dy,
                z: origin.z + dz,
            });
        }
    }
    let mut potential_crystal_placements: Vec<BlockPos> = Vec::new();
    let air = default_block_state("minecraft:air");
    for point_inside in between_closed(
        BlockPos {
            x: origin.x + min_gen_offset,
            y: origin.y + min_gen_offset,
            z: origin.z + min_gen_offset,
        },
        BlockPos {
            x: origin.x + max_gen_offset,
            y: origin.y + max_gen_offset,
            z: origin.z + max_gen_offset,
        },
    ) {
        let noise_offset = noise.get_value(
            f64::from(point_inside.x),
            f64::from(point_inside.y),
            f64::from(point_inside.z),
        ) * config.noise_multiplier;
        let mut dist_sum_shell = 0.0;
        let mut dist_sum_crack = 0.0;
        for (point, offset) in &points {
            dist_sum_shell +=
                inv_sqrt(dist_to_low_corner_sqr(point_inside, *point) + f64::from(*offset))
                    + noise_offset;
        }
        for point in &crack_points {
            dist_sum_crack += inv_sqrt(
                dist_to_low_corner_sqr(point_inside, *point) + f64::from(config.crack_point_offset),
            ) + noise_offset;
        }
        if dist_sum_shell < outer_crust {
            continue;
        }
        if should_generate_crack && dist_sum_crack >= crack_size && dist_sum_shell < inner_air {
            safe_set_block(level, point_inside, air, &config.cannot_replace);
            for direction in DIRECTIONS {
                let adjacent_pos = offset_block_pos(point_inside, direction);
                if !block_state_fluid_state(level.get_block(adjacent_pos)).is_empty() {
                    level.schedule_fluid_tick(adjacent_pos, 0);
                }
            }
        } else if dist_sum_shell >= inner_air {
            let state = config
                .filling_provider
                .get_state(level, random, point_inside)?;
            safe_set_block(level, point_inside, state, &config.cannot_replace);
        } else if dist_sum_shell >= innermost_block_layer {
            let use_alternate_layer =
                f64::from(random.next_float()) < config.use_alternate_layer0_chance;
            let provider = if use_alternate_layer {
                &config.alternate_inner_layer_provider
            } else {
                &config.inner_layer_provider
            };
            let state = provider.get_state(level, random, point_inside)?;
            safe_set_block(level, point_inside, state, &config.cannot_replace);
            if (!config.placements_require_layer0_alternate || use_alternate_layer)
                && f64::from(random.next_float()) < config.use_potential_placements_chance
            {
                potential_crystal_placements.push(point_inside);
            }
        } else if dist_sum_shell >= inner_crust {
            let state = config
                .middle_layer_provider
                .get_state(level, random, point_inside)?;
            safe_set_block(level, point_inside, state, &config.cannot_replace);
        } else if dist_sum_shell >= outer_crust {
            let state = config
                .outer_layer_provider
                .get_state(level, random, point_inside)?;
            safe_set_block(level, point_inside, state, &config.cannot_replace);
        }
    }
    let inner_placements = &config.inner_placements;
    for crystal_pos in potential_crystal_placements {
        let mut block_state =
            inner_placements[random.next_int(inner_placements.len() as i32) as usize];
        for direction in DIRECTIONS {
            if block_has_property(block_state, "facing") {
                block_state = block_set_property(block_state, "facing", direction_name(direction));
            }
            let place_pos = offset_block_pos(crystal_pos, direction);
            let place_state = level.get_block(place_pos);
            if block_has_property(block_state, "waterlogged") {
                block_state = block_set_bool(
                    block_state,
                    "waterlogged",
                    block_state_fluid_state(place_state).is_source(),
                );
            }
            if can_cluster_grow_at_state(place_state) {
                safe_set_block(level, place_pos, block_state, &config.cannot_replace);
                break;
            }
        }
    }
    Some(true)
}

/// `Mth.invSqrt(double)` (`Mth.java:426-428`), JOML `1.0 / Math.sqrt(x)`.
fn inv_sqrt(x: f64) -> f64 {
    1.0 / x.sqrt()
}

/// `Feature.safeSetBlock` (`Feature.java:177-181`) with
/// `Feature.isReplaceable(cannotReplace)` (`:173-175`).
fn safe_set_block<L: WorldGenLevel>(
    level: &mut L,
    pos: BlockPos,
    state: i32,
    cannot_replace: &str,
) {
    let existing = level.get_block(pos);
    if !level.block_state_in_tag(existing, cannot_replace) {
        let _ = level.set_block(pos, state, UPDATE_CLIENTS);
    }
}

/// `BuddingAmethystBlock.canClusterGrowAtState` (`BuddingAmethystBlock.java:52-54`).
fn can_cluster_grow_at_state(state: i32) -> bool {
    is_empty_block(state)
        || (block_is(state, "minecraft:water") && block_state_fluid_state(state).is_full())
}

fn direction_name(direction: bbb_protocol::packets::Direction) -> &'static str {
    use bbb_protocol::packets::Direction;
    match direction {
        Direction::Down => "down",
        Direction::Up => "up",
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
    use bbb_protocol::packets::BlockPos;

    use crate::test_support::SeamLevel;
    use crate::tests::{default_state, origin, place_feature, retained_from_json};

    /// Vanilla `amethyst_geode` (`data/minecraft/worldgen/configured_feature/amethyst_geode.json`).
    const AMETHYST_GEODE: &str = r##"{"type":"minecraft:geode","config":{"blocks":{"alternate_inner_layer_provider":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:budding_amethyst"}},"cannot_replace":"#minecraft:features_cannot_replace","filling_provider":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:air"}},"inner_layer_provider":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:amethyst_block"}},"inner_placements":[{"Name":"minecraft:small_amethyst_bud","Properties":{"facing":"up","waterlogged":"false"}},{"Name":"minecraft:medium_amethyst_bud","Properties":{"facing":"up","waterlogged":"false"}},{"Name":"minecraft:large_amethyst_bud","Properties":{"facing":"up","waterlogged":"false"}},{"Name":"minecraft:amethyst_cluster","Properties":{"facing":"up","waterlogged":"false"}}],"invalid_blocks":"#minecraft:geode_invalid_blocks","middle_layer_provider":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:calcite"}},"outer_layer_provider":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:smooth_basalt"}}},"crack":{"base_crack_size":2.0,"crack_point_offset":2,"generate_crack_chance":0.95},"distribution_points":{"type":"minecraft:uniform","max_inclusive":4,"min_inclusive":3},"invalid_blocks_threshold":1,"layers":{"filling":1.7,"inner_layer":2.2,"middle_layer":3.2,"outer_layer":4.2},"max_gen_offset":16,"min_gen_offset":-16,"noise_multiplier":0.05,"outer_wall_distance":{"type":"minecraft:uniform","max_inclusive":6,"min_inclusive":4},"placements_require_layer0_alternate":true,"point_offset":{"type":"minecraft:uniform","max_inclusive":2,"min_inclusive":1},"use_alternate_layer0_chance":0.083,"use_potential_placements_chance":0.35}}"##;

    fn block_type(name: &'static str) -> Option<i32> {
        vanilla_block_state_block_type_id(crate::catalog_default_state(name))
    }

    #[test]
    fn amethyst_geode_in_stone_builds_nested_shells_inside_the_gen_cube() {
        let mut level = SeamLevel::filled(default_state("minecraft:stone"));
        level.seed = 1234;
        let retained = retained_from_json(AMETHYST_GEODE.as_bytes());
        assert!(place_feature(&retained, &mut level, origin()));

        let origin = origin();
        let sets = level.inner.sets.borrow().clone();
        assert!(!sets.is_empty());
        let mut families = std::collections::BTreeSet::new();
        for (pos, state, _) in &sets {
            assert!(
                (pos.x - origin.x).abs() <= 16
                    && (pos.y - origin.y).abs() <= 16
                    && (pos.z - origin.z).abs() <= 16
            );
            families.insert(vanilla_block_state_block_type_id(*state));
        }
        for name in [
            "minecraft:smooth_basalt",
            "minecraft:calcite",
            "minecraft:amethyst_block",
            "minecraft:air",
        ] {
            assert!(families.contains(&block_type(name)), "missing {name}");
        }
        // Every crystal stands on a cell that was carved to air first, and
        // the outer shell is nearer the rim than the filling.
        let shell: Vec<_> = sets
            .iter()
            .filter(|(_, s, _)| {
                vanilla_block_state_block_type_id(*s) == block_type("minecraft:smooth_basalt")
            })
            .collect();
        let filling: Vec<_> = sets
            .iter()
            .filter(|(_, s, _)| {
                vanilla_block_state_block_type_id(*s) == block_type("minecraft:air")
            })
            .collect();
        let mean_dist = |cells: &[&(BlockPos, i32, i32)]| {
            cells
                .iter()
                .map(|(p, _, _)| {
                    (((p.x - origin.x).pow(2) + (p.y - origin.y).pow(2) + (p.z - origin.z).pow(2))
                        as f64)
                        .sqrt()
                })
                .sum::<f64>()
                / cells.len() as f64
        };
        assert!(mean_dist(&shell) > mean_dist(&filling));
    }

    #[test]
    fn geode_noise_provider_samples_the_point_inside_position() {
        let json = AMETHYST_GEODE.replace(
            r#""filling_provider":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:air"}}"#,
            r#""filling_provider":{"type":"minecraft:noise_provider","seed":1,"noise":{"firstOctave":-3,"amplitudes":[1.0]},"scale":0.5,"states":[{"Name":"minecraft:air"},{"Name":"minecraft:water"}]}"#,
        );
        let mut level = SeamLevel::filled(default_state("minecraft:stone"));
        level.seed = 1234;
        let retained = retained_from_json(json.as_bytes());
        assert!(place_feature(&retained, &mut level, origin()));

        let filling: std::collections::BTreeSet<_> = level
            .inner
            .sets
            .borrow()
            .iter()
            .map(|(_, state, _)| vanilla_block_state_block_type_id(*state))
            .filter(|state| {
                *state == block_type("minecraft:air") || *state == block_type("minecraft:water")
            })
            .collect();
        assert_eq!(
            filling,
            [block_type("minecraft:air"), block_type("minecraft:water")]
                .into_iter()
                .collect()
        );
    }

    #[test]
    fn geode_gives_up_after_too_many_invalid_points() {
        let retained = retained_from_json(AMETHYST_GEODE.as_bytes());
        let mut seeded = SeamLevel::admitting();
        seeded.seed = 7;
        assert!(!place_feature(&retained, &mut seeded, origin()));
        assert!(seeded.inner.sets.borrow().is_empty());
    }
}
