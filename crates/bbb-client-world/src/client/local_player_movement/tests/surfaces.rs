use super::*;

#[test]
fn local_player_spectator_no_physics_crosses_blocks_and_world_border() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    set_test_block(&mut world, 0, 1, 2, GRASS_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 2, 2, GRASS_BLOCK_STATE_ID);
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
    world.apply_game_event(ProtocolGameEvent {
        event_id: 3,
        param: 3.0,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        delta_movement: vec3(0.0, 0.0, 3.0),
        on_ground: true,
        horizontal_collision: true,
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

    assert_f64_near(pose.position.z, 3.5, 0.000001);
    assert!(!pose.on_ground);
    assert!(!pose.horizontal_collision);
}

#[test]
fn local_player_climbable_limits_downward_velocity_and_resets_fall_distance() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, LADDER_SOUTH_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        delta_movement: vec3(0.0, -0.5, 0.0),
        fall_distance: 2.0,
        on_ground: false,
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

    assert_f64_near(pose.position.y, 1.05, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.2254, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_sneak_does_not_suppress_scaffolding_climbable_sliding() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        0,
        SCAFFOLDING_BOTTOM_DISTANCE_0_BLOCK_STATE_ID,
    );
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        delta_movement: vec3(0.0, -0.5, 0.0),
        fall_distance: 2.0,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.05, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.2254, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_scaffolding_top_supports_player_when_not_descending() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        0,
        SCAFFOLDING_BOTTOM_DISTANCE_0_BLOCK_STATE_ID,
    );
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 2.0, 0.5),
        delta_movement: vec3(0.0, -0.2, 0.0),
        fall_distance: 2.0,
        on_ground: false,
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

    assert_f64_near(pose.position.y, 2.0, 0.000001);
    // Grounded equilibrium: fall-on callback zeroes y, then gravity and
    // the 0.98 vertical friction rebuild -0.08 * 0.98.
    assert_f64_near(pose.delta_movement.y, -0.08 * 0.98, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(pose.on_ground);
}

#[test]
fn local_player_sneak_descends_through_scaffolding_top() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        0,
        SCAFFOLDING_BOTTOM_DISTANCE_0_BLOCK_STATE_ID,
    );
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 2.0, 0.5),
        delta_movement: vec3(0.0, -0.2, 0.0),
        fall_distance: 2.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.8, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.2744, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_scaffolding_unstable_bottom_supports_inside_player() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        0,
        SCAFFOLDING_BOTTOM_DISTANCE_1_BLOCK_STATE_ID,
    );
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        delta_movement: vec3(0.0, -0.5, 0.0),
        fall_distance: 2.0,
        on_ground: false,
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

    assert_f64_near(pose.position.y, 1.125, 0.000001);
    // Grounded equilibrium: fall-on callback zeroes y, then gravity and
    // the 0.98 vertical friction rebuild -0.08 * 0.98.
    assert_f64_near(pose.delta_movement.y, -0.08 * 0.98, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(pose.on_ground);
}

#[test]
fn local_player_open_trapdoor_above_matching_ladder_counts_as_climbable() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, LADDER_SOUTH_BLOCK_STATE_ID);
    set_test_block(
        &mut world,
        0,
        1,
        0,
        OAK_TOP_OPEN_SOUTH_TRAPDOOR_BLOCK_STATE_ID,
    );
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        delta_movement: vec3(0.0, -0.5, 0.0),
        fall_distance: 2.0,
        on_ground: false,
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

    assert_f64_near(pose.position.y, 1.05, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.2254, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_open_trapdoor_requires_matching_ladder_facing_for_climbable() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, LADDER_SOUTH_BLOCK_STATE_ID);
    set_test_block(
        &mut world,
        0,
        1,
        0,
        OAK_TOP_OPEN_NORTH_TRAPDOOR_BLOCK_STATE_ID,
    );
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        delta_movement: vec3(0.0, -0.1, 0.0),
        fall_distance: 2.0,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.1, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.1764, 0.000001);
    assert_f64_near(pose.fall_distance, 2.1, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_closed_trapdoor_above_matching_ladder_is_not_climbable() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, LADDER_NORTH_BLOCK_STATE_ID);
    set_test_block(
        &mut world,
        0,
        1,
        0,
        OAK_TOP_CLOSED_NORTH_TRAPDOOR_BLOCK_STATE_ID,
    );
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        delta_movement: vec3(0.0, -0.1, 0.0),
        fall_distance: 2.0,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.1, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.1764, 0.000001);
    assert_f64_near(pose.fall_distance, 2.1, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_jump_on_climbable_applies_vanilla_upward_velocity() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, LADDER_SOUTH_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        fall_distance: 2.0,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                jump: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.2, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.1176, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_climbable_clamps_horizontal_velocity() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, LADDER_SOUTH_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        // Carried momentum above the 0.15 climbable clamp: the clamp, not
        // the per-tick acceleration, bounds this tick's movement.
        delta_movement: vec3(0.0, 0.0, 0.4),
        on_ground: false,
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

    assert_f64_near(pose.position.z, 0.65, 0.000001);
    // The clamped 0.15 carries through the airborne 0.91 friction multiply.
    assert_f64_near(pose.delta_movement.z, 0.15 * 0.91, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_steps_onto_bottom_slab() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 2, OAK_BOTTOM_SLAB_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 0.5);

    assert_f64_near(pose.position.y, 1.5, 0.0005);
    assert!(pose.position.z > 1.7, "position was {:?}", pose.position);
    assert!(pose.on_ground);
    assert!(!pose.horizontal_collision);
}

