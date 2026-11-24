//! `CreakingHeartBlock` (`net/minecraft/world/level/block/CreakingHeartBlock.java`).

use super::BlockTags;
use super::{BlockShapeCtx, LevelCtx};

use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use bbb_protocol::block_types::{vanilla_block_type, VanillaGetStateForPlacementOverride};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::directions::relative;

use super::{block_type_of, default_block_state_id, string_property};

pub const AXIS: &str = "axis";
pub const STATE: &str = "creaking_heart_state";
// `CreakingHeartState.java:5-24`.
pub const UPROOTED: &str = "uprooted";
pub const DORMANT: &str = "dormant";
pub const AWAKE: &str = "awake";
const PALE_OAK_LOGS_TAG: &str = "minecraft:pale_oak_logs";

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    clicked_face: Direction,
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    if vanilla_block_type(block_type_id)?.get_state_for_placement_override
        != Some(VanillaGetStateForPlacementOverride::CreakingHeartBlock)
    {
        return None;
    }
    let (axis, directions) = match clicked_face {
        Direction::West | Direction::East => ("x", [Direction::East, Direction::West]),
        Direction::Down | Direction::Up => ("y", [Direction::Up, Direction::Down]),
        Direction::North | Direction::South => ("z", [Direction::South, Direction::North]),
    };
    let state = with_property(default_block_state_id(block_type_id), AXIS, axis)?;
    for direction in directions {
        let neighbour = block_state_at(relative(pos, direction));
        block_type_of(neighbour)?;
        if !tags.block_in_tag(neighbour, "minecraft:pale_oak_logs") {
            return Some(state);
        }
        if string_property(neighbour, AXIS)?.as_str() != axis {
            return Some(state);
        }
    }
    None
}

/// `CreakingHeartBlock.getStateForPlacement` (`CreakingHeartBlock.java:102-123, 138-140`):
/// when sandwiched between two coaxial pale oak logs, activates immediately using
/// the level's `minecraft:gameplay/creaking_active` environment attribute.
pub(crate) fn state_for_placement_with_active(
    block_type_id: i32,
    pos: BlockPos,
    clicked_face: Direction,
    creaking_active: bool,
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    if vanilla_block_type(block_type_id)?.get_state_for_placement_override
        != Some(VanillaGetStateForPlacementOverride::CreakingHeartBlock)
    {
        return None;
    }
    let (axis, directions) = match clicked_face {
        Direction::West | Direction::East => ("x", [Direction::East, Direction::West]),
        Direction::Down | Direction::Up => ("y", [Direction::Up, Direction::Down]),
        Direction::North | Direction::South => ("z", [Direction::South, Direction::North]),
    };
    let state = with_property(default_block_state_id(block_type_id), AXIS, axis)?;
    let mut has_logs = true;
    for direction in directions {
        let neighbour = block_state_at(relative(pos, direction));
        block_type_of(neighbour)?;
        if !tags.block_in_tag(neighbour, PALE_OAK_LOGS_TAG)
            || string_property(neighbour, AXIS).as_deref() != Some(axis)
        {
            has_logs = false;
            break;
        }
    }
    if has_logs {
        let next = if creaking_active { AWAKE } else { DORMANT };
        with_property(state, STATE, next)
    } else {
        Some(state)
    }
}

#[must_use]
pub fn ticks_block_entity(block_state_id: i32) -> bool {
    matches!(string_property(block_state_id, STATE).as_deref(), Some(state) if state != UPROOTED)
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    if let Some(block) = BlockTickType::of_block_state(block_state_id) {
        level.schedule_block_tick(pos, block, 1, TickPriority::Normal);
    }
    block_state_id
}

#[must_use]
pub(crate) fn has_required_logs(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) -> Option<bool> {
    let axis = string_property(block_state_id, AXIS)?;
    let directions = match axis.as_str() {
        "x" => [Direction::West, Direction::East],
        "y" => [Direction::Down, Direction::Up],
        "z" => [Direction::North, Direction::South],
        _ => return None,
    };
    Some(directions.into_iter().all(|direction| {
        let neighbour = level.block_state(relative(pos, direction));
        level.block_state_in_tag(neighbour, PALE_OAK_LOGS_TAG)
            && string_property(neighbour, AXIS).as_deref() == Some(axis.as_str())
    }))
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if string_property(block_state_id, AXIS).is_none() {
        return;
    }
    if has_required_logs(level, pos, block_state_id) != Some(true)
        || string_property(block_state_id, STATE).as_deref() != Some(UPROOTED)
    {
        return;
    }

    // CreakingHeartBlock.java:100-107.
    let next = if level.creaking_active(pos) {
        AWAKE
    } else {
        DORMANT
    };
    if let Some(block_state_id) = with_property(block_state_id, STATE, next) {
        level.set_block(
            pos,
            block_state_id,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}
