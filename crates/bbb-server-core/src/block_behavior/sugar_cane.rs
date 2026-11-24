//! `SugarCaneBlock` (`net/minecraft/world/level/block/SugarCaneBlock.java`).
//!
//! The first of the growth family: a one-block-wide column that counts how
//! tall it already is, ages sixteen times, and then pushes a fresh cane onto
//! its own head.
//!
//! `randomTick` (`:50-68`) is the growth,
//! `canSurvive` (`:89-108`) is the two-step support test — the cell below is
//! either more cane or a `#minecraft:supports_sugar_cane` block with water (or
//! a `#minecraft:supports_sugar_cane_adjacently` block) beside it — and
//! `updateShape` (`:71-86`) arms the tick that removes an unsupported cane.
//!
//!
//! # What is not here
//!
//! * `getShape` (`:38-40`), a collision surface.

use super::BlockTags;
use super::{block_tag_matches, fluid_tag_matches, BlockGetter, BlockShapeCtx, LevelCtx};
use bbb_protocol::block_types::VanillaRandomTickOverride;
use bbb_protocol::packets::BlockPos;

use crate::fluid::{above, below, HORIZONTAL, SET_BLOCK_UPDATE_ALL};
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::directions::relative;
use bbb_server_types::fluid::{block_state_fluid_state, block_state_is_air};

use super::{
    block_type_of, default_block_state_id, int_property, offset, random_tick_override,
    state_is_block, with_property, BlockStateAt, SET_BLOCK_UPDATE_NONE,
};

/// The property `SugarCaneBlock.AGE` names (`SugarCaneBlock.java:24`,
/// `BlockStateProperties.AGE_15` = `IntegerProperty.create("age", 0, 15)`,
/// `BlockStateProperties.java:97`).
const AGE: &str = "age";
const SUPPORTS_SUGAR_CANE_TAG: &str = "minecraft:supports_sugar_cane";
const SUPPORTS_SUGAR_CANE_ADJACENTLY_TAG: &str = "minecraft:supports_sugar_cane_adjacently";

/// The age at which the next random tick grows a cane instead of ageing it
/// (`SugarCaneBlock.java:60`). `SugarCaneBlock` spells the 15 out rather than
/// naming `BlockStateProperties.MAX_AGE_15`, so it is a literal here too.
pub const MAX_AGE: i32 = 15;

/// The column height at which growth stops (`SugarCaneBlock.java:58`).
///
/// The count starts at 1 for the ticking block itself (`:52`), so this is
/// "two blocks of cane already below me", i.e. a three-tall stack.
pub const MAX_HEIGHT: i32 = 3;

/// The tick delay `updateShape` arms (`SugarCaneBlock.java:82`).
pub const TICK_DELAY: i32 = 1;

/// Whether a stored state's behaviour belongs to `SugarCaneBlock`.
///
/// The generated `random_tick_override` class tag is the whole test:
/// `SugarCaneBlock` has no subclass in 26.1, so the class that declares
/// `randomTick` names the class outright.
#[must_use]
pub fn is_sugar_cane(block_state_id: i32) -> bool {
    block_type_of(block_state_id).is_some_and(|block_type_id| {
        random_tick_override(block_type_id) == Some(VanillaRandomTickOverride::SugarCaneBlock)
    })
}

pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if !block_state_is_air(level.block_state(above(pos))) {
        return;
    }
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    // `int height = 1;` then `while (level.getBlockState(pos.below(height)).is(this))`
    // (`:52-56`): the counter starts at the ticking block and walks down.
    let mut height = 1;
    while state_is_block(level.block_state(offset(pos, 0, -height, 0)), block_type_id) {
        height += 1;
    }
    if height >= MAX_HEIGHT {
        return;
    }
    let Some(age) = int_property(block_state_id, AGE) else {
        return;
    };
    if age == MAX_AGE {
        level.set_block(
            above(pos),
            default_block_state_id(block_type_id),
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        if let Some(reset) = with_property(block_state_id, AGE, "0") {
            level.set_block(
                pos,
                reset,
                SET_BLOCK_UPDATE_NONE,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        }
    } else if let Some(older) = with_property(block_state_id, AGE, &(age + 1).to_string()) {
        level.set_block(
            pos,
            older,
            SET_BLOCK_UPDATE_NONE,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}

///
/// The second half reads the four cells beside the block *below* the cane, not
/// beside the cane itself (`:96`), which is why a cane stands on sand whose
/// neighbour is water rather than needing water at its own height.
///
#[must_use]
pub fn can_survive(
    block_state_id: i32,
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> bool {
    let below_pos = below(pos);
    let below_state = block_state_at(below_pos);
    // `stateBelow.is(this)` (`:91`) is the *same block*, not "any
    // `SugarCaneBlock`"; the ticking state names it.
    if block_type_of(below_state) == block_type_of(block_state_id) {
        return true;
    }
    if !tags.block_in_tag(below_state, "minecraft:supports_sugar_cane") {
        return false;
    }
    HORIZONTAL.into_iter().any(|direction| {
        let beside = block_state_at(relative(below_pos, direction));
        tags.fluid_in_tag(
            block_state_fluid_state(beside).identity(),
            "minecraft:supports_sugar_cane_adjacently",
        ) || tags.block_in_tag(beside, "minecraft:supports_sugar_cane_adjacently")
    })
}

fn can_survive_at<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> bool {
    let below_pos = below(pos);
    let below_state = level.block_state(below_pos);
    if block_type_of(below_state) == block_type_of(block_state_id) {
        return true;
    }
    if !block_tag_matches(below_state, SUPPORTS_SUGAR_CANE_TAG, membership) {
        return false;
    }
    HORIZONTAL.into_iter().any(|direction| {
        let beside = level.block_state(relative(below_pos, direction));
        fluid_tag_matches(
            block_state_fluid_state(beside).identity(),
            SUPPORTS_SUGAR_CANE_ADJACENTLY_TAG,
            membership,
        ) || block_tag_matches(beside, SUPPORTS_SUGAR_CANE_ADJACENTLY_TAG, membership)
    })
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if !can_survive_at(level, block_state_id, pos, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    }) {
        level.destroy_block(pos, true);
    }
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    if !can_survive_at(level, block_state_id, pos, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    }) {
        if let Some(block) = BlockTickType::of_block_state(block_state_id) {
            level.schedule_block_tick(pos, block, TICK_DELAY, TickPriority::Normal);
        }
    }
    block_state_id
}
