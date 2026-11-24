//! Canonical application of typed winit and control commands.

use bbb_client_clock::Instant;
use std::{cell::RefCell, time::Duration};

mod effects;
mod keyboard_and_ime;
mod measurers;
mod pointer;
use crate::input::key_events::{
    collect_death_screen_platform_effects, collect_disconnected_screen_platform_effects,
    collect_game_event_screen_platform_effects, collect_social_interactions_platform_effects,
    recipe_book_screen_is_open,
};
use crate::narrator_runtime::{ScreenNarrationInput, ScreenNarrationTarget};
use effects::*;
use measurers::*;

use crate::control::{
    CodeOfConductControlRequest, ControlCompletion, ControlCompletionSender, ControlWriteRequest,
    InputControlRequest, InputKeyControl, InputMouseButtonControl, LookAtAnchorControl,
    LookAtTargetControl,
};
use bbb_client_net::NetCommand;
use bbb_client_render::item_model::NativeItemRuntime;
use bbb_client_render::{
    HudBookHitTarget, HudBookMode, HudChatHitTarget, HudChatHitTargetKind, HudDeathScreenHitTarget,
    HudDisconnectedScreenHitTarget, HudFontGlyphMap, HudGameEventHitTarget,
    HudSocialInteractionsHitTarget,
};
use bbb_client_world::WorldStore;
use bbb_protocol::{
    packets::{ClientInformation, CustomClickAction},
    ComponentClickEvent,
};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::{KeyCode, PhysicalKey},
};

use super::{
    commands::{DebugProfilingLifecycle, SimulationCommand},
    effects::{DebugProfilingCommand, PlatformEffect, PlatformEffects},
};
use crate::{
    audio_runtime::AudioEventSink,
    code_of_conduct::CodeOfConductAcceptance,
    code_of_conduct_overlay::{CodeOfConductOverlayProjection, CodeOfConductOverlayState},
    input::{
        disconnected_screen::{DisconnectedScreenDetails, DisconnectedScreenTitle},
        flush_local_interaction_sound_actions, handle_focus_change, handle_inventory_cursor_moved,
        handle_inventory_mouse_input_with_item_runtime, handle_inventory_mouse_wheel,
        handle_mouse_input_at_partial_tick_with_effects_and_presentation, handle_mouse_motion,
        handle_mouse_wheel, inventory_screen_layout, queue_debug_recreate_server_query_request,
        queue_lectern_button_click, recipe_book_button_at_position, release_active_input,
        CommandSuggestionTextMeasurer, DebugClipboard, DebugFeatureCountRequest,
        DebugFrustumRequest, DebugProfilingToggleRequest, PauseScreenLinkRequest, Screen,
        TerrainReloadRequestSink,
    },
    resource_pack_runtime::{ResourcePackReloadFinish, ServerResourcePackRuntime},
    runtime::{
        apply_code_of_conduct_request, apply_control_net_request,
        handle_dialog_cursor_moved_with_measurer,
        handle_dialog_mouse_input_with_modifiers_and_measurer, handle_dialog_mouse_wheel,
        request_net_disconnect, DialogTextMeasurer,
    },
};

