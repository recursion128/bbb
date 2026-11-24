use super::*;

#[test]
fn the_level_loading_screen_projects_for_the_whole_wait_and_picks_the_portal_background() {
    let mut world = WorldStore::new();
    let terrain_textures = TerrainTextureState::default();
    assert!(hud_level_loading_screen(&world, &terrain_textures, None, 0.0).is_none());

    world.apply_login(&PlayLogin {
        player_id: 42,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 0,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: false,
    });
    let screen = hud_level_loading_screen(&world, &terrain_textures, None, 12.5)
        .expect("the login wait shows the level loading screen");
    assert_eq!(
        screen.background,
        HudLevelLoadingBackground::Panorama {
            rotation_degrees: 12.5
        }
    );
    assert_eq!(screen.downloading_terrain, "Loading terrain...");

    world.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id:
            bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID,
        param: 0.0,
    });
    assert!(hud_level_loading_screen(&world, &terrain_textures, None, 0.0).is_some());
    assert!(world.complete_client_level_load(false));
    assert!(hud_level_loading_screen(&world, &terrain_textures, None, 0.0).is_none());

    world.apply_respawn(&bbb_protocol::packets::Respawn {
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 2,
            dimension: "minecraft:the_end".to_string(),
            seed: 0,
            game_type: 0,
            previous_game_type: 0,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        data_to_keep: 0,
    });
    let screen = hud_level_loading_screen(&world, &terrain_textures, None, 0.0)
        .expect("the dimension change shows the level loading screen");
    assert_eq!(screen.background, HudLevelLoadingBackground::EndPortal);
}

#[test]
fn hud_scoreboard_projection_localizes_rich_team_and_score_runs() {
    use bbb_protocol::packets::{
        ChatFormatting, NumberFormatSummary, ObjectiveRenderType, PlayerTeamMethod,
        PlayerTeamParameters, ScoreboardDisplaySlot, SetDisplayObjective, SetObjective,
        SetObjectiveMethod, SetObjectiveParameters, SetPlayerTeam, SetScore, TeamCollisionRule,
        TeamVisibility,
    };

    let styled = |text: &str, color: Option<&str>, bold: bool| {
        let mut entries = vec![NbtSummaryEntry {
            name: "text".into(),
            value: NbtSummaryValue::string(text),
        }];
        if let Some(color) = color {
            entries.push(NbtSummaryEntry {
                name: "color".into(),
                value: NbtSummaryValue::string(color),
            });
        }
        if bold {
            entries.push(NbtSummaryEntry {
                name: "bold".into(),
                value: NbtSummaryValue::Byte(1),
            });
        }
        bbb_protocol::RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(entries))
            .unwrap()
    };
    let mut world = WorldStore::new();
    assert!(world.apply_set_objective(SetObjective {
        objective_name: "kills".into(),
        method: SetObjectiveMethod::Add.into(),
        parameters: Some(SetObjectiveParameters {
            display_name: styled("Kills", None, true),
            render_type: ObjectiveRenderType::Integer,
            number_format: None,
        }),
    }));
    assert!(world.apply_set_display_objective(SetDisplayObjective {
        slot: ScoreboardDisplaySlot::Sidebar,
        objective_name: Some("kills".into()),
    }));
    assert!(world.apply_set_player_team(SetPlayerTeam {
        name: "red".into(),
        method: PlayerTeamMethod::Add.into(),
        parameters: Some(PlayerTeamParameters {
            display_name: bbb_protocol::RichTextComponent::plain("Red"),
            options: 0,
            nametag_visibility: TeamVisibility::Always,
            collision_rule: TeamCollisionRule::Always,
            color: ChatFormatting::Red,
            player_prefix: bbb_protocol::RichTextComponent::plain("["),
            player_suffix: bbb_protocol::RichTextComponent::plain("]"),
        }),
        players: vec!["Alice".into()],
    }));
    assert!(world.apply_set_score(SetScore {
        owner: "Alice".into(),
        objective_name: "kills".into(),
        score: 9,
        display: Some(styled("Ace", Some("aqua"), false)),
        number_format: Some(NumberFormatSummary::Styled(
            bbb_protocol::RichTextStyle::default(),
        )),
    }));

    let mut input = ClientInputState::new(true);
    let sidebar = hud_scoreboard_sidebar_from_world(&input, &world, None)
        .expect("scoreboard sidebar projection");
    assert_eq!(sidebar.title_runs[0].text, "Kills");
    assert!(sidebar.title_runs[0].style.bold);
    assert_eq!(
        sidebar.entries[0]
            .name_runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<Vec<_>>(),
        ["[", "Ace", "]"]
    );
    assert_eq!(sidebar.entries[0].name_runs[0].color, Some(0xFF5555));
    assert_eq!(sidebar.entries[0].name_runs[1].color, Some(0x55FFFF));
    assert_eq!(sidebar.entries[0].name_runs[2].color, Some(0xFF5555));
    assert_eq!(sidebar.entries[0].score_runs[0].text, "9");
    assert_eq!(sidebar.entries[0].score_runs[0].color, None);

    crate::input::handle_key_input(
        &mut input,
        &mut NetCounters::default(),
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::F1),
        ElementState::Pressed,
    );
    assert_eq!(
        hud_scoreboard_sidebar_from_world(&input, &world, None),
        None
    );
}

#[test]
fn vibration_follows_entity_position_and_eye_offset_until_removal() {
    let mut world = WorldStore::new();
    world.apply_add_entity(test_add_entity(42, VANILLA_ENTITY_TYPE_PLAYER_ID));
    let mut particles = ParticleEngine::default();
    particles.spawn(
        ParticleSpawn::new(ParticleKind::VIBRATION, [0.0; 3], [0.0; 3]).with_params(
            ParticleParams::Vibration {
                target: Some([0.0; 3]),
                source: Some(bbb_client_world::particles::ParticleEntityTargetSource {
                    entity_id: 42,
                    y_offset: 0.5,
                }),
                arrival_ticks: Some(3),
            },
        ),
    );
    particles.advance(2, &world, ParticleTickContext::default());
    assert_eq!(
        particles.instances().next().unwrap().position,
        [0.5, 1.25, 1.5]
    );
    world.apply_remove_entities(bbb_protocol::packets::RemoveEntities {
        entity_ids: vec![42],
    });
    particles.advance(1, &world, ParticleTickContext::default());
    assert!(particles.is_empty());
}

#[test]
fn particle_light_for_world_samples_chunk_light_or_full_bright_fallback() {
    let missing = WorldStore::new();
    assert_eq!(
        particle_light_for_world(&missing, [0.5, 1.0, 0.5]),
        [0.0, 1.0]
    );

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_sky_light(
        world.dimension(),
        42,
        9,
    ));

    assert_eq!(
        particle_light_for_world(&world, [0.5, 1.25, 0.5]),
        [0.0, 9.0 / 15.0]
    );
}

#[test]
fn entity_animation_partial_tick_tracks_time_since_last_client_tick() {
    let now = Instant::now();
    let mut ticks = ClientAnimationTickState::default();
    let mut world = WorldStore::new();

    assert_eq!(ticks.entity_partial_tick(), 0.0);
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, now),
        0
    );
    assert_eq!(ticks.entity_partial_tick(), 0.0);
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, now + Duration::from_millis(25)),
        0
    );
    assert_eq!(ticks.entity_partial_tick(), 0.5);
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, now + Duration::from_millis(75)),
        1
    );
    assert_eq!(ticks.entity_partial_tick(), 0.5);
}

