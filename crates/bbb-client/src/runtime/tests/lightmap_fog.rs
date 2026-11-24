use super::*;

#[test]
fn fog_environment_uses_vanilla_render_distance_range_and_dimension_fog_distances() {
    let mut overworld = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut overworld, 6_000);
    let fog = fog_environment_for_world_at_camera(
        &overworld,
        camera_pose_from_world(&overworld),
        12,
        0.0,
        0.0,
        false,
    );
    assert_fog_environment_close(
        fog,
        FogEnvironment::world_with_visibility_ends(
            clear_color_to_fog_color(clear_color_for_world(&overworld, false)),
            VANILLA_DEFAULT_FOG_START_DISTANCE,
            VANILLA_DEFAULT_FOG_END_DISTANCE,
            12,
            VANILLA_DEFAULT_SKY_FOG_END_DISTANCE.min(12.0 * 16.0),
            VANILLA_DEFAULT_CLOUD_FOG_END_DISTANCE,
        ),
    );

    for (cloud_range_chunks, cloud_fog_end_distance, expected_cloud_end) in [
        (64, None, 1024.0),
        (64, Some(1536.0), 1024.0),
        (64, Some(512.0), 512.0),
    ] {
        let colors = CameraEnvironmentColors {
            cloud_fog_end_distance,
            ..camera_environment_colors_for_default_projection(
                &overworld,
                camera_pose_from_world(&overworld),
            )
        };
        let configured = fog_environment_for_world_with_environment_colors(
            &overworld,
            colors,
            12,
            cloud_range_chunks,
            0.0,
            0.0,
            false,
        );
        assert_fog_environment_close(
            configured,
            FogEnvironment {
                cloud_end: expected_cloud_end,
                ..fog
            },
        );
    }

    let mut nether = world_with_dimension(1, "minecraft:the_nether");
    set_world_day_time(&mut nether, 6_000);
    let fog = fog_environment_for_world_at_camera(
        &nether,
        camera_pose_from_world(&nether),
        20,
        0.0,
        0.0,
        false,
    );
    assert_eq!(fog.environmental_start, VANILLA_NETHER_FOG_START_DISTANCE);
    assert_eq!(fog.environmental_end, VANILLA_NETHER_FOG_END_DISTANCE);
    assert_eq!(fog.render_distance_start, 288.0);
    assert_eq!(fog.render_distance_end, 320.0);
    assert_eq!(fog.sky_end, 320.0);
    assert_eq!(fog.cloud_end, VANILLA_DEFAULT_CLOUD_FOG_END_DISTANCE);
}

#[test]
fn fog_environment_applies_vanilla_rain_fog_distance_multiplier() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);

    let fog = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.0,
        0.5,
        false,
    );

    assert_eq!(fog.environmental_start, -80.0);
    assert_eq!(fog.environmental_end, 896.0);
    assert_eq!(
        fog.sky_end,
        VANILLA_DEFAULT_SKY_FOG_END_DISTANCE.min(12.0 * 16.0)
    );
    assert_eq!(fog.cloud_end, VANILLA_DEFAULT_CLOUD_FOG_END_DISTANCE);
}

#[test]
fn fog_environment_clamps_atmospheric_distance_for_boss_world_fog() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    add_boss_bar(&mut world, false, true);

    let fog = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.0,
        0.0,
        false,
    );

    assert_eq!(fog.environmental_start, VANILLA_DEFAULT_FOG_START_DISTANCE);
    assert_eq!(fog.environmental_end, VANILLA_NETHER_FOG_END_DISTANCE);
    assert_eq!(fog.sky_end, VANILLA_NETHER_FOG_END_DISTANCE);
    assert_eq!(fog.cloud_end, VANILLA_NETHER_FOG_END_DISTANCE);
    let configured = fog_environment_for_world_with_environment_colors(
        &world,
        CameraEnvironmentColors {
            cloud_fog_end_distance: Some(32.0),
            ..camera_environment_colors_for_default_projection(
                &world,
                camera_pose_from_world(&world),
            )
        },
        12,
        2,
        0.0,
        0.0,
        false,
    );
    assert_fog_environment_close(configured, fog);
}

#[test]
fn fog_environment_uses_water_fog_distances_when_eye_is_in_water() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 42));
    insert_uniform_biome_neighbors(&mut world, 42);
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    install_biome_environment_profiles(
        &mut world,
        &[(
            42,
            &[(
                "minecraft:visual/water_fog_color",
                TestEnvironmentAttributeValue::String("#022044"),
            )],
        )],
    );

    let fog = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.5,
        0.0,
        false,
    );

    assert_fog_environment_close(
        fog,
        FogEnvironment::world_with_visibility_ends(
            clear_color_to_fog_color(clear_color_from_argb(rgb_u8_to_argb([0x04, 0x4b, 0xa1]))),
            VANILLA_DEFAULT_WATER_FOG_START_DISTANCE,
            VANILLA_DEFAULT_WATER_FOG_END_DISTANCE * 0.5,
            12,
            VANILLA_DEFAULT_WATER_FOG_END_DISTANCE * 0.5,
            VANILLA_DEFAULT_WATER_FOG_END_DISTANCE * 0.5,
        ),
    );
}

