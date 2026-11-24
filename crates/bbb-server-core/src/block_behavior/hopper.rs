//! Hopper placement, enabled state and analog output.

use super::LevelCtx;
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaGetStateForPlacementOverride, VanillaNeighborChangedOverride,
};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::with_property;
use bbb_server_types::directions::opposite;

use super::{
    block_type_of, bool_property, default_block_state_id, direction_serialized_name,
    get_state_for_placement_override, SET_BLOCK_UPDATE_CLIENTS,
};

const FACING: &str = "facing";
const ENABLED: &str = "enabled";

/// Whether a stored state is `minecraft:hopper`.
///
#[must_use]
pub fn is_hopper(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.neighbor_changed_override)
        == Some(VanillaNeighborChangedOverride::HopperBlock)
}

pub(super) fn state_for_placement(block_type_id: i32, clicked_face: Direction) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::HopperBlock)
    {
        return None;
    }
    let direction = opposite(clicked_face);
    let facing = if matches!(direction, Direction::Down | Direction::Up) {
        Direction::Down
    } else {
        direction
    };
    let state = with_property(
        default_block_state_id(block_type_id),
        FACING,
        direction_serialized_name(facing),
    )?;
    with_property(state, ENABLED, "true")
}

fn check_powered_state(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let should_be_on = !level.has_neighbor_signal(pos);
    let enabled = bool_property(block_state_id, ENABLED).unwrap_or(true);
    if should_be_on != enabled {
        if let Some(next) = with_property(
            block_state_id,
            ENABLED,
            if should_be_on { "true" } else { "false" },
        ) {
            level.set_block(
                pos,
                next,
                SET_BLOCK_UPDATE_CLIENTS,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        }
    }
}

pub(super) fn neighbor_changed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    check_powered_state(level, pos, block_state_id);
}

pub(super) fn on_place(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    old_state: i32,
) {
    if block_type_of(old_state) == block_type_of(block_state_id) {
        return;
    }
    check_powered_state(level, pos, block_state_id);
}
