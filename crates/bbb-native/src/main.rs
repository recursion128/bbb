use anyhow::{Context, Result};
use bbb_control::{AudioCounters, ControlState, NetCounters};
use bbb_world::{ChatPresentationConfig, ChatVisibilityState, WorldStore};
use std::time::Instant;
use winit::{
    event::{DeviceEvent, ElementState, Event, Ime, WindowEvent},
    event_loop::ControlFlow,
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, CursorIcon, Window},
};

mod audio_runtime;
mod banner_scene;
mod bed_scene;
mod bell_scene;
mod biome_tint;
mod block_outline;
mod camera_pose;
mod chest_scene;
mod code_of_conduct;
mod code_of_conduct_overlay;
mod component_text;
mod conduit_scene;
mod crosshair;
mod debug_entries;
mod debug_profiler;
mod decorated_pot_scene;
mod enchanting_table_book_scene;
mod end_portal_scene;
mod entity_assets;
mod entity_features;
mod entity_name_tags;
mod entity_scene;
mod entity_variant_textures;
mod environment_animate_runtime;
mod environment_audio_runtime;
mod game_test_highlights;
mod hud_assets;
mod input;
mod item_entities;
mod item_frames;
mod item_models;
#[cfg(test)]
mod item_registry_test_fixture;
mod lectern_book_scene;
mod level_load_runtime;
mod map_textures;
mod music_runtime;
mod narrator_runtime;
mod particle_runtime;
mod platform;
mod recipe_book_ui;
mod remote_debug;
mod resource_pack_runtime;
mod resource_reload;
mod runtime;
mod shulker_box_scene;
mod sign_scene;
mod simulation;
mod skull_scene;
mod sky_assets;
mod spawner_scene;
mod startup;
mod terrain_runtime;
mod win_screen_resources;

use audio_runtime::{AudioEventSink, NativeAudioRuntime};
use bbb_item_model::default_player_skin_cache_dir;
use bbb_item_model::NativeItemRuntime;
use code_of_conduct::{default_code_of_conduct_store_path, CodeOfConductAcceptance};
use code_of_conduct_overlay::CodeOfConductOverlayRenderer;
use debug_entries::DebugScreenEntryList;
use debug_profiler::{DebugProfiler, DebugProfilerSectionSample};
use entity_assets::load_entity_model_textures;
use entity_scene::EntityRenderExtractionScratch;
use entity_variant_textures::NativeEntityVariantTextureRuntime;
use hud_assets::load_hud_textures;
use input::{ClientInputState, DebugClipboard};
use level_load_runtime::LevelLoadRuntime;
use music_runtime::MusicRuntime;
use narrator_runtime::{
    NarratorStatus, NativeNarratorRuntime, ScreenNarrationInput, ScreenNarrationRuntime,
};
use particle_runtime::NativeParticleResources;
use remote_debug::{RemoteDebugRendererSelection, RemoteDebugSelectionCache};
use resource_pack_runtime::{default_server_resource_pack_cache_dir, ServerResourcePackRuntime};
use resource_reload::{
    apply_item_runtime_world_defaults, apply_server_resource_pack_reload_effect,
    block_destroy_profiles_for_world, block_sound_profiles_for_world,
    client_resource_pack_diagnostics, item_particle_sprite_uvs,
};
use runtime::{
    control_renderer_counters, dialog_ime_cursor_area, narrate_command_suggestion_change,
    project_dynamic_texture_counters, world_client_tick_interval, CameraFovTickState,
    ClientAnimationTickState, DebugCollisionCache, DebugSkyLightSectionCache,
    DebugSupportingBlockCache, HudDebugFpsSampler, HudDebugNetworkSampler, HudDebugTpsSampler,
    HudRenderRuntime, LevelEventSoundRandomState, LightmapTickState,
};
use simulation::{
    commands::{SimulationCommand, SimulationMailbox},
    effects::{
        ParticleBatchSink, PlatformEffect, PlatformEffectSink, RenderEffectConsumer,
        ScreenNarrationScheduleInput,
    },
    publication::{SimulationPublicationInput, SimulationPublisher},
    thread::SimulationThread,
    update_renderer_from_publication, SimulationCore, SimulationRenderContext,
};
use sky_assets::load_sky_textures;
use startup::{
    build_window, client_framerate_limit_interval, create_event_loop, init_tracing,
    load_known_packs_manager, load_pack_roots, parse_args, run_probe_if_requested,
    run_status_if_requested, spawn_frame_tick, start_control_api, start_network_if_requested,
    NetworkHandles,
};
use terrain_runtime::{load_terrain_textures, TerrainUploadState};
use win_screen_resources::WinScreenResources;

impl input::DebugOptionsSearchTextMeasurer for bbb_renderer::Renderer {
    fn debug_options_search_display_start_for_width(
        &self,
        search_text: &str,
        scroll_to: usize,
        width: i32,
    ) -> usize {
        let width = u32::try_from(width.max(0)).unwrap_or(u32::MAX);
        self.hud_plain_text_display_start_for_width(search_text, scroll_to, width)
            .unwrap_or_else(|| {
                input::debug_options_search_display_start_for_width_fallback(
                    search_text,
                    scroll_to,
                    i32::try_from(width).unwrap_or(i32::MAX),
                )
            })
    }

    fn debug_options_search_cursor_for_text_offset_from_display_start(
        &self,
        search_text: &str,
        display_start: usize,
        offset: i32,
    ) -> usize {
        let offset = offset.max(0);
        let width = u32::try_from(offset).unwrap_or(u32::MAX);
        self.hud_plain_text_cursor_for_width_from(search_text, display_start, width)
            .unwrap_or_else(|| {
                input::debug_options_search_cursor_for_text_offset_from_display_start_fallback(
                    search_text,
                    display_start,
                    offset,
                )
            })
    }
}

#[derive(Default)]
struct NativeDebugClipboard {
    clipboard: Option<arboard::Clipboard>,
}

impl NativeDebugClipboard {
    fn clipboard_mut(&mut self) -> Option<&mut arboard::Clipboard> {
        if self.clipboard.is_none() {
            match arboard::Clipboard::new() {
                Ok(clipboard) => self.clipboard = Some(clipboard),
                Err(err) => {
                    tracing::warn!(?err, "clipboard unavailable");
                    return None;
                }
            }
        }
        self.clipboard.as_mut()
    }
}

impl DebugClipboard for NativeDebugClipboard {
    fn set_debug_clipboard_text(&mut self, text: &str) -> bool {
        let Some(clipboard) = self.clipboard_mut() else {
            return false;
        };
        if let Err(err) = clipboard.set_text(text.to_string()) {
            tracing::warn!(?err, "failed to set clipboard text");
            return false;
        }
        true
    }

    fn get_debug_clipboard_text(&mut self) -> Option<String> {
        let clipboard = self.clipboard_mut()?;
        match clipboard.get_text() {
            Ok(text) => Some(text),
            Err(err) => {
                tracing::warn!(?err, "failed to get clipboard text");
                None
            }
        }
    }
}

