//! Vanilla `PlacedFeature.place` and the
//! placement modifiers.

use std::sync::OnceLock;

use crate::random::WorldgenRandom;
use bbb_protocol::packets::BlockPos;
use bbb_registry_codec::random_source::RandomSource;

pub use bbb_server_datapack::worldgen::{
    ConfiguredFeatureHolder, Heightmap, PlacedFeature, PlacedFeatureHolder, PlacementModifier,
    CONFIGURED_FEATURE_REGISTRY, PLACED_FEATURE_REGISTRY,
};
use bbb_server_types::fluid::block_state_is_air;

use super::block_predicates::block_predicate_test;
use super::height_provider::sample_height_provider;
use super::value_providers::sample_int;
use super::{block_is, offset_block_pos, place_configured_feature, WorldGenLevel};

/// `Holder<PlacedFeature>.value().place(level, generator, random, origin)`.
pub fn place_placed_feature_holder<L: WorldGenLevel>(
    holder: &PlacedFeatureHolder,
    level: &mut L,
    random: &mut dyn RandomSource,
    origin: BlockPos,
) -> bool {
    match holder {
        PlacedFeatureHolder::Named(resource_id) => level
            .placed_feature(resource_id)
            .is_some_and(|placed| place_placed_feature(&placed, level, random, origin)),
        PlacedFeatureHolder::Inline(placed) => place_placed_feature(placed, level, random, origin),
    }
}

/// `PlacedFeature.placeWithBiomeCheck` installs this placed feature as the
/// `BiomeFilter` top feature; `top_feature` is
/// its key in the decoration plan.
pub fn place_placed_feature_with_biome_check<L: WorldGenLevel>(
    placed: &PlacedFeature,
    level: &mut L,
    random: &mut dyn RandomSource,
    origin: BlockPos,
    top_feature: &str,
) -> bool {
    place_placed_feature_with_context(placed, level, random, origin, Some(top_feature))
}

/// `PlacedFeature.place`.
pub fn place_placed_feature<L: WorldGenLevel>(
    placed: &PlacedFeature,
    level: &mut L,
    random: &mut dyn RandomSource,
    origin: BlockPos,
) -> bool {
    place_placed_feature_with_context(placed, level, random, origin, None)
}

/// `PlacedFeature.placeWithContext` (`PlacedFeature.java:43-61`): the
/// modifier `flatMap` chain is depth-first, each modifier drawing from the
/// random when its input position is expanded.
fn place_placed_feature_with_context<L: WorldGenLevel>(
    placed: &PlacedFeature,
    level: &mut L,
    random: &mut dyn RandomSource,
    origin: BlockPos,
    top_feature: Option<&str>,
) -> bool {
    let mut placed_any = false;
    place_through_modifiers(
        &placed.feature,
        &placed.placement,
        level,
        random,
        origin,
        top_feature,
        &mut placed_any,
    );
    placed_any
}

