use super::*;

#[test]
fn unresolved_delete_chat_routes_fatal_stream_effect_without_world_mutation() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    let leftover = store.apply_play_packet(
        PlayClientbound::DeleteChat(DeleteChat {
            message_signature: PackedMessageSignature {
                cache_id: Some(0),
                full_signature: None,
            },
        }),
        &mut random,
        &mut effects,
    );

    assert!(leftover.is_none());
    assert_eq!(
        effects.chat_disconnects,
        vec![ChatDisconnectReason::InvalidPackedSignature]
    );
    assert!(store.client_chat().deleted_messages.is_empty());
    assert_eq!(store.counters().delete_chat_packets, 1);
    assert_eq!(store.counters().chat_unknown_packed_signatures, 1);
}

#[test]
fn removed_vehicle_teleport_routes_recovery_effect() {
    let mut store = WorldStore::new();
    store.local_player_id = Some(7);
    store.set_local_player_pose(LocalPlayerPoseState::default());
    store.apply_add_entity(add_entity(
        10,
        VANILLA_ENTITY_TYPE_HORSE_ID,
        vec3(0.0, 64.0, 0.0),
    ));
    assert!(store.apply_set_passengers(SetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![7],
    }));
    store.apply_remove_entities(RemoveEntities {
        entity_ids: vec![10],
    });

    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();
    assert!(store
        .apply_play_packet(
            PlayClientbound::TeleportEntity(TeleportEntity {
                id: 10,
                position: vec3(4.0, 70.0, -2.0),
                delta_movement: Vec3d::default(),
                y_rot: 80.0,
                x_rot: -10.0,
                relatives_mask: 0,
                on_ground: true,
            }),
            &mut random,
            &mut effects,
        )
        .is_none());
    assert_eq!(effects.removed_vehicle_teleports.len(), 1);
    assert_eq!(
        effects.removed_vehicle_teleports[0].player_pose.position.x,
        4.0
    );
}

#[test]
fn controlled_vehicle_teleport_routes_post_snap_move_report() {
    let mut store = WorldStore::new();
    store.local_player_id = Some(7);
    store.set_local_player_pose(LocalPlayerPoseState::default());
    store.apply_add_entity(add_entity(
        10,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        vec3(0.0, 64.0, 0.0),
    ));
    assert!(store.apply_set_passengers(SetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![7],
    }));

    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();
    assert!(store
        .apply_play_packet(
            PlayClientbound::TeleportEntity(TeleportEntity {
                id: 10,
                position: vec3(4.0, 70.0, -2.0),
                delta_movement: Vec3d::default(),
                y_rot: 80.0,
                x_rot: -10.0,
                relatives_mask: 0,
                on_ground: true,
            }),
            &mut random,
            &mut effects,
        )
        .is_none());

    assert!(effects.removed_vehicle_teleports.is_empty());
    assert_eq!(effects.vehicle_move_reports.len(), 1);
    let report = effects.vehicle_move_reports[0];
    let vehicle = store.probe_entity(10).unwrap();
    assert_eq!(report.position, vehicle.position);
    assert_eq!(report.y_rot, vehicle.y_rot);
    assert_eq!(report.x_rot, vehicle.x_rot);
    assert!(report.snapped);
}

#[test]
fn apply_play_packet_forwards_positioned_sound_effects() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    let leftover = store.apply_play_packet(
        PlayClientbound::Sound(SoundEvent {
            sound: SoundEventHolder::Direct {
                location: "minecraft:block.stone.break".to_string(),
                fixed_range: None,
            },
            source: SoundSource::Blocks,
            position: Vec3d {
                x: 0.5,
                y: 64.5,
                z: 0.5,
            },
            volume: 1.0,
            pitch: 1.0,
            seed: 42,
        }),
        &mut random,
        &mut effects,
    );

    assert!(leftover.is_none());
    assert_eq!(effects.positioned_sounds.len(), 1);
    assert_eq!(effects.positioned_sounds[0].seed, 42);
}