#[test]
fn entity_client_tick_observation_reports_recent_work_and_running_peak() {
    let mut counters = NetCounters::default();
    record_entity_client_tick_observation(
        &mut counters,
        Duration::from_micros(20),
        EntityClientTickWork {
            scanned_entities: 500,
            mutated_entities: 498,
            preparation_duration: Duration::from_micros(5),
            schedule_duration: Duration::from_micros(4),
            projectile_duration: Duration::from_micros(3),
            interpolation_duration: Duration::from_micros(2),
            block_effect_duration: Duration::from_micros(6),
            block_effect_entities: 42,
            block_effect_blocks_visited: 84,
            block_effect_callbacks_applied: 21,
            animation_duration: Duration::from_micros(1),
        },
    );
    record_entity_client_tick_observation(
        &mut counters,
        Duration::from_micros(10),
        EntityClientTickWork {
            scanned_entities: 100,
            mutated_entities: 100,
            preparation_duration: Duration::from_micros(4),
            schedule_duration: Duration::from_micros(3),
            projectile_duration: Duration::from_micros(2),
            interpolation_duration: Duration::from_micros(1),
            block_effect_duration: Duration::from_micros(4),
            block_effect_entities: 11,
            block_effect_blocks_visited: 22,
            block_effect_callbacks_applied: 7,
            animation_duration: Duration::from_nanos(500),
        },
    );

    assert_eq!(counters.simulation_entity_client_tick_nanos, 10_000);
    assert_eq!(counters.simulation_entity_client_tick_peak_nanos, 20_000);
    assert_eq!(
        counters.simulation_entity_client_tick_preparation_nanos,
        4_000
    );
    assert_eq!(
        counters.simulation_entity_client_tick_preparation_peak_nanos,
        5_000
    );
    assert_eq!(counters.simulation_entity_client_tick_schedule_nanos, 3_000);
    assert_eq!(
        counters.simulation_entity_client_tick_schedule_peak_nanos,
        4_000
    );
    assert_eq!(
        counters.simulation_entity_client_tick_projectile_nanos,
        2_000
    );
    assert_eq!(
        counters.simulation_entity_client_tick_projectile_peak_nanos,
        3_000
    );
    assert_eq!(
        counters.simulation_entity_client_tick_interpolation_nanos,
        1_000
    );
    assert_eq!(
        counters.simulation_entity_client_tick_interpolation_peak_nanos,
        2_000
    );
    assert_eq!(
        counters.simulation_entity_client_tick_block_effect_nanos,
        4_000
    );
    assert_eq!(
        counters.simulation_entity_client_tick_block_effect_peak_nanos,
        6_000
    );
    assert_eq!(
        counters.simulation_entity_client_tick_block_effect_entities,
        11
    );
    assert_eq!(
        counters.simulation_entity_client_tick_block_effect_blocks_visited,
        22
    );
    assert_eq!(
        counters.simulation_entity_client_tick_block_effect_callbacks_applied,
        7
    );
    assert_eq!(counters.simulation_entity_client_tick_animation_nanos, 500);
    assert_eq!(
        counters.simulation_entity_client_tick_animation_peak_nanos,
        1_000
    );
    assert_eq!(counters.simulation_entity_client_tick_scanned_entities, 100);
    assert_eq!(counters.simulation_entity_client_tick_mutated_entities, 100);
}

#[test]
fn lightmap_tick_state_matches_vanilla_block_light_flicker_formula() {
    let mut lightmap = LightmapTickState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);

    let first_delta = (expected_random.next_float() - expected_random.next_float())
        * expected_random.next_float()
        * expected_random.next_float()
        * 0.1;
    let first_flicker = first_delta * 0.9;
    let first_factor = lightmap.advance(1);
    assert!((first_factor - (VANILLA_DEFAULT_LIGHTMAP_BLOCK_FACTOR + first_flicker)).abs() < 1e-6);

    let second_delta = (expected_random.next_float() - expected_random.next_float())
        * expected_random.next_float()
        * expected_random.next_float()
        * 0.1;
    let second_flicker = (first_flicker + second_delta) * 0.9;
    let second_factor = lightmap.advance(1);
    assert!(
        (second_factor - (VANILLA_DEFAULT_LIGHTMAP_BLOCK_FACTOR + second_flicker)).abs() < 1e-6
    );
}

#[test]
fn lightmap_environment_uses_vanilla_dimension_attributes() {
    let overworld = world_with_dimension(0, "minecraft:overworld");
    let overworld_environment = lightmap_environment_for_world(&overworld, 0.75, 1.25);
    assert_eq!(overworld_environment.sky_factor, 1.0);
    assert_close3(
        overworld_environment.sky_light_color,
        VANILLA_DEFAULT_LIGHTMAP_SKY_LIGHT_COLOR,
    );
    assert_close3(overworld_environment.ambient_color, [10.0 / 255.0; 3]);
    assert_eq!(overworld_environment.brightness_factor, 0.75);
    assert_eq!(overworld_environment.block_factor, 1.25);
    assert_eq!(overworld_environment.level_lighting, LevelLighting::Default);

    let nether = world_with_dimension(1, "minecraft:the_nether");
    let nether_environment = lightmap_environment_for_world(&nether, 0.5, 1.4);
    assert_eq!(nether_environment.sky_factor, 0.0);
    assert_close3(
        nether_environment.sky_light_color,
        [122.0 / 255.0, 122.0 / 255.0, 1.0],
    );
    assert_close3(
        nether_environment.ambient_color,
        [48.0 / 255.0, 40.0 / 255.0, 33.0 / 255.0],
    );
    assert_eq!(nether_environment.level_lighting, LevelLighting::Nether);

    let end = world_with_dimension(2, "minecraft:the_end");
    let end_environment = lightmap_environment_for_world(&end, 0.5, 1.4);
    assert_eq!(end_environment.sky_factor, 0.0);
    assert_close3(
        end_environment.sky_light_color,
        [172.0 / 255.0, 96.0 / 255.0, 205.0 / 255.0],
    );
    assert_close3(
        end_environment.ambient_color,
        [63.0 / 255.0, 71.0 / 255.0, 63.0 / 255.0],
    );
    assert_eq!(end_environment.level_lighting, LevelLighting::Default);
}

#[test]
fn lightmap_environment_uses_overworld_day_timeline_sky_light_attributes() {
    let mut overworld = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut overworld, 18_000);
    let night_environment = lightmap_environment_for_world(&overworld, 0.5, 1.4);
    assert_eq!(night_environment.sky_factor, 0.24);
    assert_close3(
        night_environment.sky_light_color,
        [122.0 / 255.0, 122.0 / 255.0, 1.0],
    );

    set_world_day_time(&mut overworld, 6_000);
    let noon_environment = lightmap_environment_for_world(&overworld, 0.5, 1.4);
    assert_eq!(noon_environment.sky_factor, 1.0);
    assert_close3(
        noon_environment.sky_light_color,
        VANILLA_DEFAULT_LIGHTMAP_SKY_LIGHT_COLOR,
    );

    let mut nether = world_with_dimension(1, "minecraft:the_nether");
    set_world_day_time(&mut nether, 18_000);
    let nether_environment = lightmap_environment_for_world(&nether, 0.5, 1.4);
    assert_eq!(nether_environment.sky_factor, 0.0);
    assert_close3(
        nether_environment.sky_light_color,
        [122.0 / 255.0, 122.0 / 255.0, 1.0],
    );
}

#[test]
fn lightmap_environment_applies_overworld_weather_layers_after_timeline() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    set_world_weather(&mut world, 1.0, 0.0);
    let rain_environment = lightmap_environment_for_world(&world, 0.5, 1.4);
    assert!((rain_environment.sky_factor - 0.7625).abs() < 1e-6);
    assert_close3(
        rain_environment.sky_light_color,
        [213.0 / 255.0, 213.0 / 255.0, 1.0],
    );

    set_world_weather(&mut world, 1.0, 1.0);
    let thunder_environment = lightmap_environment_for_world(&world, 0.5, 1.4);
    assert!((thunder_environment.sky_factor - 0.5992187).abs() < 1e-6);
    assert_close3(
        thunder_environment.sky_light_color,
        [185.0 / 255.0, 185.0 / 255.0, 1.0],
    );

    set_world_weather(&mut world, 0.0, 1.0);
    let dry_thunder_environment = lightmap_environment_for_world(&world, 0.5, 1.4);
    assert_eq!(dry_thunder_environment.sky_factor, 1.0);
    assert_close3(
        dry_thunder_environment.sky_light_color,
        VANILLA_DEFAULT_LIGHTMAP_SKY_LIGHT_COLOR,
    );
}

