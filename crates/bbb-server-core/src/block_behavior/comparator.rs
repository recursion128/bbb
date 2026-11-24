//! `ComparatorBlock` (`net/minecraft/world/level/block/ComparatorBlock.java`).

use super::LevelCtx;
use bbb_protocol::block_types::VANILLA_BLOCK_COMPARATOR_ID;
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::with_property;
use crate::redstone;
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::directions::{clock_wise, counter_clock_wise, opposite, relative};
use bbb_server_types::fluid::air_block_state_id;

use super::diode;
use super::support::str_property;
use super::{
    block_type_of, bool_property, cycle_property, direction_property, DIRECTION_VALUES,
    SET_BLOCK_UPDATE_CLIENTS,
};

/// `ComparatorBlock.MODE` (`ComparatorBlock.java:33`).
const MODE: &str = "mode";
const POWERED: &str = "powered";
const FACING: &str = "facing";
const CLICK_SOUND: &str = "minecraft:block.comparator.click";
/// `ComparatorBlock.getDelay` (`ComparatorBlock.java:46-48`).
const DELAY: i32 = 2;

/// Whether a stored state is `minecraft:comparator`.
#[must_use]
pub fn is_comparator(block_state_id: i32) -> bool {
    block_type_of(block_state_id) == Some(VANILLA_BLOCK_COMPARATOR_ID)
}

fn facing(block_state_id: i32) -> Option<Direction> {
    direction_property(block_state_id, FACING)
}

fn powered(block_state_id: i32) -> Option<bool> {
    bool_property(block_state_id, POWERED)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComparatorMode {
    Compare,
    Subtract,
}

fn comparator_mode(block_state_id: i32) -> ComparatorMode {
    if str_property(block_state_id, MODE) == Some("subtract") {
        ComparatorMode::Subtract
    } else {
        ComparatorMode::Compare
    }
}

fn comparator_facing(block_state_id: i32) -> Direction {
    facing(block_state_id).unwrap_or(Direction::North)
}

#[must_use]
pub fn signal(block_state_id: i32, direction: Direction, output: i32) -> i32 {
    if powered(block_state_id) == Some(true) && facing(block_state_id) == Some(direction) {
        output
    } else {
        0
    }
}

fn side_signal(level: &mut LevelCtx<'_, '_>, pos: BlockPos, facing_dir: Direction) -> i32 {
    let clockwise = clock_wise(facing_dir);
    let counter = counter_clock_wise(facing_dir);
    let from_cw = level.get_control_input_signal(relative(pos, clockwise), clockwise, false);
    let from_ccw = level.get_control_input_signal(relative(pos, counter), counter, false);
    from_cw.max(from_ccw)
}

fn input_signal(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) -> i32 {
    let facing_dir = comparator_facing(block_state_id);
    let mut result = diode::input_signal(level, pos, facing_dir);
    let target_pos = relative(pos, facing_dir);
    let target_state = level.block_state(target_pos);
    if redstone::has_analog_output_signal(target_state) {
        return level.analog_output_signal(target_pos, opposite(facing_dir));
    }
    if result < 15 && level.is_redstone_conductor_at(target_pos) {
        let behind = relative(target_pos, facing_dir);
        let behind_state = level.block_state(behind);
        let frame_output = level.comparator_item_frame_output(behind, facing_dir);
        let block_output = redstone::has_analog_output_signal(behind_state)
            .then(|| level.analog_output_signal(behind, opposite(facing_dir)));
        if let Some(output) = match (frame_output, block_output) {
            (Some(frame), Some(block)) => Some(frame.max(block)),
            (output, None) | (None, output) => output,
        } {
            result = output;
        }
    }
    result
}

pub(crate) fn calculate_output(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) -> i32 {
    let input = input_signal(level, pos, block_state_id);
    if input == 0 {
        return 0;
    }
    let alternate = side_signal(level, pos, comparator_facing(block_state_id));
    if alternate > input {
        0
    } else if comparator_mode(block_state_id) == ComparatorMode::Subtract {
        input - alternate
    } else {
        input
    }
}

fn should_turn_on(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) -> bool {
    let input = input_signal(level, pos, block_state_id);
    if input == 0 {
        return false;
    }
    let side = side_signal(level, pos, comparator_facing(block_state_id));
    input > side || (input == side && comparator_mode(block_state_id) == ComparatorMode::Compare)
}

fn refresh_output_state(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let output = calculate_output(level, pos, block_state_id);
    let old = level
        .comparator_block_entity_output_signal(pos)
        .unwrap_or(0);
    level.set_comparator_block_entity_output_signal(pos, output);
    if old != output || comparator_mode(block_state_id) == ComparatorMode::Compare {
        let source_on = should_turn_on(level, pos, block_state_id);
        let is_on = powered(block_state_id) == Some(true);
        if is_on != source_on {
            if let Some(next) = with_property(
                block_state_id,
                POWERED,
                if source_on { "true" } else { "false" },
            ) {
                level.set_block(
                    pos,
                    next,
                    SET_BLOCK_UPDATE_CLIENTS,
                    crate::neighbor_updater::UPDATE_LIMIT,
                );
            }
        }
        if let Some(block_type_id) = block_type_of(block_state_id) {
            diode::update_neighbors_in_front(
                level,
                pos,
                comparator_facing(block_state_id),
                block_type_id,
            );
        }
    }
}

pub(super) fn check_tick_on_neighbor(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    let Some(block) = BlockTickType::of_block_state(block_state_id) else {
        return;
    };
    if level.will_tick_this_tick(pos, block) {
        return;
    }
    let output = calculate_output(level, pos, block_state_id);
    let old = level
        .comparator_block_entity_output_signal(pos)
        .unwrap_or(0);
    let is_powered = powered(block_state_id) == Some(true);
    if output != old || is_powered != should_turn_on(level, pos, block_state_id) {
        let priority = if diode::should_prioritize(level, pos, comparator_facing(block_state_id)) {
            TickPriority::High
        } else {
            TickPriority::Normal
        };
        level.schedule_block_tick(pos, block, DELAY, priority);
    }
}

pub(super) fn neighbor_changed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if diode::can_survive_live(level, pos) == Some(false) {
        let Some(block_type_id) = block_type_of(block_state_id) else {
            return;
        };
        level.drop_resources(pos, block_state_id);
        level.remove_block(pos);
        for direction in DIRECTION_VALUES {
            level.update_neighbors_at(relative(pos, direction), block_type_id);
        }
        return;
    }
    check_tick_on_neighbor(level, pos, block_state_id);
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    refresh_output_state(level, pos, block_state_id);
}

