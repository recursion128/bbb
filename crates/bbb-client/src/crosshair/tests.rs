use super::*;
use bbb_client_world::{
    ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
    PalettedContainerData, WorldDimension,
};
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID, VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
};
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID, VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
};
use bbb_protocol::packets::{
    AddEntity, AdventureModePredicateDataSummary, AttackRangeSummary, BlockPredicateSummary,
    EntityDataValue, EntityDataValueKind, EntityPositionSync, GameEvent, HolderSetSummary,
    InitializeBorder, InteractionHand, ItemStackSummary, PlayerAbilities, SetEntityData,
    SetPlayerInventory, Vec3d as ProtocolVec3d,
};
use uuid::Uuid;

const VANILLA_GRASS_BLOCK_STATE_ID: i32 = 9;

fn ender_dragon_phase_data(phase_id: i32) -> EntityDataValue {
    let (data_id, _) =
        bbb_client_world::vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        )
        .expect("generated EnderDragon phase accessor must resolve");
    EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(phase_id),
    }
}

pub(super) fn eye_height(pose: LocalPlayerPoseState) -> f64 {
    WorldStore::new().local_player_eye_height(pose)
}

#[test]
fn crosshair_raycast_hits_first_selectable_block() {
    let pose = player_pose(0.0, 0.0, 0.0);
    let hit = raycast_crosshair_block(pose, 5.0, |pos| {
        if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
            Some(bbb_client_world::TerrainMaterialClass::Opaque)
        } else {
            Some(bbb_client_world::TerrainMaterialClass::Empty)
        }
    });

    assert_eq!(hit, Some(BlockPos { x: 0, y: 1, z: 3 }));
}

#[test]
fn crosshair_raycast_reports_hit_face() {
    let pose = player_pose(0.0, 0.0, 0.0);

    let hit = raycast_crosshair_block_hit(pose, 5.0, |pos| {
        if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
            Some(BlockOutlineTarget::full_block(
                bbb_client_world::TerrainMaterialClass::Opaque,
            ))
        } else {
            None
        }
    });

    assert_eq!(
        hit,
        Some(CrosshairBlockHit {
            pos: BlockPos { x: 0, y: 1, z: 3 },
            face: ProtocolDirection::North,
            cursor: [0.0, eye_height(pose) - 1.0, 0.0],
            inside: false,
        })
    );
}

#[test]
fn item_use_pov_raycast_selects_source_and_flowing_fluid_modes() {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    insert_air_chunk(&mut world);
    let pos = ProtocolBlockPos { x: 0, y: 1, z: 0 };
    let source_water = bbb_client_world::vanilla_block_state_id(
        "minecraft:water",
        &std::collections::BTreeMap::from([("level".to_string(), "0".to_string())]),
    )
    .expect("source water state");
    let flowing_water = bbb_client_world::vanilla_block_state_id(
        "minecraft:water",
        &std::collections::BTreeMap::from([("level".to_string(), "1".to_string())]),
    )
    .expect("flowing water state");
    assert!(
        world.apply_block_update(bbb_protocol::packets::BlockUpdate {
            pos,
            block_state_id: source_water,
        })
    );
    let pose = player_pose(0.0, 0.0, 0.0);
    let query = |fluid_mode| LocalItemUsePovQuery {
        fluid_mode,
        max_distance: 4.5,
    };

    assert_eq!(
        item_use_pov_block_hit_from_world(&world, pose, query(LocalItemUsePovFluidMode::None)),
        None
    );
    assert_eq!(
        item_use_pov_block_hit_from_world(
            &world,
            pose,
            query(LocalItemUsePovFluidMode::SourceOnly)
        )
        .map(|hit| hit.pos),
        Some(BlockPos { x: 0, y: 1, z: 0 })
    );

    assert!(
        world.apply_block_update(bbb_protocol::packets::BlockUpdate {
            pos,
            block_state_id: flowing_water,
        })
    );
    assert_eq!(
        item_use_pov_block_hit_from_world(
            &world,
            pose,
            query(LocalItemUsePovFluidMode::SourceOnly)
        ),
        None
    );
    assert_eq!(
        item_use_pov_block_hit_from_world(&world, pose, query(LocalItemUsePovFluidMode::Any))
            .map(|hit| hit.pos),
        Some(BlockPos { x: 0, y: 1, z: 0 })
    );

    assert!(
        world.apply_block_update(bbb_protocol::packets::BlockUpdate {
            pos,
            block_state_id: source_water,
        })
    );
    assert!(
        world.apply_block_update(bbb_protocol::packets::BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 2, z: 0 },
            block_state_id: source_water,
        })
    );
    assert_eq!(
        item_use_pov_block_hit_from_world(
            &world,
            player_pose_at([0.0, 0.33, 0.0], 0.0, 0.0),
            query(LocalItemUsePovFluidMode::SourceOnly),
        )
        .map(|hit| hit.pos),
        Some(BlockPos { x: 0, y: 1, z: 0 })
    );
}

#[test]
fn crosshair_raycast_clips_partial_outline_shape() {
    let pose = player_pose(0.0, 0.0, 0.0);

    let hit = raycast_crosshair_block_hit(pose, 5.0, |pos| {
        if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
            Some(BlockOutlineTarget::from_box(
                bbb_client_world::TerrainMaterialClass::Opaque,
                [0.0, 0.5, 0.0],
                [1.0, 1.0, 1.0],
            ))
        } else {
            None
        }
    });

    assert_eq!(
        hit,
        Some(CrosshairBlockHit {
            pos: BlockPos { x: 0, y: 1, z: 3 },
            face: ProtocolDirection::North,
            cursor: [0.0, eye_height(pose) - 1.0, 0.0],
            inside: false,
        })
    );
}

