//! Client-only per-tick entity particles and local sounds that Vanilla runs
//! exclusively on the client without server broadcast packets.

use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_DIGGING_ID, VANILLA_ENTITY_POSE_EMERGING_ID,
    VANILLA_ENTITY_POSE_INHALING_ID, VANILLA_ENTITY_POSE_LONG_JUMPING_ID,
    VANILLA_ENTITY_POSE_SHOOTING_ID, VANILLA_ENTITY_POSE_SLIDING_ID,
    VANILLA_ENTITY_POSE_STANDING_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID, VANILLA_ENTITY_TYPE_BEE_ID,
    VANILLA_ENTITY_TYPE_BREEZE_ID, VANILLA_ENTITY_TYPE_DOLPHIN_ID,
    VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID, VANILLA_ENTITY_TYPE_EVOKER_ID,
    VANILLA_ENTITY_TYPE_GLOW_SQUID_ID, VANILLA_ENTITY_TYPE_ILLUSIONER_ID,
    VANILLA_ENTITY_TYPE_PANDA_ID, VANILLA_ENTITY_TYPE_PHANTOM_ID, VANILLA_ENTITY_TYPE_SNIFFER_ID,
    VANILLA_ENTITY_TYPE_WARDEN_ID, VANILLA_ENTITY_TYPE_WITHER_ID, VANILLA_ENTITY_TYPE_WOLF_ID,
};
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID;

use crate::particles::{ParticleKind, ParticleParams, ParticleSpawn, SpawnSprite};
use crate::BlockPos;

use super::super::animations::{
    BreezeAnimationState, EntityClientAnimationState, EntityClientTickParticleTriggers,
    EntityKindAnimation,
};
use super::super::dragon::{ender_dragon_head_look_vector, ender_dragon_head_position};
use super::super::entity_data::{
    entity_data_bool, entity_data_byte, entity_data_float, entity_data_int,
};
use super::super::record::{EntityAttributes, EntityEquipment, EntityIdentity, EntityMetadata};
use super::super::{
    EntityClientMovementEffectState, EntityClientMovementSoundKind, EntityClientMovementSoundState,
    EntityTransform, EntityVec3, VANILLA_SNIFFER_STATE_DIGGING_ID,
    VANILLA_SNIFFER_STATE_SEARCHING_ID,
};
use super::*;

/// Cap AreaEffectCloud per-tick particles against malicious large radii.
const MAX_AREA_EFFECT_CLOUD_PARTICLES_PER_TICK: usize = 256;

#[inline]
pub(super) fn entity_may_emit_client_tick_particles(
    entity_type_id: i32,
    is_living: bool,
    has_metadata_values: bool,
) -> bool {
    (is_living && has_metadata_values)
        || matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID
                | VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID
                | VANILLA_ENTITY_TYPE_WITHER_ID
                | VANILLA_ENTITY_TYPE_BREEZE_ID
                | VANILLA_ENTITY_TYPE_PHANTOM_ID
                | VANILLA_ENTITY_TYPE_EVOKER_ID
                | VANILLA_ENTITY_TYPE_ILLUSIONER_ID
                | VANILLA_ENTITY_TYPE_WARDEN_ID
                | VANILLA_ENTITY_TYPE_SNIFFER_ID
                | VANILLA_ENTITY_TYPE_GLOW_SQUID_ID
                | VANILLA_ENTITY_TYPE_BEE_ID
                | VANILLA_ENTITY_TYPE_WOLF_ID
                | VANILLA_ENTITY_TYPE_DOLPHIN_ID
                | VANILLA_ENTITY_TYPE_PANDA_ID
        )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub(super) fn emit_entity_client_tick_particles_and_sounds(
    pending_particles: &mut Vec<ParticleSpawn>,
    pending_effects: &mut Vec<EntityClientMovementEffectState>,
    identity: &EntityIdentity,
    transform: &EntityTransform,
    metadata: &EntityMetadata,
    attributes: &EntityAttributes,
    equipment: Option<&EntityEquipment>,
    animations: &mut EntityClientAnimationState,
    is_living: bool,
    is_passenger: bool,
    is_silent: bool,
    in_water: bool,
    pose: Option<i32>,
    local_player_id: Option<i32>,
    profiles: &crate::ItemProfiles,
    block_effect_world: &crate::client::EntityBlockEffectWorld<'_>,
    triggers: &EntityClientTickParticleTriggers,
    content_registry: &ContentRegistry,
) {
    let entity_id = identity.id;
    let entity_type_id = identity.entity_type_id;

    // 1. LivingEntity.tickEffects (`LivingEntity.java:850-858`): synced
    // DATA_EFFECT_PARTICLES / DATA_EFFECT_AMBIENCE_ID spiral particles.
    if is_living && !metadata.data_values.is_empty() {
        emit_living_effect_particles(
            pending_particles,
            identity,
            transform,
            metadata,
            attributes,
            animations,
            content_registry,
        );
        // Remote LivingEntity eat/drink item crumb particles (`LivingEntity.java:3406-3498`).
        if local_player_id != Some(entity_id) {
            emit_remote_living_consume_particles(
                pending_particles,
                identity,
                transform,
                metadata,
                attributes,
                equipment,
                animations,
                profiles,
            );
        }
    }

    match entity_type_id {
        VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID => {
            emit_area_effect_cloud_particles(
                pending_particles,
                entity_id,
                transform,
                metadata,
                animations,
                content_registry,
            );
        }
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => {
            emit_ender_dragon_particles(
                pending_particles,
                entity_id,
                transform,
                animations,
                triggers,
            );
        }
        VANILLA_ENTITY_TYPE_WITHER_ID => {
            emit_wither_particles(
                pending_particles,
                identity,
                transform,
                metadata,
                attributes,
                animations,
            );
        }
        VANILLA_ENTITY_TYPE_BREEZE_ID => {
            emit_breeze_particles_and_sounds(
                pending_particles,
                pending_effects,
                entity_id,
                transform,
                animations,
                pose,
                is_passenger,
                is_silent,
                block_effect_world,
            );
        }
        VANILLA_ENTITY_TYPE_PHANTOM_ID => {
            emit_phantom_particles_and_sounds(
                pending_particles,
                pending_effects,
                identity,
                transform,
                metadata,
                attributes,
                animations,
                is_silent,
            );
        }
        VANILLA_ENTITY_TYPE_EVOKER_ID | VANILLA_ENTITY_TYPE_ILLUSIONER_ID => {
            emit_spellcaster_particles(
                pending_particles,
                identity,
                transform,
                metadata,
                attributes,
                animations,
            );
        }
        VANILLA_ENTITY_TYPE_WARDEN_ID => {
            emit_warden_digging_particles(
                pending_particles,
                entity_id,
                transform,
                animations,
                pose,
                block_effect_world,
            );
        }
        VANILLA_ENTITY_TYPE_SNIFFER_ID => {
            emit_sniffer_particles_and_sounds(
                pending_particles,
                pending_effects,
                entity_id,
                entity_type_id,
                transform,
                metadata,
                animations,
                is_silent,
                block_effect_world,
            );
        }
        VANILLA_ENTITY_TYPE_GLOW_SQUID_ID => {
            emit_glow_squid_particles(
                pending_particles,
                identity,
                transform,
                metadata,
                attributes,
                animations,
            );
        }
        VANILLA_ENTITY_TYPE_BEE_ID => {
            emit_bee_nectar_particles(
                pending_particles,
                entity_id,
                entity_type_id,
                transform,
                metadata,
                animations,
            );
        }
        VANILLA_ENTITY_TYPE_WOLF_ID => {
            emit_wolf_shake_particles(
                pending_particles,
                identity,
                transform,
                metadata,
                attributes,
                animations,
            );
        }
        VANILLA_ENTITY_TYPE_DOLPHIN_ID => {
            emit_dolphin_water_particles(
                pending_particles,
                entity_id,
                transform,
                in_water,
                animations,
            );
        }
        VANILLA_ENTITY_TYPE_PANDA_ID => {
            emit_panda_particles(
                pending_particles,
                identity,
                transform,
                metadata,
                attributes,
                equipment,
                animations,
            );
        }
        _ => {}
    }
}

