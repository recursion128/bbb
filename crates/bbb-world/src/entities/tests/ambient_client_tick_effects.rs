use super::super::schema::{
    vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type,
    vanilla_entity_metadata_guardian_accessors_for_entity_type, VANILLA_ENDER_DRAGON_PHASES_26_1,
};
use super::*;

#[test]
fn blaze_client_tick_damps_fall_and_emits_ordered_ambient_effects() {
    let mut store = WorldStore::new();
    let mut blaze = protocol_add_entity_with_type(140, VANILLA_ENTITY_TYPE_BLAZE_ID);
    blaze.delta_movement = ProtocolVec3d {
        x: 0.25,
        y: -1.0,
        z: -0.5,
    };
    store.apply_add_entity(blaze);
    let mut retained = store.publication_snapshot();

    store.advance_entity_client_animations(1);

    let current = store.probe_entity(140).expect("live blaze");
    assert_eq!(current.delta_movement.x, 0.25);
    assert!((current.delta_movement.y + 0.6).abs() < 1.0e-12);
    assert_eq!(current.delta_movement.z, -0.5);
    assert_eq!(
        retained.probe_entity(140).unwrap().delta_movement.y,
        -1.0,
        "retained generation keeps its pre-tick velocity"
    );

    let effects = store
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    let smoke = effects
        .iter()
        .filter_map(|effect| match effect {
            EntityClientMovementEffectState::Particle(
                EntityClientMovementParticleState::BlazeAmbient(particle),
            ) => Some(*particle),
            _ => None,
        })
        .collect::<Vec<_>>();
    let [smoke] = smoke.as_slice() else {
        panic!("expected one two-particle blaze batch, got {smoke:?}");
    };
    assert!(smoke.positions.iter().all(|position| {
        (0.7..=1.3).contains(&position.x)
            && (64.0..=65.8).contains(&position.y)
            && (-2.3..=-1.7).contains(&position.z)
    }));
    assert!(retained
        .drain_entity_client_movement_effect_states()
        .next()
        .is_none());
    retained.advance_entity_client_animations(1);
    let retained_effects = retained
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    let retained_smoke = retained_effects.iter().find_map(|effect| match effect {
        EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::BlazeAmbient(particle),
        ) => Some(*particle),
        _ => None,
    });
    assert_eq!(
        retained_smoke,
        Some(*smoke),
        "advancing the live generation did not consume the retained RNG cursor"
    );
}

#[test]
fn blaze_burn_sound_uses_vanilla_random_ranges_and_precedes_smoke() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        141,
        VANILLA_ENTITY_TYPE_BLAZE_ID,
    ));

    let mut sound_ticks = Vec::new();
    for tick in 1..=96 {
        store.advance_entity_client_animations(1);
        let effects = store
            .drain_entity_client_movement_effect_states()
            .collect::<Vec<_>>();
        let sound_index = effects.iter().position(|effect| {
            matches!(
                effect,
                EntityClientMovementEffectState::Sound(EntityClientMovementSoundState {
                    kind: EntityClientMovementSoundKind::BlazeBurn { .. },
                    ..
                })
            )
        });
        let first_smoke_index = effects.iter().position(|effect| {
            matches!(
                effect,
                EntityClientMovementEffectState::Particle(
                    EntityClientMovementParticleState::BlazeAmbient(_)
                )
            )
        });
        assert_eq!(
            effects
                .iter()
                .filter(|effect| matches!(effect, EntityClientMovementEffectState::Particle(_)))
                .count(),
            1
        );
        if let Some(sound_index) = sound_index {
            assert!(sound_index < first_smoke_index.unwrap());
            let EntityClientMovementEffectState::Sound(sound) = effects[sound_index] else {
                unreachable!()
            };
            let EntityClientMovementSoundKind::BlazeBurn { volume, pitch } = sound.kind else {
                unreachable!()
            };
            assert!((1.0..2.0).contains(&volume));
            assert!((0.3..1.0).contains(&pitch));
            assert_eq!(
                sound.position,
                EntityVec3 {
                    x: 1.5,
                    y: 64.5,
                    z: -1.5
                }
            );
            sound_ticks.push(tick);
        }
    }
    assert_eq!(sound_ticks, vec![6, 23, 31, 32, 68, 73, 75, 83]);
}

