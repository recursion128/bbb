use super::*;

#[test]
fn panorama_rotation_scales_with_the_scroll_speed_option() {
    // Vanilla `Panorama.extractRenderState`: `spin += (float)(realtimeDelta *
    // optionsRenderState.panoramaSpeed) * 0.1F`; `panoramaScrollSpeed` 0
    // freezes the pan entirely.
    let delta = HudFrameDeltaTicks {
        game_time: 1.0,
        realtime: 5.0,
    };
    let mut hud = HudRenderRuntime::default();
    assert_eq!(hud.panorama_rotation_degrees(true, delta), -0.5);

    let mut half = HudRenderRuntime::default();
    half.set_panorama_scroll_speed(0.5);
    assert_eq!(half.panorama_rotation_degrees(true, delta), -0.25);

    let mut frozen = HudRenderRuntime::default();
    frozen.set_panorama_scroll_speed(0.0);
    assert_eq!(frozen.panorama_rotation_degrees(true, delta), 0.0);
    // A non-finite input keeps the previously configured speed.
    frozen.set_panorama_scroll_speed(f64::NAN);
    assert_eq!(frozen.panorama_rotation_degrees(true, delta), 0.0);
}

#[test]
fn disconnected_screen_preserves_styled_reason_and_server_report_link() {
    use bbb_protocol::packets::{
        ServerLinkEntry, ServerLinkKnownType, ServerLinkType, ServerLinks,
    };

    let mut input = ClientInputState::default();
    let mut world = WorldStore::new();
    world.apply_server_links(ServerLinks {
        links: vec![
            ServerLinkEntry {
                link_type: ServerLinkType::Known(ServerLinkKnownType::Support),
                url: "https://example.invalid/support".into(),
            },
            ServerLinkEntry {
                link_type: ServerLinkType::Known(ServerLinkKnownType::BugReport),
                url: "https://example.invalid/bug".into(),
            },
        ],
    });
    let reason = bbb_client_net::DisconnectReason::from_styled(vec![bbb_protocol::StyledTextRun {
        text: "Styled reason".to_string(),
        style: bbb_protocol::ComponentStyle {
            bold: Some(true),
            color: Some(0x12_34_56),
            shadow_color: Some(0),
            ..Default::default()
        },
    }]);
    crate::runtime::simulation_step::end_connection_and_show_disconnect(
        &mut world,
        &mut input,
        crate::input::disconnected_screen::DisconnectedScreenTitle::ConnectionLost,
        reason,
    );

    assert!(world.server_links().is_empty());
    let screen = hud_disconnected_screen(&input, None, -12.5).unwrap();
    assert_eq!(screen.panorama_rotation_degrees, -12.5);
    assert_eq!(screen.title, "Connection Lost");
    assert_eq!(
        screen.reconnect_label.as_deref(),
        if cfg!(target_arch = "wasm32") {
            None
        } else {
            Some("Reconnect")
        }
    );
    assert_eq!(
        screen.back_label,
        if cfg!(target_arch = "wasm32") {
            "Reconnect"
        } else {
            "Quit Game"
        }
    );
    assert_eq!(screen.reason_runs[0].visual.text, "Styled reason");
    assert!(screen.reason_runs[0].visual.style.bold);
    assert_eq!(screen.reason_runs[0].visual.color, Some(0x12_34_56));
    assert_eq!(screen.reason_runs[0].visual.shadow_color, Some(0));
    assert_eq!(
        screen.report_to_server_label.as_deref(),
        Some("Report to Server")
    );
    assert_eq!(
        input
            .disconnected_screen_runtime()
            .unwrap()
            .details
            .bug_report_link
            .as_deref(),
        Some("https://example.invalid/bug")
    );
    let narration = disconnected_screen_narration_frame(&input, None).unwrap();
    assert!(narration.all_text_for_test().contains("Styled reason"));
}

#[test]
fn disconnected_screen_title_tracks_vanilla_listener_phase() {
    use crate::input::disconnected_screen::DisconnectedScreenTitle;

    assert_eq!(
        disconnected_screen_title_for_state(None),
        DisconnectedScreenTitle::ConnectFailed
    );
    assert_eq!(
        disconnected_screen_title_for_state(Some("Login")),
        DisconnectedScreenTitle::ConnectFailed
    );
    assert_eq!(
        disconnected_screen_title_for_state(Some("Configuration")),
        DisconnectedScreenTitle::ConnectionLost
    );
    assert_eq!(
        disconnected_screen_title_for_state(Some("Play")),
        DisconnectedScreenTitle::ConnectionLost
    );
    assert!(!remote_disconnect_stops_audio(None));
    assert!(!remote_disconnect_stops_audio(Some("Login")));
    assert!(remote_disconnect_stops_audio(Some("Configuration")));
    assert!(remote_disconnect_stops_audio(Some("Play")));
}

#[test]
fn renderer_surface_recovery_counters_cross_the_control_projection() {
    let source = bbb_client_render::RendererCounters {
        width: 1280,
        height: 720,
        gui_width: 427,
        gui_height: 240,
        surface_resizes: 2,
        surface_zero_size_skips: 1,
        surface_lost: 3,
        surface_outdated: 4,
        surface_timeouts: 5,
        surface_reconfigurations: 7,
        surface_frames_skipped: 12,
        window_resize_requests: 2,
        window_minimize_requests: 1,
        window_restore_requests: 1,
        terrain_selected_chunks: 625,
        terrain_resident_chunks: 624,
        terrain_selected_sections: 15_000,
        terrain_resident_sections: 14_976,
        terrain_pending_jobs: 1,
        terrain_cache_ready: false,
        panorama_resource_ready: true,
        panorama_draw_calls: 2,
        panorama_resident_bytes: 25_165_828,
        ..Default::default()
    };
    let projected = control_renderer_counters(source);
    assert_eq!([projected.width, projected.height], [1280, 720]);
    assert_eq!([projected.gui_width, projected.gui_height], [427, 240]);
    assert_eq!(projected.surface_resizes, 2);
    assert_eq!(projected.surface_zero_size_skips, 1);
    assert_eq!(projected.surface_lost, 3);
    assert_eq!(projected.surface_outdated, 4);
    assert_eq!(projected.surface_timeouts, 5);
    assert_eq!(projected.surface_reconfigurations, 7);
    assert_eq!(projected.surface_frames_skipped, 12);
    assert_eq!(projected.window_resize_requests, 2);
    assert_eq!(projected.window_minimize_requests, 1);
    assert_eq!(projected.window_restore_requests, 1);
    assert_eq!(projected.terrain_selected_chunks, 625);
    assert_eq!(projected.terrain_resident_chunks, 624);
    assert_eq!(projected.terrain_selected_sections, 15_000);
    assert_eq!(projected.terrain_resident_sections, 14_976);
    assert_eq!(projected.terrain_pending_jobs, 1);
    assert!(!projected.terrain_cache_ready);
    assert!(projected.panorama_resource_ready);
    assert_eq!(projected.panorama_draw_calls, 2);
    assert_eq!(projected.panorama_resident_bytes, 25_165_828);
}

#[test]
fn jigsaw_priority_tooltip_hit_testing_matches_priority_fields() {
    let layout = hud_jigsaw_block_editor_layout(winit::dpi::PhysicalSize::new(320, 240));
    assert_eq!(
        jigsaw_priority_tooltip_key(layout, (layout.fields[4].x + 1, 161)).map(|(key, _)| key),
        Some("jigsaw_block.selection_priority.tooltip")
    );
    assert_eq!(
        jigsaw_priority_tooltip_key(layout, (layout.fields[5].x + 1, 161)).map(|(key, _)| key),
        Some("jigsaw_block.placement_priority.tooltip")
    );
    assert!(jigsaw_priority_tooltip_key(layout, (layout.fields[0].x + 1, 21)).is_none());
}

#[test]
fn hud_demo_popup_projects_exact_fallback_copy_and_initial_focus() {
    let mut world = WorldStore::new();
    world.apply_game_event(ProtocolGameEvent {
        event_id: bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_DEMO_EVENT_ID,
        param: 0.0,
    });
    let mut input = ClientInputState::new(true);
    input.open_game_event_screen_with_logo_sample(&mut world, false);
    let resources = Arc::new(crate::win_screen_resources::WinScreenResources::default());
    let mut state = HudRenderRuntime::default();
    let screen = hud_game_event_screen(&mut state, &input, &resources, None).expect("demo HUD");
    let narration = game_event_screen_narration_frame(&screen, &resources, None);
    let HudGameEventScreen::DemoPopup(screen) = screen else {
        panic!("demo popup");
    };
    assert_eq!(screen.title, "Minecraft Demo Mode");
    assert_eq!(
        screen.controls_message,
        "Move by pressing the W, A, S, D keys\nLook around using the mouse\nJump by pressing the Space key\nUse the E key to open your inventory"
    );
    assert!(screen
        .duration_message
        .starts_with("This demo will last 5 in-game days"));
    assert_eq!(screen.focused, HudGameEventButton::DemoBuy);
    assert_eq!(screen.hovered, None);
    assert_eq!(
        narration.all_text_for_test(),
        "Minecraft Demo Mode. Move by pressing the W, A, S, D keys\nLook around using the mouse\nJump by pressing the Space key\nUse the E key to open your inventory. This demo will last 5 in-game days (about 1 hour and 40 minutes of real time). Check the advancements for hints! Have fun!. Purchase Now! button. Screen element 4 out of 5. Press Tab to navigate to the next element. Press Enter to activate"
    );
}

