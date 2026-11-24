//! `DiodeBlock` (`net/minecraft/world/level/block/DiodeBlock.java`), the
//! abstract base every repeater and comparator behaviour goes through.
//!
//!
//! # This is the first block whose output is a *delayed* function of its input
//!
//! The lamp reads a signal, the torch inverts one two ticks later, and a diode
//! is the first block that keeps a whole schedule of its own: `neighborChanged`
//! never writes (`:84-98`), it arms a tick `getDelay` ticks out with a priority
//! chosen from the neighbourhood (`:100-115`), and `tick` (`:54-67`) does the
//! writing. A save taken between those two therefore *carries* the pending
//! tick, which is what makes a repeater the first redstone block the world
//! oracle can measure at all.
//!
//! # The `tick` body re-arms itself, and that is not redundant
//!
//! `tick`'s `else if (!on)` branch writes `POWERED=true` and then, when the
//! input has *already* gone away again, schedules a second tick at
//! `TickPriority.VERY_HIGH` (`:60-65`). That second tick is what turns the
//! repeater back off one delay later, and it is the whole reason a repeater
//! passes a pulse shorter than its own delay instead of swallowing it. Dropping
//! it leaves a repeater latched on.
//!
//! # `ComparatorBlock` is not a client of this module's tick machine
//!

use super::LevelCtx;
use bbb_protocol::block_types::VANILLA_BLOCK_REDSTONE_WIRE_ID;
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{below, with_property};
use crate::redstone::{is_diode, SIGNAL_MAX};
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::directions::{clock_wise, counter_clock_wise, opposite, relative};

use super::{
    block_type_of, bool_property, direction_property, face_sturdy_at, int_property, repeater,
    BlockStateAt, SupportType, DIRECTION_VALUES, SET_BLOCK_UPDATE_CLIENTS,
};

/// `DiodeBlock.POWERED` (`DiodeBlock.java:28`), which is
/// `BlockStateProperties.POWERED` (`BlockStateProperties.java:36`); its
/// serialized name is `powered`.
const POWERED: &str = "powered";

/// `HorizontalDirectionalBlock.FACING` (`HorizontalDirectionalBlock.java:11`),
/// which is `BlockStateProperties.HORIZONTAL_FACING`
/// (`BlockStateProperties.java:57`); its serialized name is `facing`.
///
/// It points at the diode's **input**, not its output:
/// `getInputSignal` reads `pos.relative(FACING)` (`DiodeBlock.java:126-127`)
/// and `updateNeighborsInFront` notifies `pos.relative(FACING.getOpposite())`
/// (`:178-179`).
const FACING: &str = "facing";

/// `RedStoneWireBlock.POWER` as `DiodeBlock.getInputSignal` reads it
/// (`DiodeBlock.java:134`).
const POWER: &str = "power";

/// `DiodeBlock.getOutputSignal` (`DiodeBlock.java:189-191`), a flat fifteen
/// that `RepeaterBlock` does not replace.
///
/// Spelled as its own constant rather than as [`SIGNAL_MAX`] because vanilla
/// writes the bare literal here and reads `Redstone.SIGNAL_MAX` elsewhere; the
/// two happen to be the same number.
const OUTPUT_SIGNAL: i32 = 15;

/// The `DiodeBlock` subclass whose overrides this slice carries.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiodeOwner {
    Repeater,
}

/// [`DiodeOwner`] for a stored state, or `None` when the state is not a diode
/// this slice owns.
#[must_use]
pub fn owner(block_state_id: i32) -> Option<DiodeOwner> {
    repeater::is_repeater(block_state_id).then_some(DiodeOwner::Repeater)
}

/// The three things every level-touching body here reads off the state before
/// it touches the world.
///
/// Visible to [`super::repeater`] because that is the subclass: vanilla's
/// overrides read `state.getValue(FACING)` and call back into the base's
/// `getAlternateSignal` off the very same state.
#[derive(Debug, Clone, Copy)]
pub(super) struct Diode {
    pub(super) owner: DiodeOwner,
    /// `state.getValue(FACING)`, the side the input is on.
    pub(super) facing: Direction,
    /// `state.getValue(POWERED)`, which vanilla's `tick` and
    /// `checkTickOnNeighbor` both call `on`.
    powered: bool,
}