#[test]
fn crosshair_raycast_skips_partial_outline_shape() {
    let pose = player_pose(0.0, 0.0, 0.0);
    let hit = raycast_crosshair_block_hit(pose, 5.0, |pos| {
        if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
            Some(BlockOutlineTarget::from_box(
                bbb_client_world::TerrainMaterialClass::Opaque,
                [0.0, 0.0, 0.0],
                [1.0, 0.5, 1.0],
            ))
        } else if pos == (BlockPos { x: 0, y: 1, z: 4 }) {
            Some(BlockOutlineTarget::full_block(
                bbb_client_world::TerrainMaterialClass::Opaque,
            ))
        } else {
            None
        }
    });

    assert_eq!(
        hit,
        Some(CrosshairBlockHit {
            pos: BlockPos { x: 0, y: 1, z: 4 },
            face: ProtocolDirection::North,
            cursor: [0.0, eye_height(pose) - 1.0, 0.0],
            inside: false,
        })
    );
}

#[test]
fn crosshair_raycast_visits_blocks_by_grid_boundary() {
    let pose = player_pose(0.0, 0.0, 0.0);
    let hit = raycast_crosshair_block(pose, 5.0, |pos| {
        if pos == (BlockPos { x: 0, y: 1, z: 1 }) {
            Some(bbb_client_world::TerrainMaterialClass::Opaque)
        } else {
            Some(bbb_client_world::TerrainMaterialClass::Empty)
        }
    });

    assert_eq!(hit, Some(BlockPos { x: 0, y: 1, z: 1 }));
}

#[test]
fn crosshair_raycast_ignores_fluid_blocks() {
    let pose = player_pose(0.0, 0.0, 0.0);
    let hit = raycast_crosshair_block(pose, 5.0, |pos| {
        if pos == (BlockPos { x: 0, y: 1, z: 2 }) {
            Some(bbb_client_world::TerrainMaterialClass::Fluid)
        } else if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
            Some(bbb_client_world::TerrainMaterialClass::Opaque)
        } else {
            Some(bbb_client_world::TerrainMaterialClass::Empty)
        }
    });

    assert_eq!(hit, Some(BlockPos { x: 0, y: 1, z: 3 }));
}

#[test]
fn crosshair_raycast_uses_vanilla_default_block_interaction_range() {
    let pose = player_pose(0.0, 0.0, 0.0);
    let hit = raycast_crosshair_block(pose, DEFAULT_BLOCK_INTERACTION_RANGE, |pos| {
        if pos == (BlockPos { x: 0, y: 1, z: 5 }) {
            Some(bbb_client_world::TerrainMaterialClass::Opaque)
        } else {
            Some(bbb_client_world::TerrainMaterialClass::Empty)
        }
    });

    assert_eq!(DEFAULT_BLOCK_INTERACTION_RANGE, 4.5);
    assert_eq!(hit, None);
}

#[test]
fn crosshair_target_hits_pickable_entity_from_world_state() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        10,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [0.0, 1.0, 3.0],
    ));
    let target = crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0)));

    let CrosshairTarget::Entity(hit) = target.unwrap() else {
        panic!("expected entity hit");
    };
    let eye_height = eye_height(LocalPlayerPoseState::default());
    assert_eq!(hit.entity_id, 10);
    assert_vec3_close(hit.location, [0.0, eye_height, 2.509999990463257]);
    assert_vec3_close(
        hit.relative_location,
        [0.0, eye_height - 1.0, -0.49000000953674316],
    );
}

#[test]
fn crosshair_target_skips_unknown_and_non_pickable_entity_types() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        10,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        [0.0, 1.0, 3.0],
    ));
    world.apply_add_entity(protocol_add_entity(11, 999, [0.0, 1.0, 2.0]));

    assert_eq!(
        crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0))),
        None
    );
}

#[test]
fn crosshair_target_skips_marker_armor_stand() {
    let mut world = WorldStore::new();
    let accessors =
        bbb_client_world::vanilla_entity_metadata_armor_stand_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        )
        .expect("generated ArmorStand metadata accessors must resolve");
    world.apply_add_entity(protocol_add_entity(
        12,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        [0.0, 1.0, 3.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 12,
        values: vec![EntityDataValue {
            data_id: accessors.client_flags_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(accessors.marker_mask),
        }],
    }));

    assert_eq!(
        crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0))),
        None
    );
}

#[test]
fn crosshair_target_hits_interaction_entity_with_metadata_bounds() {
    let mut world = WorldStore::new();
    let accessors =
        bbb_client_world::vanilla_entity_metadata_interaction_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_INTERACTION_ID,
        )
        .expect("generated Interaction metadata accessors must resolve");
    world.apply_add_entity(protocol_add_entity(
        12,
        VANILLA_ENTITY_TYPE_INTERACTION_ID,
        [0.0, 1.0, 3.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 12,
        values: vec![
            EntityDataValue {
                data_id: accessors.width_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: EntityDataValueKind::Float(2.0),
            },
            EntityDataValue {
                data_id: accessors.height_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: EntityDataValueKind::Float(1.25),
            },
        ],
    }));

    let target = crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0)));

    let CrosshairTarget::Entity(hit) = target.unwrap() else {
        panic!("expected interaction entity hit");
    };
    assert_eq!(hit.entity_id, 12);
    assert_vec3_close(hit.relative_location, [0.0, 0.6200000047683716, -1.0]);
}

