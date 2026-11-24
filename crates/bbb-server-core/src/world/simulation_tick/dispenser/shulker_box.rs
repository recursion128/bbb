//! `ShulkerBoxDispenseBehavior`: `BlockItem.place` through a
//! `DirectionalPlaceContext` in front of the dispenser.

use bbb_protocol::block_sound_facts_26_1::vanilla_block_place_sound_26_1;
use bbb_protocol::item_types::vanilla_item_type;
use bbb_protocol::packets::Direction;
use bbb_server_types::block_queries::direction_serialized_name;
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result as CoreResult;

use super::{BlockSource, Dispensed};
use crate::block_behavior::default_block_state_id;
use crate::entity::falling_block::can_be_replaced_by_empty_handed_place;
use crate::fluid::with_property;
use crate::level_event::VanillaGameEvent;
use crate::neighbor_updater::UPDATE_LIMIT;
use crate::world::block_use::item_components::{
    apply_block_state, apply_components, apply_custom_data,
};
use crate::world::ServerWorld;

/// `Block.UPDATE_ALL_IMMEDIATE`, the flags `BlockItem.placeBlock` writes with.
const UPDATE_ALL_IMMEDIATE: i32 = 11;

pub(super) fn is_shulker_box(item_id: i32) -> bool {
    use bbb_protocol::item_types::*;
    matches!(
        item_id,
        VANILLA_ITEM_SHULKER_BOX_ID
            | VANILLA_ITEM_WHITE_SHULKER_BOX_ID
            | VANILLA_ITEM_ORANGE_SHULKER_BOX_ID
            | VANILLA_ITEM_MAGENTA_SHULKER_BOX_ID
            | VANILLA_ITEM_LIGHT_BLUE_SHULKER_BOX_ID
            | VANILLA_ITEM_YELLOW_SHULKER_BOX_ID
            | VANILLA_ITEM_LIME_SHULKER_BOX_ID
            | VANILLA_ITEM_PINK_SHULKER_BOX_ID
            | VANILLA_ITEM_GRAY_SHULKER_BOX_ID
            | VANILLA_ITEM_LIGHT_GRAY_SHULKER_BOX_ID
            | VANILLA_ITEM_CYAN_SHULKER_BOX_ID
            | VANILLA_ITEM_PURPLE_SHULKER_BOX_ID
            | VANILLA_ITEM_BLUE_SHULKER_BOX_ID
            | VANILLA_ITEM_BROWN_SHULKER_BOX_ID
            | VANILLA_ITEM_GREEN_SHULKER_BOX_ID
            | VANILLA_ITEM_RED_SHULKER_BOX_ID
            | VANILLA_ITEM_BLACK_SHULKER_BOX_ID
    )
}

pub(super) fn dispense(
    world: &mut ServerWorld,
    source: &BlockSource,
    mut stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    let placed = place(world, source, &stack);
    if placed {
        stack.shrink(1);
    }
    Ok(Dispensed::optional(stack, placed))
}

fn place(world: &mut ServerWorld, source: &BlockSource, stack: &RuntimeItemStack) -> bool {
    let dimension = source.level;
    let Some(block_type_id) = stack
        .item_id()
        .and_then(vanilla_item_type)
        .and_then(|item| item.block_item_behavior)
        .map(|block| block.block_type_id)
    else {
        return false;
    };
    let target = source.target();
    let below = bbb_server_types::directions::relative(target, Direction::Down);
    let (replaced, below_empty) = world.with_level(source.level, |level| {
        (
            level.block_state(target),
            block_state_is_air(level.block_state(below)),
        )
    });
    // `ShulkerBoxBlock.getStateForPlacement` faces the clicked face.
    let clicked_face = if below_empty {
        source.facing
    } else {
        Direction::Up
    };
    let Some(state) = with_property(
        default_block_state_id(block_type_id),
        "facing",
        direction_serialized_name(clicked_face),
    ) else {
        return false;
    };
    if !can_be_replaced_by_empty_handed_place(replaced)
        || world.placement_candidate_is_unobstructed(dimension, state, target) == Some(false)
    {
        return false;
    }
    if !world.with_level(source.level, |level| {
        level.set_block(target, state, UPDATE_ALL_IMMEDIATE, UPDATE_LIMIT)
    }) {
        return false;
    }
    apply_block_state(world, dimension, target, stack);
    apply_custom_data(world, dimension, target, None, stack);
    apply_components(world, dimension, target, stack);
    let placed = world.block_state(dimension, target).unwrap_or(state);
    let sound = vanilla_block_place_sound_26_1(placed);
    let sink = &mut world.level_mut_by_id(&dimension).event_sink;
    sink.play_block_sound_with_volume_pitch(target, sound.sound, sound.volume, sound.pitch);
    sink.game_event(VanillaGameEvent::BLOCK_PLACE, target, None);
    true
}
