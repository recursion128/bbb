use std::collections::HashMap;

use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TRIDENT_ID;
use bbb_protocol::item_types::VANILLA_ITEM_FIREWORK_ROCKET_ID;
use bbb_protocol::packets::EntityDataValueKind;
use bbb_protocol::packets::EquipmentSlot as ProtocolEquipmentSlot;
use hecs::Entity;

use super::components::{
    EntityEquipment, EntityHurtingProjectile, EntityIdentity, EntityMetadata, EntityTransform,
};
use super::schema::{
    vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type,
    vanilla_entity_metadata_entity_accessors,
    vanilla_entity_metadata_firework_rocket_accessors_for_entity_type,
    vanilla_entity_metadata_primed_tnt_accessors_for_entity_type,
    vanilla_entity_metadata_type_client_movement_policy, vanilla_entity_metadata_type_is_living,
    vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type,
    VanillaEntityClientMovementPolicy,
};
use super::{
    EntityClientMovementParticleState, EntityClientMovementSimpleParticleKind,
    EntityClientMovementSimpleParticleState, EntityPickBoundsState, EntityStore, EntityVec3,
    EyeOfEnderTrailParticleState, FireworkRocketTrailParticleState,
    ShulkerBulletTrailParticleState, VANILLA_ENTITY_TYPE_PLAYER_ID,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityClientMovementCollisionInput {
    pub(crate) entity_id: i32,
    pub(crate) position: EntityVec3,
    pub(crate) requested_movement: EntityVec3,
    pub(crate) bounds: EntityPickBoundsState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityClientMovementCollisionResult {
    pub(crate) requested_movement: EntityVec3,
    pub(crate) clipped_movement: EntityVec3,
}

pub(super) fn client_movement_uses_in_water(entity_type_id: i32) -> bool {
    matches!(
        vanilla_entity_metadata_type_client_movement_policy(entity_type_id),
        Some(
            VanillaEntityClientMovementPolicy::EyeOfEnder
                | VanillaEntityClientMovementPolicy::AbstractArrow
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
    pub(crate) fn client_movement_collision_inputs(
        &self,
    ) -> Vec<EntityClientMovementCollisionInput> {
        let mut inputs = Vec::new();
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                continue;
            };
            let entity_type_id = identity.entity_type_id;
            let Some(policy) = vanilla_entity_metadata_type_client_movement_policy(entity_type_id)
            else {
                continue;
            };
            if !matches!(
                policy,
                VanillaEntityClientMovementPolicy::FallingBlock
                    | VanillaEntityClientMovementPolicy::PrimedTnt
            ) {
                continue;
            }
            drop(identity);
            let Ok(mut query) = self
                .ecs
                .query_one::<(&EntityMetadata, &EntityTransform)>(entity)
            else {
                continue;
            };
            let Some((metadata, transform)) = query.get() else {
                continue;
            };
            let position = transform.position;
            let requested_movement =
                nonliving_gravity_movement(metadata, transform.delta_movement, 0.04);
            drop(query);
            let Some(bounds) = self.pick_bounds(*id) else {
                continue;
            };
            debug_assert!(matches!(
                vanilla_entity_metadata_type_client_movement_policy(entity_type_id),
                Some(
                    VanillaEntityClientMovementPolicy::FallingBlock
                        | VanillaEntityClientMovementPolicy::PrimedTnt
                )
            ));
            inputs.push(EntityClientMovementCollisionInput {
                entity_id: *id,
                position,
                requested_movement,
                bounds,
            });
        }
        inputs
    }

    /// Advances client-owned projectile and nonliving movement in the frozen
    /// entity schedule. Server-authoritative hit consequences and removals are
    /// never manufactured here; incoming packets remain their canonical owner.
    /// Vehicle, item, orb, and fishing-hook movement stay with their dedicated
    /// collision/authority owners.
    pub(super) fn advance_direct_client_movement(
        &mut self,
        scheduled: &[Entity],
        in_water_by_id: &HashMap<i32, bool>,
        collisions_by_id: &HashMap<i32, EntityClientMovementCollisionResult>,
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
                | VanillaEntityClientMovementPolicy::ShulkerBullet => {
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
                        _ => unreachable!("narrow direct movement policy match"),
                    }
                }
                VanillaEntityClientMovementPolicy::LlamaSpit => {
                    let Ok((metadata, transform)) = self
                        .ecs
                        .query_one_mut::<(&EntityMetadata, &mut EntityTransform)>(entity)
                    else {
                        continue;
                    };
                    advance_llama_spit(metadata, transform);
                }
                VanillaEntityClientMovementPolicy::AbstractArrow => {
                    let Some(accessors) =
                        vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
                            entity_type_id,
                        )
                    else {
                        continue;
                    };
                    let Ok((metadata, transform)) = self
                        .ecs
                        .query_one_mut::<(&EntityMetadata, &mut EntityTransform)>(entity)
                    else {
                        continue;
                    };
                    advance_abstract_arrow(
                        entity_id,
                        entity_type_id,
                        metadata,
                        transform,
                        accessors,
                        in_water_by_id.get(&entity_id).copied().unwrap_or(false),
                        &mut particles,
                    );
                }
                VanillaEntityClientMovementPolicy::FallingBlock => {
                    let Ok((metadata, transform)) = self
                        .ecs
                        .query_one_mut::<(&EntityMetadata, &mut EntityTransform)>(entity)
                    else {
                        continue;
                    };
                    advance_falling_block(
                        metadata,
                        transform,
                        collisions_by_id.get(&entity_id).copied(),
                    );
                }
                VanillaEntityClientMovementPolicy::PrimedTnt => {
                    let Some(accessors) =
                        vanilla_entity_metadata_primed_tnt_accessors_for_entity_type(
                            entity_type_id,
                        )
                    else {
                        continue;
                    };
                    let Ok((metadata, transform)) = self
                        .ecs
                        .query_one_mut::<(&mut EntityMetadata, &mut EntityTransform)>(entity)
                    else {
                        continue;
                    };
                    advance_primed_tnt(
                        metadata,
                        transform,
                        accessors.fuse_data_id,
                        accessors.fuse_default,
                        collisions_by_id.get(&entity_id).copied(),
                    );
                }
                VanillaEntityClientMovementPolicy::FireworkRocket => {
                    self.advance_firework_rocket(entity, entity_id, entity_type_id, &mut particles);
                }
                _ => {}
            }
        }
        self.pending_entity_client_movement_particles
            .extend(particles);
    }

    fn advance_firework_rocket(
        &mut self,
        entity: Entity,
        entity_id: i32,
        entity_type_id: i32,
        particles: &mut Vec<EntityClientMovementParticleState>,
    ) {
        let Some(accessors) =
            vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(entity_type_id)
        else {
            return;
        };
        let (attached_to_target, shot_at_angle) = {
            let Ok(metadata) = self.ecs.get::<&EntityMetadata>(entity) else {
                return;
            };
            (
                entity_data_optional_unsigned_int(
                    &metadata.data_values,
                    accessors.attached_to_target_data_id,
                    accessors.attached_to_target_default,
                ),
                entity_data_bool(
                    &metadata.data_values,
                    accessors.shot_at_angle_data_id,
                    accessors.shot_at_angle_default,
                ),
            )
        };

        let attachment =
            attached_to_target.and_then(|target_id| self.firework_attachment_target(target_id));
        let Ok(mut transform) = self.ecs.get::<&mut EntityTransform>(entity) else {
            return;
        };
        if let Some((position, delta_movement)) = attachment {
            transform.position = position;
            transform.delta_movement = delta_movement;
        } else if attached_to_target.is_none() {
            if !shot_at_angle {
                // The exact `horizontalCollision ? 1.0 : 1.15` branch remains
                // false in this free-flight owner; collision facts belong to
                // the dedicated movement/collision stage.
                transform.delta_movement.x *= 1.15;
                transform.delta_movement.y += 0.04;
                transform.delta_movement.z *= 1.15;
            }
            advance_position(&mut transform);
        }
        update_projectile_rotation(&mut transform);
        particles.push(EntityClientMovementParticleState::FireworkRocket(
            FireworkRocketTrailParticleState {
                entity_id,
                position: transform.position,
                delta_movement: transform.delta_movement,
            },
        ));
    }

    fn firework_attachment_target(&mut self, target_id: i32) -> Option<(EntityVec3, EntityVec3)> {
        let target = self.by_protocol_id.get(&target_id).copied()?;
        let (target_type_id, is_fall_flying) = {
            let identity = self.ecs.get::<&EntityIdentity>(target).ok()?;
            if !vanilla_entity_metadata_type_is_living(identity.entity_type_id) {
                return None;
            }
            let metadata = self.ecs.get::<&EntityMetadata>(target).ok()?;
            let entity_accessors = vanilla_entity_metadata_entity_accessors()?;
            let shared_flags = metadata
                .data_values
                .iter()
                .find(|value| value.data_id == entity_accessors.shared_flags_data_id)
                .and_then(|value| match value.value {
                    EntityDataValueKind::Byte(value) => Some(value),
                    _ => None,
                })
                .unwrap_or(entity_accessors.shared_flags_default);
            (
                identity.entity_type_id,
                shared_flags as u8 & entity_accessors.fall_flying_mask as u8 != 0,
            )
        };
        let item_arm_right = if target_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
            let (main_firework, off_firework) = self
                .ecs
                .get::<&EntityEquipment>(target)
                .ok()
                .map(|equipment| {
                    let item_in = |slot| {
                        equipment
                            .equipment
                            .iter()
                            .find(|update| update.slot == slot)
                            .and_then(|update| update.item.item_id)
                            == Some(VANILLA_ITEM_FIREWORK_ROCKET_ID)
                    };
                    (
                        item_in(ProtocolEquipmentSlot::MainHand),
                        item_in(ProtocolEquipmentSlot::OffHand),
                    )
                })
                .unwrap_or_default();
            let main_arm_left = self.main_arm_left(target_id).unwrap_or(false);
            if off_firework && !main_firework {
                main_arm_left
            } else {
                !main_arm_left
            }
        } else {
            false
        };

        let mut target_transform = self.ecs.get::<&mut EntityTransform>(target).ok()?;
        let hand_angle = if is_fall_flying {
            let look = calculate_view_vector(target_transform.x_rot, target_transform.y_rot);
            let movement = target_transform.delta_movement;
            target_transform.delta_movement = EntityVec3 {
                x: movement.x + look.x * 0.1 + (look.x * 1.5 - movement.x) * 0.5,
                y: movement.y + look.y * 0.1 + (look.y * 1.5 - movement.y) * 0.5,
                z: movement.z + look.z * 0.1 + (look.z * 1.5 - movement.z) * 0.5,
            };
            if target_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
                scale(
                    calculate_view_vector(
                        0.0,
                        target_transform.y_rot + if item_arm_right { 80.0 } else { -80.0 },
                    ),
                    0.5,
                )
            } else {
                EntityVec3::default()
            }
        } else {
            EntityVec3::default()
        };
        Some((
            add(target_transform.position, hand_angle),
            target_transform.delta_movement,
        ))
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

