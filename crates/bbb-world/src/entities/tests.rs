use super::*;

use bbb_protocol::packets::{
    AddEntity as ProtocolAddEntity, AttributeModifier as ProtocolAttributeModifier,
    AttributeSnapshot as ProtocolAttributeSnapshot, CommonPlayerSpawnInfo as ProtocolSpawnInfo,
    EntityAnimation as ProtocolEntityAnimation, EntityDataValue as ProtocolEntityDataValue,
    EntityDataValueKind, EntityEvent as ProtocolEntityEvent, EquipmentSlot, EquipmentSlotUpdate,
    HurtAnimation as ProtocolHurtAnimation, ItemStackSummary,
    ItemStackSummary as ProtocolItemStackSummary, MoveVehicle as ProtocolMoveVehicle,
    PlayLogin as ProtocolPlayLogin, RemoveEntities as ProtocolRemoveEntities,
    SetEntityData as ProtocolSetEntityData, SetEntityLink as ProtocolSetEntityLink,
    SetEquipment as ProtocolSetEquipment, SetPassengers as ProtocolSetPassengers,
    TakeItemEntity as ProtocolTakeItemEntity, UpdateAttributes as ProtocolUpdateAttributes,
    Vec3d as ProtocolVec3d, PLAYER_RELATIVE_DELTA_Y, PLAYER_RELATIVE_X,
};

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
    assert_eq!(store.counters().entity_position_syncs_received, 2);
    assert_eq!(store.counters().entity_position_syncs_applied, 1);
    assert_eq!(store.counters().entity_moves_received, 2);
    assert_eq!(store.counters().entity_moves_applied, 2);
    assert_eq!(store.counters().entity_teleports_received, 1);
    assert_eq!(store.counters().entity_teleports_applied, 1);
    assert_eq!(store.counters().entity_data_updates_received, 2);
    assert_eq!(store.counters().entity_data_values_received, 3);
    assert_eq!(store.counters().entity_data_updates_applied, 2);
    assert_eq!(store.counters().entity_equipment_updates_received, 3);
    assert_eq!(store.counters().entity_equipment_slots_received, 4);
    assert_eq!(store.counters().entity_equipment_updates_applied, 2);
    assert_eq!(store.counters().entity_attribute_updates_received, 3);
    assert_eq!(store.counters().entity_attributes_received, 4);
    assert_eq!(store.counters().entity_attribute_updates_applied, 2);
    assert_eq!(store.counters().entity_motion_updates_applied, 1);
    assert_eq!(store.counters().entity_head_rotations_applied, 1);

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
    assert_eq!(store.counters().entities_tracked, 0);
}

#[test]
fn tracks_entity_passenger_updates() {
    let mut store = WorldStore::new();
    for id in [10, 20, 21, 30] {
        store.apply_add_entity(protocol_add_entity(id));
    }

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20, 21, 999, 20],
    }));
    assert_eq!(store.probe_entity(10).unwrap().passengers, vec![20, 21]);
    assert_eq!(store.probe_entity(20).unwrap().vehicle_id, Some(10));
    assert_eq!(store.probe_entity(21).unwrap().vehicle_id, Some(10));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 30,
        passenger_ids: vec![20],
    }));
    assert_eq!(store.probe_entity(10).unwrap().passengers, vec![21]);
    assert_eq!(store.probe_entity(20).unwrap().vehicle_id, Some(30));
    assert_eq!(store.probe_entity(30).unwrap().passengers, vec![20]);

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: Vec::new(),
    }));
    assert!(store.probe_entity(10).unwrap().passengers.is_empty());
    assert_eq!(store.probe_entity(21).unwrap().vehicle_id, None);

    assert!(!store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 999,
        passenger_ids: vec![21],
    }));
    assert_eq!(
        store.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![30],
        }),
        1
    );
    assert_eq!(store.probe_entity(20).unwrap().vehicle_id, None);
    assert!(store.probe_entity(30).is_none());

    assert_eq!(store.counters().entity_passenger_updates_received, 4);
    assert_eq!(store.counters().entity_passenger_ids_received, 6);
    assert_eq!(store.counters().entity_passenger_updates_applied, 3);
}