#[test]
fn lava_fog_uses_normal_fire_resistance_and_spectator_distance_precedence() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    let colors = CameraEnvironmentColors {
        fog_type: CameraFogType::Lava,
        ..CameraEnvironmentColors::default()
    };

    let normal =
        fog_environment_for_world_with_environment_colors(&world, colors, 12, 128, 0.0, 0.0, false);
    assert_eq!(normal.environmental_start, 0.25);
    assert_eq!(normal.environmental_end, 1.0);
    assert_eq!(normal.sky_end, 1.0);
    assert_eq!(normal.cloud_end, 1.0);
    assert_eq!(
        normal.color,
        clear_color_to_fog_color(clear_color_from_argb(VANILLA_LAVA_FOG_COLOR))
    );

    assert!(world.apply_update_mob_effect(mob_effect(
        42,
        VANILLA_MOB_EFFECT_FIRE_RESISTANCE_ID,
        200,
    )));
    let resistant =
        fog_environment_for_world_with_environment_colors(&world, colors, 12, 128, 0.0, 0.0, false);
    assert_eq!(resistant.environmental_start, 0.0);
    assert_eq!(resistant.environmental_end, 5.0);
    assert_eq!(resistant.sky_end, 5.0);
    assert_eq!(resistant.cloud_end, 5.0);

    world.apply_game_event(ProtocolGameEvent {
        event_id: 3,
        param: 3.0,
    });
    let spectator =
        fog_environment_for_world_with_environment_colors(&world, colors, 12, 128, 0.0, 0.0, false);
    assert_eq!(spectator.environmental_start, -8.0);
    assert_eq!(spectator.environmental_end, 96.0);
    assert_eq!(spectator.sky_end, 96.0);
    assert_eq!(spectator.cloud_end, 96.0);
}

#[test]
fn lava_fog_precedes_mob_effect_distances_and_suppresses_the_sky_pass() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_BLINDNESS_ID, -1,)));
    let colors = CameraEnvironmentColors {
        fog_type: CameraFogType::Lava,
        ..CameraEnvironmentColors::default()
    };

    let fog =
        fog_environment_for_world_with_environment_colors(&world, colors, 12, 128, 0.0, 0.0, false);
    assert_eq!(fog.environmental_start, 0.25);
    assert_eq!(fog.environmental_end, 1.0);
    assert_eq!(
        fog.color,
        clear_color_to_fog_color(clear_color_from_argb(VANILLA_LAVA_FOG_COLOR))
    );
    assert_eq!(
        sky_environment_for_world_with_environment_colors(&world, colors, false),
        SkyEnvironment::disabled()
    );
}

#[test]
fn powder_snow_fog_precedes_mob_effect_distances_preserves_color_and_suppresses_sky() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_BLINDNESS_ID, -1,)));
    let colors = CameraEnvironmentColors {
        fog_type: CameraFogType::PowderSnow,
        ..CameraEnvironmentColors::default()
    };

    let normal =
        fog_environment_for_world_with_environment_colors(&world, colors, 12, 128, 0.0, 0.0, false);
    assert_eq!(normal.environmental_start, 0.0);
    assert_eq!(normal.environmental_end, 2.0);
    assert_eq!(normal.sky_end, 2.0);
    assert_eq!(normal.cloud_end, 2.0);
    assert_eq!(
        normal.color,
        clear_color_to_fog_color(clear_color_from_argb(VANILLA_POWDER_SNOW_FOG_COLOR)),
        "powder snow excludes the blindness darkness multiplier"
    );
    assert_eq!(
        sky_environment_for_world_with_environment_colors(&world, colors, false),
        SkyEnvironment::disabled()
    );

    world.apply_game_event(ProtocolGameEvent {
        event_id: 3,
        param: 3.0,
    });
    let spectator =
        fog_environment_for_world_with_environment_colors(&world, colors, 12, 128, 0.0, 0.0, false);
    assert_eq!(spectator.environmental_start, -8.0);
    assert_eq!(spectator.environmental_end, 96.0);
    assert_eq!(spectator.sky_end, 96.0);
    assert_eq!(spectator.cloud_end, 96.0);
}

#[test]
fn fog_environment_applies_biome_water_fog_end_distance_modifier() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 42));
    insert_uniform_biome_neighbors(&mut world, 42);
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    install_biome_environment_profiles(
        &mut world,
        &[(
            42,
            &[(
                "minecraft:visual/water_fog_end_distance",
                TestEnvironmentAttributeValue::Multiply(0.85),
            )],
        )],
    );

    let fog = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.5,
        0.0,
        false,
    );
    let expected_end = VANILLA_DEFAULT_WATER_FOG_END_DISTANCE * 0.85 * 0.5;

    assert!((fog.environmental_start - VANILLA_DEFAULT_WATER_FOG_START_DISTANCE).abs() < 1e-6);
    assert!((fog.environmental_end - expected_end).abs() < 1e-5);
    assert!((fog.sky_end - expected_end).abs() < 1e-5);
    assert!((fog.cloud_end - expected_end).abs() < 1e-5);
}

#[test]
fn blindness_overrides_atmospheric_fog_and_darkens_its_color() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    attach_lightmap_local_player(&mut world, 42);
    let baseline = clear_color_for_world(&world, false);
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_BLINDNESS_ID, 10,)));

    let fog = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.0,
        0.0,
        false,
    );

    // lerp(10 / 20, 12 * 16, 5) = 98.5.
    assert_eq!(fog.environmental_start, 24.625);
    assert_eq!(fog.environmental_end, 98.5);
    assert!((fog.sky_end - 78.8).abs() < 1e-5);
    assert!((fog.cloud_end - 78.8).abs() < 1e-5);
    // Blindness duration 10 contributes darkness 0.5, then FogRenderer
    // multiplies the base color by square(1 - darkness) = 0.25.
    assert_eq!(
        fog.color[0],
        ((baseline.r * 255.0) as i32 / 4) as f32 / 255.0
    );
    assert_eq!(
        fog.color[1],
        ((baseline.g * 255.0) as i32 / 4) as f32 / 255.0
    );
    assert_eq!(
        fog.color[2],
        ((baseline.b * 255.0) as i32 / 4) as f32 / 255.0
    );
}

