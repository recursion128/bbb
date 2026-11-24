use super::*;

#[test]
fn local_player_rideable_jumping_vehicle_id_tracks_vanilla_controlled_mounts() {
    let mut store = WorldStore::new();
    store.set_test_equippable_defaults([(
        TEST_SADDLE_ITEM_ID,
        crate::item_profiles::test_equippable(EquipmentSlot::Saddle),
    )]);
    store.apply_login(&protocol_play_login(99));
    store.apply_world_time(bbb_protocol::packets::PlayTime {
        game_time: 100,
        clock_updates: Vec::new(),
    });
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        30,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        40,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    assert_eq!(
        store.local_player_rideable_jumping_vehicle_id(),
        None,
        "vanilla PlayerRideableJumping.canJump requires a saddle"
    );
    equip_test_saddle(&mut store, 10);
    assert_eq!(store.local_player_rideable_jumping_vehicle_id(), Some(10));

    equip_test_saddle(&mut store, 20);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_rideable_jumping_vehicle_id(), Some(20));

    equip_test_saddle(&mut store, 30);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 30,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_rideable_jumping_vehicle_id(), Some(30));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 40,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_rideable_jumping_vehicle_id(), None);
}

#[test]
fn local_player_rideable_jumping_vehicle_requires_controlling_passenger() {
    let mut store = WorldStore::new();
    store.set_test_equippable_defaults([(
        TEST_SADDLE_ITEM_ID,
        crate::item_profiles::test_equippable(EquipmentSlot::Saddle),
    )]);
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        123,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    equip_test_saddle(&mut store, 10);

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![123, 99],
    }));

    assert_eq!(store.local_player_vehicle_id(), Some(10));
    assert_eq!(store.local_player_rideable_jumping_vehicle_id(), None);
}

#[test]
fn local_player_sprintable_vehicle_requires_controlling_passenger() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        123,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![123, 99],
    }));

    assert_eq!(store.local_player_vehicle_id(), Some(10));
    assert_eq!(store.local_player_sprintable_vehicle_id(), None);
}

#[test]
fn local_abstract_horse_input_uses_ridden_rotation_air_travel_and_gravity() {
    let mut store = locally_controlled_horse();
    let body_before = store
        .entity_model_source_at_partial_tick(10, 1.0)
        .unwrap()
        .y_rot;
    store.set_local_player_pose(crate::LocalPlayerPoseState {
        y_rot: 90.0,
        x_rot: 30.0,
        ..crate::LocalPlayerPoseState::default()
    });
    let before = store.entities.transform(10).unwrap();

    let report = store
        .advance_local_abstract_horse_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let after = store.entities.transform(10).unwrap();

    assert_eq!(report.vehicle_id, 10);
    assert_eq!(report.position, after.position);
    assert_eq!(after.y_rot, 90.0);
    assert_eq!(after.old_y_rot, 90.0);
    assert_eq!(after.x_rot, 15.0);
    assert_eq!(after.y_head_rot, 90.0);
    assert_eq!(
        store
            .entity_model_source_at_partial_tick(10, 0.0)
            .unwrap()
            .y_rot,
        body_before
    );
    assert_eq!(
        store
            .entity_model_source_at_partial_tick(10, 1.0)
            .unwrap()
            .y_rot,
        90.0
    );
    assert!(after.position.x < before.position.x);
    assert!((after.position.z - before.position.z).abs() < 1.0e-7);
    assert_eq!(after.position.y, before.position.y);
    assert!((after.delta_movement.y - -0.08 * f64::from(0.98_f32)).abs() < 1.0e-12);
}

