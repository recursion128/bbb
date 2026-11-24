use super::*;

pub(super) fn end_connection_and_show_disconnect(
    world: &mut WorldStore,
    input: &mut ClientInputState,
    title: crate::input::disconnected_screen::DisconnectedScreenTitle,
    reason: bbb_client_net::DisconnectReason,
) {
    let bug_report_kind = bbb_protocol::packets::ServerLinkKnownType::BugReport.vanilla_name();
    let bug_report_link = world
        .server_links()
        .iter()
        .find(|link| link.known_type.as_deref() == Some(bug_report_kind))
        .map(|link| link.url.clone());
    world.end_connection_session();
    input.open_disconnected_screen(
        title,
        crate::input::disconnected_screen::DisconnectedScreenDetails {
            reason,
            report_directory: None,
            bug_report_link,
        },
    );
}

pub(super) fn queue_client_tick_end_commands(
    world: &WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    client_ticks: u32,
) {
    if world.local_player_id().is_none() {
        return;
    }
    let Some(tx) = net_commands else {
        return;
    };
    for _ in 0..client_ticks {
        // The render thread must never park on the command channel: under a
        // saturated channel the connection is already unusable, and a dropped
        // tick-end is harmless while a blocked render thread would stop
        // draining net events entirely.
        if tx.try_send(NetCommand::ClientTickEnd).is_err() {
            break;
        }
    }
}
static NEXT_SIMULATION_STEP_GENERATION: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(1);

#[derive(Debug, Clone)]
pub(crate) struct SimulationFrameState {
    pub(super) step_generation: u64,
    pub(super) now: Instant,
    pub(super) advanced_ticks: u32,
    pub(crate) network_work_applied: bool,
    pub(super) camera_spinning_ticks: CameraSpinningEffectTickState,
    pub(super) lightmap_environment: LightmapEnvironment,
    pub(super) sky_darken: u8,
    pub(super) water_vision: f32,
    pub(super) rain_fog_multiplier: f32,
    pub(super) hide_lightning_flash: bool,
    pub(super) cloud_environment: CloudEnvironment,
    terrain_textures_changed: bool,
    profiling_sections: Vec<SimulationProfileSection>,
}

impl SimulationFrameState {
    pub(crate) fn tick_anchor(&self) -> Instant {
        self.now
    }

    pub(crate) fn advanced_ticks(&self) -> u32 {
        self.advanced_ticks
    }

    pub(crate) fn profiling_sections(&self) -> &[SimulationProfileSection] {
        &self.profiling_sections
    }

    pub(crate) fn terrain_textures_changed(&self) -> bool {
        self.terrain_textures_changed
    }
}

pub(super) fn optional_profile_elapsed(started_at: Option<Instant>) -> Duration {
    started_at.map_or(Duration::ZERO, |started_at| started_at.elapsed())
}

fn push_simulation_profile_section(
    sections: &mut Vec<SimulationProfileSection>,
    path: &'static [&'static str],
    started_at: Option<Instant>,
) {
    if let Some(started_at) = started_at {
        sections.push(SimulationProfileSection {
            path,
            duration: started_at.elapsed(),
        });
    }
}

fn push_simulation_profile_duration(
    sections: &mut Vec<SimulationProfileSection>,
    path: &'static [&'static str],
    duration: Duration,
    enabled: bool,
) {
    if enabled {
        sections.push(SimulationProfileSection { path, duration });
    }
}

pub(super) fn advance_first_multiplayer_server_tutorial(
    world: &mut WorldStore,
    client_information: &bbb_protocol::packets::ClientInformation,
    platform_effects: &mut PlatformEffects,
) {
    if !world.show_social_interactions_tutorial_if_first_join() {
        return;
    }
    platform_effects.submit(
        crate::simulation::effects::PlatformEffect::SaveClientOptions {
            writes: vec![crate::options::ClientOptionWrite {
                key: "joinedFirstServer",
                value: crate::options::ClientOptionValue::Boolean(true),
            }],
            broadcast: client_information.clone(),
        },
    );
}

