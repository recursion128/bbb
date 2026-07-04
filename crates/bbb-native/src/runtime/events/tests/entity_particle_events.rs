use super::*;

#[test]
fn entity_events_update_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(19);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity(123),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity(456),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityPositionSync(
        EntityPositionSync {
            id: 123,
            position: ProtocolVec3d {
                x: 2.0,
                y: 65.0,
                z: -3.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.25,
                z: 0.0,
            },
            y_rot: 180.0,
            x_rot: 30.0,
            on_ground: true,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::MoveEntity(EntityMove {
        id: 123,
        delta_x: 4096,
        delta_y: 0,
        delta_z: -2048,
        y_rot: Some(-90.0),
        x_rot: Some(45.0),
        on_ground: false,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::TeleportEntity(
        TeleportEntity {
            id: 123,
            position: ProtocolVec3d {
                x: 0.5,
                y: 70.0,
                z: -4.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.2,
                z: 0.0,
            },
            y_rot: 10.0,
            x_rot: -120.0,
            relatives_mask: 0,
            on_ground: true,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityPositionSync(
        EntityPositionSync {
            id: 999,
            position: ProtocolVec3d::default(),
            delta_movement: ProtocolVec3d::default(),
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::MoveEntity(EntityMove {
        id: 999,
        delta_x: 0,
        delta_y: 0,
        delta_z: 0,
        y_rot: None,
        x_rot: None,
        on_ground: false,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::TeleportEntity(
        TeleportEntity {
            id: 999,
            position: ProtocolVec3d::default(),
            delta_movement: ProtocolVec3d::default(),
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
            on_ground: false,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityMotion(
        SetEntityMotion {
            id: 123,
            delta_movement: ProtocolVec3d {
                x: 0.1,
                y: 0.0,
                z: -0.1,
            },
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::RotateHead(RotateHead {
        id: 123,
        y_head_rot: 90.0,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityAnimation(
        EntityAnimation { id: 123, action: 3 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 123,
        event_id: 35,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::HurtAnimation(
        HurtAnimation { id: 123, yaw: 45.5 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityData(
        SetEntityData {
            id: 123,
            values: vec![EntityDataValue {
                data_id: 0,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(0x20),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEquipment(
        SetEquipment {
            entity_id: 123,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: item_stack(42, 1),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::UpdateAttributes(
        UpdateAttributes {
            entity_id: 123,
            attributes: vec![AttributeSnapshot {
                attribute_id: 21,
                base: 20.0,
                modifiers: Vec::new(),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityLink(
        SetEntityLink {
            source_id: 123,
            dest_id: 456,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetPassengers(
        SetPassengers {
            vehicle_id: 123,
            passenger_ids: vec![456],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::RemoveEntities(
        RemoveEntities {
            entity_ids: vec![456, 999],
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        19
    );

    let world_counters = world.counters();
    assert_eq!(world_counters.entities_received, 2);
    assert_eq!(world_counters.entities_tracked, 1);
    assert_eq!(world_counters.entity_position_syncs_received, 2);
    assert_eq!(world_counters.entity_position_syncs_applied, 1);
    assert_eq!(world_counters.entity_position_syncs_ignored, 1);
    assert_eq!(world_counters.entity_moves_received, 2);
    assert_eq!(world_counters.entity_moves_applied, 1);
    assert_eq!(world_counters.entity_moves_ignored, 1);
    assert_eq!(world_counters.entity_teleports_received, 2);
    assert_eq!(world_counters.entity_teleports_applied, 1);
    assert_eq!(world_counters.entity_teleports_ignored, 1);
    assert_eq!(world_counters.entity_motion_updates_received, 1);
    assert_eq!(world_counters.entity_motion_updates_applied, 1);
    assert_eq!(world_counters.entity_head_rotations_received, 1);
    assert_eq!(world_counters.entity_head_rotations_applied, 1);
    assert_eq!(world_counters.entity_animation_updates_received, 1);
    assert_eq!(world_counters.entity_animation_updates_applied, 1);
    assert_eq!(world_counters.entity_events_received, 1);
    assert_eq!(world_counters.entity_events_applied, 1);
    assert_eq!(world_counters.entity_hurt_animations_received, 1);
    assert_eq!(world_counters.entity_hurt_animations_applied, 1);
    assert_eq!(world_counters.entity_data_updates_received, 1);
    assert_eq!(world_counters.entity_data_values_received, 1);
    assert_eq!(world_counters.entity_data_updates_applied, 1);
    assert_eq!(world_counters.entity_equipment_updates_received, 1);
    assert_eq!(world_counters.entity_equipment_slots_received, 1);
    assert_eq!(world_counters.entity_equipment_updates_applied, 1);
    assert_eq!(world_counters.entity_attribute_updates_received, 1);
    assert_eq!(world_counters.entity_attributes_received, 1);
    assert_eq!(world_counters.entity_attribute_updates_applied, 1);
    assert_eq!(world_counters.entity_link_updates_received, 1);
    assert_eq!(world_counters.entity_link_updates_applied, 1);
    assert_eq!(world_counters.entity_passenger_updates_received, 1);
    assert_eq!(world_counters.entity_passenger_ids_received, 1);
    assert_eq!(world_counters.entity_passenger_updates_applied, 1);
    assert_eq!(world_counters.entity_removes_received, 2);
    assert_eq!(world_counters.entities_removed, 1);
    assert_eq!(world_counters.entity_removes_ignored, 1);
}

#[test]
fn firework_entity_event_with_empty_explosions_emits_poof_particles() {
    let (tx, mut rx) = mpsc::channel(8);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(100, VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityData(
        firework_item_entity_data(100, Some(0)),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 100,
        event_id: 17,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(200, VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityData(
        firework_item_entity_data(200, Some(1)),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 200,
        event_id: 17,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(300, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 300,
        event_id: 17,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        8
    );

    assert_eq!(
        particles.firework_empty_explosion_positions,
        vec![[1.0, 64.0, -2.0]]
    );
    assert_eq!(
        particles.firework_empty_explosion_camera_positions,
        vec![None]
    );
    assert!(particles.firework_explosion_states.is_empty());
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().entity_events_applied, 3);
}

#[test]
fn firework_entity_event_with_explosions_emits_firework_particles() {
    let explosion = FireworkExplosionSummary {
        shape: FireworkExplosionShapeSummary::Star,
        colors: vec![0x112233, 0x445566],
        fade_colors: vec![0x778899],
        has_trail: true,
        has_twinkle: true,
    };
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(100, VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityMotion(
        SetEntityMotion {
            id: 100,
            delta_movement: ProtocolVec3d {
                x: 0.2,
                y: 0.4,
                z: -0.6,
            },
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityData(
        firework_item_entity_data_with_explosions(100, vec![explosion.clone()]),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 100,
        event_id: 17,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        4
    );

    assert!(particles.firework_empty_explosion_positions.is_empty());
    assert_eq!(particles.firework_explosion_states.len(), 1);
    assert_eq!(particles.firework_explosion_camera_positions, vec![None]);
    let state = &particles.firework_explosion_states[0];
    assert_eq!(state.entity_id, 100);
    assert_eq!(
        state.position,
        bbb_world::EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert_eq!(
        state.delta_movement,
        bbb_world::EntityVec3 {
            x: 0.2,
            y: 0.4,
            z: -0.6,
        }
    );
    assert!(state.has_explosions);
    assert_eq!(state.explosions, vec![explosion]);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().entity_events_applied, 1);
}

#[test]
fn totem_entity_event_emits_tracking_emitter_particles() {
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(123, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 123,
        event_id: 35,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 999,
        event_id: 35,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        3
    );

    assert_eq!(particles.tracking_emitter_states.len(), 1);
    let state = particles.tracking_emitter_states[0];
    assert_eq!(
        state.particle_type_id,
        crate::particle_runtime::TOTEM_OF_UNDYING_PARTICLE_TYPE_ID
    );
    assert_eq!(state.position, [1.0, 64.0, -2.0]);
    assert_close(state.width, 0.6);
    assert_close(state.height, 1.95);
    assert_eq!(state.lifetime_ticks, 30);
    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    match &audio.commands[0] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Hostile);
            assert_eq!(command.position, [1.0, 64.0, -2.0]);
            assert_eq!(command.packet_volume, 1.0);
            assert_eq!(command.packet_pitch, 1.0);
            assert_eq!(command.seed, 0);
            assert_eq!(command.fixed_range, None);
            assert_eq!(command.sound.event_id, "minecraft:item.totem.use");
            assert_eq!(command.sound.sound_name, "minecraft:item/totem/use");
        }
        other => panic!("expected totem positioned sound command, got {other:?}"),
    }
    assert_eq!(world.counters().entity_events_applied, 1);
    assert_eq!(world.counters().entity_events_ignored, 1);
}

#[test]
fn ravager_roar_entity_event_emits_poof_particle_state() {
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(77, VANILLA_ENTITY_TYPE_RAVAGER_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 77,
        event_id: 69,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(78, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 78,
        event_id: 69,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 999,
        event_id: 69,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        5
    );

    assert_eq!(particles.ravager_roar_states.len(), 1);
    let state = particles.ravager_roar_states[0];
    assert_eq!(state.entity_id, 77);
    assert_eq!(state.center.x, 1.0);
    assert_close64(state.center.y, 65.1);
    assert_eq!(state.center.z, -2.0);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().entity_events_applied, 2);
    assert_eq!(world.counters().entity_events_ignored, 1);
}

#[test]
fn witch_magic_entity_event_emits_particle_state() {
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(71, VANILLA_ENTITY_TYPE_WITCH_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 71,
        event_id: 15,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(72, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 72,
        event_id: 15,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 999,
        event_id: 15,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        5
    );

    assert_eq!(particles.witch_magic_states.len(), 1);
    let state = particles.witch_magic_states[0];
    assert_eq!(state.entity_id, 71);
    assert_eq!(state.position.x, 1.0);
    assert_eq!(state.position.y, 64.0);
    assert_eq!(state.position.z, -2.0);
    assert_close64(state.bounding_box_max_y, 65.95);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().entity_events_applied, 2);
    assert_eq!(world.counters().entity_events_ignored, 1);
}

#[test]
fn living_entity_poof_entity_event_emits_particle_state() {
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(73, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 73,
        event_id: 60,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(74, VANILLA_ENTITY_TYPE_ITEM_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 74,
        event_id: 60,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 999,
        event_id: 60,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        5
    );

    assert_eq!(particles.living_entity_poof_states.len(), 1);
    let state = particles.living_entity_poof_states[0];
    assert_eq!(state.entity_id, 73);
    assert_eq!(state.position.x, 1.0);
    assert_eq!(state.position.y, 64.0);
    assert_eq!(state.position.z, -2.0);
    assert_close(state.width, 0.6);
    assert_close(state.height, 1.95);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().entity_events_applied, 2);
    assert_eq!(world.counters().entity_events_ignored, 1);
}

#[test]
fn living_entity_drown_entity_event_emits_particle_state() {
    let (tx, mut rx) = mpsc::channel(6);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(75, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityMotion(
        SetEntityMotion {
            id: 75,
            delta_movement: ProtocolVec3d {
                x: 0.1,
                y: -0.2,
                z: 0.3,
            },
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 75,
        event_id: 67,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(76, VANILLA_ENTITY_TYPE_ITEM_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 76,
        event_id: 67,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 999,
        event_id: 67,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        6
    );

    assert_eq!(particles.living_entity_drown_states.len(), 1);
    let state = particles.living_entity_drown_states[0];
    assert_eq!(state.entity_id, 75);
    assert_eq!(state.position.x, 1.0);
    assert_eq!(state.position.y, 64.0);
    assert_eq!(state.position.z, -2.0);
    assert_eq!(state.delta_movement.x, 0.1);
    assert_eq!(state.delta_movement.y, -0.2);
    assert_eq!(state.delta_movement.z, 0.3);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().entity_events_applied, 2);
    assert_eq!(world.counters().entity_events_ignored, 1);
}

#[test]
fn living_entity_portal_entity_event_emits_particle_state() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(76, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(77, VANILLA_ENTITY_TYPE_ITEM_ID),
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        2
    );
    world.advance_entity_client_animations(1);

    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::Play(PlayClientbound::EntityPositionSync(
        EntityPositionSync {
            id: 76,
            position: ProtocolVec3d {
                x: 3.0,
                y: 65.0,
                z: -1.0,
            },
            delta_movement: ProtocolVec3d::default(),
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 76,
        event_id: 46,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 77,
        event_id: 46,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 999,
        event_id: 46,
    })))
    .unwrap();

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        4
    );

    assert_eq!(particles.living_entity_portal_states.len(), 1);
    let state = particles.living_entity_portal_states[0];
    assert_eq!(state.entity_id, 76);
    assert_eq!(state.previous_position.x, 1.0);
    assert_eq!(state.previous_position.y, 64.0);
    assert_eq!(state.previous_position.z, -2.0);
    assert_eq!(state.position.x, 3.0);
    assert_eq!(state.position.y, 65.0);
    assert_eq!(state.position.z, -1.0);
    assert_close(state.width, 0.6);
    assert_close(state.height, 1.95);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().entity_events_applied, 2);
    assert_eq!(world.counters().entity_events_ignored, 1);
}

#[test]
fn arrow_effect_clear_entity_event_emits_particle_state() {
    let (tx, mut rx) = mpsc::channel(13);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(118, VANILLA_ENTITY_TYPE_ARROW_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityData(
        SetEntityData {
            id: 118,
            values: vec![EntityDataValue {
                data_id: 11,
                serializer_id: 1,
                value: EntityDataValueKind::Int(0x0033_66cc),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(119, VANILLA_ENTITY_TYPE_ARROW_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityData(
        SetEntityData {
            id: 119,
            values: vec![EntityDataValue {
                data_id: 11,
                serializer_id: 1,
                value: EntityDataValueKind::Int(0),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(120, VANILLA_ENTITY_TYPE_ARROW_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityData(
        SetEntityData {
            id: 120,
            values: vec![EntityDataValue {
                data_id: 11,
                serializer_id: 1,
                value: EntityDataValueKind::Int(-1),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(121, VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityData(
        SetEntityData {
            id: 121,
            values: vec![EntityDataValue {
                data_id: 11,
                serializer_id: 1,
                value: EntityDataValueKind::Int(0x0011_2233),
            }],
        },
    )))
    .unwrap();
    for entity_id in [118, 119, 120, 121, 999] {
        tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
            entity_id,
            event_id: 0,
        })))
        .unwrap();
    }

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        13
    );

    assert_eq!(particles.arrow_effect_states.len(), 2);
    assert_eq!(particles.arrow_effect_states[0].entity_id, 118);
    assert_eq!(particles.arrow_effect_states[0].position.x, 1.0);
    assert_eq!(particles.arrow_effect_states[0].position.y, 64.0);
    assert_eq!(particles.arrow_effect_states[0].position.z, -2.0);
    assert_close(particles.arrow_effect_states[0].width, 0.5);
    assert_close(particles.arrow_effect_states[0].height, 0.5);
    assert_eq!(particles.arrow_effect_states[0].color_rgb, 0x0033_66cc);
    assert_eq!(particles.arrow_effect_states[1].entity_id, 119);
    assert_eq!(particles.arrow_effect_states[1].color_rgb, 0);
    assert_eq!(particles.batches.len(), 2);
    assert_eq!(world.counters().entity_events_applied, 4);
    assert_eq!(world.counters().entity_events_ignored, 1);
}

#[test]
fn love_entity_event_emits_animal_and_allay_particle_states() {
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(128, VANILLA_ENTITY_TYPE_COW_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(129, VANILLA_ENTITY_TYPE_ALLAY_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 128,
        event_id: 18,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 129,
        event_id: 18,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 999,
        event_id: 18,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        5
    );

    assert_eq!(particles.animal_love_states.len(), 1);
    let state = particles.animal_love_states[0];
    assert_eq!(state.entity_id, 128);
    assert_eq!(state.position.x, 1.0);
    assert_eq!(state.position.y, 64.0);
    assert_eq!(state.position.z, -2.0);
    assert_close(state.width, 0.9);
    assert_close(state.height, 1.4);
    assert_eq!(particles.allay_duplication_states.len(), 1);
    let state = particles.allay_duplication_states[0];
    assert_eq!(state.entity_id, 129);
    assert_eq!(state.position.x, 1.0);
    assert_eq!(state.position.y, 64.0);
    assert_eq!(state.position.z, -2.0);
    assert_close(state.width, 0.35);
    assert_close(state.height, 0.6);
    assert_eq!(particles.batches.len(), 2);
    assert_eq!(world.counters().entity_events_applied, 2);
    assert_eq!(world.counters().entity_events_ignored, 1);
}

#[test]
fn taming_entity_events_emit_tamable_and_horse_particle_states() {
    let (tx, mut rx) = mpsc::channel(7);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(130, VANILLA_ENTITY_TYPE_CAT_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(131, VANILLA_ENTITY_TYPE_HORSE_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(132, VANILLA_ENTITY_TYPE_COW_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 130,
        event_id: 7,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 131,
        event_id: 6,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 132,
        event_id: 7,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 999,
        event_id: 6,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        7
    );

    assert_eq!(particles.entity_taming_states.len(), 2);
    let state = particles.entity_taming_states[0];
    assert_eq!(state.entity_id, 130);
    assert!(state.success);
    assert_eq!(state.position.x, 1.0);
    assert_eq!(state.position.y, 64.0);
    assert_eq!(state.position.z, -2.0);
    assert_close(state.width, 0.6);
    assert_close(state.height, 0.7);
    let state = particles.entity_taming_states[1];
    assert_eq!(state.entity_id, 131);
    assert!(!state.success);
    assert_eq!(state.position.x, 1.0);
    assert_eq!(state.position.y, 64.0);
    assert_eq!(state.position.z, -2.0);
    assert_close(state.width, 1.396_484_4);
    assert_close(state.height, 1.6);
    assert_eq!(particles.batches.len(), 2);
    assert_eq!(world.counters().entity_events_applied, 3);
    assert_eq!(world.counters().entity_events_ignored, 1);
}

#[test]
fn villager_entity_events_emit_particle_states() {
    let (tx, mut rx) = mpsc::channel(8);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(133, VANILLA_ENTITY_TYPE_VILLAGER_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(134, VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID),
    )))
    .unwrap();
    for event_id in [12, 13, 14, 42] {
        tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 133,
            event_id,
        })))
        .unwrap();
    }
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 134,
        event_id: 12,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 999,
        event_id: 13,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        8
    );

    assert_eq!(particles.villager_states.len(), 4);
    assert_eq!(
        particles
            .villager_states
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
    let state = particles.villager_states[0];
    assert_eq!(state.entity_id, 133);
    assert_eq!(state.position.x, 1.0);
    assert_eq!(state.position.y, 64.0);
    assert_eq!(state.position.z, -2.0);
    assert_close(state.width, 0.6);
    assert_close(state.height, 1.95);
    assert_eq!(particles.batches.len(), 4);
    assert_eq!(world.counters().entity_events_applied, 5);
    assert_eq!(world.counters().entity_events_ignored, 1);
}

#[test]
fn dolphin_happy_entity_event_emits_particle_state() {
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(135, VANILLA_ENTITY_TYPE_DOLPHIN_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(136, VANILLA_ENTITY_TYPE_COW_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 135,
        event_id: 38,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 136,
        event_id: 38,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 999,
        event_id: 38,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        5
    );

    assert_eq!(particles.dolphin_happy_states.len(), 1);
    let state = particles.dolphin_happy_states[0];
    assert_eq!(state.entity_id, 135);
    assert_eq!(state.position.x, 1.0);
    assert_eq!(state.position.y, 64.0);
    assert_eq!(state.position.z, -2.0);
    assert_close(state.width, 0.9);
    assert_close(state.height, 0.6);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().entity_events_applied, 2);
    assert_eq!(world.counters().entity_events_ignored, 1);
}

#[test]
fn fox_eat_entity_event_emits_main_hand_item_particle_state() {
    let (tx, mut rx) = mpsc::channel(7);
    let mut fox = protocol_add_entity_with_type(137, VANILLA_ENTITY_TYPE_FOX_ID);
    fox.y_rot = 45.0;
    fox.x_rot = -30.0;
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(fox)))
        .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEquipment(
        SetEquipment {
            entity_id: 137,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: item_stack(42, 3),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(138, VANILLA_ENTITY_TYPE_COW_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEquipment(
        SetEquipment {
            entity_id: 138,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: item_stack(42, 1),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 137,
        event_id: 45,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 138,
        event_id: 45,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 999,
        event_id: 45,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        7
    );

    assert_eq!(particles.fox_eat_states.len(), 1);
    let state = &particles.fox_eat_states[0];
    assert_eq!(state.entity_id, 137);
    assert_eq!(state.position.x, 1.0);
    assert_eq!(state.position.y, 64.0);
    assert_eq!(state.position.z, -2.0);
    assert_eq!(state.y_rot, 45.0);
    assert_eq!(state.x_rot, -30.0);
    assert_eq!(state.item_stack, item_stack(42, 3));
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().entity_events_applied, 2);
    assert_eq!(world.counters().entity_events_ignored, 1);
}

#[test]
fn snowball_hit_entity_event_emits_particle_state() {
    let (tx, mut rx) = mpsc::channel(8);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(78, VANILLA_ENTITY_TYPE_SNOWBALL_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(79, VANILLA_ENTITY_TYPE_SNOWBALL_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityData(
        SetEntityData {
            id: 79,
            values: vec![EntityDataValue {
                data_id: 8,
                serializer_id: 7,
                value: EntityDataValueKind::ItemStack(ItemStackSummary::empty()),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(80, VANILLA_ENTITY_TYPE_ITEM_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 78,
        event_id: 3,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 79,
        event_id: 3,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 80,
        event_id: 3,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 999,
        event_id: 3,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        8
    );

    assert_eq!(particles.snowball_hit_states.len(), 2);
    assert_eq!(particles.snowball_hit_states[0].entity_id, 78);
    assert_eq!(particles.snowball_hit_states[0].position.x, 1.0);
    assert_eq!(particles.snowball_hit_states[0].position.y, 64.0);
    assert_eq!(particles.snowball_hit_states[0].position.z, -2.0);
    assert_eq!(
        particles.snowball_hit_states[0].item_stack,
        Some(item_stack(1017, 1))
    );
    assert_eq!(particles.snowball_hit_states[1].entity_id, 79);
    assert_eq!(particles.snowball_hit_states[1].item_stack, None);
    assert_eq!(particles.batches.len(), 2);
    assert_eq!(world.counters().entity_events_applied, 3);
    assert_eq!(world.counters().entity_events_ignored, 1);
}

#[test]
fn thrown_egg_hit_entity_event_emits_particle_state() {
    let (tx, mut rx) = mpsc::channel(8);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(88, VANILLA_ENTITY_TYPE_EGG_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(89, VANILLA_ENTITY_TYPE_EGG_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityData(
        SetEntityData {
            id: 89,
            values: vec![EntityDataValue {
                data_id: 8,
                serializer_id: 7,
                value: EntityDataValueKind::ItemStack(ItemStackSummary::empty()),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(90, VANILLA_ENTITY_TYPE_ITEM_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 88,
        event_id: 3,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 89,
        event_id: 3,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 90,
        event_id: 3,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 999,
        event_id: 3,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        8
    );

    assert_eq!(particles.thrown_egg_hit_states.len(), 1);
    assert_eq!(particles.thrown_egg_hit_states[0].entity_id, 88);
    assert_eq!(particles.thrown_egg_hit_states[0].position.x, 1.0);
    assert_eq!(particles.thrown_egg_hit_states[0].position.y, 64.0);
    assert_eq!(particles.thrown_egg_hit_states[0].position.z, -2.0);
    assert_eq!(
        particles.thrown_egg_hit_states[0].item_stack,
        item_stack(1032, 1)
    );
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().entity_events_applied, 3);
    assert_eq!(world.counters().entity_events_ignored, 1);
}

#[test]
fn honey_block_entity_events_emit_particle_state() {
    let (tx, mut rx) = mpsc::channel(6);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(76, VANILLA_ENTITY_TYPE_ITEM_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(77, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 76,
        event_id: 53,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 77,
        event_id: 54,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 76,
        event_id: 54,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 999,
        event_id: 53,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        6
    );

    let honey_block_state_id = vanilla_block_state_id("minecraft:honey_block", []);
    assert_eq!(particles.honey_block_states.len(), 2);
    assert_eq!(particles.honey_block_states[0].entity_id, 76);
    assert_eq!(particles.honey_block_states[0].count, 5);
    assert_eq!(
        particles.honey_block_states[0].block_state_id,
        honey_block_state_id
    );
    assert_eq!(particles.honey_block_states[0].position.x, 1.0);
    assert_eq!(particles.honey_block_states[0].position.y, 64.0);
    assert_eq!(particles.honey_block_states[0].position.z, -2.0);
    assert_eq!(particles.honey_block_states[1].entity_id, 77);
    assert_eq!(particles.honey_block_states[1].count, 10);
    assert_eq!(
        particles.honey_block_states[1].block_state_id,
        honey_block_state_id
    );
    assert_eq!(particles.batches.len(), 2);
    assert_eq!(world.counters().entity_events_applied, 3);
    assert_eq!(world.counters().entity_events_ignored, 1);
}

#[test]
fn entity_animation_emits_crit_tracking_emitter_particles() {
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(123, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityAnimation(
        EntityAnimation { id: 123, action: 4 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityAnimation(
        EntityAnimation { id: 123, action: 5 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityAnimation(
        EntityAnimation { id: 999, action: 4 },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        4
    );

    assert_eq!(particles.tracking_emitter_states.len(), 2);
    assert_eq!(
        particles.tracking_emitter_states[0].particle_type_id,
        crate::particle_runtime::CRIT_PARTICLE_TYPE_ID
    );
    assert_eq!(
        particles.tracking_emitter_states[1].particle_type_id,
        crate::particle_runtime::ENCHANTED_HIT_PARTICLE_TYPE_ID
    );
    for state in &particles.tracking_emitter_states {
        assert_eq!(state.position, [1.0, 64.0, -2.0]);
        assert_close(state.width, 0.6);
        assert_close(state.height, 1.95);
        assert_eq!(state.lifetime_ticks, 3);
    }
    assert_eq!(world.counters().entity_animation_updates_applied, 2);
    assert_eq!(world.counters().entity_animation_updates_ignored, 1);
}

#[test]
fn entity_events_materialize_ender_dragon_part_pick_targets() {
    const ENDER_DRAGON_TYPE_ID: i32 = 43;

    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(100, ENDER_DRAGON_TYPE_ID),
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    world.advance_entity_client_animations(1);

    assert_eq!(world.probe_entity_pick_bounds(100), None);
    let targets = world.entity_pick_targets();
    assert_eq!(
        targets
            .iter()
            .map(|target| target.entity_id)
            .collect::<Vec<_>>(),
        vec![101, 102, 103, 104, 105, 106, 107, 108]
    );
    assert_eq!(
        targets[0].bounds,
        bbb_world::EntityPickBoundsState::from_base_size(1.0, 1.0, 0.0)
    );
    assert_eq!(
        targets[2].bounds,
        bbb_world::EntityPickBoundsState::from_base_size(5.0, 3.0, 0.0)
    );
    assert_eq!(
        targets[6].bounds,
        bbb_world::EntityPickBoundsState::from_base_size(4.0, 2.0, 0.0)
    );
    assert_eq!(world.counters().entities_received, 1);
    assert_eq!(world.counters().entities_tracked, 1);
}
