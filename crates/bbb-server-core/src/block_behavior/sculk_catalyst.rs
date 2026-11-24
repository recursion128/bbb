//! The Sculk Catalyst's scheduled bloom reset.

use super::LevelCtx;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use bbb_protocol::packets::BlockPos;

use super::bool_property;

const BLOOM: &str = "bloom";

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if bool_property(block_state_id, BLOOM) != Some(true) {
        return;
    }
    if let Some(dormant) = with_property(block_state_id, BLOOM, "false") {
        level.set_block(
            pos,
            dormant,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}
