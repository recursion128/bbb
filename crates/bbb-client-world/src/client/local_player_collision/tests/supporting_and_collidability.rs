use super::*;

#[test]
fn debug_camera_block_collision_boxes_use_inflated_player_bounds_and_shape_components() {
    let mut world = empty_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 0.5,
            y: 0.0,
            z: 0.5,
        },
        ..LocalPlayerPoseState::default()
    });
    let slab = vanilla_block_state_id(
        "minecraft:oak_slab",
        [("type", "bottom"), ("waterlogged", "false")],
    );
    let stone = vanilla_block_state_id("minecraft:stone", []);
    set_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, slab);
    set_block(&mut world, BlockPos { x: 1, y: 0, z: 0 }, stone);
    set_block(&mut world, BlockPos { x: 3, y: 0, z: 0 }, stone);

    let boxes = world.debug_camera_block_collision_boxes(1.0);
    assert_eq!(
        boxes,
        [
            WorldCollisionBox {
                min: [0.0, 0.0, 0.0],
                max: [1.0, 0.5, 1.0],
            },
            WorldCollisionBox {
                min: [1.0, 0.0, 0.0],
                max: [2.0, 1.0, 1.0],
            },
        ]
    );
    assert!(world
        .debug_camera_block_collision_boxes(f64::NAN)
        .is_empty());
}

#[test]
fn debug_camera_block_collision_boxes_require_a_camera_entity_pose() {
    let world = empty_world();
    assert!(world.debug_camera_block_collision_boxes(6.0).is_empty());
}

#[test]
fn debug_camera_block_collision_boxes_follow_vanilla_cursor_x_y_z_order() {
    let mut world = empty_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 0.5,
            y: 1.0,
            z: 0.5,
        },
        ..LocalPlayerPoseState::default()
    });
    let stone = vanilla_block_state_id("minecraft:stone", []);
    set_block(&mut world, BlockPos { x: 0, y: 1, z: 0 }, stone);
    set_block(&mut world, BlockPos { x: 0, y: 0, z: 1 }, stone);

    assert_eq!(
        world.debug_camera_block_collision_boxes(2.0),
        [
            WorldCollisionBox {
                min: [0.0, 1.0, 0.0],
                max: [1.0, 2.0, 1.0],
            },
            WorldCollisionBox {
                min: [0.0, 0.0, 1.0],
                max: [1.0, 1.0, 2.0],
            },
        ]
    );
}

#[test]
fn debug_supporting_block_membership_uses_camera_bounds_inflated_by_radius() {
    let mut world = empty_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 0.5,
            y: 1.0,
            z: 0.5,
        },
        ..LocalPlayerPoseState::default()
    });
    world.local_player_id = Some(99);
    add_collision_test_entity(
        &mut world,
        99,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [0.5, 1.0, 0.5],
    );
    add_collision_test_entity(
        &mut world,
        10,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        [17.2, 1.0, 0.5],
    );
    add_collision_test_entity(
        &mut world,
        11,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        [17.3, 1.0, 0.5],
    );

    assert_eq!(world.debug_supporting_block_entity_ids(16.0), [10]);
    assert!(world.debug_supporting_block_entity_ids(f64::NAN).is_empty());
    assert!(world.debug_supporting_block_entity_ids(-1.0).is_empty());
}

#[test]
fn debug_local_supporting_block_uses_vanilla_distance_and_block_pos_tie_break() {
    let mut world = empty_world();
    let stone = vanilla_block_state_id("minecraft:stone", []);
    set_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, stone);
    set_block(&mut world, BlockPos { x: 1, y: 0, z: 0 }, stone);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 1.0,
            y: 1.0,
            z: 0.5,
        },
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let debug = world
        .debug_local_player_supporting_blocks()
        .expect("grounded player should have a support block");
    assert_eq!(debug.supporting.pos, BlockPos { x: 1, y: 0, z: 0 });
    assert_eq!(
        debug.supporting.collision_boxes,
        [WorldCollisionBox {
            min: [1.0, 0.0, 0.0],
            max: [2.0, 1.0, 1.0],
        }]
    );
    assert_eq!(debug.legacy_effect, None);

    let mut airborne = world.local_player_pose().unwrap();
    airborne.on_ground = false;
    world.set_local_player_pose(airborne);
    assert_eq!(world.debug_local_player_supporting_blocks(), None);
}

