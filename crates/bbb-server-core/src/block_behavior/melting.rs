//! The two light-driven random ticks:
//! `SnowLayerBlock` (`net/minecraft/world/level/block/SnowLayerBlock.java`)
//! and `IceBlock` (`IceBlock.java`).
//!
//!
//! # This server has no light engine
//!
//!
//! # `FrostedIceBlock` is beside them, not under them
//!
//! It extends `IceBlock` but declares no `randomTick` at all: its melt is a
//! *scheduled* tick that reads its own `AGE` and how many of its own kind are
//! beside it, so it has its own bodies here and never reaches the plain ice
//! one.

use super::LevelCtx;
use bbb_protocol::block_types::{vanilla_block_type, VanillaTickOverride, VANILLA_BLOCK_WATER_ID};
use bbb_protocol::packets::BlockPos;

use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::directions::relative;

use super::{
    block_type_of, default_block_state_id, int_property, light_dampening, LightLayer,
    DIRECTION_VALUES, SET_BLOCK_UPDATE_CLIENTS,
};

/// The block-light level `SnowLayerBlock.randomTick` melts above
/// (`SnowLayerBlock.java:106`, `> 11`).
pub const SNOW_MELT_BRIGHTNESS: i32 = 11;

/// The same constant inside `IceBlock.randomTick` (`IceBlock.java:60`), where
/// it is reduced by the ice's own `getLightDampening()` before the comparison,
/// so thicker-blocking ice melts at a lower reading.
pub const ICE_MELT_BRIGHTNESS: i32 = 11;

#[must_use]
pub fn ice_melts_into() -> i32 {
    default_block_state_id(VANILLA_BLOCK_WATER_ID)
}

///
/// The whole layer goes at once regardless of how many `LAYERS` it had, and it
/// drops the snowballs for all of them, because `dropResources` is handed the
/// state before the removal.
pub(super) fn snow_layer_random_tick(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    if level.brightness(LightLayer::Block, pos) > SNOW_MELT_BRIGHTNESS {
        level.drop_resources(pos, block_state_id);
        level.remove_block(pos);
    }
}

pub(super) fn ice_random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if level.brightness(LightLayer::Block, pos)
        > ICE_MELT_BRIGHTNESS - light_dampening(block_state_id)
    {
        melt(level, pos);
    }
}

///
/// In a dimension where water evaporates the ice simply disappears; anywhere
/// else it becomes a water source and then notifies itself, which is what lets
/// the fresh water start spreading in the same tick
/// ([`crate::fluid::liquid_block_neighbor_changed`]).
pub(super) fn melt(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    if level.water_evaporates(pos) {
        level.remove_block(pos);
        return;
    }
    let water = ice_melts_into();
    level.set_block(
        pos,
        water,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.neighbor_changed(pos, VANILLA_BLOCK_WATER_ID);
}

// ---------------------------------------------------------------------------
// `FrostedIceBlock`
// ---------------------------------------------------------------------------

/// `FrostedIceBlock.AGE` (`FrostedIceBlock.java:25`) and its `MAX_AGE`
/// (`:24`).
const AGE: &str = "age";
const MAX_AGE: i32 = 3;

/// `FrostedIceBlock.NEIGHBORS_TO_AGE` and `NEIGHBORS_TO_MELT`
/// (`FrostedIceBlock.java:26-27`).
const NEIGHBORS_TO_AGE: i32 = 4;
const NEIGHBORS_TO_MELT: i32 = 2;

/// The two `Mth.nextInt` ranges the class schedules with
/// (`FrostedIceBlock.java:41`, `:55`, `:63`), inclusive at both ends
/// (`Mth.java:145-147`).
const FIRST_TICK_MIN: i32 = 60;
const FIRST_TICK_MAX: i32 = 120;
const RETRY_TICK_MIN: i32 = 20;
const RETRY_TICK_MAX: i32 = 40;

#[must_use]
pub fn is_frosted_ice(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.tick_override)
        == Some(VanillaTickOverride::FrostedIceBlock)
}

fn next_int_between(level: &mut LevelCtx<'_, '_>, min: i32, max: i32) -> i32 {
    level.level_random().next_int(max - min + 1) + min
}

fn schedule(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32, delay: i32) {
    if let Some(block) = BlockTickType::of_block_state(block_state_id) {
        level.schedule_block_tick(pos, block, delay, TickPriority::Normal);
    }
}

pub(super) fn frosted_ice_on_place(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    let delay = next_int_between(level, FIRST_TICK_MIN, FIRST_TICK_MAX);
    schedule(level, pos, block_state_id, delay);
}

fn fewer_neighbours_than(level: &LevelCtx<'_, '_>, pos: BlockPos, limit: i32) -> bool {
    let mut result = 0;
    for direction in DIRECTION_VALUES {
        if is_frosted_ice(level.block_state(relative(pos, direction))) {
            result += 1;
            if result >= limit {
                return false;
            }
        }
    }
    true
}

fn slightly_melt(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) -> bool {
    let age = int_property(block_state_id, AGE).unwrap_or(MAX_AGE);
    if age < MAX_AGE {
        if let Some(older) = with_property(block_state_id, AGE, &(age + 1).to_string()) {
            level.set_block(
                pos,
                older,
                SET_BLOCK_UPDATE_CLIENTS,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        }
        return false;
    }
    melt(level, pos);
    true
}

///
/// The light read is `getMaxLocalRawBrightness` for every dimension: vanilla
/// takes block light alone in the End (`:47`), and the two agree there because
/// a dimension with no sky light reports zero for it while `getSkyDarken` is
/// never negative, so the sky term can never win the `max`.
pub(super) fn frosted_ice_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let melting_turn = level.level_random().next_int(3) == 0
        || fewer_neighbours_than(level, pos, NEIGHBORS_TO_AGE);
    if melting_turn {
        let brightness = level.max_block_brightness(pos);
        let threshold = ICE_MELT_BRIGHTNESS
            - int_property(block_state_id, AGE).unwrap_or(0)
            - light_dampening(block_state_id);
        if brightness > threshold && slightly_melt(level, pos, block_state_id) {
            for direction in DIRECTION_VALUES {
                let neighbour_pos = relative(pos, direction);
                let neighbour = level.block_state(neighbour_pos);
                if is_frosted_ice(neighbour) && !slightly_melt(level, neighbour_pos, neighbour) {
                    let delay = next_int_between(level, RETRY_TICK_MIN, RETRY_TICK_MAX);
                    schedule(level, neighbour_pos, neighbour, delay);
                }
            }
            return;
        }
    }
    let delay = next_int_between(level, RETRY_TICK_MIN, RETRY_TICK_MAX);
    schedule(level, pos, block_state_id, delay);
}

pub(super) fn frosted_ice_neighbor_changed(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    changed_block_type_id: i32,
) {
    if block_type_of(block_state_id) != Some(changed_block_type_id) {
        return;
    }
    if fewer_neighbours_than(level, pos, NEIGHBORS_TO_MELT) {
        melt(level, pos);
    }
}
