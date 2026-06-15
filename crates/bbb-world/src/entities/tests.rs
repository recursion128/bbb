use super::*;

use bbb_protocol::packets::{
    AddEntity as ProtocolAddEntity, AttributeModifier as ProtocolAttributeModifier,
    AttributeSnapshot as ProtocolAttributeSnapshot, CommonPlayerSpawnInfo as ProtocolSpawnInfo,
    DataComponentPatchSummary, EntityAnimation as ProtocolEntityAnimation,
    EntityDataEnumSerializer, EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind,
    EntityEvent as ProtocolEntityEvent, EntityMove as ProtocolEntityMove,
    EntityPositionSync as ProtocolEntityPositionSync, EquipmentSlot, EquipmentSlotUpdate,
    GameProfile as ProtocolGameProfile, GameType as ProtocolGameType,
    HurtAnimation as ProtocolHurtAnimation, ItemStackSummary,
    ItemStackSummary as ProtocolItemStackSummary, MinecartStep as ProtocolMinecartStep,
    MoveMinecartAlongTrack as ProtocolMoveMinecartAlongTrack, MoveVehicle as ProtocolMoveVehicle,
    PlayLogin as ProtocolPlayLogin, PlayerInfoAction as ProtocolPlayerInfoAction,
    PlayerInfoEntry as ProtocolPlayerInfoEntry, PlayerInfoUpdate as ProtocolPlayerInfoUpdate,
    RemoveEntities as ProtocolRemoveEntities, RotateHead as ProtocolRotateHead,
    SetEntityData as ProtocolSetEntityData, SetEntityLink as ProtocolSetEntityLink,
    SetEntityMotion as ProtocolSetEntityMotion, SetEquipment as ProtocolSetEquipment,
    SetPassengers as ProtocolSetPassengers, TakeItemEntity as ProtocolTakeItemEntity,
    TeleportEntity as ProtocolTeleportEntity, UpdateAttributes as ProtocolUpdateAttributes,
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

#[test]
fn ender_dragon_pick_targets_use_vanilla_part_ids_and_bounds() {
    const ENDER_DRAGON_TYPE_ID: i32 = 43;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type_y_rot(
        100,
        ENDER_DRAGON_TYPE_ID,
        0.0,
    ));
    store.advance_entity_client_animations(1);

    assert_eq!(store.probe_entity_pick_bounds(100), None);
    let targets = store.entity_pick_targets();
    assert_eq!(
        targets
            .iter()
            .map(|target| target.entity_id)
            .collect::<Vec<_>>(),
        vec![101, 102, 103, 104, 105, 106, 107, 108]
    );

    let expected = [
        (
            EntityVec3 {
                x: 1.0,
                y: 63.0,
                z: -8.5,
            },
            EntityPickBoundsState::from_base_size(1.0, 1.0, 0.0),
        ),
        (
            EntityVec3 {
                x: 1.0,
                y: 63.0,
                z: -7.5,
            },
            EntityPickBoundsState::from_base_size(3.0, 3.0, 0.0),
        ),
        (
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.5,
            },
            EntityPickBoundsState::from_base_size(5.0, 3.0, 0.0),
        ),
        (
            EntityVec3 {
                x: 1.0,
                y: 65.5,
                z: 1.5,
            },
            EntityPickBoundsState::from_base_size(2.0, 2.0, 0.0),
        ),
        (
            EntityVec3 {
                x: 1.0,
                y: 65.5,
                z: 3.5,
            },
            EntityPickBoundsState::from_base_size(2.0, 2.0, 0.0),
        ),
        (
            EntityVec3 {
                x: 1.0,
                y: 65.5,
                z: 5.5,
            },
            EntityPickBoundsState::from_base_size(2.0, 2.0, 0.0),
        ),
        (
            EntityVec3 {
                x: 5.5,
                y: 66.0,
                z: -2.0,
            },
            EntityPickBoundsState::from_base_size(4.0, 2.0, 0.0),
        ),
        (
            EntityVec3 {
                x: -3.5,
                y: 66.0,
                z: -2.0,
            },
            EntityPickBoundsState::from_base_size(4.0, 2.0, 0.0),
        ),
    ];

    for (target, (position, bounds)) in targets.iter().zip(expected) {
        assert_entity_vec3_close(target.position, position);
        assert_eq!(target.bounds, bounds);
    }
}

#[test]
fn ender_dragon_pick_targets_follow_flight_history_and_phase() {
    const ENDER_DRAGON_TYPE_ID: i32 = 43;
    const ENDER_DRAGON_PHASE_DATA_ID: u8 = 16;
    const HOLDING_PATTERN_PHASE_ID: i32 = 0;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type_y_rot(
        120,
        ENDER_DRAGON_TYPE_ID,
        0.0,
    ));
    store.advance_entity_client_animations(1);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 120,
        values: vec![protocol_int_data(
            ENDER_DRAGON_PHASE_DATA_ID,
            HOLDING_PATTERN_PHASE_ID,
        )],
    }));
    assert!(
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 120,
            position: ProtocolVec3d {
                x: 1.0,
                y: 70.0,
                z: -2.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 90.0,
            x_rot: 0.0,
            on_ground: false,
        })
    );
    store.advance_entity_client_animations(1);

    let targets = store.entity_pick_targets();
    assert_entity_vec3_close(
        pick_target(&targets, 121).position,
        EntityVec3 {
            x: 7.5,
            y: 64.0,
            z: -2.0,
        },
    );
    assert_eq!(
        pick_target(&targets, 121).bounds,
        EntityPickBoundsState::from_base_size(1.0, 1.0, 0.0)
    );
    assert_entity_vec3_close(
        pick_target(&targets, 122).position,
        EntityVec3 {
            x: 6.5,
            y: 64.0,
            z: -2.0,
        },
    );
    assert_entity_vec3_close(
        pick_target(&targets, 124).position,
        EntityVec3 {
            x: -2.5,
            y: 71.5,
            z: -2.0,
        },
    );

    let cloned = store.clone();
    let cloned_targets = cloned.entity_pick_targets();
    assert_entity_vec3_close(
        pick_target(&cloned_targets, 121).position,
        pick_target(&targets, 121).position,
    );

    let restored: WorldStore = serde_json::from_value(serde_json::to_value(&store).unwrap())
        .expect("world store should roundtrip");
    let restored_targets = restored.entity_pick_targets();
    assert_entity_vec3_close(
        pick_target(&restored_targets, 121).position,
        pick_target(&targets, 121).position,
    );
    assert_entity_vec3_close(
        pick_target(&restored_targets, 124).position,
        pick_target(&targets, 124).position,
    );
}

