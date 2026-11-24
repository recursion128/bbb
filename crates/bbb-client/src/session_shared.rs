//! Shared client session bootstrap, GPU asset upload, frame extraction/profiling,
//! resource-reload commit, narrator ticking, and redraw-scheduling glue used by
//! both the native desktop client (`desktop.rs`) and WebGPU browser client (`web.rs`).

use std::{collections::VecDeque, sync::Arc, time::Duration};

use crate::terrain_upload::{TerrainPrioritizeChunkUpdates, TerrainUploadState};
use anyhow::{Context, Result};
use bbb_client_assets::PackRoots;
use bbb_client_clock::Elapsed as _;
use bbb_client_net::{NetCommand, NetEvent};
use bbb_client_render::item_model::{
    font::{
        load_hud_font_atlas_with_options, load_hud_font_atlas_with_options_and_max_dimension,
        providers::FontOptions, HudFontAtlasImage,
    },
    NativeItemRuntime,
};
use bbb_client_render::scene::terrain_textures::{load_terrain_textures, TerrainTextureState};
use bbb_client_world::{ChatPresentationConfig, ChatVisibilityState, WorldStore};
use tokio::sync::mpsc;
use winit::{
    dpi::PhysicalSize,
    event::{DeviceEvent, Event, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow},
    window::{CursorIcon, Window},
};

use crate::{
    audio_runtime::{AudioEventSink, NativeAudioRuntime},
    camera_fov::CameraFovTickState,
    camera_pose::CameraEyeHeightTickState,
    code_of_conduct::CodeOfConductAcceptance,
    code_of_conduct_overlay,
    control::{self, AudioCounters, NetCounters, SharedSnapshot},
    debug_entries::DebugScreenEntryId,
    debug_profiler::{
        DebugProfiler, DebugProfilerEvent, DebugProfilerFrameMetrics, DebugProfilerSectionSample,
    },
    entity_assets::load_entity_model_textures,
    entity_scene::EntityRenderExtractionScratch,
    entity_variant_textures::NativeEntityVariantTextureRuntime,
    environment_animate_runtime::EnvironmentAnimateRuntime,
    environment_audio_runtime::EnvironmentAudioRuntime,
    event_loop_shared::{
        apply_window_cursor_icon, drain_dynamic_player_skin_downloads, PendingScreenshot,
    },
    frame_diagnostics::{self, FramePhaseDiagnostics, FramePhaseSample},
    hud_assets::{load_hud_textures, load_panorama},
    input::{self, ClientInputState, CommandSuggestionNarrationState},
    item_texture_animation::{maybe_upload_item_texture_animation, ItemTextureAnimationState},
    level_load_runtime::LevelLoadRuntime,
    music_runtime::MusicRuntime,
    narrator_runtime::{self, NarratorStatus},
    particle_runtime::{
        NativeParticleRenderRuntime, NativeParticleResources, NativeParticleRuntime,
    },
    recording,
    remote_debug::{RemoteDebugRendererSelection, RemoteDebugSelectionCache},
    resource_pack_runtime::ServerResourcePackRuntime,
    resource_reload::{
        self, apply_item_runtime_world_defaults, apply_server_resource_pack_reload_effect,
        client_resource_pack_diagnostics, commit_prepared_server_resource_pack_reload,
        prepare_native_shader_resources, PreparedServerResourcePackReload, ResourceReloadAck,
        SELECTED_LANGUAGE_PROBE_KEY,
    },
    runtime::{
        self, control_renderer_counters, dialog_ime_cursor_area, hud_player_list_is_visible,
        hud_player_list_narration_from_world, narrate_command_suggestion_change,
        narrate_player_list_visibility_change, project_dynamic_texture_counters, update_renderer,
        ClientAnimationTickState, DebugCollisionCache, DebugSkyLightSectionCache,
        DebugSupportingBlockCache, HudDebugFpsSampler, HudDebugNetworkSampler, HudDebugTpsSampler,
        HudRenderRuntime, LevelEventSoundRandomState, LightmapTickState, NativeRenderProfile,
    },
    server_list::ServerListPersistence,
    simulation::{
        self,
        commands::{CommandQueue, DebugProfilingLifecycle, SimulationCommand},
        effects::{PlatformEffects, ResourceReloadEffect},
        InputLatencyTracker, SimulationCore, SimulationRenderContext, SimulationView, StepOutcome,
    },
    sky_assets::load_sky_textures,
    sound_subtitle_runtime::SoundSubtitleRuntime,
    startup::{self, client_framerate_limit_interval, effective_framerate_limit},
    weather_effect_runtime,
    win_screen_resources::WinScreenResources,
    window_surface,
};

pub(crate) fn build_initial_world_and_net_counters(
    args: &startup::Args,
    pack_roots: Option<&PackRoots>,
    client_locale: &str,
) -> (WorldStore, NetCounters) {
    let mut world = WorldStore::new();
    world.set_local_player_auto_jump_enabled(args.client_auto_jump);
    world.set_tutorial_step(args.tutorial_step.into());
    world.set_joined_first_server(args.client_joined_first_server);
    world.set_social_server_label(args.server.clone());
    world.set_chat_presentation_config(ChatPresentationConfig {
        visibility: ChatVisibilityState::from(bbb_protocol::packets::ClientChatVisibility::from(
            args.client_chat_visibility,
        )),
        only_show_secure: args.client_only_show_secure_chat,
        hide_matched_names: args.client_hide_matched_names,
        message_delay_millis: (args.client_chat_delay_seconds * 1_000.0) as u64,
        hidden_players: Vec::new(),
        blocked_players: args.client_blocked_players.clone(),
    });
    let net_counters = NetCounters {
        client_resources: Arc::new(client_resource_pack_diagnostics(
            pack_roots,
            false,
            Some(client_locale),
            None,
        )),
        ..NetCounters::default()
    };
    (world, net_counters)
}