#[test]
fn local_player_does_not_step_through_top_slab() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 2, OAK_TOP_SLAB_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(
        pose.position.z <= 1.7005,
        "position was {:?}",
        pose.position
    );
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_steps_up_bottom_stair_from_low_side() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        2,
        OAK_BOTTOM_STRAIGHT_SOUTH_STAIR_BLOCK_STATE_ID,
    );
    let pose = advance_forward_from_standard_start(&mut world, 0.6);

    assert_f64_near(pose.position.y, 2.0, 0.0005);
    assert!(pose.position.z > 2.2, "position was {:?}", pose.position);
    assert!(pose.on_ground);
    assert!(!pose.horizontal_collision);
}

#[test]
fn local_player_does_not_step_up_bottom_stair_from_high_side() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        2,
        OAK_BOTTOM_STRAIGHT_NORTH_STAIR_BLOCK_STATE_ID,
    );
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(
        pose.position.z <= 1.7005,
        "position was {:?}",
        pose.position
    );
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_does_not_step_through_top_stair() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        2,
        OAK_TOP_STRAIGHT_SOUTH_STAIR_BLOCK_STATE_ID,
    );
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(
        pose.position.z <= 1.7005,
        "position was {:?}",
        pose.position
    );
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_steps_over_thin_ground_shapes() {
    let cases = [
        ("white carpet", WHITE_CARPET_BLOCK_STATE_ID, 1.0625),
        (
            "pale moss carpet base",
            PALE_MOSS_CARPET_BOTTOM_NO_SIDES_BLOCK_STATE_ID,
            1.0625,
        ),
        ("five snow layers", SNOW_5_LAYERS_BLOCK_STATE_ID, 1.5),
    ];

    for (name, block_state_id, expected_y) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 0.3);

        assert_f64_near(pose.position.y, expected_y, 0.0005);
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
fn local_player_steps_onto_low_campfire_and_lantern_shapes() {
    let cases = [
        ("campfire", CAMPFIRE_NORTH_LIT_BLOCK_STATE_ID, 1.4375),
        ("standing lantern", LANTERN_STANDING_BLOCK_STATE_ID, 1.5625),
    ];

    for (name, block_state_id, expected_y) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 0.3);

        assert_f64_near(pose.position.y, expected_y, 0.0005);
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
fn local_player_ignores_vanilla_no_collision_hazard_blocks() {
    let cases = [
        ("cobweb", COBWEB_BLOCK_STATE_ID),
        ("sweet berry bush", SWEET_BERRY_BUSH_AGE_3_BLOCK_STATE_ID),
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
fn local_player_walks_up_to_narrow_cactus_and_honey_columns() {
    let cases = [
        ("cactus", CACTUS_AGE_0_BLOCK_STATE_ID),
        ("honey block", HONEY_BLOCK_STATE_ID),
        ("dragon egg", DRAGON_EGG_BLOCK_STATE_ID),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z > 0.7005 && pose.position.z <= 0.763,
            "{name} position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_does_not_collide_with_invisible_nonblocking_blocks() {
    let cases = [
        ("end portal", END_PORTAL_BLOCK_STATE_ID),
        ("end gateway", END_GATEWAY_BLOCK_STATE_ID),
        ("structure void", STRUCTURE_VOID_BLOCK_STATE_ID),
        ("light", LIGHT_LEVEL_15_DRY_BLOCK_STATE_ID),
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
fn local_player_respects_piston_head_rod_extending_outside_block() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 2, PISTON_HEAD_SOUTH_LONG_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert!(
        pose.position.z > 0.7005 && pose.position.z <= 1.4505,
        "position was {:?}",
        pose.position
    );
    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_does_not_collide_with_moving_piston_without_block_entity() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, MOVING_PISTON_NORTH_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 0.2);

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(pose.position.z > 1.0, "position was {:?}", pose.position);
    assert!(!pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_does_not_walk_through_ladder_sheet_on_approached_side() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, LADDER_SOUTH_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert!(
        pose.position.z <= 0.7005,
        "position was {:?}",
        pose.position
    );
    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_moves_through_scaffolding_side_instead_of_full_block() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        1,
        SCAFFOLDING_BOTTOM_DISTANCE_0_BLOCK_STATE_ID,
    );
    let pose = advance_forward_from_standard_start(&mut world, 0.2);

    assert!(pose.position.z > 0.8, "position was {:?}", pose.position);
    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(!pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_does_not_step_over_tall_snow_layer() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, SNOW_6_LAYERS_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(
        pose.position.z <= 0.7005,
        "position was {:?}",
        pose.position
    );
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_steps_onto_lectern_base_but_stops_at_center_column() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, LECTERN_NORTH_NO_BOOK_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert_f64_near(pose.position.y, 1.125, 0.0005);
    assert!(
        pose.position.z > 0.7005 && pose.position.z <= 0.9505,
        "position was {:?}",
        pose.position
    );
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_steps_onto_common_low_object_shapes() {
    let cases = [
        ("north foot bed", WHITE_BED_NORTH_FOOT_BLOCK_STATE_ID),
        ("north stonecutter", STONECUTTER_NORTH_BLOCK_STATE_ID),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 0.3);

        assert_f64_near(pose.position.y, 1.5625, 0.0005);
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
fn local_player_steps_onto_brewing_stand_base_but_stops_at_center_rod() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, BREWING_STAND_EMPTY_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert_f64_near(pose.position.y, 1.125, 0.0005);
    assert!(
        pose.position.z > 1.0 && pose.position.z <= 1.1376,
        "position was {:?}",
        pose.position
    );
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_composter_collision_uses_level_zero_shape_for_every_level() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, COMPOSTER_LEVEL_7_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 1.5),
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

    assert_f64_near(pose.position.y, 1.125, 0.0005);
    assert!(pose.on_ground);
    assert!(!pose.horizontal_collision);
}

#[test]
fn local_player_sneak_backs_off_from_block_edge() {
    let mut world = single_floor_block_world();
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
            1.0,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(
        pose.position.z <= 1.3005,
        "position was {:?}",
        pose.position
    );
    assert!(pose.sneaking);
    assert!(pose.on_ground);
    assert!(!pose.horizontal_collision);
}

#[test]
fn local_player_sneak_edge_backoff_stops_after_step_height_fall_distance() {
    let mut world = single_floor_block_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.3, 1.25),
        fall_distance: LOCAL_PLAYER_STEP_HEIGHT,
        on_ground: false,
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

    // Airborne: the sneak-scaled input drives the 0.02 air acceleration.
    let expected_step = LOCAL_PLAYER_INPUT_SCALE
        * LOCAL_INPUT_SNEAKING_SPEED_MULTIPLIER
        * LOCAL_PLAYER_AIR_ACCELERATION;
    assert_f64_near(pose.position.z, 1.25 + expected_step, 0.000001);
    assert_eq!(pose.fall_distance, LOCAL_PLAYER_STEP_HEIGHT);
}

#[test]
fn local_player_without_sneak_walks_off_block_edge() {
    let mut world = single_floor_block_world();
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
            0.5,
        )
        .unwrap();

    assert!(pose.position.z > 1.3, "position was {:?}", pose.position);
    assert!(
        pose.position.y < 1.0 || !pose.on_ground,
        "pose was {:?}",
        pose
    );
}

#[test]
fn local_player_sneak_backs_off_from_block_corner_diagonally() {
    let mut world = single_floor_block_world();
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
                right: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            1.0,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(
        pose.position.x >= -0.3005 && pose.position.z <= 1.3005,
        "position was {:?}",
        pose.position
    );
    assert!(pose.on_ground);
    assert!(!pose.horizontal_collision);
}

#[test]
fn local_player_slow_block_speed_factor_scales_horizontal_movement() {
    for (name, resource_id, block_state_id, player_y) in [
        (
            "soul sand",
            "minecraft:soul_sand",
            SOUL_SAND_BLOCK_STATE_ID,
            0.875,
        ),
        (
            "honey block",
            "minecraft:honey_block",
            HONEY_BLOCK_STATE_ID,
            0.9375,
        ),
    ] {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 0, 0, block_state_id);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, player_y, 0.5),
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

        // Vanilla applies `getBlockSpeedFactor` to the carried delta at the
        // end of `Entity.move`, so the first step from rest is unscaled and
        // the factor shows up in the carried delta (and the next tick).
        let speed_factor = f64::from(crate::terrain::vanilla_block_speed_factor(Some(
            resource_id,
        )));
        let first_step = first_ground_step(TEST_DEFAULT_MOVEMENT_SPEED, 1.0);
        assert_f64_near(pose.position.z, 0.5 + first_step, 0.000001);
        assert_f64_near(
            pose.delta_movement.z,
            carried_after_ground_step(first_step) * speed_factor,
            0.000001,
        );
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_movement_efficiency_offsets_slow_block_speed_factor() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, SOUL_SAND_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_MOVEMENT_EFFICIENCY_ID,
            base: 1.0,
            modifiers: Vec::new(),
        }],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 0.875, 0.5),
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

    let expected_step = first_ground_step(TEST_DEFAULT_MOVEMENT_SPEED, 1.0);
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        carried_after_ground_step(expected_step),
        0.000001,
    );
    assert!(pose.on_ground);
}

