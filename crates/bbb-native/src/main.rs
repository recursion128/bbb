// bbb supports macOS and Linux. Windows is a declared non-goal, so its
// platform dispatch, window-icon path and store roots do not exist; a build
// for that target would silently be missing them. Refuse it here instead, so
// the scope decision is enforced by the compiler rather than by documentation.
#[cfg(target_os = "windows")]
compile_error!(
    "bbb supports macOS and Linux; Windows is a declared non-goal and has no platform dispatch"
);

use anyhow::{Context, Result};
use bbb_control::{AudioCounters, ControlState, NetCounters, SharedControlRequests};
use bbb_world::{ChatPresentationConfig, ChatVisibilityState, WorldStore};
use std::{cell::RefCell, rc::Rc, time::Instant};
use winit::{
    dpi::PhysicalSize,
    event::{DeviceEvent, ElementState, Event, Ime, WindowEvent},
    event_loop::ControlFlow,
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, CursorIcon, Fullscreen, Window},
};

mod audio_runtime;
mod banner_scene;
mod beacon_scene;
mod bed_scene;
mod bell_scene;
mod biome_tint;
mod block_entity_visibility;
mod block_outline;
mod bounding_box_gizmos;
mod camera_fov;
mod camera_pose;
mod cartography_preview;
mod chest_scene;
mod client_options;
mod code_of_conduct;
mod code_of_conduct_overlay;
mod component_text;
mod conduit_scene;
mod copper_golem_statue_scene;
mod crash_report;
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
mod frame_diagnostics;
mod game_test_highlights;
mod hotbar_presets;
mod hud_assets;
mod input;
mod item_entities;
mod item_frames;
mod item_models;
#[cfg(test)]
mod item_registry_test_fixture;
mod item_texture_animation;
mod lectern_book_scene;
mod level_load_runtime;
#[cfg(feature = "lifecycle-soak")]
mod lifecycle_soak;
mod map_textures;
mod music_runtime;
mod narrator_runtime;
mod network_runtime;
mod particle_runtime;
mod platform;
mod recipe_book_ui;
mod remote_debug;
mod resource_pack_runtime;
mod resource_reload;
mod runtime;
mod screen_effects;
mod screenshot;
mod server_list;
mod services_block_list_runtime;
mod shulker_box_scene;
mod sign_scene;
mod simulation;
mod skull_scene;
mod sky_assets;
mod sound_subtitle_runtime;
mod spawner_scene;
mod startup;
mod terrain_runtime;
mod test_instance_gizmos;
mod weather_effect_runtime;
mod win_screen_resources;
mod window_icon;
mod window_surface;

use audio_runtime::{AudioEventSink, NativeAudioRuntime};
use bbb_item_model::{
    default_player_skin_cache_dir, font::providers::FontOptions, NativeItemRuntime,
};
use camera_fov::CameraFovTickState;
use camera_pose::CameraEyeHeightTickState;
use client_options::{ClientOptionHost, ClientOptionsFile};
use code_of_conduct::{default_code_of_conduct_store_path, CodeOfConductAcceptance};
use crash_report::{
    default_crash_report_dir, exit_after_failure, CrashReportRuntime, RuntimeFailure,
    RuntimeFailureAccumulator,
};
use debug_entries::{DebugScreenEntryId, DebugScreenEntryList};
use debug_profiler::{
    DebugProfiler, DebugProfilerEvent, DebugProfilerFrameMetrics, DebugProfilerSectionSample,
};
use entity_assets::load_entity_model_textures;
use entity_scene::EntityRenderExtractionScratch;
use entity_variant_textures::NativeEntityVariantTextureRuntime;
use frame_diagnostics::{FramePhaseDiagnostics, FramePhaseSample};
use hud_assets::{load_hud_textures, load_panorama};
use input::{ClientInputState, DebugClipboard};
use item_texture_animation::{maybe_upload_item_texture_animation, ItemTextureAnimationState};
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
    client_resource_pack_diagnostics, item_particle_sprite_uvs, prepare_native_shader_resources,
    SELECTED_LANGUAGE_PROBE_KEY,
};
use runtime::{
    control_renderer_counters, dialog_ime_cursor_area, hud_player_list_is_visible,
    hud_player_list_narration_from_world, input_screen_is_open, narrate_command_suggestion_change,
    narrate_player_list_visibility_change, project_dynamic_texture_counters,
    world_client_tick_interval, ClientAnimationTickState, DebugCollisionCache,
    DebugSkyLightSectionCache, DebugSupportingBlockCache, HudDebugFpsSampler,
    HudDebugNetworkSampler, HudDebugTpsSampler, HudRenderRuntime, LevelEventSoundRandomState,
    LightmapTickState,
};
use server_list::ServerListPersistence;
use services_block_list_runtime::ServicesBlockListRuntime;
use simulation::{
    commands::{DebugProfilingLifecycle, SimulationCommand, SimulationMailbox},
    effects::{
        ParticleBatchSink, PlatformEffect, PlatformEffectSink, RenderEffectConsumer,
        ScreenNarrationScheduleInput,
    },
    publication::{SimulationPublicationInput, SimulationPublisher},
    thread::SimulationThread,
    update_renderer_from_publication, SimulationCore, SimulationRenderContext,
};
use sky_assets::load_sky_textures;
use sound_subtitle_runtime::SoundSubtitleRuntime;
use startup::{
    build_window, client_framerate_limit_interval, create_event_loop, effective_framerate_limit,
    init_tracing, load_known_packs_manager, load_pack_roots_with_options, parse_args,
    run_build_info_if_requested, run_probe_if_requested, run_status_if_requested, spawn_frame_tick,
    start_control_api, start_network_if_requested,
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
    /// `ClipboardManager.setClipboard`. Its `pushClipboard` half — the 8 KiB
    /// scratch `ByteBuffer`, `MemoryUtil.memAlloc` overflow path and
    /// NUL-terminated UTF-8 push GLFW requires — has no equivalent because
    /// `arboard` takes an owned Rust `String` directly.
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

    /// `ClipboardManager.getClipboard`, with backend errors collapsing to
    /// `None` the way vanilla suppresses `FORMAT_UNAVAILABLE` to `""`.
    /// Vanilla's `StringDecomposer.filterBrokenSurrogates` pass has no
    /// equivalent step: a Rust `String` is well-formed UTF-8 by construction,
    /// so broken surrogates cannot reach the paste path.
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
    /// `Options.enableVsync`; re-applied by `Window.updateVsync` after every
    /// fullscreen transition. Winit reports no synchronous fullscreen failure,
    /// so `Minecraft.onFullscreenError` has no runtime trigger on this backend
    /// (see `/tmp/window-surface-work/audit.md`).
    vsync_enabled: bool,
    /// The stored `fullscreenResolution` (`Options.fullscreenVideoModeString`);
    /// `None` means no preferred exclusive mode, exactly like
    /// `Window.preferredFullscreenVideoMode` empty. It never changes after
    /// startup — `Window.changeFullscreenVideoMode`'s dirty-flag re-apply only
    /// serves the video-settings screen writing the option at runtime, and
    /// that options UI is a non-goal — so no re-apply path exists.
    preferred_video_mode: Option<window_surface::PreferredVideoMode>,
    /// Backend capability, not a stored option: Wayland no-ops
    /// `Fullscreen::Exclusive`, so BBB falls back to borderless there.
    exclusive_fullscreen_supported: bool,
    mailbox: &'a SimulationMailbox,
    clipboard: &'a mut NativeDebugClipboard,
    terrain_upload: &'a mut TerrainUploadState,
    client_locale: &'a mut String,
    debug_profiler: &'a mut DebugProfiler,
    debug_options_screen_narration: &'a mut ScreenNarrationRuntime,
    stats_screen_narration: &'a mut ScreenNarrationRuntime,
    recipe_book_narration: &'a mut ScreenNarrationRuntime,
    social_interactions_narration: &'a mut ScreenNarrationRuntime,
    player_reporting_narration: &'a mut ScreenNarrationRuntime,
    death_screen_narration: &'a mut ScreenNarrationRuntime,
    disconnected_screen_narration: &'a mut ScreenNarrationRuntime,
    chat_screen_narration: &'a mut ScreenNarrationRuntime,
    game_event_screen_narration: &'a mut ScreenNarrationRuntime,
    narrator: Option<&'a mut dyn narrator_runtime::SystemNarrator>,
    control_screenshots: &'a mut std::collections::VecDeque<PendingScreenshot>,
    game_dir: &'a std::path::Path,
    client_options: Option<&'a mut ClientOptionsFile>,
    exit_requested: &'a mut bool,
}

impl ParticleBatchSink for NativeRenderEffectSink<'_> {
    fn submit_particle_batch(&mut self, batch: bbb_renderer::ParticleSpawnBatch) {
        self.renderer.submit_particle_spawns(batch);
    }
}

/// Complete Vanilla's synchronous `Options.save` contract: attempt the file
/// write first, then publish the client-information completion regardless of
/// a missing file owner or write failure. The caller owns presentation/logging
/// of the optional write result; the ordering lives here once.
fn save_client_options_and_enqueue_broadcast(
    client_options: Option<&mut ClientOptionsFile>,
    writes: &[client_options::ClientOptionWrite],
    broadcast: bbb_protocol::packets::ClientInformation,
    mailbox: &SimulationMailbox,
) -> Option<Result<bool>> {
    let result = client_options.map(|options| options.apply(writes));
    mailbox.enqueue_or_report(
        SimulationCommand::ClientOptionsSaveCompleted(broadcast),
        "client options save completion",
    );
    result
}

