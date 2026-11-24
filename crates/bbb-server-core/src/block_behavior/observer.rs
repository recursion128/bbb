//! The Observer's scheduled pulse consumer.

use super::{BlockShapeCtx, LevelCtx};
use crate::fluid::with_property;
use bbb_protocol::block_types::VanillaGetStateForPlacementOverride;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::directions::{opposite, relative};

use super::{
    block_type_of, bool_property, default_block_state_id, direction_property,
    direction_serialized_name, get_state_for_placement_override, SET_BLOCK_UPDATE_CLIENTS,
};

const POWERED: &str = "powered";
const FACING: &str = "facing";
const PULSE_TICKS: i32 = 2;
const UPDATE_CLIENTS_AND_KNOWN_SHAPE: i32 = 18;

pub(super) fn state_for_placement(
    block_type_id: i32,
    nearest_looking_direction: Direction,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::ObserverBlock)
    {
        return None;
    }
    with_property(
        default_block_state_id(block_type_id),
        FACING,
        direction_serialized_name(nearest_looking_direction),
    )
}

#[must_use]
pub fn signal(block_state_id: i32, direction: Direction) -> i32 {
    if bool_property(block_state_id, POWERED) == Some(true)
        && direction_property(block_state_id, FACING) == Some(direction)
    {
        15
    } else {
        0
    }
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    if direction_property(block_state_id, FACING) != Some(direction_to_neighbour)
        || bool_property(block_state_id, POWERED) != Some(false)
    {
        return block_state_id;
    }
    let Some(tick_type) = BlockTickType::of_block_state(block_state_id) else {
        return block_state_id;
    };
    if !level.has_scheduled_tick(pos, tick_type) {
        level.schedule_block_tick(pos, tick_type, PULSE_TICKS, TickPriority::Normal);
    }
    block_state_id
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(powered) = bool_property(block_state_id, POWERED) else {
        return;
    };
    let Some(next_state) = with_property(
        block_state_id,
        POWERED,
        if powered { "false" } else { "true" },
    ) else {
        return;
    };
    let tick_type = if powered {
        None
    } else {
        let Some(tick_type) = BlockTickType::of_block_state(block_state_id) else {
            return;
        };
        Some(tick_type)
    };

    level.set_block(
        pos,
        next_state,
        SET_BLOCK_UPDATE_CLIENTS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    if let Some(tick_type) = tick_type {
        level.schedule_block_tick(pos, tick_type, PULSE_TICKS, TickPriority::Normal);
    }
    update_neighbors_in_front(level, pos, next_state);
}

fn update_neighbors_in_front(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(facing) = direction_property(block_state_id, FACING) else {
        return;
    };
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    let output_pos = relative(pos, opposite(facing));
    level.neighbor_changed(output_pos, block_type_id);
    level.update_neighbors_at_except_from_facing(output_pos, block_type_id, facing);
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
    if level.has_scheduled_tick(pos, tick_type) {
        return;
    }
    let Some(unpowered) = with_property(block_state_id, POWERED, "false") else {
        return;
    };
    level.set_block(
        pos,
        unpowered,
        UPDATE_CLIENTS_AND_KNOWN_SHAPE,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    update_neighbors_in_front(level, pos, unpowered);
}

pub(super) fn affect_neighbors_after_removal(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    if bool_property(block_state_id, POWERED) != Some(true) {
        return;
    }
    let Some(tick_type) = BlockTickType::of_block_state(block_state_id) else {
        return;
    };
    if !level.has_scheduled_tick(pos, tick_type) {
        return;
    }
    if let Some(unpowered) = with_property(block_state_id, POWERED, "false") {
        update_neighbors_in_front(level, pos, unpowered);
    }
}