fn entity_width_height(
    identity: &EntityIdentity,
    metadata: &EntityMetadata,
    attributes: &EntityAttributes,
    animations: &EntityClientAnimationState,
) -> (f64, f64) {
    vanilla_bounds_for_entity_data(
        identity.entity_type_id,
        identity.data,
        &metadata.data_values,
        &attributes.attributes,
        Some(*animations),
        metadata.painting_variant_projection.as_ref(),
    )
    .map_or((0.6, 1.8), |bounds| {
        (
            f64::from((bounds.max[0] - bounds.min[0]).max(0.0)),
            f64::from((bounds.max[1] - bounds.min[1]).max(0.0)),
        )
    })
}

fn emit_living_effect_particles(
    pending_particles: &mut Vec<ParticleSpawn>,
    identity: &EntityIdentity,
    transform: &EntityTransform,
    metadata: &EntityMetadata,
    attributes: &EntityAttributes,
    animations: &mut EntityClientAnimationState,
    content_registry: &ContentRegistry,
) {
    let Some(particles_data_id) =
        vanilla_entity_metadata_living_entity_effect_particles_accessor_for_entity_type(
            identity.entity_type_id,
        )
    else {
        return;
    };
    let particles = metadata.data_values.iter().find_map(|value| {
        if value.data_id == particles_data_id {
            if let EntityDataValueKind::Particles(list) = &value.value {
                return Some(list.as_slice());
            }
        }
        None
    });
    let Some(particles) = particles.filter(|list| !list.is_empty()) else {
        return;
    };
    let is_ambient =
        vanilla_entity_metadata_living_entity_effect_ambience_accessor_for_entity_type(
            identity.entity_type_id,
        )
        .is_some_and(|(data_id, default)| {
            entity_data_bool(&metadata.data_values, data_id, default)
        });
    let is_invisible = vanilla_entity_metadata_entity_accessors().is_some_and(|acc| {
        (entity_data_byte(
            &metadata.data_values,
            acc.shared_flags_data_id,
            acc.shared_flags_default,
        ) & acc.invisible_mask)
            != 0
    });
    // Vanilla `LivingEntity.tickEffects` (`LivingEntity.java:850-858`).
    let bound = if is_invisible { 15 } else { 4 };
    let ambient_factor = if is_ambient { 5 } else { 1 };
    if animations.next_local_interaction_random_int_bound(identity.id, bound * ambient_factor) != 0
    {
        return;
    }
    let idx = animations.next_local_interaction_random_int_bound(
        identity.id,
        particles.len().min(i32::MAX as usize) as i32,
    ) as usize;
    let (width, height) = entity_width_height(identity, metadata, attributes, animations);
    let rx = f64::from(animations.next_local_interaction_random_float(identity.id));
    let ry = f64::from(animations.next_local_interaction_random_float(identity.id));
    let rz = f64::from(animations.next_local_interaction_random_float(identity.id));
    let pos = [
        transform.position.x + width * (2.0 * rx - 1.0) * 0.5,
        transform.position.y + height * ry,
        transform.position.z + width * (2.0 * rz - 1.0) * 0.5,
    ];
    if let Some(spawn) = ParticleSpawn::from_protocol_payload_with_registry(
        &particles[idx],
        pos,
        [1.0, 1.0, 1.0],
        content_registry,
    ) {
        pending_particles.push(spawn);
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_remote_living_consume_particles(
    pending_particles: &mut Vec<ParticleSpawn>,
    identity: &EntityIdentity,
    transform: &EntityTransform,
    metadata: &EntityMetadata,
    attributes: &EntityAttributes,
    equipment: Option<&EntityEquipment>,
    animations: &mut EntityClientAnimationState,
    profiles: &crate::ItemProfiles,
) {
    let Some(living_accessors) =
        vanilla_entity_metadata_living_entity_accessors_for_entity_type(identity.entity_type_id)
    else {
        return;
    };
    let flags = entity_data_byte(
        &metadata.data_values,
        living_accessors.flags_data_id,
        living_accessors.flags_default,
    );
    if flags & living_accessors.is_using_mask == 0 {
        return;
    }
    // Emit every 4 ticks matching `Consumable.shouldEmitParticlesAndSounds`.
    if !animations.age_ticks.is_multiple_of(4) {
        return;
    }
    let slot = if flags & living_accessors.off_hand_mask != 0 {
        ProtocolEquipmentSlot::OffHand
    } else {
        ProtocolEquipmentSlot::MainHand
    };
    let Some(stack) = equipment.and_then(|eq| {
        eq.equipment
            .iter()
            .find(|entry| {
                entry.slot == slot && entry.item.count > 0 && entry.item.item_id.is_some()
            })
            .map(|entry| &entry.item)
    }) else {
        return;
    };
    let consumable = profiles.component_source(
        stack.item_id,
        &stack.component_patch,
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
    );
    if consumable.consumable_has_consume_particles != Some(true) {
        return;
    }
    let eye_height = vanilla_eye_height_for_entity_data(
        identity.entity_type_id,
        identity.data,
        &metadata.data_values,
        &attributes.attributes,
        Some(*animations),
        metadata.painting_variant_projection.as_ref(),
    )
    .unwrap_or(1.62);
    // Vanilla `LivingEntity.spawnItemParticles` (`LivingEntity.java:3482-3498`).
    for _ in 0..5 {
        let rx = f64::from(animations.next_local_interaction_random_float(identity.id));
        let ry = f64::from(animations.next_local_interaction_random_float(identity.id));
        let vel = rotate_entity_local(
            [(rx - 0.5) * 0.1, ry * 0.1 + 0.1, 0.0],
            transform.x_rot,
            transform.y_rot,
        );
        let py1 =
            -f64::from(animations.next_local_interaction_random_float(identity.id)) * 0.6 - 0.3;
        let px1 =
            (f64::from(animations.next_local_interaction_random_float(identity.id)) - 0.5) * 0.3;
        let offset = rotate_entity_local([px1, py1, 0.6], transform.x_rot, transform.y_rot);
        pending_particles.push(
            ParticleSpawn::new(
                ParticleKind::ITEM,
                [
                    transform.position.x + offset[0],
                    transform.position.y + f64::from(eye_height) + offset[1],
                    transform.position.z + offset[2],
                ],
                [vel[0], vel[1] + 0.05, vel[2]],
            )
            .with_sprite(SpawnSprite::ItemStack(Box::new(stack.clone()))),
        );
    }
}

fn emit_area_effect_cloud_particles(
    pending_particles: &mut Vec<ParticleSpawn>,
    entity_id: i32,
    transform: &EntityTransform,
    metadata: &EntityMetadata,
    animations: &mut EntityClientAnimationState,
    content_registry: &ContentRegistry,
) {
    // Vanilla `AreaEffectCloud.clientTick` (`AreaEffectCloud.java:153-188`).
    let is_waiting = vanilla_entity_metadata_area_effect_cloud_waiting_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
    )
    .is_some_and(|(data_id, default)| entity_data_bool(&metadata.data_values, data_id, default));
    if is_waiting && animations.next_local_interaction_random_int_bound(entity_id, 2) == 0 {
        return;
    }
    let radius = vanilla_entity_metadata_area_effect_cloud_radius_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
    )
    .map_or(3.0_f32, |(data_id, default)| {
        entity_data_float(&metadata.data_values, data_id, default)
    })
    .clamp(0.0, 32.0);
    let particle_payload =
        vanilla_entity_metadata_area_effect_cloud_particle_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
        )
        .and_then(|data_id| {
            metadata.data_values.iter().find_map(|value| {
                if value.data_id == data_id {
                    if let EntityDataValueKind::Particle(payload) = &value.value {
                        return Some(payload);
                    }
                }
                None
            })
        });
    let (particle_count, particle_radius) = if is_waiting {
        (2usize, 0.2_f32)
    } else {
        (
            ((std::f32::consts::PI * radius * radius).ceil() as usize)
                .min(MAX_AREA_EFFECT_CLOUD_PARTICLES_PER_TICK),
            radius,
        )
    };
    let is_entity_effect = particle_payload
        .is_none_or(|payload| payload.particle_type_id == VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID);
    for _ in 0..particle_count {
        let angle =
            animations.next_local_interaction_random_float(entity_id) * std::f32::consts::TAU;
        let distance = animations
            .next_local_interaction_random_float(entity_id)
            .sqrt()
            * particle_radius;
        let pos = [
            transform.position.x + f64::from(angle.cos() * distance),
            transform.position.y,
            transform.position.z + f64::from(angle.sin() * distance),
        ];
        if is_entity_effect {
            let use_default =
                is_waiting && animations.next_local_interaction_random_int_bound(entity_id, 2) == 0;
            if use_default || particle_payload.is_none() {
                pending_particles.push(
                    ParticleSpawn::new(ParticleKind::ENTITY_EFFECT, pos, [0.0, 0.0, 0.0])
                        .with_params(ParticleParams::Color([1.0, 1.0, 1.0, 1.0])),
                );
            } else if let Some(payload) = particle_payload {
                if let Some(spawn) = ParticleSpawn::from_protocol_payload_with_registry(
                    payload,
                    pos,
                    [0.0, 0.0, 0.0],
                    content_registry,
                ) {
                    pending_particles.push(spawn);
                }
            }
        } else if let Some(payload) = particle_payload {
            let vel = if is_waiting {
                [0.0, 0.0, 0.0]
            } else {
                let dx = (0.5 - animations.next_local_interaction_random_double(entity_id)) * 0.15;
                let dz = (0.5 - animations.next_local_interaction_random_double(entity_id)) * 0.15;
                [dx, 0.01, dz]
            };
            if let Some(spawn) = ParticleSpawn::from_protocol_payload_with_registry(
                payload,
                pos,
                vel,
                content_registry,
            ) {
                pending_particles.push(spawn);
            }
        }
    }
}

