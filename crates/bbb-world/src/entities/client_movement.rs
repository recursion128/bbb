use std::collections::HashMap;

use bbb_protocol::packets::EntityDataValueKind;
use hecs::Entity;

use super::components::{EntityHurtingProjectile, EntityIdentity, EntityMetadata, EntityTransform};
use super::schema::{
    vanilla_entity_metadata_entity_accessors, vanilla_entity_metadata_type_client_movement_policy,
    vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type,
    VanillaEntityClientMovementPolicy,
};
use super::{
    EntityClientMovementParticleState, EntityClientMovementSimpleParticleKind,
    EntityClientMovementSimpleParticleState, EntityStore, EntityVec3, EyeOfEnderTrailParticleState,
    FireworkRocketTrailParticleState, ShulkerBulletTrailParticleState,
};

pub(super) fn client_movement_uses_in_water(entity_type_id: i32) -> bool {
    matches!(
        vanilla_entity_metadata_type_client_movement_policy(entity_type_id),
        Some(
            VanillaEntityClientMovementPolicy::EyeOfEnder
                | VanillaEntityClientMovementPolicy::ThrowableProjectile
                | VanillaEntityClientMovementPolicy::ThrownExperienceBottle
                | VanillaEntityClientMovementPolicy::ThrownPotion
                | VanillaEntityClientMovementPolicy::HurtingProjectile
                | VanillaEntityClientMovementPolicy::WindCharge
                | VanillaEntityClientMovementPolicy::DragonFireball
                | VanillaEntityClientMovementPolicy::WitherSkull
        )
    )
}

impl EntityStore {
    /// Advances client-owned projectile free flight in the frozen entity
    /// schedule. Server-authoritative hit consequences and removals are never
    /// manufactured here; incoming packets remain their canonical owner.
    /// Vehicle, item, orb, falling-block, TNT, and fishing-hook movement stay
    /// with their dedicated collision/authority owners.
    pub(super) fn advance_direct_client_movement(
        &mut self,
        scheduled: &[Entity],
        in_water_by_id: &HashMap<i32, bool>,
    ) {
        let mut particles = Vec::new();
        for &entity in scheduled {
            let (entity_id, entity_type_id) = {
                let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                    continue;
                };
                (identity.id, identity.entity_type_id)
            };
            let Some(policy) = vanilla_entity_metadata_type_client_movement_policy(entity_type_id)
            else {
                continue;
            };
            match policy {
                VanillaEntityClientMovementPolicy::ThrowableProjectile
                | VanillaEntityClientMovementPolicy::ThrownExperienceBottle
                | VanillaEntityClientMovementPolicy::ThrownPotion => {
                    let Ok((metadata, transform)) = self
                        .ecs
                        .query_one_mut::<(&EntityMetadata, &mut EntityTransform)>(entity)
                    else {
                        continue;
                    };
                    advance_throwable_projectile(
                        entity_id,
                        metadata,
                        transform,
                        policy,
                        in_water_by_id.get(&entity_id).copied().unwrap_or(false),
                        &mut particles,
                    );
                }
                VanillaEntityClientMovementPolicy::HurtingProjectile
                | VanillaEntityClientMovementPolicy::WindCharge
                | VanillaEntityClientMovementPolicy::DragonFireball
                | VanillaEntityClientMovementPolicy::WitherSkull => {
                    let Ok((metadata, hurting_projectile, transform)) =
                        self.ecs.query_one_mut::<(
                            &EntityMetadata,
                            Option<&EntityHurtingProjectile>,
                            &mut EntityTransform,
                        )>(entity)
                    else {
                        continue;
                    };
                    advance_hurting_projectile(
                        entity_id,
                        entity_type_id,
                        metadata,
                        hurting_projectile,
                        transform,
                        policy,
                        in_water_by_id.get(&entity_id).copied().unwrap_or(false),
                        &mut particles,
                    );
                }
                VanillaEntityClientMovementPolicy::EyeOfEnder
                | VanillaEntityClientMovementPolicy::ShulkerBullet
                | VanillaEntityClientMovementPolicy::FireworkRocket => {
                    let Ok(mut transform) = self.ecs.get::<&mut EntityTransform>(entity) else {
                        continue;
                    };
                    match policy {
                        VanillaEntityClientMovementPolicy::EyeOfEnder => {
                            // `EyeOfEnder.tick`: `super.tick()` has already run through
                            // the projectile lifecycle pass, then the client keeps the
                            // server-provided velocity and advances one position step.
                            advance_position(&mut transform);
                            particles.push(EntityClientMovementParticleState::EyeOfEnder(
                                EyeOfEnderTrailParticleState {
                                    entity_id,
                                    origin: EntityVec3 {
                                        x: transform.position.x - transform.delta_movement.x * 0.25,
                                        y: transform.position.y - transform.delta_movement.y * 0.25,
                                        z: transform.position.z - transform.delta_movement.z * 0.25,
                                    },
                                    delta_movement: transform.delta_movement,
                                    in_water: in_water_by_id
                                        .get(&entity_id)
                                        .copied()
                                        .unwrap_or(false),
                                },
                            ));
                        }
                        VanillaEntityClientMovementPolicy::ShulkerBullet => {
                            // `ShulkerBullet.tick` changes its homing vector only on
                            // the server. The client integrates that synchronized
                            // vector and eases rotation toward it by one half.
                            advance_position(&mut transform);
                            rotate_towards_movement(&mut transform, 0.5);
                            particles.push(EntityClientMovementParticleState::ShulkerBullet(
                                ShulkerBulletTrailParticleState {
                                    entity_id,
                                    position: EntityVec3 {
                                        x: transform.position.x - transform.delta_movement.x,
                                        y: transform.position.y - transform.delta_movement.y + 0.15,
                                        z: transform.position.z - transform.delta_movement.z,
                                    },
                                },
                            ));
                        }
                        VanillaEntityClientMovementPolicy::FireworkRocket => {
                            particles.push(EntityClientMovementParticleState::FireworkRocket(
                                FireworkRocketTrailParticleState {
                                    entity_id,
                                    position: transform.position,
                                    delta_movement: transform.delta_movement,
                                },
                            ));
                        }
                        _ => unreachable!("narrow direct movement policy match"),
                    }
                }
                _ => {}
            }
        }
        self.pending_entity_client_movement_particles
            .extend(particles);
    }
}