#[test]
fn apply_play_packet_orders_typed_explosion_effects_before_knockback() {
    let mut store = WorldStore::new();
    store.set_local_player_pose(LocalPlayerPoseState {
        delta_movement: Vec3d {
            x: 0.5,
            y: -0.25,
            z: 1.0,
        },
        ..LocalPlayerPoseState::default()
    });
    let mut random = LevelEventSoundRandomState::with_seed(12_345);
    let mut expected_random = LevelEventSoundRandomState::with_seed(12_345);
    let expected_pitch =
        (1.0 + (expected_random.next_float() - expected_random.next_float()) * 0.2) * 0.7;
    let expected_seed = expected_random.next_long();
    let mut effects = RecordingEffects::default();
    let packet = Explosion {
        center: Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        radius: 4.5,
        block_count: 7,
        player_knockback: Some(Vec3d {
            x: 0.25,
            y: -0.5,
            z: 1.5,
        }),
        explosion_particle: ParticlePayload {
            particle_type_id: VANILLA_PARTICLE_TYPE_EXPLOSION_ID,
            raw_options: Vec::new(),
        },
        explosion_sound: SoundEventHolder::Reference { registry_id: 41 },
        block_particles: vec![WeightedExplosionParticle {
            particle: ParticlePayload {
                particle_type_id: VANILLA_PARTICLE_TYPE_CLOUD_ID,
                raw_options: Vec::new(),
            },
            scaling: 0.5,
            speed: 1.25,
            weight: 3,
        }],
    };

    assert!(store
        .apply_play_packet(
            PlayClientbound::Explosion(packet.clone()),
            &mut random,
            &mut effects,
        )
        .is_none());

    assert_eq!(effects.positioned_sounds.len(), 1);
    assert_eq!(effects.positioned_sounds[0].position, packet.center);
    assert_eq!(effects.positioned_sounds[0].source, "block");
    assert_eq!(effects.positioned_sounds[0].volume, 4.0);
    assert_eq!(effects.positioned_sounds[0].pitch, expected_pitch);
    assert_eq!(effects.positioned_sounds[0].seed, expected_seed);
    assert!(!effects.positioned_sounds[0].distance_delay);
    assert_eq!(
        effects.explosions,
        vec![store.last_explosion().unwrap().clone()]
    );
    assert_eq!(
        effects.explosion_callback_delta_movements,
        vec![Some(Vec3d {
            x: 0.5,
            y: -0.25,
            z: 1.0,
        })]
    );
    assert_eq!(
        store.local_player_pose().unwrap().delta_movement,
        Vec3d {
            x: 0.75,
            y: -0.75,
            z: 2.5,
        }
    );
    let mut sinkless_store = WorldStore::new();
    let mut sinkless_random = LevelEventSoundRandomState::with_seed(12_345);
    assert!(sinkless_store
        .apply_play_packet(
            PlayClientbound::Explosion(packet),
            &mut sinkless_random,
            &mut NoPlayApplyEffects,
        )
        .is_none());
    assert_eq!(
        sinkless_store.last_sound(),
        store.last_sound(),
        "sink-less apply must retain the same canonical sound"
    );
    let expected_next = expected_random.next_float();
    assert_eq!(random.next_float(), expected_next);
    assert_eq!(sinkless_random.next_float(), expected_next);
}

#[test]
fn local_player_damage_event_resolves_reordered_type_and_forwards_hurt_sound() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        DAMAGE_TYPE_REGISTRY,
        0,
        vec![
            RegistryPacketEntry::with_raw_data(
                "example:frozen",
                damage_type_nbt("custom.freeze", "freezing"),
            ),
            RegistryPacketEntry::with_raw_data(
                "example:embers",
                damage_type_nbt("custom.fire", "burning"),
            ),
        ],
    );
    store.begin_registry_configuration();
    store.finish_registry_configuration().unwrap();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();
    let position = Vec3d {
        x: 3.0,
        y: 72.0,
        z: -5.0,
    };
    store.local_player_id = Some(71);
    store.set_local_player_pose(LocalPlayerPoseState {
        position,
        ..LocalPlayerPoseState::default()
    });

    for (source_type_id, expected_sound) in [
        (0, "minecraft:entity.player.hurt_freeze"),
        (1, "minecraft:entity.player.hurt_on_fire"),
    ] {
        let expected_pitch =
            (expected_random.next_float() - expected_random.next_float()) * 0.2 + 1.0;
        assert!(store
            .apply_play_packet(
                PlayClientbound::DamageEvent(DamageEvent {
                    entity_id: 71,
                    source_type_id,
                    source_cause_id: -1,
                    source_direct_id: -1,
                    source_position: None,
                }),
                &mut random,
                &mut effects,
            )
            .is_none());
        let sound = effects.positioned_sounds.last().unwrap();
        assert_eq!(sound.sound.location.as_deref(), Some(expected_sound));
        assert_eq!(sound.source, "player");
        assert_eq!(sound.position, position);
        assert_eq!(sound.volume, 1.0);
        assert_eq!(sound.pitch, expected_pitch);
        assert_eq!(sound.seed, 0);
        assert!(!sound.distance_delay);
    }
    assert_eq!(effects.positioned_sounds.len(), 2);
    assert_eq!(store.last_sound(), effects.positioned_sounds.last());
}

