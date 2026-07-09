use super::*;

#[test]
fn tracks_entity_lifecycle_and_absolute_state_updates() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity(123));

    let entity = store.probe_entity(123).unwrap();
    assert_eq!(entity.entity_type_id, 7);
    assert_eq!(
        entity.position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert_eq!(store.entity_count(), 1);
    assert_eq!(store.counters().entities_received, 1);
    assert_eq!(store.counters().entities_tracked, 1);

    assert!(
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
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
        })
    );
    assert!(store.apply_set_entity_motion(ProtocolSetEntityMotion {
        id: 123,
        delta_movement: ProtocolVec3d {
            x: 0.1,
            y: 0.0,
            z: -0.1,
        },
    }));
    assert!(store.apply_rotate_head(ProtocolRotateHead {
        id: 123,
        y_head_rot: 90.0,
    }));

    let entity = store.probe_entity(123).unwrap();
    assert_eq!(
        entity.position,
        EntityVec3 {
            x: 2.0,
            y: 65.0,
            z: -3.0,
        }
    );
    assert_eq!(
        entity.delta_movement,
        EntityVec3 {
            x: 0.1,
            y: 0.0,
            z: -0.1,
        }
    );
    assert_eq!(entity.y_rot, 180.0);
    assert_eq!(entity.x_rot, 30.0);
    assert_eq!(entity.y_head_rot, 90.0);
    assert_eq!(entity.on_ground, Some(true));

    assert!(store.apply_entity_move(ProtocolEntityMove {
        id: 123,
        delta_x: 4096,
        delta_y: 0,
        delta_z: -2048,
        y_rot: Some(-90.0),
        x_rot: Some(45.0),
        on_ground: false,
    }));
    let entity = store.probe_entity(123).unwrap();
    assert_eq!(
        entity.position,
        EntityVec3 {
            x: 3.0,
            y: 65.0,
            z: -3.5,
        }
    );
    assert_eq!(entity.position_base, entity.position);
    assert_eq!(entity.y_rot, -90.0);
    assert_eq!(entity.x_rot, 45.0);
    assert_eq!(entity.on_ground, Some(false));

    assert!(store.apply_entity_move(ProtocolEntityMove {
        id: 123,
        delta_x: 0,
        delta_y: 0,
        delta_z: 0,
        y_rot: Some(30.0),
        x_rot: Some(-15.0),
        on_ground: true,
    }));
    let entity = store.probe_entity(123).unwrap();
    assert_eq!(
        entity.position,
        EntityVec3 {
            x: 3.0,
            y: 65.0,
            z: -3.5,
        }
    );
    assert_eq!(entity.y_rot, 30.0);
    assert_eq!(entity.x_rot, -15.0);
    assert_eq!(entity.on_ground, Some(true));

    assert!(store.apply_teleport_entity(ProtocolTeleportEntity {
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
        relatives_mask: PLAYER_RELATIVE_X | PLAYER_RELATIVE_DELTA_Y,
        on_ground: true,
    }));
    let entity = store.probe_entity(123).unwrap();
    assert_eq!(
        entity.position,
        EntityVec3 {
            x: 3.5,
            y: 70.0,
            z: -4.0,
        }
    );
    assert_eq!(
        entity.delta_movement,
        EntityVec3 {
            x: 0.0,
            y: 0.2,
            z: 0.0,
        }
    );
    assert_eq!(entity.y_rot, 10.0);
    assert_eq!(entity.x_rot, -90.0);
    assert_eq!(entity.on_ground, Some(true));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![
            ProtocolEntityDataValue {
                data_id: 0,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(0x20),
            },
            ProtocolEntityDataValue {
                data_id: 2,
                serializer_id: 1,
                value: EntityDataValueKind::Int(300),
            },
        ],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![ProtocolEntityDataValue {
            data_id: 2,
            serializer_id: 1,
            value: EntityDataValueKind::Int(301),
        }],
    }));
    let entity = store.probe_entity(123).unwrap();
    assert_eq!(
        entity.data_values,
        vec![
            ProtocolEntityDataValue {
                data_id: 0,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(0x20),
            },
            ProtocolEntityDataValue {
                data_id: 2,
                serializer_id: 1,
                value: EntityDataValueKind::Int(301),
            },
        ]
    );
    assert_eq!(
        store.entities.metadata(123).unwrap().data_values,
        entity.data_values
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 123,
        slots: vec![
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: ItemStackSummary {
                    item_id: Some(42),
                    count: 1,
                    component_patch: Default::default(),
                },
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: ItemStackSummary::empty(),
            },
        ],
    }));
    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 123,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Head,
            item: ItemStackSummary {
                item_id: Some(51),
                count: 2,
                component_patch: Default::default(),
            },
        }],
    }));
    assert!(!store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 999,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::OffHand,
            item: ItemStackSummary::empty(),
        }],
    }));
    let entity = store.probe_entity(123).unwrap();
    assert_eq!(
        entity.equipment,
        vec![
            EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: ItemStackSummary::empty(),
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: ItemStackSummary {
                    item_id: Some(51),
                    count: 2,
                    component_patch: Default::default(),
                },
            },
        ]
    );
    assert_eq!(
        store.entities.equipment(123).unwrap().equipment,
        entity.equipment
    );
    assert_eq!(
        store
            .equipment_item(123, EquipmentSlot::Head)
            .unwrap()
            .item_id,
        Some(51)
    );
    assert!(store.equipment_item(123, EquipmentSlot::MainHand).is_none());
    assert!(store.equipment_item(999, EquipmentSlot::Head).is_none());

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![
            ProtocolAttributeSnapshot {
                attribute_id: 21,
                base: 20.0,
                modifiers: vec![ProtocolAttributeModifier {
                    id: "minecraft:health_bonus".to_string(),
                    amount: 4.0,
                    operation_id: 0,
                }],
            },
            ProtocolAttributeSnapshot {
                attribute_id: 26,
                base: 0.7,
                modifiers: Vec::new(),
            },
        ],
    }));
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: 26,
            base: 0.9,
            modifiers: vec![ProtocolAttributeModifier {
                id: "minecraft:speed_bonus".to_string(),
                amount: 0.2,
                operation_id: 2,
            }],
        }],
    }));
    assert!(!store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 999,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: 21,
            base: 20.0,
            modifiers: Vec::new(),
        }],
    }));
    let entity = store.probe_entity(123).unwrap();
    assert_eq!(
        entity.attributes,
        vec![
            ProtocolAttributeSnapshot {
                attribute_id: 21,
                base: 20.0,
                modifiers: vec![ProtocolAttributeModifier {
                    id: "minecraft:health_bonus".to_string(),
                    amount: 4.0,
                    operation_id: 0,
                }],
            },
            ProtocolAttributeSnapshot {
                attribute_id: 26,
                base: 0.9,
                modifiers: vec![ProtocolAttributeModifier {
                    id: "minecraft:speed_bonus".to_string(),
                    amount: 0.2,
                    operation_id: 2,
                }],
            },
        ]
    );
    assert_eq!(
        store.entities.attributes(123).unwrap().attributes,
        entity.attributes
    );

    assert!(
        !store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 999,
            position: ProtocolVec3d::default(),
            delta_movement: ProtocolVec3d::default(),
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        })
    );
    assert!(!store.apply_entity_move(ProtocolEntityMove {
        id: 999,
        delta_x: 0,
        delta_y: 0,
        delta_z: 0,
        y_rot: None,
        x_rot: None,
        on_ground: false,
    }));
    assert!(!store.apply_teleport_entity(ProtocolTeleportEntity {
        id: 999,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        y_rot: 0.0,
        x_rot: 0.0,
        relatives_mask: 0,
        on_ground: false,
    }));
    assert!(!store.apply_set_entity_motion(ProtocolSetEntityMotion {
        id: 999,
        delta_movement: ProtocolVec3d::default(),
    }));
    assert!(!store.apply_rotate_head(ProtocolRotateHead {
        id: 999,
        y_head_rot: 0.0,
    }));
    assert_eq!(store.counters().entity_position_syncs_received, 2);
    assert_eq!(store.counters().entity_position_syncs_applied, 1);
    assert_eq!(store.counters().entity_position_syncs_ignored, 1);
    assert_eq!(store.counters().entity_moves_received, 3);
    assert_eq!(store.counters().entity_moves_applied, 2);
    assert_eq!(store.counters().entity_moves_ignored, 1);
    assert_eq!(store.counters().entity_teleports_received, 2);
    assert_eq!(store.counters().entity_teleports_applied, 1);
    assert_eq!(store.counters().entity_teleports_ignored, 1);
    assert_eq!(store.counters().entity_data_updates_received, 2);
    assert_eq!(store.counters().entity_data_values_received, 3);
    assert_eq!(store.counters().entity_data_updates_applied, 2);
    assert_eq!(store.counters().entity_equipment_updates_received, 3);
    assert_eq!(store.counters().entity_equipment_slots_received, 4);
    assert_eq!(store.counters().entity_equipment_updates_applied, 2);
    assert_eq!(store.counters().entity_equipment_updates_ignored, 1);
    assert_eq!(store.counters().entity_attribute_updates_received, 3);
    assert_eq!(store.counters().entity_attributes_received, 4);
    assert_eq!(store.counters().entity_attribute_updates_applied, 2);
    assert_eq!(store.counters().entity_attribute_updates_ignored, 1);
    assert_eq!(store.counters().entity_motion_updates_received, 2);
    assert_eq!(store.counters().entity_motion_updates_applied, 1);
    assert_eq!(store.counters().entity_motion_updates_ignored, 1);
    assert_eq!(store.counters().entity_head_rotations_received, 2);
    assert_eq!(store.counters().entity_head_rotations_applied, 1);
    assert_eq!(store.counters().entity_head_rotations_ignored, 1);

    assert_eq!(
        store.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![123, 456],
        }),
        1
    );
    assert!(store.probe_entity(123).is_none());
    assert_eq!(store.entity_count(), 0);
    assert_eq!(store.counters().entity_removes_received, 2);
    assert_eq!(store.counters().entities_removed, 1);
    assert_eq!(store.counters().entity_removes_ignored, 1);
    assert_eq!(store.counters().entities_tracked, 0);
}

