use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn append_client_tick_particle_triggers(
    pending_ravager_stun_particles: &mut Vec<RavagerStunParticleState>,
    pending_evoker_fangs_crit_particles: &mut Vec<EvokerFangsCritParticleState>,
    pending_entity_client_movement_effects: &mut Vec<EntityClientMovementEffectState>,
    identity: &EntityIdentity,
    transform: &EntityTransform,
    data_values: &[bbb_protocol::packets::EntityDataValue],
    attributes: &[ProtocolAttributeSnapshot],
    client_animations: Option<EntityClientAnimationState>,
    triggers: &mut EntityClientTickParticleTriggers,
) {
    if let Some(landing) = triggers.slime_landing_particles.take() {
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::SlimeLanding(SlimeLandingParticleState {
                entity_id: identity.id,
                kind: landing.kind,
                positions: landing
                    .offsets
                    .into_iter()
                    .map(|offset| super::super::EntityVec3 {
                        x: transform.position.x + offset.x,
                        y: transform.position.y,
                        z: transform.position.z + offset.z,
                    })
                    .collect(),
            }),
        ));
    }

    append_nautilus_tick_triggers(
        pending_entity_client_movement_effects,
        identity,
        transform,
        triggers,
    );
    if triggers.minecart_tnt_smoke {
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::Simple(EntityClientMovementSimpleParticleState {
                entity_id: identity.id,
                kind: EntityClientMovementSimpleParticleKind::Smoke,
                position: super::super::EntityVec3 {
                    x: transform.position.x,
                    y: transform.position.y + 0.5,
                    z: transform.position.z,
                },
                velocity: super::super::EntityVec3::default(),
            }),
        ));
    }
    if triggers.minecart_furnace_large_smoke {
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::Simple(EntityClientMovementSimpleParticleState {
                entity_id: identity.id,
                kind: EntityClientMovementSimpleParticleKind::LargeSmoke,
                position: super::super::EntityVec3 {
                    x: transform.position.x,
                    y: transform.position.y + 0.8,
                    z: transform.position.z,
                },
                velocity: super::super::EntityVec3::default(),
            }),
        ));
    }
    if let Some(sound) = triggers.blaze_burn_sound {
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Sound(
            super::super::EntityClientMovementSoundState {
                entity_id: identity.id,
                kind: super::super::EntityClientMovementSoundKind::BlazeBurn {
                    volume: sound.volume,
                    pitch: sound.pitch,
                },
                position: super::super::EntityVec3 {
                    x: transform.position.x + 0.5,
                    y: transform.position.y + 0.5,
                    z: transform.position.z + 0.5,
                },
            },
        ));
    }
    if let Some(pitch) = triggers.warden_heartbeat_pitch {
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Sound(
            super::super::EntityClientMovementSoundState {
                entity_id: identity.id,
                kind: super::super::EntityClientMovementSoundKind::WardenHeartbeat { pitch },
                position: transform.position,
            },
        ));
    }
    if triggers.copper_golem_spin_sound {
        let weather_state =
            vanilla_entity_metadata_copper_golem_accessors_for_entity_type(identity.entity_type_id)
                .map_or(0, |acc| {
                    entity_data_int(
                        data_values,
                        acc.weather_state_data_id,
                        acc.weather_state_default,
                    )
                });
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Sound(
            super::super::EntityClientMovementSoundState {
                entity_id: identity.id,
                kind: super::super::EntityClientMovementSoundKind::CopperGolemSpin {
                    weather_state,
                },
                position: transform.position,
            },
        ));
    }
    if let Some(pitch) = triggers.ender_dragon_sounds.flap_pitch {
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Sound(
            super::super::EntityClientMovementSoundState {
                entity_id: identity.id,
                kind: super::super::EntityClientMovementSoundKind::EnderDragonFlap { pitch },
                position: transform.position,
            },
        ));
    }
    if let Some(pitch) = triggers.ender_dragon_sounds.growl_pitch {
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Sound(
            super::super::EntityClientMovementSoundState {
                entity_id: identity.id,
                kind: super::super::EntityClientMovementSoundKind::EnderDragonGrowl { pitch },
                position: transform.position,
            },
        ));
    }
    if let Some(sounds) = triggers.lightning_bolt_sounds {
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Sound(
            super::super::EntityClientMovementSoundState {
                entity_id: identity.id,
                kind: super::super::EntityClientMovementSoundKind::LightningBoltThunder {
                    pitch: sounds.thunder_pitch,
                },
                position: transform.position,
            },
        ));
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Sound(
            super::super::EntityClientMovementSoundState {
                entity_id: identity.id,
                kind: super::super::EntityClientMovementSoundKind::LightningBoltImpact {
                    pitch: sounds.impact_pitch,
                },
                position: transform.position,
            },
        ));
    }
    if let Some(offsets) = triggers.blaze_large_smoke {
        // Blaze has no pose/baby-specific dimensions. Avoid the general model
        // projection ladder in this every-tick path: generated type dimensions
        // times the canonical LivingEntity SCALE attribute are its exact box.
        let Some([base_width, base_height]) =
            vanilla_entity_type_dimensions(identity.entity_type_id)
        else {
            return;
        };
        let scale = vanilla_render_scale(identity.entity_type_id, attributes);
        let width = f64::from(base_width * scale);
        let height = f64::from(base_height * scale);
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::BlazeAmbient(BlazeAmbientParticleState {
                entity_id: identity.id,
                positions: offsets.map(|offset| super::super::EntityVec3 {
                    x: transform.position.x + offset.x_unit * width,
                    y: transform.position.y + offset.y_unit * height,
                    z: transform.position.z + offset.z_unit * width,
                }),
            }),
        ));
    }
    if let Some(offsets) = triggers.ender_portal {
        let Some([base_width, base_height]) =
            vanilla_entity_type_dimensions(identity.entity_type_id)
        else {
            return;
        };
        let scale = vanilla_render_scale(identity.entity_type_id, attributes);
        let width = f64::from(base_width * scale);
        let height = f64::from(base_height * scale);
        let y_offset = if identity.entity_type_id == VANILLA_ENTITY_TYPE_ENDERMAN_ID {
            -0.25
        } else {
            0.0
        };
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::EnderAmbient(EnderAmbientParticleState {
                entity_id: identity.id,
                positions: offsets.map(|offset| super::super::EntityVec3 {
                    x: transform.position.x + offset.x_unit * width,
                    y: transform.position.y + offset.y_unit * height + y_offset,
                    z: transform.position.z + offset.z_unit * width,
                }),
                velocities: offsets.map(|offset| offset.velocity),
            }),
        ));
    }
    if triggers.guardian_flop_sound {
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Sound(
            super::super::EntityClientMovementSoundState {
                entity_id: identity.id,
                kind: super::super::EntityClientMovementSoundKind::GuardianFlop {
                    elder:
                        super::super::renderer_catalog::vanilla_entity_renderer_is_elder_guardian(
                            identity.entity_type_id,
                        ),
                },
                position: transform.position,
            },
        ));
    }
    if let Some(offsets) = triggers.guardian_moving_bubbles {
        let Some([base_width, base_height]) =
            vanilla_entity_type_dimensions(identity.entity_type_id)
        else {
            return;
        };
        let scale = vanilla_render_scale(identity.entity_type_id, attributes);
        let width = f64::from(base_width * scale);
        let height = f64::from(base_height * scale);
        // Guardian calls `getViewVector(0.0F)`: Vanilla samples the tick-start
        // rotation endpoints, even if this tick advanced interpolation.
        let view = super::super::calculate_view_vector(transform.old_x_rot, transform.old_y_rot);
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::GuardianAmbient(GuardianAmbientParticleState {
                entity_id: identity.id,
                positions: offsets.map(|offset| super::super::EntityVec3 {
                    x: transform.position.x + offset.x_unit * width - view.x * 1.5,
                    y: transform.position.y + offset.y_unit * height - view.y * 1.5,
                    z: transform.position.z + offset.z_unit * width - view.z * 1.5,
                }),
            }),
        ));
    }
    if let Some(positions) = triggers.guardian_attack_bubbles.take() {
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::GuardianAttack(GuardianAttackParticleState {
                entity_id: identity.id,
                positions,
            }),
        ));
    }
    if triggers.ravager_stun.is_none() && triggers.evoker_fangs_crit.is_none() {
        return;
    }
    let Some(client_animations) = client_animations else {
        debug_assert!(false, "particle bounds trigger lacked its pre-tick state");
        return;
    };
    let Some(bounds) = vanilla_model_source_bounds_for_entity_data(
        identity.entity_type_id,
        identity.data,
        data_values,
        attributes,
        Some(client_animations),
        None,
    ) else {
        return;
    };

    if let Some(offset) = triggers.ravager_stun {
        let width = f64::from(bounds.max[0] - bounds.min[0]);
        let height = f64::from(bounds.max[1] - bounds.min[1]);
        let y_body_rot = f64::from(transform.y_rot).to_radians();
        pending_ravager_stun_particles.push(RavagerStunParticleState {
            entity_id: identity.id,
            position: super::super::EntityVec3 {
                x: transform.position.x - width * y_body_rot.sin() + offset.jitter_x,
                y: transform.position.y + height - 0.3,
                z: transform.position.z + width * y_body_rot.cos() + offset.jitter_z,
            },
        });
    }

    if let Some(offsets) = triggers.evoker_fangs_crit {
        let half_width = f64::from(bounds.max[0] - bounds.min[0]) * 0.5;
        pending_evoker_fangs_crit_particles.extend(offsets.into_iter().map(|offset| {
            EvokerFangsCritParticleState {
                entity_id: identity.id,
                position: super::super::EntityVec3 {
                    x: transform.position.x + offset.x_unit * half_width,
                    y: transform.position.y + offset.y_offset,
                    z: transform.position.z + offset.z_unit * half_width,
                },
                velocity: offset.velocity,
            }
        }));
    }
}

