use super::*;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_AXOLOTL_ID, VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_PAINTING_ID,
    VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
};

#[test]
fn tracks_entity_lifecycle_and_absolute_state_updates() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity(123));

    let entity = store.probe_entity(123).unwrap();
    assert_eq!(entity.entity_type_id, VANILLA_ENTITY_TYPE_AXOLOTL_ID);
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
    assert_eq!(entity.y_head_rot, 30.0);
    assert_eq!(entity.on_ground, Some(true));

    assert!(store.apply_entity_move(ProtocolEntityMove {
        id: 123,
        has_position: true,
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
            x: 2.0,
            y: 65.0,
            z: -3.0,
        }
    );
    assert_eq!(
        entity.position_base,
        EntityVec3 {
            x: 3.0,
            y: 65.0,
            z: -3.5,
        }
    );
    assert_eq!(entity.interpolation.as_ref().unwrap().remaining_steps, 3);
    assert_eq!(entity.y_rot, 180.0);
    assert_eq!(entity.x_rot, 30.0);
    assert_eq!(entity.on_ground, Some(false));
    store.advance_entity_client_animations(3);
    let entity = store.probe_entity(123).unwrap();
    assert_eq!(entity.position, entity.position_base);
    assert_eq!(entity.y_rot, 270.0);
    assert_eq!(entity.x_rot, 45.0);
    assert_eq!(entity.interpolation, None);

    assert!(store.apply_entity_move(ProtocolEntityMove {
        id: 123,
        has_position: false,
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
    assert_eq!(entity.y_rot, 270.0);
    assert_eq!(entity.x_rot, 45.0);
    assert_eq!(entity.on_ground, Some(true));
    assert_eq!(entity.interpolation.as_ref().unwrap().remaining_steps, 3);
    store.advance_entity_client_animations(3);
    let entity = store.probe_entity(123).unwrap();
    assert_eq!(entity.y_rot, 30.0);
    assert_eq!(entity.x_rot, -15.0);
    assert_eq!(entity.interpolation, None);

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
        entity
            .interpolation
            .as_ref()
            .map(|state| state.target_position),
        Some(EntityVec3 {
            x: 3.5,
            y: 70.0,
            z: -4.0,
        })
    );
    assert_eq!(entity.position, entity.position_base);
    assert_eq!(
        entity.delta_movement,
        EntityVec3 {
            x: 0.0,
            y: 0.2,
            z: 0.0,
        }
    );
    assert_eq!(entity.on_ground, Some(true));
    store.advance_entity_client_animations(3);
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
    assert_eq!(entity.interpolation, None);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![entity_invisible_data(), entity_air_supply_data(300)],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![entity_air_supply_data(301)],
    }));
    let entity = store.probe_entity(123).unwrap();
    assert_eq!(
        entity.data_values,
        vec![entity_invisible_data(), entity_air_supply_data(301)]
    );
    assert_eq!(
        store
            .entities
            .metadata(123)
            .unwrap()
            .data_values
            .into_value(),
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
        store
            .entities
            .equipment(123)
            .unwrap()
            .equipment
            .into_value(),
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
                attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
                base: 20.0,
                modifiers: vec![ProtocolAttributeModifier {
                    id: "minecraft:health_bonus".to_string(),
                    amount: 4.0,
                    operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
                }],
            },
            ProtocolAttributeSnapshot {
                attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                base: 0.7,
                modifiers: Vec::new(),
            },
        ],
    }));
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
            base: 0.9,
            modifiers: vec![ProtocolAttributeModifier {
                id: "minecraft:speed_bonus".to_string(),
                amount: 0.2,
                operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
            }],
        }],
    }));
    assert!(!store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 999,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
            base: 20.0,
            modifiers: Vec::new(),
        }],
    }));
    let entity = store.probe_entity(123).unwrap();
    assert_eq!(
        entity.attributes,
        vec![
            ProtocolAttributeSnapshot {
                attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
                base: 20.0,
                modifiers: vec![ProtocolAttributeModifier {
                    id: "minecraft:health_bonus".to_string(),
                    amount: 4.0,
                    operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
                }],
            },
            ProtocolAttributeSnapshot {
                attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                base: 0.9,
                modifiers: vec![ProtocolAttributeModifier {
                    id: "minecraft:speed_bonus".to_string(),
                    amount: 0.2,
                    operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
                }],
            },
        ]
    );
    assert_eq!(
        store
            .entities
            .attributes(123)
            .unwrap()
            .attributes
            .into_value(),
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
        has_position: false,
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
fn camera_visual_distance_uses_living_attributes_mount_maximum_and_nonliving_default() {
    let mut store = WorldStore::new();
    store.local_player_id = Some(123);
    store.apply_add_entity(protocol_add_entity_with_type(
        123,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert_eq!(store.camera_entity_third_person_distance(), Some(4.0));

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![
            ProtocolAttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
                base: 2.0,
                modifiers: Vec::new(),
            },
            ProtocolAttributeSnapshot {
                attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_CAMERA_DISTANCE_ID,
                base: 3.0,
                modifiers: Vec::new(),
            },
        ],
    }));
    assert_eq!(store.camera_entity_third_person_distance(), Some(6.0));
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_CAMERA_DISTANCE_ID,
            base: 100.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(store.camera_entity_third_person_distance(), Some(64.0));
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_CAMERA_DISTANCE_ID,
            base: 3.0,
            modifiers: Vec::new(),
        }],
    }));

    store.apply_add_entity(protocol_add_entity_with_type(
        124,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 124,
        attributes: vec![
            ProtocolAttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
                base: 3.0,
                modifiers: Vec::new(),
            },
            ProtocolAttributeSnapshot {
                attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_CAMERA_DISTANCE_ID,
                base: 4.0,
                modifiers: Vec::new(),
            },
        ],
    }));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 124,
        passenger_ids: vec![123],
    }));
    assert_eq!(store.camera_entity_third_person_distance(), Some(12.0));

    store.apply_add_entity(protocol_add_entity_with_type(
        125,
        VANILLA_ENTITY_TYPE_PAINTING_ID,
    ));
    assert!(store.apply_set_camera(ProtocolSetCamera { camera_id: 125 }));
    assert_eq!(store.camera_entity_third_person_distance(), Some(4.0));
}