#[test]
fn crosshair_target_hits_ender_dragon_part_id_from_world_state() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        100,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        [0.0, 2.0, 9.0],
    ));
    world.advance_entity_client_animations(1);
    let target = crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0)));

    let CrosshairTarget::Entity(hit) = target.unwrap() else {
        panic!("expected dragon part entity hit");
    };
    let eye_height = eye_height(LocalPlayerPoseState::default());
    assert_eq!(hit.entity_id, 101);
    assert_vec3_close(hit.location, [0.0, eye_height, 2.0]);
    assert_vec3_close(hit.relative_location, [0.0, eye_height - 1.0, -0.5]);
}

#[test]
fn crosshair_target_hits_ender_dragon_tail_and_wing_part_ids() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        100,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        [0.0, 2.0, 9.0],
    ));
    world.advance_entity_client_animations(1);

    let eye_height = eye_height(LocalPlayerPoseState::default());
    let tail_pose = player_pose_at([0.0, 4.5 - eye_height, 11.25], 0.0, 0.0);
    let CrosshairTarget::Entity(tail_hit) =
        crosshair_target_from_world(&world, Some(tail_pose)).unwrap()
    else {
        panic!("expected dragon tail part entity hit");
    };
    assert_eq!(tail_hit.entity_id, 104);
    assert_vec3_close(tail_hit.location, [0.0, 4.5, 11.5]);
    assert_vec3_close(tail_hit.relative_location, [0.0, 1.0, -1.0]);

    let wing_pose = player_pose_at([4.5, 5.0 - eye_height, 6.0], 0.0, 0.0);
    let CrosshairTarget::Entity(wing_hit) =
        crosshair_target_from_world(&world, Some(wing_pose)).unwrap()
    else {
        panic!("expected dragon wing part entity hit");
    };
    assert_eq!(wing_hit.entity_id, 107);
    assert_vec3_close(wing_hit.location, [4.5, 5.0, 7.0]);
    assert_vec3_close(wing_hit.relative_location, [0.0, 1.0, -2.0]);
}

#[test]
fn crosshair_target_uses_current_dragon_part_positions() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        130,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        [1.0, 64.0, -2.0],
    ));
    world.advance_entity_client_animations(1);
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 130,
        values: vec![ender_dragon_phase_data(
            ENDER_DRAGON_HOLDING_PATTERN_PHASE_ID,
        )],
    }));
    assert!(world.apply_entity_position_sync(EntityPositionSync {
        id: 130,
        position: ProtocolVec3d {
            x: 1.0,
            y: 74.0,
            z: -2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        y_rot: 90.0,
        x_rot: 0.0,
        on_ground: false,
    }));
    world.advance_entity_client_animations(1);

    let pose = player_pose_at(
        [
            7.5,
            64.0 - eye_height(LocalPlayerPoseState::default()),
            -5.0,
        ],
        0.0,
        0.0,
    );

    world.set_local_player_pose(pose);
    for partial_tick in [0.0, 0.5, 1.0] {
        let camera =
            crate::camera_pose::camera_pose_from_world_at_partial_tick(&world, partial_tick)
                .unwrap();
        let eye_y = crosshair_ray_from_camera_pose(camera).eye[1];
        let Some(CrosshairTarget::Entity(hit)) = crosshair_target_from_camera(&world, Some(camera))
        else {
            panic!("expected current dragon part at camera partial tick {partial_tick}");
        };
        assert_eq!(hit.entity_id, 132);
        assert_vec3_close(hit.location, [7.5, eye_y, -3.5]);
        assert_vec3_close(hit.relative_location, [1.0, eye_y - 64.0, -1.5]);
    }
}

#[test]
fn frame_crosshair_pick_follows_camera_and_world_changes() {
    let mut world = WorldStore::new();
    let mut frame = crate::runtime::NonEntityRenderExtractionScratch::default();
    let camera = Some(camera_pose_at([0.0; 3], 0.0, 0.0));
    assert!(frame.crosshair_pick(&world, 1, camera).target.is_none());
    world.apply_add_entity(protocol_add_entity(
        10,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [0.0, 1.0, 3.0],
    ));
    assert!(matches!(
        frame.crosshair_pick(&world, 2, camera).target,
        Some(CrosshairTarget::Entity(hit)) if hit.entity_id == 10
    ));
    let turned = Some(camera_pose_at([0.0; 3], 90.0, 0.0));
    assert!(frame.crosshair_pick(&world, 2, turned).target.is_none());
}

#[test]
fn crosshair_target_hits_direction_aware_item_frame_bounds() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity_with_data(
        13,
        VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
        [0.0, 1.0, 2.0],
        2,
    ));

    let target =
        crosshair_target_from_world(&world, Some(player_pose_at([0.5, 0.0, 0.0], 0.0, 0.0)));

    let CrosshairTarget::Entity(hit) = target.unwrap() else {
        panic!("expected item frame entity hit");
    };
    let eye_height = eye_height(LocalPlayerPoseState::default());
    assert_eq!(hit.entity_id, 13);
    assert_vec3_close(hit.location, [0.5, eye_height, 2.9375]);
    assert_vec3_close(hit.relative_location, [0.0, eye_height - 1.5, -0.03125]);
}

