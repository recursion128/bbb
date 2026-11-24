//! `PlaceOnWaterBlockItem.use` (`PlaceOnWaterBlockItem.java:23-27`): a lily
//! pad or frogspawn goes onto the cell above the fluid source the player
//! looks at.

use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::item_types::vanilla_item_type;
use bbb_server_types::directions::relative;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::{ItemUseContext, ItemUseResult};
use crate::world::pov_clip::{player_pov_clip, ClipFluid};
use crate::world::{ServerWorld, WorldEvent};
use bbb_protocol::packets::Direction;

pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    // Both items share their block's id (`Items.java`).
    let Some(block_type_id) = context
        .item
        .item_id
        .and_then(vanilla_item_type)
        .and_then(|item| vanilla_block_type_for_resource_id(item.resource_id))
        .map(|block| block.id)
    else {
        return ItemUseResult::Pass;
    };
    let Some(hit) = player_pov_clip(world, context.connection_id, ClipFluid::SourceOnly) else {
        return ItemUseResult::Pass;
    };
    let above = relative(hit.pos, Direction::Up);
    if !world.player_may_place_block(context.connection_id, &context.item, above) {
        return ItemUseResult::Fail;
    }
    let held = RuntimeItemStack::from_summary(context.item.clone());
    if world.place_default_block_from_item(
        context.connection_id,
        context.dimension,
        above,
        hit.direction,
        block_type_id,
        context.hand,
        &held,
        None,
        events,
    ) {
        ItemUseResult::Success
    } else {
        ItemUseResult::Fail
    }
}
