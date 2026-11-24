//! `BambooStalkBlock` (`net/minecraft/world/level/block/BambooStalkBlock.java`)
//! and `BambooSaplingBlock` (`BambooSaplingBlock.java`).
//!
//!
//! # The leaf shuffle is the interesting part
//!
//! `BambooStalkBlock.growBamboo` (`:191-211`) does not just place a block: a
//! new segment takes `SMALL` leaves when the segment below has none, and
//! `LARGE` when it does — and in that second case it *demotes* the two
//! segments below it, `SMALL` for the one directly beneath and `NONE` two
//! down, so the leaves stay bunched at the top of the stalk while the trunk
//! goes bare.
//!
//! # `STAGE` is the growth latch, `AGE` the thickness
//!
//! `STAGE` is 0 while a segment may still grow and 1 once it may not
//! (`:36-37`), and `isRandomlyTicking` is exactly `STAGE == 0` (`:113-115`).
//! `AGE` is the thin/thick flag (`:38-39`) and is what the shape reads, not a
//! counter.
//!
//! # What is not here
//!

use super::BlockTags;
use super::{block_tag_matches, BlockGetter, BlockShapeCtx, LevelCtx};
use bbb_protocol::block_types::{
    VanillaGetStateForPlacementOverride, VanillaRandomTickOverride, VANILLA_BLOCK_BAMBOO_ID,
    VANILLA_BLOCK_BAMBOO_SAPLING_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{above, below, with_property, SET_BLOCK_UPDATE_ALL};
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::fluid::{air_block_state_id, block_state_fluid_state, block_state_is_air};

use super::{
    block_type_of, cycle_property, default_block_state_id, get_state_for_placement_override,
    int_property, offset, random_tick_override, state_is_block, string_property, BlockStateAt,
};

/// The property `BambooStalkBlock.AGE` names (`BambooStalkBlock.java:32`,
/// `BlockStateProperties.AGE_1` = `IntegerProperty.create("age", 0, 1)`,
/// `BlockStateProperties.java:91`).
const AGE: &str = "age";

/// The property `BambooStalkBlock.LEAVES` names (`BambooStalkBlock.java:33`,
/// `BlockStateProperties.BAMBOO_LEAVES` = `EnumProperty.create("leaves", …)`,
/// `BlockStateProperties.java:138`).
const LEAVES: &str = "leaves";

/// The property `BambooStalkBlock.STAGE` names (`BambooStalkBlock.java:34`,
/// `BlockStateProperties.STAGE` = `IntegerProperty.create("stage", 0, 1)`,
/// `BlockStateProperties.java:121`).
const STAGE: &str = "stage";

/// `BambooLeaves.NONE.getSerializedName()` (`BambooLeaves.java:6`).
const LEAVES_NONE: &str = "none";

/// `BambooLeaves.SMALL.getSerializedName()` (`BambooLeaves.java:7`).
const LEAVES_SMALL: &str = "small";

/// `BambooLeaves.LARGE.getSerializedName()` (`BambooLeaves.java:8`).
const LEAVES_LARGE: &str = "large";

/// `BambooStalkBlock.MAX_HEIGHT` (`BambooStalkBlock.java:35`).
pub const MAX_HEIGHT: i32 = 16;

/// `BambooStalkBlock.STAGE_GROWING` (`BambooStalkBlock.java:36`).
pub const STAGE_GROWING: i32 = 0;

/// `BambooStalkBlock.STAGE_DONE_GROWING` (`BambooStalkBlock.java:37`).
pub const STAGE_DONE_GROWING: i32 = 1;

/// `BambooStalkBlock.AGE_THIN_BAMBOO` (`BambooStalkBlock.java:38`).
pub const AGE_THIN_BAMBOO: i32 = 0;

/// `BambooStalkBlock.AGE_THICK_BAMBOO` (`BambooStalkBlock.java:39`).
pub const AGE_THICK_BAMBOO: i32 = 1;

/// The bound both random ticks draw against (`BambooStalkBlock.java:120`,
/// `BambooSaplingBlock.java:41`).
pub const GROWTH_DRAW_BOUND: i32 = 3;

/// The raw brightness both random ticks demand at the cell they would fill
/// (`BambooStalkBlock.java:120`, `BambooSaplingBlock.java:41`).
pub const GROWTH_LIGHT_THRESHOLD: i32 = 9;

/// The column height above which `growBamboo` may latch `STAGE` to 1
/// (`BambooStalkBlock.java:209`).
pub const STAGE_LATCH_MIN_HEIGHT: i32 = 11;

/// The chance `growBamboo` latches `STAGE` once past
/// [`STAGE_LATCH_MIN_HEIGHT`] (`BambooStalkBlock.java:209`).
pub const STAGE_LATCH_CHANCE: f32 = 0.25;

/// The column height at which `growBamboo` latches `STAGE` outright
/// (`BambooStalkBlock.java:209`).
pub const STAGE_LATCH_HEIGHT: i32 = 15;

/// The tick delay `BambooStalkBlock.updateShape` arms
/// (`BambooStalkBlock.java:146`).
pub const TICK_DELAY: i32 = 1;

const SUPPORTS_BAMBOO_TAG: &str = "minecraft:supports_bamboo";

fn can_survive_at<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> Option<bool> {
    Some(block_tag_matches(
        level.block_state(below(pos)),
        SUPPORTS_BAMBOO_TAG,
        membership,
    ))
}

/// Whether a stored state's behaviour belongs to `BambooStalkBlock`.
#[must_use]
pub fn is_bamboo_stalk(block_state_id: i32) -> bool {
    block_type_of(block_state_id).is_some_and(|block_type_id| {
        random_tick_override(block_type_id) == Some(VanillaRandomTickOverride::BambooStalkBlock)
    })
}

/// Whether a stored state's behaviour belongs to `BambooSaplingBlock`.
#[must_use]
pub fn is_bamboo_sapling(block_state_id: i32) -> bool {
    block_type_of(block_state_id).is_some_and(|block_type_id| {
        random_tick_override(block_type_id) == Some(VanillaRandomTickOverride::BambooSaplingBlock)
    })
}

fn is_bamboo_block(block_state_id: i32) -> bool {
    state_is_block(block_state_id, VANILLA_BLOCK_BAMBOO_ID)
}

#[must_use]
pub fn can_survive(pos: BlockPos, tags: &dyn BlockTags, block_state_at: BlockStateAt<'_>) -> bool {
    tags.block_in_tag(block_state_at(below(pos)), "minecraft:supports_bamboo")
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::BambooStalkBlock)
    {
        return None;
    }
    let replaced = block_state_at(pos);
    block_type_of(replaced)?;
    if !block_state_fluid_state(replaced).is_empty() {
        return None;
    }

    let below_state = block_state_at(below(pos));
    block_type_of(below_state)?;
    if !tags.block_in_tag(below_state, "minecraft:supports_bamboo") {
        return None;
    }
    if is_bamboo_sapling(below_state) {
        return with_property(default_block_state_id(block_type_id), AGE, "0");
    }
    if is_bamboo_block(below_state) {
        return with_property(
            default_block_state_id(block_type_id),
            AGE,
            if int_property(below_state, AGE)? > 0 {
                "1"
            } else {
                "0"
            },
        );
    }

    let above_state = block_state_at(above(pos));
    block_type_of(above_state)?;
    if !is_bamboo_block(above_state) {
        return Some(default_block_state_id(VANILLA_BLOCK_BAMBOO_SAPLING_ID));
    }
    let age = int_property(above_state, AGE)?;
    with_property(
        default_block_state_id(block_type_id),
        AGE,
        match age {
            0 => "0",
            1 => "1",
            _ => return None,
        },
    )
}