fn advance_throwable_projectile(
    entity_id: i32,
    metadata: &EntityMetadata,
    transform: &mut EntityTransform,
    policy: VanillaEntityClientMovementPolicy,
    in_water: bool,
    particles: &mut Vec<EntityClientMovementParticleState>,
) {
    let no_gravity = vanilla_entity_metadata_entity_accessors().is_some_and(|accessors| {
        entity_data_bool(
            &metadata.data_values,
            accessors.no_gravity_data_id,
            accessors.no_gravity_default,
        )
    });
    if !no_gravity {
        transform.delta_movement.y -= throwable_gravity(policy);
    }

    let movement_before_inertia = transform.delta_movement;
    if in_water {
        append_bubbles(
            entity_id,
            transform.position,
            movement_before_inertia,
            particles,
        );
    }
    let inertia = f64::from(if in_water { 0.8_f32 } else { 0.99_f32 });
    transform.delta_movement = scale(movement_before_inertia, inertia);
    advance_position(transform);
    update_projectile_rotation(transform);
}

fn advance_hurting_projectile(
    entity_id: i32,
    entity_type_id: i32,
    metadata: &EntityMetadata,
    hurting_projectile: Option<&EntityHurtingProjectile>,
    transform: &mut EntityTransform,
    policy: VanillaEntityClientMovementPolicy,
    in_water: bool,
    particles: &mut Vec<EntityClientMovementParticleState>,
) {
    let movement_before_inertia = transform.delta_movement;
    if in_water {
        append_bubbles(
            entity_id,
            transform.position,
            movement_before_inertia,
            particles,
        );
    }
    let acceleration_power = hurting_projectile
        .map(|projectile| projectile.acceleration_power)
        .unwrap_or_else(|| {
            if matches!(policy, VanillaEntityClientMovementPolicy::WindCharge) {
                0.0
            } else {
                0.1
            }
        });
    let acceleration = scale(normalize(movement_before_inertia), acceleration_power);
    let inertia = hurting_projectile_inertia(policy, entity_type_id, metadata, in_water);
    transform.delta_movement = scale(add(movement_before_inertia, acceleration), inertia);
    rotate_towards_movement(transform, 0.2);
    advance_position(transform);

    let trail = match policy {
        VanillaEntityClientMovementPolicy::WindCharge => None,
        VanillaEntityClientMovementPolicy::DragonFireball => {
            Some(EntityClientMovementSimpleParticleKind::DragonBreath)
        }
        _ => Some(EntityClientMovementSimpleParticleKind::Smoke),
    };
    if let Some(kind) = trail {
        particles.push(EntityClientMovementParticleState::Simple(
            EntityClientMovementSimpleParticleState {
                entity_id,
                kind,
                position: EntityVec3 {
                    y: transform.position.y + 0.5,
                    ..transform.position
                },
                velocity: EntityVec3::default(),
            },
        ));
    }
}

