//! `PitcherCropBlock` (`net/minecraft/world/level/block/PitcherCropBlock.java`),
//! the one `DoublePlantBlock` that grows.
//!
//! It borrows `CropBlock`'s growth speed and light threshold from
//! [`super::crop`] and its two-cell survival from [`super::double_plant`]; what
//! is its own is the age that turns a one-cell bulb into a two-cell plant at
//! `DOUBLE_PLANT_AGE_INTERSECTION`.

use super::{BlockShapeCtx, LevelCtx};
use crate::block_getter::BlockGetter;

use bbb_protocol::block_types::{
    VanillaGetStateForPlacementOverride, VANILLA_BLOCK_PITCHER_CROP_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{above, SET_BLOCK_UPDATE_ALL};
use bbb_server_types::fluid::block_state_is_air;

use super::crop::{GROWTH_DRAW_NUMERATOR, SURVIVE_LIGHT_THRESHOLD};
use super::double_plant::{self, DoubleBlockHalf};
use super::{
    air_block_state_id, block_type_of, default_block_state_id, get_state_for_placement_override,
    int_property, state_is_block, with_property, SET_BLOCK_UPDATE_CLIENTS,
};

/// `PitcherCropBlock.AGE` (`PitcherCropBlock.java:36`).
const AGE: &str = "age";

pub(crate) fn state_for_placement(block_type_id: i32) -> Option<i32> {
    (get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::PitcherCropBlock))
    .then(|| default_block_state_id(block_type_id))
}

/// `PitcherCropBlock.MAX_AGE` (`PitcherCropBlock.java:35`).
pub const MAX_AGE: i32 = 4;

/// `PitcherCropBlock.DOUBLE_PLANT_AGE_INTERSECTION`
/// (`PitcherCropBlock.java:38`), the age from which the plant occupies two
/// cells.
pub const DOUBLE_PLANT_AGE_INTERSECTION: i32 = 3;

/// `PitcherCropBlock.isDouble` (`PitcherCropBlock.java:175-177`).
const fn is_double(age: i32) -> bool {
    age >= DOUBLE_PLANT_AGE_INTERSECTION
}

fn is_lower(block_state_id: i32) -> bool {
    state_is_block(block_state_id, VANILLA_BLOCK_PITCHER_CROP_ID)
        && double_plant::half(block_state_id) == Some(DoubleBlockHalf::Lower)
}

fn is_max_age(block_state_id: i32) -> bool {
    int_property(block_state_id, AGE).is_some_and(|age| age >= MAX_AGE)
}

fn sufficient_light(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    level.raw_brightness(pos, 0) >= SURVIVE_LIGHT_THRESHOLD
}

fn can_survive_at_brightness(
    block_state_id: i32,
    pos: BlockPos,
    raw_brightness: Option<i32>,
    tags: &dyn super::BlockTags,
    block_state_at: super::BlockStateAt<'_>,
) -> Option<bool> {
    if !state_is_block(block_state_id, VANILLA_BLOCK_PITCHER_CROP_ID) {
        return None;
    }
    if is_lower(block_state_id) && raw_brightness? < SURVIVE_LIGHT_THRESHOLD {
        return Some(false);
    }
    double_plant::can_survive(block_state_id, pos, tags, block_state_at)
}

fn can_grow_into(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    let state = level.block_state(pos);
    block_state_is_air(state) || state_is_block(state, VANILLA_BLOCK_PITCHER_CROP_ID)
}

fn can_grow(level: &LevelCtx<'_, '_>, lower_pos: BlockPos, lower_state: i32, new_age: i32) -> bool {
    let above_pos = above(lower_pos);
    !is_max_age(lower_state)
        && sufficient_light(level, lower_pos)
        && above_pos.y >= level.get_min_y()
        && above_pos.y <= level.get_max_y()
        && (!is_double(new_age) || can_grow_into(level, above_pos))
}

