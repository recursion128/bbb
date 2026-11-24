//! `MovingPistonBlock.useWithoutItem` (`MovingPistonBlock.java:83-91`): drop
//! an orphan cell that lost its moving block entity.

use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::world::LevelId;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::VANILLA_BLOCK_MOVING_PISTON_ID;
use bbb_protocol::packets::BlockPos;
use bbb_server_types::fluid::air_block_state_id;

use super::BlockUseContext;
use crate::world::{ServerWorld, WorldEvent};

/// The `matches:` column: `Blocks.MOVING_PISTON`.
pub(super) fn is_moving_piston(block_state_id: i32) -> bool {
    vanilla_block_state_block_type_id(block_state_id) == Some(VANILLA_BLOCK_MOVING_PISTON_ID)
}

/// The `without_item:` column: [`ServerWorld::use_moving_piston_without_item`].
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    _events: &mut Vec<WorldEvent>,
) -> bool {
    world.use_moving_piston_without_item(context.dimension, context.update.hit.pos)
}

impl ServerWorld {
    /// `MovingPistonBlock.useWithoutItem` (`MovingPistonBlock.java:83-91`).
    pub(super) fn use_moving_piston_without_item(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
    ) -> bool {
        let dimension = dimension.into();
        if self
            .level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(pos)
            .is_some()
        {
            return false;
        }
        let _ = self.set_block(dimension, pos, air_block_state_id(), SET_BLOCK_UPDATE_ALL);
        true
    }

    /// Test-only: drop the moving-piston object so the orphan `removeBlock`
    /// arm can run (`MovingPistonBlock.java:86`).
    #[cfg(test)]
    pub(crate) fn remove_block_entity_for_test(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
    ) {
        let dimension = dimension.into();
        self.level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .remove(pos);
    }
}
