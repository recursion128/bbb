//!
//! Thirty-two blocks, sixteen dyes each, and the same pair of bodies the signs
//! carry with one difference: a banner is not waterloggable, so the branch
//! that is not air is `BlockBehaviour.updateShape`'s own `return state`. The
//! banner block entity, its patterns and the loom are not here.

use super::BlockShapeCtx;
use bbb_protocol::block_types::{VanillaCanSurviveOverride, VanillaGetStateForPlacementOverride};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::below;
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::block_state_is_solid;

use super::{
    block_type_of, can_survive_override, default_block_state_id, direction_property,
    direction_serialized_name, get_state_for_placement_override, with_property, BlockStateAt,
};

/// `HorizontalDirectionalBlock.FACING` as `WallBannerBlock` re-exports it
/// (`WallBannerBlock.java:26`).
const FACING: &str = "facing";

/// `BannerBlock.ROTATION` (`BannerBlock.java:28`).
const ROTATION: &str = "rotation";

/// Derived from the generated `can_survive_override` class tag, which names
/// the two classes over exactly the thirty-two banners.
fn wall_mounted(block_state_id: i32) -> Option<bool> {
    match can_survive_override(block_type_of(block_state_id)?)? {
        VanillaCanSurviveOverride::BannerBlock => Some(false),
        VanillaCanSurviveOverride::WallBannerBlock => Some(true),
        _ => None,
    }
}

#[must_use]
pub fn is_banner(block_state_id: i32) -> bool {
    wall_mounted(block_state_id).is_some()
}

/// The one cell the banner hangs on, which is also the one direction its
/// `updateShape` reacts to.
fn support_direction(block_state_id: i32) -> Option<Direction> {
    if wall_mounted(block_state_id)? {
        direction_property(block_state_id, FACING).map(opposite)
    } else {
        Some(Direction::Down)
    }
}

pub(super) fn can_survive(
    block_state_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt,
) -> Option<bool> {
    let support = if wall_mounted(block_state_id)? {
        relative(pos, support_direction(block_state_id)?)
    } else {
        below(pos)
    };
    let support = block_state_at(support);
    block_type_of(support)?;
    Some(block_state_is_solid(support))
}

pub(super) fn standing_state_for_placement(block_type_id: i32, rotation: i32) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::BannerBlock)
    {
        return None;
    }
    with_property(
        default_block_state_id(block_type_id),
        ROTATION,
        &rotation.to_string(),
    )
}

pub(super) fn wall_state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    directions: [Direction; 6],
    block_state_at: BlockStateAt<'_>,
) -> Option<Option<i32>> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::WallBannerBlock)
    {
        return None;
    }
    let mut state = default_block_state_id(block_type_id);
    for direction in directions {
        if matches!(direction, Direction::Down | Direction::Up) {
            continue;
        }
        state = with_property(
            state,
            FACING,
            direction_serialized_name(opposite(direction)),
        )?;
        match can_survive(state, pos, block_state_at) {
            Some(true) => return Some(Some(state)),
            Some(false) => {}
            None => return None,
        }
    }
    Some(None)
}

pub(super) fn update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    if support_direction(block_state_id) == Some(direction_to_neighbour)
        && can_survive(block_state_id, pos, &mut |at| level.block_state(at)) == Some(false)
    {
        super::air_block_state_id()
    } else {
        block_state_id
    }
}
