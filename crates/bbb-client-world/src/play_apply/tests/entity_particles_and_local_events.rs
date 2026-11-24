use super::*;

#[test]
fn living_entity_drown_event_forwards_particle_state() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();
    let mut zombie = add_entity(
        94,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: 1.25,
            y: 64.0,
            z: -2.5,
        },
    );
    zombie.delta_movement = Vec3d {
        x: 0.1,
        y: -0.2,
        z: 0.3,
    };

    for packet in [
        PlayClientbound::AddEntity(zombie),
        PlayClientbound::AddEntity(add_entity(
            95,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            Vec3d {
                x: 4.0,
                y: 70.0,
                z: 8.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 94,
            event_id: LIVING_ENTITY_DROWN_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 95,
            event_id: LIVING_ENTITY_DROWN_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: LIVING_ENTITY_DROWN_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.living_entity_drown_particles.len(), 1);
    let state = effects.living_entity_drown_particles[0];
    assert_eq!(state.entity_id, 94);
    assert_eq!(
        state.position,
        crate::EntityVec3 {
            x: 1.25,
            y: 64.0,
            z: -2.5,
        }
    );
    assert_eq!(
        state.delta_movement,
        crate::EntityVec3 {
            x: 0.1,
            y: -0.2,
            z: 0.3,
        }
    );
    assert_eq!(store.counters().entity_events_applied, 2);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn living_entity_portal_event_forwards_particle_state() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();
    let old_position = Vec3d {
        x: 1.25,
        y: 64.0,
        z: -2.5,
    };
    let new_position = Vec3d {
        x: 2.75,
        y: 65.0,
        z: -1.0,
    };

    for packet in [
        PlayClientbound::AddEntity(add_entity(96, VANILLA_ENTITY_TYPE_ZOMBIE_ID, old_position)),
        PlayClientbound::AddEntity(add_entity(
            97,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            Vec3d {
                x: 4.0,
                y: 70.0,
                z: 8.0,
            },
        )),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }
    store.advance_entity_client_animations(1);

    for packet in [
        PlayClientbound::EntityPositionSync(EntityPositionSync {
            id: 96,
            position: new_position,
            delta_movement: Vec3d::default(),
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 96,
            event_id: LIVING_ENTITY_PORTAL_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 97,
            event_id: LIVING_ENTITY_PORTAL_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: LIVING_ENTITY_PORTAL_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.living_entity_portal_particles.len(), 1);
    let state = effects.living_entity_portal_particles[0];
    assert_eq!(state.entity_id, 96);
    assert_eq!(
        state.previous_position,
        crate::EntityVec3 {
            x: old_position.x,
            y: old_position.y,
            z: old_position.z,
        }
    );
    assert_eq!(
        state.position,
        crate::EntityVec3 {
            x: new_position.x,
            y: new_position.y,
            z: new_position.z,
        }
    );
    assert!((state.width - 0.6).abs() < 1.0e-6);
    assert!((state.height - 1.95).abs() < 1.0e-5);
    assert_eq!(store.counters().entity_events_applied, 2);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn arrow_effect_clear_event_forwards_particle_state() {
    let arrow_accessors = crate::entities::vanilla_entity_metadata_arrow_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ARROW_ID,
    )
    .expect("generated Arrow metadata accessors");
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            118,
            VANILLA_ENTITY_TYPE_ARROW_ID,
            Vec3d {
                x: 2.25,
                y: 65.0,
                z: -4.5,
            },
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 118,
            values: vec![int_entity_data(
                arrow_accessors.effect_color_data_id,
                0x0033_66cc,
            )],
        }),
        PlayClientbound::AddEntity(add_entity(
            119,
            VANILLA_ENTITY_TYPE_ARROW_ID,
            Vec3d {
                x: 3.0,
                y: 66.0,
                z: -5.0,
            },
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 119,
            values: vec![int_entity_data(arrow_accessors.effect_color_data_id, 0)],
        }),
        PlayClientbound::AddEntity(add_entity(
            120,
            VANILLA_ENTITY_TYPE_ARROW_ID,
            Vec3d {
                x: 4.0,
                y: 67.0,
                z: -6.0,
            },
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 120,
            values: vec![int_entity_data(arrow_accessors.effect_color_data_id, -1)],
        }),
        PlayClientbound::AddEntity(add_entity(
            121,
            VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
            Vec3d {
                x: 5.0,
                y: 68.0,
                z: -7.0,
            },
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 121,
            values: vec![int_entity_data(
                arrow_accessors.effect_color_data_id,
                0x0011_2233,
            )],
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 118,
            event_id: ARROW_EFFECT_CLEAR_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 119,
            event_id: ARROW_EFFECT_CLEAR_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 120,
            event_id: ARROW_EFFECT_CLEAR_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 121,
            event_id: ARROW_EFFECT_CLEAR_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: ARROW_EFFECT_CLEAR_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.arrow_effect_particles.len(), 2);
    assert_eq!(effects.arrow_effect_particles[0].entity_id, 118);
    assert_eq!(
        effects.arrow_effect_particles[0].position,
        crate::EntityVec3 {
            x: 2.25,
            y: 65.0,
            z: -4.5,
        }
    );
    assert!((effects.arrow_effect_particles[0].width - 0.5).abs() < 1.0e-6);
    assert!((effects.arrow_effect_particles[0].height - 0.5).abs() < 1.0e-6);
    assert_eq!(effects.arrow_effect_particles[0].color_rgb, 0x0033_66cc);
    assert_eq!(effects.arrow_effect_particles[1].entity_id, 119);
    assert_eq!(effects.arrow_effect_particles[1].color_rgb, 0);
    assert_eq!(store.counters().entity_events_applied, 4);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn taming_events_forward_tamable_and_horse_particle_states() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            130,
            VANILLA_ENTITY_TYPE_CAT_ID,
            Vec3d {
                x: 2.25,
                y: 65.0,
                z: -4.5,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            131,
            VANILLA_ENTITY_TYPE_HORSE_ID,
            Vec3d {
                x: 3.0,
                y: 66.0,
                z: -5.0,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            132,
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 4.0,
                y: 67.0,
                z: -6.0,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            133,
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
            Vec3d {
                x: 5.0,
                y: 68.0,
                z: -7.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 130,
            event_id: TAMING_SUCCEEDED_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 131,
            event_id: TAMING_FAILED_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 132,
            event_id: TAMING_SUCCEEDED_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 133,
            event_id: TAMING_SUCCEEDED_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: TAMING_FAILED_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.entity_taming_particles.len(), 3);
    let state = effects.entity_taming_particles[0];
    assert_eq!(state.entity_id, 130);
    assert!(state.success);
    assert_eq!(
        state.position,
        crate::EntityVec3 {
            x: 2.25,
            y: 65.0,
            z: -4.5,
        }
    );
    assert!((state.width - 0.6).abs() < 1.0e-6);
    assert!((state.height - 0.7).abs() < 1.0e-6);
    let state = effects.entity_taming_particles[1];
    assert_eq!(state.entity_id, 131);
    assert!(!state.success);
    assert_eq!(
        state.position,
        crate::EntityVec3 {
            x: 3.0,
            y: 66.0,
            z: -5.0,
        }
    );
    assert!((state.width - 1.396_484_4).abs() < 1.0e-6);
    assert!((state.height - 1.6).abs() < 1.0e-6);
    let state = effects.entity_taming_particles[2];
    assert_eq!(state.entity_id, 133);
    assert!(state.success);
    assert_eq!(store.counters().entity_events_applied, 4);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn ocelot_trusting_events_forward_shared_heart_and_smoke_particle_states() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            134,
            VANILLA_ENTITY_TYPE_OCELOT_ID,
            Vec3d {
                x: 2.25,
                y: 65.0,
                z: -4.5,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            135,
            VANILLA_ENTITY_TYPE_CAT_ID,
            Vec3d {
                x: 3.0,
                y: 66.0,
                z: -5.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 134,
            event_id: OCELOT_TRUSTING_SUCCEEDED_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 134,
            event_id: OCELOT_TRUSTING_FAILED_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 135,
            event_id: OCELOT_TRUSTING_SUCCEEDED_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: OCELOT_TRUSTING_FAILED_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.entity_taming_particles.len(), 2);
    let success = effects.entity_taming_particles[0];
    assert_eq!(success.entity_id, 134);
    assert!(success.success);
    assert_eq!(
        success.position,
        crate::EntityVec3 {
            x: 2.25,
            y: 65.0,
            z: -4.5,
        }
    );
    assert!((success.width - 0.6).abs() < 1.0e-6);
    assert!((success.height - 0.7).abs() < 1.0e-6);
    let failure = effects.entity_taming_particles[1];
    assert_eq!(failure.entity_id, 134);
    assert!(!failure.success);
    assert_eq!(store.counters().entity_events_applied, 3);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn villager_events_forward_particle_states() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            133,
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            Vec3d {
                x: 2.25,
                y: 65.0,
                z: -4.5,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            134,
            VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
            Vec3d {
                x: 3.0,
                y: 66.0,
                z: -5.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 133,
            event_id: VILLAGER_HEART_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 133,
            event_id: VILLAGER_ANGRY_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 133,
            event_id: VILLAGER_HAPPY_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 133,
            event_id: VILLAGER_SPLASH_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 134,
            event_id: VILLAGER_HEART_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: VILLAGER_ANGRY_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.villager_particles.len(), 4);
    assert_eq!(
        effects
            .villager_particles
            .iter()
            .map(|state| state.kind)
            .collect::<Vec<_>>(),
        vec![
            VillagerParticleKind::Heart,
            VillagerParticleKind::Angry,
            VillagerParticleKind::Happy,
            VillagerParticleKind::Splash,
        ]
    );
    let state = effects.villager_particles[0];
    assert_eq!(state.entity_id, 133);
    assert_eq!(
        state.position,
        crate::EntityVec3 {
            x: 2.25,
            y: 65.0,
            z: -4.5,
        }
    );
    assert!((state.width - 0.6).abs() < 1.0e-6);
    assert!((state.height - 1.95).abs() < 1.0e-6);
    assert_eq!(store.counters().entity_events_applied, 5);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn dolphin_happy_event_forwards_particle_state() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            135,
            VANILLA_ENTITY_TYPE_DOLPHIN_ID,
            Vec3d {
                x: 2.25,
                y: 65.0,
                z: -4.5,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            136,
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 3.0,
                y: 66.0,
                z: -5.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 135,
            event_id: DOLPHIN_HAPPY_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 136,
            event_id: DOLPHIN_HAPPY_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: DOLPHIN_HAPPY_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.dolphin_happy_particles.len(), 1);
    let state = effects.dolphin_happy_particles[0];
    assert_eq!(state.entity_id, 135);
    assert_eq!(
        state.position,
        crate::EntityVec3 {
            x: 2.25,
            y: 65.0,
            z: -4.5,
        }
    );
    assert!((state.width - 0.9).abs() < 1.0e-6);
    assert!((state.height - 0.6).abs() < 1.0e-6);
    assert_eq!(store.counters().entity_events_applied, 2);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn fox_eat_event_forwards_main_hand_item_particle_state() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();
    let mut fox = add_entity(
        137,
        VANILLA_ENTITY_TYPE_FOX_ID,
        Vec3d {
            x: 1.25,
            y: 64.0,
            z: -2.5,
        },
    );
    fox.y_rot = 45.0;
    fox.x_rot = -30.0;

    for packet in [
        PlayClientbound::AddEntity(fox),
        PlayClientbound::SetEquipment(SetEquipment {
            entity_id: 137,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: item_stack(42, 3),
            }],
        }),
        PlayClientbound::AddEntity(add_entity(
            138,
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 4.0,
                y: 65.0,
                z: -8.0,
            },
        )),
        PlayClientbound::SetEquipment(SetEquipment {
            entity_id: 138,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: item_stack(42, 1),
            }],
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 137,
            event_id: FOX_EAT_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 138,
            event_id: FOX_EAT_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: FOX_EAT_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.fox_eat_particles.len(), 1);
    let state = &effects.fox_eat_particles[0];
    assert_eq!(state.entity_id, 137);
    assert_eq!(
        state.position,
        crate::EntityVec3 {
            x: 1.25,
            y: 64.0,
            z: -2.5,
        }
    );
    assert_eq!(state.y_rot, 45.0);
    assert_eq!(state.x_rot, -30.0);
    assert_eq!(state.item_stack, item_stack(42, 3));
    assert_eq!(store.counters().entity_events_applied, 2);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn love_event_forwards_animal_and_allay_particle_states() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            128,
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 2.25,
                y: 65.0,
                z: -4.5,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            129,
            VANILLA_ENTITY_TYPE_ALLAY_ID,
            Vec3d {
                x: 3.0,
                y: 66.0,
                z: -5.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 128,
            event_id: ANIMAL_LOVE_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 129,
            event_id: ANIMAL_LOVE_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: ANIMAL_LOVE_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.animal_love_particles.len(), 1);
    let state = effects.animal_love_particles[0];
    assert_eq!(state.entity_id, 128);
    assert_eq!(
        state.position,
        crate::EntityVec3 {
            x: 2.25,
            y: 65.0,
            z: -4.5,
        }
    );
    assert!((state.width - 0.9).abs() < 1.0e-6);
    assert!((state.height - 1.4).abs() < 1.0e-6);
    assert_eq!(effects.allay_duplication_particles.len(), 1);
    let state = effects.allay_duplication_particles[0];
    assert_eq!(state.entity_id, 129);
    assert_eq!(
        state.position,
        crate::EntityVec3 {
            x: 3.0,
            y: 66.0,
            z: -5.0,
        }
    );
    assert!((state.width - 0.35).abs() < 1.0e-6);
    assert!((state.height - 0.6).abs() < 1.0e-6);
    assert_eq!(store.counters().entity_events_applied, 2);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn snowball_hit_event_forwards_particle_state() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            98,
            VANILLA_ENTITY_TYPE_SNOWBALL_ID,
            Vec3d {
                x: 1.25,
                y: 64.0,
                z: -2.5,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            99,
            VANILLA_ENTITY_TYPE_SNOWBALL_ID,
            Vec3d {
                x: 4.0,
                y: 70.0,
                z: 8.0,
            },
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 99,
            values: vec![item_stack_entity_data(
                VANILLA_ENTITY_TYPE_SNOWBALL_ID,
                ItemStackSummary::empty(),
            )],
        }),
        PlayClientbound::AddEntity(add_entity(
            100,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            Vec3d {
                x: 6.0,
                y: 71.0,
                z: 9.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 98,
            event_id: THROWN_ITEM_HIT_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 99,
            event_id: THROWN_ITEM_HIT_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 100,
            event_id: THROWN_ITEM_HIT_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: THROWN_ITEM_HIT_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.snowball_hit_particles.len(), 2);
    assert_eq!(effects.snowball_hit_particles[0].entity_id, 98);
    assert_eq!(
        effects.snowball_hit_particles[0].position,
        crate::EntityVec3 {
            x: 1.25,
            y: 64.0,
            z: -2.5,
        }
    );
    assert_eq!(
        effects.snowball_hit_particles[0].item_stack,
        Some(item_stack(crate::entities::VANILLA_ITEM_SNOWBALL_ID, 1))
    );
    assert_eq!(effects.snowball_hit_particles[1].entity_id, 99);
    assert_eq!(effects.snowball_hit_particles[1].item_stack, None);
    assert_eq!(store.counters().entity_events_applied, 3);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn thrown_egg_hit_event_forwards_particle_state() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            108,
            VANILLA_ENTITY_TYPE_EGG_ID,
            Vec3d {
                x: 2.25,
                y: 65.0,
                z: -4.5,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            109,
            VANILLA_ENTITY_TYPE_EGG_ID,
            Vec3d {
                x: 5.0,
                y: 71.0,
                z: 9.0,
            },
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 109,
            values: vec![item_stack_entity_data(
                VANILLA_ENTITY_TYPE_EGG_ID,
                ItemStackSummary::empty(),
            )],
        }),
        PlayClientbound::AddEntity(add_entity(
            110,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            Vec3d {
                x: 6.0,
                y: 71.0,
                z: 9.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 108,
            event_id: THROWN_ITEM_HIT_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 109,
            event_id: THROWN_ITEM_HIT_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 110,
            event_id: THROWN_ITEM_HIT_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: THROWN_ITEM_HIT_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.thrown_egg_hit_particles.len(), 1);
    assert_eq!(effects.thrown_egg_hit_particles[0].entity_id, 108);
    assert_eq!(
        effects.thrown_egg_hit_particles[0].position,
        crate::EntityVec3 {
            x: 2.25,
            y: 65.0,
            z: -4.5,
        }
    );
    assert_eq!(
        effects.thrown_egg_hit_particles[0].item_stack,
        item_stack(crate::entities::VANILLA_ITEM_EGG_ID, 1)
    );
    assert_eq!(store.counters().entity_events_applied, 3);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn honey_block_entity_events_forward_particle_state() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            96,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            Vec3d {
                x: 4.0,
                y: 70.0,
                z: 8.0,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            97,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 1.25,
                y: 64.0,
                z: -2.5,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            98,
            VANILLA_ENTITY_TYPE_SNOWBALL_ID,
            Vec3d {
                x: 2.0,
                y: 64.0,
                z: 3.0,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            99,
            VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID,
            Vec3d {
                x: 3.0,
                y: 64.0,
                z: 4.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 96,
            event_id: HONEY_BLOCK_SLIDE_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 97,
            event_id: HONEY_BLOCK_JUMP_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 96,
            event_id: HONEY_BLOCK_JUMP_EVENT_ID,
        }),
        // Snowball and MinecartSpawner do not forward unhandled ids to Entity, so event 53
        // cannot reach Entity's honey-slide consumer for either runtime class.
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 98,
            event_id: HONEY_BLOCK_SLIDE_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 99,
            event_id: HONEY_BLOCK_SLIDE_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: HONEY_BLOCK_SLIDE_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    let honey_block_state_id = crate::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:honey_block", &std::collections::BTreeMap::new())
        .unwrap()
        .id;
    assert_eq!(effects.honey_block_particles.len(), 2);
    assert_eq!(effects.honey_block_particles[0].entity_id, 96);
    assert_eq!(
        effects.honey_block_particles[0].count,
        HONEY_BLOCK_SLIDE_PARTICLE_COUNT
    );
    assert_eq!(
        effects.honey_block_particles[0].block_state_id,
        honey_block_state_id
    );
    assert_eq!(
        effects.honey_block_particles[0].position,
        crate::EntityVec3 {
            x: 4.0,
            y: 70.0,
            z: 8.0,
        }
    );
    assert_eq!(effects.honey_block_particles[1].entity_id, 97);
    assert_eq!(
        effects.honey_block_particles[1].count,
        HONEY_BLOCK_JUMP_PARTICLE_COUNT
    );
    assert_eq!(
        effects.honey_block_particles[1].block_state_id,
        honey_block_state_id
    );
    assert_eq!(
        effects.honey_block_particles[1].position,
        crate::EntityVec3 {
            x: 1.25,
            y: 64.0,
            z: -2.5,
        }
    );
    assert_eq!(store.counters().entity_events_applied, 5);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn rabbit_jump_event_forwards_sprint_block_particle_before_animation() {
    let stone_state_id = crate::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:stone", &BTreeMap::new())
        .expect("vanilla stone state")
        .id;
    let air_state_id = crate::vanilla_air_block_state_id();
    let mut store = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    store.insert_decoded_chunk(single_block_state_chunk(stone_state_id));
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();
    let mut rabbit = add_entity(
        173,
        VANILLA_ENTITY_TYPE_RABBIT_ID,
        Vec3d {
            x: 3.25,
            y: 1.2,
            z: 4.75,
        },
    );
    rabbit.delta_movement = Vec3d {
        x: 0.125,
        y: -0.25,
        z: -0.375,
    };
    assert!(store
        .apply_play_packet(
            PlayClientbound::AddEntity(rabbit),
            &mut random,
            &mut effects,
        )
        .is_none());
    let bounds = store.probe_entity_pick_bounds(173).expect("rabbit bounds");
    let width = f64::from(bounds.max[0] - bounds.min[0]);
    let expected_x = 3.25 + (expected_random.next_double() - 0.5) * width;
    let expected_z = 4.75 + (expected_random.next_double() - 0.5) * width;

    assert!(store
        .apply_play_packet(
            PlayClientbound::EntityEvent(EntityEvent {
                entity_id: 173,
                event_id: RABBIT_JUMP_EVENT_ID,
            }),
            &mut random,
            &mut effects,
        )
        .is_none());
    assert_eq!(effects.rabbit_sprint_particles.len(), 1);
    assert_eq!(effects.rabbit_sprint_world_hop_active, vec![false]);
    let state = effects.rabbit_sprint_particles[0];
    assert_eq!(state.entity_id, 173);
    assert_eq!(state.block_state_id, stone_state_id);
    assert!((state.position.x - expected_x).abs() < 1.0e-12);
    assert!((state.position.y - 1.3).abs() < 1.0e-12);
    assert!((state.position.z - expected_z).abs() < 1.0e-12);
    assert_eq!(
        state.velocity,
        crate::EntityVec3 {
            x: -0.5,
            y: 1.5,
            z: 1.5,
        }
    );
    assert!(store
        .probe_entity(173)
        .expect("rabbit state")
        .client_animations
        .rabbit_kind()
        .is_some());

    assert!(store
        .apply_play_packet(
            PlayClientbound::BlockUpdate(BlockUpdate {
                pos: ProtocolBlockPos { x: 3, y: 1, z: 4 },
                block_state_id: air_state_id,
            }),
            &mut random,
            &mut effects,
        )
        .is_none());
    assert!(store
        .apply_play_packet(
            PlayClientbound::EntityEvent(EntityEvent {
                entity_id: 173,
                event_id: RABBIT_JUMP_EVENT_ID,
            }),
            &mut random,
            &mut effects,
        )
        .is_none());
    assert_eq!(effects.rabbit_sprint_particles.len(), 1);
    assert_eq!(random.next_double(), expected_random.next_double());
}

#[test]
fn ravager_roar_entity_event_knocks_back_local_authoritative_player() {
    let mut store = ravager_roar_world(vec3(2.0, 64.0, 0.0), vec3(0.5, -0.25, 1.0), 20.0);

    let effects = apply_ravager_roar_entity_event(&mut store);
    assert_eq!(effects.ravager_roar_particles.len(), 1);
    let pose = store.local_player_pose().unwrap();
    assert_vec3_close(pose.delta_movement, vec3(2.5, -0.05, 1.0));
}

#[test]
fn ravager_roar_entity_event_does_not_knock_back_dead_local_player() {
    let mut store = ravager_roar_world(vec3(2.0, 64.0, 0.0), vec3(0.5, -0.25, 1.0), 0.0);

    let effects = apply_ravager_roar_entity_event(&mut store);
    assert_eq!(effects.ravager_roar_particles.len(), 1);
    let pose = store.local_player_pose().unwrap();
    assert_eq!(pose.delta_movement, vec3(0.5, -0.25, 1.0));
}

#[test]
fn ravager_roar_entity_event_does_not_knock_back_out_of_range_local_player() {
    let mut store = ravager_roar_world(vec3(6.0, 64.0, 0.0), vec3(0.5, -0.25, 1.0), 20.0);

    let effects = apply_ravager_roar_entity_event(&mut store);
    assert_eq!(effects.ravager_roar_particles.len(), 1);
    let pose = store.local_player_pose().unwrap();
    assert_eq!(pose.delta_movement, vec3(0.5, -0.25, 1.0));
}

#[test]
fn fishing_hook_pull_event_adds_owner_to_hook_delta_to_local_player_velocity() {
    let mut store = WorldStore::new();
    store.local_player_id = Some(7);
    store.set_local_player_pose(LocalPlayerPoseState {
        delta_movement: vec3(0.25, -0.5, 2.0),
        ..LocalPlayerPoseState::default()
    });
    store.apply_add_entity(add_entity(
        7,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        vec3(1.0, 64.0, 1.0),
    ));
    store.apply_add_entity(add_entity(
        8,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        vec3(10.0, 70.0, -4.0),
    ));
    let mut hook = add_entity(
        76,
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
        vec3(2.0, 65.0, 6.0),
    );
    hook.data = 8;
    store.apply_add_entity(hook);
    assert!(store.apply_set_entity_data(SetEntityData {
        id: 76,
        values: vec![int_entity_data(
            crate::entities::VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID,
            8,
        )],
    }));

    let mut random = LevelEventSoundRandomState::with_seed(0);
    assert!(store
        .apply_play_packet(
            PlayClientbound::EntityEvent(EntityEvent {
                entity_id: 76,
                event_id: FISHING_HOOK_PULL_EVENT_ID,
            }),
            &mut random,
            &mut NoPlayApplyEffects,
        )
        .is_none());

    assert_vec3_close(
        store.local_player_pose().unwrap().delta_movement,
        vec3(1.05, 0.0, 1.0),
    );
    assert_eq!(store.probe_entity(76).unwrap().last_event_id, Some(31));
}

#[test]
fn fishing_hook_pull_event_ignores_remote_targets_and_non_hook_entities() {
    let mut store = WorldStore::new();
    store.local_player_id = Some(7);
    store.set_local_player_pose(LocalPlayerPoseState {
        delta_movement: vec3(0.25, -0.5, 2.0),
        ..LocalPlayerPoseState::default()
    });
    for (entity_id, position) in [
        (7, vec3(1.0, 64.0, 1.0)),
        (8, vec3(10.0, 70.0, -4.0)),
        (9, vec3(3.0, 64.0, 3.0)),
    ] {
        store.apply_add_entity(add_entity(
            entity_id,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            position,
        ));
    }
    let mut hook = add_entity(
        76,
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
        vec3(2.0, 65.0, 6.0),
    );
    hook.data = 8;
    store.apply_add_entity(hook);
    assert!(store.apply_set_entity_data(SetEntityData {
        id: 76,
        values: vec![int_entity_data(
            crate::entities::VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID,
            10,
        )],
    }));
    store.apply_add_entity(add_entity(
        77,
        VANILLA_ENTITY_TYPE_ARROW_ID,
        vec3(2.0, 65.0, 6.0),
    ));

    let mut random = LevelEventSoundRandomState::with_seed(0);
    for entity_id in [76, 77] {
        assert!(store
            .apply_play_packet(
                PlayClientbound::EntityEvent(EntityEvent {
                    entity_id,
                    event_id: FISHING_HOOK_PULL_EVENT_ID,
                }),
                &mut random,
                &mut NoPlayApplyEffects,
            )
            .is_none());
    }
    assert_eq!(
        store.local_player_pose().unwrap().delta_movement,
        vec3(0.25, -0.5, 2.0)
    );
}

#[test]
fn player_entity_events_toggle_only_the_local_reduced_debug_gate() {
    let mut store = WorldStore::new();
    store.local_player_id = Some(7);
    for entity_id in [7, 8] {
        store.apply_add_entity(add_entity(
            entity_id,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            Vec3d::default(),
        ));
    }
    let mut random = LevelEventSoundRandomState::with_seed(0);

    for (entity_id, event_id, expected) in [(7, 22, true), (8, 23, true), (7, 23, false)] {
        assert!(store
            .apply_play_packet(
                PlayClientbound::EntityEvent(EntityEvent {
                    entity_id,
                    event_id,
                }),
                &mut random,
                &mut NoPlayApplyEffects,
            )
            .is_none());
        assert_eq!(store.local_player_has_reduced_debug_info(), expected);
    }

    assert_eq!(store.probe_entity(7).unwrap().last_event_id, Some(23));
    assert_eq!(store.probe_entity(8).unwrap().last_event_id, Some(23));
}

#[test]
fn player_finish_using_event_stops_only_the_local_item_use() {
    let mut store = WorldStore::new();
    store.local_player_id = Some(7);
    for entity_id in [7, 8] {
        store.apply_add_entity(add_entity(
            entity_id,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            Vec3d::default(),
        ));
    }
    store.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            item_id: Some(VANILLA_ITEM_SPYGLASS_ID),
            count: 1,
            component_patch: Default::default(),
        },
    });
    store.set_local_using_item(true);
    store.advance_local_using_item_ticks(4);
    let mut random = LevelEventSoundRandomState::with_seed(0);

    for entity_id in [8, 7] {
        assert!(store
            .apply_play_packet(
                PlayClientbound::EntityEvent(EntityEvent {
                    entity_id,
                    event_id: 9,
                }),
                &mut random,
                &mut NoPlayApplyEffects,
            )
            .is_none());
        assert_eq!(store.local_player().interaction.using_item, entity_id == 8);
    }

    assert_eq!(store.local_player().interaction.using_item_hand, None);
    assert_eq!(store.local_player().interaction.using_item_ticks, 0);
    assert_eq!(store.probe_entity(7).unwrap().last_event_id, Some(9));
    assert_eq!(store.probe_entity(8).unwrap().last_event_id, Some(9));
    assert_eq!(
        store.take_pending_local_interaction_sound_actions(),
        vec![LocalInteractionSoundAction::SpyglassStopUsing]
    );
}

#[test]
fn player_finish_using_event_spawns_only_the_active_hands_consume_particles() {
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID;

    let mut store = WorldStore::new();
    store.local_player_id = Some(7);
    store.entities.spawn_local_player(7, uuid::Uuid::nil());
    let pose = LocalPlayerPoseState {
        position: Vec3d {
            x: 3.0,
            y: 80.0,
            z: 5.0,
        },
        ..Default::default()
    };
    store.set_local_player_pose(pose);
    let mut effects = RecordingEffects::default();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    for particles in [false, true] {
        let mut food = item_stack(bbb_protocol::item_types::VANILLA_ITEM_APPLE_ID, 2);
        food.component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID);
        food.component_patch.consumable_has_consume_particles = Some(particles);
        food.component_patch.consumable = Some(bbb_protocol::packets::ConsumableSummary {
            consume_seconds: 1.0,
            animation: bbb_protocol::item_component_value_types::ItemUseAnimationSummary::Drink,
        });
        food.component_patch.consumable_sound = Some(bbb_protocol::packets::SoundEventSummary {
            registry_id: None,
            sound_id: Some("example:sip".into()),
            fixed_range_bits: None,
        });
        store.apply_set_player_inventory(SetPlayerInventory {
            slot: 40,
            item: food.clone(),
        });
        store.set_local_using_item_with_hand(true, bbb_protocol::packets::InteractionHand::OffHand);
        store.apply_play_packet(
            PlayClientbound::EntityEvent(EntityEvent {
                entity_id: 7,
                event_id: 9,
            }),
            &mut random,
            &mut effects,
        );
        let sound = effects.positioned_sounds.pop().unwrap();
        assert_eq!(sound.sound.location.as_deref(), Some("example:sip"));
        assert_eq!(sound.volume, 0.5);
        assert!((0.9..=1.0).contains(&sound.pitch));
        if particles {
            let [effect] = effects.living_entity_item_particles.as_slice() else {
                panic!("expected one item particle effect")
            };
            assert_eq!(effect.count, 16);
            assert_eq!(effect.position, pose.position);
            assert_eq!(effect.item_stack, food);
        } else {
            assert!(effects.living_entity_item_particles.is_empty());
        }
    }
}
