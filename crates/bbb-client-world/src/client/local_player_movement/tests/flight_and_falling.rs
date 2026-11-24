use super::*;

#[test]
fn local_player_minor_horizontal_collision_fall_flying_uses_modified_input_direction() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(apply_fall_flying(&mut world, 123));
    set_test_block(&mut world, 0, 1, 0, GRASS_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 2, 0, GRASS_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(1.3, 1.0, 0.5),
        delta_movement: vec3(-0.2, 0.0, 0.2),
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

    assert!(pose.horizontal_collision, "pose {pose:?}");
    assert!(pose.minor_horizontal_collision, "pose {pose:?}");
}

#[test]
fn local_player_auto_jump_uses_collidable_entity_bounds() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    world.apply_add_entity(ProtocolAddEntity {
        id: 456,
        uuid: Uuid::from_u128(0x45645678456456784564567845645690),
        entity_type_id: VANILLA_ENTITY_TYPE_SHULKER_ID,
        position: vec3(0.5, 1.0, 1.5),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    let pose = LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        y_rot: 0.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    };
    let targets = world.entities.client_movement_entity_collision_targets();

    update_local_player_auto_jump(
        &mut world,
        pose,
        LocalPlayerInputState {
            focused: true,
            forward: true,
            ..LocalPlayerInputState::default()
        },
        0.01,
        0.0,
        &targets,
    );

    assert!(world.local_player_auto_jump_is_armed());
}

#[test]
fn local_player_auto_jump_excludes_world_border_collision_shape() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 2, GRASS_BLOCK_STATE_ID);
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
    let pose = LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 1.0),
        y_rot: 0.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    };
    let targets = world.entities.client_movement_entity_collision_targets();

    update_local_player_auto_jump(
        &mut world,
        pose,
        LocalPlayerInputState {
            focused: true,
            forward: true,
            ..LocalPlayerInputState::default()
        },
        0.0,
        0.4,
        &targets,
    );

    // The block starts exactly at the border. Vanilla's prospective
    // `getCollisions` still sees it because the border is queried elsewhere.
    assert!(world.local_player_auto_jump_is_armed());
}

#[test]
fn local_player_auto_jump_rejects_low_ceiling() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, GRASS_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 2, 0, GRASS_BLOCK_STATE_ID);
    let pose = LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        y_rot: 0.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    };
    let targets = world.entities.client_movement_entity_collision_targets();

    update_local_player_auto_jump(
        &mut world,
        pose,
        LocalPlayerInputState {
            focused: true,
            forward: true,
            ..LocalPlayerInputState::default()
        },
        0.01,
        0.0,
        &targets,
    );

    assert!(!world.local_player_auto_jump_is_armed());
}

#[test]
fn local_player_gravity_lands_on_floor_and_reports_grounded() {
    let mut world = flat_collision_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            1.0,
        )
        .unwrap();

    assert!(
        pose.position.y >= 0.9999,
        "position was {:?}",
        pose.position
    );
    assert!(pose.on_ground);
    assert!(!pose.horizontal_collision);
    // Grounded equilibrium: fall-on callback zeroes y, then gravity and
    // the 0.98 vertical friction rebuild -0.08 * 0.98.
    assert_f64_near(pose.delta_movement.y, -0.08 * 0.98, 0.000001);
}

