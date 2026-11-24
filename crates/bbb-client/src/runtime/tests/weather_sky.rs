use super::*;

#[test]
fn debug_supporting_block_cache_keeps_membership_but_reads_live_support_state() {
    assert_eq!(
        DEBUG_SUPPORTING_BLOCK_REFRESH_INTERVAL,
        Duration::from_millis(100)
    );
    assert_eq!(DEBUG_SUPPORTING_BLOCK_QUERY_RADIUS, 16.0);
    assert_eq!(DEBUG_SUPPORTING_BLOCK_ENTITY_COLOR, [0, 255, 0, 255]);

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 1.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let stone = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:stone", &BTreeMap::new())
        .expect("vanilla stone state");
    let air = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:air", &BTreeMap::new())
        .expect("vanilla air state");
    add_remote_supporting_block_test_entity(&mut world, 52, 2, 1.0, stone);

    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeEntitySupportingBlocks,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let start = Instant::now();
    let mut cache = DebugSupportingBlockCache::default();
    let first = cache.lines(&input, &world, start);
    assert_eq!(first.len(), 24);
    assert_eq!(first[0].color, [0, 255, 0, 102]);
    assert!((first[0].from[0] - 1.958_84).abs() < 1.0e-6);

    add_remote_supporting_block_test_entity(&mut world, 53, 4, 1.0, stone);
    assert_eq!(
        cache
            .lines(
                &input,
                &world,
                start + DEBUG_SUPPORTING_BLOCK_REFRESH_INTERVAL,
            )
            .len(),
        24,
        "vanilla refreshes membership only after more than 100 ms"
    );
    let refreshed_at = start + DEBUG_SUPPORTING_BLOCK_REFRESH_INTERVAL + Duration::from_nanos(1);
    assert_eq!(cache.lines(&input, &world, refreshed_at).len(), 48);

    set_lightmap_test_block(&mut world, BlockPos { x: 2, y: 0, z: 0 }, air);
    for _ in 0..2 {
        assert!(world.apply_entity_move(bbb_protocol::packets::EntityMove {
            id: 52,
            has_position: false,
            delta_x: 0,
            delta_y: 0,
            delta_z: 0,
            y_rot: None,
            x_rot: None,
            on_ground: true,
        }));
    }
    assert_eq!(
        cache
            .lines(&input, &world, refreshed_at + Duration::from_millis(1))
            .len(),
        24,
        "cached members still project their live supporting-block state each frame"
    );

    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeEntitySupportingBlocks,
        crate::debug_entries::DebugScreenEntryStatus::Never,
    );
    assert!(cache.lines(&input, &world, refreshed_at).is_empty());
    add_remote_supporting_block_test_entity(&mut world, 54, 6, 1.0, stone);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeEntitySupportingBlocks,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    assert_eq!(
        cache.lines(&input, &world, refreshed_at).len(),
        48,
        "re-enabling creates a fresh membership cache and refreshes immediately"
    );
}

#[test]
fn debug_supporting_block_remote_legacy_effect_uses_biased_cyan_outline() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 1.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let piston_head = world
        .registries()
        .block_state_id_by_name_and_properties(
            "minecraft:piston_head",
            &BTreeMap::from([
                ("facing".to_string(), "down".to_string()),
                ("short".to_string(), "false".to_string()),
                ("type".to_string(), "normal".to_string()),
            ]),
        )
        .expect("vanilla downward piston head state");
    add_remote_supporting_block_test_entity(&mut world, 41, 2, 1.25, piston_head);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeEntitySupportingBlocks,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let lines = DebugSupportingBlockCache::default().lines(&input, &world, Instant::now());
    let cyan_outline = lines
        .iter()
        .filter(|line| line.color == [0, 255, 255, 102])
        .collect::<Vec<_>>();
    assert_eq!(cyan_outline.len(), 12);
    assert!((cyan_outline[0].from[0] - 1.945_08).abs() < 1.0e-6);
    assert!((cyan_outline[0].from[1] - 0.945_08).abs() < 1.0e-6);
    assert!(lines
        .iter()
        .any(|line| line.color == DEBUG_SUPPORTING_BLOCK_ENTITY_COLOR));
}

#[test]
fn boss_bar_name_runs_consume_legacy_codes_with_the_root_style_as_reset() {
    use bbb_protocol::packets::{
        BossBarColor, BossBarOverlay, BossEvent, BossEventFlags, BossEventOperation,
    };
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue, RichTextComponent};

    let name = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "text".into(),
            value: NbtSummaryValue::String("K§cX§rY§?Z§".into()),
        },
        NbtSummaryEntry {
            name: "bold".into(),
            value: NbtSummaryValue::Byte(1),
        },
    ]))
    .unwrap();
    let mut world = WorldStore::new();
    world.apply_boss_event(BossEvent {
        id: uuid::Uuid::from_u128(11),
        operation: BossEventOperation::Add {
            name: Box::new(name),
            progress: 0.5,
            color: BossBarColor::Purple,
            overlay: BossBarOverlay::Progress,
            flags: BossEventFlags {
                darken_screen: false,
                play_music: false,
                create_world_fog: false,
            },
        },
    });

    // `BossHealthOverlay` measures and draws the name Component, so
    // the centred x and the glyphs share these runs.
    let bars = hud_boss_bars_from_world(&world);
    assert_eq!(
        bars[0]
            .name_runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<Vec<_>>(),
        ["K", "X", "YZ"]
    );
    assert!(bars[0].name_runs[0].style.bold);
    assert_eq!(bars[0].name_runs[0].color, None);
    assert_eq!(bars[0].name_runs[1].color, Some(0xFF_55_55));
    assert!(!bars[0].name_runs[1].style.bold);
    // `§r` returns to the component's own bold, and `YZ` merges because the
    // unknown code and the trailing `§` changed nothing.
    assert!(bars[0].name_runs[2].style.bold);
    assert_eq!(bars[0].name_runs[2].color, None);
}

