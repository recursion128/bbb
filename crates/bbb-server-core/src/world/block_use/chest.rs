//! `ChestBlock` (`ChestBlock.java`), whose `useWithoutItem` and
//! `getMenuProvider` are the same body: the chest menu the container surface
//! owns.
//!

use crate::world::{ServerWorld, WorldEvent};

use super::BlockUseContext;

/// `ChestBlock.useWithoutItem`, which is
/// [`ServerWorld::use_chest_without_item`] over in [`crate::world`]'s container
/// module: the menu it opens is the one the container surface owns, not
/// something this file builds.
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    world.use_chest_without_item(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        context.block_state_id,
        events,
    )
}

/// The `MenuProvider` the spectator arm of `ServerPlayerGameMode.useItemOn`
/// reaches, which is [`ServerWorld::open_chest_menu`] and the same menu.
pub(super) fn open_menu(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    world.open_chest_menu(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        context.block_state_id,
        events,
    )
}
