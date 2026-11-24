use super::*;

#[test]
fn local_player_collision_excludes_same_root_vehicle() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    world.apply_add_entity(ProtocolAddEntity {
        id: 456,
        uuid: Uuid::from_u128(0x45645678456456784564567845645682),
        entity_type_id: VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        position: vec3(0.5, 1.0, 1.5),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 456,
        passenger_ids: vec![123],
    }));
    let pose = LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    };
    let entity_collision_targets = world.entities.client_movement_entity_collision_targets();

    assert_eq!(
        world.clip_local_player_collision_movement(
            pose,
            [0.0, 0.0, 2.0],
            LOCAL_PLAYER_STEP_HEIGHT as f32,
            false,
            &entity_collision_targets,
        ),
        [0.0, 0.0, 2.0]
    );
}

#[test]
fn local_player_horizontal_collision_uses_vanilla_equal_threshold() {
    assert!(!local_player_horizontal_axis_collided(9.999e-6, 0.0));
    assert!(local_player_horizontal_axis_collided(
        VANILLA_HORIZONTAL_COLLISION_EPSILON,
        0.0
    ));
    assert!(local_player_horizontal_axis_collided(1.0, 0.0));
}

#[test]
fn local_player_respects_conduit_and_azalea_collision_shapes() {
    let cases = [
        ("conduit", CONDUIT_DRY_BLOCK_STATE_ID, 0.9, 1.013),
        ("azalea", AZALEA_BLOCK_STATE_ID, 0.0, 0.7005),
        (
            "flowering azalea",
            FLOWERING_AZALEA_BLOCK_STATE_ID,
            0.0,
            0.7005,
        ),
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
fn local_player_ignores_pale_moss_carpet_topper_collision() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        0,
        PALE_MOSS_CARPET_TOPPER_WITH_SIDES_BLOCK_STATE_ID,
    );
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            2.0,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(pose.on_ground);
    assert!(!pose.horizontal_collision);
}

#[test]
fn local_player_ignores_bamboo_sapling_collision() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, BAMBOO_SAPLING_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            2.0,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(pose.on_ground);
    assert!(!pose.horizontal_collision);
}

#[test]
fn local_player_ignores_vanilla_no_collision_vegetation_and_overlays() {
    let cases = [
        ("dandelion", DANDELION_BLOCK_STATE_ID),
        ("dead bush", DEAD_BUSH_BLOCK_STATE_ID),
        ("bush", BUSH_BLOCK_STATE_ID),
        ("firefly bush", FIREFLY_BUSH_BLOCK_STATE_ID),
        (
            "mangrove propagule",
            MANGROVE_PROPAGULE_AGE_4_DRY_BLOCK_STATE_ID,
        ),
        ("crimson fungus", CRIMSON_FUNGUS_BLOCK_STATE_ID),
        ("nether sprouts", NETHER_SPROUTS_BLOCK_STATE_ID),
        ("wheat", WHEAT_AGE_7_BLOCK_STATE_ID),
        ("carrots", CARROTS_AGE_7_BLOCK_STATE_ID),
        ("potatoes", POTATOES_AGE_7_BLOCK_STATE_ID),
        ("beetroots", BEETROOTS_AGE_3_BLOCK_STATE_ID),
        ("nether wart", NETHER_WART_AGE_3_BLOCK_STATE_ID),
        ("sugar cane", SUGAR_CANE_AGE_0_BLOCK_STATE_ID),
        ("pink petals", PINK_PETALS_ONE_NORTH_BLOCK_STATE_ID),
        ("leaf litter", LEAF_LITTER_ONE_NORTH_BLOCK_STATE_ID),
        ("sculk vein", SCULK_VEIN_UP_DRY_BLOCK_STATE_ID),
        ("fire", FIRE_UP_AGE_0_BLOCK_STATE_ID),
        ("redstone wire", REDSTONE_WIRE_POWER_0_DOT_BLOCK_STATE_ID),
        ("tripwire", TRIPWIRE_EMPTY_BLOCK_STATE_ID),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

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
fn local_player_ignores_full_tilt_big_dripleaf_and_stem_collision() {
    let cases = [
        ("full tilt leaf", BIG_DRIPLEAF_NORTH_FULL_DRY_BLOCK_STATE_ID),
        ("stem", BIG_DRIPLEAF_STEM_NORTH_DRY_BLOCK_STATE_ID),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, block_state_id);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 3.0, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                2.0,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.on_ground, "{name}");
        assert!(!pose.horizontal_collision, "{name}");
    }
}

