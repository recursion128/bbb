//! Stored output and wire-line lifecycle for `TripWireHookBlock`.

use super::LevelCtx;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::block_types::{
    VanillaGetStateForPlacementOverride, VANILLA_BLOCK_TRIPWIRE_HOOK_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::directions::{opposite, relative};

use super::{
    block_type_of, bool_property, default_block_state_id, direction_property,
    direction_serialized_name, get_state_for_placement_override, BlockStateAt,
};

const ATTACHED: &str = "attached";
const FACING: &str = "facing";
const POWERED: &str = "powered";
const DISARMED: &str = "disarmed";
const WIRE_DIST_MAX: i32 = 42;
const RECHECK_PERIOD: i32 = 10;

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    directions: [Direction; 6],
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::TripWireHookBlock)
    {
        return None;
    }
    let mut state = with_property(default_block_state_id(block_type_id), POWERED, "false")
        .and_then(|state| with_property(state, ATTACHED, "false"))?;
    for direction in directions {
        if matches!(direction, Direction::Down | Direction::Up) {
            continue;
        }
        state = with_property(
            state,
            FACING,
            direction_serialized_name(opposite(direction)),
        )?;
        match super::footing::tripwire_hook_can_survive(state, pos, block_state_at) {
            Some(true) => return Some(state),
            Some(false) => {}
            None => return None,
        }
    }
    None
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