#[test]
fn entity_shadow_sky_darken_uses_gameplay_timeline_and_weather_layers() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    assert_eq!(entity_shadow_sky_darken(&world), 0);

    set_world_day_time(&mut world, 18_000);
    assert_eq!(entity_shadow_sky_darken(&world), 11);

    set_world_day_time(&mut world, 6_000);
    set_world_weather(&mut world, 1.0, 0.0);
    assert_eq!(entity_shadow_sky_darken(&world), 3);
    set_world_weather(&mut world, 1.0, 1.0);
    assert_eq!(entity_shadow_sky_darken(&world), 5);
    set_world_weather(&mut world, 0.0, 1.0);
    assert_eq!(entity_shadow_sky_darken(&world), 0);
}

#[test]
fn lightmap_tick_state_smooths_rain_fog_multiplier_like_vanilla_atmospheric_fog() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_sky_light(
        world.dimension(),
        0,
        15,
    ));
    let textures = TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
        biome_profile_with_precipitation(0, true),
    ]));
    set_world_weather(&mut world, 1.0, 0.0);

    let mut lightmap = LightmapTickState::with_seed(0);
    lightmap.advance_rain_fog_for_world(1, &world, &textures);
    assert!((lightmap.rain_fog_multiplier() - 0.2).abs() < 1e-6);

    lightmap.advance_rain_fog_for_world(1, &world, &textures);
    assert!((lightmap.rain_fog_multiplier() - 0.36).abs() < 1e-6);

    set_world_weather(&mut world, 0.0, 0.0);
    lightmap.advance_rain_fog_for_world(1, &world, &textures);
    assert!((lightmap.rain_fog_multiplier() - 0.288).abs() < 1e-6);
}

#[test]
fn lightmap_tick_state_gates_rain_fog_multiplier_by_camera_sky_light() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_sky_light(
        world.dimension(),
        0,
        8,
    ));
    assert_eq!(
        camera_block_position(&world),
        Some(BlockPos { x: 0, y: 1, z: 0 })
    );
    assert_eq!(
        world
            .sample_block_light(BlockPos { x: 0, y: 1, z: 0 })
            .unwrap()
            .sky,
        8
    );
    let textures = TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
        biome_profile_with_precipitation(0, true),
    ]));
    set_world_weather(&mut world, 1.0, 0.0);

    let mut lightmap = LightmapTickState::with_seed(0);
    lightmap.advance_rain_fog_for_world(1, &world, &textures);

    assert_eq!(lightmap.rain_fog_multiplier(), 0.0);
}

#[test]
fn lightmap_tick_state_halves_rain_fog_target_when_camera_biome_has_no_precipitation() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_sky_light(
        world.dimension(),
        42,
        15,
    ));
    let textures = TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
        biome_profile_with_precipitation(42, false),
    ]));
    set_world_weather(&mut world, 1.0, 0.0);

    let mut lightmap = LightmapTickState::with_seed(0);
    lightmap.advance_rain_fog_for_world(1, &world, &textures);

    assert!((lightmap.rain_fog_multiplier() - 0.1).abs() < 1e-6);
}

#[test]
fn lightmap_environment_applies_client_sky_flash_layer_after_timeline() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 18_000);
    let baseline_environment = lightmap_environment_for_world(&world, 0.5, 1.4);
    assert!(baseline_environment.sky_factor < 1.0);

    world.set_sky_flash_time(2);
    let mut lightmap =
        LightmapTickState::with_brightness_factor_and_hide_lightning_flash(0.5, false);
    lightmap.advance_for_world(0, &world);
    let environment = lightmap.environment_for_world(&world);

    assert_eq!(environment.sky_factor, 1.0);
    assert_close3(
        environment.sky_light_color,
        baseline_environment.sky_light_color,
    );
}

#[test]
fn lightmap_environment_hides_client_sky_flash_when_option_is_enabled() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 18_000);
    let baseline_environment = lightmap_environment_for_world(&world, 0.5, 1.4);
    world.set_sky_flash_time(2);
    let mut lightmap =
        LightmapTickState::with_brightness_factor_and_hide_lightning_flash(0.5, true);

    lightmap.advance_for_world(0, &world);
    let environment = lightmap.environment_for_world(&world);

    assert!((environment.sky_factor - baseline_environment.sky_factor).abs() < 1e-6);
}

#[test]
fn renderer_frame_sky_flash_environment_extracts_after_client_level_tick() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 18_000);
    world.set_sky_flash_time(1);
    world.advance_sky_flash_time(1);
    let mut lightmap =
        LightmapTickState::with_brightness_factor_and_hide_lightning_flash(0.5, false);
    lightmap.advance_for_world(0, &world);

    let baseline = lightmap_environment_for_world(&world, 0.5, 1.4);
    let environment = lightmap.environment_for_world(&world);

    assert!((environment.sky_factor - baseline.sky_factor).abs() < 1e-6);
}

#[test]
fn animate_player_wake_sleep_overlay_alpha_matches_vanilla_boundaries() {
    use bbb_client_render::HudSleepOverlay;

    assert_eq!(hud_sleep_overlay_from_counter(-1), None);
    assert_eq!(hud_sleep_overlay_from_counter(0), None);
    assert_eq!(
        hud_sleep_overlay_from_counter(1),
        Some(HudSleepOverlay { alpha: 2 })
    );
    assert_eq!(
        hud_sleep_overlay_from_counter(50),
        Some(HudSleepOverlay { alpha: 110 })
    );
    assert_eq!(
        hud_sleep_overlay_from_counter(99),
        Some(HudSleepOverlay { alpha: 217 })
    );
    assert_eq!(
        hud_sleep_overlay_from_counter(100),
        Some(HudSleepOverlay { alpha: 220 })
    );
    assert_eq!(
        hud_sleep_overlay_from_counter(101),
        Some(HudSleepOverlay { alpha: 198 })
    );
    assert_eq!(
        hud_sleep_overlay_from_counter(109),
        Some(HudSleepOverlay { alpha: 22 })
    );
    assert_eq!(hud_sleep_overlay_from_counter(110), None);
}

#[test]
fn camera_fov_tick_state_eases_player_abilities_modifier_like_vanilla() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: false,
        flying: false,
        can_fly: false,
        instabuild: false,
        flying_speed: 0.05,
        walking_speed: 0.2,
    });
    let mut fov = CameraFovTickState::default();

    assert!((fov.unmodified_fov_degrees(1.0) - 70.0).abs() < 1.0e-6);
    fov.advance_for_world(1, &world, true, false);
    assert!((fov.unmodified_fov_degrees(0.0) - 70.0).abs() < 1.0e-6);
    assert!((fov.unmodified_fov_degrees(1.0) - 61.25).abs() < 1.0e-6);

    fov.advance_for_world(1, &world, true, false);
    assert!((fov.unmodified_fov_degrees(1.0) - 56.875).abs() < 1.0e-6);

    world.apply_add_entity(test_add_entity(100, VANILLA_ENTITY_TYPE_CREEPER_ID));
    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 100 }));
    fov.advance_for_world(1, &world, true, false);
    assert!((fov.unmodified_fov_degrees(1.0) - 63.4375).abs() < 1.0e-6);

    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 42 }));
    world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: false,
        flying: false,
        can_fly: false,
        instabuild: false,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_BOW_ID, 1),
    });
    world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    world.advance_local_using_item_ticks(10);
    let mut bow_fov = CameraFovTickState::default();
    bow_fov.advance_for_world(1, &world, true, false);
    assert!((bow_fov.unmodified_fov_degrees(1.0) - 68.6875).abs() < 1.0e-6);

    world.take_local_using_item();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_SPYGLASS_ID, 1),
    });
    world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    let mut spyglass_fov = CameraFovTickState::default();
    spyglass_fov.advance_for_world(1, &world, true, false);
    assert!((spyglass_fov.unmodified_fov_degrees(1.0) - 38.5).abs() < 1.0e-6);
}

