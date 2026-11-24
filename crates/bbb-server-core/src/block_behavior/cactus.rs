//! `CactusBlock` (`net/minecraft/world/level/block/CactusBlock.java`).
//!
//! The same column shape as [`super::sugar_cane`] with two additions: at age 8
//! the head of the column may grow a `cactus_flower` instead of more cactus,
//! and the age counter keeps running in the same tick that grew a block, so a
//! cactus that just pushed a neighbour upward is left at age 1 rather than 0.
//!
//! `randomTick` (`:54-82`), `canSurvive` (`:113-123`), `updateShape`
//! (`:95-110`) and `tick` (`:47-51`) are all here; the last is the same
//! `Level.destroyBlock` body [`super::sugar_cane::tick`] runs.
//!
//! # The two writes at age 15 are not the same write
//!
//! `randomTick`'s `age == 15 && height < 3` branch (`:71-76`) writes `AGE = 0`
//! at the ticking position and then *falls through* to the `age < 15` guard
//! (`:78-80`), which is false, so the reset stands; the `age == 8` branch
//! (`:66-70`) does not write at the ticking position at all and therefore does
//! fall into the `age < 15` increment. Both are single statements in vanilla
//! and both are transcribed literally rather than folded.
//!
//! # What is not here
//!
//! * `entityInside` (`:126-130`), the one point of contact damage; it needs an
//!   `Entity` and belongs to the entity slice.
//! * `getShape`/`getCollisionShape` (`:85-92`) and `isPathfindable`
//!   (`:138-140`), all collision surfaces.

use super::BlockTags;
use super::{block_tag_matches, fluid_tag_matches, BlockGetter, BlockShapeCtx, LevelCtx};
use bbb_protocol::block_types::{VanillaRandomTickOverride, VANILLA_BLOCK_CACTUS_FLOWER_ID};
use bbb_protocol::packets::BlockPos;

use crate::fluid::{above, below, HORIZONTAL, SET_BLOCK_UPDATE_ALL};
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::directions::relative;
use bbb_server_types::fluid::{block_state_fluid_state, block_state_is_air, block_state_is_solid};

use super::{
    block_type_of, default_block_state_id, int_property, offset, random_tick_override,
    state_is_block, state_liquid, with_property, BlockStateAt, SET_BLOCK_UPDATE_NONE,
};

/// The property `CactusBlock.AGE` names (`CactusBlock.java:27`,
/// `BlockStateProperties.AGE_15`, `BlockStateProperties.java:97`).
const AGE: &str = "age";
const SUPPORTS_CACTUS_TAG: &str = "minecraft:supports_cactus";
const LAVA_TAG: &str = "minecraft:lava";

/// `CactusBlock.MAX_AGE` (`CactusBlock.java:28`).
pub const MAX_AGE: i32 = 15;

/// `CactusBlock.MAX_CACTUS_GROWING_HEIGHT` (`CactusBlock.java:31`).
pub const MAX_CACTUS_GROWING_HEIGHT: i32 = 3;

/// `CactusBlock.ATTEMPT_GROW_CACTUS_FLOWER_AGE` (`CactusBlock.java:32`).
pub const ATTEMPT_GROW_CACTUS_FLOWER_AGE: i32 = 8;

/// `CactusBlock.ATTEMPT_GROW_CACTUS_FLOWER_SMALL_CACTUS_CHANCE`
/// (`CactusBlock.java:33`).
pub const ATTEMPT_GROW_CACTUS_FLOWER_SMALL_CACTUS_CHANCE: f64 = 0.1;

/// `CactusBlock.ATTEMPT_GROW_CACTUS_FLOWER_TALL_CACTUS_CHANCE`
/// (`CactusBlock.java:34`).
pub const ATTEMPT_GROW_CACTUS_FLOWER_TALL_CACTUS_CHANCE: f64 = 0.25;

/// The tick delay `updateShape` arms (`CactusBlock.java:106`).
pub const TICK_DELAY: i32 = 1;