#[test]
fn local_player_ticks_frozen_applies_powder_snow_slowdown() {
    let mut half_frozen_world = flat_collision_world();
    attach_local_player_entity(&mut half_frozen_world, 123);
    assert!(apply_ticks_frozen(&mut half_frozen_world, 123, 70));

    let half_frozen_pose =
        advance_forward_from_standard_start(&mut half_frozen_world, LOCAL_PHYSICS_TICK_SECONDS);
    let half_frozen_step = first_ground_step(TEST_DEFAULT_MOVEMENT_SPEED, 0.75);
    assert_f64_near(
        half_frozen_pose.position.z,
        0.5 + half_frozen_step,
        0.000001,
    );
    assert_f64_near(
        half_frozen_pose.delta_movement.z,
        carried_after_ground_step(half_frozen_step),
        0.000001,
    );

    let mut fully_frozen_world = flat_collision_world();
    attach_local_player_entity(&mut fully_frozen_world, 124);
    assert!(apply_ticks_frozen(&mut fully_frozen_world, 124, 280));

    let fully_frozen_pose =
        advance_forward_from_standard_start(&mut fully_frozen_world, LOCAL_PHYSICS_TICK_SECONDS);
    let fully_frozen_step = first_ground_step(TEST_DEFAULT_MOVEMENT_SPEED, 0.5);
    assert_f64_near(
        fully_frozen_pose.position.z,
        0.5 + fully_frozen_step,
        0.000001,
    );
    assert_f64_near(
        fully_frozen_pose.delta_movement.z,
        carried_after_ground_step(fully_frozen_step),
        0.000001,
    );
}