#[test]
fn silent_blaze_suppresses_burn_sound_without_suppressing_smoke() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        142,
        VANILLA_ENTITY_TYPE_BLAZE_ID,
    ));
    let accessors = vanilla_entity_metadata_entity_accessors().unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 142,
        values: vec![protocol_bool_data(accessors.silent_data_id, true)],
    }));

    for _ in 0..96 {
        store.advance_entity_client_animations(1);
        let effects = store
            .drain_entity_client_movement_effect_states()
            .collect::<Vec<_>>();
        assert_eq!(effects.len(), 1);
        assert!(effects
            .iter()
            .all(|effect| matches!(effect, EntityClientMovementEffectState::Particle(_))));
    }
}

#[test]
fn warden_heartbeat_edge_emits_hostile_sound_and_honors_silent_metadata() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        160,
        VANILLA_ENTITY_TYPE_WARDEN_ID,
    ));

    store.advance_entity_client_animations(39);
    assert!(store
        .drain_entity_client_movement_effect_states()
        .all(|effect| !matches!(effect, EntityClientMovementEffectState::Sound(_))));
    let mut retained = store.publication_snapshot();

    store.advance_entity_client_animations(1);
    let effects = store
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    let [EntityClientMovementEffectState::Sound(sound)] = effects.as_slice() else {
        panic!("expected one heartbeat sound, got {effects:?}");
    };
    let EntityClientMovementSoundKind::WardenHeartbeat { pitch } = sound.kind else {
        panic!("expected warden heartbeat, got {sound:?}");
    };
    assert!((0.8..1.2).contains(&pitch));
    assert_eq!(sound.entity_id, 160);
    assert_eq!(
        sound.position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );

    let resolved = store.record_entity_client_movement_sound(*sound);
    assert_eq!(
        resolved.sound.location.as_deref(),
        Some("minecraft:entity.warden.heartbeat")
    );
    assert_eq!(resolved.source, "hostile");
    assert_eq!(resolved.volume, 5.0);
    assert_eq!(resolved.pitch, pitch);

    retained.advance_entity_client_animations(1);
    let retained_effects = retained
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    let [EntityClientMovementEffectState::Sound(retained_sound)] = retained_effects.as_slice()
    else {
        panic!("expected retained heartbeat sound, got {retained_effects:?}");
    };
    assert_eq!(retained_sound.kind, sound.kind);

    let accessors = vanilla_entity_metadata_entity_accessors().unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 160,
        values: vec![protocol_bool_data(accessors.silent_data_id, true)],
    }));
    store.advance_entity_client_animations(40);
    assert!(store
        .drain_entity_client_movement_effect_states()
        .all(|effect| !matches!(effect, EntityClientMovementEffectState::Sound(_))));
}

#[test]
fn ender_dragon_flap_and_ambient_growl_follow_vanilla_edges_and_silent_gate() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        162,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
    ));
    let (phase_data_id, _) = vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
    )
    .unwrap();
    let flying_phase = VANILLA_ENDER_DRAGON_PHASES_26_1
        .iter()
        .find(|phase| !phase.is_sitting)
        .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 162,
        values: vec![protocol_int_data(phase_data_id, flying_phase.phase_id)],
    }));

    store.advance_entity_client_animations(5);
    let sounds = store
        .drain_entity_client_movement_effect_states()
        .filter_map(|effect| match effect {
            EntityClientMovementEffectState::Sound(sound) => Some(sound),
            _ => None,
        })
        .collect::<Vec<_>>();
    let [flap] = sounds.as_slice() else {
        panic!("expected one fifth-tick flap sound, got {sounds:?}");
    };
    let EntityClientMovementSoundKind::EnderDragonFlap { pitch } = flap.kind else {
        panic!("expected dragon flap sound, got {flap:?}");
    };
    assert!((0.8..1.1).contains(&pitch));
    let resolved = store.record_entity_client_movement_sound(*flap);
    assert_eq!(
        resolved.sound.location.as_deref(),
        Some("minecraft:entity.ender_dragon.flap")
    );
    assert_eq!(resolved.source, "hostile");
    assert_eq!(resolved.volume, 5.0);
    assert_eq!(resolved.pitch, pitch);

    let accessors = vanilla_entity_metadata_entity_accessors().unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 162,
        values: vec![protocol_bool_data(accessors.silent_data_id, true)],
    }));
    store.advance_entity_client_animations(101);
    assert!(store
        .drain_entity_client_movement_effect_states()
        .next()
        .is_none());

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 162,
        values: vec![protocol_bool_data(accessors.silent_data_id, false)],
    }));
    // The five audible flap-setup ticks above already decremented the
    // countdown from 100 to 95; the 101 silent ticks consumed none.
    store.advance_entity_client_animations(95);
    assert!(store
        .drain_entity_client_movement_effect_states()
        .all(|effect| !matches!(
            effect,
            EntityClientMovementEffectState::Sound(EntityClientMovementSoundState {
                kind: EntityClientMovementSoundKind::EnderDragonGrowl { .. },
                ..
            })
        )));
    store.advance_entity_client_animations(1);
    let effects = store
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    let growl = effects.iter().find_map(|effect| match effect {
        EntityClientMovementEffectState::Sound(EntityClientMovementSoundState {
            kind: EntityClientMovementSoundKind::EnderDragonGrowl { pitch },
            ..
        }) => Some(*pitch),
        _ => None,
    });
    let pitch = growl.expect("101st total audible non-sitting tick emits ambient growl");
    assert!((0.8..1.1).contains(&pitch));
}