#[test]
fn local_abstract_horse_standing_gate_and_controller_gate_match_vanilla() {
    let mut store = locally_controlled_horse();
    store.entities.with_transform_mut(10, |transform| {
        transform.on_ground = Some(true);
    });
    let accessors = vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_HORSE_ID,
    )
    .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![ProtocolEntityDataValue {
            data_id: accessors.flags_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(accessors.standing_mask),
        }],
    }));
    let before = store.entities.transform(10).unwrap();
    store
        .advance_local_abstract_horse_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let after = store.entities.transform(10).unwrap();
    assert_eq!(after.position.x, before.position.x);
    assert_eq!(after.position.z, before.position.z);

    let mut unsaddled = WorldStore::new();
    unsaddled.apply_login(&protocol_play_login(99));
    unsaddled.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    assert!(unsaddled.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    unsaddled.set_local_player_pose(crate::LocalPlayerPoseState::default());
    assert_eq!(
        unsaddled.advance_local_abstract_horse_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        ),
        None
    );
}

#[test]
fn local_abstract_horse_movement_isolates_retained_publication() {
    let mut store = locally_controlled_horse();
    let generation_a = store.publication_snapshot();
    let before = generation_a.probe_entity(10).unwrap();

    store
        .advance_local_abstract_horse_vehicle_input(
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
fn zero_riding_jump_still_uses_vanilla_minimum_horse_scale() {
    let mut store = locally_controlled_horse();
    assert!(store.apply_local_player_riding_jump(0));
    let state = store.entities.ridden_jump_state(10).unwrap();
    assert_eq!(state.player_jump_pending_scale, 0.4);
    assert!(state.allow_stand_sliding);
}

#[test]
fn recreated_horse_id_does_not_retain_local_riding_jump_state() {
    let mut store = locally_controlled_horse();
    assert!(store.apply_local_player_riding_jump(90));
    assert_eq!(
        store
            .entities
            .ridden_jump_state(10)
            .unwrap()
            .player_jump_pending_scale,
        1.0
    );

    assert_eq!(
        store.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![10],
        }),
        1
    );
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    assert_eq!(
        store.entities.ridden_jump_state(10).unwrap(),
        super::super::super::animations::RiddenJumpState::default()
    );
}

#[test]
fn local_pig_boost_uses_vanilla_post_increment_and_isolates_old_generation() {
    let (boost_data_id, _) =
        vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PIG_ID,
        )
        .unwrap();
    let mut boosted = locally_controlled_item_based_steering(
        VANILLA_ENTITY_TYPE_PIG_ID,
        VANILLA_ITEM_CARROT_ON_A_STICK_ID,
    );
    assert!(boosted.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![protocol_int_data(boost_data_id, 4)],
    }));
    assert_eq!(
        boosted.entities.item_based_steering_debug_state(10),
        Some((true, 0))
    );
    let generation_a = boosted.publication_snapshot();
    let before = boosted.entities.transform(10).unwrap();

    let mut steady = locally_controlled_item_based_steering(
        VANILLA_ENTITY_TYPE_PIG_ID,
        VANILLA_ITEM_CARROT_ON_A_STICK_ID,
    );
    boosted
        .advance_local_item_based_steering_vehicle_input(1)
        .unwrap();
    steady
        .advance_local_item_based_steering_vehicle_input(1)
        .unwrap();
    let after_boosted = boosted.entities.transform(10).unwrap();
    let after_steady = steady.entities.transform(10).unwrap();
    assert!(after_boosted.position.z > after_steady.position.z);
    assert_eq!(
        boosted.entities.item_based_steering_debug_state(10),
        Some((true, 1))
    );

    boosted
        .advance_local_item_based_steering_vehicle_input(4)
        .unwrap();
    assert_eq!(
        boosted.entities.item_based_steering_debug_state(10),
        Some((true, 5))
    );
    boosted
        .advance_local_item_based_steering_vehicle_input(1)
        .unwrap();
    assert_eq!(
        boosted.entities.item_based_steering_debug_state(10),
        Some((false, 6))
    );

    assert_eq!(generation_a.entities.transform(10), Some(before));
    assert_eq!(
        generation_a.entities.item_based_steering_debug_state(10),
        Some((true, 0))
    );
}

