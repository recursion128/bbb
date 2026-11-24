//! `BarrelBlock` (`BarrelBlock.java`), whose `useWithoutItem` and
//! `getMenuProvider` both open the barrel the container surface owns.

use crate::world::{ServerWorld, WorldEvent};

use super::BlockUseContext;

/// `BarrelBlock.useWithoutItem`, which is
/// [`ServerWorld::use_barrel_without_item`] over in [`crate::world`]'s container
/// module: the menu it opens is the one the container surface owns, not
/// something this file builds.
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    world.use_barrel_without_item(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        events,
    )
}

/// The `MenuProvider` the spectator arm of `ServerPlayerGameMode.useItemOn`
/// reaches, which is [`ServerWorld::open_barrel_menu`] and the same menu.
pub(super) fn open_menu(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    world.open_barrel_menu(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        events,
    )
}
