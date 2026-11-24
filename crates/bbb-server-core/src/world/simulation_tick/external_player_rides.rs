//! The two post-entity-loop seams an external session's own vehicle needs:
//! the shift dismount and the vehicle move finalization.

use super::*;
use crate::world::LevelId;

use crate::world::external_player_ride_events::DismountLanding;

impl ServerWorld {
    pub(super) fn tick_external_player_ride_dismounts(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        let shift_dismounts = self
            .levels
            .player_entries()
            .filter(|(_, player)| {
                &player.dimension == dimension
                    && !player.avatar_removed
                    && player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN)
                    && self
                        .level_by_id(dimension)
                        .entities
                        .external_player_vehicle(player.entity_id())
                        .is_some()
            })
            .map(|(&connection_id, _)| connection_id)
            .collect::<Vec<_>>();
        for connection_id in shift_dismounts {
            if self
                .stop_external_player_ride(
                    dimension,
                    connection_id,
                    DismountLanding::Search,
                    events,
                )
                .unwrap_or(false)
            {
                self.apply_level_entity_lifecycle_by_key(dimension, events);
                self.levels[connection_id]
                    .set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, false);
            }
        }
    }

    pub(in crate::world) fn finalize_external_player_boat_move_by_key(
        &mut self,
        dimension: &LevelId,
        finalization: crate::entity::ExternalPlayerBoatMoveFinalization,
        events: &mut Vec<WorldEvent>,
    ) {
        let players = self.nearby_players_by_key(dimension);
        let mut leftover = PendingEntitySpawns::default();
        self.with_level_ctx(dimension, &players, &mut leftover, |level, rest| {
            rest.entities
                .finalize_external_player_boat_move(finalization, level);
        });
        // The frame the finalization ran on is over.
        self.post_pending_game_events(dimension);
        self.apply_pending_spawns(dimension, leftover);
        self.apply_pending_player_hurts(dimension, events);
    }
}
