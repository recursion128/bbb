//! F3 debug hotkeys (`KeyboardHandler.keyPress` / `handleDebugKeys`) and their chat feedback.

use super::*;
use bbb_protocol::{
    MC_BUILD_TIME, MC_DATA_PACK_FORMAT, MC_DATA_VERSION, MC_DATA_VERSION_SERIES,
    MC_RESOURCE_PACK_FORMAT, MC_STABLE, MC_VERSION, PROTOCOL_VERSION,
};

/// What a hotkey may touch; every part is absent for some callers.
pub(super) struct DebugKeyContext<'a, 't, 'c> {
    pub(super) world: Option<&'a mut WorldStore>,
    terrain_upload: Option<&'t mut dyn TerrainReloadRequestSink>,
    pub(super) clipboard: Option<&'c mut dyn DebugClipboard>,
    net: Option<DebugNetContext<'a>>,
}

#[derive(Clone, Copy)]
enum DebugKeyGate {
    Always,
    /// `SharedConstants.DEBUG_HOTKEYS` (`KeyboardHandler.handleChunkDebugKeys`).
    DevHotkeys,
    /// `SharedConstants.DEBUG_FEATURE_COUNT`.
    FeatureCount,
}

#[derive(Clone, Copy)]
enum DebugKeyNeeds {
    Player,
    /// A player without reduced debug info.
    UnreducedPlayer,
}

#[derive(Clone, Copy)]
enum DebugKeyLabels {
    ShownHidden,
    EnabledDisabled,
}

impl DebugKeyLabels {
    fn label(self, on: bool) -> &'static str {
        match (self, on) {
            (Self::ShownHidden, true) => "shown",
            (Self::ShownHidden, false) => "hidden",
            (Self::EnabledDisabled, true) => "enabled",
            (Self::EnabledDisabled, false) => "disabled",
        }
    }
}