///
/// Level-taking rather than a `can_survive:` column for the same reason
/// [`super::crop::can_survive`] is: `sufficientLight` reads
/// `getRawBrightness`, which no `BlockStateAt` answers.
#[must_use]
pub fn can_survive<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> Option<bool> {
    if !state_is_block(block_state_id, VANILLA_BLOCK_PITCHER_CROP_ID) {
        return None;
    }
    if is_lower(block_state_id)
        && level.raw_brightness(pos, 0).unwrap_or(0) < SURVIVE_LIGHT_THRESHOLD
    {
        return Some(false);
    }
    double_plant::can_survive_at(level, block_state_id, pos, membership)
}

/// The same body on the shape seam. Only the lower half reads the light, so
/// an upper half still answers over a view that carries none.
pub(super) fn can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
) -> Option<bool> {
    can_survive_at_brightness(
        block_state_id,
        pos,
        level.raw_brightness(pos, 0),
        level.tags(),
        &mut |at| level.block_state(at),
    )
}

pub(super) fn update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    let Some(age) = int_property(block_state_id, AGE) else {
        return block_state_id;
    };
    if is_double(age) {
        return double_plant::update_shape(
            block_state_id,
            direction_to_neighbour,
            neighbour_state,
            &mut || {
                can_survive(level, block_state_id, pos, &|registry, tag, id| {
                    level.registry_tag_membership(registry, tag, id)
                })
            },
        );
    }
    match can_survive(level, block_state_id, pos, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    }) {
        Some(false) => air_block_state_id(),
        Some(true) | None => block_state_id,
    }
}

pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    let Some(speed) = super::crop::growth_speed(level, block_type_id, pos) else {
        return;
    };
    // Java truncates the float division towards zero before adding one.
    let bound = (GROWTH_DRAW_NUMERATOR / speed) as i32 + 1;
    if level.level_random().next_int(bound) == 0 {
        grow(level, block_state_id, pos, 1);
    }
}

fn grow(level: &mut LevelCtx<'_, '_>, lower_state: i32, lower_pos: BlockPos, increase: i32) {
    let Some(age) = int_property(lower_state, AGE) else {
        return;
    };
    let updated_age = (age + increase).min(MAX_AGE);
    if !can_grow(level, lower_pos, lower_state, updated_age) {
        return;
    }
    let Some(new_lower_state) = with_property(lower_state, AGE, &updated_age.to_string()) else {
        return;
    };
    level.set_block(
        lower_pos,
        new_lower_state,
        SET_BLOCK_UPDATE_CLIENTS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    if is_double(updated_age) {
        if let Some(upper_state) = with_property(new_lower_state, "half", "upper") {
            level.set_block(
                above(lower_pos),
                upper_state,
                SET_BLOCK_UPDATE_ALL,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        }
    }
}

/// `PitcherCropBlock.getLowerHalf` (`PitcherCropBlock.java:190-198`).
fn lower_half(level: &LevelCtx<'_, '_>, pos: BlockPos, state: i32) -> Option<(BlockPos, i32)> {
    if is_lower(state) {
        return Some((pos, state));
    }
    let lower_pos = super::below(pos);
    let lower_state = level.block_state(lower_pos);
    is_lower(lower_state).then_some((lower_pos, lower_state))
}

/// `PitcherCropBlock.isValidBonemealTarget` (`:201-204`).
pub(super) fn is_valid_bonemeal_target(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    state: i32,
) -> bool {
    lower_half(level, pos, state).is_some_and(|(lower_pos, lower_state)| {
        int_property(lower_state, AGE)
            .is_some_and(|age| can_grow(level, lower_pos, lower_state, age + 1))
    })
}

/// `PitcherCropBlock.performBonemeal` (`:212-217`).
pub(super) fn perform_bonemeal(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    if let Some((lower_pos, lower_state)) = lower_half(level, pos, state) {
        grow(level, lower_state, lower_pos, 1);
    }
}
