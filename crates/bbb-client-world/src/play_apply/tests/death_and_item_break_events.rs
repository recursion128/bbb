use super::*;

#[test]
fn remote_death_events_mutate_entities_without_client_owned_sounds() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        bbb_protocol::packets::EntityDataRegistryHolder::CowSoundVariant.registry(),
        0,
        vec![crate::RegistryPacketEntry::with_raw_data(
            "example:custom",
            cow_sound_variant_nbt(),
        )],
    );
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            84,
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            Vec3d {
                x: -2.0,
                y: 61.5,
                z: 5.75,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 84,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            85,
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            Vec3d {
                x: 4.0,
                y: 70.0,
                z: -8.0,
            },
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 85,
            values: vec![silent_entity_data(true)],
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 85,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            86,
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 9.0,
                y: 64.0,
                z: -3.0,
            },
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 86,
            values: vec![cow_sound_variant_data(0)],
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 86,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert!(effects.positioned_sounds.is_empty());
    assert_eq!(store.last_sound(), None);
    assert_eq!(store.entities.living_entity_dead(84), Some(true));
    assert_eq!(store.entities.living_entity_dead(85), Some(true));
    assert_eq!(store.entities.living_entity_dead(86), Some(true));
    assert_eq!(random.next_float(), expected_random.next_float());
    assert_eq!(store.counters().entity_events_applied, 3);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn generated_remote_living_death_events_do_not_cross_the_play_effect_boundary() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();
    let mut living_count = 0;

    for entry in bbb_protocol::entity_types::VANILLA_ENTITY_TYPES {
        if matches!(
            vanilla_entity_metadata_type_death_sound_policy(entry.id),
            Some(VanillaEntityDeathSoundPolicy::NonLiving)
        ) {
            continue;
        }
        living_count += 1;
        let entity_id = 30_000 + entry.id;
        let position = Vec3d {
            x: f64::from(entry.id),
            y: 64.0,
            z: -f64::from(entry.id),
        };
        if entry.id == VANILLA_ENTITY_TYPE_PLAYER_ID {
            record_packet_player_info(&mut store, entity_id);
        }
        assert!(store
            .apply_play_packet(
                PlayClientbound::AddEntity(add_entity(entity_id, entry.id, position)),
                &mut random,
                &mut effects,
            )
            .is_none());
        assert!(store
            .apply_play_packet(
                PlayClientbound::EntityEvent(EntityEvent {
                    entity_id,
                    event_id: LIVING_ENTITY_DEATH_EVENT_ID,
                }),
                &mut random,
                &mut effects,
            )
            .is_none());

        assert!(
            effects.positioned_sounds.is_empty(),
            "{}",
            entry.resource_id
        );
        if entry.id == VANILLA_ENTITY_TYPE_PLAYER_ID {
            assert_eq!(
                store.entities.living_entity_health(entity_id),
                Some(1.0),
                "{}",
                entry.resource_id
            );
            assert_eq!(
                store.entities.living_entity_dead(entity_id),
                Some(false),
                "{}",
                entry.resource_id
            );
        } else {
            assert_eq!(
                store.entities.living_entity_health(entity_id),
                Some(0.0),
                "{}",
                entry.resource_id
            );
            assert_eq!(
                store.entities.living_entity_dead(entity_id),
                Some(true),
                "{}",
                entry.resource_id
            );
        }
    }
    assert_eq!(living_count, 92);

    // Silent, non-living, and missing owners also resolve no sound and consume no samples.
    let silent_id = 31_000;
    for packet in [
        PlayClientbound::AddEntity(add_entity(
            silent_id,
            VANILLA_ENTITY_TYPE_DOLPHIN_ID,
            Vec3d::default(),
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: silent_id,
            values: vec![silent_entity_data(true)],
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: silent_id,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            silent_id + 1,
            VANILLA_ENTITY_TYPE_SNOWBALL_ID,
            Vec3d::default(),
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: silent_id + 1,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: i32::MAX,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
    ] {
        assert!(store
            .apply_play_packet(packet, &mut random, &mut effects)
            .is_none());
    }
    assert!(effects.positioned_sounds.is_empty());
    assert_eq!(random.next_float(), expected_random.next_float());
}

#[test]
fn local_player_death_event_forwards_player_death_sound() {
    let mut store = WorldStore::new();
    let position = Vec3d {
        x: 3.0,
        y: 72.0,
        z: -5.0,
    };
    store.local_player_id = Some(31_050);
    store.entities.spawn_local_player(31_050, uuid::Uuid::nil());
    store.set_local_player_pose(LocalPlayerPoseState {
        position,
        ..LocalPlayerPoseState::default()
    });
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_pitch = (expected_random.next_float() - expected_random.next_float()) * 0.2 + 1.0;
    let mut effects = RecordingEffects::default();

    assert!(store
        .apply_play_packet(
            PlayClientbound::EntityEvent(EntityEvent {
                entity_id: 31_050,
                event_id: LIVING_ENTITY_DEATH_EVENT_ID,
            }),
            &mut random,
            &mut effects,
        )
        .is_none());

    assert_eq!(effects.positioned_sounds.len(), 1);
    let sound = &effects.positioned_sounds[0];
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:entity.player.death")
    );
    assert_eq!(sound.source, "player");
    assert_eq!(sound.position, position);
    assert_eq!(sound.volume, 1.0);
    assert_eq!(sound.pitch, expected_pitch);
    assert_eq!(sound.seed, 0);
    assert!(!sound.distance_delay);
    assert_eq!(random.next_float(), expected_random.next_float());
}