#[test]
fn hud_boss_bar_projection_keeps_packet_add_order_and_maps_style_names() {
    let mut world = WorldStore::new();
    assert!(hud_boss_bars_from_world(&world).is_empty());

    // Insert the higher UUID first: Vanilla's LinkedHashMap and the world
    // both preserve packet-add order rather than sorting by UUID.
    let dragon = uuid::Uuid::from_u128(7);
    let wither = uuid::Uuid::from_u128(2);
    world.apply_boss_event(bbb_protocol::packets::BossEvent {
        id: dragon,
        operation: bbb_protocol::packets::BossEventOperation::Add {
            name: styled_boss_component("Ender Dragon"),
            progress: 0.75,
            color: bbb_protocol::packets::BossBarColor::Purple,
            overlay: bbb_protocol::packets::BossBarOverlay::Progress,
            flags: bbb_protocol::packets::BossEventFlags {
                darken_screen: true,
                play_music: false,
                create_world_fog: true,
            },
        },
    });
    world.apply_boss_event(bbb_protocol::packets::BossEvent {
        id: wither,
        operation: bbb_protocol::packets::BossEventOperation::Add {
            name: Box::new(bbb_protocol::RichTextComponent::plain("Wither")),
            progress: 0.5,
            color: bbb_protocol::packets::BossBarColor::Red,
            overlay: bbb_protocol::packets::BossBarOverlay::Notched10,
            flags: bbb_protocol::packets::BossEventFlags {
                darken_screen: false,
                play_music: false,
                create_world_fog: false,
            },
        },
    });

    let bars = hud_boss_bars_from_world(&world);
    assert_eq!(bars.len(), 2);
    assert_eq!(bars[0].name_runs.len(), 1);
    assert_eq!(bars[0].name_runs[0].text, "Ender Dragon");
    assert!(bars[0].name_runs[0].style.bold);
    assert_eq!(bars[0].name_runs[0].color, Some(0xFF_55_55));
    assert_eq!(bars[0].progress, 0.75);
    assert_eq!(bars[0].color, bbb_client_render::HudBossBarColor::Purple);
    assert_eq!(
        bars[0].overlay,
        bbb_client_render::HudBossBarOverlay::Progress
    );
    assert_eq!(
        bars[1].name_runs,
        vec![bbb_client_render::HudStyledTextRun::plain("Wither")]
    );
    assert_eq!(bars[1].color, bbb_client_render::HudBossBarColor::Red);
    assert_eq!(
        bars[1].overlay,
        bbb_client_render::HudBossBarOverlay::Notched10
    );

    // Style updates re-project; removing a bar drops it from the list. The
    // darken/fog flags never ride the bar draw (they stay behind the world's
    // `boss_overlay_should_*` queries).
    world.apply_boss_event(bbb_protocol::packets::BossEvent {
        id: wither,
        operation: bbb_protocol::packets::BossEventOperation::UpdateStyle {
            color: bbb_protocol::packets::BossBarColor::Yellow,
            overlay: bbb_protocol::packets::BossBarOverlay::Notched20,
        },
    });
    let bars = hud_boss_bars_from_world(&world);
    assert_eq!(bars[1].color, bbb_client_render::HudBossBarColor::Yellow);
    assert_eq!(
        bars[1].overlay,
        bbb_client_render::HudBossBarOverlay::Notched20
    );
    world.apply_boss_event(bbb_protocol::packets::BossEvent {
        id: wither,
        operation: bbb_protocol::packets::BossEventOperation::Remove,
    });
    let bars = hud_boss_bars_from_world(&world);
    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].progress, 0.75);
}

#[test]
fn hud_boss_bar_runtime_interpolates_progress_for_exactly_one_hundred_milliseconds() {
    let id = uuid::Uuid::from_u128(7);
    let start = Instant::now();
    let mut world = WorldStore::new();
    add_boss_bar_with_id_and_progress(&mut world, id, 0.0);
    let mut runtime = HudRenderRuntime::default();

    assert_eq!(
        boss_bar_progresses(runtime.boss_bars_update(&world, None, 1, start, start, false)),
        Some(vec![0.0])
    );
    assert!(world.apply_boss_event(ProtocolBossEvent {
        id,
        operation: ProtocolBossEventOperation::UpdateProgress { progress: 1.0 },
    }));
    assert_eq!(
        boss_bar_progresses(runtime.boss_bars_update(&world, None, 1, start, start, false)),
        Some(vec![0.0])
    );
    assert_eq!(
        boss_bar_progresses(runtime.boss_bars_update(
            &world,
            None,
            1,
            start,
            start + Duration::from_millis(50),
            true,
        )),
        Some(vec![0.5])
    );
    assert_eq!(
        boss_bar_progresses(runtime.boss_bars_update(
            &world,
            None,
            1,
            start,
            start + Duration::from_millis(100),
            true,
        )),
        Some(vec![1.0])
    );
    assert!(matches!(
        runtime.boss_bars_update(
            &world,
            None,
            1,
            start,
            start + Duration::from_millis(101),
            true,
        ),
        RendererFrameUpdate::Unchanged
    ));
}

#[test]
fn hud_boss_bar_runtime_samples_interrupted_and_equal_target_updates() {
    let id = uuid::Uuid::from_u128(8);
    let start = Instant::now();
    let mut world = WorldStore::new();
    add_boss_bar_with_id_and_progress(&mut world, id, 0.0);
    let mut runtime = HudRenderRuntime::default();
    let _ = runtime.boss_bars_update(&world, None, 1, start, start, false);

    assert!(world.apply_boss_event(ProtocolBossEvent {
        id,
        operation: ProtocolBossEventOperation::UpdateProgress { progress: 1.0 },
    }));
    assert_eq!(
        boss_bar_progresses(runtime.boss_bars_update(
            &world,
            None,
            1,
            start,
            start + Duration::from_millis(50),
            false,
        )),
        Some(vec![0.5])
    );

    // Even an equal-valued packet restarts `LerpingBossEvent` from the
    // currently visible value rather than being coalesced away.
    assert!(world.apply_boss_event(ProtocolBossEvent {
        id,
        operation: ProtocolBossEventOperation::UpdateProgress { progress: 1.0 },
    }));
    assert_eq!(
        boss_bar_progresses(runtime.boss_bars_update(
            &world,
            None,
            1,
            start + Duration::from_millis(50),
            start + Duration::from_millis(100),
            false,
        )),
        Some(vec![0.75])
    );

    // An interrupted update likewise samples the active curve at the packet
    // time, then begins the replacement curve from that visible value.
    assert!(world.apply_boss_event(ProtocolBossEvent {
        id,
        operation: ProtocolBossEventOperation::UpdateProgress { progress: 0.0 },
    }));
    assert_eq!(
        boss_bar_progresses(runtime.boss_bars_update(
            &world,
            None,
            1,
            start + Duration::from_millis(100),
            start + Duration::from_millis(150),
            false,
        )),
        Some(vec![0.375])
    );
}

#[test]
fn hud_boss_bar_runtime_jumps_on_add_replacement_remove_and_connection_reset() {
    let id = uuid::Uuid::from_u128(9);
    let start = Instant::now();
    let mut world = WorldStore::new();
    add_boss_bar_with_id_and_progress(&mut world, id, 0.25);
    let mut runtime = HudRenderRuntime::default();
    let _ = runtime.boss_bars_update(&world, None, 4, start, start, false);

    add_boss_bar_with_id_and_progress(&mut world, id, 0.8);
    assert_eq!(
        boss_bar_progresses(runtime.boss_bars_update(
            &world,
            None,
            4,
            start + Duration::from_millis(10),
            start + Duration::from_millis(10),
            false,
        )),
        Some(vec![0.8])
    );
    assert_eq!(
        boss_bar_progresses(runtime.boss_bars_update(
            &world,
            None,
            5,
            start + Duration::from_millis(20),
            start + Duration::from_millis(20),
            true,
        )),
        Some(vec![0.8])
    );

    assert!(world.apply_boss_event(ProtocolBossEvent {
        id,
        operation: ProtocolBossEventOperation::Remove,
    }));
    assert_eq!(
        boss_bar_progresses(runtime.boss_bars_update(
            &world,
            None,
            5,
            start + Duration::from_millis(30),
            start + Duration::from_millis(30),
            false,
        )),
        Some(Vec::new())
    );
}

