use super::*;

#[test]
fn hud_debug_overlay_projects_custom_looking_at_entity() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    let mut entries = (0..=VANILLA_ENTITY_TYPE_ZOMBIE_ID)
        .map(|id| RegistryPacketEntry::stub(format!("minecraft:test_entity_{id}")))
        .collect::<Vec<_>>();
    entries[VANILLA_ENTITY_TYPE_ZOMBIE_ID as usize] =
        RegistryPacketEntry::stub("minecraft:debug_target_zombie");
    world.record_registry_entries("minecraft:entity_type", 0, entries);
    let mut target = test_add_entity(77, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    target.position = Vec3d {
        x: 0.5,
        y: 0.0,
        z: 0.0,
    };
    world.apply_add_entity(target);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::LookingAtEntity,
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
    .expect("custom looking-at entity entry should show target entity");

    assert_eq!(
        overlay.left_lines,
        vec![
            "Targeted Entity".to_string(),
            "minecraft:debug_target_zombie".to_string(),
        ]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_entity_type_name_uses_world_registry_source_rules() {
    let mut world = WorldStore::new();
    assert_eq!(
        hud_debug_entity_type_name(&world, VANILLA_ENTITY_TYPE_ZOMBIE_ID).as_deref(),
        Some("minecraft:zombie")
    );

    world.record_registry_entries(
        "minecraft:entity_type",
        0,
        vec![RegistryPacketEntry::stub("example:server_first")],
    );
    assert_eq!(
        hud_debug_entity_type_name(&world, 0).as_deref(),
        Some("example:server_first")
    );
    assert_eq!(
        hud_debug_entity_type_name(&world, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
        None
    );
}

#[test]
fn hud_debug_overlay_projects_custom_looking_at_entity_tags() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    apply_entity_type_tags(
        &mut world,
        vec![
            ("minecraft:arrows", vec![VANILLA_ENTITY_TYPE_ARROW_ID]),
            ("minecraft:hostile", vec![VANILLA_ENTITY_TYPE_ZOMBIE_ID]),
            ("minecraft:zombies", vec![VANILLA_ENTITY_TYPE_ZOMBIE_ID]),
        ],
    );
    let mut target = test_add_entity(77, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    target.position = Vec3d {
        x: 0.5,
        y: 0.0,
        z: 0.0,
    };
    world.apply_add_entity(target);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::LookingAtEntityTags,
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
    .expect("custom looking-at entity-tags entry should show target tags");

    assert_eq!(
        overlay.left_lines,
        vec![
            "#minecraft:hostile".to_string(),
            "#minecraft:zombies".to_string(),
        ]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_chunk_render_stats_under_reduced_debug_info() {
    let world =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 384, true);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::ChunkRenderStats,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let renderer_counters = bbb_client_render::RendererCounters {
        visible_sections: 7,
        uploaded_sections: 10,
        queued_sections: 3,
        ..Default::default()
    };

    let overlay = hud_debug_overlay(HudDebugOverlayInputs {
        renderer_counters: &renderer_counters,
        render_distance_chunks: 12,
        ..HudDebugTestDefaults::default().inputs(
            &input,
            &world,
            Some(CameraPose {
                position: [0.5, 0.0, -2.5],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: 1.62,
            }),
        )
    })
    .expect("chunk render stats should be allowed under reduced debug info");

    assert_eq!(
        overlay.left_lines,
        vec!["C: 7/10 (s) D: 12, pC: 003, aB: 00".to_string()]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_suppresses_custom_chunk_generation_stats_without_integrated_server() {
    let world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::ChunkGenerationStats,
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
fn hud_debug_overlay_projects_current_frame_entity_render_stats_under_reduced_debug_info() {
    let mut world =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 384, true);
    world.apply_simulation_distance(bbb_protocol::packets::SetSimulationDistance { distance: 12 });
    world.apply_add_entity(test_add_entity(7, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID));
    world.apply_add_entity(test_add_entity(8, VANILLA_ENTITY_TYPE_PLAYER_ID));
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::EntityRenderStats,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(HudDebugOverlayInputs {
        visible_entity_render_state_count: 1,
        ..HudDebugTestDefaults::default().inputs(
            &input,
            &world,
            Some(CameraPose {
                position: [0.5, 0.0, -2.5],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: 1.62,
            }),
        )
    })
    .expect("entity render stats should be allowed under reduced debug info");

    // ClientLevel.getEntityCount includes the LocalPlayer.
    assert_eq!(overlay.left_lines, vec!["E: 1/3, SD: 12".to_string()]);
    assert!(overlay.right_lines.is_empty());
    assert_eq!(
        hud_debug_entity_render_stats_line(&WorldStore::new(), 0),
        "E: 0/0, SD: 0"
    );
}

#[test]
fn hud_debug_overlay_projects_custom_particle_render_stats() {
    let world = world_with_dimension(0, "minecraft:overworld");
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::ParticleRenderStats,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let renderer_counters = bbb_client_render::RendererCounters {
        active_particle_instances: 23,
        ..Default::default()
    };

    let overlay = hud_debug_overlay(HudDebugOverlayInputs {
        renderer_counters: &renderer_counters,
        ..HudDebugTestDefaults::default().inputs(
            &input,
            &world,
            Some(CameraPose {
                position: [0.5, 0.0, -2.5],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: 1.62,
            }),
        )
    })
    .expect("particle render stats should show outside reduced debug info");

    assert_eq!(overlay.left_lines, vec!["P: 23".to_string()]);
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_chunk_source_stats_under_reduced_debug_info() {
    let mut world =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 384, true);
    world.apply_set_chunk_cache_radius(bbb_protocol::packets::SetChunkCacheRadius { radius: 2 });
    let mut first_chunk = empty_lightmap_test_chunk(world.dimension());
    first_chunk.pos = ChunkPos { x: 0, z: 0 };
    world.insert_decoded_chunk(first_chunk);
    let mut second_chunk = empty_lightmap_test_chunk(world.dimension());
    second_chunk.pos = ChunkPos { x: 1, z: 0 };
    world.insert_decoded_chunk(second_chunk);
    world.apply_add_entity(test_add_entity(7, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID));
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::ChunkSourceStats,
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
    .expect("chunk source stats should be allowed under reduced debug info");

    assert_eq!(
        overlay.left_lines,
        vec!["Chunks[C] W: 121, 2 E: 2,0,2".to_string()]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_sound_cache_under_reduced_debug_info() {
    let world =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 384, true);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::SoundCache,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let audio_counters = AudioCounters {
        sound_cache_buffers: 7,
        sound_cache_bytes: 1_048_577,
        ..AudioCounters::default()
    };

    assert_eq!(hud_debug_sound_cache_bytes_to_mebibytes(0), 0);
    assert_eq!(hud_debug_sound_cache_bytes_to_mebibytes(1), 1);
    assert_eq!(hud_debug_sound_cache_bytes_to_mebibytes(2_097_152), 2);

    let overlay = hud_debug_overlay(HudDebugOverlayInputs {
        audio_counters: &audio_counters,
        ..HudDebugTestDefaults::default().inputs(
            &input,
            &world,
            Some(CameraPose {
                position: [0.5, 0.0, -2.5],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: 1.62,
            }),
        )
    })
    .expect("sound cache should be allowed under reduced debug info");

    assert_eq!(
        overlay.left_lines,
        vec!["Sound cache: 7 buffers, 2 MiB".to_string()]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_sound_mood() {
    let world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::SoundMood,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let audio_counters = AudioCounters {
        sound_static_channels_used: 3,
        sound_static_channels_capacity: 32,
        sound_streaming_channels_used: 1,
        sound_streaming_channels_capacity: 8,
        sound_mood_percent: 42,
        ..AudioCounters::default()
    };

    let overlay = hud_debug_overlay(HudDebugOverlayInputs {
        audio_counters: &audio_counters,
        ..HudDebugTestDefaults::default().inputs(
            &input,
            &world,
            Some(CameraPose {
                position: [0.5, 0.0, -2.5],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: 1.62,
            }),
        )
    })
    .expect("custom sound mood entry should show while always-on");

    assert_eq!(
        overlay.left_lines,
        vec!["Sounds: 3/32 + 1/8 (Mood 42%)".to_string()]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_suppresses_post_effect_without_current_effect() {
    let world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let mut input = ClientInputState::new(true);
    input.synchronize_camera_post_effect(
        world.client_level_generation(),
        world.counters().set_camera_updates_applied,
        current_post_effect_id_from_world(&world),
    );
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::PostEffect,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(HudDebugTestDefaults::default().inputs(&input, &world, None));

    assert!(
        overlay.is_none(),
        "post-effect entry should not render until a current post effect exists"
    );
}

#[test]
fn hud_debug_post_effect_tracks_vanilla_camera_entity_families() {
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    for (entity_id, entity_type_id, expected) in [
        (100, VANILLA_ENTITY_TYPE_CREEPER_ID, "minecraft:creeper"),
        (101, VANILLA_ENTITY_TYPE_SPIDER_ID, "minecraft:spider"),
        (102, VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID, "minecraft:spider"),
        (103, VANILLA_ENTITY_TYPE_ENDERMAN_ID, "minecraft:invert"),
    ] {
        world.apply_add_entity(test_add_entity(entity_id, entity_type_id));
        assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera {
            camera_id: entity_id,
        }));
        assert_eq!(current_post_effect_id_from_world(&world), Some(expected));
        assert_eq!(
            hud_debug_post_effect_line(Some(expected)).as_deref(),
            Some(format!("Post: {expected}").as_str())
        );
    }

    let mut input = ClientInputState::new(true);
    input.synchronize_camera_post_effect(
        world.client_level_generation(),
        world.counters().set_camera_updates_applied,
        current_post_effect_id_from_world(&world),
    );
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::PostEffect,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let overlay = hud_debug_overlay(HudDebugTestDefaults::default().inputs(&input, &world, None))
        .expect("current enderman camera post effect should render");
    assert_eq!(overlay.left_lines, vec!["Post: minecraft:invert"]);

    world.apply_add_entity(test_add_entity(104, VANILLA_ENTITY_TYPE_PLAYER_ID));
    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 104 }));
    input.synchronize_camera_post_effect(
        world.client_level_generation(),
        world.counters().set_camera_updates_applied,
        current_post_effect_id_from_world(&world),
    );
    assert_eq!(current_post_effect_id_from_world(&world), None);
    assert_eq!(input.camera_post_effect_id(), None);
}

#[test]
fn hud_debug_post_effect_is_filtered_by_reduced_debug_info() {
    let mut world =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 384, true);
    world.apply_add_entity(test_add_entity(100, VANILLA_ENTITY_TYPE_CREEPER_ID));
    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 100 }));
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::PostEffect,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    assert!(
        hud_debug_overlay(HudDebugTestDefaults::default().inputs(&input, &world, None)).is_none()
    );
}

#[test]
fn hud_debug_overlay_filters_default_entries_in_reduced_debug_info() {
    let world =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 384, true);
    let mut input = ClientInputState::new(true);
    let fps_sampler = hud_debug_fps_sampler_with_reported_fps(60);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    let overlay = hud_debug_overlay(HudDebugOverlayInputs {
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
    .expect("reduced-debug still projects allowed default entries");

    assert!(overlay
        .left_lines
        .contains(&format!("Minecraft {MC_VERSION} ({MC_VERSION}/bbb-client)")));
    assert!(overlay
        .left_lines
        .contains(&"60 fps T: inf vsync".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"Section-relative: 10 00 10".to_string()));
    assert!(!overlay
        .left_lines
        .iter()
        .any(|line| line.starts_with("XYZ: ")));
    assert!(!overlay
        .left_lines
        .iter()
        .any(|line| line.starts_with("Block: ")));
    assert!(!overlay
        .left_lines
        .iter()
        .any(|line| line.starts_with("Facing: ")));
    assert_eq!(overlay.debug_crosshair, None);
}

#[test]
fn hud_debug_overlay_projects_custom_detailed_memory_under_reduced_debug_info() {
    let world =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 384, true);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::DetailedMemory,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(HudDebugTestDefaults::default().inputs(&input, &world, None))
        .expect("detailed memory is allowed under reduced-debug info");

    assert_eq!(overlay.left_lines, Vec::<String>::new());
    assert_eq!(overlay.right_lines.len(), 2);
    assert!(overlay.right_lines[0].starts_with("Memory (heap): "));
    assert!(overlay.right_lines[1].starts_with("Memory (non-heap): "));
}

#[test]
fn hud_debug_fps_sampler_reports_completed_one_second_windows() {
    let start = Instant::now();
    let mut sampler = HudDebugFpsSampler::default();

    sampler.record_frame(start);
    assert!(sampler.frame_time_nanos().is_empty());
    sampler.record_frame(start + Duration::from_millis(500));
    assert_eq!(sampler.fps(), 0);
    assert_eq!(sampler.frame_time_nanos(), vec![500_000_000]);

    sampler.record_frame(start + Duration::from_secs(1));
    assert_eq!(sampler.fps(), 3);
    assert_eq!(sampler.frame_time_nanos(), vec![500_000_000, 500_000_000]);

    sampler.record_frame(start + Duration::from_millis(1200));
    assert_eq!(sampler.fps(), 3);

    sampler.record_frame(start + Duration::from_secs(2));
    assert_eq!(sampler.fps(), 2);
}

#[test]
fn hud_debug_fps_sampler_keeps_vanilla_sample_capacity() {
    let start = Instant::now();
    let mut sampler = HudDebugFpsSampler::default();
    for frame in 0..=HUD_DEBUG_FRAME_TIME_SAMPLE_CAPACITY + 2 {
        sampler.record_frame(start + Duration::from_millis(frame as u64));
    }

    let samples = sampler.frame_time_nanos();
    assert_eq!(samples.len(), HUD_DEBUG_FRAME_TIME_SAMPLE_CAPACITY);
    assert_eq!(samples[0], 1_000_000);
    assert_eq!(samples[HUD_DEBUG_FRAME_TIME_SAMPLE_CAPACITY - 1], 1_000_000);
}

#[test]
fn hud_debug_network_sampler_logs_bandwidth_and_debug_ping_ticks() {
    let start = Instant::now();
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Digit3),
        ElementState::Pressed,
        None,
        None
    ));
    let net_counters = NetCounters {
        connected: true,
        state: Some("Play".to_string()),
        ..NetCounters::default()
    };
    let (tx, mut rx) = mpsc::channel(4);
    let net_commands = Some(tx);
    let mut sampler = HudDebugNetworkSampler::default();

    sampler.record_received_packet(12);
    sampler.record_received_packet(8);
    sampler.advance_tick(&input, &net_counters, &net_commands, start, 1_000);
    assert_eq!(sampler.bandwidth_bytes_per_tick(), vec![20]);
    assert_eq!(rx.try_recv().unwrap(), NetCommand::PingRequest(1_000));

    sampler.record_pong_response(940, 1_000);
    assert_eq!(sampler.ping_millis(), vec![60]);

    sampler.record_received_packet(5);
    sampler.advance_tick(
        &input,
        &net_counters,
        &net_commands,
        start + Duration::from_millis(49),
        1_049,
    );
    assert_eq!(sampler.bandwidth_bytes_per_tick(), vec![20]);
    assert!(rx.try_recv().is_err());

    sampler.advance_tick(
        &input,
        &net_counters,
        &net_commands,
        start + Duration::from_millis(50),
        1_050,
    );
    assert_eq!(sampler.bandwidth_bytes_per_tick(), vec![20, 5]);
    assert_eq!(rx.try_recv().unwrap(), NetCommand::PingRequest(1_050));
}