type DebugKeyFlag = fn(&mut ClientInputState) -> &mut bool;
type DebugKeyRun = fn(&mut ClientInputState, &mut DebugKeyContext<'_, '_, '_>) -> bool;

#[derive(Clone, Copy)]
enum DebugKeyAction {
    /// Toggles a debug screen entry and reports `<prefix><label>`.
    Entry(
        DebugScreenEntryId,
        DebugKeyNeeds,
        &'static str,
        DebugKeyLabels,
    ),
    /// Toggles a client flag, saving it under the options key when one is given.
    Flag(
        DebugKeyFlag,
        &'static str,
        DebugKeyLabels,
        Option<&'static str>,
    ),
    Run(DebugKeyRun),
}

use DebugKeyAction::{Entry, Flag, Run};
use DebugKeyGate::{Always, DevHotkeys, FeatureCount};
use DebugKeyLabels::{EnabledDisabled, ShownHidden};
use DebugKeyNeeds::{Player, UnreducedPlayer};

/// F3+key actions in `KeyboardHandler.handleDebugKeys` order. The first enabled row whose action
/// reports handled consumes the key; a row that declines falls through to later rows for the
/// same key, so the gated development keys shadow the always-on ones only when they apply.
const DEBUG_KEYS: &[(DebugKeyGate, KeyCode, DebugKeyAction)] = &[
    (
        DevHotkeys,
        KeyCode::KeyE,
        Entry(
            DebugScreenEntryId::ChunkSectionPaths,
            Player,
            "SectionPath: ",
            ShownHidden,
        ),
    ),
    (
        DevHotkeys,
        KeyCode::KeyF,
        Flag(
            |input| &mut input.debug_fog_enabled,
            "Fog: ",
            EnabledDisabled,
            None,
        ),
    ),
    (
        DevHotkeys,
        KeyCode::KeyL,
        Flag(
            |input| &mut input.debug_smart_cull_enabled,
            "SmartCull: ",
            EnabledDisabled,
            None,
        ),
    ),
    (
        DevHotkeys,
        KeyCode::KeyO,
        Entry(
            DebugScreenEntryId::ChunkSectionOctree,
            Player,
            "Frustum culling Octree: ",
            EnabledDisabled,
        ),
    ),
    (DevHotkeys, KeyCode::KeyU, Run(debug_key_frustum)),
    (
        DevHotkeys,
        KeyCode::KeyV,
        Entry(
            DebugScreenEntryId::ChunkSectionVisibility,
            Player,
            "SectionVisibility: ",
            EnabledDisabled,
        ),
    ),
    (
        DevHotkeys,
        KeyCode::KeyW,
        Flag(
            |input| &mut input.debug_wireframe_enabled,
            "WireFrame: ",
            EnabledDisabled,
            None,
        ),
    ),
    (
        FeatureCount,
        KeyCode::KeyL,
        Run(|input, _| {
            input
                .debug_feature_count_requests
                .push(DebugFeatureCountRequest::Log);
            true
        }),
    ),
    (
        FeatureCount,
        KeyCode::KeyR,
        Run(|input, _| {
            input
                .debug_feature_count_requests
                .push(DebugFeatureCountRequest::Clear);
            true
        }),
    ),
    (
        Always,
        KeyCode::Escape,
        Run(|input, _| {
            input.debug_pause_screen_requests = input.debug_pause_screen_requests.saturating_add(1);
            true
        }),
    ),
    (Always, KeyCode::KeyA, Run(debug_key_reload_chunks)),
    (
        Always,
        KeyCode::KeyB,
        Entry(
            DebugScreenEntryId::EntityHitboxes,
            UnreducedPlayer,
            "Hitboxes: ",
            ShownHidden,
        ),
    ),
    (
        Always,
        KeyCode::KeyD,
        Run(|_, cx| {
            let Some(world) = cx.world.as_deref_mut() else {
                return false;
            };
            world.clear_client_chat_display_messages();
            true
        }),
    ),
    (
        Always,
        KeyCode::KeyG,
        Entry(
            DebugScreenEntryId::ChunkBorders,
            UnreducedPlayer,
            "Chunk borders: ",
            ShownHidden,
        ),
    ),
    (
        Always,
        KeyCode::KeyH,
        Flag(
            |input| &mut input.debug_advanced_item_tooltips,
            "Advanced tooltips: ",
            ShownHidden,
            Some("advancedItemTooltips"),
        ),
    ),
    (
        Always,
        KeyCode::KeyI,
        Run(|input, cx| {
            input.copy_debug_recreate_command(cx);
            true
        }),
    ),
    (Always, KeyCode::KeyN, Run(debug_key_spectate)),
    (Always, KeyCode::F4, Run(debug_key_game_mode_switcher)),
    (
        Always,
        KeyCode::F6,
        Run(|input, _| {
            input.debug_options_screen_requests =
                input.debug_options_screen_requests.saturating_add(1);
            true
        }),
    ),
    (
        Always,
        KeyCode::KeyP,
        Flag(
            |input| &mut input.debug_pause_on_lost_focus,
            "Pause on lost focus: ",
            EnabledDisabled,
            Some("pauseOnLostFocus"),
        ),
    ),
    (
        Always,
        KeyCode::KeyS,
        Run(|input, cx| {
            input.debug_dynamic_texture_dump_requests =
                input.debug_dynamic_texture_dump_requests.saturating_add(1);
            push_debug_dynamic_texture_dump_feedback(cx.world.as_deref_mut());
            true
        }),
    ),
    (
        Always,
        KeyCode::KeyT,
        Run(|input, cx| {
            input.debug_resource_pack_reload_requests =
                input.debug_resource_pack_reload_requests.saturating_add(1);
            push_debug_feedback_chat_message(cx.world.as_deref_mut(), "Reloaded resource packs");
            true
        }),
    ),
    (
        Always,
        KeyCode::KeyL,
        Run(|input, _| {
            input.debug_profiling_recording = !input.debug_profiling_recording;
            input
                .debug_profiling_toggle_requests
                .push(if input.debug_profiling_recording {
                    DebugProfilingToggleRequest::Start
                } else {
                    DebugProfilingToggleRequest::Stop
                });
            true
        }),
    ),
    // Vanilla shares C between copy-location and the manual crash key, so F3+C
    // counts as a debug action even when no copy happens.
    (Always, KeyCode::KeyC, Run(debug_key_copy_location)),
    (
        Always,
        KeyCode::KeyV,
        Run(|_, cx| {
            if let Some(world) = cx.world.as_deref_mut() {
                push_debug_version_chat_messages(world);
            }
            true
        }),
    ),
    (
        Always,
        KeyCode::Digit1,
        Run(|input, _| {
            input.toggle_debug_profiler_chart();
            true
        }),
    ),
    (
        Always,
        KeyCode::Digit2,
        Run(|input, _| {
            input.toggle_debug_fps_charts();
            true
        }),
    ),
    (
        Always,
        KeyCode::Digit3,
        Run(|input, _| {
            input.toggle_debug_network_charts();
            true
        }),
    ),
    (
        Always,
        KeyCode::Digit4,
        Run(|input, _| {
            input.toggle_debug_lightmap_texture();
            true
        }),
    ),
];

fn debug_key_frustum(input: &mut ClientInputState, cx: &mut DebugKeyContext<'_, '_, '_>) -> bool {
    let (request, feedback) = if input.shift_down() {
        (DebugFrustumRequest::Kill, "Killed frustum")
    } else {
        (DebugFrustumRequest::Capture, "Captured frustum")
    };
    input.debug_frustum_requests.push(request);
    push_debug_feedback_chat_message(cx.world.as_deref_mut(), feedback);
    true
}

fn debug_key_reload_chunks(_: &mut ClientInputState, cx: &mut DebugKeyContext<'_, '_, '_>) -> bool {
    let Some(terrain_upload) = cx.terrain_upload.as_deref_mut() else {
        return false;
    };
    terrain_upload.request_reload_all_chunks();
    push_debug_feedback_chat_message(cx.world.as_deref_mut(), "Reloading all chunks");
    true
}

fn debug_key_spectate(_: &mut ClientInputState, cx: &mut DebugKeyContext<'_, '_, '_>) -> bool {
    let Some(world) = cx.world.as_deref_mut() else {
        return true;
    };
    if !world.local_player_has_gamemaster_permission() {
        push_debug_feedback_chat_message(Some(world), "Unable to switch game mode; no permission");
    } else if let Some(net) = cx.net.as_mut() {
        queue_change_game_mode_command(
            net.counters,
            net.net_commands,
            ChangeGameModeCommand {
                game_mode: debug_spectate_target_game_mode(world),
            },
        );
    }
    true
}

fn debug_key_game_mode_switcher(
    input: &mut ClientInputState,
    cx: &mut DebugKeyContext<'_, '_, '_>,
) -> bool {
    if let Some(switcher) = input.screen.game_mode_switcher_mut() {
        switcher.selected = next_debug_game_mode_icon(switcher.selected);
        switcher.first_mouse_position = None;
        return true;
    }
    let Some(world) = cx.world.as_deref_mut() else {
        return false;
    };
    if !input.debug_game_mode_switcher_can_open(world) {
        return false;
    }
    if !world.local_player_has_gamemaster_permission() {
        push_debug_feedback_chat_message(
            Some(world),
            "Unable to open game mode switcher; no permission",
        );
    } else {
        // `debug_game_mode_switcher_can_open` admits no open screen to replace.
        input
            .screen
            .open_child(Screen::GameModeSwitcher(DebugGameModeSwitcherState {
                selected: default_debug_game_mode_switcher_selection(world),
                first_mouse_position: None,
            }));
    }
    true
}

fn debug_key_copy_location(_: &mut ClientInputState, cx: &mut DebugKeyContext<'_, '_, '_>) -> bool {
    let command = cx.world.as_deref().and_then(debug_copy_location_command);
    if let (Some(command), Some(clipboard)) = (command, cx.clipboard.as_deref_mut()) {
        if clipboard.set_debug_clipboard_text(&command) {
            push_debug_feedback_chat_message(
                cx.world.as_deref_mut(),
                "Copied location to clipboard",
            );
        }
    }
    true
}

impl ClientInputState {
    #[cfg(test)]
    pub(crate) fn handle_debug_overlay_key(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
        world: Option<&mut WorldStore>,
        terrain_upload: Option<&mut dyn TerrainReloadRequestSink>,
    ) -> bool {
        self.handle_debug_overlay_key_with_clipboard(
            physical_key,
            state,
            world,
            terrain_upload,
            None,
        )
    }

    #[cfg(test)]
    pub(crate) fn handle_debug_overlay_key_with_clipboard(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
        world: Option<&mut WorldStore>,
        terrain_upload: Option<&mut dyn TerrainReloadRequestSink>,
        clipboard: Option<&mut dyn DebugClipboard>,
    ) -> bool {
        self.handle_debug_overlay_key_inner(
            physical_key,
            state,
            DebugKeyContext {
                world,
                terrain_upload,
                clipboard,
                net: None,
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_debug_overlay_key_with_clipboard_and_net(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
        world: Option<&mut WorldStore>,
        terrain_upload: Option<&mut dyn TerrainReloadRequestSink>,
        clipboard: Option<&mut dyn DebugClipboard>,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) -> bool {
        self.handle_debug_overlay_key_inner(
            physical_key,
            state,
            DebugKeyContext {
                world,
                terrain_upload,
                clipboard,
                net: Some(DebugNetContext {
                    counters,
                    net_commands,
                }),
            },
        )
    }

    /// Owns the F3 bookkeeping: F3 release toggles the overlay unless a key used it as a
    /// modifier, and closes an open game mode switcher with its selection.
    fn handle_debug_overlay_key_inner(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
        mut cx: DebugKeyContext<'_, '_, '_>,
    ) -> bool {
        let PhysicalKey::Code(code) = physical_key else {
            return false;
        };

        if code == KeyCode::F3 {
            match state {
                ElementState::Pressed => {
                    self.debug_modifier_down = true;
                }
                ElementState::Released => {
                    self.debug_modifier_down = false;
                    self.reset_debug_crash_hold();
                    if let Some(Screen::GameModeSwitcher(switcher)) = self
                        .screen
                        .close_if(|screen| matches!(screen, Screen::GameModeSwitcher(_)))
                    {
                        self.debug_modifier_used = false;
                        if let (Some(world), Some(net)) = (cx.world.as_deref_mut(), cx.net.as_mut())
                        {
                            queue_debug_game_mode_switcher_selection(world, net, switcher.selected);
                        }
                        return true;
                    }
                    if self.debug_modifier_used {
                        self.debug_modifier_used = false;
                    } else {
                        self.debug_entries.toggle_overlay();
                    }
                }
            }
            return true;
        }

        if matches!(state, ElementState::Released) && code == KeyCode::KeyC {
            self.reset_debug_crash_hold();
        }

        if matches!(state, ElementState::Pressed)
            && self.debug_modifier_down
            && DEBUG_KEYS.iter().any(|&(gate, key, action)| {
                key == code && self.debug_key_gate_open(gate) && self.run_debug_key(action, &mut cx)
            })
        {
            self.debug_modifier_used = true;
            return true;
        }

        self.screen.game_mode_switcher().is_some()
    }

    fn debug_key_gate_open(&self, gate: DebugKeyGate) -> bool {
        match gate {
            Always => true,
            DevHotkeys => self.debug_hotkeys_enabled,
            FeatureCount => self.debug_feature_count_enabled,
        }
    }

    fn run_debug_key(
        &mut self,
        action: DebugKeyAction,
        cx: &mut DebugKeyContext<'_, '_, '_>,
    ) -> bool {
        match action {
            Entry(entry, needs, prefix, labels) => {
                let allowed = match needs {
                    Player => cx
                        .world
                        .as_deref()
                        .is_some_and(|world| world.local_player_id().is_some()),
                    UnreducedPlayer => {
                        Self::debug_world_status_toggles_allowed(cx.world.as_deref())
                    }
                };
                if !allowed {
                    return false;
                }
                let enabled = self.toggle_debug_screen_entry_status(entry);
                push_debug_feedback_chat_message(
                    cx.world.as_deref_mut(),
                    &format!("{prefix}{}", labels.label(enabled)),
                );
                true
            }
            Flag(flag, prefix, labels, option_key) => {
                let flag = flag(self);
                *flag = !*flag;
                let enabled = *flag;
                if let Some(key) = option_key {
                    self.debug_client_option_writes.push(ClientOptionWrite {
                        key,
                        value: ClientOptionValue::Boolean(enabled),
                    });
                }
                push_debug_feedback_chat_message(
                    cx.world.as_deref_mut(),
                    &format!("{prefix}{}", labels.label(enabled)),
                );
                true
            }
            Run(run) => run(self, cx),
        }
    }

    pub(super) fn reset_debug_crash_hold(&mut self) {
        self.debug_crash_started_at = None;
        self.debug_crash_last_reported_at = None;
        self.debug_crash_report_count = 0;
    }

    pub(super) fn advance_debug_crash_hold(&mut self, world: &mut WorldStore, now: Instant) {
        if !(self.debug_modifier_down && self.pressed_keys.contains(&KeyCode::KeyC)) {
            self.reset_debug_crash_hold();
            return;
        }

        let started_at = match self.debug_crash_started_at {
            Some(started_at) => started_at,
            None => {
                self.debug_crash_started_at = Some(now);
                self.debug_crash_last_reported_at = Some(now);
                self.debug_crash_report_count = 0;
                return;
            }
        };
        let elapsed = now.saturating_duration_since(started_at);
        if elapsed > DEBUG_CRASH_TIME {
            // `KeyboardHandler.tick` first checks `hasControlDown()` and calls
            // `Blaze3D.youJustLostTheGame()` — a deliberate native death that
            // never reaches the crash-report path. `abort` is the closest
            // process-level equivalent; the plain hold still panics into the
            // guarded crash report below.
            if self.control_down() {
                #[cfg(not(target_arch = "wasm32"))]
                std::process::abort();
                #[cfg(target_arch = "wasm32")]
                std::arch::wasm32::unreachable();
            }
            panic!("Manually triggered debug crash");
        }

        let last_reported_at = self.debug_crash_last_reported_at.unwrap_or(started_at);
        if now.saturating_duration_since(last_reported_at) < DEBUG_CRASH_REPORT_INTERVAL {
            return;
        }

        if self.debug_crash_report_count == 0 {
            push_debug_feedback_chat_message(
                Some(world),
                "F3 + C is held down. This will crash the game unless released.",
            );
        } else {
            let remaining = DEBUG_CRASH_TIME.saturating_sub(elapsed);
            let remaining_seconds = remaining.as_secs() + u64::from(remaining.subsec_nanos() > 0);
            push_debug_warning_chat_message(
                Some(world),
                &format!("Crashing in {remaining_seconds}..."),
            );
        }

        self.debug_crash_last_reported_at = Some(now);
        self.debug_crash_report_count = self.debug_crash_report_count.saturating_add(1);
    }

    pub(super) fn debug_world_status_toggles_allowed(world: Option<&WorldStore>) -> bool {
        world.is_some_and(|world| {
            world.local_player_id().is_some() && !world.local_player_has_reduced_debug_info()
        })
    }

    fn debug_game_mode_switcher_can_open(&self, world: &WorldStore) -> bool {
        world.level_info().is_some() && !self.debug_game_mode_switcher_blocked_by_screen(world)
    }

    fn debug_game_mode_switcher_blocked_by_screen(&self, world: &WorldStore) -> bool {
        self.screen.is_open()
            || self.command_entry_is_active()
            || self.sign_editor_is_open()
            || world.open_container_id().is_some()
            || self.dialog_screen_is_open()
    }

    fn toggle_debug_profiler_chart(&mut self) {
        self.debug_profiler_chart_visible =
            !self.debug_overlay_visible() || !self.debug_profiler_chart_visible;
        if self.debug_profiler_chart_visible {
            self.debug_entries.set_overlay_visible(true);
        }
    }

    fn toggle_debug_fps_charts(&mut self) {
        self.debug_fps_charts_visible =
            !self.debug_overlay_visible() || !self.debug_fps_charts_visible;
        if self.debug_fps_charts_visible {
            self.debug_entries.set_overlay_visible(true);
            self.debug_network_charts_visible = false;
            self.debug_lightmap_texture_visible = false;
        }
    }

    fn toggle_debug_network_charts(&mut self) {
        self.debug_network_charts_visible =
            !self.debug_overlay_visible() || !self.debug_network_charts_visible;
        if self.debug_network_charts_visible {
            self.debug_entries.set_overlay_visible(true);
            self.debug_fps_charts_visible = false;
            self.debug_lightmap_texture_visible = false;
        }
    }

    fn toggle_debug_lightmap_texture(&mut self) {
        self.debug_lightmap_texture_visible =
            !self.debug_overlay_visible() || !self.debug_lightmap_texture_visible;
        if self.debug_lightmap_texture_visible {
            self.debug_entries.set_overlay_visible(true);
            self.debug_fps_charts_visible = false;
            self.debug_network_charts_visible = false;
        }
    }

    pub(super) fn record_debug_profiler_chart_navigation_key(&mut self, code: KeyCode) {
        if !self.debug_profiler_chart_visible() || self.debug_modifier_down {
            return;
        }
        if let Some(digit) = debug_profiler_chart_digit(code) {
            self.debug_profiler_chart_navigation_requests.push(digit);
        }
    }
}

fn debug_profiler_chart_digit(code: KeyCode) -> Option<u8> {
    Some(match code {
        KeyCode::Digit0 => 0,
        KeyCode::Digit1 => 1,
        KeyCode::Digit2 => 2,
        KeyCode::Digit3 => 3,
        KeyCode::Digit4 => 4,
        KeyCode::Digit5 => 5,
        KeyCode::Digit6 => 6,
        KeyCode::Digit7 => 7,
        KeyCode::Digit8 => 8,
        KeyCode::Digit9 => 9,
        _ => return None,
    })
}

fn debug_spectate_target_game_mode(world: &WorldStore) -> GameType {
    if world.local_player_is_spectator() {
        world
            .gameplay()
            .previous_game_type
            .map(GameType::from_id)
            .unwrap_or(GameType::Creative)
    } else {
        GameType::Spectator
    }
}

fn default_debug_game_mode_switcher_selection(world: &WorldStore) -> GameType {
    if let Some(previous_game_type) = world.gameplay().previous_game_type {
        GameType::from_id(previous_game_type)
    } else if GameType::from_id(world.gameplay().game_type) == GameType::Creative {
        GameType::Survival
    } else {
        GameType::Creative
    }
}

fn next_debug_game_mode_icon(game_type: GameType) -> GameType {
    match game_type {
        GameType::Creative => GameType::Survival,
        GameType::Survival => GameType::Adventure,
        GameType::Adventure => GameType::Spectator,
        GameType::Spectator => GameType::Creative,
    }
}

pub(super) fn debug_game_mode_switcher_mouse_pos(
    position: PhysicalPosition<f64>,
) -> Option<(i32, i32)> {
    (position.x.is_finite() && position.y.is_finite())
        .then(|| (position.x.floor() as i32, position.y.floor() as i32))
}

pub(super) fn debug_game_mode_switcher_hovered_game_type(
    mouse_pos: (i32, i32),
    surface_size: PhysicalSize<u32>,
) -> Option<GameType> {
    const MODES: [GameType; 4] = [
        GameType::Creative,
        GameType::Survival,
        GameType::Adventure,
        GameType::Spectator,
    ];

    let center_x = i32::try_from(surface_size.width / 2).unwrap_or(i32::MAX);
    let center_y = i32::try_from(surface_size.height / 2).unwrap_or(i32::MAX);
    let slot_start_x = center_x - DEBUG_GAME_MODE_SWITCHER_ALL_SLOTS_WIDTH / 2;
    let slot_y = center_y - DEBUG_GAME_MODE_SWITCHER_SLOT_Y_OFFSET;
    let (mouse_x, mouse_y) = mouse_pos;
    MODES.iter().enumerate().find_map(|(index, mode)| {
        let slot_x = slot_start_x.saturating_add(
            i32::try_from(index).unwrap_or(i32::MAX) * DEBUG_GAME_MODE_SWITCHER_SLOT_PADDED,
        );
        let slot_right = slot_x.saturating_add(DEBUG_GAME_MODE_SWITCHER_SLOT_AREA);
        let slot_bottom = slot_y.saturating_add(DEBUG_GAME_MODE_SWITCHER_SLOT_AREA);
        (mouse_x >= slot_x && mouse_x < slot_right && mouse_y >= slot_y && mouse_y < slot_bottom)
            .then_some(*mode)
    })
}

fn queue_debug_game_mode_switcher_selection(
    world: &WorldStore,
    context: &mut DebugNetContext<'_>,
    selected: GameType,
) {
    if !world.local_player_has_gamemaster_permission()
        || GameType::from_id(world.gameplay().game_type) == selected
    {
        return;
    }
    queue_change_game_mode_command(
        context.counters,
        context.net_commands,
        ChangeGameModeCommand {
            game_mode: selected,
        },
    );
}

fn push_debug_version_chat_messages(world: &mut WorldStore) {
    push_debug_feedback_chat_message(Some(world), "Client version info:");
    world.push_client_system_chat_message(format!("id = {MC_VERSION}"));
    world.push_client_system_chat_message(format!("name = {MC_VERSION}"));
    world.push_client_system_chat_message(format!("data = {MC_DATA_VERSION}"));
    world.push_client_system_chat_message(format!("series = {MC_DATA_VERSION_SERIES}"));
    world.push_client_system_chat_message(format!(
        "protocol = {PROTOCOL_VERSION} (0x{PROTOCOL_VERSION:x})"
    ));
    world.push_client_system_chat_message(format!("build_time = {MC_BUILD_TIME}"));
    world.push_client_system_chat_message(format!(
        "pack_resource = {}",
        MC_RESOURCE_PACK_FORMAT.to_vanilla_string()
    ));
    world.push_client_system_chat_message(format!(
        "pack_data = {}",
        MC_DATA_PACK_FORMAT.to_vanilla_string()
    ));
    world.push_client_system_chat_message(if MC_STABLE {
        "stable = yes"
    } else {
        "stable = no"
    });
}

pub(super) fn push_debug_feedback_chat_message(world: Option<&mut WorldStore>, message: &str) {
    push_debug_feedback_chat_runs(
        world,
        vec![StyledTextRun {
            text: message.to_string(),
            style: ComponentStyle::default(),
        }],
    );
}

fn push_debug_dynamic_texture_dump_feedback(world: Option<&mut WorldStore>) {
    push_debug_feedback_chat_runs(
        world,
        vec![
            StyledTextRun {
                text: "Saved dynamic textures to ".to_string(),
                style: ComponentStyle::default(),
            },
            StyledTextRun {
                text: DEBUG_DYNAMIC_TEXTURE_DUMP_RELATIVE_PATH.to_string(),
                style: ComponentStyle {
                    underlined: Some(true),
                    click_event: Some(ComponentClickEvent::OpenFile {
                        path: DEBUG_DYNAMIC_TEXTURE_DUMP_RELATIVE_PATH.into(),
                    }),
                    ..ComponentStyle::default()
                },
            },
        ],
    );
}

pub(super) fn push_debug_feedback_chat_runs(
    world: Option<&mut WorldStore>,
    body_runs: Vec<StyledTextRun>,
) {
    if let Some(world) = world {
        world.push_styled_client_system_chat_message(debug_feedback_styled_runs(body_runs));
    }
}

/// `KeyboardHandler.debugWarningComponent`: the same decorated line as debug
/// feedback with a `ChatFormatting.RED` prefix instead of yellow.
fn push_debug_warning_chat_message(world: Option<&mut WorldStore>, message: &str) {
    if let Some(world) = world {
        world.push_styled_client_system_chat_message(decorated_debug_styled_runs(
            VANILLA_DEBUG_WARNING_COLOR,
            vec![StyledTextRun {
                text: message.to_string(),
                style: ComponentStyle::default(),
            }],
        ));
    }
}

fn debug_feedback_styled_runs(body_runs: Vec<StyledTextRun>) -> Vec<StyledTextRun> {
    decorated_debug_styled_runs(VANILLA_DEBUG_FEEDBACK_COLOR, body_runs)
}

fn decorated_debug_styled_runs(
    prefix_color: u32,
    body_runs: Vec<StyledTextRun>,
) -> Vec<StyledTextRun> {
    let mut runs = vec![
        StyledTextRun {
            text: "[Debug]:".to_string(),
            style: ComponentStyle {
                bold: Some(true),
                color: Some(prefix_color),
                ..ComponentStyle::default()
            },
        },
        StyledTextRun {
            text: " ".to_string(),
            style: ComponentStyle::default(),
        },
    ];
    runs.extend(body_runs);
    runs
}

fn debug_copy_location_command(world: &WorldStore) -> Option<String> {
    if !ClientInputState::debug_world_status_toggles_allowed(Some(world)) {
        return None;
    }
    let level = world.level_info()?;
    let pose = world.local_player_pose()?;
    Some(format!(
        "/execute in {} run tp @s {:.2} {:.2} {:.2} {:.2} {:.2}",
        level.dimension, pose.position.x, pose.position.y, pose.position.z, pose.y_rot, pose.x_rot
    ))
}