pub(super) fn use_without_item(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    player: super::BlockInteractionPlayer,
) -> bool {
    if !player.may_build {
        return false;
    }
    let Some(cycled) = cycle_property(block_state_id, MODE) else {
        return false;
    };
    let pitch = match comparator_mode(cycled) {
        ComparatorMode::Subtract => 0.55,
        ComparatorMode::Compare => 0.5,
    };
    // `ComparatorBlock.java:138`: the clicking player predicted its own sound.
    level.play_block_sound_except(pos, CLICK_SOUND, 0.3, pitch, player.source_entity_id());
    level.set_block(
        pos,
        cycled,
        SET_BLOCK_UPDATE_CLIENTS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    refresh_output_state(level, pos, cycled);
    true
}

pub(super) fn update_shape(
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    if direction_to_neighbour == Direction::Down
        && !diode::can_survive_on(neighbour_state, relative(pos, Direction::Down))
    {
        return air_block_state_id();
    }
    block_state_id
}

pub(super) fn set_placed_by(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if should_turn_on(level, pos, block_state_id) {
        if let Some(block) = BlockTickType::of_block_state(block_state_id) {
            level.schedule_block_tick(pos, block, 1, TickPriority::Normal);
        }
    }
}

pub(super) fn on_place(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(facing_dir) = facing(block_state_id) else {
        return;
    };
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    diode::update_neighbors_in_front(level, pos, facing_dir, block_type_id);
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
    let Some(facing_dir) = facing(block_state_id) else {
        return;
    };
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    diode::update_neighbors_in_front(level, pos, facing_dir, block_type_id);
}

pub(super) fn can_survive(pos: BlockPos, block_state_at: super::BlockStateAt<'_>) -> Option<bool> {
    diode::can_survive(pos, block_state_at)
}
