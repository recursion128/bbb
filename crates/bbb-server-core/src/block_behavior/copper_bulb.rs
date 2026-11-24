use super::LevelCtx;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{vanilla_block_type, VanillaNeighborChangedOverride};
use bbb_protocol::packets::BlockPos;

use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};

use super::block_type_of;
use super::{bool_property, weathering};

const POWERED: &str = "powered";
const LIT: &str = "lit";

/// `SoundEvents.COPPER_BULB_TURN_ON` (`SoundEvents.java:398`).
pub const TURN_ON_SOUND: &str = "minecraft:block.copper_bulb.turn_on";
/// `SoundEvents.COPPER_BULB_TURN_OFF` (`SoundEvents.java:399`).
pub const TURN_OFF_SOUND: &str = "minecraft:block.copper_bulb.turn_off";

/// `CopperBulbBlock.neighborChanged` is unique to that class, including the
/// four waxed copies that do not oxidize.
#[must_use]
pub fn is_copper_bulb(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.neighbor_changed_override)
        == Some(VanillaNeighborChangedOverride::CopperBulbBlock)
}

#[must_use]
fn powered(block_state_id: i32) -> bool {
    bool_property(block_state_id, POWERED).unwrap_or(false)
}

#[must_use]
pub fn lit(block_state_id: i32) -> bool {
    bool_property(block_state_id, LIT).unwrap_or(false)
}

#[must_use]
pub fn analog_output(block_state_id: i32) -> i32 {
    if lit(block_state_id) {
        15
    } else {
        0
    }
}

pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if weathering::age(block_state_id).is_some() {
        weathering::random_tick(level, pos, block_state_id);
    }
}

pub(super) fn neighbor_changed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    check_and_flip(level, pos, block_state_id);
}

pub(super) fn on_place(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    old_state: i32,
) {
    if vanilla_block_state_block_type_id(old_state)
        != vanilla_block_state_block_type_id(block_state_id)
    {
        check_and_flip(level, pos, block_state_id);
    }
}

fn check_and_flip(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    check_and_flip_inner(level, pos, block_state_id);
}

fn check_and_flip_inner(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let signal = level.has_neighbor_signal(pos);
    if powered(block_state_id) == signal {
        return;
    }
    let mut next = block_state_id;
    if !powered(block_state_id) {
        let lit_now = !lit(block_state_id);
        if let Some(cycled) = with_property(next, LIT, if lit_now { "true" } else { "false" }) {
            next = cycled;
        }
        level.play_block_sound(
            pos,
            if lit(next) {
                TURN_ON_SOUND
            } else {
                TURN_OFF_SOUND
            },
        );
    }
    let Some(next) = with_property(next, POWERED, if signal { "true" } else { "false" }) else {
        return;
    };
    // `:59`: flag 3 is `UPDATE_ALL`.
    level.set_block(
        pos,
        next,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}