#[test]
fn blindness_takes_fog_precedence_over_darkness_and_blocks_sky() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_DARKNESS_ID, 260,)));
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_BLINDNESS_ID, -1,)));

    let fog = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.0,
        0.0,
        false,
    );

    assert_eq!(fog.environmental_start, 1.25);
    assert_eq!(fog.environmental_end, 5.0);
    assert_eq!(fog.sky_end, 4.0);
    assert_eq!(fog.cloud_end, 4.0);
    assert_eq!(
        sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), false),
        SkyEnvironment::disabled()
    );
}

#[test]
fn darkness_fog_uses_canonical_blend_factor() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(world.apply_update_mob_effect(mob_effect_with_blend(
        42,
        VANILLA_MOB_EFFECT_DARKNESS_ID,
        260,
        true,
    )));

    let initial = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.0,
        0.0,
        false,
    );
    assert_eq!(initial.environmental_end, 192.0);

    world.advance_client_level_entity_ticks_with_observation(11);
    let blended = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.0,
        0.0,
        false,
    );
    let expected_end = 192.0 + 0.5 * (15.0 - 192.0);
    assert!((blended.environmental_start - expected_end * 0.75).abs() < 1e-5);
    assert!((blended.environmental_end - expected_end).abs() < 1e-5);
    assert!((blended.sky_end - expected_end).abs() < 1e-5);
    assert!((blended.cloud_end - expected_end).abs() < 1e-5);
}

#[test]
fn camera_effect_fog_and_sky_follow_the_active_camera_entity() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    world.apply_add_entity(test_add_entity(100, VANILLA_ENTITY_TYPE_PLAYER_ID));
    assert!(world.apply_update_mob_effect(mob_effect(100, VANILLA_MOB_EFFECT_BLINDNESS_ID, -1,)));

    let local_fog = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.0,
        0.0,
        false,
    );
    assert_ne!(local_fog.environmental_end, 5.0);

    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 100 }));
    let remote_fog = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.0,
        0.0,
        false,
    );
    assert_eq!(remote_fog.environmental_end, 5.0);
    assert_eq!(
        sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), false),
        SkyEnvironment::disabled()
    );
}

#[test]
fn night_vision_brightens_atmospheric_fog_unless_darkness_is_present() {
    let mut world = world_with_custom_dimension_environment();
    attach_lightmap_local_player(&mut world, 42);
    let baseline = clear_color_for_world_at_camera(&world, camera_pose_from_world(&world), false);
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_NIGHT_VISION_ID, 240,)));

    let night_vision =
        clear_color_for_world_at_camera(&world, camera_pose_from_world(&world), false);
    assert!(night_vision.r >= baseline.r);
    assert!(night_vision.g >= baseline.g);
    assert!(night_vision.b >= baseline.b);
    assert_ne!(night_vision, baseline);
    assert_eq!(night_vision.r.max(night_vision.g).max(night_vision.b), 1.0);

    assert!(world.apply_update_mob_effect(mob_effect_with_blend(
        42,
        VANILLA_MOB_EFFECT_DARKNESS_ID,
        260,
        true,
    )));
    let darkness = clear_color_for_world_at_camera(&world, camera_pose_from_world(&world), false);
    assert_clear_color_close(darkness, baseline);
}

#[test]
fn camera_biome_sky_color_uses_vanilla_gaussian_spatial_weights() {
    let mut world = world_with_custom_dimension_environment();
    world.set_local_player_pose(local_player_pose(
        [8.0, 32.0 - f64::from(CameraPose::STANDING_EYE_HEIGHT), 8.0],
        0.0,
        0.0,
    ));
    insert_split_x_biome_neighborhood(&mut world, 10, 20);
    install_biome_environment_profiles(
        &mut world,
        &[
            (
                10,
                &[(
                    "minecraft:visual/sky_color",
                    TestEnvironmentAttributeValue::String("#000000"),
                )],
            ),
            (
                20,
                &[(
                    "minecraft:visual/sky_color",
                    TestEnvironmentAttributeValue::String("#64c8fa"),
                )],
            ),
        ],
    );
    assert_eq!(
        world
            .sampled_client_environment_attributes_at(
                [0.0, 32.0, 8.0],
                EnvironmentBiomeSampling::Direct,
            )
            .unwrap()
            .sky_color,
        Some([0, 0, 0])
    );
    assert_eq!(
        world
            .sampled_client_environment_attributes_at(
                [12.0, 32.0, 8.0],
                EnvironmentBiomeSampling::Direct,
            )
            .unwrap()
            .sky_color,
        Some([100, 200, 250])
    );

    let sky_color =
        camera_environment_colors_for_default_projection(&world, camera_pose_from_world(&world))
            .sky_color
            .unwrap();

    assert_eq!(sky_color, [50, 100, 125]);
}

