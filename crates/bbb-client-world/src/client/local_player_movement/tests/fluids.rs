use super::*;

#[test]
fn local_player_fluid_movement_uses_shared_entity_collision_ordering() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    world.apply_add_entity(ProtocolAddEntity {
        id: 456,
        uuid: Uuid::from_u128(0x45645678456456784564567845645681),
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

    assert_f64_near(pose.position.z, 1.7, 0.000001);
    assert!(pose.horizontal_collision);
    assert_f64_near(pose.delta_movement.z, 0.0, 0.000001);
}

#[test]
fn local_player_low_tunnel_forces_crawling_swimming_pose() {
    let mut world = flat_collision_world();
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
    assert!(!pose.sneaking);
    assert!(pose.swimming);
    assert!(!pose.horizontal_collision);
    assert_f64_near(world.local_player_dimensions(pose).height, 0.6, 0.000001);
    assert_f64_near(world.local_player_eye_height(pose), 0.4, 0.000001);
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        carried_after_ground_step(expected_step),
        0.000001,
    );
}

#[test]
fn local_player_underwater_sprint_uses_swimming_pose() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 2, 0, SOURCE_WATER_BLOCK_STATE_ID);
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
                sprinting: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert!(!pose.sneaking);
    assert!(pose.swimming);
    assert_f64_near(world.local_player_dimensions(pose).height, 0.6, 0.000001);
    assert_f64_near(world.local_player_eye_height(pose), 0.4, 0.000001);
}

#[test]
fn local_player_fall_distance_resets_when_touching_water_surface() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.95, 0.5),
        delta_movement: vec3(0.0, -0.1, 0.0),
        fall_distance: 0.4,
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

    assert!(!pose.on_ground);
    assert_f64_near(pose.position.y, 1.85, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
}

#[test]
fn local_player_fall_distance_does_not_reset_above_low_flowing_water_surface() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.7, 0.5),
        delta_movement: vec3(0.0, -0.1, 0.0),
        fall_distance: 0.4,
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

    assert!(!pose.on_ground);
    assert_f64_near(pose.position.y, 1.6, 0.000001);
    assert_f64_near(pose.fall_distance, 0.5, 0.000001);
}

#[test]
fn local_player_in_water_uses_fluid_relative_acceleration_and_drag() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        fall_distance: 0.4,
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

    assert_f64_near(pose.position.z, 0.5 + first_fluid_step(0.02), 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        first_fluid_step(0.02) * 0.8,
        0.000001,
    );
    assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_in_flowing_water_applies_vanilla_current_before_drag() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 1, FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
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

    assert_f64_near(pose.position.z, 0.514, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.0112, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
}

#[test]
fn local_player_water_movement_efficiency_half_applies_when_airborne() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_WATER_MOVEMENT_EFFICIENCY_ID,
            base: 1.0,
            modifiers: Vec::new(),
        }],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
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

    assert_f64_near(pose.position.z, 0.5 + first_fluid_step(0.06), 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        first_fluid_step(0.06) * 0.67300003,
        0.000001,
    );
    assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
}

#[test]
fn local_player_water_movement_efficiency_fully_applies_on_ground() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_WATER_MOVEMENT_EFFICIENCY_ID,
            base: 1.0,
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

    assert_f64_near(pose.position.z, 0.5 + first_fluid_step(0.1), 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        first_fluid_step(0.1) * 0.54600006,
        0.000001,
    );
    assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
}

#[test]
fn local_player_dolphins_grace_overrides_water_horizontal_drag() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_mob_effect(mob_effect(
        123,
        VANILLA_MOB_EFFECT_DOLPHINS_GRACE_ID,
        0,
    )));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
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

    assert_f64_near(pose.position.z, 0.5 + first_fluid_step(0.02), 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        first_fluid_step(0.02) * 0.96,
        0.000001,
    );
    assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
}

#[test]
fn local_player_no_gravity_metadata_suppresses_water_falling_gravity() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(apply_no_gravity(&mut world, 123, true));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
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

    assert_f64_near(pose.position.z, 0.5 + first_fluid_step(0.02), 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        first_fluid_step(0.02) * 0.8,
        0.000001,
    );
    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
}

#[test]
fn local_player_bad_omen_does_not_apply_dolphins_grace_water_drag() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_BAD_OMEN_ID, 0,)));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
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

    assert_f64_near(pose.position.z, 0.5 + first_fluid_step(0.02), 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        first_fluid_step(0.02) * 0.8,
        0.000001,
    );
    assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
}