#[test]
fn relative_move_handler_overrides_snap_non_interpolating_entities() {
    let mut store = WorldStore::new();
    for (id, entity_type_id) in [
        (201, VANILLA_ENTITY_TYPE_ITEM_ID),
        (202, VANILLA_ENTITY_TYPE_SHULKER_ID),
        (203, VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID),
    ] {
        store.apply_add_entity(protocol_add_entity_with_type(id, entity_type_id));
        assert!(store.apply_entity_move(ProtocolEntityMove {
            id,
            has_position: true,
            delta_x: 4096,
            delta_y: 0,
            delta_z: 0,
            y_rot: Some(450.0),
            x_rot: Some(-450.0),
            on_ground: true,
        }));
    }

    for id in [201, 203] {
        let entity = store.probe_entity(id).unwrap();
        assert_eq!(entity.position, entity.position_base);
        assert_eq!(entity.interpolation, None);
    }
    let shulker = store.probe_entity(202).unwrap();
    assert_eq!(
        shulker.position_base,
        EntityVec3 {
            x: 2.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert_eq!(
        shulker.position,
        EntityVec3 {
            x: 2.5,
            y: 64.0,
            z: -1.5,
        }
    );
    assert_eq!(shulker.interpolation, None);
    for id in [201, 202] {
        let entity = store.probe_entity(id).unwrap();
        assert_eq!(entity.y_rot, 90.0);
        assert_eq!(entity.x_rot, -90.0);
    }
    let display = store.probe_entity(203).unwrap();
    assert_eq!(display.y_rot, 450.0);
    assert_eq!(display.x_rot, -90.0);
}

#[test]
fn rotation_only_retarget_keeps_the_active_position_target() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity(204));
    let initial = store.probe_entity(204).unwrap();
    assert!(store.apply_entity_move(ProtocolEntityMove {
        id: 204,
        has_position: true,
        delta_x: 4096,
        delta_y: 0,
        delta_z: 0,
        y_rot: None,
        x_rot: None,
        on_ground: false,
    }));
    store.advance_entity_client_animations(1);
    assert!(store.apply_entity_move(ProtocolEntityMove {
        id: 204,
        has_position: false,
        delta_x: 0,
        delta_y: 0,
        delta_z: 0,
        y_rot: Some(90.0),
        x_rot: Some(0.0),
        on_ground: false,
    }));

    let retargeted = store.probe_entity(204).unwrap();
    let interpolation = retargeted.interpolation.unwrap();
    assert_eq!(interpolation.remaining_steps, 3);
    assert_eq!(interpolation.target_position.x, initial.position.x + 1.0);
    assert_eq!(interpolation.target_position.y, initial.position.y);
    assert_eq!(interpolation.target_position.z, initial.position.z);
    store.advance_entity_client_animations(3);
    let finished = store.probe_entity(204).unwrap();
    assert_eq!(finished.position, finished.position_base);
    assert_eq!(finished.y_rot, 90.0);
    assert_eq!(finished.x_rot, 0.0);
    assert_eq!(finished.interpolation, None);
}

#[test]
fn position_sync_uses_loaded_chunk_ticking_distance_and_keeps_packet_velocity_unchanged() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        204,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 204,
            position: ProtocolVec3d {
                x: 3.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: ProtocolVec3d::default(),
            y_rot: 40.0,
            x_rot: 10.0,
            on_ground: true,
        })
    );
    assert_eq!(store.entities.transform(204).unwrap().position.x, 3.0);
    assert_eq!(store.entities.transform(204).unwrap().interpolation, None);

    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(999));
    store.apply_add_entity(protocol_add_entity(205));

    assert!(
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 205,
            position: ProtocolVec3d {
                x: 2.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: ProtocolVec3d {
                x: 4.0,
                y: 5.0,
                z: 6.0,
            },
            y_rot: 40.0,
            x_rot: 10.0,
            on_ground: true,
        })
    );
    let snapped = store.entities.transform(205).unwrap();
    assert_eq!(snapped.position.x, 2.0);
    assert_eq!(snapped.old_position, snapped.position);
    assert_eq!(snapped.delta_movement, EntityVec3::default());
    assert_eq!(snapped.interpolation, None);

    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    assert!(
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 205,
            position: ProtocolVec3d {
                x: 5.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: ProtocolVec3d {
                x: 7.0,
                y: 8.0,
                z: 9.0,
            },
            y_rot: 80.0,
            x_rot: 20.0,
            on_ground: false,
        })
    );
    let interpolating = store.entities.transform(205).unwrap();
    assert_eq!(interpolating.position.x, 2.0);
    assert_eq!(interpolating.position_base.x, 5.0);
    assert_eq!(interpolating.delta_movement, EntityVec3::default());
    assert_eq!(
        interpolating
            .interpolation
            .map(|state| state.target_position.x),
        Some(5.0)
    );

    assert!(
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 205,
            position: ProtocolVec3d {
                x: 66.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: ProtocolVec3d::default(),
            y_rot: 100.0,
            x_rot: 25.0,
            on_ground: true,
        })
    );
    let boundary = store.entities.transform(205).unwrap();
    assert_eq!(boundary.position.x, 2.0);
    assert_eq!(
        boundary.interpolation.map(|state| state.target_position.x),
        Some(66.0),
        "a squared displacement of exactly 4096 still interpolates"
    );

    assert!(
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 205,
            position: ProtocolVec3d {
                x: 67.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: ProtocolVec3d::default(),
            y_rot: 120.0,
            x_rot: 30.0,
            on_ground: true,
        })
    );
    let threshold = store.entities.transform(205).unwrap();
    assert_eq!(threshold.position.x, 67.0);
    assert_eq!(threshold.old_position.x, 67.0);
    assert!(
        threshold.interpolation.is_some(),
        "snapTo does not cancel a handler target"
    );
}

#[test]
fn teleport_resolves_relative_values_from_active_target_and_snap_old_endpoints() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity(206));
    assert!(store.apply_entity_move(ProtocolEntityMove {
        id: 206,
        has_position: true,
        delta_x: 4096,
        delta_y: 0,
        delta_z: 0,
        y_rot: Some(40.0),
        x_rot: Some(10.0),
        on_ground: false,
    }));
    assert!(store.apply_teleport_entity(ProtocolTeleportEntity {
        id: 206,
        position: ProtocolVec3d {
            x: 0.5,
            y: 70.0,
            z: -4.0,
        },
        delta_movement: ProtocolVec3d::default(),
        y_rot: 5.0,
        x_rot: 15.0,
        relatives_mask: PLAYER_RELATIVE_X | PLAYER_RELATIVE_Y_ROT,
        on_ground: true,
    }));
    let active = store.entities.transform(206).unwrap();
    let active_target = active.interpolation.unwrap();
    assert_eq!(active_target.target_position.x, 2.5);
    assert_eq!(active_target.target_position.y, 70.0);
    assert_eq!(active_target.target_y_rot, 45.0);
    assert_eq!(active_target.target_x_rot, 15.0);

    store
        .entities
        .with_transform_mut(206, |transform| {
            transform.cancel_interpolation();
            transform.position = EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            };
            transform.y_rot = 20.0;
            transform.x_rot = 5.0;
            transform.set_old_pos_and_rot_to(
                EntityVec3 {
                    x: 0.0,
                    y: 63.0,
                    z: -3.0,
                },
                5.0,
                -5.0,
            );
        })
        .unwrap();
    assert!(store.apply_teleport_entity(ProtocolTeleportEntity {
        id: 206,
        position: ProtocolVec3d {
            x: 100.0,
            y: 80.0,
            z: 9.0,
        },
        delta_movement: ProtocolVec3d::default(),
        y_rot: 10.0,
        x_rot: 20.0,
        relatives_mask: PLAYER_RELATIVE_X | PLAYER_RELATIVE_Y_ROT,
        on_ground: false,
    }));
    let snapped = store.entities.transform(206).unwrap();
    assert_eq!(snapped.position.x, 101.0);
    assert_eq!(snapped.old_position.x, 100.0);
    assert_eq!(snapped.old_position.y, 80.0);
    assert_eq!(snapped.y_rot, 30.0);
    assert_eq!(snapped.old_y_rot, 15.0);
    assert_eq!(snapped.x_rot, 20.0);
    assert_eq!(snapped.old_x_rot, 20.0);
}