fn throwable_gravity(policy: VanillaEntityClientMovementPolicy) -> f64 {
    match policy {
        VanillaEntityClientMovementPolicy::ThrownExperienceBottle => 0.07,
        VanillaEntityClientMovementPolicy::ThrownPotion => 0.05,
        _ => 0.03,
    }
}

fn hurting_projectile_inertia(
    policy: VanillaEntityClientMovementPolicy,
    entity_type_id: i32,
    metadata: &EntityMetadata,
    in_water: bool,
) -> f64 {
    if matches!(policy, VanillaEntityClientMovementPolicy::WindCharge) {
        return 1.0;
    }
    if in_water {
        return f64::from(0.8_f32);
    }
    let dangerous = matches!(policy, VanillaEntityClientMovementPolicy::WitherSkull)
        && vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type(entity_type_id)
            .is_some_and(|(data_id, default)| {
                entity_data_bool(&metadata.data_values, data_id, default)
            });
    f64::from(if dangerous { 0.73_f32 } else { 0.95_f32 })
}

fn append_bubbles(
    entity_id: i32,
    position: EntityVec3,
    movement: EntityVec3,
    particles: &mut Vec<EntityClientMovementParticleState>,
) {
    let position = EntityVec3 {
        x: position.x - movement.x * 0.25,
        y: position.y - movement.y * 0.25,
        z: position.z - movement.z * 0.25,
    };
    particles.extend((0..4).map(|_| {
        EntityClientMovementParticleState::Simple(EntityClientMovementSimpleParticleState {
            entity_id,
            kind: EntityClientMovementSimpleParticleKind::Bubble,
            position,
            velocity: movement,
        })
    }));
}

fn entity_data_bool(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: bool,
) -> bool {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match value.value {
            EntityDataValueKind::Boolean(value) => Some(value),
            _ => None,
        })
        .unwrap_or(default)
}

fn add(left: EntityVec3, right: EntityVec3) -> EntityVec3 {
    EntityVec3 {
        x: left.x + right.x,
        y: left.y + right.y,
        z: left.z + right.z,
    }
}

fn scale(value: EntityVec3, scale: f64) -> EntityVec3 {
    EntityVec3 {
        x: value.x * scale,
        y: value.y * scale,
        z: value.z * scale,
    }
}