pub(crate) struct LoadedCoreClientAssets {
    pub(crate) client_particle_status: bbb_protocol::packets::ClientParticleStatus,
    pub(crate) particle_runtime: Option<NativeParticleRuntime>,
    pub(crate) particle_render_runtime: Option<NativeParticleRenderRuntime>,
    pub(crate) font_options: FontOptions,
    pub(crate) font_atlas: Option<HudFontAtlasImage>,
    pub(crate) item_runtime: Option<NativeItemRuntime>,
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn load_core_client_assets(
    args: &startup::Args,
    pack_roots: Option<&PackRoots>,
    client_locale: &str,
    world: &mut WorldStore,
    max_texture_dimension_2d: Option<u32>,
    log_stage_timings: bool,
) -> Result<LoadedCoreClientAssets> {
    let font_options = FontOptions {
        uniform: args.client_force_unicode_font,
        japanese_variants: args.client_japanese_glyph_variants,
    };
    let assets = load_core_client_assets_detached(
        args.client_particles.into(),
        font_options,
        pack_roots,
        client_locale,
        max_texture_dimension_2d,
        log_stage_timings,
    )?;
    apply_loaded_core_client_assets_to_world(&assets, world)?;
    Ok(assets)
}

/// Loads pack-derived core assets (particles, font atlas, item runtime) without
/// mutating `WorldStore`, so desktop startup can overlap this work with window
/// and WebGPU renderer initialization.
pub(crate) fn load_core_client_assets_detached(
    client_particle_status: bbb_protocol::packets::ClientParticleStatus,
    font_options: FontOptions,
    pack_roots: Option<&PackRoots>,
    client_locale: &str,
    max_texture_dimension_2d: Option<u32>,
    log_stage_timings: bool,
) -> Result<LoadedCoreClientAssets> {
    let particles_started_at = bbb_client_clock::Instant::now();
    let (particle_runtime, particle_render_runtime) = pack_roots
        .and_then(|roots| {
            NativeParticleResources::load(roots, client_particle_status)
                .map_err(|err| {
                    tracing::warn!(?err, "continuing without particle runtime");
                    err
                })
                .ok()
        })
        .map(|resources| (Some(resources.simulation), Some(resources.render)))
        .unwrap_or((None, None));
    if log_stage_timings {
        tracing::info!(
            stage = "particles",
            elapsed_ms = particles_started_at.elapsed().as_secs_f64() * 1_000.0,
            "native startup stage completed"
        );
    }

    let fonts_started_at = bbb_client_clock::Instant::now();
    let font_atlas = pack_roots
        .map(|roots| match max_texture_dimension_2d {
            Some(max_dim) => {
                load_hud_font_atlas_with_options_and_max_dimension(roots, font_options, max_dim)
            }
            None => load_hud_font_atlas_with_options(roots, font_options),
        })
        .transpose()?;
    if log_stage_timings {
        tracing::info!(
            stage = "fonts",
            elapsed_ms = fonts_started_at.elapsed().as_secs_f64() * 1_000.0,
            "native startup stage completed"
        );
    }

    let items_started_at = bbb_client_clock::Instant::now();
    let item_runtime = pack_roots
        .zip(font_atlas.as_ref())
        .and_then(|(roots, atlas)| {
            NativeItemRuntime::load_with_locale_and_fonts(
                roots,
                client_locale,
                Some(atlas.fonts.clone()),
            )
            .map(|items| {
                items
                    .with_bound_key_lookup(
                        crate::options::key_bindings::bound_key_display_key_units,
                    )
                    .with_local_time_formatter(bbb_client_world::local_time_select_value)
            })
            .map_err(|err| {
                tracing::warn!(?err, "continuing without item asset runtime");
                err
            })
            .ok()
        });
    if log_stage_timings {
        tracing::info!(
            stage = "items",
            elapsed_ms = items_started_at.elapsed().as_secs_f64() * 1_000.0,
            "native startup stage completed"
        );
    }

    if let Some(items) = &item_runtime {
        items.enable_profile_network_queues();
        if log_stage_timings {
            let (atlas_width, atlas_height) = items.atlas_size();
            let missingno_index = items.texture_index("minecraft:missingno");
            tracing::info!(
                item_definitions = items.item_definition_count(),
                item_registry_entries = items.item_registry_count(),
                resolved_models = items.resolved_model_count(),
                textures = items.texture_count(),
                icon_textures = items.icon_texture_count(),
                item_crafting_remainders = items.item_crafting_remainder_count(),
                furnace_fuel_items = items.furnace_fuel_item_count(),
                brewing_potion_items = items.brewing_potion_item_count(),
                brewing_ingredient_items = items.brewing_ingredient_item_count(),
                enchantment_lapis_lazuli_items = items.enchantment_lapis_lazuli_item_count(),
                cartography_additional_items = items.cartography_additional_item_count(),
                freeze_immune_wearable_items = items.freeze_immune_wearable_item_count(),
                powder_snow_walkable_foot_items = items.powder_snow_walkable_foot_item_count(),
                missing_models = items.missing_model_count(),
                missing_textures = items.missing_texture_count(),
                missingno_index,
                atlas_width,
                atlas_height,
                "loaded native item assets"
            );
        }
    }

    Ok(LoadedCoreClientAssets {
        client_particle_status,
        particle_runtime,
        particle_render_runtime,
        font_options,
        font_atlas,
        item_runtime,
    })
}

pub(crate) fn apply_loaded_core_client_assets_to_world(
    assets: &LoadedCoreClientAssets,
    world: &mut WorldStore,
) -> Result<()> {
    if let Some(items) = &assets.item_runtime {
        apply_item_runtime_world_defaults(items, world)?;
        narrator_runtime::update_localized_status_labels(|key| items.localized_text(key));
    }
    Ok(())
}

pub(crate) struct UploadedInitialGpuAssets {
    pub(crate) terrain_textures: TerrainTextureState,
    pub(crate) particles: bbb_client_world::particles::ParticleEngine,
    pub(crate) win_screen_resources: Arc<WinScreenResources>,
    pub(crate) entity_variant_textures: NativeEntityVariantTextureRuntime,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn configure_and_upload_initial_gpu_assets(
    renderer: &mut bbb_client_render::Renderer,
    args: &startup::Args,
    pack_roots: &Option<PackRoots>,
    font_atlas: &mut Option<HudFontAtlasImage>,
    particle_runtime: &mut Option<NativeParticleRuntime>,
    particle_render_runtime: &Option<NativeParticleRenderRuntime>,
    item_runtime: &mut Option<NativeItemRuntime>,
    client_locale: &str,
    net_counters: &mut NetCounters,
    command_queue: &CommandQueue,
    strict_resource_admission: bool,
) -> Result<UploadedInitialGpuAssets> {
    renderer.set_lightmap_brightness_factor(args.client_gamma);
    renderer.set_glint_settings(args.client_glint_speed, args.client_glint_strength as f32);
    renderer.set_terrain_max_anisotropy_bit(args.client_max_anisotropy_bit);
    renderer.set_cloud_status(
        args.render_clouds != startup::RenderCloudsArg::Off,
        match args.render_clouds {
            startup::RenderCloudsArg::Fast => bbb_client_render::CloudShape::Flat,
            _ => bbb_client_render::CloudShape::Fancy,
        },
    );
    renderer.set_menu_background_blurriness(args.menu_background_blurriness);
    renderer.set_hud_text_background_options(
        args.text_background_opacity,
        args.background_for_chat_only,
    );
    renderer.set_hud_chat_options(startup::hud_chat_options_from_args(args));

    if let Some(roots) = pack_roots.as_ref() {
        let shader_resources =
            prepare_native_shader_resources(roots).map(|resources| resources.renderer_projection());
        if let Err(error) = shader_resources.and_then(|resources| {
            renderer.transactional_shader_resource_reload(&resources, true, |_| Ok(()))
        }) {
            tracing::warn!(
                %error,
                "continuing with embedded shaders after startup load failure"
            );
        }
    }

    let terrain_atlas_base_padding = startup::terrain_atlas_base_padding_for_filtering(
        renderer.terrain_texture_filtering(),
        args.client_max_anisotropy_bit,
    );
    let terrain_textures = load_terrain_textures(
        renderer,
        pack_roots.as_ref(),
        args.cutout_leaves,
        args.client_smooth_lighting,
        args.biome_blend_radius,
        args.client_mipmap_levels,
        terrain_atlas_base_padding,
    );
    // Both item and terrain atlases have finished stitching from the shared
    // `"blocks"` PNG cache on this generation (shared with `base_pack_roots`).
    if let Some(roots) = pack_roots.as_ref() {
        roots.release_cached_block_texture_images();
    }
    renderer
        .finish_upload_batch()
        .context("upload terrain atlas batch")?;

    let mut particles = bbb_client_world::particles::ParticleEngine::default();
    if let Some(runtime) = particle_runtime.as_mut() {
        runtime.set_terrain_textures(&terrain_textures);
        if let Some(items) = item_runtime.as_ref() {
            runtime.set_item_runtime(items);
        }
        particles.set_sprites(runtime.sprite_table());
    }
    // If the detached startup loader built the font atlas before the GPU
    // device limits were known and the device max texture dimension is smaller
    // than the default dynamic atlas cap, rebuild with the actual device cap.
    let device_max_dim = renderer.max_texture_dimension_2d();
    if let (Some(roots), Some(atlas)) = (pack_roots.as_ref(), font_atlas.as_ref()) {
        let (atlas_w, atlas_h) = atlas.atlas_dimensions();
        if atlas_w > device_max_dim || atlas_h > device_max_dim || device_max_dim < 4_096 {
            let font_options = FontOptions {
                uniform: args.client_force_unicode_font,
                japanese_variants: args.client_japanese_glyph_variants,
            };
            if let Ok(rebuilt) = load_hud_font_atlas_with_options_and_max_dimension(
                roots,
                font_options,
                device_max_dim,
            ) {
                *font_atlas = Some(rebuilt);
            }
        }
    }
    load_hud_textures(renderer, pack_roots.as_ref().zip(font_atlas.take()));
    renderer
        .finish_upload_batch()
        .context("upload HUD atlas batch")?;
    load_panorama(renderer, pack_roots.as_ref());
    renderer.set_default_text_right_to_left(
        item_runtime
            .as_ref()
            .is_some_and(NativeItemRuntime::language_bidirectional),
    );

    let win_screen_resources = if strict_resource_admission {
        Arc::new(
            pack_roots
                .as_ref()
                .map(|roots| {
                    WinScreenResources::load(
                        roots,
                        &args.username,
                        &renderer.hud_text_measurements(),
                    )
                })
                .transpose()?
                .unwrap_or_default(),
        )
    } else {
        Arc::new(
            pack_roots
                .as_ref()
                .and_then(|roots| {
                    WinScreenResources::load(
                        roots,
                        &args.username,
                        &renderer.hud_text_measurements(),
                    )
                    .map_err(|err| {
                        tracing::warn!(?err, "continuing without win screen resources");
                        err
                    })
                    .ok()
                })
                .unwrap_or_default(),
        )
    };

    command_queue.push(SimulationCommand::TextMeasurements(
        renderer.hud_text_measurements(),
    ));

    let prepared_entity_textures = load_entity_model_textures(
        renderer,
        pack_roots.as_ref(),
        item_runtime
            .as_ref()
            .map(NativeItemRuntime::equipment_asset_catalog),
        item_runtime
            .as_ref()
            .map_or(0, NativeItemRuntime::resource_generation),
    );
    load_sky_textures(renderer, pack_roots.as_ref());
    if let Some(particles_gpu) = particle_render_runtime {
        if let Err(err) = particles_gpu.upload_particle_atlas(renderer) {
            tracing::warn!(?err, "continuing without particle atlas");
        }
    }
    if let Some(items) = item_runtime.as_mut() {
        if let Err(err) = resource_reload::upload_item_atlas(renderer, items) {
            tracing::warn!(?err, "continuing without item atlas");
        }
    }
    renderer
        .finish_upload_batch()
        .context("upload entity and item atlas batch")?;

    if let Some(roots) = pack_roots.as_ref() {
        if strict_resource_admission {
            roots
                .ensure_resource_data_admitted()
                .context("admit startup resource data generation")?;
        } else if let Err(err) = roots.ensure_resource_data_admitted() {
            tracing::warn!(?err, "startup resource data budget exceeded");
        }
    }

    net_counters.client_resources = Arc::new(client_resource_pack_diagnostics(
        pack_roots.as_ref(),
        item_runtime
            .as_ref()
            .is_some_and(NativeItemRuntime::language_bidirectional),
        Some(client_locale),
        item_runtime
            .as_ref()
            .map(|rt| rt.localized_text(SELECTED_LANGUAGE_PROBE_KEY)),
    ));

    let rig_catalog = Arc::new(
        pack_roots
            .as_ref()
            .and_then(|roots| bbb_client_assets::EntityRigCatalog::load(Some(roots)).ok())
            .unwrap_or_else(bbb_client_assets::EntityRigCatalog::builtin_default),
    );
    renderer.set_data_driven_entity_rigs(rig_catalog.rigs());

    let entity_variant_textures =
        NativeEntityVariantTextureRuntime::new(pack_roots.clone(), prepared_entity_textures)
            .with_rig_catalog(rig_catalog);

    Ok(UploadedInitialGpuAssets {
        terrain_textures,
        particles,
        win_screen_resources,
        entity_variant_textures,
    })
}

pub(crate) fn build_configured_input_state(
    args: &startup::Args,
    focused: bool,
    narrator_available: bool,
) -> ClientInputState {
    let mut input = ClientInputState::new(focused);
    input.set_dialog_link_options(args.client_chat_links, args.client_chat_links_prompt);
    input.set_save_chat_drafts(args.client_save_chat_drafts);
    input.set_client_text_filtering_enabled(args.client_text_filtering);
    input.set_hotbar_preset_store_path(crate::hotbar_presets::default_hotbar_preset_store_path());
    #[cfg(not(target_arch = "wasm32"))]
    {
        let startup_debug_profile = args.debug_profile.into();
        if let Some(path) = args.debug_profile_store.clone() {
            input.set_debug_profile_store_path(path.clone());
            match crate::debug_entries::DebugScreenEntryList::load_from_debug_profile_file(
                &path,
                startup_debug_profile,
            ) {
                Ok(entries) => input.set_debug_screen_entries(entries),
                Err(err) => {
                    tracing::warn!(
                        ?err,
                        path = %path.display(),
                        "resetting debug profile store to startup profile"
                    );
                    input.load_debug_screen_profile(startup_debug_profile);
                }
            }
        } else {
            input.load_debug_screen_profile(startup_debug_profile);
        }
    }
    #[cfg(target_arch = "wasm32")]
    input.load_debug_screen_profile(args.debug_profile.into());

    input.set_qol_extensions(args.extension_set());
    input.set_debug_advanced_item_tooltips(args.advanced_item_tooltips);
    input.set_debug_pause_on_lost_focus(args.pause_on_lost_focus);
    input.set_reduced_debug_info_option(args.client_reduced_debug_info);
    input.set_narrator_status(NarratorStatus::from(args.client_narrator));
    input.set_narrator_hotkey_enabled(args.client_narrator_hotkey);
    input.set_narrator_available(narrator_available);
    input.set_key_bindings(args.client_key_bindings.clone());
    input.set_toggle_key_options(
        args.client_toggle_crouch,
        args.client_toggle_sprint,
        args.client_toggle_attack,
        args.client_toggle_use,
    );
    input.set_fullscreen(args.fullscreen);
    input.set_mouse_sensitivity(args.client_mouse_sensitivity);
    input.set_invert_mouse(args.client_invert_mouse_x, args.client_invert_mouse_y);
    input.set_mouse_scroll_options(
        args.client_discrete_mouse_scroll,
        args.client_mouse_wheel_sensitivity,
    );
    input.set_debug_show_local_server_entity_hit_boxes(
        args.debug_show_local_server_entity_hit_boxes,
    );
    input.set_debug_hotkeys_enabled(args.debug_hotkeys);
    input.set_debug_feature_count_enabled(args.debug_feature_count);
    input.set_remote_debug_renderers(
        args.remote_debug_renderers
            .iter()
            .copied()
            .collect::<RemoteDebugRendererSelection>(),
    );
    input
}

pub(crate) struct SessionCoreInputs {
    pub(crate) net_events: Option<mpsc::Receiver<NetEvent>>,
    pub(crate) net_commands: Option<mpsc::Sender<NetCommand>>,
    pub(crate) server_resource_packs: ServerResourcePackRuntime,
    pub(crate) input: ClientInputState,
    pub(crate) world: WorldStore,
    pub(crate) net_counters: NetCounters,
    pub(crate) client_particle_status: bbb_protocol::packets::ClientParticleStatus,
    pub(crate) particle_runtime: Option<NativeParticleRuntime>,
    pub(crate) item_runtime: Option<NativeItemRuntime>,
    pub(crate) uploaded_gpu: UploadedInitialGpuAssets,
    pub(crate) code_of_conduct: CodeOfConductAcceptance,
    pub(crate) server_list: Option<ServerListPersistence>,
    pub(crate) audio_status: AudioCounters,
    pub(crate) control_views_enabled: bool,
    pub(crate) publication_demand: Arc<control::ControlPublicationDemand>,
}

pub(crate) struct SessionRenderState {
    pub(crate) profile_network: Option<bbb_client_net::ProfileNetworkService>,
    pub(crate) entity_variant_textures: NativeEntityVariantTextureRuntime,
    pub(crate) entity_render_extraction: EntityRenderExtractionScratch,
    pub(crate) non_entity_render_extraction: runtime::NonEntityRenderExtractionScratch,
    pub(crate) camera_environment_samples: runtime::CameraEnvironmentSampleCache,
    pub(crate) recording: recording::RecordingRuntime,
    pub(crate) terrain_upload: TerrainUploadState,
    pub(crate) lod_terrain: crate::lod_terrain::LodTerrainRuntime,
    pub(crate) item_texture_animation: ItemTextureAnimationState,
    pub(crate) hud_debug_fps_sampler: HudDebugFpsSampler,
    pub(crate) debug_collision_cache: DebugCollisionCache,
    pub(crate) debug_supporting_block_cache: DebugSupportingBlockCache,
    pub(crate) debug_sky_light_section_cache: DebugSkyLightSectionCache,
    pub(crate) remote_debug_selection_cache: RemoteDebugSelectionCache,
    pub(crate) debug_profiler: DebugProfiler,
    pub(crate) frame_phase_diagnostics: FramePhaseDiagnostics,
    pub(crate) last_profiled_step: Option<u64>,
    pub(crate) step_generation: u64,
    pub(crate) rendered_client_tick: Option<u64>,
    pub(crate) latest_simulation_profile_sections: Vec<runtime::SimulationProfileSection>,
    pub(crate) frame_interval: Option<Duration>,
    pub(crate) latest_input_at: bbb_client_clock::Instant,
    pub(crate) next_redraw_at: bbb_client_clock::Instant,
    pub(crate) last_command_suggestion_narration: Option<CommandSuggestionNarrationState>,
    pub(crate) player_list_narration_visible: bool,
    pub(crate) screen_narrations: narrator_runtime::ScreenNarrations,
    pub(crate) pending_control_screenshots: VecDeque<PendingScreenshot>,
    pub(crate) pending_recording_completions: Vec<(
        control::ControlCompletionSender,
        std::result::Result<control::RecordingStatus, String>,
    )>,
    pub(crate) platform_exit_requested: bool,
    pub(crate) hud_render_runtime: HudRenderRuntime,
    pub(crate) renderer_counters: control::RendererCounters,
    pub(crate) control_views_enabled: bool,
    pub(crate) publication_demand: Arc<control::ControlPublicationDemand>,
    pub(crate) pending_world_publication: bool,
    pub(crate) last_snapshot_published_at: Option<bbb_client_clock::Instant>,
    pub(crate) cached_base_client_ui: Option<serde_json::Value>,
    pub(crate) input_latency: InputLatencyTracker,
    pub(crate) event_cursor_position: window_surface::CursorPositionCache,
    pub(crate) current_cursor_icon: CursorIcon,
    pub(crate) current_window_title: Option<String>,
    pub(crate) keyboard_modifiers: winit::keyboard::ModifiersState,
    pub(crate) last_ime_cursor_area:
        Option<(winit::dpi::PhysicalPosition<i32>, PhysicalSize<u32>, u64)>,
    pub(crate) redraw_requested: bool,
}

pub(crate) fn build_simulation_core_and_render_state(
    args: &startup::Args,
    pack_roots: Option<&PackRoots>,
    renderer: &mut bbb_client_render::Renderer,
    inputs: SessionCoreInputs,
) -> Result<(SimulationCore, SessionRenderState)> {
    let startup_client_information = startup::client_information_from_args(args)?;
    let UploadedInitialGpuAssets {
        terrain_textures,
        particles,
        win_screen_resources,
        entity_variant_textures,
    } = inputs.uploaded_gpu;

    let mut hud_render_runtime = HudRenderRuntime::default();
    hud_render_runtime.set_panorama_scroll_speed(args.panorama_scroll_speed);
    if let Some(roots) = pack_roots {
        match roots
            .load_periodic_notification_catalog()
            .and_then(|catalog| {
                let generation = catalog.generation();
                HudRenderRuntime::prepare_periodic_notifications(&catalog)
                    .map(|schedule| (generation, schedule))
            }) {
            Ok((generation, schedule)) => hud_render_runtime.install_periodic_notifications(
                generation,
                schedule,
                bbb_client_clock::now(),
            ),
            Err(error) => {
                tracing::warn!(
                    %error,
                    "continuing without periodic notifications after startup load failure"
                );
            }
        }
    }

    let mut simulation_input_runtime = simulation::input_commands::SimulationInputRuntime::new(
        renderer.physical_surface_size(),
        false,
        startup_client_information,
    );
    simulation_input_runtime
        .set_gui_scale_options(args.client_gui_scale, args.client_force_unicode_font);
    let (gui_size, gui_scale) = simulation_input_runtime.gui_viewport();
    renderer.set_gui_viewport(gui_size, gui_scale);

    let mut renderer_counters = control_renderer_counters(renderer.counters());
    project_dynamic_texture_counters(&mut renderer_counters, inputs.item_runtime.as_ref());
    let mut world = inputs.world;
    let level_event_sound_random = LevelEventSoundRandomState::with_seed(world.next_sound_seed());

    let launched_idle = inputs.net_events.is_none();
    let core = SimulationCore {
        net_events: inputs.net_events,
        net_commands: inputs.net_commands,
        server_resource_packs: inputs.server_resource_packs,
        input: inputs.input,
        first_person_hand_presentation: Default::default(),
        world,
        net_counters: inputs.net_counters,
        client_animation_ticks: ClientAnimationTickState::default(),
        camera_fov_ticks: CameraFovTickState::new(
            f32::from(args.client_fov),
            args.client_fov_effect_scale,
        ),
        camera_eye_height_ticks: CameraEyeHeightTickState::default(),
        camera_spinning_ticks: runtime::CameraSpinningEffectTickState::default(),
        hud_debug_network_sampler: HudDebugNetworkSampler::default(),
        hud_debug_tps_sampler: HudDebugTpsSampler::default(),
        lightmap_ticks: LightmapTickState::with_brightness_factor_and_hide_lightning_flash(
            args.client_gamma,
            args.hide_lightning_flash,
        ),
        level_event_sound_random,
        environment_audio: EnvironmentAudioRuntime::default(),
        environment_animate: EnvironmentAnimateRuntime::default(),
        weather_effects: weather_effect_runtime::WeatherEffectRuntime::new_with_radius(
            inputs.client_particle_status,
            args.client_weather_radius,
        ),
        music: MusicRuntime::configured(
            args.client_music_frequency,
            args.client_audio_volumes
                .final_source_volume(&bbb_client_audio::AudioCategory::Music),
        ),
        win_screen_resources,
        particle_runtime: inputs.particle_runtime,
        particles,
        terrain_textures,
        item_runtime: inputs.item_runtime,
        code_of_conduct: inputs.code_of_conduct,
        server_list: inputs.server_list,
        input_runtime: simulation_input_runtime,
        level_load_runtime: LevelLoadRuntime::default(),
        audio_counters: inputs.audio_status,
        hide_lightning_flash: args.hide_lightning_flash,
        platform_effects: PlatformEffects::default(),
        frame: None,
        world_revision: 0,
        terrain_textures_changed: false,
        synced_content_registry: None,
        launched_idle,
        next_tick_deadline: bbb_client_clock::now(),
    };

    let player_skin_cache_dir = args
        .player_skin_cache_dir
        .clone()
        .unwrap_or_else(bbb_client_net::default_player_skin_cache_dir);
    let profile_network = match bbb_client_net::ProfileNetworkService::new(player_skin_cache_dir) {
        Ok(service) => Some(service),
        Err(error) => {
            tracing::warn!(
                %error,
                "continuing without profile network service after startup initialization failure"
            );
            None
        }
    };

    let now = bbb_client_clock::now();
    let state = SessionRenderState {
        profile_network,
        entity_variant_textures,
        entity_render_extraction: EntityRenderExtractionScratch::default(),
        non_entity_render_extraction: runtime::NonEntityRenderExtractionScratch::default(),
        camera_environment_samples: runtime::CameraEnvironmentSampleCache::default(),
        recording: recording::RecordingRuntime::default(),
        terrain_upload: TerrainUploadState::default(),
        lod_terrain: crate::lod_terrain::LodTerrainRuntime::default(),
        item_texture_animation: ItemTextureAnimationState::default(),
        hud_debug_fps_sampler: HudDebugFpsSampler::default(),
        debug_collision_cache: DebugCollisionCache::default(),
        debug_supporting_block_cache: DebugSupportingBlockCache::default(),
        debug_sky_light_section_cache: DebugSkyLightSectionCache::default(),
        remote_debug_selection_cache: RemoteDebugSelectionCache::default(),
        debug_profiler: DebugProfiler::with_report_writer(
            input::DEBUG_PROFILING_RESULTS_RELATIVE_DIR,
            args.server.clone(),
        ),
        frame_phase_diagnostics: FramePhaseDiagnostics::default(),
        last_profiled_step: None,
        step_generation: 0,
        rendered_client_tick: None,
        latest_simulation_profile_sections: Vec::new(),
        frame_interval: client_framerate_limit_interval(args.client_framerate_limit)
            .filter(|_| !bbb_client_clock::is_virtual()),
        latest_input_at: now,
        next_redraw_at: now,
        last_command_suggestion_narration: None,
        player_list_narration_visible: false,
        screen_narrations: narrator_runtime::ScreenNarrations::default(),
        pending_control_screenshots: VecDeque::new(),
        pending_recording_completions: Vec::new(),
        platform_exit_requested: false,
        hud_render_runtime,
        renderer_counters,
        control_views_enabled: inputs.control_views_enabled,
        publication_demand: {
            if inputs.control_views_enabled {
                inputs.publication_demand.attach_simulation();
            }
            inputs.publication_demand
        },
        pending_world_publication: true,
        last_snapshot_published_at: None,
        cached_base_client_ui: None,
        input_latency: InputLatencyTracker::new(args.renderer_input_latency_diagnostics),
        event_cursor_position: window_surface::CursorPositionCache::default(),
        current_cursor_icon: CursorIcon::Default,
        current_window_title: None,
        keyboard_modifiers: winit::keyboard::ModifiersState::empty(),
        last_ime_cursor_area: None,
        redraw_requested: false,
    };

    Ok((core, state))
}

pub(crate) struct FrameUpdateTimings {
    pub(crate) profiler_enabled: bool,
    pub(crate) profiler_frame_started_at: bbb_client_clock::Instant,
    pub(crate) profiler_update_duration: Duration,
    pub(crate) native_render_profile: NativeRenderProfile,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_and_extract_frame(
    view: &SimulationView<'_>,
    terrain_textures: &mut TerrainTextureState,
    item_runtime: Option<&NativeItemRuntime>,
    code_of_conduct_accepted: bool,
    state: &mut SessionRenderState,
    renderer: &mut bbb_client_render::Renderer,
    window: &Window,
    args: &startup::Args,
    command_queue: &CommandQueue,
    audio_runtime: &mut Option<NativeAudioRuntime>,
    sound_subtitle_runtime: &mut SoundSubtitleRuntime,
    particle_render_runtime: &mut Option<NativeParticleRenderRuntime>,
    narrator: Option<&mut dyn narrator_runtime::SystemNarrator>,
) -> FrameUpdateTimings {
    let profiler_frame_started_at = bbb_client_clock::Instant::now();

    let profiler_enabled =
        state.debug_profiler.is_recording() || view.input.debug_profiler_chart_visible();
    if !profiler_enabled {
        state.last_profiled_step = None;
        state.latest_simulation_profile_sections.clear();
    }
    let gpu_timing_enabled = state.debug_profiler.is_recording()
        || view.input.debug_screen_entry_enabled(
            DebugScreenEntryId::GpuUtilization,
            view.input.show_only_reduced_info(view.world),
        );
    renderer.set_gpu_timing_enabled(gpu_timing_enabled);
    renderer.set_submission_profiling_enabled(state.debug_profiler.is_recording());
    state
        .frame_phase_diagnostics
        .set_enabled(gpu_timing_enabled);

    let dynamic_texture_resource_generation =
        item_runtime.map_or(0, NativeItemRuntime::dynamic_texture_resource_generation);
    if let Some(items) = item_runtime {
        items.synchronize_dynamic_texture_session(
            view.net_counters.connection_generation,
            view.net_counters.connected,
        );
        if let Some(network) = state.profile_network.as_mut() {
            items.apply_profile_resolution_results(network.drain_profile_results());
            let validated =
                items.apply_dynamic_texture_fetch_results(network.drain_texture_results());
            if !validated.is_empty() {
                network.cache_validated_textures(validated);
            }
        }
    }
    renderer.synchronize_dynamic_entity_texture_generation(
        dynamic_texture_resource_generation,
        view.net_counters.connection_generation,
        view.net_counters.connected,
    );
    if let Some(items) = item_runtime {
        drain_dynamic_player_skin_downloads(items, renderer);
    }

    let profiler_update_started_at = bbb_client_clock::Instant::now();
    let native_render_profile = update_renderer(
        view,
        SimulationRenderContext {
            rendered_client_tick: &mut state.rendered_client_tick,
            gui_cursor_position: state.event_cursor_position.project(
                view.input_runtime.surface_size,
                PhysicalSize::new(
                    view.input_runtime.gui_scaled_width,
                    view.input_runtime.gui_scaled_height,
                ),
            ),
            audio_events: audio_runtime
                .as_mut()
                .map(|rt| rt as &mut dyn AudioEventSink),
            renderer,
            commands: command_queue,
            input_latency: state.input_latency.pending(),
            terrain_upload: &mut state.terrain_upload,
            lod_terrain: &mut state.lod_terrain,
            terrain_textures,
            entity_variant_textures: &mut state.entity_variant_textures,
            entity_render_extraction: &mut state.entity_render_extraction,
            non_entity_render_extraction: &mut state.non_entity_render_extraction,
            camera_environment_samples: &mut state.camera_environment_samples,
            item_runtime,
            hud_render_runtime: &mut state.hud_render_runtime,
            hud_debug_fps_sampler: &mut state.hud_debug_fps_sampler,
            debug_collision_cache: &mut state.debug_collision_cache,
            debug_supporting_block_cache: &mut state.debug_supporting_block_cache,
            debug_sky_light_section_cache: &mut state.debug_sky_light_section_cache,
            remote_debug_selection_cache: &mut state.remote_debug_selection_cache,
            render_distance_chunks: args.render_distance_chunks,
            prioritize_chunk_updates: match args.prioritize_chunk_updates {
                startup::PrioritizeChunkUpdatesArg::None => TerrainPrioritizeChunkUpdates::None,
                startup::PrioritizeChunkUpdatesArg::PlayerAffected => {
                    TerrainPrioritizeChunkUpdates::PlayerAffected
                }
                startup::PrioritizeChunkUpdatesArg::Nearby => TerrainPrioritizeChunkUpdates::Nearby,
            },
            entity_distance_scaling: args.client_entity_distance_scaling,
            weather_radius: args.client_weather_radius,
            cloud_range_chunks: args.cloud_range_chunks,
            client_framerate_limit: args.client_framerate_limit,
            client_vsync: args.client_vsync,
            damage_tilt_strength: args.damage_tilt_strength,
            view_bobbing: args.client_view_bobbing,
            vignette: args.client_vignette,
            attack_indicator: args.client_attack_indicator.into(),
            screen_effect_scale: args.client_screen_effect_scale,
            notification_display_time: args.notification_display_time,
            music_toast_display: args.client_music_toast,
            final_music_volume: args
                .client_audio_volumes
                .final_source_volume(&bbb_client_audio::AudioCategory::Music),
            entity_shadows: args.entity_shadows,
            high_contrast_block_outline: args.high_contrast_block_outline,
            improved_transparency: args.improved_transparency,
            dynamic_lights_mode: args.dynamic_lights.into(),
            lod_distance_chunks: args.lod_distance_chunks,
            extensions: args.extension_set(),
            hud_debug_profiler_chart: state.debug_profiler.chart(),
        },
    );
    if let (Some(items), Some(network)) = (item_runtime, state.profile_network.as_mut()) {
        network.submit_profile_requests(items.take_pending_profile_resolution_requests());
        network.submit_texture_requests(items.take_pending_dynamic_texture_requests());
    }

    let subtitle_now = bbb_client_clock::now();
    if let Some(audio) = audio_runtime.as_mut() {
        let events = audio.drain_subtitle_events();
        if args.client_show_subtitles {
            for event in events {
                sound_subtitle_runtime.record(event, subtitle_now);
            }
        }
    }
    let sound_subtitles = if args.client_show_subtitles && audio_runtime.is_some() {
        sound_subtitle_runtime.project(
            runtime::audio_listener_state_from_world(view.world),
            args.notification_display_time,
            subtitle_now,
            |key| {
                item_runtime
                    .as_ref()
                    .map(|rt| rt.localized_text(key))
                    .unwrap_or(key)
                    .to_string()
            },
        )
    } else {
        bbb_client_render::HudSoundSubtitles::default()
    };
    renderer.set_hud_sound_subtitles(sound_subtitles);

    if let Some(items) = item_runtime {
        maybe_upload_item_texture_animation(renderer, &mut state.item_texture_animation, items);
    }
    if let Some(particles_gpu) = particle_render_runtime.as_mut() {
        particles_gpu.maybe_upload_particle_atlas_animation(renderer);
    }

    state.screen_narrations.tick(
        state.hud_render_runtime.screen_narrations(),
        bbb_client_clock::now(),
        view.input.last_input_type(),
        narrator,
    );

    let dialog_preedit = view.input.dialog_ime_preedit_hud_state();
    let sign_preedit_area = renderer.hud_sign_editor_ime_cursor_area(renderer.gui_surface_size());
    let dialog_preedit_area = || {
        dialog_ime_cursor_area(
            view.input,
            view.world,
            dialog_preedit.as_ref(),
            renderer.gui_surface_size(),
            renderer,
        )
    };
    // The IME caret comes from logical GUI layout; the platform cursor
    // rectangle is physical, so convert back at the platform boundary.
    let gui_surface_size = renderer.gui_surface_size();
    let anvil_ime_area = || {
        if !crate::input::anvil_screen_is_open(view.world) {
            return None;
        }
        let field = view.input.anvil_rename_field()?;
        let layout =
            crate::input::inventory_screen_layout_for_surface(view.world, gui_surface_size)?;
        Some(bbb_client_render::hud_inventory_text_field_ime_cursor_area(
            &field.hud(true, false),
            u32::try_from(layout.width).unwrap_or_default(),
            u32::try_from(layout.height).unwrap_or_default(),
            62,
            24,
            103,
            10,
            gui_surface_size,
            renderer.hud_font_book_ref().glyphs(None),
        ))
    };
    let logical_ime_area = sign_preedit_area
        .or_else(dialog_preedit_area)
        .or_else(|| renderer.hud_ime_cursor_area(gui_surface_size))
        .or_else(anvil_ime_area)
        .or_else(|| {
            view.input.text_input_is_active(view.world).then(|| {
                (
                    winit::dpi::PhysicalPosition::new(
                        (gui_surface_size.width / 2) as i32,
                        (gui_surface_size.height / 2) as i32,
                    ),
                    PhysicalSize::new(1, 12),
                )
            })
        });
    let next_ime_cursor_area = logical_ime_area
        .and_then(|(position, size)| {
            window_surface::gui_rect_to_physical(
                position,
                size,
                gui_surface_size,
                window.inner_size(),
            )
        })
        .map(|(position, size)| (position, size, window.scale_factor().to_bits()));
    if state.last_ime_cursor_area != next_ime_cursor_area {
        state.last_ime_cursor_area = next_ime_cursor_area;
        if let Some((position, size, _)) = next_ime_cursor_area {
            window.set_ime_cursor_area(position, size);
            #[cfg(target_arch = "wasm32")]
            crate::web::ime::set_web_ime_cursor_area(position, size, window.scale_factor());
        }
    }

    let profiler_update_duration = profiler_update_started_at
        .elapsed()
        .saturating_sub(native_render_profile.extract)
        .saturating_sub(native_render_profile.toasts);

    let code_of_conduct_screen = code_of_conduct_overlay::code_of_conduct_renderer_screen(
        view.world,
        view.input_runtime.code_of_conduct_overlay,
        code_of_conduct_accepted,
        PhysicalSize::new(
            view.input_runtime.gui_scaled_width,
            view.input_runtime.gui_scaled_height,
        ),
        renderer.hud_text_measurements_ref(),
    );
    renderer.set_hud_code_of_conduct_screen(code_of_conduct_screen);

    let pointer = state.event_cursor_position.project(
        renderer.physical_surface_size(),
        renderer.gui_surface_size(),
    );
    apply_window_cursor_icon(
        window,
        args.client_allow_cursor_changes,
        &mut state.current_cursor_icon,
        window_surface::hud_cursor_icon(renderer, pointer),
    );

    FrameUpdateTimings {
        profiler_enabled,
        profiler_frame_started_at,
        profiler_update_duration,
        native_render_profile,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn finish_rendered_frame(
    view: &simulation::SimulationView<'_>,
    item_runtime: Option<&NativeItemRuntime>,
    timings: FrameUpdateTimings,
    profiler_render_duration: Duration,
    state: &mut SessionRenderState,
    renderer: &mut bbb_client_render::Renderer,
    args: &startup::Args,
    command_queue: &CommandQueue,
    window_minimized: bool,
) {
    bbb_client_clock::replay::count(&bbb_client_clock::replay::FRAMES_RENDERED);
    if bbb_client_clock::is_virtual() {
        bbb_client_clock::replay::flag(
            &bbb_client_clock::replay::RENDER_WORK_PENDING,
            state.terrain_upload.has_pending_work(view.world)
                || !state.pending_control_screenshots.is_empty(),
        );
    }
    bbb_client_clock::replay::flag(&bbb_client_clock::replay::STATE_UNRENDERED, false);

    let renderer_frame_profile = renderer.last_frame_profile();
    if let Some(presented) = renderer_frame_profile.input_latency {
        state.input_latency.presented(presented.sequence);
    }

    let profiler_publish_started_at = bbb_client_clock::Instant::now();
    if state.control_views_enabled || timings.profiler_enabled {
        state.renderer_counters = control_renderer_counters(renderer.counters());
        state.renderer_counters.recording = state.recording.status();
        let entity_observation = state.entity_render_extraction.observation();
        state.renderer_counters.entity_render_candidates = entity_observation.candidates;
        state.renderer_counters.entity_render_survivors = entity_observation.survivors;
        state.renderer_counters.entity_render_projected = entity_observation.projected;
        state.renderer_counters.entity_candidate_nanos = entity_observation.candidate_nanos;
        state.renderer_counters.entity_candidate_peak_nanos =
            entity_observation.candidate_peak_nanos;
        state.renderer_counters.entity_cull_nanos = entity_observation.cull_nanos;
        state.renderer_counters.entity_cull_peak_nanos = entity_observation.cull_peak_nanos;
        state.renderer_counters.entity_full_projection_nanos = entity_observation.projection_nanos;
        state.renderer_counters.entity_full_projection_peak_nanos =
            entity_observation.projection_peak_nanos;
        state.renderer_counters.entity_model_build_nanos = entity_observation.model_build_nanos;
        state.renderer_counters.entity_model_build_peak_nanos =
            entity_observation.model_build_peak_nanos;
        state.renderer_counters.entity_extraction_capacity_growths =
            entity_observation.capacity_growths;
        let non_entity_observation = state.non_entity_render_extraction.observation();
        state.renderer_counters.non_entity_hud_nanos = non_entity_observation.hud_nanos;
        state.renderer_counters.non_entity_hud_peak_nanos = non_entity_observation.hud_peak_nanos;
        state.renderer_counters.non_entity_item_nanos = non_entity_observation.item_nanos;
        state.renderer_counters.non_entity_item_peak_nanos = non_entity_observation.item_peak_nanos;
        state.renderer_counters.item_instance_extraction_nanos = non_entity_observation.item_nanos;
        state.renderer_counters.item_instance_extraction_peak_nanos =
            non_entity_observation.item_peak_nanos;
        state.renderer_counters.non_entity_block_entity_nanos =
            non_entity_observation.block_entity_nanos;
        state.renderer_counters.non_entity_block_entity_peak_nanos =
            non_entity_observation.block_entity_peak_nanos;
        state.renderer_counters.non_entity_world_debug_nanos =
            non_entity_observation.world_debug_nanos;
        state.renderer_counters.non_entity_world_debug_peak_nanos =
            non_entity_observation.world_debug_peak_nanos;
        state.renderer_counters.non_entity_apply_nanos = non_entity_observation.apply_nanos;
        state.renderer_counters.non_entity_apply_peak_nanos =
            non_entity_observation.apply_peak_nanos;
        state
            .renderer_counters
            .non_entity_extraction_capacity_growths = non_entity_observation.capacity_growths;
        state
            .renderer_counters
            .item_extraction_scratch_capacity_growths = non_entity_observation.capacity_growths;
        state.renderer_counters.non_entity_cache_hits = non_entity_observation.cache_hits;
        state.renderer_counters.non_entity_cache_misses = non_entity_observation.cache_misses;
        state.renderer_counters.non_entity_rebuilt_owners = non_entity_observation.rebuilt_owners;
        project_dynamic_texture_counters(&mut state.renderer_counters, item_runtime);
    }

    let completed_gpu_timing = renderer.take_gpu_timing_profile();
    let profiler_frame_metrics = timings.profiler_enabled.then(|| {
        DebugProfilerFrameMetrics::from_renderer(
            Duration::ZERO,
            &state.renderer_counters,
            args.render_distance_chunks,
            renderer.gpu_utilization(),
            completed_gpu_timing,
            renderer_frame_profile,
        )
    });
    let profiler_publish_duration = profiler_publish_started_at.elapsed();
    let profiler_frame_work_duration = timings.profiler_frame_started_at.elapsed();
    state.frame_phase_diagnostics.record(FramePhaseSample {
        frame_interval: renderer_frame_profile.frame_interval,
        frame_work: profiler_frame_work_duration,
        update: timings.profiler_update_duration,
        extract: timings.native_render_profile.extract,
        render_call_total: renderer_frame_profile.render_call_total,
        cpu_encode_before_wait: renderer_frame_profile.cpu_encode_before_wait,
        previous_submission_wait: renderer_frame_profile.previous_submission_wait,
        previous_submission_poll: renderer_frame_profile.previous_submission_poll,
        previous_submission_blocking_wait: renderer_frame_profile.previous_submission_blocking_wait,
        surface_acquire: renderer_frame_profile.surface_acquire,
        presentation_tail_encode: renderer_frame_profile.presentation_tail_encode,
        queue_submit: renderer_frame_profile.queue_submit,
        swap_buffers_present: renderer_frame_profile.swap_buffers_present,
        post_present_fence_submit: renderer_frame_profile.post_present_fence_submit,
        gpu_async_poll: renderer_frame_profile.gpu_async_poll,
        legacy_render_encode: renderer_frame_profile.render,
        publish: profiler_publish_duration,
        retired_submission_render_to_callback: renderer_frame_profile
            .retired_submission_render_to_callback,
        retired_submission_submit_to_callback: renderer_frame_profile
            .retired_submission_submit_to_callback,
        retired_submission_callback_to_wait_return: renderer_frame_profile
            .retired_submission_callback_to_wait_return,
    });

    let (profiler_frame_duration, profiler_frame_limiter_duration) =
        frame_diagnostics::profiler_frame_timing(
            renderer_frame_profile.frame_interval,
            profiler_frame_work_duration,
            state.frame_interval.is_some(),
        );
    if profiler_frame_work_duration > Duration::from_secs(1) {
        tracing::warn!(
            frame_millis = profiler_frame_work_duration.as_millis() as u64,
            update_millis = timings.profiler_update_duration.as_millis() as u64,
            extract_millis = timings.native_render_profile.extract.as_millis() as u64,
            toast_millis = timings.native_render_profile.toasts.as_millis() as u64,
            render_millis = profiler_render_duration.as_millis() as u64,
            publish_millis = profiler_publish_duration.as_millis() as u64,
            "slow frame"
        );
    }

    if let Some(mut profiler_frame_metrics) = profiler_frame_metrics {
        profiler_frame_metrics.tick_time_nanos = profiler_frame_duration.as_nanos() as f64;
        let simulation_generation_changed = state.last_profiled_step != Some(state.step_generation);
        if simulation_generation_changed {
            state.latest_simulation_profile_sections = view.frame.profiling_sections().to_vec();
            state.last_profiled_step = Some(state.step_generation);
        }
        let mut sections = Vec::with_capacity(state.latest_simulation_profile_sections.len() + 16);
        sections.extend(
            state
                .latest_simulation_profile_sections
                .iter()
                .map(|section| DebugProfilerSectionSample {
                    path: section.path,
                    duration: section.duration,
                    record_in_report: simulation_generation_changed,
                }),
        );
        sections.extend([
            DebugProfilerSectionSample {
                path: &["toasts"],
                duration: timings.native_render_profile.toasts,
                record_in_report: true,
            },
            DebugProfilerSectionSample {
                path: &["frame"],
                duration: profiler_frame_duration
                    .saturating_sub(timings.native_render_profile.toasts),
                record_in_report: true,
            },
            DebugProfilerSectionSample {
                path: &["frame", "update"],
                duration: timings.profiler_update_duration,
                record_in_report: true,
            },
            DebugProfilerSectionSample {
                path: &["frame", "extract"],
                duration: timings.native_render_profile.extract,
                record_in_report: true,
            },
            DebugProfilerSectionSample {
                path: &["frame", "surfaceAcquire"],
                duration: renderer_frame_profile.surface_acquire,
                record_in_report: true,
            },
            DebugProfilerSectionSample {
                path: &["frame", "render"],
                duration: renderer_frame_profile.render,
                record_in_report: true,
            },
            DebugProfilerSectionSample {
                path: &["frame", "cpuEncodeBeforeWait"],
                duration: renderer_frame_profile.cpu_encode_before_wait,
                record_in_report: true,
            },
            DebugProfilerSectionSample {
                path: &["frame", "previousSubmissionWait"],
                duration: renderer_frame_profile.previous_submission_wait,
                record_in_report: true,
            },
            DebugProfilerSectionSample {
                path: &["frame", "previousSubmissionPoll"],
                duration: renderer_frame_profile.previous_submission_poll,
                record_in_report: true,
            },
            DebugProfilerSectionSample {
                path: &["frame", "previousSubmissionBlockingWait"],
                duration: renderer_frame_profile.previous_submission_blocking_wait,
                record_in_report: true,
            },
            DebugProfilerSectionSample {
                path: &["frame", "presentationTailEncode"],
                duration: renderer_frame_profile.presentation_tail_encode,
                record_in_report: true,
            },
            DebugProfilerSectionSample {
                path: &["frame", "postPresentFenceSubmit"],
                duration: renderer_frame_profile.post_present_fence_submit,
                record_in_report: true,
            },
            DebugProfilerSectionSample {
                path: &["frame", "renderCallTotal"],
                duration: renderer_frame_profile.render_call_total,
                record_in_report: true,
            },
            DebugProfilerSectionSample {
                path: &["frame", "present"],
                duration: renderer_frame_profile.present,
                record_in_report: true,
            },
            DebugProfilerSectionSample {
                path: &["frame", "swapBuffers"],
                duration: renderer_frame_profile.swap_buffers,
                record_in_report: true,
            },
            DebugProfilerSectionSample {
                path: &["frame", "fpsUpdate"],
                duration: profiler_publish_duration,
                record_in_report: true,
            },
        ]);
        if !renderer_frame_profile.gpu_async.is_zero() {
            sections.push(DebugProfilerSectionSample {
                path: &["frame", "gpuAsync"],
                duration: renderer_frame_profile.gpu_async,
                record_in_report: true,
            });
        }
        if !profiler_frame_limiter_duration.is_zero() {
            sections.push(DebugProfilerSectionSample {
                path: &["frame", "frameLimiter"],
                duration: profiler_frame_limiter_duration,
                record_in_report: true,
            });
        }
        state.debug_profiler.record_frame(
            bbb_client_clock::Instant::now(),
            profiler_frame_duration,
            &sections,
            profiler_frame_metrics,
        );
    }

    #[cfg(target_arch = "wasm32")]
    for (file_name, bytes) in state.debug_profiler.take_browser_downloads() {
        crate::web::trigger_browser_blob_download(&file_name, "application/zip", &bytes);
    }

    for event in state.debug_profiler.take_events() {
        let lifecycle = match event {
            DebugProfilerEvent::RecordingStarted => {
                tracing::info!("client profiling started");
                DebugProfilingLifecycle::Started
            }
            DebugProfilerEvent::RecordingStopped {
                duration,
                ticks,
                ticks_per_second,
            } => {
                tracing::info!(
                    duration_seconds = duration.as_secs_f64(),
                    ticks,
                    ticks_per_second,
                    "client profiling stopped"
                );
                DebugProfilingLifecycle::Stopped {
                    duration,
                    ticks,
                    ticks_per_second,
                }
            }
            DebugProfilerEvent::ReportSaved { path } => {
                tracing::info!(path = %path.display(), "client profiling report saved");
                DebugProfilingLifecycle::ReportSaved { path }
            }
            DebugProfilerEvent::ReportFailed { error } => {
                tracing::warn!(error, "client profiling report failed");
                DebugProfilingLifecycle::ReportFailed { error }
            }
        };
        command_queue.push(SimulationCommand::DebugProfilingLifecycle(lifecycle));
    }

    let previous_interval = state.frame_interval;
    state.frame_interval = client_framerate_limit_interval(effective_framerate_limit(
        args.client_framerate_limit,
        args.inactivity_fps_limit,
        window_minimized,
        state.latest_input_at.clock_elapsed().as_millis(),
        view.world.level_info().is_some(),
        view.input.blocks_world_input(),
    ))
    .filter(|_| !bbb_client_clock::is_virtual());
    if let Some(interval) = state.frame_interval {
        let now = bbb_client_clock::now();
        state.next_redraw_at = advance_next_redraw_at(
            state.next_redraw_at,
            now,
            interval,
            previous_interval.is_none(),
        );
    }
}

/// Advances a frame-capped redraw deadline from its previous scheduled slot so render time does
/// not steadily drag FPS below the configured cap, while clamping if the loop fell behind or the
/// target interval just shrank.
fn advance_next_redraw_at(
    scheduled: bbb_client_clock::Instant,
    now: bbb_client_clock::Instant,
    interval: Duration,
    newly_capped: bool,
) -> bbb_client_clock::Instant {
    if newly_capped {
        return now + interval;
    }
    let next = scheduled + interval;
    if next <= now {
        now
    } else {
        next.min(now + interval)
    }
}

fn finalize_reload_commit(
    ack: ResourceReloadAck,
    pack_roots: Option<&PackRoots>,
    core: &mut SimulationCore,
    state: &mut SessionRenderState,
    renderer: &mut bbb_client_render::Renderer,
) -> [SimulationCommand; 2] {
    core.resynchronize_terrain_textures();
    core.invalidate_content_registry_sync();
    let _ = core.synchronize_content_registry(pack_roots, Some(renderer));
    if let Some(runtime) = &core.particle_runtime {
        core.particles.set_sprites(runtime.sprite_table());
    }
    state.non_entity_render_extraction.invalidate_resources();
    [
        SimulationCommand::TextMeasurements(renderer.hud_text_measurements()),
        SimulationCommand::ResourceReloadAck(Box::new(ack)),
    ]
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn commit_prepared_reload_into_session(
    prepared: PreparedServerResourcePackReload,
    pack_roots: &mut Option<PackRoots>,
    audio_runtime: &mut Option<NativeAudioRuntime>,
    particle_render_runtime: &mut Option<NativeParticleRenderRuntime>,
    core: &mut SimulationCore,
    state: &mut SessionRenderState,
    renderer: &mut bbb_client_render::Renderer,
) -> [SimulationCommand; 2] {
    let ack = commit_prepared_server_resource_pack_reload(
        prepared,
        pack_roots,
        audio_runtime,
        &mut core.audio_counters,
        &mut core.particle_runtime,
        particle_render_runtime,
        &mut core.item_runtime,
        &mut core.terrain_textures,
        &mut state.entity_variant_textures,
        &mut state.terrain_upload,
        &mut state.hud_render_runtime,
        renderer,
    );
    finalize_reload_commit(ack, pack_roots.as_ref(), core, state, renderer)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_virtual_clock_reload_into_session(
    reload: ResourceReloadEffect,
    base_pack_roots: &Option<PackRoots>,
    pack_roots: &mut Option<PackRoots>,
    audio_runtime: &mut Option<NativeAudioRuntime>,
    particle_render_runtime: &mut Option<NativeParticleRenderRuntime>,
    core: &mut SimulationCore,
    state: &mut SessionRenderState,
    renderer: &mut bbb_client_render::Renderer,
    client_locale: &str,
    font_options: FontOptions,
    client_particle_status: bbb_protocol::packets::ClientParticleStatus,
    username: &str,
    command_queue: &CommandQueue,
) {
    let ack = apply_server_resource_pack_reload_effect(
        reload,
        base_pack_roots,
        pack_roots,
        audio_runtime,
        &mut core.audio_counters,
        &mut core.particle_runtime,
        particle_render_runtime,
        &mut core.item_runtime,
        &mut core.terrain_textures,
        &mut state.entity_variant_textures,
        &mut state.terrain_upload,
        &mut state.hud_render_runtime,
        renderer,
        client_locale,
        font_options,
        client_particle_status,
        username,
    );
    for cmd in finalize_reload_commit(ack, pack_roots.as_ref(), core, state, renderer) {
        command_queue.push(cmd);
    }
}

const CONTROL_PUBLICATION_MIN_INTERVAL: Duration = Duration::from_millis(50);

pub(crate) fn should_publish_control_snapshot(
    demand: &control::ControlPublicationDemand,
    has_pending_completions: bool,
    last_published_at: Option<bbb_client_clock::Instant>,
    now: bbb_client_clock::Instant,
) -> bool {
    if demand.take_flush_requested() || has_pending_completions {
        return true;
    }
    if !demand.has_demand_at(now) {
        return false;
    }
    last_published_at
        .is_none_or(|last| now.saturating_duration_since(last) >= CONTROL_PUBLICATION_MIN_INTERVAL)
}

pub(crate) fn publish_step_completion(
    core: &mut SimulationCore,
    outcome: StepOutcome,
    state: &mut SessionRenderState,
    audio_runtime: Option<&NativeAudioRuntime>,
    snapshot: &SharedSnapshot,
) -> Option<u64> {
    let published_generation = if state.control_views_enabled {
        state.pending_world_publication |= outcome.world_changed;
        let now = bbb_client_clock::Instant::now();
        if should_publish_control_snapshot(
            &state.publication_demand,
            core.input_runtime.has_pending_control_completions(),
            state.last_snapshot_published_at,
            now,
        ) {
            let rebuild_world = std::mem::take(&mut state.pending_world_publication);
            state.last_snapshot_published_at = Some(now);
            state.publication_demand.clear_dirty();
            state.renderer_counters.recording = state.recording.status();
            let audio_counters = audio_runtime
                .map(NativeAudioRuntime::counters)
                .unwrap_or_else(|| core.audio_counters.clone());
            Some(simulation::publication::publish(
                snapshot,
                core,
                rebuild_world,
                &state.renderer_counters,
                audio_counters,
                &mut state.cached_base_client_ui,
            ))
        } else {
            state.publication_demand.mark_dirty();
            None
        }
    } else {
        None
    };
    core.input_runtime.complete_control_requests(
        published_generation.unwrap_or_else(|| snapshot.load().generation),
    );
    for (completion, result) in state.pending_recording_completions.drain(..) {
        let _ = completion.send(result.map(control::ControlCompletion::Recording));
    }
    bbb_client_clock::replay::set(
        &bbb_client_clock::replay::CLIENT_TICKS,
        core.client_tick_index(),
    );
    bbb_client_clock::replay::flag(&bbb_client_clock::replay::STATE_UNRENDERED, true);
    published_generation
}

pub(crate) fn tick_session_narrator(
    core: &SimulationCore,
    state: &mut SessionRenderState,
    mut narrator: Option<&mut dyn narrator_runtime::SystemNarrator>,
) {
    narrate_player_list_visibility_change(
        hud_player_list_is_visible(&core.input),
        &mut state.player_list_narration_visible,
        narrator
            .as_mut()
            .map(|n| &mut **n as &mut dyn narrator_runtime::SystemNarrator),
        || hud_player_list_narration_from_world(&core.world, core.item_runtime.as_ref()),
    );
    narrate_command_suggestion_change(
        (!core.input.chat_entry_is_active())
            .then(|| core.input.command_suggestion_narration())
            .flatten(),
        &mut state.last_command_suggestion_narration,
        core.item_runtime.as_ref(),
        narrator,
    );
}

pub(crate) fn schedule_next_frame_and_control_flow(
    window: &Window,
    target: &ActiveEventLoop,
    core: &SimulationCore,
    state: &mut SessionRenderState,
    window_hidden: bool,
    poll_when_uncapped: bool,
    poll_reload_interval: Option<Duration>,
) {
    if window_hidden {
        state.redraw_requested = false;
    } else {
        let now = bbb_client_clock::now();
        let redraw_due = state.frame_interval.is_none() || now >= state.next_redraw_at;
        if redraw_due && !state.redraw_requested {
            window.request_redraw();
            state.redraw_requested = if poll_when_uncapped {
                state.frame_interval.is_some()
            } else {
                true
            };
        }
    }
    let control_flow =
        if core.network_work_pending() || (poll_when_uncapped && state.frame_interval.is_none()) {
            ControlFlow::Poll
        } else if let Some(interval) = poll_reload_interval {
            ControlFlow::WaitUntil(
                core.next_tick_deadline
                    .min(bbb_client_clock::now() + interval),
            )
        } else if window_hidden || state.redraw_requested || state.frame_interval.is_none() {
            ControlFlow::WaitUntil(core.next_tick_deadline)
        } else {
            ControlFlow::WaitUntil(core.next_tick_deadline.min(state.next_redraw_at))
        };
    target.set_control_flow(control_flow);
}

pub(crate) fn is_afk_activity_event(event: &Event<()>) -> bool {
    matches!(
        event,
        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { .. }
                | WindowEvent::MouseInput { .. }
                | WindowEvent::MouseWheel { .. }
                | WindowEvent::CursorMoved { .. },
            ..
        } | Event::DeviceEvent {
            event: DeviceEvent::MouseMotion { .. },
            ..
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detached_core_client_asset_loader_runs_off_main_thread_and_applies_to_world() {
        let loader = std::thread::spawn(|| {
            load_core_client_assets_detached(
                bbb_protocol::packets::ClientParticleStatus::All,
                FontOptions {
                    uniform: false,
                    japanese_variants: false,
                },
                None,
                "en_us",
                None,
                false,
            )
        });
        let assets = loader.join().unwrap().unwrap();
        assert!(assets.item_runtime.is_none());
        let mut world = WorldStore::new();
        apply_loaded_core_client_assets_to_world(&assets, &mut world).unwrap();
    }

    #[test]
    fn initial_core_assets_apply_localizes_narrator_status_labels() {
        struct ResetNarratorLabels;
        impl Drop for ResetNarratorLabels {
            fn drop(&mut self) {
                narrator_runtime::update_localized_status_labels(|key| key);
            }
        }
        let _reset_guard = ResetNarratorLabels;

        static NEXT_TEMP_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let seq = NEXT_TEMP_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "bbb-session-narrator-locale-{}-{seq}-{nanos}",
            std::process::id()
        ));
        struct RemoveDir(std::path::PathBuf);
        impl Drop for RemoveDir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _dir_guard = RemoveDir(root.clone());

        crate::item_registry_test_fixture::write_item_registry_fixture(&root, &["stone"]);
        let source_root = root.join("sources").join(bbb_client_assets::MC_VERSION);
        std::fs::create_dir_all(&source_root).unwrap();
        std::fs::write(
            source_root.join("pack.mcmeta"),
            r#"{"pack":{"pack_format":84,"description":"test"},"language":{"fr_fr":{"region":"FR","name":"French","bidirectional":false}}}"#,
        )
        .unwrap();
        let assets_dir = source_root.join("assets").join("minecraft");
        for (segments, contents) in [
            (
                &["atlases", "items.json"][..],
                r#"{"sources":[{"type":"minecraft:directory","prefix":"item/","source":"item"}]}"#,
            ),
            (
                &["atlases", "blocks.json"][..],
                r#"{"sources":[{"type":"minecraft:directory","prefix":"block/","source":"block"}]}"#,
            ),
            (
                &["items", "stone.json"][..],
                r#"{"model":{"type":"minecraft:model","model":"minecraft:item/stone"}}"#,
            ),
            (
                &["models", "item", "stone.json"][..],
                r#"{"textures":{"layer0":"minecraft:item/stone"}}"#,
            ),
            (
                &["lang", "fr_fr.json"][..],
                r#"{"options.narrator":"Narrateur","options.narrator.chat":"Tchat uniquement"}"#,
            ),
        ] {
            let mut path = assets_dir.clone();
            for segment in segments {
                path.push(segment);
            }
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, contents).unwrap();
        }
        let texture = assets_dir.join("textures").join("item").join("stone.png");
        std::fs::create_dir_all(texture.parent().unwrap()).unwrap();
        image::save_buffer(
            texture,
            &[120, 120, 120, 255],
            1,
            1,
            image::ColorType::Rgba8,
        )
        .unwrap();

        let roots = PackRoots::from_root(&root).unwrap();
        let item_runtime =
            NativeItemRuntime::load_with_locale_and_fonts(&roots, "fr_fr", None).unwrap();
        let assets = LoadedCoreClientAssets {
            client_particle_status: bbb_protocol::packets::ClientParticleStatus::All,
            particle_runtime: None,
            particle_render_runtime: None,
            font_options: FontOptions {
                uniform: false,
                japanese_variants: false,
            },
            font_atlas: None,
            item_runtime: Some(item_runtime),
        };
        let mut world = WorldStore::new();
        apply_loaded_core_client_assets_to_world(&assets, &mut world).unwrap();

        assert_eq!(
            narrator_runtime::localized_status_announcement(NarratorStatus::Chat),
            "Narrateur : Tchat uniquement"
        );
    }

    #[test]
    fn frame_cap_pacing_advances_from_scheduled_deadline_and_clamps_when_behind() {
        let start = bbb_client_clock::now();
        let interval = Duration::from_millis(16);

        // Finishing a 4 ms render keeps the exact 16 ms cadence instead of drifting to 20 ms.
        let next = advance_next_redraw_at(start, start + Duration::from_millis(4), interval, false);
        assert_eq!(next, start + Duration::from_millis(16));

        // Slight wakeup jitter (e.g. finished at 21 ms) still aligns to the 32 ms slot.
        let next2 =
            advance_next_redraw_at(next, start + Duration::from_millis(21), interval, false);
        assert_eq!(next2, start + Duration::from_millis(32));

        // Falling behind (frame took 50 ms) clamps to `now` so we do not burst catch-up frames.
        let late = start + Duration::from_millis(82);
        assert_eq!(advance_next_redraw_at(next2, late, interval, false), late);

        // Shrinking from a long inactivity interval clamps to `now + interval`.
        let far_future = start + Duration::from_millis(500);
        let now = start + Duration::from_millis(10);
        assert_eq!(
            advance_next_redraw_at(far_future, now, interval, false),
            now + interval
        );

        // Transitioning from uncapped into capped starts cleanly one interval from `now`.
        assert_eq!(
            advance_next_redraw_at(start, late, interval, true),
            late + interval
        );
    }
}