#[test]
fn client_tick_captures_transform_and_head_render_endpoints_before_lerp() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity(207));
    assert!(store.apply_entity_move(ProtocolEntityMove {
        id: 207,
        has_position: true,
        delta_x: 4096,
        delta_y: 0,
        delta_z: 0,
        y_rot: Some(80.0),
        x_rot: Some(20.0),
        on_ground: false,
    }));
    assert!(store.apply_rotate_head(ProtocolRotateHead {
        id: 207,
        y_head_rot: 90.0,
    }));

    store.advance_entity_client_animations(1);
    let advanced = store.entities.transform(207).unwrap();
    assert_eq!(advanced.old_position.x, 1.0);
    assert!((advanced.position.x - 4.0 / 3.0).abs() < 1.0e-12);
    assert_eq!(advanced.old_y_rot, 20.0);
    assert_eq!(advanced.y_rot, 40.0);
    assert_eq!(advanced.old_x_rot, -10.0);
    assert_eq!(advanced.x_rot, 0.0);
    assert_eq!(advanced.old_y_head_rot, 30.0);
    assert_eq!(advanced.y_head_rot, 50.0);
}

#[test]
fn partial_tick_queries_share_canonical_transform_render_endpoints() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        214,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    store.advance_entity_client_animations(1);
    store.entities.with_transform_mut(214, |transform| {
        transform.old_position = EntityVec3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        transform.position = EntityVec3 {
            x: 5.0,
            y: 6.0,
            z: 7.0,
        };
        transform.old_y_rot = 170.0;
        transform.y_rot = -170.0;
        transform.old_x_rot = -20.0;
        transform.x_rot = 20.0;
        transform.old_y_head_rot = 170.0;
        transform.y_head_rot = -170.0;
    });

    let old_candidate = store.entity_render_candidates_at_partial_tick(0.0)[0];
    let mid_candidate = store.entity_render_candidates_at_partial_tick(0.5)[0];
    let current_candidate = store.entity_render_candidates_at_partial_tick(1.0)[0];
    assert_eq!(
        old_candidate.position,
        EntityVec3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        }
    );
    assert_eq!(
        mid_candidate.position,
        EntityVec3 {
            x: 3.0,
            y: 4.0,
            z: 5.0,
        }
    );
    assert_eq!(
        current_candidate.position,
        EntityVec3 {
            x: 5.0,
            y: 6.0,
            z: 7.0,
        }
    );

    let old_source = store.entity_model_sources_at_partial_tick(0.0)[0].clone();
    let mid_source = store.entity_model_sources_at_partial_tick(0.5)[0].clone();
    let current_source = store.entity_model_sources_at_partial_tick(1.0)[0].clone();
    assert_eq!(old_source.position, old_candidate.position);
    assert_eq!(mid_source.position, mid_candidate.position);
    assert_eq!(current_source.position, current_candidate.position);
    assert_eq!(
        (old_source.y_rot, old_source.x_rot, old_source.y_head_rot),
        (170.0, -20.0, 170.0)
    );
    assert_eq!(
        (mid_source.y_rot, mid_source.x_rot, mid_source.y_head_rot),
        (180.0, 0.0, 180.0)
    );
    assert_eq!(
        (
            current_source.y_rot,
            current_source.x_rot,
            current_source.y_head_rot
        ),
        (190.0, 20.0, 190.0)
    );

    let pick = store.entity_pick_targets_at_partial_tick(0.5);
    let debug = store.entity_debug_hitbox_targets_at_partial_tick(0.5);
    assert_eq!(pick[0].position, mid_candidate.position);
    assert_eq!(debug[0].position, mid_candidate.position);
}

#[test]
fn interpolation_and_head_lerp_pause_outside_ticking_client_sections() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(999));
    store.apply_add_entity(protocol_add_entity(208));
    assert!(store.apply_entity_move(ProtocolEntityMove {
        id: 208,
        has_position: true,
        delta_x: 4096,
        delta_y: 0,
        delta_z: 0,
        y_rot: Some(80.0),
        x_rot: Some(20.0),
        on_ground: false,
    }));
    assert!(store.apply_rotate_head(ProtocolRotateHead {
        id: 208,
        y_head_rot: 90.0,
    }));

    store.advance_entity_client_animations(1);
    let paused = store.entities.transform(208).unwrap();
    assert_eq!(paused.position.x, 1.0);
    assert_eq!(paused.y_head_rot, 30.0);
    assert_eq!(paused.interpolation.unwrap().remaining_steps, 3);

    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    store.advance_entity_client_animations(1);
    let ticking = store.entities.transform(208).unwrap();
    assert!((ticking.position.x - 4.0 / 3.0).abs() < 1.0e-12);
    assert_eq!(ticking.y_head_rot, 50.0);
    assert_eq!(ticking.interpolation.unwrap().remaining_steps, 2);
}