#[test]
fn hud_debug_tps_sampler_records_tick_time_samples_and_syncs_subscription() {
    let mut world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Digit2),
        ElementState::Pressed,
        None,
        None
    ));
    let net_counters = NetCounters {
        connected: true,
        state: Some("Play".to_string()),
        ..NetCounters::default()
    };
    let (tx, mut rx) = mpsc::channel(4);
    let net_commands = Some(tx);
    let mut sampler = HudDebugTpsSampler::default();

    sampler.sync_subscriptions_and_tick(&input, &mut world, &net_counters, &net_commands);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::DebugSubscriptionRequest {
            subscription_ids: vec![0]
        }
    );
    assert_eq!(
        world
            .requested_debug_subscriptions()
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![0]
    );
    sampler.sync_subscriptions_and_tick(&input, &mut world, &net_counters, &net_commands);
    assert!(rx.try_recv().is_err());

    sampler.record_net_event(&NetEvent::Play(
        bbb_protocol::packets::PlayClientbound::DebugSample(bbb_protocol::packets::DebugSample {
            sample: vec![70_000_000, 30_000_000, 10_000_000, 5_000_000],
            sample_type: bbb_protocol::packets::RemoteDebugSampleType::TickTime,
        }),
    ));
    assert_eq!(
        sampler.samples(),
        vec![HudDebugTpsSample {
            full_tick_nanos: 70_000_000,
            tick_server_method_nanos: 30_000_000,
            scheduled_tasks_nanos: 10_000_000,
            idle_nanos: 5_000_000,
        }]
    );

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Digit3),
        ElementState::Pressed,
        None,
        None
    ));
    sampler.sync_subscriptions_and_tick(&input, &mut world, &net_counters, &net_commands);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::DebugSubscriptionRequest {
            subscription_ids: Vec::new()
        }
    );
    assert!(world.requested_debug_subscriptions().is_empty());
}

