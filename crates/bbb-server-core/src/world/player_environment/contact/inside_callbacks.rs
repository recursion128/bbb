use super::{ConnectionId, GameType, ServerWorld};
use crate::block_behavior::{self, footing, InsideEntity};
use bbb_protocol::packets::BlockPos;

impl ServerWorld {
    pub(in crate::world) fn player_block_trigger_inside(
        &mut self,
        id: ConnectionId,
        pos: BlockPos,
        state: i32,
    ) -> Option<()> {
        if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
            == Some(bbb_protocol::block_types::VANILLA_BLOCK_BIG_DRIPLEAF_ID)
        {
            let player = self.levels.player(id)?;
            let (dimension, on_ground, y) =
                (player.dimension, player.on_ground(), player.position().y);
            self.with_level(dimension, |level| {
                block_behavior::big_dripleaf::entity_inside(level, pos, state, on_ground, y)
            });
            return Some(());
        }
        if !footing::is_pressure_plate(state) && !block_behavior::tripwire::is_tripwire(state) {
            return Some(());
        }
        let player = self.levels.player(id)?;
        let dimension = player.dimension;
        let inside = InsideEntity {
            source: Some(player.entity_id()),
            bounds: player.bounding_box().ok()?,
            living: true,
            minecart: false,
            ignores_block_triggers: player.game_type == GameType::Spectator,
        };
        self.with_level(dimension, |level| {
            block_behavior::entity_inside(level, pos, state, inside)
        })
        .then_some(())
    }
}

#[cfg(test)]
#[path = "session_pressure_plate_tests.rs"]
mod tests;