#[test]
fn remote_living_death_wakes_entity_without_client_owned_sound() {
    let mut store = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    store.insert_decoded_chunk(single_block_state_chunk(crate::vanilla_air_block_state_id()));
    let registry = crate::BlockStateRegistry::vanilla_26_1();
    let bed_pos = ProtocolBlockPos { x: 2, y: 1, z: 2 };
    let bed_state_id = registry
        .find_by_name_and_properties(
            "minecraft:white_bed",
            &BTreeMap::from([
                ("facing".to_string(), "north".to_string()),
                ("occupied".to_string(), "true".to_string()),
                ("part".to_string(), "foot".to_string()),
            ]),
        )
        .unwrap()
        .id;
    let stone_state_id = registry
        .find_by_name_and_properties("minecraft:stone", &BTreeMap::new())
        .unwrap()
        .id;
    assert!(store.apply_block_update(BlockUpdate {
        pos: bed_pos,
        block_state_id: bed_state_id,
    }));
    assert!(store.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 3, y: 0, z: 2 },
        block_state_id: stone_state_id,
    }));

    let entity_id = 31_100;
    let old_position = Vec3d {
        x: 1.25,
        y: 4.0,
        z: 1.25,
    };
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();
    assert!(store
        .apply_play_packet(
            PlayClientbound::AddEntity(add_entity(
                entity_id,
                VANILLA_ENTITY_TYPE_CHICKEN_ID,
                old_position,
            )),
            &mut random,
            &mut effects,
        )
        .is_none());
    let entity_accessors = vanilla_entity_metadata_entity_accessors().unwrap();
    let living_accessors = vanilla_entity_metadata_living_entity_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    )
    .unwrap();
    assert!(store
        .apply_play_packet(
            PlayClientbound::SetEntityData(SetEntityData {
                id: entity_id,
                values: vec![
                    EntityDataValue {
                        data_id: entity_accessors.pose_data_id,
                        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
                        value: EntityDataValueKind::Pose(VANILLA_ENTITY_POSE_SLEEPING_ID),
                    },
                    EntityDataValue {
                        data_id: living_accessors.sleeping_pos_data_id,
                        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
                        value: EntityDataValueKind::OptionalBlockPos(Some(bed_pos)),
                    },
                ],
            }),
            &mut random,
            &mut effects,
        )
        .is_none());
    assert!(store
        .apply_play_packet(
            PlayClientbound::EntityEvent(EntityEvent {
                entity_id,
                event_id: LIVING_ENTITY_DEATH_EVENT_ID,
            }),
            &mut random,
            &mut effects,
        )
        .is_none());

    assert!(effects.positioned_sounds.is_empty());
    assert_eq!(
        store.probe_entity(entity_id).unwrap().position,
        crate::EntityVec3 {
            x: 3.5,
            y: 1.0,
            z: 2.5,
        }
    );
}

