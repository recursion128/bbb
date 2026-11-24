//! `StemBlock` (`net/minecraft/world/level/block/StemBlock.java`).
//!
//! The pumpkin and melon stem. It is a `CropBlock` in everything but its class
//! hierarchy — same light threshold, same `CropBlock.getGrowthSpeed` draw
//! (`:84-86`) — right up to the point where it runs out of `AGE`: at 7 the
//! draw stops ageing the stem and instead picks a random horizontal direction,
//! plants the fruit there, and turns the stem itself into the attached stem
//! that leans towards it (`:91-104`).
//!
//! # The three things the class does not know
//!
//!
//! Nothing here is written down per block: `pumpkin_stem` and `melon_stem`
//! reach the same body and differ only in what those facts answer.
//!
//! # What is not here
//!

use super::{BlockShapeCtx, LevelCtx};
use bbb_protocol::block_types::{VanillaMayPlaceOnOverride, VanillaRandomTickOverride};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{below, SET_BLOCK_UPDATE_ALL};
use bbb_server_types::directions::relative;
use bbb_server_types::fluid::block_state_is_air;

use super::crop::{growth_speed, GROWTH_DRAW_NUMERATOR, GROWTH_LIGHT_THRESHOLD};
use super::{
    block_type_of, constructor_block_tag, constructor_block_type_id, default_block_state_id,
    direction_property, direction_serialized_name, int_property, may_place_on_override,
    random_horizontal_direction, random_tick_override, with_property, SET_BLOCK_UPDATE_CLIENTS,
};

/// The property `StemBlock.AGE` names (`StemBlock.java:42`,
/// `BlockStateProperties.AGE_7` = `IntegerProperty.create("age", 0, 7)`,
/// `BlockStateProperties.java:96`).
const AGE: &str = "age";

/// The property `HorizontalDirectionalBlock.FACING` names
/// (`HorizontalDirectionalBlock.java:11`), which is what an attached stem is
/// given when the fruit lands (`StemBlock.java:101`).
const FACING: &str = "facing";

/// `StemBlock.MAX_AGE` (`StemBlock.java:41`).
pub const MAX_AGE: i32 = 7;

/// The instance field `randomTick` reads for the block it plants
/// (`StemBlock.java:44`, used at `:97`).
const FRUIT_FIELD: &str = "fruit";

/// The instance field an attached stem reads when its fruit is removed
/// (`AttachedStemBlock.java:31`, used at `:80-83`).
const STEM_FIELD: &str = "stem";

/// The instance field `randomTick` reads for the stem it leaves behind
/// (`StemBlock.java:45`, used at `:98`).
const ATTACHED_STEM_FIELD: &str = "attachedStem";

/// The instance field `randomTick` reads for the tag the fruit needs under it
/// (`StemBlock.java:48`, used at `:95`).
const FRUIT_SUPPORT_BLOCKS_FIELD: &str = "fruitSupportBlocks";

/// Whether a stored state's behaviour belongs to `StemBlock`.
///
/// `StemBlock` has no subclass in 26.1 — `AttachedStemBlock` is a sibling, not
/// a child — so the generated `random_tick_override` class tag names the class
/// outright.
#[must_use]
pub fn is_stem(block_state_id: i32) -> bool {
    block_type_of(block_state_id).is_some_and(|block_type_id| {
        random_tick_override(block_type_id) == Some(VanillaRandomTickOverride::StemBlock)
    })
}

#[must_use]
pub fn is_attached_stem(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(may_place_on_override)
        == Some(VanillaMayPlaceOnOverride::AttachedStemBlock)
}

pub(super) fn attached_update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    let base_answer = || super::vegetation::update_shape(level, block_state_id, pos);
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return base_answer();
    };
    let Some(facing) = direction_property(block_state_id, FACING) else {
        return base_answer();
    };
    let Some(fruit) = constructor_block_type_id(block_type_id, FRUIT_FIELD) else {
        return base_answer();
    };
    if direction_to_neighbour != facing || block_type_of(neighbour_state) == Some(fruit) {
        return base_answer();
    }
    let Some(stem) = constructor_block_type_id(block_type_id, STEM_FIELD) else {
        return base_answer();
    };
    with_property(default_block_state_id(stem), AGE, &MAX_AGE.to_string())
        .unwrap_or_else(base_answer)
}

///
/// The light gate and the growth draw are `CropBlock`'s, borrowed rather than
/// restated: `getRawBrightness(pos, 0) >= 9` (`:84`) is
/// [`super::crop::GROWTH_LIGHT_THRESHOLD`] and
/// `random.nextInt((int)(25.0F / growthSpeed) + 1)` (`:86`) is the same
/// truncated float division [`super::crop::random_tick`] performs, because
/// `StemBlock` calls `CropBlock.getGrowthSpeed` directly (`:85`).
///
/// Both fruiting writes are `setBlockAndUpdate` (`:100-101`, flag 3), unlike
/// the ageing write above them (`:90`, flag 2): a melon appearing beside a
/// stem is a shape its neighbours have to hear about.
pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if level.raw_brightness(pos, 0) < GROWTH_LIGHT_THRESHOLD {
        return;
    }
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    let Some(speed) = growth_speed(level, block_type_id, pos) else {
        return;
    };
    // Java truncates the float division towards zero before adding one.
    let bound = (GROWTH_DRAW_NUMERATOR / speed) as i32 + 1;
    if level.level_random().next_int(bound) != 0 {
        return;
    }
    let Some(age) = int_property(block_state_id, AGE) else {
        return;
    };
    if age < MAX_AGE {
        if let Some(grown) = with_property(block_state_id, AGE, &(age + 1).to_string()) {
            level.set_block(
                pos,
                grown,
                SET_BLOCK_UPDATE_CLIENTS,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        }
        return;
    }

    // The direction is drawn even when nothing can grow there, so a stem
    // hemmed in on all four sides still consumes one number per successful
    // growth draw (`:92`).
    let direction = random_horizontal_direction(level.level_random());
    let relative_pos = relative(pos, direction);
    // `stateBelow` is read before the `isAir` test (`:94-95`), and only the
    // tag question behind it short-circuits.
    let state_below = level.block_state(below(relative_pos));
    if !block_state_is_air(level.block_state(relative_pos)) {
        return;
    }
    let Some(fruit_support) = constructor_block_tag(block_type_id, FRUIT_SUPPORT_BLOCKS_FIELD)
    else {
        return;
    };
    if !level.block_state_in_tag(state_below, fruit_support) {
        return;
    }
    // `blocks.getOptional(this.fruit)` and `blocks.getOptional(this.attachedStem)`
    // (`:97-98`): both have to resolve before either write happens (`:99`).
    let Some(fruit) = constructor_block_type_id(block_type_id, FRUIT_FIELD) else {
        return;
    };
    let Some(attached_stem) = constructor_block_type_id(block_type_id, ATTACHED_STEM_FIELD) else {
        return;
    };
    let Some(leaning) = with_property(
        default_block_state_id(attached_stem),
        FACING,
        direction_serialized_name(direction),
    ) else {
        return;
    };
    level.set_block(
        relative_pos,
        default_block_state_id(fruit),
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.set_block(
        pos,
        leaning,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}
