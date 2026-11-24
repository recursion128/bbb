use super::*;

#[test]
fn malformed_passenger_cycle_falls_back_to_packet_positions_within_the_loaded_entity_bound() {
    let mut store = WorldStore::new();
    for (id, x) in [(10, 10.0), (20, 20.0)] {
        store.apply_add_entity(ProtocolAddEntity {
            position: ProtocolVec3d { x, y: 64.0, z: 0.0 },
            ..protocol_add_entity_with_type(id, VANILLA_ENTITY_TYPE_COW_ID)
        });
    }
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20],
    }));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![10],
    }));

    for (id, x) in [(10, 10.0), (20, 20.0)] {
        assert_eq!(
            store
                .entity_model_source_at_partial_tick(id, 0.5)
                .expect("cyclic passenger source")
                .position,
            EntityVec3 { x, y: 64.0, z: 0.0 }
        );
    }
}

#[test]
fn shulker_mount_edges_reset_attach_position_interpolation() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_SHULKER_ID,
    ));
    store.advance_entity_client_animations(1);
    let initial = store.entities.transform(20).unwrap().position;
    let move_shulker = |store: &mut WorldStore, x: f64| {
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 20,
            position: ProtocolVec3d {
                x,
                y: initial.y,
                z: initial.z,
            },
            delta_movement: ProtocolVec3d::default(),
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        })
    };

    assert!(move_shulker(&mut store, initial.x + 3.0));
    assert_eq!(
        store
            .probe_entity(20)
            .unwrap()
            .client_animations
            .shulker_kind()
            .and_then(|k| k.teleport)
            .unwrap()
            .interpolation_ticks,
        6
    );
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20],
    }));
    assert_eq!(
        store
            .probe_entity(20)
            .unwrap()
            .client_animations
            .shulker_kind()
            .and_then(|k| k.teleport),
        None
    );

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![],
    }));
    let stopped = store
        .probe_entity(20)
        .unwrap()
        .client_animations
        .shulker_kind()
        .and_then(|k| k.teleport)
        .unwrap();
    assert_eq!(stopped.interpolation_ticks, 0);
    assert!(stopped.old_attach_position.is_some());

    assert!(move_shulker(&mut store, initial.x + 6.0));
    assert_eq!(
        store
            .probe_entity(20)
            .unwrap()
            .client_animations
            .shulker_kind()
            .and_then(|k| k.teleport)
            .unwrap()
            .interpolation_ticks,
        6
    );
}

#[test]
fn ordinary_living_push_dispatch_excludes_bat_armor_stand_and_parrot() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    for (entity_id, entity_type_id) in [
        (20, VANILLA_ENTITY_TYPE_BAT_ID),
        (21, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID),
        (22, VANILLA_ENTITY_TYPE_PARROT_ID),
        (23, VANILLA_ENTITY_TYPE_COW_ID),
    ] {
        let mut entity = protocol_add_entity_with_type(entity_id, entity_type_id);
        entity.uuid = Uuid::from_u128(entity_id as u128);
        store.apply_add_entity(entity);
    }
    let mut local_player = protocol_add_entity_with_type(99, VANILLA_ENTITY_TYPE_PLAYER_ID);
    local_player.uuid = Uuid::from_u128(0x99);
    local_player.position.x = 1.4;
    store.apply_add_entity(local_player);
    store.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 1.4,
            y: 64.0,
            z: -2.0,
        },
        ..LocalPlayerPoseState::default()
    });

    store.advance_entity_client_animations(1);

    let distance = 0.4_f64.sqrt();
    let expected_push = (0.4 / distance) * f64::from(0.05_f32);
    assert!(
        (store.local_player_pose().unwrap().delta_movement.x - expected_push).abs() < 1.0e-12,
        "exactly the generated Cow dispatch pushes the local player"
    );
}

