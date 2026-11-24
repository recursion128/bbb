use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{vanilla_block_type, VanillaBlockUseWithoutItemOverride};

use crate::world::{ServerWorld, WorldEvent};

use super::BlockUseContext;

pub(super) fn is_crafter(block_state_id: i32) -> bool {
    vanilla_block_state_block_type_id(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|block| block.use_without_item_override)
        == Some(VanillaBlockUseWithoutItemOverride::CrafterBlock)
}

/// `CrafterBlock.useWithoutItem` (`CrafterBlock.java:139-147`).
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    open_menu(world, context, events);
    true
}

pub(super) fn open_menu(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    world.open_crafter_menu(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        events,
    )
}