#[test]
fn hud_boss_bar_projection_covers_every_vanilla_color_and_overlay() {
    // The world stores vanilla `getName` strings and the projection re-parses
    // them (`HudBossBarColor::from_name` / `HudBossBarOverlay::from_name`):
    // every protocol color x overlay combination must survive, or bars would
    // silently vanish on a name mismatch.
    let colors = [
        bbb_protocol::packets::BossBarColor::Pink,
        bbb_protocol::packets::BossBarColor::Blue,
        bbb_protocol::packets::BossBarColor::Red,
        bbb_protocol::packets::BossBarColor::Green,
        bbb_protocol::packets::BossBarColor::Yellow,
        bbb_protocol::packets::BossBarColor::Purple,
        bbb_protocol::packets::BossBarColor::White,
    ];
    let overlays = [
        bbb_protocol::packets::BossBarOverlay::Progress,
        bbb_protocol::packets::BossBarOverlay::Notched6,
        bbb_protocol::packets::BossBarOverlay::Notched10,
        bbb_protocol::packets::BossBarOverlay::Notched12,
        bbb_protocol::packets::BossBarOverlay::Notched20,
    ];
    let mut world = WorldStore::new();
    let mut id = 0u128;
    for color in colors {
        for overlay in overlays {
            id += 1;
            world.apply_boss_event(bbb_protocol::packets::BossEvent {
                id: uuid::Uuid::from_u128(id),
                operation: bbb_protocol::packets::BossEventOperation::Add {
                    name: Box::new(bbb_protocol::RichTextComponent::plain(format!("boss {id}"))),
                    progress: 1.0,
                    color,
                    overlay,
                    flags: bbb_protocol::packets::BossEventFlags {
                        darken_screen: false,
                        play_music: false,
                        create_world_fog: false,
                    },
                },
            });
        }
    }
    let bars = hud_boss_bars_from_world(&world);
    assert_eq!(bars.len(), colors.len() * overlays.len());
}

#[test]
fn crosshair_attack_indicator_maps_world_semantics_without_restoring_target_aabb() {
    assert_eq!(
        hud_crosshair_attack_indicator(true, None),
        HudCrosshairAttackIndicator::None
    );
    assert_eq!(
        hud_crosshair_attack_indicator(
            true,
            Some(LocalPlayerCrosshairAttackIndicatorState {
                strength_scale: 0.5,
                full: false,
            })
        ),
        HudCrosshairAttackIndicator::Progress {
            strength_scale: 0.5,
        }
    );
    assert_eq!(
        hud_crosshair_attack_indicator(
            true,
            Some(LocalPlayerCrosshairAttackIndicatorState {
                strength_scale: 1.0,
                full: true,
            })
        ),
        HudCrosshairAttackIndicator::Full
    );
    assert_eq!(
        hud_crosshair_attack_indicator(
            true,
            Some(LocalPlayerCrosshairAttackIndicatorState {
                strength_scale: 1.0,
                full: false,
            })
        ),
        HudCrosshairAttackIndicator::None
    );
    assert_eq!(
        hud_crosshair_attack_indicator(
            false,
            Some(LocalPlayerCrosshairAttackIndicatorState {
                strength_scale: 0.5,
                full: false,
            })
        ),
        HudCrosshairAttackIndicator::None
    );
}

#[test]
fn clear_color_applies_client_sky_flash_color_layer() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    let baseline = clear_color_for_world(&world, false);

    world.set_sky_flash_time(2);
    let flashed = clear_color_for_world(&world, false);
    let expected = clear_color_with_sky_flash(baseline);

    assert_clear_color_close(flashed, expected);
    assert_ne!(flashed, baseline);
}

#[test]
fn clear_color_hides_client_sky_flash_when_option_is_enabled() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    let baseline = clear_color_for_world(&world, false);

    world.set_sky_flash_time(2);
    let hidden = clear_color_for_world(&world, true);

    assert_eq!(hidden, baseline);
}

#[test]
fn void_darkness_below_the_level_floor_darkens_the_fog_color() {
    let camera_at = |y: f32| {
        Some(CameraPose {
            position: [0.0, y, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        })
    };
    let clear_at = |world: &WorldStore, y: f32| {
        clear_color_for_world_at_camera_with_water_vision(
            world,
            camera_at(y),
            VANILLA_ATMOSPHERIC_FOG_RENDER_DISTANCE_CHUNKS as u32,
            0.0,
            false,
        )
    };
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);

    // Overworld min_y = -64, onset range 32: a camera above min_y + 32 sees
    // no darkening.
    let baseline = clear_at(&world, 0.0);
    // Halfway into the onset range the vanilla (1 - darkness)^2 factor dims
    // the fog without blacking it out.
    let mid = clear_at(&world, -48.0);
    assert!(mid.r < baseline.r && mid.r > 0.0);
    assert!(mid.g < baseline.g && mid.g > 0.0);
    assert!(mid.b < baseline.b && mid.b > 0.0);
    assert_eq!(mid.a, baseline.a);
    // Below the full onset range the fog is black.
    let deep = clear_at(&world, -112.0);
    assert!(deep.r < 1.0 / 255.0 + 1e-6);
    assert!(deep.g < 1.0 / 255.0 + 1e-6);
    assert!(deep.b < 1.0 / 255.0 + 1e-6);
    assert_eq!(deep.a, baseline.a);

    // A flat level compresses the onset range to one block, so the same
    // mid-range camera height stays untouched.
    let mut flat = world_with_dimension(0, "minecraft:overworld");
    let mut flat_login = PlayLogin {
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
            seed: 12345,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: true,
    };
    flat_login.common_spawn_info.is_flat = true;
    flat.apply_login(&flat_login);
    set_world_day_time(&mut flat, 6_000);
    assert_clear_color_close(clear_at(&flat, -48.0), baseline);
}

#[test]
fn sky_environment_renders_the_dark_disc_only_below_the_horizon() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    // No player pose keeps the disc hidden.
    assert!(
        !sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), false)
            .dark_disc
    );

    // Standing eye height 1.62 puts the eye at 62.62, under the 63.0 horizon.
    world.set_local_player_pose(local_player_pose([0.0, 61.0, 0.0], 0.0, 0.0));
    assert!(
        sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), false)
            .dark_disc
    );

    world.set_local_player_pose(local_player_pose([0.0, 61.5, 0.0], 0.0, 0.0));
    assert!(
        !sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), false)
            .dark_disc
    );
}

#[test]
fn clear_color_uses_vanilla_dimension_fog_and_sky_color_without_camera_biome() {
    let mut overworld = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut overworld, 6_000);
    assert_clear_color_close(
        clear_color_for_world(&overworld, false),
        clear_color_for_day_time(6_000, 0.0, 0.0),
    );

    let mut nether = world_with_dimension(1, "minecraft:the_nether");
    set_world_day_time(&mut nether, 6_000);
    assert_clear_color_close(
        clear_color_for_world(&nether, false),
        clear_color_for_day_time_with_environment_colors(
            6_000,
            0.0,
            0.0,
            None,
            Some([0, 0, 0]),
            VanillaLightmapDimensionKind::Nether,
        ),
    );

    let mut end = world_with_dimension(2, "minecraft:the_end");
    set_world_day_time(&mut end, 6_000);
    assert_clear_color_close(
        clear_color_for_world(&end, false),
        clear_color_for_day_time_with_environment_colors(
            6_000,
            0.0,
            0.0,
            Some(VANILLA_END_FOG_COLOR),
            Some([0, 0, 0]),
            VanillaLightmapDimensionKind::End,
        ),
    );
}