#[test]
fn lightning_bolt_add_entity_triggers_client_sky_flash() {
    let mut store = WorldStore::new();

    store.apply_add_entity(protocol_add_entity_with_type(
        123,
        VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID,
    ));

    assert_eq!(store.sky_flash_time(), 2);
    store.advance_sky_flash_time(1);
    assert_eq!(store.sky_flash_time(), 1);

    store.apply_add_entity(protocol_add_entity_with_type(
        124,
        VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID,
    ));
    assert_eq!(store.sky_flash_time(), 2);
}

#[test]
fn equipment_item_queries_non_hand_slots_and_skips_empty_stacks() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity(123));

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 123,
        slots: vec![
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Saddle,
                item: ItemStackSummary {
                    item_id: Some(77),
                    count: 1,
                    component_patch: Default::default(),
                },
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::OffHand,
                item: ItemStackSummary::empty(),
            },
        ],
    }));

    assert_eq!(
        store
            .equipment_item(123, EquipmentSlot::Saddle)
            .unwrap()
            .item_id,
        Some(77)
    );
    assert!(store.equipment_item(123, EquipmentSlot::OffHand).is_none());
    assert!(store.equipment_item(999, EquipmentSlot::Saddle).is_none());
}

#[test]
fn set_equipment_ignores_non_living_entities() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        124,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    ));

    assert!(!store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 124,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::OffHand,
            item: ItemStackSummary::empty(),
        }],
    }));

    assert!(store.probe_entity(124).unwrap().equipment.is_empty());
    assert_eq!(store.counters().entity_equipment_updates_received, 1);
    assert_eq!(store.counters().entity_equipment_slots_received, 1);
    assert_eq!(store.counters().entity_equipment_updates_applied, 0);
    assert_eq!(store.counters().entity_equipment_updates_ignored, 1);
}

