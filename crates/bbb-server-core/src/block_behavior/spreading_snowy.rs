//! `SpreadingSnowyBlock` and its base `SnowyBlock`
//! (`net/minecraft/world/level/block/SpreadingSnowyBlock.java`,
//! `SnowyBlock.java`).
//!
//!

use super::BlockTags;
use super::{BlockShapeCtx, LevelCtx};

use bbb_protocol::block_types::{
    VanillaGetStateForPlacementOverride, VANILLA_BLOCK_DIRT_ID, VANILLA_BLOCK_GRASS_BLOCK_ID,
    VANILLA_BLOCK_MYCELIUM_ID, VANILLA_BLOCK_SNOW_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::SET_BLOCK_UPDATE_ALL;

use super::{
    above, block_type_of, default_block_state_id, get_state_for_placement_override, int_property,
    light_dampening, offset, state_is_block, with_property, BlockStateAt,
};

/// The brightness `randomTick` requires above a block before it spreads
/// (`SpreadingSnowyBlock.java:57`).
pub const SPREAD_LIGHT_THRESHOLD: i32 = 9;

/// The `getLightBlockInto` result at which a block stops being able to stay
/// alive (`SpreadingSnowyBlock.java:41`, `lightBlockInto < 15`).
pub const OPACITY_CUTOFF: i32 = 15;

/// The number of spread attempts one random tick makes
/// (`SpreadingSnowyBlock.java:60`).
pub const SPREAD_ATTEMPTS: i32 = 4;

const SNOWY: &str = "snowy";
const LAYERS: &str = "layers";
const SNOW_TAG: &str = "minecraft:snow";
const WATER_TAG: &str = "minecraft:water";

#[must_use]
pub fn is_snowy(block_state_id: i32) -> bool {
    block_type_of(block_state_id).is_some_and(|block_type_id| {
        get_state_for_placement_override(block_type_id)
            == Some(VanillaGetStateForPlacementOverride::SnowyBlock)
    })
}

/// Whether a `SnowyBlock` is one of the two `SpreadingSnowyBlock` subclasses,
/// i.e. whether it also answers [`random_tick`].
///
#[must_use]
pub fn is_spreading(block_state_id: i32) -> bool {
    base_block_type_id(block_state_id).is_some()
}

///
/// `None` for a state that is not one of the two owners.
#[must_use]
pub fn base_block_type_id(block_state_id: i32) -> Option<i32> {
    match block_type_of(block_state_id)? {
        VANILLA_BLOCK_GRASS_BLOCK_ID | VANILLA_BLOCK_MYCELIUM_ID => Some(VANILLA_BLOCK_DIRT_ID),
        _ => None,
    }
}

#[must_use]
pub fn is_snowy_setting(tags: &dyn BlockTags, above_state: i32) -> bool {
    tags.block_in_tag(above_state, "minecraft:snow")
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    let above_state = block_state_at(above(pos));
    block_type_of(above_state)?;
    let state = default_block_state_id(block_type_id);
    if !is_snowy(state) {
        return None;
    }
    with_property(
        state,
        SNOWY,
        if is_snowy_setting(tags, above_state) {
            "true"
        } else {
            "false"
        },
    )
}

///
/// Three branches in order: a single snow layer on top is always survivable, a
/// full fluid on top is always fatal, and otherwise the block survives while
/// light still gets in from above.
#[must_use]
pub(crate) fn can_stay_alive(level: &LevelCtx<'_, '_>, block_state_id: i32, pos: BlockPos) -> bool {
    let above_pos = above(pos);
    let above_state = level.block_state(above_pos);
    if state_is_block(above_state, VANILLA_BLOCK_SNOW_ID)
        && int_property(above_state, LAYERS) == Some(1)
    {
        return true;
    }
    if level.fluid_state(above_pos).is_full() {
        return false;
    }
    let opacity = level.light_block_into(
        block_state_id,
        above_state,
        Direction::Up,
        light_dampening(above_state),
    );
    opacity < OPACITY_CUTOFF
}

#[must_use]
pub(crate) fn can_propagate(level: &LevelCtx<'_, '_>, block_state_id: i32, pos: BlockPos) -> bool {
    can_stay_alive(level, block_state_id, pos)
        && !level.fluid_identity_in_tag(level.fluid_state(above(pos)).identity(), WATER_TAG)
}

///
/// Note where the draws happen: the four offsets are only drawn once the
/// brightness gate at `:57` has passed, so a dark grass block that is still
/// alive advances the random source not at all. Java evaluates
/// `pos.offset(x, y, z)`'s arguments left to right, so the three draws per
/// attempt are `nextInt(3)`, `nextInt(5)`, `nextInt(3)` in that order.
pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(base_block) = base_block_type_id(block_state_id) else {
        return;
    };
    if !can_stay_alive(level, block_state_id, pos) {
        level.set_block(
            pos,
            default_block_state_id(base_block),
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        return;
    }
    if level.max_block_brightness(above(pos)) < SPREAD_LIGHT_THRESHOLD {
        return;
    }

    // `this.defaultBlockState()`, i.e. `SNOWY=false`, is what both the
    // `canPropagate` probe and the written state start from (`:58`, `:63`).
    let own_type = block_type_of(block_state_id).expect("a spreading block has a registry id");
    let default_state = default_block_state_id(own_type);
    for _ in 0..SPREAD_ATTEMPTS {
        let dx = level.level_random().next_int(3) - 1;
        let dy = level.level_random().next_int(5) - 3;
        let dz = level.level_random().next_int(3) - 1;
        let test_pos = offset(pos, dx, dy, dz);
        if state_is_block(level.block_state(test_pos), base_block)
            && can_propagate(level, default_state, test_pos)
        {
            let snowy = level.block_state_in_tag(level.block_state(above(test_pos)), SNOW_TAG);
            let spread = with_property(default_state, SNOWY, if snowy { "true" } else { "false" })
                .unwrap_or(default_state);
            level.set_block(
                test_pos,
                spread,
                SET_BLOCK_UPDATE_ALL,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        }
    }
}

pub(super) fn update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    if direction_to_neighbour != Direction::Up {
        return block_state_id;
    }
    let snowy = level.block_state_in_tag(neighbour_state, SNOW_TAG);
    with_property(block_state_id, SNOWY, if snowy { "true" } else { "false" })
        .unwrap_or(block_state_id)
}