#[test]
fn zombie_death_event_waits_for_server_owned_sound_packet() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            87,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: -3.0,
                y: 61.5,
                z: 5.75,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 87,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            88,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 4.0,
                y: 70.0,
                z: -8.0,
            },
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 88,
            values: vec![silent_entity_data(true)],
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 88,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            89,
            VANILLA_ENTITY_TYPE_DOLPHIN_ID,
            Vec3d {
                x: 9.0,
                y: 64.0,
                z: -3.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 89,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert!(effects.positioned_sounds.is_empty());
    assert_eq!(store.last_sound(), None);
    assert_eq!(store.entities.living_entity_dead(87), Some(true));
    assert_eq!(store.entities.living_entity_dead(88), Some(true));
    assert_eq!(store.entities.living_entity_dead(89), Some(true));
    assert_eq!(random.next_float(), expected_random.next_float());
    assert_eq!(store.counters().entity_events_applied, 3);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn zombie_villager_death_event_waits_for_server_owned_sound_packet() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            90,
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
            Vec3d {
                x: -4.0,
                y: 61.5,
                z: 5.75,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 90,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            91,
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
            Vec3d {
                x: 4.0,
                y: 70.0,
                z: -8.0,
            },
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 91,
            values: vec![silent_entity_data(true)],
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 91,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            92,
            VANILLA_ENTITY_TYPE_DOLPHIN_ID,
            Vec3d {
                x: 9.0,
                y: 64.0,
                z: -3.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 92,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert!(effects.positioned_sounds.is_empty());
    assert_eq!(store.last_sound(), None);
    assert_eq!(store.entities.living_entity_dead(90), Some(true));
    assert_eq!(store.entities.living_entity_dead(91), Some(true));
    assert_eq!(store.entities.living_entity_dead(92), Some(true));
    assert_eq!(random.next_float(), expected_random.next_float());
    assert_eq!(store.counters().entity_events_applied, 3);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn attack_entity_death_events_wait_for_server_owned_sound_packets() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            93,
            VANILLA_ENTITY_TYPE_RAVAGER_ID,
            Vec3d {
                x: -5.0,
                y: 61.5,
                z: 5.75,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 93,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            94,
            VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
            Vec3d {
                x: 1.0,
                y: 70.0,
                z: -8.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 94,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            95,
            VANILLA_ENTITY_TYPE_RAVAGER_ID,
            Vec3d {
                x: 4.0,
                y: 70.0,
                z: -8.0,
            },
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 95,
            values: vec![silent_entity_data(true)],
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 95,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            96,
            VANILLA_ENTITY_TYPE_DOLPHIN_ID,
            Vec3d {
                x: 9.0,
                y: 64.0,
                z: -3.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 96,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert!(effects.positioned_sounds.is_empty());
    assert_eq!(store.last_sound(), None);
    for entity_id in 93..=96 {
        assert_eq!(store.entities.living_entity_dead(entity_id), Some(true));
    }
    assert_eq!(random.next_float(), expected_random.next_float());
    assert_eq!(store.counters().entity_events_applied, 4);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn villager_and_witch_death_events_wait_for_server_owned_sound_packets() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            97,
            VANILLA_ENTITY_TYPE_WITCH_ID,
            Vec3d {
                x: -6.0,
                y: 61.5,
                z: 5.75,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 97,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            98,
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            Vec3d {
                x: 2.0,
                y: 70.0,
                z: -8.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 98,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            99,
            VANILLA_ENTITY_TYPE_WITCH_ID,
            Vec3d {
                x: 4.0,
                y: 70.0,
                z: -8.0,
            },
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 99,
            values: vec![silent_entity_data(true)],
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 99,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            100,
            VANILLA_ENTITY_TYPE_DOLPHIN_ID,
            Vec3d {
                x: 9.0,
                y: 64.0,
                z: -3.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 100,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert!(effects.positioned_sounds.is_empty());
    assert_eq!(store.last_sound(), None);
    for entity_id in 97..=100 {
        assert_eq!(store.entities.living_entity_dead(entity_id), Some(true));
    }
    assert_eq!(random.next_float(), expected_random.next_float());
    assert_eq!(store.counters().entity_events_applied, 4);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn skeleton_family_death_events_wait_for_server_owned_sound_packets() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            101,
            VANILLA_ENTITY_TYPE_SKELETON_ID,
            Vec3d {
                x: -7.0,
                y: 61.5,
                z: 5.75,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 101,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            102,
            VANILLA_ENTITY_TYPE_STRAY_ID,
            Vec3d {
                x: 3.0,
                y: 70.0,
                z: -8.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 102,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            103,
            VANILLA_ENTITY_TYPE_BOGGED_ID,
            Vec3d {
                x: 4.5,
                y: 65.25,
                z: -1.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 103,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            104,
            VANILLA_ENTITY_TYPE_SKELETON_ID,
            Vec3d {
                x: 4.0,
                y: 70.0,
                z: -8.0,
            },
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 104,
            values: vec![silent_entity_data(true)],
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 104,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            105,
            VANILLA_ENTITY_TYPE_DOLPHIN_ID,
            Vec3d {
                x: 9.0,
                y: 64.0,
                z: -3.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 105,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: LIVING_ENTITY_DEATH_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert!(effects.positioned_sounds.is_empty());
    assert_eq!(store.last_sound(), None);
    for entity_id in 101..=105 {
        assert_eq!(store.entities.living_entity_dead(entity_id), Some(true));
    }
    assert_eq!(random.next_float(), expected_random.next_float());
    assert_eq!(store.counters().entity_events_applied, 5);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn witch_magic_entity_event_forwards_particle_state() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            90,
            VANILLA_ENTITY_TYPE_WITCH_ID,
            Vec3d {
                x: 1.25,
                y: 64.0,
                z: -2.5,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            91,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 4.0,
                y: 70.0,
                z: 8.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 90,
            event_id: WITCH_MAGIC_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 91,
            event_id: WITCH_MAGIC_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: WITCH_MAGIC_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.witch_magic_particles.len(), 1);
    let state = effects.witch_magic_particles[0];
    assert_eq!(state.entity_id, 90);
    assert_eq!(
        state.position,
        crate::EntityVec3 {
            x: 1.25,
            y: 64.0,
            z: -2.5,
        }
    );
    assert!((state.bounding_box_max_y - 65.95).abs() < 1.0e-5);
    assert_eq!(store.counters().entity_events_applied, 2);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn living_entity_poof_event_forwards_particle_state() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            92,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 1.25,
                y: 64.0,
                z: -2.5,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            93,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            Vec3d {
                x: 4.0,
                y: 70.0,
                z: 8.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 92,
            event_id: LIVING_ENTITY_POOF_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 93,
            event_id: LIVING_ENTITY_POOF_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: LIVING_ENTITY_POOF_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.living_entity_poof_particles.len(), 1);
    let state = effects.living_entity_poof_particles[0];
    assert_eq!(state.entity_id, 92);
    assert_eq!(
        state.position,
        crate::EntityVec3 {
            x: 1.25,
            y: 64.0,
            z: -2.5,
        }
    );
    assert!((state.width - 0.6).abs() < 1.0e-6);
    assert!((state.height - 1.95).abs() < 1.0e-5);
    assert_eq!(store.counters().entity_events_applied, 2);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn living_entity_item_break_events_select_all_vanilla_equipment_slots() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();
    let mut zombie = add_entity(
        146,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: 1.25,
            y: 64.0,
            z: -2.5,
        },
    );
    zombie.x_rot = -30.0;
    zombie.y_rot = 45.0;
    let slots = [
        (
            LIVING_ENTITY_MAIN_HAND_BREAK_EVENT_ID,
            EquipmentSlot::MainHand,
            1,
        ),
        (
            LIVING_ENTITY_OFF_HAND_BREAK_EVENT_ID,
            EquipmentSlot::OffHand,
            2,
        ),
        (LIVING_ENTITY_HEAD_BREAK_EVENT_ID, EquipmentSlot::Head, 3),
        (LIVING_ENTITY_CHEST_BREAK_EVENT_ID, EquipmentSlot::Chest, 4),
        (LIVING_ENTITY_LEGS_BREAK_EVENT_ID, EquipmentSlot::Legs, 5),
        (LIVING_ENTITY_FEET_BREAK_EVENT_ID, EquipmentSlot::Feet, 6),
        (LIVING_ENTITY_BODY_BREAK_EVENT_ID, EquipmentSlot::Body, 7),
        (
            LIVING_ENTITY_SADDLE_BREAK_EVENT_ID,
            EquipmentSlot::Saddle,
            8,
        ),
    ];
    for packet in [
        PlayClientbound::AddEntity(zombie),
        PlayClientbound::SetEquipment(SetEquipment {
            entity_id: 146,
            slots: slots
                .iter()
                .map(|(_, slot, item_id)| EquipmentSlotUpdate {
                    slot: *slot,
                    item: item_stack(*item_id, 1),
                })
                .collect(),
        }),
        PlayClientbound::AddEntity(add_entity(
            147,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            Vec3d::default(),
        )),
        PlayClientbound::SetEquipment(SetEquipment {
            entity_id: 147,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: item_stack(VANILLA_ITEM_STONE_ID, 1),
            }],
        }),
    ] {
        assert!(store
            .apply_play_packet(packet, &mut random, &mut effects)
            .is_none());
    }
    for (event_id, _, _) in slots {
        assert!(store
            .apply_play_packet(
                PlayClientbound::EntityEvent(EntityEvent {
                    entity_id: 146,
                    event_id,
                }),
                &mut random,
                &mut effects,
            )
            .is_none());
    }
    assert!(store
        .apply_play_packet(
            PlayClientbound::EntityEvent(EntityEvent {
                entity_id: 147,
                event_id: LIVING_ENTITY_MAIN_HAND_BREAK_EVENT_ID,
            }),
            &mut random,
            &mut effects,
        )
        .is_none());

    assert_eq!(effects.living_entity_item_particles.len(), 8);
    for (state, expected_item_id) in effects.living_entity_item_particles.iter().zip(1..=8) {
        assert_eq!(state.entity_id, 146);
        assert_eq!(state.item_stack.item_id, Some(expected_item_id));
        assert_eq!(state.y_rot, 45.0);
        assert_eq!(state.x_rot, -30.0);
        assert!(state.eye_height > 0.0);
    }
    assert_eq!(effects.positioned_sounds.len(), 8);
    assert!(effects.positioned_sounds.iter().all(|sound| {
        sound.sound.location.as_deref() == Some("minecraft:entity.item.break")
            && sound.source == "hostile"
            && sound.volume == 0.8
            && (0.8..=1.2).contains(&sound.pitch)
    }));
}

#[test]
fn living_entity_item_break_sound_honors_patch_removal_and_generated_defaults() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();
    assert!(store
        .apply_play_packet(
            PlayClientbound::AddEntity(add_entity(
                148,
                VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                Vec3d::default(),
            )),
            &mut random,
            &mut effects,
        )
        .is_none());

    let mut direct = item_stack(VANILLA_ITEM_STONE_ID, 1);
    direct.component_patch.break_sound = Some(SoundEventSummary {
        registry_id: None,
        sound_id: Some("example:custom_break".to_string()),
        fixed_range_bits: Some(12.5_f32.to_bits()),
    });
    let mut removed = item_stack(VANILLA_ITEM_STONE_ID, 1);
    removed
        .component_patch
        .removed_type_ids
        .push(VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID);
    for item in [direct, removed, item_stack(VANILLA_ITEM_SHIELD_ID, 1)] {
        for packet in [
            PlayClientbound::SetEquipment(SetEquipment {
                entity_id: 148,
                slots: vec![EquipmentSlotUpdate {
                    slot: EquipmentSlot::MainHand,
                    item,
                }],
            }),
            PlayClientbound::EntityEvent(EntityEvent {
                entity_id: 148,
                event_id: LIVING_ENTITY_MAIN_HAND_BREAK_EVENT_ID,
            }),
        ] {
            assert!(store
                .apply_play_packet(packet, &mut random, &mut effects)
                .is_none());
        }
    }

    assert_eq!(effects.living_entity_item_particles.len(), 3);
    assert_eq!(effects.positioned_sounds.len(), 2);
    assert_eq!(
        effects.positioned_sounds[0].sound.location.as_deref(),
        Some("example:custom_break")
    );
    assert_eq!(effects.positioned_sounds[0].sound.fixed_range, Some(12.5));
    assert_eq!(
        effects.positioned_sounds[1].sound.location.as_deref(),
        Some("minecraft:item.shield.break")
    );
}

#[test]
fn mob_spawn_event_forwards_shared_poof_particle_state_only_for_mobs() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            94,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 1.25,
                y: 64.0,
                z: -2.5,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            95,
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            Vec3d {
                x: 4.0,
                y: 70.0,
                z: 8.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 94,
            event_id: MOB_SPAWN_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 95,
            event_id: MOB_SPAWN_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: MOB_SPAWN_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.living_entity_poof_particles.len(), 1);
    let state = effects.living_entity_poof_particles[0];
    assert_eq!(state.entity_id, 94);
    assert_eq!(
        state.position,
        crate::EntityVec3 {
            x: 1.25,
            y: 64.0,
            z: -2.5,
        }
    );
    assert!((state.width - 0.6).abs() < 1.0e-6);
    assert!((state.height - 1.95).abs() < 1.0e-5);
    assert_eq!(store.counters().entity_events_applied, 2);
    assert_eq!(store.counters().entity_events_ignored, 1);
}