#[test]
fn local_strider_tick_recomputes_suffocating_modifier_from_lava() {
    let mut cold = locally_controlled_item_based_steering(
        VANILLA_ENTITY_TYPE_STRIDER_ID,
        VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID,
    );
    let (suffocating_data_id, _) =
        vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_STRIDER_ID,
        )
        .unwrap();
    let generation_a = cold.publication_snapshot();
    let warm_speed = cold
        .entities
        .attribute_value(
            10,
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
        )
        .unwrap();

    cold.advance_local_item_based_steering_vehicle_input(1)
        .unwrap();
    assert!(cold
        .entities
        .metadata_bool_for_entity(10, suffocating_data_id, false)
        .unwrap());
    let cold_speed = cold
        .entities
        .attribute_value(
            10,
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
        )
        .unwrap();
    assert!((cold_speed - warm_speed * 0.66).abs() < 1.0e-6);
    assert!(!generation_a
        .entities
        .metadata_bool_for_entity(10, suffocating_data_id, false)
        .unwrap());

    let mut warm = locally_controlled_item_based_steering(
        VANILLA_ENTITY_TYPE_STRIDER_ID,
        VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID,
    );
    warm.insert_decoded_chunk(empty_test_chunk());
    assert!(warm.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 1, y: -63, z: 1 },
        block_state_id: vanilla_block_state_id("minecraft:lava", [("level", "0")]),
    }));
    warm.entities.with_transform_mut(10, |transform| {
        transform.position = EntityVec3 {
            x: 1.0,
            y: -63.0,
            z: 1.0,
        };
        transform.old_position = transform.position;
        transform.on_ground = Some(false);
    });
    assert!(warm.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![protocol_bool_data(suffocating_data_id, true)],
    }));
    warm.advance_local_item_based_steering_vehicle_input(1)
        .unwrap();
    assert!(!warm
        .entities
        .metadata_bool_for_entity(10, suffocating_data_id, false)
        .unwrap());
    assert!(warm.entities.transform(10).unwrap().delta_movement.y > 0.0);
    assert!(
        (warm
            .entities
            .attribute_value(
                10,
                bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
            )
            .unwrap()
            - warm_speed)
            .abs()
            < 1.0e-6
    );
}

#[test]
fn local_strider_source_lava_surface_is_a_stable_half_block_collision() {
    let mut store = locally_controlled_item_based_steering(
        VANILLA_ENTITY_TYPE_STRIDER_ID,
        VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID,
    );
    store.insert_decoded_chunk(empty_test_chunk());
    let lava_source = vanilla_block_state_id("minecraft:lava", [("level", "0")]);
    for z in 1..=7 {
        assert!(store.apply_block_update(ProtocolBlockUpdate {
            pos: ProtocolBlockPos { x: 1, y: -64, z },
            block_state_id: lava_source,
        }));
    }
    store.entities.with_transform_mut(10, |transform| {
        transform.position = EntityVec3 {
            x: 1.0,
            y: -63.5,
            z: 1.0,
        };
        transform.old_position = transform.position;
        transform.delta_movement = EntityVec3::default();
        transform.on_ground = Some(true);
    });

    let initial_y = store.entities.transform(10).unwrap().position.y;
    store
        .advance_local_item_based_steering_vehicle_input(20)
        .unwrap();
    let after = store.entities.transform(10).unwrap();
    assert!((after.position.y - initial_y).abs() < 1.0e-9);
    assert!(
        after.position.z > 3.0,
        "standing on lava must bypass travelInLava and travel at ground speed, got z={}",
        after.position.z
    );
    assert!(after.on_ground.unwrap_or(false));
    assert!(after.delta_movement.y < 0.0);
}