pub(super) fn append_nautilus_tick_triggers(
    pending_entity_client_movement_effects: &mut Vec<EntityClientMovementEffectState>,
    identity: &EntityIdentity,
    transform: &EntityTransform,
    triggers: &EntityClientTickParticleTriggers,
) {
    if let Some(velocity) = triggers.nautilus_bubble {
        let view = super::super::calculate_view_vector(
            transform.x_rot.clamp(-10.0, 10.0),
            transform.y_rot,
        );
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::Simple(EntityClientMovementSimpleParticleState {
                entity_id: identity.id,
                kind: EntityClientMovementSimpleParticleKind::Bubble,
                position: super::super::EntityVec3 {
                    x: transform.position.x - view.x * 1.1,
                    y: transform.position.y - view.y + 0.25,
                    z: transform.position.z - view.z * 1.1,
                },
                velocity,
            }),
        ));
    }
}

pub(super) fn block_pos_dist_sqr(left: crate::BlockPos, right: crate::BlockPos) -> f64 {
    let dx = f64::from(left.x) - f64::from(right.x);
    let dy = f64::from(left.y) - f64::from(right.y);
    let dz = f64::from(left.z) - f64::from(right.z);
    dx * dx + dy * dy + dz * dz
}

pub(super) fn add_entity_vec3(
    left: super::super::EntityVec3,
    right: super::super::EntityVec3,
) -> super::super::EntityVec3 {
    super::super::EntityVec3 {
        x: left.x + right.x,
        y: left.y + right.y,
        z: left.z + right.z,
    }
}