#[test]
fn crosshair_target_skips_entity_beyond_vanilla_default_entity_interaction_range() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        12,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [0.0, 1.0, 4.0],
    ));

    assert_eq!(
        DEFAULT_ENTITY_INTERACTION_RANGE, 3.0,
        "vanilla Player.DEFAULT_ENTITY_INTERACTION_RANGE"
    );
    assert_eq!(
        crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0))),
        None
    );
}

#[test]
fn debug_targeted_entity_uses_vanilla_eight_block_probe() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        12,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [0.0, 1.0, 7.0],
    ));
    let camera = Some(camera_pose_at([0.0, 0.0, 0.0], 0.0, 0.0));

    assert_eq!(crosshair_target_from_camera(&world, camera), None);
    assert_eq!(debug_targeted_entity_from_camera(&world, camera), Some(12));
}

#[test]
fn crosshair_target_uses_selected_item_attack_range_for_extended_entity_hit() {
    let mut world = WorldStore::new();
    set_selected_attack_range(&mut world, 0.0, 4.5, 0.0, 4.5, 0.0);
    world.apply_add_entity(protocol_add_entity(
        12,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [0.0, 1.0, 4.0],
    ));

    let target = crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0)));

    let CrosshairTarget::Entity(hit) = target.unwrap() else {
        panic!("expected entity hit");
    };
    assert_eq!(hit.entity_id, 12);
    assert_vec3_close(
        hit.location,
        [0.0, eye_height(LocalPlayerPoseState::default()), 3.51],
    );
}

#[test]
fn crosshair_target_attack_range_extends_max_reach_by_forward_movement() {
    let mut world = WorldStore::new();
    set_selected_attack_range(&mut world, 0.0, 3.5, 0.0, 3.5, 0.0);
    let pose = player_pose_with_delta(0.0, 0.0, 0.0, [0.0, 0.0, 0.25]);
    world.set_local_player_pose(pose);
    world.apply_add_entity(protocol_add_entity(
        12,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [0.0, 1.0, 4.0],
    ));

    let target = crosshair_target_from_world(&world, Some(pose));

    let CrosshairTarget::Entity(hit) = target.unwrap() else {
        panic!("expected entity hit from movement-extended attack range");
    };
    assert_eq!(hit.entity_id, 12);
    assert_vec3_close(
        hit.location,
        [0.0, eye_height(LocalPlayerPoseState::default()), 3.51],
    );
}

#[test]
fn crosshair_target_attack_range_uses_the_active_offhand_item() {
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 40,
        item: item_stack_with_attack_range(42, 0.0, 4.5, 0.0, 4.5, 0.0),
    });
    world.set_local_using_item_with_hand(true, InteractionHand::OffHand);
    world.apply_add_entity(protocol_add_entity(
        12,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [0.0, 1.0, 4.0],
    ));

    let target = crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0)));

    let CrosshairTarget::Entity(hit) = target.unwrap() else {
        panic!("expected active offhand attack-range entity hit");
    };
    assert_eq!(hit.entity_id, 12);
}

#[test]
fn crosshair_target_attack_range_does_not_extend_max_reach_by_backward_movement() {
    let mut world = WorldStore::new();
    set_selected_attack_range(&mut world, 0.0, 3.5, 0.0, 3.5, 0.0);
    let pose = player_pose_with_delta(0.0, 0.0, 0.0, [0.0, 0.0, -0.25]);
    world.set_local_player_pose(pose);
    world.apply_add_entity(protocol_add_entity(
        12,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [0.0, 1.0, 4.0],
    ));

    assert_eq!(crosshair_target_from_world(&world, Some(pose)), None);
}

#[test]
fn crosshair_target_attack_range_uses_hitbox_margin() {
    let mut world = WorldStore::new();
    set_selected_attack_range(&mut world, 0.0, 3.3, 0.0, 3.3, 0.25);
    world.apply_add_entity(protocol_add_entity(
        12,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [0.0, 1.0, 4.0],
    ));

    let target = crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0)));

    let CrosshairTarget::Entity(hit) = target.unwrap() else {
        panic!("expected entity hit from attack range margin");
    };
    assert_eq!(hit.entity_id, 12);
    assert_vec3_close(hit.location, [0.0, 1.5287837829456616, 3.51]);
}

#[test]
fn crosshair_target_attack_range_uses_creative_reach_when_instabuild() {
    let mut world = WorldStore::new();
    set_selected_attack_range(&mut world, 0.0, 1.0, 0.0, 5.0, 0.0);
    world.apply_player_abilities(PlayerAbilities {
        invulnerable: false,
        flying: false,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    world.apply_add_entity(protocol_add_entity(
        12,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [0.0, 1.0, 4.0],
    ));

    let target = crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0)));

    let CrosshairTarget::Entity(hit) = target.unwrap() else {
        panic!("expected creative reach entity hit");
    };
    assert_eq!(hit.entity_id, 12);
}

