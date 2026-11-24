//! `BeehiveBlock` identity, honey analog, harvest and fire emergency release.

use super::shape_context::BlockShapeCtx;
use super::{
    block_type_of, default_block_state_id, direction_serialized_name,
    get_state_for_placement_override,
};
use crate::fluid::with_property;
use bbb_protocol::block_state_types::vanilla_block_state_property;
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaBlockUseItemOnOverride, VanillaGetStateForPlacementOverride,
};
use bbb_protocol::item_types::{VANILLA_ITEM_GLASS_BOTTLE_ID, VANILLA_ITEM_SHEARS_ID};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::directions::opposite;

const FACING: &str = "facing";
const HONEY_LEVEL: &str = "honey_level";
/// `BeehiveBlock.MAX_HONEY_LEVELS` (`BeehiveBlock.java:67`).
pub const MAX_HONEY_LEVELS: i32 = 5;
/// `BuiltInLootTables.HARVEST_BEEHIVE` (`BuiltInLootTables.java:107`).
pub const HARVEST_LOOT_TABLE: &str = "minecraft:harvest/beehive";
/// `SoundEvents.BEEHIVE_SHEAR` (`SoundEvents.java:189`).
pub const SHEAR_SOUND: &str = "minecraft:block.beehive.shear";
/// `SoundEvents.BOTTLE_FILL` (`SoundEvents.java:220`).
pub const BOTTLE_FILL_SOUND: &str = "minecraft:item.bottle.fill";

/// `BeehiveBlock.useItemOn` is unique to that class, covering both
/// `minecraft:beehive` and `minecraft:bee_nest`.
#[must_use]
pub fn is_beehive(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.use_item_on_override)
        == Some(VanillaBlockUseItemOnOverride::BeehiveBlock)
}

/// `BeehiveBlock.getStateForPlacement` (`BeehiveBlock.java`).
pub(super) fn state_for_placement(
    block_type_id: i32,
    horizontal_direction: Direction,
) -> Option<i32> {
    if !matches!(
        horizontal_direction,
        Direction::North | Direction::South | Direction::West | Direction::East
    ) || get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::BeehiveBlock)
    {
        return None;
    }
    with_property(
        default_block_state_id(block_type_id),
        FACING,
        direction_serialized_name(opposite(horizontal_direction)),
    )
}

/// `BeehiveBlock.updateShape` (`BeehiveBlock.java:337-351`).
pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    state: i32,
    pos: BlockPos,
    neighbour_state: i32,
) -> i32 {
    if block_type_of(neighbour_state) == Some(bbb_protocol::block_types::VANILLA_BLOCK_FIRE_ID) {
        level.beehive_emergency_release(pos, state);
    }
    state
}

/// `BeehiveBlock.getHoneyLevel` (`BeehiveBlockEntity.java`).
#[must_use]
pub fn honey_level(block_state_id: i32) -> i32 {
    vanilla_block_state_property(block_state_id, HONEY_LEVEL)
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

/// `BeehiveBlock.getAnalogOutputSignal` (`BeehiveBlock.java`).
#[must_use]
pub fn analog_output(block_state_id: i32) -> i32 {
    honey_level(block_state_id)
}

/// `BeehiveBlock.resetHoneyLevel` (`BeehiveBlock.java`).
#[must_use]
pub fn reset_honey_state(block_state_id: i32) -> Option<i32> {
    with_property(block_state_id, HONEY_LEVEL, "0")
}

#[must_use]
pub fn item_is_shears(item_id: Option<i32>) -> bool {
    item_id == Some(VANILLA_ITEM_SHEARS_ID)
}

#[must_use]
pub fn item_is_glass_bottle(item_id: Option<i32>) -> bool {
    item_id == Some(VANILLA_ITEM_GLASS_BOTTLE_ID)
}