#[test]
fn camera_biome_fog_and_water_fog_use_vanilla_gaussian_spatial_weights() {
    let mut world = world_with_custom_dimension_environment();
    world.set_local_player_pose(local_player_pose(
        [8.0, 32.0 - f64::from(CameraPose::STANDING_EYE_HEIGHT), 8.0],
        0.0,
        0.0,
    ));
    insert_split_x_biome_neighborhood(&mut world, 10, 20);
    install_biome_environment_profiles(
        &mut world,
        &[
            (
                10,
                &[
                    (
                        "minecraft:visual/fog_color",
                        TestEnvironmentAttributeValue::String("#0a141e"),
                    ),
                    (
                        "minecraft:visual/water_fog_color",
                        TestEnvironmentAttributeValue::String("#010203"),
                    ),
                ],
            ),
            (
                20,
                &[
                    (
                        "minecraft:visual/fog_color",
                        TestEnvironmentAttributeValue::String("#6edce6"),
                    ),
                    (
                        "minecraft:visual/water_fog_color",
                        TestEnvironmentAttributeValue::String("#65cacb"),
                    ),
                ],
            ),
        ],
    );

    let camera_pose = camera_pose_from_world(&world);
    let colors = camera_environment_colors_for_default_projection(&world, camera_pose);
    let fog_color = colors.fog_color.unwrap();
    let water_fog_color = colors.water_fog_color.unwrap();

    assert_eq!(fog_color, [60, 120, 130]);
    assert_eq!(water_fog_color, [51, 102, 103]);
}

#[test]
fn lightmap_tick_state_applies_end_flash_sky_factor_from_end_clock() {
    let mut world = world_with_dimension(2, "minecraft:the_end");
    set_world_end_clock_time(&mut world, 1_486);
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.5);

    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);

    assert!((environment.sky_factor - 1.0).abs() < 1e-6);
    assert_close3(
        environment.sky_light_color,
        [172.0 / 255.0, 96.0 / 255.0, 205.0 / 255.0],
    );
    assert_close3(
        environment.ambient_color,
        [63.0 / 255.0, 71.0 / 255.0, 63.0 / 255.0],
    );
}

#[test]
fn lightmap_tick_state_uses_locally_advanced_end_clock() {
    let mut world = world_with_dimension(2, "minecraft:the_end");
    world.apply_world_time(PlayTime {
        game_time: 100,
        clock_updates: vec![ProtocolClockUpdate {
            clock_id: 1,
            total_ticks: 1_485,
            partial_tick: 0.75,
            rate: 0.5,
        }],
    });
    world.advance_client_time(1);
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.5);

    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);

    assert!((environment.sky_factor - 1.0).abs() < 1e-6);
}

#[test]
fn lightmap_tick_state_does_not_use_overworld_clock_for_end_flash() {
    let mut world = world_with_dimension(2, "minecraft:the_end");
    set_world_day_time(&mut world, 1_486);
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.5);

    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);

    assert_eq!(environment.sky_factor, 0.0);
}

#[test]
fn lightmap_tick_state_hides_end_flash_when_option_is_enabled() {
    let mut world = world_with_dimension(2, "minecraft:the_end");
    set_world_end_clock_time(&mut world, 1_486);
    let mut lightmap =
        LightmapTickState::with_brightness_factor_and_hide_lightning_flash(0.5, true);

    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);

    assert_eq!(environment.sky_factor, 0.0);
    assert_close3(
        environment.sky_light_color,
        [172.0 / 255.0, 96.0 / 255.0, 205.0 / 255.0],
    );
}

#[test]
fn lightmap_tick_state_divides_end_flash_sky_factor_for_boss_world_fog() {
    let mut world = world_with_dimension(2, "minecraft:the_end");
    set_world_end_clock_time(&mut world, 1_486);
    add_boss_bar(&mut world, false, true);
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.5);

    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);

    assert!((environment.sky_factor - (1.0 / 3.0)).abs() < 1e-6);
    assert_eq!(environment.boss_overlay_world_darkening, 0.0);
}

#[test]
fn lightmap_tick_state_projects_boss_overlay_world_darkening() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    add_boss_bar(&mut world, true, false);
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.5);

    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);
    assert!((environment.boss_overlay_world_darkening - 0.05).abs() < 1e-6);

    lightmap.advance_for_world(19, &world);
    let environment = lightmap.environment_for_world(&world);
    assert_eq!(environment.boss_overlay_world_darkening, 1.0);

    clear_boss_overlay_properties(&mut world);
    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);
    assert!((environment.boss_overlay_world_darkening - 0.9875).abs() < 1e-6);
}

#[test]
fn lightmap_tick_state_environment_preserves_gamma_and_flicker() {
    let world = world_with_dimension(1, "minecraft:the_nether");
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.8);
    let block_factor = lightmap.advance(1);
    let environment = lightmap.environment_for_world(&world);

    assert_eq!(environment.brightness_factor, 0.8);
    assert!((environment.block_factor - block_factor).abs() < 1e-6);
    assert_eq!(environment.sky_factor, 0.0);
    assert_close3(
        environment.ambient_color,
        [48.0 / 255.0, 40.0 / 255.0, 33.0 / 255.0],
    );
}

#[test]
fn lightmap_environment_projects_local_player_night_vision() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_NIGHT_VISION_ID, 240,)));

    let environment = lightmap_environment_for_world_at_tick(
        &world,
        0.7,
        1.4,
        0,
        VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK,
    );
    assert_eq!(environment.night_vision_factor, 1.0);
    assert_eq!(environment.brightness_factor, 0.7);

    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_NIGHT_VISION_ID, 40,)));
    let environment = lightmap_environment_for_world_at_tick(
        &world,
        0.7,
        1.4,
        0,
        VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK,
    );
    let expected = 0.7
        + ((40.0 - VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK) * std::f32::consts::PI * 0.2).sin() * 0.3;
    assert!((environment.night_vision_factor - expected).abs() < 1e-6);
}