#[test]
fn interpolation_carries_independent_motion_only_into_collision_free_target_bounds() {
    let make_store = || {
        let mut store = WorldStore::with_dimension(crate::WorldDimension {
            min_y: 0,
            height: 16,
        });
        store.insert_decoded_chunk(empty_test_chunk());
        store.apply_add_entity(ProtocolAddEntity {
            id: 209,
            uuid: default_entity_uuid(),
            entity_type_id: VANILLA_ENTITY_TYPE_COW_ID,
            position: ProtocolVec3d {
                x: 1.5,
                y: 1.0,
                z: 0.5,
            },
            delta_movement: ProtocolVec3d::default(),
            y_rot: 20.0,
            x_rot: 0.0,
            y_head_rot: 20.0,
            data: 0,
        });
        assert!(store.apply_entity_move(ProtocolEntityMove {
            id: 209,
            has_position: true,
            delta_x: 4096,
            delta_y: 0,
            delta_z: 0,
            y_rot: Some(40.0),
            x_rot: Some(0.0),
            on_ground: false,
        }));
        store
            .entities
            .with_transform_mut(209, |transform| {
                transform.position.x += 1.0;
                transform.y_rot += 30.0;
            })
            .unwrap();
        store
    };

    let mut clear = make_store();
    clear.advance_entity_client_animations(1);
    let carried = clear.entities.transform(209).unwrap();
    assert_eq!(
        carried.interpolation.unwrap().target_position.x,
        3.5,
        "collision-free client motion carries into the packet target"
    );
    assert!((carried.position.x - (2.5 + 1.0 / 3.0)).abs() < 1.0e-12);
    assert_eq!(carried.interpolation.unwrap().target_y_rot, 70.0);

    let mut blocked = make_store();
    let stone = vanilla_block_state_id("minecraft:stone", []);
    assert!(blocked.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 3, y: 1, z: 0 },
        block_state_id: stone,
    }));
    blocked.advance_entity_client_animations(1);
    let rejected = blocked.entities.transform(209).unwrap();
    assert_eq!(rejected.interpolation.unwrap().target_position.x, 2.5);
    assert_eq!(rejected.position.x, 2.5);
    assert_eq!(
        rejected.interpolation.unwrap().target_y_rot,
        70.0,
        "rotation carry is independent of the position collision result"
    );

    let mut entity_blocked = make_store();
    entity_blocked.apply_add_entity(ProtocolAddEntity {
        id: 210,
        uuid: Uuid::from_u128(0x210),
        entity_type_id: VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        position: ProtocolVec3d {
            x: 3.5,
            y: 1.0,
            z: 0.5,
        },
        delta_movement: ProtocolVec3d::default(),
        y_rot: 0.0,
        x_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    entity_blocked.advance_entity_client_animations(1);
    let entity_rejected = entity_blocked.entities.transform(209).unwrap();
    assert_eq!(
        entity_rejected.interpolation.unwrap().target_position.x,
        2.5,
        "another collidable entity rejects target carry without a second target scan"
    );
}

#[test]
fn turtle_pairing_state_uses_generated_defaults_and_resets_on_readd() {
    let entity_id = 199;
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type_data(
        entity_id,
        VANILLA_ENTITY_TYPE_TURTLE_ID,
        73,
    ));

    let turtle =
        vanilla_entity_metadata_turtle_accessors_for_entity_type(VANILLA_ENTITY_TYPE_TURTLE_ID)
            .expect("generated Turtle accessors");
    let living = vanilla_entity_metadata_living_entity_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_TURTLE_ID,
    )
    .expect("generated LivingEntity accessors for Turtle");
    let spawned = store.probe_entity(entity_id).unwrap();
    assert_eq!(spawned.entity_type_id, VANILLA_ENTITY_TYPE_TURTLE_ID);
    assert_eq!(spawned.data, 73, "AddEntity data stays diagnostic state");
    assert!(spawned.data_values.is_empty());
    assert!(spawned.attributes.is_empty());
    assert_eq!(turtle.has_egg_default, false);
    assert_eq!(turtle.laying_egg_default, false);
    assert_eq!(living.health_default, 1.0);
    assert_eq!(
        store.entities.attribute_value(
            entity_id,
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
        ),
        Some(30.0)
    );
    assert_eq!(
        store.entities.attribute_value(
            entity_id,
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
        ),
        Some(0.25)
    );
    assert_eq!(
        store.entities.attribute_base_value(
            entity_id,
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_STEP_HEIGHT_ID,
        ),
        Some(1.0)
    );
    assert_eq!(
        store.entities.metadata_bool_for_entity(
            entity_id,
            turtle.has_egg_data_id,
            turtle.has_egg_default,
        ),
        Some(false)
    );
    assert_eq!(
        store.entities.metadata_bool_for_entity(
            entity_id,
            turtle.laying_egg_data_id,
            turtle.laying_egg_default,
        ),
        Some(false)
    );

    // ServerEntity.sendPairingData bundles non-default synced data and any
    // instantiated client-syncable attributes after AddEntity. A packet value
    // overrides the same constructor supplier defaults retained above.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: entity_id,
        values: vec![
            protocol_float_data(living.health_data_id, 30.0),
            protocol_bool_data(turtle.has_egg_data_id, true),
            protocol_bool_data(turtle.laying_egg_data_id, true),
        ],
    }));
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id,
        attributes: vec![
            ProtocolAttributeSnapshot {
                attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
                base: 42.0,
                modifiers: Vec::new(),
            },
            ProtocolAttributeSnapshot {
                attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                base: 0.4,
                modifiers: Vec::new(),
            },
            ProtocolAttributeSnapshot {
                attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_STEP_HEIGHT_ID,
                base: 1.5,
                modifiers: Vec::new(),
            },
        ],
    }));
    assert_eq!(store.probe_entity(entity_id).unwrap().attributes.len(), 3);
    assert_eq!(
        store.entities.attribute_value(
            entity_id,
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
        ),
        Some(42.0)
    );

    store.apply_add_entity(protocol_add_entity_with_type_data(
        entity_id,
        VANILLA_ENTITY_TYPE_TURTLE_ID,
        0,
    ));
    let replaced = store.probe_entity(entity_id).unwrap();
    assert_eq!(replaced.data, 0);
    assert!(replaced.data_values.is_empty());
    assert!(replaced.attributes.is_empty());
    assert_eq!(
        store.entities.attribute_value(
            entity_id,
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
        ),
        Some(30.0)
    );
    assert_eq!(
        store.entities.metadata_bool_for_entity(
            entity_id,
            turtle.has_egg_data_id,
            turtle.has_egg_default,
        ),
        Some(false)
    );
    assert_eq!(store.entity_count(), 1);
}

#[test]
fn every_fixed_entity_type_exposes_its_generated_default_attribute_supplier() {
    let mut store = WorldStore::new();

    for entry in VANILLA_ENTITY_TYPE_METADATA_26_1 {
        store.apply_add_entity(protocol_add_entity_with_type(
            entry.entity_type_id,
            entry.entity_type_id,
        ));
        assert!(
            store
                .probe_entity(entry.entity_type_id)
                .unwrap()
                .attributes
                .is_empty(),
            "{} must keep constructor defaults virtual",
            entry.resource_id,
        );

        for attribute in bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_TYPES {
            let expected = entry
                .default_attributes
                .iter()
                .find(|default| default.attribute_id == attribute.id)
                .map(|default| {
                    default
                        .base_value_override_bits
                        .unwrap_or(attribute.default_value_bits)
                })
                .map(f64::from_bits);
            assert_eq!(
                store
                    .entities
                    .attribute_base_value(entry.entity_type_id, attribute.id),
                expected,
                "{} / {}",
                entry.resource_id,
                attribute.resource_id,
            );
            assert_eq!(
                store
                    .entities
                    .attribute_value(entry.entity_type_id, attribute.id),
                expected,
                "{} / {}",
                entry.resource_id,
                attribute.resource_id,
            );
        }
    }

    assert_eq!(
        store.entity_count(),
        VANILLA_ENTITY_TYPE_METADATA_26_1.len()
    );
}

