//! `DoublePlantBlock` (`net/minecraft/world/level/block/DoublePlantBlock.java`)
//! and the two subclasses that replace its `canSurvive`: `TallSeagrassBlock`
//! and `PitcherCropBlock`.
//!
//! The family is nine blocks over four classes, and one `updateShape` body
//! answers for all of them: the classes differ only in which `canSurvive` that
//! body consults, so [`update_shape`] takes it as an argument.
//! `SmallDripleafBlock` reaches the same body through [`super::vegetation`],
//! and `PitcherCropBlock`'s young half through [`super::pitcher_crop`].

use super::{fluid_tag_matches, BlockGetter, BlockShapeCtx, BlockTags, LevelCtx};
use bbb_protocol::block_types::VanillaCanSurviveOverride;
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{above, SET_BLOCK_UPDATE_ALL};

use super::vegetation::{may_place_on, may_place_on_at, may_place_on_body};
use super::{
    air_block_state_id, below, block_state_fluid_state, block_type_of, can_survive_override,
    default_block_state_id, string_property, with_property, BlockStateAt,
};

/// `BlockStateProperties.DOUBLE_BLOCK_HALF` (`BlockStateProperties.java:71`).
const HALF: &str = "half";
const WATER_TAG: &str = "minecraft:water";
const WATERLOGGED: &str = "waterlogged";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoubleBlockHalf {
    Upper,
    Lower,
}

/// `state.getValue(DoublePlantBlock.HALF)`.
#[must_use]
pub fn half(block_state_id: i32) -> Option<DoubleBlockHalf> {
    match string_property(block_state_id, HALF)?.as_str() {
        "upper" => Some(DoubleBlockHalf::Upper),
        "lower" => Some(DoubleBlockHalf::Lower),
        _ => None,
    }
}

///
/// `copyWaterloggedFrom` (`:92-94`) is the identity here — the class adds only
/// `HALF` (`:135-137`) and none of the seven carries `WATERLOGGED`.
#[must_use]
pub fn upper_state_after_placement(block_state_id: i32) -> Option<i32> {
    if half(block_state_id) != Some(DoubleBlockHalf::Lower) || !is_double_plant(block_state_id) {
        return None;
    }
    with_property(
        default_block_state_id(block_type_of(block_state_id)?),
        HALF,
        "upper",
    )
}

pub(super) fn set_placed_by(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    let above_pos = above(pos);
    let Some(upper) = with_property(default_block_state_id(block_type_id), HALF, "upper")
        .and_then(|upper| copy_waterlogged_from(level.tags(), level.block_state(above_pos), upper))
    else {
        return;
    };
    level.set_block(
        above_pos,
        upper,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}

#[must_use]
pub fn copy_waterlogged_from(tags: &dyn BlockTags, state_at_pos: i32, state: i32) -> Option<i32> {
    if string_property(state, WATERLOGGED).is_none() {
        return Some(state);
    }
    let water = tags.fluid_in_tag(block_state_fluid_state(state_at_pos).identity(), WATER_TAG);
    with_property(state, WATERLOGGED, if water { "true" } else { "false" })
}

#[must_use]
pub fn is_double_plant(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(can_survive_override)
        == Some(VanillaCanSurviveOverride::DoublePlantBlock)
}

#[must_use]
pub fn is_tall_seagrass(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(can_survive_override)
        == Some(VanillaCanSurviveOverride::TallSeagrassBlock)
}

fn stands_on_its_lower_half(
    block_state_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt<'_>,
) -> bool {
    let below_state = block_state_at(below(pos));
    block_type_of(below_state) == block_type_of(block_state_id)
        && half(below_state) == Some(DoubleBlockHalf::Lower)
}

#[must_use]
pub fn can_survive(
    block_state_id: i32,
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<bool> {
    if half(block_state_id)? == DoubleBlockHalf::Upper {
        return Some(stands_on_its_lower_half(
            block_state_id,
            pos,
            block_state_at,
        ));
    }
    let support = may_place_on_body(block_type_of(block_state_id)?)?;
    may_place_on(support, pos, tags, block_state_at)
}

#[must_use]
pub fn tall_seagrass_can_survive(
    block_state_id: i32,
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<bool> {
    if half(block_state_id)? == DoubleBlockHalf::Upper {
        return Some(stands_on_its_lower_half(
            block_state_id,
            pos,
            block_state_at,
        ));
    }
    let own_fluid = block_state_fluid_state(block_state_at(pos));
    Some(
        can_survive(block_state_id, pos, tags, block_state_at)?
            && tags.fluid_in_tag(own_fluid.identity(), WATER_TAG)
            && own_fluid.is_full(),
    )
}

pub(super) fn can_survive_at<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> Option<bool> {
    if half(block_state_id)? == DoubleBlockHalf::Upper {
        let below_state = level.block_state(below(pos));
        return Some(
            block_type_of(below_state) == block_type_of(block_state_id)
                && half(below_state) == Some(DoubleBlockHalf::Lower),
        );
    }
    let support = may_place_on_body(block_type_of(block_state_id)?)?;
    Some(may_place_on_at(level, support, pos, membership))
}

pub(super) fn tall_seagrass_can_survive_at<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> Option<bool> {
    if half(block_state_id)? == DoubleBlockHalf::Upper {
        return can_survive_at(level, block_state_id, pos, membership);
    }
    let own_fluid = block_state_fluid_state(level.block_state(pos));
    if !own_fluid.is_full() {
        return Some(false);
    }
    Some(
        can_survive_at(level, block_state_id, pos, membership)?
            && fluid_tag_matches(own_fluid.identity(), WATER_TAG, membership),
    )
}

///
///
/// `can_survive` is a thunk because each class in the family brings its own,
/// and `None` from it keeps the state — the same fail-closed direction the
/// rest of the survival slice takes.
#[must_use]
pub fn update_shape(
    block_state_id: i32,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
    can_survive: &mut dyn FnMut() -> Option<bool>,
) -> i32 {
    let Some(half_of_state) = half(block_state_id) else {
        return block_state_id;
    };
    let vertical = matches!(direction_to_neighbour, Direction::Up | Direction::Down);
    let from_the_other_half_side =
        (half_of_state == DoubleBlockHalf::Lower) == (direction_to_neighbour == Direction::Up);
    let neighbour_is_the_other_half = block_type_of(neighbour_state)
        == block_type_of(block_state_id)
        && half(neighbour_state) != Some(half_of_state);
    if !vertical || !from_the_other_half_side || neighbour_is_the_other_half {
        if can_survive().unwrap_or(true) {
            block_state_id
        } else {
            air_block_state_id()
        }
    } else {
        air_block_state_id()
    }
}

/// The table's `update_shape:` column for the seven plain `DoublePlantBlock`
/// blocks.
pub(super) fn plain_update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    update_shape(
        block_state_id,
        direction_to_neighbour,
        neighbour_state,
        &mut || {
            can_survive_at(level, block_state_id, pos, &|registry, tag, id| {
                level.registry_tag_membership(registry, tag, id)
            })
        },
    )
}

/// The table's `update_shape:` column for `minecraft:tall_seagrass`.
pub(super) fn tall_seagrass_update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    update_shape(
        block_state_id,
        direction_to_neighbour,
        neighbour_state,
        &mut || {
            tall_seagrass_can_survive_at(level, block_state_id, pos, &|registry, tag, id| {
                level.registry_tag_membership(registry, tag, id)
            })
        },
    )
}
