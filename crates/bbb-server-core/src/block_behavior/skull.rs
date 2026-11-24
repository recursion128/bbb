use super::LevelCtx;
use bbb_protocol::block_types::{vanilla_block_type, VanillaNeighborChangedOverride};
use bbb_protocol::packets::BlockPos;

use crate::fluid::with_property;

use super::block_type_of;
use super::{bool_property, BlockPlacedBy, SET_BLOCK_UPDATE_CLIENTS};

const POWERED: &str = "powered";

/// `AbstractSkullBlock.neighborChanged` is unique to that class.
#[must_use]
pub fn is_skull(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.neighbor_changed_override)
        == Some(VanillaNeighborChangedOverride::AbstractSkullBlock)
}

#[must_use]
fn powered(block_state_id: i32) -> bool {
    bool_property(block_state_id, POWERED).unwrap_or(false)
}

pub(super) fn set_placed_by(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    placed_by: &BlockPlacedBy<'_>,
) {
    super::carved_pumpkin::wither_check_spawn(level, pos, block_state_id, placed_by.peaceful);
}

pub(super) fn neighbor_changed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    check_powered_state(level, pos, block_state_id);
}

fn check_powered_state(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let signal = level.has_neighbor_signal(pos);
    if powered(block_state_id) == signal {
        return;
    }
    let Some(next) = with_property(
        block_state_id,
        POWERED,
        if signal { "true" } else { "false" },
    ) else {
        return;
    };
    // `:79`: flag 2 is `UPDATE_CLIENTS`.
    level.set_block(
        pos,
        next,
        SET_BLOCK_UPDATE_CLIENTS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}
