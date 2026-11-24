use super::*;
use crate::entity::create_trader_event_root;
use crate::world::LevelId;
use bbb_protocol::packets::Vec3d;

impl ServerWorld {
    pub(in super::super) fn spawn_trader_event(
        &mut self,
        dimension: &LevelId,
        resource_id: &str,
        position: BlockPos,
    ) -> Option<i32> {
        self.with_spawn_runtime(dimension, &[], |runtime, random, _| {
            // EntityType.java:1296-1297, EntityType.java:1327-1344. This overload never moves down.
            let mut tree = create_trader_event_root(
                runtime.entities,
                resource_id,
                Vec3d {
                    x: f64::from(position.x) + 0.5,
                    y: f64::from(position.y),
                    z: f64::from(position.z) + 0.5,
                },
            )?;
            let degrees = random.next_float() * 360.0;
            let yaw = if degrees >= 180.0 {
                degrees - 360.0
            } else {
                degrees
            };
            tree.set_spawn_rotation(yaw);
            let id = runtime.finalize_and_add(tree, SpawnReason::Event, random)?;
            // EntityType.java:1308-1313: publication precedes the ambient sound.
            runtime.entities.play_trader_spawn_ambient(id);
            Some(id)
        })?
    }
}
