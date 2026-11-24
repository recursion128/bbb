//! `PistonHeadBlock` support, neighbour forwarding, and base removal.

use super::{BlockShapeCtx, LevelCtx};
use bbb_protocol::block_types::{
    VANILLA_BLOCK_MOVING_PISTON_ID, VANILLA_BLOCK_PISTON_HEAD_ID, VANILLA_BLOCK_PISTON_ID,
    VANILLA_BLOCK_STICKY_PISTON_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::directions::{opposite, relative};

use super::{
    air_block_state_id, block_type_of, bool_property, direction_property, state_is_block,
    string_property, BlockStateAt,
};

const FACING: &str = "facing";
const TYPE: &str = "type";
const EXTENDED: &str = "extended";

#[must_use]
pub fn is_piston_head(block_state_id: i32) -> bool {
    block_type_of(block_state_id) == Some(VANILLA_BLOCK_PISTON_HEAD_ID)
}

pub(super) fn can_survive(
    block_state_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt,
) -> Option<bool> {
    let facing = direction_property(block_state_id, FACING)?;
    let base = block_state_at(relative(pos, opposite(facing)));
    Some(
        is_fitting_base(block_state_id, base)?
            || state_is_block(base, VANILLA_BLOCK_MOVING_PISTON_ID)
                && direction_property(base, FACING) == Some(facing),
    )
}

pub(super) fn update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    let facing = direction_property(block_state_id, FACING);
    if facing == Some(opposite(direction_to_neighbour))
        && can_survive(block_state_id, pos, &mut |at| level.block_state(at)) == Some(false)
    {
        air_block_state_id()
    } else {
        block_state_id
    }
}

pub(super) fn neighbor_changed(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    changed_block_type_id: i32,
) {
    if can_survive(block_state_id, pos, &mut |at| level.block_state(at)) != Some(true) {
        return;
    }
    let Some(base_pos) =
        direction_property(block_state_id, FACING).map(|facing| relative(pos, opposite(facing)))
    else {
        return;
    };
    level.neighbor_changed(base_pos, changed_block_type_id);
}

pub(super) fn affect_neighbors_after_removal(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    let Some(base_pos) =
        direction_property(block_state_id, FACING).map(|facing| relative(pos, opposite(facing)))
    else {
        return;
    };
    if is_fitting_base(block_state_id, level.block_state(base_pos)) == Some(true) {
        level.destroy_block(base_pos, true);
    }
}

pub(super) fn is_fitting_base(block_state_id: i32, base_state_id: i32) -> Option<bool> {
    let base_block_type_id = match string_property(block_state_id, TYPE)?.as_str() {
        "normal" => VANILLA_BLOCK_PISTON_ID,
        "sticky" => VANILLA_BLOCK_STICKY_PISTON_ID,
        _ => return None,
    };
    Some(
        state_is_block(base_state_id, base_block_type_id)
            && bool_property(base_state_id, EXTENDED) == Some(true)
            && direction_property(base_state_id, FACING)
                == direction_property(block_state_id, FACING),
    )
}