#[test]
fn local_player_gravity_attribute_scales_airborne_fall_velocity() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_GRAVITY_ID,
            base: 0.04,
            modifiers: Vec::new(),
        }],
    }));
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
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 3.0, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.0392, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_no_gravity_metadata_suppresses_airborne_gravity() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(apply_no_gravity(&mut world, 123, true));
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
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 3.0, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_slow_falling_clamps_downward_gravity() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_SLOW_FALLING_ID, 0,)));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 0.5),
        delta_movement: vec3(0.0, -0.08, 0.0),
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

    assert_f64_near(pose.position.y, 2.92, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.0882, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_lands_on_vanilla_low_ground_shapes() {
    let cases = [
        ("farmland", FARMLAND_MOISTURE_0_BLOCK_STATE_ID, 1.9375),
        ("dirt path", DIRT_PATH_BLOCK_STATE_ID, 1.9375),
        ("soul sand", SOUL_SAND_BLOCK_STATE_ID, 1.875),
        ("mud", MUD_BLOCK_STATE_ID, 1.875),
        ("cake", CAKE_BITES_0_BLOCK_STATE_ID, 1.5),
        ("lily pad", LILY_PAD_BLOCK_STATE_ID, 1.09375),
        ("flower pot", FLOWER_POT_BLOCK_STATE_ID, 1.375),
        ("potted dandelion", POTTED_DANDELION_BLOCK_STATE_ID, 1.375),
        ("one candle", CANDLE_ONE_DRY_UNLIT_BLOCK_STATE_ID, 1.375),
        ("four candles", CANDLE_FOUR_DRY_UNLIT_BLOCK_STATE_ID, 1.375),
        ("one sea pickle", SEA_PICKLE_ONE_DRY_BLOCK_STATE_ID, 1.375),
        (
            "four sea pickles",
            SEA_PICKLE_FOUR_DRY_BLOCK_STATE_ID,
            1.4375,
        ),
        (
            "pointed dripstone tip up",
            POINTED_DRIPSTONE_TIP_UP_BLOCK_STATE_ID,
            1.6875,
        ),
        (
            "pointed dripstone tip down",
            POINTED_DRIPSTONE_TIP_DOWN_BLOCK_STATE_ID,
            2.0,
        ),
        (
            "pointed dripstone base",
            POINTED_DRIPSTONE_BASE_UP_BLOCK_STATE_ID,
            2.0,
        ),
        ("skeleton skull", SKELETON_SKULL_BLOCK_STATE_ID, 1.5),
        ("piglin head", PIGLIN_HEAD_BLOCK_STATE_ID, 1.5),
        ("one turtle egg", TURTLE_EGG_ONE_BLOCK_STATE_ID, 1.4375),
        ("two turtle eggs", TURTLE_EGG_TWO_BLOCK_STATE_ID, 1.4375),
        ("sniffer egg", SNIFFER_EGG_BLOCK_STATE_ID, 2.0),
        ("dried ghast", DRIED_GHAST_NORTH_DRY_BLOCK_STATE_ID, 1.625),
        (
            "daylight detector",
            DAYLIGHT_DETECTOR_POWER_0_BLOCK_STATE_ID,
            1.375,
        ),
        ("sculk sensor", SCULK_SENSOR_INACTIVE_BLOCK_STATE_ID, 1.5),
        (
            "calibrated sculk sensor",
            CALIBRATED_SCULK_SENSOR_NORTH_INACTIVE_BLOCK_STATE_ID,
            1.5,
        ),
        ("sculk shrieker", SCULK_SHRIEKER_IDLE_BLOCK_STATE_ID, 1.5),
        ("heavy core", HEAVY_CORE_DRY_BLOCK_STATE_ID, 1.5),
        (
            "copper golem statue",
            COPPER_GOLEM_STATUE_STANDING_NORTH_BLOCK_STATE_ID,
            1.875,
        ),
        (
            "up-facing amethyst cluster",
            AMETHYST_CLUSTER_UP_BLOCK_STATE_ID,
            1.4375,
        ),
        (
            "up-facing small amethyst bud",
            SMALL_AMETHYST_BUD_UP_BLOCK_STATE_ID,
            1.1875,
        ),
        (
            "empty end portal frame",
            END_PORTAL_FRAME_EMPTY_NORTH_BLOCK_STATE_ID,
            1.8125,
        ),
        (
            "eye end portal frame",
            END_PORTAL_FRAME_EYE_NORTH_BLOCK_STATE_ID,
            2.0,
        ),
        ("candle cake", CANDLE_CAKE_UNLIT_BLOCK_STATE_ID, 1.875),
        ("oak shelf", OAK_SHELF_NORTH_BLOCK_STATE_ID, 2.0),
        ("decorated pot", DECORATED_POT_NORTH_DRY_BLOCK_STATE_ID, 2.0),
        ("chorus plant", CHORUS_PLANT_NONE_BLOCK_STATE_ID, 1.8125),
        ("chorus flower", CHORUS_FLOWER_AGE_0_BLOCK_STATE_ID, 2.0),
        ("conduit", CONDUIT_DRY_BLOCK_STATE_ID, 1.6875),
        ("azalea", AZALEA_BLOCK_STATE_ID, 2.0),
        ("flowering azalea", FLOWERING_AZALEA_BLOCK_STATE_ID, 2.0),
    ];

    for (name, block_state_id, expected_y) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
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

        assert_f64_near(pose.position.y, expected_y, 0.0005);
        assert!(pose.on_ground, "{name}");
        assert!(!pose.horizontal_collision, "{name}");
    }
}

