use super::*;

#[test]
fn particle_y_collision_clips_downward_motion_to_shape_top() {
    let bounds = particle_collision_bounds([0.5, 1.05, 0.5], 0.1, 0.2);
    let clipped = clipped_particle_axis_against_box(
        bounds,
        CollisionAxis::Y,
        -0.2,
        -0.2,
        BlockPos { x: 0, y: 0, z: 0 },
        WorldCollisionBox::FULL,
    );

    assert_f64_near(clipped, -0.05);
}

#[test]
fn particle_falls_inside_a_cauldron_but_cannot_cross_its_wall() {
    let mut world = empty_world();
    let cauldron = vanilla_block_state_id("minecraft:cauldron", []);
    set_block(&mut world, BlockPos { x: 0, y: 0, z: 3 }, cauldron);

    assert_eq!(
        world.clip_particle_collision_movement([0.5, 1.1, 3.5], [0.0, -0.5, 0.0], 0.05, 0.1),
        [0.0, -0.5, 0.0]
    );
    let clipped =
        world.clip_particle_collision_movement([-0.2, 0.6, 3.5], [0.4, 0.0, 0.0], 0.05, 0.1);
    assert_f64_near(clipped[0], 0.15);
}

#[test]
fn entity_collision_clip_accepts_catalog_relative_bounds() {
    let mut world = empty_world();
    let grass = vanilla_block_state_id("minecraft:grass_block", [("snowy", "false")]);
    set_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, grass);

    let clipped = world.clip_entity_collision_movement(
        [0.5, 1.1, 0.5],
        [0.0, -0.2, 0.0],
        [-0.49, 0.0, -0.49],
        [0.49, 0.98, 0.49],
    );

    assert_f64_near(clipped[0], 0.0);
    assert_f64_near(clipped[1], -0.1);
    assert_f64_near(clipped[2], 0.0);
}

#[test]
fn boat_ground_friction_samples_the_thin_shape_slice_below_the_hull() {
    let mut world = empty_world();
    let stone = vanilla_block_state_id("minecraft:stone", []);
    set_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, stone);

    let friction =
        world.boat_ground_friction([0.5, 1.0, 0.5], [-0.49, 0.0, -0.49], [0.49, 0.5625, 0.49]);

    assert!((friction - 0.6).abs() < f32::EPSILON);
    assert!(world
        .boat_ground_friction([2.5, 1.0, 2.5], [-0.49, 0.0, -0.49], [0.49, 0.5625, 0.49],)
        .is_nan());
}

#[test]
fn projectile_block_clip_stops_at_the_first_collision_shape_surface() {
    let mut world = empty_world();
    let stone = vanilla_block_state_id("minecraft:stone", []);
    set_block(&mut world, BlockPos { x: 1, y: 1, z: 0 }, stone);

    let clipped = world.clip_projectile_block_movement(
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
        [0.5, 1.5, 0.5],
        [2.0, 0.0, 0.0],
        false,
    );

    assert_f64_near(clipped.movement[0], 0.5);
    assert_f64_near(clipped.movement[1], 0.0);
    assert_f64_near(clipped.movement[2], 0.0);
    assert!(!clipped.world_border_hit);
    assert_eq!(clipped.block_state_id, Some(stone));
}

#[test]
fn camera_visual_block_clip_stops_at_the_nearest_visual_surface() {
    let mut world = empty_world();
    world.set_local_player_pose(LocalPlayerPoseState::default());
    let stone = vanilla_block_state_id("minecraft:stone", []);
    set_block(&mut world, BlockPos { x: 1, y: 1, z: 0 }, stone);

    let hit = world
        .camera_visual_block_hit([0.5, 1.5, 0.5], [2.5, 1.5, 0.5])
        .expect("stone visual hit");

    assert_f64_near(hit[0], 1.0);
    assert_f64_near(hit[1], 1.5);
    assert_f64_near(hit[2], 0.5);
}