pub(crate) struct SimulationStepContext<'a> {
    pub(crate) net_events: &'a mut Option<mpsc::Receiver<NetEvent>>,
    pub(crate) net_commands: &'a Option<mpsc::Sender<NetCommand>>,
    pub(crate) server_resource_packs: &'a mut ServerResourcePackRuntime,
    pub(crate) particle_runtime: &'a mut Option<NativeParticleRuntime>,
    pub(crate) particles: &'a mut ParticleEngine,
    pub(crate) input: &'a mut ClientInputState,
    pub(crate) first_person_hand_presentation: &'a mut FirstPersonHandPresentation,
    pub(crate) world: &'a mut WorldStore,
    pub(crate) net_counters: &'a mut NetCounters,
    pub(crate) client_animation_ticks: &'a mut ClientAnimationTickState,
    pub(crate) camera_fov_ticks: &'a mut CameraFovTickState,
    pub(crate) camera_eye_height_ticks: &'a mut CameraEyeHeightTickState,
    pub(crate) camera_spinning_ticks: &'a mut CameraSpinningEffectTickState,
    pub(crate) hud_debug_network_sampler: &'a mut HudDebugNetworkSampler,
    pub(crate) hud_debug_tps_sampler: &'a mut HudDebugTpsSampler,
    pub(crate) lightmap_ticks: &'a mut LightmapTickState,
    pub(crate) level_event_sound_random: &'a mut LevelEventSoundRandomState,
    pub(crate) environment_audio: &'a mut crate::environment_audio_runtime::EnvironmentAudioRuntime,
    pub(crate) environment_animate:
        &'a mut crate::environment_animate_runtime::EnvironmentAnimateRuntime,
    pub(crate) weather_effects: &'a mut crate::weather_effect_runtime::WeatherEffectRuntime,
    pub(crate) music: &'a mut MusicRuntime,
    pub(crate) win_screen_resources: &'a mut Arc<crate::win_screen_resources::WinScreenResources>,
    pub(crate) terrain_textures: &'a mut TerrainTextureState,
    pub(crate) item_runtime: &'a mut Option<NativeItemRuntime>,
    pub(crate) code_of_conduct: &'a mut CodeOfConductAcceptance,
    pub(crate) server_list: &'a mut Option<crate::server_list::ServerListPersistence>,
    pub(crate) commands: Vec<crate::simulation::commands::SimulationCommand>,
    pub(crate) input_runtime: &'a mut crate::simulation::input_commands::SimulationInputRuntime,
    pub(crate) level_load_runtime: &'a mut LevelLoadRuntime,
    pub(crate) platform_effects: &'a mut PlatformEffects,
    pub(crate) hide_lightning_flash: bool,
    pub(crate) now: Instant,
}

