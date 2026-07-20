use std::collections::HashMap;

use hecs::Entity;

use super::components::{EntityIdentity, EntityTransform};
use super::schema::{
    vanilla_entity_metadata_type_client_movement_policy, VanillaEntityClientMovementPolicy,
};
use super::{
    EntityStore, EntityVec3, EyeOfEnderTrailParticleState, ShulkerBulletTrailParticleState,
};

pub(super) fn client_movement_uses_in_water(entity_type_id: i32) -> bool {
    matches!(
        vanilla_entity_metadata_type_client_movement_policy(entity_type_id),
        Some(VanillaEntityClientMovementPolicy::EyeOfEnder)
    )
}

impl EntityStore {
    /// Advances client-owned transform work whose Vanilla tick never performs
    /// a client-side block/entity hit decision. Collision-bearing projectile,
    /// vehicle, and item policies remain with their dedicated owners.
    pub(super) fn advance_direct_client_movement(
        &mut self,
        scheduled: &[Entity],
        in_water_by_id: &HashMap<i32, bool>,
    ) {
        let mut eye_particles = Vec::new();
        let mut shulker_particles = Vec::new();
        for &entity in scheduled {
            let Ok((identity, transform)) = self
                .ecs
                .query_one_mut::<(&EntityIdentity, &mut EntityTransform)>(entity)
            else {
                continue;
            };
            match vanilla_entity_metadata_type_client_movement_policy(identity.entity_type_id) {
                Some(VanillaEntityClientMovementPolicy::EyeOfEnder) => {
                    // `EyeOfEnder.tick`: `super.tick()` has already run through
                    // the projectile lifecycle pass, then the client keeps the
                    // server-provided velocity and advances one position step.
                    advance_position(transform);
                    eye_particles.push(EyeOfEnderTrailParticleState {
                        entity_id: identity.id,
                        origin: EntityVec3 {
                            x: transform.position.x - transform.delta_movement.x * 0.25,
                            y: transform.position.y - transform.delta_movement.y * 0.25,
                            z: transform.position.z - transform.delta_movement.z * 0.25,
                        },
                        delta_movement: transform.delta_movement,
                        in_water: in_water_by_id.get(&identity.id).copied().unwrap_or(false),
                    });
                }
                Some(VanillaEntityClientMovementPolicy::ShulkerBullet) => {
                    // `ShulkerBullet.tick` changes its homing vector only on
                    // the server. The client integrates that synchronized
                    // vector and eases rotation toward it by one half.
                    advance_position(transform);
                    rotate_towards_movement(transform, 0.5);
                    shulker_particles.push(ShulkerBulletTrailParticleState {
                        entity_id: identity.id,
                        position: EntityVec3 {
                            x: transform.position.x - transform.delta_movement.x,
                            y: transform.position.y - transform.delta_movement.y + 0.15,
                            z: transform.position.z - transform.delta_movement.z,
                        },
                    });
                }
                _ => {}
            }
        }
        self.pending_eye_of_ender_trail_particles
            .extend(eye_particles);
        self.pending_shulker_bullet_trail_particles
            .extend(shulker_particles);
    }
}

fn advance_position(transform: &mut EntityTransform) {
    transform.position.x += transform.delta_movement.x;
    transform.position.y += transform.delta_movement.y;
    transform.position.z += transform.delta_movement.z;
}

fn rotate_towards_movement(transform: &mut EntityTransform, rotation_speed: f32) {
    let movement = transform.delta_movement;
    if movement == EntityVec3::default() {
        return;
    }
    let horizontal = (movement.x * movement.x + movement.z * movement.z).sqrt();
    let target_y_rot = movement.z.atan2(movement.x).to_degrees() as f32 + 90.0;
    let target_x_rot = horizontal.atan2(movement.y).to_degrees() as f32 - 90.0;
    transform.old_x_rot = unwrap_old_rotation(transform.old_x_rot, target_x_rot);
    transform.old_y_rot = unwrap_old_rotation(transform.old_y_rot, target_y_rot);
    transform.x_rot = lerp(rotation_speed, transform.old_x_rot, target_x_rot);
    transform.y_rot = lerp(rotation_speed, transform.old_y_rot, target_y_rot);
}

fn unwrap_old_rotation(mut old: f32, target: f32) -> f32 {
    while target - old < -180.0 {
        old -= 360.0;
    }
    while target - old >= 180.0 {
        old += 360.0;
    }
    old
}

fn lerp(alpha: f32, start: f32, end: f32) -> f32 {
    start + alpha * (end - start)
}

#[cfg(test)]
mod tests {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID, VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
    };
    use bbb_protocol::packets::{AddEntity as ProtocolAddEntity, Vec3d as ProtocolVec3d};
    use uuid::Uuid;

    use crate::WorldStore;

    use super::*;

    #[test]
    fn direct_projectiles_integrate_synced_motion_and_shulker_rotation() {
        let mut store = WorldStore::new();
        store.apply_add_entity(projectile(
            10,
            VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID,
            ProtocolVec3d {
                x: 0.25,
                y: -0.5,
                z: 1.0,
            },
        ));
        store.apply_add_entity(projectile(
            20,
            VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
            ProtocolVec3d {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        ));
        let generation_a = store.publication_snapshot();

        store.advance_entity_client_animations(1);

        assert_eq!(
            store.take_eye_of_ender_trail_particle_states(),
            vec![EyeOfEnderTrailParticleState {
                entity_id: 10,
                origin: EntityVec3 {
                    x: 1.1875,
                    y: 63.625,
                    z: -1.25,
                },
                delta_movement: EntityVec3 {
                    x: 0.25,
                    y: -0.5,
                    z: 1.0,
                },
                in_water: false,
            }]
        );
        assert_eq!(
            store.take_shulker_bullet_trail_particle_states(),
            vec![ShulkerBulletTrailParticleState {
                entity_id: 20,
                position: EntityVec3 {
                    x: 1.0,
                    y: 64.15,
                    z: -2.0,
                },
            }]
        );

        assert_eq!(
            store.probe_entity_transform(10).unwrap().position,
            EntityVec3 {
                x: 1.25,
                y: 63.5,
                z: -1.0,
            }
        );
        let bullet = store.probe_entity_transform(20).unwrap();
        assert_eq!(
            bullet.position,
            EntityVec3 {
                x: 2.0,
                y: 64.0,
                z: -2.0,
            }
        );
        assert_eq!(bullet.y_rot, 45.0);
        assert_eq!(bullet.x_rot, 0.0);
        assert_eq!(
            generation_a.probe_entity_transform(20).unwrap().position,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            "the retained generation remains isolated from client motion"
        );
    }

    fn projectile(
        id: i32,
        entity_type_id: i32,
        delta_movement: ProtocolVec3d,
    ) -> ProtocolAddEntity {
        ProtocolAddEntity {
            id,
            uuid: Uuid::from_u128(id as u128),
            entity_type_id,
            position: ProtocolVec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement,
            y_rot: 0.0,
            x_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        }
    }
}
