//! `TntBlock`'s redstone-triggered priming transaction.

use super::LevelCtx;
use bbb_protocol::packets::BlockPos;

use super::block_type_of;

fn prime_from_signal(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    let powered = level.has_neighbor_signal(pos);
    if powered {
        let _ = level.prime_tnt_and_remove(pos);
    }
}

pub(super) fn on_place(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32, old_state: i32) {
    let Some(block) = block_type_of(state) else {
        return;
    };
    if block_type_of(old_state) == Some(block) {
        return;
    }
    prime_from_signal(level, pos);
}

pub(super) fn neighbor_changed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, _state: i32) {
    prime_from_signal(level, pos);
}
