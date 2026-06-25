use super::*;

use bbb_protocol::packets::{
    AddEntity as ProtocolAddEntity, AttributeModifier as ProtocolAttributeModifier,
    AttributeSnapshot as ProtocolAttributeSnapshot, BlockPos as ProtocolBlockPos,
    BlockUpdate as ProtocolBlockUpdate, CommonPlayerSpawnInfo as ProtocolSpawnInfo,
    DamageEvent as ProtocolDamageEvent, DataComponentPatchSummary,
    EntityAnimation as ProtocolEntityAnimation, EntityDataEnumSerializer,
    EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind,
    EntityEvent as ProtocolEntityEvent, EntityMove as ProtocolEntityMove,
    EntityPositionSync as ProtocolEntityPositionSync, EquipmentSlot, EquipmentSlotUpdate,
    GameProfile as ProtocolGameProfile, GameType as ProtocolGameType,
    HurtAnimation as ProtocolHurtAnimation, InteractionHand, ItemStackSummary,
    ItemStackSummary as ProtocolItemStackSummary, MinecartStep as ProtocolMinecartStep,
    MoveMinecartAlongTrack as ProtocolMoveMinecartAlongTrack, MoveVehicle as ProtocolMoveVehicle,
    PlayLogin as ProtocolPlayLogin, PlayerInfoAction as ProtocolPlayerInfoAction,
    PlayerInfoEntry as ProtocolPlayerInfoEntry, PlayerInfoUpdate as ProtocolPlayerInfoUpdate,
    RemoveEntities as ProtocolRemoveEntities, RotateHead as ProtocolRotateHead,
    SetEntityData as ProtocolSetEntityData, SetEntityLink as ProtocolSetEntityLink,
    SetEntityMotion as ProtocolSetEntityMotion, SetEquipment as ProtocolSetEquipment,
    SetPassengers as ProtocolSetPassengers, SetPlayerInventory as ProtocolSetPlayerInventory,
    TakeItemEntity as ProtocolTakeItemEntity, TeleportEntity as ProtocolTeleportEntity,
    UpdateAttributes as ProtocolUpdateAttributes, Vec3d as ProtocolVec3d, PLAYER_RELATIVE_DELTA_Y,
    PLAYER_RELATIVE_X,
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
fn entity_model_sources_project_narrow_render_state_from_pick_targets() {
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
    const AGEABLE_BABY_DATA_ID: u8 = 16;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type_y_rot(
        35,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        135.0,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 35,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));

    let sources = store.entity_model_sources_at_partial_tick(1.0);

    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].entity_id, 35);
    assert_eq!(sources[0].entity_type_id, VANILLA_ENTITY_TYPE_CHICKEN_ID);
    assert_eq!(
        sources[0].position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert_eq!(sources[0].y_rot, 135.0);
    assert_eq!(sources[0].age_ticks, 0);
    assert_eq!(
        sources[0].data_values,
        vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)]
    );

    store.advance_entity_client_animations(3);
    let sources = store.entity_model_sources_at_partial_tick(0.5);
    assert_eq!(sources[0].age_ticks, 3);
}

#[test]
fn entity_model_sources_project_worn_armor_materials() {
    use std::collections::BTreeMap;
    const VANILLA_ENTITY_TYPE_ZOMBIE_ID: i32 = 150;
    // The registry-derived item id → armor material table (installed by the native layer).
    let iron_helmet = 800;
    let iron_chestplate = 801;
    let diamond_leggings = 802;
    let gold_boots = 803;
    let stone_sword = 900;

    let mut store = WorldStore::new();
    store.set_item_armor_materials(BTreeMap::from([
        (iron_helmet, ArmorMaterialKind::Iron),
        (iron_chestplate, ArmorMaterialKind::Iron),
        (diamond_leggings, ArmorMaterialKind::Diamond),
        (gold_boots, ArmorMaterialKind::Gold),
    ]));
    store.apply_add_entity(protocol_add_entity_with_type(
        50,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));

    // A bare zombie projects no worn armor.
    let bare = store.entity_model_sources_at_partial_tick(1.0);
    assert_eq!(bare[0].head_armor, None);
    assert_eq!(bare[0].chest_armor, None);

    fn armor_item(item_id: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: Default::default(),
        }
    }

    // Equip all four armor slots; a held sword fills MainHand but is not armor.
    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 50,
        slots: vec![
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: armor_item(iron_helmet),
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Chest,
                item: armor_item(iron_chestplate),
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Legs,
                item: armor_item(diamond_leggings),
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Feet,
                item: armor_item(gold_boots),
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: armor_item(stone_sword),
            },
        ],
    }));

    let sources = store.entity_model_sources_at_partial_tick(1.0);
    assert_eq!(sources[0].head_armor, Some(ArmorMaterialKind::Iron));
    assert_eq!(sources[0].chest_armor, Some(ArmorMaterialKind::Iron));
    assert_eq!(sources[0].legs_armor, Some(ArmorMaterialKind::Diamond));
    assert_eq!(sources[0].feet_armor, Some(ArmorMaterialKind::Gold));

    // A non-armor item (the held sword, absent from the armor map) leaves its slot bare; clearing the
    // helmet (empty stack) drops the head armor.
    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 50,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Head,
            item: ItemStackSummary::empty(),
        }],
    }));
    let sources = store.entity_model_sources_at_partial_tick(1.0);
    assert_eq!(sources[0].head_armor, None);
    assert_eq!(sources[0].chest_armor, Some(ArmorMaterialKind::Iron));
}

#[test]
fn entity_model_sources_project_worn_armor_dye_colors() {
    use std::collections::BTreeMap;
    const VANILLA_ENTITY_TYPE_ZOMBIE_ID: i32 = 150;
    let leather_chestplate = 810;
    let leather_boots = 811;
    let iron_helmet = 812;
    let dye = 0x3F_6CDA;

    let mut store = WorldStore::new();
    store.set_item_armor_materials(BTreeMap::from([
        (leather_chestplate, ArmorMaterialKind::Leather),
        (leather_boots, ArmorMaterialKind::Leather),
        (iron_helmet, ArmorMaterialKind::Iron),
    ]));
    store.apply_add_entity(protocol_add_entity_with_type(
        51,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));

    fn dyed_armor_item(item_id: i32, dye: Option<i32>) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: DataComponentPatchSummary {
                dyed_color: dye,
                ..Default::default()
            },
        }
    }

    // A custom-dyed leather chestplate, an undyed leather boot, and an iron helmet (non-dyeable).
    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 51,
        slots: vec![
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Chest,
                item: dyed_armor_item(leather_chestplate, Some(dye)),
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Feet,
                item: dyed_armor_item(leather_boots, None),
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: dyed_armor_item(iron_helmet, None),
            },
        ],
    }));

    let sources = store.entity_model_sources_at_partial_tick(1.0);
    // The dyed leather chestplate carries its `dyed_color`; the undyed leather boot and the bare-of-dye
    // iron helmet carry None (each paired with its resolved material).
    assert_eq!(sources[0].chest_armor, Some(ArmorMaterialKind::Leather));
    assert_eq!(sources[0].chest_armor_dye, Some(dye));
    assert_eq!(sources[0].feet_armor, Some(ArmorMaterialKind::Leather));
    assert_eq!(sources[0].feet_armor_dye, None);
    assert_eq!(sources[0].head_armor, Some(ArmorMaterialKind::Iron));
    assert_eq!(sources[0].head_armor_dye, None);
    assert_eq!(sources[0].legs_armor_dye, None);
}

#[test]
fn entity_model_sources_project_in_water_from_world_fluid() {
    // Vanilla `LivingEntityRenderState.isInWater = entity.isInWater()`: the scene projects
    // the `wasTouchingWater` overlap of the entity's world AABB against the chunk fluid
    // state. A cod (0.5 × 0.3 box) submerged in a water source block is in water; the same
    // cod in air is not.
    const VANILLA_ENTITY_TYPE_COD_ID: i32 = 27;
    const SOURCE_WATER_BLOCK_STATE_ID: i32 = 86;

    let mut store = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    store.insert_decoded_chunk(empty_test_chunk());
    store.apply_add_entity(ProtocolAddEntity {
        id: 50,
        uuid: default_entity_uuid(),
        entity_type_id: VANILLA_ENTITY_TYPE_COD_ID,
        position: ProtocolVec3d {
            x: 8.5,
            y: 2.0,
            z: 8.5,
        },
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 99,
    });

    let dry = store.entity_model_sources_at_partial_tick(1.0);
    assert_eq!(dry.len(), 1);
    assert!(!dry[0].in_water, "a cod in air is not in water");

    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 8, y: 2, z: 8 },
        block_state_id: SOURCE_WATER_BLOCK_STATE_ID,
    }));
    let wet = store.entity_model_sources_at_partial_tick(1.0);
    assert_eq!(wet.len(), 1);
    assert!(wet[0].in_water, "a cod inside a water column is in water");
}

#[test]
fn entity_model_sources_project_on_ground_from_movement() {
    // Vanilla `Entity.onGround()`: the scene projects the entity's last synced movement ground
    // flag (combined with `isInWater` to drive the `TurtleRenderer` walk/swim branch). It
    // defaults to `false` until a movement packet sets it.
    const VANILLA_ENTITY_TYPE_TURTLE_ID: i32 = 137;

    let on_ground = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == 60)
            .unwrap()
            .on_ground
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        60,
        VANILLA_ENTITY_TYPE_TURTLE_ID,
    ));
    assert!(
        !on_ground(&store),
        "a freshly spawned entity defaults to not on ground"
    );

    assert!(store.apply_entity_move(ProtocolEntityMove {
        id: 60,
        delta_x: 0,
        delta_y: 0,
        delta_z: 0,
        y_rot: None,
        x_rot: None,
        on_ground: true,
    }));
    assert!(
        on_ground(&store),
        "a grounded movement packet projects on_ground"
    );
}

#[test]
fn entity_model_sources_project_is_moving_from_velocity() {
    // Vanilla `DolphinRenderState.isMoving` (`getDeltaMovement().horizontalDistanceSqr() > 1e-7`):
    // the scene projects the entity's synced velocity into the swim-animation gate.
    const VANILLA_ENTITY_TYPE_DOLPHIN_ID: i32 = 35;

    let is_moving = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == 61)
            .unwrap()
            .is_moving
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        61,
        VANILLA_ENTITY_TYPE_DOLPHIN_ID,
    ));
    assert!(
        !is_moving(&store),
        "a freshly spawned entity defaults to not moving"
    );

    assert!(store.apply_set_entity_motion(ProtocolSetEntityMotion {
        id: 61,
        delta_movement: ProtocolVec3d {
            x: 0.1,
            y: 0.5,
            z: -0.1,
        },
    }));
    assert!(
        is_moving(&store),
        "a horizontal velocity above 1e-7 projects is_moving"
    );

    // A purely vertical velocity (`horizontalDistanceSqr == 0`) is not moving.
    assert!(store.apply_set_entity_motion(ProtocolSetEntityMotion {
        id: 61,
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.5,
            z: 0.0,
        },
    }));
    assert!(
        !is_moving(&store),
        "a purely vertical velocity is not horizontally moving"
    );
}

#[test]
fn entity_model_sources_project_hurt_overlay_for_ten_ticks() {
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;

    let red_overlay = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == 40)
            .unwrap()
            .has_red_overlay
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        40,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(!red_overlay(&store));

    // Vanilla animateHurt sets hurtTime = hurtDuration = 10, so hasRedOverlay
    // stays true through the next 9 client ticks and clears on the 10th.
    assert!(store.apply_hurt_animation(ProtocolHurtAnimation { id: 40, yaw: 0.0 }));
    assert!(red_overlay(&store));
    store.advance_entity_client_animations(9);
    assert!(red_overlay(&store));
    store.advance_entity_client_animations(1);
    assert!(!red_overlay(&store));

    // A damage event re-triggers the same hurtTime countdown.
    assert!(store.apply_damage_event(ProtocolDamageEvent {
        entity_id: 40,
        source_type_id: 0,
        source_cause_id: 0,
        source_direct_id: 0,
        source_position: None,
    }));
    assert!(red_overlay(&store));
}

#[test]
fn entity_model_sources_project_attack_swing_ramp() {
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;

    let attack = |store: &WorldStore, partial: f32| {
        let source = store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 50)
            .unwrap();
        (source.attack_anim, source.attack_arm_off_hand)
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        50,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert_eq!(attack(&store, 1.0), (0.0, false));

    // Vanilla `ClientboundAnimate` action 0 = swing main hand → `LivingEntity.swing` arms the
    // 6-tick ramp. `updateSwingTime` then ramps `attackAnim` 0, 1/6, .. 5/6 over ticks 1..6 (the
    // current-tick value is read at partialTick 1.0; partialTick 0.0 yields the previous tick's).
    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 50, action: 0 }));
    store.advance_entity_client_animations(1); // tick 1: swingTime 0 → 0
    assert_eq!(attack(&store, 1.0).0, 0.0);
    store.advance_entity_client_animations(1); // tick 2: swingTime 1 → 1/6
    assert!((attack(&store, 1.0).0 - 1.0 / 6.0).abs() < 1e-6);

    store.advance_entity_client_animations(4); // through tick 6: swingTime 5 → 5/6 (prev 4/6)
    assert!((attack(&store, 1.0).0 - 5.0 / 6.0).abs() < 1e-6);
    assert!((attack(&store, 0.0).0 - 4.0 / 6.0).abs() < 1e-6);
    // The partial tick lerps between the previous and current attackAnim (vanilla getAttackAnim).
    assert!((attack(&store, 0.5).0 - 0.75).abs() < 1e-6);

    store.advance_entity_client_animations(1); // tick 7: swingTime hits 6 → reset, swinging stops
    assert_eq!(attack(&store, 1.0).0, 0.0);
    store.advance_entity_client_animations(1); // tick 8: the decayed swing state is dropped
    assert_eq!(attack(&store, 1.0), (0.0, false));

    // Action 3 = off-hand swing → the off (left) arm is flagged.
    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 50, action: 3 }));
    store.advance_entity_client_animations(2);
    assert!(attack(&store, 1.0).1, "off-hand swing flags the left arm");
}

#[test]
fn entity_model_sources_project_death_animation_counter() {
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
    const VANILLA_ENTITY_HEALTH_DATA_ID: u8 = 9;
    const FLOAT_SERIALIZER_ID: i32 = 3;

    let source = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 60)
            .unwrap()
    };
    let set_health = |store: &mut WorldStore, health: f32| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id: 60,
            values: vec![ProtocolEntityDataValue {
                data_id: VANILLA_ENTITY_HEALTH_DATA_ID,
                serializer_id: FLOAT_SERIALIZER_ID,
                value: EntityDataValueKind::Float(health),
            }],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        60,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    // A healthy living entity is not dying.
    assert!(set_health(&mut store, 4.0));
    store.advance_entity_client_animations(3);
    assert_eq!(source(&store, 0.0).death_time, 0.0);
    assert!(!source(&store, 0.0).has_red_overlay);

    // Vanilla isDeadOrDying(): health <= 0 begins the death animation. Before the
    // first tickDeath, deathTime is 0, so the model is upright and not yet red.
    assert!(set_health(&mut store, 0.0));
    assert_eq!(source(&store, 0.0).death_time, 0.0);
    assert!(!source(&store, 0.0).has_red_overlay);

    // tickDeath increments deathTime each client tick; the projected value lerps
    // by the partial tick (entity.deathTime + partialTick) and drives the red
    // overlay (hasRedOverlay = deathTime > 0).
    store.advance_entity_client_animations(1);
    assert_eq!(source(&store, 0.0).death_time, 1.0);
    assert_eq!(source(&store, 0.5).death_time, 1.5);
    assert!(source(&store, 0.0).has_red_overlay);

    store.advance_entity_client_animations(10);
    assert_eq!(source(&store, 0.0).death_time, 11.0);

    // The counter caps at 20 (vanilla removes the entity at deathTime >= 20).
    store.advance_entity_client_animations(20);
    assert_eq!(source(&store, 0.0).death_time, 20.0);
    store.advance_entity_client_animations(5);
    assert_eq!(source(&store, 0.0).death_time, 20.0);

    // Restoring health clears the death animation (the model stands back up).
    assert!(set_health(&mut store, 6.0));
    assert_eq!(source(&store, 0.0).death_time, 0.0);
    assert!(!source(&store, 0.0).has_red_overlay);
}

#[test]
fn entity_model_sources_project_full_freeze_for_living_entities() {
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
    const VANILLA_ENTITY_TYPE_OAK_BOAT_ID: i32 = 89;
    const VANILLA_ENTITY_TICKS_FROZEN_DATA_ID: u8 = 7;
    const INT_SERIALIZER_ID: i32 = 1;

    let fully_frozen = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .is_fully_frozen
    };
    let set_ticks_frozen = |store: &mut WorldStore, id: i32, ticks: i32| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![ProtocolEntityDataValue {
                data_id: VANILLA_ENTITY_TICKS_FROZEN_DATA_ID,
                serializer_id: INT_SERIALIZER_ID,
                value: EntityDataValueKind::Int(ticks),
            }],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        70,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(!fully_frozen(&store, 70));

    // Vanilla Entity.isFullyFrozen(): ticksFrozen >= getTicksRequiredToFreeze()
    // (140). One tick below the threshold is not yet fully frozen.
    assert!(set_ticks_frozen(&mut store, 70, 139));
    assert!(!fully_frozen(&store, 70));
    assert!(set_ticks_frozen(&mut store, 70, 140));
    assert!(fully_frozen(&store, 70));

    // A non-living entity (boat) never counts as fully frozen even past the
    // threshold: only LivingEntityRenderer shakes.
    store.apply_add_entity(protocol_add_entity_with_type(
        71,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(set_ticks_frozen(&mut store, 71, 200));
    assert!(!fully_frozen(&store, 71));
}

#[test]
fn entity_model_sources_project_auto_spin_attack_flag() {
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
    const VANILLA_ENTITY_TYPE_OAK_BOAT_ID: i32 = 89;
    // Vanilla LivingEntity.LIVING_ENTITY_FLAG_SPIN_ATTACK (4); IS_USING is bit 1.
    const LIVING_ENTITY_FLAG_SPIN_ATTACK: i8 = 4;
    const LIVING_ENTITY_FLAG_IS_USING: i8 = 1;

    let auto_spin = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .is_auto_spin_attack
    };
    let set_flags = |store: &mut WorldStore, id: i32, flags: i8| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![living_entity_flags_data(flags)],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        72,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    // A living entity with no living-entity flags is not spinning.
    assert!(!auto_spin(&store, 72));

    // Vanilla LivingEntity.isAutoSpinAttack(): (DATA_LIVING_ENTITY_FLAGS & 4) != 0.
    // The bit is detected even alongside other living-entity flags.
    assert!(set_flags(
        &mut store,
        72,
        LIVING_ENTITY_FLAG_SPIN_ATTACK | LIVING_ENTITY_FLAG_IS_USING,
    ));
    assert!(auto_spin(&store, 72));

    // Clearing the spin bit (other flags still set) stops the spin.
    assert!(set_flags(&mut store, 72, LIVING_ENTITY_FLAG_IS_USING));
    assert!(!auto_spin(&store, 72));

    // A non-living entity (boat) never spins even with a stray spin-attack bit at
    // the living-entity-flags id: only LivingEntityRenderer reads it.
    store.apply_add_entity(protocol_add_entity_with_type(
        73,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(set_flags(&mut store, 73, LIVING_ENTITY_FLAG_SPIN_ATTACK));
    assert!(!auto_spin(&store, 73));
}

#[test]
fn entity_model_sources_project_aggressive_for_zombie_model_family() {
    const VANILLA_ENTITY_TYPE_ZOMBIE_ID: i32 = 150;
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
    // Vanilla Mob.DATA_MOB_FLAGS_ID (15); MOB_FLAG_AGGRESSIVE (4), LEFTHANDED is bit 2.
    const VANILLA_MOB_FLAGS_DATA_ID: u8 = 15;
    const MOB_FLAG_AGGRESSIVE: i8 = 4;
    const MOB_FLAG_LEFTHANDED: i8 = 2;

    let aggressive = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .is_aggressive
    };
    let set_mob_flags = |store: &mut WorldStore, id: i32, flags: i8| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![ProtocolEntityDataValue {
                data_id: VANILLA_MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(flags),
            }],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));
    // A zombie with no mob flags is calm.
    assert!(!aggressive(&store, 80));

    // Vanilla Mob.isAggressive(): (DATA_MOB_FLAGS_ID & 4) != 0, detected alongside other flags.
    assert!(set_mob_flags(
        &mut store,
        80,
        MOB_FLAG_AGGRESSIVE | MOB_FLAG_LEFTHANDED,
    ));
    assert!(aggressive(&store, 80));
    // Clearing the aggressive bit (left-handed still set) returns to calm.
    assert!(set_mob_flags(&mut store, 80, MOB_FLAG_LEFTHANDED));
    assert!(!aggressive(&store, 80));

    // A chicken is a Mob too (it carries the mob-flags byte), but it does not render with the
    // zombie model's `animateZombieArms`, so the projection is gated out: a stray aggressive
    // bit never reaches the chicken's render state.
    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(set_mob_flags(&mut store, 81, MOB_FLAG_AGGRESSIVE));
    assert!(!aggressive(&store, 81));
}