#[test]
fn local_player_lands_on_big_dripleaf_leaf_collision_shapes() {
    let cases = [
        (
            "none tilt",
            BIG_DRIPLEAF_NORTH_NONE_DRY_BLOCK_STATE_ID,
            1.9375,
        ),
        (
            "partial tilt",
            BIG_DRIPLEAF_NORTH_PARTIAL_DRY_BLOCK_STATE_ID,
            1.8125,
        ),
    ];

    for (name, block_state_id, expected_y) in cases {
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

        assert_f64_near(pose.position.y, expected_y, 0.0005);
        assert!(pose.on_ground, "{name}");
        assert!(!pose.horizontal_collision, "{name}");
    }
}

#[test]
fn local_player_lands_on_up_extended_piston_base_plate() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, PISTON_EXTENDED_UP_BLOCK_STATE_ID);
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

    assert_f64_near(pose.position.y, 1.75, 0.0005);
    assert!(pose.on_ground);
    assert!(!pose.horizontal_collision);
}

#[test]
fn local_player_low_ceiling_does_not_force_crouch_while_flying() {
    let mut world = flat_collision_world();
    apply_flying_abilities(&mut world, 0.05);
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
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    // Vanilla `Player.updatePlayerPose` (`Player.java:342-370`) still downgrades
    // flying players to crouching under a 1.5-block ceiling so they do not clip.
    assert!(pose.sneaking);
    assert!(!pose.swimming);
    assert_f64_near(pose.position.y, 1.0, 0.000001);
}

#[test]
fn local_player_low_tunnel_does_not_force_crawling_while_flying() {
    let mut world = flat_collision_world();
    apply_flying_abilities(&mut world, 0.05);
    set_test_block(&mut world, 0, 2, 0, GRASS_BLOCK_STATE_ID);
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
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    // Vanilla `Player.updatePlayerPose` (`Player.java:342-370`) downgrades flying
    // players in a 1-block tunnel to swimming/crawling pose instead of clipping.
    assert!(!pose.sneaking);
    assert!(pose.swimming);
    assert_f64_near(pose.position.y, 1.0, 0.000001);
}

#[test]
fn local_player_sneak_backs_off_from_near_ground_edge_while_falling() {
    let mut world = single_floor_block_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 1.25),
        fall_distance: 0.2,
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

    assert!(
        pose.position.z <= 1.3005,
        "position was {:?}",
        pose.position
    );
    assert_f64_near(pose.position.y, 1.2, 0.0005);
    assert!(!pose.on_ground);
    assert!(pose.fall_distance < LOCAL_PLAYER_STEP_HEIGHT);
}

#[test]
fn local_player_fall_distance_accumulates_downward_motion_and_resets_on_ground() {
    let mut world = flat_collision_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 2.0, 0.5),
        delta_movement: vec3(0.0, -0.1, 0.0),
        fall_distance: 0.2,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let falling = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(falling.position.y, 1.9, 0.000001);
    assert_f64_near(falling.fall_distance, 0.3, 0.000001);
    assert!(!falling.on_ground);

    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.05, 0.5),
        delta_movement: vec3(0.0, -0.1, 0.0),
        fall_distance: falling.fall_distance,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let landed = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(landed.position.y, 1.0, 0.0005);
    assert!(landed.on_ground);
    assert_f64_near(landed.fall_distance, 0.0, 0.000001);
}

#[test]
fn local_player_sneak_edge_backoff_does_not_apply_while_flying() {
    let mut world = single_floor_block_world();
    apply_flying_abilities(&mut world, 0.05);
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
            0.5,
        )
        .unwrap();

    assert!(pose.position.z > 1.3, "position was {:?}", pose.position);
    assert!(!pose.sneaking);
}

#[test]
fn local_player_flying_hovers_without_gravity() {
    let mut world = flat_collision_world();
    apply_flying_abilities(&mut world, 0.05);
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
            1.0,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 3.0, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_flying_jump_and_sneak_move_vertically() {
    let mut world = flat_collision_world();
    apply_flying_abilities(&mut world, 0.05);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let upward = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                jump: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();
    assert_f64_near(upward.position.y, 3.15, 0.000001);
    assert_f64_near(upward.delta_movement.y, 0.09, 0.000001);
    assert!(!upward.on_ground);

    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });
    let downward = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();
    assert_f64_near(downward.position.y, 2.85, 0.000001);
    assert_f64_near(downward.delta_movement.y, -0.09, 0.000001);
    assert!(!downward.sneaking);
    assert!(!downward.on_ground);
}