#[test]
fn remote_debug_renderer_requests_share_one_world_owned_subscription_set() {
    let mut world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    input.set_remote_debug_renderers(
        [
            crate::remote_debug::RemoteDebugRenderer::Bees,
            crate::remote_debug::RemoteDebugRenderer::GameEventListeners,
        ]
        .into_iter()
        .collect(),
    );
    let mut net_counters = NetCounters {
        connected: true,
        state: Some("Play".to_string()),
        ..NetCounters::default()
    };
    let (tx, mut rx) = mpsc::channel(4);
    let net_commands = Some(tx);
    let mut sampler = HudDebugTpsSampler::default();

    sampler.sync_subscriptions_and_tick(&input, &mut world, &net_counters, &net_commands);
    let expected = vec![1, 4, 7, 13, 15];
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::DebugSubscriptionRequest {
            subscription_ids: expected.clone(),
        }
    );
    assert_eq!(
        world
            .requested_debug_subscriptions()
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        expected
    );

    net_counters.connected = false;
    sampler.sync_subscriptions_and_tick(&input, &mut world, &net_counters, &net_commands);
    assert!(world.requested_debug_subscriptions().is_empty());
    assert!(world.client_debug_game().values_by_subscription.is_empty());
}

#[test]
fn remote_debug_subscription_retries_after_backpressure_and_listener_clear() {
    let mut world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    input.set_remote_debug_renderers(
        [crate::remote_debug::RemoteDebugRenderer::Bees]
            .into_iter()
            .collect(),
    );
    let net_counters = NetCounters {
        connected: true,
        state: Some("Play".to_string()),
        ..NetCounters::default()
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetCommand::DebugSubscriptionRequest {
        subscription_ids: Vec::new(),
    })
    .unwrap();
    let net_commands = Some(tx);
    let mut sampler = HudDebugTpsSampler::default();

    sampler.sync_subscriptions_and_tick(&input, &mut world, &net_counters, &net_commands);
    assert!(world.requested_debug_subscriptions().is_empty());
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::DebugSubscriptionRequest { subscription_ids } if subscription_ids.is_empty()
    ));

    sampler.sync_subscriptions_and_tick(&input, &mut world, &net_counters, &net_commands);
    let expected = vec![1, 4, 7];
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::DebugSubscriptionRequest {
            subscription_ids: expected.clone(),
        }
    );
    assert_eq!(
        world
            .requested_debug_subscriptions()
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        expected
    );

    world.clear_debug_subscriber();
    sampler.sync_subscriptions_and_tick(&input, &mut world, &net_counters, &net_commands);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::DebugSubscriptionRequest {
            subscription_ids: vec![1, 4, 7],
        }
    );
}

#[test]
fn remote_debug_subscription_purges_at_current_pre_increment_game_time() {
    let mut world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    input.set_remote_debug_renderers(
        [crate::remote_debug::RemoteDebugRenderer::NeighborUpdates]
            .into_iter()
            .collect(),
    );
    let net_counters = NetCounters {
        connected: true,
        state: Some("Play".to_string()),
        ..NetCounters::default()
    };
    let (tx, mut rx) = mpsc::channel(2);
    let net_commands = Some(tx);
    let mut sampler = HudDebugTpsSampler::default();
    sampler.sync_subscriptions_and_tick(&input, &mut world, &net_counters, &net_commands);
    let _ = rx.try_recv().unwrap();
    world.apply_debug_event(bbb_protocol::packets::DebugEvent {
        event: bbb_protocol::packets::DebugSubscriptionEvent {
            subscription_id: 14,
            value: bbb_protocol::packets::DebugSubscriptionValue::NeighborUpdate(
                ProtocolBlockPos { x: 1, y: 2, z: 3 },
            ),
        },
    });
    assert_eq!(world.debug_subscription_values(14).unwrap().events.len(), 1);

    world.apply_world_time(PlayTime {
        game_time: 200,
        clock_updates: Vec::new(),
    });
    sampler.sync_subscriptions_and_tick(&input, &mut world, &net_counters, &net_commands);
    assert!(world
        .debug_subscription_values(14)
        .unwrap()
        .events
        .is_empty());
    assert!(rx.try_recv().is_err());
}

#[test]
fn hud_debug_overlay_projects_network_charts_for_connected_f3_3() {
    let world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Digit3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    let net_counters = NetCounters {
        connected: true,
        state: Some("Play".to_string()),
        ..NetCounters::default()
    };
    let mut network_sampler = HudDebugNetworkSampler::default();
    network_sampler.record_pong_response(900, 1_000);
    network_sampler.record_received_packet(42);
    network_sampler.advance_tick(&input, &net_counters, &None, Instant::now(), 1_000);

    let overlay = hud_debug_overlay(HudDebugOverlayInputs {
        network_sampler: &network_sampler,
        net_counters: &net_counters,
        ..HudDebugTestDefaults::default().inputs(&input, &world, None)
    })
    .expect("F3+3 should force the debug overlay visible");

    assert!(overlay
        .left_lines
        .contains(&"[F3+3] Network visible; [F3+4] Lightmap hidden".to_string()));
    assert_eq!(overlay.fps_chart, None);
    assert_eq!(
        overlay.network_charts,
        Some(HudDebugNetworkCharts {
            ping_millis: vec![100],
            bandwidth_bytes_per_tick: vec![42],
            show_bandwidth: true,
        })
    );
}

#[test]
fn hud_debug_overlay_projects_tps_chart_for_f3_2_remote_samples() {
    let mut world = WorldStore::new();
    world.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 10.0,
        frozen: false,
    });
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Digit2),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    let fps_sampler = hud_debug_fps_sampler_with_frame_times(&[16_000_000]);
    let mut tps_sampler = HudDebugTpsSampler::default();
    tps_sampler.record_debug_sample(&[50_000_000, 20_000_000, 5_000_000, 10_000_000]);

    let overlay = hud_debug_overlay(HudDebugOverlayInputs {
        fps_sampler: &fps_sampler,
        tps_sampler: &tps_sampler,
        ..HudDebugTestDefaults::default().inputs(&input, &world, None)
    })
    .expect("F3+2 should force the debug overlay visible");

    assert_eq!(
        overlay.fps_chart,
        Some(HudDebugFrameTimeChart {
            frame_time_nanos: vec![16_000_000],
            configured_framerate_limit: None,
        })
    );
    assert_eq!(
        overlay.tps_chart,
        Some(HudDebugTpsChart {
            samples: vec![HudDebugTpsSample {
                full_tick_nanos: 50_000_000,
                tick_server_method_nanos: 20_000_000,
                scheduled_tasks_nanos: 5_000_000,
                idle_nanos: 10_000_000,
            }],
            milliseconds_per_tick: 100.0,
        })
    );
    assert_eq!(overlay.network_charts, None);
}

#[test]
fn hud_debug_fps_line_matches_vanilla_shape_without_configured_cap() {
    assert_eq!(
        hud_debug_fps_line(144, VANILLA_UNLIMITED_FRAMERATE_LIMIT, true),
        "144 fps T: inf vsync"
    );
    assert_eq!(hud_debug_fps_line(60, 120, false), "60 fps T: 120");
    assert_eq!(hud_debug_configured_framerate_limit(120), Some(120));
    assert_eq!(
        hud_debug_configured_framerate_limit(VANILLA_UNLIMITED_FRAMERATE_LIMIT),
        None
    );
}