fn emit_ender_dragon_particles(
    pending_particles: &mut Vec<ParticleSpawn>,
    entity_id: i32,
    transform: &EntityTransform,
    animations: &mut EntityClientAnimationState,
    triggers: &EntityClientTickParticleTriggers,
) {
    // 1. Dying tick explosion (`EnderDragon.java:173-178`).
    if let Some([xo, yo, zo]) = triggers.ender_dragon_sounds.death_explosion_offset {
        pending_particles.push(ParticleSpawn::new(
            ParticleKind::EXPLOSION,
            [
                transform.position.x + f64::from(xo),
                transform.position.y + f64::from(yo),
                transform.position.z + f64::from(zo),
            ],
            [0.0, 0.0, 0.0],
        ));
    }
    let EntityKindAnimation::EnderDragon(dragon) = animations.kind else {
        return;
    };
    let Some(state) = dragon.state else {
        return;
    };
    // 2. Final death explosion emitter ticks 180..=200 (`EnderDragon.java:511-516`).
    if state.dragon_dying && (180..=200).contains(&state.dragon_death_time) {
        let xo = f64::from(animations.next_local_interaction_random_float(entity_id) - 0.5) * 8.0;
        let yo = f64::from(animations.next_local_interaction_random_float(entity_id) - 0.5) * 4.0;
        let zo = f64::from(animations.next_local_interaction_random_float(entity_id) - 0.5) * 8.0;
        pending_particles.push(ParticleSpawn::new(
            ParticleKind::EXPLOSION_EMITTER,
            [
                transform.position.x + xo,
                transform.position.y + 2.0 + yo,
                transform.position.z + zo,
            ],
            [0.0, 0.0, 0.0],
        ));
        return;
    }
    if state.dragon_dying || state.no_ai {
        return;
    }
    // 3. Phase-specific client particles.
    match state.phase_id {
        // `DragonDeathPhase.doClientTick` (`DragonDeathPhase.java:21-30`).
        9 if dragon.death_phase_ticks % 10 == 0 => {
            let xo =
                f64::from(animations.next_local_interaction_random_float(entity_id) - 0.5) * 8.0;
            let yo =
                f64::from(animations.next_local_interaction_random_float(entity_id) - 0.5) * 4.0;
            let zo =
                f64::from(animations.next_local_interaction_random_float(entity_id) - 0.5) * 8.0;
            pending_particles.push(ParticleSpawn::new(
                ParticleKind::EXPLOSION_EMITTER,
                [
                    transform.position.x + xo,
                    transform.position.y + 2.0 + yo,
                    transform.position.z + zo,
                ],
                [0.0, 0.0, 0.0],
            ));
        }
        // `DragonLandingPhase.doClientTick` (`DragonLandingPhase.java:21-47`).
        3 => {
            let head = ender_dragon_head_position(*transform, state);
            let look = ender_dragon_head_look_vector(*transform, state);
            let mut dir = rotate_y(look, -std::f64::consts::FRAC_PI_4);
            for _ in 0..8 {
                let px =
                    head.x + animations.next_local_interaction_random_gaussian(entity_id) / 2.0;
                let py = head.y
                    + 0.5
                    + animations.next_local_interaction_random_gaussian(entity_id) / 2.0;
                let pz =
                    head.z + animations.next_local_interaction_random_gaussian(entity_id) / 2.0;
                pending_particles.push(ParticleSpawn::new(
                    ParticleKind::DRAGON_BREATH,
                    [px, py, pz],
                    [
                        -dir.x * 0.08 + transform.delta_movement.x,
                        -dir.y * 0.3 + transform.delta_movement.y,
                        -dir.z * 0.08 + transform.delta_movement.z,
                    ],
                ));
                dir = rotate_y(dir, std::f64::consts::PI / 16.0);
            }
        }
        // `DragonSittingFlamingPhase.doClientTick` (`DragonSittingFlamingPhase.java:28-53`).
        5 if dragon.flame_ticks % 2 == 0 && dragon.flame_ticks < 10 => {
            let head = ender_dragon_head_position(*transform, state);
            let look = ender_dragon_head_look_vector(*transform, state);
            let mut dir = rotate_y(look, -std::f64::consts::FRAC_PI_4);
            for _ in 0..8 {
                let px =
                    head.x + animations.next_local_interaction_random_gaussian(entity_id) / 2.0;
                let py = head.y
                    + 0.5
                    + animations.next_local_interaction_random_gaussian(entity_id) / 2.0;
                let pz =
                    head.z + animations.next_local_interaction_random_gaussian(entity_id) / 2.0;
                for j in 0..6 {
                    let scale = 0.08 * f64::from(j);
                    pending_particles.push(ParticleSpawn::new(
                        ParticleKind::DRAGON_BREATH,
                        [px, py, pz],
                        [-dir.x * scale, -dir.y * 0.6, -dir.z * scale],
                    ));
                }
                dir = rotate_y(dir, std::f64::consts::PI / 16.0);
            }
        }
        _ => {}
    }
}