#[test]
fn lightmap_environment_projects_local_player_darkness() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_DARKNESS_ID, 260,)));

    let environment = lightmap_environment_for_world_at_tick(
        &world,
        0.8,
        1.4,
        10,
        VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK,
    );

    assert_eq!(environment.brightness_factor, 0.0);
    let expected_darkness =
        (((10.0 - VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK) * std::f32::consts::PI * 0.025).cos()
            * 0.45)
            .max(0.0);
    assert!((environment.darkness_scale - expected_darkness).abs() < 1e-6);
    assert_eq!(environment.night_vision_factor, 0.0);
}

#[test]
fn lightmap_tick_state_blends_local_player_darkness_like_vanilla() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(world.apply_update_mob_effect(mob_effect_with_blend(
        42,
        VANILLA_MOB_EFFECT_DARKNESS_ID,
        260,
        true,
    )));
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.8);

    lightmap.advance_for_world(0, &world);
    let environment = lightmap.environment_for_world(&world);
    assert_eq!(environment.brightness_factor, 0.8);
    assert_eq!(environment.darkness_scale, 0.0);

    world.advance_client_level_entity_ticks_with_observation(1);
    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);
    let first_tick_factor = 1.0 / 22.0;
    assert!((environment.brightness_factor - (0.8 - first_tick_factor)).abs() < 1e-6);
    assert!((environment.darkness_scale - 0.45 * first_tick_factor).abs() < 1e-6);

    world.advance_client_level_entity_ticks_with_observation(21);
    lightmap.advance_for_world(21, &world);
    let environment = lightmap.environment_for_world(&world);
    assert_eq!(environment.brightness_factor, 0.0);
    let expected_darkness =
        (((22.0 - VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK) * std::f32::consts::PI * 0.025).cos()
            * 0.45)
            .max(0.0);
    assert!((environment.darkness_scale - expected_darkness).abs() < 1e-6);
}

#[test]
fn lightmap_tick_state_applies_and_removes_non_blending_darkness_immediately() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_DARKNESS_ID, 260,)));
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.8);

    lightmap.advance_for_world(0, &world);
    let environment = lightmap.environment_for_world(&world);
    assert_eq!(environment.brightness_factor, 0.0);
    assert!(environment.darkness_scale > 0.0);

    assert!(world.apply_remove_mob_effect(RemoveMobEffect {
        entity_id: 42,
        effect_id: VANILLA_MOB_EFFECT_DARKNESS_ID,
    }));
    lightmap.advance_for_world(0, &world);
    let environment = lightmap.environment_for_world(&world);
    assert_eq!(environment.brightness_factor, 0.8);
    assert_eq!(environment.darkness_scale, 0.0);
}

#[test]
fn lightmap_tick_state_advances_night_vision_duration_before_sampling() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_NIGHT_VISION_ID, 201,)));
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.8);

    lightmap.advance_for_world(0, &world);
    assert_eq!(
        lightmap.environment_for_world(&world).night_vision_factor,
        1.0
    );

    world.advance_client_level_entity_ticks_with_observation(1);
    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);
    let expected = 0.7
        + ((200.0 - VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK) * std::f32::consts::PI * 0.2).sin() * 0.3;
    assert!((environment.night_vision_factor - expected).abs() < 1e-6);
}

#[test]
fn lightmap_tick_state_uses_conduit_water_vision_without_night_vision() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    assert!(world.apply_update_mob_effect(mob_effect(
        42,
        VANILLA_MOB_EFFECT_CONDUIT_POWER_ID,
        260,
    )));
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.8);

    lightmap.advance_for_world(0, &world);
    assert_eq!(
        lightmap.environment_for_world(&world).night_vision_factor,
        0.0
    );

    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);
    assert!((environment.night_vision_factor - 0.006).abs() < 1e-6);

    lightmap.advance_for_world(99, &world);
    let environment = lightmap.environment_for_world(&world);
    assert!((environment.night_vision_factor - 0.6).abs() < 1e-6);
}

#[test]
fn lightmap_tick_state_prefers_night_vision_over_conduit_water_vision() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    assert!(world.apply_update_mob_effect(mob_effect(
        42,
        VANILLA_MOB_EFFECT_CONDUIT_POWER_ID,
        260,
    )));
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_NIGHT_VISION_ID, 240,)));
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.8);

    world.advance_client_level_entity_ticks_with_observation(100);
    lightmap.advance_for_world(100, &world);
    let expected = 0.7
        + ((140.0 - VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK) * std::f32::consts::PI * 0.2).sin() * 0.3;
    let environment = lightmap.environment_for_world(&world);
    assert!((environment.night_vision_factor - expected).abs() < 1e-6);
    assert!((environment.night_vision_factor - 0.6).abs() > 0.01);
}