#[test]
fn local_player_powder_snow_speed_modifier_does_not_double_apply_synced_attribute_modifier() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
            base: LOCAL_INPUT_DEFAULT_MOVEMENT_SPEED_ATTRIBUTE,
            modifiers: vec![ProtocolAttributeModifier {
                id: VANILLA_POWDER_SNOW_SPEED_MODIFIER_ID.to_string(),
                amount: -0.025,
                operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
            }],
        }],
    }));
    assert!(apply_ticks_frozen(&mut world, 123, 70));

    let pose = advance_forward_from_standard_start(&mut world, LOCAL_PHYSICS_TICK_SECONDS);
    let expected_step = first_ground_step(TEST_DEFAULT_MOVEMENT_SPEED, 0.75);
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        carried_after_ground_step(expected_step),
        0.000001,
    );
}

#[test]
fn local_player_in_powder_snow_increments_ticks_frozen() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);

    advance_effects_tick_at(&mut world, vec3(0.5, 1.2, 0.5));

    assert_eq!(world.entities.ticks_frozen(123), Some(1));
    assert_eq!(
        world
            .entities
            .metadata(123)
            .unwrap()
            .data_values
            .iter()
            .find(|value| { value.data_id == crate::entities::VANILLA_ENTITY_TICKS_FROZEN_DATA_ID })
            .map(|value| value.serializer_id),
        Some(bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID)
    );
}

#[test]
fn local_player_in_powder_snow_caps_ticks_frozen_at_required_ticks() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(apply_ticks_frozen(
        &mut world,
        123,
        LOCAL_PLAYER_TICKS_REQUIRED_TO_FREEZE,
    ));
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);

    advance_effects_tick_at(&mut world, vec3(0.5, 1.2, 0.5));

    assert_eq!(
        world.entities.ticks_frozen(123),
        Some(LOCAL_PLAYER_TICKS_REQUIRED_TO_FREEZE)
    );
}

#[test]
fn local_player_moving_in_powder_snow_emits_snowflake_chance() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    world.drain_entity_client_movement_effect_states();

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

    let snowflakes = world
        .drain_entity_client_movement_effect_states()
        .filter_map(|effect| match effect {
            crate::entities::EntityClientMovementEffectState::Particle(
                crate::entities::EntityClientMovementParticleState::PowderSnowFlake(particle),
            ) => Some(particle),
            _ => None,
        })
        .collect::<Vec<_>>();
    let [snowflake] = snowflakes.as_slice() else {
        panic!("expected one snowflake chance, got {snowflakes:?}");
    };
    assert_eq!(snowflake.entity_id, 123);
    // Anchored at the step's end position over the Vanilla `pos.getY() + 1`.
    assert_f64_near(snowflake.position.x, pose.position.x, 0.000001);
    assert_f64_near(snowflake.position.z, pose.position.z, 0.000001);
    assert_f64_near(snowflake.position.y, 2.0, 0.000001);
}

#[test]
fn local_player_still_in_powder_snow_rolls_no_snowflake_chance() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    world.drain_entity_client_movement_effect_states();

    advance_effects_tick_at(&mut world, vec3(0.5, 1.2, 0.5));

    assert_eq!(world.entities.ticks_frozen(123), Some(1));
    assert!(world
        .drain_entity_client_movement_effect_states()
        .all(|effect| !matches!(
            effect,
            crate::entities::EntityClientMovementEffectState::Particle(
                crate::entities::EntityClientMovementParticleState::PowderSnowFlake(_),
            )
        )));
}

#[test]
fn local_player_out_of_powder_snow_keeps_synced_ticks_frozen() {
    // Vanilla thaws `ticksFrozen` only on the server (`LivingEntity.aiStep`
    // freezing block is `ServerLevel`-gated); the client keeps the synced
    // value until the server sends a new one.
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(apply_ticks_frozen(&mut world, 123, 5));

    advance_effects_tick_at(&mut world, vec3(0.5, 1.2, 0.5));

    assert_eq!(world.entities.ticks_frozen(123), Some(5));
}

#[test]
fn local_player_spectator_in_powder_snow_skips_block_effects() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(apply_ticks_frozen(&mut world, 123, 5));
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    world.apply_game_event(ProtocolGameEvent {
        event_id: 3,
        param: 3.0,
    });

    advance_effects_tick_at(&mut world, vec3(0.5, 1.2, 0.5));

    assert_eq!(world.entities.ticks_frozen(123), Some(5));
}

#[test]
fn local_player_freeze_immune_wearable_in_powder_snow_stops_freezing() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(apply_ticks_frozen(&mut world, 123, 5));
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    world.set_freeze_immune_wearable_item_ids(BTreeSet::from([FREEZE_IMMUNE_WEARABLE_ITEM_ID]));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_FEET_SLOT,
        item: item_stack(FREEZE_IMMUNE_WEARABLE_ITEM_ID, 1),
    });

    advance_effects_tick_at(&mut world, vec3(0.5, 1.2, 0.5));

    assert_eq!(world.entities.ticks_frozen(123), Some(5));
}

#[test]
fn local_player_empty_freeze_immune_wearable_stack_in_powder_snow_still_freezes() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(apply_ticks_frozen(&mut world, 123, 5));
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    world.set_freeze_immune_wearable_item_ids(BTreeSet::from([FREEZE_IMMUNE_WEARABLE_ITEM_ID]));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_FEET_SLOT,
        item: item_stack(FREEZE_IMMUNE_WEARABLE_ITEM_ID, 0),
    });

    advance_effects_tick_at(&mut world, vec3(0.5, 1.2, 0.5));

    assert_eq!(world.entities.ticks_frozen(123), Some(6));
}

