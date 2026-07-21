use super::*;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_CROUCHING_ID, VANILLA_ENTITY_POSE_DIGGING_ID, VANILLA_ENTITY_POSE_DYING_ID,
    VANILLA_ENTITY_POSE_EMERGING_ID, VANILLA_ENTITY_POSE_LONG_JUMPING_ID,
    VANILLA_ENTITY_POSE_ROARING_ID, VANILLA_ENTITY_POSE_SITTING_ID,
    VANILLA_ENTITY_POSE_SLEEPING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
    VANILLA_ENTITY_POSE_SWIMMING_ID,
};

fn painting_variant_data(registry_id: i32) -> ProtocolEntityDataValue {
    let data_id = vanilla_entity_metadata_painting_variant_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_PAINTING_ID,
    )
    .expect("generated Painting variant accessor must resolve");
    ProtocolEntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_PAINTING_VARIANT_ID,
        value: EntityDataValueKind::PaintingVariant(bbb_protocol::packets::PaintingVariantData {
            registry: Some(
                bbb_protocol::packets::EntityDataRegistryReference::unresolved(registry_id),
            ),
            direct: None,
        }),
    }
}

/// Deliberately sends the Painting holder field to an ItemFrame to cover slot-9 type gating.
fn painting_variant_collision_data(registry_id: i32) -> ProtocolEntityDataValue {
    painting_variant_data(registry_id)
}

fn item_frame_item_data(entity_type_id: i32, item: ItemStackSummary) -> ProtocolEntityDataValue {
    let accessors = vanilla_entity_metadata_item_frame_accessors_for_entity_type(entity_type_id)
        .expect("generated ItemFrame metadata accessors must resolve");
    ProtocolEntityDataValue {
        data_id: accessors.item_data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
        value: EntityDataValueKind::ItemStack(item),
    }
}

fn entity_pose_data(pose_id: i32) -> ProtocolEntityDataValue {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    assert_eq!(accessors.pose_default, VANILLA_ENTITY_POSE_STANDING_ID);
    protocol_pose_data(accessors.pose_data_id, pose_id)
}

fn pufferfish_puff_state_data(puff_state: i32) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
    )
    .expect("generated Pufferfish puff-state accessor must resolve");
    protocol_int_data(data_id, puff_state)
}

fn slime_size_data(entity_type_id: i32, size: i32) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_slime_size_accessor_for_entity_type(entity_type_id)
        .expect("generated Slime size accessor must resolve");
    protocol_int_data(data_id, size)
}

fn phantom_size_data(size: i32) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_phantom_size_accessor_for_entity_type(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PHANTOM_ID,
    )
    .expect("generated Phantom size accessor must resolve");
    protocol_int_data(data_id, size)
}

fn salmon_variant_data(variant: i32) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_salmon_variant_accessor_for_entity_type(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SALMON_ID,
    )
    .expect("generated Salmon variant accessor must resolve");
    protocol_int_data(data_id, variant)
}

fn camel_baby_data(entity_type_id: i32, is_baby: bool) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_baby_accessor_for_entity_type(entity_type_id)
        .expect("generated Camel inherited baby accessor must resolve");
    protocol_bool_data(data_id, is_baby)
}

fn goat_baby_data(is_baby: bool) -> ProtocolEntityDataValue {
    let (data_id, default) = vanilla_entity_metadata_baby_accessor_for_entity_type(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GOAT_ID,
    )
    .expect("generated Goat inherited baby accessor must resolve");
    assert!(!default);
    protocol_bool_data(data_id, is_baby)
}

fn camel_last_pose_change_tick_data(entity_type_id: i32, tick: i64) -> ProtocolEntityDataValue {
    let (data_id, _) =
        vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
            entity_type_id,
        )
        .expect("generated Camel last-pose-change accessor must resolve");
    protocol_long_data(data_id, tick)
}

fn sniffer_state_data(state_id: i32) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_sniffer_state_accessor_for_entity_type(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNIFFER_ID,
    )
    .expect("generated Sniffer state accessor must resolve");
    protocol_enum_data(data_id, EntityDataEnumSerializer::SnifferState, state_id)
}

fn sniffer_baby_data(is_baby: bool) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_baby_accessor_for_entity_type(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNIFFER_ID,
    )
    .expect("generated Sniffer inherited baby accessor must resolve");
    protocol_bool_data(data_id, is_baby)
}

fn ender_dragon_phase_data(phase_id: i32) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
    )
    .expect("generated EnderDragon phase accessor must resolve");
    protocol_int_data(data_id, phase_id)
}