fn normalize(value: EntityVec3) -> EntityVec3 {
    let length = (value.x * value.x + value.y * value.y + value.z * value.z).sqrt();
    if length < f64::from(1.0e-5_f32) {
        EntityVec3::default()
    } else {
        scale(value, length.recip())
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
    let target_y_rot = vanilla_degrees(movement.z.atan2(movement.x)) + 90.0;
    let target_x_rot = vanilla_degrees(horizontal.atan2(movement.y)) - 90.0;
    transform.old_x_rot = unwrap_old_rotation(transform.old_x_rot, target_x_rot);
    transform.old_y_rot = unwrap_old_rotation(transform.old_y_rot, target_y_rot);
    transform.x_rot = lerp(rotation_speed, transform.old_x_rot, target_x_rot);
    transform.y_rot = lerp(rotation_speed, transform.old_y_rot, target_y_rot);
}

fn update_projectile_rotation(transform: &mut EntityTransform) {
    let movement = transform.delta_movement;
    let horizontal = (movement.x * movement.x + movement.z * movement.z).sqrt();
    let target_x_rot = vanilla_degrees(movement.y.atan2(horizontal));
    let target_y_rot = vanilla_degrees(movement.x.atan2(movement.z));
    transform.old_x_rot = unwrap_old_rotation(transform.old_x_rot, target_x_rot);
    transform.old_y_rot = unwrap_old_rotation(transform.old_y_rot, target_y_rot);
    transform.x_rot = lerp(0.2, transform.old_x_rot, target_x_rot);
    transform.y_rot = lerp(0.2, transform.old_y_rot, target_y_rot);
}

fn vanilla_degrees(radians: f64) -> f32 {
    (radians * f64::from(180.0_f32) / f64::from(std::f32::consts::PI)) as f32
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
        VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID, VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID,
        VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID, VANILLA_ENTITY_TYPE_FIREBALL_ID,
        VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID, VANILLA_ENTITY_TYPE_SNOWBALL_ID,
        VANILLA_ENTITY_TYPE_SPLASH_POTION_ID, VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
        VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
    };
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, EntityDataValue, SetEntityData as ProtocolSetEntityData,
        Vec3d as ProtocolVec3d,
    };
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
            store
                .drain_entity_client_movement_particle_states()
                .collect::<Vec<_>>(),
            vec![
                EntityClientMovementParticleState::EyeOfEnder(EyeOfEnderTrailParticleState {
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
                }),
                EntityClientMovementParticleState::ShulkerBullet(ShulkerBulletTrailParticleState {
                    entity_id: 20,
                    position: EntityVec3 {
                        x: 1.0,
                        y: 64.15,
                        z: -2.0,
                    },
                },),
            ]
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

    #[test]
    fn throwable_projectiles_apply_generated_gravity_and_float_inertia() {
        let mut store = WorldStore::new();
        for (id, entity_type_id) in [
            (30, VANILLA_ENTITY_TYPE_SNOWBALL_ID),
            (31, VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID),
            (32, VANILLA_ENTITY_TYPE_SPLASH_POTION_ID),
            (33, VANILLA_ENTITY_TYPE_SNOWBALL_ID),
        ] {
            store.apply_add_entity(projectile(
                id,
                entity_type_id,
                ProtocolVec3d {
                    x: 0.25,
                    y: 0.5,
                    z: -0.75,
                },
            ));
        }
        let entity_accessors = vanilla_entity_metadata_entity_accessors().unwrap();
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 33,
            values: vec![EntityDataValue {
                data_id: entity_accessors.no_gravity_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            }],
        }));

        store.advance_entity_client_animations(1);

        let inertia = f64::from(0.99_f32);
        for (id, gravity) in [(30, 0.03), (31, 0.07), (32, 0.05)] {
            let transform = store.probe_entity_transform(id).unwrap();
            assert_vec3_near(
                transform.delta_movement,
                EntityVec3 {
                    x: 0.25 * inertia,
                    y: (0.5 - gravity) * inertia,
                    z: -0.75 * inertia,
                },
            );
            assert_vec3_near(
                transform.position,
                EntityVec3 {
                    x: 1.0 + 0.25 * inertia,
                    y: 64.0 + (0.5 - gravity) * inertia,
                    z: -2.0 - 0.75 * inertia,
                },
            );
        }
        let no_gravity = store.probe_entity_transform(33).unwrap();
        assert_vec3_near(
            no_gravity.delta_movement,
            EntityVec3 {
                x: 0.25 * inertia,
                y: 0.5 * inertia,
                z: -0.75 * inertia,
            },
        );
    }

    #[test]
    fn hurting_projectile_families_apply_acceleration_inertia_and_typed_trails() {
        let mut store = WorldStore::new();
        for (id, entity_type_id) in [
            (40, VANILLA_ENTITY_TYPE_FIREBALL_ID),
            (41, VANILLA_ENTITY_TYPE_WIND_CHARGE_ID),
            (42, VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID),
            (43, VANILLA_ENTITY_TYPE_WITHER_SKULL_ID),
        ] {
            store.apply_add_entity(projectile(
                id,
                entity_type_id,
                ProtocolVec3d {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
            ));
        }
        let (dangerous_data_id, _) =
            vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
            )
            .unwrap();
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 43,
            values: vec![EntityDataValue {
                data_id: dangerous_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            }],
        }));
        let generation_a = store.publication_snapshot();

        store.advance_entity_client_animations(1);

        let fireball_delta = 1.1 * f64::from(0.95_f32);
        let wither_delta = 1.1 * f64::from(0.73_f32);
        assert_vec3_near(
            store.probe_entity_transform(40).unwrap().delta_movement,
            EntityVec3 {
                x: fireball_delta,
                y: 0.0,
                z: 0.0,
            },
        );
        assert_vec3_near(
            store.probe_entity_transform(41).unwrap().delta_movement,
            EntityVec3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        );
        assert_vec3_near(
            store.probe_entity_transform(43).unwrap().delta_movement,
            EntityVec3 {
                x: wither_delta,
                y: 0.0,
                z: 0.0,
            },
        );
        assert_eq!(
            store
                .drain_entity_client_movement_particle_states()
                .collect::<Vec<_>>(),
            vec![
                simple_particle(
                    40,
                    EntityClientMovementSimpleParticleKind::Smoke,
                    fireball_delta
                ),
                simple_particle(
                    42,
                    EntityClientMovementSimpleParticleKind::DragonBreath,
                    fireball_delta,
                ),
                simple_particle(
                    43,
                    EntityClientMovementSimpleParticleKind::Smoke,
                    wither_delta
                ),
            ]
        );
        assert_vec3_near(
            generation_a.probe_entity_transform(43).unwrap().position,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
        );
    }

    #[test]
    fn throwable_water_path_emits_four_pre_inertia_bubbles() {
        let metadata = EntityMetadata::default();
        let mut transform = test_transform(EntityVec3 {
            x: 0.4,
            y: -0.2,
            z: 0.6,
        });
        let mut particles = Vec::new();

        advance_throwable_projectile(
            50,
            &metadata,
            &mut transform,
            VanillaEntityClientMovementPolicy::ThrowableProjectile,
            true,
            &mut particles,
        );

        let gravity_movement = EntityVec3 {
            x: 0.4,
            y: -0.23,
            z: 0.6,
        };
        let bubble_position = EntityVec3 {
            x: 1.0 - gravity_movement.x * 0.25,
            y: 64.0 - gravity_movement.y * 0.25,
            z: -2.0 - gravity_movement.z * 0.25,
        };
        assert_eq!(particles.len(), 4);
        assert!(particles.iter().all(|state| *state
            == EntityClientMovementParticleState::Simple(
                EntityClientMovementSimpleParticleState {
                    entity_id: 50,
                    kind: EntityClientMovementSimpleParticleKind::Bubble,
                    position: bubble_position,
                    velocity: gravity_movement,
                }
            )));
        assert_vec3_near(
            transform.delta_movement,
            scale(gravity_movement, f64::from(0.8_f32)),
        );
    }

    fn simple_particle(
        entity_id: i32,
        kind: EntityClientMovementSimpleParticleKind,
        moved_x: f64,
    ) -> EntityClientMovementParticleState {
        EntityClientMovementParticleState::Simple(EntityClientMovementSimpleParticleState {
            entity_id,
            kind,
            position: EntityVec3 {
                x: 1.0 + moved_x,
                y: 64.5,
                z: -2.0,
            },
            velocity: EntityVec3::default(),
        })
    }

    fn assert_vec3_near(actual: EntityVec3, expected: EntityVec3) {
        for (actual, expected) in [
            (actual.x, expected.x),
            (actual.y, expected.y),
            (actual.z, expected.z),
        ] {
            assert!(
                (actual - expected).abs() <= 1.0e-12,
                "{actual} != {expected}"
            );
        }
    }

    fn test_transform(delta_movement: EntityVec3) -> EntityTransform {
        let position = EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        };
        EntityTransform {
            position,
            old_position: position,
            position_base: position,
            delta_movement,
            y_rot: 0.0,
            old_y_rot: 0.0,
            x_rot: 0.0,
            old_x_rot: 0.0,
            y_head_rot: 0.0,
            old_y_head_rot: 0.0,
            on_ground: None,
            interpolation: None,
        }
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
