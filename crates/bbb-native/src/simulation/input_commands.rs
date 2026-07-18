//! Canonical application of typed winit and control commands.

use std::cell::RefCell;

use bbb_control::{
    CodeOfConductControlRequest, ControlWriteRequest, InputControlRequest, SharedControlRequests,
};
use bbb_item_model::NativeItemRuntime;
use bbb_net::NetCommand;
use bbb_renderer::{HudFontGlyphMap, Renderer};
use bbb_world::WorldStore;
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::{KeyCode, PhysicalKey},
};

use super::{
    commands::{SequencedSimulationCommand, SimulationCommand},
    effects::{PlatformEffect, PlatformEffectCollector, StatsScreenInput},
};
use crate::{
    code_of_conduct::CodeOfConductAcceptance,
    code_of_conduct_overlay::CodeOfConductOverlayState,
    input::{
        handle_advancements_screen_cursor_moved, handle_advancements_screen_mouse_input,
        handle_advancements_screen_mouse_wheel, handle_book_screen_mouse_input,
        handle_focus_change, handle_inventory_cursor_moved,
        handle_inventory_mouse_input_with_item_runtime, handle_inventory_mouse_wheel,
        handle_key_input_with_item_runtime, handle_mouse_input_at_partial_tick,
        handle_mouse_motion, handle_mouse_wheel, handle_text_input_with_item_runtime,
        queue_debug_recreate_server_query_request, release_active_input,
        CommandSuggestionTextMeasurer, DebugClipboard, DebugFeatureCountRequest,
        DebugFrustumRequest, DebugOptionsSearchTextMeasurer, DebugProfilingToggleRequest,
        PauseScreenLinkRequest, TerrainReloadRequestSink,
    },
    resource_pack_runtime::ServerResourcePackRuntime,
    runtime::{
        handle_dialog_cursor_moved_with_measurer,
        handle_dialog_key_input_with_modifiers_for_surface_and_measurer,
        handle_dialog_mouse_input_with_modifiers_and_measurer, handle_dialog_mouse_wheel,
        handle_dialog_text_input_with_measurer, pump_control_input_requests,
        pump_control_net_requests, request_net_disconnect, DialogTextMeasurer,
    },
};

/// Simulation-owned state that used to live as mutable locals in the winit
/// callback.
#[derive(Debug)]
pub(crate) struct SimulationInputRuntime {
    surface_size: PhysicalSize<u32>,
    cursor_position: Option<PhysicalPosition<f64>>,
    cursor_captured: bool,
    text_measurements: HudFontGlyphMap,
    code_of_conduct_overlay: CodeOfConductOverlayState,
    net_disconnect_requested: bool,
    shutdown_requested: bool,
    last_applied_sequence: u64,
}

impl SimulationInputRuntime {
    pub(crate) fn new(surface_size: PhysicalSize<u32>, cursor_captured: bool) -> Self {
        Self {
            surface_size,
            cursor_position: None,
            cursor_captured,
            text_measurements: HudFontGlyphMap::default(),
            code_of_conduct_overlay: CodeOfConductOverlayState::default(),
            net_disconnect_requested: false,
            shutdown_requested: false,
            last_applied_sequence: 0,
        }
    }

    pub(crate) fn update_code_of_conduct_overlay_renderer(
        &mut self,
        renderer: &mut Renderer,
        world: &WorldStore,
        accepted_current_code_of_conduct: bool,
    ) {
        self.code_of_conduct_overlay.update_renderer(
            renderer,
            world,
            accepted_current_code_of_conduct,
        );
    }