#[test]
fn entity_model_sources_project_enderman_carrying_and_creepy() {
    const VANILLA_ENTITY_TYPE_ENDERMAN_ID: i32 = 41;
    const VANILLA_ENTITY_TYPE_ZOMBIE_ID: i32 = 150;
    // Vanilla Enderman accessors: DATA_CARRY_STATE (16, OPTIONAL_BLOCK_STATE serializer 15),
    // DATA_CREEPY (17, BOOLEAN serializer 8).
    const CARRY_STATE_DATA_ID: u8 = 16;
    const CREEPY_DATA_ID: u8 = 17;
    const OPTIONAL_BLOCK_STATE_SERIALIZER_ID: i32 = 15;
    const BOOLEAN_SERIALIZER_ID: i32 = 8;

    let source = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
    };
    let set_carry_and_creepy =
        |store: &mut WorldStore, id: i32, block: Option<i32>, creepy: bool| {
            store.apply_set_entity_data(ProtocolSetEntityData {
                id,
                values: vec![
                    ProtocolEntityDataValue {
                        data_id: CARRY_STATE_DATA_ID,
                        serializer_id: OPTIONAL_BLOCK_STATE_SERIALIZER_ID,
                        value: EntityDataValueKind::OptionalBlockState(block),
                    },
                    ProtocolEntityDataValue {
                        data_id: CREEPY_DATA_ID,
                        serializer_id: BOOLEAN_SERIALIZER_ID,
                        value: EntityDataValueKind::Boolean(creepy),
                    },
                ],
            })
        };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        90,
        VANILLA_ENTITY_TYPE_ENDERMAN_ID,
    ));
    // A freshly spawned enderman carries nothing and is not creepy.
    let calm = source(&store, 90);
    assert!(!calm.enderman_carrying);
    assert!(!calm.enderman_creepy);

    // A present carried block (non-zero state id → `Some`) poses the arms; `isCreepy` true
    // drops the head.
    assert!(set_carry_and_creepy(&mut store, 90, Some(10), true));
    let primed = source(&store, 90);
    assert!(primed.enderman_carrying);
    assert!(primed.enderman_creepy);

    // Dropping the block (empty optional) and clearing creepy returns to rest.
    assert!(set_carry_and_creepy(&mut store, 90, None, false));
    let rest = source(&store, 90);
    assert!(!rest.enderman_carrying);
    assert!(!rest.enderman_creepy);

    // A zombie does not define the enderman accessors, so even if the same data ids arrive
    // the projection is gated out and both flags stay false.
    store.apply_add_entity(protocol_add_entity_with_type(
        91,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));
    assert!(set_carry_and_creepy(&mut store, 91, Some(10), true));
    let zombie = source(&store, 91);
    assert!(!zombie.enderman_carrying);
    assert!(!zombie.enderman_creepy);
}

#[test]
fn entity_model_sources_project_bat_resting_from_flags() {
    const VANILLA_ENTITY_TYPE_BAT_ID: i32 = 10;
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
    // Vanilla Bat.DATA_ID_FLAGS (16, BYTE); FLAG_RESTING (1).
    const VANILLA_BAT_FLAGS_DATA_ID: u8 = 16;
    const BAT_FLAG_RESTING: i8 = 1;

    let resting = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .bat_resting
    };
    let set_flags = |store: &mut WorldStore, id: i32, flags: i8| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![ProtocolEntityDataValue {
                data_id: VANILLA_BAT_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(flags),
            }],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        70,
        VANILLA_ENTITY_TYPE_BAT_ID,
    ));
    // A bat with no flags is flying.
    assert!(!resting(&store, 70));
    // Setting Bat.FLAG_RESTING (DATA_ID_FLAGS & 1) projects the hanging pose; clearing it
    // returns to flying.
    assert!(set_flags(&mut store, 70, BAT_FLAG_RESTING));
    assert!(resting(&store, 70));
    assert!(set_flags(&mut store, 70, 0));
    assert!(!resting(&store, 70));

    // A chicken carries no bat flags byte; a stray bit at the same data id never reaches its
    // render state.
    store.apply_add_entity(protocol_add_entity_with_type(
        71,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(set_flags(&mut store, 71, BAT_FLAG_RESTING));
    assert!(!resting(&store, 71));
}

#[test]
fn entity_model_sources_project_wither_invulnerable_ticks() {
    const VANILLA_ENTITY_TYPE_WITHER_ID: i32 = 145;
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
    // Vanilla WitherBoss.DATA_ID_INV (19, INT): the spawn-invulnerability countdown.
    const VANILLA_WITHER_INV_DATA_ID: u8 = 19;

    let ticks = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .wither_invulnerable_ticks
    };
    let set_inv = |store: &mut WorldStore, id: i32, value: i32| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![ProtocolEntityDataValue {
                data_id: VANILLA_WITHER_INV_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Int(value),
            }],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_WITHER_ID,
    ));
    // A fully-spawned wither (DATA_ID_INV = 0) projects 0.0.
    assert_eq!(ticks(&store, 80, 0.0), 0.0);
    // A freshly-summoned wither (220) lerps `invulnerableTicks - partialTicks`.
    assert!(set_inv(&mut store, 80, 220));
    assert!((ticks(&store, 80, 0.0) - 220.0).abs() < 1.0e-6);
    assert!((ticks(&store, 80, 0.5) - 219.5).abs() < 1.0e-6);
    // Clearing it returns to 0.0 (a non-positive countdown is not lerped).
    assert!(set_inv(&mut store, 80, 0));
    assert_eq!(ticks(&store, 80, 0.5), 0.0);

    // A chicken carries no DATA_ID_INV accessor; a stray int at the same data id never reaches its
    // render state.
    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(set_inv(&mut store, 81, 220));
    assert_eq!(ticks(&store, 81, 0.0), 0.0);
}

#[test]
fn entity_model_sources_project_bee_stinger_from_flags() {
    const VANILLA_ENTITY_TYPE_BEE_ID: i32 = 11;
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
    // Vanilla Bee.DATA_FLAGS_ID (18, BYTE); FLAG_HAS_STUNG (4).
    const VANILLA_BEE_FLAGS_DATA_ID: u8 = 18;
    const BEE_FLAG_HAS_STUNG: i8 = 4;

    let has_stinger = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .bee_has_stinger
    };
    let set_flags = |store: &mut WorldStore, id: i32, flags: i8| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![ProtocolEntityDataValue {
                data_id: VANILLA_BEE_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(flags),
            }],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        72,
        VANILLA_ENTITY_TYPE_BEE_ID,
    ));
    // A fresh bee has not stung, so it keeps its stinger.
    assert!(has_stinger(&store, 72));
    // Setting Bee.hasStung (DATA_FLAGS_ID & 4) hides the stinger; clearing it restores it.
    assert!(set_flags(&mut store, 72, BEE_FLAG_HAS_STUNG));
    assert!(!has_stinger(&store, 72));
    assert!(set_flags(&mut store, 72, 0));
    assert!(has_stinger(&store, 72));

    // A non-bee keeps the `true` stinger default regardless of a stray bit at the same data id.
    store.apply_add_entity(protocol_add_entity_with_type(
        73,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(set_flags(&mut store, 73, BEE_FLAG_HAS_STUNG));
    assert!(has_stinger(&store, 73));
}

#[test]
fn entity_model_sources_gate_crouch_pose_on_the_player() {
    const VANILLA_ENTITY_TYPE_PLAYER_ID: i32 = 155;
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
    const POSE_STANDING: i32 = 0;
    const POSE_CROUCHING: i32 = 5;

    let crouching = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .is_crouching
    };
    let set_pose = |store: &mut WorldStore, id: i32, pose: i32| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![protocol_pose_data(
                super::dimensions::ENTITY_DATA_POSE_ID,
                pose,
            )],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        74,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    // A standing player is not crouching.
    assert!(!crouching(&store, 74));
    // Vanilla Pose.CROUCHING marks the player sneaking; standing again clears it.
    assert!(set_pose(&mut store, 74, POSE_CROUCHING));
    assert!(crouching(&store, 74));
    assert!(set_pose(&mut store, 74, POSE_STANDING));
    assert!(!crouching(&store, 74));

    // A non-player entity is never crouched, even with a CROUCHING pose: only the player model
    // has the `HumanoidModel.setupAnim` crouch.
    store.apply_add_entity(protocol_add_entity_with_type(
        75,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(set_pose(&mut store, 75, POSE_CROUCHING));
    assert!(!crouching(&store, 75));
}

#[test]
fn entity_model_sources_project_dinnerbone_upside_down() {
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
    const VANILLA_ENTITY_TYPE_OAK_BOAT_ID: i32 = 89;
    const VANILLA_ENTITY_TYPE_PLAYER_ID: i32 = 155;
    const VANILLA_ENTITY_CUSTOM_NAME_DATA_ID: u8 = 2;
    const OPTIONAL_COMPONENT_SERIALIZER_ID: i32 = 6;

    let source = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
    };
    let set_custom_name = |store: &mut WorldStore, id: i32, name: Option<&str>| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![ProtocolEntityDataValue {
                data_id: VANILLA_ENTITY_CUSTOM_NAME_DATA_ID,
                serializer_id: OPTIONAL_COMPONENT_SERIALIZER_ID,
                value: EntityDataValueKind::OptionalComponent(name.map(str::to_string)),
            }],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    // A normally-named living entity is upright but still carries a real bb height.
    assert!(!source(&store, 80).is_upside_down);
    assert!(source(&store, 80).bounding_box_height > 0.0);

    // Vanilla LivingEntityRenderer.isUpsideDownName: "Dinnerbone" and "Grumm" flip.
    assert!(set_custom_name(&mut store, 80, Some("Dinnerbone")));
    assert!(source(&store, 80).is_upside_down);
    assert!(set_custom_name(&mut store, 80, Some("Grumm")));
    assert!(source(&store, 80).is_upside_down);
    // Any other name (or clearing it) leaves the entity upright.
    assert!(set_custom_name(&mut store, 80, Some("Dinnerbon")));
    assert!(!source(&store, 80).is_upside_down);
    assert!(set_custom_name(&mut store, 80, None));
    assert!(!source(&store, 80).is_upside_down);

    // A non-living entity (boat) named Dinnerbone is never flipped: only
    // LivingEntityRenderer reads the easter egg.
    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(set_custom_name(&mut store, 81, Some("Dinnerbone")));
    assert!(!source(&store, 81).is_upside_down);

    // The player path keys off the GameProfile name + cape part (AvatarRenderer),
    // not the custom name, so a player with only a "Dinnerbone" custom name (no
    // player-info profile, no shown cape) stays upright. The profile-driven player
    // flip is covered by `entity_model_sources_project_player_upside_down`.
    store.apply_add_entity(protocol_add_entity_with_type(
        82,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(set_custom_name(&mut store, 82, Some("Dinnerbone")));
    assert!(!source(&store, 82).is_upside_down);
}

#[test]
fn entity_model_sources_project_player_upside_down() {
    // Vanilla AvatarRenderer.isEntityUpsideDown: a Player is flipped only when its
    // cape model part is shown (DATA_PLAYER_MODE_CUSTOMISATION id 16, CAPE bit 0x01)
    // AND its GameProfile name (from the player-info list, not the custom name) is
    // "Dinnerbone"/"Grumm".
    const VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID: u8 = 16;
    const VANILLA_AVATAR_CAPE_PART_MASK: i8 = 0x01;

    let upside_down = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .is_upside_down
    };
    let set_customization = |store: &mut WorldStore, id: i32, mask: i8| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![ProtocolEntityDataValue {
                data_id: VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(mask),
            }],
        })
    };
    let add_player = |store: &mut WorldStore, id: i32, uuid: Uuid| {
        let mut add = protocol_add_entity_with_type(id, VANILLA_ENTITY_TYPE_PLAYER_ID);
        add.uuid = uuid;
        store.apply_add_entity(add);
    };
    let add_profile = |store: &mut WorldStore, uuid: Uuid, name: &str| {
        store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![ProtocolPlayerInfoAction::AddPlayer],
            entries: vec![ProtocolPlayerInfoEntry {
                profile_id: uuid,
                profile: Some(ProtocolGameProfile {
                    uuid,
                    name: name.to_string(),
                    properties: Vec::new(),
                }),
                listed: true,
                latency: 0,
                game_mode: ProtocolGameType::Survival,
                display_name: None,
                show_hat: true,
                list_order: 0,
                chat_session: None,
            }],
        });
    };

    let mut store = WorldStore::new();
    add_player(&mut store, 90, default_entity_uuid());

    // A shown cape but no player-info profile yet: the GameProfile name is unknown,
    // so the player stays upright.
    assert!(set_customization(
        &mut store,
        90,
        VANILLA_AVATAR_CAPE_PART_MASK
    ));
    assert!(!upside_down(&store, 90));

    // Dinnerbone profile + shown cape: flipped.
    add_profile(&mut store, default_entity_uuid(), "Dinnerbone");
    assert!(upside_down(&store, 90));

    // Hiding the cape (CAPE bit clear) suppresses the flip even for Dinnerbone.
    assert!(set_customization(&mut store, 90, 0));
    assert!(!upside_down(&store, 90));

    // Other customization bits without the cape bit also do not flip.
    assert!(set_customization(
        &mut store,
        90,
        !VANILLA_AVATAR_CAPE_PART_MASK
    ));
    assert!(!upside_down(&store, 90));

    // Showing the cape again restores the flip.
    assert!(set_customization(
        &mut store,
        90,
        VANILLA_AVATAR_CAPE_PART_MASK
    ));
    assert!(upside_down(&store, 90));

    // A cape-showing player whose profile name is not Dinnerbone/Grumm is upright.
    let steve_uuid = Uuid::from_u128(0xAAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAA);
    add_player(&mut store, 91, steve_uuid);
    add_profile(&mut store, steve_uuid, "Steve");
    assert!(set_customization(
        &mut store,
        91,
        VANILLA_AVATAR_CAPE_PART_MASK
    ));
    assert!(!upside_down(&store, 91));

    // The other easter-egg name, "Grumm", flips too (cape shown).
    let grumm_uuid = Uuid::from_u128(0xBBBB_BBBB_BBBB_BBBB_BBBB_BBBB_BBBB_BBBB);
    add_player(&mut store, 92, grumm_uuid);
    add_profile(&mut store, grumm_uuid, "Grumm");
    assert!(set_customization(
        &mut store,
        92,
        VANILLA_AVATAR_CAPE_PART_MASK
    ));
    assert!(upside_down(&store, 92));
}

#[test]
fn sleeping_bed_yaw_and_offset_matches_vanilla() {
    let eye = 2.0_f32;
    let ho = eye - 0.1;
    let bed = |facing: &str| {
        let mut props = std::collections::BTreeMap::new();
        props.insert("facing".to_string(), facing.to_string());
        super::sleeping_bed_yaw_and_offset("minecraft:white_bed", &props, eye)
    };
    // Vanilla LivingEntityRenderer.sleepDirectionToRotation + Direction.getStepX/Z;
    // the head-offset translate is [-stepX * (eye - 0.1), -stepZ * (eye - 0.1)].
    assert_eq!(bed("south"), Some((90.0, [0.0, -ho])));
    assert_eq!(bed("west"), Some((0.0, [ho, 0.0])));
    assert_eq!(bed("north"), Some((270.0, [0.0, ho])));
    assert_eq!(bed("east"), Some((180.0, [-ho, 0.0])));

    // A non-bed block, or a bed without a facing, never resolves.
    let mut props = std::collections::BTreeMap::new();
    props.insert("facing".to_string(), "north".to_string());
    assert_eq!(
        super::sleeping_bed_yaw_and_offset("minecraft:stone", &props, eye),
        None
    );
    assert_eq!(
        super::sleeping_bed_yaw_and_offset(
            "minecraft:white_bed",
            &std::collections::BTreeMap::new(),
            eye,
        ),
        None
    );
}

