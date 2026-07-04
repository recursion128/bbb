use super::*;

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
fn camel_body_anchor_y_offset_matches_vanilla_sit_stand_curve() {
    const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;
    const ENTITY_DATA_POSE_ID: u8 = 6;
    const AGEABLE_BABY_DATA_ID: u8 = 16;
    const CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID: u8 = 20;
    const POSE_STANDING: i32 = 0;
    const POSE_SITTING: i32 = 10;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(45, 19));
    store.apply_add_entity(protocol_add_entity_with_type(46, 20));
    store.apply_add_entity(protocol_add_entity_with_type(47, 30));
    store.apply_world_time(ProtocolPlayTime {
        game_time: 200,
        clock_updates: Vec::new(),
    });

    let anchor = |store: &WorldStore, id, front| {
        store
            .entity_body_anchor_y_offset(id, front, 0.0)
            .unwrap_or_else(|| panic!("missing body anchor for {id}"))
    };
    let assert_close = |actual: f32, expected: f32| {
        assert!(
            (actual - expected).abs() < 1.0e-5,
            "actual={actual} expected={expected}"
        );
    };

    // Vanilla `Camel.getBodyAnchorAnimationYOffset`: standing adult dimensions are
    // 2.375 high and the base seat anchor is `height - 0.375`.
    assert_close(anchor(&store, 45, true), 2.0);
    assert_close(anchor(&store, 45, false), 2.0);
    assert_eq!(store.entity_body_anchor_y_offset(47, true, 0.0), None);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 45,
        values: vec![
            protocol_pose_data(ENTITY_DATA_POSE_ID, POSE_SITTING),
            protocol_long_data(CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID, -100),
        ],
    }));
    store.apply_world_time(ProtocolPlayTime {
        game_time: 128,
        clock_updates: Vec::new(),
    });
    // Sit-down tick 28 is the vanilla front flex point: base sitting height
    // 0.945 - 0.375 plus `(1.43 - 0.5 * 1.23)`.
    assert_close(anchor(&store, 45, true), 1.385);
    // The rear anchor uses the smaller 0.1 flex offset, so it remains much higher
    // at the same keyframe.
    assert_close(anchor(&store, 45, false), 1.877);

    store.apply_world_time(ProtocolPlayTime {
        game_time: 160,
        clock_updates: Vec::new(),
    });
    // Once sitting, the anchor is the sitting dimensions base plus `0.2`.
    assert_close(anchor(&store, 45, true), 0.77);
    assert_close(anchor(&store, 45, false), 0.77);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 45,
        values: vec![
            protocol_pose_data(ENTITY_DATA_POSE_ID, POSE_STANDING),
            protocol_long_data(CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID, 200),
        ],
    }));
    store.apply_world_time(ProtocolPlayTime {
        game_time: 224,
        clock_updates: Vec::new(),
    });
    // Stand-up tick 24 is the front flex point: standing base 2.0 plus
    // `0.2 - (1.43 - 0.6 * 1.23)`.
    assert_close(anchor(&store, 45, true), 1.508);
    store.apply_world_time(ProtocolPlayTime {
        game_time: 232,
        clock_updates: Vec::new(),
    });
    // The rear stand-up flex point is later (tick 32) and uses offset 0.35.
    assert_close(anchor(&store, 45, false), 1.2005);

    store.apply_world_time(ProtocolPlayTime {
        game_time: 260,
        clock_updates: Vec::new(),
    });
    assert_close(anchor(&store, 45, true), 2.0);
    assert_close(anchor(&store, 45, false), 2.0);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 45,
        values: vec![
            protocol_pose_data(ENTITY_DATA_POSE_ID, POSE_SITTING),
            protocol_bool_data(AGEABLE_BABY_DATA_ID, true),
            protocol_long_data(CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID, -100),
        ],
    }));
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 45,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    store.apply_world_time(ProtocolPlayTime {
        game_time: 160,
        clock_updates: Vec::new(),
    });
    // Normal baby camel: dimensions and the anchor scale both use ageScale 0.6,
    // then the SCALE attribute doubles both.
    assert_close(anchor(&store, 45, true), 0.924);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 46,
        values: vec![
            protocol_pose_data(ENTITY_DATA_POSE_ID, POSE_SITTING),
            protocol_bool_data(AGEABLE_BABY_DATA_ID, true),
            protocol_long_data(CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID, -100),
        ],
    }));
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 46,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    // Vanilla `CamelHusk.isBaby()` is always false, so the husk ignores the baby
    // flag and only applies the SCALE attribute.
    assert_close(anchor(&store, 46, true), 1.54);
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
