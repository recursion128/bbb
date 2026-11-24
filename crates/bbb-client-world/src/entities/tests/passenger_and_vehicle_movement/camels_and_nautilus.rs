use super::*;

#[test]
fn local_player_sprintable_vehicle_tracks_vanilla_controlled_camels() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        30,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        40,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_sprintable_vehicle_id(), Some(10));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_sprintable_vehicle_id(), Some(20));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 30,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_sprintable_vehicle_id(), None);

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 40,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_sprintable_vehicle_id(), None);
}

#[test]
fn local_nautilus_water_dash_uses_controller_look_and_retains_old_generation() {
    let mut store = locally_controlled_nautilus(VANILLA_ENTITY_TYPE_NAUTILUS_ID);
    store.insert_decoded_chunk(empty_test_chunk());
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 1, y: -63, z: 1 },
        block_state_id: vanilla_block_state_id("minecraft:water", [("level", "0")]),
    }));
    store.entities.with_transform_mut(10, |transform| {
        transform.position = EntityVec3 {
            x: 1.0,
            y: -63.0,
            z: 1.0,
        };
        transform.old_position = transform.position;
        transform.delta_movement = EntityVec3::default();
        transform.on_ground = Some(false);
    });
    store.set_local_player_pose(LocalPlayerPoseState {
        x_rot: -30.0,
        y_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let before = store.entities.transform(10).unwrap();
    let generation_a = store.publication_snapshot();

    assert!(store.apply_local_player_riding_jump(90));
    store
        .advance_local_nautilus_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();

    let after = store.entities.transform(10).unwrap();
    assert!(after.position.y > before.position.y + 0.6);
    assert!(after.position.z > before.position.z + 1.0);
    assert!((after.x_rot - -15.0).abs() < 1.0e-6);
    assert_eq!(store.entities.nautilus_jump_cooldown_ticks(10), Some(39));
    assert!(store
        .entities
        .metadata_bool_for_entity(
            10,
            VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
            VANILLA_ABSTRACT_NAUTILUS_DEFAULT_DASH,
        )
        .unwrap());
    assert_eq!(generation_a.entities.transform(10).unwrap(), before);
    assert_eq!(
        generation_a.entities.nautilus_jump_cooldown_ticks(10),
        Some(0)
    );
    assert!(!generation_a
        .entities
        .metadata_bool_for_entity(
            10,
            VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
            VANILLA_ABSTRACT_NAUTILUS_DEFAULT_DASH,
        )
        .unwrap());
}

#[test]
fn local_nautilus_land_travel_half_turns_toward_controller() {
    let mut store = locally_controlled_nautilus(VANILLA_ENTITY_TYPE_NAUTILUS_ID);
    store.entities.with_transform_mut(10, |transform| {
        transform.y_rot = 0.0;
        transform.old_y_rot = 0.0;
        transform.on_ground = Some(true);
    });
    store.set_local_player_pose(LocalPlayerPoseState {
        y_rot: 90.0,
        ..LocalPlayerPoseState::default()
    });
    let before = store.entities.transform(10).unwrap();

    store
        .advance_local_nautilus_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();

    let after = store.entities.transform(10).unwrap();
    assert!((after.y_rot - 45.0).abs() < 1.0e-6);
    assert_eq!(after.old_y_rot, after.y_rot);
    assert_eq!(after.y_head_rot, after.y_rot);
    assert!(after.position.x < before.position.x);
    assert!(after.position.z > before.position.z);
}

#[test]
fn local_nautilus_dash_keeps_five_full_ticks_and_clears_cooldown() {
    let mut store = locally_controlled_nautilus(VANILLA_ENTITY_TYPE_NAUTILUS_ID);
    assert!(store.apply_local_player_riding_jump(90));

    store
        .advance_local_nautilus_vehicle_input(crate::LocalPlayerInputState::default(), 1)
        .unwrap();
    assert_eq!(store.entities.nautilus_jump_cooldown_ticks(10), Some(39));
    store
        .advance_local_nautilus_vehicle_input(crate::LocalPlayerInputState::default(), 5)
        .unwrap();
    assert_eq!(store.entities.nautilus_jump_cooldown_ticks(10), Some(34));
    assert!(store
        .entities
        .metadata_bool_for_entity(
            10,
            VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
            VANILLA_ABSTRACT_NAUTILUS_DEFAULT_DASH,
        )
        .unwrap());

    store
        .advance_local_nautilus_vehicle_input(crate::LocalPlayerInputState::default(), 1)
        .unwrap();
    assert_eq!(store.entities.nautilus_jump_cooldown_ticks(10), Some(33));
    assert!(!store
        .entities
        .metadata_bool_for_entity(
            10,
            VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
            VANILLA_ABSTRACT_NAUTILUS_DEFAULT_DASH,
        )
        .unwrap());

    store
        .advance_local_nautilus_vehicle_input(crate::LocalPlayerInputState::default(), 33)
        .unwrap();
    assert_eq!(store.entities.nautilus_jump_cooldown_ticks(10), Some(0));
}

#[test]
fn remote_nautilus_dash_clears_after_five_full_duration_ticks() {
    let mut store = WorldStore::new();
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    ));
    store.advance_entity_client_animations(1);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![ProtocolEntityDataValue {
            data_id: VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert_eq!(store.entities.nautilus_jump_cooldown_ticks(10), Some(40));

    store.advance_entity_client_animations(6);
    assert!(store
        .entities
        .metadata_bool_for_entity(
            10,
            VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
            VANILLA_ABSTRACT_NAUTILUS_DEFAULT_DASH,
        )
        .unwrap());
    assert_eq!(store.entities.nautilus_jump_cooldown_ticks(10), Some(34));
    store.advance_entity_client_animations(1);
    assert!(!store
        .entities
        .metadata_bool_for_entity(
            10,
            VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
            VANILLA_ABSTRACT_NAUTILUS_DEFAULT_DASH,
        )
        .unwrap());
    assert_eq!(store.entities.nautilus_jump_cooldown_ticks(10), Some(33));
}

#[test]
fn nautilus_dash_cooldown_clears_and_old_generation_stays_isolated() {
    let mut store = WorldStore::new();
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    ));
    store.advance_entity_client_animations(1);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![ProtocolEntityDataValue {
            data_id: VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    let generation_a = store.publication_snapshot();

    store.advance_entity_client_animations(40);
    assert_eq!(store.entities.nautilus_jump_cooldown_ticks(10), Some(0));
    assert_eq!(
        generation_a.entities.nautilus_jump_cooldown_ticks(10),
        Some(40)
    );
    assert!(generation_a
        .entities
        .metadata_bool_for_entity(
            10,
            VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
            VANILLA_ABSTRACT_NAUTILUS_DEFAULT_DASH,
        )
        .unwrap());
}

#[test]
fn remote_nautilus_water_tick_emits_bubbles_from_clamped_mouth_direction() {
    let mut store = WorldStore::new();
    store.insert_decoded_chunk(empty_test_chunk());
    for y in [-63, -62] {
        assert!(store.apply_block_update(ProtocolBlockUpdate {
            pos: ProtocolBlockPos { x: 1, y, z: 1 },
            block_state_id: vanilla_block_state_id("minecraft:water", [("level", "0")]),
        }));
    }
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    ));
    store.entities.with_transform_mut(10, |transform| {
        transform.position = EntityVec3 {
            x: 1.0,
            y: -63.0,
            z: 1.0,
        };
        transform.old_position = transform.position;
        transform.x_rot = 45.0;
        transform.y_rot = 0.0;
    });

    store.advance_entity_client_animations(40);
    let effects = store.take_entity_client_movement_effect_states();
    let bubbles = effects
        .iter()
        .filter_map(|effect| match effect {
            super::super::super::EntityClientMovementEffectState::Particle(
                super::super::super::EntityClientMovementParticleState::Simple(state),
            ) if state.kind
                == super::super::super::EntityClientMovementSimpleParticleKind::Bubble =>
            {
                Some(state)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(!bubbles.is_empty());
    for bubble in bubbles {
        let view = super::super::super::calculate_view_vector(10.0, 0.0);
        assert!((bubble.position.x - 1.0).abs() < 1.0e-12);
        assert!((bubble.position.y - (-63.0 - view.y + 0.25)).abs() < 1.0e-12);
        assert!((bubble.position.z - (1.0 - view.z * 1.1)).abs() < 1.0e-12);
    }
}

#[test]
fn silent_nautilus_completes_dash_cooldown() {
    let mut store = WorldStore::new();
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
    ));
    store.advance_entity_client_animations(1);
    let accessors = vanilla_entity_metadata_entity_accessors().unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![
            ProtocolEntityDataValue {
                data_id: VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: accessors.silent_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
        ],
    }));

    store.advance_entity_client_animations(40);
    assert_eq!(store.entities.nautilus_jump_cooldown_ticks(10), Some(0));
}

#[test]
fn recreated_nautilus_id_does_not_retain_dash_or_rng_owner() {
    let mut store = locally_controlled_nautilus(VANILLA_ENTITY_TYPE_NAUTILUS_ID);
    assert!(store.apply_local_player_riding_jump(90));
    store
        .advance_local_nautilus_vehicle_input(crate::LocalPlayerInputState::default(), 1)
        .unwrap();
    let generation_a = store.publication_snapshot();
    assert_eq!(
        generation_a.entities.nautilus_jump_cooldown_ticks(10),
        Some(39)
    );

    assert_eq!(
        store.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![10],
        }),
        1
    );
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    ));
    equip_test_saddle(&mut store, 10);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));

    assert_eq!(store.entities.nautilus_jump_cooldown_ticks(10), Some(0));
    assert!(!store
        .entities
        .metadata_bool_for_entity(
            10,
            VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
            VANILLA_ABSTRACT_NAUTILUS_DEFAULT_DASH,
        )
        .unwrap());
    assert_eq!(
        generation_a.entities.nautilus_jump_cooldown_ticks(10),
        Some(39)
    );
}