#[test]
fn local_player_low_food_prevents_water_sprint_and_swimming_pose() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    apply_player_health(&mut world, 6);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
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

    assert!(!pose.swimming);
    assert_f64_near(pose.position.z, 0.5 + first_fluid_step(0.02), 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        first_fluid_step(0.02) * 0.8,
        0.000001,
    );
    assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
}

#[test]
fn local_player_swimming_pitch_down_pulls_velocity_toward_look_y() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 2, 0, SOURCE_WATER_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        x_rot: 30.0,
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

    assert!(pose.swimming);
    assert_f64_near(pose.position.y, 1.0575, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.034, 0.000001);
    assert_f64_near(pose.position.z, 0.5 + first_fluid_step(0.02), 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        first_fluid_step(0.02) * 0.9,
        0.000001,
    );
}

#[test]
fn local_player_swimming_pitch_up_rises_when_fluid_above() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 2, 0, SOURCE_WATER_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        x_rot: -30.0,
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

    assert!(pose.swimming);
    assert_f64_near(pose.position.y, 1.13, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.024, 0.000001);
}

#[test]
fn local_player_swimming_pitch_up_near_surface_does_not_auto_rise() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        x_rot: -30.0,
        swimming: true,
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

    assert!(pose.swimming);
    assert_f64_near(pose.position.y, 1.1, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
}

#[test]
fn local_player_jump_and_sneak_in_water_apply_liquid_vertical_impulses() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
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

    assert_f64_near(upward.position.y, 1.14, 0.000001);
    assert_f64_near(upward.delta_movement.y, 0.027, 0.000001);

    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
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

    assert_f64_near(downward.position.y, 1.06, 0.000001);
    assert_f64_near(downward.delta_movement.y, -0.037, 0.000001);
}

#[test]
fn local_player_inside_upward_bubble_column_applies_vanilla_push() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    set_test_block(&mut world, 0, 1, 0, bubble_column_block_state_id(false));
    set_test_block(&mut world, 0, 2, 0, SOURCE_WATER_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
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

    assert_f64_near(pose.delta_movement.y, 0.055, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
}

#[test]
fn local_player_above_upward_bubble_column_uses_surface_push() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    set_test_block(&mut world, 0, 1, 0, bubble_column_block_state_id(false));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
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

    assert_f64_near(pose.delta_movement.y, 0.095, 0.000001);
}

#[test]
fn local_player_drag_down_bubble_column_applies_downward_velocity() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    set_test_block(&mut world, 0, 1, 0, bubble_column_block_state_id(true));
    set_test_block(&mut world, 0, 2, 0, SOURCE_WATER_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
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

    assert_f64_near(pose.delta_movement.y, -0.035, 0.000001);
}

#[test]
fn local_player_flying_ignores_bubble_column_push() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, bubble_column_block_state_id(false));
    apply_flying_abilities(&mut world, 0.05);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
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

    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
}

#[test]
fn local_player_in_water_jumps_out_when_horizontal_collision_has_clear_space() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 1, GRASS_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.45, 0.69),
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
    assert_f64_near(
        pose.delta_movement.y,
        LOCAL_INPUT_FLUID_JUMP_OUT_VELOCITY_PER_TICK,
        0.000001,
    );
}

#[test]
fn local_player_in_water_does_not_jump_out_when_clearance_still_contains_fluid() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 1, GRASS_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.69),
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
    assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
}

#[test]
fn local_player_in_lava_jumps_out_when_horizontal_collision_has_clear_space() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 1, GRASS_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.45, 0.69),
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
    assert_f64_near(
        pose.delta_movement.y,
        LOCAL_INPUT_FLUID_JUMP_OUT_VELOCITY_PER_TICK,
        0.000001,
    );
}

#[test]
fn local_player_in_lava_uses_lava_drag_and_gravity() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
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

    assert_f64_near(pose.position.z, 0.5 + first_fluid_step(0.02), 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        first_fluid_step(0.02) * 0.5,
        0.000001,
    );
    assert_f64_near(pose.delta_movement.y, -0.02, 0.000001);
}

#[test]
fn local_player_no_gravity_metadata_suppresses_lava_gravity() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(apply_no_gravity(&mut world, 123, true));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
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

    assert_f64_near(pose.position.z, 0.5 + first_fluid_step(0.02), 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        first_fluid_step(0.02) * 0.5,
        0.000001,
    );
    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
}

