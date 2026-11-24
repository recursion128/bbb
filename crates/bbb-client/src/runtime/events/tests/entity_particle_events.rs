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
        has_position: true,
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
        has_position: false,
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
            values: vec![entity_invisible_data()],
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
                attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
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
        firework_item_entity_data_with_explosions(100, vec![explosion]),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 100,
        event_id: 17,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = TestParticles::new();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            particles.target(),
            None,
            &mut level_event_sound_random,
        ),
        4
    );

    // `FireworkRocketEntity.handleEntityEvent(17)`: one flash plus the
    // star's sparks.
    let kinds = particles.live_kinds(&world);
    let count = |kind| kinds.iter().filter(|&&live| live == kind).count();
    assert_eq!(count(bbb_client_world::particles::ParticleKind::FLASH), 1);
    assert!(count(bbb_client_world::particles::ParticleKind::FIREWORK) > 0);
    assert_eq!(world.counters().entity_events_applied, 1);
}

#[test]
fn animate_player_wake_flows_from_network_event_to_sleep_overlay_projection() {
    let player = protocol_add_entity_with_type(99, VANILLA_ENTITY_TYPE_PLAYER_ID);
    let player_uuid = player.uuid;
    let entity_accessors =
        vanilla_entity_metadata_entity_accessors().expect("base entity metadata accessors");
    let living_accessors =
        bbb_client_world::vanilla_entity_metadata_living_entity_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
        )
        .expect("player living metadata accessors");
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::Login(protocol_play_login(
        99,
    ))))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerInfoUpdate(
        bbb_protocol::packets::PlayerInfoUpdate {
            actions: vec![bbb_protocol::packets::PlayerInfoAction::AddPlayer],
            entries: vec![bbb_protocol::packets::PlayerInfoEntry {
                profile_id: player_uuid,
                profile: Some(bbb_protocol::packets::GameProfile {
                    uuid: player_uuid,
                    name: "sleeping-player".to_string(),
                    properties: Vec::new(),
                }),
                listed: true,
                latency: 0,
                game_mode: bbb_protocol::game_types::GameType::default(),
                display_name: None,
                show_hat: true,
                list_order: 0,
                chat_session: None,
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(player)))
        .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityData(
        SetEntityData {
            id: 99,
            values: vec![
                EntityDataValue {
                    data_id: entity_accessors.pose_data_id,
                    serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
                    value: EntityDataValueKind::Pose(
                        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID,
                    ),
                },
                EntityDataValue {
                    data_id: living_accessors.sleeping_pos_data_id,
                    serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
                    value: EntityDataValueKind::OptionalBlockPos(Some(ProtocolBlockPos {
                        x: 33,
                        y: 70,
                        z: -17,
                    })),
                },
            ],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityAnimation(
        EntityAnimation { id: 99, action: 2 },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        5
    );

    assert_eq!(world.local_player_sleep_counter(), 100);
    assert_eq!(
        crate::runtime::hud_sleep_overlay_from_world(&world),
        Some(bbb_client_render::HudSleepOverlay { alpha: 220 })
    );
    let entity = world.probe_entity(99).expect("local player entity");
    assert_eq!(entity.player_sleep_counter, Some(100));
    assert!(entity.data_values.iter().any(|value| {
        value.data_id == entity_accessors.pose_data_id
            && value.value
                == EntityDataValueKind::Pose(
                    bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID,
                )
    }));
    assert!(entity.data_values.iter().any(|value| {
        value.data_id == living_accessors.sleeping_pos_data_id
            && value.value == EntityDataValueKind::OptionalBlockPos(None)
    }));
}

#[test]
fn entity_events_materialize_ender_dragon_part_pick_targets() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(100, VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID),
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
        bbb_client_world::EntityPickBoundsState::from_base_size(1.0, 1.0, 0.0)
    );
    assert_eq!(
        targets[2].bounds,
        bbb_client_world::EntityPickBoundsState::from_base_size(5.0, 3.0, 0.0)
    );
    assert_eq!(
        targets[6].bounds,
        bbb_client_world::EntityPickBoundsState::from_base_size(4.0, 2.0, 0.0)
    );
    assert_eq!(world.counters().entities_received, 1);
    assert_eq!(world.counters().entities_tracked, 1);
}