#[test]
fn remote_damage_event_mutates_state_without_client_owned_hurt_sound() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        DAMAGE_TYPE_REGISTRY,
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:damage",
            damage_type_nbt("custom.damage", "hurt"),
        )],
    );
    store.begin_registry_configuration();
    store.finish_registry_configuration().unwrap();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();
    let position = Vec3d {
        x: 3.0,
        y: 72.0,
        z: -5.0,
    };
    store.apply_play_packet(
        PlayClientbound::AddEntity(add_entity(72, VANILLA_ENTITY_TYPE_COW_ID, position)),
        &mut random,
        &mut effects,
    );

    assert!(store
        .apply_play_packet(
            PlayClientbound::DamageEvent(DamageEvent {
                entity_id: 72,
                source_type_id: 0,
                source_cause_id: -1,
                source_direct_id: -1,
                source_position: None,
            }),
            &mut random,
            &mut effects,
        )
        .is_none());

    let damage = store.entity_last_damage(72).unwrap();
    assert_eq!(damage.source_effects, Some(DamageEffects::Hurt));
    assert_eq!(
        damage.source_type_resource_id.as_deref(),
        Some("example:damage")
    );
    assert!(effects.positioned_sounds.is_empty());
    assert_eq!(store.last_sound(), None);
    assert_eq!(random.next_float(), expected_random.next_float());
}

