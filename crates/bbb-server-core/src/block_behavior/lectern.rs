//! `LecternBlock` (`LecternBlock.java`) identity, pulse, and analog.

use super::LevelCtx;
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaBlockUseWithoutItemOverride, VanillaGetStateForPlacementOverride,
};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{below, with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;
use crate::redstone::SIGNAL_MAX;
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};

use super::{
    block_type_of, bool_property, default_block_state_id, direction_serialized_name,
    get_state_for_placement_override,
};
use bbb_server_types::directions::opposite;

const HAS_BOOK: &str = "has_book";
const POWERED: &str = "powered";
/// `LecternBlock.PAGE_CHANGE_IMPULSE_TICKS` (`LecternBlock.java:58`).
const PAGE_CHANGE_IMPULSE_TICKS: i32 = 2;
/// `LecternBlock.signalPageChange` (`:158`) `levelEvent(1043, ...)`.
pub const PAGE_CHANGE_LEVEL_EVENT: i32 = 1043;
/// `SoundEvents.BOOK_PUT`.
pub const BOOK_PUT_SOUND: &str = "minecraft:item.book.put";

/// `LecternBlock.getStateForPlacement` (`LecternBlock.java`).
pub(crate) fn state_for_placement(
    block_type_id: i32,
    horizontal_direction: Direction,
    has_book: bool,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::LecternBlock)
        || !matches!(
            horizontal_direction,
            Direction::North | Direction::East | Direction::South | Direction::West
        )
    {
        return None;
    }
    let state = with_property(
        default_block_state_id(block_type_id),
        "facing",
        direction_serialized_name(opposite(horizontal_direction)),
    )?;
    with_property(state, HAS_BOOK, if has_book { "true" } else { "false" })
}

/// `LecternBlock.useWithoutItem` is unique to that class.
#[must_use]
pub fn is_lectern(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.use_without_item_override)
        == Some(VanillaBlockUseWithoutItemOverride::LecternBlock)
}

#[must_use]
pub fn has_book(block_state_id: i32) -> bool {
    bool_property(block_state_id, HAS_BOOK).unwrap_or(false)
}

/// `LecternBlock.FACING`, which is `BlockStateProperties.HORIZONTAL_FACING`
/// (`LecternBlock.java`).
#[must_use]
pub fn facing(block_state_id: i32) -> Direction {
    match bbb_protocol::block_state_types::vanilla_block_state_property(block_state_id, "facing") {
        Some("south") => Direction::South,
        Some("west") => Direction::West,
        Some("east") => Direction::East,
        _ => Direction::North,
    }
}

#[must_use]
pub fn powered(block_state_id: i32) -> bool {
    bool_property(block_state_id, POWERED).unwrap_or(false)
}

/// `LecternBlock.getSignal` (`LecternBlock.java`).
#[must_use]
pub fn signal(block_state_id: i32) -> i32 {
    if powered(block_state_id) {
        SIGNAL_MAX
    } else {
        0
    }
}

/// `LecternBlock.getDirectSignal` (`LecternBlock.java`).
#[must_use]
pub fn direct_signal(block_state_id: i32, direction: Direction) -> i32 {
    if direction == Direction::Up {
        signal(block_state_id)
    } else {
        0
    }
}

/// `LecternBlock.tick` (`LecternBlock.java`): clear `POWERED`.
pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    change_powered(level, pos, block_state_id, false);
}

/// `LecternBlock.affectNeighborsAfterRemoval` (`LecternBlock.java`).
pub(super) fn affect_neighbors_after_removal(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    if powered(block_state_id) {
        update_below(level, pos, block_state_id);
    }
}

/// `LecternBlock.resetBookState` (`LecternBlock.java`).
pub(crate) fn reset_book_state(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    source_entity_id: Option<i32>,
    has_book: bool,
) {
    let Some(unpowered) = with_property(block_state_id, POWERED, "false") else {
        return;
    };
    let Some(next) = with_property(unpowered, HAS_BOOK, if has_book { "true" } else { "false" })
    else {
        return;
    };
    if next != block_state_id {
        level.set_block(
            pos,
            next,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
    level.game_event_from_entity(
        VanillaGameEvent::BLOCK_CHANGE,
        pos,
        source_entity_id,
        Some(next),
    );
    update_below(level, pos, block_state_id);
}

/// `LecternBlock.signalPageChange` (`LecternBlock.java`).
pub(crate) fn signal_page_change(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    change_powered(level, pos, block_state_id, true);
    if let Some(block) = BlockTickType::of_block_state(block_state_id) {
        level.schedule_block_tick(pos, block, PAGE_CHANGE_IMPULSE_TICKS, TickPriority::Normal);
    }
    level.level_event(PAGE_CHANGE_LEVEL_EVENT, pos, 0);
}

fn change_powered(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    is_powered: bool,
) {
    let Some(next) = with_property(
        block_state_id,
        POWERED,
        if is_powered { "true" } else { "false" },
    ) else {
        return;
    };
    if next != block_state_id {
        level.set_block(
            pos,
            next,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
    update_below(level, pos, block_state_id);
}

/// `LecternBlock.updateBelow` (`LecternBlock.java`) without the
/// redstone-experiments `Orientation`.
fn update_below(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    level.update_neighbors_at(below(pos), block_type_id);
}