#[test]
fn set_entity_data_ignores_unknown_entities() {
    let mut store = WorldStore::new();

    assert!(!store.apply_set_entity_data(ProtocolSetEntityData {
        id: 999,
        values: vec![ProtocolEntityDataValue {
            data_id: 0,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(0x20),
        }],
    }));

    assert_eq!(store.counters().entity_data_updates_received, 1);
    assert_eq!(store.counters().entity_data_values_received, 1);
    assert_eq!(store.counters().entity_data_updates_applied, 0);
    assert_eq!(store.counters().entity_data_updates_ignored, 1);
}

#[test]
fn local_player_using_item_tracks_living_entity_flags() {
    let mut store = WorldStore::new();
    store.local_player_id = Some(123);
    store.apply_add_entity(protocol_add_entity_with_type(
        123,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![living_entity_flags_data(0x01)],
    }));
    assert!(store.local_player().interaction.using_item);
    assert_eq!(
        store.local_player().interaction.using_item_hand,
        Some(InteractionHand::MainHand)
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![living_entity_flags_data(0x00)],
    }));
    assert!(!store.local_player().interaction.using_item);
    assert_eq!(store.local_player().interaction.using_item_hand, None);

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 40,
        item: item_stack(43, 1),
    });
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![living_entity_flags_data(0x03)],
    }));
    assert!(store.local_player().interaction.using_item);
    assert_eq!(
        store.local_player().interaction.using_item_hand,
        Some(InteractionHand::OffHand)
    );
}

