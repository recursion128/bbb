use super::*;

#[test]
fn local_player_movement_collides_with_collidable_entity_bounds() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    world.apply_add_entity(ProtocolAddEntity {
        id: 456,
        uuid: Uuid::from_u128(0x45645678456456784564567845645678),
        entity_type_id: VANILLA_ENTITY_TYPE_SHULKER_ID,
        position: vec3(0.5, 1.0, 2.0),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        delta_movement: vec3(0.0, 0.0, 2.0),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.x, 0.5, 0.000001);
    assert_f64_near(pose.position.z, 1.7, 0.000001);
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_movement_collides_with_world_border() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    world.apply_initialize_border(ProtocolInitializeBorder {
        new_center_x: 0.0,
        new_center_z: 0.0,
        old_size: 4.0,
        new_size: 4.0,
        lerp_time: 0,
        new_absolute_max_size: 29_999_984,
        warning_blocks: 5,
        warning_time: 15,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(1.5, 1.0, 0.5),
        delta_movement: vec3(0.5, 0.0, 0.0),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.x, 1.7, 0.000001);
    assert_f64_near(pose.position.z, 0.5, 0.000001);
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_pose_fit_includes_collidable_entities() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    world.apply_add_entity(ProtocolAddEntity {
        id: 456,
        uuid: Uuid::from_u128(0x45645678456456784564567845645679),
        entity_type_id: VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        position: vec3(0.5, 2.5, 0.5),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        sneaking: true,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    let entity_collision_targets = world.entities.client_movement_entity_collision_targets();
    let standing_pose = LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    };
    assert!(!world.local_player_collision_bounds_are_free(
        standing_pose,
        world.local_player_bounds(standing_pose),
        false,
        &entity_collision_targets,
    ));

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert!(pose.sneaking);
    assert!(!pose.swimming);
    assert_f64_near(world.local_player_dimensions(pose).height, 1.5, 0.000001);
}

#[test]
fn local_player_sneak_edge_probe_accepts_entity_support() {
    let mut world = single_floor_block_world();
    attach_local_player_entity(&mut world, 123);
    world.apply_add_entity(ProtocolAddEntity {
        id: 456,
        uuid: Uuid::from_u128(0x45645678456456784564567845645680),
        entity_type_id: VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        position: vec3(0.5, 0.4, 1.5),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    let pose = LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    };
    let entity_collision_targets = world.entities.client_movement_entity_collision_targets();

    assert!(!local_player_can_fall_at_least(
        &world,
        pose,
        0.0,
        1.0,
        LOCAL_PLAYER_STEP_HEIGHT,
        &entity_collision_targets,
    ));
    assert_eq!(
        back_off_from_edge(&world, pose, 0.0, 1.0, &entity_collision_targets),
        (0.0, 1.0)
    );
}

#[test]
fn local_player_does_not_walk_through_leaves() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, OAK_LEAVES_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 2, 1, OAK_LEAVES_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert!(
        pose.position.z <= 0.7005,
        "position was {:?}",
        pose.position
    );
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_does_not_walk_through_mangrove_roots() {
    let cases = [
        ("dry mangrove roots", MANGROVE_ROOTS_DRY_BLOCK_STATE_ID),
        (
            "waterlogged mangrove roots",
            MANGROVE_ROOTS_WATERLOGGED_BLOCK_STATE_ID,
        ),
        (
            "muddy mangrove roots",
            MUDDY_MANGROVE_ROOTS_X_BLOCK_STATE_ID,
        ),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z <= 0.7005,
            "{name} position was {:?}",
            pose.position
        );
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_does_not_walk_through_bamboo_stalk_collision() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        1,
        BAMBOO_AGE_0_NO_LEAVES_STAGE_0_BLOCK_STATE_ID,
    );
    set_test_block(
        &mut world,
        0,
        2,
        1,
        BAMBOO_AGE_0_NO_LEAVES_STAGE_0_BLOCK_STATE_ID,
    );
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert!(pose.position.z < 1.7, "position was {:?}", pose.position);
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_does_not_walk_through_chorus_collision() {
    let cases = [
        (
            "chorus plant center cube",
            CHORUS_PLANT_NONE_BLOCK_STATE_ID,
            0.888,
        ),
        (
            "chorus plant north arm",
            CHORUS_PLANT_NORTH_BLOCK_STATE_ID,
            0.7005,
        ),
        ("chorus flower", CHORUS_FLOWER_AGE_0_BLOCK_STATE_ID, 0.7005),
    ];

    for (name, block_state_id, max_z) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z <= max_z,
            "{name} position was {:?}",
            pose.position
        );
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_walks_through_nether_portal_plane() {
    let cases = [
        ("x-axis nether portal", NETHER_PORTAL_X_BLOCK_STATE_ID),
        ("z-axis nether portal", NETHER_PORTAL_Z_BLOCK_STATE_ID),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 0.2);

        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(
            pose.position.z > 1.0,
            "{name} position was {:?}",
            pose.position
        );
        assert!(!pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_does_not_walk_through_copper_grates() {
    let cases = [
        ("copper grate", COPPER_GRATE_BLOCK_STATE_ID),
        ("waxed copper grate", WAXED_COPPER_GRATE_BLOCK_STATE_ID),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z <= 0.7005,
            "{name} position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_does_not_walk_through_chain_or_rod_shapes() {
    let cases = [
        (
            "vertical chain",
            IRON_CHAIN_Y_AXIS_BLOCK_STATE_ID,
            0.9,
            1.107,
        ),
        (
            "vertical lightning rod",
            LIGHTNING_ROD_UP_UNPOWERED_BLOCK_STATE_ID,
            0.9,
            1.076,
        ),
        ("north end rod", END_ROD_NORTH_BLOCK_STATE_ID, 0.0, 0.7005),
    ];

    for (name, block_state_id, min_z, max_z) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z > min_z && pose.position.z <= max_z,
            "{name} position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_does_not_walk_through_door_and_trapdoor_shapes() {
    let cases = [
        ("closed door", OAK_CLOSED_NORTH_DOOR_BLOCK_STATE_ID, 1.5135),
        (
            "closed top trapdoor",
            OAK_TOP_CLOSED_NORTH_TRAPDOOR_BLOCK_STATE_ID,
            0.7005,
        ),
    ];

    for (name, block_state_id, max_z) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z <= max_z,
            "{name} position was {:?}",
            pose.position
        );
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_does_not_walk_through_fence_pane_or_wall_connections() {
    let cases = [
        ("north fence", OAK_NORTH_FENCE_BLOCK_STATE_ID),
        ("north pane", GLASS_NORTH_PANE_BLOCK_STATE_ID),
        (
            "north/east wall",
            COBBLESTONE_NORTH_EAST_WALL_BLOCK_STATE_ID,
        ),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z <= 0.7005,
            "{name} position was {:?}",
            pose.position
        );
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_does_not_walk_through_common_object_shapes() {
    let cases = [
        ("single chest", CHEST_SINGLE_NORTH_BLOCK_STATE_ID, 0.763),
        ("double chest left", CHEST_LEFT_NORTH_BLOCK_STATE_ID, 0.763),
        (
            "trapped double chest right",
            TRAPPED_CHEST_RIGHT_NORTH_BLOCK_STATE_ID,
            0.763,
        ),
        ("ender chest", ENDER_CHEST_NORTH_BLOCK_STATE_ID, 0.763),
        (
            "water cauldron",
            WATER_CAULDRON_LEVEL_3_BLOCK_STATE_ID,
            0.7005,
        ),
        ("north hopper", HOPPER_NORTH_ENABLED_BLOCK_STATE_ID, 0.7005),
        ("enchanting table", ENCHANTING_TABLE_BLOCK_STATE_ID, 0.7005),
        ("anvil", ANVIL_NORTH_BLOCK_STATE_ID, 0.7005),
        (
            "north wall skull",
            SKELETON_WALL_SKULL_NORTH_BLOCK_STATE_ID,
            1.2005,
        ),
        (
            "north wall grindstone",
            GRINDSTONE_WALL_NORTH_BLOCK_STATE_ID,
            0.7005,
        ),
        (
            "pointed dripstone base",
            POINTED_DRIPSTONE_BASE_UP_BLOCK_STATE_ID,
            0.909,
        ),
    ];

    for (name, block_state_id, max_z) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z <= max_z,
            "{name} position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_does_not_walk_through_grindstone_floor_and_ceiling_faces() {
    let cases = [
        (
            "north floor grindstone",
            GRINDSTONE_FLOOR_NORTH_BLOCK_STATE_ID,
        ),
        (
            "north ceiling grindstone",
            GRINDSTONE_CEILING_NORTH_BLOCK_STATE_ID,
        ),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z <= 0.8255,
            "{name} position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_does_not_walk_through_bell_attachments() {
    let cases = [
        ("floor bell", BELL_FLOOR_NORTH_BLOCK_STATE_ID, 0.9505),
        (
            "single-wall bell",
            BELL_SINGLE_WALL_NORTH_BLOCK_STATE_ID,
            0.7005,
        ),
        (
            "double-wall bell",
            BELL_DOUBLE_WALL_NORTH_BLOCK_STATE_ID,
            0.7005,
        ),
    ];

    for (name, block_state_id, max_z) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z <= max_z,
            "{name} position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_sneak_uses_crouching_collision_height() {
    let mut standing_world = flat_collision_world();
    set_test_block(&mut standing_world, 0, 2, 1, OAK_TOP_SLAB_BLOCK_STATE_ID);
    standing_world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        y_rot: 0.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let standing = standing_world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            1.0,
        )
        .unwrap();

    assert!(standing.horizontal_collision);
    assert!(
        standing.position.z < 1.0,
        "position was {:?}",
        standing.position
    );
    assert!(!standing.sneaking);
    assert!(!standing.swimming);

    let mut crouching_world = flat_collision_world();
    set_test_block(&mut crouching_world, 0, 2, 1, OAK_TOP_SLAB_BLOCK_STATE_ID);
    crouching_world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        y_rot: 0.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let crouching = crouching_world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            1.0,
        )
        .unwrap();

    assert!(
        crouching.position.z > 1.0,
        "position was {:?}",
        crouching.position
    );
    assert!(crouching.sneaking);
    assert!(!crouching.swimming);
    assert!(!crouching.horizontal_collision);
}

#[test]
fn local_player_low_ceiling_forces_crouching_pose() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 2, 0, OAK_TOP_SLAB_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        y_rot: 0.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_step = first_ground_step(
        TEST_DEFAULT_MOVEMENT_SPEED,
        LOCAL_INPUT_SNEAKING_SPEED_MULTIPLIER,
    );
    assert!(pose.sneaking);
    assert!(!pose.swimming);
    assert!(!pose.horizontal_collision);
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        carried_after_ground_step(expected_step),
        0.000001,
    );
}

#[test]
fn local_player_sneak_moves_over_supported_ground() {
    let mut world = flat_collision_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            0.2,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    // Four ticks of the accumulate -> move -> friction chain.
    let mut carried = 0.0;
    let mut expected_z = 0.5;
    for _ in 0..4 {
        let step = carried
            + first_ground_step(
                TEST_DEFAULT_MOVEMENT_SPEED,
                LOCAL_INPUT_SNEAKING_SPEED_MULTIPLIER,
            );
        expected_z += step;
        carried = carried_after_ground_step(step);
    }
    assert_f64_near(pose.position.z, expected_z, 0.000001);
    assert!(pose.on_ground);
}

#[test]
fn local_player_movement_speed_attribute_scales_horizontal_movement() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
            base: 0.2,
            modifiers: Vec::new(),
        }],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_step = first_ground_step(TEST_DEFAULT_MOVEMENT_SPEED, 2.0);
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        carried_after_ground_step(expected_step),
        0.000001,
    );
    assert!(pose.on_ground);
}