#[test]
fn remote_pig_boost_uses_equipped_controller_hand_and_rotation() {
    let mut store = WorldStore::new();
    let mut pig = protocol_add_entity_with_type(10, VANILLA_ENTITY_TYPE_PIG_ID);
    pig.uuid = Uuid::from_u128(10);
    pig.y_head_rot = 0.0;
    store.apply_add_entity(pig);
    let mut controller = protocol_add_entity_with_type(20, VANILLA_ENTITY_TYPE_PLAYER_ID);
    controller.uuid = Uuid::from_u128(20);
    controller.x_rot = 24.0;
    controller.y_rot = 70.0;
    store.apply_add_entity(controller);
    store.set_test_equippable_defaults([(
        TEST_SADDLE_ITEM_ID,
        crate::item_profiles::test_equippable(EquipmentSlot::Saddle),
    )]);
    equip_test_saddle(&mut store, 10);
    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 20,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item: item_stack(VANILLA_ITEM_CARROT_ON_A_STICK_ID, 1),
        }],
    }));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20],
    }));
    let (boost_data_id, _) =
        vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PIG_ID,
        )
        .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![protocol_int_data(boost_data_id, 4)],
    }));

    store.advance_entity_client_animations(1);
    let transform = store.entities.transform(10).unwrap();
    assert!((transform.x_rot - 12.0).abs() < 1.0e-6);
    assert!((transform.y_rot - 70.0).abs() < 1.0e-6);
    assert!((transform.old_y_rot - 70.0).abs() < 1.0e-6);
    assert!((transform.y_head_rot - 70.0).abs() < 1.0e-6);
    assert_eq!(
        store
            .entity_model_source_at_partial_tick(10, 0.0)
            .unwrap()
            .y_rot,
        0.0
    );
    assert_eq!(
        store
            .entity_model_source_at_partial_tick(10, 1.0)
            .unwrap()
            .y_rot,
        70.0
    );
    assert_eq!(
        store.entities.item_based_steering_debug_state(10),
        Some((true, 1))
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 20,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item: item_stack(0, 0),
        }],
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(
        store.entities.item_based_steering_debug_state(10),
        Some((true, 1))
    );
}

#[test]
fn remote_strider_tick_updates_suffocation_before_passenger_tick() {
    let mut store = WorldStore::new();
    for id in [10, 20] {
        let mut strider = protocol_add_entity_with_type(id, VANILLA_ENTITY_TYPE_STRIDER_ID);
        strider.uuid = Uuid::from_u128(id as u128);
        store.apply_add_entity(strider);
    }
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20],
    }));
    let (suffocating_data_id, _) =
        vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_STRIDER_ID,
        )
        .unwrap();
    let generation_a = store.publication_snapshot();

    store.advance_entity_client_animations(1);

    for id in [10, 20] {
        assert!(store
            .entities
            .metadata_bool_for_entity(id, suffocating_data_id, false)
            .unwrap());
        assert!(!generation_a
            .entities
            .metadata_bool_for_entity(id, suffocating_data_id, false)
            .unwrap());
    }
}

#[test]
fn recreated_item_based_steering_id_does_not_retain_boost_cursor() {
    let mut store = locally_controlled_item_based_steering(
        VANILLA_ENTITY_TYPE_PIG_ID,
        VANILLA_ITEM_CARROT_ON_A_STICK_ID,
    );
    let (boost_data_id, _) =
        vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PIG_ID,
        )
        .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![protocol_int_data(boost_data_id, 4)],
    }));
    store
        .advance_local_item_based_steering_vehicle_input(1)
        .unwrap();
    let generation_a = store.publication_snapshot();

    assert_eq!(
        store.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![10],
        }),
        1
    );
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_PIG_ID,
    ));
    assert_eq!(
        store.entities.item_based_steering_debug_state(10),
        Some((false, 0))
    );
    assert_eq!(
        generation_a.entities.item_based_steering_debug_state(10),
        Some((true, 1))
    );
}