#[test]
fn projectile_block_clip_uses_vanilla_inside_shape_test_point() {
    let mut world = empty_world();
    let stone = vanilla_block_state_id("minecraft:stone", []);
    set_block(&mut world, BlockPos { x: 0, y: 1, z: 0 }, stone);

    let clipped = world.clip_projectile_block_movement(
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
        [0.5, 1.5, 0.5],
        [2.0, 0.0, 0.0],
        false,
    );

    assert_f64_near(clipped.movement[0], 0.002);
    assert_f64_near(clipped.movement[1], 0.0);
    assert_f64_near(clipped.movement[2], 0.0);
    assert!(!clipped.world_border_hit);
    assert_eq!(clipped.block_state_id, Some(stone));
    assert_eq!(
        world.projectile_position_inside_block_collision(
            VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
            [0.5, 1.5, 0.5],
            false,
        ),
        Some(stone),
    );
    assert_eq!(
        world.projectile_position_inside_block_collision(
            VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
            [1.5, 1.5, 0.5],
            false,
        ),
        None,
    );
}

#[test]
fn projectile_block_clip_includes_the_world_border() {
    let mut world = empty_world();
    world.apply_set_border_size(ProtocolSetBorderSize { size: 4.0 });

    let clipped = world.clip_projectile_block_movement(
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
        [0.0, 1.5, 0.0],
        [10.0, 0.0, 0.0],
        false,
    );

    assert_f64_near(clipped.movement[0], 2.0 - f64::from(1.0e-5_f32));
    assert_f64_near(clipped.movement[1], 0.0);
    assert_f64_near(clipped.movement[2], 0.0);
    assert!(clipped.world_border_hit);
    assert_eq!(clipped.block_state_id, None);
}

#[test]
fn particle_y_collision_ignores_boxes_outside_horizontal_overlap() {
    let bounds = particle_collision_bounds([1.5, 1.05, 0.5], 0.1, 0.2);
    let clipped = clipped_particle_axis_against_box(
        bounds,
        CollisionAxis::Y,
        -0.2,
        -0.2,
        BlockPos { x: 0, y: 0, z: 0 },
        WorldCollisionBox::FULL,
    );

    assert_f64_near(clipped, -0.2);
}

#[test]
fn particle_y_collision_clips_upward_motion_to_shape_bottom() {
    let bounds = particle_collision_bounds([0.5, -0.25, 0.5], 0.1, 0.2);
    let clipped = clipped_particle_axis_against_box(
        bounds,
        CollisionAxis::Y,
        0.2,
        0.2,
        BlockPos { x: 0, y: 0, z: 0 },
        WorldCollisionBox::FULL,
    );

    assert_f64_near(clipped, 0.05);
}

#[test]
fn particle_x_collision_clips_horizontal_motion_to_shape_side() {
    let bounds = particle_collision_bounds([-0.15, 0.5, 0.5], 0.1, 0.2);
    let clipped = clipped_particle_axis_against_box(
        bounds,
        CollisionAxis::X,
        0.2,
        0.2,
        BlockPos { x: 0, y: 0, z: 0 },
        WorldCollisionBox::FULL,
    );

    assert_f64_near(clipped, 0.05);
}

#[test]
fn particle_collision_axis_order_matches_vanilla_horizontal_priority() {
    assert_eq!(
        particle_collision_axis_order([0.3, -0.1, 0.2]).map(axis_name),
        ["y", "x", "z"]
    );
    assert_eq!(
        particle_collision_axis_order([0.2, -0.1, 0.3]).map(axis_name),
        ["y", "z", "x"]
    );
}

#[test]
fn particle_block_fluid_surface_height_uses_block_collision_shape() {
    let mut world = empty_world();
    let air_sample = world.particle_block_fluid_surface_sample([0.5, 0.25, 0.5]);
    assert!(air_sample.block_is_air);

    let bottom_slab_id = vanilla_block_state_id(
        "minecraft:oak_slab",
        [("type", "bottom"), ("waterlogged", "false")],
    );
    set_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, bottom_slab_id);

    let sample = world.particle_block_fluid_surface_sample([0.5, 0.25, 0.5]);
    assert_f64_near(sample.block_collision_height, 0.5);
    assert_f64_near(sample.fluid_height, 0.0);
    assert_eq!(sample.fluid_kind, None);
    assert!(!sample.block_is_air);
    assert_f64_near(
        world.particle_block_fluid_surface_height([0.5, 0.25, 0.5]),
        0.5,
    );
}