#[test]
fn sitting_attacking_dragon_growls_even_when_silent_and_keeps_generation_rng_isolated() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        163,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
    ));
    let (phase_data_id, _) = vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
    )
    .unwrap();
    let sitting_attacking = VANILLA_ENDER_DRAGON_PHASES_26_1
        .iter()
        .find(|phase| phase.java_field_name == "SITTING_ATTACKING")
        .unwrap();
    let accessors = vanilla_entity_metadata_entity_accessors().unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 163,
        values: vec![
            protocol_int_data(phase_data_id, sitting_attacking.phase_id),
            protocol_bool_data(accessors.silent_data_id, true),
        ],
    }));
    let mut retained = store.publication_snapshot();

    store.advance_entity_client_animations(1);
    retained.advance_entity_client_animations(1);
    let live = store
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    let retained = retained
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    assert_eq!(live, retained);
    let [EntityClientMovementEffectState::Sound(sound)] = live.as_slice() else {
        panic!("expected one sitting-attack growl, got {live:?}");
    };
    let EntityClientMovementSoundKind::EnderDragonGrowl { pitch } = sound.kind else {
        panic!("expected dragon growl, got {sound:?}");
    };
    assert!((0.8..1.1).contains(&pitch));
    let resolved = store.record_entity_client_movement_sound(*sound);
    assert_eq!(
        resolved.sound.location.as_deref(),
        Some("minecraft:entity.ender_dragon.growl")
    );
    assert_eq!(resolved.source, "hostile");
    assert_eq!(resolved.volume, 2.5);
    assert_eq!(resolved.pitch, pitch);
}

#[test]
fn lightning_bolt_first_tick_queues_world_owned_thunder_then_impact() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        165,
        VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID,
    ));
    let initial_sources = store.lightning_bolt_render_sources();
    let [initial_source] = initial_sources.as_slice() else {
        panic!("expected one lightning render source, got {initial_sources:?}");
    };
    assert_eq!(initial_source.entity_id, 165);
    assert_eq!(
        initial_source.position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
    let mut retained = store.publication_snapshot();
    assert_eq!(
        retained.lightning_bolt_render_sources(),
        initial_sources,
        "immutable publication clones the constructor RNG projection"
    );

    store.advance_entity_client_animations(1);
    retained.advance_entity_client_animations(1);
    let effects = store
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    let retained_effects = retained
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    assert_eq!(
        retained_effects, effects,
        "advancing the live generation does not consume retained lightning RNG"
    );
    let [EntityClientMovementEffectState::Sound(thunder), EntityClientMovementEffectState::Sound(impact)] =
        effects.as_slice()
    else {
        panic!("expected ordered thunder and impact sounds, got {effects:?}");
    };
    assert_eq!(thunder.entity_id, 165);
    assert_eq!(thunder.position, initial_source.position);
    assert_eq!(impact.entity_id, 165);
    assert_eq!(impact.position, initial_source.position);
    let EntityClientMovementSoundKind::LightningBoltThunder {
        pitch: thunder_pitch,
    } = thunder.kind
    else {
        panic!("expected lightning thunder first, got {thunder:?}");
    };
    let EntityClientMovementSoundKind::LightningBoltImpact {
        pitch: impact_pitch,
    } = impact.kind
    else {
        panic!("expected lightning impact second, got {impact:?}");
    };
    assert!((0.8..1.0).contains(&thunder_pitch));
    assert!((0.5..0.7).contains(&impact_pitch));

    let resolved_thunder = store.record_entity_client_movement_sound(*thunder);
    assert_eq!(
        resolved_thunder.sound.location.as_deref(),
        Some("minecraft:entity.lightning_bolt.thunder")
    );
    assert_eq!(resolved_thunder.source, "weather");
    assert_eq!(resolved_thunder.volume, 10_000.0);
    assert_eq!(resolved_thunder.pitch, thunder_pitch);
    let resolved_impact = store.record_entity_client_movement_sound(*impact);
    assert_eq!(
        resolved_impact.sound.location.as_deref(),
        Some("minecraft:entity.lightning_bolt.impact")
    );
    assert_eq!(resolved_impact.source, "weather");
    assert_eq!(resolved_impact.volume, 2.0);
    assert_eq!(resolved_impact.pitch, impact_pitch);

    store.advance_entity_client_animations(1);
    assert!(
        store
            .drain_entity_client_movement_effect_states()
            .next()
            .is_none(),
        "later lightning ticks do not replay constructor sounds"
    );
}

