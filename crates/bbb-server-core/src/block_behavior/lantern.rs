//!
//! Ten blocks. `HANGING` picks which of the two cells along the Y axis holds
//! the lantern up, and the lantern asks that cell for a supported centre, so a
//! chain carries one either way. The waterlogged re-arm runs before the
//! survival gate, which is why a hanging lantern whose ceiling goes still
//! leaves a scheduled water tick behind.

use super::support::{can_support_center_at_tagged, can_support_center_tagged};
use super::BlockShapeCtx;
use crate::block_getter::{BlockGetter, BlockTags};

use bbb_protocol::block_types::VanillaCanSurviveOverride;
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{with_property, FlowingFluid};
use bbb_server_types::chunk_ticks::{FluidTickType, TickPriority};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::FluidFamily;

use super::{
    block_type_of, bool_property, can_survive_override, default_block_state_id, BlockStateAt,
};

const HANGING: &str = "hanging";
const WATERLOGGED: &str = "waterlogged";

#[must_use]
pub fn is_lantern(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(can_survive_override)
        == Some(VanillaCanSurviveOverride::LanternBlock)
}

fn connected_direction(block_state_id: i32) -> Option<Direction> {
    Some(if bool_property(block_state_id, HANGING)? {
        Direction::Down
    } else {
        Direction::Up
    })
}

pub(super) fn can_survive(
    block_state_id: i32,
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt,
) -> Option<bool> {
    let direction = opposite(connected_direction(block_state_id)?);
    can_support_center_tagged(
        tags,
        block_state_at(relative(pos, direction)),
        opposite(direction),
    )
}

pub(super) fn can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
) -> Option<bool> {
    let direction = connected_direction(block_state_id).map(opposite)?;
    let support_pos = relative(pos, direction);
    Some(can_support_center_at_tagged(
        level.tags(),
        level.block_state(support_pos),
        support_pos,
        opposite(direction),
    ))
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    directions: [Direction; 6],
    source_water: bool,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt,
) -> Option<i32> {
    for direction in directions {
        if !matches!(direction, Direction::Down | Direction::Up) {
            continue;
        }
        let state = with_property(
            default_block_state_id(block_type_id),
            HANGING,
            if direction == Direction::Up {
                "true"
            } else {
                "false"
            },
        )?;
        match can_survive(state, pos, tags, &mut *block_state_at) {
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
    let support_changed =
        connected_direction(block_state_id).map(opposite) == Some(direction_to_neighbour);
    let survives = support_changed
        .then(|| can_survive_live(level, block_state_id, pos))
        .flatten();
    if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
    if survives == Some(false) {
        super::air_block_state_id()
    } else {
        block_state_id
    }
}