pub(crate) fn advance_simulation(
    step: SimulationStepContext<'_>,
    mut audio_events: Option<&mut dyn AudioEventSink>,
) -> SimulationFrameState {
    let SimulationStepContext {
        net_events,
        net_commands,
        server_resource_packs,
        particle_runtime,
        particles,
        input,
        first_person_hand_presentation,
        world,
        net_counters,
        client_animation_ticks,
        camera_fov_ticks,
        camera_eye_height_ticks,
        camera_spinning_ticks,
        hud_debug_network_sampler,
        hud_debug_tps_sampler,
        lightmap_ticks,
        level_event_sound_random,
        environment_audio,
        environment_animate,
        weather_effects,
        music,
        win_screen_resources,
        terrain_textures,
        item_runtime,
        code_of_conduct,
        server_list,
        commands,
        input_runtime,
        level_load_runtime,
        platform_effects,
        hide_lightning_flash,
        now,
    } = step;
    let pump_started_at = Instant::now();
    let input_screen_was_open = input.blocks_world_input();
    let cursor_screen_was_open =
        input.current_screen().is_some() || world.level_load().screen_is_up();
    let level_generation_before_drain = world.client_level_generation();
    let mut remote_disconnect = None;
    let drained = if let Some(rx) = net_events.as_mut() {
        let audio_events_for_drain = audio_events
            .as_mut()
            .map(|audio_events| &mut **audio_events as &mut dyn AudioEventSink);
        events::drain_net_events_with_sinks_and_debug(
            input,
            rx,
            world,
            net_counters,
            net_commands,
            events::NetEventDispatchSinks {
                audio_events: audio_events_for_drain,
                particles: particle_target(particle_runtime, particles),
                item_runtime: item_runtime.as_ref(),
                hud_debug_network_sampler: Some(hud_debug_network_sampler),
                hud_debug_tps_sampler: Some(hud_debug_tps_sampler),
                server_list: server_list.as_mut(),
                remote_disconnect: Some(&mut remote_disconnect),
            },
            level_event_sound_random,
        )
    } else {
        0
    };
    if world.client_level_generation() != level_generation_before_drain {
        particles.clear();
    }
    let receiver_closed = net_events
        .as_ref()
        .is_some_and(|events| events.is_closed() && events.is_empty());
    if receiver_closed {
        *net_events = None;
    }
    if let Some(reason) = remote_disconnect {
        let listener_state = net_counters.state.as_deref();
        let title = disconnected_screen_title_for_state(listener_state);
        let stop_audio = remote_disconnect_stops_audio(listener_state);
        end_connection_and_show_disconnect(world, input, title, reason);
        particles.clear();
        if stop_audio {
            net_counters.simulation_remote_disconnect_audio_stop_requests = net_counters
                .simulation_remote_disconnect_audio_stop_requests
                .saturating_add(1);
            if let Some(audio_events) = audio_events.as_deref_mut() {
                audio_events.stop_all_sounds();
            }
        }
        input_runtime.request_terminal_screen_cursor(platform_effects);
    }
    world.expire_game_test_highlight_markers_at(monotonic_millis());
    world.poll_queued_light_updates();
    let pump_net_drained_at = Instant::now();
    server_resource_packs.synchronize(world, net_commands, net_counters.connected);
    input.synchronize_camera_post_effect(
        world.client_level_generation(),
        world.counters().set_camera_updates_applied,
        current_post_effect_id_from_world(world),
    );
    input.apply_screen_requests(world, net_counters, net_commands);
    input.sync_social_interactions_screen(world);
    input.sync_sign_editor(world, net_counters, net_commands);
    input.sync_book_screen(world);
    if let Some(screen) = input.game_rules_screen_mut() {
        sync_game_rules_screen_projection(
            screen,
            item_runtime.as_ref(),
            input_runtime.gui_surface_size(),
        );
    }
    let terrain_textures_changed =
        crate::terrain_upload::synchronize_terrain_biome_registry(terrain_textures, world);
    if terrain_textures_changed {
        if let Some(particle_runtime) = particle_runtime.as_mut() {
            particle_runtime.set_terrain_biomes(terrain_textures);
        }
    }
    release_input_if_screen_opened(
        input_screen_was_open,
        input,
        world,
        net_counters,
        net_commands,
    );
    input_runtime.reconcile_screen_cursor(
        cursor_screen_was_open,
        input,
        world,
        net_counters,
        platform_effects,
    );
    let input_profiling_enabled =
        input.debug_profiler_chart_visible() || input.debug_profiling_recording();
    let input_profile = input_runtime.apply_commands(
        commands,
        &mut crate::simulation::input_commands::SimulationInputContext {
            input,
            now,
            first_person_hand_presentation,
            world,
            net_counters,
            net_commands,
            server_resource_packs,
            item_runtime,
            code_of_conduct: Some(code_of_conduct),
            win_screen_resources,
            entity_partial_tick: client_animation_ticks.entity_partial_tick(),
            level_random: level_event_sound_random,
            audio_events: &mut audio_events,
            platform: platform_effects,
        },
        input_profiling_enabled,
    );
    let local_disconnect_teardown = input_runtime.take_pending_local_disconnect_teardown()
        || (receiver_closed
            && input_runtime.net_disconnect_requested()
            && input_runtime.running()
            && input.disconnected_screen_runtime().is_none());
    if local_disconnect_teardown && input_runtime.running() {
        end_connection_and_show_disconnect(
            world,
            input,
            crate::input::disconnected_screen::DisconnectedScreenTitle::ConnectionLost,
            bbb_client_net::DisconnectReason::from_plain("Disconnected"),
        );
        if let Some(audio_events) = audio_events.as_deref_mut() {
            audio_events.stop_all_sounds();
        }
        particles.clear();
        input_runtime.request_terminal_screen_cursor(platform_effects);
    }
    if let (Some(store), Some(preference)) =
        (server_list.as_mut(), world.resource_pack_preference())
    {
        if let Err(error) = store.persist_pack_preference(preference) {
            tracing::error!(%error, "failed to save the server resource-pack preference");
        }
    }
    // Vanilla `turnPlayer` only runs while the mouse is grabbed; mirror the
    // capture owner after this step's window/control commands settled it.
    input.set_mouse_look_live(input_runtime.cursor_captured());
    sync_stonecutter_recipe_scroll_state(input, world);
    sync_beacon_effect_selection_state(input, world);
    sync_loom_pattern_state_for_hud(input, world);
    let cursor_position = input.inventory_cursor_position();
    if let Some(screen) = input.advancements_screen_mut() {
        update_advancement_hover_fade(
            screen,
            world,
            cursor_position,
            input_runtime.gui_surface_size(),
        );
    }
    if let Some(stats) = input.stats_screen_mut() {
        super::stats_screen::sync_stats_screen_interaction(
            stats,
            world,
            item_runtime.as_ref(),
            input_runtime.gui_surface_size(),
        );
    }
    let pump_control_synced_at = Instant::now();
    let profiling_enabled =
        input.debug_profiler_chart_visible() || input.debug_profiling_recording();
    let mut profiling_sections = Vec::new();
    let level_services_started_at = profiling_enabled.then(Instant::now);
    let player_section_ready = input_runtime
        .player_section_ready(world.client_level_generation(), world.level_load().starts);
    let loading_screen_was_up = world.level_load().screen_is_up();
    level_load_runtime.tick(now, world, net_commands, player_section_ready);
    input_runtime.reconcile_screen_cursor(
        loading_screen_was_up,
        input,
        world,
        net_counters,
        platform_effects,
    );
    hud_debug_network_sampler.advance_tick(
        input,
        net_counters,
        net_commands,
        now,
        wall_clock_millis_i64(),
    );
    hud_debug_tps_sampler.sync_subscriptions_and_tick(input, world, net_counters, net_commands);
    let mut level_profile_duration = optional_profile_elapsed(level_services_started_at);
    let entities_profile_started_at = profiling_enabled.then(Instant::now);
    let entity_tick_started_at = Instant::now();
    let advanced_ticks = advance_native_client_ticks(input, world, client_animation_ticks, now);
    push_simulation_profile_section(
        &mut profiling_sections,
        &["tick", "entities"],
        entities_profile_started_at,
    );
    let gui_profile_started_at = profiling_enabled.then(Instant::now);
    sync_recipe_book_presentation(
        input,
        world,
        net_counters,
        net_commands,
        item_runtime.as_ref(),
        advanced_ticks,
    );
    if advanced_ticks > 0 {
        record_entity_client_tick_observation(
            net_counters,
            entity_tick_started_at.elapsed(),
            client_animation_ticks.last_entity_tick_work,
        );
    }
    let running_ticks = client_animation_ticks.last_running_level_ticks;
    let entity_partial_tick = client_animation_ticks.entity_partial_tick();
    camera_eye_height_ticks.advance_for_world(advanced_ticks, world);
    world.advance_tickable_entity_sounds(advanced_ticks, running_ticks);
    // Vanilla `Minecraft.tick` calls `Gui.tick` once per client tick (Gui.java:
    // 1145-1166), outside the tick-rate manager's freeze gate, so the action
    // bar / title countdowns advance on raw client ticks, not running ticks.
    world.advance_hud_text_ticks(advanced_ticks);
    world.advance_chat_gui_ticks(advanced_ticks);
    for _ in 0..advanced_ticks {
        for acknowledgement in world.advance_chat_listener(monotonic_millis(), false) {
            events::queue_chat_acknowledgement(net_commands, net_counters, acknowledgement);
        }
    }
    input.advance_dialog_waiting_ticks(advanced_ticks);
    push_simulation_profile_section(
        &mut profiling_sections,
        &["tick", "gui"],
        gui_profile_started_at,
    );
    let level_environment_started_at = profiling_enabled.then(Instant::now);
    lightmap_ticks.advance_for_world(advanced_ticks, world);
    lightmap_ticks.advance_rain_fog_for_world(advanced_ticks, world, terrain_textures);
    let water_vision = lightmap_ticks.water_vision(world);
    let rain_fog_multiplier = lightmap_ticks.rain_fog_multiplier();
    // Vanilla `Minecraft.tick` runs `ClientLevel.tick` before `GameRenderer.extract`;
    // `ClientLevel.tick` decrements `skyFlashTime`, and the render extract reads
    // the resulting EnvironmentAttributes / lightmap state.
    world.advance_sky_flash_time(advanced_ticks);
    let lightmap_environment = lightmap_ticks.environment_for_world(world);
    let render_camera_pose = render_camera_pose_from_world_at_partial_tick(
        world,
        entity_partial_tick,
        input.camera_type(),
        *camera_eye_height_ticks,
    );
    let cloud_environment = cloud_environment_for_world(world);
    // Vanilla `GameRenderer.tick` calls `LevelRenderer.tick` before entity and
    // block-entity ticks. Inside `LevelRenderer.tick`, weather sampling precedes
    // stale block-breaking cleanup. Canonical
    // entity mutation is already batched above, but submit these client-local
    // effects before the later entity/animate effect drains to preserve the
    // observable effect order.
    weather_effects.advance_for_ticks(
        world,
        running_ticks,
        render_camera_pose.map(|camera| camera_eye_position(camera).map(f64::from)),
        particle_target(particle_runtime, particles),
        &mut audio_events,
        item_runtime.as_ref(),
    );
    world.advance_block_destruction_render_ticks(running_ticks);
    // Vanilla `GameRenderer.tick` runs `ScreenEffectRenderer.tick` (the
    // item-activation countdown) inside the `tickRateManager().runsNormally()`
    // branch, so the totem animation freezes with the level.
    world.advance_item_activation_ticks(running_ticks);
    level_profile_duration = level_profile_duration
        .saturating_add(optional_profile_elapsed(level_environment_started_at));
    push_simulation_profile_duration(
        &mut profiling_sections,
        &["tick", "level"],
        level_profile_duration,
        profiling_enabled,
    );
    let block_entities_profile_started_at = profiling_enabled.then(Instant::now);
    // Vanilla `ChestBlockEntity.lidAnimateTick` is a client block-entity ticker
    // (`ClientLevel.tickBlockEntities`), gated by the tick-rate manager like the
    // rest of the level tick, so the lids advance on running ticks.
    world.advance_chest_lid_ticks(running_ticks);
    // Vanilla `BellBlockEntity.clientTick` is the same kind of client
    // block-entity ticker, so the bell shakes advance on running ticks too.
    world.advance_bell_shake_ticks(running_ticks);
    // Vanilla `ShulkerBoxBlockEntity.tick` (updateAnimation) is the same kind
    // of client block-entity ticker, so the box lids advance on running ticks.
    world.advance_shulker_box_lid_ticks(running_ticks);
    // The decorated pot wobble clock is vanilla `gameTime -
    // wobbleStartedAtTick`, re-expressed as running ticks since the block event.
    world.advance_decorated_pot_wobble_ticks(running_ticks);
    // Vanilla `EnchantingTableBlockEntity.bookAnimationTick` is another client
    // block-entity ticker, so the hovering books advance on running ticks too
    // (loaded state reconciles before the animation tick gate).
    world.advance_enchanting_table_book_ticks(running_ticks);
    // Vanilla `BeaconBlockEntity.tick` walks the beam ten blocks per tick and
    // republishes its coloured sections when the scan reaches the surface.
    world.advance_beacon_block_ticks(running_ticks);
    // Vanilla `ConduitBlockEntity.clientTick` is another client block-entity
    // ticker: it advances tickCount/activeRotation and refreshes the water +
    // prismarine frame every 40 game ticks.
    world.advance_conduit_ticks(running_ticks);
    // Vanilla `TheEndGatewayBlockEntity.beamAnimationTick` advances age and
    // cooldown on the client block-entity ticker.
    world.advance_end_gateway_ticks(running_ticks);
    // Vanilla `SkullBlockEntity.animation` is the client ticker for powered
    // dragon/piglin skull/head blocks.
    world.advance_skull_block_ticks(running_ticks);
    // Vanilla `BaseSpawner.clientTick` advances the ordinary spawner's display
    // entity spin only while a nearby player exists.
    world.advance_spawner_block_ticks(running_ticks);
    // Vanilla `TrialSpawner.tickClient` advances the cached display entity's
    // state-dependent spin independently of the ordinary nearby-player gate.
    world.advance_trial_spawner_block_ticks(running_ticks);
    // Vanilla `VaultBlockEntity.Client.tick` advances display-item spin before
    // its local particles and ambient sound.
    world.advance_vault_block_ticks(running_ticks);
    // `PistonMovingBlockEntity.tick` is a client block-entity ticker too: it
    // advances only with running level ticks, before scene extraction samples
    // the previous/current progress pair at the frame partial tick.
    world.advance_moving_piston_ticks(running_ticks);
    environment_animate.advance_block_entity_effects_for_ticks(
        world,
        net_counters.connection_generation,
        running_ticks,
        particle_target(particle_runtime, particles),
        &mut audio_events,
        item_runtime.as_ref(),
    );
    push_simulation_profile_section(
        &mut profiling_sections,
        &["tick", "blockEntities"],
        block_entities_profile_started_at,
    );
    let screen_profile_started_at = profiling_enabled.then(Instant::now);
    world.advance_item_cooldowns(advanced_ticks);
    input.advance_death_screen_ticks(advanced_ticks);
    // Vanilla `EnchantmentScreen.containerTick` advances its screen-local
    // book once per raw client tick, alongside the other active screen ticks.
    input.advance_enchantment_screen(world, advanced_ticks);
    advance_creative_inventory_commands(input, world, net_counters, net_commands, advanced_ticks);
    push_simulation_profile_section(
        &mut profiling_sections,
        &["tick", "screen"],
        screen_profile_started_at,
    );
    let keybindings_profile_started_at = profiling_enabled.then(Instant::now);
    // Vanilla `LocalPlayer.tick` (`LocalPlayer.java:227-251`): skip player and
    // vehicle movement/physics while the level-loading screen is still up.
    if !world.level_load().screen_is_up() {
        advance_player_input_for_client_ticks(
            input,
            world,
            net_counters,
            net_commands,
            now,
            advanced_ticks,
        );
        world.sync_local_player_entity();
    }
    push_simulation_profile_section(
        &mut profiling_sections,
        &["tick", "Keybindings"],
        keybindings_profile_started_at,
    );
    let animate_tick_profile_started_at = profiling_enabled.then(Instant::now);
    if let Some(audio_events) = audio_events.as_deref_mut() {
        // `LocalPlayer.tick` advances underwater, bubble-column, and biome
        // ambient handlers after player movement. Player entities keep ticking
        // while the level is frozen, so these follow raw local-player ticks.
        environment_audio.advance_for_ticks(
            world,
            net_counters.connection_generation,
            advanced_ticks,
            audio_events,
        );
    } else {
        // Audio may be disabled by a headless/test platform. Transition edges
        // are per-tick effects and must not replay if a sink appears later.
        world.take_pending_local_player_ambient_audio_ticks();
    }
    environment_animate.advance_for_ticks(
        world,
        net_counters.connection_generation,
        running_ticks,
        particle_target(particle_runtime, particles),
        &mut audio_events,
        item_runtime.as_ref(),
    );
    push_simulation_profile_section(
        &mut profiling_sections,
        &["tick", "animateTick"],
        animate_tick_profile_started_at,
    );
    let music_profile_started_at = profiling_enabled.then(Instant::now);
    if let Some(audio_events) = audio_events.as_deref_mut() {
        // `Minecraft.tick` advances `MusicManager` on every raw client tick
        // after the running-level animate loop.
        music.set_win_screen(input.win_screen_generation());
        music.advance_for_ticks(
            world,
            net_counters.connection_generation,
            advanced_ticks,
            audio_events,
        );
    }
    let mut sound_profile_duration = optional_profile_elapsed(music_profile_started_at);
    let game_renderer_profile_started_at = profiling_enabled.then(Instant::now);
    // Vanilla Minecraft.tick handles keybinds before GameRenderer.tick, whose
    // Camera.tickFov eases the current camera's target once per client tick.
    camera_fov_ticks.advance_for_world(
        advanced_ticks,
        world,
        input.camera_type().is_first_person(),
        world.local_player_is_sprinting(),
    );
    let target_zoom = input.target_zoom_factor();
    for _ in 0..advanced_ticks {
        camera_fov_ticks.step_zoom(target_zoom);
    }
    camera_spinning_ticks.advance_for_world(advanced_ticks, world);
    push_simulation_profile_section(
        &mut profiling_sections,
        &["tick", "gameRenderer"],
        game_renderer_profile_started_at,
    );
    let game_mode_profile_started_at = profiling_enabled.then(Instant::now);
    let audio_events_for_destroy = audio_events
        .as_mut()
        .map(|audio_events| &mut **audio_events as &mut dyn AudioEventSink);
    advance_destroying_block_at_partial_tick_with_biome_blend_radius(
        input,
        world,
        net_counters,
        net_commands,
        audio_events_for_destroy,
        entity_partial_tick,
        advanced_ticks,
        terrain_textures.biome_blend_radius(),
    );
    advance_using_item_at_partial_tick_with_effects_and_presentation(
        input,
        first_person_hand_presentation,
        world,
        net_counters,
        net_commands,
        entity_partial_tick,
        advanced_ticks,
        level_event_sound_random,
        &mut audio_events,
    );
    advance_local_using_item_ticks_with_effects(
        world,
        net_counters,
        net_commands,
        advanced_ticks,
        level_event_sound_random,
        &mut audio_events,
    );
    input_runtime.advance_scheduled_control_timers(
        advanced_ticks,
        &mut crate::simulation::input_commands::SimulationInputContext {
            input,
            now,
            first_person_hand_presentation,
            world,
            net_counters,
            net_commands,
            server_resource_packs,
            item_runtime,
            code_of_conduct: Some(code_of_conduct),
            win_screen_resources,
            entity_partial_tick,
            level_random: level_event_sound_random,
            audio_events: &mut audio_events,
            platform: platform_effects,
        },
    );
    submit_local_player_action_particles(
        world,
        input,
        particle_target(particle_runtime, particles),
        item_runtime.as_ref(),
    );
    if advanced_ticks > 0 {
        let mut looked_at_block = None;
        let mut raycast_computed = false;
        for _ in 0..advanced_ticks {
            advance_first_multiplayer_server_tutorial(
                world,
                input_runtime.client_information(),
                platform_effects,
            );
            if world.tutorial_step() != bbb_client_world::TutorialStep::None {
                if !raycast_computed {
                    raycast_computed = true;
                    looked_at_block = match crosshair_target_from_camera(
                        world,
                        camera_pose_from_world_at_partial_tick(world, entity_partial_tick),
                    ) {
                        Some(CrosshairTarget::Block(hit)) => Some(hit.pos),
                        _ => None,
                    };
                }
                if let Some(pos) = looked_at_block {
                    world.tutorial_on_look_at_block(pos);
                }
            }
            world.advance_tutorial_ticks(1);
        }
    }
    push_simulation_profile_section(
        &mut profiling_sections,
        &["tick", "gameMode"],
        game_mode_profile_started_at,
    );
    let particles_profile_started_at = profiling_enabled.then(Instant::now);
    first_person_hand_presentation.advance_for_ticks(
        world,
        item_runtime.as_ref(),
        local_player_hands_are_busy(input, world),
        advanced_ticks,
    );
    let particle_camera_pose = render_camera_pose;
    let particle_scope_context = particle_local_player_scope_context(
        world,
        particle_camera_pose,
        input.camera_type().is_first_person(),
    );
    let particle_sound_camera_position =
        particle_camera_pose.map(|camera| camera_eye_position(camera).map(f64::from));
    if advanced_ticks > 0 {
        if let Some(particle_runtime) = particle_runtime.as_mut() {
            particle_runtime.tick_explosion_particles(
                particles,
                world,
                level_event_sound_random,
                particle_sound_camera_position,
                item_runtime.as_ref(),
            );
        }
    }
    submit_primed_tnt_smoke_particles(particles, world, advanced_ticks);
    submit_local_player_sprint_particles(
        world,
        particle_target(particle_runtime, particles),
        advanced_ticks,
    );
    submit_entity_client_tick_effects(
        particles,
        world,
        particle_runtime.as_mut(),
        &mut audio_events,
    );
    submit_ominous_item_spawner_particles(world, particle_target(particle_runtime, particles));
    for event in world.take_pending_local_level_events() {
        events::dispatch_local_level_event(
            world,
            &event,
            &mut audio_events,
            particle_target(particle_runtime, particles),
            item_runtime.as_ref(),
            level_event_sound_random,
        );
    }
    events::dispatch_pending_client_particle_spawns(
        world,
        particle_runtime.as_ref(),
        particles,
        item_runtime.as_ref(),
    );
    // `Minecraft.tick`: `particleEngine.tick()` follows `level.animateTick`
    // while the level runs normally.
    if running_ticks > 0 {
        particles.advance(
            running_ticks,
            world,
            ParticleTickContext {
                scope: particle_scope_context,
                players: &particle_player_motion_contexts(world),
                sound_camera_position: particle_sound_camera_position,
            },
        );
    }
    emit_particle_sounds(&mut audio_events, particles);
    push_simulation_profile_section(
        &mut profiling_sections,
        &["tick", "particles"],
        particles_profile_started_at,
    );
    let sound_profile_started_at = profiling_enabled.then(Instant::now);
    if let Some(audio_events) = audio_events.as_deref_mut() {
        audio_events.tick_entity_sound_positions(
            audio_listener_state_from_world(world),
            &|entity_id| audio_entity_position_from_world(world, entity_id),
            &world.tickable_entity_sounds(),
        );
    }
    sound_profile_duration =
        sound_profile_duration.saturating_add(optional_profile_elapsed(sound_profile_started_at));
    push_simulation_profile_duration(
        &mut profiling_sections,
        &["sound"],
        sound_profile_duration,
        profiling_enabled,
    );
    let (portal_transition_trigger_sound, portal_transition_container_close) =
        world.take_pending_portal_transition_effects();
    if portal_transition_trigger_sound {
        if let Some(audio_events) = audio_events {
            // Vanilla `LocalPlayer.handlePortalTransitionEffect` plays
            // `SimpleSoundInstance.forLocalAmbience(PORTAL_TRIGGER,
            // random.nextFloat() * 0.4F + 0.8F, 0.25F)` on the 0 -> rising
            // intensity edge.
            audio_events.play_local_sound(&LocalSoundEventState {
                sound: SoundHolderState {
                    kind: "direct".to_string(),
                    registry_id: None,
                    location: Some("minecraft:block.portal.trigger".to_string()),
                    fixed_range: None,
                },
                source: "ambient".to_string(),
                volume: 0.25,
                pitch: level_event_sound_random.next_float() * 0.4 + 0.8,
                seed: 0,
            });
        }
    }
    if portal_transition_container_close {
        input.handle_portal_transition_screen_close(world, net_counters, net_commands);
    }
    // Vanilla sends the marker after `particleEngine.tick()`.
    queue_client_tick_end_commands(world, net_commands, advanced_ticks);
    flush_command_confirmation_narrations(input, item_runtime.as_ref(), platform_effects);
    for narration in world.take_pending_chat_narrations() {
        let text = chat_narration_text(&narration, item_runtime.as_ref());
        if text.is_empty() {
            continue;
        }
        platform_effects.submit(crate::simulation::effects::PlatformEffect::ChatNarration {
            kind: narration.kind,
            text,
        });
    }
    if let Err(err) = world.run_light_updates() {
        tracing::warn!(%err, "client light propagation skipped");
    }
    let pump_client_ticked_at = Instant::now();
    if profiling_enabled {
        if !input_profile.keyboard.is_zero() {
            profiling_sections.push(SimulationProfileSection {
                path: &["tick", "keyboard"],
                duration: input_profile.keyboard,
            });
        }
        if !input_profile.mouse.is_zero() {
            profiling_sections.push(SimulationProfileSection {
                path: &["mouse"],
                duration: input_profile.mouse,
            });
        }
        profiling_sections.extend([
            SimulationProfileSection {
                path: &["scheduledPacketProcessing"],
                duration: pump_net_drained_at - pump_started_at,
            },
            SimulationProfileSection {
                path: &["scheduledExecutables"],
                duration: (pump_control_synced_at - pump_net_drained_at)
                    .saturating_sub(input_profile.keyboard)
                    .saturating_sub(input_profile.mouse),
            },
            SimulationProfileSection {
                path: &["tick"],
                duration: (pump_client_ticked_at - pump_control_synced_at)
                    .saturating_sub(sound_profile_duration),
            },
        ]);
    }
    SimulationFrameState {
        step_generation: NEXT_SIMULATION_STEP_GENERATION
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        now,
        advanced_ticks,
        network_work_applied: drained != 0 || receiver_closed,
        camera_spinning_ticks: *camera_spinning_ticks,
        lightmap_environment,
        sky_darken: entity_shadow_sky_darken(world),
        water_vision,
        rain_fog_multiplier,
        hide_lightning_flash,
        cloud_environment,
        terrain_textures_changed,
        profiling_sections,
    }
}