#[test]
fn local_player_without_powder_snow_walkable_boots_sinks_through_powder_snow() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 2.0, 0.5),
        ..LocalPlayerPoseState::default()
    });

    world
        .advance_local_player_input(LocalPlayerInputState::default(), LOCAL_PHYSICS_TICK_SECONDS)
        .unwrap();
    world
        .advance_local_player_input(LocalPlayerInputState::default(), LOCAL_PHYSICS_TICK_SECONDS)
        .unwrap();

    let pose = world.local_player_pose().unwrap();
    assert!(pose.position.y < 2.0);
    assert!(!pose.on_ground);
    assert!(pose.delta_movement.y < 0.0);
}

#[test]
fn local_player_leather_boots_stand_on_powder_snow_top_collision() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    equip_powder_snow_walkable_boots(&mut world, 1);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 2.0, 0.5),
        ..LocalPlayerPoseState::default()
    });

    world
        .advance_local_player_input(
            LocalPlayerInputState::default(),
            LOCAL_PHYSICS_TICK_SECONDS * 2.0,
        )
        .unwrap();

    let pose = world.local_player_pose().unwrap();
    assert_f64_near(pose.position.y, 2.0, 0.000001);
    assert!(pose.on_ground);
    // Grounded equilibrium: fall-on callback zeroes y, then gravity and
    // the 0.98 vertical friction rebuild -0.08 * 0.98.
    assert_f64_near(pose.delta_movement.y, -0.08 * 0.98, 0.000001);
}

#[test]
fn local_player_freeze_immune_non_leather_boots_do_not_walk_on_powder_snow() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    world.set_freeze_immune_wearable_item_ids(BTreeSet::from([FREEZE_IMMUNE_WEARABLE_ITEM_ID]));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_FEET_SLOT,
        item: item_stack(FREEZE_IMMUNE_WEARABLE_ITEM_ID, 1),
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 2.0, 0.5),
        ..LocalPlayerPoseState::default()
    });

    world
        .advance_local_player_input(
            LocalPlayerInputState::default(),
            LOCAL_PHYSICS_TICK_SECONDS * 2.0,
        )
        .unwrap();

    let pose = world.local_player_pose().unwrap();
    assert!(pose.position.y < 2.0);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_sneak_descends_through_powder_snow_even_with_leather_boots() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    equip_powder_snow_walkable_boots(&mut world, 1);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 2.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS * 2.0,
        )
        .unwrap();

    let pose = world.local_player_pose().unwrap();
    assert!(pose.position.y < 2.0);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_high_fall_distance_uses_powder_snow_falling_collision_shape() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);

    assert!(!world.local_player_pose_collides_with_block(
        BlockPos { x: 0, y: 1, z: 0 },
        LocalPlayerPoseState {
            position: vec3(0.5, 1.85, 0.5),
            fall_distance: 2.5,
            ..LocalPlayerPoseState::default()
        },
    ));
    assert!(world.local_player_pose_collides_with_block(
        BlockPos { x: 0, y: 1, z: 0 },
        LocalPlayerPoseState {
            position: vec3(0.5, 1.85, 0.5),
            fall_distance: 2.500001,
            ..LocalPlayerPoseState::default()
        },
    ));
}

#[test]
fn local_player_powder_snow_fall_sound_uses_vanilla_distance_thresholds() {
    for (fall_distance, expected_big) in [(3.0, None), (4.0, Some(false)), (7.0, Some(true))] {
        let mut world = flat_collision_world();
        attach_local_player_entity(&mut world, 123);
        set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 2.1, 0.5),
            delta_movement: vec3(0.0, -0.5, 0.0),
            fall_distance,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });
        world.drain_entity_client_movement_effect_states();

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState::default(),
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert!(pose.on_ground, "fall distance {fall_distance}");
        assert_f64_near(pose.position.y, 1.9, 0.0001);
        assert_f64_near(pose.fall_distance, 0.0, 0.000001);
        let sounds = world
            .drain_entity_client_movement_effect_states()
            .filter_map(|effect| match effect {
                crate::entities::EntityClientMovementEffectState::Sound(sound) => Some(sound),
                _ => None,
            })
            .collect::<Vec<_>>();
        match expected_big {
            None => assert!(sounds.is_empty(), "fall distance {fall_distance}"),
            Some(expected_big) => {
                let [sound] = sounds.as_slice() else {
                    panic!("expected one powder-snow fall sound, got {sounds:?}");
                };
                assert_eq!(sound.entity_id, 123);
                assert_eq!(
                    sound.kind,
                    crate::entities::EntityClientMovementSoundKind::PlayerPowderSnowFall {
                        big: expected_big,
                    }
                );
                assert_f64_near(sound.position.x, pose.position.x, 0.000001);
                assert_f64_near(sound.position.y, pose.position.y, 0.000001);
                assert_f64_near(sound.position.z, pose.position.z, 0.000001);
            }
        }
    }
}

#[test]
fn powder_snow_fall_sound_thresholds_are_exact() {
    for (fall_distance, expected) in [
        (3.999_999, None),
        (4.0, Some(false)),
        (6.999_999, Some(false)),
        (7.0, Some(true)),
    ] {
        assert_eq!(
            player_powder_snow_fall_sound_size(fall_distance),
            expected,
            "fall distance {fall_distance}"
        );
    }
}

