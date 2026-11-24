//! `CarpetBlock` (`net/minecraft/world/level/block/CarpetBlock.java`).
//!
//!
//!
//! # What is not here
//!

use super::BlockShapeCtx;
use bbb_protocol::block_types::VanillaCanSurviveOverride;
use bbb_protocol::packets::BlockPos;

use crate::fluid::below;
use bbb_server_types::fluid::block_state_is_air;

use super::{block_type_of, can_survive_override, BlockStateAt};

/// Whether this slice owns a stored state's survival.
///
#[must_use]
pub fn is_carpet(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(can_survive_override)
        == Some(VanillaCanSurviveOverride::CarpetBlock)
}

#[must_use]
pub fn can_survive(pos: BlockPos, block_state_at: BlockStateAt<'_>) -> bool {
    !block_state_is_air(block_state_at(below(pos)))
}

pub(super) fn update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    if can_survive(pos, &mut |at| level.block_state(at)) {
        block_state_id
    } else {
        super::air_block_state_id()
    }
}