#[test]
fn ordinary_living_push_reuses_team_and_local_target_gates() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    let cow_uuid = Uuid::from_u128(0x20);
    let mut cow = protocol_add_entity_with_type(20, VANILLA_ENTITY_TYPE_COW_ID);
    cow.uuid = cow_uuid;
    store.apply_add_entity(cow);
    let local_uuid = Uuid::from_u128(0x99);
    let mut local_player = protocol_add_entity_with_type(99, VANILLA_ENTITY_TYPE_PLAYER_ID);
    local_player.uuid = local_uuid;
    local_player.position.x = 1.4;
    store.apply_add_entity(local_player);
    store.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 1.4,
            y: 64.0,
            z: -2.0,
        },
        ..LocalPlayerPoseState::default()
    });
    assert!(store.apply_set_player_team(ProtocolSetPlayerTeam {
        name: "never-push".into(),
        method: ProtocolPlayerTeamMethod::Add.into(),
        parameters: Some(ProtocolPlayerTeamParameters {
            display_name: bbb_protocol::RichTextComponent::plain("Never push"),
            options: 0,
            nametag_visibility: ProtocolTeamVisibility::Always,
            collision_rule: ProtocolTeamCollisionRule::Never,
            color: ProtocolChatFormatting::Reset,
            player_prefix: bbb_protocol::RichTextComponent::plain(""),
            player_suffix: bbb_protocol::RichTextComponent::plain(""),
        }),
        players: vec![cow_uuid.to_string().into()],
    }));

    store.advance_entity_client_animations(1);
    assert_eq!(
        store.local_player_pose().unwrap().delta_movement,
        ProtocolVec3d::default()
    );

    store.apply_player_health(bbb_protocol::packets::PlayerHealth {
        health: 0.0,
        food: 20,
        saturation: 5.0,
    });
    assert!(store.apply_set_player_team(ProtocolSetPlayerTeam {
        name: "never-push".into(),
        method: ProtocolPlayerTeamMethod::Remove.into(),
        parameters: None,
        players: Vec::new(),
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(
        store.local_player_pose().unwrap().delta_movement,
        ProtocolVec3d::default(),
        "a dead local player remains excluded after the team gate is removed"
    );
}

#[test]
fn unpushable_warden_still_pushes_local_player_without_reciprocal_impulse() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_WARDEN_ID,
    ));
    let entity_accessors =
        super::super::schema::vanilla_entity_metadata_entity_accessors().unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 20,
        values: vec![protocol_pose_data(
            entity_accessors.pose_data_id,
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_DIGGING_ID,
        )],
    }));
    let mut local_player = protocol_add_entity_with_type(99, VANILLA_ENTITY_TYPE_PLAYER_ID);
    local_player.uuid = Uuid::from_u128(0x99);
    local_player.position.x = 1.4;
    store.apply_add_entity(local_player);
    store.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 1.4,
            y: 64.0,
            z: -2.0,
        },
        ..LocalPlayerPoseState::default()
    });

    store.advance_entity_client_animations(1);

    let distance = 0.4_f64.sqrt();
    let expected_push = (0.4 / distance) * f64::from(0.05_f32);
    assert!((store.local_player_pose().unwrap().delta_movement.x - expected_push).abs() < 1.0e-12);
    assert_eq!(store.entities.transform(20).unwrap().delta_movement.x, 0.0);
}

#[test]
fn generic_mob_controller_uses_no_ai_and_generated_non_controlling_rider_tag() {
    let mut no_ai = WorldStore::new();
    no_ai.set_test_equippable_defaults([(
        TEST_SADDLE_ITEM_ID,
        crate::item_profiles::test_equippable(EquipmentSlot::Saddle),
    )]);
    no_ai.apply_login(&protocol_play_login(99));
    no_ai.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));
    no_ai.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    equip_test_saddle(&mut no_ai, 20);
    assert!(no_ai.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    assert!(no_ai.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20],
    }));
    assert!(no_ai.entity_is_locally_authoritative(10));
    let mob_accessors =
        vanilla_entity_metadata_mob_accessors_for_entity_type(VANILLA_ENTITY_TYPE_ZOMBIE_ID)
            .unwrap();
    assert!(no_ai.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![ProtocolEntityDataValue {
            data_id: mob_accessors.flags_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(mob_accessors.no_ai_mask),
        }],
    }));
    assert!(!no_ai.entity_is_locally_authoritative(10));

    let mut tag = WorldStore::new();
    tag.set_test_equippable_defaults([(
        TEST_SADDLE_ITEM_ID,
        crate::item_profiles::test_equippable(EquipmentSlot::Saddle),
    )]);
    tag.apply_login(&protocol_play_login(99));
    for (id, entity_type_id) in [
        (10, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
        (20, VANILLA_ENTITY_TYPE_SLIME_ID),
        (30, VANILLA_ENTITY_TYPE_HORSE_ID),
    ] {
        tag.apply_add_entity(protocol_add_entity_with_type(id, entity_type_id));
    }
    equip_test_saddle(&mut tag, 30);
    assert!(tag.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 30,
        passenger_ids: vec![99],
    }));
    assert!(tag.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![30],
    }));
    assert!(tag.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20],
    }));
    assert!(tag.entity_is_locally_authoritative(20));
    assert!(!tag.entity_is_locally_authoritative(10));
}
