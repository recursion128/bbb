use super::*;

#[test]
fn debug_passenger_vehicle_targets_follow_vanilla_boat_attachment_slab() {
    let mut store = WorldStore::new();
    store.apply_add_entity(ProtocolAddEntity {
        y_rot: 90.0,
        ..protocol_add_entity_with_type(10, VANILLA_ENTITY_TYPE_OAK_BOAT_ID)
    });
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        21,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20, 21],
    }));

    let front = store
        .entity_debug_passenger_vehicle_target(20)
        .expect("front passenger target");
    assert_entity_vec3_close(
        front.position,
        EntityVec3 {
            x: 0.6,
            y: 64.1875,
            z: -2.0,
        },
    );
    assert_pick_bounds_close(
        Some(front.bounds),
        EntityPickBoundsState::from_base_size(0.4, 0.0625, 0.0),
    );

    let rear = store
        .entity_debug_passenger_vehicle_target(21)
        .expect("rear passenger target");
    assert_entity_vec3_close(
        rear.position,
        EntityVec3 {
            x: 1.4,
            y: 64.1875,
            z: -2.0,
        },
    );
    assert_pick_bounds_close(
        Some(rear.bounds),
        EntityPickBoundsState::from_base_size(0.9, 0.0625, 0.0),
    );

    store.apply_add_entity(protocol_add_entity_with_type(
        11,
        VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        22,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 11,
        passenger_ids: vec![22],
    }));
    let raft = store
        .entity_debug_passenger_vehicle_target(22)
        .expect("raft passenger target");
    assert!((raft.position.y - 64.5).abs() < 1.0e-6);
    assert_eq!(store.entity_debug_passenger_vehicle_target(10), None);
}

#[test]
fn first_local_non_boat_mount_shows_overlay_without_changing_yaw() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.set_local_player_pose(LocalPlayerPoseState {
        y_rot: 12.0,
        ..LocalPlayerPoseState::default()
    });
    store.apply_add_entity(ProtocolAddEntity {
        y_rot: 75.0,
        ..protocol_add_entity_with_type(10, VANILLA_ENTITY_TYPE_MINECART_ID)
    });

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_pose().unwrap().y_rot, 12.0);
    assert_eq!(
        store.action_bar().unwrap().content.text(),
        "Press Left Shift to Dismount"
    );
}

#[test]
fn remote_boat_passenger_does_not_change_local_mount_presentation() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.set_local_player_pose(LocalPlayerPoseState {
        y_rot: 12.0,
        ..LocalPlayerPoseState::default()
    });
    store.apply_add_entity(ProtocolAddEntity {
        y_rot: 75.0,
        ..protocol_add_entity_with_type(10, VANILLA_ENTITY_TYPE_OAK_BOAT_ID)
    });
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20],
    }));
    assert_eq!(store.local_player_pose().unwrap().y_rot, 12.0);
    assert_eq!(store.action_bar(), None);
}

#[test]
fn local_player_root_boat_vehicle_id_tracks_vanilla_boat_roots() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        30,
        VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID,
    ));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_root_vehicle_id(), Some(10));
    assert_eq!(store.local_player_root_boat_vehicle_id(), None);

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_root_vehicle_id(), Some(20));
    assert_eq!(store.local_player_root_boat_vehicle_id(), Some(20));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 30,
        passenger_ids: vec![20],
    }));
    assert_eq!(store.local_player_root_vehicle_id(), Some(30));
    assert_eq!(store.local_player_root_boat_vehicle_id(), Some(30));
}

#[test]
fn local_boat_input_advances_root_boat_and_reports_move() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    let initial = store.probe_entity(20).unwrap();

    let report = store
        .advance_local_boat_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                right: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();

    assert_eq!(report.vehicle_id, 20);
    assert!(!report.snapped);
    assert_eq!(report.y_rot, initial.y_rot + 1.0);
    let yaw = report.y_rot * (std::f64::consts::PI / 180.0) as f32;
    let expected_x = initial.position.x
        + f64::from(super::super::super::client_movement::vanilla_mth_sin(-f64::from(yaw)) * 0.04);
    let expected_z = initial.position.z
        + f64::from(super::super::super::client_movement::vanilla_mth_cos(f64::from(yaw)) * 0.04);
    assert!((report.position.x - expected_x).abs() < 1.0e-12);
    assert!((report.position.z - expected_z).abs() < 1.0e-12);
    assert_eq!(report.position, store.probe_entity(20).unwrap().position);
    assert_eq!(store.local_player_root_boat_vehicle_id(), Some(20));
}