#[test]
fn level_reset_and_id_reuse_restore_generated_attribute_defaults() {
    let entity_id = 205;
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        entity_id,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
            base: 41.0,
            modifiers: Vec::new(),
        }],
    }));
    let generation_a = store.publication_snapshot();

    store.clear_client_level();
    assert!(store.probe_entity(entity_id).is_none());
    store.apply_add_entity(protocol_add_entity_with_type(
        entity_id,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    let generation_b = store.publication_snapshot();

    assert_eq!(
        generation_a.entities.attribute_base_value(
            entity_id,
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID
        ),
        Some(41.0),
    );
    assert_eq!(
        generation_b.entities.attribute_base_value(
            entity_id,
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID
        ),
        Some(10.0),
    );
    assert!(generation_b
        .probe_entity(entity_id)
        .unwrap()
        .attributes
        .is_empty());
}

#[test]
fn rotate_head_lerps_living_entities_over_three_client_ticks() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        200,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        201,
        VANILLA_ENTITY_TYPE_ARROW_ID,
    ));

    assert!(store.apply_rotate_head(ProtocolRotateHead {
        id: 200,
        y_head_rot: 90.0,
    }));
    assert_eq!(store.probe_entity(200).unwrap().y_head_rot, 30.0);
    store.advance_entity_client_animations(1);
    assert_eq!(store.probe_entity(200).unwrap().y_head_rot, 50.0);
    store.advance_entity_client_animations(1);
    assert_eq!(store.probe_entity(200).unwrap().y_head_rot, 70.0);
    store.advance_entity_client_animations(1);
    assert_eq!(store.probe_entity(200).unwrap().y_head_rot, 90.0);

    assert!(store.apply_rotate_head(ProtocolRotateHead {
        id: 200,
        y_head_rot: -150.0,
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(store.probe_entity(200).unwrap().y_head_rot, 130.0);

    assert!(store.apply_rotate_head(ProtocolRotateHead {
        id: 201,
        y_head_rot: 90.0,
    }));
    assert_eq!(store.probe_entity(201).unwrap().y_head_rot, 90.0);
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
        values: vec![entity_invisible_data()],
    }));

    assert_eq!(store.counters().entity_data_updates_received, 1);
    assert_eq!(store.counters().entity_data_values_received, 1);
    assert_eq!(store.counters().entity_data_updates_applied, 0);
    assert_eq!(store.counters().entity_data_updates_ignored, 1);
}

#[test]
fn set_entity_data_rejects_unknown_or_mismatched_schema_without_partial_mutation() {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity(123));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![entity_air_supply_data(42)],
    }));
    let generation_a = store.publication_snapshot();
    let expected = generation_a.probe_entity(123).unwrap().data_values;

    assert!(!store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![
            entity_invisible_data(),
            ProtocolEntityDataValue {
                data_id: accessors.air_supply_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
        ],
    }));
    assert_eq!(store.probe_entity(123).unwrap().data_values, expected);
    assert!(store
        .apply_diagnostics()
        .last_apply_error
        .as_ref()
        .is_some_and(|error| error.source == "entity_data_schema"
            && error.message.contains("expected serializer")));

    assert!(!store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![ProtocolEntityDataValue {
            data_id: u8::MAX - 1,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(0),
        }],
    }));
    assert_eq!(store.probe_entity(123).unwrap().data_values, expected);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![entity_invisible_data()],
    }));
    let generation_b = store.publication_snapshot();
    assert_eq!(
        generation_a.probe_entity(123).unwrap().data_values,
        expected
    );
    assert_ne!(
        generation_b.probe_entity(123).unwrap().data_values,
        expected
    );
    assert_eq!(store.counters().entity_data_updates_received, 4);
    assert_eq!(store.counters().entity_data_updates_applied, 2);
    assert_eq!(store.counters().entity_data_updates_ignored, 2);
    assert_eq!(store.counters().world_apply_errors, 2);
}

#[test]
fn set_entity_data_preserves_rich_component_and_rejects_invalid_owner_payload_atomically() {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity(123));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![entity_air_supply_data(42)],
    }));
    let expected = store.probe_entity(123).unwrap().data_values;

    let invalid = bbb_protocol::RichTextComponent::try_from_component_nbt(
        bbb_protocol::NbtSummaryValue::Compound(vec![
            bbb_protocol::NbtSummaryEntry {
                name: "text".to_string(),
                value: bbb_protocol::NbtSummaryValue::String("Open".to_string()),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "click_event".to_string(),
                value: bbb_protocol::NbtSummaryValue::Compound(vec![
                    bbb_protocol::NbtSummaryEntry {
                        name: "action".to_string(),
                        value: bbb_protocol::NbtSummaryValue::String("show_dialog".to_string()),
                    },
                    bbb_protocol::NbtSummaryEntry {
                        name: "dialog".to_string(),
                        value: bbb_protocol::NbtSummaryValue::String("example:missing".to_string()),
                    },
                ]),
            },
        ]),
    )
    .unwrap();
    assert!(!store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![
            entity_invisible_data(),
            ProtocolEntityDataValue {
                data_id: accessors.custom_name_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_COMPONENT_ID,
                value: EntityDataValueKind::OptionalComponent(Some(invalid)),
            },
        ],
    }));
    assert_eq!(store.probe_entity(123).unwrap().data_values, expected);
    assert!(store
        .apply_diagnostics()
        .last_apply_error
        .as_ref()
        .is_some_and(|error| error.source == "entity_data_component"
            && error.message.contains("example:missing")));

    let rich = bbb_protocol::RichTextComponent::try_from_component_nbt(
        bbb_protocol::NbtSummaryValue::Compound(vec![
            bbb_protocol::NbtSummaryEntry {
                name: "text".to_string(),
                value: bbb_protocol::NbtSummaryValue::String("Alice".to_string()),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "bold".to_string(),
                value: bbb_protocol::NbtSummaryValue::Byte(1),
            },
        ]),
    )
    .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![ProtocolEntityDataValue {
            data_id: accessors.custom_name_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_COMPONENT_ID,
            value: EntityDataValueKind::OptionalComponent(Some(rich.clone())),
        }],
    }));
    let generation_a = store.publication_snapshot();
    assert_eq!(
        generation_a
            .probe_entity(123)
            .unwrap()
            .data_values
            .iter()
            .find(|value| value.data_id == accessors.custom_name_data_id)
            .map(|value| &value.value),
        Some(&EntityDataValueKind::OptionalComponent(Some(rich)))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![ProtocolEntityDataValue {
            data_id: accessors.custom_name_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_COMPONENT_ID,
            value: EntityDataValueKind::OptionalComponent(Some(
                bbb_protocol::RichTextComponent::plain("Bob"),
            )),
        }],
    }));
    let generation_b = store.publication_snapshot();
    assert_eq!(
        generation_a
            .probe_entity(123)
            .unwrap()
            .data_values
            .iter()
            .find(|value| value.data_id == accessors.custom_name_data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::OptionalComponent(Some(component)) => {
                    Some(component.text())
                }
                _ => None,
            }),
        Some("Alice")
    );
    assert_eq!(
        generation_b
            .probe_entity(123)
            .unwrap()
            .data_values
            .iter()
            .find(|value| value.data_id == accessors.custom_name_data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::OptionalComponent(Some(component)) => {
                    Some(component.text())
                }
                _ => None,
            }),
        Some("Bob")
    );
}