fn ender_ambient_effect(
    effects: &[EntityClientMovementEffectState],
    entity_id: i32,
) -> EnderAmbientParticleState {
    effects
        .iter()
        .find_map(|effect| match effect {
            EntityClientMovementEffectState::Particle(
                EntityClientMovementParticleState::EnderAmbient(state),
            ) if state.entity_id == entity_id => Some(*state),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing ender ambient state for {entity_id}: {effects:?}"))
}

#[test]
fn enderman_and_endermite_emit_two_exactly_bounded_portal_particles() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        143,
        VANILLA_ENTITY_TYPE_ENDERMAN_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        144,
        VANILLA_ENTITY_TYPE_ENDERMITE_ID,
    ));
    let mut retained = store.publication_snapshot();

    store.advance_entity_client_animations(1);
    let effects = store
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    assert_eq!(effects.len(), 2);

    for (entity_id, entity_type_id, y_offset) in [
        (143, VANILLA_ENTITY_TYPE_ENDERMAN_ID, -0.25),
        (144, VANILLA_ENTITY_TYPE_ENDERMITE_ID, 0.0),
    ] {
        let state = ender_ambient_effect(&effects, entity_id);
        let [width, height] =
            bbb_protocol::entity_types::vanilla_entity_type_dimensions(entity_type_id).unwrap();
        let half_width = f64::from(width) * 0.5;
        for (position, velocity) in state.positions.into_iter().zip(state.velocities) {
            assert!((1.0 - half_width..=1.0 + half_width).contains(&position.x));
            assert!((64.0 + y_offset..=64.0 + f64::from(height) + y_offset).contains(&position.y));
            assert!((-2.0 - half_width..=-2.0 + half_width).contains(&position.z));
            assert!((-1.0..1.0).contains(&velocity.x));
            assert!((-1.0..=0.0).contains(&velocity.y));
            assert!((-1.0..1.0).contains(&velocity.z));
        }
    }

    retained.advance_entity_client_animations(1);
    let retained_effects = retained
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    assert_eq!(
        ender_ambient_effect(&retained_effects, 143),
        ender_ambient_effect(&effects, 143)
    );
    assert_eq!(
        ender_ambient_effect(&retained_effects, 144),
        ender_ambient_effect(&effects, 144)
    );
}

#[test]
fn ender_ambient_rng_resets_on_same_id_recreation() {
    let mut store = WorldStore::new();
    let packet = protocol_add_entity_with_type(145, VANILLA_ENTITY_TYPE_ENDERMAN_ID);
    store.apply_add_entity(packet.clone());
    store.advance_entity_client_animations(1);
    let first = store
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();

    assert_eq!(
        store.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![145],
        }),
        1
    );
    store.apply_add_entity(packet);
    store.advance_entity_client_animations(1);
    let recreated = store
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();

    assert_eq!(
        ender_ambient_effect(&recreated, 145),
        ender_ambient_effect(&first, 145)
    );
}

