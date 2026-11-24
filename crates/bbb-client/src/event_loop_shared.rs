//! Cross-platform event-loop helpers, platform-effect executor, and window
//! input translation shared between the desktop and Web clients.

#[cfg(not(target_arch = "wasm32"))]
use std::{cell::RefCell, rc::Rc};

use crate::terrain_upload::TerrainUploadState;
use anyhow::Result;
use bbb_client_assets::PackRoots;
use bbb_client_render::item_model::{font::providers::FontOptions, NativeItemRuntime};
#[cfg(not(target_arch = "wasm32"))]
use bbb_client_world::WorldStore;
use std::time::Duration;
use winit::dpi::PhysicalSize;
#[cfg(not(target_arch = "wasm32"))]
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::{
    event::{DeviceEvent, ElementState, Event, Ime, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
    window::{CursorGrabMode, CursorIcon, Fullscreen, Window},
};

#[cfg(not(target_arch = "wasm32"))]
use crate::crash_report::{RuntimeFailure, RuntimeFailureAccumulator};
#[cfg(not(target_arch = "wasm32"))]
use crate::input::ClientInputState;
use crate::{
    audio_runtime::{AudioEventSink, NativeAudioRuntime},
    control::{self, SharedSnapshot},
    debug_profiler::DebugProfiler,
    input::{self, DebugClipboard},
    narrator_runtime,
    options::ClientOptionsFile,
    particle_runtime::NativeParticleRenderRuntime,
    platform, pointer_input, recording,
    resource_reload::PreparedServerResourcePackReload,
    screenshot,
    session_shared::{self, SessionRenderState},
    simulation::{
        self,
        commands::{CommandInbox, CommandQueue, SimulationCommand},
        effects::{PlatformEffect, ResourceReloadEffect},
        InputLatencyTracker, RenderParts, SimulationCore, StepOutcome,
    },
    sound_subtitle_runtime::SoundSubtitleRuntime,
    startup, window_surface,
};

pub(crate) struct PendingScreenshot {
    pub(crate) path: std::path::PathBuf,
    pub(crate) announce: bool,
    pub(crate) completion: Option<control::ControlCompletionSender>,
}

impl PendingScreenshot {
    pub(crate) fn complete(
        self,
        result: Result<bbb_client_render::CapturedScreenshot>,
        commands: &CommandQueue,
    ) {
        if self.announce {
            commands.push(SimulationCommand::ScreenshotSaved {
                name: self
                    .path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                error: result.as_ref().err().map(|error| format!("{error:#}")),
            });
        }
        if let Some(completion) = self.completion {
            let result = result
                .map(|image| control::ControlCompletion::Screenshot {
                    path: self.path.to_string_lossy().into_owned(),
                    image,
                })
                .map_err(|error| format!("{error:#}"));
            let _ = completion.send(result);
        } else if let Err(error) = result {
            tracing::warn!(%error, path = %self.path.display(), "screenshot failed");
        }
    }
}

/// Applies a step's platform effects to the window, renderer and narrators.
pub(crate) struct PlatformEffectTarget<'a> {
    pub(crate) renderer: &'a mut bbb_client_render::Renderer,
    pub(crate) window: &'a Window,
    pub(crate) input_runtime: &'a mut simulation::input_commands::SimulationInputRuntime,
    pub(crate) vsync_enabled: bool,
    pub(crate) preferred_video_mode: Option<window_surface::PreferredVideoMode>,
    pub(crate) exclusive_fullscreen_supported: bool,
    pub(crate) commands: &'a CommandQueue,
    pub(crate) clipboard: &'a mut dyn DebugClipboard,
    pub(crate) terrain_upload: &'a mut TerrainUploadState,
    pub(crate) client_locale: &'a mut String,
    pub(crate) debug_profiler: &'a mut DebugProfiler,
    pub(crate) screen_narrations: &'a mut narrator_runtime::ScreenNarrations,
    pub(crate) narrator: Option<&'a mut dyn narrator_runtime::SystemNarrator>,
    pub(crate) control_screenshots: &'a mut std::collections::VecDeque<PendingScreenshot>,
    pub(crate) recording: &'a mut recording::RecordingRuntime,
    pub(crate) recording_audio: Option<&'a mut NativeAudioRuntime>,
    pub(crate) recording_completions: &'a mut Vec<(
        control::ControlCompletionSender,
        std::result::Result<control::RecordingStatus, String>,
    )>,
    pub(crate) recording_fps: u32,
    pub(crate) recording_bitrate: u32,
    pub(crate) game_dir: &'a std::path::Path,
    pub(crate) client_options: Option<&'a mut ClientOptionsFile>,
    #[cfg(target_arch = "wasm32")]
    pub(crate) server: &'a str,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) reconnect_requested: &'a mut bool,
    pub(crate) exit_requested: &'a mut bool,
}