#[test]
fn particle_block_fluid_surface_height_uses_fluid_height() {
    let mut world = empty_world();
    let water_id = vanilla_block_state_id("minecraft:water", [("level", "0")]);
    set_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, water_id);

    let sample = world.particle_block_fluid_surface_sample([0.5, 0.25, 0.5]);
    assert_f64_near(sample.block_collision_height, 0.0);
    assert_f64_near(sample.fluid_height, 8.0 / 9.0);
    assert_eq!(sample.fluid_kind, Some(TerrainFluidKind::Water));
    assert!(!sample.block_is_air);
    assert_f64_near(
        world.particle_block_fluid_surface_height([0.5, 0.25, 0.5]),
        8.0 / 9.0,
    );
}

#[test]
fn environment_animate_block_sample_reports_fluid_shape_and_drip_facts() {
    let mut world = empty_world();
    let pos = BlockPos { x: 0, y: 0, z: 0 };

    assert_eq!(
        world.environment_animate_block_sample(pos),
        Some(EnvironmentAnimateBlockSample {
            fluid: None,
            block_is_air: true,
            can_occlude: false,
            solid_render: false,
            full_face_sturdy: [false; 6],
            collision_full_face: [false; 6],
            fire_flammable: false,
            triggers_dried_ghast_ambient_sound: false,
            enchantment_power_provider: false,
            enchantment_power_transmitter: true,
            impermeable: false,
            collision_bounds: None,
        })
    );

    let stone_id = vanilla_block_state_id("minecraft:stone", []);
    set_block(&mut world, pos, stone_id);
    assert_eq!(
        world.environment_animate_block_sample(pos),
        Some(EnvironmentAnimateBlockSample {
            fluid: None,
            block_is_air: false,
            can_occlude: true,
            solid_render: true,
            full_face_sturdy: [true; 6],
            collision_full_face: [true; 6],
            fire_flammable: false,
            triggers_dried_ghast_ambient_sound: false,
            enchantment_power_provider: false,
            enchantment_power_transmitter: false,
            impermeable: false,
            collision_bounds: Some(EnvironmentAnimateCollisionBounds {
                min_x: 0.0,
                min_y: 0.0,
                min_z: 0.0,
                max_x: 1.0,
                max_y: 1.0,
                max_z: 1.0,
            }),
        })
    );

    let top_slab_id = vanilla_block_state_id(
        "minecraft:oak_slab",
        [("type", "top"), ("waterlogged", "false")],
    );
    set_block(&mut world, pos, top_slab_id);
    let top_slab = world
        .environment_animate_block_sample(pos)
        .expect("loaded top slab");
    assert!(!top_slab.solid_render);
    assert!(top_slab.can_occlude);
    assert_eq!(
        top_slab.full_face_sturdy,
        [false, true, false, false, false, false]
    );
    assert_eq!(
        top_slab.collision_full_face,
        [false, true, false, false, false, false]
    );
    assert_eq!(
        top_slab.collision_bounds,
        Some(EnvironmentAnimateCollisionBounds {
            min_x: 0.0,
            min_y: 0.5,
            min_z: 0.0,
            max_x: 1.0,
            max_y: 1.0,
            max_z: 1.0,
        })
    );

    let glass_id = vanilla_block_state_id("minecraft:glass", []);
    set_block(&mut world, pos, glass_id);
    let glass = world
        .environment_animate_block_sample(pos)
        .expect("loaded glass");
    assert!(glass.impermeable);
    assert!(!glass.can_occlude);
    assert!(!glass.solid_render);

    let oak_planks_id = vanilla_block_state_id("minecraft:oak_planks", []);
    set_block(&mut world, pos, oak_planks_id);
    assert!(
        world
            .environment_animate_block_sample(pos)
            .expect("loaded oak planks")
            .fire_flammable
    );

    let soul_sand_id = vanilla_block_state_id("minecraft:soul_sand", []);
    set_block(&mut world, pos, soul_sand_id);
    assert!(
        world
            .environment_animate_block_sample(pos)
            .expect("loaded soul sand")
            .triggers_dried_ghast_ambient_sound
    );

    let bookshelf_id = vanilla_block_state_id("minecraft:bookshelf", []);
    set_block(&mut world, pos, bookshelf_id);
    let bookshelf = world
        .environment_animate_block_sample(pos)
        .expect("loaded bookshelf");
    assert!(bookshelf.enchantment_power_provider);
    assert!(!bookshelf.enchantment_power_transmitter);

    set_block(&mut world, pos, 0);
    let air = world
        .environment_animate_block_sample(pos)
        .expect("loaded air");
    assert!(!air.enchantment_power_provider);
    assert!(air.enchantment_power_transmitter);

    let water_id = vanilla_block_state_id("minecraft:water", [("level", "0")]);
    set_block(&mut world, pos, water_id);
    assert_eq!(
        world
            .environment_animate_block_sample(pos)
            .expect("loaded water")
            .fluid,
        Some(crate::TerrainFluidState::new(
            TerrainFluidKind::Water,
            8,
            false,
        ))
    );
    assert_eq!(
        world.environment_animate_block_sample(BlockPos { x: 32, ..pos }),
        None
    );
}