#[test]
fn hud_debug_overlay_projects_configured_framerate_limit_into_fps_chart() {
    let world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Digit2),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    let fps_sampler = hud_debug_fps_sampler_with_reported_fps(60);

    let overlay = hud_debug_overlay(HudDebugOverlayInputs {
        fps_sampler: &fps_sampler,
        client_framerate_limit: 120,
        client_vsync: false,
        ..HudDebugTestDefaults::default().inputs(&input, &world, None)
    })
    .expect("F3+2 should force the debug overlay visible");

    assert_eq!(overlay.left_lines[1], "60 fps T: 120");
    assert_eq!(
        overlay.fps_chart,
        Some(HudDebugFrameTimeChart {
            frame_time_nanos: fps_sampler.frame_time_nanos(),
            configured_framerate_limit: Some(120),
        })
    );
}

#[test]
fn hud_debug_overlay_formats_memory_lines_like_vanilla_debug_entries() {
    assert_eq!(
        hud_debug_memory_lines(HudDebugMemorySnapshot {
            used_mib: 32,
            max_mib: 256,
            allocated_mib: 128,
            allocation_rate_mib_per_s: 7,
        }),
        vec![
            "Mem: 12% 032/256MiB".to_string(),
            "Allocation rate: 007MiB/s".to_string(),
            "Allocated: 50% 128MiB".to_string(),
        ]
    );
}

#[test]
fn hud_debug_overlay_formats_detailed_memory_lines_like_vanilla_debug_entry() {
    assert_eq!(
        hud_debug_detailed_memory_lines(HudDebugDetailedMemorySnapshot {
            heap: HudDebugMemoryUsageSnapshot {
                init_mib: 1,
                used_mib: 32,
                committed_mib: 128,
                max_mib: 256,
            },
            non_heap: HudDebugMemoryUsageSnapshot {
                init_mib: 0,
                used_mib: 8,
                committed_mib: 16,
                max_mib: 0,
            },
        }),
        vec![
            "Memory (heap): i=001MiB u=032MiB c=128MiB m=256MiB".to_string(),
            "Memory (non-heap): i=000MiB u=008MiB c=016MiB m=000MiB".to_string(),
        ]
    );
}

#[test]
fn hud_debug_overlay_projects_tps_server_brand_and_freeze_status() {
    assert_eq!(hud_debug_tps_line(&WorldStore::new()), None);

    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    world.apply_custom_payload(bbb_protocol::packets::CustomPayload {
        id: "minecraft:brand".to_string(),
        payload: bbb_protocol::packets::CustomPayloadBody::Brand {
            brand: "vanilla".to_string(),
        },
    });
    assert_eq!(
        hud_debug_tps_line(&world),
        Some("\"vanilla\" server, 0 tx, 0 rx".to_string())
    );

    world.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 20.0,
        frozen: true,
    });
    assert_eq!(
        hud_debug_tps_line(&world),
        Some("\"vanilla\" server (frozen), 0 tx, 0 rx".to_string())
    );

    world.apply_ticking_step(bbb_protocol::packets::TickingStep { tick_steps: 2 });
    let expected = "\"vanilla\" server (frozen - stepping), 0 tx, 0 rx".to_string();
    assert_eq!(hud_debug_tps_line(&world), Some(expected.clone()));

    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None
    ));
    let overlay = hud_debug_overlay(HudDebugTestDefaults::default().inputs(&input, &world, None))
        .expect("F3 should show the debug overlay");

    assert!(overlay.left_lines.contains(&expected));
}

#[test]
fn hud_debug_overlay_help_lines_reflect_chart_toggle_state() {
    let world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    let fps_sampler = hud_debug_fps_sampler_with_frame_times(&[16_000_000, 33_000_000]);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Digit2),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));

    let overlay = hud_debug_overlay(HudDebugOverlayInputs {
        fps_sampler: &fps_sampler,
        ..HudDebugTestDefaults::default().inputs(&input, &world, None)
    })
    .expect("chart toggle should force the debug overlay visible");

    assert!(overlay
        .left_lines
        .contains(&"Debug charts: [F3+1] Profiler hidden; [F3+2] FPS visible;".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"[F3+3] Network hidden; [F3+4] Lightmap hidden".to_string()));
    assert_eq!(
        overlay.fps_chart,
        Some(HudDebugFrameTimeChart {
            frame_time_nanos: fps_sampler.frame_time_nanos(),
            configured_framerate_limit: None,
        })
    );
    assert!(!overlay.show_lightmap_preview);
}

#[test]
fn hud_debug_overlay_projects_profiler_toggle_without_fake_chart_data() {
    let world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Digit1),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));

    let overlay = hud_debug_overlay(HudDebugTestDefaults::default().inputs(&input, &world, None))
        .expect("F3+1 should force the debug overlay visible");

    assert!(overlay
        .left_lines
        .contains(&"Debug charts: [F3+1] Profiler visible; [F3+2] FPS hidden;".to_string()));
    assert_eq!(overlay.profiler_chart, None);
    assert_eq!(overlay.fps_chart, None);
    assert_eq!(overlay.network_charts, None);
}

#[test]
fn hud_debug_overlay_merges_native_profiler_chart() {
    let chart = HudDebugProfilerChart {
        current_node_name: "root".to_string(),
        current_global_percentage: 100.0,
        slices: vec![bbb_client_render::HudDebugProfilerSlice {
            name: "render".to_string(),
            percentage: 75.0,
            global_percentage: 75.0,
        }],
    };

    let world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    let defaults = HudDebugTestDefaults::default();
    let chart_only = hud_debug_overlay(HudDebugOverlayInputs {
        profiler_chart: Some(chart.clone()),
        ..defaults.inputs(&input, &world, None)
    })
    .expect("chart overlay");
    assert_eq!(chart_only.profiler_chart, Some(chart.clone()));
    assert!(chart_only.left_lines.is_empty());

    for (key, state) in [
        (KeyCode::F3, ElementState::Pressed),
        (KeyCode::F3, ElementState::Released),
    ] {
        assert!(input.handle_debug_overlay_key(PhysicalKey::Code(key), state, None, None));
    }
    let merged = hud_debug_overlay(HudDebugOverlayInputs {
        profiler_chart: Some(chart.clone()),
        ..defaults.inputs(&input, &world, None)
    })
    .expect("merged overlay");
    assert!(!merged.left_lines.is_empty());
    assert_eq!(merged.profiler_chart, Some(chart));
}

