//! `RedstoneTorchBlock` (`net/minecraft/world/level/block/RedstoneTorchBlock.java`)
//! and its one subclass `RedstoneWallTorchBlock`.
//!
//!
//! This is the slice's first redstone **source** with a state of its own: the
//! lamp reads a signal, the redstone block emits a constant one, and a torch
//! is the first block whose own output is a function of its input.
//!
//! # The torch is inverted, and the inversion runs on a scheduled tick
//!
//! `neighborChanged` (`:95-101`) never writes. It only *arms* a tick two ticks
//! out, and only when `LIT` currently **agrees** with the neighbour signal --
//! which is the disagreeing case for an inverter, because a lit torch is one
//! whose input is off. The write happens in `tick` (`:73-92`), which reads the
//! signal again. That second read is what makes a torch ignore an input pulse
//! shorter than its two-tick delay.
//!
//! # Burnout
//!
//!
//! Only the *unlighting* half records a toggle (`:84`, `isToggledTooFrequently`
//! with `add = true`); the relighting half only reads (`:89`). So the counter
//! measures how often the torch was forced off, and the eighth such forcing
//! inside the window is what burns it out.

use super::{BlockShapeCtx, LevelCtx};
use crate::block_getter::BlockGetter;

use bbb_protocol::block_types::{
    vanilla_block_type, VanillaBlockRedstoneSignalOverride, VanillaNeighborChangedOverride,
};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{below, with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::LEVEL_EVENT_REDSTONE_TORCH_BURNOUT;
use crate::redstone::SIGNAL_MAX;
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::directions::{opposite, relative};

use super::{
    air_block_state_id, block_type_of, bool_property, can_support_center, can_support_center_at,
    direction_property, face_sturdy, face_sturdy_at, BlockStateAt, SupportType, DIRECTION_VALUES,
};

/// `RedstoneTorchBlock.LIT` (`RedstoneTorchBlock.java:26`), which is
/// `BlockStateProperties.LIT`; `RedstoneWallTorchBlock.LIT`
/// (`RedstoneWallTorchBlock.java:27`) is the very same object.
const LIT: &str = "lit";

/// `RedstoneWallTorchBlock.FACING` (`RedstoneWallTorchBlock.java:26`), which is
/// `HorizontalDirectionalBlock.FACING`. The plain torch has no such property.
const FACING: &str = "facing";

/// `RedstoneTorchBlock.RECENT_TOGGLE_TIMER` (`RedstoneTorchBlock.java:28`): how
/// far back the burnout window reaches, in game ticks.
///
/// The prune at `:77` spells the number as a bare `60L` rather than reading the
/// constant, and `isToggledTooFrequently` spells [`MAX_RECENT_TOGGLES`] as a
/// bare `8` at `:138`; both are the same values under their declared names.
pub const RECENT_TOGGLE_TIMER: i64 = 60;

/// `RedstoneTorchBlock.MAX_RECENT_TOGGLES` (`RedstoneTorchBlock.java:29`): the
/// count at which a torch burns out.
pub const MAX_RECENT_TOGGLES: usize = 8;

/// `RedstoneTorchBlock.RESTART_DELAY` (`RedstoneTorchBlock.java:30`): how long
/// a burnt-out torch stays dark before its next tick (`:86`).
pub const RESTART_DELAY: i32 = 160;

/// `RedstoneTorchBlock.TOGGLE_DELAY` (`RedstoneTorchBlock.java:31`): the delay
/// `neighborChanged` arms its own tick with (`:99`).
pub const TOGGLE_DELAY: i32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RedstoneTorchToggle {
    /// `Toggle.pos` (`:152`), stored from `pos.immutable()` (`:131`).
    pub pos: BlockPos,
    /// `Toggle.when` (`:153`), stored from `level.getGameTime()` (`:131`).
    pub when: i64,
}

/// Whether a stored state is one of the two redstone torches.
///
#[must_use]
pub fn is_redstone_torch(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.neighbor_changed_override)
        == Some(VanillaNeighborChangedOverride::RedstoneTorchBlock)
}

/// Whether a stored state is the wall variant, i.e. `instanceof
/// RedstoneWallTorchBlock`.
///
/// Derived from the generated `signal_override` class tag: `getSignal` is the
/// one member the subclass replaces and no other 26.1 block reaches
/// `RedstoneWallTorchBlock`'s body, so the tag is the class test. Reading the
/// `FACING` property instead would be a guess -- plenty of blocks carry one.
#[must_use]
pub fn is_wall_torch(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.signal_override)
        == Some(VanillaBlockRedstoneSignalOverride::RedstoneWallTorchBlock)
}