#[test]
fn ender_dragon_pick_targets_use_vanilla_part_ids_and_bounds() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type_y_rot(
        100,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        0.0,
    ));
    store.advance_entity_client_animations(1);

    assert_eq!(store.probe_entity_pick_bounds(100), None);
    let debug_targets = store.entity_debug_hitbox_targets_at_partial_tick(1.0);
    assert_eq!(
        debug_targets
            .iter()
            .map(|target| target.entity_id)
            .collect::<Vec<_>>(),
        vec![100, 101, 102, 103, 104, 105, 106, 107, 108]
    );
    assert_entity_vec3_close(
        debug_targets[0].position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
    );
    assert_eq!(
        debug_targets[0].bounds,
        EntityPickBoundsState::from_base_size(16.0, 8.0, 0.0)
    );
    let dragon_pose = store
        .probe_entity_camera_pose(100)
        .expect("dragon parent should expose vanilla eye height");
    assert!((dragon_pose.eye_height - 6.8).abs() < 1e-5);

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
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type_y_rot(
        120,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        0.0,
    ));
    store.advance_entity_client_animations(1);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 120,
        values: vec![ender_dragon_phase_data(
            VANILLA_ENDER_DRAGON_PHASE_HOLDING_PATTERN_ID,
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
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type_y_rot(
        130,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        0.0,
    ));
    store.advance_entity_client_animations(1);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 130,
        values: vec![ender_dragon_phase_data(
            VANILLA_ENDER_DRAGON_PHASE_HOLDING_PATTERN_ID,
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
    store.apply_add_entity(protocol_add_entity_with_type_data(
        20,
        VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
        2,
    ));
    store.apply_add_entity(protocol_add_entity_with_type_data(
        21,
        VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
        5,
    ));
    store.apply_add_entity(protocol_add_entity_with_type_data(
        22,
        VANILLA_ENTITY_TYPE_PAINTING_ID,
        3,
    ));
    store.apply_add_entity(protocol_add_entity_with_type_data(
        23,
        VANILLA_ENTITY_TYPE_LEASH_KNOT_ID,
        0,
    ));

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
    assert!(!store.apply_set_entity_data(ProtocolSetEntityData {
        id: 20,
        // Slot 9 belongs to ItemFrame.DATA_ITEM here; a mismatched painting serializer must not
        // populate the Painting-only client projection.
        values: vec![painting_variant_collision_data(21)],
    }));
    assert_eq!(
        store.probe_entity(20).unwrap().painting_variant_projection,
        None
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
        values: vec![item_frame_item_data(
            VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
            ItemStackSummary {
                item_id: Some(999),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added: 1,
                    added_type_ids: vec![46],
                    removed_type_ids: Vec::new(),
                    ..DataComponentPatchSummary::default()
                },
            },
        )],
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
        values: vec![painting_variant_data(21)],
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
    let painting_source = store
        .entity_model_source_at_partial_tick(22, 1.0)
        .expect("painting must project a renderer source");
    assert_eq!(
        painting_source.painting_facing,
        Some(ItemFrameFacing::South)
    );
    assert_eq!(
        painting_source
            .painting_variant_projection
            .as_ref()
            .map(|variant| (variant.width, variant.height, variant.asset_id.as_str())),
        Some((4, 4, "minecraft:pointer"))
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
    store.apply_add_entity(protocol_add_entity_with_type(
        24,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SLIME_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        25,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
    ));

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
        values: vec![slime_size_data(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SLIME_ID,
            4,
        )],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 25,
        values: vec![slime_size_data(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
            3,
        )],
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
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        26,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
    ));

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
        values: vec![pufferfish_puff_state_data(1)],
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
        values: vec![pufferfish_puff_state_data(2)],
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
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        27,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SALMON_ID,
    ));

    assert_eq!(
        store.probe_entity_pick_bounds(27),
        Some(EntityPickBoundsState::from_base_size(0.7, 0.4, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 27,
        values: vec![salmon_variant_data(0)],
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
        values: vec![salmon_variant_data(2)],
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
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        28,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PHANTOM_ID,
    ));

    assert_eq!(
        store.probe_entity_pick_bounds(28),
        Some(EntityPickBoundsState::from_base_size(0.9, 0.5, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 28,
        values: vec![phantom_size_data(4)],
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
                    operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
                },
                ProtocolAttributeModifier {
                    id: "minecraft:add_base_scale".to_string(),
                    amount: 0.5,
                    operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
                },
                ProtocolAttributeModifier {
                    id: "minecraft:multiply_total_scale".to_string(),
                    amount: 1.0,
                    operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
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
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        32,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(33, 83));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 32,
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_CROUCHING_ID)],
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
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_SWIMMING_ID)],
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
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_SLEEPING_ID)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(32),
        Some(EntityPickBoundsState::from_base_size(0.2, 0.2, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 32,
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_DYING_ID)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(32),
        Some(EntityPickBoundsState::from_base_size(0.2, 0.2, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 33,
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_CROUCHING_ID)],
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
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_SLEEPING_ID)],
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
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_SLEEPING_ID)],
    }));
    assert!(!store.local_player_is_sleeping());

    store.apply_login(&protocol_play_login(33));
    assert!(!store.local_player_is_sleeping());
}