#[test]
fn local_boat_input_ignores_unfocused_controls_but_keeps_vanilla_physics() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    let initial = store.probe_entity(20).unwrap();

    let report = store
        .advance_local_boat_vehicle_input(
            crate::LocalPlayerInputState {
                focused: false,
                forward: true,
                right: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();

    assert_eq!(report.position.x, initial.position.x);
    assert_eq!(report.position.z, initial.position.z);
    assert!((report.position.y - (initial.position.y - 0.04)).abs() < 1.0e-12);
    assert_eq!(report.y_rot, initial.y_rot);
}

#[test]
fn local_boat_input_updates_generated_paddle_metadata() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    let accessors =
        super::super::super::schema::vanilla_entity_metadata_abstract_boat_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        )
        .expect("generated boat accessors");

    store
        .advance_local_boat_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    assert_eq!(
        store.entities.metadata_bool_for_entity(
            20,
            accessors.paddle_left_data_id,
            accessors.paddle_left_default,
        ),
        Some(true)
    );
    assert_eq!(
        store.entities.metadata_bool_for_entity(
            20,
            accessors.paddle_right_data_id,
            accessors.paddle_right_default,
        ),
        Some(true)
    );

    store
        .advance_local_boat_vehicle_input(crate::LocalPlayerInputState::default(), 1)
        .unwrap();
    assert_eq!(
        store.entities.metadata_bool_for_entity(
            20,
            accessors.paddle_left_data_id,
            accessors.paddle_left_default,
        ),
        Some(false)
    );
    assert_eq!(
        store.entities.metadata_bool_for_entity(
            20,
            accessors.paddle_right_data_id,
            accessors.paddle_right_default,
        ),
        Some(false)
    );
}

#[test]
fn local_boat_paddles_advance_after_control_without_a_client_paddle_sound() {
    const ADVANCE: f32 = std::f32::consts::PI / 8.0;
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: 1,
            y: -64,
            z: -2,
        },
        block_state_id: vanilla_block_state_id("minecraft:water", [("level", "0")]),
    }));
    let mut boat = protocol_add_entity_with_type(20, VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
    boat.position.y = -63.2;
    store.apply_add_entity(boat);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    let retained = store.publication_snapshot();
    let forward = crate::LocalPlayerInputState {
        focused: true,
        forward: true,
        ..crate::LocalPlayerInputState::default()
    };

    store.advance_entity_client_animations(1);
    store.advance_local_boat_vehicle_input(forward, 1).unwrap();
    let first = store.entity_model_source_at_partial_tick(20, 1.0).unwrap();
    assert!((first.render.kind.boat().rowing_time_left - ADVANCE).abs() < 1.0e-6);
    assert!((first.render.kind.boat().rowing_time_right - ADVANCE).abs() < 1.0e-6);

    store.advance_entity_client_animations(1);
    store.advance_local_boat_vehicle_input(forward, 1).unwrap();
    assert_eq!(store.last_sound(), None);

    let retained_boat = retained
        .entity_model_source_at_partial_tick(20, 1.0)
        .unwrap();
    assert_eq!(retained_boat.render.kind.boat().rowing_time_left, 0.0);
    assert_eq!(retained_boat.render.kind.boat().rowing_time_right, 0.0);
}

#[test]
fn remote_boat_tick_updates_status_and_discards_server_velocity() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: 1,
            y: -64,
            z: -2,
        },
        block_state_id: vanilla_block_state_id("minecraft:stone", []),
    }));
    let mut boat = protocol_add_entity_with_type(20, VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
    boat.position.y = -63.0;
    boat.delta_movement.x = 0.4;
    store.apply_add_entity(boat);
    store.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 10.0,
            y: -63.0,
            z: -2.0,
        },
        ..LocalPlayerPoseState::default()
    });
    let generation_a = store.publication_snapshot();

    store.advance_entity_client_animations(1);

    let transform = store.entities.transform(20).unwrap();
    let movement = store.entities.boat_movement(20).unwrap();
    assert_eq!(transform.delta_movement, EntityVec3::default());
    assert_eq!(movement.old_status, None);
    assert_eq!(
        movement.status,
        Some(super::super::super::EntityBoatStatus::OnLand)
    );
    assert!(movement.land_friction > 0.0);
    assert_eq!(
        generation_a.entities.boat_movement(20),
        Some(super::super::super::record::EntityBoatMovement::default())
    );
    assert_eq!(generation_a.probe_entity(20).unwrap().delta_movement.x, 0.4);
}