#[test]
fn camera_fov_tick_state_includes_effective_sprint_for_ground_and_swim_sprint() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: false,
        flying: false,
        can_fly: false,
        instabuild: false,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });

    let mut ground_sprint = CameraFovTickState::default();
    ground_sprint.advance_for_world(1, &world, true, true);
    assert!((ground_sprint.unmodified_fov_degrees(1.0) - 75.25).abs() < 1.0e-6);

    world.set_local_player_pose(LocalPlayerPoseState {
        swimming: true,
        ..LocalPlayerPoseState::default()
    });
    let mut swim_sprint = CameraFovTickState::default();
    swim_sprint.advance_for_world(1, &world, true, true);
    assert!((swim_sprint.unmodified_fov_degrees(1.0) - 75.25).abs() < 1.0e-6);

    let mut not_sprinting = CameraFovTickState::default();
    not_sprinting.advance_for_world(1, &world, true, false);
    assert!((not_sprinting.unmodified_fov_degrees(1.0) - 70.0).abs() < 1.0e-6);
}

#[test]
fn camera_perspective_third_person_suppresses_first_person_item_fov() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_SPYGLASS_ID, 1),
    });
    world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    let mut fov = CameraFovTickState::default();

    fov.advance_for_world(1, &world, false, false);

    assert!((fov.unmodified_fov_degrees(1.0) - 70.0).abs() < 1.0e-6);
}

#[test]
fn camera_fov_frame_separates_configured_world_culling_and_hud_angles() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: false,
        flying: false,
        can_fly: false,
        instabuild: false,
        flying_speed: 0.05,
        walking_speed: 0.2,
    });
    let mut fov = CameraFovTickState::new(90.0, 1.0);
    fov.advance_for_world(1, &world, true, false);

    let frame = fov.frame(None, CameraFluid::None, 1.0);

    assert!((frame.world_degrees - 78.75).abs() < 1.0e-6);
    assert_eq!(frame.culling_degrees, 90.0);
    assert_eq!(frame.hud_degrees, 70.0);

    let mut disabled_effects = CameraFovTickState::new(90.0, 0.0);
    disabled_effects.advance_for_world(1, &world, true, false);
    let frame = disabled_effects.frame(None, CameraFluid::None, 1.0);
    assert_eq!(frame.world_degrees, 90.0);
    assert_eq!(frame.culling_degrees, 90.0);
    assert_eq!(frame.hud_degrees, 70.0);
}

#[test]
fn camera_fov_frame_applies_death_to_world_and_fixed_base_hud() {
    let fov = CameraFovTickState::new(90.0, 1.0);
    let death = bbb_client_render::CameraHurtEffect {
        hurt_time: 0.0,
        hurt_duration: 0.0,
        hurt_dir: 0.0,
        death_time: 20.0,
        is_dead_or_dying: true,
        damage_tilt_strength: 1.0,
    };

    let frame = fov.frame(Some(death), CameraFluid::None, 1.0);

    assert!((frame.world_degrees - 83.571_43).abs() < 1.0e-5);
    assert_eq!(frame.culling_degrees, 90.0);
    assert!((frame.hud_degrees - 65.0).abs() < 1.0e-5);
}

#[test]
fn camera_fov_frame_samples_water_at_eye_and_lava_on_base_fov_near_plane() {
    let mut water_world = world_with_dimension(0, "minecraft:overworld");
    water_world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    water_world.insert_decoded_chunk(empty_lightmap_test_chunk(water_world.dimension()));
    set_lightmap_test_block(
        &mut water_world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    let water_pose = camera_pose_from_world(&water_world).unwrap();
    let water_fluid = camera_fluid_in_camera(&water_world, water_pose, [1920, 1080], 70.0);
    let water_frame = CameraFovTickState::default().frame(None, water_fluid, 1.0);
    assert!((water_frame.world_degrees - 60.0).abs() < 1.0e-5);
    assert_eq!(water_frame.culling_degrees, 70.0);
    assert!((water_frame.hud_degrees - 60.0).abs() < 1.0e-5);

    let mut lava_world = world_with_dimension(0, "minecraft:overworld");
    lava_world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.97], 0.0, 0.0));
    lava_world.insert_decoded_chunk(empty_lightmap_test_chunk(lava_world.dimension()));
    let source_lava = lava_world
        .registries()
        .block_state_id_by_name_and_properties(
            "minecraft:lava",
            &BTreeMap::from([("level".to_string(), "0".to_string())]),
        )
        .expect("vanilla source lava state");
    set_lightmap_test_block(&mut lava_world, BlockPos { x: 0, y: 1, z: 1 }, source_lava);
    let lava_pose = camera_pose_from_world(&lava_world).unwrap();
    let lava_fluid = camera_fluid_in_camera(&lava_world, lava_pose, [1920, 1080], 70.0);
    let lava_frame = CameraFovTickState::default().frame(None, lava_fluid, 1.0);
    assert!((lava_frame.world_degrees - 60.0).abs() < 1.0e-5);
    assert_eq!(lava_frame.culling_degrees, 70.0);
    assert!((lava_frame.hud_degrees - 60.0).abs() < 1.0e-5);
}

#[test]
fn camera_fluid_sampler_uses_water_priority_viewport_base_fov_and_inclusive_lava_surface() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.97, 0.0, 0.97], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let source_lava = world
        .registries()
        .block_state_id_by_name_and_properties(
            "minecraft:lava",
            &BTreeMap::from([("level".to_string(), "0".to_string())]),
        )
        .expect("vanilla source lava state");
    set_lightmap_test_block(&mut world, BlockPos { x: 1, y: 1, z: 1 }, source_lava);
    let camera = camera_pose_from_world(&world).unwrap();

    assert_eq!(
        camera_fluid_in_camera(&world, camera, [1920, 1080], 70.0),
        CameraFluid::Lava,
        "a wide base-FOV corner reaches the adjacent lava cell"
    );
    assert_eq!(
        camera_fluid_in_camera(&world, camera, [1, 4], 70.0),
        CameraFluid::None,
        "the actual narrow viewport keeps every corner in the camera cell"
    );
    assert_eq!(
        camera_fluid_in_camera(&world, camera, [1920, 1080], 30.0),
        CameraFluid::None,
        "Camera.getNearPlane consumes the configured base FOV"
    );

    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    assert_eq!(
        camera_fluid_in_camera(&world, camera, [1920, 1080], 70.0),
        CameraFluid::Water,
        "the eye-point water check precedes all near-plane lava samples"
    );

    let mut boundary_world = world_with_dimension(0, "minecraft:overworld");
    boundary_world.insert_decoded_chunk(empty_lightmap_test_chunk(boundary_world.dimension()));
    set_lightmap_test_block(
        &mut boundary_world,
        BlockPos { x: 0, y: 1, z: 1 },
        source_lava,
    );
    let surface = (1.0_f64 + 8.0 / 9.0) as f32;
    let on_surface = CameraPose {
        position: [0.5, surface, 0.97],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 0.0,
    };
    // A zero-height near plane isolates the inclusive surface predicate from
    // the top/bottom corner offsets already covered above.
    assert_eq!(
        camera_fluid_in_camera(&boundary_world, on_surface, [1920, 1080], 0.0),
        CameraFluid::Lava
    );
    let above_surface = CameraPose {
        position: [
            on_surface.position[0],
            f32::from_bits(surface.to_bits() + 1),
            on_surface.position[2],
        ],
        ..on_surface
    };
    assert_eq!(
        camera_fluid_in_camera(&boundary_world, above_surface, [1920, 1080], 0.0),
        CameraFluid::None
    );
}

#[test]
fn camera_powder_snow_near_plane_matches_vanilla_sample_order_without_fov_scaling() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.97, 0.0, 0.97], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let powder_snow = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:powder_snow", &BTreeMap::new())
        .expect("vanilla powder snow state");
    let source_lava = world
        .registries()
        .block_state_id_by_name_and_properties(
            "minecraft:lava",
            &BTreeMap::from([("level".to_string(), "0".to_string())]),
        )
        .expect("vanilla source lava state");
    set_lightmap_test_block(&mut world, BlockPos { x: 0, y: 1, z: 1 }, powder_snow);
    set_lightmap_test_block(&mut world, BlockPos { x: 1, y: 1, z: 1 }, source_lava);
    let camera = camera_pose_from_world(&world).unwrap();

    let medium = camera_fluid_in_camera(&world, camera, [1920, 1080], 70.0);
    assert_eq!(
        medium,
        CameraFluid::PowderSnow,
        "the center sample returns powder snow before a later corner reaches lava"
    );
    let frame = CameraFovTickState::default().frame(None, medium, 1.0);
    assert_eq!(frame.world_degrees, 70.0);
    assert_eq!(frame.culling_degrees, 70.0);
    assert_eq!(frame.hud_degrees, 70.0);
}