pub(super) fn advance_entity_client_animations(
    world: &mut WorldStore,
    ticks: &mut ClientAnimationTickState,
    now: Instant,
) -> u32 {
    ticks.last_entity_tick_work = EntityClientTickWork::default();
    ticks.last_running_level_ticks = 0;
    let Some(last) = ticks.last_entity_animation_at else {
        ticks.last_entity_animation_at = Some(now);
        return 0;
    };
    let elapsed = now.saturating_duration_since(last);
    ticks.last_entity_animation_at = Some(now);
    let tick_interval = world_client_tick_interval(world);
    // Vanilla `DeltaTracker.Timer.advanceGameTime` converts only the time since
    // the previous frame with the current target MSPT, then carries a
    // dimensionless residual. Keeping that shape matters when a TickingState
    // update changes the target between two frames.
    ticks.entity_animation_tick_residual += elapsed.as_secs_f64() / tick_interval.as_secs_f64();
    let raw_ticks = ticks.entity_animation_tick_residual.floor();
    if raw_ticks == 0.0 {
        return 0;
    }

    // `Minecraft.runTick` consumes every elapsed tick from the timer but runs
    // at most ten `Minecraft.tick` calls; a longer stall drops the excess.
    ticks.entity_animation_tick_residual -= raw_ticks;
    let advanced_ticks = raw_ticks.min(f64::from(VANILLA_MAX_CLIENT_TICKS_PER_STEP)) as u32;
    (ticks.last_entity_tick_work, ticks.last_running_level_ticks) =
        world.advance_client_level_entity_ticks_with_observation(advanced_ticks);
    advanced_ticks
}