/// Mutable simulation state owned across input/render ticks.
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
    /// usable surface size (Window.onFramebufferResize).
    pub(crate) minimized: bool,
    cursor_position: Option<PhysicalPosition<f64>>,
    active_mouse_button: Option<MouseButton>,
    cursor_captured: bool,
    /// Set by `input.focus({focused: true, capture_cursor: true})` so synthetic
    /// MCP sessions keep logical gameplay cursor capture across screen closes
    /// even when the OS window is not foregrounded.
    synthetic_cursor_capture: bool,
    /// Process-lifetime receipts for the physical mouse-button path. These
    /// distinguish winit delivery from screen routing and consumption without
    /// exposing mutable input state to the control thread.
    mouse_button_events: u64,
    left_mouse_button_events: u64,
    left_mouse_button_press_events: u64,
    inventory_mouse_button_events: u64,
    inventory_mouse_button_consumed_events: u64,
    inventory_recipe_book_button_hit_events: u64,
    keyboard_events: u64,
    keyboard_press_events: u64,
    key_t_events: u64,
    inventory_keyboard_events: u64,
    inventory_key_t_press_events: u64,
    recipe_book_language_requests: u64,
    recipe_book_language_target_available: bool,
    recipe_book_language_transitions: u64,
    text_measurements: HudFontGlyphMap,
    code_of_conduct_overlay: CodeOfConductOverlayState,
    /// Tracks whether `release_active_input` already ran for the active
    /// cursor-blocking screen so steady screen frames do not re-release input
    /// or spam `PaddleBoat(false, false)`.
    screen_released_gameplay_input: bool,
    net_disconnect_requested: bool,
    pending_local_disconnect_teardown: bool,
    shutdown_requested: bool,
    processed_controls: Vec<ControlCompletionSender>,
    scheduled_releases: Vec<ScheduledControlRelease>,
    scheduled_waits: Vec<ScheduledTickWait>,
    terrain_readiness: Option<(u64, usize, bool)>,
    /// Vanilla `KeyboardHandler.lastPreeditEvent`: stored on every pre-edit
    /// callback so a focus change can hand the open composition to the
    /// gaining element (`Minecraft.onTextInputFocusChange`).
    last_ime_preedit: Option<(String, Option<(usize, usize)>)>,
    ime_preedit_target: ImePreeditTarget,
    /// `TextInputManager.textInputEnabled`: the last state handed to the
    /// window, so a steady screen does not re-request it every step.
    text_input_enabled: bool,
    /// What `Options.buildPlayerInformation` would return. Most values remain
    /// startup configuration; the recipe-book `excitedze` action replaces the
    /// language before `Options.save` calls `broadcastOptions`.
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
    pub(crate) mouse_button_events: u64,
    pub(crate) left_mouse_button_events: u64,
    pub(crate) left_mouse_button_press_events: u64,
    pub(crate) inventory_mouse_button_events: u64,
    pub(crate) inventory_mouse_button_consumed_events: u64,
    pub(crate) inventory_recipe_book_button_hit_events: u64,
    pub(crate) keyboard_events: u64,
    pub(crate) keyboard_press_events: u64,
    pub(crate) key_t_events: u64,
    pub(crate) inventory_keyboard_events: u64,
    pub(crate) inventory_key_t_press_events: u64,
    pub(crate) recipe_book_language_requests: u64,
    pub(crate) recipe_book_language_target_available: bool,
    pub(crate) recipe_book_language_transitions: u64,
    pub(crate) code_of_conduct_overlay: CodeOfConductOverlayProjection,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SimulationInputProfile {
    pub(crate) keyboard: Duration,
    pub(crate) mouse: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AppliedInputLatency {
    pub(crate) sequence: u64,
    pub(crate) enqueued_at: Instant,
    pub(crate) simulation_applied_at: Instant,
    pub(crate) superseded_events: u64,
}

fn sync_command_completion_target(world: &mut WorldStore, entity_partial_tick: f32) {
    let camera_pose =
        crate::camera_pose::camera_pose_from_world_at_partial_tick(world, entity_partial_tick);
    let target = crate::crosshair::crosshair_target_from_camera(world, camera_pose);
    let (looked_at_block, targeted_entity_uuid) = match target {
        Some(crate::crosshair::CrosshairTarget::Block(hit)) => {
            let location = [
                f64::from(hit.pos.x) + hit.cursor[0],
                f64::from(hit.pos.y) + hit.cursor[1],
                f64::from(hit.pos.z) + hit.cursor[2],
            ];
            (Some((hit.pos, location)), None)
        }
        Some(crate::crosshair::CrosshairTarget::Entity(hit)) => (
            None,
            world
                .probe_entity_transform(hit.entity_id)
                .map(|entity| entity.uuid),
        ),
        None => (None, None),
    };
    world.set_command_completion_target(looked_at_block, targeted_entity_uuid);
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
    Chat,
    CommandBlock,
    Dialog,
    SignEditor,
    BookEditor(crate::input::BookInputMode),
    AnvilRename,
    RecipeBookSearch,
    CreativeSearch,
    StructureBlock(usize),
    JigsawBlock(usize),
    TestBlock,
    TestInstanceBlock(usize),
    SocialInteractions,
    GameRulesSearch,
    GameRulesInteger,
    DebugOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScheduledReleaseTarget {
    Key(InputKeyControl),
    MouseButton(InputMouseButtonControl),
}

#[derive(Debug)]
struct ScheduledControlRelease {
    target: ScheduledReleaseTarget,
    remaining_ticks: u32,
    release_on_screen_open: bool,
    completion: Option<ControlCompletionSender>,
}

#[derive(Debug)]
struct ScheduledTickWait {
    remaining_ticks: u32,
    completion: Option<ControlCompletionSender>,
}

impl SimulationInputRuntime {
    pub(crate) fn has_pending_control_completions(&self) -> bool {
        !self.processed_controls.is_empty()
    }

    pub(crate) fn complete_control_requests(&mut self, generation: u64) {
        for completion in self.processed_controls.drain(..) {
            let _ = completion.send(Ok(ControlCompletion::Processed { generation }));
        }
    }

    pub(crate) fn reject_control_requests(&mut self) {
        for completion in self.processed_controls.drain(..) {
            let _ = completion.send(Err("client stopped before publishing the result".into()));
        }
        for scheduled in self.scheduled_releases.drain(..) {
            if let Some(completion) = scheduled.completion {
                let _ = completion.send(Err("client stopped before publishing the result".into()));
            }
        }
        for wait in self.scheduled_waits.drain(..) {
            if let Some(completion) = wait.completion {
                let _ = completion.send(Err("client stopped before publishing the result".into()));
            }
        }
    }

    pub(crate) fn synthetic_cursor_capture(&self) -> bool {
        self.synthetic_cursor_capture
    }

    pub(crate) fn active_mouse_button(&self) -> Option<MouseButton> {
        self.active_mouse_button
    }

    /// Advances active `hold_ticks` auto-releases and `sim.wait_ticks` timers
    /// after client ticks have executed in this step.
    pub(crate) fn advance_scheduled_control_timers(
        &mut self,
        advanced_ticks: u32,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        if self.scheduled_releases.is_empty() && self.scheduled_waits.is_empty() {
            return;
        }
        let interrupted_by_screen = self.shutdown_requested
            || self.code_of_conduct_overlay.is_visible(context.world)
            || context.input.wants_cursor();

        let mut remaining_releases = Vec::with_capacity(self.scheduled_releases.len());
        let mut due_targets = Vec::new();
        for mut release in self.scheduled_releases.drain(..) {
            let interrupted = interrupted_by_screen && release.release_on_screen_open;
            if !interrupted && advanced_ticks > 0 {
                release.remaining_ticks = release.remaining_ticks.saturating_sub(advanced_ticks);
            }
            if interrupted || release.remaining_ticks == 0 {
                due_targets.push(release.target);
                if let Some(completion) = release.completion {
                    self.processed_controls.push(completion);
                }
            } else {
                remaining_releases.push(release);
            }
        }
        self.scheduled_releases = remaining_releases;

        for target in due_targets {
            match target {
                ScheduledReleaseTarget::Key(key) => {
                    let code = crate::runtime::input_key_code(key);
                    self.apply_key(
                        PhysicalKey::Code(code),
                        ElementState::Released,
                        None,
                        context,
                    );
                }
                ScheduledReleaseTarget::MouseButton(button) => {
                    self.apply_control_mouse_button(button, ElementState::Released, context);
                }
            }
        }

        if advanced_ticks > 0 || self.shutdown_requested {
            let mut remaining_waits = Vec::with_capacity(self.scheduled_waits.len());
            for mut wait in self.scheduled_waits.drain(..) {
                wait.remaining_ticks = wait.remaining_ticks.saturating_sub(advanced_ticks);
                if wait.remaining_ticks == 0 || self.shutdown_requested {
                    if let Some(completion) = wait.completion {
                        self.processed_controls.push(completion);
                    }
                } else {
                    remaining_waits.push(wait);
                }
            }
            self.scheduled_waits = remaining_waits;
        }
    }

    fn cancel_scheduled_release(&mut self, target: ScheduledReleaseTarget) {
        let mut kept = Vec::with_capacity(self.scheduled_releases.len());
        for release in self.scheduled_releases.drain(..) {
            if release.target == target {
                if let Some(completion) = release.completion {
                    self.processed_controls.push(completion);
                }
            } else {
                kept.push(release);
            }
        }
        self.scheduled_releases = kept;
    }

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
            active_mouse_button: None,
            cursor_captured,
            synthetic_cursor_capture: false,
            mouse_button_events: 0,
            left_mouse_button_events: 0,
            left_mouse_button_press_events: 0,
            inventory_mouse_button_events: 0,
            inventory_mouse_button_consumed_events: 0,
            inventory_recipe_book_button_hit_events: 0,
            keyboard_events: 0,
            keyboard_press_events: 0,
            key_t_events: 0,
            inventory_keyboard_events: 0,
            inventory_key_t_press_events: 0,
            recipe_book_language_requests: 0,
            recipe_book_language_target_available: false,
            recipe_book_language_transitions: 0,
            text_measurements: HudFontGlyphMap::default(),
            code_of_conduct_overlay: CodeOfConductOverlayState::default(),
            screen_released_gameplay_input: false,
            net_disconnect_requested: false,
            pending_local_disconnect_teardown: false,
            shutdown_requested: false,
            processed_controls: Vec::new(),
            scheduled_releases: Vec::new(),
            scheduled_waits: Vec::new(),
            terrain_readiness: None,
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

    /// The window owner's grab result for the last capture request.
    pub(crate) fn set_cursor_captured(&mut self, captured: bool) {
        self.cursor_captured = captured;
    }

    pub(crate) fn reconcile_screen_cursor(
        &mut self,
        screen_was_open: bool,
        input: &crate::input::ClientInputState,
        world: &WorldStore,
        counters: &crate::control::NetCounters,
        platform: &mut PlatformEffects,
    ) {
        // Minecraft.setScreen(null) grabs only after returning to an active game.
        if screen_was_open
            && input.current_screen().is_none()
            && !world.level_load().screen_is_up()
            && input.is_focused()
            && counters.state.as_deref() == Some("Play")
            && world.local_player_id().is_some()
            && !self.code_of_conduct_overlay.is_visible(world)
            && !self.net_disconnect_requested
            && !self.shutdown_requested
        {
            self.request_cursor_capture(true, platform);
        }
    }

    /// The logical GUI viewport and scale the renderer lays out with.
    pub(crate) fn gui_viewport(&self) -> (PhysicalSize<u32>, u32) {
        (self.gui_surface_size, self.gui_scale)
    }

    /// The viewport a queued `SurfaceResized { size }` will project, so the
    /// renderer and later input projection use it before the queue applies.
    pub(crate) fn gui_viewport_for(&self, size: PhysicalSize<u32>) -> (PhysicalSize<u32>, u32) {
        let (gui_scale, width, height) =
            project_gui_scale(size, self.gui_scale_max, self.enforce_unicode);
        (PhysicalSize::new(width, height), gui_scale)
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
            mouse_button_events: self.mouse_button_events,
            left_mouse_button_events: self.left_mouse_button_events,
            left_mouse_button_press_events: self.left_mouse_button_press_events,
            inventory_mouse_button_events: self.inventory_mouse_button_events,
            inventory_mouse_button_consumed_events: self.inventory_mouse_button_consumed_events,
            inventory_recipe_book_button_hit_events: self.inventory_recipe_book_button_hit_events,
            keyboard_events: self.keyboard_events,
            keyboard_press_events: self.keyboard_press_events,
            key_t_events: self.key_t_events,
            inventory_keyboard_events: self.inventory_keyboard_events,
            inventory_key_t_press_events: self.inventory_key_t_press_events,
            recipe_book_language_requests: self.recipe_book_language_requests,
            recipe_book_language_target_available: self.recipe_book_language_target_available,
            recipe_book_language_transitions: self.recipe_book_language_transitions,
            code_of_conduct_overlay: self.code_of_conduct_overlay.projection(),
        }
    }

    pub(crate) fn running(&self) -> bool {
        !self.shutdown_requested
    }

    pub(crate) fn gui_surface_size(&self) -> PhysicalSize<u32> {
        self.gui_surface_size
    }

    pub(crate) fn client_information(&self) -> &ClientInformation {
        &self.client_information
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
        platform: &mut PlatformEffects,
    ) {
        if self.shutdown_requested {
            return;
        }
        self.shutdown_requested = true;
        request_net_disconnect(net_commands, &mut self.net_disconnect_requested);
        platform.submit(PlatformEffect::Exit);
    }

    pub(crate) fn request_terminal_screen_cursor(&mut self, platform: &mut PlatformEffects) {
        self.request_cursor_capture(false, platform);
    }

    pub(crate) fn request_local_disconnect(
        &mut self,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        request_net_disconnect(net_commands, &mut self.net_disconnect_requested);
        if !self.shutdown_requested {
            self.pending_local_disconnect_teardown = true;
        }
    }

    pub(crate) fn net_disconnect_requested(&self) -> bool {
        self.net_disconnect_requested
    }

    pub(crate) fn take_pending_local_disconnect_teardown(&mut self) -> bool {
        std::mem::take(&mut self.pending_local_disconnect_teardown)
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
    pub(crate) fn reset_net_disconnect_requested(&mut self) {
        self.net_disconnect_requested = false;
        self.pending_local_disconnect_teardown = false;
    }

    pub(crate) fn apply_commands(
        &mut self,
        commands: Vec<SimulationCommand>,
        context: &mut SimulationInputContext<'_, '_>,
        profiling_enabled: bool,
    ) -> SimulationInputProfile {
        crate::runtime::flush_command_confirmation_narrations(
            context.input,
            context.item_runtime.as_ref(),
            context.platform,
        );
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
            let (command, mut completion) = match command {
                SimulationCommand::WithCompletion {
                    command,
                    completion,
                } => (*command, Some(completion)),
                command => (command, None),
            };
            let profile_kind = simulation_command_profile_kind(&command);
            let profile_started_at = profiling_enabled.then(Instant::now);
            context
                .input
                .set_chat_input_surface_size(self.gui_surface_size());
            let screen_was_open = context.input.current_screen().is_some()
                || context.world.level_load().screen_is_up()
                || self.code_of_conduct_overlay.is_visible(context.world);
            self.apply_command(command, context, &mut completion);
            if context
                .input
                .pause_screen_disconnect_requested_or_local_disconnect()
            {
                self.net_disconnect_requested = true;
                if !self.shutdown_requested {
                    self.pending_local_disconnect_teardown = true;
                }
            }
            context.input.apply_screen_requests(
                context.world,
                context.net_counters,
                context.net_commands,
            );
            self.reconcile_screen_cursor(
                screen_was_open,
                context.input,
                context.world,
                context.net_counters,
                context.platform,
            );
            self.flush_narrator_requests(context);
            crate::runtime::flush_command_confirmation_narrations(
                context.input,
                context.item_runtime.as_ref(),
                context.platform,
            );
            if let Some(completion) = completion {
                self.processed_controls.push(completion);
            }
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
        // `Minecraft.pauseIfInactive` runs after
        // every command for the step is applied: a `FocusChanged` arriving in
        // the same step must update `input.focused` first, so a focus-regained
        // window never spuriously opens the pause screen. The 500 ms delay is
        // strict (`> 500L`) and an already-open screen suppresses reopening.
        context.input.pause_if_inactive(
            context.now,
            context.world,
            context.net_counters,
            context.net_commands,
        );
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
        if context.input.command_entry_is_active() {
            sync_command_completion_target(context.world, context.entity_partial_tick);
        }
        let _ = context.input.command_suggestion_hud_state(
            context.world,
            &self.text_measurements,
            self.gui_surface_size(),
        );
        crate::runtime::flush_command_confirmation_narrations(
            context.input,
            context.item_runtime.as_ref(),
            context.platform,
        );
        for text in context.input.take_command_confirmation_clipboard() {
            context
                .platform
                .submit(PlatformEffect::ClipboardWrite(text));
        }
        flush_platform_effects(&queued_effects, context.platform);
        let wants_cursor_now = self.code_of_conduct_overlay.is_visible(context.world)
            || context.world.level_load().screen_is_up()
            || context.input.wants_cursor();
        if wants_cursor_now {
            if context.input.dialog_screen_is_open() {
                context.input.apply_screen_requests(
                    context.world,
                    context.net_counters,
                    context.net_commands,
                );
            } else if !context.input.chat_restrictions_screen_is_open() {
                context.input.interrupt_chat_screen();
            }
            self.request_cursor_capture(false, context.platform);
            if !self.screen_released_gameplay_input || context.input.has_active_gameplay_input() {
                release_active_input(
                    context.input,
                    context.world,
                    context.net_counters,
                    context.net_commands,
                );
            }
            self.screen_released_gameplay_input = true;
        } else {
            self.screen_released_gameplay_input = false;
            if context.input.command_entry_is_active() {
                self.request_cursor_capture(false, context.platform);
            }
        }
        profile
    }

    fn flush_narrator_requests(&mut self, context: &mut SimulationInputContext<'_, '_>) {
        let narrator_status_requests = context.input.take_narrator_status_requests();
        if !narrator_status_requests.is_empty() {
            let writes = context.input.take_debug_client_option_writes();
            if !writes.is_empty() {
                context.platform.submit(PlatformEffect::SaveClientOptions {
                    writes,
                    broadcast: self.client_information.clone(),
                });
            }
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
    }

    fn apply_command(
        &mut self,
        command: SimulationCommand,
        context: &mut SimulationInputContext<'_, '_>,
        completion: &mut Option<ControlCompletionSender>,
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
                command_confirmation_target,
                death_screen_target,
                game_event_target,
                social_interactions_target,
            } => {
                self.apply_mouse_button(
                    button,
                    state,
                    position,
                    pointer::HudMouseHitTargets {
                        chat: chat_target,
                        book: book_target,
                        disconnected_screen: disconnected_screen_target,
                        command_confirmation: command_confirmation_target,
                        death_screen: death_screen_target,
                        game_event: game_event_target,
                        social_interactions: social_interactions_target,
                    },
                    context,
                );
            }
            SimulationCommand::MouseMotion { delta } => self.apply_mouse_motion(delta, context),
            SimulationCommand::MouseWheel { delta, position } => {
                self.cursor_position = position;
                self.apply_mouse_wheel(delta, context);
            }
            SimulationCommand::CursorMoved {
                position,
                disconnected_screen_target,
                death_screen_target,
                game_event_target,
                social_interactions_target,
            } => {
                self.apply_cursor_moved(
                    position,
                    disconnected_screen_target,
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
                context.input.update_screen_cursor_position(None);
                match context.input.current_screen() {
                    Some(Screen::Disconnected(_)) => {
                        context.input.handle_disconnected_screen_hover(None);
                    }
                    Some(Screen::Death(_)) => {
                        context.input.handle_death_screen_hover(None);
                    }
                    Some(Screen::GameEvent(_)) => {
                        context.input.handle_game_event_screen_hover(None);
                    }
                    Some(Screen::Social(_)) => {
                        context
                            .input
                            .handle_social_interactions_hover(context.world, None);
                    }
                    Some(Screen::Sign(_)) => {
                        context.input.handle_sign_editor_cursor_moved(None);
                    }
                    Some(Screen::StructureBlock(_)) => {
                        context
                            .input
                            .handle_structure_block_editor_cursor_moved(None);
                    }
                    Some(Screen::JigsawBlock(_)) => {
                        context
                            .input
                            .handle_jigsaw_block_editor_cursor_moved(None, self.gui_surface_size());
                    }
                    Some(Screen::TestBlock(_)) => {
                        context.input.handle_test_block_editor_cursor_moved(None);
                    }
                    Some(Screen::TestInstanceBlock(_)) => {
                        context
                            .input
                            .handle_test_instance_block_editor_cursor_moved(None);
                    }
                    Some(Screen::CommandEntry(_)) => {
                        context
                            .input
                            .handle_command_suggestion_cursor_moved(context.world, None);
                    }
                    _ => {}
                }
            }
            SimulationCommand::ImePreedit { text, cursor_range } => {
                self.apply_ime_preedit(&text, cursor_range, context);
            }
            SimulationCommand::ImeCommit(text) => {
                self.apply_ime_commit(&text, context);
            }
            SimulationCommand::ImeDisabled => {
                self.last_ime_preedit = None;
                context.input.clear_all_ime_preedits();
            }
            SimulationCommand::SurfaceResized { size } => {
                let resized = self.set_surface_size(size);
                self.apply_surface_resized(resized, context);
            }
            SimulationCommand::TextMeasurements(measurements) => {
                self.text_measurements = measurements;
                self.code_of_conduct_overlay.sync(
                    context.world,
                    self.gui_surface_size(),
                    &self.text_measurements,
                );
            }
            SimulationCommand::HudChatScrollPage(page) => {
                context.input.set_chat_scroll_page(page);
            }
            SimulationCommand::HudChatAppendedLines(appended) => {
                context.input.follow_chat_appended_lines(&appended);
            }
            SimulationCommand::ScreenshotSaved { name, error } => {
                crate::input::ClientInputState::push_screenshot_saved_message(
                    Some(context.world),
                    &name,
                    error.as_deref(),
                );
            }
            SimulationCommand::RecordingNotice(message) => {
                context.world.push_client_system_chat_message(message);
            }
            SimulationCommand::ClientOptionsSaveCompleted(information) => {
                apply_client_options_save_completed(
                    information,
                    context.net_counters,
                    context.net_commands,
                );
            }
            SimulationCommand::ResourceReloadAck(ack) => {
                match apply_resource_reload_ack(*ack, context) {
                    ResourceReloadAckEffect::None => {}
                    ResourceReloadAckEffect::RebuildScreen => {
                        context.input.rebuild_chat_input(
                            false,
                            self.gui_surface_size(),
                            &self.text_measurements,
                        );
                        context.input.refresh_chat_command_suggestions(
                            context.net_counters,
                            context.world,
                            context.net_commands,
                        );
                    }
                    ResourceReloadAckEffect::ShowLoadFailureToast => {
                        context.input.rebuild_chat_input(
                            false,
                            self.gui_surface_size(),
                            &self.text_measurements,
                        );
                        context.input.refresh_chat_command_suggestions(
                            context.net_counters,
                            context.world,
                            context.net_commands,
                        );
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
                                reason: bbb_client_net::DisconnectReason::from_plain(format!(
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
            SimulationCommand::Control(request) => self.apply_control(request, context, completion),
            SimulationCommand::WithCompletion { .. } => {
                unreachable!("completion is unwrapped before applying input")
            }
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

    /// The relayout half of a framebuffer resize whose projection the window
    /// owner already applied with [`Self::set_surface_size`].
    fn apply_surface_resized(
        &mut self,
        resized: bool,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        if resized {
            context.input.rebuild_chat_input(
                true,
                self.gui_surface_size(),
                &self.text_measurements,
            );
            context.input.refresh_chat_command_suggestions(
                context.net_counters,
                context.world,
                context.net_commands,
            );
        }
        self.code_of_conduct_overlay.sync(
            context.world,
            self.gui_surface_size,
            &self.text_measurements,
        );
    }

    /// `Window.onFramebufferResize` plus
    /// `Minecraft.resizeGui`: zero dimensions mean
    /// minimized and keep the last usable framebuffer; nonzero sizes replace it
    /// and recompute the GUI scale and projection. Returns whether the usable
    /// surface changed.
    pub(crate) fn set_surface_size(&mut self, size: PhysicalSize<u32>) -> bool {
        if size.width == 0 || size.height == 0 {
            self.minimized = true;
            return false;
        }
        self.minimized = false;
        let resized = size != self.surface_size;
        self.surface_size = size;
        let (gui_scale, gui_scaled_width, gui_scaled_height) =
            project_gui_scale(size, self.gui_scale_max, self.enforce_unicode);
        self.gui_scale = gui_scale;
        self.gui_scaled_width = gui_scaled_width;
        self.gui_scaled_height = gui_scaled_height;
        self.gui_surface_size = PhysicalSize::new(gui_scaled_width, gui_scaled_height);
        resized
    }

    fn apply_cursor_moved(
        &mut self,
        position: PhysicalPosition<f64>,
        disconnected_screen_target: Option<HudDisconnectedScreenHitTarget>,
        death_screen_target: Option<HudDeathScreenHitTarget>,
        game_event_target: Option<HudGameEventHitTarget>,
        social_interactions_target: Option<HudSocialInteractionsHitTarget>,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        let previous_cursor_position = self.cursor_position;
        self.cursor_position = Some(position);
        if context.input.chat_entry_is_active() {
            context
                .platform
                .submit(PlatformEffect::ScreenNarrationInput {
                    target: ScreenNarrationTarget::Chat,
                    input: ScreenNarrationInput::MouseMove,
                });
        }
        match context.input.current_screen() {
            Some(Screen::CommandConfirmation(_)) => {
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::CommandConfirmation,
                        input: ScreenNarrationInput::MouseMove,
                    });
                self.request_cursor_capture(false, context.platform);
            }
            Some(Screen::Disconnected(_)) => {
                context
                    .input
                    .handle_disconnected_screen_hover(disconnected_screen_target);
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::Disconnected,
                        input: ScreenNarrationInput::MouseMove,
                    });
                self.request_cursor_capture(false, context.platform);
            }
            Some(Screen::ChatRestrictions(_)) => {
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::Chat,
                        input: ScreenNarrationInput::MouseMove,
                    });
                self.request_cursor_capture(false, context.platform);
            }
            _ if self.code_of_conduct_overlay.handle_cursor_moved(
                context.world,
                previous_cursor_position,
                self.cursor_position,
                self.gui_surface_size(),
                &self.text_measurements,
            ) =>
            {
                self.request_cursor_capture(false, context.platform);
            }
            Some(Screen::GameModeSwitcher(_)) => {
                context.input.handle_debug_game_mode_switcher_cursor_moved(
                    self.cursor_position,
                    self.gui_surface_size(),
                );
            }
            Some(Screen::DebugOptions(_)) => {
                context
                    .input
                    .handle_debug_options_screen_cursor_moved_with_text_measurer(
                        self.cursor_position,
                        self.gui_surface_size(),
                        &self.text_measurements,
                    );
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::DebugOptions,
                        input: ScreenNarrationInput::MouseMove,
                    });
            }
            Some(Screen::Pause(_)) => {
                context
                    .input
                    .handle_debug_pause_screen_cursor_moved(self.cursor_position);
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::Pause,
                        input: ScreenNarrationInput::MouseMove,
                    });
            }
            Some(Screen::Death(_)) => {
                context.input.handle_death_screen_hover(death_screen_target);
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::Death,
                        input: ScreenNarrationInput::MouseMove,
                    });
                context
                    .input
                    .update_screen_cursor_position(self.cursor_position);
            }
            Some(Screen::GameEvent(_)) => {
                context
                    .input
                    .handle_game_event_screen_hover(game_event_target);
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::GameEvent,
                        input: ScreenNarrationInput::MouseMove,
                    });
                context
                    .input
                    .update_screen_cursor_position(self.cursor_position);
            }
            Some(Screen::Social(_)) => {
                context
                    .input
                    .refresh_social_interactions_scroll_bounds(self.gui_surface_size());
                context
                    .input
                    .handle_social_interactions_hover(context.world, social_interactions_target);
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::SocialInteractions,
                        input: ScreenNarrationInput::MouseMove,
                    });
                context
                    .input
                    .update_screen_cursor_position(self.cursor_position);
            }
            Some(
                Screen::Dialog(_)
                | Screen::DialogWarning(_)
                | Screen::DialogLink(_)
                | Screen::DialogWaiting(_),
            ) => {
                context
                    .input
                    .update_screen_cursor_position(self.cursor_position);
                if self.active_mouse_button == Some(MouseButton::Left) {
                    handle_dialog_cursor_moved_with_measurer(
                        context.input,
                        context.world,
                        self.cursor_position,
                        self.gui_surface_size(),
                        &self.text_measurements,
                    );
                }
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::Dialog,
                        input: ScreenNarrationInput::MouseMove,
                    });
            }
            Some(Screen::StructureBlock(_)) => {
                context
                    .input
                    .handle_structure_block_editor_cursor_moved(self.cursor_position);
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::StructureBlock,
                        input: ScreenNarrationInput::MouseMove,
                    });
            }
            Some(Screen::JigsawBlock(_)) => {
                context.input.handle_jigsaw_block_editor_cursor_moved(
                    self.cursor_position,
                    self.gui_surface_size(),
                );
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::JigsawBlock,
                        input: ScreenNarrationInput::MouseMove,
                    });
            }
            Some(Screen::TestBlock(_)) => {
                context
                    .input
                    .handle_test_block_editor_cursor_moved(self.cursor_position);
            }
            Some(Screen::TestInstanceBlock(_)) => {
                context
                    .input
                    .handle_test_instance_block_editor_cursor_moved(self.cursor_position);
            }
            Some(Screen::Sign(_)) => {
                context
                    .input
                    .handle_sign_editor_cursor_moved(self.cursor_position);
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::Sign,
                        input: ScreenNarrationInput::MouseMove,
                    });
            }
            Some(Screen::Book(_)) => {
                context
                    .input
                    .update_screen_cursor_position(self.cursor_position);
                context.input.handle_book_screen_cursor_moved(
                    self.cursor_position,
                    self.gui_surface_size(),
                    &self.text_measurements,
                );
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::Book,
                        input: ScreenNarrationInput::MouseMove,
                    });
            }
            Some(Screen::CommandEntry(_)) => {
                context.input.handle_chat_input_cursor_moved(
                    self.cursor_position,
                    self.gui_surface_size(),
                    &self.text_measurements,
                );
                if context.input.command_block_editor_hud_state().is_some() {
                    context
                        .platform
                        .submit(PlatformEffect::ScreenNarrationInput {
                            target: ScreenNarrationTarget::CommandBlock,
                            input: ScreenNarrationInput::MouseMove,
                        });
                }
                if context
                    .input
                    .handle_command_suggestion_cursor_moved(context.world, self.cursor_position)
                {
                }
            }
            Some(Screen::GameRules(_)) => {
                context.input.handle_game_rules_screen_cursor_moved(
                    self.cursor_position,
                    self.gui_surface_size(),
                );
            }
            Some(Screen::Stats(_)) => {
                context.input.handle_stats_screen_cursor_moved(
                    self.cursor_position,
                    self.gui_surface_size(),
                );
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::Stats,
                        input: ScreenNarrationInput::MouseMove,
                    });
            }
            Some(Screen::Advancements(_)) => {
                context.input.handle_advancements_screen_cursor_moved(
                    context.world,
                    previous_cursor_position,
                    self.cursor_position,
                );
            }
            Some(Screen::Container(_) | Screen::Creative(_)) => {
                handle_inventory_cursor_moved(
                    context.input,
                    context.world,
                    context.net_counters,
                    context.net_commands,
                    self.cursor_position,
                    self.gui_surface_size(),
                );
                let target = if matches!(context.input.current_screen(), Some(Screen::Container(_)))
                    && context.world.lectern_book_view().is_some()
                {
                    ScreenNarrationTarget::Book
                } else if recipe_book_screen_is_open(context.input, context.world) {
                    ScreenNarrationTarget::RecipeBook
                } else {
                    ScreenNarrationTarget::Inventory
                };
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target,
                        input: ScreenNarrationInput::MouseMove,
                    });
            }
            None => {}
        }
    }

    fn apply_control(
        &mut self,
        request: ControlWriteRequest,
        context: &mut SimulationInputContext<'_, '_>,
        completion: &mut Option<ControlCompletionSender>,
    ) {
        match request {
            ControlWriteRequest::Shutdown => self.request_shutdown(context),
            ControlWriteRequest::LockstepWake => {}
            ControlWriteRequest::WaitTicks { ticks } => {
                self.scheduled_waits.push(ScheduledTickWait {
                    remaining_ticks: ticks,
                    completion: completion.take(),
                });
            }
            ControlWriteRequest::Recording(request) => {
                context.platform.submit(PlatformEffect::Recording {
                    request,
                    completion: completion.take(),
                });
            }
            ControlWriteRequest::Screenshot { path } => {
                context.platform.submit(PlatformEffect::Screenshot {
                    path: path.into(),
                    completion: completion.take(),
                })
            }
            ControlWriteRequest::CodeOfConduct(request) => {
                self.apply_code_of_conduct_control(request, context);
            }
            ControlWriteRequest::Net(request) => {
                self.apply_net_control(request, context);
            }
            ControlWriteRequest::Input(request) => {
                self.apply_input_control(request, context, completion);
            }
        }
    }

    fn apply_code_of_conduct_control(
        &mut self,
        request: CodeOfConductControlRequest,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        let is_decline = matches!(request, CodeOfConductControlRequest::Decline);
        apply_code_of_conduct_request(
            request,
            context.net_commands,
            context.world,
            context.code_of_conduct.as_deref_mut(),
        );
        if is_decline {
            self.net_disconnect_requested = true;
            if !self.shutdown_requested {
                self.pending_local_disconnect_teardown = true;
            }
        }
    }

    fn apply_net_control(
        &mut self,
        request: crate::control::NetControlRequest,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        if matches!(
            request,
            crate::control::NetControlRequest::RequestGameRuleValues
        ) {
            if context.input.open_game_rules_screen(
                context.world,
                context.net_counters,
                context.net_commands,
            ) {
                self.request_cursor_capture(false, context.platform);
            }
            return;
        }
        apply_control_net_request(
            request,
            context.net_commands,
            context.net_counters,
            context.world,
        );
    }

    fn apply_input_control(
        &mut self,
        request: InputControlRequest,
        context: &mut SimulationInputContext<'_, '_>,
        completion: &mut Option<ControlCompletionSender>,
    ) {
        // Keys and buttons take the window-event path, so control input is
        // exactly what a player at the keyboard and mouse would produce.
        let element_state = |pressed| {
            if pressed {
                ElementState::Pressed
            } else {
                ElementState::Released
            }
        };
        match request {
            InputControlRequest::Key { key, pressed } => {
                if !pressed {
                    self.cancel_scheduled_release(ScheduledReleaseTarget::Key(key));
                }
                let code = crate::runtime::input_key_code(key);
                self.apply_key(
                    PhysicalKey::Code(code),
                    element_state(pressed),
                    None,
                    context,
                );
                return;
            }
            InputControlRequest::KeyHold { key, hold_ticks } => {
                self.cancel_scheduled_release(ScheduledReleaseTarget::Key(key));
                let release_on_screen_open = !context.input.wants_cursor();
                let code = crate::runtime::input_key_code(key);
                self.apply_key(
                    PhysicalKey::Code(code),
                    ElementState::Pressed,
                    None,
                    context,
                );
                let hold_completion = if bbb_client_clock::is_virtual() {
                    None
                } else {
                    completion.take()
                };
                self.scheduled_releases.push(ScheduledControlRelease {
                    target: ScheduledReleaseTarget::Key(key),
                    remaining_ticks: hold_ticks,
                    release_on_screen_open,
                    completion: hold_completion,
                });
                return;
            }
            InputControlRequest::MouseButton { button, pressed } => {
                if !pressed {
                    self.cancel_scheduled_release(ScheduledReleaseTarget::MouseButton(button));
                }
                self.apply_control_mouse_button(button, element_state(pressed), context);
                return;
            }
            InputControlRequest::MouseButtonHold { button, hold_ticks } => {
                self.cancel_scheduled_release(ScheduledReleaseTarget::MouseButton(button));
                let release_on_screen_open = !context.input.wants_cursor();
                self.apply_control_mouse_button(button, ElementState::Pressed, context);
                let hold_completion = if bbb_client_clock::is_virtual() {
                    None
                } else {
                    completion.take()
                };
                self.scheduled_releases.push(ScheduledControlRelease {
                    target: ScheduledReleaseTarget::MouseButton(button),
                    remaining_ticks: hold_ticks,
                    release_on_screen_open,
                    completion: hold_completion,
                });
                return;
            }
            InputControlRequest::MouseTap { button } => {
                self.cancel_scheduled_release(ScheduledReleaseTarget::MouseButton(button));
                self.apply_control_mouse_button(button, ElementState::Pressed, context);
                self.apply_control_mouse_button(button, ElementState::Released, context);
                return;
            }
            _ => {}
        }
        let narrates_chat = matches!(&request, InputControlRequest::Text { .. });
        let chat_screen_was_open = context.input.chat_entry_is_active()
            || context.input.chat_restrictions_screen_is_open();
        context
            .input
            .set_chat_input_surface_size(self.gui_surface_size());
        if self.code_of_conduct_overlay.is_visible(context.world) {
            if matches!(
                request,
                InputControlRequest::Look { .. } | InputControlRequest::LookAt { .. }
            ) {
                if let Some(sender) = completion.take() {
                    let _ = sender.send(Err(
                        "cannot turn camera while code of conduct overlay is visible".to_string(),
                    ));
                }
            }
        } else {
            match request {
                InputControlRequest::Focus { focused } => {
                    handle_focus_change(
                        context.input,
                        context.world,
                        context.net_counters,
                        context.net_commands,
                        focused,
                    );
                    if !focused {
                        self.synthetic_cursor_capture = false;
                        self.request_cursor_capture(false, context.platform);
                    }
                }
                InputControlRequest::FocusCapture {
                    focused,
                    capture_cursor,
                } => {
                    handle_focus_change(
                        context.input,
                        context.world,
                        context.net_counters,
                        context.net_commands,
                        focused,
                    );
                    if focused
                        && capture_cursor
                        && !context.input.wants_cursor()
                        && !context.input.command_entry_is_active()
                    {
                        self.synthetic_cursor_capture = true;
                        self.cursor_captured = true;
                    } else if !focused || !capture_cursor {
                        self.synthetic_cursor_capture = false;
                        self.request_cursor_capture(false, context.platform);
                    }
                }
                InputControlRequest::Text { text } => {
                    crate::input::handle_text_input_with_item_runtime(
                        context.input,
                        context.net_counters,
                        context.world,
                        context.net_commands,
                        context.item_runtime.as_ref(),
                        &text,
                        &self.text_measurements,
                    );
                }
                InputControlRequest::LookDelta { dx, dy } => {
                    if !context.input.wants_cursor() && !context.input.command_entry_is_active() {
                        handle_mouse_motion(context.input, (dx, dy));
                    }
                }
                InputControlRequest::Look { yaw, pitch } => {
                    if !context.input.is_focused()
                        || !self.cursor_captured
                        || context.input.wants_cursor()
                        || context.input.command_entry_is_active()
                    {
                        if let Some(sender) = completion.take() {
                            let _ = sender.send(Err(
                                "cannot turn camera while unfocused, cursor uncaptured, or a screen is open"
                                    .to_string(),
                            ));
                        }
                    } else if context
                        .world
                        .set_local_player_look_angles(yaw, pitch)
                        .is_none()
                    {
                        if let Some(sender) = completion.take() {
                            let _ = sender
                                .send(Err("local player pose is not initialized yet".to_string()));
                        }
                    } else {
                        context.input.clear_accumulated_mouse_turn();
                    }
                }
                InputControlRequest::LookAt { target } => {
                    if !context.input.is_focused()
                        || !self.cursor_captured
                        || context.input.wants_cursor()
                        || context.input.command_entry_is_active()
                    {
                        if let Some(sender) = completion.take() {
                            let _ = sender.send(Err(
                                "cannot turn camera while unfocused, cursor uncaptured, or a screen is open"
                                    .to_string(),
                            ));
                        }
                    } else {
                        let resolved = match target {
                            LookAtTargetControl::Position { x, y, z } => {
                                Some(bbb_protocol::packets::Vec3d { x, y, z })
                            }
                            LookAtTargetControl::Entity { entity_id, anchor } => match anchor {
                                LookAtAnchorControl::Center => {
                                    context.world.entity_look_center_position(entity_id)
                                }
                                LookAtAnchorControl::Eyes => {
                                    context.world.entity_look_target_position(
                                        entity_id,
                                        bbb_protocol::packets::EntityAnchor::Eyes,
                                    )
                                }
                                LookAtAnchorControl::Feet => {
                                    context.world.entity_look_target_position(
                                        entity_id,
                                        bbb_protocol::packets::EntityAnchor::Feet,
                                    )
                                }
                            },
                        };
                        match resolved.and_then(|pos| {
                            let (yaw, pitch) = context.world.look_angles_toward(pos)?;
                            context.world.set_local_player_look_angles(yaw, pitch)
                        }) {
                            Some(_) => context.input.clear_accumulated_mouse_turn(),
                            None => {
                                if let Some(sender) = completion.take() {
                                    let message = match target {
                                        LookAtTargetControl::Entity { entity_id, .. } => format!(
                                            "target entity {entity_id} is not loaded or local player pose is unavailable"
                                        ),
                                        LookAtTargetControl::Position { .. } => {
                                            "local player pose is not initialized yet".to_string()
                                        }
                                    };
                                    let _ = sender.send(Err(message));
                                }
                            }
                        }
                    }
                }
                // Pointer requests are resolved against the rendered frame;
                // keys and buttons have already returned through their owners.
                InputControlRequest::Cursor { .. }
                | InputControlRequest::Click { .. }
                | InputControlRequest::Scroll { .. }
                | InputControlRequest::Key { .. }
                | InputControlRequest::KeyHold { .. }
                | InputControlRequest::MouseButton { .. }
                | InputControlRequest::MouseButtonHold { .. }
                | InputControlRequest::MouseTap { .. } => {}
            }
        }
        // Control input is a synthetic transport into the same screen owner,
        // not an alternate behavior path. RecipeBookComponent's hidden
        // language request is produced inside the shared key/text handler and
        // must cross the same runtime resource/options boundary as a winit
        // event before the next request can observe it.
        self.apply_recipe_book_language_requests(context);
        if narrates_chat {
            if recipe_book_screen_is_open(context.input, context.world) {
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::RecipeBook,
                        input: ScreenNarrationInput::KeyboardAction,
                    });
            } else if matches!(
                context.input.current_screen(),
                Some(Screen::Container(_) | Screen::Creative(_))
            ) {
                let target = if matches!(context.input.current_screen(), Some(Screen::Container(_)))
                    && context.world.lectern_book_view().is_some()
                {
                    ScreenNarrationTarget::Book
                } else {
                    ScreenNarrationTarget::Inventory
                };
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target,
                        input: ScreenNarrationInput::KeyboardAction,
                    });
            }
        }
        if narrates_chat
            && (chat_screen_was_open
                || context.input.chat_entry_is_active()
                || context.input.chat_restrictions_screen_is_open())
        {
            context
                .platform
                .submit(PlatformEffect::ScreenNarrationInput {
                    target: ScreenNarrationTarget::Chat,
                    input: ScreenNarrationInput::KeyboardAction,
                });
        }
    }

    fn request_cursor_capture(&mut self, captured: bool, platform: &mut PlatformEffects) {
        crate::input::key_events::capture_cursor(&mut self.cursor_captured, captured, platform);
    }

    fn request_shutdown(&mut self, context: &mut SimulationInputContext<'_, '_>) {
        self.request_network_terminal_shutdown(context.net_commands, context.platform);
    }
}