///
/// The `height < 16` guard is inside the loop condition, so the answer
/// saturates at [`MAX_HEIGHT`] instead of walking a tall column to its root.
fn height_below_up_to_max(level: &LevelCtx<'_, '_>, pos: BlockPos) -> i32 {
    let mut height = 0;
    while height < MAX_HEIGHT
        && is_bamboo_block(level.block_state(offset(pos, 0, -(height + 1), 0)))
    {
        height += 1;
    }
    height
}

///
/// The three conjuncts are drawn in vanilla's order and short-circuit as it
/// does, so a stalk with something above it consumes the `nextInt(3)` and then
/// nothing else.
pub(super) fn stalk_random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if int_property(block_state_id, STAGE) != Some(STAGE_GROWING) {
        return;
    }
    let above_pos = above(pos);
    if level.level_random().next_int(GROWTH_DRAW_BOUND) != 0
        || !block_state_is_air(level.block_state(above_pos))
        || level.raw_brightness(above_pos, 0) < GROWTH_LIGHT_THRESHOLD
    {
        return;
    }
    let height = height_below_up_to_max(level, pos) + 1;
    if height < MAX_HEIGHT {
        grow_bamboo(level, pos, block_state_id, height);
    }
}

///
/// `height >= 1` (`:196`) is transcribed rather than folded away even though
/// every 26.1 caller passes at least 1 — `randomTick` adds one to a
/// non-negative count (`:121`) and `performBonemeal` passes a total height
/// (`:185`) — because the guard is the class's own statement about when the
/// leaf shuffle applies.
fn grow_bamboo(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32, height: i32) {
    let below_pos = below(pos);
    let below_state = level.block_state(below_pos);
    let two_below_pos = offset(pos, 0, -2, 0);
    let two_below_state = level.block_state(two_below_pos);

    let mut leaves = LEAVES_NONE;
    if height >= 1 {
        if !is_bamboo_block(below_state)
            || string_property(below_state, LEAVES).as_deref() == Some(LEAVES_NONE)
        {
            leaves = LEAVES_SMALL;
        } else if is_bamboo_block(below_state)
            && string_property(below_state, LEAVES).as_deref() != Some(LEAVES_NONE)
        {
            leaves = LEAVES_LARGE;
            if is_bamboo_block(two_below_state) {
                if let Some(demoted) = with_property(below_state, LEAVES, LEAVES_SMALL) {
                    level.set_block(
                        below_pos,
                        demoted,
                        SET_BLOCK_UPDATE_ALL,
                        crate::neighbor_updater::UPDATE_LIMIT,
                    );
                }
                if let Some(bare) = with_property(two_below_state, LEAVES, LEAVES_NONE) {
                    level.set_block(
                        two_below_pos,
                        bare,
                        SET_BLOCK_UPDATE_ALL,
                        crate::neighbor_updater::UPDATE_LIMIT,
                    );
                }
            }
        }
    }

    let age = if int_property(block_state_id, AGE) != Some(AGE_THICK_BAMBOO)
        && !is_bamboo_block(two_below_state)
    {
        AGE_THIN_BAMBOO
    } else {
        AGE_THICK_BAMBOO
    };
    // `(height < 11 || !(random.nextFloat() < 0.25F)) && height != 15 ? 0 : 1`
    // (`:209`). The `||` short-circuits, so a stalk shorter than eleven draws
    // no float at all -- the draw order is observable and this is it.
    let stage = if (height < STAGE_LATCH_MIN_HEIGHT
        || level
            .level_random()
            .next_float()
            .partial_cmp(&STAGE_LATCH_CHANCE)
            != Some(std::cmp::Ordering::Less))
        && height != STAGE_LATCH_HEIGHT
    {
        STAGE_GROWING
    } else {
        STAGE_DONE_GROWING
    };

    let Some(grown) = with_property(
        default_block_state_id(VANILLA_BLOCK_BAMBOO_ID),
        AGE,
        &age.to_string(),
    )
    .and_then(|state| with_property(state, LEAVES, leaves))
    .and_then(|state| with_property(state, STAGE, &stage.to_string())) else {
        return;
    };
    level.set_block(
        above(pos),
        grown,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}

pub(super) fn stalk_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    if can_survive_at(level, pos, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    }) == Some(false)
    {
        level.destroy_block(pos, true);
    }
}

