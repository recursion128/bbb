use super::BlockTags;
use super::{block_tag_matches, fluid_tag_matches, BlockGetter, BlockShapeCtx, LevelCtx};

use bbb_protocol::block_types::{VANILLA_BLOCK_BUBBLE_COLUMN_ID, VANILLA_BLOCK_WATER_ID};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{above, below, with_property, FlowingFluid};
use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType, TickPriority};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};

use super::{
    bucket_pickup, default_block_state_id, state_is_block, BlockStateAt, SET_BLOCK_UPDATE_CLIENTS,
};

/// `BubbleColumnBlock.DRAG_DOWN` (`BubbleColumnBlock.java:39`), i.e.
/// `BlockStateProperties.DRAG` (`BlockStateProperties.java:17`).
const DRAG: &str = "drag";

/// `BubbleColumnBlock.CHECK_PERIOD` (`BubbleColumnBlock.java:40`).
pub const CHECK_PERIOD: i32 = 5;

/// `LiquidBlock.BUBBLE_COLUMN_CHECK_DELAY` (`LiquidBlock.java:60`).
const LIQUID_CHECK_DELAY: i32 = 20;
const PUSH_UP_TAG: &str = "minecraft:enables_bubble_column_push_up";
const DRAG_DOWN_TAG: &str = "minecraft:enables_bubble_column_drag_down";
const CAN_OCCUPY_TAG: &str = "minecraft:bubble_column_can_occupy";

fn is_column(block_state_id: i32) -> bool {
    state_is_block(block_state_id, VANILLA_BLOCK_BUBBLE_COLUMN_ID)
}

pub(super) fn can_survive(
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt,
) -> bool {
    let below_state = block_state_at(below(pos));
    is_column(below_state)
        || tags.block_in_tag(below_state, "minecraft:enables_bubble_column_push_up")
        || tags.block_in_tag(below_state, "minecraft:enables_bubble_column_drag_down")
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let below_state = level.block_state(below(pos));
    update_column_from(level, pos, block_state_id, below_state);
}

fn can_survive_at<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> bool {
    let below_state = level.block_state(below(pos));
    is_column(below_state)
        || block_tag_matches(below_state, PUSH_UP_TAG, membership)
        || block_tag_matches(below_state, DRAG_DOWN_TAG, membership)
}

pub(crate) fn update_column(level: &mut LevelCtx<'_, '_>, occupy_at: BlockPos, below_state: i32) {
    let occupy_state = level.block_state(occupy_at);
    update_column_from(level, occupy_at, occupy_state, below_state);
}

pub(crate) fn schedule_liquid_bubble_column_tick(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    liquid_state: i32,
    below_state: i32,
) {
    if !liquid_can_start_column_at(liquid_state, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    }) || !(level.block_state_in_tag(below_state, DRAG_DOWN_TAG)
        || level.block_state_in_tag(below_state, PUSH_UP_TAG))
    {
        return;
    }
    if let Some(block) = BlockTickType::of_block_state(liquid_state) {
        level.schedule_block_tick(pos, block, LIQUID_CHECK_DELAY, TickPriority::Normal);
    }
}

pub(crate) fn tick_liquid_bubble_column(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    liquid_state: i32,
) {
    if liquid_can_start_column_at(liquid_state, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    }) {
        let below_state = level.block_state(below(pos));
        update_column(level, pos, below_state);
    }
}

fn update_column_from(
    level: &mut LevelCtx<'_, '_>,
    occupy_at: BlockPos,
    occupy_state: i32,
    below_state: i32,
) {
    if !can_occupy_at(occupy_state, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    }) {
        return;
    }
    let column_state = column_state(below_state, occupy_state, |tag| {
        level.block_state_in_tag(below_state, tag)
    });
    // `:86`: flag 2 is `UPDATE_CLIENTS`.
    level.set_block(
        occupy_at,
        column_state,
        SET_BLOCK_UPDATE_CLIENTS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    let mut pos = above(occupy_at);
    while can_occupy_at(level.block_state(pos), &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    }) {
        if !level.set_block(
            pos,
            column_state,
            SET_BLOCK_UPDATE_CLIENTS,
            crate::neighbor_updater::UPDATE_LIMIT,
        ) {
            return;
        }
        pos = above(pos);
    }
}

fn can_occupy_at(occupy_state: i32, membership: &dyn Fn(&str, &str, &str) -> bool) -> bool {
    is_column(occupy_state) || liquid_can_start_column_at(occupy_state, membership)
}

fn liquid_can_start_column_at(
    occupy_state: i32,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> bool {
    let fluid = block_state_fluid_state(occupy_state);
    bucket_pickup::is_liquid_block(occupy_state)
        && fluid.is_source()
        && fluid.is_full()
        && fluid_tag_matches(fluid.identity(), CAN_OCCUPY_TAG, membership)
}

fn column_state(below_state: i32, occupy_state: i32, mut in_tag: impl FnMut(&str) -> bool) -> i32 {
    if is_column(below_state) {
        return below_state;
    }
    let default = default_block_state_id(VANILLA_BLOCK_BUBBLE_COLUMN_ID);
    if in_tag(PUSH_UP_TAG) {
        return with_property(default, DRAG, "false").unwrap_or(default);
    }
    if in_tag(DRAG_DOWN_TAG) {
        return with_property(default, DRAG, "true").unwrap_or(default);
    }
    if is_column(occupy_state) {
        default_block_state_id(VANILLA_BLOCK_WATER_ID)
    } else {
        occupy_state
    }
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    level.schedule_fluid_tick(
        pos,
        FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
        FlowingFluid::WATER_TICK_DELAY,
        TickPriority::Normal,
    );
    let survives = can_survive_at(level, pos, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    });
    let occupiable_water_arrived_above = direction_to_neighbour == Direction::Up
        && !is_column(neighbour_state)
        && can_occupy_at(neighbour_state, &|registry, tag, id| {
            level.registry_tag_membership(registry, tag, id)
        });
    if !survives || direction_to_neighbour == Direction::Down || occupiable_water_arrived_above {
        if let Some(block) = BlockTickType::of_block_state(block_state_id) {
            level.schedule_block_tick(pos, block, CHECK_PERIOD, TickPriority::Normal);
        }
    }
    block_state_id
}
