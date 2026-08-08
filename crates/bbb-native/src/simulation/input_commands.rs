//! Canonical application of typed winit and control commands.

use std::{
    cell::RefCell,
    time::{Duration, Instant},
};

use bbb_control::{CodeOfConductControlRequest, ControlWriteRequest, InputControlRequest};
use bbb_item_model::NativeItemRuntime;
use bbb_net::NetCommand;
use bbb_protocol::{
    packets::{ClientInformation, CustomClickAction},
    ComponentClickEvent,
};
use bbb_renderer::{
    HudBookHitTarget, HudBookMode, HudChatHitTarget, HudChatHitTargetKind, HudDeathScreenHitTarget,
    HudDisconnectedScreenHitTarget, HudFontGlyphMap, HudGameEventHitTarget, HudPauseDraftButton,
    HudPlayerReportingHitTarget, HudSocialInteractionsHitTarget,
};
use bbb_world::WorldStore;
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::{KeyCode, PhysicalKey},
};

use super::{
    commands::{DebugProfilingLifecycle, SequencedSimulationCommand, SimulationCommand},
    effects::{
        DebugProfilingCommand, PlatformEffect, PlatformEffectCollector,
        ScreenNarrationScheduleInput,
    },
};
use crate::{
    audio_runtime::AudioEventSink,
    code_of_conduct::CodeOfConductAcceptance,
    code_of_conduct_overlay::{CodeOfConductOverlayProjection, CodeOfConductOverlayState},
    input::{
        death_screen::DeathScreenPlatformRequest,
        disconnected_screen::{
            DisconnectedScreenDetails, DisconnectedScreenPlatformRequest, DisconnectedScreenTitle,
        },
        flush_local_interaction_sound_actions,
        game_event_screen::GameEventScreenPlatformRequest,
        handle_advancements_screen_cursor_moved, handle_advancements_screen_mouse_input,
        handle_advancements_screen_mouse_wheel, handle_focus_change, handle_inventory_cursor_moved,
        handle_inventory_mouse_input_with_item_runtime, handle_inventory_mouse_wheel,
        handle_key_input_with_item_runtime,
        handle_mouse_input_at_partial_tick_with_effects_and_presentation, handle_mouse_motion,
        handle_mouse_wheel, handle_text_input_with_item_runtime, inventory_screen_layout,
        queue_chat_command, queue_debug_recreate_server_query_request, queue_lectern_button_click,
        recipe_book_type_for_background, recipe_book_type_settings, release_active_input,
        reporting::PlayerReportingPlatformRequest,
        social::SocialInteractionsPlatformRequest,
        CommandSuggestionTextMeasurer, DebugClipboard, DebugFeatureCountRequest,
        DebugFrustumRequest, DebugOptionsSearchTextMeasurer, DebugProfilingToggleRequest,
        PauseScreenLinkRequest, TerrainReloadRequestSink,
    },
    resource_pack_runtime::{ResourcePackReloadFinish, ServerResourcePackRuntime},
    runtime::{
        apply_code_of_conduct_request, apply_control_input_request_with_effects,
        apply_control_net_request, handle_dialog_cursor_moved_with_measurer,
        handle_dialog_key_input_with_modifiers_for_surface_and_measurer,
        handle_dialog_mouse_input_with_modifiers_and_measurer, handle_dialog_mouse_wheel,
        handle_dialog_text_input_with_measurer, request_net_disconnect, DialogTextMeasurer,
    },
};