#[test]
fn local_player_fall_reset_ray_is_bounded_to_eight_blocks() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 4, 0, LADDER_SOUTH_BLOCK_STATE_ID);
    let pose = LocalPlayerPoseState {
        position: vec3(0.5, 14.0, 0.5),
        fall_distance: 10.0,
        ..LocalPlayerPoseState::default()
    };

    assert!(!world.local_player_fall_reset_ray_hits(pose, [0.0, -12.0, 0.0]));
    assert!(!world.local_player_fall_reset_ray_hits(pose, [0.0, -0.999, 0.0]));
}

#[test]
fn local_player_leather_boots_jump_climbs_out_of_powder_snow() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    equip_powder_snow_walkable_boots(&mut world, 1);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                jump: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.2, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.1176, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_without_leather_boots_jump_does_not_climb_out_of_powder_snow() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                jump: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.2, 0.000001);
    assert!(pose.delta_movement.y < 0.0);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_leather_boots_horizontal_collision_climbs_out_of_powder_snow() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 1, GRASS_BLOCK_STATE_ID);
    equip_powder_snow_walkable_boots(&mut world, 1);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        // Carried momentum so this tick reaches the grass wall face at z=1.
        delta_movement: vec3(0.0, 0.0, 0.25),
        on_ground: false,
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

    assert!(pose.horizontal_collision);
    assert_f64_near(pose.delta_movement.y, 0.1176, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_advance_keeps_ticks_frozen_across_physics_steps() {
    // Vanilla thaw is `ServerLevel`-gated, so multiple client physics steps
    // outside powder snow leave the synced value untouched.
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(apply_ticks_frozen(&mut world, 123, 7));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    world
        .advance_local_player_input(
            LocalPlayerInputState::default(),
            LOCAL_PHYSICS_TICK_SECONDS * 3.0,
        )
        .unwrap();

    assert_eq!(world.entities.ticks_frozen(123), Some(7));
}

#[test]
fn local_player_honey_block_jump_factor_scales_base_jump() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, HONEY_BLOCK_STATE_ID);
    let honey_top_y = 15.0 / 16.0;
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, honey_top_y, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                jump: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_jump = LOCAL_INPUT_DEFAULT_JUMP_STRENGTH_ATTRIBUTE * HONEY_BLOCK_JUMP_FACTOR;
    assert_f64_near(pose.position.y, honey_top_y + expected_jump, 0.000001);
    assert!(!pose.on_ground);
    assert!(pose.delta_movement.y > 0.0);
}

#[test]
fn local_player_honey_block_jump_factor_keeps_jump_boost_additive() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, HONEY_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_JUMP_BOOST_ID, 0,)));
    let honey_top_y = 15.0 / 16.0;
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, honey_top_y, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                jump: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_jump = LOCAL_INPUT_DEFAULT_JUMP_STRENGTH_ATTRIBUTE * HONEY_BLOCK_JUMP_FACTOR
        + JUMP_BOOST_VELOCITY_PER_LEVEL;
    assert_f64_near(pose.position.y, honey_top_y + expected_jump, 0.000001);
    assert!(!pose.on_ground);
    assert!(pose.delta_movement.y > 0.0);
}

#[test]
fn local_player_bounces_after_landing_on_slime_block() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, SLIME_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        delta_movement: vec3(0.0, -0.5, 0.0),
        fall_distance: 2.0,
        on_ground: false,
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

    assert_f64_near(pose.position.y, 1.0, 0.0001);
    // Slime reflects the fall, then gravity and the 0.98 vertical friction
    // shape the rebound: (0.5 - 0.08) * 0.98.
    assert_f64_near(pose.delta_movement.y, (0.5 - 0.08) * 0.98, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(pose.on_ground);
}

#[test]
fn local_player_sneak_suppresses_slime_block_bounce() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, SLIME_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        delta_movement: vec3(0.0, -0.5, 0.0),
        fall_distance: 2.0,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.0, 0.0001);
    // Grounded equilibrium: fall-on callback zeroes y, then gravity and
    // the 0.98 vertical friction rebuild -0.08 * 0.98.
    assert_f64_near(pose.delta_movement.y, -0.08 * 0.98, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(pose.on_ground);
}

#[test]
fn local_player_bed_bounce_uses_living_factor_and_sneak_suppression() {
    for suppressing_bounce in [false, true] {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 0, 0, WHITE_BED_NORTH_FOOT_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            delta_movement: vec3(0.0, -0.8, 0.0),
            fall_distance: 2.0,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    sneak: suppressing_bounce,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 9.0 / 16.0, 0.0001);
        let fall_on_velocity = if suppressing_bounce {
            0.0
        } else {
            0.8 * f64::from(LOCAL_PLAYER_BED_BOUNCE_FACTOR)
        };
        assert_f64_near(
            pose.delta_movement.y,
            (fall_on_velocity - 0.08) * 0.98,
            0.000001,
        );
        assert_f64_near(pose.fall_distance, 0.0, 0.000001);
        assert!(pose.on_ground);
    }
}

#[test]
fn local_player_honey_block_landing_queues_slide_sound_before_post_move_effects() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    set_test_block(&mut world, 0, 0, 0, HONEY_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        delta_movement: vec3(0.0, -0.5, 0.0),
        fall_distance: 2.0,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });
    world.drain_entity_client_movement_effect_states();

    let pose = world
        .advance_local_player_input(LocalPlayerInputState::default(), LOCAL_PHYSICS_TICK_SECONDS)
        .unwrap();

    assert!(pose.on_ground);
    assert_f64_near(pose.position.y, 15.0 / 16.0, 0.0001);
    let effects = world
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    let [crate::entities::EntityClientMovementEffectState::Sound(sound)] = effects.as_slice()
    else {
        panic!("expected one honey landing sound, got {effects:?}");
    };
    assert_eq!(sound.entity_id, 123);
    assert_eq!(
        sound.kind,
        crate::entities::EntityClientMovementSoundKind::PlayerHoneyBlockSlide
    );
    assert_f64_near(sound.position.y, pose.position.y, 0.000001);
}

