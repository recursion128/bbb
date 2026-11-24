//! Scheduled reset and waterlogged/removal updates for `LightningRodBlock`.

use super::{BlockShapeCtx, LevelCtx};
use crate::fluid::{with_property, FlowingFluid, SET_BLOCK_UPDATE_ALL};
use bbb_protocol::block_types::VanillaTickOverride;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType, TickPriority};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};

use super::{
    block_type_of, bool_property, default_block_state_id, direction_property,
    direction_serialized_name, tick_override, BlockStateAt,
};

const POWERED: &str = "powered";
const FACING: &str = "facing";
const WATERLOGGED: &str = "waterlogged";

#[must_use]
pub fn is_lightning_rod(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(tick_override)
        == Some(VanillaTickOverride::LightningRodBlock)
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
    if !is_lightning_rod(state) || bool_property(state, POWERED) != Some(false) {
        return None;
    }
    let state = with_property(state, FACING, direction_serialized_name(clicked_face))?;
    with_property(
        state,
        WATERLOGGED,
        if replaced_fluid.is_source() && replaced_fluid.is_same_family(FluidFamily::Water) {
            "true"
        } else {
            "false"
        },
    )
}

#[must_use]
pub fn signal(block_state_id: i32) -> i32 {
    if bool_property(block_state_id, POWERED) == Some(true) {
        15
    } else {
        0
    }
}

#[must_use]
pub fn direct_signal(block_state_id: i32, direction: Direction) -> i32 {
    if signal(block_state_id) > 0 && direction_property(block_state_id, FACING) == Some(direction) {
        15
    } else {
        0
    }
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
    block_state_id
}

fn update_neighbours(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(facing) = direction_property(block_state_id, FACING) else {
        return;
    };
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    level.update_neighbors_at(relative(pos, opposite(facing)), block_type_id);
}

/// `LightningRodBlock.onLightningStrike` (`LightningRodBlock.java:84-89`).
pub(crate) fn on_lightning_strike(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    let Some(powered) = with_property(block_state_id, POWERED, "true") else {
        return;
    };
    level.set_block(
        pos,
        powered,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    update_neighbours(level, pos, block_state_id);
    if let Some(tick_type) = BlockTickType::of_block_state(block_state_id) {
        level.schedule_block_tick(pos, tick_type, 8, TickPriority::Normal);
    }
    let axis = match direction_property(block_state_id, FACING) {
        Some(Direction::West | Direction::East) => 0,
        Some(Direction::Down | Direction::Up) => 1,
        _ => 2,
    };
    // `LevelEvent.PARTICLES_ELECTRIC_SPARK`.
    level.level_event(3002, pos, axis);
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(unpowered) = with_property(block_state_id, POWERED, "false") else {
        return;
    };
    level.set_block(
        pos,
        unpowered,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    update_neighbours(level, pos, block_state_id);
}

pub(super) fn on_place(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    old_state: i32,
) {
    if block_type_of(block_state_id) == block_type_of(old_state)
        || bool_property(block_state_id, POWERED) != Some(true)
    {
        return;
    }
    let Some(tick_type) = BlockTickType::of_block_state(block_state_id) else {
        return;
    };
    if !level.has_scheduled_tick(pos, tick_type) {
        level.schedule_block_tick(pos, tick_type, 8, TickPriority::Normal);
    }
}

pub(super) fn affect_neighbors_after_removal(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    if bool_property(block_state_id, POWERED) == Some(true) {
        update_neighbours(level, pos, block_state_id);
    }
}