#[test]
fn hud_win_screen_reuses_resource_lines_and_sanitizes_vanilla_realtime_delta() {
    let mut world = WorldStore::new();
    world.apply_game_event(ProtocolGameEvent {
        event_id: bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_WIN_GAME_ID,
        param: 0.0,
    });
    let mut input = ClientInputState::new(true);
    input.open_game_event_screen_with_logo_sample(&mut world, true);
    let resources = Arc::new(crate::win_screen_resources::WinScreenResources {
        lines: vec![crate::win_screen_resources::WinScreenLine {
            runs: vec![HudStyledTextRun::plain("line")],
            centered: true,
        }],
        narration_components: vec!["line".to_string()],
        total_scroll_length: 12,
    });
    let mut state = HudRenderRuntime::default();
    let screen = hud_game_event_screen(&mut state, &input, &resources, None).expect("win HUD");
    let narration = game_event_screen_narration_frame(&screen, &resources, None);
    let HudGameEventScreen::Win(screen) = screen else {
        panic!("win screen");
    };
    assert!(screen.show_easter_egg_logo);
    assert_eq!(screen.lines[0].runs[0].text, "line");
    assert!(Arc::ptr_eq(&screen.lines, &state.win_screen_lines));
    assert_eq!(
        narration.all_text_for_test(),
        "line. Use mouse cursor or Tab button to select element"
    );

    let start = Instant::now();
    let first_delta = state.frame_delta_ticks(start, Duration::from_millis(50));
    assert_eq!(first_delta, HudFrameDeltaTicks::default());
    assert_eq!(
        HudRenderRuntime::win_screen_realtime_delta_ticks(Some(screen.generation), first_delta),
        None
    );
    let ordinary_delta =
        state.frame_delta_ticks(start + Duration::from_millis(50), Duration::from_millis(50));
    assert_eq!(
        ordinary_delta,
        HudFrameDeltaTicks {
            game_time: 1.0,
            realtime: 1.0,
        }
    );
    assert_eq!(
        HudRenderRuntime::win_screen_realtime_delta_ticks(Some(screen.generation), ordinary_delta),
        Some((screen.generation, 1.0))
    );
    let slow_delta = state.frame_delta_ticks(
        start + Duration::from_millis(450),
        Duration::from_millis(50),
    );
    assert_eq!(
        slow_delta,
        HudFrameDeltaTicks {
            game_time: 8.0,
            realtime: 0.5,
        }
    );
    assert_eq!(
        HudRenderRuntime::win_screen_realtime_delta_ticks(Some(screen.generation), slow_delta),
        Some((screen.generation, 0.5))
    );
    let invalid_tick_interval =
        state.frame_delta_ticks(start + Duration::from_millis(500), Duration::ZERO);
    assert_eq!(
        invalid_tick_interval,
        HudFrameDeltaTicks {
            game_time: 0.0,
            realtime: 1.0,
        }
    );
}

#[test]
fn panorama_spin_uses_realtime_ticks_only_while_visible_and_vanilla_wrap() {
    let mut runtime = HudRenderRuntime::default();
    let one_realtime_tick = HudFrameDeltaTicks {
        game_time: 8.0,
        realtime: 1.0,
    };
    assert_eq!(
        runtime.panorama_rotation_degrees(false, one_realtime_tick),
        0.0
    );
    assert_eq!(
        runtime.panorama_rotation_degrees(true, one_realtime_tick),
        -0.1
    );

    runtime.panorama_spin_degrees = 179.9;
    assert_eq!(
        runtime.panorama_rotation_degrees(true, one_realtime_tick),
        180.0,
        "Vanilla Mth.wrapDegrees maps positive 180 to negative 180 before projection"
    );
    assert_eq!(runtime.panorama_spin_degrees, -180.0);
}

#[test]
fn hud_heart_health_frame_matches_vanilla_damage_hold_and_blink_phase() {
    let start = Instant::now();
    let mut runtime = HudRenderRuntime::default();

    assert_eq!(
        runtime.player_health_frame(start, 0, 20, 0),
        HudPlayerHealthFrame {
            display_health: 20,
            blinking: false,
        }
    );

    // The edge arms tick + 20 but this frame uses the phase computed before
    // the edge and holds the old displayed health.
    assert_eq!(
        runtime.player_health_frame(start + Duration::from_millis(100), 1, 16, 20),
        HudPlayerHealthFrame {
            display_health: 20,
            blinking: false,
        }
    );
    // remaining = 17, (17 / 3) % 2 == 1.
    assert_eq!(
        runtime.player_health_frame(start + Duration::from_millis(250), 4, 16, 19),
        HudPlayerHealthFrame {
            display_health: 20,
            blinking: true,
        }
    );
    // The wall-clock comparison is strict: equality does not expire the hold.
    assert_eq!(
        runtime.player_health_frame(start + Duration::from_millis(1_100), 8, 16, 15),
        HudPlayerHealthFrame {
            display_health: 20,
            blinking: false,
        }
    );
    assert_eq!(
        runtime.player_health_frame(start + Duration::from_millis(1_101), 10, 16, 14),
        HudPlayerHealthFrame {
            display_health: 16,
            blinking: true,
        }
    );
}

#[test]
fn hud_heart_health_frame_uses_ten_tick_heal_and_invulnerability_gate() {
    let start = Instant::now();
    let mut runtime = HudRenderRuntime::default();
    assert_eq!(
        runtime.player_health_frame(start, 30, 16, 0),
        HudPlayerHealthFrame {
            display_health: 16,
            blinking: false,
        }
    );

    // A healing edge under invulnerability arms tick + 10, after extracting
    // this frame's false phase.
    assert_eq!(
        runtime.player_health_frame(start + Duration::from_millis(50), 31, 20, 1),
        HudPlayerHealthFrame {
            display_health: 16,
            blinking: false,
        }
    );
    assert_eq!(
        runtime.player_health_frame(start + Duration::from_millis(100), 32, 20, 0),
        HudPlayerHealthFrame {
            display_health: 16,
            blinking: true,
        }
    );
    assert_eq!(
        runtime.player_health_frame(start + Duration::from_millis(500), 41, 20, 0),
        HudPlayerHealthFrame {
            display_health: 16,
            blinking: false,
        }
    );

    // A later edge without invulnerability does not create a new deadline.
    assert_eq!(
        runtime.player_health_frame(start + Duration::from_millis(501), 42, 18, 0),
        HudPlayerHealthFrame {
            display_health: 16,
            blinking: false,
        }
    );
    assert_eq!(runtime.player_health.blink_until_tick, 41);
}

#[test]
fn hud_air_bubble_pop_sound_matches_vanilla_dedup_reset_and_gain_formulas() {
    let mut runtime = HudRenderRuntime::default();
    let popping = HudAirSupply {
        air: 61,
        max_air: 300,
        eye_in_water: true,
        tick_count: 0,
    };

    assert_eq!(
        runtime.air_bubble_pop_sound(popping, true),
        Some(HudAirBubblePopSound {
            volume: 1.0,
            pitch: 1.3,
        })
    );
    assert_eq!(runtime.air_bubble_pop_sound(popping, true), None);

    // Vanilla does not enter `extractAirBubbles` when the HUD/player-health
    // branch is skipped or when a full on-land row is hidden, so neither
    // case resets `lastBubblePopSoundPlayed`.
    let surfaced = HudAirSupply {
        eye_in_water: false,
        ..popping
    };
    assert_eq!(runtime.air_bubble_pop_sound(surfaced, false), None);
    assert_eq!(runtime.air_bubble_pop_sound(popping, true), None);
    assert_eq!(
        runtime.air_bubble_pop_sound(
            HudAirSupply {
                air: 300,
                ..surfaced
            },
            true,
        ),
        None
    );
    assert_eq!(runtime.air_bubble_pop_sound(popping, true), None);

    // A visible, refilling row on land enters the extraction and resets the
    // debounce; the same underwater bursting frame may then sound again.
    assert_eq!(runtime.air_bubble_pop_sound(surfaced, true), None);
    assert_eq!(
        runtime.air_bubble_pop_sound(popping, true),
        Some(HudAirBubblePopSound {
            volume: 1.0,
            pitch: 1.3,
        })
    );
}

#[test]
fn hud_spyglass_scope_matches_vanilla_frame_interpolation_and_visibility() {
    let mut state = HudRenderRuntime::default();

    assert_eq!(
        state.spyglass_scope(0.0, false, true, true),
        Some(HudSpyglassScope { scale: 0.0 })
    );
    assert_eq!(state.spyglass_scope(1.0, true, true, true), None);
    assert_eq!(
        state.spyglass_scope(0.5, false, true, true),
        Some(HudSpyglassScope { scale: 0.28125 })
    );

    assert_eq!(state.spyglass_scope(1.0, false, false, true), None);
    assert_eq!(
        state.spyglass_scope(0.0, false, true, true),
        Some(HudSpyglassScope { scale: 0.703125 })
    );

    assert_eq!(state.spyglass_scope(1.0, false, true, false), None);
    assert_eq!(
        state.spyglass_scope(f32::NAN, false, true, true),
        Some(HudSpyglassScope { scale: 0.5 })
    );
    assert_eq!(
        state.spyglass_scope(4.0, false, true, true),
        Some(HudSpyglassScope { scale: 1.125 })
    );
}

#[test]
fn hud_death_screen_projection_and_narration_track_main_and_quit_focus() {
    let mut world = WorldStore::new();
    world.apply_login(&PlayLogin {
        player_id: 7,
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
    world.apply_player_combat_kill(PlayerCombatKill {
        player_id: 7,
        message: bbb_protocol::RichTextComponent::try_from_component_nbt(
            NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "text".into(),
                    value: NbtSummaryValue::string("You died"),
                },
                NbtSummaryEntry {
                    name: "font".into(),
                    value: NbtSummaryValue::string("example:wide"),
                },
                NbtSummaryEntry {
                    name: "click_event".into(),
                    value: NbtSummaryValue::Compound(vec![
                        NbtSummaryEntry {
                            name: "action".into(),
                            value: NbtSummaryValue::string("open_url"),
                        },
                        NbtSummaryEntry {
                            name: "url".into(),
                            value: NbtSummaryValue::string("https://example.invalid/death"),
                        },
                    ]),
                },
            ]),
        )
        .expect("rich death cause"),
    });
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    input.apply_screen_requests(&mut world, &mut counters, &None);

    let bbb_client_render::HudDeathScreen::Main(screen) =
        hud_death_screen(&input, None).expect("death HUD")
    else {
        panic!("main death screen");
    };
    assert_eq!(screen.title, "You Died!");
    assert_eq!(screen.cause_runs[0].visual.text, "You died");
    assert_eq!(screen.cause_runs[0].font.as_deref(), Some("example:wide"));
    assert_eq!(screen.cause_link_run_indices, vec![Some(0)]);
    assert_eq!(
        screen
            .score_runs
            .iter()
            .map(|run| run.visual.text.as_str())
            .collect::<String>(),
        "Score: 0"
    );
    assert_eq!(
        screen.score_runs.last().and_then(|run| run.visual.color),
        Some(0xFF_FF_55)
    );
    assert!(!screen.buttons_active);

    assert_eq!(
        death_screen_narration_frame(&input, None)
            .expect("death narration")
            .all_text_for_test(),
        "You Died!. Respawn button. Screen element 1 out of 2. Press Tab to navigate to the next element. Press Enter to activate"
    );

    input.advance_death_screen_ticks(20);
    input.handle_death_screen_key(&mut counters, &None, KeyCode::Tab, true);
    input.handle_death_screen_key(&mut counters, &None, KeyCode::Enter, true);
    let bbb_client_render::HudDeathScreen::QuitConfirmation(screen) =
        hud_death_screen(&input, None).expect("quit HUD")
    else {
        panic!("quit confirmation");
    };
    assert_eq!(screen.title, "Are you sure you want to quit?");
    assert!(!screen.buttons_active);
    assert_eq!(
        death_screen_narration_frame(&input, None)
            .expect("quit confirmation narration")
            .all_text_for_test(),
        "Are you sure you want to quit?. Title Screen button. Screen element 1 out of 2. Press Tab to navigate to the next element. Press Enter to activate"
    );
}