#[test]
fn crosshair_target_attack_range_keeps_block_when_block_clips_before_extended_entity() {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    insert_air_chunk(&mut world);
    set_selected_attack_range(&mut world, 0.0, 4.5, 0.0, 4.5, 0.0);
    assert!(
        world.apply_block_update(bbb_protocol::packets::BlockUpdate {
            pos: bbb_protocol::packets::BlockPos { x: 0, y: 1, z: 2 },
            block_state_id: VANILLA_GRASS_BLOCK_STATE_ID,
        })
    );
    world.apply_add_entity(protocol_add_entity(
        12,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [0.0, 1.0, 4.0],
    ));

    let target = crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0)));

    let CrosshairTarget::Block(hit) = target.unwrap() else {
        panic!("expected nearer block hit");
    };
    assert_eq!(hit.pos, BlockPos { x: 0, y: 1, z: 2 });
}

#[test]
fn crosshair_target_keeps_block_when_block_is_nearer_than_entity() {
    let eye = [0.0, eye_height(LocalPlayerPoseState::default()), 0.0];
    let block = CrosshairBlockHit {
        pos: BlockPos { x: 0, y: 1, z: 2 },
        face: ProtocolDirection::North,
        cursor: [0.0, 0.62, 0.0],
        inside: false,
    };
    let entity = RaycastEntityHit {
        hit: CrosshairEntityHit {
            entity_id: 10,
            location: ProtocolVec3d {
                x: 0.0,
                y: eye[1],
                z: 3.0,
            },
            relative_location: ProtocolVec3d {
                x: 0.0,
                y: 0.62,
                z: -0.5,
            },
        },
        distance_sq: 9.0,
    };

    assert_eq!(
        choose_crosshair_target(
            eye,
            Some(block),
            Some(entity),
            DEFAULT_ENTITY_INTERACTION_RANGE
        ),
        Some(CrosshairTarget::Block(block))
    );
}

#[test]
fn crosshair_target_prefers_entity_when_entity_is_nearer_than_block() {
    let eye = [0.0, eye_height(LocalPlayerPoseState::default()), 0.0];
    let block = CrosshairBlockHit {
        pos: BlockPos { x: 0, y: 1, z: 3 },
        face: ProtocolDirection::North,
        cursor: [0.0, 0.62, 0.0],
        inside: false,
    };
    let entity_hit = CrosshairEntityHit {
        entity_id: 10,
        location: ProtocolVec3d {
            x: 0.0,
            y: eye[1],
            z: 2.0,
        },
        relative_location: ProtocolVec3d {
            x: 0.0,
            y: 0.62,
            z: -0.5,
        },
    };
    let entity = RaycastEntityHit {
        hit: entity_hit,
        distance_sq: 4.0,
    };

    assert_eq!(
        choose_crosshair_target(
            eye,
            Some(block),
            Some(entity),
            DEFAULT_ENTITY_INTERACTION_RANGE
        ),
        Some(CrosshairTarget::Entity(entity_hit))
    );
}

#[test]
fn crosshair_target_misses_when_nearer_entity_exceeds_entity_interaction_range() {
    let eye = [0.0, eye_height(LocalPlayerPoseState::default()), 0.0];
    let block = CrosshairBlockHit {
        pos: BlockPos { x: 0, y: 1, z: 4 },
        face: ProtocolDirection::North,
        cursor: [0.0, 0.62, 0.0],
        inside: false,
    };
    let entity = RaycastEntityHit {
        hit: CrosshairEntityHit {
            entity_id: 10,
            location: ProtocolVec3d {
                x: 0.0,
                y: eye[1],
                z: 3.5,
            },
            relative_location: ProtocolVec3d {
                x: 0.0,
                y: 0.62,
                z: -0.5,
            },
        },
        distance_sq: 3.5 * 3.5,
    };

    assert_eq!(
        choose_crosshair_target(
            eye,
            Some(block),
            Some(entity),
            DEFAULT_ENTITY_INTERACTION_RANGE
        ),
        None
    );
}