#[test]
fn remote_boat_pushes_only_the_live_local_player_generation() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    let mut local_player = protocol_add_entity_with_type(99, VANILLA_ENTITY_TYPE_PLAYER_ID);
    local_player.uuid = Uuid::from_u128(0x99);
    local_player.position.x = 1.5;
    store.apply_add_entity(local_player);
    store.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 1.5,
            y: 64.0,
            z: -2.0,
        },
        ..LocalPlayerPoseState::default()
    });
    let generation_a = store.publication_snapshot();

    store.advance_entity_client_animations(1);

    let distance = 0.5_f64.sqrt();
    let expected_push = (0.5 / distance) * f64::from(0.05_f32);
    let live_pose = store.local_player_pose().unwrap();
    let live_boat = store.entities.transform(20).unwrap();
    assert!((live_pose.delta_movement.x - expected_push).abs() < 1.0e-12);
    assert!((live_boat.delta_movement.x + expected_push).abs() < 1.0e-12);
    assert_eq!(live_pose.delta_movement.z, 0.0);
    assert_eq!(live_boat.delta_movement.z, 0.0);
    assert_eq!(
        generation_a.local_player_pose().unwrap().delta_movement,
        ProtocolVec3d::default()
    );
    assert_eq!(
        generation_a.probe_entity(20).unwrap().delta_movement,
        EntityVec3::default()
    );
    let generation_b = store.publication_snapshot();
    assert_eq!(generation_b.local_player_pose(), Some(live_pose));
    assert_eq!(
        generation_b.probe_entity(20).unwrap().delta_movement.x,
        live_boat.delta_movement.x
    );
}

#[test]
fn zero_client_ticks_do_not_advance_boat_physics_or_paddles() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    let before = store.publication_snapshot();

    store
        .advance_local_boat_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                right: true,
                ..crate::LocalPlayerInputState::default()
            },
            0,
        )
        .unwrap();

    assert_eq!(store.probe_entity(20), before.probe_entity(20));
    assert_eq!(
        store.entities.boat_movement(20),
        before.entities.boat_movement(20)
    );
}

#[test]
fn boat_physics_state_isolated_in_publication_and_reset_on_entity_recreation() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    let generation_a = store.publication_snapshot();

    store
        .advance_local_boat_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                right: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let generation_b = store.publication_snapshot();
    assert_eq!(
        generation_a.entities.boat_movement(20),
        Some(super::super::super::record::EntityBoatMovement::default())
    );
    let advanced = generation_b.entities.boat_movement(20).unwrap();
    assert_eq!(advanced.old_status, None);
    assert_eq!(
        advanced.status,
        Some(super::super::super::EntityBoatStatus::InAir)
    );
    assert_eq!(advanced.delta_rotation, 1.0);
    assert_ne!(
        generation_a.probe_entity(20).unwrap().position,
        generation_b.probe_entity(20).unwrap().position
    );

    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert_eq!(
        store.entities.boat_movement(20),
        Some(super::super::super::record::EntityBoatMovement::default())
    );
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    assert_eq!(store.entities.boat_movement(20), None);
}

#[test]
fn local_boat_uses_vanilla_land_friction_and_collision_response() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: 1,
            y: -64,
            z: -2
        },
        block_state_id: vanilla_block_state_id("minecraft:stone", []),
    }));
    let mut boat = protocol_add_entity_with_type(20, VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
    boat.position.y = -63.0;
    boat.delta_movement.x = 0.1;
    store.apply_add_entity(boat);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));

    let report = store
        .advance_local_boat_vehicle_input(crate::LocalPlayerInputState::default(), 1)
        .unwrap();
    let transform = store.entities.transform(20).unwrap();
    let movement = store.entities.boat_movement(20).unwrap();

    assert_eq!(movement.old_status, None);
    assert_eq!(
        movement.status,
        Some(super::super::super::EntityBoatStatus::OnLand)
    );
    assert_eq!(movement.land_friction, 0.3);
    assert!(
        (report.position.x - 1.06).abs() < 1.0e-8,
        "x: {}, delta: {:?}",
        report.position.x,
        transform.delta_movement
    );
    assert_eq!(report.position.y, -63.0);
    assert_eq!(transform.delta_movement.y, 0.0);
    assert_eq!(transform.on_ground, Some(true));
    assert_eq!(
        store.entities.supporting_block(20).unwrap().main,
        Some(crate::BlockPos {
            x: 1,
            y: -64,
            z: -2
        })
    );
}

