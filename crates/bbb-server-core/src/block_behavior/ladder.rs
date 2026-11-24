//!
//! One block, the same wall question `WallTorchBlock` asks, with the
//! waterlogged re-arm after the survival gate rather than before it: a ladder
//! whose wall goes leaves no scheduled water tick behind, where a lantern
//! does.

use super::BlockShapeCtx;
use crate::block_getter::BlockGetter;

use bbb_protocol::block_state_types::VanillaBlockSupportType as SupportType;
use bbb_protocol::block_types::{VanillaCanSurviveOverride, VanillaGetStateForPlacementOverride};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{with_property, FlowingFluid};
use bbb_server_types::chunk_ticks::{FluidTickType, TickPriority};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::FluidFamily;

use super::{
    block_type_of, bool_property, can_survive_override, default_block_state_id, direction_property,
    direction_serialized_name, face_sturdy, face_sturdy_at, get_state_for_placement_override,
    BlockStateAt,
};

const FACING: &str = "facing";
const WATERLOGGED: &str = "waterlogged";

#[must_use]
pub fn is_ladder(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(can_survive_override)
        == Some(VanillaCanSurviveOverride::LadderBlock)
}

pub(super) fn can_survive(
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

pub(super) fn can_survive_live<L: BlockGetter + ?Sized>(
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

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    directions: [Direction; 6],
    source_water: bool,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::LadderBlock)
    {
        return None;
    }
    for direction in directions {
        if matches!(direction, Direction::Down | Direction::Up) {
            continue;
        }
        let state = with_property(
            default_block_state_id(block_type_id),
            FACING,
            direction_serialized_name(opposite(direction)),
        )?;
        match can_survive(state, pos, block_state_at) {
            Some(true) => {
                return with_property(
                    state,
                    WATERLOGGED,
                    if source_water { "true" } else { "false" },
                );
            }
            Some(false) => {}
            None => return None,
        }
    }
    None
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    let wall = direction_property(block_state_id, FACING).map(opposite);
    if wall == Some(direction_to_neighbour)
        && can_survive_live(level, block_state_id, pos) == Some(false)
    {
        return super::air_block_state_id();
    }
    if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
    block_state_id
}