#[test]
fn camera_spinning_effect_ticks_and_interpolates_nausea_like_vanilla() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(world.apply_update_mob_effect(mob_effect_with_blend(
        42,
        VANILLA_MOB_EFFECT_NAUSEA_ID,
        260,
        true,
    )));
    let mut spinning = CameraSpinningEffectTickState::default();

    assert_eq!(spinning.effect_for_world(&world, 1.0), None);
    world.advance_client_level_entity_ticks_with_observation(1);
    spinning.advance_for_world(1, &world);

    assert_eq!(spinning.effect_for_world(&world, 0.0), None);
    let effect = spinning.effect_for_world(&world, 0.5).unwrap();
    assert!((effect.intensity - 1.0 / 300.0).abs() < 1e-6);
    assert_eq!(effect.angle_degrees, 10.5);
    let effect = spinning.effect_for_world(&world, 1.0).unwrap();
    assert!((effect.intensity - 1.0 / 150.0).abs() < 1e-6);
    assert_eq!(effect.angle_degrees, 14.0);
}

#[test]
fn camera_spinning_effect_blends_portal_intensity_like_vanilla() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id:
            bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID,
        param: 0.0,
    });
    world.complete_client_level_load(false);
    attach_lightmap_local_player(&mut world, 42);
    world.set_local_player_pose(local_player_pose([0.5, 1.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let portal = world
        .registries()
        .block_state_id_by_name_and_properties(
            "minecraft:nether_portal",
            &BTreeMap::from([("axis".to_string(), "x".to_string())]),
        )
        .expect("vanilla nether portal state");
    set_lightmap_test_block(&mut world, BlockPos { x: 0, y: 1, z: 0 }, portal);
    set_lightmap_test_block(&mut world, BlockPos { x: 0, y: 2, z: 0 }, portal);

    let mut spinning = CameraSpinningEffectTickState::default();
    world.advance_client_level_entity_ticks_with_observation(1);
    spinning.advance_for_world(1, &world);

    // A pure portal source spins at the vanilla-weighted 20 deg/tick.
    assert_eq!(spinning.effect_for_world(&world, 0.0), None);
    let effect = spinning.effect_for_world(&world, 1.0).unwrap();
    assert!((effect.intensity - 0.0125).abs() < 1e-6);
    assert_eq!(effect.angle_degrees, 40.0);

    assert!(world.apply_update_mob_effect(mob_effect_with_blend(
        42,
        VANILLA_MOB_EFFECT_NAUSEA_ID,
        260,
        true,
    )));
    world.advance_client_level_entity_ticks_with_observation(1);
    spinning.advance_for_world(1, &world);

    let portal_intensity = 0.025_f32;
    let nausea_intensity = 1.0_f32 / 150.0;
    let blended_speed =
        (portal_intensity * 20.0 + nausea_intensity * 7.0) / (portal_intensity + nausea_intensity);
    let effect = spinning.effect_for_world(&world, 1.0).unwrap();
    assert!((effect.intensity - portal_intensity).abs() < 1e-6);
    assert!((effect.angle_degrees - (20.0 + 2.0 * blended_speed)).abs() < 1e-4);
}

#[test]
fn camera_spinning_effect_stops_without_resetting_phase_after_nausea_removal() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_NAUSEA_ID, 260,)));
    let mut spinning = CameraSpinningEffectTickState::default();
    spinning.advance_for_world(2, &world);
    assert_eq!(
        spinning
            .effect_for_world(&world, 1.0)
            .unwrap()
            .angle_degrees,
        21.0
    );

    assert!(world.apply_remove_mob_effect(RemoveMobEffect {
        entity_id: 42,
        effect_id: VANILLA_MOB_EFFECT_NAUSEA_ID,
    }));
    spinning.advance_for_world(3, &world);
    assert_eq!(spinning.effect_for_world(&world, 1.0), None);

    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_NAUSEA_ID, 260,)));
    spinning.advance_for_world(1, &world);
    assert_eq!(
        spinning
            .effect_for_world(&world, 0.0)
            .unwrap()
            .angle_degrees,
        21.0
    );
}

#[test]
fn hud_mob_effect_projection_uses_catalog_category_flags_level_and_duration() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    let mut speed = mob_effect(42, VANILLA_MOB_EFFECT_SPEED_ID, 1_220);
    speed.amplifier = 1;
    speed.flags = MobEffectFlags {
        raw: 0b0011,
        ambient: true,
        visible: true,
        show_icon: false,
        blend: false,
    };
    assert!(world.apply_update_mob_effect(speed));
    world.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 10.0,
        frozen: false,
    });
    let mut blindness = mob_effect(42, VANILLA_MOB_EFFECT_BLINDNESS_ID, -1);
    blindness.amplifier = 10;
    assert!(world.apply_update_mob_effect(blindness));

    let effects = hud_mob_effects_from_world(&world, None);
    assert_eq!(effects.len(), 2);
    let speed = effects
        .iter()
        .find(|effect| effect.effect_id == VANILLA_MOB_EFFECT_SPEED_ID)
        .expect("speed effect");
    assert!(speed.beneficial);
    assert_eq!(
        speed.color,
        vanilla_mob_effect_type(VANILLA_MOB_EFFECT_SPEED_ID)
            .unwrap()
            .color
    );
    assert_eq!(speed.amplifier, 1);
    assert_eq!(speed.duration_ticks, 1_220);
    assert!(speed.ambient);
    assert!(!speed.show_icon);
    assert_eq!(speed.display_name, "Speed II");
    assert_eq!(speed.duration_text, "02:02");

    let blindness = effects
        .iter()
        .find(|effect| effect.effect_id == VANILLA_MOB_EFFECT_BLINDNESS_ID)
        .expect("blindness effect");
    assert!(!blindness.beneficial);
    assert_eq!(blindness.display_name, "Blindness");
    assert_eq!(blindness.duration_text, "∞");
}

#[test]
fn hud_mob_effect_projection_requires_local_player_and_formats_vanilla_time_ranges() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.apply_add_entity(test_add_entity(100, VANILLA_ENTITY_TYPE_CHICKEN_ID));
    assert!(world.apply_update_mob_effect(mob_effect(100, VANILLA_MOB_EFFECT_SPEED_ID, 200,)));
    assert!(hud_mob_effects_from_world(&world, None).is_empty());

    assert_eq!(hud_mob_effect_duration_text(None, 20, 20.0), "00:01");
    assert_eq!(hud_mob_effect_duration_text(None, 72_020, 20.0), "01:00:01");
    assert_eq!(hud_mob_effect_duration_text(None, 20, 0.0), "00:01");
    assert_eq!(
        humanize_resource_path("hero_of_the_village"),
        "Hero Of The Village"
    );
}

