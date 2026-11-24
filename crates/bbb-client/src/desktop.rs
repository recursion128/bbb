use std::{cell::RefCell, rc::Rc, time::Duration};

use anyhow::Result;
use winit::{dpi::PhysicalSize, event::Event, event_loop::ActiveEventLoop, window::Fullscreen};

#[cfg(feature = "lifecycle-soak")]
use crate::lifecycle_soak;
use crate::{
    application_lifecycle::{NativeApplication, NativeEventHandler},
    audio_runtime::{AudioEventSink, NativeAudioRuntime},
    code_of_conduct::{default_code_of_conduct_store_path, CodeOfConductAcceptance},
    control::{self, AudioCounters, ControlState, SharedControlRequests},
    crash_report::{
        default_crash_report_dir, exit_after_failure, should_show_crash_dialog, CrashReportRuntime,
        RuntimeFailure, RuntimeFailureAccumulator,
    },
    event_loop_shared::{
        apply_startup_fullscreen, drive_session_event, platform_surface_size,
        record_runtime_failure, PlatformEventLoopHooks, SessionEventLoopContext,
    },
    input::DebugClipboard,
    mcp_runtime,
    narrator_runtime::{self, NarratorStatus, NativeNarratorRuntime},
    options::{ClientOptionHost, ClientOptionsFile},
    resource_pack_runtime::{default_server_resource_pack_cache_dir, ServerResourcePackRuntime},
    resource_reload::{
        failed_server_resource_pack_reload_ack, prepare_server_resource_pack_reload,
        PreparedServerResourcePackReload,
    },
    server_list::ServerListPersistence,
    session_shared::{self, LoadedCoreClientAssets, SessionCoreInputs},
    simulation::{
        commands::{command_queue, LoopWake, SimulationCommand},
        effects::ResourceReloadEffect,
        StepOutcome,
    },
    sound_subtitle_runtime::SoundSubtitleRuntime,
    startup::{
        self, build_window, create_event_loop, init_tracing, load_known_packs_manager,
        load_required_desktop_pack_roots, parse_args, run_build_info_if_requested,
        run_probe_if_requested, run_status_if_requested, start_control_api,
        start_network_if_requested,
    },
    window_icon, window_surface,
};

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
    // Raw Java text is converted by DebugClipboard before reaching this UTF-8 backend.
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

pub fn run_desktop_main() {
    init_tracing();
    let args = parse_args();
    if args.lockstep {
        bbb_client_clock::enable_virtual();
    }
    let crash_report_dir = args
        .crash_report_dir
        .clone()
        .unwrap_or_else(default_crash_report_dir);
    let show_crash_dialog = should_show_crash_dialog(
        args.probe_server,
        args.control.is_some(),
        args.mcp_bind.is_some(),
        args.lockstep,
        args.build_info,
        args.status_server.is_some(),
    );
    let mut crash_reports =
        CrashReportRuntime::new_interactive(crash_report_dir, show_crash_dialog);
    if let Err(failures) = crash_reports.guard(|| run_client(args)) {
        exit_after_failure(crash_reports.finish_failure(*failures));
    }
}

