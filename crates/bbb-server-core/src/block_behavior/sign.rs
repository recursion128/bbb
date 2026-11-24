//!
//!
//! The sign block entity — text, dye, glow, wax and the editor the click
//! opens — is not here, and neither is `getStateForPlacement`.

use super::BlockShapeCtx;
use super::BlockTags;
use crate::block_getter::BlockGetter;

use bbb_protocol::block_state_types::VanillaBlockSupportType as SupportType;
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaCanSurviveOverride, VanillaGetStateForPlacementOverride,
};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{above, below, FlowingFluid};
use bbb_server_types::chunk_ticks::{FluidTickType, TickPriority};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::{block_state_is_solid, FluidFamily};

use bbb_server_types::directions::{clock_wise, counter_clock_wise};

use super::{
    block_type_of, bool_property, can_survive_override, direction_property, face_sturdy,
    face_sturdy_at, BlockStateAt,
};

const WATERLOGGED: &str = "waterlogged";

/// `HorizontalDirectionalBlock.FACING` as `WallSignBlock` re-exports it
/// (`WallSignBlock.java:30`).
const FACING: &str = "facing";

/// The subclass whose `canSurvive` and `updateShape` a sign reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SignKind {
    Standing,
    Wall,
    CeilingHanging,
}

/// Derived from the generated `can_survive_override` class tag: the three
/// classes that declare the member are exactly the three kinds.
fn sign_kind(block_state_id: i32) -> Option<SignKind> {
    match can_survive_override(block_type_of(block_state_id)?)? {
        VanillaCanSurviveOverride::StandingSignBlock => Some(SignKind::Standing),
        VanillaCanSurviveOverride::WallSignBlock => Some(SignKind::Wall),
        VanillaCanSurviveOverride::CeilingHangingSignBlock => Some(SignKind::CeilingHanging),
        _ => None,
    }
}

#[must_use]
pub fn is_sign(block_state_id: i32) -> bool {
    sign_kind(block_state_id).is_some()
}

#[must_use]
pub fn is_wall_hanging_sign(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|block| block.get_state_for_placement_override)
        == Some(VanillaGetStateForPlacementOverride::WallHangingSignBlock)
}

pub(super) fn can_survive(
    block_state_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt,
) -> Option<bool> {
    let support = support_pos(block_state_id, pos)?;
    match sign_kind(block_state_id)? {
        SignKind::Standing | SignKind::Wall => Some(block_state_is_solid(block_state_at(support))),
        SignKind::CeilingHanging => face_sturdy(
            block_state_at(support),
            Direction::Down,
            SupportType::Center,
        ),
    }
}

pub(super) fn can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
) -> Option<bool> {
    let support_pos = support_pos(block_state_id, pos)?;
    let support = level.block_state(support_pos);
    match sign_kind(block_state_id) {
        Some(SignKind::Standing | SignKind::Wall) => Some(block_state_is_solid(support)),
        Some(SignKind::CeilingHanging) => Some(face_sturdy_at(
            support,
            support_pos,
            Direction::Down,
            SupportType::Center,
        )),
        None => None,
    }
}

/// The one cell each subclass reads, which is also the one direction its
/// `updateShape` reacts to.
fn support_direction(block_state_id: i32) -> Option<Direction> {
    match sign_kind(block_state_id)? {
        SignKind::Standing => Some(Direction::Down),
        SignKind::Wall => direction_property(block_state_id, FACING).map(opposite),
        SignKind::CeilingHanging => Some(Direction::Up),
    }
}

fn support_pos(block_state_id: i32, pos: BlockPos) -> Option<BlockPos> {
    Some(match sign_kind(block_state_id)? {
        SignKind::Standing => below(pos),
        SignKind::Wall => relative(pos, support_direction(block_state_id)?),
        SignKind::CeilingHanging => above(pos),
    })
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    if support_direction(block_state_id) == Some(direction_to_neighbour)
        && can_survive_live(level, block_state_id, pos) == Some(false)
    {
        return super::air_block_state_id();
    }
    wall_hanging_update_shape(level, block_state_id, pos)
}

pub(super) fn wall_hanging_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
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

fn same_axis(left: Direction, right: Direction) -> bool {
    matches!(
        (left, right),
        (
            Direction::North | Direction::South,
            Direction::North | Direction::South
        ) | (
            Direction::West | Direction::East,
            Direction::West | Direction::East
        )
    )
}

///
/// A neighbouring wall hanging sign attaches by agreeing on the axis; anything
/// else has to present a full face towards the sign.
fn wall_hanging_can_attach_to(
    tags: &dyn BlockTags,
    block_state_id: i32,
    attach_state: i32,
    attach_face: Direction,
) -> Option<bool> {
    if tags.block_in_tag(attach_state, "minecraft:wall_hanging_signs") {
        return Some(same_axis(
            direction_property(attach_state, FACING)?,
            direction_property(block_state_id, FACING)?,
        ));
    }
    face_sturdy(attach_state, attach_face, SupportType::Full)
}

#[must_use]
pub fn wall_hanging_can_place(
    block_state_id: i32,
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<bool> {
    let facing = direction_property(block_state_id, FACING)?;
    let clockwise = clock_wise(facing);
    let counter_clockwise = counter_clock_wise(facing);
    let clockwise_state = block_state_at(relative(pos, clockwise));
    let counter_clockwise_state = block_state_at(relative(pos, counter_clockwise));
    Some(
        wall_hanging_can_attach_to(tags, block_state_id, clockwise_state, counter_clockwise)
            == Some(true)
            || wall_hanging_can_attach_to(tags, block_state_id, counter_clockwise_state, clockwise)
                == Some(true),
    )
}