#[test]
fn direct_painting_components_reject_atomically_and_recover() {
    let data_id = crate::vanilla_entity_metadata_painting_variant_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_PAINTING_ID,
    )
    .expect("generated Painting variant accessor must resolve");
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        124,
        VANILLA_ENTITY_TYPE_PAINTING_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 124,
        values: vec![ProtocolEntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_PAINTING_VARIANT_ID,
            value: EntityDataValueKind::PaintingVariant(
                bbb_protocol::packets::PaintingVariantData {
                    registry: None,
                    direct: Some(bbb_protocol::packets::DirectPaintingVariantData {
                        width: 1,
                        height: 1,
                        asset_id: "example:old".to_string(),
                        title: Some(bbb_protocol::RichTextComponent::plain("Old")),
                        author: Some(bbb_protocol::RichTextComponent::plain("Author")),
                    }),
                },
            ),
        }],
    }));
    let expected = store.probe_entity(124).unwrap().data_values;

    let invalid_author = bbb_protocol::RichTextComponent::try_from_component_nbt(
        bbb_protocol::NbtSummaryValue::Compound(vec![
            bbb_protocol::NbtSummaryEntry {
                name: "text".to_string(),
                value: bbb_protocol::NbtSummaryValue::String("Invalid".to_string()),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "click_event".to_string(),
                value: bbb_protocol::NbtSummaryValue::Compound(vec![
                    bbb_protocol::NbtSummaryEntry {
                        name: "action".to_string(),
                        value: bbb_protocol::NbtSummaryValue::String("show_dialog".to_string()),
                    },
                    bbb_protocol::NbtSummaryEntry {
                        name: "dialog".to_string(),
                        value: bbb_protocol::NbtSummaryValue::String("example:missing".to_string()),
                    },
                ]),
            },
        ]),
    )
    .unwrap();
    assert!(!store.apply_set_entity_data(ProtocolSetEntityData {
        id: 124,
        values: vec![
            entity_invisible_data(),
            ProtocolEntityDataValue {
                data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_PAINTING_VARIANT_ID,
                value: EntityDataValueKind::PaintingVariant(
                    bbb_protocol::packets::PaintingVariantData {
                        registry: None,
                        direct: Some(bbb_protocol::packets::DirectPaintingVariantData {
                            width: 2,
                            height: 1,
                            asset_id: "example:invalid".to_string(),
                            title: Some(bbb_protocol::RichTextComponent::plain("New")),
                            author: Some(invalid_author),
                        }),
                    },
                ),
            },
        ],
    }));
    assert_eq!(store.probe_entity(124).unwrap().data_values, expected);
    assert!(store
        .apply_diagnostics()
        .last_apply_error
        .as_ref()
        .is_some_and(|error| error.source == "entity_data_component"
            && error.message.contains("painting variant author")
            && error.message.contains("example:missing")));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 124,
        values: vec![ProtocolEntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_PAINTING_VARIANT_ID,
            value: EntityDataValueKind::PaintingVariant(
                bbb_protocol::packets::PaintingVariantData {
                    registry: None,
                    direct: Some(bbb_protocol::packets::DirectPaintingVariantData {
                        width: 2,
                        height: 1,
                        asset_id: "example:new".to_string(),
                        title: Some(bbb_protocol::RichTextComponent::plain("New")),
                        author: Some(bbb_protocol::RichTextComponent::plain("Recovered")),
                    }),
                },
            ),
        }],
    }));
    assert!(store
        .probe_entity(124)
        .unwrap()
        .data_values
        .iter()
        .find_map(|value| match &value.value {
            EntityDataValueKind::PaintingVariant(variant) => variant.direct.as_ref(),
            _ => None,
        })
        .is_some_and(
            |direct| direct.title.as_ref().map(|title| title.text()) == Some("New")
                && direct.author.as_ref().map(|author| author.text()) == Some("Recovered")
        ));
}