#[test]
fn local_player_settles_on_next_support_after_block_below_vanishes() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, GRASS_BLOCK_STATE_ID);
    // The live-gate landing pose: resting a clip epsilon above a vacated
    // cell with the carried grounded gravity delta still applied each tick.
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.000003, 0.5),
        delta_movement: vec3(0.0, -0.0784, 0.0),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let mut pose = None;
    for _ in 0..40 {
        pose = Some(
            world
                .advance_local_player_input(
                    LocalPlayerInputState {
                        focused: true,
                        ..LocalPlayerInputState::default()
                    },
                    LOCAL_PHYSICS_TICK_SECONDS,
                )
                .unwrap(),
        );
    }
    let pose = pose.unwrap();
    assert!(
        (pose.position.y - 2.0).abs() < 0.001,
        "expected to settle on the block top at 2.0, got {:.9}",
        pose.position.y
    );
    assert!(pose.on_ground);
}

#[test]
fn local_player_fall_flying_on_climbable_clears_flag_and_uses_air_travel() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 4, 0, LADDER_SOUTH_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(apply_fall_flying(&mut world, 123));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 4.1, 0.5),
        delta_movement: vec3(0.0, 0.0, 0.4),
        on_ground: false,
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

    // `travelFallFlying` on a climbable runs `stopFallFlying` locally and
    // falls through to `travelInAir`, whose `handleOnClimbable` clamp caps
    // the carried horizontal delta at 0.15 before the move.
    assert_eq!(world.entities.is_fall_flying(123), Some(false));
    assert_f64_near(pose.position.z, 0.5 + 0.15, 0.000001);

    let second = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();
    assert!(second.position.z > 0.65);
}

#[test]
fn local_player_footsteps_follow_real_stone_travel_catch_up_and_wall_stop() {
    let mut world = footstep_world("minecraft:stone", None);
    let forward = LocalPlayerInputState {
        focused: true,
        forward: true,
        ..LocalPlayerInputState::default()
    };
    for _ in 0..7 {
        world.advance_local_player_input(forward, 0.05).unwrap();
    }
    assert!(
        take_footstep_sounds(&mut world).is_empty(),
        "no step before traveling 1/0.6 blocks"
    );
    for _ in 0..3 {
        world.advance_local_player_input(forward, 0.05).unwrap();
    }
    let first = take_footstep_sounds(&mut world);
    assert_eq!(first.len(), 1);
    assert_eq!(
        first[0].sound.location.as_deref(),
        Some("minecraft:block.stone.step")
    );
    assert_eq!(first[0].source, "player");
    assert_f32_near(first[0].volume, 0.15);
    assert_f32_near(first[0].pitch, 1.0);

    world.advance_local_player_input(forward, 0.5).unwrap();
    let caught_up = take_footstep_sounds(&mut world);
    assert_eq!(
        caught_up.len(),
        1,
        "catch-up emits the next actual-distance step"
    );
    assert!(caught_up[0].position.z > first[0].position.z + 1.5);
    let pose = world.local_player_pose().unwrap();
    world.set_local_player_pose(LocalPlayerPoseState {
        delta_movement: ProtocolVec3d::default(),
        ..pose
    });
    world
        .advance_local_player_input(LocalPlayerInputState::default(), 0.5)
        .unwrap();
    assert!(take_footstep_sounds(&mut world).is_empty());

    world.set_local_player_auto_jump_enabled(false);
    for x in 0..3 {
        for y in 1..3 {
            set_test_block(&mut world, x, y, 6, 1);
        }
    }
    world.advance_local_player_input(forward, 1.0).unwrap();
    take_footstep_sounds(&mut world);
    let at_wall = world.local_player_pose().unwrap();
    world.advance_local_player_input(forward, 0.5).unwrap();
    assert_f64_near(
        world.local_player_pose().unwrap().position.z,
        at_wall.position.z,
        1.0e-6,
    );
    assert!(
        take_footstep_sounds(&mut world).is_empty(),
        "held input against a wall produces no steps"
    );
}