/// Complete Vanilla's synchronous `Options.save` contract: attempt the file
/// write first, then publish the client-information completion regardless of
/// a missing file owner or write failure.
pub(crate) fn save_client_options_and_enqueue_broadcast(
    client_options: Option<&mut ClientOptionsFile>,
    writes: &[crate::options::ClientOptionWrite],
    broadcast: bbb_protocol::packets::ClientInformation,
    commands: &CommandQueue,
) -> Option<Result<bool>> {
    let result = client_options.map(|options| options.apply(writes));
    commands.push(SimulationCommand::ClientOptionsSaveCompleted(broadcast));
    result
}

impl PlatformEffectTarget<'_> {
    fn apply_recording(
        &mut self,
        request: control::RecordingControlRequest,
        completion: Option<control::ControlCompletionSender>,
    ) {
        let size = self.renderer.physical_surface_size();
        let result = self.recording.apply(
            request,
            [size.width, size.height],
            self.recording_audio.as_deref_mut(),
        );
        let message = match &result {
            Ok(status) => format!(
                "{}: {}",
                if status.state == control::RecordingState::Recording {
                    "Recording video"
                } else {
                    "Saved video"
                },
                status.path.as_deref().unwrap_or("<unknown>")
            ),
            Err(error) => format!("Video recording failed: {error:#}"),
        };
        self.commands
            .push(SimulationCommand::RecordingNotice(message));
        if let Some(completion) = completion {
            self.recording_completions
                .push((completion, result.map_err(|error| format!("{error:#}"))));
        }
    }

    pub(crate) fn apply(&mut self, effect: PlatformEffect) {
        match effect {
            PlatformEffect::SetCursorCapture(capture) => {
                if capture && self.input_runtime.synthetic_cursor_capture() {
                    self.input_runtime.set_cursor_captured(true);
                } else {
                    let captured = set_cursor_capture(self.window, capture);
                    self.input_runtime.set_cursor_captured(captured);
                }
            }
            PlatformEffect::SetTextInputEnabled(enabled) => {
                self.window.set_ime_allowed(enabled);
                #[cfg(target_arch = "wasm32")]
                crate::web::ime::set_web_ime_allowed(enabled);
            }
            PlatformEffect::ClipboardWrite(text) => {
                self.clipboard.set_debug_clipboard_java_text(&text);
            }
            PlatformEffect::OpenUri(url) => {
                if let Err(err) = platform::open_uri(&url) {
                    tracing::warn!(?err, url = %url.to_jdk_string(), "failed to open dialog URI");
                }
            }
            PlatformEffect::OpenPath(path) => {
                if let Err(err) = platform::open_path(&path) {
                    tracing::warn!(?err, path = %path.display(), "failed to open path");
                }
            }
            PlatformEffect::SaveClientOptions { writes, broadcast } => {
                match save_client_options_and_enqueue_broadcast(
                    self.client_options.as_deref_mut(),
                    &writes,
                    broadcast,
                    self.commands,
                ) {
                    Some(Err(error)) => {
                        tracing::warn!(%error, "failed to save options.txt");
                    }
                    None => tracing::warn!("no options.txt owner to save client options into"),
                    Some(Ok(_)) => {}
                }
            }
            PlatformEffect::Screenshot { path, completion } => {
                self.control_screenshots.push_back(PendingScreenshot {
                    path,
                    announce: false,
                    completion,
                });
            }
            PlatformEffect::Recording {
                request,
                completion,
            } => {
                self.apply_recording(request, completion);
            }
            PlatformEffect::ToggleRecording => {
                let request = if self.recording.status().state == control::RecordingState::Recording
                {
                    control::RecordingControlRequest::Stop
                } else {
                    let path = self.game_dir.join("videos").join(
                        chrono::Local::now()
                            .format("%Y-%m-%d_%H.%M.%S%.3f.mp4")
                            .to_string(),
                    );
                    control::RecordingControlRequest::Start {
                        path: path.to_string_lossy().into_owned(),
                        fps: self.recording_fps,
                        bitrate: self.recording_bitrate,
                    }
                };
                self.apply_recording(request, None);
            }
            PlatformEffect::GrabScreenshot => {
                let directory = self.game_dir.join(screenshot::SCREENSHOT_DIR);
                #[cfg(not(target_arch = "wasm32"))]
                if let Err(err) = std::fs::create_dir_all(&directory) {
                    tracing::warn!(?err, path = %directory.display(), "couldn't create screenshot directory");
                    self.commands.push(SimulationCommand::ScreenshotSaved {
                        name: String::new(),
                        error: Some(err.to_string()),
                    });
                    return;
                }
                let stamp = chrono::Local::now().format("%Y-%m-%d_%H.%M.%S").to_string();
                let path = screenshot::next_screenshot_file(&directory, &stamp, |path| {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        path.exists()
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        let _ = path;
                        false
                    }
                });
                self.control_screenshots.push_back(PendingScreenshot {
                    path,
                    announce: true,
                    completion: None,
                });
            }
            PlatformEffect::ReloadTerrain => self.terrain_upload.request_reload_all_chunks(),
            PlatformEffect::DumpDynamicTextures { requests } => {
                let path = self
                    .game_dir
                    .join(input::DEBUG_DYNAMIC_TEXTURE_DUMP_RELATIVE_PATH);
                #[cfg(target_arch = "wasm32")]
                {
                    tracing::info!(
                        requests,
                        path = %path.display(),
                        "debug texture dumps (F3+S) are unavailable on wasm32"
                    );
                }
                #[cfg(not(target_arch = "wasm32"))]
                match self.renderer.dump_dynamic_textures(&path) {
                    Ok(summary) => tracing::info!(
                        requests,
                        files_written = summary.files_written,
                        terrain_atlas = summary.terrain_atlas,
                        gui_atlas = summary.gui_atlas,
                        font_atlas = summary.font_atlas,
                        particle_atlas = summary.particle_atlas,
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
                #[cfg(target_arch = "wasm32")]
                if let Some(options) = self.client_options.as_deref_mut() {
                    let _ = options.save_web_raw_overrides(&[("lang", locale.as_str())]);
                }
                *self.client_locale = locale;
            }
            PlatformEffect::ProfilingToggled { commands } => {
                let requests = commands.len();
                let mut starts = 0;
                let mut stops = 0;
                for command in commands {
                    match command {
                        simulation::effects::DebugProfilingCommand::Start => {
                            starts += usize::from(
                                self.debug_profiler
                                    .start_recording(bbb_client_clock::Instant::now()),
                            );
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
            PlatformEffect::ScreenNarrationInput { target, input } => {
                self.screen_narrations
                    .after_input(target, input, bbb_client_clock::now());
            }
            PlatformEffect::CommandConfirmationInitialized {
                frame,
                last_input_type,
                at,
            } => {
                self.screen_narrations.initialize_confirmation(
                    &frame,
                    at,
                    last_input_type,
                    self.narrator.as_mut().map(|narrator| {
                        &mut **narrator as &mut dyn narrator_runtime::SystemNarrator
                    }),
                );
            }
            PlatformEffect::ChatNarration { kind, text } => {
                let Some(narrator) = self.narrator.as_deref_mut() else {
                    return;
                };
                narrator_runtime::submit_chat_narration(narrator, kind, &text);
            }
            PlatformEffect::SetFullscreen(fullscreen) => {
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
                    let now = bbb_client_clock::now();
                    self.screen_narrations.schedule_after_narrator_enabled(now);
                }
            }
            #[cfg(target_arch = "wasm32")]
            PlatformEffect::Reconnect => crate::web::reconnect_to_server(self.server),
            #[cfg(not(target_arch = "wasm32"))]
            PlatformEffect::Reconnect => *self.reconnect_requested = true,
            PlatformEffect::Exit => *self.exit_requested = true,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn record_runtime_failure(
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

/// Applies a capture request and returns the resulting grab state.
pub(crate) fn set_cursor_capture(window: &Window, capture: bool) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        if capture {
            window.set_ime_allowed(false);
            crate::web::ime::set_web_ime_allowed(false);
            // Attempt Pointer Lock immediately when transient user activation is active
            // (e.g. closing Inventory with `E` or clicking "Back to Game"); `index.html`
            // swallows rejected promises when called outside a user gesture.
            if !bbb_client_clock::is_virtual() {
                let _ = window.set_cursor_grab(CursorGrabMode::Locked);
            }
            // Defer cursor hiding until `pointerlockchange` confirms lock so a
            // rejected request during Chrome's Escape cooldown never leaves an
            // invisible cursor while logical keyboard capture stays active.
            return true;
        }
        crate::web::notify_intentional_pointer_unlock();
        let _ = window.set_cursor_grab(CursorGrabMode::None);
        window.set_cursor_visible(true);
        return false;
    }

    #[cfg(not(target_arch = "wasm32"))]
    if capture {
        match window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
        {
            Ok(()) => {
                let size = window.inner_size();
                let _ = window.set_cursor_position(winit::dpi::PhysicalPosition::new(
                    f64::from(size.width) * 0.5,
                    f64::from(size.height) * 0.5,
                ));
                window.set_cursor_visible(false);
                true
            }
            Err(_) if bbb_client_clock::is_virtual() => true,
            Err(err) => {
                tracing::warn!(?err, "unable to capture cursor for first-person input");
                false
            }
        }
    } else {
        if let Err(err) = window.set_cursor_grab(CursorGrabMode::None) {
            tracing::warn!(?err, "unable to release cursor capture");
        }
        let size = window.inner_size();
        let _ = window.set_cursor_position(winit::dpi::PhysicalPosition::new(
            f64::from(size.width) * 0.5,
            f64::from(size.height) * 0.5,
        ));
        window.set_cursor_visible(true);
        false
    }
}

/// `Window.selectCursor` applied through winit.
pub(crate) fn apply_window_cursor_icon(
    window: &Window,
    allow_cursor_changes: bool,
    current: &mut CursorIcon,
    requested: CursorIcon,
) {
    if let Some(next) = window_surface::select_cursor(allow_cursor_changes, requested, *current) {
        window.set_cursor(next);
        *current = next;
    }
}

pub(crate) fn requested_window_fullscreen(
    fullscreen: bool,
    preferred: Option<window_surface::PreferredVideoMode>,
    monitor: Option<&winit::monitor::MonitorHandle>,
    exclusive_supported: bool,
) -> Option<Fullscreen> {
    if !fullscreen {
        return None;
    }
    let exact = preferred.and_then(|mode| window_surface::find_exact_video_mode(mode, monitor));
    Some(
        match (
            window_surface::fullscreen_choice(preferred, exact.is_some(), exclusive_supported),
            exact,
        ) {
            (window_surface::FullscreenChoice::Exclusive, Some(mode)) => {
                Fullscreen::Exclusive(mode)
            }
            _ => Fullscreen::Borderless(None),
        },
    )
}

#[cfg(any(not(target_arch = "wasm32"), test))]
pub(crate) fn platform_surface_size(
    target_os: &str,
    exclusive_fullscreen: bool,
    preferred: Option<window_surface::PreferredVideoMode>,
    observed: PhysicalSize<u32>,
) -> PhysicalSize<u32> {
    if observed.width == 0 || observed.height == 0 {
        return observed;
    }
    if target_os == "macos" && exclusive_fullscreen {
        if let Some(preferred) = preferred {
            return PhysicalSize::new(preferred.width, preferred.height);
        }
    }
    observed
}

pub(crate) fn set_window_fullscreen(
    window: &Window,
    fullscreen: bool,
    preferred: Option<window_surface::PreferredVideoMode>,
    exclusive_supported: bool,
) {
    let monitor = window.current_monitor();
    window.set_fullscreen(requested_window_fullscreen(
        fullscreen,
        preferred,
        monitor.as_ref(),
        exclusive_supported,
    ));
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn apply_startup_fullscreen(
    window: &Window,
    input: &mut ClientInputState,
    applied_at_creation: bool,
    preferred: Option<window_surface::PreferredVideoMode>,
    exclusive_supported: bool,
) {
    if !input.is_fullscreen() || applied_at_creation {
        return;
    }
    set_window_fullscreen(window, true, preferred, exclusive_supported);
}

/// `Minecraft.updateTitle`.
pub(crate) fn update_window_title(window: &Window, connected: bool, current: &mut Option<String>) {
    let next = window_surface::create_title(bbb_protocol::MC_VERSION, connected);
    if current.as_deref() != Some(next.as_str()) {
        window.set_title(&next);
        *current = Some(next);
    }
}

pub(crate) struct WindowEventDispatchContext<'a> {
    pub(crate) commands: &'a CommandQueue,
    pub(crate) input_latency: &'a mut InputLatencyTracker,
    pub(crate) clipboard: &'a mut dyn DebugClipboard,
    pub(crate) renderer: &'a bbb_client_render::Renderer,
    pub(crate) window: &'a Window,
    pub(crate) cursor_position: &'a mut window_surface::CursorPositionCache,
    pub(crate) cursor_captured: bool,
    pub(crate) allow_cursor_changes: bool,
    pub(crate) current_cursor_icon: &'a mut CursorIcon,
    pub(crate) keyboard_modifiers: &'a mut ModifiersState,
}

fn update_keyboard_modifiers_for_key(
    modifiers: &mut ModifiersState,
    physical_key: PhysicalKey,
    state: ElementState,
) {
    let flag = match physical_key {
        PhysicalKey::Code(KeyCode::ControlLeft | KeyCode::ControlRight) => ModifiersState::CONTROL,
        PhysicalKey::Code(KeyCode::SuperLeft | KeyCode::SuperRight) => ModifiersState::SUPER,
        PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight) => ModifiersState::SHIFT,
        PhysicalKey::Code(KeyCode::AltLeft | KeyCode::AltRight) => ModifiersState::ALT,
        _ => return,
    };
    modifiers.set(flag, matches!(state, ElementState::Pressed));
}

fn should_read_clipboard_for_key_event(
    modifiers: ModifiersState,
    physical_key: PhysicalKey,
    state: ElementState,
) -> bool {
    matches!(state, ElementState::Pressed)
        && matches!(physical_key, PhysicalKey::Code(KeyCode::KeyV))
        && (modifiers.control_key() || modifiers.super_key())
        && !modifiers.shift_key()
        && !modifiers.alt_key()
}

/// Translates winit window events into `SimulationCommand`s.
pub(crate) fn enqueue_simulation_window_event(
    event: &WindowEvent,
    ctx: WindowEventDispatchContext<'_>,
) -> bool {
    let WindowEventDispatchContext {
        commands,
        input_latency,
        clipboard,
        renderer,
        window,
        cursor_position,
        cursor_captured,
        allow_cursor_changes,
        current_cursor_icon,
        keyboard_modifiers,
    } = ctx;
    let command = match event {
        WindowEvent::CloseRequested => SimulationCommand::Shutdown,
        WindowEvent::CursorMoved { position, .. } => {
            if cursor_captured {
                return true;
            }
            let (command, gui_position) =
                pointer_input::cursor_moved(*position, renderer, cursor_position);
            let requested = window_surface::hud_cursor_icon(renderer, Some(gui_position));
            apply_window_cursor_icon(window, allow_cursor_changes, current_cursor_icon, requested);
            command
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
        WindowEvent::Focused(focused) => {
            if !focused {
                *keyboard_modifiers = ModifiersState::empty();
            }
            SimulationCommand::FocusChanged(*focused)
        }
        WindowEvent::KeyboardInput { event, .. } => {
            update_keyboard_modifiers_for_key(keyboard_modifiers, event.physical_key, event.state);
            if event.physical_key == PhysicalKey::Code(KeyCode::F9) && event.repeat {
                return true;
            }
            let clipboard_text = should_read_clipboard_for_key_event(
                *keyboard_modifiers,
                event.physical_key,
                event.state,
            )
            .then(|| {
                #[cfg(target_arch = "wasm32")]
                if crate::web::paste_deferred_to_dom_event() {
                    return None;
                }
                clipboard.get_debug_clipboard_text()
            })
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
            pointer_input::mouse_button(*button, *state, renderer, cursor_position)
        }
        WindowEvent::MouseWheel { delta, .. } => SimulationCommand::MouseWheel {
            delta: *delta,
            position: cursor_position.project(
                renderer.physical_surface_size(),
                renderer.gui_surface_size(),
            ),
        },
        _ => return false,
    };
    queue_input(commands, input_latency, command);
    if let WindowEvent::KeyboardInput { event, .. } = event {
        #[cfg(not(target_arch = "wasm32"))]
        let key_text = event.text_with_all_modifiers();
        #[cfg(target_arch = "wasm32")]
        let key_text = (!crate::web::shortcut_modifier_active())
            .then_some(event.text.as_deref())
            .flatten();
        if let Some(text) = SimulationCommand::native_key_text(event.state, key_text) {
            queue_input(commands, input_latency, text);
        }
    }
    true
}

pub(crate) fn queue_input(
    commands: &CommandQueue,
    input_latency: &mut InputLatencyTracker,
    command: SimulationCommand,
) {
    if command.is_platform_input() {
        input_latency.queued(bbb_client_clock::now());
    }
    commands.push(command);
}

pub(crate) fn drain_dynamic_player_skin_downloads(
    items: &NativeItemRuntime,
    renderer: &mut bbb_client_render::Renderer,
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

pub(crate) struct SessionEventLoopContext {
    pub(crate) window: Window,
    pub(crate) renderer: bbb_client_render::Renderer,
    pub(crate) core: SimulationCore,
    pub(crate) state: SessionRenderState,
    pub(crate) args: startup::Args,
    pub(crate) command_queue: CommandQueue,
    pub(crate) command_inbox: CommandInbox,
    pub(crate) snapshot: SharedSnapshot,
    pub(crate) client_locale: String,
    pub(crate) client_options: Option<ClientOptionsFile>,
    pub(crate) base_pack_roots: Option<PackRoots>,
    pub(crate) pack_roots: Option<PackRoots>,
    pub(crate) audio_runtime: Option<NativeAudioRuntime>,
    pub(crate) sound_subtitle_runtime: SoundSubtitleRuntime,
    pub(crate) particle_render_runtime: Option<NativeParticleRenderRuntime>,
    pub(crate) font_options: FontOptions,
    pub(crate) client_particle_status: bbb_protocol::packets::ClientParticleStatus,
}

impl SessionEventLoopContext {
    pub(crate) fn commit_prepared_reload(
        &mut self,
        prepared: PreparedServerResourcePackReload,
    ) -> [SimulationCommand; 2] {
        session_shared::commit_prepared_reload_into_session(
            prepared,
            &mut self.pack_roots,
            &mut self.audio_runtime,
            &mut self.particle_render_runtime,
            &mut self.core,
            &mut self.state,
            &mut self.renderer,
        )
    }

    pub(crate) fn apply_virtual_clock_reload(&mut self, reload: ResourceReloadEffect) {
        session_shared::apply_virtual_clock_reload_into_session(
            reload,
            &self.base_pack_roots,
            &mut self.pack_roots,
            &mut self.audio_runtime,
            &mut self.particle_render_runtime,
            &mut self.core,
            &mut self.state,
            &mut self.renderer,
            &self.client_locale,
            self.font_options,
            self.client_particle_status,
            &self.args.username,
            &self.command_queue,
        );
    }
}

pub(crate) trait PlatformEventLoopHooks {
    fn shutdown_platform(&mut self, ctx: &mut SessionEventLoopContext);

    fn on_before_event(&mut self, _ctx: &mut SessionEventLoopContext, _event: &Event<()>) {}

    fn intercept_window_event(
        &mut self,
        _ctx: &mut SessionEventLoopContext,
        _event: &WindowEvent,
    ) -> bool {
        false
    }

    fn resolve_surface_resize(
        &mut self,
        ctx: &SessionEventLoopContext,
        size: PhysicalSize<u32>,
    ) -> Option<PhysicalSize<u32>>;

    fn on_resize_error(
        &mut self,
        ctx: &SessionEventLoopContext,
        old_size: PhysicalSize<u32>,
        new_size: PhysicalSize<u32>,
        error: anyhow::Error,
    );

    fn clipboard_and_narrator_mut(
        &mut self,
    ) -> (
        &mut dyn DebugClipboard,
        Option<&mut dyn narrator_runtime::SystemNarrator>,
    );

    fn clipboard_mut(&mut self) -> &mut dyn DebugClipboard {
        self.clipboard_and_narrator_mut().0
    }

    fn narrator_mut(&mut self) -> Option<&mut dyn narrator_runtime::SystemNarrator> {
        self.clipboard_and_narrator_mut().1
    }

    fn on_occluded(&mut self, _ctx: &mut SessionEventLoopContext, _occluded: bool) {}

    fn render_frame(
        &mut self,
        ctx: &mut SessionEventLoopContext,
        target: &ActiveEventLoop,
    ) -> Option<Duration>;

    fn is_window_minimized(&self, _window: &Window) -> bool {
        false
    }

    fn after_finish_frame(
        &mut self,
        _ctx: &mut SessionEventLoopContext,
        _target: &ActiveEventLoop,
    ) {
    }

    fn allow_mouse_motion(&self) -> bool {
        true
    }

    fn collect_pre_step_commands(
        &mut self,
        ctx: &mut SessionEventLoopContext,
        commands: &mut Vec<SimulationCommand>,
        queued: &mut u64,
    );

    fn step_simulation(
        &mut self,
        ctx: &mut SessionEventLoopContext,
        target: &ActiveEventLoop,
        now: bbb_client_clock::Instant,
        commands: Vec<SimulationCommand>,
    ) -> Option<Option<StepOutcome>> {
        let _ = target;
        Some(
            ctx.core.step(
                now,
                commands,
                ctx.audio_runtime
                    .as_mut()
                    .map(|rt| rt as &mut dyn AudioEventSink),
            ),
        )
    }

    fn preferred_video_mode(
        &self,
        _args: &startup::Args,
    ) -> Option<window_surface::PreferredVideoMode> {
        None
    }

    fn exclusive_fullscreen_supported(&self) -> bool {
        false
    }

    fn dispatch_resource_reload(
        &mut self,
        ctx: &mut SessionEventLoopContext,
        reload: ResourceReloadEffect,
    );

    fn on_published_generation(&mut self, _generation: u64) {}

    #[cfg(not(target_arch = "wasm32"))]
    fn on_reconnect(&mut self, _ctx: &mut SessionEventLoopContext) {}

    fn before_post_step_schedule(&mut self, _ctx: &mut SessionEventLoopContext) {}

    fn schedule_policy(&self, window: &Window) -> (bool, bool, Option<Duration>);
}

pub(crate) fn drive_session_event<H: PlatformEventLoopHooks>(
    ctx: &mut SessionEventLoopContext,
    hooks: &mut H,
    event: Event<()>,
    target: &ActiveEventLoop,
) {
    if matches!(&event, Event::LoopExiting) {
        ctx.state.publication_demand.detach_simulation();
        ctx.command_inbox.close();
        ctx.state.pending_control_screenshots.clear();
        ctx.core.input_runtime.reject_control_requests();
        if ctx.core.input_runtime.cursor_captured() {
            set_cursor_capture(&ctx.window, false);
        }
        hooks.shutdown_platform(ctx);
        ctx.snapshot.rcu(|current| {
            let mut next = (**current).clone();
            next.app.running = false;
            next.net.connected = false;
            next
        });
        return;
    }

    hooks.on_before_event(ctx, &event);
    if matches!(&event, Event::NewEvents(_)) {
        if let Some(audio) = ctx.audio_runtime.as_mut() {
            audio.poll_audio_device();
        }
    }
    if session_shared::is_afk_activity_event(&event) {
        ctx.state.latest_input_at = bbb_client_clock::now();
    }

    match event {
        Event::WindowEvent { event, window_id } if window_id == ctx.window.id() => {
            if hooks.intercept_window_event(ctx, &event) {
                return;
            }
            match event {
                WindowEvent::Resized(size) => {
                    let Some(surface_size) = hooks.resolve_surface_resize(ctx, size) else {
                        return;
                    };
                    let old_size = ctx.renderer.physical_surface_size();
                    if let Err(error) = ctx.renderer.resize(surface_size) {
                        hooks.on_resize_error(ctx, old_size, surface_size, error);
                        target.exit();
                        return;
                    }
                    if surface_size.width != 0 && surface_size.height != 0 {
                        let (gui_size, gui_scale) =
                            ctx.core.input_runtime.gui_viewport_for(surface_size);
                        ctx.renderer.set_gui_viewport(gui_size, gui_scale);
                    }
                    queue_input(
                        &ctx.command_queue,
                        &mut ctx.state.input_latency,
                        SimulationCommand::SurfaceResized { size: surface_size },
                    );
                }
                WindowEvent::ModifiersChanged(modifiers) => {
                    ctx.state.keyboard_modifiers = modifiers.state();
                }
                event
                    if enqueue_simulation_window_event(
                        &event,
                        WindowEventDispatchContext {
                            commands: &ctx.command_queue,
                            input_latency: &mut ctx.state.input_latency,
                            clipboard: hooks.clipboard_mut(),
                            renderer: &ctx.renderer,
                            window: &ctx.window,
                            cursor_position: &mut ctx.state.event_cursor_position,
                            cursor_captured: ctx.core.input_runtime.cursor_captured(),
                            allow_cursor_changes: ctx.args.client_allow_cursor_changes,
                            current_cursor_icon: &mut ctx.state.current_cursor_icon,
                            keyboard_modifiers: &mut ctx.state.keyboard_modifiers,
                        },
                    ) => {}
                WindowEvent::Occluded(occluded) => {
                    hooks.on_occluded(ctx, occluded);
                }
                WindowEvent::RedrawRequested => {
                    ctx.state.redraw_requested = false;
                    let timings = {
                        let Some(RenderParts {
                            view,
                            terrain_textures,
                            item_runtime,
                            code_of_conduct_accepted,
                        }) = ctx.core.render_parts()
                        else {
                            return;
                        };
                        let timings = session_shared::prepare_and_extract_frame(
                            &view,
                            terrain_textures,
                            item_runtime,
                            code_of_conduct_accepted,
                            &mut ctx.state,
                            &mut ctx.renderer,
                            &ctx.window,
                            &ctx.args,
                            &ctx.command_queue,
                            &mut ctx.audio_runtime,
                            &mut ctx.sound_subtitle_runtime,
                            &mut ctx.particle_render_runtime,
                            hooks.narrator_mut(),
                        );
                        timings
                    };
                    let Some(profiler_render_duration) = hooks.render_frame(ctx, target) else {
                        return;
                    };
                    if let Some(RenderParts {
                        view, item_runtime, ..
                    }) = ctx.core.render_parts()
                    {
                        let minimized = hooks.is_window_minimized(&ctx.window);
                        session_shared::finish_rendered_frame(
                            &view,
                            item_runtime,
                            timings,
                            profiler_render_duration,
                            &mut ctx.state,
                            &mut ctx.renderer,
                            &ctx.args,
                            &ctx.command_queue,
                            minimized,
                        );
                    }
                    hooks.after_finish_frame(ctx, target);
                }
                _ => {}
            }
        }
        Event::DeviceEvent {
            event: DeviceEvent::MouseMotion { delta },
            ..
        } => {
            if ctx.core.input_runtime.cursor_captured() && hooks.allow_mouse_motion() {
                queue_input(
                    &ctx.command_queue,
                    &mut ctx.state.input_latency,
                    SimulationCommand::MouseMotion { delta },
                );
            }
        }
        Event::AboutToWait => {
            bbb_client_clock::replay::flag(&bbb_client_clock::replay::SIMULATION_IDLE, false);
            bbb_client_clock::replay::observe_virtual_clock();
            let now = bbb_client_clock::now();
            let (commands, mut queued) = ctx.command_inbox.drain();
            let mut commands = pointer_input::resolve_control_pointer(
                commands,
                &ctx.renderer,
                &mut ctx.state.event_cursor_position,
            );
            hooks.collect_pre_step_commands(ctx, &mut commands, &mut queued);
            if !commands.is_empty() {
                ctx.state.input_latency.applied(now);
            }
            let Some(outcome) = hooks.step_simulation(ctx, target, now, commands) else {
                return;
            };
            if let Some(outcome) = outcome {
                ctx.state.step_generation = ctx.state.step_generation.wrapping_add(1);
                if ctx
                    .core
                    .synchronize_content_registry(ctx.pack_roots.as_ref(), Some(&mut ctx.renderer))
                {
                    ctx.state
                        .non_entity_render_extraction
                        .invalidate_resources();
                }
                let preferred_video_mode = hooks.preferred_video_mode(&ctx.args);
                let exclusive_fullscreen_supported = hooks.exclusive_fullscreen_supported();
                #[cfg(not(target_arch = "wasm32"))]
                let mut reconnect_requested = false;
                {
                    let (clipboard, narrator) = hooks.clipboard_and_narrator_mut();
                    let mut platform = PlatformEffectTarget {
                        renderer: &mut ctx.renderer,
                        window: &ctx.window,
                        input_runtime: &mut ctx.core.input_runtime,
                        vsync_enabled: ctx.args.client_vsync,
                        preferred_video_mode,
                        exclusive_fullscreen_supported,
                        commands: &ctx.command_queue,
                        clipboard,
                        terrain_upload: &mut ctx.state.terrain_upload,
                        client_locale: &mut ctx.client_locale,
                        debug_profiler: &mut ctx.state.debug_profiler,
                        screen_narrations: &mut ctx.state.screen_narrations,
                        narrator,
                        control_screenshots: &mut ctx.state.pending_control_screenshots,
                        recording: &mut ctx.state.recording,
                        recording_audio: ctx.audio_runtime.as_mut(),
                        recording_completions: &mut ctx.state.pending_recording_completions,
                        recording_fps: ctx.args.record_fps,
                        recording_bitrate: ctx.args.record_bitrate,
                        game_dir: ctx.args.game_dir.as_path(),
                        client_options: ctx.client_options.as_mut(),
                        #[cfg(target_arch = "wasm32")]
                        server: &ctx.args.server,
                        #[cfg(not(target_arch = "wasm32"))]
                        reconnect_requested: &mut reconnect_requested,
                        exit_requested: &mut ctx.state.platform_exit_requested,
                    };
                    for effect in ctx.core.platform_effects.take() {
                        platform.apply(effect);
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                if reconnect_requested {
                    hooks.on_reconnect(ctx);
                }
                if let Some(reload) = ctx.core.take_resource_reload() {
                    hooks.dispatch_resource_reload(ctx, reload);
                }
                if let Some(published_gen) = session_shared::publish_step_completion(
                    &mut ctx.core,
                    outcome,
                    &mut ctx.state,
                    ctx.audio_runtime.as_ref(),
                    &ctx.snapshot,
                ) {
                    hooks.on_published_generation(published_gen);
                }
            }
            bbb_client_clock::replay::set(&bbb_client_clock::replay::COMMANDS_APPLIED, queued);
            if ctx.state.platform_exit_requested || !ctx.core.running() {
                target.exit();
                return;
            }
            hooks.before_post_step_schedule(ctx);
            update_window_title(
                &ctx.window,
                ctx.core.net_counters.connected,
                &mut ctx.state.current_window_title,
            );
            session_shared::tick_session_narrator(&ctx.core, &mut ctx.state, hooks.narrator_mut());
            let (window_hidden, poll_when_uncapped, poll_reload_interval) =
                hooks.schedule_policy(&ctx.window);
            session_shared::schedule_next_frame_and_control_flow(
                &ctx.window,
                target,
                &ctx.core,
                &mut ctx.state,
                window_hidden,
                poll_when_uncapped,
                poll_reload_interval,
            );
            bbb_client_clock::replay::flag(&bbb_client_clock::replay::SIMULATION_IDLE, true);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_read_for_key_v_requires_paste_modifier() {
        let key_v = PhysicalKey::Code(KeyCode::KeyV);
        assert!(!should_read_clipboard_for_key_event(
            ModifiersState::empty(),
            key_v,
            ElementState::Pressed,
        ));
        assert!(!should_read_clipboard_for_key_event(
            ModifiersState::SHIFT,
            key_v,
            ElementState::Pressed,
        ));
        assert!(!should_read_clipboard_for_key_event(
            ModifiersState::CONTROL | ModifiersState::ALT,
            key_v,
            ElementState::Pressed,
        ));
        assert!(should_read_clipboard_for_key_event(
            ModifiersState::CONTROL,
            key_v,
            ElementState::Pressed,
        ));
        assert!(should_read_clipboard_for_key_event(
            ModifiersState::SUPER,
            key_v,
            ElementState::Pressed,
        ));
        assert!(!should_read_clipboard_for_key_event(
            ModifiersState::CONTROL,
            key_v,
            ElementState::Released,
        ));
    }
}