#[test]
fn entity_model_sources_gate_sleeping_pose_on_living_entities() {
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
    const VANILLA_ENTITY_TYPE_OAK_BOAT_ID: i32 = 89;
    const POSE_STANDING: i32 = 0;
    const POSE_SLEEPING: i32 = 2;

    let source = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
    };
    let set_pose = |store: &mut WorldStore, id: i32, pose: i32| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![protocol_pose_data(
                super::dimensions::ENTITY_DATA_POSE_ID,
                pose,
            )],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        90,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    // An awake (standing) entity is not sleeping.
    assert!(!source(&store, 90).is_sleeping);

    // Vanilla Pose.SLEEPING marks the entity sleeping; with no bed resolved the bed
    // yaw/offset stay at the no-bed fallback (the renderer uses the body yaw).
    assert!(set_pose(&mut store, 90, POSE_SLEEPING));
    let asleep = source(&store, 90);
    assert!(asleep.is_sleeping);
    assert_eq!(asleep.sleeping_bed_yaw, None);
    assert_eq!(asleep.sleeping_bed_offset, [0.0, 0.0]);

    // Standing again clears it.
    assert!(set_pose(&mut store, 90, POSE_STANDING));
    assert!(!source(&store, 90).is_sleeping);

    // A non-living entity (boat) with a SLEEPING pose never sleeps: only
    // LivingEntityRenderer lays entities down.
    store.apply_add_entity(protocol_add_entity_with_type(
        91,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(set_pose(&mut store, 91, POSE_SLEEPING));
    assert!(!source(&store, 91).is_sleeping);
}

#[test]
fn entity_model_sources_resolve_sleeping_bed_orientation() {
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
    const POSE_SLEEPING: i32 = 2;
    const SLEEPING_POS_DATA_ID: u8 = 14;
    const OPTIONAL_BLOCK_POS_SERIALIZER_ID: i32 = 11;

    let mut store = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    store.insert_decoded_chunk(empty_test_chunk());

    // Place a north-facing bed and point the entity's sleeping position at it.
    let mut bed_props = std::collections::BTreeMap::new();
    bed_props.insert("facing".to_string(), "north".to_string());
    bed_props.insert("occupied".to_string(), "false".to_string());
    bed_props.insert("part".to_string(), "foot".to_string());
    let bed_id = crate::registries::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:white_bed", &bed_props)
        .expect("vanilla 26.1 north white_bed state exists")
        .id;
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 2, y: 1, z: 2 },
        block_state_id: bed_id,
    }));

    store.apply_add_entity(protocol_add_entity_with_type(
        92,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 92,
        values: vec![
            protocol_pose_data(super::dimensions::ENTITY_DATA_POSE_ID, POSE_SLEEPING),
            ProtocolEntityDataValue {
                data_id: SLEEPING_POS_DATA_ID,
                serializer_id: OPTIONAL_BLOCK_POS_SERIALIZER_ID,
                value: EntityDataValueKind::OptionalBlockPos(Some(ProtocolBlockPos {
                    x: 2,
                    y: 1,
                    z: 2,
                })),
            },
        ],
    }));

    let source = store
        .entity_model_sources_at_partial_tick(0.0)
        .into_iter()
        .find(|source| source.entity_id == 92)
        .unwrap();
    assert!(source.is_sleeping);
    // Vanilla BedBlock.getBedOrientation reads FACING; sleepDirectionToRotation(NORTH) = 270.
    assert_eq!(source.sleeping_bed_yaw, Some(270.0));
    // headOffset = standingEyeHeight - 0.1 > 0; the NORTH step (0, -1) lifts the
    // offset to [0, +headOffset].
    assert!(source.sleeping_bed_offset[0].abs() < 1e-6);
    assert!(source.sleeping_bed_offset[1] > 0.0);
}

#[test]
fn entity_model_sources_project_scale_attribute() {
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
    const VANILLA_ENTITY_TYPE_OAK_BOAT_ID: i32 = 89;
    const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;

    let scale = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .scale
    };
    let set_scale = |store: &mut WorldStore, id: i32, value: f64| {
        store.apply_update_attributes(ProtocolUpdateAttributes {
            entity_id: id,
            attributes: vec![ProtocolAttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
                base: value,
                modifiers: Vec::new(),
            }],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        95,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    // No SCALE attribute synced -> vanilla getScale() default of 1.0.
    assert_eq!(scale(&store, 95), 1.0);

    // Vanilla LivingEntity.getScale() is the SCALE attribute value.
    assert!(set_scale(&mut store, 95, 1.5));
    assert_eq!(scale(&store, 95), 1.5);

    // The SCALE attribute is clamped to [0.0625, 16.0].
    assert!(set_scale(&mut store, 95, 20.0));
    assert_eq!(scale(&store, 95), 16.0);
    assert!(set_scale(&mut store, 95, 0.001));
    assert_eq!(scale(&store, 95), 0.0625);

    // A non-living entity (boat) is gated out of the living render scale (the same
    // `vanilla_living_entity_type` gate as the other render-state projections).
    store.apply_add_entity(protocol_add_entity_with_type(
        96,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert_eq!(scale(&store, 96), 1.0);
}

#[test]
fn death_animation_gates_on_living_entity_health() {
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
    const VANILLA_ENTITY_TYPE_ITEM_ID: i32 = 71;
    const VANILLA_ENTITY_HEALTH_DATA_ID: u8 = 9;
    const FLOAT_SERIALIZER_ID: i32 = 3;

    let zero_health = vec![ProtocolEntityDataValue {
        data_id: VANILLA_ENTITY_HEALTH_DATA_ID,
        serializer_id: FLOAT_SERIALIZER_ID,
        value: EntityDataValueKind::Float(0.0),
    }];

    // A non-living entity (item) is not a LivingEntity, so a stray float at the
    // health id never starts the death animation.
    let mut item = EntityClientAnimationState::default();
    item.sync_targets_from_metadata(VANILLA_ENTITY_TYPE_ITEM_ID, &zero_health);
    assert!(item.death.is_none());

    // A living entity at zero health begins it (deathTime 0 until the first tick).
    let mut chicken = EntityClientAnimationState::default();
    chicken.sync_targets_from_metadata(VANILLA_ENTITY_TYPE_CHICKEN_ID, &zero_health);
    assert!(chicken.death.is_some());
    assert_eq!(chicken.death_time(0.0), 0.0);
    assert!(!chicken.has_red_overlay());
}

#[test]
fn entity_model_sources_project_creeper_swelling_fuse() {
    const VANILLA_ENTITY_TYPE_CREEPER_ID: i32 = 32;
    const CREEPER_SWELL_DIR_DATA_ID: u8 = 16;

    // Read at partial tick 1.0 so getSwelling returns the current swell.
    let swelling = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 50)
            .unwrap()
            .creeper_swelling
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        50,
        VANILLA_ENTITY_TYPE_CREEPER_ID,
    ));
    // Default swell direction is -1 (resting): the fuse stays at zero.
    assert_eq!(swelling(&store), 0.0);
    store.advance_entity_client_animations(5);
    assert_eq!(swelling(&store), 0.0);

    // A positive swell direction advances the fuse one step per client tick;
    // getSwelling divides the lerped swell by maxSwell - 2 = 28.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![ProtocolEntityDataValue {
            data_id: CREEPER_SWELL_DIR_DATA_ID,
            serializer_id: 1,
            value: EntityDataValueKind::Int(1),
        }],
    }));
    store.advance_entity_client_animations(3);
    assert_eq!(swelling(&store), 3.0 / 28.0);

    // Flipping the direction back to -1 drains the fuse toward zero again.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![ProtocolEntityDataValue {
            data_id: CREEPER_SWELL_DIR_DATA_ID,
            serializer_id: 1,
            value: EntityDataValueKind::Int(-1),
        }],
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(swelling(&store), 2.0 / 28.0);
}

#[test]
fn entity_model_sources_project_squid_tentacle_and_body_animation() {
    const VANILLA_ENTITY_TYPE_SQUID_ID: i32 = 127;
    const SQUID_RESET_MOVEMENT_EVENT_ID: i8 = 19;

    let source = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 70)
            .unwrap()
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        70,
        VANILLA_ENTITY_TYPE_SQUID_ID,
    ));
    // Give the squid a horizontal+downward velocity so the body pitch turns away
    // from zero (vanilla `xBodyRot` is driven by `atan2(horizontal, dm.y)`).
    assert!(store.apply_set_entity_motion(ProtocolSetEntityMotion {
        id: 70,
        delta_movement: ProtocolVec3d {
            x: 0.2,
            y: -0.1,
            z: 0.0,
        },
    }));

    // A floating squid that has never been ticked is frozen at the bind pose.
    let resting = source(&store, 1.0);
    assert_eq!(resting.squid_tentacle_angle, 0.0);
    assert_eq!(resting.squid_x_body_rot, 0.0);
    assert_eq!(resting.squid_z_body_rot, 0.0);

    // A few ticks in (still early in the half-cycle, `scale < 0.75`) the tentacle
    // flex is already off the bind pose, but the body roll has not yet engaged:
    // vanilla only sets `rotateSpeed = 1` once `scale > 0.75`.
    store.advance_entity_client_animations(5);
    let after_five = source(&store, 1.0);
    assert!(
        after_five.squid_tentacle_angle > 0.0,
        "the tentacle angle leaves the bind pose: {}",
        after_five.squid_tentacle_angle
    );
    assert!(
        after_five.squid_x_body_rot < 0.0,
        "a diving squid pitches its body negative: {}",
        after_five.squid_x_body_rot
    );

    // Advance deep into the half-cycle so `scale > 0.75` engages `rotateSpeed = 1`,
    // after which the body roll accumulates each tick (`zBodyRot += π·rotateSpeed·1.5`).
    store.advance_entity_client_animations(18);
    let after_roll = source(&store, 1.0);
    assert!(
        after_roll.squid_z_body_rot > 0.0,
        "the body roll accumulates once the half-cycle passes 0.75: {}",
        after_roll.squid_z_body_rot
    );

    store.advance_entity_client_animations(1);
    let after_more = source(&store, 1.0);
    assert!(
        after_more.squid_z_body_rot > after_roll.squid_z_body_rot,
        "the body roll keeps advancing across ticks"
    );

    // The lerped getters track the partial tick between the old and current
    // endpoints: at partial 0.0 the projection equals last tick's value (the
    // half-way point of the lerp at 0.5 sits strictly between the two endpoints).
    let at_zero = source(&store, 0.0).squid_z_body_rot;
    let at_half = source(&store, 0.5).squid_z_body_rot;
    let at_one = source(&store, 1.0).squid_z_body_rot;
    assert!(
        at_zero < at_half && at_half < at_one,
        "partial tick lerps the roll: {at_zero} < {at_half} < {at_one}"
    );
    assert!(
        (at_half - (at_zero + (at_one - at_zero) * 0.5)).abs() < 1.0e-4,
        "the projection is a linear lerp between the endpoints"
    );

    // Entity event 19 (`Squid.handleEntityEvent`) resets `tentacleMovement` to 0.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 70,
        event_id: SQUID_RESET_MOVEMENT_EVENT_ID,
    }));
    // After the reset, the next tick restarts the half-cycle from near zero, so the
    // tentacle angle is small (`sin(scale²·π)·π·0.25` with `scale` just above 0).
    store.advance_entity_client_animations(1);
    let after_reset = source(&store, 1.0);
    assert!(
        after_reset.squid_tentacle_angle < after_five.squid_tentacle_angle,
        "the event-19 reset rewinds the tentacle cycle"
    );
}

#[test]
fn squid_tentacle_speed_is_seeded_by_entity_id() {
    const VANILLA_ENTITY_TYPE_SQUID_ID: i32 = 127;
    const VANILLA_ENTITY_TYPE_GLOW_SQUID_ID: i32 = 61;

    // The per-tick tentacle advance equals `tentacleSpeed`, so after one tick the
    // tentacle movement (read indirectly via the angle the half-cycle produces) is
    // a deterministic function of the id-seeded speed. Two squids with different
    // ids advance at different rates, while a glow squid is seeded the same way.
    let tentacle_angle_after_one_tick = |id: i32, entity_type_id: i32| {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity_with_type(id, entity_type_id));
        store.advance_entity_client_animations(1);
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .squid_tentacle_angle
    };

    let squid_a = tentacle_angle_after_one_tick(7, VANILLA_ENTITY_TYPE_SQUID_ID);
    let squid_b = tentacle_angle_after_one_tick(1000, VANILLA_ENTITY_TYPE_SQUID_ID);
    assert!(squid_a > 0.0 && squid_b > 0.0);
    assert!(
        (squid_a - squid_b).abs() > 1.0e-6,
        "different ids seed different tentacle speeds: {squid_a} vs {squid_b}"
    );

    // A glow squid uses the same id-seeded animation (vanilla `GlowSquid extends Squid`).
    let glow = tentacle_angle_after_one_tick(7, VANILLA_ENTITY_TYPE_GLOW_SQUID_ID);
    assert!(
        (glow - squid_a).abs() < 1.0e-6,
        "the glow squid shares the squid animation seeding for the same id"
    );
}

#[test]
fn guardian_tail_animation_speed_branches_match_vanilla_ai_step() {
    // Vanilla `Guardian.aiStep` ramps `clientSideTailAnimationSpeed` differently per
    // tick depending on `isInWater()` and the synced `isMoving()` (`DATA_ID_MOVING`),
    // then integrates `clientSideTailAnimation += speed`. The projected
    // `guardian_tail_animation` advances by that per-tick speed, so its one-tick delta
    // pins which branch ran:
    //   - out of water  → speed = 2.0   (the frantic flop)
    //   - in water, moving, from rest (speed < 0.5) → speed snaps to 4.0
    //   - in water, idle → speed eases toward 0.125 (≈ 0.025 from rest, by 0.2)
    const VANILLA_ENTITY_TYPE_GUARDIAN_ID: i32 = 63;
    const SOURCE_WATER_BLOCK_STATE_ID: i32 = 86;

    let tail = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 80)
            .unwrap()
            .guardian_tail_animation
    };

    // A guardian standing in a tall water column (submerged) and flagged moving.
    let make_store = |moving: bool, in_water: bool| {
        let mut store = WorldStore::with_dimension(crate::WorldDimension {
            min_y: 0,
            height: 16,
        });
        store.insert_decoded_chunk(empty_test_chunk());
        store.apply_add_entity(ProtocolAddEntity {
            id: 80,
            uuid: default_entity_uuid(),
            entity_type_id: VANILLA_ENTITY_TYPE_GUARDIAN_ID,
            position: ProtocolVec3d {
                x: 8.5,
                y: 2.0,
                z: 8.5,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 99,
        });
        if in_water {
            // Fill the column the guardian's AABB occupies so `world_aabb_in_water`
            // sees a submerged box.
            for y in 1..=4 {
                assert!(store.apply_block_update(ProtocolBlockUpdate {
                    pos: ProtocolBlockPos { x: 8, y, z: 8 },
                    block_state_id: SOURCE_WATER_BLOCK_STATE_ID,
                }));
            }
        }
        if moving {
            assert!(store.apply_set_entity_data(ProtocolSetEntityData {
                id: 80,
                values: vec![protocol_bool_data(16, true)],
            }));
        }
        store
    };

    // In water + moving: from the rest speed (`0`, which is `< 0.5`) the first tick
    // snaps the speed to `4.0`, so the tail jumps by 4 per tick.
    let mut wet_moving = make_store(true, true);
    wet_moving.advance_entity_client_animations(1);
    let after_one = tail(&wet_moving);
    assert!(
        (after_one - 4.0).abs() < 1.0e-4,
        "in-water moving guardian snaps its tail speed to 4.0 from rest: {after_one}"
    );

    // In water + idle: the speed eases toward `0.125` by `0.2` (`0 + (0.125 - 0)*0.2 =
    // 0.025`), a slow hover wave — far slower than either other branch.
    let mut wet_idle = make_store(false, true);
    wet_idle.advance_entity_client_animations(1);
    let idle_one = tail(&wet_idle);
    assert!(
        (idle_one - 0.025).abs() < 1.0e-4,
        "in-water idle guardian eases its tail speed toward 0.125 (0.025 from rest): {idle_one}"
    );

    // Out of water: the speed is forced to `2.0` regardless of the moving flag.
    let mut dry = make_store(true, false);
    dry.advance_entity_client_animations(1);
    let dry_one = tail(&dry);
    assert!(
        (dry_one - 2.0).abs() < 1.0e-4,
        "an out-of-water guardian flops its tail at speed 2.0: {dry_one}"
    );

    // The three branches advance the tail at distinctly different rates.
    assert!(after_one > dry_one && dry_one > idle_one);
}

#[test]
fn guardian_spikes_withdrawal_branches_match_vanilla_ai_step() {
    // Vanilla `Guardian.aiStep` eases `clientSideSpikesAnimation` (spawn `0`): in water toward `1`
    // while idle (by `0.06`, spikes extend) or toward `0` while moving (by `0.25`, spikes retract);
    // out of water it randomizes — deferred, so the value is HELD. `GuardianRenderState.spikesAnimation`
    // lerps it, and `setupAnim` turns it into `withdrawal = (1 - it) · 0.55`.
    const VANILLA_ENTITY_TYPE_GUARDIAN_ID: i32 = 63;
    const SOURCE_WATER_BLOCK_STATE_ID: i32 = 86;
    const AIR_BLOCK_STATE_ID: i32 = 0;

    let spikes = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 80)
            .unwrap()
            .guardian_spikes_animation
    };

    let mut store = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    store.insert_decoded_chunk(empty_test_chunk());
    store.apply_add_entity(ProtocolAddEntity {
        id: 80,
        uuid: default_entity_uuid(),
        entity_type_id: VANILLA_ENTITY_TYPE_GUARDIAN_ID,
        position: ProtocolVec3d {
            x: 8.5,
            y: 2.0,
            z: 8.5,
        },
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 99,
    });
    let fill = |store: &mut WorldStore, block_state_id: i32| {
        for y in 1..=4 {
            assert!(store.apply_block_update(ProtocolBlockUpdate {
                pos: ProtocolBlockPos { x: 8, y, z: 8 },
                block_state_id,
            }));
        }
    };

    // An unticked guardian projects the fully-extended rest pose (withdrawal `0` ⇒ spikesAnimation 1).
    assert_eq!(spikes(&store), 1.0);

    // In water + idle: from the spawn `0` the spikes ease UP toward `1` by `0.06` — first tick `0.06`.
    fill(&mut store, SOURCE_WATER_BLOCK_STATE_ID);
    store.advance_entity_client_animations(1);
    assert!(
        (spikes(&store) - 0.06).abs() < 1.0e-5,
        "in-water idle eases the spikes toward 1 by 0.06: {}",
        spikes(&store)
    );
    // They keep climbing while idle.
    store.advance_entity_client_animations(9);
    let extended = spikes(&store);
    assert!(
        extended > 0.06 && extended < 1.0,
        "the idle spikes keep extending toward 1: {extended}"
    );

    // Flag the guardian moving (synced `DATA_ID_MOVING`, idx 16): in water the spikes now RETRACT,
    // easing toward `0` by `0.25` — one tick gives `0.75 · extended`.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 80,
        values: vec![protocol_bool_data(16, true)],
    }));
    store.advance_entity_client_animations(1);
    let retracting = spikes(&store);
    assert!(
        (retracting - extended * 0.75).abs() < 1.0e-5,
        "in-water moving retracts the spikes toward 0 by 0.25: {retracting} vs {extended}"
    );

    // Out of water (drain the column): vanilla randomizes, which is deferred, so the value is HELD at
    // the last frame regardless of the still-set moving flag.
    fill(&mut store, AIR_BLOCK_STATE_ID);
    store.advance_entity_client_animations(1);
    assert!(
        (spikes(&store) - retracting).abs() < 1.0e-5,
        "out of water the spikes hold their last value (random flicker deferred): {} vs {retracting}",
        spikes(&store)
    );
}