fn advance_llama_spit(metadata: &EntityMetadata, transform: &mut EntityTransform) {
    let movement = transform.delta_movement;
    advance_position(transform);
    update_projectile_rotation(transform);
    transform.delta_movement = scale(movement, f64::from(0.99_f32));
    let no_gravity = vanilla_entity_metadata_entity_accessors().is_some_and(|accessors| {
        entity_data_bool(
            &metadata.data_values,
            accessors.no_gravity_data_id,
            accessors.no_gravity_default,
        )
    });
    if !no_gravity {
        transform.delta_movement.y -= 0.06;
    }
}

fn advance_abstract_arrow(
    entity_id: i32,
    entity_type_id: i32,
    metadata: &EntityMetadata,
    transform: &mut EntityTransform,
    accessors: super::schema::VanillaAbstractArrowMetadataAccessors,
    in_water: bool,
    particles: &mut Vec<EntityClientMovementParticleState>,
) {
    let flags = entity_data_byte(
        &metadata.data_values,
        accessors.flags_data_id,
        accessors.flags_default,
    );
    let physics_enabled = flags & accessors.no_physics_mask == 0;
    let in_ground = entity_data_bool(
        &metadata.data_values,
        accessors.in_ground_data_id,
        accessors.in_ground_default,
    );
    if in_ground && physics_enabled {
        return;
    }

    let original_position = transform.position;
    let movement = transform.delta_movement;
    if in_water {
        let water_inertia = if entity_type_id == VANILLA_ENTITY_TYPE_TRIDENT_ID {
            0.99_f32
        } else {
            0.6_f32
        };
        transform.delta_movement = scale(movement, f64::from(water_inertia));
        append_bubbles(
            entity_id,
            original_position,
            transform.delta_movement,
            particles,
        );
    }
    if flags & accessors.crit_mask != 0 {
        append_arrow_crit_particles(entity_id, original_position, movement, particles);
    }

    update_arrow_rotation(transform, movement, physics_enabled);
    transform.position = add(original_position, movement);
    if !in_water {
        transform.delta_movement = scale(movement, f64::from(0.99_f32));
    }
    let no_gravity = vanilla_entity_metadata_entity_accessors().is_some_and(|accessors| {
        entity_data_bool(
            &metadata.data_values,
            accessors.no_gravity_data_id,
            accessors.no_gravity_default,
        )
    });
    if physics_enabled && !no_gravity {
        transform.delta_movement.y -= 0.05;
    }
}