#[test]
fn hud_social_interactions_projection_tracks_faces_removed_status_links_and_narration() {
    use bbb_client_render::{
        EntityPlayerSkin, HudSocialInteractionsFocus, HudSocialInteractionsPlayerAction,
        HudSocialInteractionsScreen,
    };
    use bbb_protocol::packets::{
        GameProfile, PlayerInfoAction, PlayerInfoEntry, PlayerInfoRemove, PlayerInfoUpdate,
    };

    let local_id = uuid::Uuid::from_u128(1);
    let remote_id = uuid::Uuid::from_u128(2);
    let mut world = WorldStore::new();
    world.set_local_player_profile_id(local_id);
    world.set_social_server_label("fixture.example");
    world.apply_player_info_update(PlayerInfoUpdate {
        actions: vec![PlayerInfoAction::AddPlayer],
        entries: [(local_id, "Local"), (remote_id, "Remote")]
            .into_iter()
            .map(|(profile_id, name)| PlayerInfoEntry {
                profile_id,
                profile: Some(GameProfile {
                    uuid: profile_id,
                    name: name.to_string(),
                    properties: Vec::new(),
                }),
                listed: true,
                latency: 0,
                game_mode: bbb_protocol::packets::GameType::Survival,
                display_name: None,
                show_hat: false,
                list_order: 0,
                chat_session: None,
            })
            .collect(),
    });
    let mut input = ClientInputState::new(true);
    input.open_social_interactions_screen(&mut world, &mut NetCounters::default(), &None);
    world.hide_social_player(remote_id);
    world.apply_player_info_remove(PlayerInfoRemove {
        profile_ids: vec![remote_id],
    });
    input.sync_social_interactions_screen(&world);

    let HudSocialInteractionsScreen::Main(screen) =
        hud_social_interactions_screen(&input, &world, None, Some((230, 100)), None)
            .expect("social HUD")
    else {
        panic!("main social screen");
    };
    assert_eq!(screen.title, "Social Interactions");
    assert_eq!(screen.server_label, "fixture.example - 1 player");
    assert_eq!(screen.rows.len(), 2);
    assert_eq!(screen.rows[0].name, "Local");
    assert!(screen.rows[0].action.is_none());
    assert!(matches!(
        screen.rows[0].skin,
        EntityPlayerSkin::ProfiledDefault(_)
    ));
    let remote = screen
        .rows
        .iter()
        .find(|player| player.profile_id == *remote_id.as_bytes())
        .expect("retained removed row");
    assert!(remote.removed);
    assert!(remote.hidden);
    assert_eq!(remote.status.as_deref(), Some("Hidden - Offline"));
    assert_eq!(remote.action, Some(HudSocialInteractionsPlayerAction::Show));
    assert_eq!(
        social_interactions_narration_frame(&input, &world, None)
            .expect("main narration")
            .all_text_for_test(),
        "Social Interactions. fixture.example - 1 player. Search.... Type to search"
    );

    for _ in 0..3 {
        input.handle_social_interactions_key(&mut world, KeyCode::Tab, true, &mut None);
    }
    input.handle_social_interactions_key(&mut world, KeyCode::Enter, true, &mut None);
    input.handle_social_interactions_key(&mut world, KeyCode::Tab, true, &mut None);
    assert_eq!(
        input
            .social_interactions_runtime()
            .map(|runtime| runtime.focus()),
        Some(HudSocialInteractionsFocus::BlockingHelp)
    );
    input.handle_social_interactions_key(&mut world, KeyCode::Enter, true, &mut None);
    let HudSocialInteractionsScreen::LinkConfirmation(link) =
        hud_social_interactions_screen(&input, &world, None, None, None)
            .expect("blocking-help link HUD")
    else {
        panic!("link confirmation");
    };
    assert_eq!(link.message, "https://aka.ms/javablocking");
    assert_eq!(link.focus, HudSocialInteractionsFocus::LinkOpen);
    assert!(social_interactions_narration_frame(&input, &world, None)
        .expect("link narration")
        .all_text_for_test()
        .contains("https://aka.ms/javablocking"));
}

#[test]
fn localized_chat_format_preserves_outer_and_argument_styles() {
    let outer = bbb_protocol::ComponentStyle {
        color: Some(0xaaaaaa),
        ..bbb_protocol::ComponentStyle::default()
    };
    let argument = HudStyledTextRun {
        shadow_color: None,
        text: "Alice".to_string(),
        style: HudTextStyle {
            bold: true,
            ..HudTextStyle::default()
        },
        color: Some(0x55ffff),
    };
    let runs = format_localized_chat_runs(
        "<%1$s> %2$s %%",
        &outer,
        &[
            vec![argument.clone()],
            vec![HudStyledTextRun::plain("hello")],
        ],
    );
    assert_eq!(runs[0].text, "<");
    assert_eq!(runs[0].color, Some(0xaaaaaa));
    assert_eq!(runs[1], argument);
    assert_eq!(
        runs.iter().map(|run| run.text.as_str()).collect::<String>(),
        "<Alice> hello %"
    );
}

#[test]
fn rich_chat_projection_retains_click_hover_and_revalidates_canonical_target() {
    use bbb_protocol::{
        ComponentClickEvent, ComponentHoverEvent, NbtSummaryEntry, NbtSummaryValue,
    };

    let component =
        bbb_protocol::RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".into(),
                value: NbtSummaryValue::string("click me"),
            },
            NbtSummaryEntry {
                name: "font".into(),
                value: NbtSummaryValue::string("example:compact"),
            },
            NbtSummaryEntry {
                name: "insertion".into(),
                value: NbtSummaryValue::string("insert me"),
            },
            NbtSummaryEntry {
                name: "click_event".into(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "action".into(),
                        value: NbtSummaryValue::string("suggest_command"),
                    },
                    NbtSummaryEntry {
                        name: "command".into(),
                        value: NbtSummaryValue::string("/seed"),
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
                        value: NbtSummaryValue::string("details"),
                    },
                ]),
            },
        ]))
        .unwrap();
    let mut world = WorldStore::new();
    world.apply_system_chat(bbb_protocol::packets::SystemChat {
        content: Box::new(component),
        overlay: false,
    });

    let messages = hud_chat_messages_from_world(&world, None);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].interactions.len(), 1);
    assert!(messages[0].interactions[0].insertion);
    assert_eq!(messages[0].runs[0].font.as_deref(), Some("example:compact"));
    assert_eq!(
        messages[0].interactions[0].tooltip.as_ref().unwrap().lines[0].runs[0]
            .visual
            .text,
        "details"
    );
    assert_eq!(
        hud_chat_component_interaction(
            &world,
            None,
            messages[0].display_sequence,
            messages[0].interactions[0].target_index,
        ),
        Some(HudChatComponentInteraction {
            click_event: Some(ComponentClickEvent::SuggestCommand {
                command: "/seed".to_string().into(),
            }),
            insertion: Some("insert me".to_string().into()),
        })
    );
    assert!(matches!(
        world.client_chat().messages[0].styled_content[0]
            .style
            .hover_event,
        Some(ComponentHoverEvent::ShowText { .. })
    ));
    assert_eq!(
        world.client_chat().messages[0].styled_content[0]
            .style
            .font
            .as_deref(),
        Some("example:compact")
    );
}

#[test]
fn restrictions_projection_uses_the_canonical_chat_visibility_permissions() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut config = world.chat_presentation_config().clone();
    config.visibility = bbb_client_world::ChatVisibilityState::System;
    world.set_chat_presentation_config(config);
    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyT),
        winit::event::ElementState::Pressed,
    );
    assert!(input.open_chat_restrictions_screen());
    let screen = hud_chat_restrictions_screen(&input, &world, None, Some((10, 20)))
        .expect("system restriction screen");
    assert_eq!(screen.title, "Restrictions");
    assert_eq!(
        screen.reason.text,
        "Player chat is restricted in client settings."
    );
    assert_eq!(screen.permissions.len(), 4);
    assert_eq!(screen.permissions[0].color, Some(0xFF5555));
    assert_eq!(screen.permissions[1].color, Some(0x55FF55));
    assert_eq!(screen.permissions[2].color, Some(0x55FF55));
    assert_eq!(screen.permissions[3].color, Some(0xFF5555));
    assert_eq!(screen.cursor_position, Some((10, 20)));
}

#[test]
fn chat_and_restrictions_reuse_the_screen_narration_frame_owner() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut config = world.chat_presentation_config().clone();
    config.visibility = bbb_client_world::ChatVisibilityState::System;
    world.set_chat_presentation_config(config);
    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyT),
        winit::event::ElementState::Pressed,
    );
    crate::input::handle_text_input(&mut input, &mut counters, &mut world, &None, "hello");
    let chat = chat_screen_narration_frame(&input, &world, None)
        .expect("chat narration")
        .all_text_for_test();
    assert!(chat.contains("Chat screen"));
    assert!(chat.contains("Message to send: hello"));
    assert!(chat.contains("chat edit box: hello"));
    assert!(chat.contains("Chat is restricted"));

    assert!(input.open_chat_restrictions_screen());
    let restrictions = chat_screen_narration_frame(&input, &world, None)
        .expect("restriction narration")
        .all_text_for_test();
    assert!(restrictions.contains("Restrictions"));
    assert!(restrictions.contains("Player chat is restricted"));
    assert!(restrictions.contains("You can't send chat messages"));
    assert!(restrictions.contains("You can send commands"));
    assert!(restrictions.contains("Done button"));
}

