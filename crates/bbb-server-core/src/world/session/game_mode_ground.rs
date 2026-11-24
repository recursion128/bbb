//! `ServerPlayerGameMode.isInRangeOfGround` (`ServerPlayerGameMode.java:83-86`).

use super::*;
use crate::world::movement_collision::block_collision_boxes;
use bbb_server_types::geometry::Aabb;

impl ServerWorld {
    /// `FLIGHT_DISABLE_RANGE` (`:35`) is 1.0: not inside a collider, with one
    /// within a block below the feet. Only block colliders are consulted.
    pub(super) fn player_in_range_of_ground(&self, connection_id: ConnectionId) -> bool {
        let player = &self.levels[connection_id];
        let Ok(bounds) = player.bounding_box() else {
            return false;
        };
        let chunks = &self.level_by_id(&player.dimension).chunks;
        let below = Aabb {
            min_y: bounds.min_y - 1.0,
            max_y: bounds.min_y,
            ..bounds
        };
        block_collision_boxes(chunks, bounds).is_empty()
            && !block_collision_boxes(chunks, below).is_empty()
    }
}