#[test]
fn take_item_entity_forwards_pickup_sounds() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_item_pitch =
        (expected_random.next_float() - expected_random.next_float()) * 1.4 + 2.0;
    let expected_orb_pitch =
        (expected_random.next_float() - expected_random.next_float()) * 0.35 + 0.9;
    let expected_zombie_pitch =
        (expected_random.next_float() - expected_random.next_float()) * 1.4 + 2.0;
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            10,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            Vec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            20,
            VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
            Vec3d {
                x: 4.0,
                y: 70.0,
                z: 8.0,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            30,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: -6.0,
                y: 65.0,
                z: 9.0,
            },
        )),
        PlayClientbound::TakeItemEntity(TakeItemEntity {
            item_id: 10,
            player_id: 99,
            amount: 1,
        }),
        PlayClientbound::TakeItemEntity(TakeItemEntity {
            item_id: 20,
            player_id: 99,
            amount: 1,
        }),
        PlayClientbound::TakeItemEntity(TakeItemEntity {
            item_id: 30,
            player_id: 99,
            amount: 1,
        }),
        PlayClientbound::TakeItemEntity(TakeItemEntity {
            item_id: 404,
            player_id: 99,
            amount: 1,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.positioned_sounds.len(), 3);
    assert_eq!(
        effects.positioned_sounds[0].sound.location.as_deref(),
        Some("minecraft:entity.item.pickup")
    );
    assert_eq!(effects.positioned_sounds[0].source, "player");
    assert_eq!(
        effects.positioned_sounds[0].position,
        Vec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert_eq!(effects.positioned_sounds[0].volume, 0.2);
    assert_eq!(effects.positioned_sounds[0].pitch, expected_item_pitch);
    assert_eq!(
        effects.positioned_sounds[1].sound.location.as_deref(),
        Some("minecraft:entity.experience_orb.pickup")
    );
    assert_eq!(effects.positioned_sounds[1].source, "player");
    assert_eq!(
        effects.positioned_sounds[1].position,
        Vec3d {
            x: 4.0,
            y: 70.0,
            z: 8.0,
        }
    );
    assert_eq!(effects.positioned_sounds[1].volume, 0.1);
    assert_eq!(effects.positioned_sounds[1].pitch, expected_orb_pitch);
    assert_eq!(
        effects.positioned_sounds[2].sound.location.as_deref(),
        Some("minecraft:entity.item.pickup")
    );
    assert_eq!(effects.positioned_sounds[2].source, "player");
    assert_eq!(
        effects.positioned_sounds[2].position,
        Vec3d {
            x: -6.0,
            y: 65.0,
            z: 9.0,
        }
    );
    assert_eq!(effects.positioned_sounds[2].volume, 0.2);
    assert_eq!(effects.positioned_sounds[2].pitch, expected_zombie_pitch);
    for sound in &effects.positioned_sounds {
        assert_eq!(sound.seed, 0);
        assert!(!sound.distance_delay);
    }
    assert_eq!(store.last_sound(), Some(&effects.positioned_sounds[2]));
    assert_eq!(store.counters().take_item_entities_received, 4);
    assert_eq!(store.counters().take_item_entities_applied, 3);
    assert_eq!(store.counters().take_item_entities_ignored, 1);
    assert_eq!(store.counters().take_item_entities_removed, 2);
    assert_eq!(store.entity_count(), 1);
}

#[test]
fn game_events_forward_local_player_audio_and_particles() {
    let mut store = WorldStore::new();
    store.set_local_player_pose(LocalPlayerPoseState {
        position: Vec3d {
            x: 2.0,
            y: 64.0,
            z: -5.0,
        },
        ..LocalPlayerPoseState::default()
    });
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::GameEvent(GameEvent {
            event_id: VANILLA_CLIENT_GAME_EVENT_PLAY_ARROW_HIT_SOUND_ID,
            param: 0.0,
        }),
        PlayClientbound::GameEvent(GameEvent {
            event_id: VANILLA_CLIENT_GAME_EVENT_PUFFER_FISH_STING_ID,
            param: 0.0,
        }),
        PlayClientbound::GameEvent(GameEvent {
            event_id: VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID,
            param: 0.5,
        }),
        PlayClientbound::GameEvent(GameEvent {
            event_id: VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID,
            param: 1.5,
        }),
        PlayClientbound::GameEvent(GameEvent {
            event_id: VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID,
            param: 0.0,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.positioned_sounds.len(), 3);
    assert_eq!(
        effects.positioned_sounds[0].sound.location.as_deref(),
        Some("minecraft:entity.arrow.hit_player")
    );
    assert_eq!(effects.positioned_sounds[0].source, "player");
    assert_eq!(
        effects.positioned_sounds[0].position,
        Vec3d {
            x: 2.0,
            y: 65.62,
            z: -5.0,
        }
    );
    assert_eq!(effects.positioned_sounds[0].volume, 0.18);
    assert_eq!(effects.positioned_sounds[0].pitch, 0.45);
    assert_eq!(
        effects.positioned_sounds[1].sound.location.as_deref(),
        Some("minecraft:entity.puffer_fish.sting")
    );
    assert_eq!(effects.positioned_sounds[1].source, "neutral");
    assert_eq!(
        effects.positioned_sounds[1].position,
        Vec3d {
            x: 2.0,
            y: 64.0,
            z: -5.0,
        }
    );
    assert_eq!(
        effects.positioned_sounds[2].sound.location.as_deref(),
        Some("minecraft:entity.elder_guardian.curse")
    );
    assert_eq!(effects.positioned_sounds[2].source, "hostile");
    assert_eq!(
        effects.elder_guardian_effect_particles,
        vec![
            Vec3d {
                x: 2.0,
                y: 64.0,
                z: -5.0,
            },
            Vec3d {
                x: 2.0,
                y: 64.0,
                z: -5.0,
            },
            Vec3d {
                x: 2.0,
                y: 64.0,
                z: -5.0,
            },
        ]
    );
    assert_eq!(store.last_sound(), Some(&effects.positioned_sounds[2]));
    assert_eq!(store.counters().game_event_packets, 5);
}

#[test]
fn game_event_side_effects_require_local_player_pose() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for event_id in [
        VANILLA_CLIENT_GAME_EVENT_PLAY_ARROW_HIT_SOUND_ID,
        VANILLA_CLIENT_GAME_EVENT_PUFFER_FISH_STING_ID,
        VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID,
    ] {
        let leftover = store.apply_play_packet(
            PlayClientbound::GameEvent(GameEvent {
                event_id,
                param: 1.0,
            }),
            &mut random,
            &mut effects,
        );
        assert!(leftover.is_none());
    }

    assert!(effects.positioned_sounds.is_empty());
    assert!(effects.elder_guardian_effect_particles.is_empty());
    assert_eq!(store.counters().game_event_packets, 3);
}

#[test]
fn entity_animation_forwards_crit_tracking_emitters() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            10,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
        )),
        PlayClientbound::EntityAnimation(EntityAnimation { id: 10, action: 4 }),
        PlayClientbound::EntityAnimation(EntityAnimation { id: 10, action: 5 }),
        PlayClientbound::EntityAnimation(EntityAnimation { id: 10, action: 0 }),
        PlayClientbound::EntityAnimation(EntityAnimation { id: 404, action: 4 }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(
        effects.tracking_emitters,
        vec![
            (
                10,
                EntityTrackingEmitterParticleKind::Crit,
                TRACKING_EMITTER_DEFAULT_LIFETIME_TICKS,
            ),
            (
                10,
                EntityTrackingEmitterParticleKind::EnchantedHit,
                TRACKING_EMITTER_DEFAULT_LIFETIME_TICKS,
            ),
        ]
    );
    assert_eq!(store.counters().entity_animation_updates_applied, 3);
    assert_eq!(store.counters().entity_animation_updates_ignored, 1);
}