#[test]
fn local_player_flying_vertical_momentum_damps_without_input() {
    let mut world = flat_collision_world();
    apply_flying_abilities(&mut world, 0.05);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 0.5),
        delta_movement: vec3(0.0, 0.09, 0.0),
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

    assert_f64_near(pose.position.y, 3.09, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.054, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_flying_uses_abilities_speed_and_sprint_multiplier() {
    let mut world = WorldStore::new();
    apply_flying_abilities(&mut world, 0.1);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 64.0, 0.5),
        on_ground: false,
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

    let expected_step =
        LOCAL_PLAYER_INPUT_SCALE * f64::from(0.1_f32) * LOCAL_INPUT_FLY_SPRINT_SPEED_MULTIPLIER;
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        expected_step * LOCAL_HORIZONTAL_FRICTION,
        0.000001,
    );
    assert_f64_near(pose.position.y, 64.0, 0.000001);
}

#[test]
fn local_player_in_fire_records_private_fire_ticks_and_rain_state() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    set_test_block(&mut world, 0, 1, 0, FIRE_UP_AGE_0_BLOCK_STATE_ID);

    advance_effects_tick_at(&mut world, vec3(0.5, 1.2, 0.5));
    assert!(world.local_player_is_on_fire());

    assert!(
        world
            .entities
            .block_effects_debug_state(123)
            .expect("local player block effect state")
            .remaining_fire_ticks
            > 0
    );
    advance_effects_tick_at(&mut world, vec3(2.5, 1.2, 0.5));
    assert!(!world.local_player_is_on_fire());
}

#[test]
fn local_player_fall_reset_ray_uses_fallback_synchronized_tag_and_player_extras() {
    let mut world = flat_collision_world();
    let pose = LocalPlayerPoseState {
        position: vec3(0.5, 8.0, 0.5),
        fall_distance: 10.0,
        ..LocalPlayerPoseState::default()
    };
    let movement = [0.0, -4.0, 0.0];
    set_test_block(&mut world, 0, 5, 0, LADDER_SOUTH_BLOCK_STATE_ID);
    assert!(world.local_player_fall_reset_ray_hits(pose, movement));

    world.apply_update_tags(ProtocolUpdateTags {
        registries: vec![ProtocolRegistryTags {
            registry: "minecraft:block".to_string(),
            tags: vec![ProtocolTagNetworkPayload {
                tag: "minecraft:fall_damage_resetting".to_string(),
                entries: Vec::new(),
            }],
        }],
    });
    assert!(!world.local_player_fall_reset_ray_hits(pose, movement));

    let ladder_holder_id = world
        .resolved_registry_entry_by_resource_id("minecraft:block", "minecraft:ladder")
        .expect("vanilla ladder block holder")
        .holder_id;
    world.apply_update_tags(ProtocolUpdateTags {
        registries: vec![ProtocolRegistryTags {
            registry: "minecraft:block".to_string(),
            tags: vec![ProtocolTagNetworkPayload {
                tag: "minecraft:fall_damage_resetting".to_string(),
                entries: vec![ladder_holder_id],
            }],
        }],
    });
    assert!(world.local_player_fall_reset_ray_hits(pose, movement));

    set_test_block(&mut world, 0, 5, 0, END_PORTAL_BLOCK_STATE_ID);
    world.apply_update_tags(ProtocolUpdateTags {
        registries: vec![ProtocolRegistryTags {
            registry: "minecraft:block".to_string(),
            tags: vec![ProtocolTagNetworkPayload {
                tag: "minecraft:fall_damage_resetting".to_string(),
                entries: Vec::new(),
            }],
        }],
    });
    assert!(world.local_player_fall_reset_ray_hits(pose, movement));

    set_test_block(&mut world, 0, 5, 0, SOURCE_WATER_BLOCK_STATE_ID);
    assert!(world.local_player_fall_reset_ray_hits(pose, movement));
}