fn advance_falling_block(
    metadata: &EntityMetadata,
    transform: &mut EntityTransform,
    collision: Option<EntityClientMovementCollisionResult>,
) {
    let requested = nonliving_gravity_movement(metadata, transform.delta_movement, 0.04);
    let clipped = collision
        .filter(|collision| collision.requested_movement == requested)
        .map_or(requested, |collision| collision.clipped_movement);
    transform.position = add(transform.position, clipped);
    transform.on_ground = Some(requested.y < 0.0 && clipped.y != requested.y);
    transform.delta_movement = scale(requested, f64::from(0.98_f32));
}

fn advance_primed_tnt(
    metadata: &mut EntityMetadata,
    transform: &mut EntityTransform,
    fuse_data_id: u8,
    fuse_default: i32,
    collision: Option<EntityClientMovementCollisionResult>,
) {
    let requested = nonliving_gravity_movement(metadata, transform.delta_movement, 0.04);
    let clipped = collision
        .filter(|collision| collision.requested_movement == requested)
        .map_or(requested, |collision| collision.clipped_movement);
    transform.position = add(transform.position, clipped);
    let on_ground = requested.y < 0.0 && clipped.y != requested.y;
    transform.on_ground = Some(on_ground);
    transform.delta_movement = scale(requested, f64::from(0.98_f32));
    if on_ground {
        transform.delta_movement.x *= 0.7;
        transform.delta_movement.y *= -0.5;
        transform.delta_movement.z *= 0.7;
    }

    let fuse = entity_data_int(&metadata.data_values, fuse_data_id, fuse_default);
    set_entity_data_int(metadata, fuse_data_id, fuse.wrapping_sub(1));
}