#[test]
fn totem_entity_event_forwards_positioned_use_sound_with_entity_source() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();
    record_packet_player_info(&mut store, 11);

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            10,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            11,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            Vec3d {
                x: 4.5,
                y: 70.0,
                z: 8.25,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 10,
            event_id: 35,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 11,
            event_id: 35,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: 35,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.positioned_sounds.len(), 2);
    assert_eq!(
        effects.positioned_sounds[0].sound.location.as_deref(),
        Some("minecraft:item.totem.use")
    );
    assert_eq!(effects.positioned_sounds[0].source, "hostile");
    assert_eq!(
        effects.positioned_sounds[0].position,
        Vec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert_eq!(effects.positioned_sounds[1].source, "player");
    assert_eq!(
        effects.positioned_sounds[1].position,
        Vec3d {
            x: 4.5,
            y: 70.0,
            z: 8.25,
        }
    );
    assert_eq!(effects.positioned_sounds[0].volume, 1.0);
    assert_eq!(effects.positioned_sounds[0].pitch, 1.0);
    assert_eq!(effects.positioned_sounds[0].seed, 0);
    assert!(!effects.positioned_sounds[0].distance_delay);
    assert_eq!(
        effects.tracking_emitters,
        vec![
            (
                10,
                EntityTrackingEmitterParticleKind::TotemOfUndying,
                TOTEM_TRACKING_EMITTER_LIFETIME_TICKS,
            ),
            (
                11,
                EntityTrackingEmitterParticleKind::TotemOfUndying,
                TOTEM_TRACKING_EMITTER_LIFETIME_TICKS,
            ),
        ]
    );
    assert_eq!(store.last_sound(), Some(&effects.positioned_sounds[1]));
    assert_eq!(store.counters().entity_events_applied, 2);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn listener_special_entity_events_drive_tickable_guardian_and_sniffer_sounds() {
    let guardian_attack_target_data_id = crate::vanilla_entity_metadata_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_GUARDIAN_ID,
        "DATA_ID_ATTACK_TARGET",
    )
    .expect("generated Guardian attack-target accessor must resolve")
    .data_id;
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();
    for packet in [
        PlayClientbound::AddEntity(add_entity(
            60,
            VANILLA_ENTITY_TYPE_GUARDIAN_ID,
            Vec3d {
                x: 1.0,
                y: 62.0,
                z: -3.0,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            61,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 5.0,
                y: 62.0,
                z: -3.0,
            },
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 60,
            values: vec![int_entity_data(guardian_attack_target_data_id, 61)],
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 60,
            event_id: GUARDIAN_ATTACK_SOUND_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            62,
            VANILLA_ENTITY_TYPE_SNIFFER_ID,
            Vec3d {
                x: 8.0,
                y: 63.0,
                z: 2.0,
            },
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 62,
            values: vec![sniffer_state_entity_data(
                crate::VANILLA_SNIFFER_STATE_DIGGING_ID,
            )],
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 62,
            event_id: SNIFFER_DIGGING_SOUND_EVENT_ID,
        }),
    ] {
        assert!(store
            .apply_play_packet(packet, &mut random, &mut effects)
            .is_none());
    }

    assert_eq!(effects.entity_sounds.len(), 2);
    let guardian = &effects.entity_sounds[0];
    assert_eq!(
        guardian.0.sound.location.as_deref(),
        Some("minecraft:entity.guardian.attack")
    );
    assert_eq!(guardian.0.source, "hostile");
    assert!(guardian.0.looping);
    assert!(!guardian.0.linear_attenuation);
    assert!(guardian.0.tickable);
    assert_eq!(guardian.0.volume, 0.0);
    assert_eq!(guardian.0.pitch, 0.7);
    assert_eq!(guardian.1, Some([1.0, 62.0, -3.0]));

    let sniffer = &effects.entity_sounds[1];
    assert_eq!(
        sniffer.0.sound.location.as_deref(),
        Some("minecraft:entity.sniffer.digging")
    );
    assert_eq!(sniffer.0.source, "neutral");
    assert!(!sniffer.0.looping);
    assert!(sniffer.0.linear_attenuation);
    assert!(sniffer.0.tickable);
    assert_eq!(sniffer.1, Some([8.0, 63.0, 2.0]));

    store.advance_entity_client_animations(4);
    let active = store.tickable_entity_sounds();
    assert_eq!(active.len(), 2);
    let guardian = active
        .iter()
        .find(|sound| sound.event.entity_id == 60)
        .unwrap();
    assert!((guardian.event.volume - 0.0025).abs() < 1.0e-6);
    assert!((guardian.event.pitch - 0.725).abs() < 1.0e-6);

    assert!(store.apply_set_entity_data(SetEntityData {
        id: 62,
        values: vec![sniffer_state_entity_data(
            crate::VANILLA_SNIFFER_STATE_RISING_ID,
        )],
    }));
    assert_eq!(store.tickable_entity_sounds().len(), 1);
    store.apply_remove_entities(RemoveEntities {
        entity_ids: vec![60],
    });
    assert!(store.tickable_entity_sounds().is_empty());
}