#[test]
fn hud_action_bar_and_title_projection_matches_world_state() {
    let mut world = WorldStore::new();
    assert_eq!(hud_action_bar_text_from_world(&world, None, 0.25), None);
    assert_eq!(hud_title_text_from_world(&world, None, 0.25), None);

    world.apply_action_bar_text(bbb_protocol::packets::SetActionBarText::plain(
        "Action ready",
    ));
    world.apply_titles_animation(bbb_protocol::packets::SetTitlesAnimation {
        fade_in: 5,
        stay: 40,
        fade_out: 15,
    });
    world.apply_title_text(bbb_protocol::packets::SetTitleText::plain("Quest complete"));
    world.apply_subtitle_text(bbb_protocol::packets::SetSubtitleText::plain(
        "Return to camp",
    ));

    let action_bar =
        hud_action_bar_text_from_world(&world, None, 0.25).expect("action bar projected");
    assert_eq!(
        action_bar.runs,
        vec![bbb_client_render::HudComponentTextRun::plain(
            "Action ready"
        )]
    );
    assert_eq!(action_bar.remaining_ticks, 60);
    assert_eq!(action_bar.partial_tick, 0.25);
    // Both packet paths are vanilla `setOverlayMessage(component, false)`;
    // only the jukebox now-playing path animates.
    assert!(!action_bar.animate_color);

    let title = hud_title_text_from_world(&world, None, 0.25).expect("title projected");
    assert_eq!(
        title.title_runs,
        vec![bbb_client_render::HudComponentTextRun::plain(
            "Quest complete"
        )]
    );
    assert_eq!(
        title.subtitle_runs,
        vec![bbb_client_render::HudComponentTextRun::plain(
            "Return to camp"
        )]
    );
    assert_eq!(title.remaining_ticks, 60);
    assert_eq!((title.fade_in, title.stay, title.fade_out), (5, 40, 15));
    assert_eq!(title.partial_tick, 0.25);

    // Post-tick countdowns flow through; expired timers stop projecting.
    world.advance_hud_text_ticks(59);
    assert_eq!(
        hud_action_bar_text_from_world(&world, None, 0.0)
            .expect("last action bar tick")
            .remaining_ticks,
        1
    );
    assert_eq!(
        hud_title_text_from_world(&world, None, 0.0)
            .expect("last title tick")
            .remaining_ticks,
        1
    );
    world.advance_hud_text_ticks(1);
    assert_eq!(hud_action_bar_text_from_world(&world, None, 0.0), None);
    assert_eq!(hud_title_text_from_world(&world, None, 0.0), None);

    // A re-set title without a subtitle projects an empty subtitle line.
    world.apply_title_text(bbb_protocol::packets::SetTitleText::plain("Solo"));
    let solo = hud_title_text_from_world(&world, None, 0.5).expect("title without subtitle");
    assert!(solo.subtitle_runs.is_empty());
    assert_eq!(solo.remaining_ticks, 60);

    // A subtitle without an active title never projects (vanilla draws the
    // subtitle only inside the title branch, Gui).
    world.apply_clear_titles(bbb_protocol::packets::ClearTitles { reset_times: false });
    world.apply_subtitle_text(bbb_protocol::packets::SetSubtitleText::plain("Orphan"));
    assert_eq!(hud_title_text_from_world(&world, None, 0.0), None);
}

#[test]
fn action_bar_and_title_runs_consume_legacy_codes_with_the_root_style_as_reset() {
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue, RichTextComponent};
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let styled = |text: &str, extra: Vec<NbtSummaryEntry>| {
        let mut entries = vec![entry("text", NbtSummaryValue::String(text.into()))];
        entries.extend(extra);
        RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(entries)).unwrap()
    };

    let mut world = WorldStore::new();
    world.apply_action_bar_text(bbb_protocol::packets::SetActionBarText {
        content: styled(
            "AB§cCD§rEF§?GH§",
            vec![entry("bold", NbtSummaryValue::Byte(1))],
        ),
    });
    world.apply_titles_animation(bbb_protocol::packets::SetTitlesAnimation {
        fade_in: 5,
        stay: 40,
        fade_out: 15,
    });
    world.apply_title_text(bbb_protocol::packets::SetTitleText {
        content: styled("T§lU§rV", Vec::new()),
    });
    world.apply_subtitle_text(bbb_protocol::packets::SetSubtitleText {
        content: styled("§aW", Vec::new()),
    });

    let action_bar =
        hud_action_bar_text_from_world(&world, None, 0.25).expect("action bar projected");
    // `SubStringSource` with the part style inherited: the codes are
    // consumed, `§r` returns to that inherited root style, and no run carries
    // the `§`. `EF` and `GH` merge because they resolved to the same style.
    assert_eq!(
        action_bar
            .runs
            .iter()
            .map(|run| run.visual.text.as_str())
            .collect::<Vec<_>>(),
        ["AB", "CD", "EFGH"]
    );
    let root = bbb_client_render::HudTextStyle {
        bold: true,
        ..bbb_client_render::HudTextStyle::default()
    };
    assert_eq!(action_bar.runs[0].visual.style, root);
    assert_eq!(action_bar.runs[0].visual.color, None);
    // A colour code clears the five decorations; the font survives.
    assert_eq!(action_bar.runs[1].visual.color, Some(0xff_55_55));
    assert_eq!(
        action_bar.runs[1].visual.style,
        bbb_client_render::HudTextStyle::default()
    );
    // The unknown code changed nothing and the trailing `§` terminated.
    assert_eq!(action_bar.runs[2].visual.style, root);
    assert_eq!(action_bar.runs[2].visual.color, None);

    let title = hud_title_text_from_world(&world, None, 0.25).expect("title projected");
    assert_eq!(
        title
            .title_runs
            .iter()
            .map(|run| run.visual.text.as_str())
            .collect::<Vec<_>>(),
        ["T", "U", "V"]
    );
    assert!(!title.title_runs[0].visual.style.bold);
    assert!(title.title_runs[1].visual.style.bold);
    assert!(!title.title_runs[2].visual.style.bold);
    assert_eq!(title.subtitle_runs[0].visual.text, "W");
    assert_eq!(title.subtitle_runs[0].visual.color, Some(0x55_ff_55));
}

#[test]
fn jukebox_now_playing_localizes_for_animated_action_bar_and_system_now_narration() {
    let root = unique_runtime_temp_dir("jukebox-now-playing-language");
    write_runtime_bow_item_assets(&root);
    write_runtime_json(
        &runtime_assets_dir(&root).join("lang").join("en_us.json"),
        r#"{
            "record.nowPlaying": "Now Playing: %s",
            "jukebox_song.minecraft.cat": "C418 - cat"
        }"#,
    );
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();

    world.apply_level_event(bbb_protocol::packets::LevelEvent {
        event_type: 1010,
        pos: ProtocolBlockPos { x: 4, y: 64, z: -7 },
        data: 1,
        global: false,
    });

    let action_bar = hud_action_bar_text_from_world(&world, Some(&item_runtime), 0.25)
        .expect("jukebox action bar projected");
    assert_eq!(
        action_bar
            .runs
            .iter()
            .map(|run| run.visual.text.as_str())
            .collect::<String>(),
        "Now Playing: C418 - cat"
    );
    assert_eq!(action_bar.remaining_ticks, 60);
    assert_eq!(action_bar.partial_tick, 0.25);
    assert!(action_bar.animate_color);

    let narrations = world.take_pending_chat_narrations();
    assert_eq!(narrations.len(), 1);
    assert_eq!(
        narrations[0].kind,
        bbb_client_world::ChatNarrationKind::SystemNow
    );
    assert_eq!(
        chat_narration_text(&narrations[0], Some(&item_runtime)),
        "Now Playing: C418 - cat"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_action_bar_and_title_projection_localizes_style_and_font_without_interactions() {
    let component =
        bbb_protocol::RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "translate".into(),
                value: NbtSummaryValue::string("missing.action"),
            },
            NbtSummaryEntry {
                name: "fallback".into(),
                value: NbtSummaryValue::string("Ready, %s"),
            },
            NbtSummaryEntry {
                name: "with".into(),
                value: NbtSummaryValue::List(vec![NbtSummaryValue::string("Alex")]),
            },
            NbtSummaryEntry {
                name: "bold".into(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "italic".into(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "underlined".into(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "strikethrough".into(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "obfuscated".into(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "color".into(),
                value: NbtSummaryValue::string("#123456"),
            },
            NbtSummaryEntry {
                name: "font".into(),
                value: NbtSummaryValue::string("example:title"),
            },
            NbtSummaryEntry {
                name: "click_event".into(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "action".into(),
                        value: NbtSummaryValue::string("run_command"),
                    },
                    NbtSummaryEntry {
                        name: "command".into(),
                        value: NbtSummaryValue::string("/spawn"),
                    },
                ]),
            },
            NbtSummaryEntry {
                name: "hover_event".into(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "action".into(),
                        value: NbtSummaryValue::string("show_text"),
                    },
                    NbtSummaryEntry {
                        name: "value".into(),
                        value: NbtSummaryValue::string("Details"),
                    },
                ]),
            },
        ]))
        .unwrap();
    // `TranslatableContents.decomposeTemplate` emits the
    // literal segments and each `with` argument as separate parts (`getArgument`),
    // and `visit` hands every part the same `currentStyle`. Equal-styled parts
    // stay separate runs because the ordinals are the interaction and per-part legacy-reset
    // identities; equal styles merge only later, when the bidi pieces are cut for the font
    let style = HudTextStyle {
        bold: true,
        italic: true,
        underlined: true,
        strikethrough: true,
        obfuscated: true,
    };
    let part = |text: &str| bbb_client_render::HudComponentTextRun {
        visual: HudStyledTextRun {
            shadow_color: None,
            text: text.to_string(),
            style,
            color: Some(0x12_34_56),
        },
        font: Some("example:title".to_string()),
    };
    let expected = vec![part("Ready, "), part("Alex")];
    let mut world = WorldStore::new();
    world.apply_system_chat(bbb_protocol::packets::SystemChat {
        content: Box::new(component.clone()),
        overlay: true,
    });
    world.apply_title_text(bbb_protocol::packets::SetTitleText {
        content: component.clone(),
    });
    world.apply_subtitle_text(bbb_protocol::packets::SetSubtitleText { content: component });

    let runs = hud_action_bar_text_from_world(&world, None, 0.0)
        .expect("overlay SystemChat projected")
        .runs;
    assert_eq!(runs, expected);
    assert_eq!(
        runs.iter()
            .map(|run| run.visual.text.as_str())
            .collect::<String>(),
        "Ready, Alex"
    );
    let title = hud_title_text_from_world(&world, None, 0.0).expect("title projected");
    assert_eq!(title.title_runs, expected);
    assert_eq!(title.subtitle_runs, expected);
}