struct NativeRenderEffectSink<'a> {
    renderer: &'a mut bbb_renderer::Renderer,
    window: &'a Window,
    cursor_captured: &'a mut bool,
    mailbox: &'a SimulationMailbox,
    clipboard: &'a mut NativeDebugClipboard,
    terrain_upload: &'a mut TerrainUploadState,
    debug_profiler: &'a mut DebugProfiler,
    stats_screen_narration: &'a mut ScreenNarrationRuntime,
    death_screen_narration: &'a mut ScreenNarrationRuntime,
    game_event_screen_narration: &'a mut ScreenNarrationRuntime,
    narrator: Option<&'a mut dyn narrator_runtime::SystemNarrator>,
    control_screenshots: &'a mut std::collections::VecDeque<std::path::PathBuf>,
    exit_requested: &'a mut bool,
}

impl ParticleBatchSink for NativeRenderEffectSink<'_> {
    fn submit_particle_batch(&mut self, batch: bbb_renderer::ParticleSpawnBatch) {
        self.renderer.submit_particle_spawns(batch);
    }
}

impl PlatformEffectSink for NativeRenderEffectSink<'_> {
    fn apply_platform_effect(&mut self, effect: PlatformEffect) {
        match effect {
            PlatformEffect::SetCursorCapture(capture) => {
                set_cursor_capture(self.window, self.cursor_captured, capture);
                self.mailbox
                    .enqueue(SimulationCommand::CursorCaptureResult {
                        captured: *self.cursor_captured,
                    });
            }
            PlatformEffect::ClipboardWrite(text) => {
                let success = self.clipboard.set_debug_clipboard_text(&text);
                self.mailbox
                    .enqueue(SimulationCommand::ClipboardWriteResult { success });
            }
            PlatformEffect::OpenUri(url) => {
                let success = platform::open_uri(&url)
                    .map_err(|err| tracing::warn!(?err, %url, "failed to open dialog URI"))
                    .is_ok();
                self.mailbox
                    .enqueue(SimulationCommand::OpenUriResult { url, success });
            }
            PlatformEffect::Screenshot(path) => self.control_screenshots.push_back(path),
            PlatformEffect::ReloadTerrain => self.terrain_upload.request_reload_all_chunks(),
            PlatformEffect::DumpDynamicTextures { requests } => {
                let path = std::path::Path::new(input::DEBUG_DYNAMIC_TEXTURE_DUMP_RELATIVE_PATH);
                match self.renderer.dump_dynamic_textures(path) {
                    Ok(summary) => tracing::info!(
                        requests,
                        files_written = summary.files_written,
                        dynamic_player_skin_atlas = summary.dynamic_player_skin_atlas,
                        dynamic_player_texture_atlas = summary.dynamic_player_texture_atlas,
                        path = %path.display(),
                        "dumped dynamic textures requested by debug hotkey"
                    ),
                    Err(err) => tracing::warn!(
                        ?err,
                        requests,
                        path = %path.display(),
                        "failed to dump dynamic textures requested by debug hotkey"
                    ),
                }
            }
            PlatformEffect::ProfilingToggled {
                requests,
                starts,
                stops,
            } => {
                if stops > 0 {
                    let path = std::path::Path::new(input::DEBUG_PROFILING_RESULTS_RELATIVE_DIR);
                    if let Err(err) = std::fs::create_dir_all(path) {
                        tracing::warn!(?err, path = %path.display(), "failed to create profiling feedback directory");
                    }
                }
                tracing::info!(
                    requests,
                    starts,
                    stops,
                    "profiling toggled by debug hotkey; native profiler output is not implemented"
                );
            }
            PlatformEffect::NavigateProfiler { digits } => {
                let requests = digits.len();
                let last_digit = digits.last().copied();
                for digit in digits {
                    self.debug_profiler.navigate(digit);
                }
                if let Some(last_digit) = last_digit {
                    tracing::info!(
                        requests,
                        last_digit,
                        current_path = self.debug_profiler.current_path(),
                        "profiler chart navigation requested"
                    );
                }
            }
            PlatformEffect::StatsScreenInput(input) => {
                let input = match input {
                    ScreenNarrationScheduleInput::MouseMove => ScreenNarrationInput::MouseMove,
                    ScreenNarrationScheduleInput::MouseAction => ScreenNarrationInput::MouseAction,
                    ScreenNarrationScheduleInput::KeyboardAction => {
                        ScreenNarrationInput::KeyboardAction
                    }
                };
                self.stats_screen_narration
                    .after_input(input, Instant::now());
            }
            PlatformEffect::GameEventScreenInput(input) => {
                let input = match input {
                    ScreenNarrationScheduleInput::MouseMove => ScreenNarrationInput::MouseMove,
                    ScreenNarrationScheduleInput::MouseAction => ScreenNarrationInput::MouseAction,
                    ScreenNarrationScheduleInput::KeyboardAction => {
                        ScreenNarrationInput::KeyboardAction
                    }
                };
                self.game_event_screen_narration
                    .after_input(input, Instant::now());
            }
            PlatformEffect::DeathScreenInput(input) => {
                let input = match input {
                    ScreenNarrationScheduleInput::MouseMove => ScreenNarrationInput::MouseMove,
                    ScreenNarrationScheduleInput::MouseAction => ScreenNarrationInput::MouseAction,
                    ScreenNarrationScheduleInput::KeyboardAction => {
                        ScreenNarrationInput::KeyboardAction
                    }
                };
                self.death_screen_narration
                    .after_input(input, Instant::now());
            }
            PlatformEffect::ChatNarration { kind, text } => {
                let Some(narrator) = self.narrator.as_deref_mut() else {
                    return;
                };
                narrator_runtime::submit_chat_narration(narrator, kind, &text);
            }
            PlatformEffect::Exit => *self.exit_requested = true,
        }
    }
}