#[test]
fn tracks_local_player_passenger_without_entity() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    for id in [10, 20, 30] {
        store.apply_add_entity(protocol_add_entity(id));
    }

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99, 20],
    }));
    assert_eq!(store.local_player_id(), Some(99));
    assert_eq!(store.local_player_vehicle_id(), Some(10));
    assert!(store.probe_entity(99).is_none());
    assert_eq!(store.probe_entity(10).unwrap().passengers, vec![99, 20]);
    assert_eq!(store.probe_entity(20).unwrap().vehicle_id, Some(10));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 30,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_vehicle_id(), Some(30));
    assert_eq!(store.probe_entity(10).unwrap().passengers, vec![20]);
    assert_eq!(store.probe_entity(30).unwrap().passengers, vec![99]);

    assert!(!store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 999,
        passenger_ids: Vec::new(),
    }));
    assert_eq!(store.local_player_vehicle_id(), Some(30));
    assert_eq!(store.probe_entity(30).unwrap().passengers, vec![99]);

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 30,
        passenger_ids: Vec::new(),
    }));
    assert_eq!(store.local_player_vehicle_id(), None);
    assert!(store.probe_entity(30).unwrap().passengers.is_empty());

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    store.apply_login(&protocol_play_login(100));
    assert_eq!(store.local_player_id(), Some(100));
    assert_eq!(store.local_player_vehicle_id(), None);
    assert_eq!(
        store.probe_entity(10).unwrap().passengers,
        Vec::<i32>::new()
    );
}

#[test]
fn move_vehicle_snaps_root_vehicle_and_returns_ack() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity(10));
    store.apply_add_entity(protocol_add_entity(20));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20],
    }));

    let report = store
        .apply_move_vehicle(ProtocolMoveVehicle {
            position: ProtocolVec3d {
                x: 5.0,
                y: 66.0,
                z: -7.0,
            },
            y_rot: 45.0,
            x_rot: -5.0,
        })
        .unwrap();

    assert_eq!(store.local_player_vehicle_id(), Some(20));
    assert_eq!(store.local_player_root_vehicle_id(), Some(10));
    assert_eq!(
        report,
        VehicleMoveReport {
            vehicle_id: 10,
            position: EntityVec3 {
                x: 5.0,
                y: 66.0,
                z: -7.0,
            },
            y_rot: 45.0,
            x_rot: -5.0,
            on_ground: false,
            snapped: true,
        }
    );
    let root = store.probe_entity(10).unwrap();
    assert_eq!(root.position, report.position);
    assert_eq!(root.position_base, report.position);
    assert_eq!(root.y_rot, 45.0);
    assert_eq!(root.x_rot, -5.0);
    assert_eq!(
        store.probe_entity(20).unwrap().position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert_eq!(store.counters().vehicle_moves_received, 1);
    assert_eq!(store.counters().vehicle_moves_applied, 1);
    assert_eq!(store.counters().vehicle_moves_acked, 1);
    assert_eq!(store.counters().vehicle_moves_snapped, 1);
}

#[test]
fn move_vehicle_without_mount_is_noop() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity(10));

    assert_eq!(
        store.apply_move_vehicle(ProtocolMoveVehicle {
            position: ProtocolVec3d {
                x: 5.0,
                y: 66.0,
                z: -7.0,
            },
            y_rot: 45.0,
            x_rot: -5.0,
        }),
        None
    );

    let entity = store.probe_entity(10).unwrap();
    assert_eq!(
        entity.position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert_eq!(store.counters().vehicle_moves_received, 1);
    assert_eq!(store.counters().vehicle_moves_applied, 0);
    assert_eq!(store.counters().vehicle_moves_acked, 0);
    assert_eq!(store.counters().vehicle_moves_snapped, 0);
}

#[test]
fn move_vehicle_small_delta_acks_without_snap() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity(10));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));

    let report = store
        .apply_move_vehicle(ProtocolMoveVehicle {
            position: ProtocolVec3d {
                x: 1.000001,
                y: 64.0,
                z: -2.0,
            },
            y_rot: 80.0,
            x_rot: 35.0,
        })
        .unwrap();

    assert_eq!(
        report,
        VehicleMoveReport {
            vehicle_id: 10,
            position: EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            y_rot: 20.0,
            x_rot: -10.0,
            on_ground: false,
            snapped: false,
        }
    );
    let entity = store.probe_entity(10).unwrap();
    assert_eq!(entity.position, report.position);
    assert_eq!(entity.y_rot, 20.0);
    assert_eq!(entity.x_rot, -10.0);
    assert_eq!(store.counters().vehicle_moves_received, 1);
    assert_eq!(store.counters().vehicle_moves_applied, 1);
    assert_eq!(store.counters().vehicle_moves_acked, 1);
    assert_eq!(store.counters().vehicle_moves_snapped, 0);
}

