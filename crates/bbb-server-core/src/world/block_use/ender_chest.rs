use super::BlockUseContext;
use crate::world::{ServerWorld, WorldEvent};

pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    world.use_ender_chest_without_item(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        events,
    )
}