#[test]
fn local_player_is_sleeping_false_for_non_sleeping_pose() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(32));
    store.apply_add_entity(protocol_add_entity_with_type(
        32,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 32,
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_STANDING_ID)],
    }));

    assert!(!store.local_player_is_sleeping());
}

#[test]
fn warden_pick_bounds_follow_vanilla_pose_metadata() {
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
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_EMERGING_ID)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(34),
        Some(EntityPickBoundsState::from_base_size(0.9, 1.0, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 34,
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_DIGGING_ID)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(34),
        Some(EntityPickBoundsState::from_base_size(0.9, 1.0, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 34,
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_ROARING_ID)],
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
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        43,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        44,
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
    ));

    assert_eq!(
        store.probe_entity_pick_bounds(43),
        Some(EntityPickBoundsState::from_base_size(1.7, 2.375, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 43,
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_SITTING_ID)],
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
        values: vec![camel_baby_data(VANILLA_ENTITY_TYPE_CAMEL_ID, true)],
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
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_STANDING_ID)],
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
            entity_pose_data(VANILLA_ENTITY_POSE_SITTING_ID),
            camel_baby_data(VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, true),
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
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        45,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        46,
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        47,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
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
            entity_pose_data(VANILLA_ENTITY_POSE_SITTING_ID),
            camel_last_pose_change_tick_data(VANILLA_ENTITY_TYPE_CAMEL_ID, -100),
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
            entity_pose_data(VANILLA_ENTITY_POSE_STANDING_ID),
            camel_last_pose_change_tick_data(VANILLA_ENTITY_TYPE_CAMEL_ID, 200),
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
            entity_pose_data(VANILLA_ENTITY_POSE_SITTING_ID),
            camel_baby_data(VANILLA_ENTITY_TYPE_CAMEL_ID, true),
            camel_last_pose_change_tick_data(VANILLA_ENTITY_TYPE_CAMEL_ID, -100),
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
            entity_pose_data(VANILLA_ENTITY_POSE_SITTING_ID),
            camel_baby_data(VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, true),
            camel_last_pose_change_tick_data(VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, -100),
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
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(41, 62));

    assert_eq!(
        store.probe_entity_pick_bounds(41),
        Some(EntityPickBoundsState::from_base_size(0.9, 1.3, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 41,
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_LONG_JUMPING_ID)],
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
        values: vec![goat_baby_data(true)],
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
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_STANDING_ID)],
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
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(42, 119));

    assert_eq!(
        store.probe_entity_pick_bounds(42),
        Some(EntityPickBoundsState::from_base_size(1.9, 1.75, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 42,
        values: vec![sniffer_state_data(VANILLA_SNIFFER_STATE_DIGGING_ID)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(42),
        Some(EntityPickBoundsState::from_base_size(1.9, 1.75 - 0.4, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 42,
        values: vec![sniffer_baby_data(true)],
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
        values: vec![sniffer_state_data(VANILLA_SNIFFER_STATE_SEARCHING_ID)],
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

fn area_effect_cloud_radius_data(radius: f32) -> ProtocolEntityDataValue {
    let (data_id, default) =
        vanilla_entity_metadata_area_effect_cloud_radius_accessor_for_entity_type(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
        )
        .expect("generated AreaEffectCloud radius accessor must resolve");
    assert_eq!(default, 3.0);
    protocol_float_data(data_id, radius)
}

#[test]
fn area_effect_cloud_debug_bounds_follow_synced_radius_without_pickability() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        40,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
    ));

    // Vanilla `Entity.isPickable` stays false for the cloud, so it never
    // becomes a pick target.
    assert_eq!(store.probe_entity_pick_bounds(40), None);
    assert!(store
        .entity_pick_targets_at_partial_tick(1.0)
        .iter()
        .all(|target| target.entity_id != 40));

    // The debug hitbox still shows the vanilla default radius 3.0 as a
    // 6.0 x 0.5 box.
    let debug_targets = store.entity_debug_hitbox_targets_at_partial_tick(1.0);
    let cloud = debug_targets
        .iter()
        .find(|target| target.entity_id == 40)
        .expect("area effect cloud must keep a debug hitbox");
    assert_eq!(
        cloud.bounds,
        EntityPickBoundsState::from_base_size(6.0, 0.5, 0.0)
    );

    // Server-side decay shrinks the synced radius and the client refreshes
    // dimensions from the metadata update.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 40,
        values: vec![area_effect_cloud_radius_data(1.25)],
    }));
    let debug_targets = store.entity_debug_hitbox_targets_at_partial_tick(1.0);
    let cloud = debug_targets
        .iter()
        .find(|target| target.entity_id == 40)
        .expect("area effect cloud must keep a debug hitbox");
    assert_eq!(
        cloud.bounds,
        EntityPickBoundsState::from_base_size(2.5, 0.5, 0.0)
    );
}