/// `None` for a state that is not an owned diode.
pub(super) fn read(block_state_id: i32) -> Option<Diode> {
    Some(Diode {
        owner: owner(block_state_id)?,
        facing: direction_property(block_state_id, FACING)?,
        powered: bool_property(block_state_id, POWERED)?,
    })
}

// ---------------------------------------------------------------------------
// The subclass hooks
// ---------------------------------------------------------------------------

fn delay(diode: Diode, block_state_id: i32) -> Option<i32> {
    match diode.owner {
        DiodeOwner::Repeater => repeater::delay(block_state_id),
    }
}

fn side_input_diodes_only(diode: Diode) -> bool {
    match diode.owner {
        DiodeOwner::Repeater => repeater::SIDE_INPUT_DIODES_ONLY,
    }
}

fn is_locked(level: &mut LevelCtx<'_, '_>, pos: BlockPos, diode: Diode) -> bool {
    match diode.owner {
        DiodeOwner::Repeater => repeater::is_locked(pos, diode, |at, direction, only_diodes| {
            level.get_control_input_signal(at, direction, only_diodes)
        }),
    }
}

// ---------------------------------------------------------------------------
// The two per-state redstone members
// ---------------------------------------------------------------------------

///
/// `None` is "no answer", for a state whose `getOutputSignal` has no owner
/// here; it is never a zero. The unpowered arm (`:76-77`) answers before the
/// output is asked for at all, so even a comparator gets a real zero out of
/// this while it is off.
#[must_use]
pub(crate) fn signal(block_state_id: i32, direction: Direction) -> Option<i32> {
    // `:76-77`.
    if !bool_property(block_state_id, POWERED)? {
        return Some(0);
    }
    // `:79`. `FACING` is the input side, so the one face a diode feeds is the
    // one the *asker* is standing on when it asks with this direction.
    if direction_property(block_state_id, FACING)? != direction {
        return Some(0);
    }
    output_signal(block_state_id)
}

fn output_signal(block_state_id: i32) -> Option<i32> {
    match owner(block_state_id)? {
        // `RepeaterBlock` does not replace it.
        DiodeOwner::Repeater => Some(OUTPUT_SIGNAL),
    }
}

///
/// `:71` is the *virtual* `state.getSignal(...)`, and neither subclass replaces
/// `getSignal`, so both reach [`signal`]. A diode is therefore as strong as it
/// is weak: it powers the block in front of it hard enough for that block to
/// pass the signal on, which is what lets a repeater drive a wire through a
/// solid cube.
#[must_use]
pub(crate) fn direct_signal(block_state_id: i32, direction: Direction) -> Option<i32> {
    signal(block_state_id, direction)
}

// ---------------------------------------------------------------------------
// Reading the neighbourhood
// ---------------------------------------------------------------------------

///
/// The early return at fifteen (`:129-131`) is not an optimisation: without it
/// the `Math.max` below would still answer fifteen, but the block state at the
/// input cell would be read for nothing. What the second half adds is redstone
/// wire, whose `getSignal` towards a diode is *not* its `POWER`
/// (`RedStoneWireBlock` only feeds the sides it is connected to), so a diode
/// reads the wire's property directly instead.
pub(super) fn input_signal(level: &mut LevelCtx<'_, '_>, pos: BlockPos, facing: Direction) -> i32 {
    // `:126-128`.
    let target_pos = relative(pos, facing);
    let input = level.get_signal(target_pos, facing);
    // `:129-131`.
    if input >= SIGNAL_MAX {
        return input;
    }
    // `:133-134`. Every `redstone_wire` state carries `POWER`, so the fallback
    // is unreachable; it is here because the property read is fallible in this
    // port and vanilla's `getValue` is not.
    let target_state = level.block_state(target_pos);
    let wire_power = if block_type_of(target_state) == Some(VANILLA_BLOCK_REDSTONE_WIRE_ID) {
        int_property(target_state, POWER).unwrap_or(0)
    } else {
        0
    };
    input.max(wire_power)
}

