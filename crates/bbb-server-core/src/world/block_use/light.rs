//! `LightBlock.useWithoutItem` (`LightBlock.java:54-63`).

use super::BlockUseContext;
use crate::block_behavior::light as light_block;
use crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS;
use crate::world::{ServerWorld, WorldEvent};

pub(super) use light_block::is_light;

/// The `without_item:` column: [`ServerWorld::use_light_without_item`].
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    _events: &mut Vec<WorldEvent>,
) -> bool {
    world.use_light_without_item(context)
}

impl ServerWorld {
    /// `LightBlock.useWithoutItem` (`LightBlock.java:54-63`).
    ///
    /// The server is never `isClientSide`, so a player without
    /// `canUseGameMasterBlocks` (`Player.java:1840-1842`) still `CONSUME`s.
    pub(super) fn use_light_without_item(&mut self, context: &BlockUseContext<'_>) -> bool {
        if !self.can_use_game_master_blocks(context.connection_id) {
            return true;
        }
        let Some(cycled) = light_block::cycled_level(context.block_state_id) else {
            return true;
        };
        let _ = self.set_block(
            context.dimension,
            context.update.hit.pos,
            cycled,
            SET_BLOCK_UPDATE_CLIENTS,
        );
        self.mark_use_item_on_server_swing();
        true
    }
}
