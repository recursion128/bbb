//! Finalization and publication for block-origin mob spawns.

use crate::entity::SpawnReason;
use crate::world::LevelId;
use bbb_protocol::packets::Vec3d;

use super::ServerWorld;

impl ServerWorld {
    /// `NetherPortalBlock.randomTick`'s STRUCTURE spawn; the portal then cools
    /// root and vehicle (`NetherPortalBlock.java:77-84`).
    pub(super) fn spawn_finalized_portal_zombified_piglin(
        &mut self,
        dimension: impl Into<LevelId>,
        position: Vec3d,
        y_rot: f32,
    ) -> Option<i32> {
        let level = dimension.into();
        let root_id = self.with_spawn_runtime(&level, &[], |runtime, random, _| {
            let tree = runtime
                .entities
                .create_portal_zombified_piglin(position, y_rot)?;
            let root_id = runtime.finalize_and_add(tree, SpawnReason::Structure, random)?;
            let entities = &mut *runtime.entities;
            for id in std::iter::once(root_id).chain(entities.vehicle(root_id)) {
                if let Some(entity) = entities.get_mut(id) {
                    entity.set_portal_cooldown();
                }
            }
            Some(root_id)
        })??;
        self.play_spawn_ambient(level, root_id);
        Some(root_id)
    }
}
