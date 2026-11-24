//! The local player's `Entity.getDimensions(getPose())`: `Avatar.POSES`
//! (Avatar.java:21-35) scaled by `LivingEntity.getScale()`
//! (LivingEntity.java:536-539, :3664).

use bbb_protocol::packets::Vec3d;

use super::local_player::LocalPlayerPoseState;
use super::local_player_collision::WorldCollisionBox;
use crate::WorldStore;

// Unscaled sizes stay these f64 values, which the replay goldens pin; a
// scaled size is computed in f32 like `EntityDimensions.scale`.
const WIDTH: (f64, f32) = (0.6, 0.6);
const STANDING: [(f64, f32); 2] = [(1.8, 1.8), (1.62, 1.62)];
const CROUCHING: [(f64, f32); 2] = [(1.5, 1.5), (1.27, 1.27)];
/// SWIMMING, FALL_FLYING and SPIN_ATTACK share one box.
const LOW: [(f64, f32); 2] = [(0.6, 0.6), (0.4, 0.4)];
/// `LivingEntity.SLEEPING_DIMENSIONS`: fixed, so never scaled.
const SLEEPING: f32 = 0.2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalPlayerDimensions {
    pub width: f64,
    pub height: f64,
    pub eye_height: f64,
}

impl LocalPlayerDimensions {
    pub(crate) fn aabb_at(self, position: Vec3d) -> ([f64; 3], [f64; 3]) {
        let half_width = self.width / 2.0;
        (
            [position.x - half_width, position.y, position.z - half_width],
            [
                position.x + half_width,
                position.y + self.height,
                position.z + half_width,
            ],
        )
    }

    pub(in crate::client) fn bounds_at(self, position: Vec3d) -> WorldCollisionBox {
        let (min, max) = self.aabb_at(position);
        WorldCollisionBox { min, max }
    }
}

impl WorldStore {
    /// `pose` carries the crouch and swim/crawl body pose; sleeping, fall
    /// flying and the riptide spin come from the self entity's synced data,
    /// as `Player.getDesiredPose` ranks them above crouching.
    pub fn local_player_dimensions(&self, pose: LocalPlayerPoseState) -> LocalPlayerDimensions {
        let id = self.local_player_id;
        if id.is_some_and(|id| self.entities.sleeping_pos(id).is_some()) {
            let size = f64::from(SLEEPING);
            return LocalPlayerDimensions {
                width: size,
                height: size,
                eye_height: size,
            };
        }
        self.local_player_body_pose_dimensions(pose, self.local_player_glides())
    }

    /// `getDimensions` of the body pose alone, for the fit checks of
    /// `Player.updatePlayerPose` and `LocalPlayer.aiStep`'s crouching.
    pub(in crate::client) fn local_player_body_pose_dimensions(
        &self,
        pose: LocalPlayerPoseState,
        glides: bool,
    ) -> LocalPlayerDimensions {
        let [height, eye_height] = if pose.swimming || glides {
            LOW
        } else if pose.sneaking {
            CROUCHING
        } else {
            STANDING
        };
        let scale = self
            .local_player_id
            .and_then(|id| self.entities.render_scale(id))
            .unwrap_or(1.0);
        let scaled = |(unscaled, size): (f64, f32)| {
            if scale == 1.0 {
                unscaled
            } else {
                f64::from(size * scale)
            }
        };
        LocalPlayerDimensions {
            width: scaled(WIDTH),
            height: scaled(height),
            eye_height: scaled(eye_height),
        }
    }

    pub fn local_player_eye_height(&self, pose: LocalPlayerPoseState) -> f64 {
        self.local_player_dimensions(pose).eye_height
    }

    pub(in crate::client) fn local_player_bounds(
        &self,
        pose: LocalPlayerPoseState,
    ) -> WorldCollisionBox {
        self.local_player_dimensions(pose).bounds_at(pose.position)
    }

    /// Vanilla `isFallFlying() || isAutoSpinAttack()` for the local player.
    pub(in crate::client) fn local_player_glides(&self) -> bool {
        self.local_player_id.is_some_and(|id| {
            self.entities.is_fall_flying(id) == Some(true)
                || self.entities.is_auto_spin_attack(id) == Some(true)
        })
    }
}
