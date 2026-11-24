//! `NyliumBlock` (`net/minecraft/world/level/block/NyliumBlock.java`).
//!
//! The nether's answer to `SpreadingSnowyBlock`, minus the spread: nylium only
//! ever dies back, and only because light stopped getting in from above.

use super::LevelCtx;
use bbb_protocol::block_types::VANILLA_BLOCK_NETHERRACK_ID;
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{above, SET_BLOCK_UPDATE_ALL};

use super::{default_block_state_id, light_dampening};

/// The opacity `canBeNylium` demands light still get through under
/// (`NyliumBlock.java:36`).
const OPACITY_CUTOFF: i32 = 15;

fn can_be_nylium(level: &LevelCtx<'_, '_>, block_state_id: i32, pos: BlockPos) -> bool {
    let above_pos = above(pos);
    let above_state = level.block_state(above_pos);
    let opacity = level.light_block_into(
        block_state_id,
        above_state,
        Direction::Up,
        light_dampening(above_state),
    );
    opacity < OPACITY_CUTOFF
}

pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if can_be_nylium(level, block_state_id, pos) {
        return;
    }
    level.set_block(
        pos,
        default_block_state_id(VANILLA_BLOCK_NETHERRACK_ID),
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}