#[test]
fn local_player_no_collision_hazards_apply_stuck_speed_multiplier() {
    let cases = [
        ("cobweb", COBWEB_BLOCK_STATE_ID, 0.25),
        (
            "sweet berry bush",
            SWEET_BERRY_BUSH_AGE_3_BLOCK_STATE_ID,
            0.8,
        ),
    ];

    for (name, block_state_id, expected_multiplier) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, block_state_id);
        attach_local_player_entity(&mut world, 123);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        // Vanilla sets the multiplier in `applyEffectsFromBlocks` after the
        // move, so the first step inside the hazard runs un-multiplied.
        let full_step = first_ground_step(TEST_DEFAULT_MOVEMENT_SPEED, 1.0);
        let first = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();
        assert_f64_near(first.position.z, 0.5 + full_step, 0.000001);

        // The next `Entity.move` consumes the recorded multiplier and zeroes
        // the post-move delta.
        let second = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();
        let expected_step =
            (full_step + carried_after_ground_step(full_step)) * expected_multiplier;
        assert_f64_near(
            second.position.z,
            first.position.z + expected_step,
            0.000001,
        );
        assert_f64_near(second.delta_movement.z, 0.0, 0.000001);
        assert!(second.on_ground, "{name}");
    }
}

#[test]
fn local_player_weaving_effect_relaxes_cobweb_stuck_speed_multiplier() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, COBWEB_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_WEAVING_ID, 0)));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let first = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();
    let second = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    // Weaving relaxes the cobweb stuck multiplier to 0.5; it scales the
    // carried-momentum request of the second tick.
    let full_step = first_ground_step(TEST_DEFAULT_MOVEMENT_SPEED, 1.0);
    let expected_step = (full_step + carried_after_ground_step(full_step)) * 0.5;
    assert_f64_near(
        second.position.z,
        first.position.z + expected_step,
        0.000001,
    );
    assert_f64_near(second.delta_movement.z, 0.0, 0.000001);
    assert!(second.on_ground);
}