#[test]
fn local_player_respects_cake_bite_collision_width() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, CAKE_BITES_3_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.1, 3.0, 1.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            2.0,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(pose.on_ground);
    assert!(!pose.horizontal_collision);
}

#[test]
fn local_player_respects_cocoa_pod_collision() {
    let cases = [
        (
            "north-facing mature cocoa",
            COCOA_AGE_2_NORTH_BLOCK_STATE_ID,
            0.763,
        ),
        (
            "south-facing mature cocoa",
            COCOA_AGE_2_SOUTH_BLOCK_STATE_ID,
            1.138,
        ),
    ];

    for (name, block_state_id, max_z) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z > 0.7005 && pose.position.z <= max_z,
            "{name} position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_respects_shelf_collision() {
    let cases = [
        (
            "north-facing shelf",
            OAK_SHELF_NORTH_BLOCK_STATE_ID,
            1.38,
            1.388,
        ),
        (
            "south-facing shelf",
            OAK_SHELF_SOUTH_BLOCK_STATE_ID,
            0.699,
            0.7005,
        ),
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
fn local_player_respects_amethyst_cluster_collision() {
    let cases = [
        (
            "north-facing amethyst cluster",
            AMETHYST_CLUSTER_NORTH_BLOCK_STATE_ID,
            1.25,
            1.263,
        ),
        (
            "south-facing amethyst cluster",
            AMETHYST_CLUSTER_SOUTH_BLOCK_STATE_ID,
            0.699,
            0.7005,
        ),
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
fn local_player_respects_copper_golem_statue_collision() {
    for (name, block_state_id) in [
        (
            "copper golem statue",
            COPPER_GOLEM_STATUE_STANDING_NORTH_BLOCK_STATE_ID,
        ),
        (
            "waxed oxidized copper golem statue",
            WAXED_OXIDIZED_COPPER_GOLEM_STATUE_STANDING_NORTH_BLOCK_STATE_ID,
        ),
    ] {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z > 0.887 && pose.position.z <= 0.888,
            "{name} position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_does_not_collide_with_pressure_plate_outline() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, STONE_PRESSURE_PLATE_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 0.2);

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(pose.position.z > 1.0, "position was {:?}", pose.position);
    assert!(!pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_does_not_collide_with_no_collision_rails_and_controls() {
    let cases = [
        ("rail", RAIL_NORTH_SOUTH_BLOCK_STATE_ID),
        ("powered rail", POWERED_RAIL_NORTH_SOUTH_BLOCK_STATE_ID),
        ("detector rail", DETECTOR_RAIL_NORTH_SOUTH_BLOCK_STATE_ID),
        ("activator rail", ACTIVATOR_RAIL_NORTH_SOUTH_BLOCK_STATE_ID),
        ("torch", TORCH_BLOCK_STATE_ID),
        ("wall torch", WALL_TORCH_NORTH_BLOCK_STATE_ID),
        ("lever", LEVER_FLOOR_NORTH_BLOCK_STATE_ID),
        ("button", STONE_BUTTON_FLOOR_NORTH_BLOCK_STATE_ID),
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
fn local_player_does_not_collide_with_noncolliding_sign_outlines() {
    let cases = [
        ("standing sign", OAK_SIGN_ROTATION_0_BLOCK_STATE_ID),
        ("wall sign", OAK_WALL_SIGN_NORTH_BLOCK_STATE_ID),
        (
            "ceiling hanging sign",
            OAK_HANGING_SIGN_ATTACHED_ROTATION_0_BLOCK_STATE_ID,
        ),
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
fn local_player_collides_with_wall_hanging_sign_plank() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        1,
        OAK_WALL_HANGING_SIGN_NORTH_BLOCK_STATE_ID,
    );
    let pose = advance_forward_from_standard_start(&mut world, 0.2);

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(
        pose.position.z > 1.0 && pose.position.z <= 1.076,
        "wall hanging sign position was {:?}",
        pose.position
    );
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_does_not_collide_with_banner_outline() {
    let cases = [
        ("standing banner", WHITE_BANNER_ROTATION_0_BLOCK_STATE_ID),
        ("wall banner", WHITE_WALL_BANNER_NORTH_BLOCK_STATE_ID),
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
fn local_player_respects_extended_piston_base_shape() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, PISTON_EXTENDED_NORTH_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert!(
        pose.position.z > 0.7005 && pose.position.z <= 0.9505,
        "position was {:?}",
        pose.position
    );
    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_respects_closed_and_open_fence_gate_collision() {
    let mut closed_world = flat_collision_world();
    set_test_block(
        &mut closed_world,
        0,
        1,
        1,
        OAK_CLOSED_NORTH_FENCE_GATE_BLOCK_STATE_ID,
    );
    let closed_pose = advance_forward_from_standard_start(&mut closed_world, 1.0);

    assert!(
        closed_pose.position.z <= 1.0755,
        "position was {:?}",
        closed_pose.position
    );
    assert!(closed_pose.horizontal_collision);
    assert!(closed_pose.on_ground);

    let mut open_world = flat_collision_world();
    set_test_block(
        &mut open_world,
        0,
        1,
        1,
        OAK_OPEN_NORTH_FENCE_GATE_BLOCK_STATE_ID,
    );
    let open_pose = advance_forward_from_standard_start(&mut open_world, 0.35);

    assert!(
        open_pose.position.z > 1.5,
        "position was {:?}",
        open_pose.position
    );
    assert!(!open_pose.horizontal_collision);
    assert!(open_pose.on_ground);
}

#[test]
fn local_player_respects_piglin_wall_head_wider_collision() {
    let mut normal_world = flat_collision_world();
    set_test_block(
        &mut normal_world,
        0,
        1,
        1,
        SKELETON_WALL_SKULL_NORTH_BLOCK_STATE_ID,
    );
    normal_world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(1.06, 1.0, 0.5),
        y_rot: 0.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    let normal_pose = normal_world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            1.0,
        )
        .unwrap();

    assert!(
        normal_pose.position.z > 1.5,
        "normal wall skull position was {:?}",
        normal_pose.position
    );
    assert!(!normal_pose.horizontal_collision);
    assert!(normal_pose.on_ground);

    let mut piglin_world = flat_collision_world();
    set_test_block(
        &mut piglin_world,
        0,
        1,
        1,
        PIGLIN_WALL_HEAD_NORTH_BLOCK_STATE_ID,
    );
    piglin_world.set_local_player_auto_jump_enabled(false);
    piglin_world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(1.06, 1.0, 0.5),
        y_rot: 0.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    let piglin_pose = piglin_world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            1.0,
        )
        .unwrap();

    assert!(
        piglin_pose.position.z <= 1.2005,
        "piglin wall head position was {:?}",
        piglin_pose.position
    );
    assert!(piglin_pose.horizontal_collision);
    assert!(piglin_pose.on_ground);
}

#[test]
fn local_player_collision_detects_ceiling_bell_body() {
    let mut world = flat_collision_world();
    let pos = BlockPos { x: 0, y: 1, z: 1 };
    set_test_block(
        &mut world,
        pos.x,
        pos.y,
        pos.z,
        BELL_CEILING_NORTH_BLOCK_STATE_ID,
    );

    assert!(world.local_player_pose_collides_with_block(
        pos,
        LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 1.5),
            ..LocalPlayerPoseState::default()
        },
    ));
}
