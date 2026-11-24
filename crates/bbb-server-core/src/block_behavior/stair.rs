use super::BlockShapeCtx;
use crate::fluid::{with_property, FlowingFluid};
use bbb_protocol::block_state_types::vanilla_block_state_property;
use bbb_protocol::block_types::{vanilla_block_type, VanillaGetStateForPlacementOverride};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::chunk_ticks::{FluidTickType, TickPriority};
use bbb_server_types::directions::{counter_clock_wise, opposite, relative};
use bbb_server_types::fluid::FluidFamily;

use super::{block_type_of, bool_property, direction_property};

const FACING: &str = "facing";
const HALF: &str = "half";
const SHAPE: &str = "shape";
const WATERLOGGED: &str = "waterlogged";

#[derive(Clone, Copy)]
struct StairProperties {
    facing: Direction,
    half: &'static str,
}

#[must_use]
pub(super) fn is_stair(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.get_state_for_placement_override)
        == Some(VanillaGetStateForPlacementOverride::StairBlock)
}

fn is_horizontal(direction: Direction) -> bool {
    !matches!(direction, Direction::Down | Direction::Up)
}

fn properties(block_state_id: i32) -> Option<StairProperties> {
    if !is_stair(block_state_id) {
        return None;
    }
    let facing = direction_property(block_state_id, FACING)?;
    if !is_horizontal(facing) {
        return None;
    }
    let half = match vanilla_block_state_property(block_state_id, HALF)? {
        "bottom" => "bottom",
        "top" => "top",
        _ => return None,
    };
    Some(StairProperties { facing, half })
}

fn same_axis(left: Direction, right: Direction) -> bool {
    matches!(
        (left, right),
        (
            Direction::North | Direction::South,
            Direction::North | Direction::South
        ) | (
            Direction::East | Direction::West,
            Direction::East | Direction::West
        )
    )
}

fn can_take_shape<F: FnMut(BlockPos) -> i32 + ?Sized>(
    block_state_at: &mut F,
    pos: BlockPos,
    own: StairProperties,
    direction: Direction,
) -> bool {
    properties(block_state_at(relative(pos, direction)))
        .is_none_or(|side| side.facing != own.facing || side.half != own.half)
}

/// `StairBlock.getStairsShape` (`StairBlock.java:130-157`).
pub(super) fn shape<F: FnMut(BlockPos) -> i32 + ?Sized>(
    block_state_id: i32,
    pos: BlockPos,
    block_state_at: &mut F,
) -> Option<&'static str> {
    let own = properties(block_state_id)?;
    let behind_state = block_state_at(relative(pos, own.facing));
    if let Some(behind) = properties(behind_state) {
        if own.half == behind.half
            && !same_axis(behind.facing, own.facing)
            && can_take_shape(block_state_at, pos, own, opposite(behind.facing))
        {
            return Some(if behind.facing == counter_clock_wise(own.facing) {
                "outer_left"
            } else {
                "outer_right"
            });
        }
    }

    let front_state = block_state_at(relative(pos, opposite(own.facing)));
    if let Some(front) = properties(front_state) {
        if own.half == front.half
            && !same_axis(front.facing, own.facing)
            && can_take_shape(block_state_at, pos, own, front.facing)
        {
            return Some(if front.facing == counter_clock_wise(own.facing) {
                "inner_left"
            } else {
                "inner_right"
            });
        }
    }

    Some("straight")
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
    if !is_horizontal(direction_to_neighbour) {
        return block_state_id;
    }
    let Some(shape) = shape(block_state_id, pos, &mut |pos| level.block_state(pos)) else {
        return block_state_id;
    };
    with_property(block_state_id, SHAPE, shape).unwrap_or(block_state_id)
}
