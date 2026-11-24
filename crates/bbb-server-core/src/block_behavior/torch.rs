//!
//! Six blocks: the three that stand on the cell below and the three that hang
//! off a wall. The standing one asks `Block.canSupportCenter`, the wall one
//! asks the wall for a full face, and each turns to air only when the update
//! came from the cell it asked. `animateTick`'s flame particles are the
//! client's.

use super::BlockShapeCtx;
use crate::block_getter::BlockGetter;

use bbb_protocol::block_state_types::VanillaBlockSupportType as SupportType;
use bbb_protocol::block_types::VanillaCanSurviveOverride;
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::below;
use bbb_server_types::directions::{opposite, relative};

use super::{
    block_type_of, can_support_center, can_support_center_at, can_survive_override,
    direction_property, face_sturdy, face_sturdy_at, BlockStateAt,
};

/// `HorizontalDirectionalBlock.FACING` as `WallTorchBlock` re-exports it
/// (`WallTorchBlock.java:29`).
const FACING: &str = "facing";

fn declares(block_state_id: i32, class: VanillaCanSurviveOverride) -> bool {
    block_type_of(block_state_id).and_then(can_survive_override) == Some(class)
}

#[must_use]
pub fn is_torch(block_state_id: i32) -> bool {
    declares(block_state_id, VanillaCanSurviveOverride::BaseTorchBlock)
}

#[must_use]
pub fn is_wall_torch(block_state_id: i32) -> bool {
    declares(block_state_id, VanillaCanSurviveOverride::WallTorchBlock)
}

pub(super) fn can_survive(pos: BlockPos, block_state_at: BlockStateAt) -> Option<bool> {
    can_support_center(block_state_at(below(pos)), Direction::Up)
}

pub(super) fn wall_can_survive(
    block_state_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt,
) -> Option<bool> {
    let facing = direction_property(block_state_id, FACING)?;
    face_sturdy(
        block_state_at(relative(pos, opposite(facing))),
        facing,
        SupportType::Full,
    )
}

pub(super) fn can_survive_live<L: BlockGetter + ?Sized>(level: &L, pos: BlockPos) -> Option<bool> {
    let support_pos = below(pos);
    Some(can_support_center_at(
        level.block_state(support_pos),
        support_pos,
        Direction::Up,
    ))
}

pub(super) fn wall_can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
) -> Option<bool> {
    let facing = direction_property(block_state_id, FACING)?;
    let support_pos = relative(pos, opposite(facing));
    Some(face_sturdy_at(
        level.block_state(support_pos),
        support_pos,
        facing,
        SupportType::Full,
    ))
}

pub(super) fn update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    if direction_to_neighbour == Direction::Down && can_survive_live(level, pos) == Some(false) {
        super::air_block_state_id()
    } else {
        block_state_id
    }
}

pub(super) fn wall_update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    let wall = direction_property(block_state_id, FACING).map(opposite);
    if wall == Some(direction_to_neighbour)
        && wall_can_survive_live(level, block_state_id, pos) == Some(false)
    {
        super::air_block_state_id()
    } else {
        block_state_id
    }
}