#[test]
fn local_player_footsteps_resolve_surface_overlay_and_water_sounds() {
    for (floor, expected) in [
        ("minecraft:stone", "minecraft:block.stone.step"),
        ("minecraft:gravel", "minecraft:block.gravel.step"),
        ("minecraft:oak_planks", "minecraft:block.wood.step"),
        ("minecraft:snow_block", "minecraft:block.snow.step"),
    ] {
        let mut world = footstep_world(floor, None);
        world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                0.6,
            )
            .unwrap();
        let sounds = take_footstep_sounds(&mut world);
        assert!(!sounds.is_empty(), "{floor}");
        assert!(
            sounds
                .iter()
                .all(|sound| sound.sound.location.as_deref() == Some(expected)),
            "{floor}: {sounds:?}"
        );
    }
    for (overlay, tag, expected, combined) in [
        (
            "minecraft:snow",
            "minecraft:combination_step_sound_blocks",
            "minecraft:block.snow.step",
            true,
        ),
        (
            "minecraft:pink_petals",
            "minecraft:inside_step_sound_blocks",
            "minecraft:block.pink_petals.step",
            false,
        ),
    ] {
        let mut world = footstep_world("minecraft:stone", Some(overlay));
        let block_id = bbb_protocol::block_types::vanilla_block_type_for_resource_id(overlay)
            .unwrap()
            .id;
        world.apply_update_tags(ProtocolUpdateTags {
            registries: vec![ProtocolRegistryTags {
                registry: "minecraft:block".to_string(),
                tags: vec![ProtocolTagNetworkPayload {
                    tag: tag.to_string(),
                    entries: vec![block_id],
                }],
            }],
        });
        world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                0.6,
            )
            .unwrap();
        let sounds = take_footstep_sounds(&mut world);
        assert_eq!(
            sounds.len(),
            if combined { 2 } else { 1 },
            "{overlay}: {sounds:?}"
        );
        assert_eq!(sounds[0].sound.location.as_deref(), Some(expected));
        assert_f32_near(sounds[0].volume, 0.15);
        if combined {
            assert_eq!(
                sounds[1].sound.location.as_deref(),
                Some("minecraft:block.stone.step")
            );
            assert_f32_near(sounds[1].volume, 0.05);
            assert_f32_near(sounds[1].pitch, 0.8);
        }
    }
    let mut world = footstep_world("minecraft:stone", Some("minecraft:water"));
    world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            2.0,
        )
        .unwrap();
    let sounds = take_footstep_sounds(&mut world);
    assert!(sounds
        .iter()
        .any(|sound| sound.sound.location.as_deref() == Some("minecraft:entity.player.swim")));
    assert!(sounds.iter().any(|sound| sound.sound.location.as_deref()
        == Some("minecraft:block.stone.step")
        && sound.volume == 0.05
        && sound.pitch == 0.8));
}

#[test]
fn local_player_footsteps_respect_silent_shift_flight_passenger_and_spectator() {
    for gate in ["silent", "shift", "flight", "passenger", "spectator"] {
        let mut world = footstep_world("minecraft:stone", None);
        let mut input = LocalPlayerInputState {
            focused: true,
            forward: true,
            ..LocalPlayerInputState::default()
        };
        match gate {
            "silent" => {
                let accessors =
                    crate::entities::vanilla_entity_metadata_entity_accessors().unwrap();
                assert!(world.apply_set_entity_data(ProtocolSetEntityData { id: 123, values: vec![ProtocolEntityDataValue {
                    data_id: accessors.silent_data_id,
                    serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    value: ProtocolEntityDataValueKind::Boolean(true),
                }] }));
            }
            "shift" => input.sneak = true,
            "flight" => apply_flying_abilities(&mut world, 0.05),
            "passenger" => mount_local_player_on_entity(
                &mut world,
                123,
                456,
                VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
                vec![123],
            ),
            "spectator" => {
                world.apply_game_event(ProtocolGameEvent {
                    event_id: 3,
                    param: 3.0,
                });
            }
            _ => unreachable!(),
        }
        world.advance_local_player_input(input, 2.0).unwrap();
        assert!(take_footstep_sounds(&mut world).is_empty(), "{gate}");
    }
}

fn footstep_world(floor: &str, overlay: Option<&str>) -> WorldStore {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    let registry = crate::registries::BlockStateRegistry::vanilla_26_1();
    let floor_id = registry
        .find_by_name_and_properties(floor, &BTreeMap::new())
        .unwrap()
        .id;
    let overlay_id = overlay.map(|name| {
        let properties = match name {
            "minecraft:snow" => BTreeMap::from([("layers".to_string(), "1".to_string())]),
            "minecraft:pink_petals" => BTreeMap::from([
                ("facing".to_string(), "north".to_string()),
                ("flower_amount".to_string(), "1".to_string()),
            ]),
            "minecraft:water" => BTreeMap::from([("level".to_string(), "0".to_string())]),
            _ => unreachable!(),
        };
        registry
            .find_by_name_and_properties(name, &properties)
            .unwrap()
            .id
    });
    for x in 0..3 {
        for z in 0..16 {
            set_test_block(&mut world, x, 0, z, floor_id);
            if let Some(id) = overlay_id {
                set_test_block(&mut world, x, 1, z, id);
            }
        }
    }
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    world
}

fn take_footstep_sounds(world: &mut WorldStore) -> Vec<crate::SoundEventState> {
    let sounds: Vec<_> = world
        .drain_entity_client_movement_effect_states()
        .filter_map(|effect| match effect {
            EntityClientMovementEffectState::Sound(sound) => Some(sound),
            _ => None,
        })
        .collect();
    sounds
        .into_iter()
        .map(|sound| world.record_entity_client_movement_sound(sound))
        .collect()
}

#[test]
fn local_player_climbable_honors_synchronized_minecraft_climbable_block_tag() {
    use bbb_protocol::packets::{RegistryTags, TagNetworkPayload, UpdateTags};

    let mut world = flat_collision_world();
    let stone_state_id = crate::registries::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:stone", &BTreeMap::new())
        .unwrap()
        .id;
    let stone_block_id =
        bbb_protocol::block_state_types::vanilla_block_state_block_type_id(stone_state_id).unwrap();

    // Without climbable tag containing stone, stone is not climbable.
    set_test_block(&mut world, 0, 1, 0, stone_state_id);
    let pose = LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        ..LocalPlayerPoseState::default()
    };
    assert!(!crate::client::local_player_is_on_climbable(&world, pose));

    // Synchronize `#minecraft:climbable` containing stone; player on stone is now climbable.
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:block".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:climbable".to_string(),
                entries: vec![stone_block_id],
            }],
        }],
    });
    assert!(crate::client::local_player_is_on_climbable(&world, pose));
}
