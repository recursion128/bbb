use super::*;

#[test]
fn local_happy_ghast_input_uses_vanilla_ridden_rotation_and_flying_drag() {
    let mut store = locally_controlled_happy_ghast();
    store.set_local_player_pose(crate::LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        y_rot: 100.0,
        x_rot: 30.0,
        ..crate::LocalPlayerPoseState::default()
    });
    let before = store.entities.transform(10).unwrap();

    let report = store
        .advance_local_happy_ghast_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                left: true,
                jump: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let after = store.entities.transform(10).unwrap();

    assert_eq!(report.vehicle_id, 10);
    assert_eq!(report.position, after.position);
    assert!((after.y_rot - 26.4).abs() < 1.0e-6);
    assert_eq!(after.old_y_rot, after.y_rot);
    assert!((after.x_rot - 15.0).abs() < 1.0e-6);
    assert_eq!(after.y_head_rot, after.y_rot);
    assert!(after.position.x > before.position.x);
    assert!(after.position.z > before.position.z);
    assert!((after.position.y - before.position.y).abs() < 1.0e-6);
    assert!(
        (after.delta_movement.x - (after.position.x - before.position.x) * 0.91).abs() < 1.0e-8
    );
    assert!(
        (after.delta_movement.z - (after.position.z - before.position.z) * 0.91).abs() < 1.0e-8
    );
}

#[test]
fn local_happy_ghast_backward_pitch_and_focus_follow_controller_input() {
    let mut store = locally_controlled_happy_ghast();
    store.set_local_player_pose(crate::LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d::default(),
        x_rot: 30.0,
        ..crate::LocalPlayerPoseState::default()
    });
    let before = store.entities.transform(10).unwrap();

    store
        .advance_local_happy_ghast_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                backward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let backward = store.entities.transform(10).unwrap();
    assert!(backward.position.y > before.position.y);
    assert!(backward.position.z < before.position.z);

    let before_unfocused = backward;
    store
        .advance_local_happy_ghast_vehicle_input(
            crate::LocalPlayerInputState {
                focused: false,
                forward: true,
                jump: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let unfocused = store.entities.transform(10).unwrap();
    assert_eq!(
        unfocused.position,
        EntityVec3 {
            x: before_unfocused.position.x + before_unfocused.delta_movement.x,
            y: before_unfocused.position.y + before_unfocused.delta_movement.y,
            z: before_unfocused.position.z + before_unfocused.delta_movement.z,
        }
    );
}

#[test]
fn local_happy_ghast_movement_requires_exact_controller_gate() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    store.set_local_player_pose(crate::LocalPlayerPoseState::default());
    let input = crate::LocalPlayerInputState {
        focused: true,
        forward: true,
        ..crate::LocalPlayerInputState::default()
    };
    assert_eq!(
        store.advance_local_happy_ghast_vehicle_input(input, 1),
        None
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 10,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: item_stack(77, 1),
        }],
    }));
    assert!(store
        .advance_local_happy_ghast_vehicle_input(input, 1)
        .is_some());
    let accessors = vanilla_entity_metadata_happy_ghast_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    )
    .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![ProtocolEntityDataValue {
            data_id: accessors.stays_still_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert_eq!(
        store.advance_local_happy_ghast_vehicle_input(input, 1),
        None
    );
}

#[test]
fn local_happy_ghast_movement_isolates_retained_publication() {
    let mut store = locally_controlled_happy_ghast();
    let generation_a = store.publication_snapshot();
    let before = generation_a.probe_entity(10).unwrap();

    store
        .advance_local_happy_ghast_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let generation_b = store.publication_snapshot();

    assert_eq!(generation_a.probe_entity(10).unwrap(), before);
    assert_ne!(
        generation_b.probe_entity(10).unwrap().position,
        before.position
    );
}

#[test]
fn local_happy_ghast_move_clips_against_world_collision() {
    let mut store = locally_controlled_happy_ghast();
    store.insert_decoded_chunk(empty_test_chunk());
    store.entities.with_transform_mut(10, |transform| {
        transform.position.y = -63.0;
        transform.old_position.y = -63.0;
        transform.y_rot = 0.0;
        transform.old_y_rot = 0.0;
    });
    let bounds = store.entities.pick_bounds(10).unwrap();
    let before = store.entities.transform(10).unwrap();
    let wall_z = (before.position.z + f64::from(bounds.max[2])).floor() as i32;
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: before.position.x.floor() as i32,
            y: before.position.y.floor() as i32,
            z: wall_z,
        },
        block_state_id: vanilla_block_state_id("minecraft:stone", []),
    }));

    store
        .advance_local_happy_ghast_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let after = store.entities.transform(10).unwrap();

    assert_eq!(after.position.z, before.position.z);
    assert_eq!(after.delta_movement.z, 0.0);
    assert_eq!(after.on_ground, Some(false));
}