#[test]
fn component_hover_show_item_reuses_the_native_item_tooltip_builder() {
    use bbb_protocol::{ComponentHoverEvent, NbtSummaryEntry, NbtSummaryValue};

    let root = unique_runtime_temp_dir("component-hover-item");
    write_runtime_tooltip_item_assets(&root);
    crate::item_registry_test_fixture::write_item_registry_fixture(&root, &["air", "stone"]);
    write_runtime_json(
        &runtime_assets_dir(&root).join("lang").join("en_us.json"),
        r#"{
            "item.minecraft.stone": "Stone",
            "item.components": "%s component(s)"
        }"#,
    );
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let hover = ComponentHoverEvent::ShowItem {
        id: "minecraft:stone".to_string(),
        count: 3,
        components: Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "minecraft:custom_name".into(),
            value: NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "text".into(),
                    value: NbtSummaryValue::string("Hover Stone"),
                },
                NbtSummaryEntry {
                    name: "font".into(),
                    value: NbtSummaryValue::string("example:tooltip"),
                },
            ]),
        }])),
    };
    let tooltip = super::component_hover::hud_component_hover_tooltip(
        &WorldStore::new(),
        Some(&item_runtime),
        Some(&hover),
    )
    .unwrap();
    assert_eq!(tooltip.lines[0].runs[0].visual.text, "Hover Stone");
    assert_eq!(
        tooltip.lines[0].runs[0].font.as_deref(),
        Some("example:tooltip")
    );
    assert!(tooltip.lines[0].runs[0].visual.style.italic);
    assert!(tooltip
        .advanced_lines
        .as_ref()
        .is_some_and(|lines| lines.iter().any(|line| {
            line.runs
                .iter()
                .any(|run| run.visual.text == "minecraft:stone")
        })));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn chat_narration_text_uses_styled_content_without_duplicate_projection() {
    let narration = bbb_client_world::ChatNarrationRequest {
        sequence: 1,
        kind: bbb_client_world::ChatNarrationKind::SystemChat,
        translation_key: None,
        arguments: Vec::new(),
        content_component: None,
        styled_content: vec![
            bbb_protocol::StyledTextRun {
                text: "hello ".to_string(),
                style: bbb_protocol::ComponentStyle::default(),
            },
            bbb_protocol::StyledTextRun {
                text: "world".to_string(),
                style: bbb_protocol::ComponentStyle {
                    bold: Some(true),
                    ..bbb_protocol::ComponentStyle::default()
                },
            },
        ],
        content: "fallback".to_string(),
    };
    assert_eq!(chat_narration_text(&narration, None), "hello world");
}

#[test]
fn game_event_chat_components_localize_for_hud_and_system_narration() {
    let root = unique_runtime_temp_dir("game-event-chat-language");
    write_runtime_bow_item_assets(&root);
    write_runtime_json(
        &runtime_assets_dir(&root).join("lang").join("en_us.json"),
        r#"{
            "block.minecraft.spawn.not_valid": "No respawn anchor",
            "demo.help.movement": "Move with %s, %s, %s, and %s"
        }"#,
    );
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    world.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id:
            bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_NO_RESPAWN_BLOCK_AVAILABLE_ID,
        param: 0.0,
    });
    world.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id: bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_DEMO_EVENT_ID,
        param: 101.0,
    });

    let messages = hud_chat_messages_from_world(&world, Some(&item_runtime));
    assert_eq!(
        messages[0]
            .runs
            .iter()
            .map(|run| run.visual.text.as_str())
            .collect::<String>(),
        "No respawn anchor"
    );
    assert_eq!(
        messages[1]
            .runs
            .iter()
            .map(|run| run.visual.text.as_str())
            .collect::<String>(),
        "Move with W, A, S, and D"
    );
    let narrations = world.take_pending_chat_narrations();
    assert_eq!(
        chat_narration_text(&narrations[0], Some(&item_runtime)),
        "No respawn anchor"
    );
    assert_eq!(
        narrations[0].kind,
        bbb_client_world::ChatNarrationKind::SystemChat
    );
    assert_eq!(
        chat_narration_text(&narrations[1], Some(&item_runtime)),
        "Move with W, A, S, and D"
    );
    assert_eq!(
        narrations[1].kind,
        bbb_client_world::ChatNarrationKind::System
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn chat_render_cache_rebuilds_only_for_display_revision_or_level() {
    let mut world = WorldStore::default();
    let mut runtime = HudRenderRuntime::default();
    assert!(matches!(
        runtime.chat_messages_update(&world, None),
        RendererFrameUpdate::Replace(chat) if chat.messages.is_empty() && !chat.refreshed
    ));
    assert!(matches!(
        runtime.chat_messages_update(&world, None),
        RendererFrameUpdate::Unchanged
    ));

    world.push_client_system_chat_message("hello");
    assert!(matches!(
        runtime.chat_messages_update(&world, None),
        RendererFrameUpdate::Replace(chat)
            if !chat.refreshed
                && chat.messages.len() == 1
                && chat.messages[0]
                    .runs
                    .iter()
                    .any(|run| run.visual.text == "hello")
    ));
    assert!(matches!(
        runtime.chat_messages_update(&world, None),
        RendererFrameUpdate::Unchanged
    ));

    // `clearMessages` empties the list without re-displaying anything.
    world.clear_client_chat_display_messages();
    assert!(matches!(
        runtime.chat_messages_update(&world, None),
        RendererFrameUpdate::Replace(chat) if chat.messages.is_empty() && !chat.refreshed
    ));
}

#[test]
fn hotbar_block_item_model_cache_requires_exact_world_and_input_resource_key() {
    fn update(
        runtime: &mut HudRenderRuntime,
        stable_world_unchanged: bool,
        key: HudHotbarBlockItemModelsCacheKey,
        builds: &std::cell::Cell<u64>,
    ) -> RendererFrameUpdate<Vec<Option<bbb_client_render::HudBlockItemModel>>> {
        runtime.hotbar_block_item_models_update(stable_world_unchanged, key, || {
            builds.set(builds.get() + 1);
            Vec::new()
        })
    }

    let mut runtime = HudRenderRuntime::default();
    let builds = std::cell::Cell::new(0);
    let base = HudHotbarBlockItemModelsCacheKey {
        ordinary_hotbar: true,
        ..Default::default()
    };
    let resources_loaded = HudHotbarBlockItemModelsCacheKey {
        resources_loaded: true,
        ..base
    };
    let reloaded = HudHotbarBlockItemModelsCacheKey {
        resource_generation: 1,
        ..resources_loaded
    };
    let shifted = HudHotbarBlockItemModelsCacheKey {
        shift_down: true,
        ..reloaded
    };
    let keybound = HudHotbarBlockItemModelsCacheKey {
        keybind_context: ItemModelKeybindContext {
            forward: true,
            ..Default::default()
        },
        ..shifted
    };
    let spectator = HudHotbarBlockItemModelsCacheKey {
        ordinary_hotbar: false,
        ..keybound
    };

    for (stable_world_unchanged, key, replace) in [
        (false, base, true),
        (true, base, false),
        (false, base, true),
        (true, resources_loaded, true),
        (true, reloaded, true),
        (true, reloaded, false),
        (true, shifted, true),
        (true, keybound, true),
        (true, spectator, true),
    ] {
        let update = update(&mut runtime, stable_world_unchanged, key, &builds);
        assert_eq!(
            matches!(update, RendererFrameUpdate::Replace(models) if models.is_empty()),
            replace
        );
    }
    assert_eq!(builds.get(), 7);
}

#[test]
fn stable_hud_projection_caches_require_exact_revision_resource_and_input_keys() {
    fn scoreboard_update(
        runtime: &mut HudRenderRuntime,
        key: HudScoreboardSidebarCacheKey,
        builds: &std::cell::Cell<u64>,
    ) -> RendererFrameUpdate<Option<bbb_client_render::HudScoreboardSidebar>> {
        runtime.scoreboard_sidebar_update(key, || {
            builds.set(builds.get() + 1);
            None
        })
    }

    fn player_list_update(
        runtime: &mut HudRenderRuntime,
        key: HudPlayerListCacheKey,
        builds: &std::cell::Cell<u64>,
    ) -> RendererFrameUpdate<Option<bbb_client_render::HudPlayerList>> {
        runtime.player_list_update(key, || {
            builds.set(builds.get() + 1);
            None
        })
    }

    let mut runtime = HudRenderRuntime::default();
    let scoreboard_builds = std::cell::Cell::new(0);
    let scoreboard_base = HudScoreboardSidebarCacheKey::default();
    for key in [
        scoreboard_base,
        scoreboard_base,
        HudScoreboardSidebarCacheKey {
            display_revision: 1,
            ..scoreboard_base
        },
        HudScoreboardSidebarCacheKey {
            level_generation: 1,
            display_revision: 1,
            ..scoreboard_base
        },
        HudScoreboardSidebarCacheKey {
            level_generation: 1,
            display_revision: 1,
            resources_loaded: true,
            ..scoreboard_base
        },
        HudScoreboardSidebarCacheKey {
            level_generation: 1,
            display_revision: 1,
            resource_generation: 1,
            resources_loaded: true,
            ..scoreboard_base
        },
        HudScoreboardSidebarCacheKey {
            level_generation: 1,
            display_revision: 1,
            resource_generation: 1,
            resources_loaded: true,
            hidden: true,
        },
    ] {
        let _ = scoreboard_update(&mut runtime, key, &scoreboard_builds);
    }
    assert_eq!(scoreboard_builds.get(), 6);

    let player_list_builds = std::cell::Cell::new(0);
    let player_list_base = HudPlayerListCacheKey::default();
    for key in [
        player_list_base,
        player_list_base,
        HudPlayerListCacheKey {
            display_revision: 1,
            ..player_list_base
        },
        HudPlayerListCacheKey {
            level_generation: 1,
            display_revision: 1,
            ..player_list_base
        },
        HudPlayerListCacheKey {
            level_generation: 1,
            display_revision: 1,
            profile_presentation_revision: 1,
            ..player_list_base
        },
        HudPlayerListCacheKey {
            level_generation: 1,
            display_revision: 1,
            profile_presentation_revision: 1,
            resources_loaded: true,
            ..player_list_base
        },
        HudPlayerListCacheKey {
            level_generation: 1,
            display_revision: 1,
            profile_presentation_revision: 1,
            resource_generation: 1,
            resources_loaded: true,
            ..player_list_base
        },
        HudPlayerListCacheKey {
            level_generation: 1,
            display_revision: 1,
            profile_presentation_revision: 1,
            resource_generation: 1,
            resources_loaded: true,
            visible: true,
            ..player_list_base
        },
        HudPlayerListCacheKey {
            level_generation: 1,
            display_revision: 1,
            profile_presentation_revision: 1,
            resource_generation: 1,
            resources_loaded: true,
            visible: true,
            show_heads: true,
        },
    ] {
        let _ = player_list_update(&mut runtime, key, &player_list_builds);
    }
    assert_eq!(player_list_builds.get(), 8);
}

#[test]
fn non_entity_render_observation_keeps_recent_and_peak_phase_costs() {
    let mut scratch = NonEntityRenderExtractionScratch::default();
    scratch.record_frame(
        Duration::from_nanos(5),
        Duration::from_nanos(7),
        Duration::from_nanos(11),
        Duration::from_nanos(13),
        Duration::from_nanos(17),
    );
    scratch.record_frame(
        Duration::from_nanos(3),
        Duration::from_nanos(19),
        Duration::from_nanos(2),
        Duration::from_nanos(23),
        Duration::from_nanos(1),
    );

    let observation = scratch.observation();
    assert_eq!(observation.hud_nanos, 3);
    assert_eq!(observation.hud_peak_nanos, 5);
    assert_eq!(observation.item_nanos, 19);
    assert_eq!(observation.item_peak_nanos, 19);
    assert_eq!(observation.block_entity_nanos, 2);
    assert_eq!(observation.block_entity_peak_nanos, 11);
    assert_eq!(observation.world_debug_nanos, 23);
    assert_eq!(observation.world_debug_peak_nanos, 23);
    assert_eq!(observation.apply_nanos, 1);
    assert_eq!(observation.apply_peak_nanos, 17);
}

#[test]
fn non_entity_stable_world_cache_requires_same_revision_and_resource_epoch() {
    let mut scratch = NonEntityRenderExtractionScratch::default();

    assert!(!scratch.begin_stable_world_frame(0));
    assert!(scratch.begin_stable_world_frame(0));
    assert!(!scratch.begin_stable_world_frame(1));
    assert!(scratch.begin_stable_world_frame(1));
    scratch.invalidate_resources();
    assert!(!scratch.begin_stable_world_frame(1));

    let observation = scratch.observation();
    assert_eq!(observation.cache_hits, 8);
    assert_eq!(observation.cache_misses, 12);
    assert_eq!(observation.rebuilt_owners, 12);
}

#[test]
fn map_texture_cache_invalidates_only_on_level_generation_edge() {
    let mut scratch = NonEntityRenderExtractionScratch::default();

    assert!(!scratch.synchronize_map_texture_level(4));
    assert!(!scratch.synchronize_map_texture_level(4));
    assert!(scratch.synchronize_map_texture_level(5));
    assert!(!scratch.synchronize_map_texture_level(5));
}

#[test]
fn item_registry_key_cache_follows_registry_access_revision() {
    let mut world = WorldStore::new();
    world.record_registry_data(bbb_protocol::packets::RegistryData {
        registry: "minecraft:enchantment".to_string(),
        entries: vec![
            bbb_protocol::packets::RegistryDataEntry {
                id: "example:custom".to_string(),
                raw_data: None,
            },
            bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:sharpness".to_string(),
                raw_data: None,
            },
        ],
        raw_payload_len: 0,
    });
    let mut scratch = NonEntityRenderExtractionScratch::default();

    let first = scratch.item_registry_keys(&world);
    let second = scratch.item_registry_keys(&world);
    assert_eq!(first.trim_materials, second.trim_materials);
    assert_eq!(first.trim_patterns, second.trim_patterns);
    assert_eq!(first.enchantments, second.enchantments);
    assert_eq!(
        first.enchantments.as_deref(),
        Some(
            [
                "example:custom".to_string(),
                "minecraft:sharpness".to_string()
            ]
            .as_slice()
        )
    );
    assert_eq!(first.attributes, second.attributes);
    assert_eq!(scratch.observation().cache_misses, 4);
    assert_eq!(scratch.observation().cache_hits, 4);

    world.reset_registry_session();
    let _ = scratch.item_registry_keys(&world);
    assert_eq!(scratch.observation().cache_misses, 8);
    assert_eq!(scratch.observation().cache_hits, 4);
}