#[test]
fn lightmap_dimension_kind_prefers_registry_identity_over_bootstrap_holder_id() {
    let world = world_with_dimension(0, "minecraft:overworld");
    let mut level = world.level_info().unwrap().clone();

    level.dimension_type_id = 0;
    level.dimension_type_name = Some("minecraft:the_nether".to_string());
    assert_eq!(
        vanilla_lightmap_dimension_kind(&level),
        VanillaLightmapDimensionKind::Nether
    );

    level.dimension_type_name = Some("example:custom".to_string());
    assert_eq!(
        vanilla_lightmap_dimension_kind(&level),
        VanillaLightmapDimensionKind::Other
    );

    level.dimension_type_id = 1;
    level.dimension_type_name = None;
    assert_eq!(
        vanilla_lightmap_dimension_kind(&level),
        VanillaLightmapDimensionKind::Nether
    );
}

#[test]
fn custom_dimension_direct_environment_attributes_drive_lightmap_and_fog() {
    let mut world = world_with_custom_dimension_environment();
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));

    let lightmap = lightmap_environment_attributes_for_world(&world);
    assert_eq!(lightmap.sky_factor, 0.25);
    assert_close3(
        lightmap.block_light_tint,
        [
            0x20 as f32 / 255.0,
            0x40 as f32 / 255.0,
            0x60 as f32 / 255.0,
        ],
    );
    assert_close3(
        lightmap.sky_light_color,
        [
            0x40 as f32 / 255.0,
            0x60 as f32 / 255.0,
            0x80 as f32 / 255.0,
        ],
    );
    assert_close3(
        lightmap.ambient_color,
        [
            0x30 as f32 / 255.0,
            0x50 as f32 / 255.0,
            0x70 as f32 / 255.0,
        ],
    );
    assert_close3(
        lightmap.night_vision_color,
        [
            0x50 as f32 / 255.0,
            0x70 as f32 / 255.0,
            0x90 as f32 / 255.0,
        ],
    );

    let atmospheric = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.0,
        0.0,
        false,
    );
    assert_eq!(atmospheric.environmental_start, 12.0);
    assert_eq!(atmospheric.environmental_end, 144.0);
    assert_eq!(atmospheric.sky_end, 96.0);
    assert_eq!(atmospheric.cloud_end, 192.0);
    assert_fog_environment_close(
        atmospheric,
        FogEnvironment::world_with_visibility_ends(
            clear_color_to_fog_color(clear_color_for_atmospheric_environment(
                AtmosphericClearColorInputs {
                    day_time: 6_000,
                    rain_level: 0.0,
                    thunder_level: 0.0,
                    fog_color: Some([0x12, 0x34, 0x56]),
                    sky_color: Some([0x34, 0x56, 0x78]),
                    sun_angle_degrees: None,
                    sunrise_sunset_color: None,
                    sky_fog_end_distance: Some(96.0),
                    dimension_kind: VanillaLightmapDimensionKind::Other,
                    uses_builtin_dimension_fallback: false,
                    camera_forward: None,
                    render_distance_chunks: 12,
                },
            )),
            12.0,
            144.0,
            12,
            96.0,
            192.0,
        ),
    );

    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    let water = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.5,
        0.0,
        false,
    );
    assert_fog_environment_close(
        water,
        FogEnvironment::world_with_visibility_ends(
            clear_color_to_fog_color(clear_color_from_argb(apply_fog_brightening(
                rgb_u8_to_argb([0x56, 0x78, 0x9a]),
                0.5,
            ))),
            -4.0,
            24.0,
            12,
            24.0,
            24.0,
        ),
    );
}

#[test]
fn custom_dimension_timeline_layers_drive_lightmap_from_session_registries() {
    let world = world_with_custom_dimension_timeline();

    let sampled = world.sampled_client_environment_attributes().unwrap();
    assert_eq!(sampled.sky_light_factor, Some(0.125));
    let lightmap = lightmap_environment_attributes_for_world(&world);
    assert_eq!(lightmap.sky_factor, 0.125);
}

#[test]
fn biome_positional_environment_values_override_custom_dimension_base() {
    let mut world = world_with_custom_dimension_environment();
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 42));
    insert_uniform_biome_neighbors(&mut world, 42);
    install_biome_environment_profiles(
        &mut world,
        &[(
            42,
            &[
                (
                    "minecraft:visual/sky_color",
                    TestEnvironmentAttributeValue::String("#a0b0c0"),
                ),
                (
                    "minecraft:visual/fog_color",
                    TestEnvironmentAttributeValue::String("#908070"),
                ),
                (
                    "minecraft:visual/water_fog_color",
                    TestEnvironmentAttributeValue::String("#605040"),
                ),
                (
                    "minecraft:visual/water_fog_end_distance",
                    TestEnvironmentAttributeValue::Multiply(0.5),
                ),
                (
                    "minecraft:visual/block_light_tint",
                    TestEnvironmentAttributeValue::String("#102040"),
                ),
                (
                    "minecraft:visual/cloud_color",
                    TestEnvironmentAttributeValue::String("#80506070"),
                ),
                (
                    "minecraft:visual/sun_angle",
                    TestEnvironmentAttributeValue::Float(45.0),
                ),
            ],
        )],
    );

    let colors =
        camera_environment_colors_for_default_projection(&world, camera_pose_from_world(&world));
    assert_eq!(colors.sky_color, Some([0xa0, 0xb0, 0xc0]));
    assert_eq!(colors.fog_color, Some([0x90, 0x80, 0x70]));
    assert_eq!(colors.water_fog_color, Some([0x60, 0x50, 0x40]));
    assert_eq!(colors.water_fog_end_distance, Some(24.0));
    assert_close3(
        lightmap_environment_attributes_for_world(&world).block_light_tint,
        [
            0x10 as f32 / 255.0,
            0x20 as f32 / 255.0,
            0x40 as f32 / 255.0,
        ],
    );
    assert_close4(
        cloud_environment_for_world(&world).color,
        [
            0x50 as f32 / 255.0,
            0x60 as f32 / 255.0,
            0x70 as f32 / 255.0,
            0x80 as f32 / 255.0,
        ],
    );
    let sky = sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), false);
    assert!((sky.sun_angle_radians - 45.0_f32.to_radians()).abs() < 1e-6);
}

#[test]
fn clear_color_samples_camera_biome_fog_and_sky_color_attributes() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 42));
    insert_uniform_biome_neighbors(&mut world, 42);
    install_biome_environment_profiles(
        &mut world,
        &[(
            42,
            &[
                (
                    "minecraft:visual/sky_color",
                    TestEnvironmentAttributeValue::String("#102030"),
                ),
                (
                    "minecraft:visual/fog_color",
                    TestEnvironmentAttributeValue::String("#804020"),
                ),
            ],
        )],
    );

    let clear = clear_color_for_world_at_camera(&world, camera_pose_from_world(&world), false);
    let expected = clear_color_for_day_time_with_environment_colors(
        6_000,
        0.0,
        0.0,
        Some([0x80, 0x40, 0x20]),
        Some([0x10, 0x20, 0x30]),
        VanillaLightmapDimensionKind::Overworld,
    );

    assert_clear_color_close(clear, expected);
    assert_ne!(clear, clear_color_for_world(&world, false));
}

#[test]
fn sky_environment_samples_camera_biome_sky_color_for_sky_disc() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 42));
    insert_uniform_biome_neighbors(&mut world, 42);
    install_biome_environment_profiles(
        &mut world,
        &[(
            42,
            &[(
                "minecraft:visual/sky_color",
                TestEnvironmentAttributeValue::String("#102030"),
            )],
        )],
    );

    let sky = sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), false);

    assert_close3(
        [sky.color[0], sky.color[1], sky.color[2]],
        [
            0x10 as f32 / 255.0,
            0x20 as f32 / 255.0,
            0x30 as f32 / 255.0,
        ],
    );
    assert_eq!(sky.color[3], 1.0);

    for (thunder, expected) in [(0.0, [16, 20, 24]), (1.0, [6, 7, 8])] {
        set_world_weather(&mut world, 1.0, thunder);
        let sky =
            sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), false);
        assert_close3(
            [sky.color[0], sky.color[1], sky.color[2]],
            expected.map(|channel| channel as f32 / 255.0),
        );
    }
}