#[test]
fn ender_dragon_pick_targets_interpolate_flight_history_by_partial_tick() {
    const ENDER_DRAGON_TYPE_ID: i32 = 43;
    const ENDER_DRAGON_PHASE_DATA_ID: u8 = 16;
    const HOLDING_PATTERN_PHASE_ID: i32 = 0;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type_y_rot(
        130,
        ENDER_DRAGON_TYPE_ID,
        0.0,
    ));
    store.advance_entity_client_animations(1);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 130,
        values: vec![protocol_int_data(
            ENDER_DRAGON_PHASE_DATA_ID,
            HOLDING_PATTERN_PHASE_ID,
        )],
    }));
    assert!(
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 130,
            position: ProtocolVec3d {
                x: 1.0,
                y: 74.0,
                z: -2.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 90.0,
            x_rot: 0.0,
            on_ground: false,
        })
    );
    store.advance_entity_client_animations(1);

    let targets_at_start = store.entity_pick_targets_at_partial_tick(0.0);
    let targets_mid_tick = store.entity_pick_targets_at_partial_tick(0.5);
    let targets_at_end = store.entity_pick_targets_at_partial_tick(1.0);
    let default_targets = store.entity_pick_targets();

    assert_entity_vec3_close(
        pick_target(&targets_at_start, 131).position,
        EntityVec3 {
            x: 7.5,
            y: 74.0,
            z: -2.0,
        },
    );
    assert_entity_vec3_close(
        pick_target(&targets_mid_tick, 131).position,
        EntityVec3 {
            x: 7.5,
            y: 69.0,
            z: -2.0,
        },
    );
    assert_entity_vec3_close(
        pick_target(&targets_at_end, 131).position,
        EntityVec3 {
            x: 7.5,
            y: 64.0,
            z: -2.0,
        },
    );
    assert_entity_vec3_close(
        pick_target(&default_targets, 131).position,
        pick_target(&targets_at_end, 131).position,
    );
}

#[test]
fn block_attached_entity_pick_bounds_follow_vanilla_client_boxes() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type_data(20, 73, 2));
    store.apply_add_entity(protocol_add_entity_with_type_data(21, 60, 5));
    store.apply_add_entity(protocol_add_entity_with_type_data(22, 93, 3));
    store.apply_add_entity(protocol_add_entity_with_type_data(23, 76, 0));

    assert_eq!(
        store.probe_entity_transform(20).unwrap().position,
        EntityVec3 {
            x: 1.5,
            y: 64.5,
            z: -1.03125,
        }
    );
    assert_eq!(
        store.probe_entity_pick_bounds(20),
        Some(EntityPickBoundsState::from_centered_size(
            0.75, 0.75, 0.0625, 0.0,
        ))
    );

    assert_eq!(
        store.probe_entity_transform(21).unwrap().position,
        EntityVec3 {
            x: 1.03125,
            y: 64.5,
            z: -1.5,
        }
    );
    assert_eq!(
        store.probe_entity_pick_bounds(21),
        Some(EntityPickBoundsState::from_centered_size(
            0.0625, 0.75, 0.75, 0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 21,
        values: vec![ProtocolEntityDataValue {
            data_id: 9,
            serializer_id: 7,
            value: EntityDataValueKind::ItemStack(ItemStackSummary {
                item_id: Some(999),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added: 1,
                    added_type_ids: vec![41],
                    removed_type_ids: Vec::new(),
                },
            }),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(21),
        Some(EntityPickBoundsState::from_centered_size(
            0.0625, 1.0, 1.0, 0.0,
        ))
    );

    assert_eq!(
        store.probe_entity_transform(22).unwrap().position,
        EntityVec3 {
            x: 1.5,
            y: 64.5,
            z: -1.96875,
        }
    );
    assert_eq!(
        store.probe_entity_pick_bounds(22),
        Some(EntityPickBoundsState::from_centered_size(
            1.0, 1.0, 0.0625, 0.0,
        ))
    );
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 22,
        values: vec![ProtocolEntityDataValue {
            data_id: 9,
            serializer_id: 34,
            value: EntityDataValueKind::PaintingVariant(
                bbb_protocol::packets::PaintingVariantData {
                    registry_id: Some(21),
                    direct: None,
                }
            ),
        }],
    }));
    assert_eq!(
        store.probe_entity_transform(22).unwrap().position,
        EntityVec3 {
            x: 2.0,
            y: 65.0,
            z: -1.96875,
        }
    );
    assert_eq!(
        store.probe_entity_pick_bounds(22),
        Some(EntityPickBoundsState::from_centered_size(
            4.0, 4.0, 0.0625, 0.0,
        ))
    );

    assert_eq!(
        store.probe_entity_transform(23).unwrap().position,
        EntityVec3 {
            x: 1.5,
            y: 64.375,
            z: -1.5,
        }
    );
    assert_eq!(
        store.probe_entity_pick_bounds(23),
        Some(EntityPickBoundsState::from_base_size(0.375, 0.5, 0.0))
    );
}

#[test]
fn slime_pick_bounds_scale_with_vanilla_size_metadata() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(24, 117));
    store.apply_add_entity(protocol_add_entity_with_type(25, 80));

    assert_eq!(
        store.probe_entity_pick_bounds(24),
        Some(EntityPickBoundsState::from_base_size(0.52, 0.52, 0.0))
    );
    assert_eq!(
        store.probe_entity_pick_bounds(25),
        Some(EntityPickBoundsState::from_base_size(0.52, 0.52, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 24,
        values: vec![ProtocolEntityDataValue {
            data_id: 16,
            serializer_id: 1,
            value: EntityDataValueKind::Int(4),
        }],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 25,
        values: vec![ProtocolEntityDataValue {
            data_id: 16,
            serializer_id: 1,
            value: EntityDataValueKind::Int(3),
        }],
    }));

    assert_eq!(
        store.probe_entity_pick_bounds(24),
        Some(EntityPickBoundsState::from_base_size(
            0.52 * 4.0,
            0.52 * 4.0,
            0.0,
        ))
    );
    assert_eq!(
        store.probe_entity_pick_bounds(25),
        Some(EntityPickBoundsState::from_base_size(
            0.52 * 3.0,
            0.52 * 3.0,
            0.0,
        ))
    );
}