#[test]
fn local_player_using_item_flags_do_not_start_with_empty_hand() {
    let mut store = WorldStore::new();
    store.local_player_id = Some(123);
    store.apply_add_entity(protocol_add_entity_with_type(
        123,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![living_entity_flags_data(0x01)],
    }));

    assert!(!store.local_player().interaction.using_item);
    assert_eq!(store.local_player().interaction.using_item_hand, None);
}

#[test]
fn local_player_using_item_flags_do_not_replace_existing_started_hand() {
    let mut store = WorldStore::new();
    store.local_player_id = Some(123);
    store.apply_add_entity(protocol_add_entity_with_type(
        123,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 40,
        item: item_stack(43, 1),
    });
    store.set_local_using_item_with_hand(true, InteractionHand::MainHand);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![living_entity_flags_data(0x03)],
    }));

    assert!(store.local_player().interaction.using_item);
    assert_eq!(
        store.local_player().interaction.using_item_hand,
        Some(InteractionHand::MainHand)
    );
}

#[test]
fn primed_tnt_smoke_particles_use_current_position_when_post_decrement_fuse_survives() {
    const PRIMED_TNT_FUSE_DATA_ID: u8 = 8;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        200,
        VANILLA_ENTITY_TYPE_TNT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        201,
        VANILLA_ENTITY_TYPE_TNT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        202,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 200,
        values: vec![protocol_int_data(PRIMED_TNT_FUSE_DATA_ID, 2)],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 201,
        values: vec![protocol_int_data(PRIMED_TNT_FUSE_DATA_ID, 1)],
    }));

    let states = store.primed_tnt_smoke_particle_states();

    assert_eq!(states.len(), 1);
    assert_eq!(states[0].entity_id, 200);
    assert_eq!(
        states[0].position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
}

#[test]
fn firework_rocket_trail_particles_queue_once_per_client_tick() {
    let mut store = WorldStore::new();
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: 3.5,
            y: 70.25,
            z: -6.75,
        },
        delta_movement: ProtocolVec3d {
            x: 0.2,
            y: 0.8,
            z: -0.4,
        },
        ..protocol_add_entity_with_type(210, VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID)
    });
    store.apply_add_entity(protocol_add_entity_with_type(
        211,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));

    store.advance_entity_client_animations(2);

    let states = store.take_firework_rocket_trail_particle_states();
    assert_eq!(states.len(), 2);
    assert!(states.iter().all(|state| state.entity_id == 210));
    assert!(states.iter().all(|state| {
        state.position
            == EntityVec3 {
                x: 3.5,
                y: 70.25,
                z: -6.75,
            }
    }));
    assert!(states.iter().all(|state| {
        state.delta_movement
            == EntityVec3 {
                x: 0.2,
                y: 0.8,
                z: -0.4,
            }
    }));
    assert!(store
        .take_firework_rocket_trail_particle_states()
        .is_empty());
}

#[test]
fn ominous_item_spawner_particles_queue_on_game_time_multiples_of_five() {
    let mut store = WorldStore::new();
    store.apply_world_time(ProtocolPlayTime {
        game_time: 4,
        clock_updates: Vec::new(),
    });
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: -1.5,
            y: 80.0,
            z: 2.25,
        },
        ..protocol_add_entity_with_type(220, VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID)
    });
    store.apply_add_entity(protocol_add_entity_with_type(
        221,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));

    store.advance_client_time(6);

    let states = store.take_ominous_item_spawner_particle_states();
    assert_eq!(states.len(), 1);
    assert!(states.iter().all(|state| state.entity_id == 220));
    assert!(states.iter().all(|state| {
        state.position
            == EntityVec3 {
                x: -1.5,
                y: 80.0,
                z: 2.25,
            }
    }));
    assert!(store.take_ominous_item_spawner_particle_states().is_empty());

    store.advance_client_time(1);

    let states = store.take_ominous_item_spawner_particle_states();
    assert_eq!(states.len(), 1);
    assert_eq!(states[0].entity_id, 220);
    assert_eq!(
        states[0].position,
        EntityVec3 {
            x: -1.5,
            y: 80.0,
            z: 2.25,
        }
    );
}

