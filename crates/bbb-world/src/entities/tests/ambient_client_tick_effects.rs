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