#[test]
fn debug_local_supporting_block_projects_legacy_effect_and_fence_exception() {
    let mut piston_world = empty_world();
    let piston_head = vanilla_block_state_id(
        "minecraft:piston_head",
        [("facing", "down"), ("short", "false"), ("type", "normal")],
    );
    set_block(
        &mut piston_world,
        BlockPos { x: 0, y: 0, z: 0 },
        piston_head,
    );
    piston_world.set_local_player_pose(LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 0.5,
            y: 1.25,
            z: 0.5,
        },
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let piston_debug = piston_world
        .debug_local_player_supporting_blocks()
        .expect("the upward-extending rod should support the player");
    assert_eq!(piston_debug.supporting.pos, BlockPos { x: 0, y: 0, z: 0 });
    assert!(!piston_debug.supporting.collision_boxes.is_empty());
    assert_eq!(
        piston_debug
            .legacy_effect
            .expect("legacy on-pos should use the floored entity position")
            .pos,
        BlockPos { x: 0, y: 1, z: 0 }
    );

    let mut fence_world = empty_world();
    let fence = vanilla_block_state_id(
        "minecraft:oak_fence",
        [
            ("east", "false"),
            ("north", "false"),
            ("south", "false"),
            ("waterlogged", "false"),
            ("west", "false"),
        ],
    );
    set_block(&mut fence_world, BlockPos { x: 0, y: 0, z: 0 }, fence);
    fence_world.set_local_player_pose(LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 0.5,
            y: 1.5,
            z: 0.5,
        },
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    let fence_debug = fence_world
        .debug_local_player_supporting_blocks()
        .expect("fence should support the player at height 1.5");
    assert_eq!(fence_debug.supporting.pos, BlockPos { x: 0, y: 0, z: 0 });
    assert_eq!(fence_debug.legacy_effect, None);
}

#[test]
fn debug_supporting_highlight_uses_vanilla_empty_collision_context() {
    let mut world = empty_world();
    let powder_snow = vanilla_block_state_id("minecraft:powder_snow", []);
    let scaffolding = vanilla_block_state_id(
        "minecraft:scaffolding",
        [
            ("bottom", "false"),
            ("distance", "0"),
            ("waterlogged", "false"),
        ],
    );
    let powder_pos = BlockPos { x: 0, y: 0, z: 0 };
    let scaffolding_pos = BlockPos { x: 1, y: 0, z: 0 };
    set_block(&mut world, powder_pos, powder_snow);
    set_block(&mut world, scaffolding_pos, scaffolding);

    assert!(block_collision_shape_empty_context(
        &world.block_view(powder_pos).unwrap(),
        powder_pos,
    )
    .is_none());
    assert!(block_collision_shape_empty_context(
        &world.block_view(scaffolding_pos).unwrap(),
        scaffolding_pos,
    )
    .is_some());
}

#[test]
fn remote_guardian_stand_probe_requires_a_loaded_full_up_face() {
    let mut world = empty_world();
    let stone_pos = BlockPos { x: 0, y: 0, z: 0 };
    let top_slab_pos = BlockPos { x: 1, y: 0, z: 0 };
    let bottom_slab_pos = BlockPos { x: 2, y: 0, z: 0 };
    set_block(
        &mut world,
        stone_pos,
        vanilla_block_state_id("minecraft:stone", []),
    );
    set_block(
        &mut world,
        top_slab_pos,
        vanilla_block_state_id(
            "minecraft:stone_slab",
            [("type", "top"), ("waterlogged", "false")],
        ),
    );
    set_block(
        &mut world,
        bottom_slab_pos,
        vanilla_block_state_id(
            "minecraft:stone_slab",
            [("type", "bottom"), ("waterlogged", "false")],
        ),
    );

    assert!(block_collision_shape_has_full_up_face(
        &world.block_view(stone_pos).unwrap(),
        stone_pos
    ));
    assert!(block_collision_shape_has_full_up_face(
        &world.block_view(top_slab_pos).unwrap(),
        top_slab_pos
    ));
    assert!(!block_collision_shape_has_full_up_face(
        &world.block_view(bottom_slab_pos).unwrap(),
        bottom_slab_pos
    ));
    assert!(world.block_view(BlockPos { x: 16, y: 0, z: 0 }).is_none());
}

