//! `ChiseledBookShelfBlock` (`ChiseledBookShelfBlock.java`) identity,
//! occupancy, and `SelectableSlotContainer.getHitSlot`
//! (`SelectableSlotContainer.java`).

use super::LevelCtx;
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaBlockUseWithoutItemOverride, VanillaGetStateForPlacementOverride,
};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::with_property;
use bbb_server_types::directions::opposite;

use super::{block_type_of, default_block_state_id, direction_serialized_name};

const FACING: &str = "facing";

/// Occupancy property names in slot order (`ChiseledBookShelfBlock.java:45-47`).
pub const SLOT_OCCUPIED_PROPERTIES: [&str; 6] = [
    "slot_0_occupied",
    "slot_1_occupied",
    "slot_2_occupied",
    "slot_3_occupied",
    "slot_4_occupied",
    "slot_5_occupied",
];

pub const INSERT_SOUND: &str = "minecraft:block.chiseled_bookshelf.insert";
pub const INSERT_ENCHANTED_SOUND: &str = "minecraft:block.chiseled_bookshelf.insert.enchanted";
pub const PICKUP_SOUND: &str = "minecraft:block.chiseled_bookshelf.pickup";
pub const PICKUP_ENCHANTED_SOUND: &str = "minecraft:block.chiseled_bookshelf.pickup.enchanted";

/// `ChiseledBookShelfBlock.useWithoutItem` is unique to that class.
#[must_use]
pub fn is_chiseled_bookshelf(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.use_without_item_override)
        == Some(VanillaBlockUseWithoutItemOverride::ChiseledBookShelfBlock)
}

/// `ChiseledBookShelfBlock.getStateForPlacement`
/// (`ChiseledBookShelfBlock.java`).
pub(super) fn state_for_placement(
    block_type_id: i32,
    horizontal_direction: Direction,
) -> Option<i32> {
    if vanilla_block_type(block_type_id)?.get_state_for_placement_override
        != Some(VanillaGetStateForPlacementOverride::ChiseledBookShelfBlock)
        || matches!(horizontal_direction, Direction::Down | Direction::Up)
    {
        return None;
    }
    with_property(
        default_block_state_id(block_type_id),
        FACING,
        direction_serialized_name(opposite(horizontal_direction)),
    )
}

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
pub fn slot_occupied(block_state_id: i32, slot: usize) -> bool {
    let Some(name) = SLOT_OCCUPIED_PROPERTIES.get(slot) else {
        return false;
    };
    bbb_protocol::block_state_types::vanilla_block_state_property(block_state_id, name)
        == Some("true")
}

/// `SelectableSlotContainer.getHitSlot` (`SelectableSlotContainer.java`).
#[must_use]
pub fn hit_slot(facing: Direction, clicked: Direction, cursor: [f32; 3]) -> Option<usize> {
    if facing != clicked {
        return None;
    }
    let relative_x = match clicked {
        Direction::North => 1.0 - cursor[0],
        Direction::South => cursor[0],
        Direction::West => cursor[2],
        Direction::East => 1.0 - cursor[2],
        Direction::Down | Direction::Up => return None,
    };
    let row = section(1.0 - cursor[1], 2);
    let column = section(relative_x, 3);
    Some(column + row * 3)
}

fn section(relative: f32, sections: usize) -> usize {
    let targeted_pixel = relative * 16.0;
    let section_size = 16.0 / sections as f32;
    (targeted_pixel / section_size)
        .floor()
        .clamp(0.0, (sections - 1) as f32) as usize
}

/// Rewrite occupancy bits from the six slots (`updateState`, `:40-44`).
#[must_use]
pub fn with_occupancy<S: crate::inventory::ContainerStack>(
    block_state_id: i32,
    items: &[S],
) -> Option<i32> {
    let mut next = block_state_id;
    for (slot, name) in SLOT_OCCUPIED_PROPERTIES.iter().enumerate() {
        let occupied = items.get(slot).is_some_and(|item| !item.stack_is_empty());
        next = with_property(next, name, if occupied { "true" } else { "false" })?;
    }
    Some(next)
}

/// `ChiseledBookShelfBlock.affectNeighborsAfterRemoval` (`:166-168`).
pub(super) fn affect_neighbors_after_removal(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    level.update_neighbour_for_output_signal(pos, block_type_id);
}