#[test]
fn local_player_in_lava_ignores_water_movement_efficiency() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_WATER_MOVEMENT_EFFICIENCY_ID,
            base: 1.0,
            modifiers: Vec::new(),
        }],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
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

    assert_f64_near(pose.position.z, 0.5 + first_fluid_step(0.02), 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        first_fluid_step(0.02) * 0.5,
        0.000001,
    );
    assert_f64_near(pose.delta_movement.y, -0.02, 0.000001);
}

#[test]
fn local_player_in_lava_current_uses_vanilla_minimum_push_before_drag() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 1, FLOWING_LAVA_LEVEL_3_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
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

    assert_f64_near(pose.position.z, 0.5045, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.00225, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.02, 0.000001);
}

#[test]
fn local_player_in_lava_current_skips_minimum_push_when_already_moving() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 1, FLOWING_LAVA_LEVEL_3_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        delta_movement: vec3(0.01, 0.0, 0.0),
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

    assert_f64_near(pose.position.x, 0.51, 0.000001);
    assert_f64_near(pose.position.z, 0.5023333333333333, 0.000001);
    assert_f64_near(pose.delta_movement.x, 0.005, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.0011666666666666668, 0.000001);
}

#[test]
fn local_player_in_nether_lava_current_uses_fast_lava_push() {
    let mut world = flat_collision_world();
    world.level = Some(WorldLevelInfo {
        dimension: "minecraft:the_nether".to_string(),
        dimension_type_id: 1,
        dimension_type_name: Some("minecraft:the_nether".to_string()),
        dimension_type_properties: crate::WorldDimensionTypeProperties::default(),
        biome_zoom_seed: 0,
        default_clock_resource_id: None,
        timeline_resource_ids: Vec::new(),
        has_sky_light: false,
        has_ceiling: true,
        cardinal_lighting: WorldCardinalLighting::Nether,
        skybox: crate::WorldSkybox::None,
        ambient_light: 0.1,
        environment_attributes: crate::ClientEnvironmentAttributes::default(),
        last_death_location: None,
        sea_level: 32,
        is_debug: false,
        is_flat: false,
    });
    set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 1, FLOWING_LAVA_LEVEL_3_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
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

    assert_f64_near(pose.position.z, 0.507, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.0035, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.02, 0.000001);

    world.record_registry_entries(
        crate::DIMENSION_TYPE_REGISTRY,
        0,
        vec![crate::RegistryPacketEntry::stub("minecraft:the_nether")],
    );
    world
        .level
        .as_mut()
        .unwrap()
        .environment_attributes
        .fast_lava = Some(false);
    assert_eq!(
        world
            .sampled_client_environment_attributes()
            .unwrap()
            .fast_lava,
        Some(false)
    );
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
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
    assert_f64_near(pose.position.z, 0.5045, 0.000001);
}

#[test]
fn local_player_flying_in_water_uses_flying_movement_not_fluid_travel() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    apply_flying_abilities(&mut world, 0.05);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
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

    let expected_step = LOCAL_PLAYER_INPUT_SCALE * f64::from(0.05_f32);
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        expected_step * LOCAL_HORIZONTAL_FRICTION,
        0.000001,
    );
    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
}

#[test]
fn local_player_flying_horizontal_velocity_accumulates_and_drags() {
    let mut world = WorldStore::new();
    apply_flying_abilities(&mut world, 0.05);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 64.0, 0.5),
        on_ground: false,
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
    let acceleration = LOCAL_PLAYER_INPUT_SCALE * f64::from(0.05_f32);
    assert_f64_near(first.position.z, 0.5 + acceleration, 0.000001);
    assert_f64_near(
        first.delta_movement.z,
        acceleration * LOCAL_HORIZONTAL_FRICTION,
        0.000001,
    );

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
    let second_step = first.delta_movement.z + acceleration;
    assert_f64_near(
        second.position.z,
        0.5 + acceleration + second_step,
        0.000001,
    );
    assert_f64_near(
        second.delta_movement.z,
        second_step * LOCAL_HORIZONTAL_FRICTION,
        0.000001,
    );
}

#[test]
fn local_player_block_speed_factor_does_not_fallback_through_water() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, SOUL_SAND_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    let factor = local_player_block_speed_factor(
        &world,
        LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            ..LocalPlayerPoseState::default()
        },
    );

    assert_f64_near(factor, DEFAULT_BLOCK_SPEED_FACTOR, 0.000001);
}

#[test]
fn local_player_block_speed_factor_does_not_fallback_through_bubble_column() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, SOUL_SAND_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 0, bubble_column_block_state_id(false));
    let factor = local_player_block_speed_factor(
        &world,
        LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            ..LocalPlayerPoseState::default()
        },
    );

    assert_f64_near(factor, DEFAULT_BLOCK_SPEED_FACTOR, 0.000001);
}