#[test]
fn falling_block_free_query_matches_air_fire_liquid_and_replaceable_states() {
    let mut world = empty_world();
    let pos = BlockPos { x: 0, y: 0, z: 0 };

    for block_state_id in [
        vanilla_block_state_id("minecraft:air", []),
        vanilla_block_state_id(
            "minecraft:fire",
            [
                ("age", "0"),
                ("east", "false"),
                ("north", "false"),
                ("south", "false"),
                ("up", "false"),
                ("west", "false"),
            ],
        ),
        vanilla_block_state_id("minecraft:soul_fire", []),
        vanilla_block_state_id("minecraft:water", [("level", "0")]),
        vanilla_block_state_id("minecraft:short_grass", []),
    ] {
        set_block(&mut world, pos, block_state_id);
        assert_eq!(world.falling_block_is_free(pos), Some(true));
    }

    set_block(
        &mut world,
        pos,
        vanilla_block_state_id("minecraft:stone", []),
    );
    assert_eq!(world.falling_block_is_free(pos), Some(false));
    assert_eq!(
        world.falling_block_is_free(BlockPos { y: -1, ..pos }),
        Some(true)
    );
    assert_eq!(world.falling_block_is_free(BlockPos { x: 32, ..pos }), None);
}

#[test]
fn particle_block_fluid_surface_sample_reports_lava_kind_and_height() {
    let mut world = empty_world();
    let lava_id = vanilla_block_state_id("minecraft:lava", [("level", "0")]);
    set_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, lava_id);

    let sample = world.particle_block_fluid_surface_sample([0.5, 0.25, 0.5]);

    assert_f64_near(sample.block_collision_height, 0.0);
    assert_f64_near(sample.fluid_height, 8.0 / 9.0);
    assert_eq!(sample.fluid_kind, Some(TerrainFluidKind::Lava));
    assert!(!sample.block_is_air);
    assert_f64_near(sample.max_surface_height(), 8.0 / 9.0);
}

#[test]
fn particle_block_fluid_surface_height_is_empty_for_unloaded_or_nonfinite_position() {
    let world = empty_world();

    assert_eq!(
        world.particle_block_fluid_surface_sample([32.5, 0.25, 0.5]),
        ParticleBlockFluidSurfaceSample::default()
    );
    assert_f64_near(
        world.particle_block_fluid_surface_height([32.5, 0.25, 0.5]),
        0.0,
    );
    assert_eq!(
        world.particle_block_fluid_surface_sample([f64::NAN, 0.25, 0.5]),
        ParticleBlockFluidSurfaceSample::default()
    );
    assert_f64_near(
        world.particle_block_fluid_surface_height([f64::NAN, 0.25, 0.5]),
        0.0,
    );
}