#[test]
fn remote_supporting_block_state_matches_packet_set_on_ground_lifecycle() {
    let mut world = empty_world();
    let entity_id = 41;
    add_collision_test_entity(
        &mut world,
        entity_id,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        [0.5, 1.0, 0.5],
    );
    let stone = vanilla_block_state_id("minecraft:stone", []);
    let air = vanilla_block_state_id("minecraft:air", []);
    let support_pos = BlockPos { x: 0, y: 0, z: 0 };
    set_block(&mut world, support_pos, stone);

    assert!(
        world.apply_entity_position_sync(bbb_protocol::packets::EntityPositionSync {
            id: entity_id,
            position: bbb_protocol::packets::Vec3d {
                x: 0.5,
                y: 1.0,
                z: 0.5,
            },
            delta_movement: bbb_protocol::packets::Vec3d::default(),
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        })
    );
    let supported = world.probe_entity(entity_id).unwrap();
    assert_eq!(supported.main_supporting_block_pos, Some(support_pos));
    assert!(!supported.on_ground_no_blocks);
    assert_eq!(
        world.entity_main_supporting_block_pos(entity_id),
        Some(support_pos)
    );
    assert_eq!(
        world.debug_entity_supporting_blocks(entity_id),
        Some(DebugEntitySupportingBlocks {
            entity_id,
            supporting: DebugSupportingBlockHighlight {
                pos: support_pos,
                collision_boxes: vec![WorldCollisionBox {
                    min: [0.0, 0.0, 0.0],
                    max: [1.0, 1.0, 1.0],
                }],
            },
            legacy_effect: None,
        })
    );

    let cloned = world.clone();
    assert_eq!(
        cloned
            .probe_entity(entity_id)
            .unwrap()
            .main_supporting_block_pos,
        Some(support_pos)
    );
    let serialized = serde_json::to_vec(&world).unwrap();
    let decoded: WorldStore = serde_json::from_slice(&serialized).unwrap();
    assert_eq!(
        decoded
            .probe_entity(entity_id)
            .unwrap()
            .main_supporting_block_pos,
        Some(support_pos)
    );

    set_block(&mut world, support_pos, air);
    assert!(world.apply_entity_move(bbb_protocol::packets::EntityMove {
        id: entity_id,
        has_position: false,
        delta_x: 0,
        delta_y: 0,
        delta_z: 0,
        y_rot: None,
        x_rot: None,
        on_ground: true,
    }));
    let first_missing = world.probe_entity(entity_id).unwrap();
    assert_eq!(first_missing.main_supporting_block_pos, Some(support_pos));
    assert!(first_missing.on_ground_no_blocks);

    assert!(
        world.apply_teleport_entity(bbb_protocol::packets::TeleportEntity {
            id: entity_id,
            position: bbb_protocol::packets::Vec3d {
                x: 0.5,
                y: 1.0,
                z: 0.5,
            },
            delta_movement: bbb_protocol::packets::Vec3d::default(),
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
            on_ground: true,
        })
    );
    let second_missing = world.probe_entity(entity_id).unwrap();
    assert_eq!(second_missing.main_supporting_block_pos, None);
    assert!(second_missing.on_ground_no_blocks);
    assert_eq!(world.debug_entity_supporting_blocks(entity_id), None);

    assert!(world.apply_entity_move(bbb_protocol::packets::EntityMove {
        id: entity_id,
        has_position: false,
        delta_x: 0,
        delta_y: 0,
        delta_z: 0,
        y_rot: None,
        x_rot: None,
        on_ground: false,
    }));
    let airborne = world.probe_entity(entity_id).unwrap();
    assert_eq!(airborne.main_supporting_block_pos, None);
    assert!(!airborne.on_ground_no_blocks);
}