pub(super) fn stalk_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    match can_survive_at(level, pos, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    }) {
        None => return block_state_id,
        Some(false) => {
            if let Some(block) = BlockTickType::of_block_state(block_state_id) {
                level.schedule_block_tick(pos, block, TICK_DELAY, TickPriority::Normal);
            }
            return block_state_id;
        }
        Some(true) => {}
    }
    let neighbour_is_thicker = match (
        int_property(neighbour_state, AGE),
        int_property(block_state_id, AGE),
    ) {
        (Some(neighbour_age), Some(own_age)) => neighbour_age > own_age,
        _ => false,
    };
    if direction_to_neighbour == Direction::Up
        && is_bamboo_block(neighbour_state)
        && neighbour_is_thicker
    {
        if let Some(cycled) = cycle_property(block_state_id, AGE) {
            return cycled;
        }
    }
    block_state_id
}

pub(super) fn sapling_random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    let above_pos = above(pos);
    if level.level_random().next_int(GROWTH_DRAW_BOUND) != 0
        || !block_state_is_air(level.block_state(above_pos))
        || level.raw_brightness(above_pos, 0) < GROWTH_LIGHT_THRESHOLD
    {
        return;
    }
    sapling_grow_bamboo(level, pos);
}

///
/// A different body from the stalk's: one write, no leaf shuffle, and the new
/// segment always carries `SMALL` leaves.
fn sapling_grow_bamboo(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    let Some(grown) = with_property(
        default_block_state_id(VANILLA_BLOCK_BAMBOO_ID),
        LEAVES,
        LEAVES_SMALL,
    ) else {
        return;
    };
    level.set_block(
        above(pos),
        grown,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}

///
/// Unlike the stalk's, this one removes the block outright instead of arming a
/// tick, and it turns the shoot into a full stalk as soon as bamboo appears
/// directly above it.
pub(super) fn sapling_update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    match can_survive_at(level, pos, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    }) {
        Some(false) => return air_block_state_id(),
        Some(true) => {}
        None => return block_state_id,
    }
    if direction_to_neighbour == Direction::Up && is_bamboo_block(neighbour_state) {
        return default_block_state_id(VANILLA_BLOCK_BAMBOO_ID);
    }
    block_state_id
}