#[test]
fn entity_model_sources_project_guardian_attack_beam() {
    // Vanilla `GuardianRenderer.extractRenderState`: a guardian whose synced `DATA_ID_ATTACK_TARGET`
    // (idx 17) names a live target projects the world eye→target vector and the ramping attack timing;
    // with no target it projects no beam.
    const VANILLA_ENTITY_TYPE_GUARDIAN_ID: i32 = 63;
    const VANILLA_ENTITY_TYPE_ZOMBIE_ID: i32 = 150;
    const GUARDIAN_ATTACK_TARGET_DATA_ID: u8 = 17;

    let add_at = |id: i32, type_id: i32, x: f64| ProtocolAddEntity {
        id,
        uuid: default_entity_uuid(),
        entity_type_id: type_id,
        position: ProtocolVec3d { x, y: 64.0, z: 0.0 },
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    };

    let mut store = WorldStore::new();
    // Guardian at the origin; target zombie 10 blocks east (+X).
    store.apply_add_entity(add_at(70, VANILLA_ENTITY_TYPE_GUARDIAN_ID, 0.0));
    store.apply_add_entity(add_at(71, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 10.0));

    let beam = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == 70)
            .unwrap()
            .guardian_beam
    };

    // No active attack target → no beam.
    assert!(beam(&store).is_none());

    // Lock onto the zombie (id 71) and ramp the client-side attack time over five ticks.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![protocol_int_data(GUARDIAN_ATTACK_TARGET_DATA_ID, 71)],
    }));
    store.advance_entity_client_animations(5);
    let projected = beam(&store).expect("a guardian locked onto a live target beams");

    // The beam points east (+X) toward the target and is level (no Z drift, small Y from eye/center).
    assert!(
        projected.eye_to_target[0] > 8.0,
        "beam points +X toward target: {:?}",
        projected.eye_to_target
    );
    assert!(projected.eye_to_target[2].abs() < 0.01);
    assert!(projected.eye_height > 0.0);
    // Five client ticks ramp `clientSideAttackTime` to 5; at partial 0, `attackTime = 5` and
    // `attackScale = 5 / 80` (the guardian's `getAttackDuration`).
    assert!((projected.attack_time - 5.0).abs() < 1.0e-4);
    assert!((projected.attack_scale - 5.0 / 80.0).abs() < 1.0e-4);

    // Clearing the target stops the beam (and resets the counter, vanilla `onSyncedDataUpdated`).
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![protocol_int_data(GUARDIAN_ATTACK_TARGET_DATA_ID, 0)],
    }));
    assert!(beam(&store).is_none());
}

#[test]
fn frog_swim_idle_activates_only_in_water_and_idle() {
    // Vanilla `Frog.tick` (client): `swimIdleAnimationState.animateWhen(isInWater() &&
    // !walkAnimation.isMoving(), tickCount)`. The projected `frog_swim_idle_seconds` is `>= 0` while
    // the timer runs (in water, not moving) and the `-1.0` stopped sentinel otherwise. The frog's
    // `updateWalkAnimation` override is deferred (no `walk_animation` state), so `isMoving()` is
    // always false; the gate reduces to `isInWater()`.
    const VANILLA_ENTITY_TYPE_FROG_ID: i32 = 55;
    const SOURCE_WATER_BLOCK_STATE_ID: i32 = 86;

    let swim_idle = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 81)
            .unwrap()
            .frog_swim_idle_seconds
    };

    // A frog standing in a tall water column (submerged) or out of water.
    let make_store = |in_water: bool| {
        let mut store = WorldStore::with_dimension(crate::WorldDimension {
            min_y: 0,
            height: 16,
        });
        store.insert_decoded_chunk(empty_test_chunk());
        store.apply_add_entity(ProtocolAddEntity {
            id: 81,
            uuid: default_entity_uuid(),
            entity_type_id: VANILLA_ENTITY_TYPE_FROG_ID,
            position: ProtocolVec3d {
                x: 8.5,
                y: 2.0,
                z: 8.5,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 99,
        });
        if in_water {
            // Fill the column the frog's AABB occupies so `world_aabb_in_water` sees a submerged box.
            for y in 1..=4 {
                assert!(store.apply_block_update(ProtocolBlockUpdate {
                    pos: ProtocolBlockPos { x: 8, y, z: 8 },
                    block_state_id: SOURCE_WATER_BLOCK_STATE_ID,
                }));
            }
        }
        store
    };

    // In water and idle: the swim-idle timer starts on the first tick (`start_age == age_ticks`),
    // so at partial `1.0` the elapsed seconds are `(0 + 1.0)/20 = 0.05` and climb `1/20 = 0.05` per
    // tick thereafter — non-negative, the active branch.
    let mut wet = make_store(true);
    wet.advance_entity_client_animations(1);
    assert!(
        (swim_idle(&wet) - 0.05).abs() < 1.0e-6,
        "an in-water idle frog activates its swim-idle: {}",
        swim_idle(&wet)
    );
    wet.advance_entity_client_animations(2);
    assert!(
        (swim_idle(&wet) - 0.15).abs() < 1.0e-6,
        "the swim-idle elapsed seconds climb 1/20 per tick: {}",
        swim_idle(&wet)
    );

    // Out of water: the gate is false, the timer never starts, so the `-1.0` sentinel holds.
    let mut dry = make_store(false);
    dry.advance_entity_client_animations(3);
    assert_eq!(
        swim_idle(&dry),
        -1.0,
        "an out-of-water frog never activates its swim-idle"
    );

    // Leaving the water stops the animation: drain the column the wet frog idles in, then tick.
    for y in 1..=4 {
        assert!(wet.apply_block_update(ProtocolBlockUpdate {
            pos: ProtocolBlockPos { x: 8, y, z: 8 },
            block_state_id: 0,
        }));
    }
    wet.advance_entity_client_animations(1);
    assert_eq!(
        swim_idle(&wet),
        -1.0,
        "a frog that leaves the water stops its swim-idle (back to the sentinel)"
    );
}

#[test]
fn squid_tentacle_speed_matches_java_random_for_known_id() {
    // Vanilla `Squid` constructor: `random.setSeed(getId()); tentacleSpeed = 1 /
    // (random.nextFloat() + 1) * 0.2`. Pinned against the Java LCG: for id 0 the
    // first `nextFloat()` is 0.730_967_76 (matching the audio module's LCG test),
    // so `tentacleSpeed = 1 / 1.730_967_76 * 0.2 = 0.115_542_31`.
    let state = super::animations::SquidAnimationState::new(0);
    assert!(
        (state.tentacle_speed - 0.115_542_31).abs() < 1.0e-7,
        "id-0 tentacle speed must match the Java Random formula: {}",
        state.tentacle_speed
    );
}

#[test]
fn entity_model_sources_project_chicken_wing_flap() {
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;

    let source = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 80)
            .unwrap()
    };
    // Drives the chicken's synced ground flag (vanilla `Chicken.aiStep` reads
    // `onGround()`); the position stays put so only the flap state evolves.
    let set_on_ground = |store: &mut WorldStore, on_ground: bool| {
        assert!(store.apply_entity_move(ProtocolEntityMove {
            id: 80,
            delta_x: 0,
            delta_y: 0,
            delta_z: 0,
            y_rot: None,
            x_rot: None,
            on_ground,
        }));
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    // An unticked chicken is frozen at the bind pose (wings held).
    let resting = source(&store, 1.0);
    assert_eq!(resting.chicken_flap, 0.0);
    assert_eq!(resting.chicken_flap_speed, 0.0);

    // Airborne: vanilla `flapSpeed += 4.0 * 0.3 = 1.2` (clamped to 1) jumps straight
    // to the clamp in a single tick, and `flap += flapping * 2` advances each tick.
    set_on_ground(&mut store, false);
    store.advance_entity_client_animations(1);
    let air_one = source(&store, 1.0);
    assert!(
        (air_one.chicken_flap_speed - 1.0).abs() < 1.0e-6,
        "an airborne chicken saturates flap speed at 1 in one tick: {}",
        air_one.chicken_flap_speed
    );
    assert!(
        air_one.chicken_flap > 0.0,
        "an airborne chicken advances its flap phase: {}",
        air_one.chicken_flap
    );

    store.advance_entity_client_animations(1);
    let air_two = source(&store, 1.0);
    assert!(
        (air_two.chicken_flap_speed - 1.0).abs() < 1.0e-6,
        "flap speed holds at the clamp while airborne: {}",
        air_two.chicken_flap_speed
    );
    assert!(
        air_two.chicken_flap > air_one.chicken_flap,
        "the flap phase keeps advancing across ticks"
    );

    // The flap speed is sitting at 1; land and let vanilla `flapSpeed += -1.0 * 0.3`
    // pull it back toward 0 on the ground.
    let airborne_peak = air_two.chicken_flap_speed;
    set_on_ground(&mut store, true);
    store.advance_entity_client_animations(1);
    let grounded = source(&store, 1.0);
    assert!(
        grounded.chicken_flap_speed < airborne_peak,
        "landing drops the flap speed toward 0: {} -> {}",
        airborne_peak,
        grounded.chicken_flap_speed
    );

    // The lerped getters track the partial tick between the previous and current
    // flap endpoints (vanilla `ChickenRenderer.extractRenderState`).
    set_on_ground(&mut store, false);
    store.advance_entity_client_animations(3);
    let at_zero = source(&store, 0.0).chicken_flap;
    let at_half = source(&store, 0.5).chicken_flap;
    let at_one = source(&store, 1.0).chicken_flap;
    assert!(
        at_zero < at_half && at_half < at_one,
        "partial tick lerps the flap phase: {at_zero} < {at_half} < {at_one}"
    );
    assert!(
        (at_half - (at_zero + (at_one - at_zero) * 0.5)).abs() < 1.0e-4,
        "the projection is a linear lerp between the endpoints"
    );
}

#[test]
fn entity_model_sources_project_slime_squish_from_ground_transitions() {
    const VANILLA_ENTITY_TYPE_SLIME_ID: i32 = 117;

    let squish = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 81)
            .unwrap()
            .slime_squish
    };
    // Drives the slime's synced ground flag (vanilla `Slime.tick` reads `onGround()`
    // for the squish target); the position stays put so only the squish evolves.
    let set_on_ground = |store: &mut WorldStore, on_ground: bool| {
        assert!(store.apply_entity_move(ProtocolEntityMove {
            id: 81,
            delta_x: 0,
            delta_y: 0,
            delta_z: 0,
            y_rot: None,
            x_rot: None,
            on_ground,
        }));
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_SLIME_ID,
    ));

    // An unticked slime holds its undeformed cube (squish 0).
    assert_eq!(squish(&store, 1.0), 0.0);

    // Land from rest: vanilla seeds `targetSquish = -0.5` on the takeoff→ground
    // transition (then decays it by `0.6`), and the next tick eases `squish` toward
    // that negative target — the landing flatten/splat.
    set_on_ground(&mut store, true);
    store.advance_entity_client_animations(2);
    let landed = squish(&store, 1.0);
    assert!(
        landed < 0.0,
        "landing flattens the slime (negative squish): {landed}"
    );

    // Take off: vanilla seeds `targetSquish = 1.0` on the ground→air transition, and
    // the squish eases up through zero into the positive vertical stretch.
    set_on_ground(&mut store, false);
    store.advance_entity_client_animations(2);
    let airborne = squish(&store, 1.0);
    assert!(
        airborne > 0.0,
        "a jumping slime stretches vertically (positive squish): {airborne}"
    );
    assert!(
        airborne > landed,
        "takeoff lifts the squish above the landing splat: {landed} -> {airborne}"
    );

    // The lerped getter tracks the partial tick between the previous and current
    // squish endpoints (vanilla `SlimeRenderer.extractRenderState`).
    let at_zero = squish(&store, 0.0);
    let at_one = squish(&store, 1.0);
    assert_ne!(
        at_zero, at_one,
        "the squish is still evolving across this tick"
    );
    let at_half = squish(&store, 0.5);
    assert!(
        (at_half - (at_zero + (at_one - at_zero) * 0.5)).abs() < 1.0e-4,
        "the projection is a linear lerp between the endpoints: {at_zero} .. {at_half} .. {at_one}"
    );
}

#[test]
fn entity_model_sources_project_parrot_wing_flap() {
    const VANILLA_ENTITY_TYPE_PARROT_ID: i32 = 98;

    let flap_angle = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 90)
            .unwrap()
            .parrot_flap_angle
    };
    // Drives the parrot's synced ground flag (vanilla `Parrot.calculateFlapping` reads
    // `onGround()`); the position stays put so only the flap state evolves.
    let set_on_ground = |store: &mut WorldStore, on_ground: bool| {
        assert!(store.apply_entity_move(ProtocolEntityMove {
            id: 90,
            delta_x: 0,
            delta_y: 0,
            delta_z: 0,
            y_rot: None,
            x_rot: None,
            on_ground,
        }));
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        90,
        VANILLA_ENTITY_TYPE_PARROT_ID,
    ));

    // An unticked parrot is frozen at the bind pose (wings held): `flapAngle == 0`.
    assert_eq!(flap_angle(&store, 1.0), 0.0);

    // Airborne: vanilla `flapSpeed += 4.0 * 0.3 = 1.2` (clamped to 1) saturates in one tick, and
    // `flap += flapping * 2` advances the phase, so `flapAngle = (sin(flap) + 1) * flapSpeed > 0`.
    set_on_ground(&mut store, false);
    store.advance_entity_client_animations(1);
    let air_one = flap_angle(&store, 1.0);
    assert!(
        air_one > 0.0,
        "an airborne parrot develops a non-zero flap angle: {air_one}"
    );

    store.advance_entity_client_animations(1);
    let air_two = flap_angle(&store, 1.0);
    assert!(
        air_two > 0.0,
        "the flap angle stays live across airborne ticks: {air_two}"
    );

    // Land: vanilla `flapSpeed += -1.0 * 0.3` pulls the speed back toward 0 on the ground, and after
    // it bleeds to 0 the flap angle collapses to 0 (wings settle).
    set_on_ground(&mut store, true);
    store.advance_entity_client_animations(20);
    assert_eq!(
        flap_angle(&store, 1.0),
        0.0,
        "a grounded parrot settles its wings (flapSpeed -> 0)"
    );

    // The lerped getter tracks the partial tick between the previous and current flap angle
    // endpoints (vanilla `ParrotRenderer.extractRenderState` lerps flap+flapSpeed, then combines).
    set_on_ground(&mut store, false);
    store.advance_entity_client_animations(3);
    let at_zero = flap_angle(&store, 0.0);
    let at_one = flap_angle(&store, 1.0);
    assert_ne!(
        at_zero, at_one,
        "the projected flap angle changes across the partial tick: {at_zero} vs {at_one}"
    );
}

#[test]
fn parrot_passenger_holds_its_wings() {
    const VANILLA_ENTITY_TYPE_PARROT_ID: i32 = 98;
    // Vanilla `Parrot.calculateFlapping` gates the airborne flap build-up on `!onGround() &&
    // !isPassenger()`. A parrot riding a vehicle (its `vehicle_id` set) is a passenger, so even
    // airborne its `flapSpeed` decays toward 0 and `flapAngle` stays at 0 (wings settled).
    const VANILLA_ENTITY_TYPE_BOAT_ID: i32 = 9;

    let flap_angle = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 91)
            .unwrap()
            .parrot_flap_angle
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        92,
        VANILLA_ENTITY_TYPE_BOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        91,
        VANILLA_ENTITY_TYPE_PARROT_ID,
    ));
    // Mark the parrot airborne — without the passenger gate this would flap.
    assert!(store.apply_entity_move(ProtocolEntityMove {
        id: 91,
        delta_x: 0,
        delta_y: 0,
        delta_z: 0,
        y_rot: None,
        x_rot: None,
        on_ground: false,
    }));
    // Seat the parrot on the boat so it becomes a passenger.
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 92,
        passenger_ids: vec![91],
    }));

    store.advance_entity_client_animations(5);
    assert_eq!(
        flap_angle(&store),
        0.0,
        "an airborne passenger parrot keeps its wings settled"
    );
}

#[test]
fn entity_model_sources_project_bee_roll_amount() {
    const VANILLA_ENTITY_TYPE_BEE_ID: i32 = 11;
    // Vanilla `Bee.DATA_FLAGS_ID` is synced data id 18; `FLAG_ROLL` is mask 2 within that byte.
    let bee_flags = |raw: i8| ProtocolEntityDataValue {
        data_id: 18,
        serializer_id: 0,
        value: EntityDataValueKind::Byte(raw),
    };
    let roll = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 81)
            .unwrap()
            .bee_roll_amount
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_BEE_ID,
    ));

    // An upright (un-rolling) bee projects `0.0`.
    assert_eq!(roll(&store, 1.0), 0.0);

    // Setting `FLAG_ROLL` makes vanilla `Bee.updateRollAmount` climb `rollAmount` by `0.2`/tick.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 81,
        values: vec![bee_flags(2)],
    }));
    store.advance_entity_client_animations(1);
    assert!((roll(&store, 1.0) - 0.2).abs() < 1.0e-6);
    store.advance_entity_client_animations(1);
    assert!((roll(&store, 1.0) - 0.4).abs() < 1.0e-6);

    // It saturates at `1.0` (vanilla `Math.min(1.0, …)`): three more ticks reach 1.0 and hold.
    store.advance_entity_client_animations(5);
    assert!((roll(&store, 1.0) - 1.0).abs() < 1.0e-6);

    // Clearing the flag decays it by `0.24`/tick (vanilla `Math.max(0.0, …)`).
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 81,
        values: vec![bee_flags(0)],
    }));
    store.advance_entity_client_animations(1);
    assert!((roll(&store, 1.0) - 0.76).abs() < 1.0e-6);

    // The projected getter lerps across the partial tick (vanilla `Bee.getRollAmount`).
    let at_zero = roll(&store, 0.0);
    let at_half = roll(&store, 0.5);
    let at_one = roll(&store, 1.0);
    assert!(
        at_one < at_zero,
        "the decaying roll falls from previous to current"
    );
    assert!((at_half - (at_zero + (at_one - at_zero) * 0.5)).abs() < 1.0e-6);
}