pub(super) fn advance_native_client_ticks(
    input: &ClientInputState,
    world: &mut WorldStore,
    ticks: &mut ClientAnimationTickState,
    now: Instant,
) -> u32 {
    if native_pause_freezes_client_ticks(input, world) {
        ticks.last_entity_tick_work = EntityClientTickWork::default();
        ticks.last_running_level_ticks = 0;
        ticks.last_entity_animation_at = Some(now);
        return 0;
    }
    advance_entity_client_animations(world, ticks, now)
}

pub(super) fn record_entity_client_tick_observation(
    counters: &mut NetCounters,
    duration: Duration,
    work: EntityClientTickWork,
) {
    let duration_nanos = |duration: Duration| duration.as_nanos().min(u128::from(u64::MAX)) as u64;
    let nanos = duration_nanos(duration);
    counters.simulation_entity_client_tick_nanos = nanos;
    counters.simulation_entity_client_tick_peak_nanos =
        counters.simulation_entity_client_tick_peak_nanos.max(nanos);
    let preparation_nanos = duration_nanos(work.preparation_duration);
    counters.simulation_entity_client_tick_preparation_nanos = preparation_nanos;
    counters.simulation_entity_client_tick_preparation_peak_nanos = counters
        .simulation_entity_client_tick_preparation_peak_nanos
        .max(preparation_nanos);
    let schedule_nanos = duration_nanos(work.schedule_duration);
    counters.simulation_entity_client_tick_schedule_nanos = schedule_nanos;
    counters.simulation_entity_client_tick_schedule_peak_nanos = counters
        .simulation_entity_client_tick_schedule_peak_nanos
        .max(schedule_nanos);
    let projectile_nanos = duration_nanos(work.projectile_duration);
    counters.simulation_entity_client_tick_projectile_nanos = projectile_nanos;
    counters.simulation_entity_client_tick_projectile_peak_nanos = counters
        .simulation_entity_client_tick_projectile_peak_nanos
        .max(projectile_nanos);
    let interpolation_nanos = duration_nanos(work.interpolation_duration);
    counters.simulation_entity_client_tick_interpolation_nanos = interpolation_nanos;
    counters.simulation_entity_client_tick_interpolation_peak_nanos = counters
        .simulation_entity_client_tick_interpolation_peak_nanos
        .max(interpolation_nanos);
    let block_effect_nanos = duration_nanos(work.block_effect_duration);
    counters.simulation_entity_client_tick_block_effect_nanos = block_effect_nanos;
    counters.simulation_entity_client_tick_block_effect_peak_nanos = counters
        .simulation_entity_client_tick_block_effect_peak_nanos
        .max(block_effect_nanos);
    counters.simulation_entity_client_tick_block_effect_entities = work.block_effect_entities;
    counters.simulation_entity_client_tick_block_effect_blocks_visited =
        work.block_effect_blocks_visited;
    counters.simulation_entity_client_tick_block_effect_callbacks_applied =
        work.block_effect_callbacks_applied;
    let animation_nanos = duration_nanos(work.animation_duration);
    counters.simulation_entity_client_tick_animation_nanos = animation_nanos;
    counters.simulation_entity_client_tick_animation_peak_nanos = counters
        .simulation_entity_client_tick_animation_peak_nanos
        .max(animation_nanos);
    counters.simulation_entity_client_tick_scanned_entities = work.scanned_entities;
    counters.simulation_entity_client_tick_mutated_entities = work.mutated_entities;
}