fn should_turn_on(level: &mut LevelCtx<'_, '_>, pos: BlockPos, facing: Direction) -> bool {
    input_signal(level, pos, facing) > 0
}

pub(super) fn alternate_signal(
    pos: BlockPos,
    diode: Diode,
    mut control_input_signal: impl FnMut(BlockPos, Direction, bool) -> i32,
) -> i32 {
    let facing = diode.facing;
    let clockwise = clock_wise(facing);
    let counter_clockwise = counter_clock_wise(facing);
    let side_input_diodes_only = side_input_diodes_only(diode);
    let from_clockwise =
        control_input_signal(relative(pos, clockwise), clockwise, side_input_diodes_only);
    let from_counter_clockwise = control_input_signal(
        relative(pos, counter_clockwise),
        counter_clockwise,
        side_input_diodes_only,
    );
    from_clockwise.max(from_counter_clockwise)
}

///
/// True when the diode's *output* cell holds another diode that is not pointing
/// straight back at it — the T-junction that vanilla gives
/// `TickPriority.EXTREMELY_HIGH` so that the two diodes settle in a defined
/// order. `oppositeState.getValue(FACING)` at `:200` is only reached because
/// `isDiode` short-circuited, and every diode state carries `FACING`.
pub(super) fn should_prioritize(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    facing: Direction,
) -> bool {
    // `:198`.
    let direction = opposite(facing);
    // `:199`.
    let opposite_state = level.block_state(relative(pos, direction));
    // `:200`. `DiodeBlock.isDiode` (`:193-195`) is
    // [`crate::redstone::is_diode`], which is wider than [`owner`]: a
    // comparator standing in front of a repeater does prioritise its tick.
    is_diode(opposite_state) && direction_property(opposite_state, FACING) != Some(direction)
}

// ---------------------------------------------------------------------------
// Survival
// ---------------------------------------------------------------------------

///
/// `SupportType.RIGID` is the strict one: a diode stands on a full block or a
/// slab's flat top and not on a fence post, whose top face is sturdy enough for
/// [`SupportType::Center`] but not for this.
///
/// `None` is "no answer", never "cannot survive"; see [`super::can_survive`].
pub(super) fn can_survive(pos: BlockPos, block_state_at: BlockStateAt<'_>) -> Option<bool> {
    let below_pos = below(pos);
    Some(can_survive_on(block_state_at(below_pos), below_pos))
}

/// `DiodeBlock.canSurviveOn` (`DiodeBlock.java:49-51`), on the live shape so a
/// moving piston below is no support.
pub(super) fn can_survive_on(neighbour_state: i32, neighbour_pos: BlockPos) -> bool {
    face_sturdy_at(
        neighbour_state,
        neighbour_pos,
        Direction::Up,
        SupportType::Rigid,
    )
}

pub(super) fn can_survive_live<L: crate::block_getter::BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
) -> Option<bool> {
    let support_pos = below(pos);
    Some(can_survive_on(level.block_state(support_pos), support_pos))
}

// ---------------------------------------------------------------------------
// The block-update legs
// ---------------------------------------------------------------------------

pub(super) fn update_neighbors_in_front(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    facing: Direction,
    block_type_id: i32,
) {
    // `:178-179`.
    let opposite_pos = relative(pos, opposite(facing));
    // `:181`.
    level.neighbor_changed(opposite_pos, block_type_id);
    // `:182`.
    level.update_neighbors_at_except_from_facing(opposite_pos, block_type_id, facing);
}

pub(super) fn on_place(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(diode) = read(block_state_id) else {
        return;
    };
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    update_neighbors_in_front(level, pos, diode.facing, block_type_id);
}

///
/// The mirror of [`on_place`]: the cell a diode was feeding has to be told the
/// feed is gone. A diode a piston pushed is exempt (`:172`) because the piston
/// runs the notification itself once the whole movement has landed.
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
    let Some(diode) = read(block_state_id) else {
        return;
    };
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    update_neighbors_in_front(level, pos, diode.facing, block_type_id);
}