#[test]
fn sky_environment_applies_client_sky_flash_layer_and_dimension_gate() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    world.set_sky_flash_time(2);

    let flashed =
        sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), false);
    let hidden = sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), true);
    let expected = rgb24(argb_srgb_lerp(
        VANILLA_SKY_FLASH_SKY_COLOR_ALPHA,
        rgb_u8_to_argb(VANILLA_OVERWORLD_SKY_COLOR),
        VANILLA_SKY_FLASH_SKY_COLOR,
    ));

    assert_close3(
        [flashed.color[0], flashed.color[1], flashed.color[2]],
        expected,
    );
    assert_ne!(flashed, hidden);

    let nether = world_with_dimension(1, "minecraft:the_nether");
    assert_eq!(
        sky_environment_for_world_at_camera(&nether, camera_pose_from_world(&nether), false),
        SkyEnvironment::disabled()
    );
}

#[test]
fn sky_environment_projects_end_skybox_state() {
    let end = world_with_dimension(2, "minecraft:the_end");

    let sky = sky_environment_for_world_at_camera(&end, camera_pose_from_world(&end), false);

    assert!(sky.end_sky_visible());
    assert!(!sky.is_visible());
    assert_eq!(sky.sunrise_sunset_color[3], 0.0);
}

#[test]
fn sky_environment_suppresses_every_sky_pass_for_boss_world_fog() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    add_boss_bar(&mut world, false, true);

    assert_eq!(
        sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), false),
        SkyEnvironment::disabled()
    );
}

#[test]
fn sky_environment_projects_sunrise_sunset_render_state() {
    let day_time = 71;
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, day_time);
    set_world_weather(&mut world, 0.25, 0.5);

    let sky = sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), false);
    let effective_thunder = 0.25 * 0.5;
    let sunrise_color = apply_weather_sunrise_sunset_color_layers(
        sample_periodic_argb_keyframes(
            day_time,
            &VANILLA_OVERWORLD_SUNRISE_SUNSET_COLOR_KEYFRAMES,
            VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
        ),
        0.25,
        effective_thunder,
    );

    assert_close4(sky.sunrise_sunset_color, rgba32(sunrise_color));
    assert!(sky.sunrise_sunset_color[3] > 0.0);
    assert!((sky.sun_angle_radians - overworld_sun_angle(day_time).to_radians()).abs() < 1e-6);
    assert!((sky.moon_angle_radians - overworld_moon_angle(day_time).to_radians()).abs() < 1e-6);
    assert!((sky.rain_brightness - 0.75).abs() < 1e-6);
    assert_eq!(sky.moon_phase, SkyMoonPhase::FullMoon);
    assert!((sky.star_angle_radians - overworld_star_angle(day_time).to_radians()).abs() < 1e-6);
    let star_brightness = apply_weather_star_brightness_layers(
        sample_periodic_float_keyframes(
            day_time,
            &VANILLA_OVERWORLD_STAR_BRIGHTNESS_KEYFRAMES,
            VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
        ),
        0.25,
        effective_thunder as f32,
    );
    assert!((sky.star_brightness - star_brightness).abs() < 1e-6);
}

#[test]
fn sky_environment_projects_vanilla_moon_phase_cycle() {
    assert_eq!(overworld_moon_phase(0), SkyMoonPhase::FullMoon);
    assert_eq!(overworld_moon_phase(24_000), SkyMoonPhase::WaningGibbous);
    assert_eq!(overworld_moon_phase(48_000), SkyMoonPhase::ThirdQuarter);
    assert_eq!(overworld_moon_phase(72_000), SkyMoonPhase::WaningCrescent);
    assert_eq!(overworld_moon_phase(96_000), SkyMoonPhase::NewMoon);
    assert_eq!(overworld_moon_phase(120_000), SkyMoonPhase::WaxingCrescent);
    assert_eq!(overworld_moon_phase(144_000), SkyMoonPhase::FirstQuarter);
    assert_eq!(overworld_moon_phase(168_000), SkyMoonPhase::WaxingGibbous);
    assert_eq!(overworld_moon_phase(192_000), SkyMoonPhase::FullMoon);
}

#[test]
fn sky_and_cloud_environment_use_dynamic_dimension_attributes() {
    let world = world_with_dynamic_sky_environment();

    let sky = sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), false);
    assert_close4(
        sky.sunrise_sunset_color,
        [
            0x10 as f32 / 255.0,
            0x20 as f32 / 255.0,
            0x30 as f32 / 255.0,
            0x40 as f32 / 255.0,
        ],
    );
    assert!((sky.sun_angle_radians - 30.0_f32.to_radians()).abs() < 1e-6);
    assert!((sky.moon_angle_radians - 60.0_f32.to_radians()).abs() < 1e-6);
    assert!((sky.star_angle_radians - 90.0_f32.to_radians()).abs() < 1e-6);
    assert_eq!(sky.moon_phase, SkyMoonPhase::NewMoon);
    assert_eq!(sky.star_brightness, 0.75);

    let clouds = cloud_environment_for_world(&world);
    assert_close4(
        clouds.color,
        [
            0x50 as f32 / 255.0,
            0x60 as f32 / 255.0,
            0x70 as f32 / 255.0,
            0x80 as f32 / 255.0,
        ],
    );
    assert_eq!(clouds.height, 144.5);
}

#[test]
fn sky_environment_projects_vanilla_star_brightness_with_weather_layers() {
    let day_time = 13_228;
    let mut clear = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut clear, day_time);
    let clear_sky =
        sky_environment_for_world_at_camera(&clear, camera_pose_from_world(&clear), false);

    assert!((clear_sky.star_brightness - 0.5).abs() < 1e-6);

    let mut storm = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut storm, day_time);
    set_world_weather(&mut storm, 0.75, 0.25);
    let storm_sky =
        sky_environment_for_world_at_camera(&storm, camera_pose_from_world(&storm), false);

    let expected = apply_weather_star_brightness_layers(0.5, 0.75, 0.75 * 0.25);
    assert!((storm_sky.star_brightness - expected).abs() < 1e-6);
}

#[test]
fn cloud_environment_projects_vanilla_overworld_defaults_and_dimension_gate() {
    let overworld = world_with_dimension(0, "minecraft:overworld");
    let nether = world_with_dimension(1, "minecraft:the_nether");
    let end = world_with_dimension(2, "minecraft:the_end");

    let clouds = cloud_environment_for_world(&overworld);

    assert_eq!(clouds.color, VANILLA_DEFAULT_CLOUD_COLOR);
    assert_eq!(clouds.height, VANILLA_DEFAULT_CLOUD_HEIGHT);
    assert!(clouds.is_visible());
    assert_eq!(
        cloud_environment_for_world(&nether),
        CloudEnvironment::disabled()
    );
    assert_eq!(
        cloud_environment_for_world(&end),
        CloudEnvironment::disabled()
    );
}