#[test]
fn lightmap_tick_state_water_vision_fades_out_when_eye_leaves_water() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    assert!(world.apply_update_mob_effect(mob_effect(
        42,
        VANILLA_MOB_EFFECT_CONDUIT_POWER_ID,
        260,
    )));
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.8);
    lightmap.advance_for_world(100, &world);
    assert!((lightmap.environment_for_world(&world).night_vision_factor - 0.6).abs() < 1e-6);

    world.set_local_player_pose(local_player_pose([0.5, 4.0, 0.5], 0.0, 0.0));
    lightmap.advance_for_world(1, &world);
    assert_eq!(
        lightmap.environment_for_world(&world).night_vision_factor,
        0.0
    );

    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);
    assert!((environment.night_vision_factor - 0.546).abs() < 1e-6);
}

#[test]
fn lightmap_environment_without_local_player_effects_keeps_base_factors() {
    let world = world_with_dimension(0, "minecraft:overworld");
    let environment = lightmap_environment_for_world_at_tick(
        &world,
        0.8,
        1.35,
        10,
        VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK,
    );

    assert_eq!(environment.brightness_factor, 0.8);
    assert_eq!(environment.block_factor, 1.35);
    assert_eq!(environment.darkness_scale, 0.0);
    assert_eq!(environment.night_vision_factor, 0.0);
}

#[test]
fn server_container_open_releases_held_movement() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(bbb_protocol::packets::PlayerInput {
            forward: true,
            ..bbb_protocol::packets::PlayerInput::default()
        })
    );

    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    release_input_if_screen_opened(false, &mut input, &mut world, &mut counters, &commands);

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(bbb_protocol::packets::PlayerInput::default())
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn server_container_open_releases_held_mouse_actions() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.set_local_player_pose(local_player_pose([0.0, 64.0, 0.0], 30.0, -10.0));
    let apple_id = bbb_protocol::item_types::VANILLA_ITEM_APPLE_ID;
    world.set_default_item_component_codec_values(BTreeMap::from([(
        apple_id,
        BTreeMap::from([(
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
            serde_json::json!({"consume_seconds": 1.6}),
        )]),
    )]));
    world.rebuild_item_component_defaults().unwrap();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(apple_id, 1),
    });
    let mut counters = NetCounters::default();

    crate::input::handle_mouse_input_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
        1.0,
    );
    world.set_local_destroying_block(bbb_client_world::BlockPos { x: 1, y: 2, z: 3 });

    assert!(matches!(rx.try_recv().unwrap(), NetCommand::UseItem(_)));
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    release_input_if_screen_opened(false, &mut input, &mut world, &mut counters, &commands);

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(bbb_protocol::packets::PlayerAction {
            action: bbb_protocol::packets::PlayerActionKind::AbortDestroyBlock,
            pos: bbb_protocol::packets::BlockPos { x: 1, y: 2, z: 3 },
            direction: bbb_protocol::packets::Direction::Down,
            sequence: 0,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(bbb_protocol::packets::PlayerAction {
            action: bbb_protocol::packets::PlayerActionKind::ReleaseUseItem,
            pos: bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 },
            direction: bbb_protocol::packets::Direction::Down,
            sequence: 0,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn server_dialog_open_releases_held_movement() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    world.apply_show_dialog(ShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    release_input_if_screen_opened(false, &mut input, &mut world, &mut counters, &commands);

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(bbb_protocol::packets::PlayerInput::default())
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sign_editor_open_releases_held_movement() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let sign_pos = BlockPos { x: 1, y: 2, z: 3 };
    set_lightmap_test_block(&mut world, sign_pos, OAK_SIGN_ROTATION_0_BLOCK_STATE_ID);
    let mut counters = NetCounters::default();

    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
        is_front_text: true,
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    release_input_if_screen_opened(false, &mut input, &mut world, &mut counters, &commands);

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(bbb_protocol::packets::PlayerInput::default())
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn hud_sign_editor_screen_projects_standing_sign_preview() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let pos = BlockPos { x: 1, y: 2, z: 3 };
    set_lightmap_test_block(&mut world, pos, OAK_SIGN_ROTATION_0_BLOCK_STATE_ID);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos {
            x: pos.x,
            y: pos.y,
            z: pos.z,
        },
        is_front_text: true,
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);

    let screen = hud_sign_editor_screen(&input, &world, winit::dpi::PhysicalSize::new(320, 240))
        .expect("sign editor screen");
    assert_eq!(screen.title, "Edit Sign Message");
    assert_eq!(
        screen.kind,
        HudSignEditorKind::Standing {
            wood: SignModelWood::Oak,
            attachment: SignModelAttachment::Standing,
        }
    );
    let preview = screen.sign_preview.expect("standing sign PIP preview");
    assert_eq!(preview.lighting, GuiItemLightingEntry::ItemsFlat);
    assert_eq!(preview.rect.width, 96);
    assert_eq!(preview.rect.height, 102);
    assert_eq!(screen.lines, std::array::from_fn(|_| String::new()));
    assert_eq!(screen.cursor, 0);
    assert_eq!(screen.selection, 0);
}

#[test]
fn hud_sign_editor_screen_projects_hanging_sign_background_state() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let pos = BlockPos { x: 1, y: 2, z: 3 };
    set_lightmap_test_block(
        &mut world,
        pos,
        BAMBOO_HANGING_SIGN_ATTACHED_ROTATION_0_BLOCK_STATE_ID,
    );
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos {
            x: pos.x,
            y: pos.y,
            z: pos.z,
        },
        is_front_text: true,
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);

    let screen = hud_sign_editor_screen(&input, &world, winit::dpi::PhysicalSize::new(320, 240))
        .expect("hanging sign editor screen");
    assert_eq!(screen.title, "Edit Hanging Sign Message");
    assert_eq!(
        screen.kind,
        HudSignEditorKind::Hanging {
            wood: SignModelWood::Bamboo,
        }
    );
    assert!(screen.sign_preview.is_none());
}