#[test]
fn hud_debug_overlay_projects_game_mode_switcher_state() {
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let player_id = world
        .local_player_id()
        .expect("test world has a local player");
    assert!(world.apply_entity_event(ProtocolEntityEvent {
        entity_id: player_id,
        event_id: 26,
    }));
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));

    let overlay = hud_debug_overlay(HudDebugTestDefaults::default().inputs(&input, &world, None))
        .expect("game mode switcher should project even while the F3 overlay is hidden");
    let switcher = overlay
        .game_mode_switcher
        .expect("game mode switcher render state should be present");

    assert_eq!(switcher.selected, GameType::Creative);
    assert_eq!(switcher.title, "Creative Mode");
    // `TranslatableContents.decompose` takes
    // `Language.getOrDefault(key)`, which answers the key
    // itself when no language table is loaded, and the template then names no
    // argument to place.
    assert_eq!(
        switcher.help_runs,
        vec![HudStyledTextRun::plain("debug.gamemodes.select_next")]
    );
    assert_eq!((switcher.background_x, switcher.background_y), (98, 62));
    assert_eq!(
        switcher.slots.iter().map(|slot| slot.x).collect::<Vec<_>>(),
        vec![101, 132, 163, 194]
    );
    assert!(switcher.slots[0].selected);
    assert!(switcher.slots[1..].iter().all(|slot| !slot.selected));
    assert!(overlay.left_lines.is_empty());
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_game_mode_switcher_hint_colours_only_the_bound_key_name() {
    let root = unique_runtime_temp_dir("game-mode-switcher-hint");
    write_runtime_game_mode_switcher_item_assets(&root);
    write_runtime_json(
        &runtime_assets_dir(&root).join("lang").join("en_us.json"),
        r#"{
            "debug.gamemodes.select_next": "%s Next",
            "key.keyboard.f4": "F4"
        }"#,
    );
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let options =
        crate::options::ClientOptions::defaults(crate::options::ClientOptionHost::default());
    crate::options::key_bindings::install_bound_key_display_keys(&options);
    let terrain_textures = TerrainTextureState::default();
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let player_id = world
        .local_player_id()
        .expect("test world has a local player");
    assert!(world.apply_entity_event(ProtocolEntityEvent {
        entity_id: player_id,
        event_id: 26,
    }));
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));

    let overlay = hud_debug_overlay(HudDebugOverlayInputs {
        item_runtime: Some(&item_runtime),
        terrain_textures: &terrain_textures,
        ..HudDebugTestDefaults::default().inputs(&input, &world, None)
    })
    .expect("game mode switcher should project with a language table");
    let switcher = overlay.game_mode_switcher.expect("switcher render state");

    // The key message is the display name of whatever key the switcher mapping
    // is bound to, not a baked one: the binding table answers the key id and the
    // language table answers its name.
    let bound =
        crate::options::key_bindings::bound_key_display_key(DEBUG_GAME_MODE_SWITCHER_KEY_MAPPING)
            .expect("the switcher binding resolves to a named key");
    assert_eq!(bound, "key.keyboard.f4");
    assert_eq!(switcher.help_runs.len(), 2);
    assert_eq!(
        switcher.help_runs[0].text,
        item_runtime.localized_text(bound)
    );
    assert_eq!(
        switcher.help_runs[0].color,
        bbb_protocol::chat_formattings::ChatFormatting::Aqua.color()
    );
    // `withStyle` sits on the argument, so
    // the sentence around it keeps the draw call's own colour.
    assert_eq!(switcher.help_runs[1].text, " Next");
    assert_eq!(switcher.help_runs[1].color, None);
    assert_eq!(
        switcher.help_runs[1].style,
        bbb_client_render::HudTextStyle::default()
    );

    // Same binding id, a language table that names the key differently: the run
    // follows the table rather than a literal.
    let renamed = unique_runtime_temp_dir("game-mode-switcher-hint-renamed");
    write_runtime_game_mode_switcher_item_assets(&renamed);
    write_runtime_json(
        &runtime_assets_dir(&renamed).join("lang").join("en_us.json"),
        r#"{
            "debug.gamemodes.select_next": "Press %s",
            "key.keyboard.f4": "Fn Four"
        }"#,
    );
    let renamed_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&renamed).unwrap())
            .unwrap();
    let renamed_runs = hud_game_mode_switcher_help_runs(Some(&renamed_runtime));
    assert_eq!(renamed_runs.len(), 2);
    assert_eq!(renamed_runs[0].text, "Press ");
    assert_eq!(renamed_runs[0].color, None);
    assert_eq!(renamed_runs[1].text, "Fn Four");
    assert_eq!(
        renamed_runs[1].color,
        bbb_protocol::chat_formattings::ChatFormatting::Aqua.color()
    );

    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(renamed).unwrap();
}

#[test]
fn hud_debug_overlay_projects_game_mode_switcher_item_icons() {
    let root = unique_runtime_temp_dir("game-mode-switcher-icons");
    write_runtime_game_mode_switcher_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let terrain_textures = TerrainTextureState::default();
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let player_id = world
        .local_player_id()
        .expect("test world has a local player");
    assert!(world.apply_entity_event(ProtocolEntityEvent {
        entity_id: player_id,
        event_id: 26,
    }));
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));

    let overlay = hud_debug_overlay(HudDebugOverlayInputs {
        item_runtime: Some(&item_runtime),
        terrain_textures: &terrain_textures,
        ..HudDebugTestDefaults::default().inputs(&input, &world, None)
    })
    .expect("game mode switcher should project with runtime icons");
    let switcher = overlay.game_mode_switcher.expect("switcher render state");

    assert_eq!(
        DEBUG_GAME_MODE_SWITCHER_MODES
            .iter()
            .copied()
            .map(hud_game_mode_switcher_icon_resource_id)
            .collect::<Vec<_>>(),
        vec![
            "minecraft:grass_block",
            "minecraft:iron_sword",
            "minecraft:map",
            "minecraft:ender_eye",
        ]
    );
    for (slot, mode) in switcher.slots.iter().zip(DEBUG_GAME_MODE_SWITCHER_MODES) {
        let resource_id = hud_game_mode_switcher_icon_resource_id(mode);
        let item_id = item_runtime
            .item_protocol_id(resource_id)
            .expect("test item is registered");
        let stack = ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: Default::default(),
        };
        let expected_uv = item_runtime
            .resolve_item_model(
                &stack,
                &bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                ),
            )
            .and_then(|model| model.flat_icon())
            .unwrap()
            .layers[0]
            .uv;
        let icon = slot.icon.as_ref().expect("slot has item icon");

        assert_eq!(slot.mode, mode);
        assert_eq!(
            icon.layers[0].uv,
            HudUvRect {
                min: expected_uv.min,
                max: expected_uv.max,
            }
        );
        assert_eq!(icon.count_label, None);
        assert_eq!(icon.durability_bar, None);
        assert_eq!(icon.cooldown_progress, None);
        assert!(slot.block_model.is_none());
    }

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_debug_overlay_projects_lightmap_preview_toggle_state() {
    let world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Digit4),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));

    let overlay = hud_debug_overlay(HudDebugTestDefaults::default().inputs(&input, &world, None))
        .expect("lightmap toggle should force the debug overlay visible");

    assert!(overlay.show_lightmap_preview);
    assert!(overlay
        .left_lines
        .contains(&"[F3+3] Network hidden; [F3+4] Lightmap visible".to_string()));
}

#[test]
fn f3_a_requests_terrain_reload_without_toggling_overlay_on_release() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let mut terrain_upload = TerrainUploadState::default();
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
        Some(&mut world),
        Some(&mut terrain_upload)
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None
    ));

    assert!(terrain_upload.reload_all_chunks_requested());
    assert_eq!(world.client_chat().messages.len(), 1);
    assert_eq!(
        world.client_chat().messages[0].content,
        "[Debug]: Reloading all chunks"
    );
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_p_toggles_pause_on_lost_focus_without_world() {
    let mut input = ClientInputState::new(true);
    assert!(input.debug_pause_on_lost_focus());

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyP),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));

    assert!(!input.debug_pause_on_lost_focus());
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_p_emits_pause_on_lost_focus_debug_feedback_with_world() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyP),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyP),
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

    let feedback: Vec<_> = world
        .client_chat()
        .messages
        .iter()
        .map(|message| message.content.as_str())
        .collect();
    assert_eq!(
        feedback,
        vec![
            "[Debug]: Pause on lost focus: disabled",
            "[Debug]: Pause on lost focus: enabled",
        ]
    );
    assert!(input.debug_pause_on_lost_focus());
    assert!(!input.debug_overlay_visible());
}

#[test]
fn hud_debug_overlay_help_lines_reflect_status_toggle_state() {
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    for code in [KeyCode::KeyB, KeyCode::KeyG, KeyCode::KeyH, KeyCode::KeyP] {
        assert!(input.handle_debug_overlay_key(
            PhysicalKey::Code(code),
            ElementState::Pressed,
            Some(&mut world),
            None
        ));
    }
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None
    ));
    assert!(!input.debug_overlay_visible());
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None
    ));

    let overlay = hud_debug_overlay(HudDebugTestDefaults::default().inputs(&input, &world, None))
        .expect("plain F3 should make the debug overlay visible");

    assert!(overlay
        .left_lines
        .contains(&"Debug toggles: [F3+B] Hitboxes visible; [F3+G] Chunks visible".to_string()));
    assert!(overlay.left_lines.contains(
        &"Debug options: [F3+H] Tooltips enabled; [F3+P] Focus pause disabled".to_string()
    ));
    assert!(overlay
        .left_lines
        .contains(&"Debug actions: [F3+A] Reload chunks; [F3+C] Copy location".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"Debug actions: [F3+D] Clear chat; [F3+S] Dump textures".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"Debug actions: [F3+T] Reload packs; [F3+V] Version".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"Game mode: [F3+N] Spectator; [F3+F4] Switcher".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"To edit: press [F3+F6]".to_string()));
}

#[test]
fn debug_entity_scene_outline_follows_f3_b_hitbox_toggle() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.apply_add_entity(test_add_entity(77, VANILLA_ENTITY_TYPE_PLAYER_ID));
    let mut input = ClientInputState::new(true);

    assert_eq!(debug_entity_scene_outline(&input, &world, 1.0), None);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyB),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));

    let outline = debug_entity_scene_outline(&input, &world, 1.0)
        .expect("F3+B should enable entity AABB debug outlines");
    assert!(outline.boxes.is_empty());
    assert_eq!(outline.colored_boxes.len(), 2);
    assert_eq!(outline.lines.len(), 1);
    assert_eq!(outline.points.len(), 1);
    assert!(outline.text_labels.is_empty());
}