fn height_above_up_to_max(level: &LevelCtx<'_, '_>, pos: BlockPos) -> i32 {
    let mut height = 0;
    while height < MAX_HEIGHT && is_bamboo_block(level.block_state(offset(pos, 0, height + 1, 0))) {
        height += 1;
    }
    height
}

/// `BambooStalkBlock.isValidBonemealTarget` (`BambooStalkBlock.java:155-163`).
pub(super) fn stalk_is_valid_bonemeal_target(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    let height_above = height_above_up_to_max(level, pos);
    let height_below = height_below_up_to_max(level, pos);
    let growth_pos = offset(pos, 0, height_above + 1, 0);
    height_above + height_below + 1 < MAX_HEIGHT
        && int_property(level.block_state(offset(pos, 0, height_above, 0)), STAGE)
            != Some(STAGE_DONE_GROWING)
        && growth_pos.y <= level.get_max_y()
        && block_state_is_air(level.block_state(growth_pos))
}

/// `BambooStalkBlock.performBonemeal` (`:171-189`).
pub(super) fn stalk_perform_bonemeal(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    let base_above = height_above_up_to_max(level, pos);
    let base_total = base_above + height_below_up_to_max(level, pos) + 1;
    let new_bamboo = 1 + level.level_random().next_int(2);
    for step in 0..new_bamboo {
        let height_above = base_above + step;
        let total_height = base_total + step;
        let top_pos = offset(pos, 0, height_above, 0);
        let top_state = level.block_state(top_pos);
        let growth_pos = above(top_pos);
        if total_height >= MAX_HEIGHT
            || int_property(top_state, STAGE) == Some(STAGE_DONE_GROWING)
            || !block_state_is_air(level.block_state(growth_pos))
            || growth_pos.y > level.get_max_y()
        {
            return;
        }
        grow_bamboo(level, top_pos, top_state, total_height);
    }
}

/// `BambooSaplingBlock.isValidBonemealTarget` (`BambooSaplingBlock.java:77-79`).
pub(super) fn sapling_is_valid_bonemeal_target(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    let above_pos = above(pos);
    block_state_is_air(level.block_state(above_pos)) && above_pos.y <= level.get_max_y()
}

/// `BambooSaplingBlock.performBonemeal` (`:87-89`).
pub(super) fn sapling_perform_bonemeal(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    sapling_grow_bamboo(level, pos);
}