#[test]
fn local_boat_first_water_tick_uses_buoyancy_without_air_transition_snap() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: 1,
            y: -64,
            z: -2
        },
        block_state_id: vanilla_block_state_id("minecraft:water", [("level", "0")]),
    }));
    let mut boat = protocol_add_entity_with_type(20, VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
    boat.position.y = -63.2;
    store.apply_add_entity(boat);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    let bounds = store.entities.pick_bounds(20).unwrap();
    let height = f64::from(bounds.max[1] - bounds.min[1]);
    let water_level = f64::from(-64.0_f32 + 8.0_f32 / 9.0_f32);
    let buoyancy = (water_level - -63.2) / height;
    let expected_delta_y = (-0.04 + buoyancy * (0.04 / 0.65)) * 0.75;

    let report = store
        .advance_local_boat_vehicle_input(crate::LocalPlayerInputState::default(), 1)
        .unwrap();
    let movement = store.entities.boat_movement(20).unwrap();
    let transform = store.entities.transform(20).unwrap();

    assert_eq!(movement.old_status, None);
    assert_eq!(
        movement.status,
        Some(super::super::super::EntityBoatStatus::InWater)
    );
    assert_eq!(movement.water_level, water_level);
    assert!((transform.delta_movement.y - expected_delta_y).abs() < 1.0e-12);
    assert!((report.position.y - (-63.2 + expected_delta_y)).abs() < 1.0e-12);
}

#[test]
fn local_boat_move_clips_against_generated_entity_colliders() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    let mut boat = protocol_add_entity_with_type_y_rot(20, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, 0.0);
    boat.delta_movement.z = 0.6;
    store.apply_add_entity(boat);
    store.apply_add_entity(protocol_add_entity_with_type(
        99,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    let mut cow = protocol_add_entity_with_type(30, VANILLA_ENTITY_TYPE_COW_ID);
    cow.position.z = -0.5;
    store.apply_add_entity(cow);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    let boat_before = store.entities.transform(20).unwrap();
    let boat_bounds = store.entities.pick_bounds(20).unwrap();
    let cow_transform = store.entities.transform(30).unwrap();
    let cow_bounds = store.entities.pick_bounds(30).unwrap();
    let expected_z = cow_transform.position.z + f64::from(cow_bounds.min[2])
        - (boat_before.position.z + f64::from(boat_bounds.max[2]));

    let report = store
        .advance_local_boat_vehicle_input(crate::LocalPlayerInputState::default(), 1)
        .unwrap();
    let transform = store.entities.transform(20).unwrap();

    assert!((report.position.z - (boat_before.position.z + expected_z)).abs() < 1.0e-12);
    assert_eq!(transform.delta_movement.z, 0.0);
    assert!(
        report.position.z + f64::from(boat_bounds.max[2])
            <= cow_transform.position.z + f64::from(cow_bounds.min[2])
    );
}

#[test]
fn local_boat_collision_excludes_spectator_players_and_same_vehicle_passengers() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    let mut boat = protocol_add_entity_with_type_y_rot(20, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, 0.0);
    boat.delta_movement.z = 0.6;
    store.apply_add_entity(boat);
    store.apply_add_entity(protocol_add_entity_with_type(
        99,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    let spectator_uuid = Uuid::from_u128(0x7788);
    let mut spectator = protocol_add_entity_with_type(30, VANILLA_ENTITY_TYPE_PLAYER_ID);
    spectator.uuid = spectator_uuid;
    spectator.position.z = -0.5;
    store.apply_add_entity(spectator);
    store.apply_player_info_update(ProtocolPlayerInfoUpdate {
        actions: vec![
            ProtocolPlayerInfoAction::AddPlayer,
            ProtocolPlayerInfoAction::UpdateGameMode,
        ],
        entries: vec![protocol_player_info_entry_with_mode(
            spectator_uuid,
            ProtocolGameType::Spectator,
        )],
    });
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    let before = store.entities.transform(20).unwrap();

    let report = store
        .advance_local_boat_vehicle_input(crate::LocalPlayerInputState::default(), 1)
        .unwrap();

    let expected_delta_z = 0.6 * f64::from(0.9_f32);
    assert!((report.position.z - (before.position.z + expected_delta_z)).abs() < 1.0e-12);
}

#[test]
fn local_boat_air_to_water_snap_no_collision_checks_entity_targets() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        30,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    let transform = store.entities.transform(20).unwrap();
    let bounds = store.entities.pick_bounds(20).unwrap();
    let targets = store.entities.vehicle_entity_collision_targets();

    assert!(!store.entity_collision_bounds_are_free(
        20,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        [
            transform.position.x,
            transform.position.y,
            transform.position.z,
        ],
        bounds.min.map(f64::from),
        bounds.max.map(f64::from),
        false,
        &targets,
    ));
}

#[test]
fn local_boat_move_clips_against_world_border_collision_shape() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_set_border_size(bbb_protocol::packets::SetBorderSize { size: 4.0 });
    let mut boat = protocol_add_entity_with_type_y_rot(20, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, 0.0);
    boat.delta_movement.x = 1.0;
    store.apply_add_entity(boat);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    let before = store.entities.transform(20).unwrap();
    let bounds = store.entities.pick_bounds(20).unwrap();
    let expected_position_x = 2.0 - f64::from(bounds.max[0]);

    let report = store
        .advance_local_boat_vehicle_input(crate::LocalPlayerInputState::default(), 1)
        .unwrap();
    let transform = store.entities.transform(20).unwrap();

    assert!((report.position.x - expected_position_x).abs() < 1.0e-12);
    assert!(report.position.x > before.position.x);
    assert_eq!(transform.delta_movement.x, 0.0);
    assert_eq!(report.position.x + f64::from(bounds.max[0]), 2.0);
}

