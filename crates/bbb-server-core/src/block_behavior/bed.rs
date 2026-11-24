//! Paired-half state propagation for the sixteen Bed blocks.

use super::LevelCtx;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::neighbor_updater::{UPDATE_LIMIT, UPDATE_SHAPE_ORDER};
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaBlockUseWithoutItemOverride, VANILLA_BLOCK_AIR_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::air_block_state_id;

use super::{block_type_of, bool_property, direction_property, string_property};

const FACING: &str = "facing";
const PART: &str = "part";
const OCCUPIED: &str = "occupied";
const HEAD: &str = "head";
const FOOT: &str = "foot";

#[must_use]
pub fn is_bed(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.use_without_item_override)
        == Some(VanillaBlockUseWithoutItemOverride::BedBlock)
}

pub(super) fn update_shape(
    block_state_id: i32,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    let Some(facing) = direction_property(block_state_id, FACING) else {
        return block_state_id;
    };
    let Some(part) = string_property(block_state_id, PART) else {
        return block_state_id;
    };
    let partner_direction = match part.as_str() {
        FOOT => facing,
        HEAD => opposite(facing),
        _ => return block_state_id,
    };
    if direction_to_neighbour != partner_direction {
        return block_state_id;
    }
    let same_block = block_type_of(neighbour_state) == block_type_of(block_state_id);
    let opposite_part =
        string_property(neighbour_state, PART).is_some_and(|neighbour_part| neighbour_part != part);
    if !same_block || !opposite_part {
        return air_block_state_id();
    }
    let Some(occupied) = bool_property(neighbour_state, OCCUPIED) else {
        return block_state_id;
    };
    with_property(
        block_state_id,
        OCCUPIED,
        if occupied { "true" } else { "false" },
    )
    .unwrap_or(block_state_id)
}

///
/// A bed's foot faces the way the player faces, and the head goes one cell
/// further in that same direction (`:197`, `:299`).
#[must_use]
pub fn state_for_placement(block_type_id: i32, horizontal_direction: Direction) -> Option<i32> {
    with_property(
        super::default_block_state_id(block_type_id),
        FACING,
        super::direction_serialized_name(horizontal_direction),
    )
}

pub(super) fn set_placed_by(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some((facing, head)) = head_state_after_placement(block_state_id) else {
        return;
    };
    level.set_block(
        relative(pos, facing),
        head,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.update_neighbors_at(pos, VANILLA_BLOCK_AIR_ID);
    for direction in UPDATE_SHAPE_ORDER {
        level.neighbor_shape_changed(
            opposite(direction),
            relative(pos, direction),
            pos,
            block_state_id,
            SET_BLOCK_UPDATE_ALL,
            UPDATE_LIMIT,
        );
    }
}

#[must_use]
pub fn head_state_after_placement(block_state_id: i32) -> Option<(Direction, i32)> {
    if !is_bed(block_state_id) || string_property(block_state_id, PART)? != FOOT {
        return None;
    }
    let facing = direction_property(block_state_id, FACING)?;
    Some((facing, with_property(block_state_id, PART, HEAD)?))
}