#[test]
fn take_item_entity_shrinks_item_stacks_and_removes_entities() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(30, 7));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![item_stack_entity_data(item_stack(42, 5))],
    }));

    assert!(store.apply_take_item_entity(ProtocolTakeItemEntity {
        item_id: 10,
        player_id: 99,
        amount: 2,
    }));
    let item_entity = store.probe_entity(10).unwrap();
    assert_eq!(
        item_entity.data_values,
        vec![item_stack_entity_data(item_stack(42, 3))]
    );

    assert!(store.apply_take_item_entity(ProtocolTakeItemEntity {
        item_id: 10,
        player_id: 99,
        amount: 3,
    }));
    assert!(store.probe_entity(10).is_none());

    assert!(store.apply_take_item_entity(ProtocolTakeItemEntity {
        item_id: 20,
        player_id: 99,
        amount: 1,
    }));
    assert!(store.probe_entity(20).is_some());

    assert!(store.apply_take_item_entity(ProtocolTakeItemEntity {
        item_id: 30,
        player_id: 99,
        amount: 1,
    }));
    assert!(store.probe_entity(30).is_none());
    assert!(!store.apply_take_item_entity(ProtocolTakeItemEntity {
        item_id: 999,
        player_id: 99,
        amount: 1,
    }));

    assert_eq!(store.entity_count(), 1);
    assert_eq!(store.counters().take_item_entities_received, 5);
    assert_eq!(store.counters().take_item_entities_applied, 4);
    assert_eq!(store.counters().item_entity_stack_shrinks, 2);
    assert_eq!(store.counters().take_item_entities_removed, 2);
    assert_eq!(store.counters().entities_removed, 2);
    assert_eq!(store.counters().entities_tracked, 1);
}

#[test]
fn tracks_entity_transient_events() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity(123));

    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 123, action: 3 }));
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 123,
        event_id: 35,
    }));
    assert!(store.apply_hurt_animation(ProtocolHurtAnimation { id: 123, yaw: 45.5 }));

    let entity = store.probe_entity(123).unwrap();
    assert_eq!(entity.last_animation_action, Some(3));
    assert_eq!(entity.last_event_id, Some(35));
    assert_eq!(entity.last_hurt_yaw, Some(45.5));

    assert!(!store.apply_entity_animation(ProtocolEntityAnimation { id: 999, action: 4 }));
    assert!(!store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 999,
        event_id: 21,
    }));
    assert!(!store.apply_hurt_animation(ProtocolHurtAnimation { id: 999, yaw: 90.0 }));

    assert_eq!(store.counters().entity_animation_updates_received, 2);
    assert_eq!(store.counters().entity_animation_updates_applied, 1);
    assert_eq!(store.counters().entity_events_received, 2);
    assert_eq!(store.counters().entity_events_applied, 1);
    assert_eq!(store.counters().entity_hurt_animations_received, 2);
    assert_eq!(store.counters().entity_hurt_animations_applied, 1);
}

#[test]
fn tracks_entity_link_updates() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity(10));
    store.apply_add_entity(protocol_add_entity(20));

    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 10,
        dest_id: 20,
    }));
    assert_eq!(store.probe_entity(10).unwrap().leash_holder_id, Some(20));

    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 10,
        dest_id: 999,
    }));
    assert_eq!(store.probe_entity(10).unwrap().leash_holder_id, Some(999));

    assert!(!store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 999,
        dest_id: 20,
    }));

    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 10,
        dest_id: 0,
    }));
    assert_eq!(store.probe_entity(10).unwrap().leash_holder_id, None);

    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 10,
        dest_id: 20,
    }));
    assert_eq!(
        store.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![20],
        }),
        1
    );
    assert_eq!(store.probe_entity(10).unwrap().leash_holder_id, None);

    assert_eq!(store.counters().entity_link_updates_received, 5);
    assert_eq!(store.counters().entity_link_updates_applied, 4);
}

fn protocol_add_entity(id: i32) -> ProtocolAddEntity {
    protocol_add_entity_with_type(id, 7)
}

fn protocol_add_entity_with_type(id: i32, entity_type_id: i32) -> ProtocolAddEntity {
    ProtocolAddEntity {
        id,
        uuid: Uuid::from_u128(0x12345678123456781234567812345678),
        entity_type_id,
        position: ProtocolVec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: -10.0,
        y_rot: 20.0,
        y_head_rot: 30.0,
        data: 99,
    }
}

fn protocol_play_login(player_id: i32) -> ProtocolPlayLogin {
    ProtocolPlayLogin {
        player_id,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: ProtocolSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 0,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: false,
    }
}

fn item_stack_entity_data(item: ProtocolItemStackSummary) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id: VANILLA_ITEM_ENTITY_STACK_DATA_ID,
        serializer_id: 7,
        value: EntityDataValueKind::ItemStack(item),
    }
}

fn item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
    ProtocolItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}