pub(crate) struct SimulationInputContext<'a, 'audio> {
    pub(crate) input: &'a mut crate::input::ClientInputState,
    pub(crate) now: Instant,
    pub(crate) first_person_hand_presentation:
        &'a mut crate::item_models::FirstPersonHandPresentation,
    pub(crate) world: &'a mut WorldStore,
    pub(crate) net_counters: &'a mut crate::control::NetCounters,
    pub(crate) net_commands: &'a Option<mpsc::Sender<NetCommand>>,
    pub(crate) server_resource_packs: &'a mut ServerResourcePackRuntime,
    pub(crate) item_runtime: &'a mut Option<NativeItemRuntime>,
    pub(crate) code_of_conduct: Option<&'a mut CodeOfConductAcceptance>,
    pub(crate) win_screen_resources:
        &'a mut std::sync::Arc<crate::win_screen_resources::WinScreenResources>,
    pub(crate) entity_partial_tick: f32,
    pub(crate) level_random: &'a mut bbb_client_world::LevelEventSoundRandomState,
    pub(crate) audio_events: &'a mut Option<&'audio mut dyn AudioEventSink>,
    pub(crate) platform: &'a mut PlatformEffects,
}

fn collect_dialog_open_uri_effects(
    input: &mut crate::input::ClientInputState,
    effects: &RefCell<Vec<PlatformEffect>>,
) {
    effects.borrow_mut().extend(
        input
            .take_dialog_open_url_requests()
            .into_iter()
            .map(PlatformEffect::OpenUri),
    );
}

fn flush_platform_effects(effects: &RefCell<Vec<PlatformEffect>>, platform: &mut PlatformEffects) {
    for effect in effects.take() {
        platform.submit(effect);
    }
}

#[cfg(test)]
mod tests;