pub(crate) fn world_client_tick_interval(world: &WorldStore) -> Duration {
    let ticking = world.ticking();
    // `Minecraft.getTickTargetMillis` keeps the base 20 Hz timer while frozen;
    // otherwise it returns max(base MSPT, TickRateManager.millisecondsPerTick).
    // A server tick rate above 20 therefore never accelerates client ticks.
    if ticking.frozen || !ticking.tick_rate.is_finite() || ticking.tick_rate >= 20.0 {
        return VANILLA_CLIENT_TICK_INTERVAL;
    }

    let nanos_per_tick = (1_000_000_000.0_f64 / f64::from(ticking.tick_rate)) as u64;
    Duration::from_nanos(nanos_per_tick).max(VANILLA_CLIENT_TICK_INTERVAL)
}

pub(super) fn disconnected_screen_title_for_state(
    state: Option<&str>,
) -> crate::input::disconnected_screen::DisconnectedScreenTitle {
    if matches!(state, Some("Configuration" | "Play")) {
        crate::input::disconnected_screen::DisconnectedScreenTitle::ConnectionLost
    } else {
        crate::input::disconnected_screen::DisconnectedScreenTitle::ConnectFailed
    }
}

pub(super) fn remote_disconnect_stops_audio(state: Option<&str>) -> bool {
    matches!(state, Some("Configuration" | "Play"))
}

pub(super) fn world_ticking_milliseconds_per_tick(world: &WorldStore) -> f32 {
    let tick_rate = world.ticking().tick_rate;
    if tick_rate.is_finite() && tick_rate > 0.0 {
        1000.0 / tick_rate
    } else {
        50.0
    }
}

#[cfg(test)]
mod tests;