///
/// Either the diode still has something to stand on and the scheduling half
/// runs, or it drops itself and tells all six of its neighbours' own
/// neighbourhoods. The removal leg is two calls rather than one
/// `Level.destroyBlock`: `dropResources` (`:91`) and then `removeBlock`
/// (`:92`), which raises no particle event.
///
pub(super) fn neighbor_changed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if read(block_state_id).is_none() {
        return;
    }
    // `:87`.
    if can_survive_live(level, pos) != Some(false) {
        check_tick_on_neighbor(level, pos, block_state_id);
        return;
    }
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    level.drop_resources(pos, block_state_id);
    level.remove_block(pos);
    // `:94-96`.
    for direction in DIRECTION_VALUES {
        level.update_neighbors_at(relative(pos, direction), block_type_id);
    }
}

///
/// Three priorities, and which one is used is what decides the order two diodes
/// facing each other settle in:
///
/// * [`TickPriority::ExtremelyHigh`] when the cell this diode feeds holds
///   another diode that is not pointing back at it ([`should_prioritize`]).
/// * [`TickPriority::VeryHigh`] when this diode is currently on, i.e. it is
///   about to switch off.
/// * [`TickPriority::High`] otherwise.
///
///
fn check_tick_on_neighbor(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(diode) = read(block_state_id) else {
        return;
    };
    // `:101`.
    if is_locked(level, pos, diode) {
        return;
    }
    // `:102-103`.
    let on = diode.powered;
    let should_turn_on = should_turn_on(level, pos, diode.facing);
    // `:104`, first conjunct.
    if on == should_turn_on {
        return;
    }
    let (Some(block), Some(delay)) = (
        BlockTickType::of_block_state(block_state_id),
        delay(diode, block_state_id),
    ) else {
        return;
    };
    // `:104`, second conjunct.
    if level.will_tick_this_tick(pos, block) {
        return;
    }
    // `:105-110`.
    let priority = if should_prioritize(level, pos, diode.facing) {
        TickPriority::ExtremelyHigh
    } else if on {
        TickPriority::VeryHigh
    } else {
        TickPriority::High
    };
    // `:112`.
    level.schedule_block_tick(pos, block, delay, priority);
}

pub(super) fn set_placed_by(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(diode) = read(block_state_id) else {
        return;
    };
    if !should_turn_on(level, pos, diode.facing) {
        return;
    }
    if let Some(block) = BlockTickType::of_block_state(block_state_id) {
        level.schedule_block_tick(pos, block, 1, TickPriority::Normal);
    }
}

///
///
/// The re-arm at `:62-64` is the pulse-shortening rule: a diode that turns on
/// while its input has *already* gone away books its own switch-off one delay
/// later, at [`TickPriority::VeryHigh`], instead of waiting to be told again.
/// Dropping it latches the diode on. See this module's own docs.
pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(diode) = read(block_state_id) else {
        return;
    };
    // `:55`.
    if is_locked(level, pos, diode) {
        return;
    }
    // `:56-57`.
    let on = diode.powered;
    let should_turn_on = should_turn_on(level, pos, diode.facing);
    if on && !should_turn_on {
        // `:58-59`.
        level.set_block(
            pos,
            with_powered(block_state_id, false),
            SET_BLOCK_UPDATE_CLIENTS,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    } else if !on {
        // `:60-61`.
        level.set_block(
            pos,
            with_powered(block_state_id, true),
            SET_BLOCK_UPDATE_CLIENTS,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        // `:62-64`. The tick is filed against the state that scheduled it,
        // which is the state the write just replaced -- vanilla passes `this`,
        // the block, and both states share it.
        if !should_turn_on {
            let (Some(block), Some(delay)) = (
                BlockTickType::of_block_state(block_state_id),
                delay(diode, block_state_id),
            ) else {
                return;
            };
            level.schedule_block_tick(pos, block, delay, TickPriority::VeryHigh);
        }
    }
}

/// `state.setValue(POWERED, powered)`.
fn with_powered(block_state_id: i32, powered: bool) -> i32 {
    with_property(
        block_state_id,
        POWERED,
        if powered { "true" } else { "false" },
    )
    .expect("every diode state carries POWERED")
}