#[test]
fn local_player_fall_reset_long_move_runs_in_air_fluid_and_fall_flying_paths() {
    for path in ["air", "lava", "fall_flying"] {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 5, 0, LADDER_SOUTH_BLOCK_STATE_ID);
        if path == "lava" {
            set_test_block(&mut world, 0, 8, 0, SOURCE_LAVA_BLOCK_STATE_ID);
        }
        if path == "fall_flying" {
            attach_local_player_entity(&mut world, 123);
            assert!(apply_fall_flying(&mut world, 123));
        }
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 8.0, 0.5),
            delta_movement: vec3(0.0, -4.0, 0.0),
            fall_distance: 10.0,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState::default(),
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        let current_move_fall_distance = 8.0 - pose.position.y;
        assert_f64_near(pose.fall_distance, current_move_fall_distance, 0.000001);
        assert!(pose.fall_distance < 8.0, "{path} pose {pose:?}");
    }
}

#[test]
fn local_player_sprint_start_rejects_shallow_water_slow_item_and_blindness() {
    let input = LocalPlayerInputState {
        focused: true,
        forward: true,
        ..LocalPlayerInputState::default()
    };

    let mut shallow = flat_collision_world();
    set_test_block(&mut shallow, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    shallow.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        ..LocalPlayerPoseState::default()
    });
    assert!(!shallow.reconcile_local_player_sprinting(input, true));

    let mut using_item = flat_collision_world();
    using_item.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    using_item.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    using_item.set_local_using_item(true);
    assert!(!using_item.reconcile_local_player_sprinting(input, true));

    let mut blind = flat_collision_world();
    attach_local_player_entity(&mut blind, 123);
    blind.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    assert!(blind.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_BLINDNESS_ID, 0,)));
    assert!(!blind.reconcile_local_player_sprinting(input, true));
}

#[test]
fn local_player_sprint_start_allows_underwater_but_rejects_dry_crawling_and_fall_flying() {
    let input = LocalPlayerInputState {
        focused: true,
        forward: true,
        ..LocalPlayerInputState::default()
    };

    let mut underwater = flat_collision_world();
    set_test_block(&mut underwater, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    set_test_block(&mut underwater, 0, 2, 0, SOURCE_WATER_BLOCK_STATE_ID);
    underwater.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        swimming: true,
        ..LocalPlayerPoseState::default()
    });
    assert!(underwater.reconcile_local_player_sprinting(input, true));

    let mut crawling = flat_collision_world();
    crawling.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        swimming: true,
        ..LocalPlayerPoseState::default()
    });
    assert!(!crawling.reconcile_local_player_sprinting(input, true));

    let mut fall_flying = flat_collision_world();
    attach_local_player_entity(&mut fall_flying, 123);
    fall_flying.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 4.0, 0.5),
        ..LocalPlayerPoseState::default()
    });
    assert!(apply_fall_flying(&mut fall_flying, 123));
    assert!(!fall_flying.reconcile_local_player_sprinting(input, true));
}

#[test]
fn local_player_in_water_ladder_collision_applies_climb_impulse() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, LADDER_SOUTH_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 2, 0, SOURCE_WATER_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 1, GRASS_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 2, 1, GRASS_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 3, 1, GRASS_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.69),
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

    // `travelInWater`: the horizontal collision on a climbable rebuilds the
    // vertical delta to 0.2 before the 0.8 water drag and the falling
    // adjustment (gravity / 16).
    assert!(pose.horizontal_collision);
    assert_f64_near(
        pose.delta_movement.y,
        LOCAL_PLAYER_CLIMBABLE_UPWARD_VELOCITY_PER_TICK * 0.8 - 0.005,
        0.000001,
    );
}

#[test]
fn local_player_walking_on_water_bottom_steps_up_slab() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 2, 0, SOURCE_WATER_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 1, OAK_BOTTOM_SLAB_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 2, 1, SOURCE_WATER_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    // `Entity.move` steps up while grounded inside fluids too: walking along
    // the water bottom into a half slab climbs it instead of stopping.
    let mut pose = None;
    for _ in 0..12 {
        pose = Some(
            world
                .advance_local_player_input(
                    LocalPlayerInputState {
                        focused: true,
                        forward: true,
                        ..LocalPlayerInputState::default()
                    },
                    LOCAL_PHYSICS_TICK_SECONDS,
                )
                .unwrap(),
        );
    }
    let pose = pose.unwrap();
    assert!(
        pose.position.z > 1.0 && pose.position.y >= 1.5,
        "expected to step onto the slab, got {:?}",
        pose.position
    );
}

