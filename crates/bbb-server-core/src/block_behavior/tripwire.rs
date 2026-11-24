//! Line updates and scheduled occupancy rechecks for `TripWireBlock`.

use super::LevelCtx;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use bbb_protocol::block_types::{
    VanillaGetStateForPlacementOverride, VANILLA_BLOCK_TRIPWIRE_HOOK_ID, VANILLA_BLOCK_TRIPWIRE_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::geometry::Aabb;

use super::level::{BlockEntityQuery, InsideEntity};
use super::{
    block_type_of, bool_property, default_block_state_id, direction_property,
    get_state_for_placement_override, BlockStateAt,
};

const FACING: &str = "facing";
const POWERED: &str = "powered";
const WIRE_DIST_MAX: i32 = 42;

#[must_use]
pub fn is_tripwire(block_state_id: i32) -> bool {
    block_type_of(block_state_id) == Some(VANILLA_BLOCK_TRIPWIRE_ID)
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::TripWireBlock)
    {
        return None;
    }
    let mut state = default_block_state_id(block_type_id);
    if !is_tripwire(state) {
        return None;
    }
    for direction in [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ] {
        state = with_property(
            state,
            horizontal_property(direction)?,
            if should_connect_to(block_state_at(relative(pos, direction)), direction)? {
                "true"
            } else {
                "false"
            },
        )?;
    }
    Some(state)
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, _block_state_id: i32) {
    let state = level.block_state(pos);
    if !is_tripwire(state) || bool_property(state, POWERED) != Some(true) {
        return;
    }
    let should_be_pressed =
        level.entity_count_in_box(entity_box(pos, state), BlockEntityQuery::BlockTriggerAll) > 0;
    check_pressed(level, pos, state, should_be_pressed);
}

pub(super) fn entity_inside(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    state: i32,
    inside: InsideEntity,
) -> bool {
    if bool_property(state, POWERED) != Some(true) && !inside.ignores_block_triggers {
        check_pressed(level, pos, state, true);
    }
    true
}

fn check_pressed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32, pressed: bool) {
    if bool_property(state, POWERED) != Some(pressed) {
        let Some(next) = with_property(state, POWERED, if pressed { "true" } else { "false" })
        else {
            return;
        };
        level.set_block(
            pos,
            next,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        update_source(level, pos, next);
    }
    if pressed {
        let Some(block) = BlockTickType::of_block_state(state) else {
            return;
        };
        level.schedule_block_tick(pos, block, 10, TickPriority::Normal);
    }
}

pub(crate) fn entity_box(pos: BlockPos, block_state_id: i32) -> Aabb {
    let (min_y, max_y) = if bool_property(block_state_id, "attached") == Some(true) {
        (1.0 / 16.0, 2.5 / 16.0)
    } else {
        (0.0, 8.0 / 16.0)
    };
    Aabb {
        min_x: f64::from(pos.x),
        min_y: f64::from(pos.y) + min_y,
        min_z: f64::from(pos.z),
        max_x: f64::from(pos.x) + 1.0,
        max_y: f64::from(pos.y) + max_y,
        max_z: f64::from(pos.z) + 1.0,
    }
}

pub(super) fn update_shape(block_state_id: i32, direction: Direction, neighbour_state: i32) -> i32 {
    let Some(property) = horizontal_property(direction) else {
        return block_state_id;
    };
    let Some(connects) = should_connect_to(neighbour_state, direction) else {
        return block_state_id;
    };
    with_property(
        block_state_id,
        property,
        if connects { "true" } else { "false" },
    )
    .unwrap_or(block_state_id)
}

pub(super) fn on_place(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    old_state: i32,
) {
    if block_type_of(old_state) != Some(VANILLA_BLOCK_TRIPWIRE_ID) {
        update_source(level, pos, block_state_id);
    }
}

pub(super) fn affect_neighbors_after_removal(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    moved_by_piston: bool,
) {
    if moved_by_piston {
        return;
    }
    if let Some(powered) = with_property(block_state_id, POWERED, "true") {
        update_source(level, pos, powered);
    }
}

fn update_source(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    for direction in [Direction::South, Direction::West] {
        for distance in 1..WIRE_DIST_MAX {
            let test_pos = relative_steps(pos, direction, distance);
            let state = level.block_state(test_pos);
            if block_type_of(state) == Some(VANILLA_BLOCK_TRIPWIRE_HOOK_ID) {
                if direction_property(state, FACING) == Some(opposite(direction)) {
                    super::tripwire_hook::calculate_state(
                        level,
                        test_pos,
                        state,
                        false,
                        true,
                        Some((distance, block_state_id)),
                    );
                }
                break;
            }
            if !is_tripwire(state) {
                break;
            }
        }
    }
}

fn should_connect_to(block_state_id: i32, direction: Direction) -> Option<bool> {
    let block_type_id = block_type_of(block_state_id)?;
    if block_type_id == VANILLA_BLOCK_TRIPWIRE_ID {
        return Some(true);
    }
    if block_type_id != VANILLA_BLOCK_TRIPWIRE_HOOK_ID {
        return Some(false);
    }
    Some(direction_property(block_state_id, FACING)? == opposite(direction))
}

const fn horizontal_property(direction: Direction) -> Option<&'static str> {
    match direction {
        Direction::North => Some("north"),
        Direction::South => Some("south"),
        Direction::West => Some("west"),
        Direction::East => Some("east"),
        Direction::Down | Direction::Up => None,
    }
}

fn relative_steps(pos: BlockPos, direction: Direction, steps: i32) -> BlockPos {
    let mut answer = pos;
    for _ in 0..steps {
        answer = relative(answer, direction);
    }
    answer
}