fn place_through_modifiers<L: WorldGenLevel>(
    feature: &ConfiguredFeatureHolder,
    placement: &[PlacementModifier],
    level: &mut L,
    random: &mut dyn RandomSource,
    origin: BlockPos,
    top_feature: Option<&str>,
    placed_any: &mut bool,
) {
    let Some((modifier, rest)) = placement.split_first() else {
        if place_feature_holder(feature, level, origin, random) {
            *placed_any = true;
        }
        return;
    };
    let mut emit = |position: BlockPos, level: &mut L, random: &mut dyn RandomSource| {
        place_through_modifiers(
            feature,
            rest,
            level,
            random,
            position,
            top_feature,
            placed_any,
        );
    };
    match modifier {
        // `RandomOffsetPlacement.getPositions` (`RandomOffsetPlacement.java:46-51`):
        // x, y, then z draws.
        PlacementModifier::RandomOffset {
            xz_spread,
            y_spread,
        } => {
            let x = origin.x + sample_int(xz_spread, random);
            let y = origin.y + sample_int(y_spread, random);
            let z = origin.z + sample_int(xz_spread, random);
            emit(BlockPos { x, y, z }, level, random);
        }
        // `CountPlacement.count` (`CountPlacement.java:27-29`) through
        // `RepeatingPlacement.getPositions` (`RepeatingPlacement.java:12-14`).
        PlacementModifier::Count(count) => {
            let repeats = sample_int(count, random).max(0);
            for _ in 0..repeats {
                emit(origin, level, random);
            }
        }
        // `BlockPredicateFilter.shouldPlace`.
        PlacementModifier::BlockPredicateFilter(predicate) => {
            if block_predicate_test(predicate, level, origin) == Some(true) {
                emit(origin, level, random);
            }
        }
        PlacementModifier::EnvironmentScan {
            direction,
            target_condition,
            allowed_search_condition,
            max_steps,
        } => {
            // An absent `allowed_search_condition` is `BlockPredicate.alwaysTrue()`.
            let allowed = |level: &L, pos: BlockPos| -> Option<bool> {
                match allowed_search_condition {
                    Some(condition) => block_predicate_test(condition, level, pos),
                    None => Some(true),
                }
            };
            // `EnvironmentScanPlacement.getPositions`.
            let mut pos = origin;
            if allowed(level, pos) != Some(true) {
                return;
            }
            for _ in 0..*max_steps {
                match block_predicate_test(target_condition, level, pos) {
                    Some(true) => {
                        emit(pos, level, random);
                        return;
                    }
                    Some(false) => {}
                    None => return,
                }
                pos = offset_block_pos(pos, *direction);
                if pos.y < level.get_min_y() || pos.y > level.get_max_y() {
                    return;
                }
                match allowed(level, pos) {
                    Some(true) => {}
                    Some(false) => break,
                    None => return,
                }
            }
            if block_predicate_test(target_condition, level, pos) == Some(true) {
                emit(pos, level, random);
            }
        }
        // `RarityFilter.shouldPlace`.
        PlacementModifier::Rarity(chance) => {
            if random.next_float() < 1.0 / *chance as f32 {
                emit(origin, level, random);
            }
        }
        // `SurfaceRelativeThresholdFilter.shouldPlace`.
        PlacementModifier::SurfaceRelativeThreshold {
            heightmap,
            min_inclusive,
            max_inclusive,
        } => {
            let surface_y = i64::from(heightmap_height(level, *heightmap, origin.x, origin.z));
            let y = i64::from(origin.y);
            if surface_y + i64::from(*min_inclusive) <= y
                && y <= surface_y + i64::from(*max_inclusive)
            {
                emit(origin, level, random);
            }
        }
        // `SurfaceWaterDepthFilter.shouldPlace`.
        PlacementModifier::SurfaceWaterDepth(max_water_depth) => {
            let ocean_floor = level.get_ocean_floor_height(origin.x, origin.z);
            let world_surface = level.get_world_surface_height(origin.x, origin.z);
            if i64::from(world_surface) - i64::from(ocean_floor) <= i64::from(*max_water_depth) {
                emit(origin, level, random);
            }
        }
        // `BiomeFilter.shouldPlace` throws when no
        // top feature was installed.
        PlacementModifier::Biome => {
            if let Some(top_feature) = top_feature {
                if level.biome_filter_should_place(origin, top_feature) {
                    emit(origin, level, random);
                }
            }
        }
        // `NoiseBasedCountPlacement.count`.
        PlacementModifier::NoiseBasedCount {
            noise_to_count_ratio,
            noise_factor,
            noise_offset,
        } => {
            let flower_noise = biome_info_noise(
                f64::from(origin.x) / noise_factor,
                f64::from(origin.z) / noise_factor,
            );
            let repeats =
                (((flower_noise + noise_offset) * f64::from(*noise_to_count_ratio)).ceil() as i32)
                    .max(0);
            for _ in 0..repeats {
                emit(origin, level, random);
            }
        }
        // `NoiseThresholdCountPlacement.count`.
        PlacementModifier::NoiseThresholdCount {
            noise_level,
            below_noise,
            above_noise,
        } => {
            let flower_noise =
                biome_info_noise(f64::from(origin.x) / 200.0, f64::from(origin.z) / 200.0);
            let repeats = if flower_noise < *noise_level {
                *below_noise
            } else {
                *above_noise
            }
            .max(0);
            for _ in 0..repeats {
                emit(origin, level, random);
            }
        }
        PlacementModifier::CountOnEveryLayer(count) => {
            for position in count_on_every_layer_positions(count, level, random, origin) {
                emit(position, level, random);
            }
        }
        // `HeightmapPlacement.getPositions`.
        PlacementModifier::Heightmap(heightmap) => {
            let height = heightmap_height(level, *heightmap, origin.x, origin.z);
            if height > level.get_min_y() {
                emit(
                    BlockPos {
                        x: origin.x,
                        y: height,
                        z: origin.z,
                    },
                    level,
                    random,
                );
            }
        }
        // `HeightRangePlacement.getPositions`.
        PlacementModifier::HeightRange(height) => {
            if let Some(y) = sample_height_provider(height, level, random) {
                emit(
                    BlockPos {
                        x: origin.x,
                        y,
                        z: origin.z,
                    },
                    level,
                    random,
                );
            }
        }
        // `InSquarePlacement.getPositions`.
        PlacementModifier::InSquare => {
            let x = random.next_int(16) + origin.x;
            let z = random.next_int(16) + origin.z;
            emit(BlockPos { x, y: origin.y, z }, level, random);
        }
        // `FixedPlacement.getPositions`.
        PlacementModifier::Fixed(positions) => {
            for &position in positions {
                if position.x >> 4 == origin.x >> 4 && position.z >> 4 == origin.z >> 4 {
                    emit(position, level, random);
                }
            }
        }
    }
}

