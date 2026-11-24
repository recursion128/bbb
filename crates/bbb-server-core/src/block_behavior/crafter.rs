//! Crafter redstone edges and placement scheduling.

use super::LevelCtx;
use crate::fluid::with_property;
use bbb_protocol::block_types::VANILLA_BLOCK_CRAFTER_ID;
use bbb_protocol::packets::BlockPos;
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};

use super::{block_type_of, bool_property, SET_BLOCK_UPDATE_CLIENTS};

pub fn is_crafter(state: i32) -> bool {
    block_type_of(state) == Some(VANILLA_BLOCK_CRAFTER_ID)
}

pub(crate) fn neighbor_changed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    let powered = level.has_neighbor_signal(pos);
    let triggered = bool_property(state, "triggered").unwrap_or(false);
    if powered == triggered {
        return;
    }
    let next = with_property(state, "triggered", if powered { "true" } else { "false" })
        .expect("crafter triggered property");
    let next = if powered {
        if let Some(kind) = BlockTickType::of_block_state(state) {
            level.schedule_block_tick(pos, kind, 4, TickPriority::Normal);
        }
        next
    } else {
        with_property(next, "crafting", "false").expect("crafter crafting property")
    };
    level.set_block(
        pos,
        next,
        SET_BLOCK_UPDATE_CLIENTS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    if let Some(crafter) = level.crafter_mut(pos) {
        crafter.set_triggered(powered);
    }
}

pub(crate) fn set_placed_by(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    if bool_property(state, "triggered").unwrap_or(false) {
        if let Some(kind) = BlockTickType::of_block_state(state) {
            level.schedule_block_tick(pos, kind, 4, TickPriority::Normal);
        }
    }
}
