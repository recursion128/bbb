use super::super::{EntityClientAnimationState, ProjectileLifecycleState};
use super::*;

pub(super) fn advance_throwable_projectile(
    entity_id: i32,
    metadata: &EntityMetadata,
    transform: &mut EntityTransform,
    policy: VanillaEntityClientMovementPolicy,
    in_water: bool,
    effects: &mut Vec<EntityClientMovementEffectState>,
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
            effects,
        );
    }
    let inertia = f64::from(if in_water { 0.8_f32 } else { 0.99_f32 });
    transform.delta_movement = scale(movement_before_inertia, inertia);
    advance_position(transform);
    update_projectile_rotation(transform);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn advance_hurting_projectile(
    entity_id: i32,
    entity_type_id: i32,
    metadata: &EntityMetadata,
    hurting_projectile: Option<&crate::HurtingProjectileState>,
    transform: &mut EntityTransform,
    policy: VanillaEntityClientMovementPolicy,
    in_water: bool,
    effects: &mut Vec<EntityClientMovementEffectState>,
) {
    let movement_before_inertia = transform.delta_movement;
    if in_water {
        append_bubbles(
            entity_id,
            transform.position,
            movement_before_inertia,
            effects,
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
        effects.push(EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::Simple(EntityClientMovementSimpleParticleState {
                entity_id,
                kind,
                position: EntityVec3 {
                    y: transform.position.y + 0.5,
                    ..transform.position
                },
                velocity: EntityVec3::default(),
            }),
        ));
    }
}