/// Whether a stored state's behaviour belongs to `CactusBlock`.
///
/// `CactusBlock` has no subclass in 26.1, so the generated
/// `random_tick_override` class tag names the class outright.
#[must_use]
pub fn is_cactus(block_state_id: i32) -> bool {
    block_type_of(block_state_id).is_some_and(|block_type_id| {
        random_tick_override(block_type_id) == Some(VanillaRandomTickOverride::CactusBlock)
    })
}

///
/// The height walk here is not [`super::sugar_cane`]'s: the `++height == 3 &&
/// age == 15` test sits *inside* the loop (`:61-63`) and returns outright, so
/// a full-grown cactus whose column is already three tall does not even age.
pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let above_pos = above(pos);
    if !block_state_is_air(level.block_state(above_pos)) {
        return;
    }
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    let Some(age) = int_property(block_state_id, AGE) else {
        return;
    };

    let mut height = 1;
    while state_is_block(level.block_state(offset(pos, 0, -height, 0)), block_type_id) {
        height += 1;
        if height == MAX_CACTUS_GROWING_HEIGHT && age == MAX_AGE {
            return;
        }
    }

    // `age == 8 && this.canSurvive(this.defaultBlockState(), level, pos.above())`
    // (`:66`), short-circuited exactly as vanilla writes it: the support test
    // asks about the *cell above*, i.e. whether a cactus flower could stand
    // where the growth would go.
    let may_flower = age == ATTEMPT_GROW_CACTUS_FLOWER_AGE
        && can_survive_at(
            level,
            default_block_state_id(block_type_id),
            above_pos,
            &|registry, tag, id| level.registry_tag_membership(registry, tag, id),
        );
    if may_flower {
        let chance_to_grow_flower = if height >= MAX_CACTUS_GROWING_HEIGHT {
            ATTEMPT_GROW_CACTUS_FLOWER_TALL_CACTUS_CHANCE
        } else {
            ATTEMPT_GROW_CACTUS_FLOWER_SMALL_CACTUS_CHANCE
        };
        if level.level_random().next_double() <= chance_to_grow_flower {
            level.set_block(
                above_pos,
                default_block_state_id(VANILLA_BLOCK_CACTUS_FLOWER_ID),
                SET_BLOCK_UPDATE_ALL,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        }
    } else if age == MAX_AGE && height < MAX_CACTUS_GROWING_HEIGHT {
        level.set_block(
            above_pos,
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
        level.neighbor_changed(above_pos, block_type_id);
    }

    if age < MAX_AGE {
        if let Some(older) = with_property(block_state_id, AGE, &(age + 1).to_string()) {
            level.set_block(
                pos,
                older,
                SET_BLOCK_UPDATE_NONE,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        }
    }
}

#[must_use]
pub fn can_survive(
    block_state_id: i32,
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> bool {
    for direction in HORIZONTAL {
        let neighbor = block_state_at(relative(pos, direction));
        if block_state_is_solid(neighbor)
            || tags.fluid_in_tag(
                block_state_fluid_state(neighbor).identity(),
                "minecraft:lava",
            )
        {
            return false;
        }
    }
    let below_state = block_state_at(below(pos));
    // `belowState.is(this)` (`:122`) is the ticking block, not "any
    // `CactusBlock`".
    (block_type_of(below_state) == block_type_of(block_state_id)
        || tags.block_in_tag(below_state, "minecraft:supports_cactus"))
        && !state_liquid(block_state_at(above(pos)))
}

fn can_survive_at<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> bool {
    for direction in HORIZONTAL {
        let neighbor = level.block_state(relative(pos, direction));
        if block_state_is_solid(neighbor)
            || fluid_tag_matches(
                block_state_fluid_state(neighbor).identity(),
                LAVA_TAG,
                membership,
            )
        {
            return false;
        }
    }
    if state_liquid(level.block_state(above(pos))) {
        return false;
    }
    let below_state = level.block_state(below(pos));
    block_type_of(below_state) == block_type_of(block_state_id)
        || block_tag_matches(below_state, SUPPORTS_CACTUS_TAG, membership)
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
