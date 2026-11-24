use super::{BlockShapeCtx, LevelCtx};
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use bbb_protocol::block_types::{vanilla_block_type, VanillaOnPlaceOverride};
use bbb_protocol::packets::BlockPos;
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};

use super::{below, block_type_of, falling, FallingBlockFallRequest};

/// `BrushableBlock.TICK_DELAY` (`BrushableBlock.java:39`).
const TICK_DELAY: i32 = 2;

/// Generated `on_place_override` `BrushableBlock` (`suspicious_sand`, `suspicious_gravel`).
#[must_use]
pub fn is_brushable(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.on_place_override)
        == Some(VanillaOnPlaceOverride::BrushableBlock)
}

pub(crate) fn schedule_self(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    let Some(tick_type) = BlockTickType::of_block_state(block_state_id) else {
        return;
    };
    level.schedule_block_tick(pos, tick_type, TICK_DELAY, TickPriority::Normal);
}

pub(super) fn on_place(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    schedule_self(&mut BlockShapeCtx::Live(level), pos, block_state_id);
}

#[must_use]
pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    schedule_self(level, pos, block_state_id);
    block_state_id
}

/// `BlockStateProperties.DUSTED` set on `block_state_id`.
#[must_use]
pub fn with_dusted(block_state_id: i32, dusted: i32) -> Option<i32> {
    with_property(block_state_id, "dusted", &dusted.to_string())
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if let Some(reset) = level.brushable_check_reset(pos) {
        let current = level.block_state(pos);
        if let Some(dusted) = reset.dusted.and_then(|dusted| with_dusted(current, dusted)) {
            level.set_block(
                pos,
                dusted,
                SET_BLOCK_UPDATE_ALL,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        }
        if reset.reschedule {
            let state = level.block_state(pos);
            schedule_self(&mut BlockShapeCtx::Live(level), pos, state);
        }
    }
    if falling::is_free(level, level.block_state(below(pos))) != Some(true)
        || pos.y < level.get_min_y()
    {
        return;
    }
    let _ = level.request_falling_block_fall(FallingBlockFallRequest {
        pos,
        block_state_id,
        disable_drop: true,
        hurt_entities: None,
    });
}
