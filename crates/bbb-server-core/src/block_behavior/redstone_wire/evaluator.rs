//! `DefaultRedstoneWireEvaluator`
//! (`net/minecraft/world/level/redstone/DefaultRedstoneWireEvaluator.java`).
//!
//! Computes the strongest non-wire signal reaching the cell or the strongest
//! incoming wire signal minus one, writes the new power with `UPDATE_CLIENTS`,
//! and emits the seven-position neighbor fan-out. When
//! `minecraft:redstone_experiments` is active, [`super::experimental`] runs
//! instead.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_types::VANILLA_BLOCK_REDSTONE_WIRE_ID;
use bbb_protocol::packets::BlockPos;

use crate::fluid::{above, below, HORIZONTAL};
use crate::redstone::{WireSignal, SIGNAL_MAX};
use bbb_server_types::directions::relative;

use super::super::{DIRECTION_VALUES, SET_BLOCK_UPDATE_CLIENTS};
use super::{is_redstone_wire, power_of, with_power};

pub(super) fn update_power_strength(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    // `:21`.
    let target_strength = calculate_target_strength(level, pos);
    // `:22`.
    if power_of(block_state_id) == target_strength {
        return;
    }
    // Removal callbacks pass the wire's old state; do not restore it.
    if level.block_state(pos) == block_state_id {
        // The flag is a bare `2`, `Block.UPDATE_CLIENTS`: the write tells the
        // clients and deliberately runs no neighbour fan-out of its own,
        // because the seven calls that follow are that fan-out and are a
        // different set of positions.
        level.set_block(
            pos,
            with_power(block_state_id, target_strength),
            SET_BLOCK_UPDATE_CLIENTS,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
    // `:27-36`.
    for update_pos in positions_to_update(pos) {
        level.update_neighbors_at(update_pos, VANILLA_BLOCK_REDSTONE_WIRE_ID);
    }
}

///
/// The early return at fifteen is what stops a wire beside a full-strength
/// source from paying for the incoming-wire walk, and it is also why a wire at
/// fifteen never reads its four horizontal neighbours.
fn calculate_target_strength(level: &LevelCtx<'_, '_>, pos: BlockPos) -> i32 {
    // `:41`.
    let block_signal = level.get_best_neighbor_signal_with(pos, WireSignal::Suppressed);
    // `:42`.
    if block_signal == SIGNAL_MAX {
        block_signal
    } else {
        block_signal.max(get_incoming_wire_signal(level, pos))
    }
}

fn wire_signal(block_state_id: i32) -> i32 {
    if is_redstone_wire(block_state_id) {
        power_of(block_state_id)
    } else {
        0
    }
}

///
/// The decay is the last line (`:46`) and it is a `Math.max(0, ..)`, so a cell
/// with no wire around it answers zero rather than minus one.
fn get_incoming_wire_signal(level: &LevelCtx<'_, '_>, pos: BlockPos) -> i32 {
    // `:30`.
    let mut signal = 0;
    // `:32-44`.
    for direction in HORIZONTAL {
        // `:33-35`.
        let neighbor_pos = relative(pos, direction);
        let neighbor_state = level.block_state(neighbor_pos);
        signal = signal.max(wire_signal(neighbor_state));
        // `:36`. Vanilla recomputes `pos.above()` inside the loop; it is the
        // same cell every time.
        let above_pos = above(pos);
        let neighbor_conducts = level.is_redstone_conductor_at(neighbor_pos);
        if neighbor_conducts && !level.is_redstone_conductor_at(above_pos) {
            // `:38-39`.
            let above_neighbor = above(neighbor_pos);
            signal = signal.max(wire_signal(level.block_state(above_neighbor)));
        } else if !neighbor_conducts {
            // `:41-42`.
            let below_neighbor = below(neighbor_pos);
            signal = signal.max(wire_signal(level.block_state(below_neighbor)));
        }
    }
    // `:46`.
    0.max(signal - 1)
}

pub(in crate::block_behavior) fn positions_to_update(pos: BlockPos) -> [BlockPos; 7] {
    let mut positions = [pos; 7];
    for (index, direction) in DIRECTION_VALUES.into_iter().enumerate() {
        positions[index + 1] = relative(pos, direction);
    }
    positions
}