#[test]
fn debug_hitboxes_cover_non_pickable_entities_and_skip_invisible_ones() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        41,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        42,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));

    // Dropped items are not pickable in vanilla but still draw their white
    // debug box.
    assert_eq!(store.probe_entity_pick_bounds(41), None);
    let debug_ids: Vec<i32> = store
        .entity_debug_hitbox_targets_at_partial_tick(1.0)
        .iter()
        .map(|target| target.entity_id)
        .collect();
    assert_eq!(debug_ids, vec![41, 42]);

    // Vanilla `EntityHitboxDebugRenderer` skips `entity.isInvisible()` while
    // pick targeting keeps working for invisible entities.
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 42,
        values: vec![protocol_byte_data(
            accessors.shared_flags_data_id,
            accessors.invisible_mask,
        )],
    }));
    let debug_ids: Vec<i32> = store
        .entity_debug_hitbox_targets_at_partial_tick(1.0)
        .iter()
        .map(|target| target.entity_id)
        .collect();
    assert_eq!(debug_ids, vec![41]);
    assert!(store
        .entity_pick_targets_at_partial_tick(1.0)
        .iter()
        .any(|target| target.entity_id == 42));
}

fn probe_eye_height(store: &WorldStore, id: i32) -> f32 {
    store
        .probe_entity_camera_pose(id)
        .expect("entity camera pose must resolve")
        .eye_height
}

#[test]
fn variant_eye_heights_use_vanilla_with_eye_height_declarations() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        50,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        51,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        52,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
    ));

    // Adult chicken keeps the type-table eye height.
    assert!((probe_eye_height(&store, 50) - 0.644).abs() < 1.0e-6);
    // Vanilla `Chicken.BABY_DIMENSIONS = scalable(0.3, 0.4).withEyeHeight(0.28)`;
    // the proportional ratio would give 0.644 * 0.4 / 0.7 = 0.368.
    let (chicken_baby_id, _) = vanilla_entity_metadata_baby_accessor_for_entity_type(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHICKEN_ID,
    )
    .expect("chicken baby accessor");
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![protocol_bool_data(chicken_baby_id, true)],
    }));
    assert!((probe_eye_height(&store, 50) - 0.28).abs() < 1.0e-6);

    // Vanilla camel `SITTING_DIMENSIONS.withEyeHeight(0.845)`; proportional
    // from the 2.275 standing eye would give 0.905.
    assert!((probe_eye_height(&store, 51) - 2.275).abs() < 1.0e-6);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 51,
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_SITTING_ID)],
    }));
    assert!((probe_eye_height(&store, 51) - 0.845).abs() < 1.0e-6);

    // Vanilla `AbstractCow.BABY_DIMENSIONS = scale(0.5).withEyeHeight(0.665)`;
    // the plain half-scale ratio would give 0.65.
    let (cow_baby_id, _) = vanilla_entity_metadata_baby_accessor_for_entity_type(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
    )
    .expect("cow baby accessor");
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 52,
        values: vec![protocol_bool_data(cow_baby_id, true)],
    }));
    assert!((probe_eye_height(&store, 52) - 0.665).abs() < 1.0e-6);
}

#[test]
fn avatar_pose_eye_heights_follow_vanilla_pose_map() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        53,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));

    assert!((probe_eye_height(&store, 53) - 1.62).abs() < 1.0e-6);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 53,
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_CROUCHING_ID)],
    }));
    // Vanilla crouching eye height is the declared 1.27, not the
    // height-proportional 1.62 * 1.5 / 1.8 = 1.35.
    assert!((probe_eye_height(&store, 53) - 1.27).abs() < 1.0e-6);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 53,
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_SWIMMING_ID)],
    }));
    assert!((probe_eye_height(&store, 53) - 0.4).abs() < 1.0e-6);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 53,
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_DYING_ID)],
    }));
    // The dying fixed box keeps the standing eye height in vanilla's pose map.
    assert!((probe_eye_height(&store, 53) - 1.62).abs() < 1.0e-6);
}