#[test]
fn hud_pause_screen_projects_no_menu_title() {
    let mut input = ClientInputState::new(true);
    let surface = winit::dpi::PhysicalSize::new(320, 240);

    assert!(hud_pause_screen(&input, &WorldStore::new(), None, surface).is_none());
    input.open_debug_pause_screen_without_menu_for_test();

    let screen = hud_pause_screen(&input, &WorldStore::new(), None, surface).expect("pause screen");
    assert_eq!(screen.title, "Game Paused");
    assert!(!screen.show_pause_menu);
    assert!(!screen.return_to_game_hovered);
    assert!(!screen.advancements_hovered);
    assert!(!screen.stats_hovered);
    assert!(!screen.send_feedback_hovered);
    assert!(!screen.report_bugs_hovered);
    assert!(screen.report_bugs_enabled);
    assert!(!screen.options_hovered);
    assert!(!screen.options_enabled);
    assert!(!screen.disconnect_hovered);
    assert!(screen.disconnect_enabled);
}

#[test]
fn hud_pause_screen_projects_menu_title() {
    let mut input = ClientInputState::new(true);
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu(&WorldStore::new(), &mut Default::default(), &None);

    let screen = hud_pause_screen(&input, &WorldStore::new(), None, surface).expect("pause screen");
    assert_eq!(screen.title, "Game Menu");
    assert!(screen.show_pause_menu);
    assert!(!screen.return_to_game_hovered);
    assert!(!screen.advancements_hovered);
    assert!(!screen.stats_hovered);
    assert!(!screen.send_feedback_hovered);
    assert!(!screen.report_bugs_hovered);
    assert!(screen.report_bugs_enabled);
    assert!(!screen.options_hovered);
    assert!(!screen.options_enabled);
    assert!(!screen.disconnect_hovered);
    assert!(screen.disconnect_enabled);
}

#[test]
fn hud_pause_screen_projects_return_to_game_hover() {
    let mut input = ClientInputState::new(true);
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu(&WorldStore::new(), &mut Default::default(), &None);

    assert!(
        input.handle_debug_pause_screen_cursor_moved(Some(winit::dpi::PhysicalPosition::new(
            68.0, 78.0
        )))
    );

    let screen = hud_pause_screen(&input, &WorldStore::new(), None, surface).expect("pause screen");
    assert!(screen.show_pause_menu);
    assert!(screen.return_to_game_hovered);
    assert!(!screen.advancements_hovered);
    assert!(!screen.stats_hovered);
    assert!(!screen.send_feedback_hovered);
    assert!(!screen.report_bugs_hovered);
    assert!(!screen.options_hovered);
    assert!(!screen.disconnect_hovered);
}

#[test]
fn hud_pause_screen_projects_advancements_hover() {
    let mut input = ClientInputState::new(true);
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu(&WorldStore::new(), &mut Default::default(), &None);

    assert!(
        input.handle_debug_pause_screen_cursor_moved(Some(winit::dpi::PhysicalPosition::new(
            68.0, 102.0
        )))
    );

    let screen = hud_pause_screen(&input, &WorldStore::new(), None, surface).expect("pause screen");
    assert!(screen.show_pause_menu);
    assert!(!screen.return_to_game_hovered);
    assert!(screen.advancements_hovered);
    assert!(!screen.stats_hovered);
    assert!(!screen.send_feedback_hovered);
    assert!(!screen.report_bugs_hovered);
    assert!(!screen.options_hovered);
    assert!(!screen.disconnect_hovered);
}

#[test]
fn hud_pause_screen_projects_stats_hover() {
    let mut input = ClientInputState::new(true);
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu(&WorldStore::new(), &mut Default::default(), &None);

    assert!(
        input.handle_debug_pause_screen_cursor_moved(Some(winit::dpi::PhysicalPosition::new(
            170.0, 102.0
        )))
    );

    let screen = hud_pause_screen(&input, &WorldStore::new(), None, surface).expect("pause screen");
    assert!(screen.show_pause_menu);
    assert!(!screen.return_to_game_hovered);
    assert!(!screen.advancements_hovered);
    assert!(screen.stats_hovered);
    assert!(!screen.send_feedback_hovered);
    assert!(!screen.report_bugs_hovered);
    assert!(!screen.options_hovered);
    assert!(!screen.disconnect_hovered);
}

#[test]
fn hud_pause_screen_projects_send_feedback_hover() {
    let mut input = ClientInputState::new(true);
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu(&WorldStore::new(), &mut Default::default(), &None);

    assert!(
        input.handle_debug_pause_screen_cursor_moved(Some(winit::dpi::PhysicalPosition::new(
            68.0, 126.0
        )))
    );

    let screen = hud_pause_screen(&input, &WorldStore::new(), None, surface).expect("pause screen");
    assert!(screen.show_pause_menu);
    assert!(screen.send_feedback_hovered);
    assert!(!screen.report_bugs_hovered);
    assert!(screen.report_bugs_enabled);
    assert!(!screen.options_hovered);
    assert!(!screen.disconnect_hovered);
}