#[test]
fn entity_model_sources_project_frog_croak_seconds() {
    const VANILLA_ENTITY_TYPE_FROG_ID: i32 = 55;
    // Vanilla `Pose.CROAKING(8, …)` synced via `DATA_POSE` (id 6); `Frog.onSyncedDataUpdated` starts
    // `croakAnimationState` when the pose becomes CROAKING and stops it otherwise.
    const VANILLA_POSE_STANDING_ID: i32 = 0;
    const VANILLA_POSE_CROAKING_ID: i32 = 8;
    let croak = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 55)
            .unwrap()
            .frog_croak_seconds
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        55,
        VANILLA_ENTITY_TYPE_FROG_ID,
    ));

    // A frog not in `Pose.CROAKING` projects the `-1.0` stopped sentinel (pouch hidden).
    assert_eq!(croak(&store, 1.0), -1.0);

    // Entering `Pose.CROAKING` starts the timer at the current age, so the elapsed seconds begin at
    // `0` (plus the partial tick): vanilla `((ageInTicks - startTick)) / 20`.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 55,
        values: vec![protocol_pose_data(6, VANILLA_POSE_CROAKING_ID)],
    }));
    assert!((croak(&store, 0.0) - 0.0).abs() < 1.0e-6);
    // The partial tick folds into the live age (`(0 + 0.5) / 20`).
    assert!((croak(&store, 0.5) - 0.025).abs() < 1.0e-6);

    // Each client tick advances the elapsed seconds by `1 / 20 = 0.05` (the age climbs, the start
    // tick is fixed).
    store.advance_entity_client_animations(1);
    assert!((croak(&store, 0.0) - 0.05).abs() < 1.0e-6);
    store.advance_entity_client_animations(4);
    assert!((croak(&store, 0.0) - 0.25).abs() < 1.0e-6);

    // Leaving `Pose.CROAKING` stops the animation, returning the `-1.0` sentinel.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 55,
        values: vec![protocol_pose_data(6, VANILLA_POSE_STANDING_ID)],
    }));
    assert_eq!(croak(&store, 1.0), -1.0);
}

#[test]
fn entity_model_sources_project_frog_tongue_seconds() {
    const VANILLA_ENTITY_TYPE_FROG_ID: i32 = 55;
    // Vanilla `Pose.USING_TONGUE(9, …)` synced via `DATA_POSE` (id 6); `Frog.onSyncedDataUpdated`
    // starts `tongueAnimationState` when the pose becomes USING_TONGUE and stops it otherwise.
    const VANILLA_POSE_STANDING_ID: i32 = 0;
    const VANILLA_POSE_USING_TONGUE_ID: i32 = 9;
    let tongue = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 55)
            .unwrap()
            .frog_tongue_seconds
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        55,
        VANILLA_ENTITY_TYPE_FROG_ID,
    ));

    // A frog not in `Pose.USING_TONGUE` projects the `-1.0` stopped sentinel (no lash).
    assert_eq!(tongue(&store, 1.0), -1.0);

    // Entering `Pose.USING_TONGUE` starts the timer at the current age: vanilla `(ageInTicks -
    // startTick) / 20`, so the elapsed seconds begin at `0` (plus the partial tick).
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 55,
        values: vec![protocol_pose_data(6, VANILLA_POSE_USING_TONGUE_ID)],
    }));
    assert!((tongue(&store, 0.0) - 0.0).abs() < 1.0e-6);
    assert!((tongue(&store, 0.5) - 0.025).abs() < 1.0e-6);

    // Each client tick advances the elapsed seconds by `1 / 20 = 0.05`.
    store.advance_entity_client_animations(1);
    assert!((tongue(&store, 0.0) - 0.05).abs() < 1.0e-6);
    store.advance_entity_client_animations(4);
    assert!((tongue(&store, 0.0) - 0.25).abs() < 1.0e-6);

    // Leaving `Pose.USING_TONGUE` stops the animation, returning the `-1.0` sentinel.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 55,
        values: vec![protocol_pose_data(6, VANILLA_POSE_STANDING_ID)],
    }));
    assert_eq!(tongue(&store, 1.0), -1.0);
}

#[test]
fn entity_model_sources_project_frog_jump_seconds() {
    const VANILLA_ENTITY_TYPE_FROG_ID: i32 = 55;
    // Vanilla `Pose.LONG_JUMPING(6, …)` synced via `DATA_POSE` (id 6); `Frog.onSyncedDataUpdated`
    // starts `jumpAnimationState` when the pose becomes LONG_JUMPING and stops it otherwise.
    const VANILLA_POSE_STANDING_ID: i32 = 0;
    const VANILLA_POSE_LONG_JUMPING_ID: i32 = 6;
    let jump = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 55)
            .unwrap()
            .frog_jump_seconds
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        55,
        VANILLA_ENTITY_TYPE_FROG_ID,
    ));

    // A frog not in `Pose.LONG_JUMPING` projects the `-1.0` stopped sentinel (no keyframe applied).
    assert_eq!(jump(&store, 1.0), -1.0);

    // Entering `Pose.LONG_JUMPING` starts the timer at the current age, so the elapsed seconds begin
    // at `0` (plus the partial tick): vanilla `((ageInTicks - startTick)) / 20`.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 55,
        values: vec![protocol_pose_data(6, VANILLA_POSE_LONG_JUMPING_ID)],
    }));
    assert!((jump(&store, 0.0) - 0.0).abs() < 1.0e-6);
    // The partial tick folds into the live age (`(0 + 0.5) / 20`).
    assert!((jump(&store, 0.5) - 0.025).abs() < 1.0e-6);

    // Each client tick advances the elapsed seconds by `1 / 20 = 0.05` (the age climbs, the start
    // tick is fixed).
    store.advance_entity_client_animations(1);
    assert!((jump(&store, 0.0) - 0.05).abs() < 1.0e-6);
    store.advance_entity_client_animations(4);
    assert!((jump(&store, 0.0) - 0.25).abs() < 1.0e-6);

    // Leaving `Pose.LONG_JUMPING` stops the animation, returning the `-1.0` sentinel.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 55,
        values: vec![protocol_pose_data(6, VANILLA_POSE_STANDING_ID)],
    }));
    assert_eq!(jump(&store, 1.0), -1.0);
}

#[test]
fn entity_model_sources_project_sniffer_state_animation() {
    const VANILLA_ENTITY_TYPE_SNIFFER_ID: i32 = 119;
    // Vanilla `Sniffer.DATA_STATE` (id 18), the `Sniffer.State` ordinal VarInt;
    // `Sniffer.onSyncedDataUpdated` `resetAnimations()` then starts the matching one-shot.
    const SNIFFER_STATE_DATA_ID: u8 = 18;
    const SNIFFER_STATE_IDLING_ID: i32 = 0;
    const SNIFFER_STATE_SNIFFING_ID: i32 = 3;
    const SNIFFER_STATE_SEARCHING_ID: i32 = 4;
    const SNIFFER_STATE_DIGGING_ID: i32 = 5;
    let animation = |store: &WorldStore, partial: f32| {
        let source = store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 119)
            .unwrap();
        (
            source.sniffer_animation_id,
            source.sniffer_animation_seconds,
        )
    };
    let is_searching = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 119)
            .unwrap()
            .sniffer_is_searching
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        119,
        VANILLA_ENTITY_TYPE_SNIFFER_ID,
    ));

    // An idling sniffer projects the `(-1, -1.0)` no-animation sentinel and is not searching.
    assert_eq!(animation(&store, 1.0), (-1, -1.0));
    assert!(!is_searching(&store));

    // Entering `DIGGING` starts the dig one-shot at the current age: the id is the `DIGGING` ordinal
    // and the elapsed seconds begin at `0` (plus the partial tick), advancing `1 / 20` per tick.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 119,
        values: vec![protocol_enum_data(
            SNIFFER_STATE_DATA_ID,
            EntityDataEnumSerializer::SnifferState,
            SNIFFER_STATE_DIGGING_ID
        )],
    }));
    let (id, seconds) = animation(&store, 0.0);
    assert_eq!(id, SNIFFER_STATE_DIGGING_ID);
    assert!((seconds - 0.0).abs() < 1.0e-6);
    // The partial tick folds into the live age (`(0 + 0.5) / 20`).
    assert!((animation(&store, 0.5).1 - 0.025).abs() < 1.0e-6);
    store.advance_entity_client_animations(4);
    assert_eq!(animation(&store, 0.0), (SNIFFER_STATE_DIGGING_ID, 0.2));

    // Changing to a different animated state restarts the timer from `0` (vanilla `resetAnimations()`
    // + `startIfStopped` on the transition) and switches the id to the new state.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 119,
        values: vec![protocol_enum_data(
            SNIFFER_STATE_DATA_ID,
            EntityDataEnumSerializer::SnifferState,
            SNIFFER_STATE_SNIFFING_ID
        )],
    }));
    assert_eq!(animation(&store, 0.0), (SNIFFER_STATE_SNIFFING_ID, 0.0));

    // `SEARCHING` carries no one-shot (it drives the looping search-walk), so it clears to the
    // no-animation sentinel — but `sniffer_is_searching` flips true to swap in the search-walk.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 119,
        values: vec![protocol_enum_data(
            SNIFFER_STATE_DATA_ID,
            EntityDataEnumSerializer::SnifferState,
            SNIFFER_STATE_SEARCHING_ID
        )],
    }));
    assert_eq!(animation(&store, 1.0), (-1, -1.0));
    assert!(is_searching(&store));

    // Returning to `IDLING` likewise stays cleared and is no longer searching.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 119,
        values: vec![protocol_enum_data(
            SNIFFER_STATE_DATA_ID,
            EntityDataEnumSerializer::SnifferState,
            SNIFFER_STATE_IDLING_ID
        )],
    }));
    assert_eq!(animation(&store, 1.0), (-1, -1.0));
    assert!(!is_searching(&store));
}

#[test]
fn entity_model_sources_project_armadillo_state_animation() {
    const VANILLA_ENTITY_TYPE_ARMADILLO_ID: i32 = 4;
    // Vanilla `Armadillo.ARMADILLO_STATE` (id 18), the `ArmadilloState` id VarInt (serializer 36).
    // `Armadillo.setupAnimationStates` `.startIfStopped`s rollUp into ROLLING / rollOut into
    // UNROLLING, and `shouldHideInShell(inStateTicks)` gates the shell-ball swap.
    const ARMADILLO_STATE_DATA_ID: u8 = 18;
    const ARMADILLO_STATE_ROLLING_ID: i32 = 1;
    const ARMADILLO_STATE_UNROLLING_ID: i32 = 3;
    let project = |store: &WorldStore| {
        let source = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == 4)
            .unwrap();
        (
            source.armadillo_is_hiding_in_shell,
            source.armadillo_roll_up_seconds,
            source.armadillo_roll_out_seconds,
            source.armadillo_peek_seconds,
        )
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        4,
        VANILLA_ENTITY_TYPE_ARMADILLO_ID,
    ));

    // An IDLE armadillo (no state synced) is unrolled with no transition timers.
    assert_eq!(project(&store), (false, -1.0, -1.0, -1.0));

    // Entering ROLLING starts the roll-up timer at the current age (elapsed `0`) and does NOT yet
    // hide: vanilla `ROLLING.shouldHideInShell(inStateTicks) = inStateTicks > 5`.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 4,
        values: vec![protocol_enum_data(
            ARMADILLO_STATE_DATA_ID,
            EntityDataEnumSerializer::ArmadilloState,
            ARMADILLO_STATE_ROLLING_ID,
        )],
    }));
    let (hiding, roll_up, roll_out, peek) = project(&store);
    assert!(!hiding, "rolling does not hide until inStateTicks > 5");
    assert!((roll_up - 0.0).abs() < 1.0e-6, "roll-up starts at 0s");
    assert_eq!((roll_out, peek), (-1.0, -1.0));

    // The roll-up elapsed seconds advance `1 / 20` per client tick.
    store.advance_entity_client_animations(5);
    assert!((project(&store).1 - 0.25).abs() < 1.0e-6);
    // At inStateTicks == 5 it still does not hide (`> 5` is strict); the next tick flips it true.
    assert!(!project(&store).0, "inStateTicks == 5 is not yet hiding");
    store.advance_entity_client_animations(1);
    assert!(
        project(&store).0,
        "inStateTicks == 6 hides the body in the shell"
    );
    // The roll-up keeps advancing past the hide (vanilla applies it regardless of hiding).
    assert!((project(&store).1 - 0.3).abs() < 1.0e-6);

    // Entering UNROLLING restarts: the roll-out timer starts at 0, the roll-up stops, and the body
    // stays hidden while `inStateTicks < 26` (`UNROLLING.shouldHideInShell`).
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 4,
        values: vec![protocol_enum_data(
            ARMADILLO_STATE_DATA_ID,
            EntityDataEnumSerializer::ArmadilloState,
            ARMADILLO_STATE_UNROLLING_ID,
        )],
    }));
    let (hiding, roll_up, roll_out, _) = project(&store);
    assert!(
        hiding,
        "unrolling keeps the ball until inStateTicks reaches 26"
    );
    assert_eq!(roll_up, -1.0, "the roll-up timer stops on the transition");
    assert!((roll_out - 0.0).abs() < 1.0e-6, "roll-out starts at 0s");

    // The body stays hidden through inStateTicks 25, then un-hides at 26.
    store.advance_entity_client_animations(25);
    assert!(project(&store).0, "inStateTicks == 25 is still hiding");
    assert!(
        (project(&store).2 - 1.25).abs() < 1.0e-6,
        "roll-out advanced 25 ticks"
    );
    store.advance_entity_client_animations(1);
    assert!(!project(&store).0, "inStateTicks == 26 un-hides the body");
}

#[test]
fn entity_model_sources_project_warden_combat_animations() {
    const VANILLA_ENTITY_TYPE_WARDEN_ID: i32 = 142;
    // Vanilla `Pose.ROARING(11)` / `Pose.SNIFFING(12)` synced via `DATA_POSE` (id 6);
    // `Warden.onSyncedDataUpdated` `.start()`s the matching one-shot when the pose CHANGES to it.
    const VANILLA_POSE_STANDING_ID: i32 = 0;
    const VANILLA_POSE_ROARING_ID: i32 = 11;
    const VANILLA_POSE_SNIFFING_ID: i32 = 12;
    // Vanilla `Warden.handleEntityEvent`: id 4 starts the attack (and stops the roar); id 62 starts
    // the sonic boom.
    const WARDEN_ATTACK_EVENT_ID: i8 = 4;
    const WARDEN_SONIC_BOOM_EVENT_ID: i8 = 62;
    let combat = |store: &WorldStore, partial: f32| {
        let source = store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 142)
            .unwrap();
        (
            source.warden_roar_seconds,
            source.warden_sniff_seconds,
            source.warden_attack_seconds,
            source.warden_sonic_boom_seconds,
        )
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        142,
        VANILLA_ENTITY_TYPE_WARDEN_ID,
    ));

    // A warden in no triggered pose with no event projects all `-1.0` stopped sentinels.
    assert_eq!(combat(&store, 1.0), (-1.0, -1.0, -1.0, -1.0));

    // Entering `Pose.ROARING` starts the roar timer at the current age: the elapsed seconds begin at
    // `0` (plus the partial tick), advancing `1 / 20` per tick. Only the roar activates.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 142,
        values: vec![protocol_pose_data(6, VANILLA_POSE_ROARING_ID)],
    }));
    let (roar, sniff, attack, sonic) = combat(&store, 0.0);
    assert!((roar - 0.0).abs() < 1.0e-6);
    assert_eq!((sniff, attack, sonic), (-1.0, -1.0, -1.0));
    // The partial tick folds into the live age (`(0 + 0.5) / 20`).
    assert!((combat(&store, 0.5).0 - 0.025).abs() < 1.0e-6);
    store.advance_entity_client_animations(4);
    assert!((combat(&store, 0.0).0 - 0.2).abs() < 1.0e-6);

    // Leaving `Pose.ROARING` does NOT stop the roar (vanilla never auto-stops on pose leave); the
    // non-looping keyframe just holds its final frame, so the timer keeps advancing.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 142,
        values: vec![protocol_pose_data(6, VANILLA_POSE_STANDING_ID)],
    }));
    store.advance_entity_client_animations(1);
    assert!((combat(&store, 0.0).0 - 0.25).abs() < 1.0e-6);

    // Event 4 starts the attack AND stops the roar (vanilla `roarAnimationState.stop()` +
    // `attackAnimationState.start()`).
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 142,
        event_id: WARDEN_ATTACK_EVENT_ID,
    }));
    let (roar, _, attack, _) = combat(&store, 0.0);
    assert_eq!(roar, -1.0, "the attack event stops the roar");
    assert!((attack - 0.0).abs() < 1.0e-6, "the attack starts at 0");
    store.advance_entity_client_animations(2);
    assert!((combat(&store, 0.0).2 - 0.1).abs() < 1.0e-6);

    // Event 62 starts the sonic boom independently (the attack keeps holding its final frame).
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 142,
        event_id: WARDEN_SONIC_BOOM_EVENT_ID,
    }));
    let (_, _, attack, sonic) = combat(&store, 0.0);
    assert!((sonic - 0.0).abs() < 1.0e-6, "the sonic boom starts at 0");
    assert!((attack - 0.1).abs() < 1.0e-6, "the attack still holds");

    // Entering `Pose.SNIFFING` starts the sniff timer; the other three keep their running timers.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 142,
        values: vec![protocol_pose_data(6, VANILLA_POSE_SNIFFING_ID)],
    }));
    let (_, sniff, _, _) = combat(&store, 0.0);
    assert!((sniff - 0.0).abs() < 1.0e-6, "the sniff starts at 0");
}

#[test]
fn entity_model_sources_project_warden_emerge_and_dig() {
    const VANILLA_ENTITY_TYPE_WARDEN_ID: i32 = 142;
    // Vanilla `Pose.EMERGING(13)` / `Pose.DIGGING(14)` synced via `DATA_POSE` (id 6); like the
    // roar/sniff poses, `Warden.onSyncedDataUpdated` `.start()`s the spawn/despawn one-shot when the
    // pose CHANGES to it. These are the 6.68s `WARDEN_EMERGE` and 5.0s `WARDEN_DIG` keyframes.
    const VANILLA_POSE_STANDING_ID: i32 = 0;
    const VANILLA_POSE_EMERGING_ID: i32 = 13;
    const VANILLA_POSE_DIGGING_ID: i32 = 14;
    let spawn = |store: &WorldStore, partial: f32| {
        let source = store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 142)
            .unwrap();
        (source.warden_emerge_seconds, source.warden_dig_seconds)
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        142,
        VANILLA_ENTITY_TYPE_WARDEN_ID,
    ));

    // A warden in no triggered pose projects the `-1.0` stopped sentinels.
    assert_eq!(spawn(&store, 1.0), (-1.0, -1.0));

    // Entering `Pose.EMERGING` starts the emerge timer at the current age (elapsed begins at `0`,
    // plus the partial tick, advancing `1 / 20` per tick). The dig stays stopped.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 142,
        values: vec![protocol_pose_data(6, VANILLA_POSE_EMERGING_ID)],
    }));
    let (emerge, dig) = spawn(&store, 0.0);
    assert!((emerge - 0.0).abs() < 1.0e-6, "the emerge starts at 0");
    assert_eq!(dig, -1.0, "the dig is still stopped");
    assert!(
        (spawn(&store, 0.5).0 - 0.025).abs() < 1.0e-6,
        "partial folds in"
    );
    store.advance_entity_client_animations(4);
    assert!((spawn(&store, 0.0).0 - 0.2).abs() < 1.0e-6);

    // Leaving `Pose.EMERGING` does NOT stop the emerge (vanilla never auto-stops on pose leave); the
    // non-looping keyframe just holds its final frame, so the timer keeps advancing.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 142,
        values: vec![protocol_pose_data(6, VANILLA_POSE_STANDING_ID)],
    }));
    store.advance_entity_client_animations(1);
    assert!((spawn(&store, 0.0).0 - 0.25).abs() < 1.0e-6);

    // Entering `Pose.DIGGING` starts the dig timer; the emerge keeps holding its running timer.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 142,
        values: vec![protocol_pose_data(6, VANILLA_POSE_DIGGING_ID)],
    }));
    let (emerge, dig) = spawn(&store, 0.0);
    assert!((dig - 0.0).abs() < 1.0e-6, "the dig starts at 0");
    assert!((emerge - 0.25).abs() < 1.0e-6, "the emerge still holds");
}