#[test]
fn local_player_soul_sand_speed_factor_applies_under_water() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, SOUL_SAND_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
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

    // `Entity.move` multiplies the carried delta by the block speed factor
    // inside fluids as well; the water drag then applies on top.
    assert_f64_near(pose.position.z, 0.5 + first_fluid_step(0.02), 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        first_fluid_step(0.02) * 0.4 * 0.8,
        0.000001,
    );
}

#[test]
fn shallow_water_jump_uses_jump_from_ground_and_depth_strider_uses_sprint_speed() {
    let mut world = flat_collision_world();
    // Level 6 flowing water has height 2/9 (~0.222) <= 0.4 fluid jump threshold.
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID + 6);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    let jumped = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                jump: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();
    assert!(
        jumped.position.y > 1.3,
        "shallow grounded water should call jumpFromGround, got y={}",
        jumped.position.y
    );
    assert_eq!(jumped.no_jump_delay_ticks, LOCAL_PLAYER_JUMP_DELAY_TICKS);

    // Depth Strider with sprint should apply sprint's 1.3x ground speed multiplier.
    let mut sprint_world = flat_collision_world();
    attach_local_player_entity(&mut sprint_world, 77);
    sprint_world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 77,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_WATER_MOVEMENT_EFFICIENCY_ID,
            base: 1.0,
            modifiers: Vec::new(),
        }],
    });
    for y in 1..=3 {
        set_test_block(&mut sprint_world, 0, y, 0, SOURCE_WATER_BLOCK_STATE_ID);
    }
    sprint_world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    let walk = sprint_world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();
    sprint_world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    let sprint = sprint_world
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
    assert!(
        sprint.position.z > walk.position.z + 0.02,
        "Depth Strider sprint step {} should exceed walk step {}",
        sprint.position.z,
        walk.position.z
    );
}

#[test]
fn unloaded_chunk_clamps_falling_speed_and_ceiling_bump_clears_on_ground() {
    let mut world = flat_collision_world();
    // Chunk (4, 0) is unloaded while chunk (0, 0) is loaded.
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(64.5, 10.0, 0.5),
        delta_movement: vec3(0.0, -2.0, 0.0),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });
    let above_min = world
        .advance_local_player_input(LocalPlayerInputState::default(), LOCAL_PHYSICS_TICK_SECONDS)
        .unwrap();
    assert_f64_near(above_min.delta_movement.y, -0.1 * 0.98, 0.000001);

    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(64.5, f64::from(world.dimension.min_y) - 1.0, 0.5),
        delta_movement: vec3(0.0, -2.0, 0.0),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });
    let below_min = world
        .advance_local_player_input(LocalPlayerInputState::default(), LOCAL_PHYSICS_TICK_SECONDS)
        .unwrap();
    assert_f64_near(below_min.delta_movement.y, 0.0, 0.000001);

    // Ceiling bump while jumping with feet still at floor clears on_ground.
    set_test_block(&mut world, 0, 3, 0, GRASS_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        delta_movement: vec3(0.0, 0.42, 0.0),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    let bumped = world
        .advance_local_player_input(LocalPlayerInputState::default(), LOCAL_PHYSICS_TICK_SECONDS)
        .unwrap();
    assert!(!bumped.on_ground);
}

#[test]
fn velocity_deadzone_and_slow_falling_reset_fall_distance() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 88);
    world.apply_update_mob_effect(ProtocolUpdateMobEffect {
        entity_id: 88,
        effect_id: VANILLA_MOB_EFFECT_SLOW_FALLING_ID,
        amplifier: 0,
        duration_ticks: 200,
        flags: ProtocolMobEffectFlags::default(),
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 10.0, 0.5),
        delta_movement: vec3(0.001, -0.002, 0.001),
        fall_distance: 12.0,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });
    let pose = world
        .advance_local_player_input(LocalPlayerInputState::default(), LOCAL_PHYSICS_TICK_SECONDS)
        .unwrap();
    // Horizontal tiny velocity < sqrt(9e-6) and vertical |y| < 0.003 are zeroed before travel.
    assert_f64_near(pose.position.x, 0.5, 0.000001);
    assert_f64_near(pose.position.z, 0.5, 0.000001);
    assert_f64_near(pose.position.y, 10.0, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
}
