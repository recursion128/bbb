//! Vanilla `RootSystemFeature` (`Feature.ROOT_SYSTEM`, `Feature.java:92`): a
//! nested tree placed up a column with rooted dirt below and hanging roots
//! around it.

use bbb_protocol::block_state_types::vanilla_block_state_support_facts;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_registry_codec::random_source::RandomSource;
use bbb_server_datapack::worldgen::{HolderSet, RootSystemConfig};

use bbb_server_types::fluid::block_state_fluid_state;

use super::block_predicates::{block_predicate_test, fluid_in_holder_set};
use super::multiface::is_face_sturdy_at;
use super::placed_feature::place_placed_feature_holder;
use super::state_providers::BlockStateProviderExt;
use super::{
    block_is, feature_block_can_survive, is_empty_block, offset_block_pos, WorldGenLevel,
    UPDATE_CLIENTS,
};

/// Vanilla `RootSystemFeature.place` (`RootSystemFeature.java:20-36`).
pub fn place_root_system<L: WorldGenLevel>(
    config: &RootSystemConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    if !is_empty_block(level.get_block(origin)) {
        return Some(false);
    }
    if place_dirt_and_tree(level, config, random, origin)? {
        place_roots(level, config, random, origin)?;
    }
    Some(true)
}

/// Vanilla `RootSystemFeature.spaceForTree` (`RootSystemFeature.java:38-50`)
/// over `isAllowedTreeSpace` (`:52-59`).
fn space_for_tree<L: WorldGenLevel>(
    level: &L,
    config: &RootSystemConfig,
    pos: BlockPos,
) -> Option<bool> {
    let mut column_up_pos = pos;
    for i in 1..=config.required_vertical_space_for_tree {
        column_up_pos = offset_block_pos(column_up_pos, Direction::Up);
        let state = level.get_block(column_up_pos);
        if !is_allowed_tree_space(level, state, i, config.allowed_vertical_water_for_tree)? {
            return Some(false);
        }
    }
    Some(true)
}

fn is_allowed_tree_space<L: WorldGenLevel>(
    level: &L,
    state: i32,
    blocks_above_origin: i32,
    allowed_vertical_water_height: i32,
) -> Option<bool> {
    if is_empty_block(state) {
        return Some(true);
    }
    let blocks_above_ground = blocks_above_origin + 1;
    Some(
        blocks_above_ground <= allowed_vertical_water_height
            && fluid_in_holder_set(
                level,
                block_state_fluid_state(state),
                &HolderSet::Tag("minecraft:water".to_string()),
            )?,
    )
}

/// Vanilla `RootSystemFeature.placeDirtAndTree` (`RootSystemFeature.java:61-85`).
fn place_dirt_and_tree<L: WorldGenLevel>(
    level: &mut L,
    config: &RootSystemConfig,
    random: &mut dyn RandomSource,
    pos: BlockPos,
) -> Option<bool> {
    let mut working_pos = pos;
    for y in 0..config.root_column_max_height {
        working_pos = offset_block_pos(working_pos, Direction::Up);
        if block_predicate_test(&config.allowed_tree_position, level, working_pos)?
            && space_for_tree(level, config, working_pos)?
        {
            let below_pos = offset_block_pos(working_pos, Direction::Down);
            let below_state = level.get_block(below_pos);
            let below_facts = vanilla_block_state_support_facts(below_state)?;
            if fluid_in_holder_set(
                level,
                block_state_fluid_state(below_state),
                &HolderSet::Tag("minecraft:lava".to_string()),
            )? || !below_facts.legacy_solid()
            {
                return Some(false);
            }
            if place_placed_feature_holder(&config.feature, level, random, working_pos) {
                place_dirt(pos, pos.y + y, level, config, random)?;
                return Some(true);
            }
        }
    }
    Some(false)
}

/// Vanilla `RootSystemFeature.placeDirt` (`RootSystemFeature.java:87-97`).
fn place_dirt<L: WorldGenLevel>(
    origin: BlockPos,
    target_height: i32,
    level: &mut L,
    config: &RootSystemConfig,
    random: &mut dyn RandomSource,
) -> Option<()> {
    for y in origin.y..target_height {
        place_rooted_dirt(
            level,
            config,
            random,
            BlockPos {
                x: origin.x,
                y,
                z: origin.z,
            },
        )?;
    }
    Some(())
}

/// Vanilla `RootSystemFeature.placeRootedDirt` (`RootSystemFeature.java:99-121`).
fn place_rooted_dirt<L: WorldGenLevel>(
    level: &mut L,
    config: &RootSystemConfig,
    random: &mut dyn RandomSource,
    column_pos: BlockPos,
) -> Option<()> {
    let root_radius = config.root_radius;
    for _ in 0..config.root_placement_attempts {
        let dx = random.next_int(root_radius) - random.next_int(root_radius);
        let dz = random.next_int(root_radius) - random.next_int(root_radius);
        let working_pos = BlockPos {
            x: column_pos.x + dx,
            y: column_pos.y,
            z: column_pos.z + dz,
        };
        let working_state = level.get_block(working_pos);
        if level.block_state_in_tag(working_state, &config.root_replaceable) {
            let state = config
                .root_state_provider
                .get_state(level, random, working_pos)?;
            let _ = level.set_block(working_pos, state, UPDATE_CLIENTS);
        }
    }
    Some(())
}

/// Vanilla `RootSystemFeature.placeRoots` (`RootSystemFeature.java:123-143`).
fn place_roots<L: WorldGenLevel>(
    level: &mut L,
    config: &RootSystemConfig,
    random: &mut dyn RandomSource,
    pos: BlockPos,
) -> Option<()> {
    let root_radius = config.hanging_root_radius;
    let vertical_span = config.hanging_roots_vertical_span;
    for _ in 0..config.hanging_root_placement_attempts {
        let dx = random.next_int(root_radius) - random.next_int(root_radius);
        let dy = random.next_int(vertical_span) - random.next_int(vertical_span);
        let dz = random.next_int(root_radius) - random.next_int(root_radius);
        let working_pos = BlockPos {
            x: pos.x + dx,
            y: pos.y + dy,
            z: pos.z + dz,
        };
        if is_empty_block(level.get_block(working_pos)) {
            let target_state =
                config
                    .hanging_root_state_provider
                    .get_state(level, random, working_pos)?;
            let above = level.get_block(offset_block_pos(working_pos, Direction::Up));
            if can_survive(level, target_state, working_pos)?
                && is_face_sturdy_at(
                    level,
                    offset_block_pos(working_pos, Direction::Up),
                    above,
                    Direction::Down,
                )
            {
                let _ = level.set_block(working_pos, target_state, UPDATE_CLIENTS);
            }
        }
    }
    Some(())
}

/// `BlockState.canSurvive` with `HangingRootsBlock.canSurvive`
/// (`HangingRootsBlock.java:59-63`) spelled out, since that block has no
/// behaviour owner in the shared `can_survive` table.
fn can_survive<L: WorldGenLevel>(level: &L, state: i32, pos: BlockPos) -> Option<bool> {
    if block_is(state, "minecraft:hanging_roots") {
        let attached_to = level.get_block(offset_block_pos(pos, Direction::Up));
        let attached_pos = offset_block_pos(pos, Direction::Up);
        return Some(is_face_sturdy_at(
            level,
            attached_pos,
            attached_to,
            Direction::Down,
        ));
    }
    Some(feature_block_can_survive(level, pos, state))
}