#[test]
fn remote_packet_fall_delta_does_not_create_powder_snow_falling_shape() {
    let mut world = empty_world();
    let entity_id = 56;
    let powder_snow = vanilla_block_state_id("minecraft:powder_snow", []);
    set_block(&mut world, BlockPos { x: 10, y: 0, z: 0 }, powder_snow);
    add_collision_test_entity(
        &mut world,
        entity_id,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        [10.5, 5.0, 0.5],
    );

    // Vanilla remote packet handlers call moveOrInterpolateTo/setOnGround;
    // neither packet displacement nor synced velocity runs checkFallDamage
    // on a server-authoritative client entity, so fallDistance stays zero.
    assert!(world.apply_entity_move(bbb_protocol::packets::EntityMove {
        id: entity_id,
        has_position: true,
        delta_x: 0,
        delta_y: -16_384,
        delta_z: 0,
        y_rot: None,
        x_rot: None,
        on_ground: true,
    }));
    world.advance_entity_client_animations(3);
    assert_eq!(
        world.probe_entity(entity_id).unwrap().position.y,
        1.0,
        "the packet moves the entity downward by four blocks"
    );
    assert_eq!(world.entity_main_supporting_block_pos(entity_id), None);

    let velocity_entity_id = 57;
    set_block(&mut world, BlockPos { x: 12, y: 0, z: 0 }, powder_snow);
    add_collision_test_entity(
        &mut world,
        velocity_entity_id,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        [12.5, 1.0, 0.5],
    );
    assert!(
        world.apply_entity_position_sync(bbb_protocol::packets::EntityPositionSync {
            id: velocity_entity_id,
            position: bbb_protocol::packets::Vec3d {
                x: 12.5,
                y: 1.0,
                z: 0.5,
            },
            delta_movement: bbb_protocol::packets::Vec3d {
                x: 0.0,
                y: -4.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        })
    );
    assert_eq!(
        world
            .probe_entity(velocity_entity_id)
            .unwrap()
            .delta_movement
            .y,
        0.0,
        "ClientPacketListener position sync does not apply packet movement"
    );
    assert_eq!(
        world.entity_main_supporting_block_pos(velocity_entity_id),
        None
    );
}

#[test]
fn debug_camera_collision_boxes_prepend_collidable_entities_and_exclude_same_vehicle() {
    let mut world = empty_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 0.5,
            y: 0.0,
            z: 0.5,
        },
        ..LocalPlayerPoseState::default()
    });
    world.local_player_id = Some(99);
    add_collision_test_entity(
        &mut world,
        99,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [0.5, 0.0, 0.5],
    );
    add_collision_test_entity(
        &mut world,
        10,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        [2.5, 0.0, 0.5],
    );
    add_collision_test_entity(
        &mut world,
        11,
        VANILLA_ENTITY_TYPE_SHULKER_ID,
        [4.5, 0.0, 0.5],
    );
    add_collision_test_entity(
        &mut world,
        12,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        [5.5, 0.0, 0.5],
    );

    let expected = world
        .entity_pick_targets()
        .into_iter()
        .filter(|target| matches!(target.entity_id, 10 | 11))
        .map(world_collision_box_from_pick_target)
        .collect::<Vec<_>>();
    assert_eq!(world.debug_camera_collision_boxes(6.0), expected);

    assert!(
        world.apply_set_passengers(bbb_protocol::packets::SetPassengers {
            vehicle_id: 10,
            passenger_ids: vec![99],
        })
    );
    let boxes = world.debug_camera_collision_boxes(6.0);
    assert_eq!(boxes, expected[1..]);
}

