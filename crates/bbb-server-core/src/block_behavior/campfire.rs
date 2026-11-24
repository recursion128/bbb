//! `CampfireBlock` (`CampfireBlock.java`) lighting and shovel dousing.

use super::{BlockShapeCtx, LevelCtx};
use bbb_protocol::block_types::{vanilla_block_type, VanillaBlockUseItemOnOverride};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::chunk_ticks::{FluidTickType, TickPriority};
use bbb_server_types::fluid::FluidFamily;

use crate::fluid::{with_property, FlowingFluid, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;

use super::block_type_of;
use super::bool_property;
use super::level::SET_BLOCK_UPDATE_IMMEDIATE;

const LIT: &str = "lit";
const WATERLOGGED: &str = "waterlogged";

/// `CampfireBlock.updateShape` (`CampfireBlock.java:139-155`).
pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction: Direction,
    neighbour_state: i32,
) -> i32 {
    if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
    if direction == Direction::Down {
        let smoke_source = block_type_of(neighbour_state)
            == Some(bbb_protocol::block_types::VANILLA_BLOCK_HAY_BLOCK_ID);
        with_property(
            block_state_id,
            "signal_fire",
            if smoke_source { "true" } else { "false" },
        )
        .unwrap_or(block_state_id)
    } else {
        block_state_id
    }
}
/// `Block.UPDATE_ALL_IMMEDIATE` (`Block.java:106`).
const UPDATE_ALL_IMMEDIATE: i32 = SET_BLOCK_UPDATE_ALL | SET_BLOCK_UPDATE_IMMEDIATE;
/// `LevelEvent.SOUND_EXTINGUISH_FIRE` (`LevelEvent.java:8`).
pub const SOUND_EXTINGUISH_FIRE: i32 = 1009;

/// `CampfireBlock.useItemOn` is unique to that class: campfire and soul campfire.
#[must_use]
pub fn is_campfire(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.use_item_on_override)
        == Some(VanillaBlockUseItemOnOverride::CampfireBlock)
}

/// `CampfireBlock.LIT` on a stored campfire.
#[must_use]
pub fn is_lit(block_state_id: i32) -> bool {
    bool_property(block_state_id, LIT) == Some(true)
}

#[must_use]
pub fn can_light(block_state_id: i32) -> bool {
    is_campfire(block_state_id)
        && bool_property(block_state_id, LIT) == Some(false)
        && bool_property(block_state_id, WATERLOGGED) == Some(false)
}

pub(crate) fn dowse(level: &mut LevelCtx<'_, '_>, pos: BlockPos, source_entity_id: Option<i32>) {
    level.game_event_from_entity(VanillaGameEvent::BLOCK_CHANGE, pos, source_entity_id, None);
}

pub(crate) fn shovel_dowse(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    source_entity_id: Option<i32>,
) -> bool {
    if !is_lit(block_state_id) {
        return false;
    }
    let Some(unlit) = with_property(block_state_id, LIT, "false") else {
        return false;
    };
    level.level_event(SOUND_EXTINGUISH_FIRE, pos, 0);
    dowse(level, pos, source_entity_id);
    level.set_block(
        pos,
        unlit,
        UPDATE_ALL_IMMEDIATE,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.game_event_from_entity(
        VanillaGameEvent::BLOCK_CHANGE,
        pos,
        source_entity_id,
        Some(unlit),
    );
    true
}

/// `CampfireBlock.entityInside`: the fire damage a living body takes, if any.
pub fn inside_damage(state: i32, is_living: bool) -> Option<Option<f32>> {
    use bbb_protocol::block_types::{VANILLA_BLOCK_CAMPFIRE_ID, VANILLA_BLOCK_SOUL_CAMPFIRE_ID};
    let damage = match block_type_of(state)? {
        VANILLA_BLOCK_CAMPFIRE_ID => 1.0,
        VANILLA_BLOCK_SOUL_CAMPFIRE_ID => 2.0,
        _ => return None,
    };
    Some((bool_property(state, LIT)? && is_living).then_some(damage))
}

#[cfg(test)]
mod tests;

/// `CampfireBlock.onProjectileHit` (`CampfireBlock.java:223-232`) once the
/// projectile burns and may interact.
pub(super) fn light_by_projectile(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    if !can_light(block_state_id) {
        return;
    }
    if let Some(lit) = with_property(block_state_id, LIT, "true") {
        level.set_block(
            pos,
            lit,
            UPDATE_ALL_IMMEDIATE,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}

/// `CampfireBlock.isSmokeyPos` (`CampfireBlock.java:260-280`).
pub(crate) fn is_smokey_pos<E>(
    pos: BlockPos,
    mut state_at: impl FnMut(BlockPos) -> Result<i32, E>,
    mut blocked: impl FnMut(i32, BlockPos) -> Result<bool, E>,
    mut campfire_tag: impl FnMut(&str) -> Result<bool, E>,
) -> Result<bool, E> {
    for distance in 1..=5 {
        let below = BlockPos {
            y: pos.y - distance,
            ..pos
        };
        let state = state_at(below)?;
        if is_lit_campfire(state, &mut campfire_tag)? {
            return Ok(true);
        }
        if blocked(state, below)? {
            let state = state_at(BlockPos {
                y: below.y - 1,
                ..below
            })?;
            return is_lit_campfire(state, &mut campfire_tag);
        }
    }
    Ok(false)
}

/// `CampfireBlock.SHAPE_VIRTUAL_POST` (`CampfireBlock.java:70`).
pub(crate) fn smoke_post_intersects(bounds: bbb_server_types::geometry::Aabb) -> bool {
    bounds.max_x > 6.0 / 16.0
        && bounds.min_x < 10.0 / 16.0
        && bounds.max_y > 0.0
        && bounds.min_y < 1.0
        && bounds.max_z > 6.0 / 16.0
        && bounds.min_z < 10.0 / 16.0
}

/// `CampfireBlock.isLitCampfire` (`CampfireBlock.java:278-280`).
fn is_lit_campfire<E>(
    state: i32,
    tag: &mut impl FnMut(&str) -> Result<bool, E>,
) -> Result<bool, E> {
    if !is_lit(state) {
        return Ok(false);
    }
    let Some(name) = bbb_protocol::block_state_types::vanilla_block_state_name(state) else {
        return Ok(false);
    };
    tag(name)
}