fn nonliving_gravity_movement(
    metadata: &EntityMetadata,
    mut movement: EntityVec3,
    gravity: f64,
) -> EntityVec3 {
    let no_gravity = vanilla_entity_metadata_entity_accessors().is_some_and(|accessors| {
        entity_data_bool(
            &metadata.data_values,
            accessors.no_gravity_data_id,
            accessors.no_gravity_default,
        )
    });
    if !no_gravity {
        movement.y -= gravity;
    }
    movement
}

fn append_arrow_crit_particles(
    entity_id: i32,
    position: EntityVec3,
    movement: EntityVec3,
    particles: &mut Vec<EntityClientMovementParticleState>,
) {
    particles.extend((0..4).map(|index| {
        let fraction = f64::from(index) / 4.0;
        EntityClientMovementParticleState::Simple(EntityClientMovementSimpleParticleState {
            entity_id,
            kind: EntityClientMovementSimpleParticleKind::Crit,
            position: EntityVec3 {
                x: position.x + movement.x * fraction,
                y: position.y + movement.y * fraction,
                z: position.z + movement.z * fraction,
            },
            velocity: EntityVec3 {
                x: -movement.x,
                y: -movement.y + 0.2,
                z: -movement.z,
            },
        })
    }));
}

fn update_arrow_rotation(
    transform: &mut EntityTransform,
    movement: EntityVec3,
    physics_enabled: bool,
) {
    let horizontal = (movement.x * movement.x + movement.z * movement.z).sqrt();
    let target_y_rot = if physics_enabled {
        vanilla_degrees(movement.x.atan2(movement.z))
    } else {
        vanilla_degrees((-movement.x).atan2(-movement.z))
    };
    let target_x_rot = vanilla_degrees(movement.y.atan2(horizontal));
    transform.old_x_rot = unwrap_old_rotation(transform.old_x_rot, target_x_rot);
    transform.old_y_rot = unwrap_old_rotation(transform.old_y_rot, target_y_rot);
    transform.x_rot = lerp(0.2, transform.old_x_rot, target_x_rot);
    transform.y_rot = lerp(0.2, transform.old_y_rot, target_y_rot);
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

fn entity_data_byte(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: i8,
) -> i8 {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match value.value {
            EntityDataValueKind::Byte(value) => Some(value),
            _ => None,
        })
        .unwrap_or(default)
}

fn entity_data_int(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: i32,
) -> i32 {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match value.value {
            EntityDataValueKind::Int(value) => Some(value),
            _ => None,
        })
        .unwrap_or(default)
}

