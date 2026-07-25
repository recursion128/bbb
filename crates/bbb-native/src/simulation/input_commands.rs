//! Canonical application of typed winit and control commands.

use std::cell::RefCell;

use bbb_control::{
    CodeOfConductControlRequest, ControlWriteRequest, InputControlRequest, SharedControlRequests,
};
use bbb_item_model::NativeItemRuntime;
use bbb_net::NetCommand;
use bbb_protocol::{packets::CustomClickAction, ComponentClickEvent};
use bbb_renderer::{
    HudBookHitTarget, HudChatHitTarget, HudChatHitTargetKind, HudDeathScreenHitTarget,
    HudFontGlyphMap, HudGameEventHitTarget, HudSocialInteractionsHitTarget,
};
use bbb_world::WorldStore;
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::{KeyCode, PhysicalKey},
};

use super::{
    commands::{SequencedSimulationCommand, SimulationCommand},
    effects::{PlatformEffect, PlatformEffectCollector, ScreenNarrationScheduleInput},
};
use crate::{
    audio_runtime::AudioEventSink,
    code_of_conduct::CodeOfConductAcceptance,
    code_of_conduct_overlay::{CodeOfConductOverlayProjection, CodeOfConductOverlayState},
    input::{
        death_screen::DeathScreenPlatformRequest,
        game_event_screen::GameEventScreenPlatformRequest, handle_advancements_screen_cursor_moved,
        handle_advancements_screen_mouse_input, handle_advancements_screen_mouse_wheel,
        handle_focus_change, handle_inventory_cursor_moved,
        handle_inventory_mouse_input_with_item_runtime, handle_inventory_mouse_wheel,
        handle_key_input_with_item_runtime, handle_mouse_input_at_partial_tick_with_effects,
        handle_mouse_motion, handle_mouse_wheel, handle_text_input_with_item_runtime,
        queue_chat_command, queue_debug_recreate_server_query_request, release_active_input,
        social::SocialInteractionsPlatformRequest, CommandSuggestionTextMeasurer, DebugClipboard,
        DebugFeatureCountRequest, DebugFrustumRequest, DebugOptionsSearchTextMeasurer,
        DebugProfilingToggleRequest, PauseScreenLinkRequest, TerrainReloadRequestSink,
    },
    resource_pack_runtime::ServerResourcePackRuntime,
    runtime::{
        handle_dialog_cursor_moved_with_measurer,
        handle_dialog_key_input_with_modifiers_for_surface_and_measurer,
        handle_dialog_mouse_input_with_modifiers_and_measurer, handle_dialog_mouse_wheel,
        handle_dialog_text_input_with_measurer, pump_control_input_requests_with_effects,
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
    terrain_readiness: Option<(u64, bool)>,
    last_applied_sequence: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SimulationInputPublication {
    pub(crate) surface_size: PhysicalSize<u32>,
    pub(crate) cursor_position: Option<PhysicalPosition<f64>>,
    pub(crate) cursor_captured: bool,
    pub(crate) code_of_conduct_overlay: CodeOfConductOverlayProjection,
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
            terrain_readiness: None,
            last_applied_sequence: 0,
        }
    }

    pub(crate) fn publication(&self) -> SimulationInputPublication {
        SimulationInputPublication {
            surface_size: self.surface_size,
            cursor_position: self.cursor_position,
            cursor_captured: self.cursor_captured,
            code_of_conduct_overlay: self.code_of_conduct_overlay.projection(),
        }
    }

    pub(crate) fn running(&self) -> bool {
        !self.shutdown_requested
    }

    pub(crate) fn surface_size(&self) -> PhysicalSize<u32> {
        self.surface_size
    }

    pub(crate) fn player_section_ready(&self, level_generation: u64) -> bool {
        self.terrain_readiness
            .is_some_and(|(generation, ready)| generation == level_generation && ready)
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

    pub(crate) fn apply_commands(
        &mut self,
        commands: Vec<SequencedSimulationCommand>,
        context: &mut SimulationInputContext<'_, '_, '_>,
    ) {
        for command in commands {
            if self.shutdown_requested {
                break;
            }
            debug_assert!(command.sequence > self.last_applied_sequence);
            self.last_applied_sequence = command.sequence;
            self.apply_command(command.command, context);
        }
        if self.shutdown_requested {
            return;
        }
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
            self.surface_size,
        );
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
                chat_target,
                book_target,
                death_screen_target,
                game_event_target,
                social_interactions_target,
            } => {
                self.apply_mouse_button(
                    button,
                    state,
                    chat_target,
                    book_target,
                    death_screen_target,
                    game_event_target,
                    social_interactions_target,
                    context,
                );
            }
            SimulationCommand::MouseMotion { delta } => self.apply_mouse_motion(delta, context),
            SimulationCommand::MouseWheel { delta } => self.apply_mouse_wheel(delta, context),
            SimulationCommand::CursorMoved {
                position,
                death_screen_target,
                game_event_target,
                social_interactions_target,
            } => {
                self.apply_cursor_moved(
                    position,
                    death_screen_target,
                    game_event_target,
                    social_interactions_target,
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
                    i32::try_from(self.surface_size.height).unwrap_or(i32::MAX),
                );
            }
            SimulationCommand::CursorLeft => {
                self.cursor_position = None;
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
                    .handle_jigsaw_block_editor_cursor_moved(None, self.surface_size);
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
            SimulationCommand::ImeCommit(text) => self.apply_ime_commit(&text, context),
            SimulationCommand::ImeDisabled => {
                context.input.clear_dialog_ime_preedit();
                context.input.clear_sign_editor_ime_preedit();
            }
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
            SimulationCommand::TerrainReadiness {
                level_generation,
                player_section_compiled_and_visible,
            } => {
                self.terrain_readiness =
                    Some((level_generation, player_section_compiled_and_visible));
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

    fn apply_cursor_moved(
        &mut self,
        position: PhysicalPosition<f64>,
        death_screen_target: Option<HudDeathScreenHitTarget>,
        game_event_target: Option<HudGameEventHitTarget>,
        social_interactions_target: Option<HudSocialInteractionsHitTarget>,
        context: &mut SimulationInputContext<'_, '_, '_>,
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
        if context.world.social_interactions_screen().is_some() {
            context
                .input
                .refresh_social_interactions_scroll_bounds(self.surface_size);
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
                self.surface_size,
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
            .handle_jigsaw_block_editor_cursor_moved(self.cursor_position, self.surface_size)
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
        if context.input.handle_game_rules_screen_cursor_moved(
            context.world,
            self.cursor_position,
            self.surface_size,
        ) {
            return;
        }
        if context.input.handle_stats_screen_cursor_moved(
            context.world,
            self.cursor_position,
            self.surface_size,
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
            self.surface_size,
        );
    }

    fn apply_key(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
        clipboard_text: Option<String>,
        context: &mut SimulationInputContext<'_, '_, '_>,
    ) {
        if self.code_of_conduct_overlay.is_visible(context.world) {
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
                self.surface_size,
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
                self.surface_size,
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

        if context.world.social_interactions_screen().is_some() {
            self.request_cursor_capture(false, context.platform);
            context
                .input
                .refresh_social_interactions_scroll_bounds(self.surface_size);
            handle_key_input_with_item_runtime(
                context.input,
                context.net_counters,
                context.world,
                context.net_commands,
                context.item_runtime.as_ref(),
                physical_key,
                state,
                self.surface_size,
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

        let container_open = context.world.open_container_id().is_some();
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
            self.surface_size,
            &self.text_measurements,
            Some(&mut clipboard),
        );
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
        context: &mut SimulationInputContext<'_, '_, '_>,
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
        } else if context
            .input
            .sign_editor_is_active_or_pending(context.world)
        {
            context.input.sync_sign_editor(
                context.world,
                context.net_counters,
                context.net_commands,
            );
            context
                .input
                .handle_sign_editor_ime_preedit(text, cursor_range);
            self.request_cursor_capture(false, context.platform);
        } else {
            context.input.clear_dialog_ime_preedit();
            context.input.clear_sign_editor_ime_preedit();
        }
    }

    fn apply_ime_commit(&mut self, text: &str, context: &mut SimulationInputContext<'_, '_, '_>) {
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
        let social_interactions_open = context.world.social_interactions_screen().is_some();
        let game_rules_open = context.world.game_rules_screen().is_some();
        let command_entry_open = context.input.command_entry_is_active();
        let structure_block_editor_open = context.input.structure_block_editor_is_active();
        let jigsaw_block_editor_open = context.input.jigsaw_block_editor_is_active();
        let test_block_editor_open = context.input.test_block_editor_is_active();
        let test_instance_block_editor_open = context.input.test_instance_block_editor_is_active();
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
    }

    fn apply_mouse_button(
        &mut self,
        button: MouseButton,
        state: ElementState,
        chat_target: Option<HudChatHitTarget>,
        book_target: Option<HudBookHitTarget>,
        death_screen_target: Option<HudDeathScreenHitTarget>,
        game_event_target: Option<HudGameEventHitTarget>,
        social_interactions_target: Option<HudSocialInteractionsHitTarget>,
        context: &mut SimulationInputContext<'_, '_, '_>,
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
        if context.world.social_interactions_screen().is_some() {
            self.request_cursor_capture(false, context.platform);
            context
                .input
                .refresh_social_interactions_scroll_bounds(self.surface_size);
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
                self.surface_size,
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
        if context.input.structure_block_editor_is_active() {
            self.request_cursor_capture(false, context.platform);
            context.input.handle_structure_block_editor_mouse_input(
                context.net_counters,
                context.world,
                context.net_commands,
                button,
                state,
                self.cursor_position,
                self.surface_size,
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
                self.surface_size,
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
                self.surface_size,
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
                self.surface_size,
                &self.text_measurements,
            );
            if !simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(true, context.platform);
            }
            return;
        }
        if context.input.command_entry_is_active() {
            self.request_cursor_capture(false, context.platform);
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
                    self.surface_size,
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
                self.surface_size,
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
                self.surface_size,
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
                self.surface_size,
                &self.text_measurements,
            );
            if !simulation_wants_cursor(context.input, context.world) {
                self.request_cursor_capture(true, context.platform);
            }
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
                context.item_runtime.as_ref(),
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
        handle_mouse_input_at_partial_tick_with_effects(
            context.input,
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
        if !chat_hit_target_is_current(context.world, context.item_runtime.as_ref(), target) {
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
            HudChatHitTargetKind::Message {
                display_sequence,
                target_index,
            } => {
                let Some(event) = crate::runtime::hud_chat_component_click_event(
                    context.world,
                    context.item_runtime.as_ref(),
                    display_sequence,
                    target_index,
                ) else {
                    return false;
                };
                run_component_click_event(
                    event,
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
        match event {
            ComponentClickEvent::ChangePage { page } => {
                let page = usize::try_from(page.saturating_sub(1).max(0)).unwrap_or_default();
                context.world.set_current_book_page(page);
                true
            }
            event => run_component_click_event(
                event,
                context.input,
                context.world,
                context.net_counters,
                context.net_commands,
                context.platform,
            ),
        }
    }

    fn apply_mouse_wheel(
        &mut self,
        delta: MouseScrollDelta,
        context: &mut SimulationInputContext<'_, '_, '_>,
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
        if context.world.death_screen().is_some() || context.world.game_event_screen().is_some() {
            self.request_cursor_capture(false, context.platform);
            return;
        }
        if context.world.social_interactions_screen().is_some() {
            self.request_cursor_capture(false, context.platform);
            context
                .input
                .refresh_social_interactions_scroll_bounds(self.surface_size);
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
                self.surface_size,
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
        context: &mut SimulationInputContext<'_, '_, '_>,
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
        let game_rules_was_open = context.world.game_rules_screen().is_some();
        pump_control_net_requests(
            &requests,
            context.net_commands,
            context.net_counters,
            context.world,
            context.code_of_conduct.as_deref_mut(),
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
        pump_control_input_requests_with_effects(
            &requests,
            context.input,
            context.net_counters,
            context.world,
            context.net_commands,
            context.item_runtime.as_ref(),
            self.code_of_conduct_overlay.is_visible(context.world),
            self.surface_size,
            context.entity_partial_tick,
            context.level_random,
            context.audio_events,
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

    fn request_shutdown(&mut self, context: &mut SimulationInputContext<'_, '_, '_>) {
        self.request_network_terminal_shutdown(context.net_commands, context.platform);
    }
}

pub(crate) struct SimulationInputContext<'a, 'audio, 'collector> {
    pub(crate) input: &'a mut crate::input::ClientInputState,
    pub(crate) world: &'a mut WorldStore,
    pub(crate) net_counters: &'a mut bbb_control::NetCounters,
    pub(crate) net_commands: &'a Option<mpsc::Sender<NetCommand>>,
    pub(crate) server_resource_packs: &'a mut ServerResourcePackRuntime,
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

fn apply_resource_reload_ack(
    ack: crate::resource_reload::ResourceReloadAck,
    context: &mut SimulationInputContext<'_, '_, '_>,
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
        || world.structure_block_editor().is_some()
        || world.jigsaw_block_editor().is_some()
        || world.test_block_editor().is_some()
        || world.test_instance_block_editor().is_some()
}

fn simulation_wants_cursor(input: &crate::input::ClientInputState, world: &WorldStore) -> bool {
    world_wants_cursor(world)
        || input.sign_editor_is_active_or_pending(world)
        || input.debug_options_screen_is_open()
        || input.debug_pause_screen_is_open()
        || input.debug_game_mode_switcher_is_open()
}

fn chat_hit_target_is_current(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    target: HudChatHitTarget,
) -> bool {
    target.level_generation == world.client_level_generation()
        && target.delivery_revision == world.client_chat().delivery_revision
        && target.resource_generation
            == item_runtime.map_or(0, NativeItemRuntime::resource_generation)
        && target.resources_loaded == item_runtime.is_some()
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
    fn terrain_readiness_is_scoped_to_the_published_level_generation() {
        let mut runtime = SimulationInputRuntime::new(PhysicalSize::new(1280, 720), false);
        runtime.terrain_readiness = Some((7, true));

        assert!(runtime.player_section_ready(7));
        assert!(!runtime.player_section_ready(8));

        runtime.terrain_readiness = Some((8, false));
        assert!(!runtime.player_section_ready(8));
    }

    #[test]
    fn network_terminal_shutdown_is_idempotent_and_requests_disconnect() {
        let mut runtime = SimulationInputRuntime::new(PhysicalSize::new(1280, 720), false);
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
        let target = HudChatHitTarget {
            level_generation: world.client_level_generation(),
            delivery_revision: world.client_chat().delivery_revision,
            resource_generation: 0,
            resources_loaded: false,
            kind: HudChatHitTargetKind::Queue,
        };
        assert!(chat_hit_target_is_current(&world, None, target));
        let stale_resource_target = HudChatHitTarget {
            resources_loaded: true,
            ..target
        };
        assert!(!chat_hit_target_is_current(
            &world,
            None,
            stale_resource_target
        ));

        world.push_server_system_chat_message("new delivery");
        assert!(!chat_hit_target_is_current(&world, None, target));
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
}