fn emit_wither_particles(
    pending_particles: &mut Vec<ParticleSpawn>,
    identity: &EntityIdentity,
    transform: &EntityTransform,
    metadata: &EntityMetadata,
    attributes: &EntityAttributes,
    animations: &mut EntityClientAnimationState,
) {
    // Vanilla `WitherBoss.aiStep` (`WitherBoss.java:242-257, 381-404`).
    let invulnerable_ticks =
        vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type(
            identity.entity_type_id,
        )
        .map_or(0, |(data_id, default)| {
            entity_data_int(&metadata.data_values, data_id, default)
        });
    let health =
        vanilla_entity_metadata_living_entity_accessors_for_entity_type(identity.entity_type_id)
            .map_or(300.0_f32, |acc| {
                entity_data_float(
                    &metadata.data_values,
                    acc.health_data_id,
                    acc.health_default,
                )
            });
    let max_health = attributes
        .attributes
        .iter()
        .find(|attr| attr.attribute_id == VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
        .map_or(300.0_f32, |attr| attr.base as f32);
    let is_powered = health <= max_health / 2.0;
    let scale = f64::from(vanilla_render_scale(
        identity.entity_type_id,
        &attributes.attributes,
    ));
    let y_body_rot = animations.render_body_y_rot(*transform, 1.0);

    for head in 0..3 {
        let (hx, hy, hz) = if head == 0 {
            (
                transform.position.x,
                transform.position.y + 3.0 * scale,
                transform.position.z,
            )
        } else {
            let angle = f64::from(y_body_rot + 180.0 * (head - 1) as f32).to_radians();
            (
                transform.position.x + angle.cos() * 1.3 * scale,
                transform.position.y + 2.2 * scale,
                transform.position.z + angle.sin() * 1.3 * scale,
            )
        };
        let sx = hx + animations.next_local_interaction_random_gaussian(identity.id) * 0.3;
        let sy = hy + animations.next_local_interaction_random_gaussian(identity.id) * 0.3;
        let sz = hz + animations.next_local_interaction_random_gaussian(identity.id) * 0.3;
        pending_particles.push(ParticleSpawn::new(
            ParticleKind::SMOKE,
            [sx, sy, sz],
            [0.0, 0.0, 0.0],
        ));
        if is_powered && animations.next_local_interaction_random_int_bound(identity.id, 4) == 0 {
            let px = hx + animations.next_local_interaction_random_gaussian(identity.id) * 0.3;
            let py = hy + animations.next_local_interaction_random_gaussian(identity.id) * 0.3;
            let pz = hz + animations.next_local_interaction_random_gaussian(identity.id) * 0.3;
            pending_particles.push(
                ParticleSpawn::new(ParticleKind::ENTITY_EFFECT, [px, py, pz], [0.0, 0.0, 0.0])
                    .with_params(ParticleParams::Color([0.7, 0.7, 0.5, 1.0])),
            );
        }
    }
    if invulnerable_ticks > 0 {
        for _ in 0..3 {
            let px = transform.position.x
                + animations.next_local_interaction_random_gaussian(identity.id);
            let py = transform.position.y
                + f64::from(animations.next_local_interaction_random_float(identity.id)) * 3.3;
            let pz = transform.position.z
                + animations.next_local_interaction_random_gaussian(identity.id);
            pending_particles.push(
                ParticleSpawn::new(ParticleKind::ENTITY_EFFECT, [px, py, pz], [0.0, 0.0, 0.0])
                    .with_params(ParticleParams::Color([0.7, 0.7, 0.9, 1.0])),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_breeze_particles_and_sounds(
    pending_particles: &mut Vec<ParticleSpawn>,
    pending_effects: &mut Vec<EntityClientMovementEffectState>,
    entity_id: i32,
    transform: &EntityTransform,
    animations: &mut EntityClientAnimationState,
    pose: Option<i32>,
    is_passenger: bool,
    is_silent: bool,
    block_effect_world: &crate::client::EntityBlockEffectWorld<'_>,
) {
    // Vanilla `Breeze.tick` (`Breeze.java:122-193, 215-217`).
    let pose_id = pose.unwrap_or(VANILLA_ENTITY_POSE_STANDING_ID);
    let in_block = block_effect_world.block_state_id_at(BlockPos {
        x: transform.position.x.floor() as i32,
        y: transform.position.y.floor() as i32,
        z: transform.position.z.floor() as i32,
    });
    let ground_state_id = in_block
        .filter(|&state_id| !block_effect_world.block_state_is_air(state_id))
        .or_else(|| {
            block_effect_world.block_state_id_at(BlockPos {
                x: transform.position.x.floor() as i32,
                y: (transform.position.y - 0.2).floor() as i32,
                z: transform.position.z.floor() as i32,
            })
        })
        .filter(|&state_id| block_effect_world.block_state_has_visible_render_shape(state_id));

    let (mut sound_tick, mut ambient_sound_tick, mut jump_trail_started_tick) =
        if let EntityKindAnimation::Breeze(state) = animations.kind {
            let state = state.unwrap_or_default();
            (
                state.sound_tick,
                state.ambient_sound_tick,
                state.jump_trail_started_tick,
            )
        } else {
            (0, -80, 0)
        };

    match pose_id {
        VANILLA_ENTITY_POSE_SHOOTING_ID
        | VANILLA_ENTITY_POSE_INHALING_ID
        | VANILLA_ENTITY_POSE_STANDING_ID => {
            jump_trail_started_tick = 0;
            if !is_passenger {
                if let Some(state_id) = ground_state_id {
                    pending_particles.push(
                        ParticleSpawn::new(
                            ParticleKind::BLOCK,
                            [
                                transform.position.x,
                                transform.position.y,
                                transform.position.z,
                            ],
                            [0.0, 0.0, 0.0],
                        )
                        .with_sprite(SpawnSprite::BlockState(state_id)),
                    );
                }
            }
        }
        VANILLA_ENTITY_POSE_SLIDING_ID => {
            if !is_passenger {
                if let Some(state_id) = ground_state_id {
                    for _ in 0..20 {
                        pending_particles.push(
                            ParticleSpawn::new(
                                ParticleKind::BLOCK,
                                [
                                    transform.position.x,
                                    transform.position.y,
                                    transform.position.z,
                                ],
                                [0.0, 0.0, 0.0],
                            )
                            .with_sprite(SpawnSprite::BlockState(state_id)),
                        );
                    }
                }
            }
        }
        VANILLA_ENTITY_POSE_LONG_JUMPING_ID => {
            jump_trail_started_tick = jump_trail_started_tick.saturating_add(1);
            if jump_trail_started_tick <= 5 {
                if let Some(state_id) = ground_state_id {
                    let pos = [
                        transform.position.x + transform.delta_movement.x,
                        transform.position.y + transform.delta_movement.y + 0.1,
                        transform.position.z + transform.delta_movement.z,
                    ];
                    for _ in 0..3 {
                        pending_particles.push(
                            ParticleSpawn::new(ParticleKind::BLOCK, pos, [0.0, 0.0, 0.0])
                                .with_sprite(SpawnSprite::BlockState(state_id)),
                        );
                    }
                }
            }
        }
        _ => {}
    }

    // Whirl sound (`Breeze.java:144-147, 189-193`).
    if sound_tick == 0 {
        sound_tick = (1 + animations.next_local_interaction_random_int_bound(entity_id, 80)) as u8;
    } else {
        sound_tick -= 1;
    }
    if sound_tick == 0 && !is_silent {
        let pitch = 0.7 + 0.4 * animations.next_local_interaction_random_float(entity_id);
        let volume = 0.8 + 0.2 * animations.next_local_interaction_random_float(entity_id);
        pending_effects.push(EntityClientMovementEffectState::Sound(
            EntityClientMovementSoundState {
                entity_id,
                kind: EntityClientMovementSoundKind::BreezeWhirl { volume, pitch },
                position: transform.position,
            },
        ));
    }

    // Ambient idle sound (`Mob.baseTick` + `Breeze.playAmbientSound`, `Breeze.java:183-187, 215-217`).
    let ambient_threshold = i32::from(ambient_sound_tick);
    ambient_sound_tick = ambient_sound_tick.saturating_add(1);
    if !is_silent
        && animations.next_local_interaction_random_int_bound(entity_id, 1000) < ambient_threshold
    {
        ambient_sound_tick = -80;
        pending_effects.push(EntityClientMovementEffectState::Sound(
            EntityClientMovementSoundState {
                entity_id,
                kind: EntityClientMovementSoundKind::BreezeIdle {
                    on_ground: transform.on_ground.unwrap_or(false),
                },
                position: transform.position,
            },
        ));
    }

    if let EntityKindAnimation::Breeze(slot) = &mut animations.kind {
        let state = slot.get_or_insert_with(BreezeAnimationState::default);
        state.sound_tick = sound_tick;
        state.ambient_sound_tick = ambient_sound_tick;
        state.jump_trail_started_tick = jump_trail_started_tick;
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_phantom_particles_and_sounds(
    pending_particles: &mut Vec<ParticleSpawn>,
    pending_effects: &mut Vec<EntityClientMovementEffectState>,
    identity: &EntityIdentity,
    transform: &EntityTransform,
    metadata: &EntityMetadata,
    attributes: &EntityAttributes,
    animations: &mut EntityClientAnimationState,
    is_silent: bool,
) {
    // Vanilla `Phantom.tick` (`Phantom.java:105-136`).
    let unique_offset = identity.id.wrapping_mul(3);
    let tick = i64::from(unique_offset) + i64::from(animations.age_ticks);
    let step_rad = 7.448_451_f32.to_radians();
    let anim = ((tick as f32) * step_rad + std::f32::consts::PI).cos();
    let next_anim = (((tick + 1) as f32) * step_rad + std::f32::consts::PI).cos();
    if anim > 0.0 && next_anim <= 0.0 && !is_silent {
        let volume = 0.95 + animations.next_local_interaction_random_float(identity.id) * 0.05;
        let pitch = 0.95 + animations.next_local_interaction_random_float(identity.id) * 0.05;
        pending_effects.push(EntityClientMovementEffectState::Sound(
            EntityClientMovementSoundState {
                entity_id: identity.id,
                kind: EntityClientMovementSoundKind::PhantomFlap { volume, pitch },
                position: transform.position,
            },
        ));
    }
    let (bb_width, bb_height) = entity_width_height(identity, metadata, attributes, animations);
    let width = bb_width * 1.48;
    let yaw_rad = f64::from(transform.y_rot).to_radians();
    let c = yaw_rad.cos() * width;
    let s = yaw_rad.sin() * width;
    let h = (0.3 + f64::from(anim) * 0.45) * bb_height * 2.5;
    pending_particles.push(ParticleSpawn::new(
        ParticleKind::MYCELIUM,
        [
            transform.position.x + c,
            transform.position.y + h,
            transform.position.z + s,
        ],
        [0.0, 0.0, 0.0],
    ));
    pending_particles.push(ParticleSpawn::new(
        ParticleKind::MYCELIUM,
        [
            transform.position.x - c,
            transform.position.y + h,
            transform.position.z - s,
        ],
        [0.0, 0.0, 0.0],
    ));
}

fn emit_spellcaster_particles(
    pending_particles: &mut Vec<ParticleSpawn>,
    identity: &EntityIdentity,
    transform: &EntityTransform,
    metadata: &EntityMetadata,
    attributes: &EntityAttributes,
    animations: &EntityClientAnimationState,
) {
    // Vanilla `SpellcasterIllager.tick` (`SpellcasterIllager.java:81-142`).
    let Some((data_id, default)) =
        vanilla_entity_metadata_spellcaster_casting_accessor_for_entity_type(
            identity.entity_type_id,
        )
    else {
        return;
    };
    let spell_id = entity_data_byte(&metadata.data_values, data_id, default);
    let color = match spell_id {
        1 => [0.7, 0.7, 0.8, 1.0],  // SUMMON_VEX
        2 => [0.4, 0.3, 0.35, 1.0], // FANGS
        3 => [0.7, 0.5, 0.2, 1.0],  // WOLOLO
        4 => [0.3, 0.3, 0.8, 1.0],  // DISAPPEAR
        5 => [0.1, 0.1, 0.2, 1.0],  // BLINDNESS
        _ => return,
    };
    let scale = f64::from(vanilla_render_scale(
        identity.entity_type_id,
        &attributes.attributes,
    ));
    let y_body_rot = animations.render_body_y_rot(*transform, 1.0);
    let theta = f64::from(y_body_rot).to_radians()
        + (f64::from(animations.age_ticks) * 0.6662).cos() * 0.25;
    let cx = theta.cos();
    let cz = theta.sin();
    let y_offset = 1.8 * scale;
    for sign in [1.0, -1.0] {
        pending_particles.push(
            ParticleSpawn::new(
                ParticleKind::ENTITY_EFFECT,
                [
                    transform.position.x + sign * cx * 0.6,
                    transform.position.y + y_offset,
                    transform.position.z + sign * cz * 0.6,
                ],
                [0.0, 0.0, 0.0],
            )
            .with_params(ParticleParams::Color(color)),
        );
    }
}

fn emit_warden_digging_particles(
    pending_particles: &mut Vec<ParticleSpawn>,
    entity_id: i32,
    transform: &EntityTransform,
    animations: &mut EntityClientAnimationState,
    pose: Option<i32>,
    block_effect_world: &crate::client::EntityBlockEffectWorld<'_>,
) {
    // Vanilla `Warden.clientDiggingParticles` (`Warden.java:274-280, 330-343`):
    // 30 BLOCK particles per tick while emerge/dig elapsed time < 4500ms (< 90 ticks).
    let start_age = match (pose, animations.kind) {
        (Some(VANILLA_ENTITY_POSE_EMERGING_ID), EntityKindAnimation::Warden(warden)) => {
            warden.combat.and_then(|combat| combat.emerge.start_age)
        }
        (Some(VANILLA_ENTITY_POSE_DIGGING_ID), EntityKindAnimation::Warden(warden)) => {
            warden.combat.and_then(|combat| combat.dig.start_age)
        }
        _ => return,
    };
    let Some(start_age) = start_age else {
        return;
    };
    let elapsed_ms = u64::from(animations.age_ticks.saturating_sub(start_age)) * 50;
    if elapsed_ms >= 4500 {
        return;
    }
    let Some(state_id) = block_effect_world
        .block_state_id_at(BlockPos {
            x: transform.position.x.floor() as i32,
            y: (transform.position.y - 0.2).floor() as i32,
            z: transform.position.z.floor() as i32,
        })
        .filter(|&id| block_effect_world.block_state_has_visible_render_shape(id))
    else {
        return;
    };
    for _ in 0..30 {
        let rx = f64::from(animations.next_local_interaction_random_float(entity_id));
        let rz = f64::from(animations.next_local_interaction_random_float(entity_id));
        pending_particles.push(
            ParticleSpawn::new(
                ParticleKind::BLOCK,
                [
                    transform.position.x + (rx * 2.0 - 1.0) * 0.75,
                    transform.position.y,
                    transform.position.z + (rz * 2.0 - 1.0) * 0.75,
                ],
                [0.0, 0.0, 0.0],
            )
            .with_sprite(SpawnSprite::BlockState(state_id)),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_sniffer_particles_and_sounds(
    pending_particles: &mut Vec<ParticleSpawn>,
    pending_effects: &mut Vec<EntityClientMovementEffectState>,
    entity_id: i32,
    entity_type_id: i32,
    transform: &EntityTransform,
    metadata: &EntityMetadata,
    animations: &mut EntityClientAnimationState,
    is_silent: bool,
    block_effect_world: &crate::client::EntityBlockEffectWorld<'_>,
) {
    // Vanilla `Sniffer.tick` / `emitDiggingParticles` (`Sniffer.java:296-312, 360-394`).
    let Some((data_id, default)) =
        vanilla_entity_metadata_sniffer_state_accessor_for_entity_type(entity_type_id)
    else {
        return;
    };
    let state_id = entity_data_int(&metadata.data_values, data_id, default);
    if state_id == VANILLA_SNIFFER_STATE_SEARCHING_ID {
        if animations.age_ticks.is_multiple_of(20) && !is_silent {
            pending_effects.push(EntityClientMovementEffectState::Sound(
                EntityClientMovementSoundState {
                    entity_id,
                    kind: EntityClientMovementSoundKind::SnifferSearching,
                    position: transform.position,
                },
            ));
        }
        return;
    }
    if state_id != VANILLA_SNIFFER_STATE_DIGGING_ID {
        return;
    }
    let start_age = match animations.kind {
        EntityKindAnimation::Sniffer(Some(sniffer)) => sniffer.keyframe.start_age,
        _ => None,
    };
    let Some(start_age) = start_age else {
        return;
    };
    let elapsed_ms = u64::from(animations.age_ticks.saturating_sub(start_age)) * 50;
    if elapsed_ms <= 1700 || elapsed_ms >= 6000 {
        return;
    }
    let yaw_rad = f64::from(transform.y_rot).to_radians();
    let head_x = transform.position.x - yaw_rad.sin() * 2.25;
    let head_y = transform.position.y;
    let head_z = transform.position.z + yaw_rad.cos() * 2.25;
    let head_block_pos = BlockPos {
        x: head_x.floor() as i32,
        y: head_y.floor() as i32 - 1,
        z: head_z.floor() as i32,
    };
    let Some(ground_state_id) = block_effect_world
        .block_state_id_at(head_block_pos)
        .filter(|&id| block_effect_world.block_state_has_visible_render_shape(id))
    else {
        return;
    };
    let center_x = f64::from(head_block_pos.x) + 0.5;
    let center_y = f64::from(head_block_pos.y) + 1.0 - 0.65;
    let center_z = f64::from(head_block_pos.z) + 0.5;
    for _ in 0..30 {
        pending_particles.push(
            ParticleSpawn::new(
                ParticleKind::BLOCK,
                [center_x, center_y, center_z],
                [0.0, 0.0, 0.0],
            )
            .with_sprite(SpawnSprite::BlockState(ground_state_id)),
        );
    }
    if animations.age_ticks.is_multiple_of(10) && !is_silent {
        pending_effects.push(EntityClientMovementEffectState::Sound(
            EntityClientMovementSoundState {
                entity_id,
                kind: EntityClientMovementSoundKind::SnifferDigHit {
                    block_state_id: ground_state_id,
                },
                position: EntityVec3 {
                    x: head_x,
                    y: head_y,
                    z: head_z,
                },
            },
        ));
    }
}

fn emit_glow_squid_particles(
    pending_particles: &mut Vec<ParticleSpawn>,
    identity: &EntityIdentity,
    transform: &EntityTransform,
    metadata: &EntityMetadata,
    attributes: &EntityAttributes,
    animations: &mut EntityClientAnimationState,
) {
    // Vanilla `GlowSquid.aiStep` (`GlowSquid.java:89`).
    let (width, height) = entity_width_height(identity, metadata, attributes, animations);
    let rx = f64::from(animations.next_local_interaction_random_float(identity.id));
    let ry = f64::from(animations.next_local_interaction_random_float(identity.id));
    let rz = f64::from(animations.next_local_interaction_random_float(identity.id));
    pending_particles.push(ParticleSpawn::new(
        ParticleKind::GLOW,
        [
            transform.position.x + width * (2.0 * rx - 1.0) * 0.6,
            transform.position.y + height * ry,
            transform.position.z + width * (2.0 * rz - 1.0) * 0.6,
        ],
        [0.0, 0.0, 0.0],
    ));
}

fn emit_bee_nectar_particles(
    pending_particles: &mut Vec<ParticleSpawn>,
    entity_id: i32,
    entity_type_id: i32,
    transform: &EntityTransform,
    metadata: &EntityMetadata,
    animations: &mut EntityClientAnimationState,
) {
    // Vanilla `Bee.tick` (`Bee.java:256-262`).
    let Some(acc) = vanilla_entity_metadata_bee_accessors_for_entity_type(entity_type_id) else {
        return;
    };
    let flags = entity_data_byte(&metadata.data_values, acc.flags_data_id, acc.flags_default);
    if flags & acc.has_nectar_mask == 0
        || animations.next_local_interaction_random_float(entity_id) >= 0.05
    {
        return;
    }
    let count = 1 + animations.next_local_interaction_random_int_bound(entity_id, 2);
    for _ in 0..count {
        let dx = (animations.next_local_interaction_random_double(entity_id) - 0.5) * 0.2;
        let dz = (animations.next_local_interaction_random_double(entity_id) - 0.5) * 0.2;
        pending_particles.push(ParticleSpawn::new(
            ParticleKind::FALLING_NECTAR,
            [
                transform.position.x + dx,
                transform.position.y - 0.05,
                transform.position.z + dz,
            ],
            [0.0, 0.0, 0.0],
        ));
    }
}

fn emit_wolf_shake_particles(
    pending_particles: &mut Vec<ParticleSpawn>,
    identity: &EntityIdentity,
    transform: &EntityTransform,
    metadata: &EntityMetadata,
    attributes: &EntityAttributes,
    animations: &mut EntityClientAnimationState,
) {
    // Vanilla `Wolf.tick` (`Wolf.java:345-356`).
    let shake_anim = match animations.kind {
        EntityKindAnimation::Wolf(wolf) => wolf
            .wet
            .filter(|wet| wet.is_shaking && wet.current_shake_anim > 0.4)
            .map(|wet| wet.current_shake_anim),
        _ => None,
    };
    let Some(shake_anim) = shake_anim else {
        return;
    };
    let count = (((shake_anim - 0.4) * std::f32::consts::PI).sin() * 7.0).max(0.0) as usize;
    if count == 0 {
        return;
    }
    let (width, _) = entity_width_height(identity, metadata, attributes, animations);
    for _ in 0..count {
        let xo = (f64::from(animations.next_local_interaction_random_float(identity.id)) * 2.0
            - 1.0)
            * width
            * 0.5;
        let zo = (f64::from(animations.next_local_interaction_random_float(identity.id)) * 2.0
            - 1.0)
            * width
            * 0.5;
        pending_particles.push(ParticleSpawn::new(
            ParticleKind::SPLASH,
            [
                transform.position.x + xo,
                transform.position.y + 0.8,
                transform.position.z + zo,
            ],
            [
                transform.delta_movement.x,
                transform.delta_movement.y,
                transform.delta_movement.z,
            ],
        ));
    }
}

fn emit_dolphin_water_particles(
    pending_particles: &mut Vec<ParticleSpawn>,
    entity_id: i32,
    transform: &EntityTransform,
    in_water: bool,
    animations: &mut EntityClientAnimationState,
) {
    // Vanilla `Dolphin.tick` (`Dolphin.java:252-280`).
    let dm = transform.delta_movement;
    if !in_water || (dm.x * dm.x + dm.y * dm.y + dm.z * dm.z) <= 0.03 {
        return;
    }
    let x_rad = f64::from(transform.x_rot).to_radians();
    let y_rad = f64::from(transform.y_rot).to_radians();
    let look_x = -y_rad.sin() * x_rad.cos();
    let look_y = -x_rad.sin();
    let look_z = y_rad.cos() * x_rad.cos();
    let cos_y = y_rad.cos() * 0.3;
    let sin_y = y_rad.sin() * 0.3;
    let mul = 1.2 - f64::from(animations.next_local_interaction_random_float(entity_id)) * 0.7;
    for _ in 0..2 {
        pending_particles.push(ParticleSpawn::new(
            ParticleKind::DOLPHIN,
            [
                transform.position.x - look_x * mul + cos_y,
                transform.position.y - look_y,
                transform.position.z - look_z * mul + sin_y,
            ],
            [0.0, 0.0, 0.0],
        ));
        pending_particles.push(ParticleSpawn::new(
            ParticleKind::DOLPHIN,
            [
                transform.position.x - look_x * mul - cos_y,
                transform.position.y - look_y,
                transform.position.z - look_z * mul - sin_y,
            ],
            [0.0, 0.0, 0.0],
        ));
    }
}

fn emit_panda_particles(
    pending_particles: &mut Vec<ParticleSpawn>,
    identity: &EntityIdentity,
    transform: &EntityTransform,
    metadata: &EntityMetadata,
    attributes: &EntityAttributes,
    equipment: Option<&EntityEquipment>,
    animations: &mut EntityClientAnimationState,
) {
    // Vanilla `Panda.handleEating` / `sneeze` (`Panda.java:364-372, 421-440, 503-514`).
    let Some(acc) =
        vanilla_entity_metadata_panda_accessors_for_entity_type(identity.entity_type_id)
    else {
        return;
    };
    let eat_counter = entity_data_int(
        &metadata.data_values,
        acc.eat_counter_data_id,
        acc.eat_counter_default,
    );
    if eat_counter > 0 && eat_counter % 5 == 0 {
        if let Some(stack) = equipment.and_then(|eq| {
            eq.equipment
                .iter()
                .find(|entry| {
                    entry.slot == ProtocolEquipmentSlot::MainHand
                        && entry.item.count > 0
                        && entry.item.item_id.is_some()
                })
                .map(|entry| &entry.item)
        }) {
            let yaw_rad = f64::from(transform.y_rot).to_radians();
            for _ in 0..6 {
                let rx = f64::from(animations.next_local_interaction_random_float(identity.id));
                let ry = f64::from(animations.next_local_interaction_random_float(identity.id));
                let rz = f64::from(animations.next_local_interaction_random_float(identity.id));
                let vel = [(rx - 0.5) * 0.1, ry * 0.1 + 0.1, (rz - 0.5) * 0.1];
                let px = transform.position.x - yaw_rad.sin() * 0.6;
                let py = transform.position.y + 0.5;
                let pz = transform.position.z + yaw_rad.cos() * 0.6;
                pending_particles.push(
                    ParticleSpawn::new(ParticleKind::ITEM, [px, py, pz], vel)
                        .with_sprite(SpawnSprite::ItemStack(Box::new(stack.clone()))),
                );
            }
        }
    }
    let sneeze_counter = entity_data_int(
        &metadata.data_values,
        acc.sneeze_counter_data_id,
        acc.sneeze_counter_default,
    );
    if sneeze_counter == 20 {
        let (bb_width, _) = entity_width_height(identity, metadata, attributes, animations);
        let eye_height = vanilla_eye_height_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &metadata.data_values,
            &attributes.attributes,
            Some(*animations),
            metadata.painting_variant_projection.as_ref(),
        )
        .unwrap_or(0.9);
        let body_rot_rad = f64::from(animations.render_body_y_rot(*transform, 1.0)).to_radians();
        let dist = (bb_width + 1.0) * 0.5;
        pending_particles.push(ParticleSpawn::new(
            ParticleKind::SNEEZE,
            [
                transform.position.x - dist * body_rot_rad.sin(),
                transform.position.y + f64::from(eye_height) - 0.1,
                transform.position.z + dist * body_rot_rad.cos(),
            ],
            [transform.delta_movement.x, 0.0, transform.delta_movement.z],
        ));
    }
}

fn rotate_y(v: EntityVec3, angle: f64) -> EntityVec3 {
    let cos = angle.cos();
    let sin = angle.sin();
    EntityVec3 {
        x: v.x * cos + v.z * sin,
        y: v.y,
        z: -v.x * sin + v.z * cos,
    }
}

fn rotate_entity_local(local: [f64; 3], x_rot: f32, y_rot: f32) -> [f64; 3] {
    let x_rad = -f64::from(x_rot).to_radians();
    let x_cos = x_rad.cos();
    let x_sin = x_rad.sin();
    let after_x = [
        local[0],
        local[1] * x_cos + local[2] * x_sin,
        local[2] * x_cos - local[1] * x_sin,
    ];
    let y_rad = -f64::from(y_rot).to_radians();
    let y_cos = y_rad.cos();
    let y_sin = y_rad.sin();
    [
        after_x[0] * y_cos + after_x[2] * y_sin,
        after_x[1],
        after_x[2] * y_cos - after_x[0] * y_sin,
    ]
}