#[test]
fn hud_debug_overlay_projects_version_and_camera_position_lines() {
    let world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    let fps_sampler = hud_debug_fps_sampler_with_reported_fps(57);
    let surface_size = winit::dpi::PhysicalSize::new(320, 240);
    assert_eq!(
        hud_debug_overlay(HudDebugOverlayInputs {
            surface_size,
            physical_surface_size: surface_size,
            fps_sampler: &fps_sampler,
            ..HudDebugTestDefaults::default().inputs(&input, &world, None)
        }),
        None
    );

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    let overlay = hud_debug_overlay(HudDebugOverlayInputs {
        cloud_shape: Some(bbb_client_render::CloudShape::Flat),
        texture_filtering: bbb_client_render::TerrainTextureFiltering::Anisotropic,
        max_anisotropy: 8,
        surface_size,
        physical_surface_size: surface_size,
        fps_sampler: &fps_sampler,
        ..HudDebugTestDefaults::default().inputs(
            &input,
            &world,
            Some(CameraPose {
                position: [10.25, 64.0, -5.75],
                y_rot: 90.0,
                x_rot: 15.0,
                eye_height: 1.62,
            }),
        )
    })
    .expect("debug overlay should project when F3 is visible");

    assert_eq!(
        overlay.left_lines[0],
        format!("Minecraft {MC_VERSION} ({MC_VERSION}/bbb-client)")
    );
    assert_eq!(overlay.left_lines[1], "57 fps T: inf vsync");
    assert!(overlay
        .left_lines
        .contains(&"XYZ: 10.250 / 64.00000 / -5.750".to_string()));
    assert!(overlay.left_lines.contains(&"Block: 10 64 -6".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"Chunk: 0 4 -1 [0 31 in r.0.-1.mca]".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"Facing: west (Towards negative X) (90.0 / 15.0)".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"Section-relative: 10 00 10".to_string()));
    assert!(overlay
        .right_lines
        .iter()
        .any(|line| line.starts_with("Mem: ")));
    assert!(overlay
        .right_lines
        .contains(&"Display: 320x240 (wgpu)".to_string()));
    assert!(overlay
        .right_lines
        .iter()
        .any(|line| line == "fast-clouds B: 2"));
    assert!(overlay
        .right_lines
        .iter()
        .any(|line| line == "Filtering: Anisotropic 8x"));
    assert_eq!(
        overlay.debug_crosshair,
        Some(HudDebugCrosshair {
            x_rot_degrees: 15.0,
            y_rot_degrees: 90.0,
            gui_scale: 1,
        })
    );
}

#[test]
fn hud_debug_overlay_projects_vanilla_default_profile_entries() {
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    world.apply_custom_payload(bbb_protocol::packets::CustomPayload {
        id: "minecraft:brand".to_string(),
        payload: bbb_protocol::packets::CustomPayloadBody::Brand {
            brand: "vanilla".to_string(),
        },
    });
    let mut input = ClientInputState::new(true);
    let fps_sampler = hud_debug_fps_sampler_with_reported_fps(60);
    let surface_size = winit::dpi::PhysicalSize::new(320, 240);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None
    ));
    let overlay = hud_debug_overlay(HudDebugOverlayInputs {
        improved_transparency: true,
        cloud_shape: Some(bbb_client_render::CloudShape::Fancy),
        texture_filtering: bbb_client_render::TerrainTextureFiltering::Rgss,
        surface_size,
        physical_surface_size: surface_size,
        fps_sampler: &fps_sampler,
        ..HudDebugTestDefaults::default().inputs(
            &input,
            &world,
            Some(CameraPose {
                position: [10.25, 64.0, -5.75],
                y_rot: 90.0,
                x_rot: 15.0,
                eye_height: 1.62,
            }),
        )
    })
    .expect("default debug profile should project while F3 is visible");

    // Vanilla DebugScreenEntries.PROFILES projects these default entries into the overlay.
    assert_eq!(
        overlay.left_lines[0],
        format!("Minecraft {MC_VERSION} ({MC_VERSION}/bbb-client)")
    );
    assert_eq!(overlay.left_lines[1], "60 fps T: inf vsync");
    assert!(overlay
        .left_lines
        .contains(&"\"vanilla\" server, 0 tx, 0 rx".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"XYZ: 10.250 / 64.00000 / -5.750".to_string()));
    assert!(overlay.left_lines.contains(&"Block: 10 64 -6".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"Chunk: 0 4 -1 [0 31 in r.0.-1.mca]".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"Section-relative: 10 00 10".to_string()));
    assert!(overlay
        .right_lines
        .iter()
        .any(|line| line.starts_with("Mem: ")));
    assert!(overlay
        .right_lines
        .iter()
        .any(|line| line.starts_with("Allocation rate: ")));
    assert!(overlay
        .right_lines
        .iter()
        .any(|line| line.starts_with("Allocated: ")));
    assert!(overlay
        .right_lines
        .iter()
        .any(|line| line.starts_with("Java: ")));
    assert!(overlay
        .right_lines
        .iter()
        .any(|line| line.starts_with("CPU: ")));
    assert!(overlay
        .right_lines
        .contains(&"Display: 320x240 (wgpu)".to_string()));
    assert!(overlay
        .right_lines
        .iter()
        .any(|line| line == "improved-transparency fancy-clouds B: 2"));
    assert!(overlay
        .right_lines
        .iter()
        .any(|line| line == "Filtering: RGSS"));
    assert_eq!(
        overlay.debug_crosshair,
        Some(HudDebugCrosshair {
            x_rot_degrees: 15.0,
            y_rot_degrees: 90.0,
            gui_scale: 1,
        })
    );
}

#[test]
fn hud_debug_overlay_projects_performance_profile_fps_when_overlay_hidden() {
    let world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let mut input = ClientInputState::new(true);
    input.load_debug_screen_profile(crate::debug_entries::DebugScreenProfile::Performance);
    let fps_sampler = hud_debug_fps_sampler_with_reported_fps(144);

    let overlay = hud_debug_overlay(HudDebugOverlayInputs {
        fps_sampler: &fps_sampler,
        client_vsync: false,
        ..HudDebugTestDefaults::default().inputs(&input, &world, None)
    })
    .expect("performance profile FPS is always-on");

    assert_eq!(overlay.left_lines, vec!["144 fps T: inf".to_string()]);
    assert!(overlay.right_lines.is_empty());
    assert_eq!(overlay.debug_crosshair, None);
}