#[test]
fn hud_pause_screen_projects_report_bugs_hover() {
    let mut input = ClientInputState::new(true);
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu(&WorldStore::new(), &mut Default::default(), &None);

    assert!(
        input.handle_debug_pause_screen_cursor_moved(Some(winit::dpi::PhysicalPosition::new(
            170.0, 126.0
        )))
    );

    let screen = hud_pause_screen(&input, &WorldStore::new(), None, surface).expect("pause screen");
    assert!(screen.show_pause_menu);
    assert!(!screen.send_feedback_hovered);
    assert!(screen.report_bugs_hovered);
    assert!(screen.report_bugs_enabled);
    assert!(!screen.options_hovered);
    assert!(!screen.disconnect_hovered);
}

#[test]
fn hud_pause_screen_projects_custom_dialog_and_feedback_subscreen() {
    fn network_notice_dialog(title: &str, external_title: &str) -> Vec<u8> {
        fn write_string_tag(payload: &mut Vec<u8>, name: &str, value: &str) {
            payload.push(8);
            payload.extend_from_slice(&(name.len() as u16).to_be_bytes());
            payload.extend_from_slice(name.as_bytes());
            payload.extend_from_slice(&(value.len() as u16).to_be_bytes());
            payload.extend_from_slice(value.as_bytes());
        }

        let mut payload = vec![10];
        write_string_tag(&mut payload, "type", "minecraft:notice");
        write_string_tag(&mut payload, "title", title);
        write_string_tag(&mut payload, "external_title", external_title);
        payload.push(0);
        payload
    }

    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:dialog",
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:custom",
            network_notice_dialog("Internal", "External"),
        )],
    );
    world.apply_update_tags(bbb_protocol::packets::UpdateTags {
        registries: vec![bbb_protocol::packets::RegistryTags {
            registry: "minecraft:dialog".to_string(),
            tags: vec![bbb_protocol::packets::TagNetworkPayload {
                tag: "minecraft:pause_screen_additions".to_string(),
                entries: vec![0],
            }],
        }],
    });
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu(&world, &mut Default::default(), &None);
    assert!(
        input.handle_debug_pause_screen_cursor_moved(Some(winit::dpi::PhysicalPosition::new(
            170.0, 126.0
        )))
    );

    let screen =
        hud_pause_screen(&input, &WorldStore::new(), None, surface).expect("custom pause screen");
    assert_eq!(screen.title, "Game Menu");
    assert!(!screen.feedback_subscreen);
    assert_eq!(screen.custom_dialog_label.as_deref(), Some("External"));
    assert!(screen.custom_dialog_hovered);
    let tooltip = screen.custom_dialog_tooltip.expect("custom tooltip");
    assert!(tooltip.text.contains("third-party servers"));
    assert_eq!((tooltip.x, tooltip.y), (170, 126));
    assert!(!screen.report_bugs_hovered);

    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(68.0, 126.0)),
        surface,
    ));
    assert!(
        input.handle_debug_pause_screen_cursor_moved(Some(winit::dpi::PhysicalPosition::new(
            68.0, 68.0
        )))
    );

    let screen =
        hud_pause_screen(&input, &WorldStore::new(), None, surface).expect("feedback subscreen");
    assert_eq!(screen.title, "Feedback");
    assert!(screen.feedback_subscreen);
    assert!(screen.send_feedback_hovered);
    assert!(!screen.custom_dialog_hovered);
    assert!(screen.custom_dialog_tooltip.is_none());
    assert!(!screen.feedback_back_hovered);
}

#[test]
fn hud_pause_screen_projects_disabled_options_row_shell() {
    let mut input = ClientInputState::new(true);
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu(&WorldStore::new(), &mut Default::default(), &None);

    assert!(
        input.handle_debug_pause_screen_cursor_moved(Some(winit::dpi::PhysicalPosition::new(
            68.0, 150.0
        )))
    );

    let screen = hud_pause_screen(&input, &WorldStore::new(), None, surface).expect("pause screen");
    assert!(screen.show_pause_menu);
    assert!(!screen.options_hovered);
    assert!(!screen.options_enabled);
    let tooltip = screen
        .options_tooltip
        .as_ref()
        .expect("hovering disabled Options button should display tooltip");
    assert!(tooltip.text.contains("CLI flags"));
    assert!(tooltip.text.contains("options.txt"));
    assert!(!screen.disconnect_hovered);

    assert!(input
        .handle_debug_pause_screen_cursor_moved(Some(winit::dpi::PhysicalPosition::new(0.0, 0.0))));
    let away = hud_pause_screen(&input, &WorldStore::new(), None, surface).expect("pause screen");
    assert!(away.options_tooltip.is_none());
}

#[test]
fn hud_pause_screen_projects_disconnect_hover_and_disabled_after_click() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    let (x, y, width, height) = bbb_client_render::hud_pause_disconnect_button_rect(surface);
    let position =
        winit::dpi::PhysicalPosition::new(f64::from(x + width / 2), f64::from(y + height / 2));
    input.open_debug_pause_screen_with_menu(&WorldStore::new(), &mut Default::default(), &None);

    assert!(input.handle_debug_pause_screen_cursor_moved(Some(position)));

    let screen = hud_pause_screen(&input, &WorldStore::new(), None, surface).expect("pause screen");
    assert!(screen.show_pause_menu);
    assert!(screen.disconnect_hovered);
    assert!(screen.disconnect_enabled);

    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(position),
        surface,
    ));

    let screen = hud_pause_screen(&input, &WorldStore::new(), None, surface).expect("pause screen");
    assert!(!screen.disconnect_hovered);
    assert!(!screen.disconnect_enabled);
}