#[test]
fn local_abstract_horse_water_travel_uses_generated_ridden_float_tag() {
    let mut store = locally_controlled_horse();
    store.insert_decoded_chunk(empty_test_chunk());
    store.entities.with_transform_mut(10, |transform| {
        transform.position = EntityVec3 {
            x: 1.0,
            y: -63.0,
            z: 1.0,
        };
        transform.old_position = transform.position;
    });
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 1, y: -63, z: 1 },
        block_state_id: vanilla_block_state_id("minecraft:water", [("level", "0")]),
    }));

    store
        .advance_local_abstract_horse_vehicle_input(crate::LocalPlayerInputState::default(), 1)
        .unwrap();
    let after = store.entities.transform(10).unwrap();

    assert!(after.delta_movement.y > 0.03);
    assert!(
        bbb_protocol::entity_types::vanilla_entity_type_can_float_while_ridden(
            VANILLA_ENTITY_TYPE_HORSE_ID,
        )
    );
}

#[test]
fn local_abstract_horse_uses_step_height_attribute_for_one_block_step() {
    let mut store = locally_controlled_horse();
    store.insert_decoded_chunk(empty_test_chunk());
    let bounds = store.entities.pick_bounds(10).unwrap();
    store.entities.with_transform_mut(10, |transform| {
        transform.position = EntityVec3 {
            x: 1.0,
            y: -63.0,
            z: 2.0 - f64::from(bounds.max[2]),
        };
        transform.old_position = transform.position;
        transform.y_rot = 0.0;
        transform.old_y_rot = 0.0;
        transform.on_ground = Some(true);
    });
    let before = store.entities.transform(10).unwrap();
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 1, y: -63, z: 2 },
        block_state_id: vanilla_block_state_id("minecraft:stone", []),
    }));

    store
        .advance_local_abstract_horse_vehicle_input(
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
fn local_horse_travel_applies_block_effects_and_next_move_consumes_stuck() {
    let mut store = world_with_dimension_and_block("minecraft:cobweb");
    store.set_test_equippable_defaults([(
        TEST_SADDLE_ITEM_ID,
        crate::item_profiles::test_equippable(EquipmentSlot::Saddle),
    )]);
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: 8.5,
            y: 2.0,
            z: 8.5,
        },
        y_rot: 0.0,
        ..protocol_add_entity_with_type(10, VANILLA_ENTITY_TYPE_HORSE_ID)
    });
    equip_test_saddle(&mut store, 10);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    store.set_local_player_pose(crate::LocalPlayerPoseState::default());

    store
        .advance_local_abstract_horse_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let stuck = store
        .entities
        .block_effects_debug_state(10)
        .expect("horse block effect state");
    assert_eq!(
        stuck.stuck_speed_multiplier,
        EntityVec3 {
            x: 0.25,
            y: 0.05,
            z: 0.25,
        }
    );

    set_test_block(&mut store, "minecraft:air");
    store
        .advance_local_abstract_horse_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let after = store.entities.transform(10).unwrap();
    // `Entity.move` consumed the multiplier and zeroed the delta; the ridden
    // land-travel tail then only applies gravity and friction to zero.
    assert_eq!(after.delta_movement.x, 0.0);
    assert_eq!(after.delta_movement.z, 0.0);
    assert_eq!(
        store
            .entities
            .block_effects_debug_state(10)
            .expect("horse block effect state")
            .stuck_speed_multiplier,
        EntityVec3::default()
    );
}