#[test]
fn cloud_environment_applies_overworld_day_timeline_cloud_color() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 18_000);

    let clouds = cloud_environment_for_world(&world);

    assert_close4(
        clouds.color,
        [20.0 / 255.0, 20.0 / 255.0, 30.0 / 255.0, 1.0],
    );
    assert_ne!(clouds.color, VANILLA_DEFAULT_CLOUD_COLOR);
}

#[test]
fn cloud_environment_applies_vanilla_weather_cloud_color_layers() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    set_world_weather(&mut world, 1.0, 0.0);
    assert_close4(
        cloud_environment_for_world(&world).color,
        [126.0 / 255.0, 126.0 / 255.0, 126.0 / 255.0, 1.0],
    );

    set_world_weather(&mut world, 1.0, 1.0);
    assert_close4(
        cloud_environment_for_world(&world).color,
        [30.0 / 255.0, 30.0 / 255.0, 30.0 / 255.0, 1.0],
    );
}

#[test]
fn cloud_frame_projects_world_game_time_and_camera_eye_position() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 1234);
    world.set_local_player_pose(local_player_pose([10.0, 64.0, -5.0], 90.0, -10.0));
    let camera_pose = camera_pose_from_world(&world);
    let frame = cloud_frame_for_world(&world, camera_pose, 0.25);

    assert_eq!(frame.camera_position, [10.0, 65.62, -5.0]);
    assert_eq!(frame.game_time, 1234);
    assert_eq!(frame.partial_tick, 0.25);
}

#[test]
fn weather_render_state_projects_overworld_rain_columns_from_world_weather() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 4.0, 0.5], 0.0, 0.0));
    set_world_day_time(&mut world, 96);
    set_world_weather(&mut world, 0.5, 0.0);
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 42));
    let terrain_textures =
        TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
            biome_profile_with_weather(42, 0.8, true),
        ]));

    let state = weather_render_state_for_world(
        &world,
        &terrain_textures,
        camera_pose_from_world(&world),
        0.25,
    );

    assert_eq!(state.frame.radius, VANILLA_WEATHER_RENDER_RADIUS);
    assert_eq!(state.frame.intensity, 0.5);
    assert_eq!(state.frame.camera_position, [0.5, 5.62, 0.5]);
    assert_eq!(
        state.rain_columns.len() + state.snow_columns.len(),
        (VANILLA_WEATHER_RENDER_RADIUS * 2 + 1).pow(2) as usize
    );
    let center = state
        .rain_columns
        .iter()
        .find(|column| column.x == 0 && column.z == 0)
        .expect("center column uses the loaded precipitating biome");
    assert_eq!(center.bottom_y, -5);
    assert_eq!(center.top_y, 15);
    assert_eq!(center.u_offset, 0.0);
    assert!(center.v_offset.is_finite());
}

#[test]
fn weather_render_state_uses_the_selected_graphics_radius() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 4.0, 0.5], 0.0, 0.0));
    set_world_day_time(&mut world, 96);
    set_world_weather(&mut world, 1.0, 0.0);
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 42));
    let terrain_textures =
        TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
            biome_profile_with_weather(42, 0.8, true),
        ]));

    let state = weather_render_state_for_world_with_radius(
        &world,
        &terrain_textures,
        camera_pose_from_world(&world),
        0.25,
        5,
    );

    assert_eq!(state.frame.radius, 5);
    assert_eq!(
        state.rain_columns.len() + state.snow_columns.len(),
        (5_u32 * 2 + 1).pow(2) as usize
    );
}

#[test]
fn weather_render_state_uses_motion_blocking_heightmap_for_column_bounds() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 4.0, 0.5], 0.0, 0.0));
    set_world_day_time(&mut world, 96);
    set_world_weather(&mut world, 1.0, 0.0);
    let mut chunk = empty_lightmap_test_chunk_with_biome(world.dimension(), 42);
    chunk.heightmaps = vec![test_motion_blocking_heightmap(
        world.dimension(),
        &[(0, 0, 8)],
    )];
    world.insert_decoded_chunk(chunk);
    let terrain_textures =
        TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
            biome_profile_with_weather(42, 0.8, true),
        ]));

    let state = weather_render_state_for_world(
        &world,
        &terrain_textures,
        camera_pose_from_world(&world),
        0.25,
    );

    let center = state
        .rain_columns
        .iter()
        .find(|column| column.x == 0 && column.z == 0)
        .expect("center column uses the loaded precipitating biome");
    assert_eq!(center.bottom_y, 8);
    assert_eq!(center.top_y, 15);
}

#[test]
fn weather_tick_samples_vanilla_rain_positions_and_particle_status_counts() {
    let mut all_world = weather_effect_test_world(0);
    let mut all = crate::weather_effect_runtime::WeatherEffectRuntime::new(
        bbb_protocol::packets::ClientParticleStatus::All,
    );

    let effects = all.tick_running(&mut all_world, [7.5, 4.0, 7.5]);

    assert_eq!(effects.particles.len(), 58);
    assert!(effects.sound.is_none());
    let first = &effects.particles[0];
    assert_eq!(
        first.particle.kind,
        bbb_client_world::particles::ParticleKind::RAIN
    );
    assert!((first.position[0] - 2.893_004_380_652_515).abs() < 1.0e-12);
    assert_eq!(first.position[1], 7.0);
    assert!((first.position[2] - 14.265_544_316_978_81).abs() < 1.0e-12);

    let mut decreased_world = weather_effect_test_world(0);
    let mut decreased = crate::weather_effect_runtime::WeatherEffectRuntime::new(
        bbb_protocol::packets::ClientParticleStatus::Decreased,
    );
    assert_eq!(
        decreased
            .tick_running(&mut decreased_world, [7.5, 4.0, 7.5])
            .particles
            .len(),
        32
    );

    let mut minimal_world = weather_effect_test_world(0);
    let mut minimal = crate::weather_effect_runtime::WeatherEffectRuntime::new(
        bbb_protocol::packets::ClientParticleStatus::Minimal,
    );
    assert!(minimal
        .tick_running(&mut minimal_world, [7.5, 4.0, 7.5])
        .particles
        .is_empty());
}

#[test]
fn weather_tick_uses_the_selected_graphics_radius_for_sampling() {
    let mut world = weather_effect_test_world(0);
    let mut runtime = crate::weather_effect_runtime::WeatherEffectRuntime::new_with_radius(
        bbb_protocol::packets::ClientParticleStatus::All,
        5,
    );

    let effects = runtime.tick_running(&mut world, [7.5, 4.0, 7.5]);

    assert_eq!(effects.particles.len(), 27);
    assert!(effects.particles.iter().all(|spawn| {
        (2.0..13.0).contains(&spawn.position[0]) && (2.0..13.0).contains(&spawn.position[2])
    }));
}