fn main() -> Result<()> {
    init_tracing();
    let args = parse_args();
    let runtime = tokio::runtime::Runtime::new()?;

    if run_status_if_requested(&runtime, &args)? {
        return Ok(());
    }
    if run_probe_if_requested(&runtime, &args)? {
        return Ok(());
    }

    let mut pack_roots = load_pack_roots(&args)?;
    let base_pack_roots = pack_roots.clone();
    let known_packs_manager = load_known_packs_manager(pack_roots.as_ref());
    let control = ControlState::new(format!(
        "bbb-native {} / protocol {}",
        bbb_protocol::MC_VERSION,
        bbb_protocol::PROTOCOL_VERSION
    ));
    let snapshot = control.snapshot.clone();
    let published_world = control.world.clone();
    let simulation_mailbox = SimulationMailbox::default();
    control
        .requests
        .install_sink(std::sync::Arc::new(simulation_mailbox.clone()));
    let mut world = WorldStore::new();
    world.set_dialog_link_options(args.client_chat_links, args.client_chat_links_prompt);
    world.set_chat_presentation_config(ChatPresentationConfig {
        visibility: ChatVisibilityState::from(bbb_protocol::packets::ClientChatVisibility::from(
            args.client_chat_visibility,
        )),
        only_show_secure: args.client_only_show_secure_chat,
        hide_matched_names: args.client_hide_matched_names,
        message_delay_millis: (args.client_chat_delay_seconds * 1_000.0) as u64,
        hidden_players: Vec::new(),
        blocked_players: Vec::new(),
    });
    let mut net_counters = NetCounters::default();
    net_counters.client_resources =
        std::sync::Arc::new(client_resource_pack_diagnostics(pack_roots.as_ref()));
    let code_of_conduct_store_path = args
        .code_of_conduct_store
        .clone()
        .unwrap_or_else(default_code_of_conduct_store_path);
    let mut code_of_conduct_acceptance =
        match CodeOfConductAcceptance::load(code_of_conduct_store_path.clone()) {
            Ok(store) => store,
            Err(err) => {
                tracing::warn!(
                    ?err,
                    path = %code_of_conduct_store_path.display(),
                    "continuing with empty code-of-conduct acceptance store"
                );
                CodeOfConductAcceptance::empty(code_of_conduct_store_path)
            }
        };
    let NetworkHandles {
        events: net_events,
        commands: net_commands,
    } = start_network_if_requested(
        &runtime,
        &args,
        &mut code_of_conduct_acceptance,
        known_packs_manager,
        &simulation_mailbox,
    )?;
    let profile_id = args
        .profile_id
        .unwrap_or_else(|| bbb_protocol::codec::offline_player_uuid(&args.username));
    world.set_local_player_profile_id(profile_id);
    let server_resource_pack_runtime = ServerResourcePackRuntime::new(
        runtime.handle().clone(),
        args.server_resource_pack_cache_dir
            .clone()
            .unwrap_or_else(default_server_resource_pack_cache_dir),
        args.username.clone(),
        profile_id,
    );
    start_control_api(&runtime, args.control, &control);
    let (mut audio_runtime, mut audio_status) = match pack_roots.as_ref() {
        Some(roots) => match NativeAudioRuntime::load(roots) {
            Ok(runtime) => {
                let status = runtime.counters();
                (Some(runtime), status)
            }
            Err(err) => {
                tracing::warn!(?err, "continuing without native audio runtime");
                (None, AudioCounters::disabled(err.to_string()))
            }
        },
        None => (
            None,
            AudioCounters::disabled("pack roots unavailable for native audio runtime"),
        ),
    };
    let narrator_status = NarratorStatus::from(args.client_narrator);
    world.set_available_command_sounds(
        audio_runtime
            .as_ref()
            .map(NativeAudioRuntime::available_sound_ids)
            .unwrap_or_default(),
    );
    let mut narrator_runtime = args.narrator_command.clone().map(|command| {
        NativeNarratorRuntime::new(narrator_status, command, args.narrator_command_args.clone())
    });
    if narrator_runtime.is_none() && narrator_status != NarratorStatus::Off {
        tracing::warn!(
            ?narrator_status,
            "narrator requested without --narrator-command; continuing without TTS output"
        );
    }
    let client_particle_status = args.client_particles.into();
    let (mut particle_runtime, mut particle_render_runtime) = pack_roots
        .as_ref()
        .and_then(|roots| {
            NativeParticleResources::load(roots, client_particle_status)
                .map_err(|err| {
                    tracing::warn!(?err, "continuing without native particle runtime");
                    err
                })
                .ok()
        })
        .map(|resources| (Some(resources.simulation), Some(resources.render)))
        .unwrap_or((None, None));
    let mut item_runtime = pack_roots.as_ref().and_then(|roots| {
        NativeItemRuntime::load_with_locale(roots, &args.client_locale)
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native item asset runtime");
                err
            })
            .ok()
    });
    let player_skin_cache_dir = args
        .player_skin_cache_dir
        .clone()
        .unwrap_or_else(default_player_skin_cache_dir);
    if let Some(items) = &item_runtime {
        items.enable_http_profile_resolution();
        items.enable_http_player_skin_downloads(player_skin_cache_dir.clone());
        apply_item_runtime_world_defaults(items, &mut world);
        let (atlas_width, atlas_height) = items.atlas_size();
        let missingno_index = items.texture_index("minecraft:missingno");
        tracing::info!(
            item_definitions = items.item_definition_count(),
            item_registry_entries = items.item_registry_count(),
            resolved_models = items.resolved_model_count(),
            textures = items.texture_count(),
            icon_textures = items.icon_texture_count(),
            item_equipment_slots = items.item_equipment_slot_count(),
            mount_body_armor_kinds = items.mount_body_armor_kind_count(),
            nautilus_body_armor_materials = items.nautilus_body_armor_material_count(),
            horse_body_armor_materials = items.horse_body_armor_material_count(),
            wolf_body_armor_materials = items.wolf_body_armor_material_count(),
            default_attack_range_items = items.item_attack_range_count(),
            default_swing_animation_items = items.item_swing_animation_duration_count(),
            default_piercing_weapon_items = items.default_piercing_weapon_item_count(),
            default_item_max_damage = items.item_max_damage_count(),
            default_damageable_items = items.default_damageable_item_count(),
            item_crafting_remainders = items.item_crafting_remainder_count(),
            recipe_specific_crafting_remainder_items =
                items.recipe_specific_crafting_remainder_item_count(),
            default_use_effect_items = items.item_use_effect_count(),
            item_mining_profiles = items.item_mining_profile_count(),
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
    if let Some(roots) = &pack_roots {
        match roots.load_block_destroy_profile_catalog() {
            Ok(catalog) => {
                world
                    .set_default_block_destroy_profiles(block_destroy_profiles_for_world(&catalog));
                tracing::info!(
                    block_destroy_profiles = catalog.len(),
                    "loaded block destroy profiles"
                );
            }
            Err(err) => {
                tracing::warn!(?err, "continuing without block destroy profiles");
            }
        }
        match roots.load_block_sound_profile_catalog() {
            Ok(catalog) => {
                world.set_default_block_sound_profiles(block_sound_profiles_for_world(&catalog));
                tracing::info!(
                    block_sound_profiles = catalog.len(),
                    "loaded block sound profiles"
                );
            }
            Err(err) => {
                tracing::warn!(?err, "continuing without block sound profiles");
            }
        }
    }

    let event_loop = create_event_loop()?;
    let window = build_window(&event_loop)?;
    window.set_ime_allowed(true);
    let mut input = ClientInputState::new(window.has_focus());
    input.set_client_text_filtering_enabled(args.client_text_filtering);
    let startup_debug_profile = args.debug_profile.into();
    if let Some(path) = args.debug_profile_store.clone() {
        input.set_debug_profile_store_path(path.clone());
        match DebugScreenEntryList::load_from_debug_profile_file(&path, startup_debug_profile) {
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
    input.set_debug_advanced_item_tooltips(args.advanced_item_tooltips);
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
    let mut debug_clipboard = NativeDebugClipboard::default();
    spawn_frame_tick(&event_loop);

    let mut renderer = pollster::block_on(bbb_renderer::Renderer::new_with_vsync(
        &window,
        args.client_vsync,
    ))?;
    renderer.set_lightmap_brightness_factor(args.client_gamma);
    renderer.set_menu_background_blurriness(args.menu_background_blurriness);
    let mut terrain_textures = load_terrain_textures(&mut renderer, pack_roots.as_ref());
    if let Some(particles) = particle_runtime.as_mut() {
        particles.set_terrain_particle_sprite_ids(&terrain_textures);
        if let Some(items) = &item_runtime {
            particles.set_default_item_particle_sprite_ids(items);
        }
    }
    load_hud_textures(&mut renderer, pack_roots.as_ref());
    let win_screen_resources = std::sync::Arc::new(
        pack_roots
            .as_ref()
            .map(|roots| {
                WinScreenResources::load(roots, &args.username, &renderer.hud_text_measurements())
            })
            .transpose()?
            .unwrap_or_default(),
    );
    simulation_mailbox.enqueue(SimulationCommand::TextMeasurements(
        renderer.hud_text_measurements(),
    ));
    load_entity_model_textures(&mut renderer, pack_roots.as_ref());
    load_sky_textures(&mut renderer, pack_roots.as_ref());
    if let Some(particles) = &particle_render_runtime {
        if let Err(err) = particles.upload_particle_atlas(&mut renderer) {
            tracing::warn!(?err, "continuing without native particle atlas");
        }
    }
    if let Some(items) = &item_runtime {
        let (atlas_width, atlas_height) = items.atlas_size();
        if let Err(err) =
            renderer.upload_hud_item_atlas(atlas_width, atlas_height, items.atlas_rgba())
        {
            tracing::warn!(?err, "continuing without native item HUD atlas");
        }
        match renderer.upload_item_entity_atlas(atlas_width, atlas_height, items.atlas_rgba()) {
            Ok(()) => renderer.set_item_particle_sprite_uvs(item_particle_sprite_uvs(items)),
            Err(err) => {
                tracing::warn!(?err, "continuing without native item entity atlas");
            }
        }
    }
    if let Some(roots) = pack_roots.as_ref() {
        roots
            .ensure_resource_data_admitted()
            .context("admit startup resource data generation")?;
    }
    net_counters.client_resources =
        std::sync::Arc::new(client_resource_pack_diagnostics(pack_roots.as_ref()));
    let mut entity_variant_textures = NativeEntityVariantTextureRuntime::new(pack_roots.clone());
    let mut entity_render_extraction = EntityRenderExtractionScratch::default();
    let mut non_entity_render_extraction = runtime::NonEntityRenderExtractionScratch::default();
    let mut screenshot = args.screenshot;
    let screenshot_after_terrain = args.connect_server;
    let exit_after_screenshot = args.exit_after_screenshot;
    let mut terrain_upload = TerrainUploadState::default();
    let client_animation_ticks = ClientAnimationTickState::default();
    let camera_fov_ticks = CameraFovTickState::default();
    let camera_spinning_ticks = runtime::CameraSpinningEffectTickState::default();
    let mut hud_debug_fps_sampler = HudDebugFpsSampler::default();
    let hud_debug_network_sampler = HudDebugNetworkSampler::default();
    let hud_debug_tps_sampler = HudDebugTpsSampler::default();
    let mut debug_collision_cache = DebugCollisionCache::default();
    let mut debug_supporting_block_cache = DebugSupportingBlockCache::default();
    let mut debug_sky_light_section_cache = DebugSkyLightSectionCache::default();
    let mut remote_debug_selection_cache = RemoteDebugSelectionCache::default();
    let mut debug_profiler = DebugProfiler::default();
    let lightmap_ticks = LightmapTickState::with_brightness_factor_and_hide_lightning_flash(
        args.client_gamma,
        args.hide_lightning_flash,
    );
    let level_event_sound_random = LevelEventSoundRandomState::with_seed(world.next_sound_seed());
    let mut cursor_captured = false;
    let frame_interval = client_framerate_limit_interval(args.client_framerate_limit);
    let mut next_redraw_at = Instant::now();
    let mut last_command_suggestion_narration = None;
    let mut stats_screen_narration = ScreenNarrationRuntime::default();
    let mut death_screen_narration = ScreenNarrationRuntime::default();
    let mut game_event_screen_narration = ScreenNarrationRuntime::default();
    let mut pending_control_screenshots = std::collections::VecDeque::new();
    let mut platform_exit_requested = false;
    let mut render_effect_consumer = RenderEffectConsumer::default();
    let mut hud_render_runtime = HudRenderRuntime::default();
    let mut code_of_conduct_overlay_renderer = CodeOfConductOverlayRenderer::default();
    let simulation_input_runtime =
        simulation::input_commands::SimulationInputRuntime::new(window.inner_size(), false);
    let mut renderer_counters = control_renderer_counters(renderer.counters());
    project_dynamic_texture_counters(&mut renderer_counters, item_runtime.as_ref());
    let publisher = SimulationPublisher::new(
        SimulationPublicationInput {
            effect_sequence: 0,
            world: &world,
            input: &input,
            win_screen_resources: std::sync::Arc::clone(&win_screen_resources),
            input_runtime: simulation_input_runtime.publication(),
            frame: None,
            client_tick_index: lightmap_ticks.client_tick_count(),
            tick_interval: world_client_tick_interval(&world),
            tick_residual: client_animation_ticks.entity_partial_tick(),
            tick_anchor: Instant::now(),
            running: true,
            code_of_conduct_accepted: false,
            net_counters: &net_counters,
            renderer_counters: &renderer_counters,
            audio_counters: &audio_status,
            camera_fov_ticks,
            hud_debug_network_sampler: &hud_debug_network_sampler,
            hud_debug_tps_sampler: &hud_debug_tps_sampler,
        },
        snapshot.clone(),
        published_world.clone(),
    );
    let shared_publication = publisher.shared();
    let simulation_item_runtime = item_runtime
        .as_ref()
        .map(NativeItemRuntime::independent_simulation_projection);
    let simulation = SimulationCore {
        net_events,
        net_commands,
        server_resource_packs: server_resource_pack_runtime,
        input,
        world,
        net_counters,
        client_animation_ticks,
        camera_fov_ticks,
        camera_spinning_ticks,
        hud_debug_network_sampler,
        hud_debug_tps_sampler,
        lightmap_ticks,
        level_event_sound_random,
        environment_audio: environment_audio_runtime::EnvironmentAudioRuntime::default(),
        environment_animate: environment_animate_runtime::EnvironmentAnimateRuntime::default(),
        music: MusicRuntime::default(),
        win_screen_resources,
        particle_runtime,
        terrain_textures: terrain_textures.clone(),
        item_runtime: simulation_item_runtime,
        code_of_conduct: code_of_conduct_acceptance,
        command_mailbox: simulation_mailbox.clone(),
        input_runtime: simulation_input_runtime,
        level_load_runtime: LevelLoadRuntime::default(),
        renderer_counters,
        audio_counters: audio_status.clone(),
        publisher,
        audio_enabled: audio_runtime.is_some(),
        render_distance_chunks: args.render_distance_chunks,
        hide_lightning_flash: args.hide_lightning_flash,
        next_effect_sequence: 1,
    };
    let simulation_event_proxy = event_loop.create_proxy();
    let mut simulation_thread = SimulationThread::spawn(simulation, move || {
        let _ = simulation_event_proxy.send_event(());
    })?;
    let mut render_publication = shared_publication.load_full();
    let mut event_cursor_position = None;

    event_loop.run(move |event, target| {
        if matches!(&event, Event::LoopExiting) {
            set_cursor_capture(&window, &mut cursor_captured, false);
            if let Err(error) = simulation_thread.shutdown_and_join() {
                tracing::error!(%error, "failed to join simulation thread during shutdown");
            }
            snapshot.rcu(|current| {
                let mut next = (**current).clone();
                next.app.running = false;
                next.net.connected = false;
                next
            });
            return;
        }
        let mut resource_reloads = Vec::new();
        loop {
            let simulation_step = match simulation_thread.try_recv() {
                Ok(step) => step,
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
            };
            let render_effect_result = {
                let mut render_effect_sink = NativeRenderEffectSink {
                    renderer: &mut renderer,
                    window: &window,
                    cursor_captured: &mut cursor_captured,
                    mailbox: &simulation_mailbox,
                    clipboard: &mut debug_clipboard,
                    terrain_upload: &mut terrain_upload,
                    debug_profiler: &mut debug_profiler,
                    stats_screen_narration: &mut stats_screen_narration,
                    death_screen_narration: &mut death_screen_narration,
                    game_event_screen_narration: &mut game_event_screen_narration,
                    narrator: narrator_runtime
                        .as_mut()
                        .map(|narrator| narrator as &mut dyn narrator_runtime::SystemNarrator),
                    control_screenshots: &mut pending_control_screenshots,
                    exit_requested: &mut platform_exit_requested,
                };
                simulation_step.consume_render_effects(
                    &mut render_effect_consumer,
                    audio_runtime
                        .as_mut()
                        .map(|runtime| runtime as &mut dyn AudioEventSink),
                    &mut render_effect_sink,
                )
            };
            match render_effect_result {
                Ok(Some(reload)) => resource_reloads.push(reload),
                Ok(None) => {}
                Err(error) => {
                    tracing::error!(%error, "fatal simulation render-effect sequence gap");
                    target.exit();
                    return;
                }
            }
        }
        for resource_reload in resource_reloads {
            let ack = apply_server_resource_pack_reload_effect(
                resource_reload,
                &base_pack_roots,
                &mut pack_roots,
                &mut audio_runtime,
                &mut audio_status,
                &mut particle_render_runtime,
                &mut item_runtime,
                &mut terrain_textures,
                &mut entity_variant_textures,
                &mut terrain_upload,
                &mut renderer,
                &args.client_locale,
                client_particle_status,
                &player_skin_cache_dir,
                &args.username,
            );
            non_entity_render_extraction.invalidate_resources();
            simulation_mailbox.enqueue(SimulationCommand::TextMeasurements(
                renderer.hud_text_measurements(),
            ));
            simulation_mailbox.enqueue(SimulationCommand::ResourceReloadAck(ack));
        }
        let latest_publication = shared_publication.load_full();
        if latest_publication.effect_sequence <= render_effect_consumer.last_sequence() {
            render_publication = latest_publication;
        }
        if let Some(result) = simulation_thread.poll_finished() {
            if let Err(error) = result {
                tracing::error!(%error, "simulation thread failed");
            }
            target.exit();
            return;
        }
        if platform_exit_requested || !render_publication.running {
            target.exit();
            return;
        }
        if let Some(narrator) = narrator_runtime.as_mut() {
            narrator.tick();
        }
        narrate_command_suggestion_change(
            render_publication.input.command_suggestion_narration(),
            &mut last_command_suggestion_narration,
            item_runtime.as_ref(),
            narrator_runtime
                .as_mut()
                .map(|narrator| narrator as &mut dyn narrator_runtime::SystemNarrator),
        );
        target.set_control_flow(
            frame_interval
                .map(|_| ControlFlow::WaitUntil(next_redraw_at))
                .unwrap_or(ControlFlow::Poll),
        );
        match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => match event {
                WindowEvent::Resized(size) => {
                    renderer.resize(size);
                    simulation_mailbox.enqueue(SimulationCommand::SurfaceResized(size));
                }
                event
                    if enqueue_simulation_window_event(
                        &event,
                        &simulation_mailbox,
                        &mut debug_clipboard,
                        &renderer,
                        &window,
                        &mut event_cursor_position,
                    ) => {}
                WindowEvent::RedrawRequested => {
                    let profiler_frame_started_at = Instant::now();
                    let dynamic_texture_resource_generation = item_runtime
                        .as_ref()
                        .map_or(0, NativeItemRuntime::dynamic_texture_resource_generation);
                    if let Some(items) = item_runtime.as_ref() {
                        items.synchronize_dynamic_texture_session(
                            render_publication.net_counters.connection_generation,
                            render_publication.net_counters.connected,
                        );
                        items.drain_profile_resolution_results();
                    }
                    renderer.synchronize_dynamic_entity_texture_generation(
                        dynamic_texture_resource_generation,
                        render_publication.net_counters.connection_generation,
                        render_publication.net_counters.connected,
                    );
                    if let Some(items) = item_runtime.as_ref() {
                        drain_dynamic_player_skin_downloads(items, &mut renderer);
                    }
                    let profiler_update_started_at = Instant::now();
                    if render_publication.frame.is_none() {
                        window.request_redraw();
                        return;
                    }
                    update_renderer_from_publication(
                        &render_publication,
                        SimulationRenderContext {
                            audio_events: audio_runtime
                                .as_mut()
                                .map(|runtime| runtime as &mut dyn AudioEventSink),
                            renderer: &mut renderer,
                            mailbox: &simulation_mailbox,
                            terrain_upload: &mut terrain_upload,
                            terrain_textures: &mut terrain_textures,
                            entity_variant_textures: &mut entity_variant_textures,
                            entity_render_extraction: &mut entity_render_extraction,
                            non_entity_render_extraction: &mut non_entity_render_extraction,
                            item_runtime: item_runtime.as_ref(),
                            hud_render_runtime: &mut hud_render_runtime,
                            hud_debug_fps_sampler: &mut hud_debug_fps_sampler,
                            debug_collision_cache: &mut debug_collision_cache,
                            debug_supporting_block_cache: &mut debug_supporting_block_cache,
                            debug_sky_light_section_cache: &mut debug_sky_light_section_cache,
                            remote_debug_selection_cache: &mut remote_debug_selection_cache,
                            render_distance_chunks: args.render_distance_chunks,
                            client_framerate_limit: args.client_framerate_limit,
                            client_vsync: args.client_vsync,
                            damage_tilt_strength: args.damage_tilt_strength,
                            entity_shadows: args.entity_shadows,
                            hud_debug_profiler_chart: debug_profiler.chart(),
                        },
                    );
                    if let Some(particles) = particle_render_runtime.as_mut() {
                        particles.maybe_upload_particle_atlas_animation(&mut renderer);
                    }
                    stats_screen_narration.tick(
                        render_publication
                            .world
                            .stats_screen_is_open()
                            .then(|| hud_render_runtime.stats_screen_narration())
                            .flatten(),
                        Instant::now(),
                        narrator_runtime
                            .as_mut()
                            .map(|narrator| narrator as &mut dyn narrator_runtime::SystemNarrator),
                    );
                    game_event_screen_narration.tick(
                        (render_publication.world.death_screen().is_none()
                            && render_publication.world.game_event_screen().is_some())
                        .then(|| hud_render_runtime.game_event_screen_narration())
                        .flatten(),
                        Instant::now(),
                        narrator_runtime
                            .as_mut()
                            .map(|narrator| narrator as &mut dyn narrator_runtime::SystemNarrator),
                    );
                    death_screen_narration.tick(
                        render_publication
                            .world
                            .death_screen()
                            .is_some()
                            .then(|| hud_render_runtime.death_screen_narration())
                            .flatten(),
                        Instant::now(),
                        narrator_runtime
                            .as_mut()
                            .map(|narrator| narrator as &mut dyn narrator_runtime::SystemNarrator),
                    );
                    let dialog_preedit = render_publication.input.dialog_ime_preedit_hud_state();
                    let sign_preedit_area =
                        renderer.hud_sign_editor_ime_cursor_area(window.inner_size());
                    let dialog_preedit_area = || {
                        dialog_ime_cursor_area(
                            &render_publication.world,
                            dialog_preedit.as_ref(),
                            window.inner_size(),
                            &renderer,
                        )
                    };
                    if let Some((position, size)) = sign_preedit_area.or_else(dialog_preedit_area) {
                        window.set_ime_cursor_area(position, size);
                    }
                    let profiler_update_duration = profiler_update_started_at.elapsed();
                    code_of_conduct_overlay_renderer.update_renderer(
                        &mut renderer,
                        &render_publication.world,
                        render_publication.input_runtime.code_of_conduct_overlay,
                        render_publication.code_of_conduct_accepted,
                    );

                    let terrain_ready_for_screenshot =
                        !screenshot_after_terrain || terrain_upload.has_uploaded_chunks();
                    let cli_screenshot_path = screenshot
                        .as_deref()
                        .filter(|_| terrain_ready_for_screenshot);
                    let control_screenshot_path = if cli_screenshot_path.is_none() {
                        pending_control_screenshots.pop_front()
                    } else {
                        None
                    };
                    let render_path = cli_screenshot_path.or(control_screenshot_path.as_deref());
                    let wrote_cli_screenshot = cli_screenshot_path.is_some();

                    let profiler_render_started_at = Instant::now();
                    if let Err(err) = renderer.render(render_path) {
                        tracing::error!(?err, "render failed");
                        target.exit();
                        return;
                    }
                    let profiler_render_duration = profiler_render_started_at.elapsed();

                    if wrote_cli_screenshot {
                        screenshot = None;
                        if exit_after_screenshot {
                            target.exit();
                        }
                    }

                    let profiler_publish_started_at = Instant::now();
                    let audio_counters = audio_runtime
                        .as_ref()
                        .map(NativeAudioRuntime::counters)
                        .unwrap_or_else(|| audio_status.clone());
                    let output_backlog = simulation_thread.output_backlog();
                    let mut renderer_counters = control_renderer_counters(renderer.counters());
                    let entity_observation = entity_render_extraction.observation();
                    renderer_counters.entity_render_candidates = entity_observation.candidates;
                    renderer_counters.entity_render_survivors = entity_observation.survivors;
                    renderer_counters.entity_render_projected = entity_observation.projected;
                    renderer_counters.entity_candidate_nanos = entity_observation.candidate_nanos;
                    renderer_counters.entity_candidate_peak_nanos =
                        entity_observation.candidate_peak_nanos;
                    renderer_counters.entity_cull_nanos = entity_observation.cull_nanos;
                    renderer_counters.entity_cull_peak_nanos = entity_observation.cull_peak_nanos;
                    renderer_counters.entity_full_projection_nanos =
                        entity_observation.projection_nanos;
                    renderer_counters.entity_full_projection_peak_nanos =
                        entity_observation.projection_peak_nanos;
                    renderer_counters.entity_model_build_nanos =
                        entity_observation.model_build_nanos;
                    renderer_counters.entity_model_build_peak_nanos =
                        entity_observation.model_build_peak_nanos;
                    renderer_counters.entity_extraction_capacity_growths =
                        entity_observation.capacity_growths;
                    let non_entity_observation = non_entity_render_extraction.observation();
                    renderer_counters.non_entity_hud_nanos = non_entity_observation.hud_nanos;
                    renderer_counters.non_entity_hud_peak_nanos =
                        non_entity_observation.hud_peak_nanos;
                    renderer_counters.non_entity_item_nanos = non_entity_observation.item_nanos;
                    renderer_counters.non_entity_item_peak_nanos =
                        non_entity_observation.item_peak_nanos;
                    renderer_counters.item_instance_extraction_nanos =
                        non_entity_observation.item_nanos;
                    renderer_counters.item_instance_extraction_peak_nanos =
                        non_entity_observation.item_peak_nanos;
                    renderer_counters.non_entity_block_entity_nanos =
                        non_entity_observation.block_entity_nanos;
                    renderer_counters.non_entity_block_entity_peak_nanos =
                        non_entity_observation.block_entity_peak_nanos;
                    renderer_counters.non_entity_world_debug_nanos =
                        non_entity_observation.world_debug_nanos;
                    renderer_counters.non_entity_world_debug_peak_nanos =
                        non_entity_observation.world_debug_peak_nanos;
                    renderer_counters.non_entity_apply_nanos = non_entity_observation.apply_nanos;
                    renderer_counters.non_entity_apply_peak_nanos =
                        non_entity_observation.apply_peak_nanos;
                    renderer_counters.non_entity_extraction_capacity_growths =
                        non_entity_observation.capacity_growths;
                    renderer_counters.item_extraction_scratch_capacity_growths =
                        non_entity_observation.capacity_growths;
                    renderer_counters.non_entity_cache_hits = non_entity_observation.cache_hits;
                    renderer_counters.non_entity_cache_misses = non_entity_observation.cache_misses;
                    renderer_counters.non_entity_rebuilt_owners =
                        non_entity_observation.rebuilt_owners;
                    project_dynamic_texture_counters(&mut renderer_counters, item_runtime.as_ref());
                    simulation_mailbox.update_backend_telemetry(
                        renderer_counters,
                        audio_counters,
                        output_backlog.render_wakes_sent as u64,
                        output_backlog.render_wakes_coalesced as u64,
                        output_backlog.pending as u64,
                        output_backlog.peak as u64,
                    );
                    let profiler_publish_duration = profiler_publish_started_at.elapsed();
                    let profiler_frame_duration = profiler_frame_started_at.elapsed();
                    let newest_publication = shared_publication.load();
                    let publication_generation_lag = newest_publication
                        .generation
                        .saturating_sub(render_publication.generation);
                    let effect_sequence_lag = newest_publication
                        .effect_sequence
                        .saturating_sub(render_effect_consumer.last_sequence());
                    // Simulation now advances independently; this warning
                    // localizes render-owner latency and the measured output,
                    // publication, and ordered-effect backlogs
                    // without implying that canonical input or ticks stalled.
                    if profiler_frame_duration > std::time::Duration::from_secs(1) {
                        tracing::warn!(
                            frame_millis = profiler_frame_duration.as_millis() as u64,
                            update_millis = profiler_update_duration.as_millis() as u64,
                            render_millis = profiler_render_duration.as_millis() as u64,
                            publish_millis = profiler_publish_duration.as_millis() as u64,
                            simulation_output_backlog = output_backlog.pending as u64,
                            simulation_output_backlog_peak = output_backlog.peak as u64,
                            simulation_render_wakes_sent = output_backlog.render_wakes_sent as u64,
                            simulation_render_wakes_coalesced =
                                output_backlog.render_wakes_coalesced as u64,
                            publication_generation_lag,
                            effect_sequence_lag,
                            "slow frame"
                        );
                    }
                    debug_profiler.record_frame(
                        profiler_frame_duration,
                        &[
                            DebugProfilerSectionSample {
                                name: "update",
                                duration: profiler_update_duration,
                            },
                            DebugProfilerSectionSample {
                                name: "render",
                                duration: profiler_render_duration,
                            },
                            DebugProfilerSectionSample {
                                name: "publish",
                                duration: profiler_publish_duration,
                            },
                        ],
                    );
                    if let Some(interval) = frame_interval {
                        next_redraw_at = Instant::now() + interval;
                    }
                }
                _ => {}
            },
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                if cursor_captured {
                    simulation_mailbox.enqueue(SimulationCommand::MouseMotion { delta });
                }
            }
            Event::UserEvent(()) => {
                if frame_interval.is_none() || Instant::now() >= next_redraw_at {
                    window.request_redraw();
                }
            }
            Event::AboutToWait => {
                if frame_interval.is_none() || Instant::now() >= next_redraw_at {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    })?;

    Ok(())
}

fn set_cursor_capture(window: &Window, captured: &mut bool, capture: bool) {
    if *captured == capture {
        return;
    }

    if capture {
        match window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
        {
            Ok(()) => {
                window.set_cursor_visible(false);
                *captured = true;
            }
            Err(err) => {
                tracing::warn!(?err, "unable to capture cursor for first-person input");
            }
        }
    } else {
        if let Err(err) = window.set_cursor_grab(CursorGrabMode::None) {
            tracing::warn!(?err, "unable to release cursor capture");
        }
        window.set_cursor_visible(true);
        *captured = false;
    }
}

fn enqueue_simulation_window_event(
    event: &WindowEvent,
    mailbox: &SimulationMailbox,
    clipboard: &mut NativeDebugClipboard,
    renderer: &bbb_renderer::Renderer,
    window: &Window,
    cursor_position: &mut Option<winit::dpi::PhysicalPosition<f64>>,
) -> bool {
    let command = match event {
        WindowEvent::CloseRequested => SimulationCommand::Shutdown,
        WindowEvent::CursorMoved { position, .. } => {
            *cursor_position = Some(*position);
            window.set_cursor_icon(
                if renderer
                    .hud_death_screen_hit_target_at((position.x, position.y))
                    .is_some()
                    || renderer
                        .hud_game_event_hit_target_at((position.x, position.y))
                        .is_some()
                    || renderer
                        .hud_book_hit_target_at((position.x, position.y))
                        .is_some()
                {
                    CursorIcon::Pointer
                } else {
                    CursorIcon::Default
                },
            );
            SimulationCommand::CursorMoved {
                position: *position,
                death_screen_target: renderer
                    .hud_death_screen_hit_target_at((position.x, position.y)),
                game_event_target: renderer.hud_game_event_hit_target_at((position.x, position.y)),
            }
        }
        WindowEvent::CursorLeft { .. } => {
            *cursor_position = None;
            window.set_cursor_icon(CursorIcon::Default);
            SimulationCommand::CursorLeft
        }
        WindowEvent::Focused(focused) => SimulationCommand::FocusChanged(*focused),
        WindowEvent::KeyboardInput { event, .. } => {
            let clipboard_text = (matches!(event.state, ElementState::Pressed)
                && matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyV)))
            .then(|| clipboard.get_debug_clipboard_text())
            .flatten();
            SimulationCommand::Key {
                physical_key: event.physical_key,
                state: event.state,
                clipboard_text,
            }
        }
        WindowEvent::Ime(Ime::Preedit(text, cursor_range)) => SimulationCommand::ImePreedit {
            text: text.clone(),
            cursor_range: *cursor_range,
        },
        WindowEvent::Ime(Ime::Commit(text)) => SimulationCommand::ImeCommit(text.clone()),
        WindowEvent::Ime(Ime::Disabled) => SimulationCommand::ImeDisabled,
        WindowEvent::MouseInput { state, button, .. } => SimulationCommand::MouseButton {
            button: *button,
            state: *state,
            chat_target: (matches!(
                (*button, *state),
                (winit::event::MouseButton::Left, ElementState::Pressed)
            ))
            .then(|| {
                (*cursor_position)
                    .and_then(|position| renderer.hud_chat_hit_target_at((position.x, position.y)))
            })
            .flatten(),
            book_target: (matches!(
                (*button, *state),
                (winit::event::MouseButton::Left, ElementState::Pressed)
            ))
            .then(|| {
                (*cursor_position)
                    .and_then(|position| renderer.hud_book_hit_target_at((position.x, position.y)))
            })
            .flatten(),
            death_screen_target: (matches!(
                (*button, *state),
                (winit::event::MouseButton::Left, ElementState::Pressed)
            ))
            .then(|| {
                (*cursor_position).and_then(|position| {
                    renderer.hud_death_screen_hit_target_at((position.x, position.y))
                })
            })
            .flatten(),
            game_event_target: (matches!(
                (*button, *state),
                (winit::event::MouseButton::Left, ElementState::Pressed)
            ))
            .then(|| {
                (*cursor_position).and_then(|position| {
                    renderer.hud_game_event_hit_target_at((position.x, position.y))
                })
            })
            .flatten(),
        },
        WindowEvent::MouseWheel { delta, .. } => SimulationCommand::MouseWheel { delta: *delta },
        _ => return false,
    };
    mailbox.enqueue(command);
    true
}

fn world_wants_cursor(world: &WorldStore) -> bool {
    world.open_container_id().is_some()
        || world.death_screen().is_some()
        || world.game_event_screen().is_some()
        || world.dialog_screen_is_open()
        || world.current_book().is_some()
        || world.advancements_screen_is_open()
        || world.stats_screen_is_open()
        || world.game_rules_screen().is_some()
        || world.command_block_editor().is_some()
        || world.minecart_command_block_editor().is_some()
        || world.test_block_editor().is_some()
        || world.test_instance_block_editor().is_some()
}

pub(crate) fn runtime_wants_cursor(input: &ClientInputState, world: &WorldStore) -> bool {
    world_wants_cursor(world)
        || input.sign_editor_is_active_or_pending(world)
        || input.debug_options_screen_is_open()
        || input.debug_pause_screen_is_open()
        || input.debug_game_mode_switcher_is_open()
}

fn drain_dynamic_player_skin_downloads(
    items: &NativeItemRuntime,
    renderer: &mut bbb_renderer::Renderer,
) {
    for download in items.drain_dynamic_player_skin_download_results() {
        let Some(skin) = download.skin else {
            continue;
        };
        let handle = skin.handle;
        if let Err(err) = renderer.upload_dynamic_player_skin(skin) {
            tracing::warn!(
                ?err,
                url = %download.url,
                "failed to upload dynamic player skin"
            );
            items.mark_profile_skin_failed(&download.url);
            continue;
        }
        items.mark_profile_skin_resolved(&download.url, handle);
    }

    for download in items.drain_dynamic_player_texture_download_results() {
        match download.texture {
            Some(texture) => {
                let handle = texture.handle;
                let size = texture.size;
                if let Err(err) = renderer.upload_dynamic_player_texture(texture) {
                    tracing::warn!(
                        ?err,
                        kind = ?download.kind,
                        url = %download.url,
                        handle,
                        width = size[0],
                        height = size[1],
                        "failed to upload dynamic player profile texture"
                    );
                    continue;
                }
                tracing::debug!(
                    kind = ?download.kind,
                    url = %download.url,
                    handle,
                    width = size[0],
                    height = size[1],
                    "uploaded dynamic player profile texture"
                );
            }
            None => {
                tracing::warn!(
                    kind = ?download.kind,
                    url = %download.url,
                    "failed to download dynamic player profile texture"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        BlockPos as ProtocolBlockPos, CommonPlayerSpawnInfo, EntityEvent as ProtocolEntityEvent,
        OpenSignEditor, PlayLogin,
    };
    use bbb_world::{
        BlockEntityRecord, BlockPos, ChunkColumn, ChunkPos, ChunkState, LightData,
        SignBlockEntityTextState,
    };

    #[test]
    fn runtime_wants_cursor_for_pending_sign_editor() {
        let input = ClientInputState::new(true);
        let mut world = WorldStore::new();

        assert!(!runtime_wants_cursor(&input, &world));

        insert_empty_sign(&mut world, BlockPos { x: 1, y: 2, z: 3 });
        world.apply_open_sign_editor(OpenSignEditor {
            pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
            is_front_text: true,
        });

        assert!(runtime_wants_cursor(&input, &world));
    }

    fn insert_empty_sign(world: &mut WorldStore, pos: BlockPos) {
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos {
                x: pos.x.div_euclid(16),
                z: pos.z.div_euclid(16),
            },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: Vec::new(),
            block_entities: vec![BlockEntityRecord {
                local_x: pos.x.rem_euclid(16) as u8,
                y: i16::try_from(pos.y).unwrap(),
                local_z: pos.z.rem_euclid(16) as u8,
                type_id: bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
                raw_nbt: vec![0],
                nbt: None,
                sign_text: Some(SignBlockEntityTextState::default()),
                command_block: None,
                structure_block: None,
                jigsaw_block: None,
                test_block: None,
                test_instance_block: None,
                vault_shared_data: None,
                decorated_pot_sherds: None,
                banner_patterns: None,
                end_gateway: None,
                spawner: None,
                note_block_sound: None,
                skull_profile: None,
            }],
            light: LightData::default(),
        });
    }

    #[test]
    fn runtime_wants_cursor_for_advancements_screen() {
        let input = ClientInputState::new(true);
        let mut world = WorldStore::new();

        assert!(!runtime_wants_cursor(&input, &world));
        assert!(world.open_advancements_screen());
        assert!(runtime_wants_cursor(&input, &world));
    }

    #[test]
    fn runtime_wants_cursor_for_stats_screen() {
        let input = ClientInputState::new(true);
        let mut world = WorldStore::new();

        assert!(!runtime_wants_cursor(&input, &world));
        assert!(world.open_stats_screen());
        assert!(runtime_wants_cursor(&input, &world));
    }

    #[test]
    fn runtime_wants_cursor_for_debug_game_mode_switcher() {
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
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
        });
        assert!(world.apply_entity_event(ProtocolEntityEvent {
            entity_id: 42,
            event_id: 26,
        }));

        assert!(!runtime_wants_cursor(&input, &world));
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
        assert!(runtime_wants_cursor(&input, &world));
    }

    #[test]
    fn runtime_wants_cursor_for_debug_options_screen() {
        let mut input = ClientInputState::new(true);
        let world = WorldStore::new();

        assert!(!runtime_wants_cursor(&input, &world));
        input.open_debug_options_screen();
        assert!(runtime_wants_cursor(&input, &world));
    }

    #[test]
    fn runtime_wants_cursor_for_debug_pause_screen() {
        let mut input = ClientInputState::new(true);
        let world = WorldStore::new();

        assert!(!runtime_wants_cursor(&input, &world));
        input.open_debug_pause_screen_without_menu();
        assert!(runtime_wants_cursor(&input, &world));
    }

    #[test]
    fn item_particle_sprite_uvs_reuse_item_atlas_rects() {
        let runtime = NativeItemRuntime::empty_for_test();
        let item_uvs = runtime.atlas_sprite_uvs();
        let particle_uvs = item_particle_sprite_uvs(&runtime);

        assert_eq!(particle_uvs.len(), item_uvs.len());
        assert_eq!(particle_uvs[0].id, item_uvs[0].id);
        assert_eq!(particle_uvs[0].uv.min, item_uvs[0].uv.min);
        assert_eq!(particle_uvs[0].uv.max, item_uvs[0].uv.max);
        assert_eq!(particle_uvs[0].has_translucent, item_uvs[0].has_translucent);
    }
}