#[test]
fn set_entity_data_binds_every_top_level_registry_holder_shape_to_one_session_identity() {
    let cow_accessors =
        crate::vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_COW_ID,
        )
        .unwrap();
    let painting_data_id =
        crate::vanilla_entity_metadata_painting_variant_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PAINTING_ID,
        )
        .unwrap();
    let villager_data_id = crate::vanilla_entity_metadata_villager_data_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_VILLAGER_ID,
    )
    .unwrap();

    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:cow_variant",
        0,
        vec![
            crate::RegistryPacketEntry::stub("example:first"),
            crate::RegistryPacketEntry::stub("example:second"),
        ],
    );
    store.record_registry_entries(
        "minecraft:painting_variant",
        0,
        vec![crate::RegistryPacketEntry::stub("example:wide")],
    );
    store.apply_add_entity(protocol_add_entity_with_type(
        500,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        501,
        VANILLA_ENTITY_TYPE_PAINTING_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        502,
        VANILLA_ENTITY_TYPE_VILLAGER_ID,
    ));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 500,
        values: vec![ProtocolEntityDataValue {
            data_id: cow_accessors.variant_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_COW_VARIANT_ID,
            value: EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::CowVariant,
                value: EntityDataRegistryReference::unresolved(0),
            },
        }],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 501,
        values: vec![ProtocolEntityDataValue {
            data_id: painting_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_PAINTING_VARIANT_ID,
            value: EntityDataValueKind::PaintingVariant(
                bbb_protocol::packets::PaintingVariantData {
                    registry: Some(EntityDataRegistryReference::unresolved(0)),
                    direct: None,
                },
            ),
        }],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 502,
        values: vec![ProtocolEntityDataValue {
            data_id: villager_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_VILLAGER_DATA_ID,
            value: EntityDataValueKind::VillagerData {
                villager_type: EntityDataRegistryReference::unresolved(0),
                profession: EntityDataRegistryReference::unresolved(0),
                level: 3,
            },
        }],
    }));

    let cow = entity_registry_reference(&store, 500, cow_accessors.variant_data_id);
    let cow_identity = cow.identity().unwrap();
    assert_eq!(cow_identity.registry(), "minecraft:cow_variant");
    assert_eq!(cow_identity.resource_id(), "example:first");
    assert_eq!(cow_identity.source(), EntityDataRegistrySource::Server);
    let first_revision = cow_identity.revision();

    let painting = store
        .probe_entity(501)
        .unwrap()
        .data_values
        .into_iter()
        .find_map(|value| match value.value {
            EntityDataValueKind::PaintingVariant(variant) => variant.registry,
            _ => None,
        })
        .unwrap();
    assert_eq!(painting.identity().unwrap().resource_id(), "example:wide");

    let (villager_type, profession) = store
        .probe_entity(502)
        .unwrap()
        .data_values
        .into_iter()
        .find_map(|value| match value.value {
            EntityDataValueKind::VillagerData {
                villager_type,
                profession,
                ..
            } => Some((villager_type, profession)),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        villager_type.identity().unwrap().registry(),
        "minecraft:villager_type"
    );
    assert_eq!(
        profession.identity().unwrap().registry(),
        "minecraft:villager_profession"
    );
    assert_eq!(
        villager_type.identity().unwrap().source(),
        EntityDataRegistrySource::VanillaFallback
    );

    let before_missing = store.probe_entity(500).unwrap().data_values;
    assert!(!store.apply_set_entity_data(ProtocolSetEntityData {
        id: 500,
        values: vec![
            entity_invisible_data(),
            ProtocolEntityDataValue {
                data_id: cow_accessors.variant_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_COW_VARIANT_ID,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::CowVariant,
                    value: EntityDataRegistryReference::unresolved(99),
                },
            },
        ],
    }));
    assert_eq!(store.probe_entity(500).unwrap().data_values, before_missing);
    assert!(store
        .apply_diagnostics()
        .last_apply_error
        .as_ref()
        .is_some_and(|error| error.source == "entity_data_registry"
            && error.message.contains("cow_variant holder id 99")));

    let generation_a = store.publication_snapshot();
    store.reset_registry_session();
    store.record_registry_entries(
        "minecraft:cow_variant",
        0,
        vec![
            crate::RegistryPacketEntry::stub("example:second"),
            crate::RegistryPacketEntry::stub("example:first"),
        ],
    );
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 500,
        values: vec![ProtocolEntityDataValue {
            data_id: cow_accessors.variant_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_COW_VARIANT_ID,
            value: EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::CowVariant,
                value: EntityDataRegistryReference::unresolved(0),
            },
        }],
    }));
    let generation_b = store.publication_snapshot();
    let old = entity_registry_reference(&generation_a, 500, cow_accessors.variant_data_id);
    let new = entity_registry_reference(&generation_b, 500, cow_accessors.variant_data_id);
    assert_eq!(old.identity().unwrap().resource_id(), "example:first");
    assert_eq!(new.identity().unwrap().resource_id(), "example:second");
    assert_eq!(old.identity().unwrap().revision(), first_revision);
    assert_ne!(
        old.identity().unwrap().revision().session,
        new.identity().unwrap().revision().session
    );

    assert_eq!(
        store.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![500],
        }),
        1
    );
    store.apply_add_entity(protocol_add_entity_with_type(
        500,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 500,
        values: vec![ProtocolEntityDataValue {
            data_id: cow_accessors.variant_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_COW_VARIANT_ID,
            value: EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::CowVariant,
                value: EntityDataRegistryReference::unresolved(1),
            },
        }],
    }));
    assert_eq!(
        entity_registry_reference(&store, 500, cow_accessors.variant_data_id)
            .identity()
            .unwrap()
            .resource_id(),
        "example:first"
    );
}

fn entity_registry_reference(
    store: &WorldStore,
    entity_id: i32,
    data_id: u8,
) -> EntityDataRegistryReference {
    store
        .probe_entity(entity_id)
        .unwrap()
        .data_values
        .into_iter()
        .find_map(|value| {
            (value.data_id == data_id)
                .then_some(value.value)
                .and_then(|value| match value {
                    EntityDataValueKind::RegistryId { value, .. } => Some(value),
                    _ => None,
                })
        })
        .unwrap()
}

#[test]
fn local_player_using_item_tracks_living_entity_flags() {
    let accessors = living_entity_metadata_accessors();
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
        values: vec![living_entity_flags_data(accessors.is_using_mask)],
    }));
    assert!(store.local_player().interaction.using_item);
    assert_eq!(
        store.local_player().interaction.using_item_hand,
        Some(InteractionHand::MainHand)
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![living_entity_flags_data(accessors.flags_default)],
    }));
    assert!(!store.local_player().interaction.using_item);
    assert_eq!(store.local_player().interaction.using_item_hand, None);

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 40,
        item: item_stack(43, 1),
    });
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![living_entity_flags_data(
            accessors.is_using_mask | accessors.off_hand_mask,
        )],
    }));
    assert!(store.local_player().interaction.using_item);
    assert_eq!(
        store.local_player().interaction.using_item_hand,
        Some(InteractionHand::OffHand)
    );
}

#[test]
fn local_player_using_item_flags_do_not_start_with_empty_hand() {
    let accessors = living_entity_metadata_accessors();
    let mut store = WorldStore::new();
    store.local_player_id = Some(123);
    store.apply_add_entity(protocol_add_entity_with_type(
        123,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![living_entity_flags_data(accessors.is_using_mask)],
    }));

    assert!(!store.local_player().interaction.using_item);
    assert_eq!(store.local_player().interaction.using_item_hand, None);
}

#[test]
fn local_player_using_item_flags_do_not_replace_existing_started_hand() {
    let accessors = living_entity_metadata_accessors();
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
        values: vec![living_entity_flags_data(
            accessors.is_using_mask | accessors.off_hand_mask,
        )],
    }));

    assert!(store.local_player().interaction.using_item);
    assert_eq!(
        store.local_player().interaction.using_item_hand,
        Some(InteractionHand::MainHand)
    );
}

