//! `BuddingAmethystBlock` and `AmethystClusterBlock`
//! (`net/minecraft/world/level/block/BuddingAmethystBlock.java`,
//! `AmethystClusterBlock.java`).
//!
//! One family: the budding block's two draws walk a bud through four stages
//! one face at a time, and the four blocks it grows hold onto the face they
//! grew from.

use super::{BlockShapeCtx, LevelCtx};
use crate::block_getter::BlockGetter;

use bbb_protocol::block_state_types::VanillaBlockSupportType as SupportType;
use bbb_protocol::block_types::{
    VanillaCanSurviveOverride, VANILLA_BLOCK_AMETHYST_CLUSTER_ID,
    VANILLA_BLOCK_LARGE_AMETHYST_BUD_ID, VANILLA_BLOCK_MEDIUM_AMETHYST_BUD_ID,
    VANILLA_BLOCK_SMALL_AMETHYST_BUD_ID, VANILLA_BLOCK_WATER_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_protocol::registry_data_types::VanillaFluidIdentity;

use crate::fluid::{FlowingFluid, SET_BLOCK_UPDATE_ALL};
use bbb_server_types::chunk_ticks::{FluidTickType, TickPriority};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::{block_state_is_air, FluidFamily};

use super::{
    air_block_state_id, block_state_fluid_state, block_type_of, bool_property,
    can_survive_override, default_block_state_id, direction_property, direction_serialized_name,
    face_sturdy, face_sturdy_at, state_is_block, with_property, BlockStateAt, DIRECTION_VALUES,
};

/// `BuddingAmethystBlock.GROWTH_CHANCE` (`BuddingAmethystBlock.java:14`).
const GROWTH_CHANCE: i32 = 5;

/// `AmethystClusterBlock.FACING` (`AmethystClusterBlock.java:34`).
const FACING: &str = "facing";

/// `AmethystClusterBlock.WATERLOGGED` (`AmethystClusterBlock.java:33`).
const WATERLOGGED: &str = "waterlogged";

#[must_use]
pub fn is_amethyst_cluster(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(can_survive_override)
        == Some(VanillaCanSurviveOverride::AmethystClusterBlock)
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    clicked_face: Direction,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    let replaced = block_state_at(pos);
    block_type_of(replaced)?;
    let replaced_fluid = block_state_fluid_state(replaced);

    let state = default_block_state_id(block_type_id);
    if !is_amethyst_cluster(state) {
        return None;
    }
    let state = with_property(
        state,
        WATERLOGGED,
        if replaced_fluid.identity() == VanillaFluidIdentity::Water {
            "true"
        } else {
            "false"
        },
    )?;
    with_property(state, FACING, direction_serialized_name(clicked_face))
}

fn can_cluster_grow_at_state(block_state_id: i32) -> bool {
    block_state_is_air(block_state_id)
        || state_is_block(block_state_id, VANILLA_BLOCK_WATER_ID)
            && block_state_fluid_state(block_state_id).is_full()
}

fn stage_facing(block_state_id: i32, block_type_id: i32, grow_direction: Direction) -> bool {
    state_is_block(block_state_id, block_type_id)
        && direction_property(block_state_id, FACING) == Some(grow_direction)
}

///
/// Two draws in order: one in five decides that anything happens at all, and
/// the second picks the face out of `Direction.values()`, so the stream is
/// observable even when the face it lands on grows nothing.
pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    if level.level_random().next_int(GROWTH_CHANCE) != 0 {
        return;
    }
    let faces = i32::try_from(DIRECTION_VALUES.len()).expect("six faces");
    let index = level.level_random().next_int(faces);
    let grow_direction = DIRECTION_VALUES[usize::try_from(index).expect("a non-negative index")];
    let grow_pos = relative(pos, grow_direction);
    let relative_state = level.block_state(grow_pos);

    let next_stage = if can_cluster_grow_at_state(relative_state) {
        VANILLA_BLOCK_SMALL_AMETHYST_BUD_ID
    } else if stage_facing(
        relative_state,
        VANILLA_BLOCK_SMALL_AMETHYST_BUD_ID,
        grow_direction,
    ) {
        VANILLA_BLOCK_MEDIUM_AMETHYST_BUD_ID
    } else if stage_facing(
        relative_state,
        VANILLA_BLOCK_MEDIUM_AMETHYST_BUD_ID,
        grow_direction,
    ) {
        VANILLA_BLOCK_LARGE_AMETHYST_BUD_ID
    } else if stage_facing(
        relative_state,
        VANILLA_BLOCK_LARGE_AMETHYST_BUD_ID,
        grow_direction,
    ) {
        VANILLA_BLOCK_AMETHYST_CLUSTER_ID
    } else {
        return;
    };

    // `getFluidState().is(Fluids.WATER)` is the source singleton by identity,
    // so a flowing cell grows a dry bud.
    let waterlogged =
        block_state_fluid_state(relative_state).identity() == VanillaFluidIdentity::Water;
    let target = default_block_state_id(next_stage);
    let Some(target) = with_property(target, FACING, direction_serialized_name(grow_direction))
    else {
        return;
    };
    let Some(target) = with_property(
        target,
        WATERLOGGED,
        if waterlogged { "true" } else { "false" },
    ) else {
        return;
    };
    level.set_block(
        grow_pos,
        target,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}

#[must_use]
pub fn can_survive(
    block_state_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt<'_>,
) -> Option<bool> {
    let facing = direction_property(block_state_id, FACING)?;
    let adjacent_pos = relative(pos, opposite(facing));
    face_sturdy(block_state_at(adjacent_pos), facing, SupportType::Full)
}

fn can_survive_at<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
) -> Option<bool> {
    let facing = direction_property(block_state_id, FACING)?;
    let support_pos = relative(pos, opposite(facing));
    Some(face_sturdy_at(
        level.block_state(support_pos),
        support_pos,
        facing,
        SupportType::Full,
    ))
}

pub(super) fn can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
) -> Option<bool> {
    let facing = direction_property(block_state_id, FACING)?;
    let support_pos = relative(pos, opposite(facing));
    Some(face_sturdy_at(
        level.block_state(support_pos),
        support_pos,
        facing,
        SupportType::Full,
    ))
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
    let Some(facing) = direction_property(block_state_id, FACING) else {
        return block_state_id;
    };
    if direction_to_neighbour != opposite(facing) {
        return block_state_id;
    }
    if can_survive_at(level, block_state_id, pos) != Some(false) {
        block_state_id
    } else {
        air_block_state_id()
    }
}