pub(super) fn calculate_state(
    level: &mut LevelCtx<'_, '_>,
    pos: bbb_protocol::packets::BlockPos,
    block_state_id: i32,
    is_being_destroyed: bool,
    can_update: bool,
    wire_source: Option<(i32, i32)>,
) {
    let Some(direction) = direction_property(block_state_id, FACING) else {
        return;
    };
    let was_attached = bool_property(block_state_id, ATTACHED).unwrap_or(false);
    let was_powered = bool_property(block_state_id, POWERED).unwrap_or(false);
    let mut attached = !is_being_destroyed;
    let mut powered = false;
    let mut receiver_distance = 0;
    let mut wire_states = [None; WIRE_DIST_MAX as usize];

    for distance in 1..WIRE_DIST_MAX {
        let test_pos = relative_steps(pos, direction, distance);
        let mut wire_state = level.block_state(test_pos);
        if block_type_of(wire_state) == Some(VANILLA_BLOCK_TRIPWIRE_HOOK_ID) {
            if direction_property(wire_state, FACING) == Some(opposite(direction)) {
                receiver_distance = distance;
            }
            break;
        }

        if let Some((source_distance, source_state)) = wire_source {
            if distance == source_distance {
                wire_state = source_state;
            } else if !super::tripwire::is_tripwire(wire_state) {
                attached = false;
                continue;
            }
        } else if !super::tripwire::is_tripwire(wire_state) {
            attached = false;
            continue;
        }

        let armed = bool_property(wire_state, DISARMED) != Some(true);
        powered |= armed && bool_property(wire_state, POWERED) == Some(true);
        wire_states[distance as usize] = Some(wire_state);
        if wire_source.is_some_and(|(source_distance, _)| source_distance == distance) {
            if let Some(tick_type) = BlockTickType::of_block_state(block_state_id) {
                level.schedule_block_tick(pos, tick_type, RECHECK_PERIOD, TickPriority::Normal);
            }
            attached &= armed;
        }
    }

    attached &= receiver_distance > 1;
    powered &= attached;
    let Some(new_state) = with_property(block_state_id, ATTACHED, bool_name(attached))
        .and_then(|state| with_property(state, POWERED, bool_name(powered)))
    else {
        return;
    };

    if receiver_distance > 0 {
        let receiver_pos = relative_steps(pos, direction, receiver_distance);
        let Some(receiver_state) =
            with_property(new_state, FACING, direction_name(opposite(direction)))
        else {
            return;
        };
        level.set_block(
            receiver_pos,
            receiver_state,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        notify_neighbors(level, receiver_pos, receiver_state);
        if block_type_of(level.block_state(pos)) != Some(VANILLA_BLOCK_TRIPWIRE_HOOK_ID) {
            on_removed(level, pos, new_state);
            return;
        }
        emit_state(
            level,
            receiver_pos,
            attached,
            powered,
            was_attached,
            was_powered,
        );
    }

    emit_state(level, pos, attached, powered, was_attached, was_powered);
    if !is_being_destroyed {
        let Some(source_state) = with_property(new_state, FACING, direction_name(direction)) else {
            return;
        };
        level.set_block(
            pos,
            source_state,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        if can_update {
            notify_neighbors(level, pos, source_state);
        }
    }

    if was_attached != attached {
        for distance in 1..receiver_distance {
            let Some(wire_state) = wire_states[distance as usize] else {
                continue;
            };
            let test_pos = relative_steps(pos, direction, distance);
            let current = level.block_state(test_pos);
            if !super::tripwire::is_tripwire(current)
                && block_type_of(current) != Some(VANILLA_BLOCK_TRIPWIRE_HOOK_ID)
            {
                continue;
            }
            if let Some(updated) = with_property(wire_state, ATTACHED, bool_name(attached)) {
                level.set_block(
                    test_pos,
                    updated,
                    SET_BLOCK_UPDATE_ALL,
                    crate::neighbor_updater::UPDATE_LIMIT,
                );
            }
        }
    }
}

pub(super) fn set_placed_by(
    level: &mut LevelCtx<'_, '_>,
    pos: bbb_protocol::packets::BlockPos,
    block_state_id: i32,
) {
    calculate_state(level, pos, block_state_id, false, false, None);
}

pub(super) fn tick(
    level: &mut LevelCtx<'_, '_>,
    pos: bbb_protocol::packets::BlockPos,
    block_state_id: i32,
) {
    calculate_state(level, pos, block_state_id, false, true, None);
}

pub(super) fn affect_neighbors_after_removal(
    level: &mut LevelCtx<'_, '_>,
    pos: bbb_protocol::packets::BlockPos,
    block_state_id: i32,
    moved_by_piston: bool,
) {
    if !moved_by_piston {
        on_removed(level, pos, block_state_id);
    }
}

fn on_removed(
    level: &mut LevelCtx<'_, '_>,
    pos: bbb_protocol::packets::BlockPos,
    block_state_id: i32,
) {
    let attached = bool_property(block_state_id, ATTACHED).unwrap_or(false);
    let powered = bool_property(block_state_id, POWERED).unwrap_or(false);
    if attached || powered {
        calculate_state(level, pos, block_state_id, true, false, None);
    }
    if powered {
        notify_neighbors(level, pos, block_state_id);
    }
}

fn emit_state(
    level: &mut LevelCtx<'_, '_>,
    pos: bbb_protocol::packets::BlockPos,
    attached: bool,
    powered: bool,
    was_attached: bool,
    was_powered: bool,
) {
    let transition = if powered && !was_powered {
        Some((
            "minecraft:block.tripwire.click_on",
            0.6,
            VanillaGameEvent::BLOCK_ACTIVATE,
        ))
    } else if !powered && was_powered {
        Some((
            "minecraft:block.tripwire.click_off",
            0.5,
            VanillaGameEvent::BLOCK_DEACTIVATE,
        ))
    } else if attached && !was_attached {
        Some((
            "minecraft:block.tripwire.attach",
            0.7,
            VanillaGameEvent::BLOCK_ATTACH,
        ))
    } else if !attached && was_attached {
        Some((
            "minecraft:block.tripwire.detach",
            1.2 / (level.level_random().next_float() * 0.2 + 0.9),
            VanillaGameEvent::BLOCK_DETACH,
        ))
    } else {
        None
    };
    // TripWireHookBlock.java:204-216.
    if let Some((sound, pitch, event)) = transition {
        level.play_block_sound_with_volume_pitch(pos, sound, 0.4, pitch);
        level.block_game_event(event, pos, None);
    }
}

fn notify_neighbors(
    level: &mut LevelCtx<'_, '_>,
    pos: bbb_protocol::packets::BlockPos,
    block_state_id: i32,
) {
    let Some(direction) = direction_property(block_state_id, FACING) else {
        return;
    };
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    level.update_neighbors_at(pos, block_type_id);
    level.update_neighbors_at(relative(pos, opposite(direction)), block_type_id);
}

fn relative_steps(
    pos: bbb_protocol::packets::BlockPos,
    direction: Direction,
    steps: i32,
) -> bbb_protocol::packets::BlockPos {
    let mut answer = pos;
    for _ in 0..steps {
        answer = relative(answer, direction);
    }
    answer
}

const fn bool_name(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

const fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Down => "down",
        Direction::Up => "up",
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
    }
}