#[test]
fn stable_block_entity_cache_uses_exact_revision_filter_and_resource_keys() {
    let mut scratch = NonEntityRenderExtractionScratch::default();
    let builds = Cell::new(0);
    let revision = bbb_client_world::StableBlockEntityRenderRevision {
        client_level_generation: 4,
        terrain: 7,
        block_entity_data: 9,
    };
    let key = StableBlockEntityCacheKey {
        revision,
        text_filtering_enabled: false,
    };
    let build = || {
        builds.set(builds.get() + 1);
        (
            vec![bbb_client_render::EntityModelInstance::cod(
                -1,
                [1.0, 2.0, 3.0],
                0.0,
            )],
            Vec::new(),
            vec![bbb_client_render::EntityModelInstance::cod(
                -1,
                [4.0, 5.0, 6.0],
                0.0,
            )],
            vec![bbb_client_render::EntityModelInstance::cod(
                -1,
                [7.0, 8.0, 9.0],
                0.0,
            )],
        )
    };

    let first = scratch.stable_block_entity_frame(key, build);
    assert_eq!(first.sign_instances[0].position, [1.0, 2.0, 3.0]);
    assert_eq!(first.bed_instances[0].position, [4.0, 5.0, 6.0]);
    assert_eq!(first.lectern_instances[0].position, [7.0, 8.0, 9.0]);
    assert!(matches!(
        first.sign_text_surfaces,
        RendererFrameUpdate::Replace(_)
    ));

    let second = scratch.stable_block_entity_frame(key, build);
    assert!(matches!(
        second.sign_text_surfaces,
        RendererFrameUpdate::Unchanged
    ));
    assert_eq!(builds.get(), 1);

    let changed_revision = scratch.stable_block_entity_frame(
        StableBlockEntityCacheKey {
            revision: bbb_client_world::StableBlockEntityRenderRevision {
                terrain: 8,
                ..revision
            },
            ..key
        },
        build,
    );
    assert!(matches!(
        changed_revision.sign_text_surfaces,
        RendererFrameUpdate::Replace(_)
    ));
    assert_eq!(builds.get(), 2);

    let filtered = scratch.stable_block_entity_frame(
        StableBlockEntityCacheKey {
            revision: bbb_client_world::StableBlockEntityRenderRevision {
                terrain: 8,
                ..revision
            },
            text_filtering_enabled: true,
        },
        build,
    );
    assert!(matches!(
        filtered.sign_text_surfaces,
        RendererFrameUpdate::Replace(_)
    ));
    assert_eq!(builds.get(), 3);

    scratch.invalidate_resources();
    let reloaded = scratch.stable_block_entity_frame(
        StableBlockEntityCacheKey {
            revision: bbb_client_world::StableBlockEntityRenderRevision {
                terrain: 8,
                ..revision
            },
            text_filtering_enabled: true,
        },
        build,
    );
    assert!(matches!(
        reloaded.sign_text_surfaces,
        RendererFrameUpdate::Replace(_)
    ));
    assert_eq!(builds.get(), 4);
    assert_eq!(scratch.observation().cache_hits, 4);
    assert_eq!(scratch.observation().cache_misses, 16);
    assert_eq!(scratch.observation().rebuilt_owners, 16);
}

#[test]
fn block_entity_candidates_follow_camera_columns_and_loaded_blocks() {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    let install = |world: &mut WorldStore, x, name| {
        let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name).unwrap();
        let state =
            bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap();
        let mut chunk = empty_lightmap_test_chunk(world.dimension());
        chunk.pos = ChunkPos { x, z: 0 };
        let section = std::sync::Arc::make_mut(&mut chunk.sections[0]);
        section.block_states = single_value_container(PaletteDomain::BlockStates, 4096, state);
        section.non_empty_block_count = if name == "minecraft:air" { 0 } else { 4096 };
        if name == "minecraft:campfire" {
            std::sync::Arc::make_mut(&mut chunk.block_entities).push(
                bbb_client_world::BlockEntityRecord::empty(
                    0,
                    0,
                    0,
                    bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_CAMPFIRE_ID,
                ),
            );
        }
        world.insert_decoded_chunk(chunk);
    };
    install(&mut world, 4, "minecraft:chest");
    install(&mut world, -5, "minecraft:spawner");
    install(&mut world, 10, "minecraft:end_gateway");
    install(&mut world, 11, "minecraft:campfire");
    let camera = |x, y| {
        Some(CameraPose {
            position: [x, y, 0.0],
            eye_height: 0.0,
            x_rot: 0.0,
            y_rot: 0.0,
        })
    };
    let mut scratch = NonEntityRenderExtractionScratch::default();
    let mut presence = |world: &WorldStore, pose| {
        scratch.dynamic_block_entity_presence(
            world,
            world.stable_block_entity_render_revision(),
            pose,
        )
    };
    let west = presence(&world, camera(-0.25, 2.0));
    assert!(west.chest_columns.is_empty() && west.end_portal);
    assert_eq!(west.spawner_columns, [ChunkPos { x: -5, z: 0 }]);
    let east = presence(&world, camera(0.75, 2.0));
    assert_eq!(east.chest_columns, [ChunkPos { x: 4, z: 0 }]);
    assert!(east.spawner_columns.is_empty() && !east.item_displays);
    assert_eq!(
        presence(&world, camera(-0.25, 20.0)).chest_columns,
        east.chest_columns
    );
    let all = presence(&world, None);
    assert_eq!(all.chest_columns, east.chest_columns);
    assert_eq!(all.spawner_columns, west.spawner_columns);
    assert!(all.item_displays);
    install(&mut world, 4, "minecraft:air");
    assert!(presence(&world, camera(0.75, 2.0)).chest_columns.is_empty());
}