#[test]
fn local_player_blindness_prevents_sprint_speed() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_BLINDNESS_ID, 0,)));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sprinting: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_step = first_ground_step(TEST_DEFAULT_MOVEMENT_SPEED, 1.0);
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        carried_after_ground_step(expected_step),
        0.000001,
    );
}

#[test]
fn local_player_sprint_latch_survives_key_release_but_stops_without_forward_input() {
    let mut world = flat_collision_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    let input = LocalPlayerInputState {
        focused: true,
        forward: true,
        ..LocalPlayerInputState::default()
    };

    assert!(world.reconcile_local_player_sprinting(input, true));
    assert!(world.local_player_is_sprinting());
    assert!(!world.reconcile_local_player_sprinting(input, false));
    assert!(world.local_player_is_sprinting());
    assert!(world.reconcile_local_player_sprinting(
        LocalPlayerInputState {
            forward: false,
            ..input
        },
        false,
    ));
    assert!(!world.local_player_is_sprinting());
}

#[test]
fn local_player_speed_and_slowness_effects_scale_horizontal_movement() {
    let mut speed_world = flat_collision_world();
    attach_local_player_entity(&mut speed_world, 123);
    assert!(speed_world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_SPEED_ID, 1,)));
    let speed_pose =
        advance_forward_from_standard_start(&mut speed_world, LOCAL_PHYSICS_TICK_SECONDS);
    let speed_step = first_ground_step(TEST_DEFAULT_MOVEMENT_SPEED, 1.4);
    assert_f64_near(speed_pose.position.z, 0.5 + speed_step, 0.000001);
    assert_f64_near(
        speed_pose.delta_movement.z,
        carried_after_ground_step(speed_step),
        0.000001,
    );

    let mut slowness_world = flat_collision_world();
    attach_local_player_entity(&mut slowness_world, 124);
    assert!(slowness_world.apply_update_mob_effect(mob_effect(
        124,
        VANILLA_MOB_EFFECT_SLOWNESS_ID,
        0,
    )));
    let slowness_pose =
        advance_forward_from_standard_start(&mut slowness_world, LOCAL_PHYSICS_TICK_SECONDS);
    let slowness_step = first_ground_step(TEST_DEFAULT_MOVEMENT_SPEED, 0.85);
    assert_f64_near(slowness_pose.position.z, 0.5 + slowness_step, 0.000001);
    assert_f64_near(
        slowness_pose.delta_movement.z,
        carried_after_ground_step(slowness_step),
        0.000001,
    );
}

#[test]
fn local_player_speed_effect_does_not_double_apply_synced_attribute_modifier() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
            base: LOCAL_INPUT_DEFAULT_MOVEMENT_SPEED_ATTRIBUTE,
            modifiers: vec![ProtocolAttributeModifier {
                id: VANILLA_SPEED_EFFECT_MODIFIER_ID.to_string(),
                amount: 0.4,
                operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
            }],
        }],
    }));
    assert!(world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_SPEED_ID, 1,)));

    let pose = advance_forward_from_standard_start(&mut world, LOCAL_PHYSICS_TICK_SECONDS);
    let expected_step = first_ground_step(TEST_DEFAULT_MOVEMENT_SPEED, 1.4);
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        carried_after_ground_step(expected_step),
        0.000001,
    );
}

#[test]
fn local_player_sneak_uses_sneaking_speed_attribute_when_present() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SNEAKING_SPEED_ID,
            base: 0.4,
            modifiers: Vec::new(),
        }],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_step = first_ground_step(TEST_DEFAULT_MOVEMENT_SPEED, 0.4);
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        carried_after_ground_step(expected_step),
        0.000001,
    );
    assert!(pose.on_ground);
}