/// `Holder<ConfiguredFeature>.value().place(...)` (`PlacedFeature.java:50-53`).
fn place_feature_holder<L: WorldGenLevel>(
    feature: &ConfiguredFeatureHolder,
    level: &mut L,
    pos: BlockPos,
    random: &mut dyn RandomSource,
) -> bool {
    match feature {
        ConfiguredFeatureHolder::Named(resource_id) => level
            .configured_feature(resource_id)
            .is_some_and(|feature| place_configured_feature(&feature, level, pos, random)),
        ConfiguredFeatureHolder::Inline(feature) => {
            place_configured_feature(feature, level, pos, random)
        }
    }
}

/// `CountOnEveryLayerPlacement.getPositions` (`CountOnEveryLayerPlacement.java:35-58`).
fn count_on_every_layer_positions<L: WorldGenLevel>(
    count: &bbb_server_datapack::worldgen::IntProvider,
    level: &L,
    random: &mut dyn RandomSource,
    origin: BlockPos,
) -> Vec<BlockPos> {
    let mut positions = Vec::new();
    let mut layer = 0;
    loop {
        let mut found_any = false;
        let mut i = 0;
        while i < sample_int(count, random) {
            let x = random.next_int(16) + origin.x;
            let z = random.next_int(16) + origin.z;
            let start_y = level.get_motion_blocking_height(x, z);
            let y = find_on_ground_y(level, x, start_y, z, layer);
            if y != i32::MAX {
                positions.push(BlockPos { x, y, z });
                found_any = true;
            }
            i += 1;
        }
        layer += 1;
        if !found_any {
            return positions;
        }
    }
}

/// `CountOnEveryLayerPlacement.findOnGroundYPosition` (`CountOnEveryLayerPlacement.java:65-85`).
fn find_on_ground_y<L: WorldGenLevel>(
    level: &L,
    x: i32,
    y_start: i32,
    z: i32,
    layer_to_place_on: i32,
) -> i32 {
    let mut current = level.get_block(BlockPos { x, y: y_start, z });
    let mut current_layer = 0;
    let mut y = y_start;
    while y > level.get_min_y() {
        let below_pos = BlockPos { x, y: y - 1, z };
        let below = level.get_block(below_pos);
        if !layer_is_empty(below)
            && layer_is_empty(current)
            && !block_is(below, "minecraft:bedrock")
        {
            if current_layer == layer_to_place_on {
                return below_pos.y + 1;
            }
            current_layer += 1;
        }
        current = below;
        y -= 1;
    }
    i32::MAX
}

/// `CountOnEveryLayerPlacement.isEmpty` (`CountOnEveryLayerPlacement.java:87-89`).
fn layer_is_empty(state_id: i32) -> bool {
    block_state_is_air(state_id)
        || block_is(state_id, "minecraft:water")
        || block_is(state_id, "minecraft:lava")
}

fn heightmap_height<L: WorldGenLevel>(level: &L, heightmap: Heightmap, x: i32, z: i32) -> i32 {
    match heightmap {
        Heightmap::WorldSurfaceWg => level.get_world_surface_wg_height(x, z),
        Heightmap::WorldSurface => level.get_world_surface_height(x, z),
        Heightmap::OceanFloorWg => level.get_ocean_floor_wg_height(x, z),
        Heightmap::OceanFloor => level.get_ocean_floor_height(x, z),
        Heightmap::MotionBlocking => level.get_motion_blocking_height(x, z),
        Heightmap::MotionBlockingNoLeaves => level.get_motion_blocking_no_leaves_height(x, z),
    }
}

/// `Biome.BIOME_INFO_NOISE.getValue(x, z, false)` (`Biome.java:66`,
/// `PerlinSimplexNoise.java:64-78`) for octave set `{0}`.
fn biome_info_noise(x: f64, z: f64) -> f64 {
    static NOISE: OnceLock<crate::noise::SimplexNoise> = OnceLock::new();
    NOISE
        .get_or_init(|| crate::noise::SimplexNoise::new(&mut WorldgenRandom::with_seed(2345)))
        .get_value(x, z)
}
