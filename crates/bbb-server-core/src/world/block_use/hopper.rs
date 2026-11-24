use crate::world::{ServerWorld, WorldEvent};

use super::BlockUseContext;

/// `HopperBlock.useWithoutItem` (`HopperBlock.java:106-115`).
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    let pos = context.update.hit.pos;
    if world
        .get_block_entity(context.dimension, pos)
        .and_then(crate::block_entity::BlockEntity::hopper)
        .is_some()
    {
        open_menu(world, context, events);
        world.award_opener_stat(context.connection_id, "minecraft:inspect_hopper");
    }
    true
}

pub(super) fn open_menu(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    world.open_hopper_menu(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        events,
    )
}