#[test]
fn local_player_low_food_prevents_sprint_speed_unless_mayfly() {
    let mut survival_world = flat_collision_world();
    apply_player_health(&mut survival_world, 6);
    survival_world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let survival_pose = survival_world
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
    let walk_step = first_ground_step(TEST_DEFAULT_MOVEMENT_SPEED, 1.0);
    assert_f64_near(survival_pose.position.z, 0.5 + walk_step, 0.000001);
    assert_f64_near(
        survival_pose.delta_movement.z,
        carried_after_ground_step(walk_step),
        0.000001,
    );

    let mut creative_world = flat_collision_world();
    apply_player_health(&mut creative_world, 6);
    creative_world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: false,
        flying: false,
        can_fly: true,
        instabuild: false,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    creative_world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let creative_pose = creative_world
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
    let sprint_step = first_ground_step(
        TEST_DEFAULT_MOVEMENT_SPEED * LOCAL_PLAYER_SPRINT_SPEED_MODIFIER,
        1.0,
    );
    assert_f64_near(creative_pose.position.z, 0.5 + sprint_step, 0.000001);
    assert_f64_near(
        creative_pose.delta_movement.z,
        carried_after_ground_step(sprint_step),
        0.000001,
    );
}

#[test]
fn local_player_jump_uses_jump_strength_attribute_and_jump_boost_effect() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID,
            base: 0.5,
            modifiers: Vec::new(),
        }],
    }));
    assert!(world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_JUMP_BOOST_ID, 1,)));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
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

    assert_f64_near(pose.position.y, 1.7, 0.000001);
    assert!(!pose.on_ground);
    assert!(pose.delta_movement.y > 0.0);
}

#[test]
fn local_player_held_jump_arms_vanilla_ten_tick_lockout() {
    let mut world = flat_collision_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    let held_jump = LocalPlayerInputState {
        focused: true,
        jump: true,
        ..LocalPlayerInputState::default()
    };

    let jumped = world
        .advance_local_player_input(held_jump, LOCAL_PHYSICS_TICK_SECONDS)
        .unwrap();
    assert!(jumped.delta_movement.y > 0.0);
    assert_eq!(jumped.no_jump_delay_ticks, 10);

    // Landing again while the input is still held stays grounded until the
    // lockout expires: the gate sees the decremented counter, not a jump.
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        no_jump_delay_ticks: jumped.no_jump_delay_ticks,
        ..LocalPlayerPoseState::default()
    });
    let locked = world
        .advance_local_player_input(held_jump, LOCAL_PHYSICS_TICK_SECONDS)
        .unwrap();
    assert_f64_near(locked.position.y, 1.0, 0.000001);
    assert!(locked.on_ground);
    assert_eq!(locked.no_jump_delay_ticks, 9);
}

#[test]
fn local_player_jump_lockout_decrements_then_gates_within_the_same_tick() {
    let mut world = flat_collision_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        no_jump_delay_ticks: 1,
        ..LocalPlayerPoseState::default()
    });

    // Vanilla decrements `noJumpDelay` at the top of `aiStep`, so a counter
    // of 1 reaches 0 and re-jumps on this very tick (10-tick auto cadence).
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

    assert!(pose.delta_movement.y > 0.0);
    assert!(!pose.on_ground);
    assert_eq!(pose.no_jump_delay_ticks, 10);
}

#[test]
fn local_player_releasing_jump_clears_the_lockout_immediately() {
    let mut world = flat_collision_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        no_jump_delay_ticks: 7,
        ..LocalPlayerPoseState::default()
    });

    let released = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();
    assert_eq!(released.no_jump_delay_ticks, 0);

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
    assert!(pose.delta_movement.y > 0.0);
}

#[test]
fn local_player_sprint_jump_adds_vanilla_horizontal_impulse() {
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
                jump: true,
                sprinting: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_z =
        0.5 + first_ground_step(
            TEST_DEFAULT_MOVEMENT_SPEED * LOCAL_PLAYER_SPRINT_SPEED_MODIFIER,
            1.0,
        ) + SPRINT_JUMP_HORIZONTAL_IMPULSE;
    assert_f64_near(
        pose.position.y,
        1.0 + LOCAL_INPUT_DEFAULT_JUMP_STRENGTH_ATTRIBUTE,
        0.000001,
    );
    assert_f64_near(pose.position.z, expected_z, 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        carried_after_ground_step(expected_z - 0.5),
        0.000001,
    );
    assert!(!pose.on_ground);
}