#[test]
fn game_test_highlight_cache_rebuilds_only_for_level_or_marker_revision() {
    let mut scratch = NonEntityRenderExtractionScratch::default();
    let builds = Cell::new(0);
    let build = || {
        builds.set(builds.get() + 1);
        crate::game_test_highlights::GameTestHighlightGizmos {
            filled_boxes: vec![bbb_client_render::DebugFilledBox {
                min: [0.0; 3],
                max: [1.0; 3],
                color: [1, 2, 3, 4],
            }],
            text_billboards: vec![bbb_client_render::DebugTextBillboard::centered(
                [0.5; 3], "marker", [255; 4],
            )],
        }
    };

    let (boxes, text) = scratch.game_test_highlight_update(3, 5, build);
    assert!(matches!(boxes, RendererFrameUpdate::Replace(_)));
    assert!(matches!(text, RendererFrameUpdate::Replace(_)));
    let (boxes, text) = scratch.game_test_highlight_update(3, 5, build);
    assert!(matches!(boxes, RendererFrameUpdate::Unchanged));
    assert!(matches!(text, RendererFrameUpdate::Unchanged));
    let (boxes, text) = scratch.game_test_highlight_update(3, 6, build);
    assert!(matches!(boxes, RendererFrameUpdate::Replace(_)));
    assert!(matches!(text, RendererFrameUpdate::Replace(_)));
    let (boxes, text) = scratch.game_test_highlight_update(4, 6, build);
    assert!(matches!(boxes, RendererFrameUpdate::Replace(_)));
    assert!(matches!(text, RendererFrameUpdate::Replace(_)));

    assert_eq!(builds.get(), 3);
    assert_eq!(scratch.observation().cache_hits, 2);
    assert_eq!(scratch.observation().cache_misses, 6);
    assert_eq!(scratch.observation().rebuilt_owners, 6);
}

#[test]
fn dynamic_debug_buffers_retain_capacity_and_count_only_growth_edges() {
    let mut scratch = NonEntityRenderExtractionScratch::default();
    let mut buffers = scratch.begin_debug_frame();
    buffers.filled_boxes.reserve(8);
    buffers.lines.reserve(4);
    scratch.record_debug_capacity_growths(&buffers);
    assert_eq!(scratch.observation().capacity_growths, 2);

    buffers.filled_boxes.clear();
    buffers.lines.clear();
    scratch.recycle_debug_buffers(buffers);
    let buffers = scratch.begin_debug_frame();
    assert!(buffers.filled_boxes.capacity() >= 8);
    assert!(buffers.lines.capacity() >= 4);
    scratch.record_debug_capacity_growths(&buffers);
    assert_eq!(scratch.observation().capacity_growths, 2);
}

#[test]
fn camera_pose_uses_local_player_eye_height() {
    let mut world = WorldStore::new();
    let standing_pose = LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        y_rot: 45.0,
        x_rot: -10.0,
        ..LocalPlayerPoseState::default()
    };
    world.set_local_player_pose(standing_pose);
    let pose = camera_pose_from_world(&world).unwrap();

    assert_eq!(pose.position, [1.0, 2.0, 3.0]);
    assert_eq!(pose.y_rot, 45.0);
    assert_eq!(pose.x_rot, -10.0);
    assert_eq!(pose.eye_height, CameraPose::STANDING_EYE_HEIGHT);

    let sneaking_pose = LocalPlayerPoseState {
        sneaking: true,
        ..standing_pose
    };
    world.set_local_player_pose(sneaking_pose);
    let pose = camera_pose_from_world(&world).unwrap();
    assert_eq!(
        pose.eye_height,
        world.local_player_eye_height(sneaking_pose) as f32
    );
}

#[test]
fn camera_hurt_effect_follows_the_active_living_camera_entity() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.apply_add_entity(test_add_entity(42, VANILLA_ENTITY_TYPE_PLAYER_ID));
    assert!(world.apply_hurt_animation(bbb_protocol::packets::HurtAnimation { id: 42, yaw: 45.0 }));

    let local = camera_hurt_effect_from_world(&world, 0.5, 0.25).unwrap();
    assert_eq!(local.hurt_time, 9.5);
    assert_eq!(local.hurt_dir, 45.0);
    assert_eq!(local.damage_tilt_strength, 0.25);

    world.apply_add_entity(test_add_entity(100, VANILLA_ENTITY_TYPE_CHICKEN_ID));
    assert!(world.apply_hurt_animation(bbb_protocol::packets::HurtAnimation { id: 100, yaw: 90.0 }));
    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 100 }));
    let remote = camera_hurt_effect_from_world(&world, 0.25, 1.0).unwrap();
    assert_eq!(remote.hurt_time, 9.75);
    assert_eq!(remote.hurt_dir, 0.0);

    world.apply_add_entity(test_add_entity(101, VANILLA_ENTITY_TYPE_ARROW_ID));
    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 101 }));
    assert_eq!(camera_hurt_effect_from_world(&world, 0.25, 1.0), None);
}

#[test]
fn camera_view_bob_effect_follows_active_player_and_startup_gate() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.apply_add_entity(test_add_entity(42, VANILLA_ENTITY_TYPE_PLAYER_ID));
    world.advance_entity_client_animations(1);

    let local = camera_view_bob_effect_from_world(&world, 0.5, true).unwrap();
    let expected = world.probe_entity_camera_view_bob_state(42, 0.5).unwrap();
    assert_eq!(
        local.backwards_interpolated_walk_distance,
        expected.backwards_interpolated_walk_distance
    );
    assert_eq!(local.bob, expected.bob);
    assert_eq!(camera_view_bob_effect_from_world(&world, 0.5, false), None);

    world.apply_add_entity(test_add_entity(100, VANILLA_ENTITY_TYPE_CHICKEN_ID));
    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 100 }));
    assert_eq!(camera_view_bob_effect_from_world(&world, 0.5, true), None);

    world.apply_add_entity(test_add_entity(101, VANILLA_ENTITY_TYPE_PLAYER_ID));
    world.advance_entity_client_animations(1);
    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 101 }));
    assert!(camera_view_bob_effect_from_world(&world, 0.5, true).is_some());
}

#[test]
fn particle_light_block_pos_uses_block_pos_containing_floor() {
    assert_eq!(
        particle_light_block_pos([1.99, 64.0, -0.01]),
        BlockPos { x: 1, y: 64, z: -1 }
    );
}

#[test]
fn particle_scope_context_tracks_local_spyglass_use() {
    let mut world = WorldStore::new();
    let pose = LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        ..LocalPlayerPoseState::default()
    };
    world.set_local_player_pose(pose);
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_SPYGLASS_ID, 1),
    });
    world.set_local_using_item(true);

    let context =
        particle_local_player_scope_context(&world, camera_pose_from_world(&world), true).unwrap();
    assert_eq!(
        context.eye_position,
        camera_eye_position(camera_pose_from_world(&world).unwrap()).map(f64::from)
    );
    assert!(context.first_person);
    assert!(context.scoping);
    let detached_context =
        particle_local_player_scope_context(&world, camera_pose_from_world(&world), false).unwrap();
    assert!(!detached_context.first_person);
    assert!(detached_context.scoping);

    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(1, 1),
    });
    assert_eq!(
        particle_local_player_scope_context(&world, camera_pose_from_world(&world), true),
        None
    );
}

#[test]
fn particle_player_motion_contexts_track_local_and_remote_players() {
    let mut world = WorldStore::new();
    assert!(particle_player_motion_contexts(&world).is_empty());

    world.set_local_player_pose(LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        delta_movement: bbb_protocol::packets::Vec3d {
            x: -0.1,
            y: 0.25,
            z: 0.5,
        },
        ..LocalPlayerPoseState::default()
    });
    // Non-player entities are never nearest-player candidates.
    world.apply_add_entity(test_add_entity(7, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID));
    world.apply_add_entity(bbb_protocol::packets::AddEntity {
        id: 42,
        uuid: uuid::Uuid::from_u128(42),
        entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: bbb_protocol::packets::Vec3d {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        },
        delta_movement: bbb_protocol::packets::Vec3d {
            x: 0.0,
            y: -0.3,
            z: 0.0,
        },
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });

    assert_eq!(
        particle_player_motion_contexts(&world),
        vec![
            ParticlePlayerMotionContext {
                position: [1.0, 2.0, 3.0],
                delta_movement: [-0.1, 0.25, 0.5],
            },
            ParticlePlayerMotionContext {
                position: [4.0, 5.0, 6.0],
                delta_movement: [0.0, -0.3, 0.0],
            },
        ]
    );

    // Vanilla `EntitySelector.NO_SPECTATORS` drops spectator remote players.
    world.apply_player_info_update(bbb_protocol::packets::PlayerInfoUpdate {
        actions: vec![
            bbb_protocol::packets::PlayerInfoAction::AddPlayer,
            bbb_protocol::packets::PlayerInfoAction::UpdateGameMode,
        ],
        entries: vec![bbb_protocol::packets::PlayerInfoEntry {
            profile_id: uuid::Uuid::from_u128(42),
            profile: Some(bbb_protocol::packets::GameProfile {
                uuid: uuid::Uuid::from_u128(42),
                name: "RemoteSpectator".into(),
                properties: Vec::new(),
            }),
            listed: true,
            latency: 0,
            game_mode: bbb_protocol::packets::GameType::Spectator,
            display_name: None,
            show_hat: false,
            list_order: 0,
            chat_session: None,
        }],
    });
    assert_eq!(
        particle_player_motion_contexts(&world),
        vec![ParticlePlayerMotionContext {
            position: [1.0, 2.0, 3.0],
            delta_movement: [-0.1, 0.25, 0.5],
        }]
    );

    // ... and the spectator local player as well.
    world.apply_game_event(ProtocolGameEvent {
        event_id: 3,
        param: 3.0,
    });
    assert!(particle_player_motion_contexts(&world).is_empty());
}