#[test]
fn weather_tick_uses_hot_surface_smoke_and_above_rain_sound_throttle() {
    let magma = bbb_client_world::vanilla_block_state_id("minecraft:magma_block", &BTreeMap::new())
        .expect("vanilla magma block state");
    let mut world = weather_effect_test_world(magma);
    let mut runtime = crate::weather_effect_runtime::WeatherEffectRuntime::new(
        bbb_protocol::packets::ClientParticleStatus::All,
    );

    let first = runtime.tick_running(&mut world, [7.5, 4.0, 7.5]);
    assert_eq!(first.particles.len(), 58);
    assert!(first.particles.iter().all(|spawn| spawn.particle.kind
        == bbb_client_world::particles::ParticleKind::SMOKE
        && spawn.position[1] == 8.0));
    assert!(first.sound.is_none());

    let second = runtime.tick_running(&mut world, [7.5, 4.0, 7.5]);
    let sound = second
        .sound
        .expect("second valid rain tick passes throttle");
    let above_id = world
        .resolved_registry_entry_by_resource_id(
            "minecraft:sound_event",
            "minecraft:weather.rain.above",
        )
        .expect("weather above sound registry entry")
        .holder_id;
    assert_eq!(sound.sound.registry_id, Some(above_id));
    assert_eq!(sound.source, "weather");
    assert_eq!(sound.volume, 0.1);
    assert_eq!(sound.pitch, 0.5);
    assert_eq!(sound.position.y, 7.5);
    assert!(!sound.distance_delay);

    let mut normal_world = weather_effect_test_world(magma);
    let mut normal_runtime = crate::weather_effect_runtime::WeatherEffectRuntime::new(
        bbb_protocol::packets::ClientParticleStatus::All,
    );
    assert!(normal_runtime
        .tick_running(&mut normal_world, [7.5, 7.0, 7.5])
        .sound
        .is_none());
    let normal_sound = normal_runtime
        .tick_running(&mut normal_world, [7.5, 7.0, 7.5])
        .sound
        .expect("second valid rain tick passes normal-rain throttle");
    let rain_id = normal_world
        .resolved_registry_entry_by_resource_id("minecraft:sound_event", "minecraft:weather.rain")
        .expect("weather rain sound registry entry")
        .holder_id;
    assert_eq!(normal_sound.sound.registry_id, Some(rain_id));
    assert_eq!(normal_sound.volume, 0.2);
    assert_eq!(normal_sound.pitch, 1.0);
}

#[test]
fn weather_precipitation_uses_cold_biome_for_snow_and_brightens_snow_light() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 7));
    let terrain_textures =
        TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
            biome_profile_with_weather(7, 0.0, true),
        ]));

    assert_eq!(
        weather_precipitation_at(&world, &terrain_textures, BlockPos { x: 0, y: 4, z: 0 }, 63,),
        Some(WeatherPrecipitation::Snow)
    );

    let column = snow_weather_column(
        0,
        0,
        1,
        6,
        VanillaLightCoords::from_stored(1, 7, 0, false),
        96,
        0.25,
    );
    assert_eq!(column.light, [4.0 / 15.0, 9.0 / 15.0]);
    assert!(column.u_offset.is_finite());
    assert!(column.v_offset.is_finite());
}

#[test]
fn weather_precipitation_uses_temperature_noise_and_frozen_modifier() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 7));
    let terrain_textures =
        TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
            biome_profile_with_weather(7, 0.149, true),
        ]));

    assert_eq!(
        weather_precipitation_at(
            &world,
            &terrain_textures,
            BlockPos {
                x: -512,
                y: 81,
                z: -511,
            },
            63,
        ),
        Some(WeatherPrecipitation::Rain)
    );

    let frozen_textures =
        TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
            biome_profile_with_temperature_modifier(7, 0.0, BiomeTemperatureModifier::Frozen),
        ]));
    assert_eq!(
        weather_precipitation_at(&world, &frozen_textures, BlockPos { x: 0, y: 64, z: 0 }, 63,),
        Some(WeatherPrecipitation::Rain)
    );
}

#[test]
fn weather_render_state_is_empty_without_rain_or_weather_dimension() {
    let mut dry = world_with_dimension(0, "minecraft:overworld");
    dry.set_local_player_pose(local_player_pose([0.5, 4.0, 0.5], 0.0, 0.0));
    let terrain_textures = TerrainTextureState::default();
    assert!(weather_render_state_for_world(
        &dry,
        &terrain_textures,
        camera_pose_from_world(&dry),
        0.0,
    )
    .is_empty());

    let mut nether = world_with_dimension(1, "minecraft:the_nether");
    nether.set_local_player_pose(local_player_pose([0.5, 4.0, 0.5], 0.0, 0.0));
    set_world_weather(&mut nether, 1.0, 0.0);
    assert!(weather_render_state_for_world(
        &nether,
        &terrain_textures,
        camera_pose_from_world(&nether),
        0.0,
    )
    .is_empty());
}

#[test]
fn weather_render_state_projects_lightning_bolts_without_rain() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 4.0, 0.5], 0.0, 0.0));
    let mut lightning = test_add_entity(77, VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID);
    lightning.uuid = uuid::Uuid::from_u128(0x1234_5678_9abc_def0);
    lightning.position = bbb_protocol::packets::Vec3d {
        x: 8.0,
        y: 65.0,
        z: -3.0,
    };
    world.apply_add_entity(lightning);
    let projected_sources = world.lightning_bolt_render_sources();
    let [projected] = projected_sources.as_slice() else {
        panic!("expected one world-owned lightning render source");
    };

    let state = weather_render_state_for_world(
        &world,
        &TerrainTextureState::default(),
        camera_pose_from_world(&world),
        0.0,
    );

    assert!(!state.is_empty());
    assert_eq!(state.rain_column_count(), 0);
    assert_eq!(state.snow_column_count(), 0);
    assert_eq!(state.lightning_bolt_count(), 1);
    assert_eq!(state.lightning_bolts[0].position, [8.0, 65.0, -3.0]);
    assert_eq!(state.lightning_bolts[0].seed, projected.seed);
}

#[test]
fn world_border_render_state_matches_vanilla_extract_alpha_and_tint() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    apply_static_border(&mut world, (0.0, 0.0), 64.0);
    // Camera eye at x = 24: 8 blocks from the east border edge (+32).
    world.set_local_player_pose(local_player_pose([24.0, 64.0, 0.0], 0.0, 0.0));

    let state = world_border_render_state_for_world(
        &world,
        camera_pose_from_world(&world),
        2,
        2048.0,
        0.0,
        4_500,
    );

    assert_eq!(state.min_x, -32.0);
    assert_eq!(state.max_x, 32.0);
    assert_eq!(state.min_z, -32.0);
    assert_eq!(state.max_z, 32.0);
    // renderDistance = renderDistanceChunks * 16 (744).
    assert_eq!(state.render_distance, 32.0);
    // alpha = clamp((1 - distanceToBorder / renderDistance)^4, 0, 1)
    // = (1 - 8/32)^4.
    assert_eq!(state.alpha, 0.75_f64.powi(4));
    // Stationary border tint.
    assert_eq!(state.tint, 2_138_367);
    // depthFar = max(renderDistanceBlocks * 4, 128 chunks * 16)
    // = max(128, 2048).
    assert_eq!(state.depth_far, 2_048.0);
    // offset = (millis % 3000) / 3000.
    assert_eq!(state.texture_offset, 0.5);
    assert_eq!(state.camera_position[0], 24.0);
    assert_eq!(state.camera_position[1], f64::from(64.0_f32 + 1.62_f32));
    assert_eq!(state.camera_position[2], 0.0);
}