pub(super) fn advance_llama_spit(metadata: &EntityMetadata, transform: &mut EntityTransform) {
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

pub(super) fn advance_returning_trident(
    metadata: &mut EntityMetadata,
    projectile: &mut ProjectileLifecycleState,
    transform: &mut EntityTransform,
    arrow_accessors: super::super::schema::VanillaAbstractArrowMetadataAccessors,
    target: EntityClientMovementTridentReturnTarget,
) {
    let loyalty = target.loyalty;
    let flags = entity_data_byte(
        &metadata.data_values,
        arrow_accessors.flags_data_id,
        arrow_accessors.flags_default,
    );
    if loyalty <= 0
        || (!projectile.trident_dealt_damage && flags & arrow_accessors.no_physics_mask == 0)
    {
        return;
    }

    // Vanilla `setNoPhysics(true)` writes the synced flag and the private
    // `Entity.noPhysics` field together; `isAffectedByBlocks` reads the field.
    set_entity_data_byte(
        metadata,
        arrow_accessors.flags_data_id,
        flags | arrow_accessors.no_physics_mask,
    );
    projectile.arrow_no_physics = true;
    let to_owner = subtract(target.eye_position, transform.position);
    transform.position.y += to_owner.y * 0.015 * f64::from(loyalty);
    transform.delta_movement = add(
        scale(transform.delta_movement, 0.95),
        scale(normalize(to_owner), 0.05 * f64::from(loyalty)),
    );
    projectile.trident_return_ticks = projectile.trident_return_ticks.saturating_add(1);
}

/// One arrow tick's movement outcome: the vanilla `applyEffectsFromBlocks`
/// slot with its `Entity.baseTick` reset placement, plus the deferred
/// inertia/gravity tail that runs after the effects callback.
pub(super) struct ArrowMoveStep {
    pub(super) first_effect: EntityBlockEffectMovement,
    pub(super) collision: Option<EntityAbstractArrowCollisionFacts>,
    pub(super) finish: Option<ArrowMoveFinish>,
    pub(super) trident_initial_containment_grounded: bool,
}

pub(super) struct ArrowMoveFinish {
    pub(super) in_water: bool,
    pub(super) physics_enabled: bool,
}

pub(super) struct ArrowCollisionStep {
    pub(super) second_effect: Option<EntityBlockEffectMovement>,
    pub(super) reaches_endpoint: bool,
    pub(super) discarded: bool,
    pub(super) redirected_targets: Vec<EntityProjectileAimRedirect>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct EntityProjectileAimRedirect {
    pub(super) entity_id: i32,
    pub(super) incarnation: EntityIncarnation,
    pub(super) movement: EntityVec3,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn advance_abstract_arrow(
    entity_id: i32,
    entity_type_id: i32,
    metadata: &mut EntityMetadata,
    animations: &mut EntityClientAnimationState,
    transform: &mut EntityTransform,
    accessors: super::super::schema::VanillaAbstractArrowMetadataAccessors,
    in_water: bool,
    collision: Option<&EntityAbstractArrowCollisionFacts>,
    effects: &mut Vec<EntityClientMovementEffectState>,
) -> ArrowMoveStep {
    let flags = entity_data_byte(
        &metadata.data_values,
        accessors.flags_data_id,
        accessors.flags_default,
    );
    let physics_enabled = flags & accessors.no_physics_mask == 0;
    let mut in_ground = entity_data_bool(
        &metadata.data_values,
        accessors.in_ground_data_id,
        accessors.in_ground_default,
    );
    let mut initial_containment_grounded = false;
    if physics_enabled
        && collision.is_some_and(|collision| collision.initial_block_state_id.is_some())
    {
        initial_containment_grounded = !in_ground;
        transform.delta_movement = EntityVec3::default();
        set_entity_data_bool(metadata, accessors.in_ground_data_id, true);
        in_ground = true;
        animations.start_local_arrow_shake();
    }
    // Vanilla decrements shake after the initial containment check and before
    // either the stuck branch or a later segment hit.
    animations.advance_arrow_shake_tick();
    if in_ground && physics_enabled {
        // The stuck branch still calls the no-arg `applyEffectsFromBlocks`
        // (a degenerate old-to-current segment) but never `super.tick()`.
        return ArrowMoveStep {
            first_effect: EntityBlockEffectMovement::direct(transform.position, transform.position),
            collision: None,
            finish: None,
            trident_initial_containment_grounded: initial_containment_grounded,
        };
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
            effects,
        );
    }
    if flags & accessors.crit_mask != 0 {
        append_arrow_crit_particles(entity_id, original_position, movement, effects);
    }

    update_arrow_rotation(transform, movement, physics_enabled);
    let collision = physics_enabled.then(|| collision.cloned()).flatten();
    if physics_enabled {
        if let Some(collision) = collision.as_ref() {
            transform.position = collision
                .entity_hits
                .first()
                .map_or(collision.endpoint, |hit| hit.location);
        } else {
            transform.position = add(original_position, movement);
        }
    } else {
        transform.position = add(original_position, movement);
    }
    ArrowMoveStep {
        first_effect: EntityBlockEffectMovement::direct(original_position, transform.position),
        collision,
        finish: Some(ArrowMoveFinish {
            in_water,
            physics_enabled,
        }),
        trident_initial_containment_grounded: false,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn advance_abstract_arrow_collision_after_first_effect(
    entity_type_id: i32,
    metadata: &mut EntityMetadata,
    projectile: &mut ProjectileLifecycleState,
    runtime: &mut EntityAbstractArrowRuntime,
    transform: &mut EntityTransform,
    accessors: super::super::schema::VanillaAbstractArrowMetadataAccessors,
    collision: &EntityAbstractArrowCollisionFacts,
    owner_look_angle: Option<EntityVec3>,
) -> ArrowCollisionStep {
    if collision.entity_hits.is_empty() {
        return ArrowCollisionStep {
            second_effect: None,
            reaches_endpoint: true,
            discarded: false,
            redirected_targets: Vec::new(),
        };
    }
    let mut redirected_targets = Vec::new();
    let outcome = apply_abstract_arrow_entity_hits(
        entity_type_id,
        metadata,
        projectile,
        runtime,
        transform,
        accessors,
        &collision.entity_hits,
        owner_look_angle,
        &mut redirected_targets,
    );
    if outcome.discarded || !outcome.continues_to_block {
        return ArrowCollisionStep {
            second_effect: None,
            reaches_endpoint: false,
            discarded: outcome.discarded,
            redirected_targets,
        };
    }
    let from = transform.position;
    transform.position = collision.endpoint;
    ArrowCollisionStep {
        second_effect: Some(EntityBlockEffectMovement::direct(from, collision.endpoint)),
        reaches_endpoint: true,
        discarded: false,
        redirected_targets,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_abstract_arrow_endpoint_hit(
    entity_type_id: i32,
    metadata: &mut EntityMetadata,
    projectile: &mut ProjectileLifecycleState,
    runtime: &mut EntityAbstractArrowRuntime,
    animations: &mut EntityClientAnimationState,
    transform: &mut EntityTransform,
    accessors: super::super::schema::VanillaAbstractArrowMetadataAccessors,
    collision: &EntityAbstractArrowCollisionFacts,
) {
    if let Some(block_hit) = collision.block_hit {
        apply_abstract_arrow_block_hit(
            entity_type_id,
            metadata,
            projectile,
            runtime,
            animations,
            transform,
            accessors,
            block_hit.block_state_id,
        );
    } else if collision.world_border_hit {
        reverse_projectile(transform, runtime, None);
        transform.delta_movement = scale(transform.delta_movement, 0.2);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ArrowEntityHitOutcome {
    pub(super) continues_to_block: bool,
    pub(super) discarded: bool,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_abstract_arrow_entity_hits(
    entity_type_id: i32,
    metadata: &mut EntityMetadata,
    projectile: &mut ProjectileLifecycleState,
    runtime: &mut EntityAbstractArrowRuntime,
    transform: &mut EntityTransform,
    accessors: super::super::schema::VanillaAbstractArrowMetadataAccessors,
    hits: &[EntityAbstractArrowEntityHit],
    owner_look_angle: Option<EntityVec3>,
    redirected_targets: &mut Vec<EntityProjectileAimRedirect>,
) -> ArrowEntityHitOutcome {
    let pierce_level = entity_data_byte(
        &metadata.data_values,
        accessors.pierce_level_data_id,
        accessors.pierce_level_default,
    )
    .max(0);
    for hit in hits {
        if hit.deflects_projectiles {
            let deflector = EntityInstanceReference {
                entity_id: hit.entity_id,
                incarnation: hit.incarnation,
            };
            if runtime.last_deflected_by != Some(deflector) {
                reverse_projectile(transform, runtime, Some(deflector));
            }
            return ArrowEntityHitOutcome {
                continues_to_block: false,
                discarded: false,
            };
        }
        if hit.redirectable_projectile {
            if let Some(movement) = owner_look_angle {
                redirected_targets.push(EntityProjectileAimRedirect {
                    entity_id: hit.entity_id,
                    incarnation: hit.incarnation,
                    movement,
                });
            }
        }

        if entity_type_id == VANILLA_ENTITY_TYPE_TRIDENT_ID {
            projectile.trident_dealt_damage = true;
            reverse_projectile(transform, runtime, None);
            transform.delta_movement.x *= 0.02;
            transform.delta_movement.y *= 0.2;
            transform.delta_movement.z *= 0.02;
        } else {
            if pierce_level > 0 {
                if runtime.piercing_ignore_entity_ids.len()
                    >= usize::from(pierce_level as u8).saturating_add(1)
                {
                    return ArrowEntityHitOutcome {
                        continues_to_block: false,
                        discarded: true,
                    };
                }
                if !runtime.piercing_ignore_entity_ids.contains(&hit.entity_id) {
                    runtime.piercing_ignore_entity_ids.push(hit.entity_id);
                }
            }
            if hit.hurt_client {
                if pierce_level <= 0 {
                    return ArrowEntityHitOutcome {
                        continues_to_block: false,
                        discarded: true,
                    };
                }
            } else {
                reverse_projectile(transform, runtime, None);
                transform.delta_movement = scale(transform.delta_movement, 0.2);
            }
        }
    }
    ArrowEntityHitOutcome {
        continues_to_block: pierce_level > 0,
        discarded: false,
    }
}

pub(super) fn reverse_projectile(
    transform: &mut EntityTransform,
    runtime: &mut EntityAbstractArrowRuntime,
    deflector: Option<EntityInstanceReference>,
) {
    let rotation = 170.0 + java_random_next_float(&mut runtime.random_seed) * 20.0;
    transform.delta_movement = scale(transform.delta_movement, -0.5);
    transform.y_rot += rotation;
    transform.old_y_rot += rotation;
    if let Some(deflector) = deflector {
        runtime.last_deflected_by = Some(deflector);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_abstract_arrow_block_hit(
    entity_type_id: i32,
    metadata: &mut EntityMetadata,
    projectile: &mut ProjectileLifecycleState,
    runtime: &mut EntityAbstractArrowRuntime,
    animations: &mut EntityClientAnimationState,
    transform: &mut EntityTransform,
    accessors: super::super::schema::VanillaAbstractArrowMetadataAccessors,
    block_state_id: i32,
) {
    projectile.arrow_last_block_state_id = Some(block_state_id);
    let movement = transform.delta_movement;
    transform.position = EntityVec3 {
        x: transform.position.x - vanilla_signum(movement.x) * f64::from(0.05_f32),
        y: transform.position.y - vanilla_signum(movement.y) * f64::from(0.05_f32),
        z: transform.position.z - vanilla_signum(movement.z) * f64::from(0.05_f32),
    };
    transform.delta_movement = EntityVec3::default();
    set_entity_data_bool(metadata, accessors.in_ground_data_id, true);
    animations.start_local_arrow_shake();
    let flags = entity_data_byte(
        &metadata.data_values,
        accessors.flags_data_id,
        accessors.flags_default,
    );
    set_entity_data_byte(
        metadata,
        accessors.flags_data_id,
        flags & !accessors.crit_mask,
    );
    set_entity_data_byte(metadata, accessors.pierce_level_data_id, 0);
    if entity_type_id == VANILLA_ENTITY_TYPE_TRIDENT_ID {
        projectile.arrow_hit_ground_sound_is_arrow = true;
    }
    runtime.piercing_ignore_entity_ids.clear();
}

pub(super) fn vanilla_signum(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        value
    }
}

/// `AbstractArrow.tick` tail after the effects slot: `applyInertia(0.99F)`
/// multiplies the current delta (which block callbacks may have changed),
/// then `applyGravity`.
pub(super) fn finish_abstract_arrow(
    metadata: &EntityMetadata,
    transform: &mut EntityTransform,
    finish: ArrowMoveFinish,
    accessors: super::super::schema::VanillaAbstractArrowMetadataAccessors,
) {
    if !finish.in_water {
        transform.delta_movement = scale(transform.delta_movement, f64::from(0.99_f32));
    }
    let no_gravity = vanilla_entity_metadata_entity_accessors().is_some_and(|accessors| {
        entity_data_bool(
            &metadata.data_values,
            accessors.no_gravity_data_id,
            accessors.no_gravity_default,
        )
    });
    let in_ground = entity_data_bool(
        &metadata.data_values,
        accessors.in_ground_data_id,
        accessors.in_ground_default,
    );
    if finish.physics_enabled && !in_ground && !no_gravity {
        transform.delta_movement.y -= 0.05;
    }
}

pub(super) fn advance_falling_block(
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

pub(super) fn advance_item_entity(
    transform: &mut EntityTransform,
    supporting_block: &mut EntitySupportingBlock,
    collision: EntityClientMovementCollisionResult,
) {
    transform.delta_movement = collision.pre_move_delta;
    if collision.movement_skipped {
        return;
    }
    apply_client_collision_movement(transform, supporting_block, collision);
    let on_ground = transform.on_ground.unwrap_or(false);
    let friction = if on_ground {
        collision.ground_friction * 0.98_f32
    } else {
        0.98_f32
    };
    transform.delta_movement.x *= f64::from(friction);
    transform.delta_movement.y *= f64::from(0.98_f32);
    transform.delta_movement.z *= f64::from(friction);
    if on_ground && transform.delta_movement.y < 0.0 {
        transform.delta_movement.y *= -0.5;
    }
}

pub(super) fn advance_experience_orb(
    movement: &mut EntityExperienceOrbMovement,
    transform: &mut EntityTransform,
    supporting_block: &mut EntitySupportingBlock,
    collision: EntityClientMovementCollisionResult,
) {
    if let Some(update) = collision.experience_orb_update {
        movement.following_player_uuid = update.following_player_uuid;
        movement.random_seed = update.random_seed;
    }
    let fall_speed = collision.pre_move_delta.y;
    transform.delta_movement = collision.pre_move_delta;
    apply_client_collision_movement(transform, supporting_block, collision);
    let on_ground = transform.on_ground.unwrap_or(false);
    let friction = if on_ground {
        collision.ground_friction * 0.98_f32
    } else {
        0.98_f32
    };
    transform.delta_movement = scale(transform.delta_movement, f64::from(friction));
    if on_ground && fall_speed < -0.03 {
        transform.delta_movement.y = -fall_speed * 0.4;
    }
}

pub(super) fn advance_fishing_hook(
    movement: &mut EntityFishingHookMovement,
    metadata: &mut EntityMetadata,
    transform: &mut EntityTransform,
    supporting_block: &mut EntitySupportingBlock,
    collision: EntityClientMovementCollisionResult,
) {
    let Some(update) = collision.fishing_hook_update else {
        return;
    };
    *movement = update.movement;
    transform.delta_movement = collision.pre_move_delta;
    if let Some(deflection) = collision.fishing_hook_reverse_deflection {
        let rotation = 170.0 + java_random_next_float(&mut movement.random_seed) * 20.0;
        transform.y_rot += rotation;
        transform.old_y_rot += rotation;
        if let EntityFishingHookReverseDeflection::Entity { entity_id } = deflection {
            movement.last_deflected_by = Some(entity_id);
        }
    }
    if let Some(position) = update.position_override {
        transform.position = position;
    }
    if update.clear_hooked_entity_data {
        set_entity_data_int(
            metadata,
            super::super::schema::VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID,
            super::super::schema::VANILLA_FISHING_HOOK_DEFAULT_HOOKED_ENTITY,
        );
    }
    if collision.movement_skipped {
        return;
    }
    apply_client_collision_movement(transform, supporting_block, collision);
    update_projectile_rotation(transform);
    if movement.state == EntityFishingHookState::Flying
        && (transform.on_ground.unwrap_or(false)
            || collision.requested_movement.x != collision.clipped_movement.x
            || collision.requested_movement.z != collision.clipped_movement.z)
    {
        transform.delta_movement = EntityVec3::default();
    }
    transform.delta_movement = scale(transform.delta_movement, 0.92);
}

pub(super) fn apply_client_collision_movement(
    transform: &mut EntityTransform,
    supporting_block: &mut EntitySupportingBlock,
    collision: EntityClientMovementCollisionResult,
) {
    transform.position = add(transform.position, collision.clipped_movement);
    // `Entity.move` uses `!Mth.equal` for horizontal collision, not exact
    // floating-point inequality. Vertical collision intentionally remains exact.
    let x_collision =
        (collision.requested_movement.x - collision.clipped_movement.x).abs() >= 1.0e-5;
    let z_collision =
        (collision.requested_movement.z - collision.clipped_movement.z).abs() >= 1.0e-5;
    if x_collision {
        transform.delta_movement.x = 0.0;
    }
    if z_collision {
        transform.delta_movement.z = 0.0;
    }
    let on_ground = collision.requested_movement.y < 0.0
        && collision.requested_movement.y != collision.clipped_movement.y;
    transform.on_ground = Some(on_ground);
    if on_ground {
        supporting_block.main = collision.supporting_block;
        supporting_block.on_ground_no_blocks = collision.supporting_block.is_none();
    } else {
        supporting_block.clear_physical();
    }
}

pub(super) fn advance_primed_tnt(
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

pub(super) fn nonliving_gravity_movement(
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

pub(super) fn append_arrow_crit_particles(
    entity_id: i32,
    position: EntityVec3,
    movement: EntityVec3,
    effects: &mut Vec<EntityClientMovementEffectState>,
) {
    effects.extend((0..4).map(|index| {
        let fraction = f64::from(index) / 4.0;
        EntityClientMovementEffectState::Particle(EntityClientMovementParticleState::Simple(
            EntityClientMovementSimpleParticleState {
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
            },
        ))
    }));
}

pub(super) fn update_arrow_rotation(
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

pub(super) fn throwable_gravity(policy: VanillaEntityClientMovementPolicy) -> f64 {
    match policy {
        VanillaEntityClientMovementPolicy::ThrownExperienceBottle => 0.07,
        VanillaEntityClientMovementPolicy::ThrownPotion => 0.05,
        _ => 0.03,
    }
}

pub(super) fn hurting_projectile_inertia(
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

pub(super) fn append_bubbles(
    entity_id: i32,
    position: EntityVec3,
    movement: EntityVec3,
    effects: &mut Vec<EntityClientMovementEffectState>,
) {
    let position = EntityVec3 {
        x: position.x - movement.x * 0.25,
        y: position.y - movement.y * 0.25,
        z: position.z - movement.z * 0.25,
    };
    effects.extend((0..4).map(|_| {
        EntityClientMovementEffectState::Particle(EntityClientMovementParticleState::Simple(
            EntityClientMovementSimpleParticleState {
                entity_id,
                kind: EntityClientMovementSimpleParticleKind::Bubble,
                position,
                velocity: movement,
            },
        ))
    }));
}
