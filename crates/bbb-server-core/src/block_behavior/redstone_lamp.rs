//! `RedstoneLampBlock` (`net/minecraft/world/level/block/RedstoneLampBlock.java`).
//!
//!
//! # The asymmetry is the whole mechanism
//!
//! Turning **on** is immediate: `neighborChanged` writes the new state on the
//! spot (`:45`). Turning **off** is deferred: it arms a scheduled tick four
//! ticks out (`:43`) and the write happens in `tick` — but only after `tick`
//! asks `hasNeighborSignal` *again* (`:53`). That second read is what makes a
//! lamp survive a pulse gap: re-power it inside those four ticks and the
//! pending tick still runs, finds a signal, and leaves the lamp lit. Dropping
//! the re-read would turn every lamp off four ticks after any flicker.

use super::LevelCtx;
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaGetStateForPlacementOverride, VANILLA_BLOCK_REDSTONE_LAMP_ID,
};
use bbb_protocol::packets::BlockPos;

use crate::fluid::with_property;
use crate::redstone;
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};

use super::{
    block_type_of, bool_property, cycle_property, default_block_state_id, BlockStateAt,
    SET_BLOCK_UPDATE_CLIENTS,
};

/// `RedstoneLampBlock.LIT` (`RedstoneLampBlock.java:18`), which is
/// `RedstoneTorchBlock.LIT` (`RedstoneTorchBlock.java:26`), i.e.
/// `BlockStateProperties.LIT`; its serialized name is `lit`.
const LIT: &str = "lit";

/// The delay `RedstoneLampBlock.neighborChanged` arms its own tick with when
/// a lit lamp loses its signal (`RedstoneLampBlock.java:43`, `scheduleTick(pos,
/// this, 4)`).
///
/// The three-argument `ScheduledTickAccess.scheduleTick`
/// (`ScheduledTickAccess.java:21-23`) is the one without a priority, so the
/// tick lands at `TickPriority.NORMAL`.
pub const TICK_DELAY: i32 = 4;

/// Whether a stored state is a `RedstoneLampBlock`.
///
#[must_use]
pub fn is_redstone_lamp(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.get_state_for_placement_override)
        == Some(VanillaGetStateForPlacementOverride::RedstoneLampBlock)
}

///
/// `BlockPlaceContext.getClickedPos` is the cell the lamp goes into, so the
/// signal is read there and not at the face that was clicked.
pub fn state_for_placement(
    pos: BlockPos,
    block_state_at: BlockStateAt<'_>,
    sources: redstone::SignalSources<'_>,
) -> i32 {
    let lit = redstone::has_neighbor_signal(pos, &mut sources.emitting(block_state_at));
    with_lit(default_block_state_id(VANILLA_BLOCK_REDSTONE_LAMP_ID), lit)
}

///
/// The `!level.isClientSide()` gate (`:39`) is always open on a dedicated
/// server, so it is not restated as a branch.
pub(super) fn neighbor_changed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    // `:40`.
    let Some(is_lit) = bool_property(block_state_id, LIT) else {
        return;
    };
    // `:41`.
    let has_signal = level.has_neighbor_signal(pos);
    if is_lit == has_signal {
        return;
    }
    if is_lit {
        // `:43`: going dark waits four ticks, and `tick` re-reads the signal
        // when it gets there.
        if let Some(block) = BlockTickType::of_block_state(block_state_id) {
            level.schedule_block_tick(pos, block, TICK_DELAY, TickPriority::Normal);
        }
    } else {
        level.set_block(
            pos,
            toggle_lit(block_state_id),
            SET_BLOCK_UPDATE_CLIENTS,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}

///
/// The second `hasNeighborSignal` read (`:53`) is the reason a lamp re-powered
/// inside the four-tick window stays lit: the pending tick still runs, finds
/// the signal back, and writes nothing.
pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    // `:53`, first conjunct.
    if bool_property(block_state_id, LIT) != Some(true) {
        return;
    }
    // `:53`, second conjunct.
    if level.has_neighbor_signal(pos) {
        return;
    }
    // `:54`.
    level.set_block(
        pos,
        toggle_lit(block_state_id),
        SET_BLOCK_UPDATE_CLIENTS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}

/// `state.setValue(LIT, lit)`.
fn with_lit(block_state_id: i32, lit: bool) -> i32 {
    with_property(block_state_id, LIT, if lit { "true" } else { "false" })
        .expect("every redstone lamp state carries LIT")
}

fn toggle_lit(block_state_id: i32) -> i32 {
    cycle_property(block_state_id, LIT).expect("every redstone lamp state carries LIT")
}