#[test]
fn update_attributes_ignores_non_living_entities() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        124,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    ));

    assert!(!store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 124,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: 21,
            base: 20.0,
            modifiers: Vec::new(),
        }],
    }));

    assert!(store.probe_entity(124).unwrap().attributes.is_empty());
    assert_eq!(store.counters().entity_attribute_updates_received, 1);
    assert_eq!(store.counters().entity_attributes_received, 1);
    assert_eq!(store.counters().entity_attribute_updates_applied, 0);
    assert_eq!(store.counters().entity_attribute_updates_ignored, 1);
}

#[test]
fn entity_store_round_trips_serde_and_replaces_by_protocol_id() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(10, 7));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));
    assert!(store.apply_set_entity_motion(ProtocolSetEntityMotion {
        id: 20,
        delta_movement: ProtocolVec3d {
            x: 0.5,
            y: 0.0,
            z: -0.25,
        },
    }));
    assert_eq!(
        store.entities.transform(20).unwrap().delta_movement,
        EntityVec3 {
            x: 0.5,
            y: 0.0,
            z: -0.25,
        }
    );

    let value = serde_json::to_value(&store).unwrap();
    assert!(value["entities"].as_array().is_some());
    let mut restored: WorldStore = serde_json::from_value(value).unwrap();
    assert_eq!(restored.entity_count(), 2);
    assert_eq!(
        restored.entities.entity_type_id(20),
        Some(VANILLA_ENTITY_TYPE_MINECART_ID)
    );
    assert_eq!(
        restored.probe_entity(20).unwrap().delta_movement,
        EntityVec3 {
            x: 0.5,
            y: 0.0,
            z: -0.25,
        }
    );
    assert!(restored
        .entities
        .metadata(20)
        .unwrap()
        .data_values
        .is_empty());
    assert!(restored
        .entities
        .equipment(20)
        .unwrap()
        .equipment
        .is_empty());
    assert!(restored
        .entities
        .attributes(20)
        .unwrap()
        .attributes
        .is_empty());
    assert_eq!(
        restored.entities.transient_events(20).unwrap(),
        EntityTransientEvents {
            last_animation_action: None,
            last_event_id: None,
            last_hurt_yaw: None,
        }
    );

    restored.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    ));
    assert_eq!(restored.entity_count(), 2);
    assert_eq!(
        restored.probe_entity(20).unwrap().entity_type_id,
        VANILLA_ENTITY_TYPE_ITEM_ID
    );
    assert_eq!(
        restored.entities.entity_type_id(20),
        Some(VANILLA_ENTITY_TYPE_ITEM_ID)
    );
    assert!(restored.apply_set_entity_motion(ProtocolSetEntityMotion {
        id: 20,
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.75,
            z: 0.0,
        },
    }));
    assert_eq!(
        restored.probe_entity(20).unwrap().delta_movement,
        EntityVec3 {
            x: 0.0,
            y: 0.75,
            z: 0.0,
        }
    );
}

#[test]
fn entity_transform_queries_read_components_without_full_entity_snapshot() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(10, 7));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));
    assert!(
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 20,
            position: ProtocolVec3d {
                x: 5.0,
                y: 70.0,
                z: -8.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.1,
                y: 0.2,
                z: 0.3,
            },
            y_rot: 45.0,
            x_rot: -15.0,
            on_ground: true,
        })
    );

    let transform = store.probe_entity_transform(20).unwrap();
    assert_eq!(transform.id, 20);
    assert_eq!(transform.entity_type_id, VANILLA_ENTITY_TYPE_MINECART_ID);
    assert_eq!(transform.position, store.probe_entity(20).unwrap().position);
    assert_eq!(
        transform.delta_movement,
        store.probe_entity(20).unwrap().delta_movement
    );
    assert_eq!(transform.y_rot, 45.0);
    assert_eq!(transform.x_rot, -15.0);
    assert_eq!(transform.on_ground, Some(true));

    let transforms = store.entity_transforms();
    assert_eq!(
        transforms
            .iter()
            .map(|entity| entity.id)
            .collect::<Vec<_>>(),
        vec![10, 20]
    );

    assert_eq!(
        store.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![10],
        }),
        1
    );
    assert!(store.probe_entity_transform(10).is_none());
    assert_eq!(
        store
            .entity_transforms()
            .iter()
            .map(|entity| entity.id)
            .collect::<Vec<_>>(),
        vec![20]
    );
}

