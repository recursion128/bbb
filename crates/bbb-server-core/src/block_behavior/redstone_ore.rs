//! Redstone ore interaction and random-tick state transitions.

use super::LevelCtx;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use bbb_protocol::block_types::VanillaRandomTickOverride;
use bbb_protocol::packets::BlockPos;

use super::{block_type_of, random_tick_override};

/// `RedStoneOreBlock.LIT` (`RedStoneOreBlock.java:26`), which is
/// `RedstoneTorchBlock.LIT`, i.e. `BlockStateProperties.LIT`.
const LIT: &str = "lit";

/// Whether this slice owns a stored state's random tick.
///
#[must_use]
pub fn is_redstone_ore(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(random_tick_override)
        == Some(VanillaRandomTickOverride::RedStoneOreBlock)
}

#[must_use]
pub fn lit_state(block_state_id: i32) -> Option<i32> {
    if super::bool_property(block_state_id, LIT) == Some(true) {
        return Some(block_state_id);
    }
    with_property(block_state_id, LIT, "true")
}

pub(crate) fn interact(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) -> Option<()> {
    if !is_redstone_ore(state) {
        return None;
    }
    let lit = lit_state(state)?;
    if !super::bool_property(state, LIT)? {
        level.set_block(
            pos,
            lit,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
    Some(())
}

///
/// The write is the three-argument `setBlock` with flag 3 spelled out on
/// `:89`, i.e. the same neighbour-and-client update `setBlockAndUpdate`
/// performs.
pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if super::bool_property(block_state_id, LIT) != Some(true) {
        return;
    }
    let Some(unlit) = with_property(block_state_id, LIT, "false") else {
        return;
    };
    level.set_block(
        pos,
        unlit,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}