#[test]
fn entity_model_sources_project_fox_head_roll_and_crouch() {
    const VANILLA_ENTITY_TYPE_FOX_ID: i32 = 54;
    // Vanilla `Fox.DATA_FLAGS_ID` is synced data id 19; `FLAG_CROUCHING` is mask 4 and
    // `FLAG_INTERESTED` is mask 8 within that byte.
    let fox_flags = |raw: i8| ProtocolEntityDataValue {
        data_id: 19,
        serializer_id: 0,
        value: EntityDataValueKind::Byte(raw),
    };
    let source = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 91)
            .unwrap()
    };
    let head_roll = |store: &WorldStore, partial: f32| source(store, partial).fox_head_roll_angle;
    let crouch = |store: &WorldStore, partial: f32| source(store, partial).fox_crouch_amount;
    const HEAD_ROLL_SCALE: f32 = 0.11 * std::f32::consts::PI;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        91,
        VANILLA_ENTITY_TYPE_FOX_ID,
    ));

    // A resting fox projects level head and no crouch, and no flag bools.
    assert_eq!(head_roll(&store, 1.0), 0.0);
    assert_eq!(crouch(&store, 1.0), 0.0);
    assert!(!source(&store, 1.0).fox_is_crouching);
    assert!(!source(&store, 1.0).fox_is_sleeping);

    // Setting `FLAG_INTERESTED` eases `interestedAngle` toward 1 by `* 0.4`/tick. After one tick the
    // angle is `0.4`, so the head roll is `0.4 * 0.11 * π`.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 91,
        values: vec![fox_flags(8)],
    }));
    store.advance_entity_client_animations(1);
    assert!((head_roll(&store, 1.0) - 0.4 * HEAD_ROLL_SCALE).abs() < 1.0e-6);
    // A second tick: `0.4 + (1 - 0.4) * 0.4 = 0.64`.
    store.advance_entity_client_animations(1);
    assert!((head_roll(&store, 1.0) - 0.64 * HEAD_ROLL_SCALE).abs() < 1.0e-6);

    // Setting `FLAG_CROUCHING` (and clearing interest) climbs `crouchAmount` by `0.2`/tick and instantly
    // resets the interest ease toward 0.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 91,
        values: vec![fox_flags(4)],
    }));
    assert!(source(&store, 1.0).fox_is_crouching);
    store.advance_entity_client_animations(1);
    assert!((crouch(&store, 1.0) - 0.2).abs() < 1.0e-6);
    store.advance_entity_client_animations(1);
    assert!((crouch(&store, 1.0) - 0.4).abs() < 1.0e-6);

    // `crouchAmount` saturates at `5.0` (vanilla `MAX_CROUCH_AMOUNT`).
    store.advance_entity_client_animations(30);
    assert!((crouch(&store, 1.0) - 5.0).abs() < 1.0e-6);

    // The crouch getter lerps across the partial tick (vanilla `Fox.getCrouchAmount`); clearing the
    // flag resets `crouchAmount` to `0` INSTANTLY (vanilla's non-crouching branch is an assignment).
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 91,
        values: vec![fox_flags(0)],
    }));
    assert!(!source(&store, 1.0).fox_is_crouching);
    store.advance_entity_client_animations(1);
    assert_eq!(crouch(&store, 1.0), 0.0);
    // Mid-tick the lerp still shows the drop from `5.0` to `0.0`.
    let at_zero = crouch(&store, 0.0);
    let at_half = crouch(&store, 0.5);
    assert!((at_zero - 5.0).abs() < 1.0e-6);
    assert!((at_half - 2.5).abs() < 1.0e-6);

    // The plain sleep/sit/pounce/faceplant bools project straight off the synced byte.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 91,
        values: vec![fox_flags(32 | 1 | 16 | 64)],
    }));
    let posed = source(&store, 1.0);
    assert!(posed.fox_is_sleeping);
    assert!(posed.fox_is_sitting);
    assert!(posed.fox_is_pouncing);
    assert!(posed.fox_is_faceplanted);
    assert!(!posed.fox_is_crouching);
}

#[test]
fn chicken_flap_state_initializes_flapping_to_one() {
    // Vanilla `Chicken` field initializer `public float flapping = 1.0F;`; every
    // other flap field defaults to 0.
    let state = super::animations::ChickenFlapAnimationState::default();
    assert_eq!(state.flapping, 1.0);
    assert_eq!(state.flap, 0.0);
    assert_eq!(state.o_flap, 0.0);
    assert_eq!(state.flap_speed, 0.0);
    assert_eq!(state.o_flap_speed, 0.0);
}

#[test]
fn entity_model_sources_project_walk_animation_limb_swing() {
    const VANILLA_ENTITY_TYPE_COW_ID: i32 = 30;

    // partial tick 1.0 → WalkAnimationState.position/speed return the current
    // (un-lerped) accumulator values.
    let walk = |store: &WorldStore, partial: f32| -> (f32, f32) {
        let source = store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 60)
            .unwrap();
        (source.walk_animation_position, source.walk_animation_speed)
    };
    let sync_position = |store: &mut WorldStore, x: f64, z: f64| {
        assert!(
            store.apply_entity_position_sync(ProtocolEntityPositionSync {
                id: 60,
                position: ProtocolVec3d { x, y: 64.0, z },
                delta_movement: ProtocolVec3d {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                y_rot: 0.0,
                x_rot: 0.0,
                on_ground: true,
            })
        );
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        60,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    // Establish a known baseline feet position, then take the first tick: it only
    // records the position (vanilla `xo == getX()`), so the swing stays at rest.
    sync_position(&mut store, 0.0, 0.0);
    store.advance_entity_client_animations(1);
    assert_eq!(walk(&store, 1.0), (0.0, 0.0));

    // Move 0.5 blocks along X, then tick: vanilla distance = 0.5, targetSpeed =
    // min(0.5 * 4, 1) = 1.0, speed = 0 + (1 - 0) * 0.4 = 0.4, position = 0 + 0.4.
    sync_position(&mut store, 0.5, 0.0);
    store.advance_entity_client_animations(1);
    let (pos1, speed1) = walk(&store, 1.0);
    assert!(
        (speed1 - 0.4).abs() < 1e-5,
        "speed after one step: {speed1}"
    );
    assert!((pos1 - 0.4).abs() < 1e-5, "position after one step: {pos1}");

    // Move another 0.5 along X and tick: targetSpeed = 1.0 again, speed = 0.4 + (1
    // - 0.4) * 0.4 = 0.64, position = 0.4 + 0.64 = 1.04.
    sync_position(&mut store, 1.0, 0.0);
    store.advance_entity_client_animations(1);
    let (pos2, speed2) = walk(&store, 1.0);
    assert!(
        (speed2 - 0.64).abs() < 1e-5,
        "speed after two steps: {speed2}"
    );
    assert!(
        (pos2 - 1.04).abs() < 1e-5,
        "position after two steps: {pos2}"
    );

    // Vanilla `WalkAnimationState.position/speed(partialTicks)` lerp the projection:
    // speed(0.5) = lerp(0.5, 0.4, 0.64) = 0.52; position(0.5) = 1.04 - 0.64 * 0.5.
    let (pos_mid, speed_mid) = walk(&store, 0.5);
    assert!(
        (speed_mid - 0.52).abs() < 1e-5,
        "mid-tick speed: {speed_mid}"
    );
    assert!(
        (pos_mid - 0.72).abs() < 1e-5,
        "mid-tick position: {pos_mid}"
    );

    // Standing still (no position change) for a tick: distance = 0, targetSpeed =
    // 0, speed = 0.64 + (0 - 0.64) * 0.4 = 0.384; the position keeps integrating.
    store.advance_entity_client_animations(1);
    let (pos3, speed3) = walk(&store, 1.0);
    assert!(
        (speed3 - 0.384).abs() < 1e-5,
        "speed decays toward zero: {speed3}"
    );
    assert!(
        (pos3 - (1.04 + 0.384)).abs() < 1e-5,
        "position keeps integrating: {pos3}"
    );
}

#[test]
fn entity_model_sources_walk_animation_scales_position_for_babies() {
    const VANILLA_ENTITY_TYPE_COW_ID: i32 = 30;
    const AGEABLE_MOB_BABY_DATA_ID: u8 = 16;
    const BOOLEAN_SERIALIZER_ID: i32 = 8;

    let walk = |store: &WorldStore| -> (f32, f32) {
        let source = store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 61)
            .unwrap();
        (source.walk_animation_position, source.walk_animation_speed)
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        61,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    // Vanilla `updateWalkAnimation` passes `isBaby() ? 3.0F : 1.0F` as the position
    // scale, so a baby's limb-swing position is tripled (the speed is unscaled).
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 61,
        values: vec![ProtocolEntityDataValue {
            data_id: AGEABLE_MOB_BABY_DATA_ID,
            serializer_id: BOOLEAN_SERIALIZER_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert!(
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 61,
            position: ProtocolVec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        })
    );
    store.advance_entity_client_animations(1);
    assert!(
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 61,
            position: ProtocolVec3d {
                x: 0.5,
                y: 64.0,
                z: 0.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        })
    );
    store.advance_entity_client_animations(1);
    let (position, speed) = walk(&store);
    // speed = 0.4 (unscaled); position = 0.4 * 3 = 1.2.
    assert!(
        (speed - 0.4).abs() < 1e-5,
        "baby speed is unscaled: {speed}"
    );
    assert!(
        (position - 1.2).abs() < 1e-5,
        "baby position is tripled: {position}"
    );
}

#[test]
fn entity_model_sources_walk_animation_stops_for_passengers_and_the_dead() {
    const VANILLA_ENTITY_TYPE_COW_ID: i32 = 30;
    const VANILLA_ENTITY_TYPE_BOAT_ID: i32 = 89;
    const VANILLA_ENTITY_HEALTH_DATA_ID: u8 = 9;
    const FLOAT_SERIALIZER_ID: i32 = 3;

    let walk = |store: &WorldStore, id: i32| -> (f32, f32) {
        let source = store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap();
        (source.walk_animation_position, source.walk_animation_speed)
    };
    let move_one_step = |store: &mut WorldStore, id: i32, x0: f64, x1: f64| {
        for x in [x0, x1] {
            assert!(
                store.apply_entity_position_sync(ProtocolEntityPositionSync {
                    id,
                    position: ProtocolVec3d { x, y: 64.0, z: 0.0 },
                    delta_movement: ProtocolVec3d {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    y_rot: 0.0,
                    x_rot: 0.0,
                    on_ground: true,
                })
            );
            store.advance_entity_client_animations(1);
        }
    };

    // A cow riding a boat is a passenger: vanilla `calculateEntityAnimation` calls
    // `walkAnimation.stop()` so its limb swing stays at rest however it is moved.
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        70,
        VANILLA_ENTITY_TYPE_BOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        71,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 70,
        passenger_ids: vec![71],
    }));
    move_one_step(&mut store, 71, 0.0, 0.5);
    assert_eq!(walk(&store, 71), (0.0, 0.0));

    // A dead cow (`isAlive()` false once health <= 0) also stops its limb swing.
    store.apply_add_entity(protocol_add_entity_with_type(
        72,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 72,
        values: vec![ProtocolEntityDataValue {
            data_id: VANILLA_ENTITY_HEALTH_DATA_ID,
            serializer_id: FLOAT_SERIALIZER_ID,
            value: EntityDataValueKind::Float(0.0),
        }],
    }));
    move_one_step(&mut store, 72, 0.0, 0.5);
    assert_eq!(walk(&store, 72), (0.0, 0.0));
}

#[test]
fn entity_model_sources_defer_walk_animation_for_overridden_entities() {
    // Camel/Frog override `updateWalkAnimation` and additionally gate on pose/jump
    // animation states the client does not yet model, so their limb swing stays
    // deferred (zero) while an ordinary cow with the base mapping animates from the
    // same movement. (The `Creaking` override is pure and IS now driven — see
    // `creaking_walk_uses_the_vanilla_distance_to_speed_override`.)
    const VANILLA_ENTITY_TYPE_COW_ID: i32 = 30;
    const VANILLA_ENTITY_TYPE_CAMEL_ID: i32 = 19;

    let walk_position = |store: &WorldStore, id: i32| -> f32 {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .walk_animation_position
    };
    let move_one_step = |store: &mut WorldStore, id: i32| {
        for x in [0.0, 0.5] {
            assert!(
                store.apply_entity_position_sync(ProtocolEntityPositionSync {
                    id,
                    position: ProtocolVec3d { x, y: 64.0, z: 0.0 },
                    delta_movement: ProtocolVec3d {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    y_rot: 0.0,
                    x_rot: 0.0,
                    on_ground: true,
                })
            );
            store.advance_entity_client_animations(1);
        }
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    ));
    move_one_step(&mut store, 80);
    move_one_step(&mut store, 81);
    assert!(
        walk_position(&store, 80) > 0.0,
        "the cow's limb swing animates"
    );
    assert_eq!(walk_position(&store, 81), 0.0, "the camel stays deferred");
}

#[test]
fn creaking_walk_uses_the_vanilla_distance_to_speed_override() {
    // Vanilla `Creaking.updateWalkAnimation`: `targetSpeed = min(distance · 25, 3); walkAnimation
    // .update(targetSpeed, 0.4, 1)`. After one 0.5-block step the target saturates at `3.0`, so
    // `speed = 0 + (3 - 0) · 0.4 = 1.2` and `position = 1.2` — but `speed(partial)` clamps to `1.0`.
    // A cow with the base `min(distance · 4, 1)` mapping reaches only `position = speed = 0.4` from
    // the same movement, so the creaking ramps ~3× faster.
    const VANILLA_ENTITY_TYPE_CREAKING_ID: i32 = 31;
    const VANILLA_ENTITY_TYPE_COW_ID: i32 = 30;

    let walk = |store: &WorldStore, id: i32| -> (f32, f32) {
        let source = store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap();
        (source.walk_animation_position, source.walk_animation_speed)
    };
    let sync = |store: &mut WorldStore, id: i32, x: f64| {
        assert!(
            store.apply_entity_position_sync(ProtocolEntityPositionSync {
                id,
                position: ProtocolVec3d { x, y: 64.0, z: 0.0 },
                delta_movement: ProtocolVec3d {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                y_rot: 0.0,
                x_rot: 0.0,
                on_ground: true,
            })
        );
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        82,
        VANILLA_ENTITY_TYPE_CREAKING_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        83,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));

    // Sync both to the baseline, then take the first shared tick: it only records the feet position
    // (vanilla `xo == getX()`), so the swing stays at rest. (Both entities are advanced together each
    // tick, so neither integrates an extra non-moving tick from the other's update.)
    sync(&mut store, 82, 0.0);
    sync(&mut store, 83, 0.0);
    store.advance_entity_client_animations(1);
    assert_eq!(walk(&store, 82), (0.0, 0.0));

    // One 0.5-block step: the creaking's `min(0.5 · 25, 3) = 3.0` target gives `position = 1.2` and a
    // clamped `speed = 1.0`; the cow's `min(0.5 · 4, 1) = 1.0` target gives `position = speed = 0.4`.
    sync(&mut store, 82, 0.5);
    sync(&mut store, 83, 0.5);
    store.advance_entity_client_animations(1);
    let (creaking_pos, creaking_speed) = walk(&store, 82);
    assert!(
        (creaking_pos - 1.2).abs() < 1e-5,
        "creaking position ramps with the ·25→3 mapping: {creaking_pos}"
    );
    assert!(
        (creaking_speed - 1.0).abs() < 1e-5,
        "the projected walk speed clamps to 1.0: {creaking_speed}"
    );
    let (cow_pos, _) = walk(&store, 83);
    assert!(
        (cow_pos - 0.4).abs() < 1e-5,
        "cow position uses the base ·4→1 mapping: {cow_pos}"
    );
    assert!(
        creaking_pos > cow_pos,
        "the creaking ramps faster than the base mapping"
    );
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
                    ..DataComponentPatchSummary::default()
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
fn local_player_is_sleeping_true_for_sleeping_local_entity() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(32));
    store.apply_add_entity(protocol_add_entity_with_type(
        32,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 32,
        values: vec![protocol_pose_data(
            super::dimensions::ENTITY_DATA_POSE_ID,
            super::dimensions::VANILLA_POSE_SLEEPING_ID,
        )],
    }));

    assert!(store.local_player_is_sleeping());
}

#[test]
fn local_player_is_sleeping_false_without_local_id_or_entity() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        32,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 32,
        values: vec![protocol_pose_data(
            super::dimensions::ENTITY_DATA_POSE_ID,
            super::dimensions::VANILLA_POSE_SLEEPING_ID,
        )],
    }));
    assert!(!store.local_player_is_sleeping());

    store.apply_login(&protocol_play_login(33));
    assert!(!store.local_player_is_sleeping());
}

