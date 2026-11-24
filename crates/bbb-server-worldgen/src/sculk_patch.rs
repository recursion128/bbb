//! Vanilla `SculkPatchFeature` (`Feature.SCULK_PATCH`, `Feature.java:154`):
//! the deep-dark sculk patch grown by a world-generation `SculkSpreader`.

use bbb_protocol::packets::{BlockPos, Direction};
use bbb_registry_codec::random_source::RandomSource;
use bbb_server_datapack::worldgen::SculkPatchConfig;

use super::multiface::{
    is_collision_shape_full_block_at, is_face_sturdy_at, is_source_water, DIRECTIONS,
};
use super::sculk_spreader::SculkSpreader;
use super::value_providers::sample_int;
use super::{
    block_is, block_set_bool, catalog_default_state, is_empty_block, offset_block_pos,
    WorldGenLevel, FEATURE_SET_BLOCK,
};

/// Vanilla `SculkPatchFeature.place` (`SculkPatchFeature.java:22-63`).
pub fn place_sculk_patch<L: WorldGenLevel>(
    config: &SculkPatchConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    if !can_spread_from(level, origin) {
        return Some(false);
    }
    let mut spreader = SculkSpreader::create_world_gen_spreader();
    let total_rounds = config.spread_rounds + config.growth_rounds;
    for round in 0..total_rounds {
        for _ in 0..config.charge_count {
            spreader.add_cursors(origin, config.amount_per_charge);
        }
        let spread_veins = round < config.spread_rounds;
        for _ in 0..config.spread_attempts {
            spreader.update_cursors(level, origin, random, spread_veins)?;
        }
        spreader.clear();
    }
    let below = offset_block_pos(origin, Direction::Down);
    if random.next_float() <= config.catalyst_chance
        && is_collision_shape_full_block_at(level, below, level.get_block(below))
    {
        let _ = level.set_block(
            origin,
            catalog_default_state("minecraft:sculk_catalyst"),
            FEATURE_SET_BLOCK,
        );
    }
    let extra_growths = sample_int(&config.extra_rare_growths, random);
    for _ in 0..extra_growths {
        let dx = random.next_int(5) - 2;
        let dz = random.next_int(5) - 2;
        let candidate = BlockPos {
            x: origin.x + dx,
            y: origin.y,
            z: origin.z + dz,
        };
        let candidate_below = offset_block_pos(candidate, Direction::Down);
        if is_empty_block(level.get_block(candidate))
            && is_face_sturdy_at(
                level,
                candidate_below,
                level.get_block(candidate_below),
                Direction::Up,
            )
        {
            let shrieker = block_set_bool(
                catalog_default_state("minecraft:sculk_shrieker"),
                "can_summon",
                true,
            );
            let _ = level.set_block(candidate, shrieker, FEATURE_SET_BLOCK);
        }
    }
    Some(true)
}

/// Vanilla `SculkPatchFeature.canSpreadFrom` (`SculkPatchFeature.java:65-74`);
/// the `SculkBehaviour` blocks are `SculkBlock` and `SculkVeinBlock`.
fn can_spread_from<L: WorldGenLevel>(level: &L, origin: BlockPos) -> bool {
    let start = level.get_block(origin);
    if block_is(start, "minecraft:sculk") || block_is(start, "minecraft:sculk_vein") {
        return true;
    }
    if !is_empty_block(start) && (!block_is(start, "minecraft:water") || !is_source_water(start)) {
        return false;
    }
    for direction in DIRECTIONS {
        let pos = offset_block_pos(origin, direction);
        if is_collision_shape_full_block_at(level, pos, level.get_block(pos)) {
            return true;
        }
    }
    false
}