// ---------------------------------------------------------------------------
// The three redstone members
// ---------------------------------------------------------------------------

///
/// The two bodies differ only in which single side is dark: the standing torch
/// feeds every side but `UP`, and the wall torch feeds every side but the one
/// it is attached to -- `FACING` is the direction the flame points away from
/// the wall, so `state.getValue(FACING) != direction` excludes the wall itself.
///
/// One function for both because [`is_wall_torch`] is the very class test the
/// virtual dispatch performs; the caller in [`crate::redstone`] routes both
/// generated owners here rather than restating the split.
#[must_use]
pub(crate) fn signal(block_state_id: i32, direction: Direction) -> i32 {
    if bool_property(block_state_id, LIT) != Some(true) {
        return 0;
    }
    let dark_side = if is_wall_torch(block_state_id) {
        direction_property(block_state_id, FACING).unwrap_or(Direction::Up)
    } else {
        Direction::Up
    };
    if direction == dark_side {
        0
    } else {
        SIGNAL_MAX
    }
}

///
/// The subclass does not replace it, so a wall torch takes this body too: only
/// the cell **below** a torch is strongly powered, which is why a torch under a
/// block powers the block and a torch beside one does not.
///
/// `state.getSignal(level, pos, direction)` at `:105` is the *virtual* call, so
/// for a wall torch it reaches the wall body -- and a wall torch's `FACING` is
/// horizontal, so `DOWN` is never its dark side and the answer is always 15
/// while it is lit.
#[must_use]
pub(crate) fn direct_signal(block_state_id: i32, direction: Direction) -> i32 {
    if direction == Direction::Down {
        signal(block_state_id, direction)
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// The level-touching bodies
// ---------------------------------------------------------------------------

fn has_neighbor_signal(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) -> bool {
    if is_wall_torch(block_state_id) {
        let attached =
            opposite(direction_property(block_state_id, FACING).unwrap_or(Direction::Up));
        level.has_signal(relative(pos, attached), attached)
    } else {
        level.has_signal(below(pos), Direction::Down)
    }
}

///
/// Six `updateNeighborsAt` calls, one per `Direction.values()` face
/// ([`DIRECTION_VALUES`]) and each at the **neighbour's** position rather than
/// the torch's: a torch tells each of its six neighbours' own neighbourhoods
/// that something changed, which is what carries the torch's signal one cell
/// further than a plain `Level.setBlock` fan-out would.
///
fn notify_neighbors(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_type_id: i32) {
    for direction in DIRECTION_VALUES {
        level.update_neighbors_at(relative(pos, direction), block_type_id);
    }
}

///
/// A torch defaults to `LIT` (`:40`), so the moment it lands it is already
/// emitting and everything around it has to be told.
pub(super) fn on_place(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    notify_neighbors(level, pos, block_type_id);
}

///
/// The mirror of `onPlace`: a torch that is gone has to drop the signal it was
/// feeding, and the same six-way notification does it. A torch a piston pushed
/// is exempt (`:58`) because the piston runs the notification itself once the
/// whole movement has landed.
///
pub(super) fn affect_neighbors_after_removal(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    moved_by_piston: bool,
) {
    if moved_by_piston {
        return;
    }
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    notify_neighbors(level, pos, block_type_id);
}

///
/// Reads as a double negative and is not one: `LIT == hasNeighborSignal` is
/// "the torch is wrong", because a torch is lit exactly when its input is off.
///
pub(super) fn neighbor_changed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(is_lit) = bool_property(block_state_id, LIT) else {
        return;
    };
    // `:98`, first conjunct.
    let has_signal = has_neighbor_signal(level, pos, block_state_id);
    if is_lit != has_signal {
        return;
    }
    let Some(block) = BlockTickType::of_block_state(block_state_id) else {
        return;
    };
    // `:98`, second conjunct.
    if level.will_tick_this_tick(pos, block) {
        return;
    }
    level.schedule_block_tick(pos, block, TOGGLE_DELAY, TickPriority::Normal);
}

///
/// Four things in order: read the input, prune the burnout window, and then
/// either go dark (recording a toggle, and burning out on the eighth) or come
/// back on (only if the window is not already full).
pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    // `:74`.
    let neighbor_signal = has_neighbor_signal(level, pos, block_state_id);
    let Some(is_lit) = bool_property(block_state_id, LIT) else {
        return;
    };
    // `:75-79`: entries older than the window are dropped from the front, and
    // the list is in insertion order, so the walk stops at the first live one.
    let game_time = level.game_time();
    let toggles = level.recent_torch_toggles();
    let expired = toggles
        .iter()
        .take_while(|toggle| game_time - toggle.when > RECENT_TOGGLE_TIMER)
        .count();
    if expired > 0 {
        toggles.drain(..expired);
    }

    if is_lit {
        // `:81-88`.
        if !neighbor_signal {
            return;
        }
        level.set_block(
            pos,
            with_lit(block_state_id, false),
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        // `:84`, the one call that records a toggle.
        if !is_toggled_too_frequently(level, pos, true) {
            return;
        }
        // `:85`.
        level.level_event(LEVEL_EVENT_REDSTONE_TORCH_BURNOUT, pos, 0);
        // `:86`: the block is re-read *after* the write, so the tick is filed
        // against whatever now stands there rather than against the lit state
        // that scheduled it.
        let current = level.block_state(pos);
        if let Some(block) = BlockTickType::of_block_state(current) {
            level.schedule_block_tick(pos, block, RESTART_DELAY, TickPriority::Normal);
        }
    } else if !neighbor_signal && !is_toggled_too_frequently(level, pos, false) {
        // `:89-91`.
        level.set_block(
            pos,
            with_lit(block_state_id, true),
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}

///
/// `add` decides whether this call also *records* a toggle before counting, and
/// only the unlighting branch passes true (`:84`). The count is over entries at
/// this exact position; other torches share the list and are skipped.
fn is_toggled_too_frequently(level: &mut LevelCtx<'_, '_>, pos: BlockPos, add: bool) -> bool {
    let when = level.game_time();
    let toggles = level.recent_torch_toggles();
    if add {
        // `:130-132`.
        toggles.push(RedstoneTorchToggle { pos, when });
    }
    // `:134-142`. Vanilla stops at the eighth match rather than counting the
    // whole list; the answer is the same either way.
    toggles.iter().filter(|toggle| toggle.pos == pos).count() >= MAX_RECENT_TOGGLES
}

// ---------------------------------------------------------------------------
// Support
// ---------------------------------------------------------------------------

///
/// `None` is "no answer", never "cannot survive": it is what
/// [`super::face_sturdy`] returns for a block whose shape is dynamic, and the
/// caller must not read that as a refusal.
pub(super) fn can_survive(
    block_state_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt<'_>,
) -> Option<bool> {
    if is_wall_torch(block_state_id) {
        let facing = direction_property(block_state_id, FACING)?;
        let wall_pos = relative(pos, opposite(facing));
        face_sturdy(block_state_at(wall_pos), facing, SupportType::Full)
    } else {
        can_support_center(block_state_at(below(pos)), Direction::Up)
    }
}

pub(super) fn can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
) -> Option<bool> {
    if is_wall_torch(block_state_id) {
        let facing = direction_property(block_state_id, FACING)?;
        let support_pos = relative(pos, opposite(facing));
        Some(face_sturdy_at(
            level.block_state(support_pos),
            support_pos,
            facing,
            SupportType::Full,
        ))
    } else {
        let support_pos = below(pos);
        Some(can_support_center_at(
            level.block_state(support_pos),
            support_pos,
            Direction::Up,
        ))
    }
}

///
///
/// A [`can_survive`] that cannot answer leaves the torch standing, which is the
/// same fail-closed choice [`super::can_survive`] documents.
pub(super) fn update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    let support_side = if is_wall_torch(block_state_id) {
        let Some(facing) = direction_property(block_state_id, FACING) else {
            return block_state_id;
        };
        opposite(facing)
    } else {
        Direction::Down
    };
    if direction_to_neighbour != support_side {
        return block_state_id;
    }
    if can_survive_live(level, block_state_id, pos) == Some(false) {
        air_block_state_id()
    } else {
        block_state_id
    }
}

/// `state.setValue(LIT, lit)`.
fn with_lit(block_state_id: i32, lit: bool) -> i32 {
    with_property(block_state_id, LIT, if lit { "true" } else { "false" })
        .expect("every redstone torch state carries LIT")
}