#[test]
fn camera_crosshair_raycast_uses_camera_eye_height() {
    let ray = crosshair_ray_from_camera_pose(CameraPose {
        position: [0.0, 0.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 0.2751,
    });
    let hit = raycast_crosshair_block_hit_from_ray(ray, 5.0, |pos| {
        if pos == (BlockPos { x: 0, y: 0, z: 3 }) {
            Some(BlockOutlineTarget::full_block(
                bbb_client_world::TerrainMaterialClass::Opaque,
            ))
        } else {
            None
        }
    });

    let hit = hit.expect("camera ray should hit the low block");
    assert_eq!(hit.pos, BlockPos { x: 0, y: 0, z: 3 });
    assert!((hit.cursor[1] - 0.2751).abs() < 0.0001);
}

#[test]
fn item_use_pov_keeps_double_hit_location_beside_wire_cursor() {
    assert_eq!(
        block_hit_cursor(
            [5.125, 4.75, 6.5],
            [0.0, 0.0, 0.0],
            0.0,
            BlockPos { x: 4, y: 5, z: 6 },
        ),
        [1.125, -0.25, 0.5]
    );
    let block_hit = CrosshairBlockHit {
        pos: BlockPos { x: 4, y: 5, z: 6 },
        face: ProtocolDirection::Up,
        cursor: [0.123_456_789_012, 0.5, 0.987_654_321_098],
        inside: false,
    };

    let LocalItemUsePovResult::Block { hit, location } =
        local_item_use_pov_result_from_crosshair_hit(block_hit)
    else {
        panic!("block hit must remain a block POV result");
    };
    assert_eq!(
        location,
        ProtocolVec3d {
            x: 4.123_456_789_012,
            y: 5.5,
            z: 6.987_654_321_098,
        }
    );
    assert_eq!(hit.cursor_x, block_hit.cursor[0] as f32);
    assert_eq!(hit.cursor_y, block_hit.cursor[1] as f32);
    assert_eq!(hit.cursor_z, block_hit.cursor[2] as f32);
}

#[test]
fn player_pose_crosshair_ray_uses_local_player_eye_height() {
    let pose = LocalPlayerPoseState {
        sneaking: true,
        ..player_pose_at([0.0, 64.0, 0.0], 0.0, 0.0)
    };

    let ray = crosshair_ray_from_player_pose(pose, eye_height(pose));

    assert_eq!(ray.eye, [0.0, 64.0 + eye_height(pose), 0.0]);
}

#[test]
fn selection_outline_uses_block_bounds() {
    assert_eq!(
        selection_outline_for_block(BlockPos { x: -2, y: 63, z: 4 }),
        SelectionOutline::from_box([-2.0, 63.0, 4.0], [-1.0, 64.0, 5.0])
    );
}

#[test]
fn adventure_mode_outline_requires_matching_selected_item_predicate() {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    insert_air_chunk(&mut world);
    assert!(
        world.apply_block_update(bbb_protocol::packets::BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
            block_state_id: VANILLA_GRASS_BLOCK_STATE_ID,
        })
    );
    world.apply_game_event(GameEvent {
        event_id: 3,
        param: 2.0,
    });
    let pose = Some(camera_pose_at([0.0, 0.0, 0.0], 0.0, 0.0));

    assert_eq!(
        block_selection_from_hit(&world, crosshair_pick_from_camera(&world, pose).block_hit),
        None
    );

    set_selected_can_place_on(&mut world, "minecraft:grass_block");
    assert!(
        block_selection_from_hit(&world, crosshair_pick_from_camera(&world, pose).block_hit)
            .is_some()
    );
}

#[test]
fn spectator_mode_only_outlines_and_crosshairs_menu_provider_blocks() {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    insert_air_chunk(&mut world);
    assert!(
        world.apply_block_update(bbb_protocol::packets::BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
            block_state_id: VANILLA_GRASS_BLOCK_STATE_ID,
        })
    );
    world.apply_game_event(GameEvent {
        event_id: 3,
        param: 3.0,
    });
    let pose = Some(camera_pose_at([0.0, 0.0, 0.0], 0.0, 0.0));

    assert_eq!(
        block_selection_from_hit(&world, crosshair_pick_from_camera(&world, pose).block_hit),
        None
    );
    assert!(!can_render_crosshair_for_spectator(
        &world,
        crosshair_target_from_camera(&world, pose),
    ));

    assert!(
        world.apply_block_update(bbb_protocol::packets::BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
            block_state_id: bbb_client_world::vanilla_block_state_id(
                "minecraft:crafting_table",
                &std::collections::BTreeMap::new(),
            )
            .unwrap(),
        })
    );

    assert!(
        block_selection_from_hit(&world, crosshair_pick_from_camera(&world, pose).block_hit)
            .is_some()
    );
    assert!(can_render_crosshair_for_spectator(
        &world,
        crosshair_target_from_camera(&world, pose),
    ));
}

#[test]
fn a_non_player_camera_entity_suppresses_the_block_outline() {
    // Vanilla `shouldRenderBlockOutline`: `getCameraEntity() instanceof
    // Player` gates the selection box, so a server `SetCamera` onto a mob
    // hides it and one onto another player restores it.
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    insert_air_chunk(&mut world);
    assert!(
        world.apply_block_update(bbb_protocol::packets::BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
            block_state_id: VANILLA_GRASS_BLOCK_STATE_ID,
        })
    );
    let pose = Some(camera_pose_at([0.0, 0.0, 0.0], 0.0, 0.0));
    assert!(
        block_selection_from_hit(&world, crosshair_pick_from_camera(&world, pose).block_hit)
            .is_some()
    );

    world.apply_add_entity(protocol_add_entity(
        71,
        VANILLA_ENTITY_TYPE_PIG_ID,
        [0.0, 1.0, 2.0],
    ));
    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 71 }));
    assert_eq!(
        block_selection_from_hit(&world, crosshair_pick_from_camera(&world, pose).block_hit),
        None
    );

    world.apply_add_entity(protocol_add_entity(
        72,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
        [0.0, 1.0, 2.0],
    ));
    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 72 }));
    assert!(
        block_selection_from_hit(&world, crosshair_pick_from_camera(&world, pose).block_hit)
            .is_some()
    );
}

#[test]
fn spectator_crosshair_follows_the_hit_entity_menu_provider() {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    insert_air_chunk(&mut world);
    world.apply_game_event(GameEvent {
        event_id: 3,
        param: 3.0,
    });
    let pose = Some(camera_pose_at([0.0, 0.0, 0.0], 0.0, 0.0));

    world.apply_add_entity(protocol_add_entity(
        71,
        VANILLA_ENTITY_TYPE_PIG_ID,
        [0.0, 1.0, 2.0],
    ));
    assert!(!can_render_crosshair_for_spectator(
        &world,
        crosshair_target_from_camera(&world, pose),
    ));

    world.apply_remove_entities(bbb_protocol::packets::RemoveEntities {
        entity_ids: vec![71],
    });
    world.apply_add_entity(protocol_add_entity(
        72,
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
        [0.0, 1.0, 2.0],
    ));
    assert!(can_render_crosshair_for_spectator(
        &world,
        crosshair_target_from_camera(&world, pose),
    ));
}