fn set_entity_data_int(metadata: &mut EntityMetadata, data_id: u8, value: i32) {
    let value = bbb_protocol::packets::EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(value),
    };
    if let Some(existing) = metadata
        .data_values
        .iter_mut()
        .find(|existing| existing.data_id == data_id)
    {
        *existing = value;
    } else {
        metadata.data_values.push(value);
        metadata.data_values.sort_by_key(|value| value.data_id);
    }
}

fn entity_data_optional_unsigned_int(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: Option<i32>,
) -> Option<i32> {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match value.value {
            EntityDataValueKind::OptionalUnsignedInt(value) => Some(value),
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

fn calculate_view_vector(x_rot: f32, y_rot: f32) -> EntityVec3 {
    let radians_per_degree = (std::f64::consts::PI / 180.0) as f32;
    let real_x_rot = x_rot * radians_per_degree;
    let real_y_rot = -y_rot * radians_per_degree;
    let y_cos = vanilla_mth_cos(f64::from(real_y_rot));
    let y_sin = vanilla_mth_sin(f64::from(real_y_rot));
    let x_cos = vanilla_mth_cos(f64::from(real_x_rot));
    let x_sin = vanilla_mth_sin(f64::from(real_x_rot));
    EntityVec3 {
        x: f64::from(y_sin * x_cos),
        y: f64::from(-x_sin),
        z: f64::from(y_cos * x_cos),
    }
}

fn vanilla_mth_sin(angle: f64) -> f32 {
    vanilla_mth_sin_table_value((angle * 10_430.378_350_470_453) as i64)
}

fn vanilla_mth_cos(angle: f64) -> f32 {
    vanilla_mth_sin_table_value((angle * 10_430.378_350_470_453 + 16_384.0) as i64)
}

fn vanilla_mth_sin_table_value(raw_index: i64) -> f32 {
    let index = raw_index & 65_535;
    (index as f64 / 10_430.378_350_470_453).sin() as f32
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
        VANILLA_ENTITY_TYPE_ARROW_ID, VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID,
        VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID, VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID,
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID, VANILLA_ENTITY_TYPE_FIREBALL_ID,
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID, VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID,
        VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID, VANILLA_ENTITY_TYPE_SNOWBALL_ID,
        VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID, VANILLA_ENTITY_TYPE_SPLASH_POTION_ID,
        VANILLA_ENTITY_TYPE_TNT_ID, VANILLA_ENTITY_TYPE_TRIDENT_ID,
        VANILLA_ENTITY_TYPE_WIND_CHARGE_ID, VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    };
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, EntityDataValue, SetEntityData as ProtocolSetEntityData,
        Vec3d as ProtocolVec3d,
    };
    use uuid::Uuid;

    use crate::{
        BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain,
        PaletteKind, PalettedContainerData, WorldDimension, WorldStore,
    };

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

    #[test]
    fn abstract_arrows_honor_synced_ground_physics_and_crit_flags() {
        let mut store = WorldStore::new();
        for (id, entity_type_id) in [
            (54, VANILLA_ENTITY_TYPE_ARROW_ID),
            (55, VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID),
            (56, VANILLA_ENTITY_TYPE_TRIDENT_ID),
        ] {
            store.apply_add_entity(projectile(
                id,
                entity_type_id,
                ProtocolVec3d {
                    x: 1.0,
                    y: 0.5,
                    z: 0.0,
                },
            ));
        }
        let accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_ARROW_ID,
        )
        .unwrap();
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 54,
            values: vec![EntityDataValue {
                data_id: accessors.flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(accessors.crit_mask),
            }],
        }));
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 55,
            values: vec![EntityDataValue {
                data_id: accessors.in_ground_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            }],
        }));
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 56,
            values: vec![EntityDataValue {
                data_id: accessors.flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(accessors.no_physics_mask),
            }],
        }));
        let generation_a = store.publication_snapshot();

        store.advance_entity_client_animations(1);

        let inertia = f64::from(0.99_f32);
        let flying = store.probe_entity_transform(54).unwrap();
        assert_vec3_near(
            flying.position,
            EntityVec3 {
                x: 2.0,
                y: 64.5,
                z: -2.0,
            },
        );
        assert_vec3_near(
            flying.delta_movement,
            EntityVec3 {
                x: inertia,
                y: 0.5 * inertia - 0.05,
                z: 0.0,
            },
        );
        assert_eq!(flying.y_rot, 18.0);
        assert_eq!(
            store.probe_entity_transform(55).unwrap().position,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            }
        );
        let no_physics = store.probe_entity_transform(56).unwrap();
        assert_vec3_near(
            no_physics.delta_movement,
            EntityVec3 {
                x: inertia,
                y: 0.5 * inertia,
                z: 0.0,
            },
        );
        assert_eq!(no_physics.y_rot, -18.0);

        let particles = store
            .drain_entity_client_movement_particle_states()
            .collect::<Vec<_>>();
        assert_eq!(particles.len(), 4);
        for (index, particle) in particles.into_iter().enumerate() {
            assert_eq!(
                particle,
                EntityClientMovementParticleState::Simple(
                    EntityClientMovementSimpleParticleState {
                        entity_id: 54,
                        kind: EntityClientMovementSimpleParticleKind::Crit,
                        position: EntityVec3 {
                            x: 1.0 + index as f64 / 4.0,
                            y: 64.0 + 0.5 * index as f64 / 4.0,
                            z: -2.0,
                        },
                        velocity: EntityVec3 {
                            x: -1.0,
                            y: -0.3,
                            z: 0.0,
                        },
                    }
                )
            );
        }
        assert_eq!(
            generation_a.probe_entity_transform(54).unwrap().position,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            "retained publication remains isolated from arrow movement"
        );
    }

    #[test]
    fn abstract_arrow_water_inertia_precedes_bubbles_and_free_flight() {
        let metadata = EntityMetadata::default();
        for (entity_id, entity_type_id, inertia) in [
            (57, VANILLA_ENTITY_TYPE_ARROW_ID, 0.6_f32),
            (58, VANILLA_ENTITY_TYPE_TRIDENT_ID, 0.99_f32),
        ] {
            let mut transform = test_transform(EntityVec3 {
                x: 0.5,
                y: -0.25,
                z: 1.0,
            });
            let mut particles = Vec::new();
            let accessors =
                vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(entity_type_id)
                    .unwrap();

            advance_abstract_arrow(
                entity_id,
                entity_type_id,
                &metadata,
                &mut transform,
                accessors,
                true,
                &mut particles,
            );

            let water_movement = scale(
                EntityVec3 {
                    x: 0.5,
                    y: -0.25,
                    z: 1.0,
                },
                f64::from(inertia),
            );
            assert_vec3_near(
                transform.position,
                EntityVec3 {
                    x: 1.5,
                    y: 63.75,
                    z: -1.0,
                },
            );
            assert_vec3_near(
                transform.delta_movement,
                EntityVec3 {
                    y: water_movement.y - 0.05,
                    ..water_movement
                },
            );
            assert_eq!(particles.len(), 4);
            assert!(particles.iter().all(|particle| {
                *particle
                    == EntityClientMovementParticleState::Simple(
                        EntityClientMovementSimpleParticleState {
                            entity_id,
                            kind: EntityClientMovementSimpleParticleKind::Bubble,
                            position: EntityVec3 {
                                x: 1.0 - water_movement.x * 0.25,
                                y: 64.0 - water_movement.y * 0.25,
                                z: -2.0 - water_movement.z * 0.25,
                            },
                            velocity: water_movement,
                        },
                    )
            }));
        }
    }

    #[test]
    fn falling_block_and_tnt_apply_gravity_drag_and_client_fuse_tick() {
        let mut store = WorldStore::new();
        for (id, entity_type_id) in [
            (59, VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID),
            (60, VANILLA_ENTITY_TYPE_TNT_ID),
        ] {
            store.apply_add_entity(projectile(
                id,
                entity_type_id,
                ProtocolVec3d {
                    x: 0.5,
                    y: 0.2,
                    z: -0.25,
                },
            ));
        }
        let generation_a = store.publication_snapshot();

        store.advance_entity_client_animations(1);

        let requested = EntityVec3 {
            x: 0.5,
            y: 0.16,
            z: -0.25,
        };
        for id in [59, 60] {
            let transform = store.probe_entity_transform(id).unwrap();
            assert_vec3_near(
                transform.position,
                EntityVec3 {
                    x: 1.5,
                    y: 64.16,
                    z: -2.25,
                },
            );
            assert_vec3_near(
                transform.delta_movement,
                scale(requested, f64::from(0.98_f32)),
            );
            assert_eq!(transform.on_ground, Some(false));
        }
        assert_eq!(
            store.primed_tnt_fuse_remaining_in_ticks(60, 1.0),
            Some(79.0)
        );
        assert_eq!(
            store.primed_tnt_smoke_particle_states(),
            vec![super::super::PrimedTntSmokeParticleState {
                entity_id: 60,
                position: EntityVec3 {
                    x: 1.5,
                    y: 64.16,
                    z: -2.25,
                },
            }]
        );
        assert_eq!(
            generation_a.primed_tnt_fuse_remaining_in_ticks(60, 1.0),
            Some(80.0),
            "retained publication keeps the pre-tick fuse"
        );
    }

    #[test]
    fn nonliving_collision_response_matches_falling_and_tnt_ground_branches() {
        let metadata = EntityMetadata::default();
        let requested = EntityVec3 {
            x: 0.5,
            y: -0.04,
            z: -0.25,
        };
        let collision = Some(EntityClientMovementCollisionResult {
            requested_movement: requested,
            clipped_movement: EntityVec3 {
                x: 0.5,
                y: 0.0,
                z: -0.25,
            },
        });

        let mut falling = test_transform(EntityVec3 {
            x: 0.5,
            y: 0.0,
            z: -0.25,
        });
        advance_falling_block(&metadata, &mut falling, collision);
        assert_eq!(falling.on_ground, Some(true));
        assert_vec3_near(
            falling.position,
            EntityVec3 {
                x: 1.5,
                y: 64.0,
                z: -2.25,
            },
        );
        assert_vec3_near(
            falling.delta_movement,
            scale(requested, f64::from(0.98_f32)),
        );

        let mut tnt_metadata = EntityMetadata::default();
        let mut tnt = test_transform(EntityVec3 {
            x: 0.5,
            y: 0.0,
            z: -0.25,
        });
        let accessors = vanilla_entity_metadata_primed_tnt_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_TNT_ID,
        )
        .unwrap();
        advance_primed_tnt(
            &mut tnt_metadata,
            &mut tnt,
            accessors.fuse_data_id,
            accessors.fuse_default,
            collision,
        );
        let drag = f64::from(0.98_f32);
        assert_eq!(tnt.on_ground, Some(true));
        assert_vec3_near(
            tnt.delta_movement,
            EntityVec3 {
                x: requested.x * drag * 0.7,
                y: requested.y * drag * -0.5,
                z: requested.z * drag * 0.7,
            },
        );
        assert_eq!(
            entity_data_int(
                &tnt_metadata.data_values,
                accessors.fuse_data_id,
                accessors.fuse_default,
            ),
            79
        );
    }

    #[test]
    fn nonliving_world_collision_inputs_clip_the_canonical_tick() {
        let mut store = nonliving_collision_world();
        for (id, entity_type_id) in [
            (61, VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID),
            (62, VANILLA_ENTITY_TYPE_TNT_ID),
        ] {
            let mut entity = projectile(
                id,
                entity_type_id,
                ProtocolVec3d {
                    x: 0.0,
                    y: -0.1,
                    z: 0.0,
                },
            );
            entity.position = ProtocolVec3d {
                x: 0.5,
                y: 1.05,
                z: 0.5,
            };
            store.apply_add_entity(entity);
        }

        store.advance_entity_client_animations(1);

        let requested_y = -0.14;
        let falling = store.probe_entity_transform(61).unwrap();
        assert_eq!(falling.on_ground, Some(true));
        assert!((falling.position.y - 1.0).abs() <= 1.0e-9);
        assert!((falling.delta_movement.y - requested_y * f64::from(0.98_f32)).abs() <= 1.0e-12);
        let tnt = store.probe_entity_transform(62).unwrap();
        assert_eq!(tnt.on_ground, Some(true));
        assert!((tnt.position.y - 1.0).abs() <= 1.0e-9);
        assert!((tnt.delta_movement.y - requested_y * f64::from(0.98_f32) * -0.5).abs() <= 1.0e-12);
    }

    #[test]
    fn llama_spit_and_unattached_firework_apply_subclass_free_flight() {
        let mut store = WorldStore::new();
        store.apply_add_entity(projectile(
            60,
            VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID,
            ProtocolVec3d {
                x: 0.4,
                y: 0.2,
                z: -0.6,
            },
        ));
        store.apply_add_entity(projectile(
            61,
            VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
            ProtocolVec3d {
                x: 0.2,
                y: 0.8,
                z: -0.4,
            },
        ));
        store.apply_add_entity(projectile(
            62,
            VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
            ProtocolVec3d {
                x: 0.2,
                y: 0.8,
                z: -0.4,
            },
        ));
        let firework_accessors = vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
        )
        .unwrap();
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 62,
            values: vec![EntityDataValue {
                data_id: firework_accessors.shot_at_angle_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            }],
        }));

        store.advance_entity_client_animations(1);

        let llama_inertia = f64::from(0.99_f32);
        assert_vec3_near(
            store.probe_entity_transform(60).unwrap().position,
            EntityVec3 {
                x: 1.4,
                y: 64.2,
                z: -2.6,
            },
        );
        assert_vec3_near(
            store.probe_entity_transform(60).unwrap().delta_movement,
            EntityVec3 {
                x: 0.4 * llama_inertia,
                y: 0.2 * llama_inertia - 0.06,
                z: -0.6 * llama_inertia,
            },
        );
        assert_vec3_near(
            store.probe_entity_transform(61).unwrap().position,
            EntityVec3 {
                x: 1.23,
                y: 64.84,
                z: -2.46,
            },
        );
        assert_vec3_near(
            store.probe_entity_transform(62).unwrap().position,
            EntityVec3 {
                x: 1.2,
                y: 64.8,
                z: -2.4,
            },
        );
        assert_eq!(
            store
                .drain_entity_client_movement_particle_states()
                .map(|state| match state {
                    EntityClientMovementParticleState::FireworkRocket(state) => state.entity_id,
                    _ => -1,
                })
                .collect::<Vec<_>>(),
            vec![61, 62]
        );
    }

    #[test]
    fn attached_firework_uses_stable_target_lookup_and_boosts_fall_flying_target() {
        let mut store = WorldStore::new();
        store.apply_add_entity(projectile(
            70,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            ProtocolVec3d {
                x: 0.2,
                y: 0.1,
                z: -0.3,
            },
        ));
        store.apply_add_entity(projectile(
            71,
            VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
            ProtocolVec3d::default(),
        ));
        let entity_accessors = vanilla_entity_metadata_entity_accessors().unwrap();
        let firework_accessors = vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
        )
        .unwrap();
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 70,
            values: vec![EntityDataValue {
                data_id: entity_accessors.shared_flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(entity_accessors.fall_flying_mask),
            }],
        }));
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 71,
            values: vec![EntityDataValue {
                data_id: firework_accessors.attached_to_target_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_UNSIGNED_INT_ID,
                value: EntityDataValueKind::OptionalUnsignedInt(Some(70)),
            }],
        }));
        let generation_a = store.publication_snapshot();

        store.advance_entity_client_animations(1);

        let expected_movement = EntityVec3 {
            x: 0.1,
            y: 0.05,
            z: 0.7,
        };
        assert_vec3_near(
            store.probe_entity_transform(70).unwrap().delta_movement,
            expected_movement,
        );
        let rocket = store.probe_entity_transform(71).unwrap();
        assert_vec3_near(
            rocket.position,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
        );
        assert_vec3_near(rocket.delta_movement, expected_movement);
        assert_vec3_near(
            generation_a
                .probe_entity_transform(70)
                .unwrap()
                .delta_movement,
            EntityVec3 {
                x: 0.2,
                y: 0.1,
                z: -0.3,
            },
        );
        assert_eq!(
            store
                .drain_entity_client_movement_particle_states()
                .map(|state| match state {
                    EntityClientMovementParticleState::FireworkRocket(state) => state.entity_id,
                    _ => -1,
                })
                .collect::<Vec<_>>(),
            vec![71]
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

    fn nonliving_collision_world() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: PalettedContainerData {
                    domain: PaletteDomain::BlockStates,
                    bits_per_entry: 0,
                    palette_kind: PaletteKind::SingleValue,
                    palette_global_ids: vec![0],
                    packed_data: Vec::new(),
                    entry_count: 4096,
                },
                biomes: PalettedContainerData {
                    domain: PaletteDomain::Biomes,
                    bits_per_entry: 0,
                    palette_kind: PaletteKind::SingleValue,
                    palette_global_ids: vec![0],
                    packed_data: Vec::new(),
                    entry_count: 64,
                },
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        let grass = crate::registries::BlockStateRegistry::vanilla_26_1()
            .find_by_name_and_properties(
                "minecraft:grass_block",
                &[("snowy".to_owned(), "false".to_owned())]
                    .into_iter()
                    .collect(),
            )
            .expect("vanilla grass block state")
            .id;
        assert!(
            world.apply_block_update(bbb_protocol::packets::BlockUpdate {
                pos: bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 },
                block_state_id: grass,
            })
        );
        assert!(world.probe_block(BlockPos { x: 0, y: 0, z: 0 }).is_some());
        world
    }
}