fn guardian_packet(id: i32, entity_type_id: i32) -> ProtocolAddEntity {
    ProtocolAddEntity {
        id,
        uuid: default_entity_uuid(),
        entity_type_id,
        position: ProtocolVec3d {
            x: 8.5,
            y: 2.0,
            z: 8.5,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: -10.0,
        y_rot: 20.0,
        y_head_rot: 30.0,
        data: 99,
    }
}

fn guardian_effect(
    effects: &[EntityClientMovementEffectState],
    entity_id: i32,
) -> GuardianAmbientParticleState {
    effects
        .iter()
        .find_map(|effect| match effect {
            EntityClientMovementEffectState::Particle(
                EntityClientMovementParticleState::GuardianAmbient(state),
            ) if state.entity_id == entity_id => Some(*state),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing Guardian ambient state for {entity_id}: {effects:?}"))
}

#[test]
fn moving_submerged_guardian_emits_two_bubbles_behind_view_and_retains_rng() {
    let mut store = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    store.insert_decoded_chunk(empty_test_chunk());
    store.apply_add_entity(guardian_packet(146, VANILLA_ENTITY_TYPE_GUARDIAN_ID));
    let accessors =
        vanilla_entity_metadata_guardian_accessors_for_entity_type(VANILLA_ENTITY_TYPE_GUARDIAN_ID)
            .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 146,
        values: vec![protocol_bool_data(accessors.moving_data_id, true)],
    }));
    let source_water = vanilla_block_state_id("minecraft:water", [("level", "0")]);
    for y in 1..=4 {
        assert!(store.apply_block_update(ProtocolBlockUpdate {
            pos: ProtocolBlockPos { x: 8, y, z: 8 },
            block_state_id: source_water,
        }));
    }
    let mut retained = store.publication_snapshot();

    store.advance_entity_client_animations(1);
    let effects = store
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    assert_eq!(effects.len(), 1);
    let ambient = guardian_effect(&effects, 146);
    let [width, height] =
        bbb_protocol::entity_types::vanilla_entity_type_dimensions(VANILLA_ENTITY_TYPE_GUARDIAN_ID)
            .unwrap();
    let view = calculate_view_vector(-10.0, 20.0);
    let center = EntityVec3 {
        x: 8.5 - view.x * 1.5,
        y: 2.0 - view.y * 1.5,
        z: 8.5 - view.z * 1.5,
    };
    for position in ambient.positions {
        assert!(
            (center.x - f64::from(width) * 0.5..=center.x + f64::from(width) * 0.5)
                .contains(&position.x)
        );
        assert!((center.y..=center.y + f64::from(height)).contains(&position.y));
        assert!(
            (center.z - f64::from(width) * 0.5..=center.z + f64::from(width) * 0.5)
                .contains(&position.z)
        );
    }

    retained.advance_entity_client_animations(1);
    let retained_effects = retained
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    assert_eq!(guardian_effect(&retained_effects, 146), ambient);
}

#[test]
fn dry_guardian_support_face_arms_flop_and_silent_gate_suppresses_it() {
    let mut store = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    store.insert_decoded_chunk(empty_test_chunk());
    store.apply_add_entity(guardian_packet(147, VANILLA_ENTITY_TYPE_GUARDIAN_ID));
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 8, y: 1, z: 8 },
        block_state_id: vanilla_block_state_id("minecraft:stone", []),
    }));
    store.entities.with_transform_mut(147, |transform| {
        transform.delta_movement.y = -0.25;
    });
    store.advance_entity_client_animations(1);
    assert!(store
        .drain_entity_client_movement_effect_states()
        .all(|effect| !matches!(effect, EntityClientMovementEffectState::Sound(_))));
    let generation_a = store.publication_snapshot();

    store.entities.with_transform_mut(147, |transform| {
        transform.delta_movement.y = 0.25;
    });
    store.advance_entity_client_animations(1);
    let effects = store
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    assert!(effects.iter().any(|effect| matches!(
        effect,
        EntityClientMovementEffectState::Sound(EntityClientMovementSoundState {
            entity_id: 147,
            kind: EntityClientMovementSoundKind::GuardianFlop { elder: false },
            ..
        })
    )));
    assert_eq!(
        generation_a
            .probe_entity(147)
            .unwrap()
            .client_animations
            .guardian_tail
            .unwrap()
            .client_side_touched_ground,
        true
    );
    let stable_probe = serde_json::to_value(generation_a.probe_entity(147).unwrap()).unwrap();
    assert!(
        stable_probe
            .pointer("/client_animations/guardian_tail/client_side_touched_ground")
            .is_none(),
        "client-local contact memory must not change the stable probe schema"
    );

    let accessors = vanilla_entity_metadata_entity_accessors().unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 147,
        values: vec![protocol_bool_data(accessors.silent_data_id, true)],
    }));
    store.entities.with_transform_mut(147, |transform| {
        transform.delta_movement.y = -0.25;
    });
    store.advance_entity_client_animations(1);
    store
        .drain_entity_client_movement_effect_states()
        .for_each(drop);
    store.entities.with_transform_mut(147, |transform| {
        transform.delta_movement.y = 0.25;
    });
    store.advance_entity_client_animations(1);
    assert!(store
        .drain_entity_client_movement_effect_states()
        .all(|effect| !matches!(effect, EntityClientMovementEffectState::Sound(_))));
}