#[test]
fn local_player_is_sleeping_false_for_non_sleeping_pose() {
    const POSE_STANDING: i32 = 0;

    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(32));
    store.apply_add_entity(protocol_add_entity_with_type(
        32,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 32,
        values: vec![protocol_pose_data(
            super::dimensions::ENTITY_DATA_POSE_ID,
            POSE_STANDING,
        )],
    }));

    assert!(!store.local_player_is_sleeping());
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
fn polar_bear_standing_projects_render_stand_scale() {
    const POLAR_BEAR_TYPE_ID: i32 = 104;
    const CHICKEN_TYPE_ID: i32 = 26;
    const POLAR_BEAR_STANDING_DATA_ID: u8 = 18;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(80, POLAR_BEAR_TYPE_ID));
    store.apply_add_entity(protocol_add_entity_with_type(81, CHICKEN_TYPE_ID));

    let stand_scale = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .polar_bear_stand_scale
    };

    // A polar bear on all fours has no standing animation state.
    assert_eq!(stand_scale(&store, 80, 1.0), 0.0);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 80,
        values: vec![protocol_bool_data(POLAR_BEAR_STANDING_DATA_ID, true)],
    }));

    // After one client tick clientSideStandAnimationO=0, clientSideStandAnimation=1,
    // so getStandingAnimationScale(a) = lerp(a, 0, 1) / 6 = a / 6.
    store.advance_entity_client_animations(1);
    assert_eq!(stand_scale(&store, 80, 0.0), 0.0);
    assert_eq!(stand_scale(&store, 80, 0.5), 0.5 / 6.0);
    assert_eq!(stand_scale(&store, 80, 1.0), 1.0 / 6.0);

    // Once fully reared (prev == current == 6) the scale saturates at 1.0.
    store.advance_entity_client_animations(10);
    assert_eq!(stand_scale(&store, 80, 0.0), 1.0);
    assert_eq!(stand_scale(&store, 80, 1.0), 1.0);

    // A non-polar-bear never carries a standing scale.
    assert_eq!(stand_scale(&store, 81, 1.0), 0.0);
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
        // `ArmorStand.DATA_CLIENT_FLAGS` data id (15): the SMALL bit (1) halves the pick bounds.
        values: vec![ProtocolEntityDataValue {
            data_id: 15,
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
        // `ArmorStand.DATA_CLIENT_FLAGS` data id (15): the MARKER bit (0x10) zeroes the pick bounds.
        values: vec![ProtocolEntityDataValue {
            data_id: 15,
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
fn local_player_rideable_jumping_vehicle_id_tracks_vanilla_controlled_mounts() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
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
    assert_eq!(store.local_player_rideable_jumping_vehicle_id(), Some(10));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_rideable_jumping_vehicle_id(), Some(20));

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
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_HORSE_ID,
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
    assert_eq!(store.local_player_rideable_jumping_vehicle_id(), None);
}

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
fn local_player_server_controlled_inventory_vehicle_tracks_vanilla_types() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        30,
        VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        40,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        50,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    assert_eq!(
        store.local_player_server_controlled_inventory_vehicle_id(),
        Some(10)
    );

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    assert_eq!(
        store.local_player_server_controlled_inventory_vehicle_id(),
        Some(20)
    );

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 30,
        passenger_ids: vec![99],
    }));
    assert_eq!(
        store.local_player_server_controlled_inventory_vehicle_id(),
        Some(30)
    );

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 40,
        passenger_ids: vec![99],
    }));
    assert_eq!(
        store.local_player_server_controlled_inventory_vehicle_id(),
        None
    );

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 50,
        passenger_ids: vec![99],
    }));
    assert_eq!(
        store.local_player_server_controlled_inventory_vehicle_id(),
        None
    );
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
            0.05,
        )
        .unwrap();

    assert_eq!(report.vehicle_id, 20);
    assert!(!report.snapped);
    assert!(report.y_rot > initial.y_rot);
    assert!(report.position.z > initial.position.z);
    assert_eq!(report.position, store.probe_entity(20).unwrap().position);
    assert_eq!(store.local_player_root_boat_vehicle_id(), Some(20));
}

#[test]
fn local_boat_input_ignores_unfocused_controls() {
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
            0.05,
        )
        .unwrap();

    assert_eq!(report.position, initial.position);
    assert_eq!(report.y_rot, initial.y_rot);
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
fn item_entity_stacks_filters_and_preserves_protocol_order() {
    let mut store = WorldStore::new();

    let mut first = protocol_add_entity_with_type(30, VANILLA_ENTITY_TYPE_ITEM_ID);
    first.position = ProtocolVec3d {
        x: 3.25,
        y: 65.5,
        z: -7.75,
    };
    store.apply_add_entity(first);
    store.apply_add_entity(protocol_add_entity_with_type(20, 7));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        40,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        50,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    ));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 30,
        values: vec![item_stack_entity_data(item_stack(42, 3))],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 20,
        values: vec![item_stack_entity_data(item_stack(99, 1))],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![item_stack_entity_data(item_stack(51, 2))],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 40,
        values: vec![item_stack_entity_data(ProtocolItemStackSummary::empty())],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![item_stack_entity_data(item_stack(77, 0))],
    }));

    let items = store.item_entity_stacks();
    assert_eq!(
        items.iter().map(|item| item.entity_id).collect::<Vec<_>>(),
        vec![30, 10]
    );
    assert_eq!(
        items[0].position,
        EntityVec3 {
            x: 3.25,
            y: 65.5,
            z: -7.75,
        }
    );
    assert_eq!(items[0].stack, item_stack(42, 3));
    assert_eq!(
        items[1].position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert_eq!(items[1].stack, item_stack(51, 2));
}

#[test]
fn item_stacks_for_entity_types_collects_thrown_item_projectiles() {
    // The thrown-item projectiles carry their displayed item in the same `DATA_ITEM_STACK` (id 8) as
    // the dropped item, so the type-filtered accessor reads them for the billboard layer. Snowball is
    // type id 120, egg 39; a plain item (71) is excluded when those types are requested.
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(60, 120));
    store.apply_add_entity(protocol_add_entity_with_type(61, 39));
    store.apply_add_entity(protocol_add_entity_with_type(
        62,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    ));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 60,
        values: vec![item_stack_entity_data(item_stack(880, 1))],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 61,
        values: vec![item_stack_entity_data(item_stack(881, 1))],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 62,
        values: vec![item_stack_entity_data(item_stack(882, 1))],
    }));

    let projectiles = store.item_stacks_for_entity_types(&[120, 39]);
    assert_eq!(
        projectiles
            .iter()
            .map(|item| item.entity_id)
            .collect::<Vec<_>>(),
        vec![60, 61]
    );
    assert_eq!(projectiles[0].stack, item_stack(880, 1));
    assert_eq!(projectiles[1].stack, item_stack(881, 1));

    // The dropped item (type 71) is untouched by the projectile-only query.
    assert!(!projectiles.iter().any(|item| item.entity_id == 62));
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
fn sheep_eat_grass_event_drives_client_animation_tick() {
    const VANILLA_ENTITY_TYPE_SHEEP_ID: i32 = 111;
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        70,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        71,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let eat_tick = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .sheep_eat_animation_tick
    };

    // Vanilla Sheep.handleEntityEvent: event 10 resets eatAnimationTick to 40.
    assert_eq!(eat_tick(&store, 70), 0);
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 70,
        event_id: 10,
    }));
    assert_eq!(eat_tick(&store, 70), 40);

    // Vanilla Sheep.aiStep decrements eatAnimationTick once per client tick.
    store.advance_entity_client_animations(1);
    assert_eq!(eat_tick(&store, 70), 39);
    store.advance_entity_client_animations(38);
    assert_eq!(eat_tick(&store, 70), 1);
    store.advance_entity_client_animations(1);
    assert_eq!(eat_tick(&store, 70), 0);
    // It clamps at 0 and does not run negative.
    store.advance_entity_client_animations(5);
    assert_eq!(eat_tick(&store, 70), 0);

    // Only event 10 starts the animation; other sheep events do not.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 70,
        event_id: 6,
    }));
    assert_eq!(eat_tick(&store, 70), 0);

    // Event 10 on a non-sheep entity never starts the sheep eat animation.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 71,
        event_id: 10,
    }));
    assert_eq!(eat_tick(&store, 71), 0);
}

#[test]
fn goat_ram_events_drive_the_lower_head_tick_counter() {
    const VANILLA_ENTITY_TYPE_GOAT_ID: i32 = 62;
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        72,
        VANILLA_ENTITY_TYPE_GOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        73,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let lower_head = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .goat_lower_head_tick
    };

    // Vanilla Goat.handleEntityEvent: event 58 starts lowering the head; the counter then climbs +1 per
    // client tick (aiStep), clamped at 20.
    assert_eq!(lower_head(&store, 72), 0);
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 72,
        event_id: 58,
    }));
    assert_eq!(lower_head(&store, 72), 0);
    store.advance_entity_client_animations(1);
    assert_eq!(lower_head(&store, 72), 1);
    store.advance_entity_client_animations(19);
    assert_eq!(lower_head(&store, 72), 20);
    // It clamps at the 20 cap.
    store.advance_entity_client_animations(5);
    assert_eq!(lower_head(&store, 72), 20);

    // Event 59 raises the head; the counter then decays -2 per tick down to 0, after which the state is
    // dropped (a resting goat projects 0).
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 72,
        event_id: 59,
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(lower_head(&store, 72), 18);
    store.advance_entity_client_animations(9);
    assert_eq!(lower_head(&store, 72), 0);
    store.advance_entity_client_animations(5);
    assert_eq!(lower_head(&store, 72), 0);

    // The ram events on a non-goat entity never start the counter.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 73,
        event_id: 58,
    }));
    store.advance_entity_client_animations(3);
    assert_eq!(lower_head(&store, 73), 0);
}

#[test]
fn iron_golem_attack_and_offer_events_drive_client_animation_timers() {
    const VANILLA_ENTITY_TYPE_IRON_GOLEM_ID: i32 = 70;
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        74,
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        75,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let source = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
    };

    // Vanilla IronGolem.handleEntityEvent: event 4 sets attackAnimationTick to 10; the projection lerps
    // it with the partial tick (attackTicksRemaining = tick - partial).
    assert_eq!(
        source(&store, 74, 0.0).iron_golem_attack_ticks_remaining,
        0.0
    );
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 74,
        event_id: 4,
    }));
    assert_eq!(
        source(&store, 74, 0.0).iron_golem_attack_ticks_remaining,
        10.0
    );
    assert_eq!(
        source(&store, 74, 0.5).iron_golem_attack_ticks_remaining,
        9.5
    );
    store.advance_entity_client_animations(10);
    // After 10 ticks the attack timer has run out.
    assert_eq!(
        source(&store, 74, 0.0).iron_golem_attack_ticks_remaining,
        0.0
    );

    // Event 11 sets offerFlowerTick to 400; event 34 clears it.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 74,
        event_id: 11,
    }));
    assert_eq!(source(&store, 74, 0.0).iron_golem_offer_flower_tick, 400);
    store.advance_entity_client_animations(3);
    assert_eq!(source(&store, 74, 0.0).iron_golem_offer_flower_tick, 397);
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 74,
        event_id: 34,
    }));
    assert_eq!(source(&store, 74, 0.0).iron_golem_offer_flower_tick, 0);

    // The same events on a non-golem never start the golem timers.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 75,
        event_id: 4,
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(
        source(&store, 75, 0.0).iron_golem_attack_ticks_remaining,
        0.0
    );
}

#[test]
fn ravager_attack_stun_and_roar_timers_advance_together() {
    const VANILLA_ENTITY_TYPE_RAVAGER_ID: i32 = 109;
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        76,
        VANILLA_ENTITY_TYPE_RAVAGER_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        77,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let source = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
    };

    // Vanilla Ravager.handleEntityEvent: event 4 sets attackTick to 10 (partial-lerped projection).
    assert_eq!(source(&store, 76, 0.0).ravager_attack_ticks_remaining, 0.0);
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 76,
        event_id: 4,
    }));
    assert_eq!(source(&store, 76, 0.5).ravager_attack_ticks_remaining, 9.5);
    store.advance_entity_client_animations(10);
    assert_eq!(source(&store, 76, 0.0).ravager_attack_ticks_remaining, 0.0);

    // Event 39 sets stunnedTick to 40; when it decays to 0 the aiStep arms the post-stun roar (20).
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 76,
        event_id: 39,
    }));
    assert_eq!(
        source(&store, 76, 0.0).ravager_stunned_ticks_remaining,
        40.0
    );
    assert_eq!(source(&store, 76, 0.0).ravager_roar_animation, 0.0);
    store.advance_entity_client_animations(40);
    // Stun has ended; the roar is now armed at 20 and the roarAnimation ramp begins (0 at tick 20).
    assert_eq!(source(&store, 76, 0.0).ravager_stunned_ticks_remaining, 0.0);
    assert_eq!(source(&store, 76, 0.0).ravager_roar_animation, 0.0);
    store.advance_entity_client_animations(5);
    // After 5 roar ticks: roarTick = 15, roarAnimation = (20 - 15)/20 = 0.25.
    assert!((source(&store, 76, 0.0).ravager_roar_animation - 0.25).abs() < 1.0e-6);
    store.advance_entity_client_animations(15);
    assert_eq!(source(&store, 76, 0.0).ravager_roar_animation, 0.0);

    // The ravager events on a non-ravager never start the timers.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 77,
        event_id: 39,
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(source(&store, 77, 0.0).ravager_stunned_ticks_remaining, 0.0);
}

#[test]
fn evoker_fangs_attack_event_drives_the_bite_progress_ramp() {
    const VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID: i32 = 47;
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        78,
        VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        79,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let progress = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .evoker_fangs_bite_progress
    };

    // An un-attacked fang is hidden underground (biteProgress 0).
    assert_eq!(progress(&store, 78, 1.0), 0.0);
    store.advance_entity_client_animations(5);
    assert_eq!(progress(&store, 78, 1.0), 0.0);

    // Vanilla `EvokerFangs.handleEntityEvent`: event 4 → `clientSideAttackStarted = true`,
    // and `lifeTicks` (22) begins counting down; `getAnimationProgress` at `lifeTicks`
    // 22 is `1 - (20 - partial)/20`, i.e. just above 0 and climbing.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 78,
        event_id: 4,
    }));
    let started = progress(&store, 78, 1.0);
    assert!(started > 0.0, "the attack ramp starts climbing: {started}");

    store.advance_entity_client_animations(1);
    let after_one = progress(&store, 78, 1.0);
    assert!(
        after_one > started,
        "the bite ramp keeps climbing: {started} -> {after_one}"
    );

    // After 20 ticks `lifeTicks` has reached 2, so `getAnimationProgress` saturates at
    // 1.0 (the fang has fully snapped shut and vanished) and holds there.
    store.advance_entity_client_animations(20);
    assert_eq!(progress(&store, 78, 1.0), 1.0);
    store.advance_entity_client_animations(5);
    assert_eq!(progress(&store, 78, 1.0), 1.0);

    // The fang event on a non-fang never starts a ramp.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 79,
        event_id: 4,
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(progress(&store, 79, 1.0), 0.0);
}

#[test]
fn camel_dash_flag_drives_the_dash_animation_timer() {
    const VANILLA_ENTITY_TYPE_CAMEL_ID: i32 = 19;
    const CAMEL_DASH_DATA_ID: u8 = 19;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        90,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    ));

    let dash_seconds = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 90)
            .unwrap()
            .camel_dash_seconds
    };
    let set_dashing = |store: &mut WorldStore, dashing: bool| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 90,
            values: vec![ProtocolEntityDataValue {
                data_id: CAMEL_DASH_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(dashing),
            }],
        }));
    };

    // A non-dashing camel projects the stopped-animation sentinel.
    assert_eq!(dash_seconds(&store, 1.0), -1.0);
    store.advance_entity_client_animations(3);
    assert_eq!(dash_seconds(&store, 1.0), -1.0);

    // Vanilla `Camel.setupAnimationStates`: the synced DASH rising edge starts `dashAnimationState`,
    // and the elapsed seconds climb from there (1 tick = 0.05 s).
    set_dashing(&mut store, true);
    store.advance_entity_client_animations(1);
    let after_one = dash_seconds(&store, 1.0);
    assert!(
        after_one >= 0.0,
        "a dashing camel starts the dash timer: {after_one}"
    );
    store.advance_entity_client_animations(2);
    let after_three = dash_seconds(&store, 1.0);
    assert!(
        after_three > after_one,
        "the dash timer keeps climbing: {after_one} -> {after_three}"
    );

    // Clearing the DASH flag stops the animation (back to the sentinel).
    set_dashing(&mut store, false);
    store.advance_entity_client_animations(1);
    assert_eq!(
        dash_seconds(&store, 1.0),
        -1.0,
        "clearing DASH stops the dash animation"
    );
}

#[test]
fn allay_dancing_flag_drives_the_dance_spin_state() {
    const VANILLA_ENTITY_TYPE_ALLAY_ID: i32 = 2;
    const ALLAY_DANCING_DATA_ID: u8 = 16;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        91,
        VANILLA_ENTITY_TYPE_ALLAY_ID,
    ));

    let source = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 91)
            .unwrap()
    };
    let set_dancing = |store: &mut WorldStore, dancing: bool| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 91,
            values: vec![ProtocolEntityDataValue {
                data_id: ALLAY_DANCING_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(dancing),
            }],
        }));
    };

    // A non-dancing allay projects the inert dance state.
    let resting = source(&store, 1.0);
    assert!(!resting.allay_dancing);
    assert!(!resting.allay_spinning);
    assert_eq!(resting.allay_spinning_progress, 0.0);
    store.advance_entity_client_animations(3);
    assert!(!source(&store, 1.0).allay_dancing);

    // Vanilla `Allay.tick` (client): while DATA_DANCING is set, `dancingAnimationTicks` climbs and
    // the first 15 ticks of each 55-tick loop are the spin sub-window (`spinningAnimationTicks`
    // ramping 0->15, `spinningProgress` 0->1).
    set_dancing(&mut store, true);
    store.advance_entity_client_animations(1);
    let after_one = source(&store, 1.0);
    assert!(
        after_one.allay_dancing,
        "the synced flag marks the allay dancing"
    );
    assert!(
        after_one.allay_spinning,
        "the dance opens in the spin sub-window"
    );
    assert!(after_one.allay_spinning_progress > 0.0);

    // Ten ticks into the spin window the progress has climbed further.
    store.advance_entity_client_animations(9);
    let after_ten = source(&store, 1.0);
    assert!(
        after_ten.allay_spinning_progress > after_one.allay_spinning_progress,
        "the spin ramp climbs: {} -> {}",
        after_one.allay_spinning_progress,
        after_ten.allay_spinning_progress
    );

    // Past the 15-tick spin window the allay is still dancing but no longer spinning, and the spin
    // progress unwinds back toward 0 (`spinningAnimationTicks` decrements once `isSpinning` is false).
    store.advance_entity_client_animations(10);
    let after_twenty = source(&store, 1.0);
    assert!(after_twenty.allay_dancing);
    assert!(
        !after_twenty.allay_spinning,
        "the spin sub-window has closed"
    );
    assert!(
        after_twenty.allay_spinning_progress < after_ten.allay_spinning_progress,
        "the spin ramp unwinds once spinning stops"
    );

    // Clearing the flag resets the dance entirely.
    set_dancing(&mut store, false);
    store.advance_entity_client_animations(1);
    let stopped = source(&store, 1.0);
    assert!(!stopped.allay_dancing);
    assert!(!stopped.allay_spinning);
    assert_eq!(stopped.allay_spinning_progress, 0.0);
}