#[test]
fn living_pushability_rejects_spectator_player_instance_state() {
    let mut world = empty_world();
    let player_id = 42;
    let player_uuid = uuid::Uuid::from_u128(player_id as u128 + 1);
    add_collision_test_entity(
        &mut world,
        player_id,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [0.5, 0.0, 0.5],
    );
    assert!(world.entity_is_pushable(player_id, VANILLA_ENTITY_TYPE_PLAYER_ID, [0.5, 0.0, 0.5]));
    world.apply_player_info_update(bbb_protocol::packets::PlayerInfoUpdate {
        actions: vec![
            bbb_protocol::packets::PlayerInfoAction::AddPlayer,
            bbb_protocol::packets::PlayerInfoAction::UpdateGameMode,
        ],
        entries: vec![bbb_protocol::packets::PlayerInfoEntry {
            profile_id: player_uuid,
            profile: Some(bbb_protocol::packets::GameProfile {
                uuid: player_uuid,
                name: "Spectator".to_string(),
                properties: Vec::new(),
            }),
            listed: true,
            latency: 0,
            game_mode: bbb_protocol::packets::GameType::Spectator,
            display_name: None,
            show_hat: true,
            list_order: 0,
            chat_session: None,
        }],
    });
    assert!(!world.entity_is_pushable(player_id, VANILLA_ENTITY_TYPE_PLAYER_ID, [0.5, 0.0, 0.5]));
}

#[test]
fn spectated_minecart_collision_expands_to_pushable_entities() {
    let mut world = empty_world();
    add_collision_test_entity(
        &mut world,
        30,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID,
        [0.5, 0.0, 0.5],
    );
    add_collision_test_entity(
        &mut world,
        31,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        [2.5, 0.0, 0.5],
    );
    add_collision_test_entity(
        &mut world,
        32,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        [3.5, 0.0, 0.5],
    );
    add_collision_test_entity(&mut world, 33, VANILLA_ENTITY_TYPE_BAT_ID, [4.5, 0.0, 0.5]);
    add_collision_test_entity(
        &mut world,
        34,
        VANILLA_ENTITY_TYPE_PARROT_ID,
        [5.5, 0.0, 0.5],
    );
    add_collision_test_entity(
        &mut world,
        35,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HORSE_ID,
        [6.5, 0.0, 0.5],
    );
    world.local_player.camera = crate::CameraState {
        entity_id: Some(30),
        follows_player: false,
        entity_known: true,
    };

    let expected_ids = [31, 34, 35];
    let expected = world
        .entity_pick_targets()
        .into_iter()
        .filter(|target| expected_ids.contains(&target.entity_id))
        .map(world_collision_box_from_pick_target)
        .collect::<Vec<_>>();
    assert_eq!(world.debug_camera_collision_boxes(6.0), expected);

    assert!(
        world.apply_set_passengers(bbb_protocol::packets::SetPassengers {
            vehicle_id: 35,
            passenger_ids: vec![31],
        })
    );
    let horse_box = world
        .entity_pick_targets()
        .into_iter()
        .find(|target| target.entity_id == 35)
        .map(world_collision_box_from_pick_target)
        .expect("horse bounds");
    assert!(!world.debug_camera_collision_boxes(6.0).contains(&horse_box));
}

#[test]
fn living_pushability_rejects_entities_on_climbable_blocks() {
    let mut world = empty_world();
    add_collision_test_entity(
        &mut world,
        40,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        [0.5, 0.0, 0.5],
    );
    assert!(world.debug_entity_is_pushable(
        40,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        [0.5, 0.0, 0.5],
    ));
    let ladder = vanilla_block_state_id(
        "minecraft:ladder",
        [("facing", "north"), ("waterlogged", "false")],
    );
    set_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, ladder);
    assert!(!world.debug_entity_is_pushable(
        40,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        [0.5, 0.0, 0.5],
    ));
}