#[test]
fn debug_entity_scene_outline_includes_missing_server_label_when_startup_flag_is_enabled() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.apply_add_entity(test_add_entity(77, VANILLA_ENTITY_TYPE_PLAYER_ID));
    let mut input = ClientInputState::new(true);
    input.set_debug_show_local_server_entity_hit_boxes(true);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyB),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));

    let outline = debug_entity_scene_outline(&input, &world, 1.0)
        .expect("F3+B should enable entity AABB debug outlines");
    assert_eq!(outline.text_labels.len(), 1);
    assert_eq!(outline.text_labels[0].text, "Missing Server Entity");
    assert_eq!(
        debug_selection_text_billboards(Some(&outline)),
        [DebugTextBillboard::centered(
            outline.text_labels[0].position,
            "Missing Server Entity",
            [255, 0, 0, 255],
        )]
    );
}

#[test]
fn debug_chunk_border_outline_follows_f3_g_chunk_border_toggle() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    let mut input = ClientInputState::new(true);
    let camera_pose = Some(CameraPose {
        position: [31.8, 42.0, -0.2],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    assert_eq!(
        debug_chunk_border_outline(&input, &world, camera_pose),
        None
    );

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyG),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));

    let outline = debug_chunk_border_outline(&input, &world, camera_pose)
        .expect("F3+G should enable chunk-border debug outlines");
    assert!(outline.boxes.is_empty());
    assert_eq!(outline.colored_boxes.len(), 1);
    assert_eq!(outline.colored_boxes[0].min, [16.0, 32.0, -16.0]);
    assert_eq!(outline.colored_boxes[0].max, [32.0, 48.0, 0.0]);
    assert_eq!(outline.colored_boxes[0].color, CHUNK_BORDER_MAJOR_COLOR);
    assert_eq!(
        outline.colored_boxes[0].line_width,
        CHUNK_BORDER_THIN_LINE_WIDTH
    );
    assert!(outline.colored_boxes[0].always_on_top);
    assert_eq!(outline.lines.len(), 920);
    assert_eq!(outline.lines[0].from, [0.0, -64.0, -32.0]);
    assert_eq!(outline.lines[0].to, [0.0, 320.0, -32.0]);
    assert_eq!(outline.lines[0].color, CHUNK_BORDER_NEIGHBOR_COLOR);
    assert_eq!(outline.lines[0].width, CHUNK_BORDER_THICK_LINE_WIDTH);
    assert_eq!(outline.lines[16].from, [18.0, -64.0, -16.0]);
    assert_eq!(outline.lines[16].color, CHUNK_BORDER_YELLOW_COLOR);
    assert_eq!(outline.lines[16].width, CHUNK_BORDER_THIN_LINE_WIDTH);
    assert_eq!(outline.lines[18].from, [20.0, -64.0, -16.0]);
    assert_eq!(outline.lines[18].color, CHUNK_BORDER_CELL_COLOR);
    assert_eq!(outline.lines[44].from, [16.0, -64.0, -16.0]);
    assert_eq!(outline.lines[44].to, [16.0, -64.0, 0.0]);
    assert_eq!(outline.lines[44].color, CHUNK_BORDER_CELL_COLOR);
    assert_eq!(outline.lines[816].from, [16.0, -64.0, -16.0]);
    assert_eq!(outline.lines[816].to, [16.0, 320.0, -16.0]);
    assert_eq!(outline.lines[816].color, CHUNK_BORDER_MAJOR_COLOR);
    assert_eq!(outline.lines[816].width, CHUNK_BORDER_THICK_LINE_WIDTH);
    assert_eq!(outline.lines[920 - 1].from, [32.0, 320.0, -16.0]);
    assert_eq!(outline.lines[920 - 1].to, [16.0, 320.0, -16.0]);
    assert_eq!(outline.lines[920 - 1].color, CHUNK_BORDER_MAJOR_COLOR);
    assert!(outline.points.is_empty());
    assert_eq!(debug_chunk_border_outline(&input, &world, None), None);
}

#[test]
fn debug_heightmap_boxes_respect_reduced_debug_and_camera_gates() {
    let mut world =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 16, true);
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeHeightmap,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    assert!(debug_heightmap_boxes(
        &input,
        &world,
        Some(CameraPose {
            position: [0.0, 42.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        })
    )
    .is_empty());
    assert!(debug_heightmap_boxes(&input, &world, None).is_empty());
}

#[test]
fn debug_water_surfaces_follow_player_scan_order_and_vanilla_fluid_height() {
    assert_eq!(DEBUG_WATER_SCAN_RADIUS, 10);
    assert_eq!(DEBUG_WATER_BOX_INSET, 0.01);
    assert_eq!(DEBUG_WATER_BOX_COLOR, [0, 255, 0, 38]);

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 0, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    let flowing_water_level_3 = world
        .registries()
        .block_state_id_by_name_and_properties(
            "minecraft:water",
            &BTreeMap::from([("level".to_string(), "3".to_string())]),
        )
        .expect("vanilla level-3 flowing water state");
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 1, y: 0, z: 0 },
        flowing_water_level_3,
    );
    let lava = world
        .registries()
        .block_state_id_by_name_and_properties(
            "minecraft:lava",
            &BTreeMap::from([("level".to_string(), "0".to_string())]),
        )
        .expect("vanilla source lava state");
    set_lightmap_test_block(&mut world, BlockPos { x: 2, y: 0, z: 0 }, lava);

    let mut input = ClientInputState::new(true);
    assert!(debug_water_samples(&input, &world).is_empty());
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeWaterLevels,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let samples = debug_water_samples(&input, &world);
    let boxes = debug_water_boxes(&samples);
    assert_eq!(boxes.len(), 3, "lava is not in FluidTags.WATER");
    assert_eq!(
        boxes[0],
        DebugFilledBox {
            min: [0.01, 0.01, 0.01],
            max: [0.99, 1.0, 0.99],
            color: DEBUG_WATER_BOX_COLOR,
        },
        "same water above makes the lower source column full height"
    );
    assert_eq!(boxes[1].min, [1.01, 0.01, 0.01]);
    assert!((boxes[1].max[1] - 5.0 / 9.0).abs() < 1e-6);
    assert_eq!(boxes[2].min, [0.01, 1.01, 0.01]);
    assert!((boxes[2].max[1] - (1.0 + 8.0 / 9.0)).abs() < 1e-6);

    let labels = debug_water_amount_billboards(&samples);
    assert_eq!(labels.len(), 3);
    assert_eq!(
        labels[0],
        DebugTextBillboard::centered([0.5, 1.0, 0.5], "8", [0, 0, 0, 255])
    );
    assert_eq!(labels[1].text, "5");
    assert_eq!(labels[1].position[0], 1.5);
    assert!((labels[1].position[1] - 5.0 / 9.0).abs() < 1e-6);
    assert_eq!(labels[2].text, "8");
    assert!((labels[2].position[1] - (1.0 + 8.0 / 9.0)).abs() < 1e-6);
    assert!(labels.iter().all(|label| {
        label.scale == DebugTextBillboard::DEFAULT_SCALE && label.adjust_left.is_none()
    }));
}

#[test]
fn debug_water_boxes_respect_reduced_debug_and_player_gates() {
    let mut world =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 16, true);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 0, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeWaterLevels,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    assert!(debug_water_samples(&input, &world).is_empty());

    let mut no_player = world_with_dimension(0, "minecraft:overworld");
    no_player.insert_decoded_chunk(empty_lightmap_test_chunk(no_player.dimension()));
    assert!(debug_water_samples(&input, &no_player).is_empty());
}