fn run_client(mut args: startup::Args) -> Result<()> {
    if run_build_info_if_requested(&args)? {
        return Ok(());
    }
    #[cfg(feature = "lifecycle-soak")]
    let lifecycle_soak = lifecycle_soak::LifecycleSoakRuntime::from_env()?;
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
            locale_is_japanese: crate::options::locale_is_japanese(
                sys_locale::get_locale().as_deref(),
            ),
            platform_is_windows: crate::platform::HOST_IS_WINDOWS,
        },
    ) {
        Ok(mut options) => {
            let previous_startup_was_incomplete = !options.started_cleanly();
            if previous_startup_was_incomplete {
                args.fullscreen = false;
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

    let client_locale = args.client_locale.clone();

    let loaded_pack_roots = load_required_desktop_pack_roots(&mut args)?;
    let pack_roots = loaded_pack_roots.roots;
    let resource_pack_options_reconciliation = loaded_pack_roots.options_reconciliation;
    let base_pack_roots = pack_roots.clone();
    let known_packs_manager = load_known_packs_manager(pack_roots.as_ref());
    let event_loop = create_event_loop()?;
    let loop_wake = std::sync::Arc::new(LoopWake::new(event_loop.create_proxy()));
    let (command_queue, command_inbox) = command_queue(std::sync::Arc::clone(&loop_wake));
    let control = ControlState::new(
        format!(
            "bbb-client {} / protocol {}",
            bbb_protocol::MC_VERSION,
            bbb_protocol::PROTOCOL_VERSION
        ),
        SharedControlRequests::new(std::sync::Arc::new(command_queue.clone())),
    );
    let snapshot = control.snapshot.clone();
    let publication_demand = control.publication.clone();
    let control_views_enabled = args.control.is_some() || args.mcp_bind.is_some();
    let (mut world, mut net_counters) = session_shared::build_initial_world_and_net_counters(
        &args,
        pack_roots.as_ref(),
        &client_locale,
    );
    world.set_known_packs_manager(known_packs_manager.clone());
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
                args.server_resource_pack_policy.is_none(),
            )
        })
        .transpose()?;
    let client_identity = startup::client_identity_from_args(&args);
    let (mut network_runtime, reconnect_options) = start_network_if_requested(
        &runtime,
        &args,
        &client_identity,
        &mut code_of_conduct_acceptance,
        known_packs_manager,
        &loop_wake,
    )?;
    let (net_events, net_commands) = network_runtime.take_channels();
    if net_events.is_none() {
        tracing::info!(
            "started in idle mode (no server specified); launch with --server <HOST:PORT> to connect"
        );
    }
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
    mcp_runtime::start_mcp_api(&runtime, args.mcp_bind, &control);
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
    let audio_started_at = bbb_client_clock::Instant::now();
    let (mut audio_runtime, audio_status) = if force_no_audio_device {
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
    tracing::info!(
        stage = "audio",
        elapsed_ms = audio_started_at.elapsed().as_secs_f64() * 1_000.0,
        "native startup stage completed"
    );
    if let (Some(path), Some(audio)) = (args.lockstep_audio_log.as_deref(), audio_runtime.as_mut())
    {
        audio.log_commands_to(path)?;
    }
    let narrator_status = NarratorStatus::from(args.client_narrator);
    world.set_available_command_sounds(
        audio_runtime
            .as_ref()
            .map(NativeAudioRuntime::available_sound_ids)
            .unwrap_or_default(),
    );
    let narrator_runtime = NativeNarratorRuntime::resolve(
        narrator_status,
        args.narrator_command.clone(),
        args.narrator_command_args.clone(),
        args.narrator_voice_volume,
    );
    let sound_subtitle_runtime = SoundSubtitleRuntime::default();
    if narrator_runtime.is_none() && narrator_status != NarratorStatus::Off {
        tracing::warn!(
            ?narrator_status,
            "narrator requested without --narrator-command and no built-in desktop TTS binary was found; continuing without TTS output"
        );
    }

    let asset_pack_roots = pack_roots.clone();
    let asset_client_locale = client_locale.clone();
    let asset_particle_status = args.client_particles.into();
    let asset_font_options = bbb_client_render::item_model::font::providers::FontOptions {
        uniform: args.client_force_unicode_font,
        japanese_variants: args.client_japanese_glyph_variants,
    };
    let asset_loader = std::thread::Builder::new()
        .name("bbb-startup-assets".to_string())
        .spawn(move || {
            session_shared::load_core_client_assets_detached(
                asset_particle_status,
                asset_font_options,
                asset_pack_roots.as_ref(),
                &asset_client_locale,
                None,
                true,
            )
        })
        .map_err(|err| anyhow::anyhow!("spawn desktop startup asset loader thread: {err}"))?;

    let fatal_failures = Rc::new(RefCell::new(RuntimeFailureAccumulator::default()));
    let application_failures = Rc::clone(&fatal_failures);
    let mut application = NativeApplication::new(move |target| {
        let window_config = startup::WindowConfig::from_args(&args);
        #[cfg(target_os = "macos")]
        let initial_fullscreen = crate::event_loop_shared::requested_window_fullscreen(
            args.fullscreen,
            args.client_fullscreen_resolution,
            target.primary_monitor().as_ref(),
            true,
        );
        #[cfg(not(target_os = "macos"))]
        let initial_fullscreen = None;
        let fullscreen_applied_at_creation = initial_fullscreen.is_some();
        let window = build_window(target, &window_config, initial_fullscreen)?;
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
        window.set_ime_allowed(false);
        let mut input = session_shared::build_configured_input_state(
            &args,
            window.has_focus(),
            narrator_runtime.is_some(),
        );
        apply_startup_fullscreen(
            &window,
            &mut input,
            fullscreen_applied_at_creation,
            args.client_fullscreen_resolution,
            window_backend.supports_exclusive_fullscreen(),
        );
        if !args.client_raw_mouse_input {
            tracing::warn!(
                "rawMouseInput=false has no effect on the winit backend: \
             DeviceEvent::MouseMotion is always raw and unfiltered"
            );
        }
        let debug_clipboard = NativeDebugClipboard::default();

        let mut renderer = pollster::block_on(
            bbb_client_render::Renderer::new_with_vsync_and_terrain_filtering_for_fabulous(
                &window,
                args.client_vsync,
                args.terrain_texture_filtering.into(),
                args.fabulous_texture_filtering,
            ),
        )?;
        let startup_window_size = window.inner_size();
        let startup_surface_size = platform_surface_size(
            std::env::consts::OS,
            matches!(window.fullscreen(), Some(Fullscreen::Exclusive(_))),
            args.client_fullscreen_resolution,
            startup_window_size,
        );
        if startup_surface_size != renderer.physical_surface_size() {
            renderer.resize(startup_surface_size)?;
        }
        #[cfg(target_os = "macos")]
        {
            use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

            let presentation = window.window_handle().ok().and_then(|handle| {
                let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
                    return None;
                };
                crate::platform_macos::metal_layer_presentation(handle.ns_view).ok()
            });
            if let Some(presentation) = presentation {
                tracing::info!(
                    drawable_width = presentation.drawable_width,
                    drawable_height = presentation.drawable_height,
                    maximum_drawable_count = presentation.maximum_drawable_count,
                    "native Metal layer presentation observed"
                );
            } else {
                tracing::warn!("native Metal layer presentation probe unavailable");
            }
        }
        tracing::info!(
            window_width = startup_window_size.width,
            window_height = startup_window_size.height,
            drawable_width = startup_surface_size.width,
            drawable_height = startup_surface_size.height,
            "native renderer drawable configured"
        );

        let LoadedCoreClientAssets {
            client_particle_status,
            mut particle_runtime,
            particle_render_runtime,
            font_options,
            mut font_atlas,
            mut item_runtime,
        } = {
            let loaded = asset_loader
                .join()
                .map_err(|_| anyhow::anyhow!("desktop startup asset loader thread panicked"))??;
            session_shared::apply_loaded_core_client_assets_to_world(&loaded, &mut world)?;
            loaded
        };

        let uploaded_gpu = session_shared::configure_and_upload_initial_gpu_assets(
            &mut renderer,
            &args,
            &pack_roots,
            &mut font_atlas,
            &mut particle_runtime,
            &particle_render_runtime,
            &mut item_runtime,
            &client_locale,
            &mut net_counters,
            &command_queue,
            true,
        )?;

        let screenshot = args.screenshot.clone();
        let screenshot_after_terrain = args.connect_server;
        let exit_after_screenshot = args.exit_after_screenshot;

        let (core, mut state) = session_shared::build_simulation_core_and_render_state(
            &args,
            pack_roots.as_ref(),
            &mut renderer,
            SessionCoreInputs {
                net_events,
                net_commands,
                server_resource_packs: server_resource_pack_runtime,
                input,
                world,
                net_counters,
                client_particle_status,
                particle_runtime,
                item_runtime,
                uploaded_gpu,
                code_of_conduct: code_of_conduct_acceptance,
                server_list,
                audio_status,
                control_views_enabled,
                publication_demand,
            },
        )?;

        if let Some(path) = &args.record_video {
            let size = renderer.physical_surface_size();
            state.recording.apply(
                control::RecordingControlRequest::Start {
                    path: path.to_string_lossy().into_owned(),
                    fps: args.record_fps,
                    bitrate: args.record_bitrate,
                },
                [size.width, size.height],
                audio_runtime.as_mut(),
            )?;
        }

        if let Some(options) = client_options.as_mut() {
            if let Some(reconciliation) = resource_pack_options_reconciliation.as_ref() {
                options.apply_resource_pack_reconciliation(reconciliation);
            }
            application_failures
                .borrow_mut()
                .attach_client_options(options.options().dump_for_report());
            let rebound: Vec<_> = crate::options::key_bindings::rebound_keys(options.options())
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
        let event_failures = Rc::clone(&application_failures);
        let mut ctx = SessionEventLoopContext {
            window,
            renderer,
            core,
            state,
            args,
            command_queue,
            command_inbox,
            snapshot,
            client_locale,
            client_options,
            base_pack_roots,
            pack_roots,
            audio_runtime,
            sound_subtitle_runtime,
            particle_render_runtime,
            font_options,
            client_particle_status,
        };
        let mut hooks = DesktopEventLoopHooks {
            runtime,
            network_runtime,
            reconnect_options,
            loop_wake,
            event_failures,
            debug_clipboard,
            narrator_runtime,
            exclusive_fullscreen_supported: window_backend.supports_exclusive_fullscreen(),
            screenshot,
            screenshot_after_terrain,
            exit_after_screenshot,
            wrote_cli_screenshot: false,
            pending_resource_reload: None,
            #[cfg(feature = "lifecycle-soak")]
            lifecycle_soak,
        };

        Ok(Box::new(move |event: Event<()>, target: &ActiveEventLoop| {
            drive_session_event(&mut ctx, &mut hooks, event, target);
        }) as NativeEventHandler)
    });
    let event_loop_result = event_loop.run_app(&mut application);
    if let Some(error) = application.take_bootstrap_error() {
        record_runtime_failure(
            &fatal_failures,
            RuntimeFailure::new(
                "platform bootstrap",
                "Native application bootstrap failed",
                error,
            ),
            None,
        );
    }
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

#[allow(clippy::too_many_arguments)]
fn reconnect_desktop_session(
    runtime: &tokio::runtime::Runtime,
    network_runtime: &mut crate::network_runtime::NetworkRuntime,
    loop_wake: &std::sync::Arc<LoopWake>,
    options: bbb_client_net::ConnectionOptions,
    net_events: &mut Option<tokio::sync::mpsc::Receiver<bbb_client_net::NetEvent>>,
    net_commands: &mut Option<tokio::sync::mpsc::Sender<bbb_client_net::NetCommand>>,
    input: &mut crate::input::ClientInputState,
    net_counters: &mut control::NetCounters,
) {
    let mut next_runtime = crate::network_runtime::NetworkRuntime::start(
        runtime,
        options,
        std::sync::Arc::clone(loop_wake),
    );
    let (next_events, next_commands) = next_runtime.take_channels();
    if !next_runtime.activate() {
        tracing::warn!("reconnected network runtime ended before activation");
    }
    *network_runtime = next_runtime;
    *net_events = next_events;
    *net_commands = next_commands;
    input.close_disconnected_screen();
    net_counters.connected = false;
    net_counters.state = Some("Connecting".to_string());
}

struct DesktopEventLoopHooks {
    runtime: tokio::runtime::Runtime,
    network_runtime: crate::network_runtime::NetworkRuntime,
    reconnect_options: Option<bbb_client_net::ConnectionOptions>,
    loop_wake: std::sync::Arc<LoopWake>,
    event_failures: Rc<RefCell<RuntimeFailureAccumulator>>,
    debug_clipboard: NativeDebugClipboard,
    narrator_runtime: Option<NativeNarratorRuntime>,
    exclusive_fullscreen_supported: bool,
    screenshot: Option<std::path::PathBuf>,
    screenshot_after_terrain: bool,
    exit_after_screenshot: bool,
    wrote_cli_screenshot: bool,
    pending_resource_reload: Option<(
        u64,
        String,
        std::sync::mpsc::Receiver<PreparedServerResourcePackReload>,
    )>,
    #[cfg(feature = "lifecycle-soak")]
    lifecycle_soak: Option<lifecycle_soak::LifecycleSoakRuntime>,
}

impl PlatformEventLoopHooks for DesktopEventLoopHooks {
    fn shutdown_platform(&mut self, ctx: &mut SessionEventLoopContext) {
        if ctx.state.recording.is_active() {
            if let Err(error) = ctx.state.recording.stop(ctx.audio_runtime.as_mut()) {
                tracing::error!(%error, "could not finalize video recording at shutdown");
            }
        }
        if ctx.core.running() {
            ctx.core.input_runtime.request_network_terminal_shutdown(
                &ctx.core.net_commands,
                &mut ctx.core.platform_effects,
            );
        }
        if let Err(error) = self.network_runtime.shutdown_and_join(&self.runtime) {
            record_runtime_failure(
                &self.event_failures,
                RuntimeFailure::new("network shutdown", "Network backend shutdown failed", error),
                Some(&ctx.core.world),
            );
        }
    }

    fn on_reconnect(&mut self, ctx: &mut SessionEventLoopContext) {
        let Some(options) = self.reconnect_options.clone() else {
            tracing::warn!("desktop reconnect requested without prior connection options");
            return;
        };
        if let Err(error) = self.network_runtime.shutdown_and_join(&self.runtime) {
            tracing::warn!(%error, "previous network shutdown failed before reconnect");
        }
        ctx.core.input_runtime.reset_net_disconnect_requested();
        reconnect_desktop_session(
            &self.runtime,
            &mut self.network_runtime,
            &self.loop_wake,
            options,
            &mut ctx.core.net_events,
            &mut ctx.core.net_commands,
            &mut ctx.core.input,
            &mut ctx.core.net_counters,
        );
    }

    fn on_before_event(&mut self, ctx: &mut SessionEventLoopContext, event: &Event<()>) {
        #[cfg(feature = "lifecycle-soak")]
        if let Some(runtime) = self.lifecycle_soak.as_mut() {
            runtime.apply_due(bbb_client_clock::now(), &ctx.window, &mut ctx.renderer);
        }
        if matches!(event, Event::NewEvents(_)) {
            ctx.state.recording.poll(ctx.audio_runtime.as_mut());
        }
    }

    fn resolve_surface_resize(
        &mut self,
        ctx: &SessionEventLoopContext,
        size: PhysicalSize<u32>,
    ) -> Option<PhysicalSize<u32>> {
        Some(platform_surface_size(
            std::env::consts::OS,
            matches!(ctx.window.fullscreen(), Some(Fullscreen::Exclusive(_))),
            ctx.args.client_fullscreen_resolution,
            size,
        ))
    }

    fn on_resize_error(
        &mut self,
        ctx: &SessionEventLoopContext,
        old_size: PhysicalSize<u32>,
        new_size: PhysicalSize<u32>,
        error: anyhow::Error,
    ) {
        record_runtime_failure(
            &self.event_failures,
            RuntimeFailure::new(
                "renderer resize",
                format!(
                    "Failed to resize renderer resources from {}x{} to {}x{}",
                    old_size.width, old_size.height, new_size.width, new_size.height
                ),
                error,
            ),
            Some(&ctx.core.world),
        );
    }

    fn clipboard_and_narrator_mut(
        &mut self,
    ) -> (
        &mut dyn DebugClipboard,
        Option<&mut dyn narrator_runtime::SystemNarrator>,
    ) {
        (
            &mut self.debug_clipboard,
            self.narrator_runtime
                .as_mut()
                .map(|n| n as &mut dyn narrator_runtime::SystemNarrator),
        )
    }

    fn render_frame(
        &mut self,
        ctx: &mut SessionEventLoopContext,
        target: &ActiveEventLoop,
    ) -> Option<Duration> {
        self.wrote_cli_screenshot = false;
        let terrain_ready_for_screenshot =
            !self.screenshot_after_terrain || ctx.state.terrain_upload.has_uploaded_chunks();
        let cli_screenshot_path = self
            .screenshot
            .as_deref()
            .filter(|_| terrain_ready_for_screenshot);
        let render_path = cli_screenshot_path.or_else(|| {
            ctx.state
                .pending_control_screenshots
                .front()
                .map(|pending| pending.path.as_path())
        });

        let profiler_render_started_at = bbb_client_clock::Instant::now();
        let capture = ctx.state.recording.capture_due();
        let rendered = match ctx.renderer.render_with_capture(render_path, capture) {
            Ok(rendered) => rendered,
            Err(err) => {
                record_runtime_failure(
                    &self.event_failures,
                    RuntimeFailure::new(
                        "renderer frame",
                        "Renderer frame failed",
                        format_args!("{err:#}"),
                    ),
                    Some(&ctx.core.world),
                );
                target.exit();
                return None;
            }
        };
        let screenshot_result = match rendered {
            bbb_client_render::RenderOutcome::Skipped => None,
            bbb_client_render::RenderOutcome::Presented {
                screenshot,
                capture,
            } => {
                if let Some(pixels) = capture {
                    ctx.state
                        .recording
                        .submit(pixels, ctx.audio_runtime.as_mut());
                }
                screenshot
            }
        };
        if let Some(result) = screenshot_result {
            if cli_screenshot_path.is_some() {
                if let Err(error) = result {
                    record_runtime_failure(
                        &self.event_failures,
                        RuntimeFailure::new(
                            "screenshot",
                            "Screenshot failed",
                            format_args!("{error:#}"),
                        ),
                        Some(&ctx.core.world),
                    );
                    target.exit();
                    return None;
                }
                self.wrote_cli_screenshot = true;
            } else if let Some(pending) = ctx.state.pending_control_screenshots.pop_front() {
                pending.complete(result, &ctx.command_queue);
            }
        }
        Some(profiler_render_started_at.elapsed())
    }

    fn is_window_minimized(&self, window: &winit::window::Window) -> bool {
        window.is_minimized().unwrap_or(false)
    }

    fn after_finish_frame(&mut self, _ctx: &mut SessionEventLoopContext, target: &ActiveEventLoop) {
        if self.wrote_cli_screenshot {
            self.screenshot = None;
            if self.exit_after_screenshot {
                target.exit();
            }
        }
    }

    fn collect_pre_step_commands(
        &mut self,
        ctx: &mut SessionEventLoopContext,
        commands: &mut Vec<SimulationCommand>,
        queued: &mut u64,
    ) {
        if let Some((generation, reload_locale, rx)) = self.pending_resource_reload.as_ref() {
            match rx.try_recv() {
                Ok(prepared) => {
                    self.pending_resource_reload = None;
                    commands.extend(ctx.commit_prepared_reload(prepared));
                    *queued += 2;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    let generation = *generation;
                    let reload_locale = reload_locale.clone();
                    self.pending_resource_reload = None;
                    let ack = failed_server_resource_pack_reload_ack(
                        generation,
                        ctx.pack_roots.as_ref(),
                        ctx.core.item_runtime.as_ref(),
                        &reload_locale,
                        "prepare",
                        "background resource reload worker panicked".to_string(),
                    );
                    commands.push(SimulationCommand::ResourceReloadAck(Box::new(ack)));
                    *queued += 1;
                }
            }
        }
    }

    fn step_simulation(
        &mut self,
        ctx: &mut SessionEventLoopContext,
        target: &ActiveEventLoop,
        now: bbb_client_clock::Instant,
        commands: Vec<SimulationCommand>,
    ) -> Option<Option<StepOutcome>> {
        let step = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ctx.core.step(
                now,
                commands,
                ctx.audio_runtime
                    .as_mut()
                    .map(|runtime| runtime as &mut dyn AudioEventSink),
            )
        }));
        match step {
            Ok(outcome) => Some(outcome),
            Err(payload) => {
                record_runtime_failure(
                    &self.event_failures,
                    RuntimeFailure::from_panic("simulation step", payload.as_ref()),
                    Some(&ctx.core.world),
                );
                target.exit();
                None
            }
        }
    }

    fn preferred_video_mode(
        &self,
        args: &startup::Args,
    ) -> Option<window_surface::PreferredVideoMode> {
        args.client_fullscreen_resolution
    }

    fn exclusive_fullscreen_supported(&self) -> bool {
        self.exclusive_fullscreen_supported
    }

    fn dispatch_resource_reload(
        &mut self,
        ctx: &mut SessionEventLoopContext,
        reload: ResourceReloadEffect,
    ) {
        if bbb_client_clock::is_virtual() {
            ctx.apply_virtual_clock_reload(reload);
            return;
        }
        let generation = reload.request.generation;
        let reload_locale = ctx.client_locale.clone();
        let base = ctx.base_pack_roots.clone();
        let active = ctx.pack_roots.clone();
        let has_audio = ctx.audio_runtime.is_some();
        let font_options = ctx.font_options;
        let max_texture_dimension_2d = ctx.renderer.max_texture_dimension_2d();
        let client_particle_status = ctx.client_particle_status;
        let username = ctx.args.username.clone();
        let wake = std::sync::Arc::clone(&self.loop_wake);
        let (tx, rx) = std::sync::mpsc::channel();
        match std::thread::Builder::new()
            .name("bbb-resource-reload".to_string())
            .spawn(move || {
                let prepared = prepare_server_resource_pack_reload(
                    reload,
                    base,
                    active,
                    has_audio,
                    reload_locale,
                    font_options,
                    max_texture_dimension_2d,
                    client_particle_status,
                    username,
                );
                let _ = tx.send(prepared);
                wake.wake();
            }) {
            Ok(_) => {
                self.pending_resource_reload = Some((generation, ctx.client_locale.clone(), rx));
            }
            Err(err) => {
                let ack = failed_server_resource_pack_reload_ack(
                    generation,
                    ctx.pack_roots.as_ref(),
                    ctx.core.item_runtime.as_ref(),
                    &ctx.client_locale,
                    "prepare",
                    format!("spawn background resource reload thread: {err}"),
                );
                ctx.command_queue
                    .push(SimulationCommand::ResourceReloadAck(Box::new(ack)));
            }
        }
    }

    fn before_post_step_schedule(&mut self, _ctx: &mut SessionEventLoopContext) {
        if let Some(narrator) = self.narrator_runtime.as_mut() {
            narrator.tick();
        }
    }

    fn schedule_policy(&self, _window: &winit::window::Window) -> (bool, bool, Option<Duration>) {
        (false, true, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_loop_shared::save_client_options_and_enqueue_broadcast;
    use bbb_client_render::item_model::NativeItemRuntime;
    use winit::dpi::PhysicalSize;

    #[test]
    fn client_options_save_completion_follows_missing_or_failed_file_owner() {
        let information = bbb_protocol::packets::ClientInformation {
            language: "en_pt".to_string(),
            ..Default::default()
        };
        let writes = [crate::options::ClientOptionWrite {
            key: "lang",
            value: crate::options::ClientOptionValue::Text("en_pt"),
        }];

        let (commands, inbox) = command_queue(std::sync::Arc::new(LoopWake::default()));
        assert!(save_client_options_and_enqueue_broadcast(
            None,
            &writes,
            information.clone(),
            &commands
        )
        .is_none());
        assert!(matches!(
            inbox.drain().0.as_slice(),
            [SimulationCommand::ClientOptionsSaveCompleted(completed)]
                if completed.language == "en_pt"
        ));

        let path = std::env::temp_dir()
            .join(format!("bbb-options-save-failure-{}", std::process::id()))
            .join("missing-parent")
            .join("options.txt");
        let mut file = ClientOptionsFile::load(path, ClientOptionHost::default()).unwrap();
        assert!(matches!(
            save_client_options_and_enqueue_broadcast(
                Some(&mut file),
                &writes,
                information,
                &commands,
            ),
            Some(Err(_))
        ));
        assert!(matches!(
            inbox.drain().0.as_slice(),
            [SimulationCommand::ClientOptionsSaveCompleted(completed)]
                if completed.language == "en_pt"
        ));
    }

    #[test]
    fn only_macos_exclusive_mode_pins_the_physical_metal_drawable() {
        let preferred = window_surface::PreferredVideoMode::parse("2560x1600@60:24");
        let observed = PhysicalSize::new(1440, 900);

        assert_eq!(
            platform_surface_size("macos", true, preferred, observed),
            PhysicalSize::new(2560, 1600)
        );
        assert_eq!(
            platform_surface_size("macos", false, preferred, observed),
            observed
        );
        assert_eq!(
            platform_surface_size("linux", true, preferred, observed),
            observed
        );
        assert_eq!(
            platform_surface_size("macos", true, preferred, PhysicalSize::new(0, 0)),
            PhysicalSize::new(0, 0)
        );
    }

    #[test]
    fn item_particle_sprite_uvs_reuse_item_atlas_rects() {
        let runtime = NativeItemRuntime::empty_for_test();
        let item_uvs = runtime.atlas_sprite_uvs();
        let particle_uvs = crate::resource_reload::item_particle_sprite_uvs(&runtime);

        assert_eq!(particle_uvs.len(), item_uvs.len());
        assert_eq!(particle_uvs[0].uv.min, item_uvs[0].uv.min);
        assert_eq!(particle_uvs[0].uv.max, item_uvs[0].uv.max);
        assert_eq!(particle_uvs[0].has_translucent, item_uvs[0].has_translucent);
    }

    #[test]
    fn desktop_reconnect_restarts_network_runtime_and_closes_disconnected_screen() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let options = bbb_client_net::ConnectionOptions::offline(
            format!("127.0.0.1:{port}"),
            "ReconnectTester",
        )
        .unwrap();
        let loop_wake = std::sync::Arc::new(LoopWake::default());
        let mut network_runtime = crate::network_runtime::NetworkRuntime::idle();
        let mut net_events = None;
        let mut net_commands = None;
        let mut input = crate::input::ClientInputState::default();
        let mut net_counters = control::NetCounters {
            connected: true,
            state: Some("Play".to_string()),
            ..control::NetCounters::default()
        };
        input.open_disconnected_screen(
            crate::input::disconnected_screen::DisconnectedScreenTitle::ConnectionLost,
            crate::input::disconnected_screen::DisconnectedScreenDetails {
                reason: bbb_client_net::DisconnectReason::from_plain("Lost connection"),
                report_directory: None,
                bug_report_link: None,
            },
        );

        reconnect_desktop_session(
            &runtime,
            &mut network_runtime,
            &loop_wake,
            options,
            &mut net_events,
            &mut net_commands,
            &mut input,
            &mut net_counters,
        );

        assert!(input.disconnected_screen_runtime().is_none());
        assert!(net_events.is_some());
        assert!(net_commands.is_some());
        assert!(!net_counters.connected);
        assert_eq!(net_counters.state.as_deref(), Some("Connecting"));
        network_runtime.shutdown_and_join(&runtime).unwrap();
    }
}