#[test]
fn pig_leash_render_state_projects_offset_rope_hold_and_slack() {
    let mut store = WorldStore::new();
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: 10.0,
            y: 64.0,
            z: -5.0,
        },
        y_rot: 0.0,
        ..protocol_add_entity_with_type(60, VANILLA_ENTITY_TYPE_PIG_ID)
    });
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: 14.0,
            y: 65.0,
            z: -5.0,
        },
        ..protocol_add_entity_with_type(61, VANILLA_ENTITY_TYPE_LEASH_KNOT_ID)
    });

    assert_eq!(
        store.entity_leash_render_states_at_partial_tick(60, 1.0),
        None
    );
    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 60,
        dest_id: 61,
    }));

    let states = store
        .entity_leash_render_states_at_partial_tick(60, 1.0)
        .expect("leashed pig must project a rope state");
    assert_eq!(states.len(), 1);
    let state = states[0];
    // Vanilla `Pig.getLeashOffset` = (0, 0.6 * eyeHeight, 0.4 * width) with the
    // pig's default 0.765 eye height and 0.9 width, unrotated at body yaw 0.
    assert!((state.offset[0]).abs() < 1.0e-6, "state {state:?}");
    assert!((state.offset[1] - 0.6 * 0.765).abs() < 1.0e-6);
    assert!((state.offset[2] - 0.36).abs() < 1.0e-6);
    assert!((state.start[0] - 10.0).abs() < 1.0e-6);
    assert!((state.start[1] - (64.0 + 0.6 * 0.765)).abs() < 1.0e-6);
    // Vanilla `LeashFenceKnotEntity.getRopeHoldPosition` = position + (0, 0.2, 0),
    // on the knot's block-center snapped client position (x/z + 0.5, y + 0.375).
    assert!((state.end[0] - 14.5).abs() < 1.0e-6);
    assert!((state.end[1] - 65.575).abs() < 1.0e-6);
    assert!((state.end[2] - -4.5).abs() < 1.0e-6);
    assert!(state.slack);
    // No chunk light is loaded, so both endpoints fall back to the entity
    // full-bright probe default.
    assert_eq!((state.start_block_light, state.start_sky_light), (15, 15));
    assert_eq!((state.end_block_light, state.end_sky_light), (15, 15));
}

#[test]
fn leashed_pig_model_source_carries_the_projected_rope() {
    let mut store = WorldStore::new();
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: 10.0,
            y: 64.0,
            z: -5.0,
        },
        y_rot: 0.0,
        ..protocol_add_entity_with_type(60, VANILLA_ENTITY_TYPE_PIG_ID)
    });
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: 14.0,
            y: 65.0,
            z: -5.0,
        },
        ..protocol_add_entity_with_type(61, VANILLA_ENTITY_TYPE_LEASH_KNOT_ID)
    });
    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 60,
        dest_id: 61,
    }));

    let source = store
        .entity_model_source_at_partial_tick(60, 1.0)
        .expect("leashed pig must project a model source");
    let ropes = source
        .render
        .leash_ropes
        .expect("leashed pig projects a rope");
    assert_eq!(ropes.ropes().len(), 1);
    let rope = ropes.ropes()[0];
    // The rotated pig attach offset and the span to the knot's snapped rope-hold
    // point (start (10, 64.459, -4.64) -> end (14.5, 65.575, -4.5)), as f32.
    assert!((rope.offset[0]).abs() < 1.0e-5, "rope {rope:?}");
    assert!((rope.offset[1] - 0.459).abs() < 1.0e-5);
    assert!((rope.offset[2] - 0.36).abs() < 1.0e-5);
    assert!((rope.delta[0] - 4.5).abs() < 1.0e-5);
    assert!((rope.delta[1] - 1.116).abs() < 1.0e-5);
    assert!((rope.delta[2] - 0.14).abs() < 1.0e-5);
    assert!(rope.slack);
    assert_eq!((rope.start_block_light, rope.start_sky_light), (15, 15));
    assert_eq!((rope.end_block_light, rope.end_sky_light), (15, 15));

    // The holder side is not leashed itself: the knot's own source stays empty.
    let knot_source = store
        .entity_model_source_at_partial_tick(61, 1.0)
        .expect("knot must project a model source");
    assert!(knot_source.render.leash_ropes.is_none());
}