#[test]
fn hud_player_list_projection_uses_scores_rich_names_and_vanilla_visibility() {
    use bbb_client_render::HudPlayerListScore;
    use bbb_protocol::packets::{
        ChatFormatting, ObjectiveRenderType, PlayerTeamMethod, PlayerTeamParameters,
        ScoreboardDisplaySlot, SetDisplayObjective, SetObjective, SetObjectiveMethod,
        SetObjectiveParameters, SetPlayerTeam, SetScore, TeamCollisionRule, TeamVisibility,
    };

    let mut world = WorldStore::new();
    let entries = [
        (
            1u128,
            "alpha",
            0,
            bbb_protocol::packets::GameType::Survival,
            40,
        ),
        (
            2,
            "Zulu",
            5,
            bbb_protocol::packets::GameType::Spectator,
            900,
        ),
        (3, "beta", 5, bbb_protocol::packets::GameType::Creative, 120),
    ];
    world.apply_player_info_update(bbb_protocol::packets::PlayerInfoUpdate {
        actions: vec![
            bbb_protocol::packets::PlayerInfoAction::AddPlayer,
            bbb_protocol::packets::PlayerInfoAction::UpdateListed,
            bbb_protocol::packets::PlayerInfoAction::UpdateGameMode,
            bbb_protocol::packets::PlayerInfoAction::UpdateLatency,
            bbb_protocol::packets::PlayerInfoAction::UpdateListOrder,
            bbb_protocol::packets::PlayerInfoAction::UpdateHat,
        ],
        entries: entries
            .into_iter()
            .map(|(id, name, list_order, game_mode, latency)| {
                bbb_protocol::packets::PlayerInfoEntry {
                    profile_id: uuid::Uuid::from_u128(id),
                    profile: Some(bbb_protocol::packets::GameProfile {
                        uuid: uuid::Uuid::from_u128(id),
                        name: name.to_string(),
                        properties: Vec::new(),
                    }),
                    listed: true,
                    latency,
                    game_mode,
                    display_name: None,
                    show_hat: id == 3,
                    list_order,
                    chat_session: None,
                }
            })
            .collect(),
    });
    world.apply_tab_list(bbb_protocol::packets::TabList {
        header: *styled_boss_component("Welcome"),
        footer: bbb_protocol::RichTextComponent::plain("Online"),
    });
    assert!(world.apply_set_player_team(SetPlayerTeam {
        name: "aqua".into(),
        method: PlayerTeamMethod::Add.into(),
        parameters: Some(PlayerTeamParameters {
            display_name: bbb_protocol::RichTextComponent::plain("Aqua"),
            options: 0,
            nametag_visibility: TeamVisibility::Always,
            collision_rule: TeamCollisionRule::Always,
            color: ChatFormatting::Aqua,
            player_prefix: bbb_protocol::RichTextComponent::plain("["),
            player_suffix: bbb_protocol::RichTextComponent::plain("]"),
        }),
        players: vec!["beta".into()],
    }));
    assert!(world.apply_set_objective(SetObjective {
        objective_name: "tab".into(),
        method: SetObjectiveMethod::Add.into(),
        parameters: Some(SetObjectiveParameters {
            display_name: bbb_protocol::RichTextComponent::plain("Health"),
            render_type: ObjectiveRenderType::Integer,
            number_format: None,
        }),
    }));
    assert!(world.apply_set_display_objective(SetDisplayObjective {
        slot: ScoreboardDisplaySlot::List,
        objective_name: Some("tab".into()),
    }));
    assert!(world.apply_set_score(SetScore {
        owner: "beta".into(),
        objective_name: "tab".into(),
        score: 7,
        display: None,
        number_format: None,
    }));
    let mut input = ClientInputState::new(true);
    assert_eq!(
        hud_player_list_from_world(&input, &world, None, 0, false),
        None
    );
    crate::input::handle_key_input(
        &mut input,
        &mut NetCounters::default(),
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );

    let list = hud_player_list_from_world(&input, &world, None, 42, false).expect("Tab list");
    assert!(!list.show_heads);
    let header = list.header.as_deref().expect("styled header");
    assert_eq!(header.len(), 1);
    assert_eq!(header[0].text, "Welcome");
    assert!(header[0].style.bold);
    assert_eq!(header[0].color, Some(0xFF_55_55));
    assert_eq!(
        list.footer.as_deref(),
        Some([HudStyledTextRun::plain("Online")].as_slice())
    );
    assert_eq!(
        list.entries
            .iter()
            .map(|entry| {
                entry
                    .name_runs
                    .iter()
                    .map(|run| run.text.as_str())
                    .collect::<String>()
            })
            .collect::<Vec<_>>(),
        vec!["[beta]", "Zulu", "alpha"]
    );
    assert!(!list.entries[0].spectator);
    assert!(list.entries[1].spectator);
    assert_eq!(list.entries[0].latency, 120);
    assert_eq!(list.entries[0].profile_id, 3);
    assert!(list.entries[0].show_hat);
    assert_eq!(list.entries[0].name_runs[0].color, Some(0x55FFFF));
    assert!(list.entries[1].name_runs.iter().all(|run| run.style.italic));
    let HudPlayerListScore::Integer(score_runs) =
        list.entries[0].score.as_ref().expect("integer score")
    else {
        panic!("expected integer player-list score");
    };
    assert_eq!(score_runs[0].text, "7");
    assert_eq!(score_runs[0].color, Some(0xFFFF55));
    let HudPlayerListScore::Integer(missing_score_runs) = list.entries[2]
        .score
        .as_ref()
        .expect("missing score formats zero")
    else {
        panic!("expected integer player-list score");
    };
    assert_eq!(missing_score_runs[0].text, "0");
    assert_eq!(list.tick_count, 42);
    assert_eq!(
        hud_player_list_narration_from_world(&world, None),
        "Online players: [beta], Zulu, alpha"
    );
    assert!(
        hud_player_list_from_world(&input, &world, None, 42, true)
            .expect("encrypted Tab list")
            .show_heads
    );

    assert!(world.apply_set_objective(SetObjective {
        objective_name: "tab".into(),
        method: SetObjectiveMethod::Change.into(),
        parameters: Some(SetObjectiveParameters {
            display_name: bbb_protocol::RichTextComponent::plain("Health"),
            render_type: ObjectiveRenderType::Hearts,
            number_format: None,
        }),
    }));
    let hearts = hud_player_list_from_world(&input, &world, None, 43, false).expect("heart list");
    assert_eq!(
        hearts.entries[0].score,
        Some(HudPlayerListScore::Hearts {
            value: 7,
            hp_text: "3.5hp".to_string(),
            compact_text: "3.5".to_string(),
        })
    );
    assert_eq!(
        hearts.entries[2].score,
        Some(HudPlayerListScore::Hearts {
            value: 0,
            hp_text: "0.0hp".to_string(),
            compact_text: "0.0".to_string(),
        })
    );

    crate::input::handle_key_input(
        &mut input,
        &mut NetCounters::default(),
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::F1),
        ElementState::Pressed,
    );
    assert_eq!(
        hud_player_list_from_world(&input, &world, None, 44, false),
        None
    );
}

#[test]
fn hud_player_list_heads_use_hat_skin_and_loaded_upside_down_profile_state() {
    let mut world = WorldStore::new();
    world.apply_player_info_update(bbb_protocol::packets::PlayerInfoUpdate {
        actions: vec![
            bbb_protocol::packets::PlayerInfoAction::AddPlayer,
            bbb_protocol::packets::PlayerInfoAction::UpdateListed,
            bbb_protocol::packets::PlayerInfoAction::UpdateHat,
        ],
        entries: [("Dinnerbone", true), ("Grumm", false)]
            .into_iter()
            .enumerate()
            .map(|(offset, (name, show_hat))| {
                let id = (offset + 10) as u128;
                bbb_protocol::packets::PlayerInfoEntry {
                    profile_id: uuid::Uuid::from_u128(id),
                    profile: Some(bbb_protocol::packets::GameProfile {
                        uuid: uuid::Uuid::from_u128(id),
                        name: name.to_string(),
                        properties: Vec::new(),
                    }),
                    listed: true,
                    latency: 0,
                    game_mode: bbb_protocol::packets::GameType::Survival,
                    display_name: None,
                    show_hat,
                    list_order: 0,
                    chat_session: None,
                }
            })
            .collect(),
    });
    world.apply_add_entity(test_add_entity(
        10,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    let mut input = ClientInputState::new(true);
    crate::input::handle_key_input(
        &mut input,
        &mut NetCounters::default(),
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );

    let list = hud_player_list_from_world(&input, &world, None, 0, true).expect("Tab list");
    assert!(list.show_heads);
    let dinnerbone = list
        .entries
        .iter()
        .find(|entry| entry.profile_id == 10)
        .unwrap();
    assert!(dinnerbone.show_hat);
    assert!(dinnerbone.upside_down);
    assert!(matches!(
        dinnerbone.skin,
        bbb_client_render::EntityPlayerSkin::ProfiledDefault(_)
    ));
    let grumm = list
        .entries
        .iter()
        .find(|entry| entry.profile_id == 11)
        .unwrap();
    assert!(!grumm.show_hat);
    assert!(!grumm.upside_down);
}

#[test]
fn hud_spectator_projection_substitutes_root_and_player_pages() {
    let mut world = WorldStore::new();
    world.apply_game_event(ProtocolGameEvent {
        event_id: 3,
        param: 3.0,
    });
    let profile_id = uuid::Uuid::from_u128(0x1234);
    world.apply_player_info_update(bbb_protocol::packets::PlayerInfoUpdate {
        actions: vec![
            bbb_protocol::packets::PlayerInfoAction::AddPlayer,
            bbb_protocol::packets::PlayerInfoAction::UpdateListed,
            bbb_protocol::packets::PlayerInfoAction::UpdateGameMode,
        ],
        entries: vec![bbb_protocol::packets::PlayerInfoEntry {
            profile_id,
            profile: Some(bbb_protocol::packets::GameProfile {
                uuid: profile_id,
                name: "Target".into(),
                properties: Vec::new(),
            }),
            listed: true,
            latency: 0,
            game_mode: bbb_protocol::packets::GameType::Survival,
            display_name: None,
            show_hat: true,
            list_order: 0,
            chat_session: None,
        }],
    });
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    assert!(input.handle_spectator_hotbar_action(&world, &mut counters, &None));

    let HudHotbarMode::Spectator(Some(root)) =
        hud_hotbar_mode_from_world(&input, &world, None, Instant::now())
    else {
        panic!("spectator root hotbar");
    };
    assert!(root.alpha > 0.99);
    assert_eq!(root.selected_slot, None);
    assert_eq!(
        root.prompt_runs[0].text,
        "Press a key to select a command, and again to use it."
    );
    assert!(matches!(
        root.slots[0].as_ref().unwrap(),
        bbb_client_render::HudSpectatorGuiSlot {
            icon: bbb_client_render::HudSpectatorIcon::TeleportToPlayer,
            enabled: true,
            ..
        }
    ));
    assert!(matches!(
        root.slots[1].as_ref().unwrap(),
        bbb_client_render::HudSpectatorGuiSlot {
            icon: bbb_client_render::HudSpectatorIcon::TeleportToTeam,
            enabled: false,
            ..
        }
    ));

    input.handle_spectator_hotbar_selected(&world, &mut counters, &None, 0);
    input.handle_spectator_hotbar_selected(&world, &mut counters, &None, 0);
    let HudHotbarMode::Spectator(Some(players)) =
        hud_hotbar_mode_from_world(&input, &world, None, Instant::now())
    else {
        panic!("spectator player hotbar");
    };
    assert_eq!(players.selected_slot, None);
    assert_eq!(
        players.prompt_runs[0].text,
        "Select a player to teleport to"
    );
    let player = players.slots[0].as_ref().unwrap();
    assert_eq!(player.name_runs[0].text, "Target");
    assert!(matches!(
        player.icon,
        bbb_client_render::HudSpectatorIcon::Player {
            skin: bbb_client_render::EntityPlayerSkin::ProfiledDefault(_)
        }
    ));
}

#[test]
fn hud_spectator_projection_preserves_ordinary_and_hidden_hotbar_modes() {
    let mut world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    assert!(matches!(
        hud_hotbar_mode_from_world(&input, &world, None, Instant::now()),
        HudHotbarMode::Ordinary
    ));
    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::F1),
        ElementState::Pressed,
    );
    assert!(matches!(
        hud_hotbar_mode_from_world(&input, &world, None, Instant::now()),
        HudHotbarMode::Hidden
    ));
}