#[test]
fn primed_tnt_smoke_particles_use_current_position_when_post_decrement_fuse_survives() {
    let accessors =
        vanilla_entity_metadata_primed_tnt_accessors_for_entity_type(VANILLA_ENTITY_TYPE_TNT_ID)
            .expect("generated PrimedTnt metadata accessors");
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
        values: vec![protocol_int_data(accessors.fuse_data_id, 2)],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 201,
        values: vec![protocol_int_data(accessors.fuse_data_id, 1)],
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

    let states = store
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    assert_eq!(states.len(), 3);
    assert!(matches!(
        states.first(),
        Some(EntityClientMovementEffectState::Sound(
            EntityClientMovementSoundState {
                entity_id: 210,
                kind: EntityClientMovementSoundKind::FireworkRocketLaunch,
                ..
            }
        ))
    ));
    let trails = states
        .into_iter()
        .filter_map(|state| match state {
            EntityClientMovementEffectState::Particle(
                EntityClientMovementParticleState::FireworkRocket(state),
            ) => Some(state),
            EntityClientMovementEffectState::Sound(_) => None,
            other => panic!("unexpected movement particle: {other:?}"),
        })
        .collect::<Vec<_>>();
    let first_delta = EntityVec3 {
        x: 0.2 * 1.15,
        y: 0.8 + 0.04,
        z: -0.4 * 1.15,
    };
    let second_delta = EntityVec3 {
        x: first_delta.x * 1.15,
        y: first_delta.y + 0.04,
        z: first_delta.z * 1.15,
    };
    assert_eq!(trails[0].entity_id, 210);
    assert_eq!(
        trails[0].position,
        EntityVec3 {
            x: 3.5 + first_delta.x,
            y: 70.25 + first_delta.y,
            z: -6.75 + first_delta.z,
        }
    );
    assert_eq!(trails[0].delta_movement, first_delta);
    assert_eq!(trails[1].entity_id, 210);
    assert_eq!(
        trails[1].position,
        EntityVec3 {
            x: 3.5 + first_delta.x + second_delta.x,
            y: 70.25 + first_delta.y + second_delta.y,
            z: -6.75 + first_delta.z + second_delta.z,
        }
    );
    assert_eq!(trails[1].delta_movement, second_delta);
    assert!(store
        .drain_entity_client_movement_effect_states()
        .next()
        .is_none());
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
            attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
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
fn update_attributes_resolves_session_holders_and_validates_entity_supplier() {
    use bbb_protocol::attribute_types::{
        VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID, VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
        VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
    };
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        124,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));
    let attribute_keys = [
        "minecraft:block_interaction_range",
        "minecraft:movement_speed",
        "minecraft:max_health",
        "example:custom_attribute",
    ];
    let session_holder_id = |resource_id: &str| {
        attribute_keys
            .iter()
            .position(|candidate| *candidate == resource_id)
            .unwrap() as i32
    };
    store.record_registry_entries(
        "minecraft:attribute",
        0,
        attribute_keys
            .iter()
            .map(|resource_id| crate::RegistryPacketEntry::stub(*resource_id))
            .collect(),
    );

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 124,
        attributes: vec![
            ProtocolAttributeSnapshot {
                attribute_id: session_holder_id("minecraft:block_interaction_range"),
                base: 64.0,
                modifiers: Vec::new(),
            },
            ProtocolAttributeSnapshot {
                attribute_id: session_holder_id("minecraft:movement_speed"),
                base: 0.31,
                modifiers: Vec::new(),
            },
            ProtocolAttributeSnapshot {
                attribute_id: session_holder_id("minecraft:max_health"),
                base: 40.0,
                modifiers: Vec::new(),
            },
            ProtocolAttributeSnapshot {
                attribute_id: session_holder_id("example:custom_attribute"),
                base: 99.0,
                modifiers: Vec::new(),
            },
        ],
    }));
    assert_eq!(
        store.probe_entity(124).unwrap().attributes,
        vec![
            ProtocolAttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
                base: 40.0,
                modifiers: Vec::new(),
            },
            ProtocolAttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                base: 0.31,
                modifiers: Vec::new(),
            },
        ]
    );
    assert_eq!(store.counters().entity_attribute_updates_applied, 1);
    assert_eq!(store.counters().entity_attribute_updates_ignored, 1);

    let retained = store.publication_snapshot();
    store.reset_registry_session();
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 124,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID,
            base: 32.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(store.probe_entity(124).unwrap().attributes.len(), 2);
    assert_eq!(retained.probe_entity(124).unwrap().attributes.len(), 2);
    assert_eq!(
        retained.registry_resource_id_by_holder_id("minecraft:attribute", 0),
        Some("minecraft:block_interaction_range")
    );
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:attribute", 0),
        Some("minecraft:armor")
    );

    assert_eq!(
        store.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![124],
        }),
        1
    );
    store.apply_add_entity(protocol_add_entity_with_type(
        124,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 124,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID,
            base: 7.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity(124).unwrap().attributes,
        vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID,
            base: 7.0,
            modifiers: Vec::new(),
        }]
    );
    assert_eq!(
        retained.probe_entity(124).unwrap().entity_type_id,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID
    );
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
fn entity_pick_bounds_follow_vanilla_is_pickable() {
    let mut store = WorldStore::new();
    let interaction_accessors = vanilla_entity_metadata_interaction_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_INTERACTION_ID,
    )
    .expect("generated Interaction metadata accessors must resolve");
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
    store.apply_add_entity(protocol_add_entity_with_type(
        19,
        VANILLA_ENTITY_TYPE_INTERACTION_ID,
    ));
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
    store.apply_add_entity(protocol_add_entity_with_type(
        76,
        VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        77,
        VANILLA_ENTITY_TYPE_ARROW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        78,
        VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        79,
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID,
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
    for entity_id in [73, 74, 75, 76, 77, 78, 79, 80] {
        assert_eq!(store.probe_entity_pick_bounds(entity_id), None);
    }
    for entity_id in [77, 78, 79, 80] {
        assert!(store
            .entity_model_source_at_partial_tick(entity_id, 1.0)
            .is_some());
    }
    assert_eq!(
        store.probe_entity_pick_bounds(19),
        Some(EntityPickBoundsState::from_base_size(1.0, 1.0, 0.0))
    );
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 19,
        values: vec![
            ProtocolEntityDataValue {
                data_id: interaction_accessors.width_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: EntityDataValueKind::Float(2.5),
            },
            ProtocolEntityDataValue {
                data_id: interaction_accessors.height_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: EntityDataValueKind::Float(0.75),
            },
            ProtocolEntityDataValue {
                data_id: interaction_accessors.response_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
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
    store.apply_add_entity(protocol_add_entity_with_type(
        126,
        VANILLA_ENTITY_TYPE_ARROW_ID,
    ));

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
    assert!((store.probe_entity_camera_pose(126).unwrap().eye_height - 0.13).abs() < 0.0001);
    assert_eq!(store.probe_entity_camera_pose(404), None);
}

#[test]
fn entity_camera_pose_uses_partial_transform_endpoints() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        127,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    store.advance_entity_client_animations(1);
    store.entities.with_transform_mut(127, |transform| {
        transform.old_position = EntityVec3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        transform.position = EntityVec3 {
            x: 5.0,
            y: 6.0,
            z: 7.0,
        };
        transform.old_y_rot = 170.0;
        transform.y_rot = -170.0;
        transform.old_x_rot = -20.0;
        transform.x_rot = 20.0;
    });

    let pose = store
        .probe_entity_camera_pose_at_partial_tick(127, 0.5)
        .unwrap();
    assert_eq!(
        pose.position,
        EntityVec3 {
            x: 3.0,
            y: 4.0,
            z: 5.0,
        }
    );
    assert_eq!(pose.y_rot, 180.0);
    assert_eq!(pose.x_rot, 0.0);
    assert_eq!(store.probe_entity_camera_pose(127).unwrap().y_rot, -170.0);
}