#[test]
fn hud_debug_overlay_projects_performance_profile_gpu_utilization_when_overlay_visible() {
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let mut input = ClientInputState::new(true);
    input.load_debug_screen_profile(crate::debug_entries::DebugScreenProfile::Performance);
    let fps_sampler = hud_debug_fps_sampler_with_reported_fps(144);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None
    ));

    let overlay = hud_debug_overlay(HudDebugOverlayInputs {
        fps_sampler: &fps_sampler,
        client_vsync: false,
        ..HudDebugTestDefaults::default().inputs(&input, &world, None)
    })
    .expect("performance profile overlay should include enabled entries");

    assert!(overlay.left_lines.contains(&"144 fps T: inf".to_string()));
    assert!(overlay.right_lines.contains(&"GPU: N/A".to_string()));
    assert_eq!(hud_debug_gpu_utilization_line(Some(42.6)), "GPU: 43%");
}

#[test]
fn hud_debug_overlay_projects_custom_day_count_entry_from_world_day_clock() {
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    set_world_day_time(&mut world, 48_123);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::DayCount,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(HudDebugOverlayInputs {
        client_vsync: false,
        ..HudDebugTestDefaults::default().inputs(&input, &world, None)
    })
    .expect("custom day-count entry should show while always-on");

    assert_eq!(overlay.left_lines, vec!["Day #2".to_string()]);
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_light_levels_from_camera_feet_block() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    let mut chunk = empty_lightmap_test_chunk_with_biome(world.dimension(), 42);
    let mut sky = [0; TEST_LIGHT_ARRAY_BYTES];
    let mut block = [0; TEST_LIGHT_ARRAY_BYTES];
    let nibble_index = section_block_index(0, 1, 0);
    set_test_light_nibble(&mut sky, nibble_index, 4);
    set_test_light_nibble(&mut block, nibble_index, 13);
    let light_section_index = usize::try_from(0 - (world.dimension().min_section_y() - 1)).unwrap();
    chunk.light = LightData::default();
    chunk.light.set_sky_packed(light_section_index, sky);
    chunk.light.set_block_packed(light_section_index, block);
    world.insert_decoded_chunk(chunk);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::LightLevels,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(HudDebugTestDefaults::default().inputs(
        &input,
        &world,
        Some(CameraPose {
            position: [0.25, 1.0, 0.25],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
    ))
    .expect("custom light entry should show with loaded camera block");

    assert_eq!(
        overlay.left_lines,
        vec!["Client Light: 13 (4 sky, 13 block)".to_string()]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_heightmaps_from_camera_feet_block() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    let mut chunk = empty_lightmap_test_chunk(world.dimension());
    chunk.heightmaps = vec![
        test_heightmap(1, world.dimension(), &[(2, 3, 9)]),
        test_heightmap(4, world.dimension(), &[(2, 3, 7)]),
    ];
    world.insert_decoded_chunk(chunk);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::Heightmap,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(HudDebugTestDefaults::default().inputs(
        &input,
        &world,
        Some(CameraPose {
            position: [2.25, 1.0, 3.25],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
    ))
    .expect("custom heightmap entry should show with loaded camera chunk");

    assert_eq!(
        overlay.left_lines,
        vec![
            "CH S: 9 M: 7 ML: ??".to_string(),
            "SH S: ?? O: ?? M: ?? ML: ??".to_string(),
        ]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_biome_from_camera_feet_block() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.record_registry_entries(
        "minecraft:worldgen/biome",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:plains"),
            RegistryPacketEntry::stub("minecraft:cherry_grove"),
        ],
    );
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 1));
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::Biome,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(HudDebugTestDefaults::default().inputs(
        &input,
        &world,
        Some(CameraPose {
            position: [0.25, 1.0, 0.25],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
    ))
    .expect("custom biome entry should show with loaded camera block");

    assert_eq!(
        overlay.left_lines,
        vec!["Biome: minecraft:cherry_grove".to_string()]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_suppresses_custom_local_difficulty_without_integrated_server() {
    let world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::LocalDifficulty,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(HudDebugTestDefaults::default().inputs(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
    ));

    assert_eq!(overlay, None);
}

#[test]
fn hud_debug_overlay_suppresses_custom_entity_spawn_counts_without_integrated_server() {
    let world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::EntitySpawnCounts,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(HudDebugTestDefaults::default().inputs(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
    ));

    assert_eq!(overlay, None);
}

#[test]
fn hud_debug_overlay_projects_custom_looking_at_block_state() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let target = BlockPos { x: 0, y: 1, z: 0 };
    let properties = BTreeMap::from([
        ("facing".to_string(), "north".to_string()),
        ("open".to_string(), "true".to_string()),
    ]);
    let block_state_id = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:barrel", &properties)
        .expect("vanilla barrel block state");
    set_lightmap_test_block(&mut world, target, block_state_id);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::LookingAtBlockState,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(HudDebugTestDefaults::default().inputs(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
    ))
    .expect("custom looking-at block-state entry should show with a target block");

    assert_eq!(
        overlay.left_lines,
        vec![
            "Targeted Block: 0, 1, 0".to_string(),
            "minecraft:barrel".to_string(),
            "facing: north".to_string(),
            "open: true".to_string(),
        ]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_looking_at_block_tags() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    let stone_id = world
        .registry_holder_id_by_resource_id("minecraft:block", "minecraft:stone")
        .expect("vanilla block registry contains stone");
    let barrel_id = world
        .registry_holder_id_by_resource_id("minecraft:block", "minecraft:barrel")
        .expect("vanilla block registry contains barrel");
    let chest_id = world
        .registry_holder_id_by_resource_id("minecraft:block", "minecraft:chest")
        .expect("vanilla block registry contains chest");
    apply_block_tags(
        &mut world,
        vec![
            ("minecraft:guarded_by_piglins", vec![barrel_id]),
            ("minecraft:mineable/axe", vec![barrel_id, chest_id]),
            ("minecraft:mineable/pickaxe", vec![stone_id]),
        ],
    );
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let target = BlockPos { x: 0, y: 1, z: 0 };
    let properties = BTreeMap::from([
        ("facing".to_string(), "north".to_string()),
        ("open".to_string(), "false".to_string()),
    ]);
    let block_state_id = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:barrel", &properties)
        .expect("vanilla barrel block state");
    set_lightmap_test_block(&mut world, target, block_state_id);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::LookingAtBlockTags,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(HudDebugTestDefaults::default().inputs(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
    ))
    .expect("custom looking-at block-tags entry should show target tags");

    assert_eq!(
        overlay.left_lines,
        vec![
            "#minecraft:guarded_by_piglins".to_string(),
            "#minecraft:mineable/axe".to_string(),
        ]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_looking_at_fluid_state() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let target = BlockPos { x: 0, y: 1, z: 0 };
    let properties = BTreeMap::from([("level".to_string(), "1".to_string())]);
    let block_state_id = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:water", &properties)
        .expect("vanilla flowing water block state");
    set_lightmap_test_block(&mut world, target, block_state_id);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::LookingAtFluidState,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(HudDebugTestDefaults::default().inputs(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
    ))
    .expect("custom looking-at fluid-state entry should show target fluid");

    assert_eq!(
        overlay.left_lines,
        vec![
            "Targeted Fluid: 0, 1, 0".to_string(),
            "minecraft:flowing_water".to_string(),
            "falling: false".to_string(),
            "level: 7".to_string(),
        ]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_looking_at_fluid_tags() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.record_registry_entries(
        "minecraft:fluid",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:water"),
            RegistryPacketEntry::stub("minecraft:flowing_water"),
            RegistryPacketEntry::stub("minecraft:lava"),
        ],
    );
    apply_fluid_tags(
        &mut world,
        vec![
            ("minecraft:flowing_only", vec![1]),
            ("minecraft:lava", vec![2]),
            ("minecraft:water", vec![0, 1]),
        ],
    );
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let target = BlockPos { x: 0, y: 1, z: 0 };
    let properties = BTreeMap::from([("level".to_string(), "1".to_string())]);
    let block_state_id = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:water", &properties)
        .expect("vanilla flowing water block state");
    set_lightmap_test_block(&mut world, target, block_state_id);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::LookingAtFluidTags,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(HudDebugTestDefaults::default().inputs(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
    ))
    .expect("custom looking-at fluid-tags entry should show target tags");

    assert_eq!(
        overlay.left_lines,
        vec![
            "#minecraft:flowing_only".to_string(),
            "#minecraft:water".to_string(),
        ]
    );
    assert!(overlay.right_lines.is_empty());
}
