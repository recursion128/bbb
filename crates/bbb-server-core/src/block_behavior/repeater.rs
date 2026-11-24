//! `RepeaterBlock` (`net/minecraft/world/level/block/RepeaterBlock.java`), the
//! one `DiodeBlock` subclass this slice owns.
//!
//! The class is small because almost all of it is [`super::diode`]'s: what
//! `RepeaterBlock` adds is a settable delay, a `LOCKED` property, the rule that
//! only another diode can set it, and the right click that cycles the delay.
//!
//!
//! # Only a diode can lock a repeater
//!

use super::{BlockShapeCtx, LevelCtx};
use bbb_protocol::block_types::{vanilla_block_type, VanillaGetStateForPlacementOverride};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::redstone;
use bbb_server_types::directions::opposite;
use bbb_server_types::fluid::air_block_state_id;

use super::diode::{self, Diode};
use super::{
    block_type_of, cycle_property, default_block_state_id, direction_serialized_name, int_property,
};

const FACING: &str = "facing";

/// `RepeaterBlock.DELAY` (`RepeaterBlock.java:25`), which is
/// `BlockStateProperties.DELAY`, an `IntegerProperty` over 1..4
/// (`BlockStateProperties.java:101`); its serialized name is `delay`.
const DELAY: &str = "delay";

/// `RepeaterBlock.LOCKED` (`RepeaterBlock.java:24`), which is
/// `BlockStateProperties.LOCKED` (`BlockStateProperties.java:31`); its
/// serialized name is `locked`.
const LOCKED: &str = "locked";

/// `RepeaterBlock.sideInputDiodesOnly` (`RepeaterBlock.java:88-90`).
///
/// See this module's own documentation for what the boolean does; it is the
/// difference between "a repeater beside me locks me" and "anything powered
/// beside me locks me".
pub(super) const SIDE_INPUT_DIODES_ONLY: bool = true;

/// `RepeaterBlock.getDelay`'s multiplier (`RepeaterBlock.java:52-54`): the
/// `DELAY` property counts right clicks, and each one is worth two game ticks.
pub const TICKS_PER_DELAY_STEP: i32 = 2;

/// Whether a stored state is a `RepeaterBlock`.
///
///
/// Reading the `DELAY` or `LOCKED` property instead would be a guess; plenty of
/// blocks carry a `locked`, and `use_without_item_override` would work too but
/// is the tag a right click uses rather than a class test.
#[must_use]
pub fn is_repeater(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.get_state_for_placement_override)
        == Some(VanillaGetStateForPlacementOverride::RepeaterBlock)
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    horizontal_direction: Direction,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
    sources: redstone::SignalSources<'_>,
) -> Option<i32> {
    if vanilla_block_type(block_type_id)?.get_state_for_placement_override
        != Some(VanillaGetStateForPlacementOverride::RepeaterBlock)
        || matches!(horizontal_direction, Direction::Down | Direction::Up)
    {
        return None;
    }

    let facing = opposite(horizontal_direction);
    let state = with_property(
        default_block_state_id(block_type_id),
        FACING,
        direction_serialized_name(facing),
    )?;
    let mut context = sources.emitting(block_state_at);
    let locked = is_locked(pos, diode::read(state)?, |at, direction, only_diodes| {
        redstone::get_control_input_signal(at, direction, only_diodes, &mut context)
    });
    with_property(state, LOCKED, if locked { "true" } else { "false" })
}

///
/// One through four right clicks become two, four, six and eight ticks, which
/// is what a player calls "1 tick" through "4 ticks" — the in-game unit is a
/// redstone tick and is worth two of these.
pub(super) fn delay(block_state_id: i32) -> Option<i32> {
    Some(int_property(block_state_id, DELAY)? * TICKS_PER_DELAY_STEP)
}

pub(super) fn is_locked(
    pos: BlockPos,
    diode: Diode,
    control_input_signal: impl FnMut(BlockPos, Direction, bool) -> i32,
) -> bool {
    diode::alternate_signal(pos, diode, control_input_signal) > 0
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    if direction_to_neighbour == Direction::Down
        && !diode::can_survive_on(neighbour_state, crate::fluid::below(pos))
    {
        return air_block_state_id();
    }
    let Some(diode) = diode::read(block_state_id) else {
        return block_state_id;
    };

    if direction_to_neighbour == diode.facing || direction_to_neighbour == opposite(diode.facing) {
        return block_state_id;
    }

    let locked = is_locked(pos, diode, |at, direction, only_diodes| {
        level.get_control_input_signal(at, direction, only_diodes)
    });
    with_locked(block_state_id, locked)
}

///
/// The answer is whether the interaction was consumed: `InteractionResult.PASS`
/// for a player who may not build (`:43-45`), `InteractionResult.SUCCESS`
/// otherwise (`:48`).
///
pub(super) fn use_without_item(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    may_build: bool,
) -> bool {
    // `:43-45`.
    if !may_build {
        return false;
    }
    // `:47`.
    let Some(cycled) = cycle_property(block_state_id, DELAY) else {
        return false;
    };
    level.set_block(
        pos,
        cycled,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    // `:48`.
    true
}

/// `state.setValue(LOCKED, locked)`.
fn with_locked(block_state_id: i32, locked: bool) -> i32 {
    crate::fluid::with_property(
        block_state_id,
        LOCKED,
        if locked { "true" } else { "false" },
    )
    .expect("every repeater state carries LOCKED")
}