#[test]
fn debug_light_billboards_match_vanilla_scan_filters_positions_and_colors() {
    assert_eq!(DEBUG_LIGHT_SCAN_RADIUS, 10);
    assert_eq!(DEBUG_LIGHT_SECTION_LABEL_SCALE, 4.8);

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.25, 0.0, 0.25], 0.0, 0.0));
    let mut chunk = empty_lightmap_test_chunk(world.dimension());
    let mut sky = [0xff; TEST_LIGHT_ARRAY_BYTES];
    let mut block = [0; TEST_LIGHT_ARRAY_BYTES];
    let nibble_index = section_block_index(0, 1, 0);
    set_test_light_nibble(&mut sky, nibble_index, 4);
    set_test_light_nibble(&mut block, nibble_index, 13);
    let light_section_index = usize::try_from(0 - (world.dimension().min_section_y() - 1)).unwrap();
    chunk.light = LightData::default();
    chunk.light.set_sky_packed(light_section_index, sky);
    chunk.light.set_block_packed(light_section_index, block);
    world.insert_decoded_chunk(chunk);
    let camera_pose = camera_pose_from_world(&world);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeBlockLightLevels,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let block_only = debug_light_level_billboards(&input, &world, camera_pose);
    let section_labels = block_only
        .iter()
        .filter(|billboard| billboard.scale == DEBUG_LIGHT_SECTION_LABEL_SCALE)
        .collect::<Vec<_>>();
    assert_eq!(section_labels.len(), 8);
    assert!(section_labels.iter().all(|billboard| {
        billboard.text == "2" && billboard.color == DEBUG_LIGHT_SECTION_LABEL_COLOR
    }));
    assert_eq!(section_labels[0].position, [-8.0, -8.0, -8.0]);
    let block_label = block_only
        .iter()
        .find(|billboard| billboard.text == "13")
        .expect("non-zero block light should be labeled");
    assert_eq!(block_label.position, [0.5, 1.5, 0.5]);
    assert_eq!(block_label.color, [243, 221, 0, 255]);
    assert_eq!(block_label.scale, DebugTextBillboard::DEFAULT_SCALE);
    assert!(!block_only.iter().any(|billboard| billboard.text == "4"));

    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeSkyLightLevels,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let both = debug_light_level_billboards(&input, &world, camera_pose);
    assert_eq!(both.len(), 10);
    let sky_label = both
        .iter()
        .find(|billboard| billboard.text == "4")
        .expect("non-full sky light should be labeled");
    assert_eq!(sky_label.position, [0.5, 1.25, 0.5]);
    assert_eq!(sky_label.color, [0, 68, 255, 255]);
    let sky_index = both
        .iter()
        .position(|billboard| billboard.text == "4")
        .unwrap();
    let block_index = both
        .iter()
        .position(|billboard| billboard.text == "13")
        .unwrap();
    assert_eq!(block_index, sky_index + 1);

    let mut reduced =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 16, true);
    reduced.set_local_player_pose(local_player_pose([0.25, 0.0, 0.25], 0.0, 0.0));
    assert!(
        debug_light_level_billboards(&input, &reduced, camera_pose_from_world(&reduced)).is_empty()
    );
}

#[test]
fn debug_solid_face_rects_match_vanilla_scan_faces_inflate_and_gates() {
    assert_eq!(DEBUG_SOLID_FACE_SCAN_RADIUS, 6);
    assert_eq!(DEBUG_SOLID_FACE_INFLATE, 0.002);
    assert_eq!(DEBUG_SOLID_FACE_COLOR, [255, 0, 0, 128]);

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let stone = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:stone", &BTreeMap::new())
        .expect("vanilla stone state");
    set_lightmap_test_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, stone);
    let camera_pose = camera_pose_from_world(&world);
    let mut input = ClientInputState::new(true);
    assert!(debug_solid_face_rects(&input, &world, camera_pose).is_empty());
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeSolidFaces,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let rects = debug_solid_face_rects(&input, &world, camera_pose);
    assert_eq!(rects.len(), 6);
    assert_eq!(
        rects[0],
        DebugFilledRect {
            corners: [
                [-0.002, -0.002, -0.002],
                [-0.002, -0.002, 1.002],
                [-0.002, 1.002, 1.002],
                [-0.002, 1.002, -0.002],
            ],
            color: DEBUG_SOLID_FACE_COLOR,
        }
    );
    assert_eq!(
        rects[5].corners,
        [
            [-0.002, 1.002, -0.002],
            [-0.002, 1.002, 1.002],
            [1.002, 1.002, 1.002],
            [1.002, 1.002, -0.002],
        ]
    );
    assert!(rects
        .iter()
        .all(|rect| rect.color == DEBUG_SOLID_FACE_COLOR));

    let scaffolding = world
        .registries()
        .block_state_id_by_name_and_properties(
            "minecraft:scaffolding",
            &BTreeMap::from([
                ("bottom".to_owned(), "false".to_owned()),
                ("distance".to_owned(), "0".to_owned()),
                ("waterlogged".to_owned(), "false".to_owned()),
            ]),
        )
        .expect("vanilla stable scaffolding state");
    set_lightmap_test_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, scaffolding);
    let scaffolding_rects = debug_solid_face_rects(&input, &world, camera_pose);
    assert_eq!(scaffolding_rects.len(), 7);
    assert!(scaffolding_rects.iter().all(|rect| {
        rect.color == DEBUG_SOLID_FACE_COLOR && rect.corners.iter().all(|corner| corner[1] > 0.99)
    }));

    let mut reduced =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 16, true);
    reduced.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    reduced.insert_decoded_chunk(empty_lightmap_test_chunk(reduced.dimension()));
    set_lightmap_test_block(&mut reduced, BlockPos { x: 0, y: 0, z: 0 }, stone);
    assert!(debug_solid_face_rects(&input, &reduced, camera_pose_from_world(&reduced)).is_empty());
    assert!(debug_solid_face_rects(&input, &world, None).is_empty());
}

#[test]
fn debug_sky_light_sections_match_vanilla_shapes_colors_and_refresh_cache() {
    assert_eq!(debug_light_sections::RADIUS, 10);
    assert_eq!(
        debug_light_sections::REFRESH_INTERVAL,
        Duration::from_millis(500)
    );
    assert_eq!(
        debug_light_sections::LIGHT_AND_BLOCKS_FACE_COLOR,
        [255, 255, 0, 63]
    );
    assert_eq!(debug_light_sections::LIGHT_ONLY_FACE_COLOR, [63, 31, 0, 31]);

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let stone = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:stone", &BTreeMap::new())
        .expect("vanilla stone state");
    let air = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:air", &BTreeMap::new())
        .expect("vanilla air state");
    set_lightmap_test_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, stone);

    let mut input = ClientInputState::new(true);
    let mut cache = DebugSkyLightSectionCache::default();
    let start = Instant::now();
    assert_eq!(
        cache.gizmos(&input, &world, start),
        (Vec::new(), Vec::new())
    );
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeSkyLightSections,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let (rects, lines) = cache.gizmos(&input, &world, start);
    // One LIGHT_AND_DATA voxel contributes 6 faces/12 edges. Its 26-neighbour
    // LIGHT_ONLY shell makes the light shape a 3x3x3 cube: 54 boundary-cell
    // faces and 12 merged outer edges.
    assert_eq!(rects.len(), 60);
    assert_eq!(lines.len(), 24);
    assert!(rects[..6]
        .iter()
        .all(|rect| rect.color == debug_light_sections::LIGHT_AND_BLOCKS_FACE_COLOR));
    assert!(rects[6..]
        .iter()
        .all(|rect| rect.color == debug_light_sections::LIGHT_ONLY_FACE_COLOR));
    assert!(lines[..12]
        .iter()
        .all(|line| line.color == debug_light_sections::LIGHT_AND_BLOCKS_LINE_COLOR));
    assert!(lines[12..]
        .iter()
        .all(|line| line.color == debug_light_sections::LIGHT_ONLY_LINE_COLOR));
    assert!(lines
        .iter()
        .all(|line| line.width == debug_light_sections::LINE_WIDTH));

    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeSkyLightSections,
        crate::debug_entries::DebugScreenEntryStatus::Never,
    );
    assert_eq!(
        cache.gizmos(&input, &world, start + Duration::from_millis(100)),
        (Vec::new(), Vec::new())
    );
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeSkyLightSections,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let restarted_at = start + Duration::from_millis(100);
    assert_eq!(
        cache.gizmos(&input, &world, restarted_at),
        (rects.clone(), lines.clone())
    );

    set_lightmap_test_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, air);
    assert_eq!(
        cache.gizmos(
            &input,
            &world,
            restarted_at + debug_light_sections::REFRESH_INTERVAL
        ),
        (rects, lines)
    );
    assert_eq!(
        cache.gizmos(
            &input,
            &world,
            restarted_at + debug_light_sections::REFRESH_INTERVAL + Duration::from_nanos(1)
        ),
        (Vec::new(), Vec::new())
    );
}