#[test]
fn the_selected_item_name_timer_restarts_on_a_new_item_and_counts_down_to_zero() {
    fn hover(text: &str) -> crate::item_models::tooltip::NativeItemHoverName {
        crate::item_models::tooltip::NativeItemHoverName {
            text: text.to_string(),
            runs: vec![bbb_client_render::HudComponentTextRun::plain(text)],
        }
    }

    let mut runtime = HudRenderRuntime::default();
    let first = runtime
        .selected_item_name_frame(1, false, Some(hover("Diamond Sword")), Some(7), 1.0)
        .expect("a new selected item opens the highlight window");
    assert_eq!(first.highlight_ticks, 40);
    assert!(!first.raised);
    assert_eq!(first.runs[0].visual.text, "Diamond Sword");

    let ticked = runtime
        .selected_item_name_frame(10, false, Some(hover("Diamond Sword")), Some(7), 1.0)
        .expect("the same stack keeps counting down");
    assert_eq!(ticked.highlight_ticks, 30);

    let renamed = runtime
        .selected_item_name_frame(1, true, Some(hover("Excalibur")), Some(7), 1.0)
        .expect("a changed hover name restarts the window even for the same item");
    assert_eq!(renamed.highlight_ticks, 40);
    assert!(renamed.raised);

    let swapped = runtime
        .selected_item_name_frame(1, false, Some(hover("Excalibur")), Some(8), 0.5)
        .expect("a changed item id restarts with the configured display time");
    assert_eq!(swapped.highlight_ticks, 20);

    assert!(runtime
        .selected_item_name_frame(1, false, None, None, 1.0)
        .is_none());

    assert!(runtime
        .selected_item_name_frame(41, false, Some(hover("Excalibur")), Some(8), 1.0)
        .is_none());
}

#[test]
fn a_non_player_camera_entity_hides_the_ordinary_item_hotbar() {
    let mut world = WorldStore::new();
    let input = ClientInputState::new(true);
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
    world.apply_add_entity(test_add_entity(
        700,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    assert!(matches!(
        hud_hotbar_mode_from_world(&input, &world, None, Instant::now()),
        HudHotbarMode::Ordinary
    ));

    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 700 }));
    assert!(matches!(
        hud_hotbar_mode_from_world(&input, &world, None, Instant::now()),
        HudHotbarMode::Hidden
    ));

    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 42 }));
    assert!(matches!(
        hud_hotbar_mode_from_world(&input, &world, None, Instant::now()),
        HudHotbarMode::Ordinary
    ));
}

#[test]
fn a_remote_camera_player_keeps_the_hotbar_frame_but_projects_no_local_inventory() {
    let mut world = WorldStore::new();
    let input = ClientInputState::new(true);
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
    world.apply_add_entity(test_add_entity(
        800,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: bbb_protocol::equipment_slots::VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(VANILLA_ITEM_SPYGLASS_ID, 1),
    });
    assert!(world.hud_camera_player_is_local_player());
    assert_eq!(hud_offhand_slot_side(&world), Some(true));

    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 800 }));
    assert!(!world.hud_camera_player_is_local_player());
    assert!(matches!(
        hud_hotbar_mode_from_world(&input, &world, None, Instant::now()),
        HudHotbarMode::Ordinary
    ));
    assert!(hotbar_item_icons(&world, None, 0.0)
        .iter()
        .all(Option::is_none));
    assert!(hotbar_block_item_models(
        &HudItemModelContext::new(&world, None),
        &TerrainTextureState::default(),
        false,
        ItemModelKeybindContext::default(),
    )
    .iter()
    .all(Option::is_none));
    assert_eq!(hud_offhand_slot_side(&world), None);
}

#[test]
fn localized_identity_and_empty_values_survive_native_ui_projection() {
    let root = unique_runtime_temp_dir("localized-identity");
    write_runtime_bow_item_assets(&root);
    write_runtime_json(
        &runtime_assets_dir(&root).join("lang").join("en_us.json"),
        r#"{"test.identity":"test.identity","test.empty":"","test.advancement":"%s","gui.advancements":"§aGoals%%","advancements.empty":"Empty","advancements.sad_label":"Sad","gui.done":"Close","advancements.progress":"%2$s|%1$s","deathScreen.score.value":"","\ud800":"Raw %s","\ufffd":"Replacement %s","?":"Question %s"}"#,
    );
    let runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let raw_key_component = bbb_protocol::RichTextComponent::try_from_component_nbt(
        bbb_protocol::NbtSummaryValue::Compound(vec![
            bbb_protocol::NbtSummaryEntry {
                name: "translate".into(),
                value: bbb_protocol::NbtSummaryValue::string_from_utf16(vec![0xd800]),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "with".into(),
                value: bbb_protocol::NbtSummaryValue::List(vec![
                    bbb_protocol::NbtSummaryValue::string("X"),
                ]),
            },
        ]),
    )
    .unwrap();
    assert_eq!(
        localized_component_runs(
            &raw_key_component,
            &bbb_protocol::ComponentStyle::default(),
            Some(&runtime)
        )
        .into_iter()
        .map(|run| run.text)
        .collect::<String>(),
        "Raw X",
    );
    let static_fonts = approximate_inventory_hud_fonts();
    let static_labels =
        advancements_screen_text_labels(None, 0, 0, 0, 0, &static_fonts, Some(&runtime));
    assert_eq!(
        static_labels
            .iter()
            .map(|label| label.text.as_str())
            .collect::<Vec<_>>(),
        ["Goals%", "Empty", "Sad", "Close"]
    );
    assert_eq!(
        static_labels[0].component_runs[0].visual.color,
        Some(0x55ff55)
    );
    let progress = advancement_text::advancement_progress_runs(1, 3, Some(&runtime));
    assert_eq!(
        progress
            .iter()
            .map(|run| run.visual.text.as_str())
            .collect::<String>(),
        "3|1"
    );
    let advancement = bbb_protocol::RichTextComponent::try_from_component_nbt(
        bbb_protocol::json_value_to_nbt_summary(&serde_json::json!({
            "translate": "test.advancement", "fallback": "wrong", "with": ["§aAB"]
        }))
        .unwrap(),
    )
    .unwrap();
    let mut glyphs = bbb_client_render::HudFontGlyphMap::new();
    glyphs.set_missing_glyph(bbb_client_render::HudAsciiGlyph::empty(1.0));
    let fonts = HudFontGlyphBook::from(glyphs);
    let labels =
        advancements_screen_text_labels(Some(&advancement), 0, 0, 0, 0, &fonts, Some(&runtime));
    assert_eq!(labels[0].text, "AB");
    assert_eq!(labels[0].width, 2);
    assert_eq!(labels[0].component_runs[0].visual.color, Some(0x55ff55));
    let lines = advancement_wrap_text_lines(&advancement, 1, &fonts, Some(&runtime));
    assert_eq!(
        lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>(),
        ["A", "B"]
    );
    assert!(lines
        .iter()
        .all(|line| line.runs[0].visual.color == Some(0x55ff55)));
    assert_eq!(
        localized_gui_text(Some(&runtime), "test.identity", "wrong"),
        "test.identity"
    );
    assert_eq!(
        localized_gui_text(Some(&runtime), "test.empty", "wrong"),
        ""
    );
    assert_eq!(
        localized_gui_text(Some(&runtime), "test.missing", "fallback"),
        "fallback"
    );
    let component = bbb_protocol::RichTextComponent::try_from_component_nbt(
        bbb_protocol::json_value_to_nbt_summary(&serde_json::json!({
            "translate": "test.identity", "fallback": "wrong"
        }))
        .unwrap(),
    )
    .unwrap();
    let runs = localized_component_runs(&component, &ComponentStyle::default(), Some(&runtime));
    assert_eq!(
        runs.iter().map(|run| run.text.as_str()).collect::<String>(),
        "test.identity"
    );
    let mut world = WorldStore::new();
    world.apply_login(&PlayLogin {
        player_id: 7,
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
    world.apply_player_combat_kill(PlayerCombatKill {
        player_id: 7,
        message: component,
    });
    let mut input = ClientInputState::new(true);
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    let bbb_client_render::HudDeathScreen::Main(screen) =
        hud_death_screen(&input, Some(&runtime)).unwrap()
    else {
        panic!("main death screen");
    };
    assert!(screen.score_runs.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}
