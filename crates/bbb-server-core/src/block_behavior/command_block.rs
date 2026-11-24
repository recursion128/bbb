//! CommandBlock power, condition and scheduling.

use super::LevelCtx;
use bbb_protocol::block_types::{
    VANILLA_BLOCK_CHAIN_COMMAND_BLOCK_ID, VANILLA_BLOCK_COMMAND_BLOCK_ID,
    VANILLA_BLOCK_REPEATING_COMMAND_BLOCK_ID,
};
use bbb_protocol::packets::{BlockPos, CommandBlockMode, Direction};
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::directions::{opposite, relative};

use super::{block_type_of, bool_property, direction_property};

pub fn is_command_block(state: i32) -> bool {
    matches!(
        block_type_of(state),
        Some(
            VANILLA_BLOCK_COMMAND_BLOCK_ID
                | VANILLA_BLOCK_REPEATING_COMMAND_BLOCK_ID
                | VANILLA_BLOCK_CHAIN_COMMAND_BLOCK_ID
        )
    )
}

pub fn mode(state: i32) -> CommandBlockMode {
    match block_type_of(state) {
        Some(VANILLA_BLOCK_REPEATING_COMMAND_BLOCK_ID) => CommandBlockMode::Auto,
        Some(VANILLA_BLOCK_CHAIN_COMMAND_BLOCK_ID) => CommandBlockMode::Sequence,
        _ => CommandBlockMode::Redstone,
    }
}

pub fn facing(state: i32) -> Direction {
    direction_property(state, "facing").unwrap_or(Direction::North)
}

pub fn conditional(state: i32) -> bool {
    bool_property(state, "conditional").unwrap_or(false)
}

pub(crate) fn condition_is_met(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) -> bool {
    if conditional(state) {
        let behind = relative(pos, opposite(facing(state)));
        is_command_block(level.block_state(behind))
            && level
                .command_block(behind)
                .is_some_and(|block| block.success_count() > 0)
    } else {
        true
    }
}

pub(crate) fn mark_condition_met(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) -> bool {
    let met = condition_is_met(level, pos, state);
    if let Some(block) = level.command_block_mut(pos) {
        block.set_condition_met(met);
    }
    met
}

pub(crate) fn schedule(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    mark_condition_met(level, pos, state);
    if let Some(kind) = BlockTickType::of_block_state(state) {
        level.schedule_block_tick(pos, kind, 1, TickPriority::Normal);
    }
}

pub(crate) fn set_automatic(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    state: i32,
    automatic: bool,
) {
    let Some(block) = level.command_block_mut(pos) else {
        return;
    };
    let should_schedule = !block.automatic()
        && automatic
        && !block.powered()
        && mode(state) != CommandBlockMode::Sequence;
    block.set_automatic(automatic);
    if should_schedule {
        schedule(level, pos, state);
    }
}

pub(crate) fn mode_switched(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    if mode(state) == CommandBlockMode::Auto
        && level
            .command_block(pos)
            .is_some_and(|block| block.powered() || block.automatic())
    {
        schedule(level, pos, state);
    }
}

pub(crate) fn neighbor_changed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    let powered = level.has_neighbor_signal(pos);
    let Some(block) = level.command_block_mut(pos) else {
        return;
    };
    let rising = powered
        && !block.powered()
        && !block.automatic()
        && mode(state) != CommandBlockMode::Sequence;
    block.set_powered(powered);
    if rising {
        schedule(level, pos, state);
    }
}