#[test]
fn entity_pick_bounds_follow_vanilla_pickable_subset() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(10, 14));
    store.apply_add_entity(protocol_add_entity_with_type(11, 0));
    store.apply_add_entity(protocol_add_entity_with_type(
        12,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        13,
        VANILLA_ENTITY_TYPE_FIREBALL_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        14,
        VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(15, 45));
    store.apply_add_entity(protocol_add_entity_with_type(16, 113));
    store.apply_add_entity(protocol_add_entity_with_type(
        17,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(18, 43));
    store.apply_add_entity(protocol_add_entity_with_type(19, 69));
    store.apply_add_entity(protocol_add_entity_with_type(72, 18));
    store.apply_add_entity(protocol_add_entity_with_type(
        73,
        VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        74,
        VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        75,
        VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID,
    ));

    assert_eq!(
        store.probe_entity_pick_bounds(10),
        Some(EntityPickBoundsState::from_base_size(0.6, 1.8, 0.0))
    );
    assert_eq!(
        store.probe_entity_pick_bounds(11),
        Some(EntityPickBoundsState::from_base_size(1.375, 0.5625, 0.0))
    );
    assert_eq!(
        store.probe_entity_pick_bounds(12),
        Some(EntityPickBoundsState::from_base_size(0.98, 0.7, 0.0))
    );
    assert_eq!(
        store.probe_entity_pick_bounds(13),
        Some(EntityPickBoundsState::from_base_size(1.0, 1.0, 1.0))
    );
    assert_eq!(
        store.probe_entity_pick_bounds(14),
        Some(wind_charge_pick_bounds())
    );
    assert_eq!(
        store.probe_entity_pick_bounds(15),
        Some(EntityPickBoundsState::from_base_size(2.0, 2.0, 0.0))
    );
    assert_eq!(
        store.probe_entity_pick_bounds(16),
        Some(EntityPickBoundsState::from_base_size(0.3125, 0.3125, 1.0))
    );
    assert_eq!(
        store.probe_entity_pick_bounds(72),
        Some(wind_charge_pick_bounds())
    );
    assert_eq!(
        store.probe_entity_pick_bounds(73),
        Some(EntityPickBoundsState::from_base_size(0.3125, 0.3125, 0.0))
    );
    assert_eq!(
        store.probe_entity_pick_bounds(74),
        Some(EntityPickBoundsState::from_base_size(1.0, 1.0, 0.0))
    );
    assert_eq!(
        store.probe_entity_pick_bounds(75),
        Some(EntityPickBoundsState::from_base_size(0.25, 0.25, 0.0))
    );
    assert_eq!(
        store.probe_entity_pick_bounds(19),
        Some(EntityPickBoundsState::from_base_size(1.0, 1.0, 0.0))
    );
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 19,
        values: vec![
            ProtocolEntityDataValue {
                data_id: 8,
                serializer_id: 3,
                value: EntityDataValueKind::Float(2.5),
            },
            ProtocolEntityDataValue {
                data_id: 9,
                serializer_id: 3,
                value: EntityDataValueKind::Float(0.75),
            },
            ProtocolEntityDataValue {
                data_id: 10,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
        ],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(19),
        Some(EntityPickBoundsState::from_base_size(2.5, 0.75, 0.0))
    );
    assert_eq!(store.probe_entity_pick_bounds(17), None);
    assert_eq!(store.probe_entity_pick_bounds(18), None);
    assert_eq!(store.probe_entity_pick_bounds(99), None);
}

#[test]
fn entity_camera_pose_uses_vanilla_eye_height() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(123, 7));
    store.apply_add_entity(protocol_add_entity_with_type(124, 0));
    store.apply_add_entity(protocol_add_entity_with_type(125, 5));

    let pose = store
        .probe_entity_camera_pose(123)
        .expect("known entity has camera pose");
    assert_eq!(pose.id, 123);
    assert_eq!(
        pose.position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert_eq!(pose.y_rot, 20.0);
    assert_eq!(pose.x_rot, -10.0);
    assert!((pose.eye_height - 0.2751).abs() < 0.0001);
    assert!((store.probe_entity_camera_pose(124).unwrap().eye_height - 0.5625).abs() < 0.0001);
    assert!((store.probe_entity_camera_pose(125).unwrap().eye_height - 1.7775).abs() < 0.0001);
    assert_eq!(store.probe_entity_camera_pose(404), None);
}