#[test]
fn pufferfish_pick_bounds_scale_with_vanilla_puff_state_metadata() {
    const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;
    const PUFF_STATE_DATA_ID: u8 = 17;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(26, 107));

    assert_eq!(
        store.probe_entity_pick_bounds(26),
        Some(EntityPickBoundsState::from_base_size(
            0.7 * 0.5,
            0.7 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 26,
        values: vec![ProtocolEntityDataValue {
            data_id: PUFF_STATE_DATA_ID,
            serializer_id: 1,
            value: EntityDataValueKind::Int(1),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(26),
        Some(EntityPickBoundsState::from_base_size(
            0.7 * 0.7,
            0.7 * 0.7,
            0.0,
        ))
    );

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 26,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(26),
        Some(EntityPickBoundsState::from_base_size(
            0.7 * 0.7 * 2.0,
            0.7 * 0.7 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 26,
        values: vec![ProtocolEntityDataValue {
            data_id: PUFF_STATE_DATA_ID,
            serializer_id: 1,
            value: EntityDataValueKind::Int(2),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(26),
        Some(EntityPickBoundsState::from_base_size(
            0.7 * 2.0,
            0.7 * 2.0,
            0.0,
        ))
    );
}

#[test]
fn salmon_pick_bounds_scale_with_vanilla_variant_metadata() {
    const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;
    const SALMON_VARIANT_DATA_ID: u8 = 17;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(27, 110));

    assert_eq!(
        store.probe_entity_pick_bounds(27),
        Some(EntityPickBoundsState::from_base_size(0.7, 0.4, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 27,
        values: vec![ProtocolEntityDataValue {
            data_id: SALMON_VARIANT_DATA_ID,
            serializer_id: 1,
            value: EntityDataValueKind::Int(0),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(27),
        Some(EntityPickBoundsState::from_base_size(
            0.7 * 0.5,
            0.4 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 27,
        values: vec![ProtocolEntityDataValue {
            data_id: SALMON_VARIANT_DATA_ID,
            serializer_id: 1,
            value: EntityDataValueKind::Int(2),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(27),
        Some(EntityPickBoundsState::from_base_size(
            0.7 * 1.5,
            0.4 * 1.5,
            0.0,
        ))
    );

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 27,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(27),
        Some(EntityPickBoundsState::from_base_size(
            0.7 * 1.5 * 2.0,
            0.4 * 1.5 * 2.0,
            0.0,
        ))
    );
}

#[test]
fn phantom_pick_bounds_scale_with_vanilla_size_metadata() {
    const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;
    const PHANTOM_SIZE_DATA_ID: u8 = 16;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(28, 99));

    assert_eq!(
        store.probe_entity_pick_bounds(28),
        Some(EntityPickBoundsState::from_base_size(0.9, 0.5, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 28,
        values: vec![ProtocolEntityDataValue {
            data_id: PHANTOM_SIZE_DATA_ID,
            serializer_id: 1,
            value: EntityDataValueKind::Int(4),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(28),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * (1.0 + 0.15 * 4.0),
            0.5 * (1.0 + 0.15 * 4.0),
            0.0,
        ))
    );

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 28,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(28),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * (1.0 + 0.15 * 4.0) * 2.0,
            0.5 * (1.0 + 0.15 * 4.0) * 2.0,
            0.0,
        ))
    );
}

#[test]
fn living_pick_bounds_scale_with_vanilla_scale_attribute() {
    const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(30, 14));
    store.apply_add_entity(protocol_add_entity_with_type(31, 51));

    assert_eq!(
        store.probe_entity_pick_bounds(30),
        Some(EntityPickBoundsState::from_base_size(0.6, 1.8, 0.0))
    );

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 30,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(30),
        Some(EntityPickBoundsState::from_base_size(
            0.6 * 2.0,
            1.8 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 30,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 1.0,
            modifiers: vec![
                ProtocolAttributeModifier {
                    id: "minecraft:add_scale".to_string(),
                    amount: 1.0,
                    operation_id: 0,
                },
                ProtocolAttributeModifier {
                    id: "minecraft:add_base_scale".to_string(),
                    amount: 0.5,
                    operation_id: 1,
                },
                ProtocolAttributeModifier {
                    id: "minecraft:multiply_total_scale".to_string(),
                    amount: 1.0,
                    operation_id: 2,
                },
            ],
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(30),
        Some(EntityPickBoundsState::from_base_size(
            0.6 * 6.0,
            1.8 * 6.0,
            0.0,
        ))
    );

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 30,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: -1.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(30),
        Some(EntityPickBoundsState::from_base_size(
            0.6 * 0.0625,
            1.8 * 0.0625,
            0.0,
        ))
    );

    assert!(!store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 31,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 4.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(31),
        Some(EntityPickBoundsState::from_base_size(0.98, 0.98, 0.0))
    );
}

#[test]
fn living_pick_bounds_apply_vanilla_entity_specific_scale_caps() {
    const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(73, 58));
    store.apply_add_entity(protocol_add_entity_with_type(74, 112));

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 73,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(73),
        Some(EntityPickBoundsState::from_base_size(4.0, 4.0, 0.0))
    );

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 73,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 0.5,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(73),
        Some(EntityPickBoundsState::from_base_size(
            4.0 * 0.5,
            4.0 * 0.5,
            0.0
        ))
    );

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 74,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 4.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(74),
        Some(EntityPickBoundsState::from_base_size(
            1.0 * 3.0,
            1.0 * 3.0,
            0.0
        ))
    );
}

#[test]
fn avatar_pick_bounds_follow_vanilla_pose_metadata() {
    const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;
    const ENTITY_DATA_POSE_ID: u8 = 6;
    const POSE_SLEEPING: i32 = 2;
    const POSE_SWIMMING: i32 = 3;
    const POSE_CROUCHING: i32 = 5;
    const POSE_DYING: i32 = 7;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        32,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(33, 83));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 32,
        values: vec![protocol_pose_data(ENTITY_DATA_POSE_ID, POSE_CROUCHING)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(32),
        Some(EntityPickBoundsState::from_base_size(0.6, 1.5, 0.0))
    );

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 32,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(32),
        Some(EntityPickBoundsState::from_base_size(
            0.6 * 2.0,
            1.5 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 32,
        values: vec![protocol_pose_data(ENTITY_DATA_POSE_ID, POSE_SWIMMING)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(32),
        Some(EntityPickBoundsState::from_base_size(
            0.6 * 2.0,
            0.6 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 32,
        values: vec![protocol_pose_data(ENTITY_DATA_POSE_ID, POSE_SLEEPING)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(32),
        Some(EntityPickBoundsState::from_base_size(0.2, 0.2, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 32,
        values: vec![protocol_pose_data(ENTITY_DATA_POSE_ID, POSE_DYING)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(32),
        Some(EntityPickBoundsState::from_base_size(0.2, 0.2, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 33,
        values: vec![protocol_pose_data(ENTITY_DATA_POSE_ID, POSE_CROUCHING)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(33),
        Some(EntityPickBoundsState::from_base_size(0.6, 1.5, 0.0))
    );
}

#[test]
fn warden_pick_bounds_follow_vanilla_pose_metadata() {
    const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;
    const ENTITY_DATA_POSE_ID: u8 = 6;
    const POSE_ROARING: i32 = 11;
    const POSE_EMERGING: i32 = 13;
    const POSE_DIGGING: i32 = 14;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(34, 142));

    assert_eq!(
        store.probe_entity_pick_bounds(34),
        Some(EntityPickBoundsState::from_base_size(0.9, 2.9, 0.0))
    );

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 34,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(34),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * 2.0,
            2.9 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 34,
        values: vec![protocol_pose_data(ENTITY_DATA_POSE_ID, POSE_EMERGING)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(34),
        Some(EntityPickBoundsState::from_base_size(0.9, 1.0, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 34,
        values: vec![protocol_pose_data(ENTITY_DATA_POSE_ID, POSE_DIGGING)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(34),
        Some(EntityPickBoundsState::from_base_size(0.9, 1.0, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 34,
        values: vec![protocol_pose_data(ENTITY_DATA_POSE_ID, POSE_ROARING)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(34),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * 2.0,
            2.9 * 2.0,
            0.0,
        ))
    );
}

#[test]
fn camel_pick_bounds_follow_vanilla_sitting_pose_and_age_scale() {
    const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;
    const ENTITY_DATA_POSE_ID: u8 = 6;
    const AGEABLE_BABY_DATA_ID: u8 = 16;
    const POSE_STANDING: i32 = 0;
    const POSE_SITTING: i32 = 10;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(43, 19));
    store.apply_add_entity(protocol_add_entity_with_type(44, 20));

    assert_eq!(
        store.probe_entity_pick_bounds(43),
        Some(EntityPickBoundsState::from_base_size(1.7, 2.375, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 43,
        values: vec![protocol_pose_data(ENTITY_DATA_POSE_ID, POSE_SITTING)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(43),
        Some(EntityPickBoundsState::from_base_size(
            1.7,
            2.375 - 1.43,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 43,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(43),
        Some(EntityPickBoundsState::from_base_size(
            1.7 * 0.6,
            (2.375 - 1.43) * 0.6,
            0.0,
        ))
    );

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 43,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(43),
        Some(EntityPickBoundsState::from_base_size(
            1.7 * 0.6 * 2.0,
            (2.375 - 1.43) * 0.6 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 43,
        values: vec![protocol_pose_data(ENTITY_DATA_POSE_ID, POSE_STANDING)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(43),
        Some(EntityPickBoundsState::from_base_size(
            1.7 * 0.6 * 2.0,
            2.375 * 0.6 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 44,
        values: vec![
            protocol_pose_data(ENTITY_DATA_POSE_ID, POSE_SITTING),
            protocol_bool_data(AGEABLE_BABY_DATA_ID, true),
        ],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(44),
        Some(EntityPickBoundsState::from_base_size(
            1.7,
            2.375 - 1.43,
            0.0,
        ))
    );

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 44,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(44),
        Some(EntityPickBoundsState::from_base_size(
            1.7 * 2.0,
            (2.375 - 1.43) * 2.0,
            0.0,
        ))
    );
}

#[test]
fn goat_pick_bounds_follow_vanilla_pose_and_age_scale() {
    const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;
    const ENTITY_DATA_POSE_ID: u8 = 6;
    const AGEABLE_BABY_DATA_ID: u8 = 16;
    const POSE_STANDING: i32 = 0;
    const POSE_LONG_JUMPING: i32 = 6;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(41, 62));

    assert_eq!(
        store.probe_entity_pick_bounds(41),
        Some(EntityPickBoundsState::from_base_size(0.9, 1.3, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 41,
        values: vec![protocol_pose_data(ENTITY_DATA_POSE_ID, POSE_LONG_JUMPING)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(41),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * 0.7,
            1.3 * 0.7,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 41,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(41),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * 0.7 * 0.55,
            1.3 * 0.7 * 0.55,
            0.0,
        ))
    );

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 41,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(41),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * 0.7 * 0.55 * 2.0,
            1.3 * 0.7 * 0.55 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 41,
        values: vec![protocol_pose_data(ENTITY_DATA_POSE_ID, POSE_STANDING)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(41),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * 0.55 * 2.0,
            1.3 * 0.55 * 2.0,
            0.0,
        ))
    );
}

#[test]
fn sniffer_pick_bounds_follow_vanilla_state_and_age_scale() {
    const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;
    const AGEABLE_BABY_DATA_ID: u8 = 16;
    const SNIFFER_STATE_DATA_ID: u8 = 18;
    const SNIFFER_STATE_SEARCHING: i32 = 4;
    const SNIFFER_STATE_DIGGING: i32 = 5;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(42, 119));

    assert_eq!(
        store.probe_entity_pick_bounds(42),
        Some(EntityPickBoundsState::from_base_size(1.9, 1.75, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 42,
        values: vec![protocol_enum_data(
            SNIFFER_STATE_DATA_ID,
            EntityDataEnumSerializer::SnifferState,
            SNIFFER_STATE_DIGGING,
        )],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(42),
        Some(EntityPickBoundsState::from_base_size(1.9, 1.75 - 0.4, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 42,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(42),
        Some(EntityPickBoundsState::from_base_size(
            1.9 * 0.5,
            (1.75 - 0.4) * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 42,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(42),
        Some(EntityPickBoundsState::from_base_size(
            1.9 * 0.5 * 2.0,
            (1.75 - 0.4) * 0.5 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 42,
        values: vec![protocol_enum_data(
            SNIFFER_STATE_DATA_ID,
            EntityDataEnumSerializer::SnifferState,
            SNIFFER_STATE_SEARCHING,
        )],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(42),
        Some(EntityPickBoundsState::from_base_size(
            1.9 * 0.5 * 2.0,
            1.75 * 0.5 * 2.0,
            0.0,
        ))
    );
}

#[test]
fn baby_pick_bounds_follow_vanilla_metadata() {
    const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;
    const AGEABLE_BABY_DATA_ID: u8 = 16;
    const ZOMBIE_BABY_DATA_ID: u8 = 16;
    const PIGLIN_BABY_DATA_ID: u8 = 17;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(34, 100));
    store.apply_add_entity(protocol_add_entity_with_type(35, 26));
    store.apply_add_entity(protocol_add_entity_with_type(36, 139));
    store.apply_add_entity(protocol_add_entity_with_type(37, 150));
    store.apply_add_entity(protocol_add_entity_with_type(38, 101));
    store.apply_add_entity(protocol_add_entity_with_type(39, 30));
    store.apply_add_entity(protocol_add_entity_with_type(40, 141));
    store.apply_add_entity(protocol_add_entity_with_type(45, 4));
    store.apply_add_entity(protocol_add_entity_with_type(46, 54));
    store.apply_add_entity(protocol_add_entity_with_type(47, 108));
    store.apply_add_entity(protocol_add_entity_with_type(71, 104));
    store.apply_add_entity(protocol_add_entity_with_type(48, 7));
    store.apply_add_entity(protocol_add_entity_with_type(49, 127));
    store.apply_add_entity(protocol_add_entity_with_type(50, 61));
    store.apply_add_entity(protocol_add_entity_with_type(51, 137));
    store.apply_add_entity(protocol_add_entity_with_type(52, 35));
    store.apply_add_entity(protocol_add_entity_with_type(53, 58));
    store.apply_add_entity(protocol_add_entity_with_type(54, 96));
    store.apply_add_entity(protocol_add_entity_with_type(55, 36));
    store.apply_add_entity(protocol_add_entity_with_type(56, 87));
    store.apply_add_entity(protocol_add_entity_with_type(57, 66));
    store.apply_add_entity(protocol_add_entity_with_type(58, 116));
    store.apply_add_entity(protocol_add_entity_with_type(59, 151));
    store.apply_add_entity(protocol_add_entity_with_type(60, 78));
    store.apply_add_entity(protocol_add_entity_with_type(61, 134));
    store.apply_add_entity(protocol_add_entity_with_type(62, 11));
    store.apply_add_entity(protocol_add_entity_with_type(63, 21));
    store.apply_add_entity(protocol_add_entity_with_type(64, 64));
    store.apply_add_entity(protocol_add_entity_with_type(65, 88));
    store.apply_add_entity(protocol_add_entity_with_type(66, 91));
    store.apply_add_entity(protocol_add_entity_with_type(67, 111));
    store.apply_add_entity(protocol_add_entity_with_type(68, 129));
    store.apply_add_entity(protocol_add_entity_with_type(69, 148));
    store.apply_add_entity(protocol_add_entity_with_type(70, 149));

    assert_eq!(
        store.probe_entity_pick_bounds(34),
        Some(EntityPickBoundsState::from_base_size(0.9, 0.9, 0.0))
    );
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 34,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(34),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * 0.5,
            0.9 * 0.5,
            0.0
        ))
    );
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 34,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(34),
        Some(EntityPickBoundsState::from_base_size(0.9, 0.9, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 35,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(35),
        Some(EntityPickBoundsState::from_base_size(0.3, 0.4, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 52,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(52),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * 0.65,
            0.6 * 0.65,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 36,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(36),
        Some(EntityPickBoundsState::from_base_size(0.49, 0.99, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 37,
        values: vec![protocol_bool_data(ZOMBIE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(37),
        Some(EntityPickBoundsState::from_base_size(0.49, 0.99, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 38,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(38),
        Some(EntityPickBoundsState::from_base_size(0.6, 1.95, 0.0))
    );
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 38,
        values: vec![protocol_bool_data(PIGLIN_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(38),
        Some(EntityPickBoundsState::from_base_size(0.49, 0.99, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 39,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(39),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * 0.5,
            1.4 * 0.5,
            0.0
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 40,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(40),
        Some(EntityPickBoundsState::from_base_size(
            0.6 * 0.5,
            1.95 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 45,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(45),
        Some(EntityPickBoundsState::from_base_size(
            0.7 * 0.6,
            0.65 * 0.6,
            0.0,
        ))
    );
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 45,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(45),
        Some(EntityPickBoundsState::from_base_size(
            0.7 * 0.6 * 2.0,
            0.65 * 0.6 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 46,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(46),
        Some(EntityPickBoundsState::from_base_size(
            0.6 * 0.6,
            0.7 * 0.6,
            0.0,
        ))
    );
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 46,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(46),
        Some(EntityPickBoundsState::from_base_size(
            0.6 * 0.6 * 2.0,
            0.7 * 0.6 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 47,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(47),
        Some(EntityPickBoundsState::from_base_size(0.24, 0.4, 0.0))
    );
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 47,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(47),
        Some(EntityPickBoundsState::from_base_size(
            0.24 * 2.0,
            0.4 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 71,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(71),
        Some(EntityPickBoundsState::from_base_size(
            1.4 * 0.5,
            1.4 * 0.5,
            0.0,
        ))
    );
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 71,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(71),
        Some(EntityPickBoundsState::from_base_size(
            1.4 * 0.5 * 2.0,
            1.4 * 0.5 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 53,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(53),
        Some(EntityPickBoundsState::from_base_size(
            4.0 * 0.2375,
            4.0 * 0.2375,
            0.0,
        ))
    );
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 53,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(53),
        Some(EntityPickBoundsState::from_base_size(
            4.0 * 0.2375,
            4.0 * 0.2375,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 48,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(48),
        Some(EntityPickBoundsState::from_base_size(0.5, 0.25, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 49,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(49),
        Some(EntityPickBoundsState::from_base_size(0.5, 0.63, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(50),
        Some(EntityPickBoundsState::from_base_size(0.5, 0.63, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 54,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(54),
        Some(EntityPickBoundsState::from_base_size(
            1.3 * 0.5,
            1.25 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 55,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(55),
        Some(EntityPickBoundsState::from_base_size(
            1.3964844 * 0.5,
            1.5 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 56,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(56),
        Some(EntityPickBoundsState::from_base_size(
            1.3964844 * 0.5,
            1.6 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 57,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(57),
        Some(EntityPickBoundsState::from_base_size(
            1.3964844 * 0.7,
            1.6 * 0.7,
            0.0,
        ))
    );
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 57,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(57),
        Some(EntityPickBoundsState::from_base_size(
            1.3964844 * 0.7 * 2.0,
            1.6 * 0.7 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 58,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(58),
        Some(EntityPickBoundsState::from_base_size(
            1.3964844 * 0.7,
            1.6 * 0.7,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 59,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(59),
        Some(EntityPickBoundsState::from_base_size(
            1.3964844 * 0.7,
            1.6 * 0.7,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 60,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(60),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * 0.5,
            1.87 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 61,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(61),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * 0.5,
            1.87 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 51,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(51),
        Some(EntityPickBoundsState::from_base_size(
            1.2 * 0.3,
            0.4 * 0.3,
            0.0,
        ))
    );
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 51,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(51),
        Some(EntityPickBoundsState::from_base_size(
            1.2 * 0.3 * 2.0,
            0.4 * 0.3 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 62,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(62),
        Some(EntityPickBoundsState::from_base_size(
            0.7 * 0.5,
            0.6 * 0.5,
            0.0,
        ))
    );
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 62,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(62),
        Some(EntityPickBoundsState::from_base_size(
            0.7 * 0.5 * 2.0,
            0.6 * 0.5 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 63,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(63),
        Some(EntityPickBoundsState::from_base_size(
            0.6 * 0.5,
            0.7 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 64,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(64),
        Some(EntityPickBoundsState::from_base_size(
            1.3964844 * 0.5,
            1.4 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 65,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(65),
        Some(EntityPickBoundsState::from_base_size(
            0.875 * 0.5,
            0.95 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 66,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(66),
        Some(EntityPickBoundsState::from_base_size(
            0.6 * 0.5,
            0.7 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 67,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(67),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * 0.5,
            1.3 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 68,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(68),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * 0.5,
            1.7 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 69,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(69),
        Some(EntityPickBoundsState::from_base_size(
            0.6 * 0.5,
            0.85 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(70),
        Some(EntityPickBoundsState::from_base_size(
            1.3964844 * 0.5,
            1.4 * 0.5,
            0.0,
        ))
    );
}

#[test]
fn shulker_pick_bounds_follow_attach_face_and_peek_metadata() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(82, SHULKER_TYPE_ID));

    assert_pick_bounds_close(
        store.probe_entity_pick_bounds(82),
        shulker_pick_bounds(DIRECTION_DOWN, 0.0, 1.0),
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 82,
        values: vec![shulker_peek_data(100)],
    }));
    store.advance_entity_client_animations(20);
    assert_pick_bounds_close(
        store.probe_entity_pick_bounds(82),
        shulker_pick_bounds(DIRECTION_DOWN, 1.0, 1.0),
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 82,
        values: vec![shulker_attach_face_data(DIRECTION_UP)],
    }));
    assert_pick_bounds_close(
        store.probe_entity_pick_bounds(82),
        shulker_pick_bounds(DIRECTION_UP, 1.0, 1.0),
    );

    for (id, attach_face) in [
        (83, DIRECTION_UP),
        (84, DIRECTION_NORTH),
        (85, DIRECTION_SOUTH),
        (86, DIRECTION_WEST),
        (87, DIRECTION_EAST),
    ] {
        store.apply_add_entity(protocol_add_entity_with_type(id, SHULKER_TYPE_ID));
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![
                shulker_attach_face_data(attach_face),
                shulker_peek_data(100),
            ],
        }));
        store.advance_entity_client_animations(20);
        assert_pick_bounds_close(
            store.probe_entity_pick_bounds(id),
            shulker_pick_bounds(attach_face, 1.0, 1.0),
        );
    }
}

#[test]
fn shulker_peek_pick_bounds_advance_toward_metadata_target() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(88, SHULKER_TYPE_ID));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 88,
        values: vec![shulker_peek_data(100)],
    }));
    store.advance_entity_client_animations(0);
    assert_pick_bounds_close(
        store.probe_entity_pick_bounds(88),
        shulker_pick_bounds(DIRECTION_DOWN, 0.0, 1.0),
    );

    store.advance_entity_client_animations(1);
    assert_pick_bounds_close(
        store.probe_entity_pick_bounds(88),
        shulker_pick_bounds(DIRECTION_DOWN, 0.05, 1.0),
    );

    store.advance_entity_client_animations(9);
    assert_pick_bounds_close(
        store.probe_entity_pick_bounds(88),
        shulker_pick_bounds(DIRECTION_DOWN, 0.5, 1.0),
    );

    let cloned = store.clone();
    assert_pick_bounds_close(
        cloned.probe_entity_pick_bounds(88),
        shulker_pick_bounds(DIRECTION_DOWN, 0.5, 1.0),
    );
    let restored: WorldStore =
        serde_json::from_value(serde_json::to_value(&store).unwrap()).unwrap();
    assert_pick_bounds_close(
        restored.probe_entity_pick_bounds(88),
        shulker_pick_bounds(DIRECTION_DOWN, 0.5, 1.0),
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 88,
        values: vec![shulker_peek_data(0)],
    }));
    store.advance_entity_client_animations(1);
    assert_pick_bounds_close(
        store.probe_entity_pick_bounds(88),
        shulker_pick_bounds(DIRECTION_DOWN, 0.45, 1.0),
    );

    store.advance_entity_client_animations(9);
    assert_pick_bounds_close(
        store.probe_entity_pick_bounds(88),
        shulker_pick_bounds(DIRECTION_DOWN, 0.0, 1.0),
    );
}

#[test]
fn shulker_pick_bounds_apply_vanilla_scale_cap() {
    const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(89, SHULKER_TYPE_ID));

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 89,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 4.0,
            modifiers: Vec::new(),
        }],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 89,
        values: vec![shulker_peek_data(100)],
    }));

    store.advance_entity_client_animations(20);
    assert_pick_bounds_close(
        store.probe_entity_pick_bounds(89),
        shulker_pick_bounds(DIRECTION_DOWN, 1.0, 3.0),
    );
}

#[test]
fn polar_bear_standing_pick_bounds_follow_client_animation_ticks() {
    const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;
    const AGEABLE_BABY_DATA_ID: u8 = 16;
    const POLAR_BEAR_STANDING_DATA_ID: u8 = 18;
    const POLAR_BEAR_TYPE_ID: i32 = 104;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(80, POLAR_BEAR_TYPE_ID));

    let adult_bounds = Some(EntityPickBoundsState::from_base_size(1.4, 1.4, 0.0));
    assert_eq!(store.probe_entity_pick_bounds(80), adult_bounds);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 80,
        values: vec![protocol_bool_data(POLAR_BEAR_STANDING_DATA_ID, true)],
    }));
    assert_eq!(store.probe_entity_pick_bounds(80), adult_bounds);

    store.advance_entity_client_animations(0);
    assert_eq!(store.probe_entity_pick_bounds(80), adult_bounds);

    store.advance_entity_client_animations(1);
    assert_eq!(store.probe_entity_pick_bounds(80), adult_bounds);

    store.advance_entity_client_animations(1);
    assert_eq!(
        store.probe_entity_pick_bounds(80),
        Some(EntityPickBoundsState::from_base_size(
            1.4,
            1.4 * (1.0 + 1.0 / 6.0),
            0.0,
        ))
    );

    store.advance_entity_client_animations(5);
    assert_eq!(
        store.probe_entity_pick_bounds(80),
        Some(EntityPickBoundsState::from_base_size(1.4, 2.8, 0.0))
    );

    let cloned = store.clone();
    assert_eq!(
        cloned.probe_entity_pick_bounds(80),
        Some(EntityPickBoundsState::from_base_size(1.4, 2.8, 0.0))
    );
    let restored: WorldStore =
        serde_json::from_value(serde_json::to_value(&store).unwrap()).unwrap();
    assert_eq!(
        restored.probe_entity_pick_bounds(80),
        Some(EntityPickBoundsState::from_base_size(1.4, 2.8, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 80,
        values: vec![protocol_bool_data(POLAR_BEAR_STANDING_DATA_ID, false)],
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(
        store.probe_entity_pick_bounds(80),
        Some(EntityPickBoundsState::from_base_size(1.4, 2.8, 0.0))
    );

    store.advance_entity_client_animations(1);
    assert_eq!(
        store.probe_entity_pick_bounds(80),
        Some(EntityPickBoundsState::from_base_size(
            1.4,
            1.4 * (1.0 + 5.0 / 6.0),
            0.0,
        ))
    );

    store.advance_entity_client_animations(5);
    assert_eq!(store.probe_entity_pick_bounds(80), adult_bounds);

    store.apply_add_entity(protocol_add_entity_with_type(81, POLAR_BEAR_TYPE_ID));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 81,
        values: vec![
            protocol_bool_data(AGEABLE_BABY_DATA_ID, true),
            protocol_bool_data(POLAR_BEAR_STANDING_DATA_ID, true),
        ],
    }));
    store.advance_entity_client_animations(7);
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 81,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(81),
        Some(EntityPickBoundsState::from_base_size(
            0.7 * 2.0,
            1.4 * 2.0,
            0.0
        ))
    );
}

#[test]
fn advancing_entity_client_animations_in_batches_matches_single_ticks() {
    const POLAR_BEAR_STANDING_DATA_ID: u8 = 18;
    const POLAR_BEAR_TYPE_ID: i32 = 104;

    let mut batch = WorldStore::new();
    batch.apply_add_entity(protocol_add_entity_with_type(90, POLAR_BEAR_TYPE_ID));
    assert!(batch.apply_set_entity_data(ProtocolSetEntityData {
        id: 90,
        values: vec![protocol_bool_data(POLAR_BEAR_STANDING_DATA_ID, true)],
    }));

    let mut repeated = batch.clone();
    batch.advance_entity_client_animations(7);
    for _ in 0..7 {
        repeated.advance_entity_client_animations(1);
    }

    assert_eq!(
        batch.probe_entity_pick_bounds(90),
        repeated.probe_entity_pick_bounds(90)
    );
    assert_eq!(
        batch.probe_entity(90).unwrap().client_animations,
        repeated.probe_entity(90).unwrap().client_animations
    );
}

#[test]
fn armor_stand_pick_bounds_follow_client_flags() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(26, 5));
    store.apply_add_entity(protocol_add_entity_with_type(27, 5));
    store.apply_add_entity(protocol_add_entity_with_type(28, 5));

    assert_eq!(
        store.probe_entity_pick_bounds(26),
        Some(EntityPickBoundsState::from_base_size(0.5, 1.975, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 27,
        values: vec![ProtocolEntityDataValue {
            data_id: 16,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(1),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(27),
        Some(EntityPickBoundsState::from_base_size(
            0.5 * 0.5,
            1.975 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 28,
        values: vec![ProtocolEntityDataValue {
            data_id: 16,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(16),
        }],
    }));
    assert_eq!(store.probe_entity_pick_bounds(28), None);
}

#[test]
fn player_pick_bounds_skip_spectator_profile() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        29,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));

    assert_eq!(
        store.probe_entity_pick_bounds(29),
        Some(EntityPickBoundsState::from_base_size(0.6, 1.8, 0.0))
    );

    store.apply_player_info_update(ProtocolPlayerInfoUpdate {
        actions: vec![
            ProtocolPlayerInfoAction::AddPlayer,
            ProtocolPlayerInfoAction::UpdateGameMode,
        ],
        entries: vec![protocol_player_info_entry_with_mode(
            default_entity_uuid(),
            ProtocolGameType::Spectator,
        )],
    });
    assert_eq!(store.probe_entity_pick_bounds(29), None);

    store.apply_player_info_update(ProtocolPlayerInfoUpdate {
        actions: vec![ProtocolPlayerInfoAction::UpdateGameMode],
        entries: vec![protocol_player_info_entry_with_mode(
            default_entity_uuid(),
            ProtocolGameType::Survival,
        )],
    });
    assert_eq!(
        store.probe_entity_pick_bounds(29),
        Some(EntityPickBoundsState::from_base_size(0.6, 1.8, 0.0))
    );
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
    assert_eq!(store.entities.mount(10).unwrap().passengers, vec![20, 21]);
    assert_eq!(store.entities.mount(20).unwrap().vehicle_id, Some(10));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 30,
        passenger_ids: vec![20],
    }));
    assert_eq!(store.probe_entity(10).unwrap().passengers, vec![21]);
    assert_eq!(store.probe_entity(20).unwrap().vehicle_id, Some(30));
    assert_eq!(store.probe_entity(30).unwrap().passengers, vec![20]);
    assert_eq!(store.entities.mount(20).unwrap().vehicle_id, Some(30));
    assert_eq!(store.entities.mount(30).unwrap().passengers, vec![20]);

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
    assert_eq!(store.entities.mount(20).unwrap().vehicle_id, None);

    assert_eq!(store.counters().entity_passenger_updates_received, 4);
    assert_eq!(store.counters().entity_passenger_ids_received, 6);
    assert_eq!(store.counters().entity_passenger_updates_applied, 3);
    assert_eq!(store.counters().entity_passenger_updates_ignored, 1);
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
    assert_eq!(store.counters().vehicle_moves_ignored, 0);
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
    assert_eq!(store.counters().vehicle_moves_ignored, 1);
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
    assert_eq!(store.counters().vehicle_moves_ignored, 0);
}

#[test]
fn minecart_along_track_updates_entity_from_latest_step() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));
    store.apply_add_entity(protocol_add_entity(20));

    assert!(
        store.apply_move_minecart_along_track(ProtocolMoveMinecartAlongTrack {
            entity_id: 10,
            lerp_steps: vec![
                minecart_step(1.25, 64.1, -2.25, 0.2, 0.0, -0.2, 45.0, -10.0, 0.5),
                minecart_step(1.75, 64.2, -2.75, 0.4, 0.0, -0.4, 90.0, 5.0, 1.25),
            ],
        })
    );

    let entity = store.probe_entity(10).unwrap();
    assert_eq!(
        entity.position,
        EntityVec3 {
            x: 1.75,
            y: 64.2,
            z: -2.75,
        }
    );
    assert_eq!(
        entity.delta_movement,
        EntityVec3 {
            x: 0.4,
            y: 0.0,
            z: -0.4,
        }
    );
    assert_eq!(entity.y_rot, 90.0);
    assert_eq!(entity.x_rot, 5.0);
    assert_eq!(entity.minecart_lerp_steps.len(), 2);
    assert_eq!(store.entities.minecart_lerp(10).unwrap().steps.len(), 2);

    assert!(
        store.apply_move_minecart_along_track(ProtocolMoveMinecartAlongTrack {
            entity_id: 10,
            lerp_steps: vec![minecart_step(
                2.0, 64.3, -3.0, 0.1, 0.0, -0.1, 135.0, 0.0, 1.0,
            )],
        })
    );
    assert_eq!(store.probe_entity(10).unwrap().minecart_lerp_steps.len(), 1);
    assert_eq!(store.entities.minecart_lerp(10).unwrap().steps.len(), 1);

    assert!(
        !store.apply_move_minecart_along_track(ProtocolMoveMinecartAlongTrack {
            entity_id: 999,
            lerp_steps: vec![minecart_step(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0)],
        })
    );
    assert!(
        !store.apply_move_minecart_along_track(ProtocolMoveMinecartAlongTrack {
            entity_id: 20,
            lerp_steps: vec![minecart_step(3.0, 64.0, -4.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0)],
        })
    );
    assert_eq!(store.counters().minecart_moves_received, 4);
    assert_eq!(store.counters().minecart_moves_applied, 2);
    assert_eq!(store.counters().minecart_moves_ignored, 2);
    assert_eq!(store.counters().minecart_lerp_steps_received, 5);
    assert_eq!(store.counters().minecart_lerp_steps_tracked, 1);
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
    assert_eq!(store.counters().take_item_entities_ignored, 1);
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
    assert_eq!(
        store.entities.transient_events(123).unwrap(),
        EntityTransientEvents {
            last_animation_action: Some(3),
            last_event_id: Some(35),
            last_hurt_yaw: Some(45.5),
        }
    );

    assert!(!store.apply_entity_animation(ProtocolEntityAnimation { id: 999, action: 4 }));
    assert!(!store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 999,
        event_id: 21,
    }));
    assert!(!store.apply_hurt_animation(ProtocolHurtAnimation { id: 999, yaw: 90.0 }));

    assert_eq!(store.counters().entity_animation_updates_received, 2);
    assert_eq!(store.counters().entity_animation_updates_applied, 1);
    assert_eq!(store.counters().entity_animation_updates_ignored, 1);
    assert_eq!(store.counters().entity_events_received, 2);
    assert_eq!(store.counters().entity_events_applied, 1);
    assert_eq!(store.counters().entity_events_ignored, 1);
    assert_eq!(store.counters().entity_hurt_animations_received, 2);
    assert_eq!(store.counters().entity_hurt_animations_applied, 1);
    assert_eq!(store.counters().entity_hurt_animations_ignored, 1);
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
    assert_eq!(store.entities.leash(10).unwrap().holder_id, Some(20));

    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 10,
        dest_id: 999,
    }));
    assert_eq!(store.probe_entity(10).unwrap().leash_holder_id, Some(999));
    assert_eq!(store.entities.leash(10).unwrap().holder_id, Some(999));

    assert!(!store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 999,
        dest_id: 20,
    }));

    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 10,
        dest_id: 0,
    }));
    assert_eq!(store.probe_entity(10).unwrap().leash_holder_id, None);
    assert_eq!(store.entities.leash(10).unwrap().holder_id, None);

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
    assert_eq!(store.entities.leash(10).unwrap().holder_id, None);

    assert_eq!(store.counters().entity_link_updates_received, 5);
    assert_eq!(store.counters().entity_link_updates_applied, 4);
    assert_eq!(store.counters().entity_link_updates_ignored, 1);
}

fn minecart_step(
    x: f64,
    y: f64,
    z: f64,
    xa: f64,
    ya: f64,
    za: f64,
    y_rot: f32,
    x_rot: f32,
    weight: f32,
) -> ProtocolMinecartStep {
    ProtocolMinecartStep {
        position: ProtocolVec3d { x, y, z },
        movement: ProtocolVec3d {
            x: xa,
            y: ya,
            z: za,
        },
        y_rot,
        x_rot,
        weight,
    }
}

const SHULKER_TYPE_ID: i32 = 112;
const SHULKER_ATTACH_FACE_DATA_ID: u8 = 16;
const SHULKER_PEEK_DATA_ID: u8 = 17;
const DIRECTION_DOWN: i32 = 0;
const DIRECTION_UP: i32 = 1;
const DIRECTION_NORTH: i32 = 2;
const DIRECTION_SOUTH: i32 = 3;
const DIRECTION_WEST: i32 = 4;
const DIRECTION_EAST: i32 = 5;

fn protocol_add_entity(id: i32) -> ProtocolAddEntity {
    protocol_add_entity_with_type(id, 7)
}

fn protocol_add_entity_with_type(id: i32, entity_type_id: i32) -> ProtocolAddEntity {
    protocol_add_entity_with_type_data(id, entity_type_id, 99)
}

fn protocol_add_entity_with_type_y_rot(
    id: i32,
    entity_type_id: i32,
    y_rot: f32,
) -> ProtocolAddEntity {
    ProtocolAddEntity {
        y_rot,
        ..protocol_add_entity_with_type(id, entity_type_id)
    }
}

fn protocol_add_entity_with_type_data(
    id: i32,
    entity_type_id: i32,
    data: i32,
) -> ProtocolAddEntity {
    ProtocolAddEntity {
        id,
        uuid: default_entity_uuid(),
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
        data,
    }
}

fn default_entity_uuid() -> Uuid {
    Uuid::from_u128(0x12345678123456781234567812345678)
}

fn wind_charge_pick_bounds() -> EntityPickBoundsState {
    let half_width = 0.3125 / 2.0;
    EntityPickBoundsState {
        min: [-half_width, -0.15, -half_width],
        max: [half_width, -0.15 + 0.3125, half_width],
        pick_radius: 1.0,
    }
}

fn shulker_attach_face_data(direction_id: i32) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id: SHULKER_ATTACH_FACE_DATA_ID,
        serializer_id: 12,
        value: EntityDataValueKind::Direction(direction_id),
    }
}

fn shulker_peek_data(raw_peek: i8) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id: SHULKER_PEEK_DATA_ID,
        serializer_id: 0,
        value: EntityDataValueKind::Byte(raw_peek),
    }
}

fn shulker_pick_bounds(
    attach_face_id: i32,
    client_peek_amount: f32,
    scale: f32,
) -> EntityPickBoundsState {
    let physical_peek = 0.5 - ((0.5 + client_peek_amount) * std::f32::consts::PI).sin() * 0.5;
    let half_size = scale * 0.5;
    let mut min = [-half_size, 0.0, -half_size];
    let mut max = [half_size, scale, half_size];
    let extension = physical_peek * scale;

    match opposite_direction_id(attach_face_id) {
        DIRECTION_DOWN => min[1] -= extension,
        DIRECTION_UP => max[1] += extension,
        DIRECTION_NORTH => min[2] -= extension,
        DIRECTION_SOUTH => max[2] += extension,
        DIRECTION_WEST => min[0] -= extension,
        DIRECTION_EAST => max[0] += extension,
        _ => unreachable!("unexpected vanilla direction id"),
    }

    EntityPickBoundsState {
        min,
        max,
        pick_radius: 0.0,
    }
}

fn opposite_direction_id(direction_id: i32) -> i32 {
    match direction_id {
        DIRECTION_DOWN => DIRECTION_UP,
        DIRECTION_UP => DIRECTION_DOWN,
        DIRECTION_NORTH => DIRECTION_SOUTH,
        DIRECTION_SOUTH => DIRECTION_NORTH,
        DIRECTION_WEST => DIRECTION_EAST,
        DIRECTION_EAST => DIRECTION_WEST,
        _ => unreachable!("unexpected vanilla direction id"),
    }
}

fn assert_pick_bounds_close(
    actual: Option<EntityPickBoundsState>,
    expected: EntityPickBoundsState,
) {
    const EPSILON: f32 = 0.000_01;

    let actual = actual.expect("entity should have pick bounds");
    for axis in 0..3 {
        assert!(
            (actual.min[axis] - expected.min[axis]).abs() <= EPSILON,
            "min[{axis}] expected {:?}, got {:?}",
            expected.min,
            actual.min,
        );
        assert!(
            (actual.max[axis] - expected.max[axis]).abs() <= EPSILON,
            "max[{axis}] expected {:?}, got {:?}",
            expected.max,
            actual.max,
        );
    }
    assert!(
        (actual.pick_radius - expected.pick_radius).abs() <= EPSILON,
        "pick_radius expected {}, got {}",
        expected.pick_radius,
        actual.pick_radius,
    );
}

fn assert_entity_vec3_close(actual: EntityVec3, expected: EntityVec3) {
    const EPSILON: f64 = 0.000_000_1;

    assert!(
        (actual.x - expected.x).abs() <= EPSILON,
        "x: expected {}, got {}",
        expected.x,
        actual.x
    );
    assert!(
        (actual.y - expected.y).abs() <= EPSILON,
        "y: expected {}, got {}",
        expected.y,
        actual.y
    );
    assert!(
        (actual.z - expected.z).abs() <= EPSILON,
        "z: expected {}, got {}",
        expected.z,
        actual.z
    );
}

fn pick_target(targets: &[EntityPickTargetState], entity_id: i32) -> &EntityPickTargetState {
    targets
        .iter()
        .find(|target| target.entity_id == entity_id)
        .unwrap_or_else(|| panic!("missing pick target {entity_id}"))
}

fn protocol_player_info_entry_with_mode(
    profile_id: Uuid,
    game_mode: ProtocolGameType,
) -> ProtocolPlayerInfoEntry {
    ProtocolPlayerInfoEntry {
        profile_id,
        profile: Some(ProtocolGameProfile {
            uuid: profile_id,
            name: "PickTarget".to_string(),
            properties: Vec::new(),
        }),
        listed: true,
        latency: 0,
        game_mode,
        display_name: None,
        show_hat: true,
        list_order: 0,
        chat_session: None,
    }
}

fn protocol_pose_data(data_id: u8, pose_id: i32) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id: 20,
        value: EntityDataValueKind::Pose(pose_id),
    }
}

fn protocol_bool_data(data_id: u8, value: bool) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id: 8,
        value: EntityDataValueKind::Boolean(value),
    }
}

fn protocol_int_data(data_id: u8, value: i32) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id: 1,
        value: EntityDataValueKind::Int(value),
    }
}

fn protocol_enum_data(
    data_id: u8,
    serializer: EntityDataEnumSerializer,
    id: i32,
) -> ProtocolEntityDataValue {
    let serializer_id = match serializer {
        EntityDataEnumSerializer::SnifferState => 35,
        EntityDataEnumSerializer::ArmadilloState => 36,
        EntityDataEnumSerializer::CopperGolemState => 37,
        EntityDataEnumSerializer::WeatheringCopperState => 38,
    };

    ProtocolEntityDataValue {
        data_id,
        serializer_id,
        value: EntityDataValueKind::EnumId { serializer, id },
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
