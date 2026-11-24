//! Dispenser/dropper scheduling and the shared source-slot/transfer walk.

use super::LevelCtx;
use bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_DROPPER_ID;
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaGetStateForPlacementOverride, VanillaNeighborChangedOverride,
};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{above, with_property};
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};

use super::{
    block_type_of, bool_property, default_block_state_id, direction_serialized_name,
    get_state_for_placement_override, SET_BLOCK_UPDATE_CLIENTS,
};

const FACING: &str = "facing";
const TRIGGERED: &str = "triggered";
const TRIGGER_DELAY: i32 = 4;
pub const EMPTY_SOUND: i32 = 1001;

/// `DispenserBlock.neighborChanged` (`DispenserBlock.java`) is unique
/// to that class; `DropperBlock` inherits it, so the generated tag names both.
#[must_use]
pub fn is_dispenser_family(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.neighbor_changed_override)
        == Some(VanillaNeighborChangedOverride::DispenserBlock)
}

/// `DropperBlock` is the one 26.1 block registered as `BlockEntityType.DROPPER`.
#[must_use]
pub fn is_dropper(block_state_id: i32) -> bool {
    bbb_server_types::block_queries::block_entity_type_for_state(block_state_id)
        == Some(VANILLA_BLOCK_ENTITY_TYPE_DROPPER_ID)
}

/// `DispenserBlock.getStateForPlacement` (`DispenserBlock.java`).
pub(super) fn state_for_placement(
    block_type_id: i32,
    nearest_looking_direction: Direction,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::DispenserBlock)
    {
        return None;
    }
    with_property(
        default_block_state_id(block_type_id),
        FACING,
        direction_serialized_name(bbb_server_types::directions::opposite(
            nearest_looking_direction,
        )),
    )
}

pub fn facing(block_state_id: i32) -> Direction {
    match bbb_protocol::block_state_types::vanilla_block_state_property(block_state_id, "facing") {
        Some("south") => Direction::South,
        Some("west") => Direction::West,
        Some("east") => Direction::East,
        Some("up") => Direction::Up,
        Some("down") => Direction::Down,
        _ => Direction::North,
    }
}

/// `DispenserBlock.neighborChanged` (`DispenserBlock.java`).
pub(super) fn neighbor_changed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let powered = neighbor_powered(level, pos);
    let triggered = bool_property(block_state_id, TRIGGERED).unwrap_or(false);
    if powered && !triggered {
        if let Some(block) = BlockTickType::of_block_state(block_state_id) {
            level.schedule_block_tick(pos, block, TRIGGER_DELAY, TickPriority::Normal);
        }
        if let Some(next) = with_property(block_state_id, TRIGGERED, "true") {
            level.set_block(
                pos,
                next,
                SET_BLOCK_UPDATE_CLIENTS,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        }
    } else if !powered && triggered {
        if let Some(next) = with_property(block_state_id, TRIGGERED, "false") {
            level.set_block(
                pos,
                next,
                SET_BLOCK_UPDATE_CLIENTS,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        }
    }
}

fn neighbor_powered(level: &mut LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    level.has_neighbor_signal(pos) || level.has_neighbor_signal(above(pos))
}