fn player_pose(y_rot: f32, x_rot: f32, z: f64) -> LocalPlayerPoseState {
    player_pose_at([0.0, 0.0, z], y_rot, x_rot)
}

fn player_pose_with_delta(
    y_rot: f32,
    x_rot: f32,
    z: f64,
    delta_movement: [f64; 3],
) -> LocalPlayerPoseState {
    LocalPlayerPoseState {
        delta_movement: ProtocolVec3d {
            x: delta_movement[0],
            y: delta_movement[1],
            z: delta_movement[2],
        },
        ..player_pose(y_rot, x_rot, z)
    }
}

fn player_pose_at(position: [f64; 3], y_rot: f32, x_rot: f32) -> LocalPlayerPoseState {
    LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        },
        y_rot,
        x_rot,
        ..LocalPlayerPoseState::default()
    }
}

fn camera_pose_at(position: [f32; 3], y_rot: f32, x_rot: f32) -> CameraPose {
    CameraPose {
        position,
        y_rot,
        x_rot,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    }
}

fn protocol_add_entity(id: i32, entity_type_id: i32, position: [f64; 3]) -> AddEntity {
    protocol_add_entity_with_data(id, entity_type_id, position, 0)
}

fn set_selected_attack_range(
    world: &mut WorldStore,
    min_reach: f32,
    max_reach: f32,
    min_creative_reach: f32,
    max_creative_reach: f32,
    hitbox_margin: f32,
) {
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: item_stack_with_attack_range(
            42,
            min_reach,
            max_reach,
            min_creative_reach,
            max_creative_reach,
            hitbox_margin,
        ),
    });
}

fn set_selected_can_place_on(world: &mut WorldStore, block: &str) {
    let block_id = bbb_protocol::block_types::vanilla_block_type_for_resource_id(block)
        .unwrap()
        .id;
    let mut stack = ItemStackSummary {
        item_id: Some(1),
        count: 1,
        component_patch: Default::default(),
    };
    stack.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID];
    stack.component_patch.can_place_on_persistent = Some(AdventureModePredicateDataSummary {
        predicates: vec![BlockPredicateSummary {
            blocks: Some(HolderSetSummary {
                registry_ids: vec![block_id],
                tag: None,
            }),
            ..BlockPredicateSummary::default()
        }],
    });
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: stack,
    });
}

fn insert_air_chunk(world: &mut WorldStore) {
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos { x: 0, z: 0 },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: vec![std::sync::Arc::new(ChunkSection {
            non_empty_block_count: 0,
            fluid_count: 0,
            block_states: single_value_container(
                PaletteDomain::BlockStates,
                4096,
                bbb_client_world::vanilla_air_block_state_id(),
            ),
            biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
        })],
        block_entities: std::sync::Arc::default(),
        light: LightData::default(),
    });
}

fn single_value_container(
    domain: PaletteDomain,
    entry_count: usize,
    global_id: i32,
) -> PalettedContainerData {
    PalettedContainerData {
        domain,
        bits_per_entry: 0,
        palette_kind: PaletteKind::SingleValue,
        palette_global_ids: vec![global_id],
        packed_data: Vec::new(),
        entry_count,
    }
}

fn item_stack_with_attack_range(
    item_id: i32,
    min_reach: f32,
    max_reach: f32,
    min_creative_reach: f32,
    max_creative_reach: f32,
    hitbox_margin: f32,
) -> ItemStackSummary {
    let mut stack = ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: Default::default(),
    };
    stack.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID];
    stack.component_patch.attack_range = Some(AttackRangeSummary {
        min_reach,
        max_reach,
        min_creative_reach,
        max_creative_reach,
        hitbox_margin,
        mob_factor: 1.0,
    });
    stack
}

fn protocol_add_entity_with_data(
    id: i32,
    entity_type_id: i32,
    position: [f64; 3],
    data: i32,
) -> AddEntity {
    AddEntity {
        id,
        uuid: Uuid::from_u128(0x12345678123456781234567812345678 + id as u128),
        entity_type_id,
        position: ProtocolVec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data,
    }
}

fn assert_vec3_close(actual: ProtocolVec3d, expected: [f64; 3]) {
    assert!(
        (actual.x - expected[0]).abs() < 1.0e-6,
        "x: expected {}, got {}",
        expected[0],
        actual.x
    );
    assert!(
        (actual.y - expected[1]).abs() < 1.0e-6,
        "y: expected {}, got {}",
        expected[1],
        actual.y
    );
    assert!(
        (actual.z - expected[2]).abs() < 1.0e-6,
        "z: expected {}, got {}",
        expected[2],
        actual.z
    );
}

#[test]
fn block_selection_rejects_target_outside_world_border() {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    insert_air_chunk(&mut world);
    assert!(
        world.apply_block_update(bbb_protocol::packets::BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
            block_state_id: VANILLA_GRASS_BLOCK_STATE_ID,
        })
    );
    world.apply_initialize_border(InitializeBorder {
        new_center_x: 0.0,
        new_center_z: 0.0,
        old_size: 2.0,
        new_size: 2.0,
        lerp_time: 0,
        new_absolute_max_size: 29_999_984,
        warning_blocks: 5,
        warning_time: 15,
    });

    assert_eq!(
        block_selection_from_hit(
            &world,
            crosshair_pick_from_camera(&world, Some(camera_pose_at([0.0, 0.0, 0.0], 0.0, 0.0)))
                .block_hit
        ),
        None
    );
}