#[test]
fn axolotl_playing_dead_flag_drives_the_eased_factor() {
    const VANILLA_ENTITY_TYPE_AXOLOTL_ID: i32 = 7;
    const AXOLOTL_PLAYING_DEAD_DATA_ID: u8 = 19;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        92,
        VANILLA_ENTITY_TYPE_AXOLOTL_ID,
    ));

    let factor = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 92)
            .unwrap()
            .axolotl_playing_dead_factor
    };
    let set_playing_dead = |store: &mut WorldStore, dead: bool| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 92,
            values: vec![ProtocolEntityDataValue {
                data_id: AXOLOTL_PLAYING_DEAD_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(dead),
            }],
        }));
    };

    // An awake axolotl projects no play-dead blend.
    assert_eq!(factor(&store, 1.0), 0.0);

    // Vanilla `Axolotl.playingDeadAnimator` (`BinaryAnimator(10, IN_OUT_SINE)`): the synced
    // `DATA_PLAYING_DEAD` flag eases the factor from 0 to a full 1.0 over the animator's 10 ticks.
    set_playing_dead(&mut store, true);
    store.advance_entity_client_animations(1);
    let after_one = factor(&store, 1.0);
    assert!(
        after_one > 0.0 && after_one < 1.0,
        "the play-dead factor eases up: {after_one}"
    );
    store.advance_entity_client_animations(9);
    assert!(
        (factor(&store, 1.0) - 1.0).abs() < 1.0e-6,
        "the factor saturates at 1.0 after the 10-tick animator length"
    );

    // Clearing the flag eases the factor back down to 0 over the next 10 ticks.
    set_playing_dead(&mut store, false);
    store.advance_entity_client_animations(1);
    let easing_down = factor(&store, 1.0);
    assert!(
        easing_down < 1.0,
        "the factor eases back down once awake: {easing_down}"
    );
    store.advance_entity_client_animations(10);
    assert_eq!(factor(&store, 1.0), 0.0, "fully awake again");
}

#[test]
fn hoglin_and_zoglin_attack_event_drives_the_headbutt_timer() {
    const VANILLA_ENTITY_TYPE_HOGLIN_ID: i32 = 64;
    const VANILLA_ENTITY_TYPE_ZOGLIN_ID: i32 = 149;
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        78,
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        79,
        VANILLA_ENTITY_TYPE_ZOGLIN_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let attack_tick = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .hoglin_attack_animation_tick
    };

    // Vanilla Hoglin/Zoglin.handleEntityEvent: event 4 sets attackAnimationRemainingTicks to 10 (the
    // RAW int, decremented each tick — no partial lerp). Both the hoglin and the zoglin headbutt.
    for id in [78, 79] {
        assert_eq!(attack_tick(&store, id), 0);
        assert!(store.apply_entity_event(ProtocolEntityEvent {
            entity_id: id,
            event_id: 4,
        }));
        assert_eq!(attack_tick(&store, id), 10);
    }
    store.advance_entity_client_animations(1);
    assert_eq!(attack_tick(&store, 78), 9);
    assert_eq!(attack_tick(&store, 79), 9);
    store.advance_entity_client_animations(9);
    assert_eq!(attack_tick(&store, 78), 0);
    assert_eq!(attack_tick(&store, 79), 0);

    // The attack event on a non-hoglin never starts the headbutt timer.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 80,
        event_id: 4,
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(attack_tick(&store, 80), 0);
}

#[test]
fn rabbit_jump_event_drives_the_hop_window() {
    const VANILLA_ENTITY_TYPE_RABBIT_ID: i32 = 108;
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
    const RABBIT_JUMP_EVENT_ID: i8 = 1;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        50,
        VANILLA_ENTITY_TYPE_RABBIT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        51,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let hop = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .rabbit_hop_seconds
    };

    // A resting rabbit projects the `-1.0` stopped sentinel.
    assert_eq!(hop(&store, 50, 1.0), -1.0);

    // Vanilla `Rabbit.handleEntityEvent(1)` seeds `jumpDuration = 15; jumpTicks = 0`. The hop is NOT
    // started yet — vanilla's `setupAnimationStates` (the hop branch) runs BEFORE `aiStep` lifts
    // `jumpTicks` past `0`, so the seed tick still reads the stopped sentinel.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 50,
        event_id: RABBIT_JUMP_EVENT_ID,
    }));
    assert_eq!(hop(&store, 50, 0.0), -1.0);
    // First tick: `jumpTicks` climbs to 1, but the hop only `startIfStopped`s on the NEXT tick's
    // `setupAnimationStates`, so it is still stopped here.
    store.advance_entity_client_animations(1);
    assert_eq!(hop(&store, 50, 0.0), -1.0);
    // Second tick: `jumpTicks > 0`, so the hop starts at the current age (elapsed begins at 0).
    store.advance_entity_client_animations(1);
    assert!((hop(&store, 50, 0.0) - 0.0).abs() < 1.0e-6);
    // The hop advances `1 / 20` per tick while the window runs.
    store.advance_entity_client_animations(5);
    assert!((hop(&store, 50, 0.0) - 0.25).abs() < 1.0e-6);
    // The partial tick folds into the live age.
    assert!((hop(&store, 50, 0.5) - 0.275).abs() < 1.0e-6);

    // The window is 15 ticks; the hop holds through its end (`jumpTicks` reaches `jumpDuration` and
    // resets), then stops on the following tick (`jumpTicks` back to 0).
    store.advance_entity_client_animations(9);
    assert!(
        (hop(&store, 50, 0.0) - 0.7).abs() < 1.0e-6,
        "still hopping at tick 14"
    );
    store.advance_entity_client_animations(1);
    assert_eq!(
        hop(&store, 50, 0.0),
        -1.0,
        "the hop stops when the jump window closes"
    );

    // The jump event on a non-rabbit never starts a hop.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 51,
        event_id: RABBIT_JUMP_EVENT_ID,
    }));
    store.advance_entity_client_animations(2);
    assert_eq!(hop(&store, 51, 0.0), -1.0);
}

#[test]
fn creaking_combat_events_and_tearing_down_drive_the_keyframes() {
    const VANILLA_ENTITY_TYPE_CREAKING_ID: i32 = 31;
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
    const CREAKING_ATTACK_EVENT_ID: i8 = 4;
    const CREAKING_INVULNERABLE_EVENT_ID: i8 = 66;
    const CREAKING_CAN_MOVE_DATA_ID: u8 = 16;
    const CREAKING_IS_TEARING_DOWN_DATA_ID: u8 = 18;

    let creaking_bool = |data_id: u8, value: bool| ProtocolEntityDataValue {
        data_id,
        serializer_id: 8,
        value: EntityDataValueKind::Boolean(value),
    };
    let source = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        70,
        VANILLA_ENTITY_TYPE_CREAKING_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        71,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    // A resting creaking can move (the `CAN_MOVE` default) and projects the `-1.0` stopped sentinels.
    let rest = source(&store, 70, 1.0);
    assert!(rest.creaking_can_move, "default canMove is true");
    assert_eq!(rest.creaking_attack_seconds, -1.0);
    assert_eq!(rest.creaking_invulnerable_seconds, -1.0);
    assert_eq!(rest.creaking_death_seconds, -1.0);

    // The synced `CAN_MOVE = false` freezes the walk (a creaking observed mid-step turns to a statue).
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![creaking_bool(CREAKING_CAN_MOVE_DATA_ID, false)],
    }));
    assert!(!source(&store, 70, 1.0).creaking_can_move);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![creaking_bool(CREAKING_CAN_MOVE_DATA_ID, true)],
    }));

    // Vanilla `Creaking.handleEntityEvent(4)`: `attackAnimationRemainingTicks = 15`. The one-shot is
    // NOT started yet — it only `animateWhen`s on the NEXT tick's `setupAnimationStates`, after
    // `aiStep` has decremented the counter (still positive). So the seed tick reads the stopped value.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 70,
        event_id: CREAKING_ATTACK_EVENT_ID,
    }));
    assert_eq!(source(&store, 70, 0.0).creaking_attack_seconds, -1.0);
    // First tick: vanilla decrements `15 -> 14` BEFORE `setupAnimationStates`, so the attack starts at
    // the current age this very tick (elapsed begins at 0), unlike the rabbit (which is animate-first).
    store.advance_entity_client_animations(1);
    assert!((source(&store, 70, 0.0).creaking_attack_seconds - 0.0).abs() < 1.0e-6);
    // It advances `1 / 20` per tick, with the partial folded into the live age.
    store.advance_entity_client_animations(5);
    assert!((source(&store, 70, 0.0).creaking_attack_seconds - 0.25).abs() < 1.0e-6);
    assert!((source(&store, 70, 0.5).creaking_attack_seconds - 0.275).abs() < 1.0e-6);
    // The window is 15 ticks (`attackTicks` 15 -> 0); the attack animates while it stays positive, so
    // it holds through tick 14 then stops when the counter hits 0 on tick 15.
    store.advance_entity_client_animations(8);
    assert!(
        (source(&store, 70, 0.0).creaking_attack_seconds - 0.65).abs() < 1.0e-6,
        "still attacking at tick 14"
    );
    store.advance_entity_client_animations(1);
    assert_eq!(
        source(&store, 70, 0.0).creaking_attack_seconds,
        -1.0,
        "the attack stops when the 15-tick window closes"
    );

    // Vanilla `Creaking.handleEntityEvent(66)`: `invulnerabilityAnimationRemainingTicks = 8`, the same
    // decrement-first window (8 ticks).
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 70,
        event_id: CREAKING_INVULNERABLE_EVENT_ID,
    }));
    store.advance_entity_client_animations(1);
    assert!((source(&store, 70, 0.0).creaking_invulnerable_seconds - 0.0).abs() < 1.0e-6);
    store.advance_entity_client_animations(7);
    assert_eq!(
        source(&store, 70, 0.0).creaking_invulnerable_seconds,
        -1.0,
        "the stagger stops when the 8-tick window closes"
    );

    // Vanilla `deathAnimationState.animateWhen(isTearingDown(), tickCount)`: the synced
    // `IS_TEARING_DOWN` boolean drives the collapse directly (no counter). Setting it spins up the
    // death one-shot on the next tick; clearing it stops the timer.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![creaking_bool(CREAKING_IS_TEARING_DOWN_DATA_ID, true)],
    }));
    store.advance_entity_client_animations(1);
    assert!((source(&store, 70, 0.0).creaking_death_seconds - 0.0).abs() < 1.0e-6);
    store.advance_entity_client_animations(5);
    assert!((source(&store, 70, 0.0).creaking_death_seconds - 0.25).abs() < 1.0e-6);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![creaking_bool(CREAKING_IS_TEARING_DOWN_DATA_ID, false)],
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(
        source(&store, 70, 0.0).creaking_death_seconds,
        -1.0,
        "clearing isTearingDown stops the collapse"
    );

    // A non-creaking never gets a creaking state: its combat seconds stay stopped, and `canMove`
    // projects the gated `true` regardless of the event/metadata.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 71,
        event_id: CREAKING_ATTACK_EVENT_ID,
    }));
    store.advance_entity_client_animations(2);
    let chicken = source(&store, 71, 0.0);
    assert!(chicken.creaking_can_move);
    assert_eq!(chicken.creaking_attack_seconds, -1.0);
    assert_eq!(chicken.creaking_death_seconds, -1.0);
}

#[test]
fn breeze_pose_drives_the_action_animations() {
    // Vanilla `Breeze.onSyncedDataUpdated(DATA_POSE)` + `tick`: the synced pose starts/stops the
    // shoot/inhale/slide/longJump one-shots (active while their pose holds), and LEAVING `Pose.SLIDING`
    // fires the brief `slideBack`. Each is projected as the elapsed seconds since it started, `-1.0`
    // when stopped. The looping idle is renderer-side and not projected.
    const VANILLA_ENTITY_TYPE_BREEZE_ID: i32 = 17;
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
    const POSE_STANDING: i32 = 0;
    const POSE_LONG_JUMPING: i32 = 6;
    const POSE_SLIDING: i32 = 15;
    const POSE_SHOOTING: i32 = 16;
    const POSE_INHALING: i32 = 17;

    let actions = |store: &WorldStore, id: i32| {
        let s = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap();
        (
            s.breeze_shoot_seconds,
            s.breeze_slide_seconds,
            s.breeze_slide_back_seconds,
            s.breeze_inhale_seconds,
            s.breeze_long_jump_seconds,
        )
    };
    let set_pose = |store: &mut WorldStore, id: i32, pose: i32| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![protocol_pose_data(6, pose)],
        }));
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        60,
        VANILLA_ENTITY_TYPE_BREEZE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        61,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    // A resting breeze projects the stopped sentinel for every action.
    assert_eq!(actions(&store, 60), (-1.0, -1.0, -1.0, -1.0, -1.0));

    // `Pose.SHOOTING` starts the shoot at the current age (elapsed begins at 0, advancing 1/20 per
    // tick); the others stay stopped.
    set_pose(&mut store, 60, POSE_SHOOTING);
    assert_eq!(actions(&store, 60), (0.0, -1.0, -1.0, -1.0, -1.0));
    store.advance_entity_client_animations(5);
    assert!((actions(&store, 60).0 - 0.25).abs() < 1.0e-6);
    // Leaving SHOOTING stops the shoot (it is not a SLIDING leave, so no slideBack).
    set_pose(&mut store, 60, POSE_STANDING);
    assert_eq!(actions(&store, 60), (-1.0, -1.0, -1.0, -1.0, -1.0));

    // `Pose.SLIDING` starts the slide; LEAVING it stops the slide AND fires `slideBack` at the leave.
    set_pose(&mut store, 60, POSE_SLIDING);
    assert_eq!(actions(&store, 60).1, 0.0, "slide starts on SLIDING");
    store.advance_entity_client_animations(2);
    assert!((actions(&store, 60).1 - 0.1).abs() < 1.0e-6);
    set_pose(&mut store, 60, POSE_STANDING);
    let (shoot, slide, slide_back, _, _) = actions(&store, 60);
    assert_eq!(shoot, -1.0);
    assert_eq!(slide, -1.0, "leaving SLIDING stops the slide");
    assert_eq!(
        slide_back, 0.0,
        "leaving SLIDING fires slideBack at the leave"
    );
    store.advance_entity_client_animations(3);
    assert!(
        (actions(&store, 60).2 - 0.15).abs() < 1.0e-6,
        "the slideBack return advances"
    );

    // `Pose.INHALING` starts the inhale; switching to `Pose.LONG_JUMPING` stops it and starts longJump.
    set_pose(&mut store, 60, POSE_INHALING);
    assert_eq!(actions(&store, 60).3, 0.0, "inhale starts on INHALING");
    set_pose(&mut store, 60, POSE_LONG_JUMPING);
    let (_, _, _, inhale, long_jump) = actions(&store, 60);
    assert_eq!(inhale, -1.0, "leaving INHALING stops the inhale");
    assert_eq!(long_jump, 0.0, "LONG_JUMPING starts the jump");

    // A non-breeze never gets a breeze state: every action stays stopped regardless of the pose.
    set_pose(&mut store, 61, POSE_SHOOTING);
    store.advance_entity_client_animations(2);
    assert_eq!(actions(&store, 61), (-1.0, -1.0, -1.0, -1.0, -1.0));
}

#[test]
fn warden_tendril_event_drives_client_animation_pulse() {
    const VANILLA_ENTITY_TYPE_WARDEN_ID: i32 = 142;
    const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_WARDEN_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let tendril = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .tendril_animation
    };

    // A warden at rest reports no tendril pulse.
    assert_eq!(tendril(&store, 80, 1.0), 0.0);

    // Vanilla Warden.handleEntityEvent: event 61 resets tendrilAnimation to 10. Vanilla
    // getTendrilAnimation lerps (tendrilAnimationO, tendrilAnimation) / 10, so right after the
    // event the lerp fades from the previous 0 (partialTick 0) to the new 10 (partialTick 1).
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 80,
        event_id: 61,
    }));
    assert!((tendril(&store, 80, 0.0) - 0.0).abs() < 1.0e-6);
    assert!((tendril(&store, 80, 0.5) - 0.5).abs() < 1.0e-6);
    assert!((tendril(&store, 80, 1.0) - 1.0).abs() < 1.0e-6);

    // Vanilla Warden.tick decrements tendrilAnimation once per client tick (lerp endpoint = current).
    store.advance_entity_client_animations(1);
    assert!((tendril(&store, 80, 1.0) - 0.9).abs() < 1.0e-6);
    store.advance_entity_client_animations(9);
    assert!((tendril(&store, 80, 1.0) - 0.0).abs() < 1.0e-6);
    // It settles at 0 and stays there.
    store.advance_entity_client_animations(5);
    assert!((tendril(&store, 80, 1.0) - 0.0).abs() < 1.0e-6);

    // Only event 61 starts the pulse; other warden events do not.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 80,
        event_id: 4,
    }));
    assert_eq!(tendril(&store, 80, 1.0), 0.0);

    // Event 61 on a non-warden entity never starts the tendril pulse.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 81,
        event_id: 61,
    }));
    assert_eq!(tendril(&store, 81, 1.0), 0.0);
}

#[test]
fn probes_entity_status_from_world_store() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity(123));

    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 123, action: 3 }));
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 123,
        event_id: 35,
    }));
    assert!(store.apply_hurt_animation(ProtocolHurtAnimation { id: 123, yaw: 45.5 }));

    let status = store.probe_entity_status(123).unwrap();

    assert_eq!(status.id, 123);
    assert_eq!(status.entity_type_id, 7);
    assert_eq!(status.last_animation_action, Some(3));
    assert_eq!(status.last_event_id, Some(35));
    assert_eq!(status.last_hurt_yaw, Some(45.5));
    assert!(status.mob_effects.is_empty());
    assert!(status.last_damage.is_none());
    assert!(store.probe_entity_status(999).is_none());
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

fn living_entity_flags_data(flags: i8) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id: 8,
        serializer_id: 0,
        value: EntityDataValueKind::Byte(flags),
    }
}

/// A single decoded chunk of air at (0, 0), used to back block lookups (e.g. the
/// sleeping bed orientation) in entity-source tests.
fn empty_test_chunk() -> crate::ChunkColumn {
    crate::ChunkColumn {
        pos: crate::ChunkPos { x: 0, z: 0 },
        state: crate::ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: vec![crate::ChunkSection {
            non_empty_block_count: 0,
            fluid_count: 0,
            block_states: single_value_container(crate::PaletteDomain::BlockStates, 4096, 0),
            biomes: single_value_container(crate::PaletteDomain::Biomes, 64, 0),
        }],
        block_entities: Vec::new(),
        light: crate::LightData::default(),
    }
}

fn single_value_container(
    domain: crate::PaletteDomain,
    entry_count: usize,
    global_id: i32,
) -> crate::PalettedContainerData {
    crate::PalettedContainerData {
        domain,
        bits_per_entry: 0,
        palette_kind: crate::PaletteKind::SingleValue,
        palette_global_ids: vec![global_id],
        packed_data: Vec::new(),
        entry_count,
    }
}

fn item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
    ProtocolItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}