#[test]
fn debug_collision_lines_project_vanilla_white_stroke_cuboids_and_gates() {
    assert_eq!(DEBUG_COLLISION_QUERY_RADIUS, 6.0);
    assert_eq!(DEBUG_COLLISION_COLOR, [255; 4]);
    assert_eq!(DebugLine::DEFAULT_WIDTH, 2.5);

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let stone = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:stone", &BTreeMap::new())
        .expect("vanilla stone state");
    set_lightmap_test_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, stone);

    let now = Instant::now();
    let mut cache = DebugCollisionCache::default();
    let mut input = ClientInputState::new(true);
    assert!(cache.lines(&input, &world, now).is_empty());
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeCollisionBoxes,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let lines = cache.lines(&input, &world, now);
    assert_eq!(lines.len(), 12);
    assert_eq!(
        lines[0],
        DebugLine::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [255; 4], 2.5,)
    );
    assert!(lines
        .iter()
        .all(|line| line.color == [255; 4] && line.width == 2.5));

    let mut reduced =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 16, true);
    reduced.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    reduced.insert_decoded_chunk(empty_lightmap_test_chunk(reduced.dimension()));
    set_lightmap_test_block(&mut reduced, BlockPos { x: 0, y: 0, z: 0 }, stone);
    assert!(cache.lines(&input, &reduced, now).is_empty());
}

#[test]
fn debug_collision_cache_matches_vanilla_strict_refresh_interval_and_resets() {
    assert_eq!(DEBUG_COLLISION_REFRESH_INTERVAL, Duration::from_millis(100));

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let stone = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:stone", &BTreeMap::new())
        .expect("vanilla stone state");
    set_lightmap_test_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, stone);

    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeCollisionBoxes,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let start = Instant::now();
    let mut cache = DebugCollisionCache::default();
    assert_eq!(cache.lines(&input, &world, start).len(), 12);

    set_lightmap_test_block(&mut world, BlockPos { x: 1, y: 0, z: 0 }, stone);
    assert_eq!(
        cache
            .lines(&input, &world, start + DEBUG_COLLISION_REFRESH_INTERVAL)
            .len(),
        12,
        "vanilla refreshes only after more than 100 ms"
    );
    let refreshed_at = start + DEBUG_COLLISION_REFRESH_INTERVAL + Duration::from_nanos(1);
    assert_eq!(cache.lines(&input, &world, refreshed_at).len(), 24);

    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeCollisionBoxes,
        crate::debug_entries::DebugScreenEntryStatus::Never,
    );
    assert!(cache.lines(&input, &world, refreshed_at).is_empty());
    set_lightmap_test_block(&mut world, BlockPos { x: 2, y: 0, z: 0 }, stone);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeCollisionBoxes,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    assert_eq!(
        cache.lines(&input, &world, refreshed_at).len(),
        36,
        "re-enabling creates a fresh renderer cache and refreshes immediately"
    );
}

#[test]
fn debug_supporting_block_lines_project_local_player_and_legacy_highlights() {
    assert_eq!(DEBUG_SUPPORTING_BLOCK_OUTLINE_ALPHA, 102);
    assert_eq!(DEBUG_SUPPORTING_BLOCK_PLAYER_OFFSET, 0.02);
    assert_eq!(DEBUG_SUPPORTING_BLOCK_LEGACY_OFFSET, 0.04);

    let mut world = world_with_dimension(0, "minecraft:overworld");
    let mut pose = local_player_pose([0.5, 1.0, 0.5], 0.0, 0.0);
    pose.on_ground = true;
    world.set_local_player_pose(pose);
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let stone = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:stone", &BTreeMap::new())
        .expect("vanilla stone state");
    set_lightmap_test_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, stone);

    let now = Instant::now();
    let mut cache = DebugSupportingBlockCache::default();
    let mut input = ClientInputState::new(true);
    assert!(cache.lines(&input, &world, now).is_empty());
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeEntitySupportingBlocks,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let lines = cache.lines(&input, &world, now);
    assert_eq!(lines.len(), 24);
    assert_eq!(
        lines[0],
        DebugLine::new(
            [-0.04, -0.04, -0.04],
            [1.04, -0.04, -0.04],
            [255, 0, 0, 102],
            2.5,
        )
    );
    assert_eq!(
        lines[12],
        DebugLine::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [255, 0, 0, 255], 2.5,)
    );

    pose.on_ground = false;
    world.set_local_player_pose(pose);
    assert!(cache.lines(&input, &world, now).is_empty());

    let mut reduced =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 16, true);
    pose.on_ground = true;
    reduced.set_local_player_pose(pose);
    reduced.insert_decoded_chunk(empty_lightmap_test_chunk(reduced.dimension()));
    set_lightmap_test_block(&mut reduced, BlockPos { x: 0, y: 0, z: 0 }, stone);
    assert!(cache.lines(&input, &reduced, now).is_empty());
}

#[test]
fn debug_supporting_block_lines_use_cyan_for_legacy_effect_position() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    let mut pose = local_player_pose([0.5, 1.25, 0.5], 0.0, 0.0);
    pose.on_ground = true;
    world.set_local_player_pose(pose);
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
    set_lightmap_test_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, piston_head);
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
    assert_eq!(cyan_outline[0].from, [-0.08, 0.92, -0.08]);
    assert_eq!(cyan_outline[0].to, [1.08, 0.92, -0.08]);
    assert!(lines
        .iter()
        .all(|line| line.width == DebugLine::DEFAULT_WIDTH));
}

#[test]
fn debug_supporting_block_entity_bias_matches_vanilla_java_hash() {
    assert_eq!((52.0_f64 + 0.132_453_657).to_string(), "52.132453657");
    assert_eq!(java_string_hash_code("52.132453657"), 1_431_313_029);
    assert!((debug_supporting_block_entity_bias(52) - 0.000_58).abs() < f64::EPSILON);

    assert_eq!((41.0_f64 + 0.132_453_657).to_string(), "41.132453657");
    assert_eq!(java_string_hash_code("41.132453657"), -1_195_785_627);
    assert!((debug_supporting_block_entity_bias(41) + 0.012_54).abs() < f64::EPSILON);
}

#[test]
fn hud_info_extension_emits_two_compact_lines_only_when_debug_closed() {
    let mut world = WorldStore::new();
    let input = ClientInputState::new(true);
    let pose = CameraPose {
        position: [10.5, 64.0, -3.25],
        y_rot: 180.0,
        x_rot: 0.0,
        eye_height: 1.62,
    };

    assert!(
        world.apply_container_set_slot(bbb_protocol::packets::ContainerSetSlot {
            container_id: 0,
            state_id: 1,
            slot: 36,
            item: bbb_protocol::packets::ItemStackSummary {
                count: 1,
                item_id: bbb_protocol::item_types::vanilla_item_type_for_resource_id(
                    "minecraft:iron_pickaxe",
                )
                .map(|entry| entry.id),
                component_patch: bbb_protocol::packets::DataComponentPatchSummary {
                    added_type_ids: vec![
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                ],
                    max_damage: Some(100),
                    damage: Some(10),
                    ..Default::default()
                }
                .into(),
            },
        })
    );

    let overlay = hud_debug_overlay(HudDebugOverlayInputs {
        hud_info_enabled: true,
        ..HudDebugTestDefaults::default().inputs(&input, &world, Some(pose))
    })
    .expect("hud_info overlay");
    assert_eq!(overlay.left_lines.len(), 2);
    assert_eq!(
        overlay.left_lines[0],
        "0 FPS | XYZ: 10.5 64.0 -3.2 (N) | unknown"
    );
    assert_eq!(overlay.left_lines[1], "Day 1 06:00 | Mainhand: 90/100");
    assert!(overlay.right_lines.is_empty());

    let disabled =
        hud_debug_overlay(HudDebugTestDefaults::default().inputs(&input, &world, Some(pose)));
    assert!(disabled.is_none());

    let mut f3_input = ClientInputState::new(true);
    assert!(f3_input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None,
    ));
    assert!(f3_input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None,
    ));
    let f3_overlay = hud_debug_overlay(HudDebugOverlayInputs {
        hud_info_enabled: true,
        ..HudDebugTestDefaults::default().inputs(&f3_input, &world, Some(pose))
    })
    .expect("F3 overlay takes precedence over hud_info");
    assert!(!f3_overlay
        .left_lines
        .iter()
        .any(|line| line.starts_with("Day 1 ")));
}
