//! Session pose selection and fit checks.

#[cfg(test)]
use crate::connection::ConnectionId;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_CROUCHING_ID, VANILLA_ENTITY_POSE_FALL_FLYING_ID,
    VANILLA_ENTITY_POSE_STANDING_ID, VANILLA_ENTITY_POSE_SWIMMING_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING, VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
    VANILLA_ENTITY_SHARED_FLAG_SWIMMING,
};
use bbb_server_types::geometry::{visit_block_collisions_with_scratch, Aabb};
use bbb_server_types::interaction::player_pose_bounding_box;

use super::movement_collision::{moving_piston_at, push_block_collision_shape, CollisionSnapshot};
use super::{GameType, ServerWorld};

impl ServerWorld {
    #[cfg(test)]
    pub(crate) fn player_pose_id_for_test(&self, connection_id: ConnectionId) -> i32 {
        self.levels[connection_id].pose_id
    }

    /// `Player.updatePlayerPose` (`Player.java:342-374`).
    pub(super) fn tick_player_poses(&mut self) {
        if self.levels.player_count() == 0 {
            return;
        }
        let connections: Vec<_> = self.levels.player_keys().copied().collect();
        for connection_id in connections {
            let player = self.levels.player_at(connection_id);
            if player.avatar_removed || !player.body_tick_enabled {
                continue;
            }
            let desired = if player.cold.sleeping_pos.is_some() {
                bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID
            } else if player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SWIMMING) {
                VANILLA_ENTITY_POSE_SWIMMING_ID
            } else if player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING) {
                VANILLA_ENTITY_POSE_FALL_FLYING_ID
            } else if player.using_item.is_auto_spin_attack() {
                bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SPIN_ATTACK_ID
            } else if player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN)
                && !player.abilities.flying
            {
                VANILLA_ENTITY_POSE_CROUCHING_ID
            } else {
                VANILLA_ENTITY_POSE_STANDING_ID
            };
            let query_bounds = candidate_pose_bounds(player, desired);
            let mut snapshot: Option<CollisionSnapshot> = None;
            let mut fits = |pose_id| {
                self.player_fits_pose_lazy(
                    player,
                    connection_id,
                    pose_id,
                    query_bounds,
                    &mut snapshot,
                )
            };
            if !fits(VANILLA_ENTITY_POSE_SWIMMING_ID) {
                continue;
            }
            let riding = self
                .level_by_id(&player.dimension)
                .entities
                .external_player_vehicle(player.entity_id())
                .is_some();
            let actual = if player.game_type == GameType::Spectator || riding || fits(desired) {
                desired
            } else if fits(VANILLA_ENTITY_POSE_CROUCHING_ID) {
                VANILLA_ENTITY_POSE_CROUCHING_ID
            } else {
                VANILLA_ENTITY_POSE_SWIMMING_ID
            };
            self.levels[connection_id].set_pose(actual);
        }
    }

    fn player_fits_pose_lazy(
        &self,
        player: &super::PlayerState,
        connection_id: crate::connection::ConnectionId,
        pose_id: i32,
        query_bounds: Option<Aabb>,
        snapshot: &mut Option<CollisionSnapshot>,
    ) -> bool {
        let Ok(bounds) = player_pose_bounding_box(player.position(), pose_id, player.pose_scale())
        else {
            return false;
        };
        let bounds = bounds.inflate(-1.0e-7, -1.0e-7, -1.0e-7);
        let level = self.level_by_id(&player.dimension);
        if !level
            .world_border
            .no_collision_at(player.position(), bounds, self.game_time)
            || blocks_intersect_bounds(&level.chunks, bounds)
        {
            return false;
        }
        let snapshot = snapshot.get_or_insert_with(|| {
            let query = query_bounds.unwrap_or(bounds);
            self.player_collision_snapshot_in(&player.dimension, connection_id, query)
        });
        !snapshot
            .entity_boxes
            .iter()
            .chain(&snapshot.player_boxes)
            .any(|shape| shape.intersects(bounds))
    }
}

fn candidate_pose_bounds(player: &super::PlayerState, desired: i32) -> Option<Aabb> {
    let mut union: Option<Aabb> = None;
    for pose in [
        VANILLA_ENTITY_POSE_SWIMMING_ID,
        VANILLA_ENTITY_POSE_CROUCHING_ID,
        desired,
    ] {
        let Ok(bounds) = player_pose_bounding_box(player.position(), pose, player.pose_scale())
        else {
            continue;
        };
        let bounds = bounds.inflate(-1.0e-7, -1.0e-7, -1.0e-7);
        union = Some(match union {
            None => bounds,
            Some(existing) => Aabb {
                min_x: existing.min_x.min(bounds.min_x),
                min_y: existing.min_y.min(bounds.min_y),
                min_z: existing.min_z.min(bounds.min_z),
                max_x: existing.max_x.max(bounds.max_x),
                max_y: existing.max_y.max(bounds.max_y),
                max_z: existing.max_z.max(bounds.max_z),
            },
        });
    }
    union
}

fn blocks_intersect_bounds(chunks: &crate::chunk::ChunkStore, bounds: Aabb) -> bool {
    let mut hit = false;
    let _ = visit_block_collisions_with_scratch(
        bounds,
        |pos| moving_piston_at(chunks, pos).unwrap_or(true),
        |pos, out| push_block_collision_shape(chunks, pos, None, out),
        |_, shape| {
            if shape.intersects(bounds) {
                hit = true;
                false
            } else {
                true
            }
        },
    );
    hit
}

#[cfg(test)]
mod tests;

impl super::PlayerState {
    pub(super) fn dimensions(&self) -> bbb_server_types::Result<[f32; 2]> {
        bbb_server_types::interaction::player_pose_dimensions(self.pose_id, self.pose_scale())
    }

    pub(super) fn bounding_box(
        &self,
    ) -> bbb_server_types::Result<bbb_server_types::geometry::Aabb> {
        player_pose_bounding_box(self.position(), self.pose_id, self.pose_scale())
    }

    /// `Avatar.POSES` (Avatar.java:21-37), scaled by LivingEntity dimensions.
    pub(super) fn eye_height(&self) -> bbb_server_types::Result<f32> {
        use bbb_protocol::entity_poses::{
            VANILLA_ENTITY_POSE_DYING_ID, VANILLA_ENTITY_POSE_SLEEPING_ID,
            VANILLA_ENTITY_POSE_SPIN_ATTACK_ID,
        };
        let height = match self.pose_id {
            VANILLA_ENTITY_POSE_CROUCHING_ID => 1.27_f32,
            VANILLA_ENTITY_POSE_FALL_FLYING_ID
            | VANILLA_ENTITY_POSE_SWIMMING_ID
            | VANILLA_ENTITY_POSE_SPIN_ATTACK_ID => 0.4_f32,
            VANILLA_ENTITY_POSE_SLEEPING_ID => 0.2_f32,
            _ => bbb_server_types::interaction::player_eye_height()?,
        };
        Ok(
            if matches!(
                self.pose_id,
                VANILLA_ENTITY_POSE_SLEEPING_ID | VANILLA_ENTITY_POSE_DYING_ID
            ) {
                height
            } else {
                height * self.pose_scale()
            },
        )
    }

    pub(super) fn eye_position(&self) -> bbb_server_types::Result<bbb_protocol::packets::Vec3d> {
        Ok(bbb_protocol::packets::Vec3d {
            y: self.position().y + f64::from(self.eye_height()?),
            ..self.position()
        })
    }
}