    pub(crate) fn apply_commands(
        &mut self,
        commands: Vec<SequencedSimulationCommand>,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        for command in commands {
            debug_assert!(command.sequence > self.last_applied_sequence);
            self.last_applied_sequence = command.sequence;
            self.apply_command(command.command, context);
        }
        let queued_effects = RefCell::new(Vec::new());
        let mut clipboard = SimulationClipboard {
            read: None,
            effects: &queued_effects,
        };
        context
            .input
            .consume_debug_recreate_server_query_response(context.world, &mut clipboard);
        flush_platform_effects(&queued_effects, context.platform);
        if self.code_of_conduct_overlay.is_visible(context.world)
            || simulation_wants_cursor(context.input, context.world)
        {
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
    }

    fn apply_command(
        &mut self,
        command: SimulationCommand,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        match command {
            SimulationCommand::FocusChanged(focused) => self.apply_focus(focused, context),
            SimulationCommand::Key {
                physical_key,
                state,
                clipboard_text,
            } => self.apply_key(physical_key, state, clipboard_text, context),
            SimulationCommand::MouseButton { button, state } => {
                self.apply_mouse_button(button, state, context);
            }
            SimulationCommand::MouseMotion { delta } => self.apply_mouse_motion(delta, context),
            SimulationCommand::MouseWheel { delta } => self.apply_mouse_wheel(delta, context),
            SimulationCommand::CursorMoved { position } => {
                self.apply_cursor_moved(position, context);
            }
            SimulationCommand::ImePreedit { text, cursor_range } => {
                self.apply_ime_preedit(&text, cursor_range, context);
            }
            SimulationCommand::ImeCommit(text) => self.apply_ime_commit(&text, context),
            SimulationCommand::ImeDisabled => context.input.clear_dialog_ime_preedit(),
            SimulationCommand::SurfaceResized(size) => self.surface_size = size,
            SimulationCommand::TextMeasurements(measurements) => {
                self.text_measurements = measurements;
            }
            SimulationCommand::CursorCaptureResult { captured } => {
                self.cursor_captured = captured;
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
            SimulationCommand::ResourceReloadAck(ack) => {
                apply_resource_reload_ack(ack, context);
            }
            SimulationCommand::Control(request) => self.apply_control(request, context),
            SimulationCommand::Shutdown => self.request_shutdown(context),
        }
    }

    fn apply_focus(&mut self, focused: bool, context: &mut SimulationInputContext<'_, '_>) {
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

    fn apply_cursor_moved(
        &mut self,
        position: PhysicalPosition<f64>,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        let previous_cursor_position = self.cursor_position;
        self.cursor_position = Some(position);
        if context
            .input
            .handle_debug_game_mode_switcher_cursor_moved(self.cursor_position, self.surface_size)
        {
            return;
        }
        if context.input.debug_options_screen_is_open() {
            context
                .input
                .handle_debug_options_screen_cursor_moved_with_text_measurer(
                    self.cursor_position,
                    self.surface_size,
                    &self.text_measurements,
                );
            return;
        }
        if context
            .input
            .handle_debug_pause_screen_cursor_moved(self.cursor_position)
        {
            return;
        }
        if context.world.dialog_screen_is_open() {
            context
                .input
                .update_screen_cursor_position(self.cursor_position);
            handle_dialog_cursor_moved_with_measurer(
                context.world,
                self.cursor_position,
                self.surface_size,
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
        if context.input.handle_stats_screen_cursor_moved(
            context.world,
            self.cursor_position,
            self.surface_size,
        ) {
            context.platform.submit(PlatformEffect::StatsScreenInput(
                StatsScreenInput::MouseMove,
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
            self.surface_size,
        );
    }

    fn apply_key(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
        clipboard_text: Option<String>,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        if self.code_of_conduct_overlay.is_visible(context.world) {
            self.request_cursor_capture(false, context.platform);
            return;
        }

        let pause_was_open = context.input.debug_pause_screen_is_open();
        if context
            .input
            .handle_debug_pause_screen_key(physical_key, state)
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
        if context
            .input
            .handle_debug_options_screen_key_with_clipboard(
                physical_key,
                state,
                Some(&mut clipboard),
            )
        {
            flush_platform_effects(&queued_effects, context.platform);
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
                context.item_runtime,
                context.net_counters,
                context.net_commands,
            )
        {
            flush_platform_effects(&queued_effects, context.platform);
            self.apply_debug_requests(context);
            return;
        }
        flush_platform_effects(&queued_effects, context.platform);

        let container_open = context.world.open_container_id().is_some();
        let dialog_open = context.world.dialog_screen_is_open();
        let sign_editor_open = context
            .input
            .sign_editor_is_active_or_pending(context.world);
        let book_open = context.world.current_book().is_some();
        let advancements_open = context.world.advancements_screen_is_open();
        let stats_open = context.world.stats_screen_is_open();
        let debug_options_open = context.input.debug_options_screen_is_open();
        let pause_open = context.input.debug_pause_screen_is_open();
        let command_entry_open = context.input.command_entry_is_active();
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
                self.surface_size,
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
            || debug_options_open
            || pause_open
            || command_entry_open
        {
            self.request_cursor_capture(false, context.platform);
        }
        if stats_open && matches!(state, ElementState::Pressed) {
            context.platform.submit(PlatformEffect::StatsScreenInput(
                StatsScreenInput::KeyboardAction,
            ));
        }
        if !self.cursor_captured
            && !container_open
            && !sign_editor_open
            && !book_open
            && !advancements_open
            && !stats_open
            && !debug_options_open
            && !pause_open
            && !command_entry_open
            && !ordinary_pause_escape
        {
            return;
        }
        handle_key_input_with_item_runtime(
            context.input,
            context.net_counters,
            context.world,
            context.net_commands,
            context.item_runtime,
            physical_key,
            state,
            self.surface_size,
        );
        if command_entry_open
            && !context.input.command_entry_is_active()
            && !simulation_wants_cursor(context.input, context.world)
        {
            self.request_cursor_capture(true, context.platform);
        }
    }

    fn apply_debug_requests(&mut self, context: &mut SimulationInputContext<'_, '_>) {
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
        let profiling_toggle_requests = context.input.take_debug_profiling_toggle_requests();
        if !profiling_toggle_requests.is_empty() {
            let starts = profiling_toggle_requests
                .iter()
                .filter(|request| matches!(request, DebugProfilingToggleRequest::Start))
                .count();
            let stops = profiling_toggle_requests
                .iter()
                .filter(|request| matches!(request, DebugProfilingToggleRequest::Stop))
                .count();
            context.platform.submit(PlatformEffect::ProfilingToggled {
                requests: profiling_toggle_requests.len(),
                starts,
                stops,
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
        let pause_without_menu_requests = context.input.take_debug_pause_without_menu_requests();
        if pause_without_menu_requests > 0 {
            release_active_input(
                context.input,
                context.world,
                context.net_counters,
                context.net_commands,
            );
            context.input.open_debug_pause_screen_without_menu();
            self.request_cursor_capture(false, context.platform);
            tracing::info!(
                pause_without_menu_requests,
                "opened no-menu pause screen requested by debug hotkey"
            );
        }
    }

    fn apply_ime_preedit(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        if self.code_of_conduct_overlay.is_visible(context.world) {
            context.input.clear_dialog_ime_preedit();
            self.request_cursor_capture(false, context.platform);
        } else if context.world.dialog_screen_is_open() {
            self.request_cursor_capture(false, context.platform);
            context.input.handle_dialog_ime_preedit(
                text,
                cursor_range,
                context.world.dialog_screen_revision(),
            );
        } else {
            context.input.clear_dialog_ime_preedit();
        }
    }

    fn apply_ime_commit(&mut self, text: &str, context: &mut SimulationInputContext<'_, '_>) {
        context.input.clear_dialog_ime_preedit();
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
        let command_entry_open = context.input.command_entry_is_active();
        if context.input.debug_options_screen_is_open() {
            self.request_cursor_capture(false, context.platform);
            context.input.handle_debug_options_screen_text_input(text);
            return;
        }
        if context.world.dialog_screen_is_open() {
            self.request_cursor_capture(false, context.platform);
            handle_dialog_text_input_with_measurer(context.world, text, &self.text_measurements);
            return;
        }
        if book_open || advancements_open || stats_open {
            self.request_cursor_capture(false, context.platform);
            return;
        }
        if sign_editor_open || container_open || command_entry_open {
            self.request_cursor_capture(false, context.platform);
        }
        if !self.cursor_captured && !sign_editor_open && !container_open && !command_entry_open {
            return;
        }
        handle_text_input_with_item_runtime(
            context.input,
            context.net_counters,
            context.world,
            context.net_commands,
            context.item_runtime,
            text,
        );
    }

    fn apply_mouse_button(
        &mut self,
        button: MouseButton,
        state: ElementState,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        if self.code_of_conduct_overlay.is_visible(context.world) {
            let overlay_input = self.code_of_conduct_overlay.handle_mouse_input(
                context.world,
                button,
                state,
                self.cursor_position,
                self.surface_size,
            );
            if let Some(request) = overlay_input.request {
                self.apply_control_request(Some(request), None, None, context);
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
                    self.surface_size,
                    context.world.local_player_has_reduced_debug_info(),
                    &self.text_measurements,
                );
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
                self.surface_size,
            );
            for request in context.input.take_pause_screen_link_requests() {
                match request {
                    PauseScreenLinkRequest::SendFeedback { url } => tracing::info!(
                        url,
                        "pause screen send-feedback link requested; native confirm-link screen is not implemented"
                    ),
                    PauseScreenLinkRequest::ReportBugs { url } => tracing::info!(
                        url,
                        "pause screen report-bugs link requested; native confirm-link screen is not implemented"
                    ),
                }
            }
            if context.input.pause_screen_disconnect_requested() {
                request_net_disconnect(context.net_commands, &mut self.net_disconnect_requested);
            }
            if !simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }
        if context.world.stats_screen_is_open() {
            self.request_cursor_capture(false, context.platform);
            context.platform.submit(PlatformEffect::StatsScreenInput(
                StatsScreenInput::MouseAction,
            ));
            context.input.handle_stats_screen_mouse_input(
                context.net_counters,
                context.world,
                context.net_commands,
                button,
                state,
                self.cursor_position,
                self.surface_size,
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
                self.surface_size,
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
        if context.input.command_entry_is_active() {
            self.request_cursor_capture(false, context.platform);
            context.input.handle_command_suggestion_mouse_input(
                context.world,
                button,
                state,
                self.cursor_position,
            );
            return;
        }
        if context
            .input
            .sign_editor_is_active_or_pending(context.world)
        {
            self.request_cursor_capture(false, context.platform);
            return;
        }
        if context.world.current_book().is_some() {
            self.request_cursor_capture(false, context.platform);
            handle_book_screen_mouse_input(
                context.world,
                button,
                state,
                self.cursor_position,
                self.surface_size,
            );
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
                self.surface_size,
            );
            return;
        }
        if context.world.open_container_id().is_some() {
            self.request_cursor_capture(false, context.platform);
            handle_inventory_mouse_input_with_item_runtime(
                context.input,
                context.world,
                context.net_counters,
                context.net_commands,
                context.item_runtime,
                button,
                state,
                self.cursor_position,
                self.surface_size,
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
        handle_mouse_input_at_partial_tick(
            context.input,
            context.world,
            context.net_counters,
            context.net_commands,
            button,
            state,
            context.entity_partial_tick,
        );
    }

    fn apply_mouse_wheel(
        &mut self,
        delta: MouseScrollDelta,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        if self.code_of_conduct_overlay.is_visible(context.world) {
            self.request_cursor_capture(false, context.platform);
            return;
        }
        if context.input.debug_options_screen_is_open() {
            self.request_cursor_capture(false, context.platform);
            context
                .input
                .handle_debug_options_screen_mouse_wheel(delta, self.surface_size);
            return;
        }
        if context.world.dialog_screen_is_open() {
            self.request_cursor_capture(false, context.platform);
            handle_dialog_mouse_wheel(
                context.world,
                delta,
                self.cursor_position,
                self.surface_size,
            );
            return;
        }
        if context.input.command_entry_is_active() {
            self.request_cursor_capture(false, context.platform);
            context.input.handle_command_suggestion_mouse_wheel(delta);
            return;
        }
        if context
            .input
            .sign_editor_is_active_or_pending(context.world)
        {
            self.request_cursor_capture(false, context.platform);
            return;
        }
        if context.world.stats_screen_is_open() {
            self.request_cursor_capture(false, context.platform);
            context.platform.submit(PlatformEffect::StatsScreenInput(
                StatsScreenInput::MouseAction,
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
            handle_inventory_mouse_wheel(
                context.input,
                context.world,
                context.net_counters,
                context.net_commands,
                delta,
                self.cursor_position,
                self.surface_size,
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
        context: &mut SimulationInputContext<'_, '_>,
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
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        match request {
            ControlWriteRequest::Shutdown => self.request_shutdown(context),
            ControlWriteRequest::Screenshot { path } => context
                .platform
                .submit(PlatformEffect::Screenshot(path.into())),
            ControlWriteRequest::CodeOfConduct(request) => {
                self.apply_control_request(Some(request), None, None, context);
            }
            ControlWriteRequest::Net(request) => {
                self.apply_control_request(None, Some(request), None, context);
            }
            ControlWriteRequest::Input(request) => {
                self.apply_control_request(None, None, Some(request), context);
            }
        }
    }

    fn apply_control_request(
        &mut self,
        code_of_conduct: Option<CodeOfConductControlRequest>,
        net: Option<bbb_control::NetControlRequest>,
        input_request: Option<InputControlRequest>,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        // The individual request handlers remain shared with their focused
        // tests while the external producer has already joined this mailbox.
        let requests = SharedControlRequests::default();
        {
            let mut pending = requests
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            pending.code_of_conduct_requests.extend(code_of_conduct);
            pending.net_requests.extend(net);
            pending.input_requests.extend(input_request);
        }
        pump_control_net_requests(
            &requests,
            context.net_commands,
            context.net_counters,
            context.world,
            context.code_of_conduct.as_deref_mut(),
        );
        pump_control_input_requests(
            &requests,
            context.input,
            context.net_counters,
            context.world,
            context.net_commands,
            context.item_runtime,
            self.code_of_conduct_overlay.is_visible(context.world),
            self.surface_size,
            context.entity_partial_tick,
        );
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

    fn request_shutdown(&mut self, context: &mut SimulationInputContext<'_, '_>) {
        if self.shutdown_requested {
            return;
        }
        self.shutdown_requested = true;
        request_net_disconnect(context.net_commands, &mut self.net_disconnect_requested);
        context.platform.submit(PlatformEffect::Exit);
    }
}

pub(crate) struct SimulationInputContext<'a, 'collector> {
    pub(crate) input: &'a mut crate::input::ClientInputState,
    pub(crate) world: &'a mut WorldStore,
    pub(crate) net_counters: &'a mut bbb_control::NetCounters,
    pub(crate) net_commands: &'a Option<mpsc::Sender<NetCommand>>,
    pub(crate) server_resource_packs: &'a mut ServerResourcePackRuntime,
    pub(crate) particle_runtime: &'a mut Option<crate::particle_runtime::NativeParticleRuntime>,
    pub(crate) item_runtime: Option<&'a NativeItemRuntime>,
    pub(crate) code_of_conduct: Option<&'a mut CodeOfConductAcceptance>,
    pub(crate) entity_partial_tick: f32,
    pub(crate) platform: &'a mut PlatformEffectCollector<'collector>,
}

fn apply_resource_reload_ack(
    ack: crate::resource_reload::ResourceReloadAck,
    context: &mut SimulationInputContext<'_, '_>,
) {
    let status = ack.result.as_ref().map(|_| ()).map_err(ToString::to_string);
    if !context
        .server_resource_packs
        .finish_reload(ack.generation, status)
    {
        tracing::warn!(
            generation = ack.generation,
            "ignored stale server resource pack reload acknowledgement"
        );
        return;
    }
    match ack.result {
        Ok(commit) => {
            commit.item_defaults.apply(context.world);
            context
                .world
                .set_default_block_destroy_profiles(commit.block_destroy_profiles);
            context
                .world
                .set_default_block_sound_profiles(commit.block_sound_profiles);
            *context.particle_runtime = Some(commit.particle_runtime);
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
}

fn world_wants_cursor(world: &WorldStore) -> bool {
    world.open_container_id().is_some()
        || world.dialog_screen_is_open()
        || world.current_book().is_some()
        || world.advancements_screen_is_open()
        || world.stats_screen_is_open()
}

fn simulation_wants_cursor(input: &crate::input::ClientInputState, world: &WorldStore) -> bool {
    world_wants_cursor(world)
        || input.sign_editor_is_active_or_pending(world)
        || input.debug_options_screen_is_open()
        || input.debug_pause_screen_is_open()
        || input.debug_game_mode_switcher_is_open()
}

fn collect_dialog_open_uri_effects(world: &mut WorldStore, effects: &RefCell<Vec<PlatformEffect>>) {
    effects.borrow_mut().extend(
        world
            .take_dialog_open_url_requests()
            .into_iter()
            .map(PlatformEffect::OpenUri),
    );
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