#[test]
fn attack_entity_events_keep_canonical_counters_without_local_sound() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            70,
            VANILLA_ENTITY_TYPE_RAVAGER_ID,
            Vec3d {
                x: 1.25,
                y: 64.0,
                z: -2.5,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 70,
            event_id: ATTACK_SOUND_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            71,
            VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
            Vec3d {
                x: 4.0,
                y: 65.0,
                z: -8.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 71,
            event_id: ATTACK_SOUND_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            72,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 7.0,
                y: 66.0,
                z: -9.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 72,
            event_id: ATTACK_SOUND_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            73,
            VANILLA_ENTITY_TYPE_RAVAGER_ID,
            Vec3d {
                x: 8.0,
                y: 67.0,
                z: -10.0,
            },
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 73,
            values: vec![silent_entity_data(true)],
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 73,
            event_id: ATTACK_SOUND_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: ATTACK_SOUND_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert!(effects.positioned_sounds.is_empty());
    assert_eq!(store.last_sound(), None);
    assert_eq!(store.counters().entity_events_applied, 4);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn evoker_fangs_attack_event_forwards_randomized_positioned_sound() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_pitch = expected_random.next_float() * 0.2 + 0.85;
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            74,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 7.0,
                y: 66.0,
                z: -9.0,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            75,
            VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID,
            Vec3d {
                x: 1.5,
                y: 64.0,
                z: -3.25,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            76,
            VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID,
            Vec3d {
                x: 4.0,
                y: 65.0,
                z: -8.0,
            },
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 76,
            values: vec![silent_entity_data(true)],
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 74,
            event_id: ATTACK_SOUND_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 76,
            event_id: ATTACK_SOUND_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 75,
            event_id: ATTACK_SOUND_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: ATTACK_SOUND_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.positioned_sounds.len(), 1);
    let sound = &effects.positioned_sounds[0];
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:entity.evoker_fangs.attack")
    );
    assert_eq!(sound.source, "neutral");
    assert_eq!(
        sound.position,
        Vec3d {
            x: 1.5,
            y: 64.0,
            z: -3.25,
        }
    );
    assert_eq!(sound.volume, 1.0);
    assert!((sound.pitch - expected_pitch).abs() < 1.0e-6);
    assert_eq!(sound.seed, 0);
    assert!(!sound.distance_delay);
    assert_eq!(store.last_sound(), Some(sound));
    assert_eq!(store.counters().entity_events_applied, 3);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn zombie_villager_cure_event_forwards_eye_positioned_sound() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_volume = 1.0 + expected_random.next_float();
    let expected_pitch = expected_random.next_float() * 0.7 + 0.3;
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            77,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 7.0,
                y: 66.0,
                z: -9.0,
            },
        )),
        PlayClientbound::AddEntity(add_entity(
            78,
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
            Vec3d {
                x: 4.0,
                y: 65.0,
                z: -8.0,
            },
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 78,
            values: vec![silent_entity_data(true)],
        }),
        PlayClientbound::AddEntity(add_entity(
            79,
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
            Vec3d {
                x: 1.5,
                y: 64.0,
                z: -3.25,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 77,
            event_id: ZOMBIE_VILLAGER_CURE_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 78,
            event_id: ZOMBIE_VILLAGER_CURE_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 79,
            event_id: ZOMBIE_VILLAGER_CURE_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: ZOMBIE_VILLAGER_CURE_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.positioned_sounds.len(), 1);
    let sound = &effects.positioned_sounds[0];
    let pose = store.probe_entity_camera_pose(79).unwrap();
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:entity.zombie_villager.cure")
    );
    assert_eq!(sound.source, "hostile");
    assert_eq!(
        sound.position,
        Vec3d {
            x: pose.position.x,
            y: pose.position.y + f64::from(pose.eye_height),
            z: pose.position.z,
        }
    );
    assert!((sound.volume - expected_volume).abs() < 1.0e-6);
    assert!((sound.pitch - expected_pitch).abs() < 1.0e-6);
    assert_eq!(sound.seed, 0);
    assert!(!sound.distance_delay);
    assert_eq!(store.last_sound(), Some(sound));
    assert_eq!(store.counters().entity_events_applied, 3);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn armadillo_peek_event_forwards_positioned_sound() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            80,
            VANILLA_ENTITY_TYPE_ARMADILLO_ID,
            Vec3d {
                x: 2.25,
                y: 63.0,
                z: -4.5,
            },
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 80,
            values: vec![silent_entity_data(true)],
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 80,
            event_id: ARMADILLO_PEEK_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            81,
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 9.0,
                y: 64.0,
                z: -3.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 81,
            event_id: ARMADILLO_PEEK_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: ARMADILLO_PEEK_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.positioned_sounds.len(), 1);
    let sound = &effects.positioned_sounds[0];
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:entity.armadillo.peek")
    );
    assert_eq!(sound.source, "neutral");
    assert_eq!(
        sound.position,
        Vec3d {
            x: 2.25,
            y: 63.0,
            z: -4.5,
        }
    );
    assert_eq!(sound.volume, 1.0);
    assert_eq!(sound.pitch, 1.0);
    assert_eq!(sound.seed, 0);
    assert!(!sound.distance_delay);
    assert_eq!(store.last_sound(), Some(sound));
    assert_eq!(store.counters().entity_events_applied, 2);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn armor_stand_hit_event_forwards_positioned_sound() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    for packet in [
        PlayClientbound::AddEntity(add_entity(
            82,
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            Vec3d {
                x: -1.0,
                y: 62.5,
                z: 6.75,
            },
        )),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 82,
            values: vec![silent_entity_data(true)],
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 82,
            event_id: ARMOR_STAND_HIT_EVENT_ID,
        }),
        PlayClientbound::AddEntity(add_entity(
            83,
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 9.0,
                y: 64.0,
                z: -3.0,
            },
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 83,
            event_id: ARMOR_STAND_HIT_EVENT_ID,
        }),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 404,
            event_id: ARMOR_STAND_HIT_EVENT_ID,
        }),
    ] {
        let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
        assert!(leftover.is_none());
    }

    assert_eq!(effects.positioned_sounds.len(), 1);
    let sound = &effects.positioned_sounds[0];
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:entity.armor_stand.hit")
    );
    assert_eq!(sound.source, "neutral");
    assert_eq!(
        sound.position,
        Vec3d {
            x: -1.0,
            y: 62.5,
            z: 6.75,
        }
    );
    assert_eq!(sound.volume, 0.3);
    assert_eq!(sound.pitch, 1.0);
    assert_eq!(sound.seed, 0);
    assert!(!sound.distance_delay);
    assert_eq!(store.last_sound(), Some(sound));
    assert_eq!(store.counters().entity_events_applied, 2);
    assert_eq!(store.counters().entity_events_ignored, 1);
}