#[test]
fn dry_grounded_guardian_jump_updates_only_live_generation_and_resets_on_recreation() {
    let packet = guardian_packet(148, VANILLA_ENTITY_TYPE_GUARDIAN_ID);
    let mut store = WorldStore::new();
    store.apply_add_entity(packet.clone());
    assert!(store.apply_entity_move(ProtocolEntityMove {
        id: 148,
        has_position: false,
        delta_x: 0,
        delta_y: 0,
        delta_z: 0,
        y_rot: None,
        x_rot: None,
        on_ground: true,
    }));
    let generation_a = store.publication_snapshot();
    store.advance_entity_client_animations(1);
    let jumped = store.probe_entity(148).unwrap();
    assert!((-0.4..0.4).contains(&jumped.delta_movement.x));
    assert_eq!(jumped.delta_movement.y, 0.5);
    assert!((-0.4..0.4).contains(&jumped.delta_movement.z));
    assert!((0.0..360.0).contains(&jumped.y_rot));
    assert_eq!(jumped.on_ground, Some(false));
    assert_eq!(jumped.main_supporting_block_pos, None);
    assert_eq!(
        generation_a.probe_entity(148).unwrap().delta_movement.y,
        0.0
    );
    let first_jump = (jumped.delta_movement, jumped.y_rot);

    assert_eq!(
        store.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![148],
        }),
        1
    );
    store.apply_add_entity(packet);
    assert!(store.apply_entity_move(ProtocolEntityMove {
        id: 148,
        has_position: false,
        delta_x: 0,
        delta_y: 0,
        delta_z: 0,
        y_rot: None,
        x_rot: None,
        on_ground: true,
    }));
    store.advance_entity_client_animations(1);
    let recreated = store.probe_entity(148).unwrap();
    assert_eq!((recreated.delta_movement, recreated.y_rot), first_jump);

    let accessors =
        vanilla_entity_metadata_guardian_accessors_for_entity_type(VANILLA_ENTITY_TYPE_GUARDIAN_ID)
            .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 148,
        values: vec![protocol_int_data(accessors.attack_target_data_id, 999)],
    }));
    assert!(store.apply_entity_move(ProtocolEntityMove {
        id: 148,
        has_position: false,
        delta_x: 0,
        delta_y: 0,
        delta_z: 0,
        y_rot: None,
        x_rot: None,
        on_ground: true,
    }));
    store.entities.with_transform_mut(148, |transform| {
        transform.delta_movement = EntityVec3::default();
        transform.y_head_rot = -45.0;
    });
    store.advance_entity_client_animations(1);
    assert_eq!(
        store.probe_entity(148).unwrap().y_rot,
        -45.0,
        "active-target final body-yaw branch follows the random dry jump"
    );
}