#[test]
fn local_happy_ghast_move_uses_vanilla_candidate_height_step_up() {
    let mut store = locally_controlled_happy_ghast();
    store.insert_decoded_chunk(empty_test_chunk());
    store.entities.with_transform_mut(10, |transform| {
        transform.position.y = -63.0;
        transform.old_position.y = -63.0;
        transform.y_rot = 0.0;
        transform.old_y_rot = 0.0;
        transform.on_ground = Some(true);
    });
    let bounds = store.entities.pick_bounds(10).unwrap();
    let before = store.entities.transform(10).unwrap();
    let wall_z = (before.position.z + f64::from(bounds.max[2])).floor() as i32;
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: before.position.x.floor() as i32,
            y: before.position.y.floor() as i32,
            z: wall_z,
        },
        block_state_id: vanilla_block_state_id("minecraft:stone", []),
    }));

    store
        .advance_local_happy_ghast_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let after = store.entities.transform(10).unwrap();

    assert!(after.position.z > before.position.z);
    assert!((after.position.y - (before.position.y + 1.0)).abs() < 1.0e-12);
}

#[test]
fn leash_endpoints_follow_the_generated_passenger_render_pose() {
    let mut store = WorldStore::new();
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: 40.0,
            y: 70.0,
            z: -12.0,
        },
        ..protocol_add_entity_with_type(66, VANILLA_ENTITY_TYPE_COW_ID)
    });
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: 2.0,
            y: 3.0,
            z: 4.0,
        },
        ..protocol_add_entity_with_type(67, VANILLA_ENTITY_TYPE_PIG_ID)
    });
    store.apply_add_entity(protocol_add_entity_with_type(
        68,
        VANILLA_ENTITY_TYPE_LEASH_KNOT_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 66,
        passenger_ids: vec![67],
    }));
    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 67,
        dest_id: 68,
    }));

    let rendered = store
        .entity_render_candidates_at_partial_tick(1.0)
        .into_iter()
        .find(|candidate| candidate.entity_id == 67)
        .expect("passenger render candidate")
        .position;
    let state = store
        .entity_leash_render_states_at_partial_tick(67, 1.0)
        .expect("leashed passenger rope")[0];
    assert!((state.start[0] - state.offset[0] - rendered.x).abs() < 1.0e-6);
    assert!((state.start[1] - state.offset[1] - rendered.y).abs() < 1.0e-6);
    assert!((state.start[2] - state.offset[2] - rendered.z).abs() < 1.0e-6);
    assert!(
        (rendered.x - 2.0).abs() > 1.0,
        "test must distinguish projected pose from packet transform: {rendered:?}"
    );
}

#[test]
fn player_holder_rope_hold_uses_arm_and_crouch_offsets() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type_y_rot(
        64,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        0.0,
    ));
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: 5.0,
            y: 64.0,
            z: 5.0,
        },
        y_rot: 0.0,
        ..protocol_add_entity_with_type(65, VANILLA_ENTITY_TYPE_PLAYER_ID)
    });
    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 64,
        dest_id: 65,
    }));

    let states = store
        .entity_leash_render_states_at_partial_tick(64, 1.0)
        .expect("player-held wolf must project a rope state");
    // Standing right-handed player: `(xOff=-0.22, bbHeight - 1.0, 0.07)` at
    // body yaw 0.
    assert!((states[0].end[0] - (5.0 - 0.22)).abs() < 1.0e-6);
    assert!((states[0].end[1] - (64.0 + 0.8)).abs() < 1.0e-6);
    assert!((states[0].end[2] - 5.07).abs() < 1.0e-6);

    // Crouching drops the z offset to -0.2.
    let accessors = vanilla_entity_metadata_entity_accessors().unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 65,
        values: vec![protocol_pose_data(
            accessors.pose_data_id,
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID,
        )],
    }));
    let states = store
        .entity_leash_render_states_at_partial_tick(64, 1.0)
        .expect("player-held wolf must keep its rope state");
    // The crouching pose also lowers the player box to 1.5.
    assert!((states[0].end[1] - (64.0 + 0.5)).abs() < 1.0e-6);
    assert!((states[0].end[2] - (5.0 - 0.2)).abs() < 1.0e-6);
}