#[test]
fn local_player_ineligible_sprint_jump_does_not_add_horizontal_impulse() {
    let mut world = flat_collision_world();
    apply_player_health(&mut world, 6);
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
                jump: true,
                sprinting: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_z = 0.5 + first_ground_step(TEST_DEFAULT_MOVEMENT_SPEED, 1.0);
    assert_f64_near(
        pose.position.y,
        1.0 + LOCAL_INPUT_DEFAULT_JUMP_STRENGTH_ATTRIBUTE,
        0.000001,
    );
    assert_f64_near(pose.position.z, expected_z, 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        carried_after_ground_step(expected_z - 0.5),
        0.000001,
    );
    assert!(!pose.on_ground);
}

#[test]
fn local_player_jump_starts_only_from_ground() {
    let mut world = flat_collision_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let jump_pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                jump: true,
                ..LocalPlayerInputState::default()
            },
            0.05,
        )
        .unwrap();

    assert!(
        jump_pose.position.y > 1.0,
        "position was {:?}",
        jump_pose.position
    );
    assert!(!jump_pose.on_ground);
    assert!(jump_pose.delta_movement.y > 0.0);

    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });
    let airborne_pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                jump: true,
                ..LocalPlayerInputState::default()
            },
            0.1,
        )
        .unwrap();

    assert!(
        airborne_pose.position.y < 3.0,
        "position was {:?}",
        airborne_pose.position
    );
    assert!(!airborne_pose.on_ground);
}

#[test]
fn local_player_fall_flying_level_glide_uses_lift_and_forward_conversion() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(apply_fall_flying(&mut world, 123));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 8.0, 0.5),
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

    // `updateFallFlyingMovement` at yaw 0 / pitch 0 from rest:
    // y = 0.08 * (-1 + 0.75) = -0.02, forward conversion 0.002 puts
    // y at -0.018 and z at 0.002, the look steer pulls z back by 0.0002,
    // then the drag multiply (0.99, 0.98, 0.99).
    assert_f64_near(pose.delta_movement.y, -0.018 * 0.98, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.0018 * 0.99, 0.000001);
    assert_f64_near(pose.position.y, 8.0 - 0.018 * 0.98, 0.000001);
    assert_f64_near(pose.position.z, 0.5 + 0.0018 * 0.99, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_fall_flying_uses_shared_entity_collision_ordering() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(apply_fall_flying(&mut world, 123));
    world.apply_add_entity(ProtocolAddEntity {
        id: 456,
        uuid: Uuid::from_u128(0x45645678456456784564567845645683),
        entity_type_id: VANILLA_ENTITY_TYPE_SHULKER_ID,
        position: vec3(0.5, 1.0, 2.0),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        delta_movement: vec3(0.0, 0.0, 2.0),
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

    assert_f64_near(pose.position.z, 1.7, 0.000001);
    assert!(pose.horizontal_collision);
    assert_f64_near(pose.delta_movement.z, 0.0, 0.000001);
}

#[test]
fn local_player_fall_flying_looking_up_converts_speed_to_climb() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(apply_fall_flying(&mut world, 123));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 8.0, 0.5),
        x_rot: -30.0,
        delta_movement: vec3(0.0, 0.0, 1.0),
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

    // Pitch -30°: lift = cos² = 0.75, so y = 0.08 * (-1 + 0.75 * 0.75)
    // = -0.035; the downward conversion adds 0.002625 to y and z; the climb
    // conversion drains 0.02 from z into y * 3.2; the look steer adds
    // (1 - 0.982625) * 0.1 back to z; then the drag multiply.
    assert_f64_near(pose.delta_movement.y, 0.031625 * 0.98, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.9843625 * 0.99, 0.000001);
    assert!(pose.delta_movement.y > 0.0);
    assert!(pose.delta_movement.z < 1.0);
}

#[test]
fn local_player_fall_flying_landing_zeroes_vertical_and_keeps_server_flag() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(apply_fall_flying(&mut world, 123));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.6, 0.5),
        delta_movement: vec3(0.0, -0.5, 0.0),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let mut pose = None;
    for _ in 0..4 {
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

    // The `Entity.move` landing zeroes the vertical delta; clearing the glide
    // flag on touchdown is `LivingEntity.updateFallFlying` server logic, so
    // the local flag stays until the server re-syncs it.
    assert!(pose.on_ground);
    assert!(pose.position.y < 1.1);
    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
    assert_eq!(world.entities.is_fall_flying(123), Some(true));
}