fn guardian_attack_effect(
    effects: &[EntityClientMovementEffectState],
    entity_id: i32,
) -> GuardianAttackParticleState {
    effects
        .iter()
        .find_map(|effect| match effect {
            EntityClientMovementEffectState::Particle(
                EntityClientMovementParticleState::GuardianAttack(state),
            ) if state.entity_id == entity_id => Some(state.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing Guardian attack state for {entity_id}: {effects:?}"))
}

#[test]
fn guardian_active_living_target_updates_look_and_emits_ordered_beam_bubbles() {
    let mut store = WorldStore::new();
    store.apply_add_entity(guardian_packet(149, VANILLA_ENTITY_TYPE_GUARDIAN_ID));
    let mut cow = protocol_add_entity_with_type(150, VANILLA_ENTITY_TYPE_COW_ID);
    cow.position = ProtocolVec3d {
        x: 16.0,
        y: 2.0,
        z: 8.5,
    };
    store.apply_add_entity(cow);
    let accessors =
        vanilla_entity_metadata_guardian_accessors_for_entity_type(VANILLA_ENTITY_TYPE_GUARDIAN_ID)
            .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 149,
        values: vec![protocol_int_data(accessors.attack_target_data_id, 150)],
    }));
    let mut retained = store.publication_snapshot();

    store.advance_entity_client_animations(1);
    let effects = store
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    let attack = guardian_attack_effect(&effects, 149);
    assert!(!attack.positions.is_empty());
    assert!(attack.positions.iter().all(|position| {
        position.x > 8.5 && (position.z - 8.5).abs() < 1.0e-12 && position.y.is_finite()
    }));
    let guardian = store.probe_entity(149).unwrap();
    assert_eq!(guardian.y_head_rot, -60.0);
    assert_eq!(guardian.y_rot, guardian.y_head_rot);
    assert_ne!(
        guardian.x_rot, -10.0,
        "LookControl resets then pitches toward target"
    );
    assert_eq!(
        retained.probe_entity(149).unwrap().y_head_rot,
        30.0,
        "the retained generation keeps its pre-look transform"
    );

    retained.advance_entity_client_animations(1);
    let retained_effects = retained
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    assert_eq!(guardian_attack_effect(&retained_effects, 149), attack);
    assert_eq!(
        retained.probe_entity(149).unwrap().y_head_rot,
        guardian.y_head_rot
    );
}

#[test]
fn guardian_attack_requires_living_target_and_follows_ambient_effect_order() {
    let accessors =
        vanilla_entity_metadata_guardian_accessors_for_entity_type(VANILLA_ENTITY_TYPE_GUARDIAN_ID)
            .unwrap();
    let mut missing_living = WorldStore::new();
    missing_living.apply_add_entity(guardian_packet(151, VANILLA_ENTITY_TYPE_GUARDIAN_ID));
    missing_living.apply_add_entity(protocol_add_entity_with_type(
        152,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    ));
    assert!(missing_living.apply_set_entity_data(ProtocolSetEntityData {
        id: 151,
        values: vec![protocol_int_data(accessors.attack_target_data_id, 152)],
    }));
    missing_living.advance_entity_client_animations(1);
    assert!(missing_living
        .drain_entity_client_movement_effect_states()
        .all(|effect| !matches!(
            effect,
            EntityClientMovementEffectState::Particle(
                EntityClientMovementParticleState::GuardianAttack(_)
            )
        )));
    assert_eq!(missing_living.probe_entity(151).unwrap().y_head_rot, 30.0);

    let mut wet = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    wet.insert_decoded_chunk(empty_test_chunk());
    wet.apply_add_entity(guardian_packet(153, VANILLA_ENTITY_TYPE_GUARDIAN_ID));
    let mut cow = protocol_add_entity_with_type(154, VANILLA_ENTITY_TYPE_COW_ID);
    cow.position = ProtocolVec3d {
        x: 12.0,
        y: 2.0,
        z: 8.5,
    };
    wet.apply_add_entity(cow);
    assert!(wet.apply_set_entity_data(ProtocolSetEntityData {
        id: 153,
        values: vec![
            protocol_bool_data(accessors.moving_data_id, true),
            protocol_int_data(accessors.attack_target_data_id, 154),
        ],
    }));
    let source_water = vanilla_block_state_id("minecraft:water", [("level", "0")]);
    for y in 1..=4 {
        assert!(wet.apply_block_update(ProtocolBlockUpdate {
            pos: ProtocolBlockPos { x: 8, y, z: 8 },
            block_state_id: source_water,
        }));
    }
    wet.advance_entity_client_animations(1);
    let effects = wet
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    let ambient_index = effects
        .iter()
        .position(|effect| {
            matches!(
                effect,
                EntityClientMovementEffectState::Particle(
                    EntityClientMovementParticleState::GuardianAmbient(_)
                )
            )
        })
        .unwrap();
    let attack_index = effects
        .iter()
        .position(|effect| {
            matches!(
                effect,
                EntityClientMovementEffectState::Particle(
                    EntityClientMovementParticleState::GuardianAttack(_)
                )
            )
        })
        .unwrap();
    assert!(ambient_index < attack_index);
}