#[test]
fn local_boat_travel_runs_two_freeze_passes_in_powder_snow() {
    let mut store = world_with_dimension_and_block("minecraft:powder_snow");
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: 8.5,
            y: 2.0,
            z: 8.5,
        },
        ..protocol_add_entity_with_type(20, VANILLA_ENTITY_TYPE_OAK_BOAT_ID)
    });
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));

    store
        .advance_local_boat_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();

    // `AbstractBoat.tick` applies `applyEffectsFromBlocks` twice after the
    // local move: each pass increments the synchronized freeze counter once.
    assert_eq!(store.entities.ticks_frozen(20), Some(2));
    assert!(
        store
            .entities
            .block_effects_debug_state(20)
            .expect("boat block effect state")
            .in_powder_snow
    );
}

#[test]
fn boat_leashed_to_happy_ghast_uses_four_taut_quad_ropes() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type_y_rot(
        62,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        0.0,
    ));
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: 0.0,
            y: 80.0,
            z: 0.0,
        },
        y_rot: 0.0,
        ..protocol_add_entity_with_type(
            63,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
        )
    });
    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 62,
        dest_id: 63,
    }));

    let states = store
        .entity_leash_render_states_at_partial_tick(62, 1.0)
        .expect("quad-leashed boat must project rope states");
    assert_eq!(states.len(), 4);
    assert!(states.iter().all(|state| !state.slack));
    // Boat quad offsets: `createQuadLeashOffsets(0, 0.64, 0.382, 0.88)` on the
    // 1.375 x 0.5625 oak boat box.
    let expected_y = 0.88 * 0.5625;
    assert!((states[0].offset[0] - -0.382 * 1.375).abs() < 1.0e-6);
    assert!((states[0].offset[1] - expected_y).abs() < 1.0e-6);
    assert!((states[0].offset[2] - 0.64 * 1.375).abs() < 1.0e-6);
    assert!((states[2].offset[0] - 0.382 * 1.375).abs() < 1.0e-6);
    assert!((states[2].offset[2] - -0.64 * 1.375).abs() < 1.0e-6);
    // Holder quad points: the `Entity.getQuadLeashHolderOffsets` default
    // `(±0.5 w, 0, ±0.5 w)` on the 4 x 4 happy ghast.
    assert!((states[0].end[0] - -2.0).abs() < 1.0e-6);
    assert!((states[0].end[1] - 80.0).abs() < 1.0e-6);
    assert!((states[0].end[2] - 2.0).abs() < 1.0e-6);
    assert!((states[3].end[0] - 2.0).abs() < 1.0e-6);
    assert!((states[3].end[2] - 2.0).abs() < 1.0e-6);
}
