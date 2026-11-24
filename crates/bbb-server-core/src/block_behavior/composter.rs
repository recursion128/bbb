//!
//! Hopper `WorldlyContainer` faces stay outside this slice.

use super::LevelCtx;
use bbb_protocol::block_types::{vanilla_block_type, VanillaBlockUseItemOnOverride};
use bbb_protocol::item_types::vanilla_compost_chance_bits;
use bbb_protocol::packets::BlockPos;

use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};

use super::block_type_of;
use super::int_property;

const LEVEL: &str = "level";
/// `ComposterBlock.MAX_LEVEL` (`ComposterBlock.java:48`).
pub const MAX_LEVEL: i32 = 7;
/// `ComposterBlock.READY` (`ComposterBlock.java:46`).
pub const READY: i32 = 8;
/// `ComposterBlock.onPlace` / `addItem` delay (`ComposterBlock.java:243`, `:332`).
pub const READY_DELAY: i32 = 20;
/// `LevelEvent.COMPOSTER_FILL` (`LevelEvent.java:46`).
pub const FILL_LEVEL_EVENT: i32 = 1500;
/// `SoundEvents.COMPOSTER_READY` (`SoundEvents.java:387`).
pub const READY_SOUND: &str = "minecraft:block.composter.ready";
/// `SoundEvents.COMPOSTER_EMPTY` (`SoundEvents.java:384`).
pub const EMPTY_SOUND: &str = "minecraft:block.composter.empty";

/// `ComposterBlock.useItemOn` is unique to that class.
#[must_use]
pub fn is_composter(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.use_item_on_override)
        == Some(VanillaBlockUseItemOnOverride::ComposterBlock)
}

#[must_use]
pub fn analog_output(block_state_id: i32) -> i32 {
    fill_level(block_state_id)
}

#[must_use]
pub fn fill_level(block_state_id: i32) -> i32 {
    int_property(block_state_id, LEVEL).unwrap_or(0)
}

#[must_use]
pub fn is_compostable(item_type_id: Option<i32>) -> bool {
    item_type_id.is_some_and(|id| vanilla_compost_chance_bits(id).is_some())
}

#[must_use]
pub fn compost_chance(item_type_id: i32) -> Option<f32> {
    vanilla_compost_chance_bits(item_type_id).map(f32::from_bits)
}

///
/// Returns the state after the attempt, which may be unchanged.
pub(crate) fn add_item(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    item_type_id: i32,
    source_entity_id: Option<i32>,
) -> i32 {
    let fill = fill_level(block_state_id);
    let Some(chance) = compost_chance(item_type_id) else {
        return block_state_id;
    };
    let first_fill = fill == 0 && chance > 0.0;
    if !first_fill && level.level_random().next_double() >= f64::from(chance) {
        return block_state_id;
    }
    let new_level = fill + 1;
    let Some(next) = with_property(block_state_id, LEVEL, &new_level.to_string()) else {
        return block_state_id;
    };
    level.set_block(
        pos,
        next,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.game_event_from_entity(
        VanillaGameEvent::BLOCK_CHANGE,
        pos,
        source_entity_id,
        Some(next),
    );
    if new_level == MAX_LEVEL {
        schedule_ready(level, pos, next);
    }
    next
}

pub(crate) fn empty(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    source_entity_id: Option<i32>,
) -> i32 {
    let Some(next) = with_property(block_state_id, LEVEL, "0") else {
        return block_state_id;
    };
    level.set_block(
        pos,
        next,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.game_event_from_entity(
        VanillaGameEvent::BLOCK_CHANGE,
        pos,
        source_entity_id,
        Some(next),
    );
    next
}

pub(super) fn on_place(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if fill_level(block_state_id) == MAX_LEVEL {
        schedule_ready(level, pos, block_state_id);
    }
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if fill_level(block_state_id) != MAX_LEVEL {
        return;
    }
    let Some(next) = with_property(block_state_id, LEVEL, &READY.to_string()) else {
        return;
    };
    level.set_block(
        pos,
        next,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.play_block_sound(pos, READY_SOUND);
}

fn schedule_ready(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(block) = BlockTickType::of_block_state(block_state_id) else {
        return;
    };
    level.schedule_block_tick(pos, block, READY_DELAY, TickPriority::Normal);
}