/// Simulation-owned state that used to live as mutable locals in the winit
/// callback.
#[derive(Debug)]
pub(crate) struct SimulationInputRuntime {
    surface_size: PhysicalSize<u32>,
    /// The projected logical GUI surface (`Window.setGuiScale`, ceiling
    /// division). Every screen layout, hit test, overlay and narration helper
    /// in this runtime consumes the logical size; `surface_size` stays the
    /// physical framebuffer for renderer/world consumers that read the
    /// publication.
    gui_surface_size: PhysicalSize<u32>,
    /// `Window.guiScale` after `Minecraft.resizeGui` (Window.java:431).
    pub(crate) gui_scale: u32,
    /// Projected GUI dimensions (`Window.setGuiScale`, ceiling division).
    pub(crate) gui_scaled_width: u32,
    pub(crate) gui_scaled_height: u32,
    /// `Options.guiScale` cap (0 = auto) and `isEnforceUnicode`.
    pub(crate) gui_scale_max: u32,
    pub(crate) enforce_unicode: bool,
    /// A zero-size framebuffer callback means minimized and keeps the last
    /// usable surface size (Window.onFramebufferResize, Window.java:267-297).
    pub(crate) minimized: bool,
    cursor_position: Option<PhysicalPosition<f64>>,
    cursor_captured: bool,
    text_measurements: HudFontGlyphMap,
    code_of_conduct_overlay: CodeOfConductOverlayState,
    net_disconnect_requested: bool,
    shutdown_requested: bool,
    terrain_readiness: Option<(u64, usize, bool)>,
    last_applied_sequence: u64,
    /// Vanilla `KeyboardHandler.lastPreeditEvent`: stored on every pre-edit
    /// callback so a focus change can hand the open composition to the
    /// gaining element (`Minecraft.onTextInputFocusChange`).
    last_ime_preedit: Option<(String, Option<(usize, usize)>)>,
    ime_preedit_target: ImePreeditTarget,
    /// `TextInputManager.textInputEnabled`: the last state handed to the
    /// window, so a steady screen does not re-request it every step.
    text_input_enabled: bool,
    /// What `Options.buildPlayerInformation` would return. Nothing this client
    /// changes at runtime feeds it, so `broadcastOptions` re-sends the values
    /// the session was configured with.
    client_information: ClientInformation,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SimulationInputPublication {
    pub(crate) surface_size: PhysicalSize<u32>,
    pub(crate) gui_scale: u32,
    pub(crate) gui_scaled_width: u32,
    pub(crate) gui_scaled_height: u32,
    pub(crate) minimized: bool,
    pub(crate) cursor_position: Option<PhysicalPosition<f64>>,
    pub(crate) cursor_captured: bool,
    pub(crate) code_of_conduct_overlay: CodeOfConductOverlayProjection,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SimulationInputProfile {
    pub(crate) keyboard: Duration,
    pub(crate) mouse: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SimulationCommandProfileKind {
    Keyboard,
    Mouse,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImePreeditTarget {
    None,
    Dialog,
    SignEditor,
}

impl SimulationInputRuntime {
    pub(crate) fn new(
        surface_size: PhysicalSize<u32>,
        cursor_captured: bool,
        client_information: ClientInformation,
    ) -> Self {
        Self {
            surface_size,
            gui_surface_size: PhysicalSize::new(
                surface_size.width.max(1),
                surface_size.height.max(1),
            ),
            minimized: surface_size.width == 0 || surface_size.height == 0,
            text_input_enabled: false,
            gui_scale: 1,
            gui_scaled_width: surface_size.width.max(1),
            gui_scaled_height: surface_size.height.max(1),
            gui_scale_max: 0,
            enforce_unicode: false,
            cursor_position: None,
            cursor_captured,
            text_measurements: HudFontGlyphMap::default(),
            code_of_conduct_overlay: CodeOfConductOverlayState::default(),
            net_disconnect_requested: false,
            shutdown_requested: false,
            terrain_readiness: None,
            last_applied_sequence: 0,
            last_ime_preedit: None,
            ime_preedit_target: ImePreeditTarget::None,
            client_information,
        }
    }

    /// `Options.guiScale` / `isEnforceUnicode` are startup configuration; the
    /// owner recomputes the current projection immediately so the published
    /// window state matches the configured options before the first resize.
    pub(crate) fn set_gui_scale_options(&mut self, gui_scale_max: u32, enforce_unicode: bool) {
        self.gui_scale_max = gui_scale_max;
        self.enforce_unicode = enforce_unicode;
        let (gui_scale, gui_scaled_width, gui_scaled_height) =
            project_gui_scale(self.surface_size, gui_scale_max, enforce_unicode);
        self.gui_scale = gui_scale;
        self.gui_scaled_width = gui_scaled_width;
        self.gui_scaled_height = gui_scaled_height;
        self.gui_surface_size = PhysicalSize::new(gui_scaled_width, gui_scaled_height);
    }

    /// Vanilla `MouseHandler.isMouseGrabbed`: the capture owner the turn
    /// path mirrors before advancing or resetting the smooth camera.
    pub(crate) fn cursor_captured(&self) -> bool {
        self.cursor_captured
    }

    pub(crate) fn publication(&self) -> SimulationInputPublication {
        SimulationInputPublication {
            surface_size: self.surface_size,
            gui_scale: self.gui_scale,
            gui_scaled_width: self.gui_scaled_width,
            gui_scaled_height: self.gui_scaled_height,
            minimized: self.minimized,
            cursor_position: self.cursor_position,
            cursor_captured: self.cursor_captured,
            code_of_conduct_overlay: self.code_of_conduct_overlay.projection(),
        }
    }

    pub(crate) fn running(&self) -> bool {
        !self.shutdown_requested
    }

    pub(crate) fn gui_surface_size(&self) -> PhysicalSize<u32> {
        self.gui_surface_size
    }

    pub(crate) fn player_section_ready(
        &self,
        level_generation: u64,
        level_load_start: usize,
    ) -> bool {
        self.terrain_readiness
            .is_some_and(|(generation, load_start, ready)| {
                generation == level_generation && load_start == level_load_start && ready
            })
    }

    pub(crate) fn request_network_terminal_shutdown(
        &mut self,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        platform: &mut PlatformEffectCollector<'_>,
    ) {
        if self.shutdown_requested {
            return;
        }
        self.shutdown_requested = true;
        request_net_disconnect(net_commands, &mut self.net_disconnect_requested);
        platform.submit(PlatformEffect::Exit);
    }

    pub(crate) fn request_terminal_screen_cursor(
        &mut self,
        platform: &mut PlatformEffectCollector<'_>,
    ) {
        self.request_cursor_capture(false, platform);
    }

    pub(crate) fn apply_commands(
        &mut self,
        commands: Vec<SequencedSimulationCommand>,
        context: &mut SimulationInputContext<'_, '_, '_>,
        profiling_enabled: bool,
    ) -> SimulationInputProfile {
        self.code_of_conduct_overlay.sync(
            context.world,
            self.gui_surface_size,
            &self.text_measurements,
        );
        let mut profile = SimulationInputProfile::default();
        for command in commands {
            if self.shutdown_requested {
                break;
            }
            debug_assert!(command.sequence > self.last_applied_sequence);
            self.last_applied_sequence = command.sequence;
            let profile_kind = simulation_command_profile_kind(&command.command);
            let profile_started_at = profiling_enabled.then(Instant::now);
            self.apply_command(command.command, context);
            let Some(profile_started_at) = profile_started_at else {
                continue;
            };
            let elapsed = profile_started_at.elapsed();
            match profile_kind {
                SimulationCommandProfileKind::Keyboard => {
                    profile.keyboard = profile.keyboard.saturating_add(elapsed);
                }
                SimulationCommandProfileKind::Mouse => {
                    profile.mouse = profile.mouse.saturating_add(elapsed);
                }
                SimulationCommandProfileKind::Other => {}
            }
        }
        if self.shutdown_requested {
            return profile;
        }
        // `Minecraft.pauseIfInactive` (Minecraft.java:1382-1390) runs after
        // every command for the step is applied: a `FocusChanged` arriving in
        // the same step must update `input.focused` first, so a focus-regained
        // window never spuriously opens the pause screen. The 500 ms delay is
        // strict (`> 500L`) and an already-open screen suppresses reopening.
        context.input.pause_if_inactive(context.now, context.world);
        self.sync_ime_preedit_focus(context);
        self.sync_text_input_enabled(context);
        context
            .input
            .sync_respawn_toggle_reset(context.net_counters);
        context
            .world
            .set_spectator_outlines_key_down(context.input.spectator_outlines_key_down());
        let queued_effects = RefCell::new(Vec::new());
        let mut clipboard = SimulationClipboard {
            read: None,
            effects: &queued_effects,
        };
        context
            .input
            .consume_debug_recreate_server_query_response(context.world, &mut clipboard);
        let _ = context.input.command_suggestion_hud_state(
            context.world,
            &self.text_measurements,
            self.gui_surface_size(),
        );
        flush_platform_effects(&queued_effects, context.platform);
        if self.code_of_conduct_overlay.is_visible(context.world)
            || simulation_wants_cursor(context.input, context.world)
        {
            if context.world.dialog_screen_is_open() {
                context.input.sync_dialog_native_previous_screen(
                    context.world,
                    context.net_counters,
                    context.net_commands,
                );
            } else if !context.input.chat_restrictions_screen_is_open() {
                context.input.interrupt_chat_screen();
            }
            self.request_cursor_capture(false, context.platform);
            release_active_input(
                context.input,
                context.world,
                context.net_counters,
                context.net_commands,
            );
        } else if context.input.command_entry_is_active() {
            self.request_cursor_capture(false, context.platform);
        }
        profile
    }

    fn apply_command(
        &mut self,
        command: SimulationCommand,
        context: &mut SimulationInputContext<'_, '_, '_>,
    ) {
        match command {
            SimulationCommand::FocusChanged(focused) => self.apply_focus(focused, context),
            SimulationCommand::Key {
                physical_key,
                state,
                clipboard_text,
            } => self.apply_key(physical_key, state, clipboard_text, context),
            SimulationCommand::MouseButton {
                button,
                state,
                position,
                chat_target,
                book_target,
                disconnected_screen_target,
                death_screen_target,
                game_event_target,
                player_reporting_target,
                social_interactions_target,
                pause_draft_target,
            } => {
                self.apply_mouse_button(
                    button,
                    state,
                    position,
                    chat_target,
                    book_target,
                    disconnected_screen_target,
                    death_screen_target,
                    game_event_target,
                    player_reporting_target,
                    social_interactions_target,
                    pause_draft_target,
                    context,
                );
            }
            SimulationCommand::MouseMotion { delta } => self.apply_mouse_motion(delta, context),
            SimulationCommand::MouseWheel { delta } => self.apply_mouse_wheel(delta, context),
            SimulationCommand::CursorMoved {
                position,
                disconnected_screen_target,
                death_screen_target,
                game_event_target,
                player_reporting_target,
                social_interactions_target,
                pause_draft_target,
            } => {
                self.apply_cursor_moved(
                    position,
                    disconnected_screen_target,
                    death_screen_target,
                    game_event_target,
                    player_reporting_target,
                    social_interactions_target,
                    pause_draft_target,
                    context,
                );
            }
            SimulationCommand::GameEventScreenRenderDelta {
                generation,
                delta_ticks,
            } => {
                context.input.advance_win_screen_for_generation(
                    context.net_counters,
                    context.world,
                    context.net_commands,
                    generation,
                    delta_ticks,
                    i32::try_from(context.win_screen_resources.total_scroll_length)
                        .unwrap_or(i32::MAX),
                    i32::try_from(self.gui_surface_size().height).unwrap_or(i32::MAX),
                );
            }
            SimulationCommand::HudAirBubblePopSound {
                level_generation,
                position,
                volume,
                pitch,
            } => {
                apply_hud_air_bubble_pop_sound(
                    context.world,
                    context.audio_events,
                    level_generation,
                    position,
                    volume,
                    pitch,
                );
            }
            SimulationCommand::CursorLeft => {
                self.cursor_position = None;
                self.code_of_conduct_overlay.handle_cursor_left();
                context.input.handle_disconnected_screen_hover(None);
                context.input.handle_death_screen_hover(None);
                context.input.handle_game_event_screen_hover(None);
                context
                    .input
                    .handle_social_interactions_hover(context.world, None);
                context.input.update_screen_cursor_position(None);
                context.input.handle_sign_editor_cursor_moved(None);
                context
                    .input
                    .handle_structure_block_editor_cursor_moved(None);
                context
                    .input
                    .handle_jigsaw_block_editor_cursor_moved(None, self.gui_surface_size());
                context.input.handle_test_block_editor_cursor_moved(None);
                context
                    .input
                    .handle_test_instance_block_editor_cursor_moved(None);
                context
                    .input
                    .handle_command_suggestion_cursor_moved(context.world, None);
            }
            SimulationCommand::ImePreedit { text, cursor_range } => {
                self.apply_ime_preedit(&text, cursor_range, context);
            }
            SimulationCommand::ImeCommit(text) => {
                // A commit ends the composition; the next pre-edit callback
                // (usually the empty one that follows) restarts the store.
                self.last_ime_preedit = None;
                self.apply_ime_commit(&text, context);
            }
            SimulationCommand::ImeDisabled => {
                self.last_ime_preedit = None;
                context.input.clear_dialog_ime_preedit();
                context.input.clear_sign_editor_ime_preedit();
            }
            SimulationCommand::SurfaceResized(size) => {
                self.apply_surface_resized(size, context);
            }
            SimulationCommand::TextMeasurements(measurements) => {
                self.text_measurements = measurements;
                self.code_of_conduct_overlay.sync(
                    context.world,
                    self.gui_surface_size(),
                    &self.text_measurements,
                );
            }
            SimulationCommand::CursorCaptureResult { captured } => {
                self.cursor_captured = captured;
            }
            SimulationCommand::ScreenshotSaved { name, error } => {
                crate::input::ClientInputState::push_screenshot_saved_message(
                    Some(context.world),
                    &name,
                    error.as_deref(),
                );
            }
            SimulationCommand::ClipboardWriteResult { success } => {
                if !success {
                    tracing::warn!("platform clipboard write failed");
                }
            }
            SimulationCommand::OpenUriResult { url, success } => {
                if !success {
                    tracing::warn!(%url, "platform URI open failed");
                }
            }
            SimulationCommand::OpenPathResult { path, success } => {
                if !success {
                    tracing::warn!(path = %path.display(), "platform path open failed");
                }
            }
            SimulationCommand::ResourceReloadAck(ack) => {
                match apply_resource_reload_ack(*ack, context) {
                    ResourceReloadAckEffect::None => {}
                    ResourceReloadAckEffect::ShowLoadFailureToast => {
                        context.world.apply_resource_pack_load_failure();
                    }
                    ResourceReloadAckEffect::AbortRecovery { error } => {
                        request_net_disconnect(
                            context.net_commands,
                            &mut self.net_disconnect_requested,
                        );
                        context.world.end_connection_session();
                        context.world.apply_resource_pack_load_failure();
                        context.input.open_disconnected_screen(
                            DisconnectedScreenTitle::ConnectionLost,
                            DisconnectedScreenDetails {
                                reason: bbb_net::DisconnectReason::from_plain(format!(
                                    "Resource reload failed: {error}"
                                )),
                                report_directory: None,
                                bug_report_link: None,
                            },
                        );
                        if let Some(audio_events) = context.audio_events.as_deref_mut() {
                            audio_events.stop_all_sounds();
                        }
                        self.request_terminal_screen_cursor(context.platform);
                    }
                }
            }
            SimulationCommand::TerrainReadiness {
                level_generation,
                level_load_start,
                player_section_compiled_and_visible,
            } => {
                self.terrain_readiness = Some((
                    level_generation,
                    level_load_start,
                    player_section_compiled_and_visible,
                ));
            }
            SimulationCommand::DebugProfilingLifecycle(event) => match event {
                DebugProfilingLifecycle::Started => {
                    context.input.apply_debug_profiling_started(context.world);
                }
                DebugProfilingLifecycle::Stopped {
                    duration,
                    ticks,
                    ticks_per_second,
                } => context.input.apply_debug_profiling_stopped(
                    context.world,
                    duration,
                    ticks,
                    ticks_per_second,
                ),
                DebugProfilingLifecycle::ReportSaved { path } => {
                    context
                        .input
                        .apply_debug_profiling_report_saved(context.world, &path);
                }
                DebugProfilingLifecycle::ReportFailed { error } => {
                    context
                        .input
                        .apply_debug_profiling_report_failed(context.world, &error);
                }
            },
            SimulationCommand::ServicesBlockListCompletion(completion) => {
                if !context
                    .services_block_lists
                    .apply_completion(completion, context.world)
                {
                    tracing::debug!("ignored stale Services block-list completion");
                }
            }
            SimulationCommand::PlayerReportSubmissionCompletion(completion) => {
                if !context
                    .services_block_lists
                    .apply_player_report_submission_completion(
                        completion,
                        context.input,
                        context.world,
                    )
                {
                    tracing::debug!("ignored stale player-report submission completion");
                }
            }
            SimulationCommand::Control(request) => self.apply_control(request, context),
            SimulationCommand::Shutdown => self.request_shutdown(context),
        }
    }

    fn apply_focus(&mut self, focused: bool, context: &mut SimulationInputContext<'_, '_, '_>) {
        if !focused {
            self.request_cursor_capture(false, context.platform);
        }
        handle_focus_change(
            context.input,
            context.world,
            context.net_counters,
            context.net_commands,
            focused,
        );
    }

    fn apply_surface_resized(
        &mut self,
        size: PhysicalSize<u32>,
        context: &mut SimulationInputContext<'_, '_, '_>,
    ) {
        self.set_surface_size(size);
        self.code_of_conduct_overlay.sync(
            context.world,
            self.gui_surface_size,
            &self.text_measurements,
        );
    }

    /// `Window.onFramebufferResize` (Window.java:267-297) plus
    /// `Minecraft.resizeGui` (Minecraft.java:1434-1442): zero dimensions mean
    /// minimized and keep the last usable framebuffer; nonzero sizes replace it
    /// and recompute the GUI scale and projection.
    fn set_surface_size(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            self.minimized = true;
            return;
        }
        self.minimized = false;
        self.surface_size = size;
        let (gui_scale, gui_scaled_width, gui_scaled_height) =
            project_gui_scale(size, self.gui_scale_max, self.enforce_unicode);
        self.gui_scale = gui_scale;
        self.gui_scaled_width = gui_scaled_width;
        self.gui_scaled_height = gui_scaled_height;
        self.gui_surface_size = PhysicalSize::new(gui_scaled_width, gui_scaled_height);
    }

    fn apply_cursor_moved(
        &mut self,
        position: PhysicalPosition<f64>,
        disconnected_screen_target: Option<HudDisconnectedScreenHitTarget>,
        death_screen_target: Option<HudDeathScreenHitTarget>,
        game_event_target: Option<HudGameEventHitTarget>,
        player_reporting_target: Option<HudPlayerReportingHitTarget>,
        social_interactions_target: Option<HudSocialInteractionsHitTarget>,
        pause_draft_target: Option<HudPauseDraftButton>,
        context: &mut SimulationInputContext<'_, '_, '_>,
    ) {
        let previous_cursor_position = self.cursor_position;
        self.cursor_position = Some(position);
        if context.input.disconnected_screen_runtime().is_some() {
            context
                .input
                .handle_disconnected_screen_hover(disconnected_screen_target);
            context
                .platform
                .submit(PlatformEffect::DisconnectedScreenInput(
                    ScreenNarrationScheduleInput::MouseMove,
                ));
            self.request_cursor_capture(false, context.platform);
            return;
        }
        if context.input.chat_restrictions_screen_is_open() {
            context.platform.submit(PlatformEffect::ChatScreenInput(
                ScreenNarrationScheduleInput::MouseMove,
            ));
            self.request_cursor_capture(false, context.platform);
            return;
        }
        if context.input.chat_entry_is_active() {
            context.platform.submit(PlatformEffect::ChatScreenInput(
                ScreenNarrationScheduleInput::MouseMove,
            ));
        }
        if self.code_of_conduct_overlay.handle_cursor_moved(
            context.world,
            previous_cursor_position,
            self.cursor_position,
            self.gui_surface_size(),
            &self.text_measurements,
        ) {
            self.request_cursor_capture(false, context.platform);
            return;
        }
        if context.input.handle_debug_game_mode_switcher_cursor_moved(
            self.cursor_position,
            self.gui_surface_size(),
        ) {
            return;
        }
        if context.input.debug_options_screen_is_open() {
            context
                .input
                .handle_debug_options_screen_cursor_moved_with_text_measurer(
                    self.cursor_position,
                    self.gui_surface_size(),
                    &self.text_measurements,
                );
            context
                .platform
                .submit(PlatformEffect::DebugOptionsScreenInput(
                    ScreenNarrationScheduleInput::MouseMove,
                ));
            return;
        }
        if context
            .input
            .handle_debug_pause_screen_cursor_moved(self.cursor_position)
        {
            context.input.handle_pause_draft_hover(pause_draft_target);
            return;
        }
        if context.world.death_screen().is_some() {
            context.input.handle_death_screen_hover(death_screen_target);
            context.platform.submit(PlatformEffect::DeathScreenInput(
                ScreenNarrationScheduleInput::MouseMove,
            ));
            context
                .input
                .update_screen_cursor_position(self.cursor_position);
            return;
        }
        if context.world.game_event_screen().is_some() {
            context
                .input
                .handle_game_event_screen_hover(game_event_target);
            context
                .platform
                .submit(PlatformEffect::GameEventScreenInput(
                    ScreenNarrationScheduleInput::MouseMove,
                ));
            context
                .input
                .update_screen_cursor_position(self.cursor_position);
            return;
        }
        if context.world.player_reporting_screen().is_some() {
            context
                .input
                .handle_player_reporting_hover(context.world, player_reporting_target);
            context
                .platform
                .submit(PlatformEffect::PlayerReportingInput(
                    ScreenNarrationScheduleInput::MouseMove,
                ));
            context
                .input
                .update_screen_cursor_position(self.cursor_position);
            return;
        }
        if context.world.social_interactions_screen().is_some() {
            context
                .input
                .refresh_social_interactions_scroll_bounds(self.gui_surface_size());
            context
                .input
                .handle_social_interactions_hover(context.world, social_interactions_target);
            context
                .platform
                .submit(PlatformEffect::SocialInteractionsInput(
                    ScreenNarrationScheduleInput::MouseMove,
                ));
            context
                .input
                .update_screen_cursor_position(self.cursor_position);
            return;
        }
        if context.world.dialog_screen_is_open() {
            context
                .input
                .update_screen_cursor_position(self.cursor_position);
            handle_dialog_cursor_moved_with_measurer(
                context.world,
                self.cursor_position,
                self.gui_surface_size(),
                &self.text_measurements,
            );
            return;
        }
        if context
            .input
            .handle_structure_block_editor_cursor_moved(self.cursor_position)
        {
            return;
        }
        if context
            .input
            .handle_jigsaw_block_editor_cursor_moved(self.cursor_position, self.gui_surface_size())
        {
            return;
        }
        if context
            .input
            .handle_test_block_editor_cursor_moved(self.cursor_position)
        {
            return;
        }
        if context
            .input
            .handle_test_instance_block_editor_cursor_moved(self.cursor_position)
        {
            return;
        }
        if context
            .input
            .sign_editor_is_active_or_pending(context.world)
        {
            context
                .input
                .handle_sign_editor_cursor_moved(self.cursor_position);
            return;
        }
        if context.world.current_book().is_some() {
            context
                .input
                .update_screen_cursor_position(self.cursor_position);
            context.input.handle_book_screen_cursor_moved(
                context.world,
                self.cursor_position,
                self.gui_surface_size(),
                &self.text_measurements,
            );
            return;
        }
        if context
            .input
            .handle_command_suggestion_cursor_moved(context.world, self.cursor_position)
        {
            return;
        }
        if context.input.handle_game_rules_screen_cursor_moved(
            context.world,
            self.cursor_position,
            self.gui_surface_size(),
        ) {
            return;
        }
        if context.input.handle_stats_screen_cursor_moved(
            context.world,
            self.cursor_position,
            self.gui_surface_size(),
        ) {
            context.platform.submit(PlatformEffect::StatsScreenInput(
                ScreenNarrationScheduleInput::MouseMove,
            ));
            return;
        }
        if context.world.advancements_screen_is_open() {
            handle_advancements_screen_cursor_moved(
                context.input,
                context.world,
                previous_cursor_position,
                self.cursor_position,
            );
        }
        handle_inventory_cursor_moved(
            context.input,
            context.world,
            context.net_counters,
            context.net_commands,
            self.cursor_position,
            self.gui_surface_size(),
        );
        if recipe_book_screen_is_open(context.world) {
            context.platform.submit(PlatformEffect::RecipeBookInput(
                ScreenNarrationScheduleInput::MouseMove,
            ));
        }
    }

    fn apply_key(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
        clipboard_text: Option<String>,
        context: &mut SimulationInputContext<'_, '_, '_>,
    ) {
        // `KeyboardHandler.keyPress` (KeyboardHandler.java:465-489) answers the
        // fullscreen and screenshot keys before it hands the event to
        // `screen.keyPressed`, so every screen dispatch below runs after this
        // one rather than instead of it. Without this ordering a screen that
        // swallows unrecognised keys -- the pause screen does -- would make F11
        // and F2 dead for as long as it is open.
        if context.input.handle_global_key_press(physical_key, state) {
            self.apply_debug_requests(context);
            return;
        }

        if context.input.disconnected_screen_runtime().is_some() {
            self.request_cursor_capture(false, context.platform);
            if let PhysicalKey::Code(code) = physical_key {
                context.input.handle_disconnected_screen_key(code, state);
            }
            collect_disconnected_screen_platform_effects(context.input, context.platform);
            if matches!(state, ElementState::Pressed) {
                context
                    .platform
                    .submit(PlatformEffect::DisconnectedScreenInput(
                        ScreenNarrationScheduleInput::KeyboardAction,
                    ));
            }
            return;
        }
        if context.input.chat_restrictions_screen_is_open() {
            self.request_cursor_capture(false, context.platform);
            handle_key_input_with_item_runtime(
                context.input,
                context.net_counters,
                context.world,
                context.net_commands,
                context.item_runtime.as_ref(),
                physical_key,
                state,
                self.gui_surface_size(),
                &self.text_measurements,
                None,
            );
            if matches!(state, ElementState::Pressed) {
                context.platform.submit(PlatformEffect::ChatScreenInput(
                    ScreenNarrationScheduleInput::KeyboardAction,
                ));
            }
            return;
        }
        if self.code_of_conduct_overlay.is_visible(context.world) {
            let overlay_input = self.code_of_conduct_overlay.handle_key_input(
                context.world,
                physical_key,
                state,
                self.gui_surface_size(),
                &self.text_measurements,
            );
            if let Some(request) = overlay_input.request {
                self.apply_code_of_conduct_control(request, context);
            }
            self.request_cursor_capture(false, context.platform);
            return;
        }

        if context.world.death_screen().is_some() {
            self.request_cursor_capture(false, context.platform);
            handle_key_input_with_item_runtime(
                context.input,
                context.net_counters,
                context.world,
                context.net_commands,
                context.item_runtime.as_ref(),
                physical_key,
                state,
                self.gui_surface_size(),
                &self.text_measurements,
                None,
            );
            collect_death_screen_platform_effects(context.input, context.platform);
            context.platform.submit(PlatformEffect::DeathScreenInput(
                ScreenNarrationScheduleInput::KeyboardAction,
            ));
            if context.world.death_screen().is_none() {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }

        if context.world.game_event_screen().is_some() {
            self.request_cursor_capture(false, context.platform);
            handle_key_input_with_item_runtime(
                context.input,
                context.net_counters,
                context.world,
                context.net_commands,
                context.item_runtime.as_ref(),
                physical_key,
                state,
                self.gui_surface_size(),
                &self.text_measurements,
                None,
            );
            collect_game_event_screen_platform_effects(context.input, context.platform);
            context
                .platform
                .submit(PlatformEffect::GameEventScreenInput(
                    ScreenNarrationScheduleInput::KeyboardAction,
                ));
            if context.world.game_event_screen().is_none() {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }

        if context.world.player_reporting_screen().is_some() {
            self.request_cursor_capture(false, context.platform);
            handle_key_input_with_item_runtime(
                context.input,
                context.net_counters,
                context.world,
                context.net_commands,
                context.item_runtime.as_ref(),
                physical_key,
                state,
                self.gui_surface_size(),
                &self.text_measurements,
                None,
            );
            collect_player_reporting_platform_effects(context.input, context.platform);
            if matches!(state, ElementState::Pressed) {
                context
                    .platform
                    .submit(PlatformEffect::PlayerReportingInput(
                        ScreenNarrationScheduleInput::KeyboardAction,
                    ));
            }
            if !simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }

        if context.world.social_interactions_screen().is_some() {
            self.request_cursor_capture(false, context.platform);
            context
                .input
                .refresh_social_interactions_scroll_bounds(self.gui_surface_size());
            handle_key_input_with_item_runtime(
                context.input,
                context.net_counters,
                context.world,
                context.net_commands,
                context.item_runtime.as_ref(),
                physical_key,
                state,
                self.gui_surface_size(),
                &self.text_measurements,
                None,
            );
            collect_social_interactions_platform_effects(context.input, context.platform);
            if matches!(state, ElementState::Pressed) {
                context
                    .platform
                    .submit(PlatformEffect::SocialInteractionsInput(
                        ScreenNarrationScheduleInput::KeyboardAction,
                    ));
            }
            if !simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }

        let pause_was_open = context.input.debug_pause_screen_is_open();
        if context
            .input
            .handle_debug_pause_screen_key(context.world, physical_key, state)
        {
            if simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(false, context.platform);
            } else if pause_was_open {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }

        let queued_effects = RefCell::new(Vec::new());
        let mut clipboard = SimulationClipboard {
            read: clipboard_text,
            effects: &queued_effects,
        };
        let debug_options_search_before = context
            .input
            .debug_options_screen_search_text()
            .map(str::to_owned);
        if context
            .input
            .handle_debug_options_screen_key_with_clipboard_and_surface(
                physical_key,
                state,
                Some(&mut clipboard),
                self.gui_surface_size(),
            )
        {
            flush_platform_effects(&queued_effects, context.platform);
            if matches!(state, ElementState::Pressed) {
                let search_changed = context.input.debug_options_screen_search_text()
                    != debug_options_search_before.as_deref();
                context
                    .platform
                    .submit(PlatformEffect::DebugOptionsScreenInput(if search_changed {
                        ScreenNarrationScheduleInput::Immediate
                    } else {
                        ScreenNarrationScheduleInput::KeyboardAction
                    }));
            }
            if simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(false, context.platform);
            }
            return;
        }

        let mut terrain_reload = SimulationTerrainReload {
            effects: &queued_effects,
        };
        if context
            .input
            .handle_debug_overlay_key_with_clipboard_and_net(
                physical_key,
                state,
                Some(context.world),
                Some(&mut terrain_reload),
                Some(&mut clipboard),
                context.item_runtime.as_ref(),
                context.net_counters,
                context.net_commands,
            )
        {
            flush_platform_effects(&queued_effects, context.platform);
            self.apply_debug_requests(context);
            return;
        }
        flush_platform_effects(&queued_effects, context.platform);

        let chat_screen_was_open = context.input.chat_entry_is_active();
        let container_open = context.world.open_container_id().is_some();
        let recipe_book_open = recipe_book_screen_is_open(context.world);
        let dialog_open = context.world.dialog_screen_is_open();
        let sign_editor_open = context
            .input
            .sign_editor_is_active_or_pending(context.world);
        let book_open = context.world.current_book().is_some();
        let advancements_open = context.world.advancements_screen_is_open();
        let stats_open = context.world.stats_screen_is_open();
        let social_interactions_open = context.world.social_interactions_screen().is_some();
        let game_rules_open = context.world.game_rules_screen().is_some();
        let debug_options_open = context.input.debug_options_screen_is_open();
        let pause_open = context.input.debug_pause_screen_is_open();
        let command_entry_open = context.input.command_entry_is_active();
        let structure_block_editor_open = context.input.structure_block_editor_is_active();
        let jigsaw_block_editor_open = context.input.jigsaw_block_editor_is_active();
        let test_block_editor_open = context.input.test_block_editor_is_active();
        let test_instance_block_editor_open = context.input.test_instance_block_editor_is_active();
        if dialog_open {
            context
                .input
                .update_screen_modifier_keys(physical_key, state);
            self.request_cursor_capture(false, context.platform);
            handle_dialog_key_input_with_modifiers_for_surface_and_measurer(
                context.net_counters,
                context.world,
                context.net_commands,
                Some(&mut clipboard),
                physical_key,
                state,
                context.input.shift_down(),
                context.input.control_down(),
                self.gui_surface_size(),
                &self.text_measurements,
            );
            collect_dialog_open_uri_effects(context.world, &queued_effects);
            flush_platform_effects(&queued_effects, context.platform);
            if !simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }

        let ordinary_pause_escape = matches!(state, ElementState::Pressed)
            && matches!(physical_key, PhysicalKey::Code(KeyCode::Escape))
            && !context.input.command_entry_is_active()
            && !sign_editor_open
            && !book_open
            && !advancements_open
            && !stats_open
            && !social_interactions_open
            && !game_rules_open
            && !debug_options_open
            && !pause_open
            && !world_wants_cursor(context.world);
        if ordinary_pause_escape {
            self.request_cursor_capture(false, context.platform);
            release_active_input(
                context.input,
                context.world,
                context.net_counters,
                context.net_commands,
            );
        }
        if sign_editor_open
            || book_open
            || advancements_open
            || stats_open
            || social_interactions_open
            || game_rules_open
            || debug_options_open
            || pause_open
            || command_entry_open
            || structure_block_editor_open
            || jigsaw_block_editor_open
            || test_block_editor_open
            || test_instance_block_editor_open
        {
            self.request_cursor_capture(false, context.platform);
        }
        if stats_open && matches!(state, ElementState::Pressed) {
            context.platform.submit(PlatformEffect::StatsScreenInput(
                ScreenNarrationScheduleInput::KeyboardAction,
            ));
        }
        if recipe_book_open && matches!(state, ElementState::Pressed) {
            context.platform.submit(PlatformEffect::RecipeBookInput(
                ScreenNarrationScheduleInput::KeyboardAction,
            ));
        }
        if !self.cursor_captured
            && !container_open
            && !sign_editor_open
            && !book_open
            && !advancements_open
            && !stats_open
            && !social_interactions_open
            && !game_rules_open
            && !debug_options_open
            && !pause_open
            && !command_entry_open
            && !structure_block_editor_open
            && !jigsaw_block_editor_open
            && !test_block_editor_open
            && !test_instance_block_editor_open
            && !ordinary_pause_escape
        {
            return;
        }
        handle_key_input_with_item_runtime(
            context.input,
            context.net_counters,
            context.world,
            context.net_commands,
            context.item_runtime.as_ref(),
            physical_key,
            state,
            self.gui_surface_size(),
            &self.text_measurements,
            Some(&mut clipboard),
        );
        if matches!(state, ElementState::Pressed)
            && (chat_screen_was_open || context.input.chat_entry_is_active())
        {
            context.platform.submit(PlatformEffect::ChatScreenInput(
                ScreenNarrationScheduleInput::KeyboardAction,
            ));
        }
        flush_platform_effects(&queued_effects, context.platform);
        if !social_interactions_open
            && context.world.social_interactions_screen().is_some()
            && matches!(state, ElementState::Pressed)
        {
            self.request_cursor_capture(false, context.platform);
            context
                .platform
                .submit(PlatformEffect::SocialInteractionsInput(
                    ScreenNarrationScheduleInput::KeyboardAction,
                ));
        }
        if (command_entry_open
            || structure_block_editor_open
            || jigsaw_block_editor_open
            || test_block_editor_open
            || test_instance_block_editor_open)
            && !context.input.command_entry_is_active()
            && !context.input.structure_block_editor_is_active()
            && !context.input.jigsaw_block_editor_is_active()
            && !context.input.test_block_editor_is_active()
            && !context.input.test_instance_block_editor_is_active()
            && !simulation_wants_cursor(context.input, context.world)
        {
            self.request_cursor_capture(true, context.platform);
        }
    }

    fn apply_debug_requests(&mut self, context: &mut SimulationInputContext<'_, '_, '_>) {
        if simulation_wants_cursor(context.input, context.world) {
            self.request_cursor_capture(false, context.platform);
        }
        let reload_requests = context.input.take_debug_resource_pack_reload_requests();
        if reload_requests > 0 {
            context.server_resource_packs.request_reload();
            tracing::info!(
                reload_requests,
                "resource pack reload requested by debug hotkey"
            );
        }
        let dynamic_texture_dump_requests =
            context.input.take_debug_dynamic_texture_dump_requests();
        if dynamic_texture_dump_requests > 0 {
            context
                .platform
                .submit(PlatformEffect::DumpDynamicTextures {
                    requests: dynamic_texture_dump_requests,
                });
        }
        let client_option_writes = context.input.take_debug_client_option_writes();
        if !client_option_writes.is_empty() {
            context.platform.submit(PlatformEffect::SaveClientOptions {
                writes: client_option_writes,
            });
            crate::input::queue_broadcast_client_information_command(
                context.net_counters,
                context.net_commands,
                &self.client_information,
            );
        }
        for _ in 0..context.input.take_screenshot_requests() {
            context.platform.submit(PlatformEffect::GrabScreenshot);
        }
        for fullscreen in context.input.take_fullscreen_requests() {
            context
                .platform
                .submit(PlatformEffect::SetFullscreen(fullscreen));
        }
        let narrator_status_requests = context.input.take_narrator_status_requests();
        if !narrator_status_requests.is_empty() {
            for request in &narrator_status_requests {
                // `Options.narrator`'s update callback runs
                // `GameNarrator.updateNarratorStatus`, which pairs the spoken
                // announcement with the `NARRATOR_TOGGLE` system toast.
                context.world.push_narrator_toggle_toast(
                    request.status != crate::narrator_runtime::NarratorStatus::Off,
                    request.status.option_name(),
                );
            }
            context
                .platform
                .submit(PlatformEffect::NarratorStatusChanged {
                    requests: narrator_status_requests,
                });
        }
        let profiling_toggle_requests = context.input.take_debug_profiling_toggle_requests();
        if !profiling_toggle_requests.is_empty() {
            context.platform.submit(PlatformEffect::ProfilingToggled {
                commands: debug_profiling_commands(profiling_toggle_requests),
            });
        }
        let profiler_navigation = context
            .input
            .take_debug_profiler_chart_navigation_requests();
        if !profiler_navigation.is_empty() {
            context.platform.submit(PlatformEffect::NavigateProfiler {
                digits: profiler_navigation,
            });
        }
        let frustum_requests = context.input.take_debug_frustum_requests();
        if !frustum_requests.is_empty() {
            let captures = frustum_requests
                .iter()
                .filter(|request| matches!(request, DebugFrustumRequest::Capture))
                .count();
            let kills = frustum_requests
                .iter()
                .filter(|request| matches!(request, DebugFrustumRequest::Kill))
                .count();
            tracing::info!(
                requests = frustum_requests.len(),
                captures,
                kills,
                "debug frustum hotkey requested; native captured frustum is not implemented"
            );
        }
        let feature_count_requests = context.input.take_debug_feature_count_requests();
        if !feature_count_requests.is_empty() {
            let logs = feature_count_requests
                .iter()
                .filter(|request| matches!(request, DebugFeatureCountRequest::Log))
                .count();
            let clears = feature_count_requests
                .iter()
                .filter(|request| matches!(request, DebugFeatureCountRequest::Clear))
                .count();
            tracing::info!(
                requests = feature_count_requests.len(),
                logs,
                clears,
                "feature-count debug hotkey requested; native FeatureCountTracker is not implemented"
            );
        }
        let debug_options_requests = context.input.take_debug_options_screen_requests();
        for _ in 0..debug_options_requests {
            if context.input.debug_options_screen_is_open() {
                context.input.close_debug_options_screen();
            } else {
                release_active_input(
                    context.input,
                    context.world,
                    context.net_counters,
                    context.net_commands,
                );
                context.input.open_debug_options_screen();
            }
        }
        if debug_options_requests > 0 {
            self.request_cursor_capture(false, context.platform);
            tracing::info!(
                debug_options_requests,
                open = context.input.debug_options_screen_is_open(),
                "debug options screen toggled by debug hotkey"
            );
        }
        let recreate_requests = context
            .input
            .take_debug_recreate_server_query_requests_for_level(
                context.world.client_level_generation(),
            );
        let recreate_request_count = recreate_requests.len();
        for request in recreate_requests {
            queue_debug_recreate_server_query_request(
                context.net_counters,
                context.net_commands,
                request,
            );
        }
        if recreate_request_count > 0 {
            tracing::info!(
                recreate_request_count,
                "debug recreate server query requested by F3+I"
            );
        }
        // Vanilla `pauseGame(debugModifierDown)` only suppresses the menu when
        // an unpublished singleplayer server can really pause; an external
        // multiplayer client always opens the full pause menu, so F3+Esc and
        // Esc share one with-menu path.
        let pause_screen_requests = context.input.take_debug_pause_screen_requests();
        if pause_screen_requests > 0 {
            release_active_input(
                context.input,
                context.world,
                context.net_counters,
                context.net_commands,
            );
            context
                .input
                .open_debug_pause_screen_with_menu(context.world);
            self.request_cursor_capture(false, context.platform);
            tracing::info!(
                pause_screen_requests,
                "opened pause screen requested by debug hotkey"
            );
        }
    }

    /// The element that currently owns pre-edit rendering, in the same
    /// priority order every pre-edit consumer uses.
    fn ime_preedit_target(
        &self,
        context: &mut SimulationInputContext<'_, '_, '_>,
    ) -> ImePreeditTarget {
        if self.code_of_conduct_overlay.is_visible(context.world) {
            ImePreeditTarget::None
        } else if context.world.dialog_screen_is_open() {
            ImePreeditTarget::Dialog
        } else if context
            .input
            .sign_editor_is_active_or_pending(context.world)
        {
            ImePreeditTarget::SignEditor
        } else {
            ImePreeditTarget::None
        }
    }

    fn apply_ime_preedit(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
        context: &mut SimulationInputContext<'_, '_, '_>,
    ) {
        // Vanilla `KeyboardHandler.preeditCallback` stores the event before
        // routing, whether or not any element renders it right now.
        self.last_ime_preedit = Some((text.to_string(), cursor_range));
        let target = self.ime_preedit_target(context);
        self.ime_preedit_target = target;
        match target {
            ImePreeditTarget::None => {
                if self.code_of_conduct_overlay.is_visible(context.world) {
                    context.input.clear_dialog_ime_preedit();
                    self.request_cursor_capture(false, context.platform);
                } else {
                    context.input.clear_dialog_ime_preedit();
                    context.input.clear_sign_editor_ime_preedit();
                }
            }
            ImePreeditTarget::Dialog => {
                self.request_cursor_capture(false, context.platform);
                context.input.handle_dialog_ime_preedit(
                    text,
                    cursor_range,
                    context.world.dialog_screen_revision(),
                );
            }
            ImePreeditTarget::SignEditor => {
                context.input.sync_sign_editor(
                    context.world,
                    context.net_counters,
                    context.net_commands,
                );
                context
                    .input
                    .handle_sign_editor_ime_preedit(text, cursor_range);
                self.request_cursor_capture(false, context.platform);
            }
        }
    }

    /// Vanilla `Minecraft.onTextInputFocusChange`: when the element owning
    /// pre-edit rendering changes, the losing one is cleared
    /// (`submitPreeditEvent(element, null)`) and the stored composition is
    /// resubmitted to the gaining one (`resubmitLastPreeditEvent`).
    /// `Minecraft.onTextInputFocusChange` plus `TextInputManager.tick`
    /// (TextInputManager.java:26-69): the window is told to accept composed
    /// text exactly while a text field has focus, and told to stop otherwise.
    fn sync_text_input_enabled(&mut self, context: &mut SimulationInputContext<'_, '_, '_>) {
        let enabled = context.input.text_input_is_active(context.world);
        if enabled == self.text_input_enabled {
            return;
        }
        self.text_input_enabled = enabled;
        context
            .platform
            .submit(PlatformEffect::SetTextInputEnabled(enabled));
    }

    fn sync_ime_preedit_focus(&mut self, context: &mut SimulationInputContext<'_, '_, '_>) {
        let target = self.ime_preedit_target(context);
        if target == self.ime_preedit_target {
            return;
        }
        match self.ime_preedit_target {
            ImePreeditTarget::Dialog => context.input.clear_dialog_ime_preedit(),
            ImePreeditTarget::SignEditor => context.input.clear_sign_editor_ime_preedit(),
            ImePreeditTarget::None => {}
        }
        if let Some((text, cursor_range)) = self.last_ime_preedit.clone() {
            match target {
                ImePreeditTarget::Dialog => {
                    context.input.handle_dialog_ime_preedit(
                        &text,
                        cursor_range,
                        context.world.dialog_screen_revision(),
                    );
                }
                ImePreeditTarget::SignEditor => {
                    context.input.sync_sign_editor(
                        context.world,
                        context.net_counters,
                        context.net_commands,
                    );
                    context
                        .input
                        .handle_sign_editor_ime_preedit(&text, cursor_range);
                }
                ImePreeditTarget::None => {}
            }
        }
        self.ime_preedit_target = target;
    }

    fn apply_ime_commit(&mut self, text: &str, context: &mut SimulationInputContext<'_, '_, '_>) {
        let chat_screen_was_open = context.input.chat_entry_is_active();
        context.input.clear_dialog_ime_preedit();
        context.input.clear_sign_editor_ime_preedit();
        if self.code_of_conduct_overlay.is_visible(context.world) {
            self.request_cursor_capture(false, context.platform);
            return;
        }
        let sign_editor_open = context
            .input
            .sign_editor_is_active_or_pending(context.world);
        let container_open = context.world.open_container_id().is_some();
        let book_open = context.world.current_book().is_some();
        let advancements_open = context.world.advancements_screen_is_open();
        let stats_open = context.world.stats_screen_is_open();
        let player_reporting_open = context.world.player_reporting_screen().is_some();
        let social_interactions_open = context.world.social_interactions_screen().is_some();
        let game_rules_open = context.world.game_rules_screen().is_some();
        let command_entry_open = context.input.command_entry_is_active();
        let structure_block_editor_open = context.input.structure_block_editor_is_active();
        let jigsaw_block_editor_open = context.input.jigsaw_block_editor_is_active();
        let test_block_editor_open = context.input.test_block_editor_is_active();
        let test_instance_block_editor_open = context.input.test_instance_block_editor_is_active();
        if context.input.debug_options_screen_is_open() {
            self.request_cursor_capture(false, context.platform);
            let search_before = context
                .input
                .debug_options_screen_search_text()
                .map(str::to_owned);
            context.input.handle_debug_options_screen_text_input(text);
            if context.input.debug_options_screen_search_text() != search_before.as_deref() {
                context
                    .platform
                    .submit(PlatformEffect::DebugOptionsScreenInput(
                        ScreenNarrationScheduleInput::Immediate,
                    ));
            }
            return;
        }
        if context.world.dialog_screen_is_open() {
            self.request_cursor_capture(false, context.platform);
            handle_dialog_text_input_with_measurer(context.world, text, &self.text_measurements);
            return;
        }
        if player_reporting_open {
            self.request_cursor_capture(false, context.platform);
            context
                .input
                .handle_player_reporting_text(context.world, text);
            context
                .platform
                .submit(PlatformEffect::PlayerReportingInput(
                    ScreenNarrationScheduleInput::KeyboardAction,
                ));
            return;
        }
        if social_interactions_open {
            self.request_cursor_capture(false, context.platform);
            context
                .input
                .handle_social_interactions_text(context.world, text);
            context
                .platform
                .submit(PlatformEffect::SocialInteractionsInput(
                    ScreenNarrationScheduleInput::KeyboardAction,
                ));
            return;
        }
        if advancements_open || stats_open {
            self.request_cursor_capture(false, context.platform);
            return;
        }
        if sign_editor_open
            || container_open
            || book_open
            || game_rules_open
            || command_entry_open
            || structure_block_editor_open
            || jigsaw_block_editor_open
            || test_block_editor_open
            || test_instance_block_editor_open
        {
            self.request_cursor_capture(false, context.platform);
        }
        if !self.cursor_captured
            && !sign_editor_open
            && !container_open
            && !book_open
            && !game_rules_open
            && !command_entry_open
            && !structure_block_editor_open
            && !jigsaw_block_editor_open
            && !test_block_editor_open
            && !test_instance_block_editor_open
        {
            return;
        }
        handle_text_input_with_item_runtime(
            context.input,
            context.net_counters,
            context.world,
            context.net_commands,
            context.item_runtime.as_ref(),
            text,
            &self.text_measurements,
        );
        if chat_screen_was_open || context.input.chat_entry_is_active() {
            context.platform.submit(PlatformEffect::ChatScreenInput(
                ScreenNarrationScheduleInput::KeyboardAction,
            ));
        }
        if recipe_book_screen_is_open(context.world) {
            context.platform.submit(PlatformEffect::RecipeBookInput(
                ScreenNarrationScheduleInput::KeyboardAction,
            ));
        }
    }

    fn apply_mouse_button(
        &mut self,
        button: MouseButton,
        state: ElementState,
        position: Option<PhysicalPosition<f64>>,
        chat_target: Option<HudChatHitTarget>,
        book_target: Option<HudBookHitTarget>,
        disconnected_screen_target: Option<HudDisconnectedScreenHitTarget>,
        death_screen_target: Option<HudDeathScreenHitTarget>,
        game_event_target: Option<HudGameEventHitTarget>,
        player_reporting_target: Option<HudPlayerReportingHitTarget>,
        social_interactions_target: Option<HudSocialInteractionsHitTarget>,
        pause_draft_target: Option<HudPauseDraftButton>,
        context: &mut SimulationInputContext<'_, '_, '_>,
    ) {
        // A click re-projects the raw physical cache with the current GUI
        // projection (`MouseHandler.onButton` -> `getScaledXPos`); the screen
        // handlers below read the runtime's logical cursor position, so
        // publish the click projection before they run rather than letting a
        // stale pre-resize logical cache reach them.
        if let Some(position) = position {
            self.cursor_position = Some(position);
        }
        context.input.note_mouse_button_input_type(context.world);
        if context.input.disconnected_screen_runtime().is_some() {
            self.request_cursor_capture(false, context.platform);
            context.input.handle_disconnected_screen_mouse_input(
                button,
                state,
                disconnected_screen_target,
            );
            collect_disconnected_screen_platform_effects(context.input, context.platform);
            context
                .platform
                .submit(PlatformEffect::DisconnectedScreenInput(
                    ScreenNarrationScheduleInput::MouseAction,
                ));
            return;
        }
        if context.input.chat_restrictions_screen_is_open() {
            self.request_cursor_capture(false, context.platform);
            if matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
                self.apply_chat_hit_target(chat_target, context);
            }
            context.platform.submit(PlatformEffect::ChatScreenInput(
                ScreenNarrationScheduleInput::MouseAction,
            ));
            return;
        }
        if self.code_of_conduct_overlay.is_visible(context.world) {
            let overlay_input = self.code_of_conduct_overlay.handle_mouse_input(
                context.world,
                button,
                state,
                self.cursor_position,
                self.gui_surface_size(),
                &self.text_measurements,
            );
            if let Some(request) = overlay_input.request {
                self.apply_code_of_conduct_control(request, context);
            }
            self.request_cursor_capture(false, context.platform);
            return;
        }
        if context.input.debug_game_mode_switcher_is_open() {
            self.request_cursor_capture(false, context.platform);
            return;
        }
        if context.input.debug_options_screen_is_open() {
            self.request_cursor_capture(false, context.platform);
            context
                .input
                .handle_debug_options_screen_mouse_input_with_text_measurer(
                    button,
                    state,
                    self.cursor_position,
                    self.gui_surface_size(),
                    context.input.show_only_reduced_info(context.world),
                    &self.text_measurements,
                );
            context
                .platform
                .submit(PlatformEffect::DebugOptionsScreenInput(
                    ScreenNarrationScheduleInput::MouseAction,
                ));
            return;
        }
        if context.input.debug_pause_screen_is_open() {
            self.request_cursor_capture(false, context.platform);
            context.input.handle_debug_pause_screen_mouse_input(
                context.net_counters,
                context.world,
                context.net_commands,
                button,
                state,
                self.cursor_position,
                self.gui_surface_size(),
                pause_draft_target,
            );
            let queued_effects = RefCell::new(Vec::new());
            for request in context.input.take_pause_screen_link_requests() {
                let url = match request {
                    PauseScreenLinkRequest::SendFeedback { url }
                    | PauseScreenLinkRequest::ReportBugs { url } => url,
                };
                context.world.open_chat_url(url.to_string());
            }
            collect_dialog_open_uri_effects(context.world, &queued_effects);
            flush_platform_effects(&queued_effects, context.platform);
            if context.world.dialog_screen_is_open() {
                context.input.sync_dialog_native_previous_screen(
                    context.world,
                    context.net_counters,
                    context.net_commands,
                );
            }
            if context.input.pause_screen_disconnect_requested() {
                request_net_disconnect(context.net_commands, &mut self.net_disconnect_requested);
            }
            if context.world.social_interactions_screen().is_some() {
                context
                    .platform
                    .submit(PlatformEffect::SocialInteractionsInput(
                        ScreenNarrationScheduleInput::MouseAction,
                    ));
            }
            if !simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }
        if context.world.death_screen().is_some() {
            self.request_cursor_capture(false, context.platform);
            context.input.handle_death_screen_mouse_input(
                context.net_counters,
                context.world,
                context.net_commands,
                button,
                state,
                death_screen_target,
            );
            collect_death_screen_platform_effects(context.input, context.platform);
            context.platform.submit(PlatformEffect::DeathScreenInput(
                ScreenNarrationScheduleInput::MouseAction,
            ));
            if context.world.death_screen().is_none() {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }
        if context.world.game_event_screen().is_some() {
            self.request_cursor_capture(false, context.platform);
            context.input.handle_game_event_screen_mouse_input(
                context.world,
                button,
                state,
                game_event_target,
            );
            collect_game_event_screen_platform_effects(context.input, context.platform);
            context
                .platform
                .submit(PlatformEffect::GameEventScreenInput(
                    ScreenNarrationScheduleInput::MouseAction,
                ));
            if context.world.game_event_screen().is_none() {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }
        if context.world.player_reporting_screen().is_some() {
            self.request_cursor_capture(false, context.platform);
            context.input.handle_player_reporting_mouse_input(
                context.world,
                button,
                state,
                player_reporting_target,
            );
            collect_player_reporting_platform_effects(context.input, context.platform);
            context
                .platform
                .submit(PlatformEffect::PlayerReportingInput(
                    ScreenNarrationScheduleInput::MouseAction,
                ));
            if !simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }
        if context.world.social_interactions_screen().is_some() {
            self.request_cursor_capture(false, context.platform);
            context
                .input
                .refresh_social_interactions_scroll_bounds(self.gui_surface_size());
            context.input.handle_social_interactions_mouse_input(
                context.world,
                button,
                state,
                social_interactions_target,
            );
            collect_social_interactions_platform_effects(context.input, context.platform);
            context
                .platform
                .submit(PlatformEffect::SocialInteractionsInput(
                    ScreenNarrationScheduleInput::MouseAction,
                ));
            if !simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }
        if context.world.game_rules_screen().is_some() {
            self.request_cursor_capture(false, context.platform);
            context.input.handle_game_rules_screen_mouse_input(
                context.net_counters,
                context.world,
                context.net_commands,
                button,
                state,
                self.cursor_position,
                self.gui_surface_size(),
            );
            if !simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }
        if context.world.stats_screen_is_open() {
            self.request_cursor_capture(false, context.platform);
            context.platform.submit(PlatformEffect::StatsScreenInput(
                ScreenNarrationScheduleInput::MouseAction,
            ));
            context.input.handle_stats_screen_mouse_input(
                context.net_counters,
                context.world,
                context.net_commands,
                button,
                state,
                self.cursor_position,
                self.gui_surface_size(),
            );
            if !simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }
        if context.world.dialog_screen_is_open() {
            self.request_cursor_capture(false, context.platform);
            let double_click = context.input.dialog_mouse_click_is_double(
                button,
                state,
                context.world.dialog_screen_revision(),
            );
            let queued_effects = RefCell::new(Vec::new());
            let mut clipboard = SimulationClipboard {
                read: None,
                effects: &queued_effects,
            };
            handle_dialog_mouse_input_with_modifiers_and_measurer(
                context.net_counters,
                context.world,
                context.net_commands,
                Some(&mut clipboard),
                button,
                state,
                self.cursor_position,
                self.gui_surface_size(),
                context.input.shift_down(),
                double_click,
                &self.text_measurements,
            );
            collect_dialog_open_uri_effects(context.world, &queued_effects);
            flush_platform_effects(&queued_effects, context.platform);
            if !simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }
        if context.input.structure_block_editor_is_active() {
            self.request_cursor_capture(false, context.platform);
            context.input.handle_structure_block_editor_mouse_input(
                context.net_counters,
                context.world,
                context.net_commands,
                button,
                state,
                self.cursor_position,
                self.gui_surface_size(),
                &self.text_measurements,
            );
            if !simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }
        if context.input.jigsaw_block_editor_is_active() {
            self.request_cursor_capture(false, context.platform);
            context.input.handle_jigsaw_block_editor_mouse_input(
                context.net_counters,
                context.world,
                context.net_commands,
                button,
                state,
                self.cursor_position,
                self.gui_surface_size(),
                &self.text_measurements,
            );
            if !simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }
        if context.input.test_block_editor_is_active() {
            self.request_cursor_capture(false, context.platform);
            context.input.handle_test_block_editor_mouse_input(
                context.net_counters,
                context.world,
                context.net_commands,
                button,
                state,
                self.cursor_position,
                self.gui_surface_size(),
                &self.text_measurements,
            );
            if !simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }
        if context.input.test_instance_block_editor_is_active() {
            self.request_cursor_capture(false, context.platform);
            context.input.handle_test_instance_block_editor_mouse_input(
                context.net_counters,
                context.world,
                context.net_commands,
                button,
                state,
                self.cursor_position,
                self.gui_surface_size(),
                &self.text_measurements,
            );
            if !simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }
        if context.input.command_entry_is_active() {
            self.request_cursor_capture(false, context.platform);
            context.platform.submit(PlatformEffect::ChatScreenInput(
                ScreenNarrationScheduleInput::MouseAction,
            ));
            if context.input.handle_command_suggestion_mouse_input(
                context.world,
                button,
                state,
                self.cursor_position,
            ) {
                return;
            }
            if context
                .input
                .handle_minecart_command_block_editor_mouse_input(
                    context.world,
                    context.net_counters,
                    context.net_commands,
                    button,
                    state,
                    self.cursor_position,
                    self.gui_surface_size(),
                    &self.text_measurements,
                )
            {
                return;
            }
            if context.input.handle_command_block_editor_mouse_input(
                context.world,
                context.net_counters,
                context.net_commands,
                button,
                state,
                self.cursor_position,
                self.gui_surface_size(),
                &self.text_measurements,
            ) {
                return;
            }
            if matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
                self.apply_chat_hit_target(chat_target, context);
            }
            return;
        }
        if context
            .input
            .sign_editor_is_active_or_pending(context.world)
        {
            self.request_cursor_capture(false, context.platform);
            context.input.handle_sign_editor_mouse_input(
                context.net_counters,
                context.world,
                context.net_commands,
                button,
                state,
                self.cursor_position,
                self.gui_surface_size(),
            );
            if !simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }
        if context.world.current_book().is_some() {
            self.request_cursor_capture(false, context.platform);
            if matches!((button, state), (MouseButton::Left, ElementState::Pressed))
                && self.apply_book_hit_target(book_target, context)
            {
                return;
            }
            context.input.handle_book_screen_mouse_input(
                context.net_counters,
                context.world,
                context.net_commands,
                button,
                state,
                self.cursor_position,
                self.gui_surface_size(),
                &self.text_measurements,
            );
            if !simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }
        if context.world.lectern_book_view().is_some()
            && matches!((button, state), (MouseButton::Left, ElementState::Pressed))
            && self.apply_book_hit_target(book_target, context)
        {
            return;
        }
        if context.world.advancements_screen_is_open() {
            self.request_cursor_capture(false, context.platform);
            handle_advancements_screen_mouse_input(
                context.input,
                context.net_counters,
                context.world,
                context.net_commands,
                button,
                state,
                self.cursor_position,
                self.gui_surface_size(),
            );
            return;
        }
        if context.world.open_container_id().is_some() {
            self.request_cursor_capture(false, context.platform);
            if recipe_book_screen_is_open(context.world) {
                context.platform.submit(PlatformEffect::RecipeBookInput(
                    ScreenNarrationScheduleInput::MouseAction,
                ));
            }
            handle_inventory_mouse_input_with_item_runtime(
                context.input,
                context.world,
                context.net_counters,
                context.net_commands,
                context.item_runtime.as_ref(),
                button,
                state,
                self.cursor_position,
                self.gui_surface_size(),
            );
            flush_local_interaction_sound_actions(
                context.world,
                context.level_random,
                context.audio_events,
            );
            return;
        }
        if matches!(state, ElementState::Pressed) && !self.cursor_captured {
            if simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(false, context.platform);
            } else {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }
        if !self.cursor_captured {
            return;
        }
        handle_mouse_input_at_partial_tick_with_effects_and_presentation(
            context.input,
            context.first_person_hand_presentation,
            context.world,
            context.net_counters,
            context.net_commands,
            button,
            state,
            context.entity_partial_tick,
            context.level_random,
            context.audio_events,
        );
    }

    fn apply_chat_hit_target(
        &mut self,
        target: Option<HudChatHitTarget>,
        context: &mut SimulationInputContext<'_, '_, '_>,
    ) -> bool {
        let Some(target) = target else {
            return false;
        };
        if !chat_hit_target_is_current(
            context.input,
            context.world,
            context.item_runtime.as_ref(),
            target,
        ) {
            return false;
        }
        match target.kind {
            HudChatHitTargetKind::Queue => {
                if context.world.client_chat().delayed_messages.is_empty() {
                    return false;
                }
                if let Some(acknowledgement) = context
                    .world
                    .accept_next_delayed_chat_message(crate::runtime::monotonic_millis())
                {
                    crate::runtime::queue_chat_acknowledgement(
                        context.net_commands,
                        context.net_counters,
                        acknowledgement,
                    );
                }
                true
            }
            HudChatHitTargetKind::Restrictions => {
                if context.world.chat_presentation_config().visibility
                    == bbb_world::ChatVisibilityState::Full
                {
                    return false;
                }
                context.input.open_chat_restrictions_screen()
            }
            HudChatHitTargetKind::RestrictionsDone => {
                context.input.close_chat_restrictions_screen()
            }
            HudChatHitTargetKind::Message {
                display_sequence,
                target_index,
            } => {
                let Some(interaction) = crate::runtime::hud_chat_component_interaction(
                    context.world,
                    context.item_runtime.as_ref(),
                    display_sequence,
                    target_index,
                ) else {
                    return false;
                };
                apply_chat_component_interaction(
                    interaction,
                    context.input.shift_down(),
                    context.input,
                    context.world,
                    context.net_counters,
                    context.net_commands,
                    context.platform,
                )
            }
        }
    }

    fn apply_book_hit_target(
        &mut self,
        target: Option<HudBookHitTarget>,
        context: &mut SimulationInputContext<'_, '_, '_>,
    ) -> bool {
        let Some(target) = target else {
            return false;
        };
        let Some(event) = crate::runtime::hud_book_component_click_event(
            context.world,
            context.input,
            context.item_runtime.as_ref(),
            target,
        ) else {
            return false;
        };
        apply_book_component_click_event(
            target.mode,
            event,
            context.input,
            context.world,
            context.net_counters,
            context.net_commands,
            context.platform,
        )
    }

    fn apply_mouse_wheel(
        &mut self,
        delta: MouseScrollDelta,
        context: &mut SimulationInputContext<'_, '_, '_>,
    ) {
        let delta = context.input.scale_scroll_delta(delta);
        if self.code_of_conduct_overlay.is_visible(context.world) {
            self.code_of_conduct_overlay.handle_mouse_wheel(
                context.world,
                delta,
                self.cursor_position,
                self.gui_surface_size(),
                &self.text_measurements,
            );
            self.request_cursor_capture(false, context.platform);
            return;
        }
        if context.input.debug_options_screen_is_open() {
            self.request_cursor_capture(false, context.platform);
            context
                .input
                .handle_debug_options_screen_mouse_wheel(delta, self.gui_surface_size());
            context
                .platform
                .submit(PlatformEffect::DebugOptionsScreenInput(
                    ScreenNarrationScheduleInput::MouseAction,
                ));
            return;
        }
        if context.world.death_screen().is_some() || context.world.game_event_screen().is_some() {
            self.request_cursor_capture(false, context.platform);
            return;
        }
        if context.world.player_reporting_screen().is_some() {
            self.request_cursor_capture(false, context.platform);
            context
                .input
                .handle_player_reporting_mouse_wheel(context.world, delta);
            context
                .platform
                .submit(PlatformEffect::PlayerReportingInput(
                    ScreenNarrationScheduleInput::MouseAction,
                ));
            return;
        }
        if context.world.social_interactions_screen().is_some() {
            self.request_cursor_capture(false, context.platform);
            context
                .input
                .refresh_social_interactions_scroll_bounds(self.gui_surface_size());
            context
                .input
                .handle_social_interactions_mouse_wheel(context.world, delta);
            context
                .platform
                .submit(PlatformEffect::SocialInteractionsInput(
                    ScreenNarrationScheduleInput::MouseAction,
                ));
            return;
        }
        if context.world.dialog_screen_is_open() {
            self.request_cursor_capture(false, context.platform);
            handle_dialog_mouse_wheel(
                context.world,
                delta,
                self.cursor_position,
                self.gui_surface_size(),
            );
            return;
        }
        if context.input.command_entry_is_active() {
            self.request_cursor_capture(false, context.platform);
            if context.input.handle_command_suggestion_mouse_wheel(delta) {
                return;
            }
            if context.input.command_block_editor_is_active()
                || context.input.minecart_command_block_editor_is_active()
            {
                return;
            }
            let scroll = match delta {
                MouseScrollDelta::LineDelta(_, y) => f64::from(y),
                MouseScrollDelta::PixelDelta(position) => position.y,
            }
            .clamp(-1.0, 1.0);
            let scroll = if context.input.shift_down() {
                scroll
            } else {
                scroll * 7.0
            } as i32;
            context.input.scroll_chat_lines(scroll);
            return;
        }
        if context
            .input
            .sign_editor_is_active_or_pending(context.world)
        {
            self.request_cursor_capture(false, context.platform);
            return;
        }
        if context.world.game_rules_screen().is_some() {
            self.request_cursor_capture(false, context.platform);
            context
                .input
                .handle_game_rules_screen_mouse_wheel(context.world, delta);
            return;
        }
        if context.world.stats_screen_is_open() {
            self.request_cursor_capture(false, context.platform);
            context.platform.submit(PlatformEffect::StatsScreenInput(
                ScreenNarrationScheduleInput::MouseAction,
            ));
            context
                .input
                .handle_stats_screen_mouse_wheel(context.world, delta);
            return;
        }
        if context.world.current_book().is_some() {
            self.request_cursor_capture(false, context.platform);
            return;
        }
        if context.world.advancements_screen_is_open() {
            self.request_cursor_capture(false, context.platform);
            handle_advancements_screen_mouse_wheel(context.input, context.world, delta);
            return;
        }
        if context.world.open_container_id().is_some() {
            self.request_cursor_capture(false, context.platform);
            if recipe_book_screen_is_open(context.world) {
                context.platform.submit(PlatformEffect::RecipeBookInput(
                    ScreenNarrationScheduleInput::MouseAction,
                ));
            }
            handle_inventory_mouse_wheel(
                context.input,
                context.world,
                context.net_counters,
                context.net_commands,
                delta,
                self.cursor_position,
                self.gui_surface_size(),
            );
            return;
        }
        if simulation_wants_cursor(context.input, context.world) {
            self.request_cursor_capture(false, context.platform);
            return;
        }
        if self.cursor_captured {
            handle_mouse_wheel(
                context.input,
                context.world,
                context.net_counters,
                context.net_commands,
                delta,
            );
        }
    }

    fn apply_mouse_motion(
        &mut self,
        delta: (f64, f64),
        context: &mut SimulationInputContext<'_, '_, '_>,
    ) {
        if self.code_of_conduct_overlay.is_visible(context.world)
            || simulation_wants_cursor(context.input, context.world)
            || context.input.command_entry_is_active()
        {
            self.request_cursor_capture(false, context.platform);
            return;
        }
        if self.cursor_captured {
            handle_mouse_motion(context.input, delta);
        }
    }

    fn apply_control(
        &mut self,
        request: ControlWriteRequest,
        context: &mut SimulationInputContext<'_, '_, '_>,
    ) {
        match request {
            ControlWriteRequest::Shutdown => self.request_shutdown(context),
            ControlWriteRequest::Screenshot { path } => context
                .platform
                .submit(PlatformEffect::Screenshot(path.into())),
            ControlWriteRequest::CodeOfConduct(request) => {
                self.apply_code_of_conduct_control(request, context);
            }
            ControlWriteRequest::Net(request) => {
                self.apply_net_control(request, context);
            }
            ControlWriteRequest::Input(request) => {
                self.apply_input_control(request, context);
            }
        }
    }

    fn apply_code_of_conduct_control(
        &mut self,
        request: CodeOfConductControlRequest,
        context: &mut SimulationInputContext<'_, '_, '_>,
    ) {
        apply_code_of_conduct_request(
            request,
            context.net_commands,
            context.world,
            context.code_of_conduct.as_deref_mut(),
        );
    }

    fn apply_net_control(
        &mut self,
        request: bbb_control::NetControlRequest,
        context: &mut SimulationInputContext<'_, '_, '_>,
    ) {
        let game_rules_was_open = context.world.game_rules_screen().is_some();
        apply_control_net_request(
            request,
            context.net_commands,
            context.net_counters,
            context.world,
        );
        if !game_rules_was_open && context.world.game_rules_screen().is_some() {
            release_active_input(
                context.input,
                context.world,
                context.net_counters,
                context.net_commands,
            );
            context.input.sync_game_rules_screen(context.world);
            self.request_cursor_capture(false, context.platform);
        }
    }

    fn apply_input_control(
        &mut self,
        request: InputControlRequest,
        context: &mut SimulationInputContext<'_, '_, '_>,
    ) {
        let narrates_chat = matches!(
            &request,
            InputControlRequest::Text { .. } | InputControlRequest::Key { pressed: true, .. }
        );
        let chat_screen_was_open = context.input.chat_entry_is_active()
            || context.input.chat_restrictions_screen_is_open();
        apply_control_input_request_with_effects(
            request,
            context.input,
            context.first_person_hand_presentation,
            context.net_counters,
            context.world,
            context.net_commands,
            context.item_runtime.as_ref(),
            self.code_of_conduct_overlay.is_visible(context.world),
            self.gui_surface_size(),
            context.entity_partial_tick,
            context.level_random,
            context.audio_events,
        );
        if narrates_chat
            && (chat_screen_was_open
                || context.input.chat_entry_is_active()
                || context.input.chat_restrictions_screen_is_open())
        {
            context.platform.submit(PlatformEffect::ChatScreenInput(
                ScreenNarrationScheduleInput::KeyboardAction,
            ));
        }
    }

    fn request_cursor_capture(
        &mut self,
        captured: bool,
        platform: &mut PlatformEffectCollector<'_>,
    ) {
        if self.cursor_captured == captured {
            return;
        }
        self.cursor_captured = captured;
        platform.submit(PlatformEffect::SetCursorCapture(captured));
    }

    fn request_shutdown(&mut self, context: &mut SimulationInputContext<'_, '_, '_>) {
        self.request_network_terminal_shutdown(context.net_commands, context.platform);
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_chat_component_interaction(
    interaction: crate::runtime::HudChatComponentInteraction,
    shift_down: bool,
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    net_counters: &mut bbb_control::NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    platform: &mut PlatformEffectCollector<'_>,
) -> bool {
    if shift_down {
        let Some(insertion) = interaction.insertion else {
            return false;
        };
        return input.insert_chat_screen_text(&insertion, net_counters, world, net_commands);
    }
    let Some(event) = interaction.click_event else {
        return false;
    };
    run_component_click_event(event, input, world, net_counters, net_commands, platform)
}

fn simulation_command_profile_kind(command: &SimulationCommand) -> SimulationCommandProfileKind {
    match command {
        SimulationCommand::Key { .. }
        | SimulationCommand::ImePreedit { .. }
        | SimulationCommand::ImeCommit(_)
        | SimulationCommand::ImeDisabled => SimulationCommandProfileKind::Keyboard,
        SimulationCommand::MouseButton { .. }
        | SimulationCommand::MouseMotion { .. }
        | SimulationCommand::MouseWheel { .. }
        | SimulationCommand::CursorMoved { .. }
        | SimulationCommand::CursorLeft => SimulationCommandProfileKind::Mouse,
        _ => SimulationCommandProfileKind::Other,
    }
}

fn apply_hud_air_bubble_pop_sound(
    world: &mut WorldStore,
    audio_events: &mut Option<&mut dyn AudioEventSink>,
    level_generation: u64,
    position: bbb_protocol::packets::Vec3d,
    volume: f32,
    pitch: f32,
) {
    if world.client_level_generation() != level_generation {
        return;
    }
    let state = world.hud_air_bubble_pop_sound(position, volume, pitch);
    if let Some(audio_events) = audio_events.as_deref_mut() {
        audio_events.play_positioned_sound(&state);
    }
}

fn debug_profiling_commands(
    requests: Vec<DebugProfilingToggleRequest>,
) -> Vec<DebugProfilingCommand> {
    requests
        .into_iter()
        .map(|request| match request {
            DebugProfilingToggleRequest::Start => DebugProfilingCommand::Start,
            DebugProfilingToggleRequest::Stop => DebugProfilingCommand::Stop,
        })
        .collect()
}

fn recipe_book_screen_is_open(world: &WorldStore) -> bool {
    inventory_screen_layout(world)
        .and_then(|layout| recipe_book_type_for_background(layout.background))
        .is_some_and(|book_type| recipe_book_type_settings(world, book_type).open)
}

pub(crate) struct SimulationInputContext<'a, 'audio, 'collector> {
    pub(crate) input: &'a mut crate::input::ClientInputState,
    pub(crate) now: Instant,
    pub(crate) first_person_hand_presentation:
        &'a mut crate::item_models::FirstPersonHandPresentation,
    pub(crate) world: &'a mut WorldStore,
    pub(crate) net_counters: &'a mut bbb_control::NetCounters,
    pub(crate) net_commands: &'a Option<mpsc::Sender<NetCommand>>,
    pub(crate) server_resource_packs: &'a mut ServerResourcePackRuntime,
    pub(crate) services_block_lists:
        &'a mut crate::services_block_list_runtime::ServicesBlockListRuntime,
    pub(crate) particle_runtime: &'a mut Option<crate::particle_runtime::NativeParticleRuntime>,
    pub(crate) item_runtime: &'a mut Option<NativeItemRuntime>,
    pub(crate) terrain_textures: &'a mut crate::terrain_runtime::TerrainTextureState,
    pub(crate) code_of_conduct: Option<&'a mut CodeOfConductAcceptance>,
    pub(crate) win_screen_resources:
        &'a mut std::sync::Arc<crate::win_screen_resources::WinScreenResources>,
    pub(crate) entity_partial_tick: f32,
    pub(crate) level_random: &'a mut bbb_world::LevelEventSoundRandomState,
    pub(crate) audio_events: &'a mut Option<&'audio mut dyn AudioEventSink>,
    pub(crate) platform: &'a mut PlatformEffectCollector<'collector>,
}

/// `Minecraft.resizeGui` (Minecraft.java:1434-1442): `Window.calculateScale`
/// plus `Window.setGuiScale`'s ceiling-division projection.
fn project_gui_scale(
    surface_size: PhysicalSize<u32>,
    gui_scale_max: u32,
    enforce_unicode: bool,
) -> (u32, u32, u32) {
    let gui_scale = crate::window_surface::calculate_gui_scale(
        gui_scale_max,
        enforce_unicode,
        surface_size.width,
        surface_size.height,
    );
    let (gui_scaled_width, gui_scaled_height) =
        crate::window_surface::gui_scaled_size(surface_size.width, surface_size.height, gui_scale);
    (gui_scale, gui_scaled_width, gui_scaled_height)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ResourceReloadAckEffect {
    None,
    ShowLoadFailureToast,
    AbortRecovery { error: String },
}

fn apply_resource_reload_ack(
    ack: crate::resource_reload::ResourceReloadAck,
    context: &mut SimulationInputContext<'_, '_, '_>,
) -> ResourceReloadAckEffect {
    let status = ack.result.as_ref().map(|_| ()).map_err(ToString::to_string);
    let Some(finish) = context
        .server_resource_packs
        .finish_reload(ack.generation, status)
    else {
        tracing::warn!(
            generation = ack.generation,
            "ignored stale server resource pack reload acknowledgement"
        );
        return ResourceReloadAckEffect::None;
    };
    let recovery_error = match (&finish, &ack.result) {
        (ResourcePackReloadFinish::RecoveryFailed, Err(error)) => Some(error.clone()),
        _ => None,
    };
    context.net_counters.client_resources = std::sync::Arc::new(ack.diagnostics);
    match ack.result {
        Ok(commit) => {
            commit.item_defaults.apply(context.world);
            context
                .world
                .set_available_command_sounds(commit.available_command_sounds);
            context
                .world
                .set_default_block_destroy_profiles(commit.block_destroy_profiles);
            context
                .world
                .set_default_block_sound_profiles(commit.block_sound_profiles);
            *context.particle_runtime = Some(commit.particle_runtime);
            *context.item_runtime = Some(commit.item_runtime);
            *context.terrain_textures = commit.terrain_textures;
            *context.win_screen_resources = std::sync::Arc::new(commit.win_screen_resources);
            tracing::info!(
                generation = ack.generation,
                "applied server resource pack reload"
            );
        }
        Err(error) => tracing::warn!(
            generation = ack.generation,
            error = %error,
            "server resource pack reload failed; retained previous resources"
        ),
    }
    context
        .server_resource_packs
        .publish_runtime_diagnostics(context.world);
    match finish {
        ResourcePackReloadFinish::Accepted => ResourceReloadAckEffect::None,
        ResourcePackReloadFinish::RecoverySucceeded => {
            ResourceReloadAckEffect::ShowLoadFailureToast
        }
        ResourcePackReloadFinish::RecoveryFailed => ResourceReloadAckEffect::AbortRecovery {
            error: recovery_error.expect("failed recovery acknowledgement carries an error"),
        },
    }
}

fn world_wants_cursor(world: &WorldStore) -> bool {
    world.open_container_id().is_some()
        || world.death_screen().is_some()
        || world.game_event_screen().is_some()
        || world.dialog_screen_is_open()
        || world.current_book().is_some()
        || world.advancements_screen_is_open()
        || world.stats_screen_is_open()
        || world.player_reporting_screen().is_some()
        || world.social_interactions_screen().is_some()
        || world.game_rules_screen().is_some()
        || world.command_block_editor().is_some()
        || world.minecart_command_block_editor().is_some()
        || world.structure_block_editor().is_some()
        || world.jigsaw_block_editor().is_some()
        || world.test_block_editor().is_some()
        || world.test_instance_block_editor().is_some()
}

fn simulation_wants_cursor(input: &crate::input::ClientInputState, world: &WorldStore) -> bool {
    input.disconnected_screen_runtime().is_some()
        || input.chat_restrictions_screen_is_open()
        || world_wants_cursor(world)
        || input.sign_editor_is_active_or_pending(world)
        || input.debug_options_screen_is_open()
        || input.debug_pause_screen_is_open()
        || input.debug_game_mode_switcher_is_open()
}

fn chat_hit_target_is_current(
    input: &crate::input::ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    target: HudChatHitTarget,
) -> bool {
    target.level_generation == world.client_level_generation()
        && target.delivery_revision == world.client_chat().delivery_revision
        && target.chat_screen_revision == input.chat_screen_revision()
        && target.resource_generation
            == item_runtime.map_or(0, NativeItemRuntime::resource_generation)
        && target.resources_loaded == item_runtime.is_some()
}

#[allow(clippy::too_many_arguments)]
fn apply_book_component_click_event(
    mode: HudBookMode,
    event: ComponentClickEvent,
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    net_counters: &mut bbb_control::NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    platform: &mut PlatformEffectCollector<'_>,
) -> bool {
    match (mode, event) {
        (HudBookMode::View, ComponentClickEvent::ChangePage { page }) => {
            let page = usize::try_from(page.saturating_sub(1).max(0)).unwrap_or_default();
            world.set_current_book_page(page);
            true
        }
        (HudBookMode::Lectern, ComponentClickEvent::ChangePage { page }) => {
            let target_page = page.saturating_sub(1);
            let Some(lectern) = world.lectern_book_view() else {
                return false;
            };
            if target_page != i32::try_from(lectern.current_page).unwrap_or(i32::MAX) {
                queue_lectern_button_click(
                    world,
                    net_counters,
                    net_commands,
                    100_i32.saturating_add(target_page),
                );
            }
            true
        }
        (HudBookMode::Lectern, ComponentClickEvent::RunCommand { command }) => {
            crate::input::queue_container_close_command(net_counters, world, net_commands);
            run_component_click_event(
                ComponentClickEvent::RunCommand { command },
                input,
                world,
                net_counters,
                net_commands,
                platform,
            )
        }
        (_, event) => {
            run_component_click_event(event, input, world, net_counters, net_commands, platform)
        }
    }
}

fn run_component_click_event(
    event: ComponentClickEvent,
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    net_counters: &mut bbb_control::NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    platform: &mut PlatformEffectCollector<'_>,
) -> bool {
    match event {
        ComponentClickEvent::OpenFile { .. } | ComponentClickEvent::ChangePage { .. } => false,
        ComponentClickEvent::OpenUrl { url } => {
            let handled = world.open_chat_url(url);
            let queued_effects = RefCell::new(Vec::new());
            collect_dialog_open_uri_effects(world, &queued_effects);
            flush_platform_effects(&queued_effects, platform);
            handled
        }
        ComponentClickEvent::RunCommand { command } => {
            queue_chat_command(
                net_counters,
                world,
                net_commands,
                command.strip_prefix('/').unwrap_or(&command),
            );
            true
        }
        ComponentClickEvent::SuggestCommand { command } => input.replace_chat_entry_text(command),
        ComponentClickEvent::ShowDialog { dialog } => {
            world.show_dialog_from_component_click(&dialog)
        }
        ComponentClickEvent::CopyToClipboard { value } => {
            if value.is_empty() {
                return true;
            }
            platform.submit(PlatformEffect::ClipboardWrite(value));
            true
        }
        ComponentClickEvent::Custom { id, payload } => {
            if let Some(tx) = net_commands {
                if tx
                    .try_send(NetCommand::CustomClickAction(CustomClickAction {
                        id,
                        payload,
                    }))
                    .is_ok()
                {
                    net_counters.custom_click_action_commands_queued += 1;
                }
            }
            true
        }
    }
}

fn collect_dialog_open_uri_effects(world: &mut WorldStore, effects: &RefCell<Vec<PlatformEffect>>) {
    effects.borrow_mut().extend(
        world
            .take_dialog_open_url_requests()
            .into_iter()
            .map(PlatformEffect::OpenUri),
    );
}

fn collect_game_event_screen_platform_effects(
    input: &mut crate::input::ClientInputState,
    platform: &mut PlatformEffectCollector<'_>,
) {
    for request in input.take_game_event_screen_platform_requests() {
        platform.submit(match request {
            GameEventScreenPlatformRequest::OpenUri(url) => PlatformEffect::OpenUri(url),
            GameEventScreenPlatformRequest::ClipboardWrite(text) => {
                PlatformEffect::ClipboardWrite(text)
            }
        });
    }
}

fn collect_social_interactions_platform_effects(
    input: &mut crate::input::ClientInputState,
    platform: &mut PlatformEffectCollector<'_>,
) {
    for request in input.take_social_interactions_platform_requests() {
        platform.submit(match request {
            SocialInteractionsPlatformRequest::OpenUri(url) => PlatformEffect::OpenUri(url),
            SocialInteractionsPlatformRequest::ClipboardWrite(text) => {
                PlatformEffect::ClipboardWrite(text)
            }
        });
    }
}

fn collect_player_reporting_platform_effects(
    input: &mut crate::input::ClientInputState,
    platform: &mut PlatformEffectCollector<'_>,
) {
    for request in input.take_player_reporting_platform_requests() {
        platform.submit(match request {
            PlayerReportingPlatformRequest::OpenUri(url) => PlatformEffect::OpenUri(url),
        });
    }
}

fn collect_death_screen_platform_effects(
    input: &mut crate::input::ClientInputState,
    platform: &mut PlatformEffectCollector<'_>,
) {
    for request in input.take_death_screen_platform_requests() {
        platform.submit(match request {
            DeathScreenPlatformRequest::OpenUri(url) => PlatformEffect::OpenUri(url),
            DeathScreenPlatformRequest::ClipboardWrite(text) => {
                PlatformEffect::ClipboardWrite(text)
            }
        });
    }
}

fn collect_disconnected_screen_platform_effects(
    input: &mut crate::input::ClientInputState,
    platform: &mut PlatformEffectCollector<'_>,
) {
    for request in input.take_disconnected_screen_platform_requests() {
        platform.submit(match request {
            DisconnectedScreenPlatformRequest::OpenUri(url) => PlatformEffect::OpenUri(url),
            DisconnectedScreenPlatformRequest::OpenPath(path) => PlatformEffect::OpenPath(path),
            DisconnectedScreenPlatformRequest::Exit => PlatformEffect::Exit,
        });
    }
}

fn flush_platform_effects(
    effects: &RefCell<Vec<PlatformEffect>>,
    platform: &mut PlatformEffectCollector<'_>,
) {
    for effect in effects.take() {
        platform.submit(effect);
    }
}

struct SimulationClipboard<'a> {
    read: Option<String>,
    effects: &'a RefCell<Vec<PlatformEffect>>,
}

impl DebugClipboard for SimulationClipboard<'_> {
    fn set_debug_clipboard_text(&mut self, text: &str) -> bool {
        self.effects
            .borrow_mut()
            .push(PlatformEffect::ClipboardWrite(text.to_string()));
        true
    }

    fn get_debug_clipboard_text(&mut self) -> Option<String> {
        self.read.take()
    }
}

struct SimulationTerrainReload<'a> {
    effects: &'a RefCell<Vec<PlatformEffect>>,
}

impl TerrainReloadRequestSink for SimulationTerrainReload<'_> {
    fn request_reload_all_chunks(&mut self) {
        self.effects
            .borrow_mut()
            .push(PlatformEffect::ReloadTerrain);
    }
}

impl DebugOptionsSearchTextMeasurer for HudFontGlyphMap {
    fn debug_options_search_display_start_for_width(
        &self,
        search_text: &str,
        scroll_to: usize,
        width: i32,
    ) -> usize {
        self.plain_text_display_start_for_width(
            search_text,
            scroll_to,
            u32::try_from(width.max(0)).unwrap_or(u32::MAX),
        )
        .unwrap_or_else(|| {
            crate::input::debug_options_search_display_start_for_width_fallback(
                search_text,
                scroll_to,
                width,
            )
        })
    }

    fn debug_options_search_cursor_for_text_offset_from_display_start(
        &self,
        search_text: &str,
        display_start: usize,
        offset: i32,
    ) -> usize {
        self.plain_text_cursor_for_width_from(
            search_text,
            display_start,
            u32::try_from(offset.max(0)).unwrap_or(u32::MAX),
        )
        .unwrap_or_else(|| {
            crate::input::debug_options_search_cursor_for_text_offset_from_display_start_fallback(
                search_text,
                display_start,
                offset,
            )
        })
    }
}

impl CommandSuggestionTextMeasurer for HudFontGlyphMap {
    fn command_suggestion_text_width(&self, text: &str) -> u32 {
        self.plain_text_width(text)
    }

    fn command_suggestion_display_start(&self, text: &str, scroll_to: usize, width: u32) -> usize {
        self.plain_text_display_start_for_width(text, scroll_to, width)
            .unwrap_or(0)
    }

    fn command_suggestion_cursor_for_width(
        &self,
        text: &str,
        display_start: usize,
        width: u32,
    ) -> usize {
        self.plain_text_cursor_for_width_from(text, display_start, width)
            .unwrap_or(display_start)
    }
}

impl DialogTextMeasurer for HudFontGlyphMap {
    fn width(&self, text: &str) -> u32 {
        self.plain_text_width(text)
    }

    fn display_start(&self, text: &str, scroll_to: usize, width: u32) -> usize {
        self.plain_text_display_start_for_width(text, scroll_to, width)
            .unwrap_or(0)
    }

    fn cursor_for_width(&self, text: &str, display_start: usize, width: u32) -> usize {
        self.plain_text_cursor_for_width_from(text, display_start, width)
            .unwrap_or(display_start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingRenderSink(Vec<PlatformEffect>);

    impl crate::simulation::effects::ParticleBatchSink for RecordingRenderSink {
        fn submit_particle_batch(&mut self, _batch: bbb_renderer::ParticleSpawnBatch) {}
    }

    impl crate::simulation::effects::PlatformEffectSink for RecordingRenderSink {
        fn apply_platform_effect(&mut self, effect: PlatformEffect) {
            self.0.push(effect);
        }
    }

    #[test]
    fn profiling_toggle_requests_preserve_start_stop_order() {
        assert_eq!(
            debug_profiling_commands(vec![
                DebugProfilingToggleRequest::Start,
                DebugProfilingToggleRequest::Stop,
                DebugProfilingToggleRequest::Start,
            ]),
            vec![
                DebugProfilingCommand::Start,
                DebugProfilingCommand::Stop,
                DebugProfilingCommand::Start,
            ]
        );
    }

    #[test]
    fn text_input_is_reconciled_once_per_applied_step() {
        // `TextInputManager.tick` runs every tick, not only when a screen
        // opens: a text field can lose focus without any command of its own,
        // and an IME left allowed grabs every letter key after that.
        let source = include_str!("input_commands.rs");
        let body = source
            .split_once("pub(crate) fn apply_commands(")
            .expect("command batch owner")
            .1;
        let sync = body
            .find("self.sync_text_input_enabled(context)")
            .expect("per-step text-input reconciliation");
        let preedit = body
            .find("self.sync_ime_preedit_focus(context)")
            .expect("per-step preedit reconciliation");
        // The preedit is routed to its target first, so the window is told to
        // stop accepting text only after the composition it owns is handed on.
        assert!(preedit < sync);
        // One owner: nothing else may hand the window an IME decision.  Count
        // in production only -- this test names the effect too.
        let production = source.split_once("\nmod tests {").expect("test boundary").0;
        assert_eq!(production.matches("SetTextInputEnabled(").count(), 1);
    }

    #[test]
    fn the_global_press_keys_are_answered_before_any_screen_dispatch() {
        // `KeyboardHandler.keyPress` (KeyboardHandler.java:465-489) answers
        // `keyFullscreen` and `keyScreenshot` before `screen.keyPressed`.  The
        // pause screen here swallows every key it does not recognise, so a
        // dispatch that ran first would make F11 and F2 dead while it is open —
        // which is exactly what a live seat-driven run found.
        let source = include_str!("input_commands.rs");
        let body = source
            .split_once("fn apply_key(")
            .expect("key command owner")
            .1;
        let global = body
            .find("handle_global_key_press(physical_key, state)")
            .expect("vanilla pre-screen press block");
        for screen_branch in [
            "disconnected_screen_runtime().is_some()",
            "handle_debug_pause_screen_key(",
            "handle_debug_options_screen_key_with_clipboard_and_surface(",
            "handle_key_input_with_item_runtime(",
        ] {
            let dispatch = body.find(screen_branch).expect(screen_branch);
            assert!(
                global < dispatch,
                "{screen_branch} must not be able to hide the global press keys"
            );
        }
        // The toggle only becomes a window transition once the effect is
        // submitted, which is what this drain does.
        assert!(body[global..global + 200].contains("apply_debug_requests(context)"));
    }

    #[test]
    fn pause_screen_links_reuse_dialog_policy_and_sync_the_parent_stack() {
        let source = include_str!("input_commands.rs");
        let pause_start = source
            .find("if context.input.debug_pause_screen_is_open()")
            .expect("pause-screen mouse branch");
        let pause_end = source[pause_start..]
            .find("if context.world.death_screen().is_some()")
            .map(|offset| pause_start + offset)
            .expect("next mouse branch");
        let pause_branch = &source[pause_start..pause_end];

        let open_url = pause_branch
            .find(".open_chat_url(")
            .expect("shared URL policy");
        let collect = pause_branch
            .find("collect_dialog_open_uri_effects")
            .expect("dialog URI effects");
        let sync = pause_branch
            .find("sync_dialog_native_previous_screen")
            .expect("dialog parent sync");
        assert!(open_url < collect && collect < sync);
        assert!(!pause_branch.contains("confirm-link screen is not implemented"));
    }

    #[test]
    fn debug_pause_hotkey_opens_the_full_menu_like_vanilla_external_clients() {
        // Vanilla `pauseGame(debugModifierDown)` suppresses the menu only when
        // an unpublished singleplayer server can really pause; the external
        // client always constructs `PauseScreen(true)`.
        let source = include_str!("input_commands.rs");
        let start = source
            .find("take_debug_pause_screen_requests")
            .expect("pause hotkey consumer");
        let branch = &source[start..start + 700];
        assert!(branch.contains("open_debug_pause_screen_with_menu"));
        assert!(!include_str!("../input/pause_screen.rs")
            .contains("fn open_debug_pause_screen_without_menu("));
    }

    #[test]
    fn terrain_readiness_is_scoped_to_the_published_level_generation() {
        let mut runtime = SimulationInputRuntime::new(
            PhysicalSize::new(1280, 720),
            false,
            ClientInformation::default(),
        );
        runtime.terrain_readiness = Some((7, 1, true));

        assert!(runtime.player_section_ready(7, 1));
        assert!(!runtime.player_section_ready(7, 2));
        assert!(!runtime.player_section_ready(8, 1));

        runtime.terrain_readiness = Some((8, 1, false));
        assert!(!runtime.player_section_ready(8, 1));
    }

    #[test]
    fn surface_resize_recomputes_gui_scale_and_projection() {
        let mut runtime = SimulationInputRuntime::new(
            PhysicalSize::new(1920, 1080),
            false,
            ClientInformation::default(),
        );
        runtime.set_gui_scale_options(0, false);
        assert_eq!(runtime.gui_scale, 4);
        assert_eq!(runtime.gui_scaled_width, 480);
        assert_eq!(runtime.gui_scaled_height, 270);
        assert!(!runtime.minimized);

        runtime.set_surface_size(PhysicalSize::new(1280, 720));
        assert_eq!(runtime.gui_scale, 3);
        assert_eq!(runtime.gui_scaled_width, 427);
        assert_eq!(runtime.gui_scaled_height, 240);
        assert_eq!(runtime.surface_size, PhysicalSize::new(1280, 720));
    }

    #[test]
    fn zero_size_resize_marks_minimized_and_keeps_last_usable_surface() {
        let mut runtime = SimulationInputRuntime::new(
            PhysicalSize::new(1280, 720),
            false,
            ClientInformation::default(),
        );
        runtime.set_gui_scale_options(2, true);
        assert_eq!(runtime.gui_scale, 2);

        runtime.set_surface_size(PhysicalSize::new(0, 0));
        assert!(runtime.minimized);
        assert_eq!(runtime.surface_size, PhysicalSize::new(1280, 720));
        assert_eq!(runtime.gui_scale, 2);

        runtime.set_surface_size(PhysicalSize::new(1920, 1080));
        assert!(!runtime.minimized);
        assert_eq!(runtime.surface_size, PhysicalSize::new(1920, 1080));
        assert_eq!(runtime.gui_scale, 2);
    }

    #[test]
    fn manual_gui_scale_cap_and_unicode_rounding_flow_into_projection() {
        let mut runtime = SimulationInputRuntime::new(
            PhysicalSize::new(1920, 1080),
            false,
            ClientInformation::default(),
        );
        runtime.set_gui_scale_options(3, true);
        assert_eq!(runtime.gui_scale, 4);
        assert_eq!(runtime.gui_scaled_width, 480);
        assert_eq!(runtime.gui_scaled_height, 270);
    }

    #[test]
    fn pause_if_inactive_is_evaluated_after_focus_changed_commands_in_the_step() {
        // `Minecraft.pauseIfInactive` observes the already-updated focus state
        // (Minecraft.java:1382-1390), so the command loop — including
        // `FocusChanged` -> `apply_focus` -> `input.focused = focused` — must
        // run before the pause evaluation. The behavioral half of this
        // regression lives in `input/tests.rs`; this test pins the ordering in
        // the production source.
        let source = include_str!("input_commands.rs");
        let production = source
            .split_once("\nmod tests {")
            .expect("production/test boundary")
            .0;
        let command_loop = production
            .find("for command in commands {")
            .expect("step command loop");
        let shutdown_return = production
            .find("if self.shutdown_requested {\n            return profile;\n        }")
            .expect("shutdown early return after the loop");
        let pause = production
            .find("context.input.pause_if_inactive(context.now, context.world)")
            .expect("pause evaluation after the step");
        assert!(command_loop < shutdown_return && shutdown_return < pause);
        let loop_region = &production[command_loop..shutdown_return];
        assert!(loop_region.contains("self.apply_command(command.command, context)"));
    }

    #[tokio::test]
    async fn focus_regained_in_the_same_step_prevents_the_pause_screen() {
        use crate::services_block_list_runtime::ServicesBlockListRuntime;
        use crate::terrain_runtime::TerrainTextureState;
        use bbb_control::{AudioCounters, NetCounters};
        use tokio::runtime::Handle;
        use uuid::Uuid;

        // The production `apply_commands` path, not a direct helper call: a
        // sequenced `FocusChanged(true)` must run before the post-step
        // `pauseIfInactive` evaluation observes the focus state.
        let mailbox = crate::simulation::commands::SimulationMailbox::default();
        mailbox
            .enqueue(SimulationCommand::FocusChanged(true))
            .expect("focus command queues into an empty mailbox");
        let commands = mailbox.drain_through_barrier();
        assert_eq!(commands.len(), 1);

        let now = Instant::now();
        let mut input = crate::input::ClientInputState::new(false);
        // The window has been unfocused since `now`, so by `now + 600 ms` it
        // is past the strict >500 ms delay (`Minecraft.pauseIfInactive`).
        input.set_last_active_at_for_test(now);
        let mut world = WorldStore::new();
        let mut net_counters = NetCounters::default();
        let mut first_person_hand_presentation =
            crate::item_models::FirstPersonHandPresentation::default();
        let mut server_resource_packs = ServerResourcePackRuntime::new(
            Handle::current(),
            std::env::temp_dir(),
            "window-surface-focus-test",
            Uuid::nil(),
            None,
        );
        let mut services_block_lists = ServicesBlockListRuntime::new(Handle::current(), mailbox);
        let mut particle_runtime = None;
        let mut item_runtime = None;
        let mut terrain_textures = TerrainTextureState::default();
        let mut win_screen_resources =
            std::sync::Arc::new(crate::win_screen_resources::WinScreenResources {
                lines: Vec::new(),
                narration_components: Vec::new(),
                total_scroll_length: 0,
            });
        let mut level_random = bbb_world::LevelEventSoundRandomState::default();
        let mut audio_events: Option<&mut dyn AudioEventSink> = None;
        let collector =
            crate::simulation::effects::RenderEffectCollector::new(1, AudioCounters::default());
        let mut platform = collector.platform();

        let mut runtime = SimulationInputRuntime::new(
            PhysicalSize::new(1280, 720),
            false,
            ClientInformation::default(),
        );
        let mut context = SimulationInputContext {
            input: &mut input,
            now: now + Duration::from_millis(600),
            first_person_hand_presentation: &mut first_person_hand_presentation,
            world: &mut world,
            net_counters: &mut net_counters,
            net_commands: &None,
            server_resource_packs: &mut server_resource_packs,
            services_block_lists: &mut services_block_lists,
            particle_runtime: &mut particle_runtime,
            item_runtime: &mut item_runtime,
            terrain_textures: &mut terrain_textures,
            code_of_conduct: None,
            win_screen_resources: &mut win_screen_resources,
            entity_partial_tick: 0.0,
            level_random: &mut level_random,
            audio_events: &mut audio_events,
            platform: &mut platform,
        };
        runtime.apply_commands(commands, &mut context, false);

        // The focus-regained command applied first, so the post-step pause
        // evaluation observes the updated focus and must not open a pause
        // screen even though the window was unfocused for >500 ms. The
        // sequence also proves the command itself was applied rather than
        // dropped.
        assert_eq!(runtime.last_applied_sequence, 1);
        assert!(!input.debug_pause_screen_is_open());
    }

    #[tokio::test]
    async fn an_open_composition_hands_off_to_a_newly_focused_sign_editor() {
        use crate::services_block_list_runtime::ServicesBlockListRuntime;
        use crate::terrain_runtime::TerrainTextureState;
        use bbb_control::{AudioCounters, NetCounters};
        use tokio::runtime::Handle;
        use uuid::Uuid;

        // Vanilla `KeyboardHandler` stores `lastPreeditEvent` even when no
        // element renders it; `Minecraft.onTextInputFocusChange` then resubmits
        // it to the element gaining focus.
        let mailbox = crate::simulation::commands::SimulationMailbox::default();
        mailbox
            .enqueue(SimulationCommand::ImePreedit {
                text: "compose".to_string(),
                cursor_range: Some((3, 3)),
            })
            .expect("preedit command queues into an empty mailbox");
        let commands = mailbox.drain_through_barrier();

        let now = Instant::now();
        let mut input = crate::input::ClientInputState::new(true);
        let mut world = WorldStore::new();
        // `sign_editor_is_valid_at` needs a local player within the vanilla
        // block-interaction range of the sign.
        world.apply_login(&bbb_protocol::packets::PlayLogin {
            player_id: 7,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: bbb_protocol::packets::CommonPlayerSpawnInfo {
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
        world.set_local_player_pose(bbb_world::LocalPlayerPoseState {
            position: bbb_protocol::packets::Vec3d {
                x: -4.5,
                y: 69.0,
                z: 12.5,
            },
            on_ground: true,
            ..bbb_world::LocalPlayerPoseState::default()
        });
        let mut net_counters = NetCounters::default();
        let mut first_person_hand_presentation =
            crate::item_models::FirstPersonHandPresentation::default();
        let mut server_resource_packs = ServerResourcePackRuntime::new(
            Handle::current(),
            std::env::temp_dir(),
            "ime-preedit-handoff-test",
            Uuid::nil(),
            None,
        );
        let mut services_block_lists = ServicesBlockListRuntime::new(Handle::current(), mailbox);
        let mut particle_runtime = None;
        let mut item_runtime = None;
        let mut terrain_textures = TerrainTextureState::default();
        let mut win_screen_resources =
            std::sync::Arc::new(crate::win_screen_resources::WinScreenResources {
                lines: Vec::new(),
                narration_components: Vec::new(),
                total_scroll_length: 0,
            });
        let mut level_random = bbb_world::LevelEventSoundRandomState::default();
        let mut audio_events: Option<&mut dyn AudioEventSink> = None;
        let collector =
            crate::simulation::effects::RenderEffectCollector::new(1, AudioCounters::default());
        let mut platform = collector.platform();

        let mut runtime = SimulationInputRuntime::new(
            PhysicalSize::new(1280, 720),
            false,
            ClientInformation::default(),
        );
        {
            let mut context = SimulationInputContext {
                input: &mut input,
                now,
                first_person_hand_presentation: &mut first_person_hand_presentation,
                world: &mut world,
                net_counters: &mut net_counters,
                net_commands: &None,
                server_resource_packs: &mut server_resource_packs,
                services_block_lists: &mut services_block_lists,
                particle_runtime: &mut particle_runtime,
                item_runtime: &mut item_runtime,
                terrain_textures: &mut terrain_textures,
                code_of_conduct: None,
                win_screen_resources: &mut win_screen_resources,
                entity_partial_tick: 0.0,
                level_random: &mut level_random,
                audio_events: &mut audio_events,
                platform: &mut platform,
            };
            runtime.apply_commands(commands, &mut context, false);
        }
        assert!(input.sign_editor_hud_state(&world).is_none());

        // The sign editor opens with no further IME traffic; the next step's
        // focus sync must hand the stored composition to the gaining editor.
        world.insert_decoded_chunk(bbb_world::ChunkColumn {
            pos: bbb_world::ChunkPos { x: -1, z: 0 },
            state: bbb_world::ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: Vec::new(),
            block_entities: vec![bbb_world::BlockEntityRecord {
                local_x: 11,
                y: 70,
                local_z: 12,
                type_id: 7,
                raw_nbt: vec![0],
                nbt: None,
                sign_text: Some(bbb_world::SignBlockEntityTextState {
                    front: bbb_world::SignTextSideState::default(),
                    back: bbb_world::SignTextSideState::default(),
                    is_waxed: false,
                }),
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
            light: bbb_world::LightData::default(),
        });
        world.apply_open_sign_editor(bbb_protocol::packets::OpenSignEditor {
            pos: bbb_protocol::packets::BlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            is_front_text: true,
        });
        {
            let mut context = SimulationInputContext {
                input: &mut input,
                now,
                first_person_hand_presentation: &mut first_person_hand_presentation,
                world: &mut world,
                net_counters: &mut net_counters,
                net_commands: &None,
                server_resource_packs: &mut server_resource_packs,
                services_block_lists: &mut services_block_lists,
                particle_runtime: &mut particle_runtime,
                item_runtime: &mut item_runtime,
                terrain_textures: &mut terrain_textures,
                code_of_conduct: None,
                win_screen_resources: &mut win_screen_resources,
                entity_partial_tick: 0.0,
                level_random: &mut level_random,
                audio_events: &mut audio_events,
                platform: &mut platform,
            };
            runtime.apply_commands(Vec::new(), &mut context, false);
        }
        let editor = input
            .sign_editor_hud_state(&world)
            .expect("sign editor opened");
        assert!(
            editor.preedit.is_some(),
            "the stored composition must reach the newly focused sign editor"
        );
    }

    #[test]
    fn hud_air_bubble_pop_command_rejects_stale_level_generations() {
        let mut world = WorldStore::new();
        let position = bbb_protocol::packets::Vec3d {
            x: 1.25,
            y: 64.0,
            z: -3.5,
        };
        let mut audio_events = None;
        let level_generation = world.client_level_generation();

        apply_hud_air_bubble_pop_sound(
            &mut world,
            &mut audio_events,
            level_generation.saturating_add(1),
            position,
            1.0,
            1.3,
        );
        assert!(world.last_sound().is_none());

        apply_hud_air_bubble_pop_sound(
            &mut world,
            &mut audio_events,
            level_generation,
            position,
            1.0,
            1.3,
        );
        let sound = world.last_sound().expect("current-generation HUD sound");
        assert_eq!(
            sound.sound.location.as_deref(),
            Some("minecraft:ui.hud.bubble_pop")
        );
        assert_eq!(sound.position, position);
    }

    #[test]
    fn network_terminal_shutdown_is_idempotent_and_requests_disconnect() {
        let mut runtime = SimulationInputRuntime::new(
            PhysicalSize::new(1280, 720),
            false,
            ClientInformation::default(),
        );
        let (tx, mut rx) = mpsc::channel(2);
        let effects = crate::simulation::effects::RenderEffectCollector::new(
            1,
            bbb_control::AudioCounters::default(),
        );
        let mut platform = effects.platform();

        runtime.request_network_terminal_shutdown(&Some(tx), &mut platform);
        runtime.request_network_terminal_shutdown(&None, &mut platform);

        assert!(!runtime.running());
        assert!(matches!(rx.try_recv(), Ok(NetCommand::Disconnect)));
        assert!(rx.try_recv().is_err());
        drop(platform);
        let _batch = effects.finish(None);
    }

    #[test]
    fn chat_hit_target_revision_rejects_new_delivery() {
        let mut world = WorldStore::new();
        let mut input = crate::input::ClientInputState::new(true);
        let mut counters = bbb_control::NetCounters::default();
        crate::input::handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &None,
            PhysicalKey::Code(KeyCode::KeyT),
            ElementState::Pressed,
        );
        let target = HudChatHitTarget {
            level_generation: world.client_level_generation(),
            delivery_revision: world.client_chat().delivery_revision,
            chat_screen_revision: input.chat_screen_revision(),
            resource_generation: 0,
            resources_loaded: false,
            kind: HudChatHitTargetKind::Queue,
        };
        assert!(chat_hit_target_is_current(&input, &world, None, target));
        let stale_resource_target = HudChatHitTarget {
            resources_loaded: true,
            ..target
        };
        assert!(!chat_hit_target_is_current(
            &input,
            &world,
            None,
            stale_resource_target
        ));

        crate::input::handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &None,
            PhysicalKey::Code(KeyCode::Escape),
            ElementState::Pressed,
        );
        crate::input::handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &None,
            PhysicalKey::Code(KeyCode::KeyT),
            ElementState::Pressed,
        );
        assert!(!chat_hit_target_is_current(&input, &world, None, target));

        world.push_server_system_chat_message("new delivery");
        assert!(!chat_hit_target_is_current(&input, &world, None, target));
    }

    #[test]
    fn shift_chat_interaction_inserts_and_suppresses_the_click_event() {
        let mut input = crate::input::ClientInputState::new(true);
        let mut world = WorldStore::new();
        let mut counters = bbb_control::NetCounters::default();
        crate::input::handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &None,
            PhysicalKey::Code(KeyCode::KeyT),
            ElementState::Pressed,
        );
        let effects = crate::simulation::effects::RenderEffectCollector::new(
            1,
            bbb_control::AudioCounters::default(),
        );
        let mut platform = effects.platform();
        let interaction = crate::runtime::HudChatComponentInteraction {
            click_event: Some(ComponentClickEvent::SuggestCommand {
                command: "/clicked".to_string(),
            }),
            insertion: Some("inserted".to_string()),
        };
        assert!(apply_chat_component_interaction(
            interaction.clone(),
            true,
            &mut input,
            &mut world,
            &mut counters,
            &None,
            &mut platform,
        ));
        assert_eq!(
            input
                .command_suggestion_hud_state(
                    &world,
                    &bbb_renderer::HudFontGlyphMap::default(),
                    PhysicalSize::new(800, 600),
                )
                .map(|state| state.input_text),
            Some("inserted".to_string())
        );
        assert!(apply_chat_component_interaction(
            interaction,
            false,
            &mut input,
            &mut world,
            &mut counters,
            &None,
            &mut platform,
        ));
        assert_eq!(
            input
                .command_suggestion_hud_state(
                    &world,
                    &bbb_renderer::HudFontGlyphMap::default(),
                    PhysicalSize::new(800, 600),
                )
                .map(|state| state.input_text),
            Some("/clicked".to_string())
        );
    }

    #[test]
    fn chat_component_actions_preserve_safe_platform_and_network_boundaries() {
        let mut input = crate::input::ClientInputState::new(true);
        let mut world = WorldStore::new();
        let mut counters = bbb_control::NetCounters::default();
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        let effects = crate::simulation::effects::RenderEffectCollector::new(
            1,
            bbb_control::AudioCounters::default(),
        );
        let mut platform = effects.platform();

        assert!(run_component_click_event(
            ComponentClickEvent::RunCommand {
                command: "/seed".to_string(),
            },
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            &mut platform,
        ));
        assert!(matches!(
            rx.try_recv(),
            Ok(NetCommand::ChatCommand(command)) if command.command == "seed"
        ));
        assert!(run_component_click_event(
            ComponentClickEvent::Custom {
                id: "example:chat".to_string(),
                payload: None,
            },
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            &mut platform,
        ));
        assert!(matches!(
            rx.try_recv(),
            Ok(NetCommand::CustomClickAction(CustomClickAction { id, .. }))
                if id == "example:chat"
        ));
        assert!(run_component_click_event(
            ComponentClickEvent::CopyToClipboard {
                value: "copy me".to_string(),
            },
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            &mut platform,
        ));
        assert!(!run_component_click_event(
            ComponentClickEvent::OpenFile {
                path: "/tmp/server-controlled".to_string(),
            },
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            &mut platform,
        ));
        drop(platform);

        let batch = effects.finish(None).expect("clipboard effect batch");
        let mut consumer = crate::simulation::effects::RenderEffectConsumer::default();
        let mut sink = RecordingRenderSink::default();
        consumer.consume(batch, None, &mut sink).unwrap();
        assert_eq!(
            sink.0,
            [PlatformEffect::ClipboardWrite("copy me".to_string())]
        );
        assert_eq!(counters.chat_command_commands_queued, 1);
        assert_eq!(counters.custom_click_action_commands_queued, 1);
    }

    #[test]
    fn lectern_component_click_keeps_page_server_owned_and_closes_before_command() {
        let mut input = crate::input::ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: bbb_protocol::menu_types::VANILLA_MENU_TYPE_LECTERN_ID,
            title: bbb_protocol::RichTextComponent::plain("Lectern"),
        });
        let mut book = bbb_protocol::packets::ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: Default::default(),
        };
        book.component_patch.written_book =
            Some(bbb_protocol::packets::WrittenBookContentSummary {
                title: "Guide".to_string(),
                title_filter: None,
                author: "Alex".to_string(),
                generation: 0,
                pages: vec!["one".to_string(), "two".to_string(), "three".to_string()],
                page_filters: vec![None, None, None],
                resolved: true,
            });
        assert!(
            world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
                container_id: 7,
                state_id: 1,
                items: vec![book],
                carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
            })
        );

        let mut counters = bbb_control::NetCounters::default();
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        let effects = crate::simulation::effects::RenderEffectCollector::new(
            1,
            bbb_control::AudioCounters::default(),
        );
        let mut platform = effects.platform();

        assert!(apply_book_component_click_event(
            HudBookMode::Lectern,
            ComponentClickEvent::ChangePage { page: 3 },
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            &mut platform,
        ));
        assert_eq!(world.lectern_book_view().unwrap().current_page, 0);
        assert!(matches!(
            rx.try_recv(),
            Ok(NetCommand::ContainerButtonClick(packet))
                if packet.container_id == 7 && packet.button_id == 102
        ));

        assert!(apply_book_component_click_event(
            HudBookMode::Lectern,
            ComponentClickEvent::RunCommand {
                command: "/seed".to_string(),
            },
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            &mut platform,
        ));
        assert!(world.lectern_book_view().is_none());
        assert!(matches!(
            rx.try_recv(),
            Ok(NetCommand::ContainerClose(packet)) if packet.container_id == 7
        ));
        assert!(matches!(
            rx.try_recv(),
            Ok(NetCommand::ChatCommand(command)) if command.command == "seed"
        ));
        assert!(rx.try_recv().is_err());
        drop(platform);
        let _batch = effects.finish(None);
    }
}