#[test]
fn contextual_scaffolding_selection_and_raycast_follow_main_hand_and_bottom() {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    insert_air_chunk(&mut world);
    let pos = BlockPos { x: 0, y: 0, z: 3 };
    let camera = |x, y| {
        Some(CameraPose {
            position: [x, y, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        })
    };
    let scaffolding = || ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_SCAFFOLDING_ID),
        count: 1,
        component_patch: Default::default(),
    };
    for bottom in ["false", "true"] {
        let state = bbb_client_world::vanilla_block_state_id(
            "minecraft:scaffolding",
            &std::collections::BTreeMap::from([
                ("bottom".to_string(), bottom.to_string()),
                ("distance".to_string(), "0".to_string()),
                ("waterlogged".to_string(), "false".to_string()),
            ]),
        )
        .unwrap();
        assert!(
            world.apply_block_update(bbb_protocol::packets::BlockUpdate {
                pos: ProtocolBlockPos { x: 0, y: 0, z: 3 },
                block_state_id: state,
            })
        );
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary::empty(),
        });
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 40,
            item: scaffolding(),
        });
        assert!(crosshair_pick_from_camera(&world, camera(0.5, 0.5))
            .block_hit
            .is_none());
        let post = block_selection_from_hit(
            &world,
            crosshair_pick_from_camera(&world, camera(0.05, 0.5)).block_hit,
        )
        .unwrap();
        assert_eq!(post.pos, pos);
        assert!(post.boxes.len() > 1);
        assert!(!post.boxes.iter().any(|shape| shape.min[0] <= 0.5
            && shape.max[0] >= 0.5
            && shape.min[1] <= 0.5
            && shape.max[1] >= 0.5
            && shape.min[2] <= 3.5
            && shape.max[2] >= 3.5));
        assert_eq!(
            crosshair_pick_from_camera(&world, camera(0.5, 0.0625))
                .block_hit
                .is_some(),
            bottom == "true"
        );

        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: scaffolding(),
        });
        let held = block_selection_from_hit(
            &world,
            crosshair_pick_from_camera(&world, camera(0.5, 0.5)).block_hit,
        )
        .unwrap();
        assert_eq!(held.pos, pos);
        assert_eq!(
            held.boxes,
            vec![SelectionBox {
                min: [0.0, 0.0, 3.0],
                max: [1.0, 1.0, 4.0]
            }]
        );
    }
}

#[test]
fn contextual_shulker_selection_and_raycast_follow_opening_face() {
    let cases = [
        ("north", [2.5, 2.5, 0.0], 0.0, 0.0, 2, -0.25),
        ("east", [5.0, 2.5, 2.5], 90.0, 0.0, 0, 1.25),
        ("south", [2.5, 2.5, 5.0], 180.0, 0.0, 2, 1.25),
        ("west", [0.0, 2.5, 2.5], -90.0, 0.0, 0, -0.25),
        ("up", [2.5, 5.0, 2.5], 0.0, 90.0, 1, 1.25),
        ("down", [2.5, 0.0, 2.5], 0.0, -90.0, 1, -0.25),
    ];
    for (facing, eye, yaw, pitch, axis, open_face) in cases {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        insert_air_chunk(&mut world);
        let pos = BlockPos { x: 2, y: 2, z: 2 };
        let state = bbb_client_world::vanilla_block_state_id(
            "minecraft:shulker_box",
            &std::collections::BTreeMap::from([("facing".to_string(), facing.to_string())]),
        )
        .unwrap();
        assert!(
            world.apply_block_update(bbb_protocol::packets::BlockUpdate {
                pos: ProtocolBlockPos { x: 2, y: 2, z: 2 },
                block_state_id: state,
            })
        );
        let camera = Some(CameraPose {
            position: eye,
            y_rot: yaw,
            x_rot: pitch,
            eye_height: 0.0,
        });
        let closed =
            block_selection_from_hit(&world, crosshair_pick_from_camera(&world, camera).block_hit)
                .unwrap();
        assert_eq!(
            closed.boxes,
            vec![SelectionBox {
                min: [2.0; 3],
                max: [3.0; 3]
            }]
        );
        world.apply_block_event(bbb_protocol::packets::BlockEvent {
            pos: ProtocolBlockPos { x: 2, y: 2, z: 2 },
            b0: 1,
            b1: 1,
            block_id: 0,
        });
        world.advance_shulker_box_lid_ticks(5);
        let hit = crosshair_pick_from_camera(&world, camera)
            .block_hit
            .unwrap();
        assert_eq!(hit.pos, pos);
        assert!((hit.cursor[axis] - open_face).abs() < 1.0e-6);
        let opening =
            block_selection_from_hit(&world, crosshair_pick_from_camera(&world, camera).block_hit)
                .unwrap();
        assert_eq!(opening.boxes.len(), 1);
        let bounds = opening.boxes[0];
        let outline_face = if open_face < 0.0 {
            bounds.min[axis]
        } else {
            bounds.max[axis]
        };
        assert!((f64::from(outline_face) - 2.0 - open_face).abs() < 1.0e-6);
    }
}