#[test]
fn world_border_render_state_is_invisible_away_from_the_border() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    apply_static_border(&mut world, (0.0, 0.0), 400.0);

    // Camera deep inside the border (further than renderDistance from every
    // edge): the first extract clause fails.
    world.set_local_player_pose(local_player_pose([0.0, 64.0, 0.0], 0.0, 0.0));
    let inside = world_border_render_state_for_world(
        &world,
        camera_pose_from_world(&world),
        2,
        2048.0,
        0.0,
        0,
    );
    assert_eq!(inside.alpha, 0.0);

    // Camera further than renderDistance outside the border: the second
    // clause fails.
    world.set_local_player_pose(local_player_pose([250.0, 64.0, 0.0], 0.0, 0.0));
    let outside = world_border_render_state_for_world(
        &world,
        camera_pose_from_world(&world),
        2,
        2048.0,
        0.0,
        0,
    );
    assert_eq!(outside.alpha, 0.0);

    // No camera pose: nothing to extract.
    assert_eq!(
        world_border_render_state_for_world(&world, None, 2, 2048.0, 0.0, 0).alpha,
        0.0
    );
}

#[test]
fn world_border_render_state_interpolates_lerping_bounds_and_uses_shrinking_tint() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    apply_static_border(&mut world, (0.0, 0.0), 100.0);
    world.apply_set_border_lerp_size(ProtocolSetBorderLerpSize {
        old_size: 100.0,
        new_size: 50.0,
        lerp_time: 10,
    });
    // One border tick: previousSize = 100, size = lerp(1/10, 100, 50) = 95
    // (431-441).
    world.advance_world_border(1);
    world.set_local_player_pose(local_player_pose([45.0, 64.0, 0.0], 0.0, 0.0));

    let state = world_border_render_state_for_world(
        &world,
        camera_pose_from_world(&world),
        2,
        2048.0,
        0.5,
        0,
    );

    // Bounds interpolate previousSize -> size at the frame partial tick
    // lerp(0.5, 100, 95) / 2 = 48.75.
    assert_eq!(state.min_x, -48.75);
    assert_eq!(state.max_x, 48.75);
    // Shrinking border tint.
    assert_eq!(state.tint, 16_724_016);
    // getDistanceToBorder uses the partial-tick-0 bounds:
    // east distance = lerp(0, 100, 95) / 2 - 45 = 5.
    assert_eq!(state.alpha, (1.0 - 5.0 / 32.0_f64).powi(4));
}

#[test]
fn clear_color_mixes_sunrise_sunset_color_when_camera_faces_sun() {
    let day_time = 71;
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, day_time);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 270.0, 0.0));

    let clear = clear_color_for_world_at_camera_with_render_distance(
        &world,
        camera_pose_from_world(&world),
        12,
        false,
    );

    let fog_color = argb_multiply(
        rgb_u8_to_argb(VANILLA_OVERWORLD_FOG_COLOR),
        sample_periodic_argb_keyframes(
            day_time,
            &VANILLA_OVERWORLD_FOG_COLOR_MULTIPLIER_KEYFRAMES,
            VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
        ),
    );
    let sky_color = argb_multiply(
        rgb_u8_to_argb(VANILLA_OVERWORLD_SKY_COLOR),
        sample_periodic_argb_keyframes(
            day_time,
            &VANILLA_OVERWORLD_SKY_COLOR_MULTIPLIER_KEYFRAMES,
            VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
        ),
    );
    let sunrise_color = sample_periodic_argb_keyframes(
        day_time,
        &VANILLA_OVERWORLD_SUNRISE_SUNSET_COLOR_KEYFRAMES,
        VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
    );
    let looking_at_sun = camera_forward_vector(camera_pose_from_world(&world).unwrap())[0];
    let expected = atmospheric_clear_color(
        argb_srgb_lerp(
            looking_at_sun * argb_alpha(sunrise_color) as f32 / 255.0,
            fog_color,
            argb_opaque(sunrise_color),
        ),
        sky_color,
        12,
    );
    let baseline = clear_color_for_day_time(day_time, 0.0, 0.0);

    assert_clear_color_close(clear, expected);
    assert_ne!(clear, baseline);
}

#[test]
fn clear_color_skips_sunrise_sunset_when_not_facing_sun_or_render_distance_is_low() {
    let day_time = 71;
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, day_time);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 90.0, 0.0));

    let away = clear_color_for_world_at_camera_with_render_distance(
        &world,
        camera_pose_from_world(&world),
        12,
        false,
    );
    let away_baseline = clear_color_for_day_time(day_time, 0.0, 0.0);

    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 270.0, 0.0));
    let low_render_distance = clear_color_for_world_at_camera_with_render_distance(
        &world,
        camera_pose_from_world(&world),
        3,
        false,
    );
    let low_render_distance_baseline = clear_color_for_day_time_with_environment_colors_and_camera(
        day_time,
        0.0,
        0.0,
        Some(VANILLA_OVERWORLD_FOG_COLOR),
        Some(VANILLA_OVERWORLD_SKY_COLOR),
        VanillaLightmapDimensionKind::Overworld,
        None,
        3,
    );

    assert_clear_color_close(away, away_baseline);
    assert_clear_color_close(low_render_distance, low_render_distance_baseline);
}

#[test]
fn clear_color_samples_camera_biome_water_fog_when_eye_is_in_water() {
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

    let clear = clear_color_for_world_at_camera(&world, camera_pose_from_world(&world), false);

    assert_eq!(
        clear,
        clear_color_from_argb(rgb_u8_to_argb([0x02, 0x20, 0x44]))
    );

    world.set_sky_flash_time(2);
    let flashed = clear_color_for_world_at_camera(&world, camera_pose_from_world(&world), false);
    assert_eq!(flashed, clear);
}

#[test]
fn clear_color_brightens_water_fog_with_vanilla_water_vision() {
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

    let clear = clear_color_for_world_at_camera_with_water_vision(
        &world,
        camera_pose_from_world(&world),
        VANILLA_ATMOSPHERIC_FOG_RENDER_DISTANCE_CHUNKS as u32,
        0.5,
        false,
    );

    assert_eq!(
        clear,
        clear_color_from_argb(rgb_u8_to_argb([0x04, 0x4b, 0xa1]))
    );
}

#[test]
fn thunder_scales_by_rain_level_and_synced_sky_does_not_double_darken() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    // Thunder with zero rain must have zero effective thunder (`Level.java:913-915`).
    world.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id: bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_RAIN_LEVEL_CHANGE_ID,
        param: 0.0,
    });
    world.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id: bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_THUNDER_LEVEL_CHANGE_ID,
        param: 1.0,
    });
    let clear_with_dry_thunder =
        clear_color_for_world_at_camera(&world, camera_pose_from_world(&world), false);
    let clear_baseline = clear_color_for_day_time(6_000, 0.0, 0.0);
    assert_clear_color_close(clear_with_dry_thunder, clear_baseline);

    // Synced atmospheric clear color with `uses_builtin_dimension_fallback = false`
    // uses synced sky/fog colors directly without multiplying overworld night keyframes.
    let synced_night = clear_color_for_atmospheric_environment(AtmosphericClearColorInputs {
        day_time: 18_000,
        rain_level: 0.0,
        thunder_level: 0.0,
        fog_color: Some([0x40, 0x50, 0x60]),
        sky_color: Some([0x40, 0x50, 0x60]),
        sun_angle_degrees: Some(180.0),
        sunrise_sunset_color: None,
        sky_fog_end_distance: Some(512.0),
        dimension_kind: VanillaLightmapDimensionKind::Overworld,
        uses_builtin_dimension_fallback: false,
        camera_forward: Some([1.0, 0.0, 0.0]),
        render_distance_chunks: 12,
    });
    assert_clear_color_close(
        synced_night,
        clear_color_from_argb(rgb_u8_to_argb([0x40, 0x50, 0x60])),
    );
}