impl PlatformEffectSink for NativeRenderEffectSink<'_> {
    fn apply_platform_effect(&mut self, effect: PlatformEffect) {
        match effect {
            PlatformEffect::SetCursorCapture(capture) => {
                set_cursor_capture(self.window, self.cursor_captured, capture);
                self.mailbox.enqueue_or_report(
                    SimulationCommand::CursorCaptureResult {
                        captured: *self.cursor_captured,
                    },
                    "cursor capture result",
                );
            }
            PlatformEffect::SetTextInputEnabled(enabled) => {
                self.window.set_ime_allowed(enabled);
            }
            PlatformEffect::ClipboardWrite(text) => {
                let success = self.clipboard.set_debug_clipboard_text(&text);
                self.mailbox.enqueue_or_report(
                    SimulationCommand::ClipboardWriteResult { success },
                    "clipboard write result",
                );
            }
            PlatformEffect::OpenUri(url) => {
                let success = platform::open_uri(&url)
                    .map_err(|err| tracing::warn!(?err, %url, "failed to open dialog URI"))
                    .is_ok();
                self.mailbox.enqueue_or_report(
                    SimulationCommand::OpenUriResult { url, success },
                    "open URI result",
                );
            }
            PlatformEffect::OpenPath(path) => {
                let success = platform::open_path(&path)
                    .map_err(
                        |err| tracing::warn!(?err, path = %path.display(), "failed to open path"),
                    )
                    .is_ok();
                self.mailbox.enqueue_or_report(
                    SimulationCommand::OpenPathResult { path, success },
                    "open path result",
                );
            }
            PlatformEffect::SaveClientOptions { writes, broadcast } => {
                match save_client_options_and_enqueue_broadcast(
                    self.client_options.as_deref_mut(),
                    &writes,
                    broadcast,
                    self.mailbox,
                ) {
                    // `Options.save` only logs when the write fails.
                    Some(Err(error)) => {
                        tracing::warn!(%error, "failed to save options.txt");
                    }
                    None => tracing::warn!("no options.txt owner to save client options into"),
                    Some(Ok(_)) => {}
                }
            }
            PlatformEffect::Screenshot(path) => {
                self.control_screenshots.push_back(PendingScreenshot {
                    path,
                    announce: false,
                });
            }
            PlatformEffect::GrabScreenshot => {
                let directory = self.game_dir.join(screenshot::SCREENSHOT_DIR);
                if let Err(err) = std::fs::create_dir_all(&directory) {
                    tracing::warn!(?err, path = %directory.display(), "couldn't create screenshot directory");
                    self.mailbox.enqueue_or_report(
                        SimulationCommand::ScreenshotSaved {
                            name: String::new(),
                            error: Some(err.to_string()),
                        },
                        "screenshot directory failure",
                    );
                    return;
                }
                let stamp = chrono::Local::now().format("%Y-%m-%d_%H.%M.%S").to_string();
                let path =
                    screenshot::next_screenshot_file(&directory, &stamp, |path| path.exists());
                self.control_screenshots.push_back(PendingScreenshot {
                    path,
                    announce: true,
                });
            }
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
            PlatformEffect::SetClientLocale(locale) => {
                *self.client_locale = locale;
            }
            PlatformEffect::ProfilingToggled { commands } => {
                let requests = commands.len();
                let mut starts = 0;
                let mut stops = 0;
                for command in commands {
                    match command {
                        simulation::effects::DebugProfilingCommand::Start => {
                            starts +=
                                usize::from(self.debug_profiler.start_recording(Instant::now()));
                        }
                        simulation::effects::DebugProfilingCommand::Stop => {
                            stops += usize::from(self.debug_profiler.stop_recording());
                        }
                    }
                }
                tracing::info!(
                    requests,
                    starts,
                    stops,
                    recording = self.debug_profiler.is_recording(),
                    "profiling toggled by debug hotkey"
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
            PlatformEffect::DebugOptionsScreenInput(input) => {
                let input = match input {
                    ScreenNarrationScheduleInput::Immediate => ScreenNarrationInput::Immediate,
                    ScreenNarrationScheduleInput::MouseMove => ScreenNarrationInput::MouseMove,
                    ScreenNarrationScheduleInput::MouseAction => ScreenNarrationInput::MouseAction,
                    ScreenNarrationScheduleInput::KeyboardAction => {
                        ScreenNarrationInput::KeyboardAction
                    }
                };
                self.debug_options_screen_narration
                    .after_input(input, Instant::now());
            }
            PlatformEffect::StatsScreenInput(input) => {
                let input = match input {
                    ScreenNarrationScheduleInput::Immediate => ScreenNarrationInput::Immediate,
                    ScreenNarrationScheduleInput::MouseMove => ScreenNarrationInput::MouseMove,
                    ScreenNarrationScheduleInput::MouseAction => ScreenNarrationInput::MouseAction,
                    ScreenNarrationScheduleInput::KeyboardAction => {
                        ScreenNarrationInput::KeyboardAction
                    }
                };
                self.stats_screen_narration
                    .after_input(input, Instant::now());
            }
            PlatformEffect::RecipeBookInput(input) => {
                let input = match input {
                    ScreenNarrationScheduleInput::Immediate => ScreenNarrationInput::Immediate,
                    ScreenNarrationScheduleInput::MouseMove => ScreenNarrationInput::MouseMove,
                    ScreenNarrationScheduleInput::MouseAction => ScreenNarrationInput::MouseAction,
                    ScreenNarrationScheduleInput::KeyboardAction => {
                        ScreenNarrationInput::KeyboardAction
                    }
                };
                self.recipe_book_narration
                    .after_input(input, Instant::now());
            }
            PlatformEffect::SocialInteractionsInput(input) => {
                let input = match input {
                    ScreenNarrationScheduleInput::Immediate => ScreenNarrationInput::Immediate,
                    ScreenNarrationScheduleInput::MouseMove => ScreenNarrationInput::MouseMove,
                    ScreenNarrationScheduleInput::MouseAction => ScreenNarrationInput::MouseAction,
                    ScreenNarrationScheduleInput::KeyboardAction => {
                        ScreenNarrationInput::KeyboardAction
                    }
                };
                self.social_interactions_narration
                    .after_input(input, Instant::now());
            }
            PlatformEffect::PlayerReportingInput(input) => {
                let input = match input {
                    ScreenNarrationScheduleInput::Immediate => ScreenNarrationInput::Immediate,
                    ScreenNarrationScheduleInput::MouseMove => ScreenNarrationInput::MouseMove,
                    ScreenNarrationScheduleInput::MouseAction => ScreenNarrationInput::MouseAction,
                    ScreenNarrationScheduleInput::KeyboardAction => {
                        ScreenNarrationInput::KeyboardAction
                    }
                };
                self.player_reporting_narration
                    .after_input(input, Instant::now());
            }
            PlatformEffect::GameEventScreenInput(input) => {
                let input = match input {
                    ScreenNarrationScheduleInput::Immediate => ScreenNarrationInput::Immediate,
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
                    ScreenNarrationScheduleInput::Immediate => ScreenNarrationInput::Immediate,
                    ScreenNarrationScheduleInput::MouseMove => ScreenNarrationInput::MouseMove,
                    ScreenNarrationScheduleInput::MouseAction => ScreenNarrationInput::MouseAction,
                    ScreenNarrationScheduleInput::KeyboardAction => {
                        ScreenNarrationInput::KeyboardAction
                    }
                };
                self.death_screen_narration
                    .after_input(input, Instant::now());
            }
            PlatformEffect::DisconnectedScreenInput(input) => {
                let input = match input {
                    ScreenNarrationScheduleInput::Immediate => ScreenNarrationInput::Immediate,
                    ScreenNarrationScheduleInput::MouseMove => ScreenNarrationInput::MouseMove,
                    ScreenNarrationScheduleInput::MouseAction => ScreenNarrationInput::MouseAction,
                    ScreenNarrationScheduleInput::KeyboardAction => {
                        ScreenNarrationInput::KeyboardAction
                    }
                };
                self.disconnected_screen_narration
                    .after_input(input, Instant::now());
            }
            PlatformEffect::ChatScreenInput(input) => {
                let input = match input {
                    ScreenNarrationScheduleInput::Immediate => ScreenNarrationInput::Immediate,
                    ScreenNarrationScheduleInput::MouseMove => ScreenNarrationInput::MouseMove,
                    ScreenNarrationScheduleInput::MouseAction => ScreenNarrationInput::MouseAction,
                    ScreenNarrationScheduleInput::KeyboardAction => {
                        ScreenNarrationInput::KeyboardAction
                    }
                };
                self.chat_screen_narration
                    .after_input(input, Instant::now());
            }
            PlatformEffect::ChatNarration { kind, text } => {
                let Some(narrator) = self.narrator.as_deref_mut() else {
                    return;
                };
                narrator_runtime::submit_chat_narration(narrator, kind, &text);
            }
            PlatformEffect::SetFullscreen(fullscreen) => {
                // `Window.updateFullscreen` (Window.java:401-409): apply the
                // mode, then re-apply vsync. The mode policy is decided
                // deterministically up front (exclusive when an exact winit
                // video mode exists on a supporting backend, borderless
                // otherwise); winit's `set_fullscreen` has no synchronous
                // error channel to turn into `onFullscreenError`.
                set_window_fullscreen(
                    self.window,
                    fullscreen,
                    self.preferred_video_mode,
                    self.exclusive_fullscreen_supported,
                );
                if let Err(error) = self.renderer.set_vsync_enabled(self.vsync_enabled) {
                    tracing::warn!(?error, "failed to reapply vsync after fullscreen change");
                }
            }
            PlatformEffect::NarratorStatusChanged { requests } => {
                for request in requests {
                    if let Some(narrator) = self.narrator.as_deref_mut() {
                        narrator.update_status(request.status);
                    }
                    if !request.schedule_narration {
                        continue;
                    }
                    let now = Instant::now();
                    for narration in [
                        &mut *self.debug_options_screen_narration,
                        &mut *self.stats_screen_narration,
                        &mut *self.recipe_book_narration,
                        &mut *self.social_interactions_narration,
                        &mut *self.player_reporting_narration,
                        &mut *self.death_screen_narration,
                        &mut *self.disconnected_screen_narration,
                        &mut *self.chat_screen_narration,
                        &mut *self.game_event_screen_narration,
                    ] {
                        narration.schedule_after_narrator_enabled(now);
                    }
                }
            }
            PlatformEffect::Exit => *self.exit_requested = true,
        }
    }
}

struct PendingScreenshot {
    path: std::path::PathBuf,
    announce: bool,
}

fn main() {
    init_tracing();
    let args = parse_args();
    let crash_report_dir = args
        .crash_report_dir
        .clone()
        .unwrap_or_else(default_crash_report_dir);
    let mut crash_reports = CrashReportRuntime::new(crash_report_dir);
    if let Err(failures) = crash_reports.guard(|| run_client(args)) {
        exit_after_failure(crash_reports.finish_failure(failures));
    }
}

fn run_client(mut args: startup::Args) -> Result<()> {
    if run_build_info_if_requested(&args)? {
        return Ok(());
    }
    #[cfg(feature = "lifecycle-soak")]
    let mut lifecycle_soak = lifecycle_soak::LifecycleSoakRuntime::from_env()?;
    let runtime = tokio::runtime::Runtime::new()?;

    if run_status_if_requested(&runtime, &args)? {
        return Ok(());
    }
    if run_probe_if_requested(&runtime, &args)? {
        return Ok(());
    }
    if let Err(error) = startup::name_current_render_thread() {
        tracing::warn!(%error, "failed to name native render thread");
    }

    // `Minecraft.<init>` reads the previous marker, marks this startup dirty,
    // and saves before constructing the native window. A failed bootstrap
    // therefore remains observable on the next launch; only a fully assembled
    // initial client below writes the marker clean again.
    let mut client_options = match ClientOptionsFile::load(
        args.game_dir.join("options.txt"),
        ClientOptionHost {
            locale_is_japanese: client_options::locale_is_japanese(
                sys_locale::get_locale().as_deref(),
            ),
            platform_is_windows: crate::platform::HOST_IS_WINDOWS,
        },
    ) {
        Ok(mut options) => {
            let previous_startup_was_incomplete = !options.started_cleanly();
            if previous_startup_was_incomplete {
                args.client_fullscreen = false;
                args.client_fullscreen_resolution = None;
                tracing::warn!(
                    "detected incomplete prior startup; disabling startup fullscreen mode"
                );
            }
            if let Err(error) = options.save_started_cleanly(false) {
                tracing::warn!(%error, "failed to mark client startup incomplete");
            }
            if previous_startup_was_incomplete {
                options.clear_startup_fullscreen_resolution();
            }
            Some(options)
        }
        Err(error) => {
            tracing::warn!(%error, "failed to read options.txt");
            None
        }
    };

    let mut client_locale = args.client_locale.clone();

    let loaded_pack_roots = load_pack_roots_with_options(&args)?;
    let mut pack_roots = loaded_pack_roots.roots;
    let resource_pack_options_reconciliation = loaded_pack_roots.options_reconciliation;
    let base_pack_roots = pack_roots.clone();
    let known_packs_manager = load_known_packs_manager(pack_roots.as_ref());
    let simulation_mailbox = SimulationMailbox::default();
    let control = ControlState::new(
        format!(
            "bbb-native {} / protocol {}",
            bbb_protocol::MC_VERSION,
            bbb_protocol::PROTOCOL_VERSION
        ),
        SharedControlRequests::new(std::sync::Arc::new(simulation_mailbox.clone())),
    );
    let snapshot = control.snapshot.clone();
    let published_world = control.world.clone();
    let services_block_lists =
        ServicesBlockListRuntime::new(runtime.handle().clone(), simulation_mailbox.clone());
    let mut world = WorldStore::new();
    world.set_local_player_auto_jump_enabled(args.client_auto_jump);
    world.set_tutorial_step(args.tutorial_step.into());
    world.set_joined_first_server(args.client_joined_first_server);
    world.set_social_server_label(args.server.clone());
    world.set_dialog_link_options(args.client_chat_links, args.client_chat_links_prompt);
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
    let mut net_counters = NetCounters::default();
    net_counters.client_resources = std::sync::Arc::new(client_resource_pack_diagnostics(
        pack_roots.as_ref(),
        false,
        Some(&client_locale),
        None,
    ));
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
    let server_list = args
        .server_list_store
        .as_ref()
        .map(|path| {
            ServerListPersistence::new(
                path,
                args.server_list_name
                    .as_deref()
                    .expect("clap requires --server-list-name with --server-list-store"),
                &args.server,
            )
        })
        .transpose()?;
    let client_identity = startup::client_identity_from_args(&args);
    let mut network_runtime = start_network_if_requested(
        &runtime,
        &args,
        &client_identity,
        &mut code_of_conduct_acceptance,
        known_packs_manager,
        &simulation_mailbox,
    )?;
    let (net_events, net_commands) = network_runtime.take_channels();
    world.set_local_player_profile_id(client_identity.profile_id);
    let server_resource_pack_runtime = ServerResourcePackRuntime::new(
        runtime.handle().clone(),
        args.server_resource_pack_cache_dir
            .clone()
            .unwrap_or_else(|| default_server_resource_pack_cache_dir(&args.game_dir)),
        client_identity.username.clone(),
        client_identity.profile_id,
        startup::server_resource_pack_proxy_from_args(&args),
    )?;
    start_control_api(&runtime, args.control, &control);
    let force_no_audio_device = {
        #[cfg(feature = "lifecycle-soak")]
        {
            lifecycle_soak
                .as_ref()
                .is_some_and(lifecycle_soak::LifecycleSoakRuntime::force_no_audio_device)
        }
        #[cfg(not(feature = "lifecycle-soak"))]
        {
            false
        }
    };
    let (mut audio_runtime, mut audio_status) = if force_no_audio_device {
        (
            None,
            AudioCounters::disabled("lifecycle-soak injected no audio device"),
        )
    } else {
        match pack_roots.as_ref() {
            Some(roots) => match NativeAudioRuntime::load(
                roots,
                &args.client_sound_device,
                args.client_directional_audio,
                args.client_audio_volumes,
            ) {
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
        }
    };
    let narrator_status = NarratorStatus::from(args.client_narrator);
    world.set_available_command_sounds(
        audio_runtime
            .as_ref()
            .map(NativeAudioRuntime::available_sound_ids)
            .unwrap_or_default(),
    );
    let mut narrator_runtime = args.narrator_command.clone().map(|command| {
        NativeNarratorRuntime::new(
            narrator_status,
            command,
            args.narrator_command_args.clone(),
            args.narrator_voice_volume,
        )
    });
    let mut sound_subtitle_runtime = SoundSubtitleRuntime::default();
    // `GameNarrator.checkStatus` raises an OS `MessageBox.errorWithContinue`
    // and aborts startup via `NarratorInitException` when the user declines a
    // dead TTS library. The narrator backend here is explicit CLI
    // configuration rather than a bundled library probe, so the equivalent
    // decision — narrator requested but no backend — is a bounded startup
    // warning and a continue; there is no modal prompt to decline.
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
        NativeItemRuntime::load_with_locale_and_font_options(
            roots,
            &client_locale,
            FontOptions {
                uniform: args.client_force_unicode_font,
                japanese_variants: args.client_japanese_glyph_variants,
            },
        )
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
            default_attack_range_items = items.item_attack_range_count(),
            default_swing_animation_items = items.item_swing_animation_count(),
            default_piercing_weapon_items = items.default_piercing_weapon_item_count(),
            default_item_max_damage = items.item_max_damage_count(),
            default_damageable_items = items.default_damageable_item_count(),
            item_crafting_remainders = items.item_crafting_remainder_count(),
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
    let window_config = startup::WindowConfig::from_args(&args);
    let window = build_window(&event_loop, &window_config)?;
    let window_backend = window_icon::NativeWindowBackend::detect(&window)?;
    let window_size = window.inner_size();
    let window_platform = window_backend.platform_identity(std::env::consts::OS);
    let monitor_refresh_rate_hz = window
        .current_monitor()
        .and_then(|monitor| monitor.refresh_rate_millihertz())
        .map(window_surface::refresh_rate_hz);
    tracing::info!(
        target_os = std::env::consts::OS,
        target_arch = std::env::consts::ARCH,
        window_backend = %window_backend.label(),
        window_platform = window_platform,
        monitor_refresh_rate_hz = ?monitor_refresh_rate_hz,
        scale_factor = window.scale_factor(),
        width = window_size.width,
        height = window_size.height,
        "native platform window created"
    );
    window_icon::install_startup_window_icon(&window, pack_roots.as_ref(), window_backend);
    // `TextInputManager` starts life with text input stopped and the IME
    // revoked (TextInputManager.java:26-47 turns it off on every tick the
    // window is focused outside text input); the simulation turns it on
    // through `PlatformEffect::SetTextInputEnabled` exactly while a text field
    // has focus. Leaving it allowed for the whole session is not a harmless
    // superset -- an installed input method grabs every letter key it is
    // offered, so `T` composes a preedit instead of opening chat.
    window.set_ime_allowed(false);
    let mut input = ClientInputState::new(window.has_focus());
    input.set_save_chat_drafts(args.client_save_chat_drafts);
    input.set_client_text_filtering_enabled(args.client_text_filtering);
    input.set_hotbar_preset_store_path(hotbar_presets::default_hotbar_preset_store_path());
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
    input.set_debug_pause_on_lost_focus(args.pause_on_lost_focus);
    input.set_reduced_debug_info_option(args.client_reduced_debug_info);
    input.set_narrator_status(NarratorStatus::from(args.client_narrator));
    input.set_narrator_hotkey_enabled(args.client_narrator_hotkey);
    input.set_narrator_available(narrator_runtime.is_some());
    input.set_key_bindings(args.client_key_bindings.clone());
    input.set_toggle_key_options(
        args.client_toggle_crouch,
        args.client_toggle_sprint,
        args.client_toggle_attack,
        args.client_toggle_use,
    );
    input.set_fullscreen(args.client_fullscreen);
    apply_startup_fullscreen(
        &window,
        &mut input,
        args.client_fullscreen_resolution,
        window_backend.supports_exclusive_fullscreen(),
    );
    // `InputConstants.updateRawMouseInput` (InputConstants.java:231-235) is a
    // GLFW-specific `glfwSetInputMode(RAW_MOUSE_MOTION)` call. winit 0.29 has
    // no raw-motion toggle: `DeviceEvent::MouseMotion` is always raw and
    // unfiltered, and `CursorGrabMode` only controls confinement/locking. The
    // option is still parsed for ledger fidelity, with one bounded startup
    // warning when it is disabled.
    if !args.client_raw_mouse_input {
        tracing::warn!(
            "rawMouseInput=false has no effect on the winit backend: \
             DeviceEvent::MouseMotion is always raw and unfiltered"
        );
    }
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
    let mut debug_clipboard = NativeDebugClipboard::default();
    spawn_frame_tick(&event_loop);

    let mut renderer = pollster::block_on(
        bbb_renderer::Renderer::new_with_vsync_and_terrain_filtering_for_fabulous(
            &window,
            args.client_vsync,
            args.terrain_texture_filtering.into(),
            args.fabulous_texture_filtering,
        ),
    )?;
    renderer.set_lightmap_brightness_factor(args.client_gamma);
    renderer.set_glint_settings(args.client_glint_speed, args.client_glint_strength as f32);
    renderer.set_terrain_max_anisotropy_bit(args.client_max_anisotropy_bit);
    renderer.set_cloud_status(
        args.render_clouds != startup::RenderCloudsArg::Off,
        match args.render_clouds {
            startup::RenderCloudsArg::Fast => bbb_renderer::CloudShape::Flat,
            _ => bbb_renderer::CloudShape::Fancy,
        },
    );
    renderer.set_menu_background_blurriness(args.menu_background_blurriness);
    renderer.set_hud_text_background_options(
        args.text_background_opacity,
        args.background_for_chat_only,
    );
    if let Some(roots) = pack_roots.as_ref() {
        let shader_resources =
            prepare_native_shader_resources(roots).map(|resources| resources.renderer_projection());
        match shader_resources.and_then(|resources| {
            renderer.transactional_shader_resource_reload(&resources, true, |_| Ok(()))
        }) {
            Ok(()) => {}
            Err(error) => {
                tracing::warn!(
                    %error,
                    "continuing with embedded shader fallback after startup load failure"
                );
            }
        }
    }
    let terrain_atlas_base_padding = startup::terrain_atlas_base_padding_for_filtering(
        renderer.terrain_texture_filtering(),
        args.client_max_anisotropy_bit,
    );
    let mut terrain_textures = load_terrain_textures(
        &mut renderer,
        pack_roots.as_ref(),
        args.cutout_leaves,
        args.client_smooth_lighting,
        args.biome_blend_radius,
        args.client_mipmap_levels,
        terrain_atlas_base_padding,
    );
    if let Some(particles) = particle_runtime.as_mut() {
        particles.set_terrain_particle_sprite_ids(&terrain_textures);
        if let Some(items) = &item_runtime {
            particles.set_default_item_particle_sprite_ids(items);
        }
    }
    load_hud_textures(
        &mut renderer,
        pack_roots.as_ref(),
        FontOptions {
            uniform: args.client_force_unicode_font,
            japanese_variants: args.client_japanese_glyph_variants,
        },
    );
    load_panorama(&mut renderer, pack_roots.as_ref());
    renderer.set_default_text_right_to_left(
        item_runtime
            .as_ref()
            .is_some_and(NativeItemRuntime::language_bidirectional),
    );
    let win_screen_resources = std::sync::Arc::new(
        pack_roots
            .as_ref()
            .map(|roots| {
                WinScreenResources::load(roots, &args.username, &renderer.hud_text_measurements())
            })
            .transpose()?
            .unwrap_or_default(),
    );
    simulation_mailbox.enqueue_or_report(
        SimulationCommand::TextMeasurements(renderer.hud_text_measurements()),
        "startup text measurements",
    );
    load_entity_model_textures(
        &mut renderer,
        pack_roots.as_ref(),
        item_runtime
            .as_ref()
            .map(NativeItemRuntime::equipment_asset_catalog),
        item_runtime
            .as_ref()
            .map_or(0, NativeItemRuntime::resource_generation),
    );
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
    net_counters.client_resources = std::sync::Arc::new(client_resource_pack_diagnostics(
        pack_roots.as_ref(),
        item_runtime
            .as_ref()
            .is_some_and(NativeItemRuntime::language_bidirectional),
        Some(&client_locale),
        item_runtime
            .as_ref()
            .map(|runtime| runtime.localized_text(SELECTED_LANGUAGE_PROBE_KEY)),
    ));
    let mut entity_variant_textures = NativeEntityVariantTextureRuntime::new(pack_roots.clone());
    let mut entity_render_extraction = EntityRenderExtractionScratch::default();
    let mut non_entity_render_extraction = runtime::NonEntityRenderExtractionScratch::default();
    let mut camera_environment_samples = runtime::CameraEnvironmentSampleCache::default();
    // `Options.broadcastOptions` resends this after every save, so keep the
    // configured player information around for the session.
    let startup_client_information = startup::client_information_from_args(&args)?;
    let mut screenshot = args.screenshot;
    let screenshot_after_terrain = args.connect_server;
    let exit_after_screenshot = args.exit_after_screenshot;
    let mut terrain_upload = TerrainUploadState::default();
    let mut item_texture_animation = ItemTextureAnimationState::default();
    let client_animation_ticks = ClientAnimationTickState::default();
    let camera_fov_ticks =
        CameraFovTickState::new(f32::from(args.client_fov), args.client_fov_effect_scale);
    let camera_eye_height_ticks = CameraEyeHeightTickState::default();
    let camera_spinning_ticks = runtime::CameraSpinningEffectTickState::default();
    let mut hud_debug_fps_sampler = HudDebugFpsSampler::default();
    let hud_debug_network_sampler = HudDebugNetworkSampler::default();
    let hud_debug_tps_sampler = HudDebugTpsSampler::default();
    let mut debug_collision_cache = DebugCollisionCache::default();
    let mut debug_supporting_block_cache = DebugSupportingBlockCache::default();
    let mut debug_sky_light_section_cache = DebugSkyLightSectionCache::default();
    let mut remote_debug_selection_cache = RemoteDebugSelectionCache::default();
    let mut debug_profiler = DebugProfiler::with_report_writer(
        input::DEBUG_PROFILING_RESULTS_RELATIVE_DIR,
        args.server.clone(),
    );
    let mut frame_phase_diagnostics = FramePhaseDiagnostics::default();
    let mut last_profiled_simulation_generation = None;
    let mut latest_simulation_profile_sections = Vec::<runtime::SimulationProfileSection>::new();
    let lightmap_ticks = LightmapTickState::with_brightness_factor_and_hide_lightning_flash(
        args.client_gamma,
        args.hide_lightning_flash,
    );
    let level_event_sound_random = LevelEventSoundRandomState::with_seed(world.next_sound_seed());
    let mut cursor_captured = false;
    let mut frame_interval = client_framerate_limit_interval(args.client_framerate_limit);
    let mut latest_input_at = Instant::now();
    let mut next_redraw_at = Instant::now();
    let mut last_command_suggestion_narration = None;
    let mut player_list_narration_visible = false;
    let mut debug_options_screen_narration = ScreenNarrationRuntime::default();
    let mut stats_screen_narration = ScreenNarrationRuntime::default();
    let mut recipe_book_narration = ScreenNarrationRuntime::default();
    let mut social_interactions_narration = ScreenNarrationRuntime::default();
    let mut player_reporting_narration = ScreenNarrationRuntime::default();
    let mut death_screen_narration = ScreenNarrationRuntime::default();
    let mut disconnected_screen_narration = ScreenNarrationRuntime::default();
    let mut chat_screen_narration = ScreenNarrationRuntime::default();
    let mut game_event_screen_narration = ScreenNarrationRuntime::default();
    let mut pending_control_screenshots = std::collections::VecDeque::new();
    let mut platform_exit_requested = false;
    let mut render_effect_consumer = RenderEffectConsumer::default();
    let mut hud_render_runtime = HudRenderRuntime::default();
    hud_render_runtime.set_panorama_scroll_speed(args.panorama_scroll_speed);
    if let Some(roots) = pack_roots.as_ref() {
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
                Instant::now(),
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
        window.inner_size(),
        false,
        startup_client_information,
    );
    simulation_input_runtime
        .set_gui_scale_options(args.client_gui_scale, args.client_force_unicode_font);
    let mut renderer_counters = control_renderer_counters(renderer.counters());
    project_dynamic_texture_counters(&mut renderer_counters, item_runtime.as_ref());
    let publisher = SimulationPublisher::new(
        SimulationPublicationInput {
            effect_sequence: 0,
            world: &world,
            input: &input,
            first_person_hand_presentation: Default::default(),
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
            camera_eye_height_ticks,
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
        services_block_lists,
        input,
        first_person_hand_presentation: Default::default(),
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
        environment_audio: environment_audio_runtime::EnvironmentAudioRuntime::default(),
        environment_animate: environment_animate_runtime::EnvironmentAnimateRuntime::default(),
        weather_effects: weather_effect_runtime::WeatherEffectRuntime::new_with_radius(
            client_particle_status,
            args.client_weather_radius,
        ),
        music: MusicRuntime::configured(
            args.client_music_frequency,
            args.client_audio_volumes
                .final_source_volume(&bbb_audio::AudioCategory::Music),
        ),
        win_screen_resources,
        particle_runtime,
        terrain_textures: terrain_textures.clone(),
        item_runtime: simulation_item_runtime,
        code_of_conduct: code_of_conduct_acceptance,
        server_list,
        command_mailbox: simulation_mailbox.clone(),
        input_runtime: simulation_input_runtime,
        level_load_runtime: LevelLoadRuntime::default(),
        renderer_counters,
        audio_counters: audio_status.clone(),
        publisher,
        audio_enabled: audio_runtime.is_some(),
        hide_lightning_flash: args.hide_lightning_flash,
        next_effect_sequence: 1,
    };
    let simulation_event_proxy = event_loop.create_proxy();
    let mut simulation_thread = SimulationThread::spawn(simulation, move || {
        let _ = simulation_event_proxy.send_event(());
    })?;
    let mut render_publication = shared_publication.load_full();
    let mut last_simulation_output_telemetry = None;
    let mut event_cursor_position = window_surface::CursorPositionCache::default();
    let mut current_cursor_icon = CursorIcon::Default;
    let mut current_window_title = None;
    let fatal_failures = Rc::new(RefCell::new(RuntimeFailureAccumulator::default()));
    // `Minecraft.fillSystemReport` reports the options the failing run used, so
    // capture them once here rather than reading a file mid-failure.
    if let Some(options) = client_options.as_mut() {
        if let Some(reconciliation) = resource_pack_options_reconciliation.as_ref() {
            options.apply_resource_pack_reconciliation(reconciliation);
        }
        fatal_failures
            .borrow_mut()
            .attach_client_options(options.options().dump_for_report());
        let rebound: Vec<_> = client_options::key_bindings::rebound_keys(options.options())
            .into_iter()
            .filter(|(name, _)| name.starts_with("key.debug."))
            .collect();
        if !rebound.is_empty() {
            tracing::warn!(
                bindings = ?rebound,
                "options.txt rebinds debug keys this client still reads at their Vanilla defaults"
            );
        }
        if let Err(error) = options.save_started_cleanly(true) {
            tracing::warn!(%error, "failed to mark initial client load complete");
        }
    }
    if !network_runtime.activate() {
        tracing::warn!("network runtime ended before initial-load activation");
    }
    let event_failures = Rc::clone(&fatal_failures);

    let event_loop_result = event_loop.run(move |event, target| {
        if matches!(&event, Event::LoopExiting) {
            set_cursor_capture(&window, &mut cursor_captured, false);
            if let Err(error) = simulation_thread.shutdown_and_join() {
                record_runtime_failure(
                    &event_failures,
                    error,
                    Some(render_publication.world.as_ref()),
                );
            }
            if let Err(error) = network_runtime.shutdown_and_join(&runtime) {
                record_runtime_failure(
                    &event_failures,
                    RuntimeFailure::new(
                        "network shutdown",
                        "Network backend shutdown failed",
                        error,
                    ),
                    Some(render_publication.world.as_ref()),
                );
            }
            snapshot.rcu(|current| {
                let mut next = (**current).clone();
                next.app.running = false;
                next.net.connected = false;
                next
            });
            return;
        }
        #[cfg(feature = "lifecycle-soak")]
        if let Some(runtime) = lifecycle_soak.as_mut() {
            runtime.apply_due(Instant::now(), &window, &mut renderer);
        }
        if matches!(&event, Event::NewEvents(_)) {
            if let Some(audio) = audio_runtime.as_mut() {
                audio.poll_audio_device();
            }
        }
        loop {
            // A reload is the terminal effect of its simulation batch. Apply
            // that CPU/GPU/audio transaction and queue its acknowledgement
            // before consuming the next batch, so a later disconnect or
            // replacement generation can never overtake the barrier.
            let resource_reload = loop {
                let simulation_step = match simulation_thread.try_recv() {
                    Ok(step) => step,
                    Err(std::sync::mpsc::TryRecvError::Empty) => break None,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => break None,
                };
                let render_effect_result = {
                    let mut render_effect_sink = NativeRenderEffectSink {
                        renderer: &mut renderer,
                        window: &window,
                        cursor_captured: &mut cursor_captured,
                        vsync_enabled: args.client_vsync,
                        preferred_video_mode: args.client_fullscreen_resolution,
                        exclusive_fullscreen_supported: window_backend
                            .supports_exclusive_fullscreen(),
                        mailbox: &simulation_mailbox,
                        clipboard: &mut debug_clipboard,
                        terrain_upload: &mut terrain_upload,
                        client_locale: &mut client_locale,
                        debug_profiler: &mut debug_profiler,
                        debug_options_screen_narration: &mut debug_options_screen_narration,
                        stats_screen_narration: &mut stats_screen_narration,
                        recipe_book_narration: &mut recipe_book_narration,
                        social_interactions_narration: &mut social_interactions_narration,
                        player_reporting_narration: &mut player_reporting_narration,
                        death_screen_narration: &mut death_screen_narration,
                        disconnected_screen_narration: &mut disconnected_screen_narration,
                        chat_screen_narration: &mut chat_screen_narration,
                        game_event_screen_narration: &mut game_event_screen_narration,
                        narrator: narrator_runtime
                            .as_mut()
                            .map(|narrator| narrator as &mut dyn narrator_runtime::SystemNarrator),
                        control_screenshots: &mut pending_control_screenshots,
                        game_dir: args.game_dir.as_path(),
                        client_options: client_options.as_mut(),
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
                    Ok(Some(reload)) => break Some(reload),
                    Ok(None) => {}
                    Err(error) => {
                        record_runtime_failure(
                            &event_failures,
                            RuntimeFailure::new(
                                "render-effect apply",
                                "Fatal simulation render-effect sequence gap",
                                error,
                            ),
                            Some(render_publication.world.as_ref()),
                        );
                        target.exit();
                        return;
                    }
                }
            };
            let Some(resource_reload) = resource_reload else {
                break;
            };
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
                &mut hud_render_runtime,
                &mut renderer,
                &client_locale,
                FontOptions {
                    uniform: args.client_force_unicode_font,
                    japanese_variants: args.client_japanese_glyph_variants,
                },
                client_particle_status,
                &player_skin_cache_dir,
                &args.username,
            );
            non_entity_render_extraction.invalidate_resources();
            simulation_mailbox.enqueue_or_report(
                SimulationCommand::TextMeasurements(renderer.hud_text_measurements()),
                "reload text measurements",
            );
            simulation_mailbox.enqueue_or_report(
                SimulationCommand::ResourceReloadAck(Box::new(ack)),
                "resource reload acknowledgement",
            );
        }
        // Output transport is live even when the compositor suppresses redraws
        // for an occluded surface. Sample it after the complete drain rather
        // than tying its terminal gauge to renderer-frame publication.
        let drained_output_backlog = simulation_thread.output_backlog();
        if last_simulation_output_telemetry != Some(drained_output_backlog) {
            simulation_mailbox.update_simulation_output_telemetry(
                drained_output_backlog.render_wakes_sent as u64,
                drained_output_backlog.render_wakes_coalesced as u64,
                drained_output_backlog.pending as u64,
                drained_output_backlog.peak as u64,
            );
            last_simulation_output_telemetry = Some(drained_output_backlog);
        }
        let latest_publication = shared_publication.load_full();
        if latest_publication.effect_sequence <= render_effect_consumer.last_sequence() {
            render_publication = latest_publication;
        }
        // `Minecraft.resizeGui` (Minecraft.java:1433-1442) makes the
        // GUI-scaled surface observable to HUD layout and hit testing; the
        // renderer owns it so native event handling and draw extraction share
        // one logical size while `surface_size()` stays physical.
        renderer.set_gui_surface_size(PhysicalSize::new(
            render_publication.input_runtime.gui_scaled_width,
            render_publication.input_runtime.gui_scaled_height,
        ));
        if let Some(result) = simulation_thread.poll_finished() {
            if let Err(error) = result {
                record_runtime_failure(
                    &event_failures,
                    error,
                    Some(render_publication.world.as_ref()),
                );
            }
            target.exit();
            return;
        }
        if platform_exit_requested || !render_publication.running {
            target.exit();
            return;
        }
        update_window_title(
            &window,
            render_publication.net_counters.connected,
            &mut current_window_title,
        );
        if let Some(narrator) = narrator_runtime.as_mut() {
            narrator.tick();
        }
        let published_input = render_publication.input.as_state();
        let published_world = render_publication.world.as_ref();
        narrate_player_list_visibility_change(
            hud_player_list_is_visible(published_input, published_world),
            &mut player_list_narration_visible,
            narrator_runtime
                .as_mut()
                .map(|narrator| narrator as &mut dyn narrator_runtime::SystemNarrator),
            || hud_player_list_narration_from_world(published_world, item_runtime.as_ref()),
        );
        narrate_command_suggestion_change(
            (!published_input.chat_entry_is_active())
                .then(|| render_publication.input.command_suggestion_narration())
                .flatten(),
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
        // Vanilla `FramerateLimitTracker.onInputReceived`: `KeyboardHandler`
        // and `MouseHandler` stamp the AFK clock on every key, click, scroll
        // and cursor move.
        if matches!(
            &event,
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
        ) {
            latest_input_at = Instant::now();
        }
        match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => match event {
                // This event arm is also `Window.resetIsResized`'s stand-in:
                // vanilla latches `isResized` for `runTick` to poll and clear
                // once per frame, while winit delivers each resize as an event
                // consumed exactly once right here.
                WindowEvent::Resized(size) => {
                    let old_size = renderer.physical_surface_size();
                    if let Err(error) = renderer.resize(size) {
                        record_runtime_failure(
                            &event_failures,
                            // `Window.onFramebufferResize` failure reporting
                            // keeps the old/new dimensions in its crash
                            // category ("Window Dimensions"); carry both here.
                            RuntimeFailure::new(
                                "renderer resize",
                                format!(
                                    "Failed to resize renderer resources from {}x{} to {}x{}",
                                    old_size.width, old_size.height, size.width, size.height
                                ),
                                error,
                            ),
                            Some(render_publication.world.as_ref()),
                        );
                        target.exit();
                        return;
                    }
                    // `Window.onFramebufferResize` (Window.java:267-297)
                    // synchronously calls `Minecraft.resizeGui`
                    // (Minecraft.java:1434-1442) before the resize callback
                    // returns, so the logical GUI projection is established at
                    // event time and any later mouse event consumer reads the
                    // matching ratio. Publish it here before the simulation
                    // command is queued; a zero-size/minimized callback keeps
                    // the last usable physical and logical sizes, exactly like
                    // the simulation's own `SurfaceResized` handling.
                    if size.width != 0 && size.height != 0 {
                        renderer.set_gui_surface_size(resize_gui_surface(
                            size,
                            args.client_gui_scale,
                            args.client_force_unicode_font,
                        ));
                    }
                    simulation_mailbox.enqueue_or_report(
                        SimulationCommand::SurfaceResized(size),
                        "surface resize",
                    );
                }
                event
                    if enqueue_simulation_window_event(
                        &event,
                        &simulation_mailbox,
                        &mut debug_clipboard,
                        &renderer,
                        &window,
                        &mut event_cursor_position,
                        args.client_allow_cursor_changes,
                        &mut current_cursor_icon,
                    ) => {}
                WindowEvent::RedrawRequested => {
                    let profiler_frame_started_at = Instant::now();
                    let profiler_enabled = debug_profiler.is_recording()
                        || render_publication.input.debug_profiler_chart_visible();
                    if !profiler_enabled {
                        last_profiled_simulation_generation = None;
                        latest_simulation_profile_sections.clear();
                    }
                    let gpu_timing_enabled = debug_profiler.is_recording()
                        || render_publication.input.debug_screen_entry_enabled(
                            DebugScreenEntryId::GpuUtilization,
                            render_publication
                                .input
                                .show_only_reduced_info(&render_publication.world),
                        );
                    renderer.set_gpu_timing_enabled(gpu_timing_enabled);
                    frame_phase_diagnostics.set_enabled(gpu_timing_enabled);
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
                    let native_render_profile = update_renderer_from_publication(
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
                            camera_environment_samples: &mut camera_environment_samples,
                            item_runtime: item_runtime.as_ref(),
                            hud_render_runtime: &mut hud_render_runtime,
                            hud_debug_fps_sampler: &mut hud_debug_fps_sampler,
                            debug_collision_cache: &mut debug_collision_cache,
                            debug_supporting_block_cache: &mut debug_supporting_block_cache,
                            debug_sky_light_section_cache: &mut debug_sky_light_section_cache,
                            remote_debug_selection_cache: &mut remote_debug_selection_cache,
                            render_distance_chunks: args.render_distance_chunks,
                            prioritize_chunk_updates: match args.prioritize_chunk_updates {
                                startup::PrioritizeChunkUpdatesArg::None => {
                                    terrain_runtime::TerrainPrioritizeChunkUpdates::None
                                }
                                startup::PrioritizeChunkUpdatesArg::PlayerAffected => {
                                    terrain_runtime::TerrainPrioritizeChunkUpdates::PlayerAffected
                                }
                                startup::PrioritizeChunkUpdatesArg::Nearby => {
                                    terrain_runtime::TerrainPrioritizeChunkUpdates::Nearby
                                }
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
                                .final_source_volume(&bbb_audio::AudioCategory::Music),
                            entity_shadows: args.entity_shadows,
                            high_contrast_block_outline: args.high_contrast_block_outline,
                            improved_transparency: args.improved_transparency,
                            hud_debug_profiler_chart: debug_profiler.chart(),
                        },
                    );
                    let subtitle_now = Instant::now();
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
                            runtime::audio_listener_state_from_world(&render_publication.world),
                            args.notification_display_time,
                            subtitle_now,
                            |key| {
                                item_runtime
                                    .as_ref()
                                    .map(|runtime| runtime.localized_text(key))
                                    .unwrap_or(key)
                                    .to_string()
                            },
                        )
                    } else {
                        bbb_renderer::HudSoundSubtitles::default()
                    };
                    renderer.set_hud_sound_subtitles(sound_subtitles);
                    if let Some(items) = item_runtime.as_ref() {
                        maybe_upload_item_texture_animation(
                            &mut renderer,
                            &mut item_texture_animation,
                            items,
                        );
                    }
                    if let Some(particles) = particle_render_runtime.as_mut() {
                        particles.maybe_upload_particle_atlas_animation(&mut renderer);
                    }
                    debug_options_screen_narration.tick(
                        render_publication
                            .input
                            .debug_options_screen_is_open()
                            .then(|| hud_render_runtime.debug_options_screen_narration())
                            .flatten(),
                        Instant::now(),
                        render_publication.input.last_input_type(),
                        narrator_runtime
                            .as_mut()
                            .map(|narrator| narrator as &mut dyn narrator_runtime::SystemNarrator),
                    );
                    social_interactions_narration.tick(
                        render_publication
                            .world
                            .social_interactions_screen()
                            .is_some()
                            .then(|| hud_render_runtime.social_interactions_narration())
                            .flatten(),
                        Instant::now(),
                        render_publication.input.last_input_type(),
                        narrator_runtime
                            .as_mut()
                            .map(|narrator| narrator as &mut dyn narrator_runtime::SystemNarrator),
                    );
                    player_reporting_narration.tick(
                        render_publication
                            .world
                            .player_reporting_screen()
                            .is_some()
                            .then(|| hud_render_runtime.player_reporting_narration())
                            .flatten(),
                        Instant::now(),
                        render_publication.input.last_input_type(),
                        narrator_runtime
                            .as_mut()
                            .map(|narrator| narrator as &mut dyn narrator_runtime::SystemNarrator),
                    );
                    stats_screen_narration.tick(
                        render_publication
                            .world
                            .stats_screen_is_open()
                            .then(|| hud_render_runtime.stats_screen_narration())
                            .flatten(),
                        Instant::now(),
                        render_publication.input.last_input_type(),
                        narrator_runtime
                            .as_mut()
                            .map(|narrator| narrator as &mut dyn narrator_runtime::SystemNarrator),
                    );
                    recipe_book_narration.tick(
                        hud_render_runtime.recipe_book_narration(),
                        Instant::now(),
                        render_publication.input.last_input_type(),
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
                        render_publication.input.last_input_type(),
                        narrator_runtime
                            .as_mut()
                            .map(|narrator| narrator as &mut dyn narrator_runtime::SystemNarrator),
                    );
                    disconnected_screen_narration.tick(
                        render_publication
                            .input
                            .disconnected_screen_runtime()
                            .is_some()
                            .then(|| hud_render_runtime.disconnected_screen_narration())
                            .flatten(),
                        Instant::now(),
                        render_publication.input.last_input_type(),
                        narrator_runtime
                            .as_mut()
                            .map(|narrator| narrator as &mut dyn narrator_runtime::SystemNarrator),
                    );
                    chat_screen_narration.tick(
                        hud_render_runtime.chat_screen_narration(),
                        Instant::now(),
                        render_publication.input.last_input_type(),
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
                        render_publication.input.last_input_type(),
                        narrator_runtime
                            .as_mut()
                            .map(|narrator| narrator as &mut dyn narrator_runtime::SystemNarrator),
                    );
                    let dialog_preedit = render_publication.input.dialog_ime_preedit_hud_state();
                    let sign_preedit_area =
                        renderer.hud_sign_editor_ime_cursor_area(renderer.gui_surface_size());
                    let dialog_preedit_area = || {
                        dialog_ime_cursor_area(
                            &render_publication.world,
                            dialog_preedit.as_ref(),
                            renderer.gui_surface_size(),
                            &renderer,
                        )
                    };
                    // The IME caret comes from logical GUI layout; the native
                    // cursor rectangle is physical, so convert back at the
                    // platform boundary (`Window.guiScaledWidth` ratio).
                    let logical_ime_area = sign_preedit_area.or_else(dialog_preedit_area);
                    if let Some((position, size)) = logical_ime_area.and_then(|(position, size)| {
                        window_surface::gui_rect_to_physical(
                            position,
                            size,
                            renderer.gui_surface_size(),
                            window.inner_size(),
                        )
                    }) {
                        window.set_ime_cursor_area(position, size);
                    }
                    let profiler_update_duration = profiler_update_started_at
                        .elapsed()
                        .saturating_sub(native_render_profile.extract)
                        .saturating_sub(native_render_profile.toasts);
                    let code_of_conduct_screen =
                        code_of_conduct_overlay::code_of_conduct_renderer_screen(
                            &render_publication.world,
                            render_publication.input_runtime.code_of_conduct_overlay,
                            render_publication.code_of_conduct_accepted,
                            PhysicalSize::new(
                                render_publication.input_runtime.gui_scaled_width,
                                render_publication.input_runtime.gui_scaled_height,
                            ),
                            renderer.hud_text_measurements_ref(),
                        );
                    renderer.set_hud_code_of_conduct_screen(code_of_conduct_screen);

                    let terrain_ready_for_screenshot =
                        !screenshot_after_terrain || terrain_upload.has_uploaded_chunks();
                    let cli_screenshot_path = screenshot
                        .as_deref()
                        .filter(|_| terrain_ready_for_screenshot);
                    let control_screenshot = if cli_screenshot_path.is_none() {
                        pending_control_screenshots.pop_front()
                    } else {
                        None
                    };
                    let render_path = cli_screenshot_path.or(control_screenshot
                        .as_ref()
                        .map(|pending| pending.path.as_path()));
                    let wrote_cli_screenshot = cli_screenshot_path.is_some();

                    let profiler_render_started_at = Instant::now();
                    if let Err(err) = renderer.render(render_path) {
                        record_runtime_failure(
                            &event_failures,
                            RuntimeFailure::new(
                                "renderer frame",
                                "Renderer frame failed",
                                format_args!("{err:#}"),
                            ),
                            Some(render_publication.world.as_ref()),
                        );
                        target.exit();
                        return;
                    }
                    let profiler_render_duration = profiler_render_started_at.elapsed();
                    let renderer_frame_profile = renderer.last_frame_profile();
                    if let Some(pending) = control_screenshot
                        .as_ref()
                        .filter(|pending| pending.announce)
                    {
                        simulation_mailbox.enqueue_or_report(
                            SimulationCommand::ScreenshotSaved {
                                name: pending
                                    .path
                                    .file_name()
                                    .map(|name| name.to_string_lossy().into_owned())
                                    .unwrap_or_default(),
                                error: None,
                            },
                            "screenshot saved",
                        );
                    }

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
                    let profiler_frame_metrics = profiler_enabled.then(|| {
                        DebugProfilerFrameMetrics::from_renderer(
                            std::time::Duration::ZERO,
                            &renderer_counters,
                            args.render_distance_chunks,
                            renderer.gpu_utilization(),
                        )
                    });
                    simulation_mailbox.update_backend_telemetry(renderer_counters, audio_counters);
                    // Preserve the backlog observed while a frame was in
                    // flight; the next event-loop drain replaces `current`
                    // with zero without requiring another redraw.
                    if last_simulation_output_telemetry != Some(output_backlog) {
                        simulation_mailbox.update_simulation_output_telemetry(
                            output_backlog.render_wakes_sent as u64,
                            output_backlog.render_wakes_coalesced as u64,
                            output_backlog.pending as u64,
                            output_backlog.peak as u64,
                        );
                        last_simulation_output_telemetry = Some(output_backlog);
                    }
                    let profiler_publish_duration = profiler_publish_started_at.elapsed();
                    let profiler_frame_work_duration = profiler_frame_started_at.elapsed();
                    frame_phase_diagnostics.record(FramePhaseSample {
                        frame_interval: renderer_frame_profile.frame_interval,
                        frame_work: profiler_frame_work_duration,
                        update: profiler_update_duration,
                        extract: native_render_profile.extract,
                        render_call: profiler_render_duration,
                        surface_acquire: renderer_frame_profile.surface_acquire,
                        render_encode: renderer_frame_profile.render,
                        queue_submit: renderer_frame_profile.present,
                        swap_buffers: renderer_frame_profile.swap_buffers,
                        gpu_poll: renderer_frame_profile.gpu_async,
                        publish: profiler_publish_duration,
                    });
                    let profiler_frame_duration = renderer_frame_profile
                        .frame_interval
                        .max(profiler_frame_work_duration);
                    let profiler_frame_limiter_duration =
                        profiler_frame_duration.saturating_sub(profiler_frame_work_duration);
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
                    if profiler_frame_work_duration > std::time::Duration::from_secs(1) {
                        tracing::warn!(
                            frame_millis = profiler_frame_work_duration.as_millis() as u64,
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
                    if let Some(mut profiler_frame_metrics) = profiler_frame_metrics {
                        profiler_frame_metrics.tick_time_nanos =
                            profiler_frame_duration.as_nanos() as f64;
                        let simulation_generation_changed = last_profiled_simulation_generation
                            != Some(render_publication.generation);
                        if simulation_generation_changed {
                            latest_simulation_profile_sections = render_publication
                                .frame
                                .as_ref()
                                .map(|frame| frame.profiling_sections().to_vec())
                                .unwrap_or_default();
                            last_profiled_simulation_generation =
                                Some(render_publication.generation);
                        }
                        let mut sections =
                            Vec::with_capacity(latest_simulation_profile_sections.len() + 10);
                        sections.extend(latest_simulation_profile_sections.iter().map(|section| {
                            DebugProfilerSectionSample {
                                path: section.path,
                                duration: section.duration,
                                record_in_report: simulation_generation_changed,
                            }
                        }));
                        sections.extend([
                            DebugProfilerSectionSample {
                                path: &["toasts"],
                                duration: native_render_profile.toasts,
                                record_in_report: true,
                            },
                            DebugProfilerSectionSample {
                                path: &["frame"],
                                duration: profiler_frame_duration
                                    .saturating_sub(native_render_profile.toasts),
                                record_in_report: true,
                            },
                            DebugProfilerSectionSample {
                                path: &["frame", "update"],
                                duration: profiler_update_duration,
                                record_in_report: true,
                            },
                            DebugProfilerSectionSample {
                                path: &["frame", "extract"],
                                duration: native_render_profile.extract,
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
                        debug_profiler.record_frame(
                            Instant::now(),
                            profiler_frame_duration,
                            &sections,
                            profiler_frame_metrics,
                        );
                    }
                    for event in debug_profiler.take_events() {
                        let lifecycle = match event {
                            DebugProfilerEvent::RecordingStarted => {
                                tracing::info!("native profiling started");
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
                                    "native profiling stopped"
                                );
                                DebugProfilingLifecycle::Stopped {
                                    duration,
                                    ticks,
                                    ticks_per_second,
                                }
                            }
                            DebugProfilerEvent::ReportSaved { path } => {
                                tracing::info!(
                                    path = %path.display(),
                                    "native profiling report saved"
                                );
                                DebugProfilingLifecycle::ReportSaved { path }
                            }
                            DebugProfilerEvent::ReportFailed { error } => {
                                tracing::warn!(error, "native profiling report failed");
                                DebugProfilingLifecycle::ReportFailed { error }
                            }
                        };
                        simulation_mailbox.enqueue_or_report(
                            SimulationCommand::DebugProfilingLifecycle(lifecycle),
                            "debug profiling lifecycle",
                        );
                    }
                    // Vanilla samples `FramerateLimitTracker.getFramerateLimit`
                    // every frame, so the cap follows window/AFK/menu state.
                    frame_interval = client_framerate_limit_interval(effective_framerate_limit(
                        args.client_framerate_limit,
                        args.inactivity_fps_limit,
                        window.is_minimized().unwrap_or(false),
                        latest_input_at.elapsed().as_millis(),
                        render_publication.world.level_info().is_some(),
                        input_screen_is_open(
                            render_publication.input.as_state(),
                            &render_publication.world,
                        ),
                    ));
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
                    simulation_mailbox.enqueue_or_report(
                        SimulationCommand::MouseMotion { delta },
                        "device mouse motion",
                    );
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
    });
    if let Err(error) = event_loop_result {
        record_runtime_failure(
            &fatal_failures,
            RuntimeFailure::new("platform event loop", "Platform event loop failed", error),
            None,
        );
    }
    if let Some(failures) = fatal_failures.borrow_mut().take() {
        return Err(anyhow::Error::new(failures));
    }

    Ok(())
}

fn record_runtime_failure(
    failures: &Rc<RefCell<RuntimeFailureAccumulator>>,
    failure: RuntimeFailure,
    world: Option<&WorldStore>,
) {
    tracing::error!(%failure, "fatal runtime failure");
    let mut failures = failures.borrow_mut();
    failures.record(failure);
    if let Some(world) = world {
        tracing::error!(
            level = world.client_level_identity(),
            "attaching level crash-report details"
        );
        failures.attach_level_details(world.client_level_report_details());
    }
}

fn set_cursor_capture(window: &Window, captured: &mut bool, capture: bool) {
    if *captured == capture {
        return;
    }

    if capture {
        // `Minecraft.mouseHandler.grabMouse`-equivalent capture: request
        // `Locked` first and fall back to `Confined` where the backend does
        // not implement locking. Neither mode is a raw-motion switch — winit's
        // raw `DeviceEvent::MouseMotion` is always delivered (see the startup
        // `rawMouseInput` warning).
        //
        // Three GLFW-era `grabMouse`/`releaseMouse` steps are structurally
        // unnecessary here: the `setCursorPosition(width/2, height/2)` warp
        // (Wayland cannot warp the pointer and `Locked` pins it in place),
        // `mouseHandler.setIgnoreFirstMove` (turn deltas are relative device
        // motion, so a re-grab cannot inject an absolute jump — the same
        // reason `cursorEntered`'s only vanilla consumer has no equivalent),
        // and `KeyMapping.setAll`'s `isKeyDown` re-poll (`pressed_keys`
        // tracks every raw edge whether or not a screen consumed it, so key
        // state never desyncs and there is nothing to re-poll).
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

/// `Window.selectCursor` (Window.java:535-540) applied through winit: the
/// requested icon is gated by `allowCursorChanges` and redundant native cursor
/// changes are skipped.
fn apply_window_cursor_icon(
    window: &Window,
    allow_cursor_changes: bool,
    current: &mut CursorIcon,
    requested: CursorIcon,
) {
    if let Some(next) = window_surface::select_cursor(allow_cursor_changes, requested, *current) {
        window.set_cursor_icon(next);
        *current = next;
    }
}

/// The one transition policy shared by the startup apply and later
/// `SetFullscreen` effects (`Window.setMode`, Window.java:350-389): enter
/// exclusive mode only when the stored `fullscreenResolution` exactly matches
/// a winit video mode on a backend that supports exclusive mode, otherwise
/// borderless on the current monitor. Windowed restoration is winit-owned
/// (`set_fullscreen(None)` restores the saved windowed geometry). Winit has no
/// synchronous failure channel, so the mode choice is decided up front rather
/// than by inspecting `Window::fullscreen()` afterwards.
fn set_window_fullscreen(
    window: &Window,
    fullscreen: bool,
    preferred: Option<window_surface::PreferredVideoMode>,
    exclusive_supported: bool,
) {
    if !fullscreen {
        window.set_fullscreen(None);
        return;
    }
    let monitor = window.current_monitor();
    let exact =
        preferred.and_then(|mode| window_surface::find_exact_video_mode(mode, monitor.as_ref()));
    match window_surface::fullscreen_choice(preferred, exact.is_some(), exclusive_supported) {
        window_surface::FullscreenChoice::Exclusive => {
            window.set_fullscreen(Some(Fullscreen::Exclusive(
                exact.expect("exclusive choice requires an exact video mode"),
            )));
        }
        window_surface::FullscreenChoice::Borderless => {
            window.set_fullscreen(Some(Fullscreen::Borderless(None)));
        }
    }
}

/// `Minecraft.<init>` (Minecraft.java:647-656): a clean start with
/// `options.fullscreen` applies fullscreen before the event loop through the
/// shared transition policy. There is no observable failure branch on this
/// backend (`Window::set_fullscreen` returns `()`), so `input.fullscreen`
/// keeps the stored intent and `Minecraft.onFullscreenError` stays
/// unreachable (see `/tmp/window-surface-work/audit.md`).
fn apply_startup_fullscreen(
    window: &Window,
    input: &mut ClientInputState,
    preferred: Option<window_surface::PreferredVideoMode>,
    exclusive_supported: bool,
) {
    if !input.is_fullscreen() {
        return;
    }
    set_window_fullscreen(window, true, preferred, exclusive_supported);
}

/// `Minecraft.resizeGui` (Minecraft.java:1434-1442) computed for a fresh
/// physical framebuffer: `Window.calculateScale` plus `Window.setGuiScale`'s
/// ceiling division, through the shared pure `window_surface` functions. The
/// resize arm calls this synchronously so the renderer's logical GUI surface
/// is current before any later window event (for example a `MouseInput`
/// projection) is handled; the simulation publishes the same value later.
fn resize_gui_surface(
    size: PhysicalSize<u32>,
    gui_scale_max: u32,
    enforce_unicode: bool,
) -> PhysicalSize<u32> {
    let gui_scale = window_surface::calculate_gui_scale(
        gui_scale_max,
        enforce_unicode,
        size.width,
        size.height,
    );
    let (gui_scaled_width, gui_scaled_height) =
        window_surface::gui_scaled_size(size.width, size.height, gui_scale);
    PhysicalSize::new(gui_scaled_width, gui_scaled_height)
}

/// `Minecraft.updateTitle` (Minecraft.java:802-803): recomputes `createTitle`
/// and applies it whenever the connection state changes; BBB diffs so an
/// unchanged state does not spam the native window.
fn update_window_title(window: &Window, connected: bool, current: &mut Option<String>) {
    let next = window_surface::create_title(bbb_protocol::MC_VERSION, connected);
    if current.as_deref() != Some(next.as_str()) {
        window.set_title(&next);
        *current = Some(next);
    }
}

/// Translates winit window events into `SimulationCommand`s — the
/// `KeyboardHandler.setup`/`MouseHandler.setup` GLFW-callback equivalent.
/// Two vanilla callbacks are deliberately absent. `WindowEvent::DroppedFile`
/// cannot provide `MouseHandler.onDrop`'s batch-local failed path count: winit
/// 0.29.15 flattens successes into individual events and exposes no failure or
/// batch receipt. The exact failed-file-drop capability receipt is generated at
/// `docs/evidence/vanilla-26.1/file-drop-capability.json`; on the declared
/// Linux/Wayland + Java 25 path, official LWJGL decoding makes that failure
/// branch unreachable and the only valid-path consumer is an excluded
/// pack-selection screen. `WindowEvent::CursorEntered` is also absent
/// (`cursorEntered`'s only consumer is `setIgnoreFirstMove`, which relative
/// turn deltas make structurally unnecessary — see `set_cursor_capture`).
fn enqueue_simulation_window_event(
    event: &WindowEvent,
    mailbox: &SimulationMailbox,
    clipboard: &mut NativeDebugClipboard,
    renderer: &bbb_renderer::Renderer,
    window: &Window,
    cursor_position: &mut window_surface::CursorPositionCache,
    allow_cursor_changes: bool,
    current_cursor_icon: &mut CursorIcon,
) -> bool {
    let command = match event {
        WindowEvent::CloseRequested => SimulationCommand::Shutdown,
        WindowEvent::CursorMoved { position, .. } => {
            // `MouseHandler.onMove` (MouseHandler.java:266-279) stores raw
            // physical coordinates; `getScaledXPos/getScaledYPos` project
            // them onto the logical GUI surface at the boundary. The cache
            // stays physical so a later resize never reuses a stale
            // projection.
            cursor_position.on_cursor_moved(*position);
            let gui_position = cursor_position
                .project(
                    renderer.physical_surface_size(),
                    renderer.gui_surface_size(),
                )
                .expect("cursor position cached by CursorMoved");
            let requested = if renderer
                .hud_disconnected_screen_hit_target_at((gui_position.x, gui_position.y))
                .is_some()
                || renderer
                    .hud_death_screen_hit_target_at((gui_position.x, gui_position.y))
                    .is_some()
                || renderer
                    .hud_game_event_hit_target_at((gui_position.x, gui_position.y))
                    .is_some()
                || renderer
                    .hud_social_interactions_hit_target_at((gui_position.x, gui_position.y))
                    .is_some()
                || renderer
                    .hud_player_reporting_hit_target_at((gui_position.x, gui_position.y))
                    .is_some()
                || renderer
                    .hud_book_hit_target_at((gui_position.x, gui_position.y))
                    .is_some()
                || renderer
                    .hud_chat_hit_target_at((gui_position.x, gui_position.y))
                    .is_some()
                || renderer
                    .hud_pause_draft_hit_target_at((gui_position.x, gui_position.y))
                    .is_some()
            {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            };
            apply_window_cursor_icon(window, allow_cursor_changes, current_cursor_icon, requested);
            SimulationCommand::CursorMoved {
                position: gui_position,
                disconnected_screen_target: renderer
                    .hud_disconnected_screen_hit_target_at((gui_position.x, gui_position.y)),
                death_screen_target: renderer
                    .hud_death_screen_hit_target_at((gui_position.x, gui_position.y)),
                game_event_target: renderer
                    .hud_game_event_hit_target_at((gui_position.x, gui_position.y)),
                player_reporting_target: renderer
                    .hud_player_reporting_hit_target_at((gui_position.x, gui_position.y)),
                social_interactions_target: renderer
                    .hud_social_interactions_hit_target_at((gui_position.x, gui_position.y)),
                pause_draft_target: renderer
                    .hud_pause_draft_hit_target_at((gui_position.x, gui_position.y)),
            }
        }
        WindowEvent::CursorLeft { .. } => {
            cursor_position.on_cursor_left();
            apply_window_cursor_icon(
                window,
                allow_cursor_changes,
                current_cursor_icon,
                CursorIcon::Default,
            );
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
        WindowEvent::MouseInput { state, button, .. } => {
            // `MouseHandler.onButton` (MouseHandler.java:88-100): recompute
            // the scaled position from the raw physical cache with the
            // *current* physical/logical sizes at click time.
            let gui_position = cursor_position.project(
                renderer.physical_surface_size(),
                renderer.gui_surface_size(),
            );
            SimulationCommand::MouseButton {
                button: *button,
                state: *state,
                position: gui_position,
                chat_target: (matches!(
                    (*button, *state),
                    (winit::event::MouseButton::Left, ElementState::Pressed)
                ))
                .then(|| {
                    gui_position.and_then(|position| {
                        renderer.hud_chat_hit_target_at((position.x, position.y))
                    })
                })
                .flatten(),
                book_target: (matches!(
                    (*button, *state),
                    (winit::event::MouseButton::Left, ElementState::Pressed)
                ))
                .then(|| {
                    gui_position.and_then(|position| {
                        renderer.hud_book_hit_target_at((position.x, position.y))
                    })
                })
                .flatten(),
                disconnected_screen_target: (matches!(
                    (*button, *state),
                    (winit::event::MouseButton::Left, ElementState::Pressed)
                ))
                .then(|| {
                    gui_position.and_then(|position| {
                        renderer.hud_disconnected_screen_hit_target_at((position.x, position.y))
                    })
                })
                .flatten(),
                death_screen_target: (matches!(
                    (*button, *state),
                    (winit::event::MouseButton::Left, ElementState::Pressed)
                ))
                .then(|| {
                    gui_position.and_then(|position| {
                        renderer.hud_death_screen_hit_target_at((position.x, position.y))
                    })
                })
                .flatten(),
                game_event_target: (matches!(
                    (*button, *state),
                    (winit::event::MouseButton::Left, ElementState::Pressed)
                ))
                .then(|| {
                    gui_position.and_then(|position| {
                        renderer.hud_game_event_hit_target_at((position.x, position.y))
                    })
                })
                .flatten(),
                player_reporting_target: (matches!(
                    (*button, *state),
                    (winit::event::MouseButton::Left, ElementState::Pressed)
                ))
                .then(|| {
                    gui_position.and_then(|position| {
                        renderer.hud_player_reporting_hit_target_at((position.x, position.y))
                    })
                })
                .flatten(),
                social_interactions_target: (matches!(
                    (*button, *state),
                    (winit::event::MouseButton::Left, ElementState::Pressed)
                ))
                .then(|| {
                    gui_position.and_then(|position| {
                        renderer.hud_social_interactions_hit_target_at((position.x, position.y))
                    })
                })
                .flatten(),
                pause_draft_target: (matches!(
                    (*button, *state),
                    (winit::event::MouseButton::Left, ElementState::Pressed)
                ))
                .then(|| {
                    gui_position.and_then(|position| {
                        renderer.hud_pause_draft_hit_target_at((position.x, position.y))
                    })
                })
                .flatten(),
            }
        }
        WindowEvent::MouseWheel { delta, .. } => SimulationCommand::MouseWheel { delta: *delta },
        _ => return false,
    };
    mailbox.enqueue_or_report(command, "window event");
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
        || world.social_interactions_screen().is_some()
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
                    items.mark_profile_texture_failed(download.kind, &download.url);
                    continue;
                }
                items.mark_profile_texture_resolved(download.kind, &download.url, handle);
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
    fn client_options_save_completion_follows_missing_or_failed_file_owner() {
        let mut information = bbb_protocol::packets::ClientInformation::default();
        information.language = "en_pt".to_string();
        let writes = [client_options::ClientOptionWrite {
            key: "lang",
            value: client_options::ClientOptionValue::Text("en_pt"),
        }];

        let missing_owner_mailbox = SimulationMailbox::default();
        assert!(save_client_options_and_enqueue_broadcast(
            None,
            &writes,
            information.clone(),
            &missing_owner_mailbox,
        )
        .is_none());
        let missing_owner = missing_owner_mailbox.drain_through_barrier();
        assert!(matches!(
            missing_owner.as_slice(),
            [simulation::commands::SequencedSimulationCommand {
                command: SimulationCommand::ClientOptionsSaveCompleted(completed),
                ..
            }] if completed.language == "en_pt"
        ));

        let path = std::env::temp_dir()
            .join(format!("bbb-options-save-failure-{}", std::process::id()))
            .join("missing-parent")
            .join("options.txt");
        let mut file = ClientOptionsFile::load(path, ClientOptionHost::default()).unwrap();
        let failed_write_mailbox = SimulationMailbox::default();
        assert!(matches!(
            save_client_options_and_enqueue_broadcast(
                Some(&mut file),
                &writes,
                information,
                &failed_write_mailbox,
            ),
            Some(Err(_))
        ));
        assert!(matches!(
            failed_write_mailbox.drain_through_barrier().as_slice(),
            [simulation::commands::SequencedSimulationCommand {
                command: SimulationCommand::ClientOptionsSaveCompleted(completed),
                ..
            }] if completed.language == "en_pt"
        ));
    }

    #[test]
    fn startup_cleanliness_brackets_native_bootstrap_and_successful_initial_load() {
        let source = include_str!("main.rs");
        let production = source
            .split_once("\nmod tests {")
            .expect("production/test boundary")
            .0;
        let dirty = production
            .find("options.save_started_cleanly(false)")
            .expect("startup dirty marker");
        let packs = production
            .find("load_pack_roots_with_options(&args)")
            .unwrap();
        let window = production
            .find("build_window(&event_loop, &window_config)")
            .unwrap();
        let simulation = production
            .find("let mut simulation_thread = SimulationThread::")
            .unwrap();
        let clean = production
            .find("options.save_started_cleanly(true)")
            .expect("successful initial-load marker");
        let connect = production
            .find("network_runtime.activate()")
            .expect("quick-play activation barrier");
        let event_loop = production
            .find("event_loop.run(move |event, target|")
            .unwrap();

        assert!(dirty < packs && packs < window && window < simulation);
        assert!(simulation < clean && clean < connect && connect < event_loop);
    }

    #[test]
    fn one_startup_identity_feeds_network_world_and_server_pack_headers() {
        let source = include_str!("main.rs");
        let production = source
            .split_once("\nmod tests {")
            .expect("production/test boundary")
            .0;
        let identity = production
            .find("let client_identity = startup::client_identity_from_args(&args)")
            .expect("single startup identity owner");
        let network = production
            .find("&client_identity,\n        &mut code_of_conduct_acceptance")
            .expect("network identity projection");
        let world = production
            .find("world.set_local_player_profile_id(client_identity.profile_id)")
            .expect("canonical world identity projection");
        let packs = production
            .find("client_identity.username.clone(),\n        client_identity.profile_id")
            .expect("server-pack request identity projection");

        assert!(identity < network && network < world && world < packs);
        assert!(!production.contains("offline_player_uuid(&args.username)"));
    }

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

    #[test]
    fn window_title_follows_the_published_connection_state_and_diffs_applications() {
        let source = include_str!("main.rs");
        let production = source
            .split_once("\nmod tests {")
            .expect("production/test boundary")
            .0;
        let running_check = production
            .find("if platform_exit_requested || !render_publication.running")
            .expect("terminal running check");
        let title_call = production
            .find("update_window_title(")
            .expect("window title owner");
        assert!(running_check < title_call);
        let call_site = &production[title_call..title_call + 160];
        assert!(call_site.contains("render_publication.net_counters.connected"));
        let title_update = production
            .split_once("fn update_window_title(")
            .expect("window title updater")
            .1;
        assert!(title_update.contains("current.as_deref() != Some(next.as_str())"));
        assert!(title_update.contains("window.set_title(&next)"));
    }

    #[test]
    fn cursor_icon_changes_flow_through_select_cursor_gating() {
        let source = include_str!("main.rs");
        let production = source
            .split_once("\nmod tests {")
            .expect("production/test boundary")
            .0;
        let gate = production
            .split_once("fn apply_window_cursor_icon(")
            .expect("cursor gate owner")
            .1;
        assert!(gate.contains("window_surface::select_cursor("));
        assert!(gate.contains("window.set_cursor_icon(next)"));
        let moved = production
            .split_once("WindowEvent::CursorMoved { position, .. } =>")
            .expect("cursor move arm")
            .1;
        assert!(moved.contains("apply_window_cursor_icon("));
        let left = production
            .split_once("WindowEvent::CursorLeft { .. } =>")
            .expect("cursor leave arm")
            .1;
        assert!(left.contains("apply_window_cursor_icon("));
    }

    #[test]
    fn fullscreen_transitions_share_one_deterministic_mode_policy() {
        let source = include_str!("main.rs");
        let production = source
            .split_once("\nmod tests {")
            .expect("production/test boundary")
            .0;
        let policy = production
            .split_once("fn set_window_fullscreen(")
            .expect("fullscreen mode policy owner")
            .1;
        assert!(policy.contains("window_surface::find_exact_video_mode("));
        assert!(policy.contains("window_surface::fullscreen_choice("));
        assert!(policy.contains("Fullscreen::Exclusive("));
        assert!(policy.contains("Fullscreen::Borderless(None)"));
        assert!(!policy.contains("window.fullscreen()"));
        // Both the startup apply and the runtime SetFullscreen effect route
        // through the same policy; no second window owner exists.
        let startup = production
            .split_once("fn apply_startup_fullscreen(")
            .expect("startup fullscreen owner")
            .1;
        assert!(startup.contains("set_window_fullscreen(window, true,"));
        let effect = production
            .split_once("PlatformEffect::SetFullscreen(fullscreen) =>")
            .expect("runtime fullscreen effect")
            .1;
        assert!(effect.contains("set_window_fullscreen("));
        // One definition plus one startup call site plus one effect call site.
        assert_eq!(production.matches("set_window_fullscreen(").count(), 3);
        // The policy helper owns all three native transitions (windowed
        // restore, exclusive entry, borderless entry).
        assert_eq!(production.matches("window.set_fullscreen(").count(), 3);
    }

    #[test]
    fn cursor_events_cache_physical_and_reproject_at_the_gui_boundary() {
        let source = include_str!("main.rs");
        let production = source
            .split_once("\nmod tests {")
            .expect("production/test boundary")
            .0;
        let moved = production
            .split_once("WindowEvent::CursorMoved { position, .. } =>")
            .expect("cursor move arm")
            .1;
        // The raw physical position is cached verbatim; the projection is
        // derived once for this event's hit tests and simulation command.
        assert!(moved.contains("cursor_position.on_cursor_moved(*position)"));
        assert!(moved.contains("let gui_position = cursor_position"));
        assert!(moved.contains(".project("));
        assert!(moved.contains("renderer.gui_surface_size()"));
        assert!(moved.contains("position: gui_position,"));
        // The click path re-projects the cached physical position with the
        // current renderer sizes instead of reusing a stale pre-resize
        // logical cache, and carries that projection into the simulation.
        let clicked = production
            .split_once("WindowEvent::MouseInput { state, button, .. } =>")
            .expect("mouse button arm")
            .1;
        assert!(clicked.contains("cursor_position.project("));
        assert!(clicked.contains("renderer.physical_surface_size()"));
        assert!(clicked.contains("renderer.gui_surface_size()"));
        assert!(clicked.contains("position: gui_position,"));
        let left = production
            .split_once("WindowEvent::CursorLeft { .. } =>")
            .expect("cursor leave arm")
            .1;
        assert!(left.contains("cursor_position.on_cursor_left()"));
    }

    #[test]
    fn cursor_capture_prefers_locked_and_falls_back_to_confined_independently_of_raw_input() {
        let source = include_str!("main.rs");
        let production = source
            .split_once("\nmod tests {")
            .expect("production/test boundary")
            .0;
        let capture = production
            .split_once("fn set_cursor_capture(")
            .expect("cursor capture owner")
            .1;
        assert!(capture.contains("CursorGrabMode::Locked"));
        assert!(capture.contains("CursorGrabMode::Confined"));
        assert!(!capture.contains("raw_mouse_input"));
        assert!(!capture.contains("updateRawMouseInput"));
    }

    #[test]
    fn resize_gui_surface_projects_1280x720_at_scale_three_to_427x240() {
        // `Window.setGuiScale` ceiling division: 1280/3 rounds up to 427 and
        // 720/3 rounds to 240, the exact logical surface Vanilla exposes for
        // `MouseHandler.getScaledXPos/getScaledYPos`.
        assert_eq!(
            resize_gui_surface(PhysicalSize::new(1280, 720), 3, false),
            PhysicalSize::new(427, 240)
        );
        // Auto scale (0) keeps the 320x240 minimum GUI floor at 1280x720.
        assert_eq!(
            resize_gui_surface(PhysicalSize::new(1280, 720), 0, false),
            PhysicalSize::new(427, 240)
        );
    }

    #[test]
    fn redraw_arm_orders_update_render_and_framerate_resample_like_render_frame() {
        // Vanilla `Minecraft.renderFrame`: update render state, render, then
        // the frame limiter samples `FramerateLimitTracker.getFramerateLimit`
        // — so the per-frame cap resample must follow the render call inside
        // the RedrawRequested arm.
        let source = include_str!("main.rs");
        let production = source
            .split_once("\nmod tests {")
            .expect("production/test boundary")
            .0;
        let arm = production
            .split_once("WindowEvent::RedrawRequested =>")
            .expect("redraw arm")
            .1;
        let update = arm
            .find("update_renderer_from_publication(")
            .expect("render state update");
        let render = arm.find("renderer.render(").expect("frame render");
        let resample = arm
            .find("frame_interval = client_framerate_limit_interval(effective_framerate_limit(")
            .expect("per-frame framerate resample");
        assert!(
            update < render && render < resample,
            "redraw arm must update, render, then resample the framerate cap"
        );
    }

    #[test]
    fn resize_arm_publishes_logical_gui_size_before_queuing_surface_resized() {
        let source = include_str!("main.rs");
        let production = source
            .split_once("\nmod tests {")
            .expect("production/test boundary")
            .0;
        let arm = production
            .split_once("WindowEvent::Resized(size) =>")
            .expect("resize arm")
            .1;
        let arm = arm
            .split_once("WindowEvent::RedrawRequested =>")
            .expect("redraw arm")
            .0;
        // The synchronous logical-GUI publish must come before the simulation
        // command is queued, so a `MouseInput` delivered before the
        // asynchronous publication projects with the new ratio.
        let publish = arm
            .find("renderer.set_gui_surface_size(")
            .expect("synchronous logical GUI publish");
        let enqueue = arm
            .find("SimulationCommand::SurfaceResized(size)")
            .expect("surface resize simulation command");
        assert!(publish < enqueue);
        // The projection is computed from the configured GUI-scale arguments
        // through the shared `window_surface` functions, not a second formula.
        assert!(arm.contains("resize_gui_surface("));
        assert!(arm.contains("args.client_gui_scale"));
        assert!(arm.contains("args.client_force_unicode_font"));
        // A zero-size/minimized callback retains both last usable sizes: the
        // synchronous publish is gated on nonzero dimensions.
        assert!(arm.contains("if size.width != 0 && size.height != 0 {"));
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
                comparator_output: None,
                structure_block: None,
                jigsaw_block: None,
                test_block: None,
                test_instance_block: None,
                vault_shared_data: None,
                campfire: None,
                shelf: None,
                brushable: None,
                conduit_destroy_target: None,
                decorated_pot_sherds: None,
                banner_patterns: None,
                end_gateway: None,
                spawner: None,
                trial_spawner: None,
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
    fn social_interactions_runtime_wants_cursor() {
        let input = ClientInputState::new(true);
        let mut world = WorldStore::new();

        assert!(!runtime_wants_cursor(&input, &world));
        world.open_social_interactions_screen();
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
        input.open_debug_pause_screen_without_menu_for_test();
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
