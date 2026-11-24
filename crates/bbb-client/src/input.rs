mod ime_preedit;
pub(crate) use ime_preedit::ImePreeditHudState;
use ime_preedit::{sign_editor_ime_preedit_hud_state, ImePreeditState};
#[cfg(test)]
pub(crate) use inventory::ContainerInputState;
pub(crate) use inventory::{anvil_screen_is_open, recipe_book_search_is_active};
pub(crate) use text_field::TextField;
mod chat_mouse;
use std::{
    collections::{BTreeSet, HashSet, VecDeque},
    path::{Path, PathBuf},
    time::Duration,
};

use crate::control::NetCounters;
use bbb_client_clock::Instant;
use bbb_client_net::NetCommand;
use bbb_client_render::{
    hud_debug_options_content_x as debug_options_content_x,
    hud_debug_options_footer_button_xs as debug_options_footer_button_xs,
    hud_debug_options_footer_button_y as debug_options_footer_button_y,
    hud_debug_options_search_box_rects,
};
use bbb_client_world::sign_wood_and_form_for_block_name;
use bbb_client_world::CreativeHotbarPresetError;
use bbb_client_world::{
    matches_suggestion_sub_str, BlockPos, CommandSuggestionState, CommandSuggestionsResultState,
    LocalBlockAttackEffect, LocalPlayerPoseState, SignBlockForm, WorldStore,
};
use bbb_protocol::{
    chat_formattings::VANILLA_CHAT_FORMATTINGS,
    packets::{
        BlockPos as ProtocolBlockPos, ChangeGameModeCommand, CommandBlockMode,
        Direction as ProtocolDirection, GameType, InteractionHand, ItemStackSummary,
        PlayerActionKind, PlayerCommandAction, PlayerInput, RecipeBookType, SetCommandBlock,
        SetCommandMinecart, SignUpdate, MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS,
    },
    ComponentClickEvent, ComponentStyle, StyledTextRun,
};
use bbb_render_types::{HudCommandInputRange, HudCommandInputSyntax, HudCommandSuggestionEntry};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::hotbar_presets::HotbarPresetStore;
use crate::narrator_runtime::NarratorStatus;
use crate::options::key_bindings::{BoundInput, KeyBindings};
use crate::options::{ClientOptionValue, ClientOptionWrite};

use crate::debug_entries::{
    DebugScreenEntryId, DebugScreenEntryList, DebugScreenEntryStatus, DebugScreenProfile,
};
use crate::particle_runtime::{LocalDestroyHitParticleState, LocalPlayerActionParticleState};

mod advancements_screen;
pub(crate) use advancements_screen::{AdvancementTabType, AdvancementsScreen};
mod book;
mod bundle;
mod command_block;
use command_block::CommandBlockEditorInputState;
mod command_confirmation;
pub(crate) mod dialog;
pub(crate) use command_confirmation::CommandConfirmationNarrationSnapshot;
mod commands;
pub(crate) mod death_screen;
mod debug_keys;
pub(crate) mod key_events;
use debug_keys::{
    debug_game_mode_switcher_hovered_game_type, debug_game_mode_switcher_mouse_pos,
    push_debug_feedback_chat_message, push_debug_feedback_chat_runs,
};
#[cfg(test)]
pub(crate) use key_events::handle_key_input;
pub(crate) use key_events::handle_text_input_with_item_runtime;
mod debug_recreate;
use debug_recreate::PendingDebugRecreateServerQuery;
pub(crate) use debug_recreate::{
    queue_debug_recreate_server_query_request, DebugRecreateServerQueryRequest,
};
pub(crate) mod disconnected_screen;
pub(crate) mod enchantment_screen;
pub(crate) mod game_event_screen;
mod game_rules;
mod inventory;
mod jigsaw_block;
mod minecart_command_block;
mod mouse;
mod movement;
mod pause_screen;
mod screen;
pub(crate) use screen::{Screen, Screens};
pub(crate) mod social;
pub(crate) mod spectator_menu;
mod stats_screen;
pub(crate) use stats_screen::{stats_screen_done_button_contains, StatsScreen, StatsScreenFocus};
mod structure_block;
mod test_block;
mod test_instance_block;
pub(crate) mod text_field;
mod text_measurements;
pub(crate) use text_measurements::CommandSuggestionTextMeasurer;
#[cfg(test)]
pub(crate) use text_measurements::FallbackTextMeasurer;
mod chat_runtime;
use chat_runtime::*;

mod debug_options_screen;
mod debug_state;
mod sign_editor;
use crate::camera_pose::{camera_pose_from_world, current_post_effect_id_from_world, CameraType};
use crate::crosshair::{crosshair_target_from_camera, CrosshairTarget};
use crate::terrain_upload::TerrainUploadState;
use bbb_client_render::hud_command_block_editor_layout;
use bbb_client_render::item_model::{ItemModelKeybindContext, NativeItemRuntime};
pub(crate) use book::{BookInputMode, BookScreen};
pub(crate) use bundle::select_bundle_item;
use commands::*;
pub(crate) use commands::{
    queue_block_entity_tag_query_command, queue_broadcast_client_information_command,
    queue_change_difficulty_command, queue_change_game_mode_command, queue_chat_command,
    queue_chat_message_command, queue_command_suggestion_request,
    queue_container_button_click_command, queue_container_click_command,
    queue_container_close_command, queue_container_close_request_command,
    queue_container_slot_state_changed_command, queue_edit_book_command,
    queue_entity_tag_query_command, queue_jigsaw_generate_command, queue_lock_difficulty_command,
    queue_perform_respawn_command, queue_place_recipe_command, queue_player_abilities_command,
    queue_recipe_book_change_settings_command, queue_recipe_book_seen_recipe_command,
    queue_rename_item_command, queue_request_game_rule_values_command, queue_request_stats_command,
    queue_seen_advancements_command, queue_select_trade_command, queue_set_beacon_command,
    queue_set_command_block_command, queue_set_creative_mode_slot_command,
    queue_set_game_rule_command, queue_set_jigsaw_block_command, queue_set_structure_block_command,
    queue_set_test_block_command, queue_sign_update_command, queue_spectate_entity_command,
    queue_teleport_to_entity_command, queue_vehicle_move_command, select_hotbar_slot,
};
#[cfg(test)]
use debug_options_screen::{
    debug_options_list_height, debug_options_scrollbar_drag_scroll_row, debug_options_scrollbar_x,
    debug_options_search_box_rect, DEBUG_OPTIONS_DOUBLE_CLICK_THRESHOLD,
    DEBUG_OPTIONS_HEADER_HEIGHT, DEBUG_OPTIONS_ROW_HEIGHT, DEBUG_OPTIONS_ROW_WIDTH,
    DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET, DEBUG_OPTIONS_STATUS_BUTTON_WIDTH,
};
pub(crate) use debug_options_screen::{
    DebugGameModeSwitcherState, DebugOptionsScreenFocus, DebugOptionsScreenHudRow,
    DebugOptionsScreenRow, DebugOptionsScreenState,
};
use debug_options_screen::{
    DEBUG_GAME_MODE_SWITCHER_ALL_SLOTS_WIDTH, DEBUG_GAME_MODE_SWITCHER_SLOT_AREA,
    DEBUG_GAME_MODE_SWITCHER_SLOT_PADDED, DEBUG_GAME_MODE_SWITCHER_SLOT_Y_OFFSET,
};
pub(crate) use game_rules::{
    GameRuleSearchEntry, GameRulesInteractionRow, GameRulesScreen, GameRulesScreenFocus,
};
pub(crate) use inventory::creative::{
    creative_inventory_matching_item_tags, creative_inventory_max_scroll_row,
    creative_inventory_visible_items, CreativeInventoryHover, CreativeInventoryHudState,
};
pub(crate) use inventory::{
    advance_creative_inventory_commands, anvil_rename_entry_consumes_key, beacon_effect_buttons,
    creative_inventory_search_entry_consumes_key, handle_inventory_cursor_moved,
    handle_inventory_key_input_with_clipboard, handle_inventory_mouse_input_with_item_runtime,
    handle_inventory_mouse_wheel, handle_inventory_text_input, inventory_screen_layout,
    inventory_screen_layout_for_surface, inventory_screen_selected_hotbar_slot_id,
    loom_pattern_row_count, maybe_close_narrow_recipe_book, queue_lectern_button_click,
    recipe_book_button_at_position, recipe_book_button_position, recipe_book_main_gui_offset,
    recipe_book_recipe_button_position, recipe_book_search_entry_consumes_key,
    recipe_book_tab_count_for_background, recipe_book_type_for_background,
    recipe_book_type_settings, recipe_book_visible_tab_indices, sync_beacon_effect_selection_state,
    sync_loom_pattern_state_for_hud, sync_recipe_book_presentation,
    sync_stonecutter_recipe_scroll_state, InventoryScreenBackground, InventorySlotLayout,
    RecipeBookPresentationHudState, BEACON_EFFECT_BUTTON_SIZE, LOOM_PATTERN_BUTTON_COLUMNS,
    LOOM_PATTERN_BUTTON_ROWS, LOOM_PATTERN_BUTTON_SIZE, LOOM_PATTERN_BUTTON_X,
    LOOM_PATTERN_BUTTON_Y, LOOM_SCROLLER_HEIGHT, LOOM_SCROLLER_MAX_OFFSET, LOOM_SCROLLER_WIDTH,
    LOOM_SCROLLER_X, LOOM_SCROLLER_Y, LOOM_SELECTED_PATTERN_DATA_ID, RECIPE_BOOK_BUTTON_HEIGHT,
    RECIPE_BOOK_BUTTON_WIDTH, RECIPE_BOOK_FILTER_BUTTON_HEIGHT, RECIPE_BOOK_FILTER_BUTTON_WIDTH,
    RECIPE_BOOK_FILTER_BUTTON_X, RECIPE_BOOK_FILTER_BUTTON_Y, RECIPE_BOOK_PAGE_BACKWARD_BUTTON_X,
    RECIPE_BOOK_PAGE_BUTTON_HEIGHT, RECIPE_BOOK_PAGE_BUTTON_WIDTH, RECIPE_BOOK_PAGE_BUTTON_Y,
    RECIPE_BOOK_PAGE_FORWARD_BUTTON_X, RECIPE_BOOK_RECIPE_BUTTON_SIZE,
    RECIPE_BOOK_SEARCH_BOX_HEIGHT, RECIPE_BOOK_SEARCH_BOX_WIDTH, RECIPE_BOOK_SEARCH_BOX_X,
    RECIPE_BOOK_SEARCH_BOX_Y, RECIPE_BOOK_SEARCH_TEXT_X_OFFSET, RECIPE_BOOK_SEARCH_TEXT_Y_OFFSET,
    RECIPE_BOOK_SELECTED_TAB_X_OFFSET, RECIPE_BOOK_TAB_HEIGHT, RECIPE_BOOK_TAB_STRIDE_Y,
    RECIPE_BOOK_TAB_WIDTH, RECIPE_BOOK_TAB_X, RECIPE_BOOK_TAB_Y, STONECUTTER_RECIPE_BUTTON_COLUMNS,
    STONECUTTER_RECIPE_BUTTON_HEIGHT, STONECUTTER_RECIPE_BUTTON_ROWS,
    STONECUTTER_RECIPE_BUTTON_SPRITE_Y, STONECUTTER_RECIPE_BUTTON_WIDTH,
    STONECUTTER_RECIPE_BUTTON_X, STONECUTTER_RECIPE_ITEM_Y, STONECUTTER_SCROLLER_HEIGHT,
    STONECUTTER_SCROLLER_MAX_OFFSET, STONECUTTER_SCROLLER_WIDTH, STONECUTTER_SCROLLER_X,
    STONECUTTER_SCROLLER_Y, STONECUTTER_SELECTED_RECIPE_DATA_ID,
    STONECUTTER_VISIBLE_RECIPE_BUTTON_COUNT,
};
#[cfg(test)]
pub(crate) use inventory::{
    handle_inventory_mouse_input, RECIPE_BOOK_RECIPE_BUTTON_X, RECIPE_BOOK_RECIPE_BUTTON_Y,
};
use minecart_command_block::MinecartCommandBlockEditorInputState;
#[cfg(test)]
pub(crate) use mouse::handle_mouse_input_at_partial_tick;
pub(crate) use mouse::{
    advance_destroying_block_at_partial_tick_with_biome_blend_radius,
    advance_local_using_item_ticks_with_effects,
    advance_using_item_at_partial_tick_with_effects_and_presentation,
    flush_local_interaction_sound_actions,
    handle_mouse_input_at_partial_tick_with_effects_and_presentation, handle_mouse_motion,
    handle_mouse_wheel, local_player_hands_are_busy,
};
#[cfg(test)]
pub(crate) use movement::advance_player_input;
pub(crate) use movement::advance_player_input_for_client_ticks;
pub(crate) use pause_screen::{
    native_pause_freezes_client_ticks, pause_screen_advancements_button_contains,
    pause_screen_disconnect_button_contains, pause_screen_feedback_back_button_contains,
    pause_screen_feedback_report_bugs_button_contains,
    pause_screen_feedback_send_feedback_button_contains, pause_screen_options_button_contains,
    pause_screen_options_button_enabled, pause_screen_report_bugs_button_contains,
    pause_screen_report_bugs_button_enabled, pause_screen_return_to_game_button_contains,
    pause_screen_send_feedback_button_contains, pause_screen_stats_button_contains,
    PauseScreenLinkRequest, PauseScreenPage,
};
pub(crate) use sign_editor::SignEditorInputState;
use sign_editor::*;

pub(crate) trait TerrainReloadRequestSink {
    fn request_reload_all_chunks(&mut self);
}

impl TerrainReloadRequestSink for TerrainUploadState {
    fn request_reload_all_chunks(&mut self) {
        TerrainUploadState::request_reload_all_chunks(self);
    }
}

const HOTBAR_BINDING_NAMES: [&str; 9] = [
    "key.hotbar.1",
    "key.hotbar.2",
    "key.hotbar.3",
    "key.hotbar.4",
    "key.hotbar.5",
    "key.hotbar.6",
    "key.hotbar.7",
    "key.hotbar.8",
    "key.hotbar.9",
];
const CREATIVE_FLIGHT_JUMP_TRIGGER_TICKS: u8 = 7;
const CREATIVE_FLIGHT_TICK_SECONDS: f64 = 0.05;
const SPRINT_TRIGGER_TICKS: u8 = 7;
const SPRINT_TRIGGER_TICK_SECONDS: f64 = 0.05;
const DEBUG_CRASH_TIME: Duration = Duration::from_secs(10);
const DEBUG_CRASH_REPORT_INTERVAL: Duration = Duration::from_secs(1);
const CHAT_ENTRY_MAX_LENGTH: usize = 256;
const RECENT_CHAT_LIMIT: usize = 100;
pub(crate) const DEBUG_DYNAMIC_TEXTURE_DUMP_RELATIVE_PATH: &str = "screenshots/debug";
pub(crate) const DEBUG_PROFILING_RESULTS_RELATIVE_DIR: &str = "debug/profiling";
const VANILLA_DEBUG_FEEDBACK_COLOR: u32 = 0xFFFF55;
/// `ChatFormatting.RED`, the prefix colour `KeyboardHandler.debugWarningComponent`
/// passes to `decorateDebugComponent` for the manual-crash countdown.
const VANILLA_DEBUG_WARNING_COLOR: u32 = 0xFF5555;
const DIALOG_DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(250);
const DIALOG_IME_PREEDIT_MAX_CHARS: usize = 256;
const DIALOG_IME_PREEDIT_CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(300);

pub(crate) trait DebugClipboard {
    fn set_debug_clipboard_text(&mut self, text: &str) -> bool;

    fn set_debug_clipboard_java_text(&mut self, text: &bbb_protocol::JavaString) -> bool {
        // ClipboardManager encodes with JDK UTF-8 replacement, then GLFW reads a C string.
        let end = text
            .units()
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(text.units().len());
        let text: String = char::decode_utf16(text.units()[..end].iter().copied())
            .map(|value| value.unwrap_or('?'))
            .collect();
        self.set_debug_clipboard_text(&text)
    }

    fn get_debug_clipboard_text(&mut self) -> Option<String> {
        None
    }
}

struct DebugNetContext<'a> {
    counters: &'a mut NetCounters,
    net_commands: &'a Option<mpsc::Sender<NetCommand>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DebugFrustumRequest {
    Capture,
    Kill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DebugFeatureCountRequest {
    Log,
    Clear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DebugProfilingToggleRequest {
    Start,
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NarratorStatusRequest {
    pub(crate) status: NarratorStatus,
    pub(crate) schedule_narration: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CameraPostEffectSelection {
    client_level_generation: u64,
    set_camera_updates_applied: usize,
    effect_id: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum LastInputType {
    #[default]
    None,
    Mouse,
    KeyboardArrow,
    KeyboardTab,
}

impl LastInputType {
    pub(crate) fn is_keyboard(self) -> bool {
        matches!(self, Self::KeyboardArrow | Self::KeyboardTab)
    }
}

/// Vanilla `net.minecraft.util.SmoothDouble`: the cinematic-camera
/// accumulator that lerps toward the queued turn target.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct SmoothDouble {
    target_value: f64,
    remaining_value: f64,
    last_amount: f64,
}

impl SmoothDouble {
    pub(crate) fn new_delta_value(&mut self, target_delta: f64, time: f64) -> f64 {
        self.target_value += target_delta;
        let mut delta = self.target_value - self.remaining_value;
        let new_last_amount = self.last_amount + (delta - self.last_amount) * 0.5;
        let delta_sign = delta.signum();
        if delta_sign * delta > delta_sign * self.last_amount {
            delta = new_last_amount;
        }
        self.last_amount = new_last_amount;
        self.remaining_value += delta * time;
        delta * time
    }

    pub(crate) fn reset(&mut self) {
        self.target_value = 0.0;
        self.remaining_value = 0.0;
        self.last_amount = 0.0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MouseSensitivity(f64);

impl Default for MouseSensitivity {
    fn default() -> Self {
        Self(0.5)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MouseWheelSensitivity(f64);

impl Default for MouseWheelSensitivity {
    fn default() -> Self {
        Self(1.0)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ClientInputState {
    focused: bool,
    /// `Minecraft.lastActiveTime`: refreshed while the
    /// client is active; `pauseIfInactive` pauses after 500 ms of inactivity.
    /// `Option` keeps the derived `Default` (never-active) honest for tests.
    last_active_at: Option<Instant>,
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    jump: bool,
    sneak: bool,
    sprint_key_down: bool,
    attack_down: bool,
    use_down: bool,
    destroy_block_held: bool,
    use_item_held: bool,
    pending_local_player_action_particles: Vec<LocalPlayerActionParticleState>,
    shift_left_down: bool,
    shift_right_down: bool,
    control_left_down: bool,
    control_right_down: bool,
    /// Active gameplay `KeyMapping` keys (cleared by `release_gameplay_pressed`).
    pressed_keys: HashSet<KeyCode>,
    /// Physically held OS keys (cleared only on focus loss), used for modifier
    /// queries (`super_down`/`alt_down`) and OS key-repeat deduplication.
    physical_pressed_keys: HashSet<KeyCode>,
    boat_paddles_active: bool,
    mouse_left_down: bool,
    mouse_right_down: bool,
    mouse_middle_down: bool,
    attack_miss_time_ticks: u16,
    use_item_repeat_delay_ticks: u8,
    mouse_delta_x: f64,
    mouse_delta_y: f64,
    scroll_accumulated_x: f64,
    scroll_accumulated_y: f64,
    bundle_scroll_accumulated_x: f64,
    bundle_scroll_accumulated_y: f64,
    inventory_cursor_position: Option<(i32, i32)>,
    creative_selected_tab: Option<String>,
    creative_drop_spam_count: u16,
    recipe_book_search_preference:
        text_field::BoundedTextField<{ inventory::RECIPE_BOOK_SEARCH_MAX_LENGTH }>,
    recipe_book_search_focused_preference: bool,
    recipe_book_overlay_preference: Option<RecipeBookOverlayHudState>,
    recipe_book_last_placed_preference: Option<(i32, i32)>,
    client_text_filtering_enabled: bool,
    dialog_screen_revision: u64,
    dialog_chat_links_disabled: bool,
    dialog_chat_links_prompt_disabled: bool,
    dialog_open_url_requests: Vec<bbb_protocol::JavaString>,
    /// `RecipeBookComponent.checkSearchStringUpdate` requests the fixed
    /// `en_pt` language when the search text lowercases to `excitedze`.
    /// Keep only a count: the target is a Vanilla constant, so owned strings
    /// would add allocation without carrying any information.
    recipe_book_language_requests: u32,
    recipe_book_crafting_tab_index: usize,
    recipe_book_furnace_tab_index: usize,
    recipe_book_blast_furnace_tab_index: usize,
    recipe_book_smoker_tab_index: usize,
    recipe_book_crafting_page: usize,
    recipe_book_furnace_page: usize,
    recipe_book_blast_furnace_page: usize,
    recipe_book_smoker_page: usize,
    pause_screen_link_requests: Vec<PauseScreenLinkRequest>,
    local_disconnect_requested: bool,
    debug_entries: DebugScreenEntryList,
    debug_profile_store_path: Option<PathBuf>,
    hotbar_preset_store: Option<HotbarPresetStore>,
    last_input_type: LastInputType,
    last_input_type_screen_was_open: bool,
    narrator_status: NarratorStatus,
    narrator_hotkey_enabled: bool,
    narrator_available: bool,
    key_bindings: KeyBindings,
    toggle_crouch: bool,
    toggle_sprint: bool,
    toggle_attack: bool,
    toggle_use: bool,
    last_seen_perform_respawn_commands: usize,
    sneak_released_by_screen_when_down: bool,
    sprint_released_by_screen_when_down: bool,
    attack_released_by_screen_when_down: bool,
    narrator_status_requests: Vec<NarratorStatusRequest>,
    fullscreen: bool,
    fullscreen_requests: Vec<bool>,
    screenshot_requests: u32,
    mouse_sensitivity: MouseSensitivity,
    invert_mouse_x: bool,
    invert_mouse_y: bool,
    discrete_mouse_scroll: bool,
    mouse_wheel_sensitivity: MouseWheelSensitivity,
    /// Vanilla `Options.smoothCamera`: a transient (never persisted) toggle
    /// flipped by the unbound-by-default `key.smoothCamera` mapping.
    smooth_camera: bool,
    smooth_turn_x: SmoothDouble,
    smooth_turn_y: SmoothDouble,
    /// Vanilla `MouseHandler.lastHandleMovementTime`: sampled every frame,
    /// even when the mouse is not grabbed.
    last_turn_sample_at: Option<Instant>,
    /// Mirrors the simulation cursor-capture owner: vanilla only calls
    /// `turnPlayer` (advancing or resetting the smooths) while grabbed.
    mouse_look_live: bool,
    pending_turn_delta: Option<(f64, f64)>,
    debug_modifier_down: bool,
    debug_modifier_used: bool,
    debug_profiler_chart_visible: bool,
    debug_fps_charts_visible: bool,
    debug_network_charts_visible: bool,
    debug_lightmap_texture_visible: bool,
    debug_advanced_item_tooltips: bool,
    debug_show_local_server_entity_hit_boxes: bool,
    debug_hotkeys_enabled: bool,
    debug_feature_count_enabled: bool,
    qol_extensions: crate::qol_extensions::ClientExtensionSet,
    zoom_scale: Option<f32>,
    remote_debug_renderers: crate::remote_debug::RemoteDebugRendererSelection,
    debug_fog_enabled: bool,
    debug_smart_cull_enabled: bool,
    debug_wireframe_enabled: bool,
    /// Vanilla `Options.hideGui`, toggled by the default `key.toggleGui` (F1).
    hide_gui: bool,
    camera_type: CameraType,
    camera_post_effect_selection: Option<CameraPostEffectSelection>,
    camera_post_effect_id: Option<&'static str>,
    camera_post_effect_active: bool,
    debug_frustum_requests: Vec<DebugFrustumRequest>,
    debug_feature_count_requests: Vec<DebugFeatureCountRequest>,
    debug_pause_on_lost_focus: bool,
    /// The `Options.reducedDebugInfo` half of `Minecraft.showOnlyReducedInfo`.
    debug_reduced_debug_info_option: bool,
    debug_client_option_writes: Vec<ClientOptionWrite>,
    debug_resource_pack_reload_requests: u32,
    debug_dynamic_texture_dump_requests: u32,
    debug_profiling_recording: bool,
    debug_profiling_toggle_requests: Vec<DebugProfilingToggleRequest>,
    debug_profiler_chart_navigation_requests: Vec<u8>,
    debug_options_screen_requests: u32,
    debug_pause_screen_requests: u32,
    screen: Screens,
    debug_recreate_server_query_requests: Vec<DebugRecreateServerQueryRequest>,
    pending_debug_recreate_server_query: Option<PendingDebugRecreateServerQuery>,
    debug_query_transaction_id: i32,
    debug_crash_started_at: Option<Instant>,
    debug_crash_last_reported_at: Option<Instant>,
    debug_crash_report_count: u32,
    chat_scroll: bbb_client_render::HudChatScroll,
    chat_scroll_page: bbb_client_render::HudChatScrollPage,
    recent_chat: VecDeque<Vec<u16>>,
    chat_input_inner_width: i32,
    latest_chat_draft: Option<ChatDraftState>,
    save_chat_drafts: bool,
    next_chat_screen_revision: u64,
    #[cfg(test)]
    last_step: Option<Instant>,
    #[cfg(test)]
    local_player_movement_tick_accumulator_seconds: f64,
    last_move_command_at: Option<Instant>,
    move_position_reminder_ticks: u32,
    last_move_command_pose: Option<LocalPlayerPoseState>,
    last_vehicle_command_at: Option<Instant>,
    riding_jump_charge_seconds: Option<f64>,
    sprint_trigger_ticks: u8,
    sprint_trigger_elapsed_seconds: f64,
    sprint_start_requested: bool,
    last_sent_sprinting: bool,
    sprint_level_generation: u64,
    sprint_level_load_start: usize,
    creative_flight_jump_trigger_ticks: u8,
    creative_flight_jump_trigger_elapsed_seconds: f64,
    spectator_gui: spectator_menu::SpectatorGuiInputState,
    disconnected_screen: disconnected_screen::DisconnectedScreenInputState,
    death_screen: death_screen::DeathScreenInputState,
    command_confirmation_generation: u64,
    command_confirmation_narrations: Vec<CommandConfirmationNarrationSnapshot>,
    command_confirmation_clipboard: Vec<bbb_protocol::JavaString>,
    game_event_screen: game_event_screen::GameEventScreenInputState,
    social_interactions: social::SocialInteractionsInputState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DialogImePreeditState {
    screen_revision: u64,
    preedit: ImePreeditState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DialogImePreeditHudState {
    pub(crate) text: String,
    pub(crate) caret: Option<usize>,
    pub(crate) focused_start: usize,
    pub(crate) focused_end: usize,
    pub(crate) screen_revision: u64,
    pub(crate) caret_visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChatCommandCompletionCache {
    contents: Vec<u16>,
    cursor: usize,
    suggestion_cursor: usize,
    revision: bbb_client_world::CommandCompletionRevision,
    completion: bbb_client_world::CommandCompletionResult,
    syntax: Option<HudCommandInputSyntax>,
    suggestions_synced: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChatEntryState {
    edit: text_field::TextField,
    suggestion_cursor: usize,
    command_completion: Option<ChatCommandCompletionCache>,
    last_suggestion_request_text: Option<bbb_protocol::JavaString>,
    suggestion_list: Option<ChatSuggestionListState>,
    hidden_suggestion_id: Option<i32>,
    suggestion_narration: Option<CommandSuggestionNarrationState>,
    mouse_position: Option<(i32, i32)>,
    suppress_open_key_commit: bool,
    owner: CommandEntryKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CommandEntryKind {
    Chat(ChatScreenEntryState),
    CommandBlock(CommandBlockEditorInputState),
    MinecartCommandBlock(MinecartCommandBlockEditorInputState),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChatScreenExitReason {
    Intentional,
    Interrupted,
    Done,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ChatMethod {
    #[default]
    Message,
    Command,
}

impl ChatMethod {
    fn prefix(self) -> &'static str {
        match self {
            Self::Message => "",
            Self::Command => "/",
        }
    }

    fn restores(self, draft: &ChatDraftState) -> bool {
        self == Self::Message || draft.method == Self::Command
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChatDraftState {
    text: Vec<u16>,
    method: ChatMethod,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChatScreenEntryState {
    method: ChatMethod,
    is_draft: bool,
    history_buffer: Vec<u16>,
    history_pos: usize,
    revision: u64,
    initial: Vec<u16>,
    dragging: bool,
    last_click: Option<(MouseButton, Instant)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChatRestrictionsScreenState {
    generation: u64,
}

const COMMAND_SUGGESTION_LINE_HEIGHT: i32 = 12;
const COMMAND_SUGGESTION_LINE_LIMIT: usize = 10;
const COMMAND_BLOCK_SUGGESTION_LINE_LIMIT: usize = 7;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChatSuggestionListState {
    result_id: i32,
    result: CommandSuggestionsResultState,
    original_contents: Vec<u16>,
    sorted_indices: Vec<usize>,
    /// `FormattedText.of(suggestion.getText())` resolved once per completed
    /// result, indexed like `result.suggestions`. The list width
    /// and the per-row draw
    /// both read this list, so neither can consume a different
    /// set of codes than the other.
    resolved: Vec<Vec<bbb_client_render::HudStyledTextRun>>,
    current: usize,
    offset: usize,
    tab_cycles: bool,
    rect: Option<CommandSuggestionRect>,
}

/// Language-neutral payload for vanilla's translatable suggestion narration.
/// A future narrator owner can localize `narration.suggestion*` without the
/// input owner pretending that an English string was spoken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandSuggestionNarrationState {
    pub(crate) position: usize,
    pub(crate) total: usize,
    /// The candidate's own units: the localization entry is what turns them
    /// into the text the narrator speaks.
    pub(crate) text: bbb_protocol::JavaString,
    pub(crate) tooltip: Option<String>,
    pub(crate) tooltip_translation_key: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CommandSuggestionRect {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl CommandSuggestionRect {
    fn contains(self, x: i32, y: i32) -> bool {
        let right = self.x.saturating_add_unsigned(self.width);
        let bottom = self.y.saturating_add_unsigned(self.height);
        x >= self.x && x <= right && y >= self.y && y <= bottom
    }

    fn hovered_line(self, x: i32, y: i32) -> Option<usize> {
        let right = self.x.saturating_add_unsigned(self.width);
        if x <= self.x || x >= right {
            return None;
        }
        let lines = usize::try_from(self.height / COMMAND_SUGGESTION_LINE_HEIGHT as u32).ok()?;
        (0..lines).find(|line| {
            let top = self.y.saturating_add(
                i32::try_from(*line)
                    .unwrap_or(i32::MAX)
                    .saturating_mul(COMMAND_SUGGESTION_LINE_HEIGHT),
            );
            y > top && y < top.saturating_add(COMMAND_SUGGESTION_LINE_HEIGHT)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandSuggestionHudTooltip {
    pub(crate) text: String,
    pub(crate) translation_key: Option<String>,
    pub(crate) runs: Vec<StyledTextRun>,
    pub(crate) x: i32,
    pub(crate) y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandSuggestionHudState {
    pub(crate) input_text: String,
    pub(crate) input_view: Option<bbb_client_render::HudChatInputView>,
    pub(crate) draft: bool,
    pub(crate) syntax: Option<HudCommandInputSyntax>,
    pub(crate) cursor: usize,
    pub(crate) selection: usize,
    pub(crate) cursor_visible: bool,
    pub(crate) suffix: Option<String>,
    pub(crate) rect: CommandSuggestionRect,
    pub(crate) entries: Vec<HudCommandSuggestionEntry>,
    pub(crate) has_previous: bool,
    pub(crate) has_next: bool,
    pub(crate) tooltip: Option<CommandSuggestionHudTooltip>,
    pub(crate) narration: Option<CommandSuggestionNarrationState>,
    pub(crate) command_block_layout: bool,
}

impl ChatEntryState {
    fn commands_only(&self) -> bool {
        matches!(
            self.owner,
            CommandEntryKind::CommandBlock(_) | CommandEntryKind::MinecartCommandBlock(_)
        )
    }

    fn suggestion_line_limit(&self) -> usize {
        if self.commands_only() {
            COMMAND_BLOCK_SUGGESTION_LINE_LIMIT
        } else {
            COMMAND_SUGGESTION_LINE_LIMIT
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AnvilRenameInputSignature {
    item: ItemStackSummary,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RecipeBookSearchHudState {
    pub(crate) edit: text_field::TextField,
    pub(crate) focused: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RecipeBookTabSelectionHudState {
    pub(crate) crafting: usize,
    pub(crate) furnace: usize,
    pub(crate) blast_furnace: usize,
    pub(crate) smoker: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RecipeBookPageHudState {
    pub(crate) crafting: usize,
    pub(crate) furnace: usize,
    pub(crate) blast_furnace: usize,
    pub(crate) smoker: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecipeBookOverlayHudState {
    pub(crate) book_type: RecipeBookType,
    pub(crate) tab_index: usize,
    pub(crate) page_index: usize,
    pub(crate) button_index: usize,
    pub(crate) x: i32,
    pub(crate) y: i32,
}

impl ClientInputState {
    fn advance_attack_miss_time(&mut self, client_ticks: u32) {
        if client_ticks == 0 {
            return;
        }
        if self.screen.is_open() {
            self.attack_miss_time_ticks = 10_000;
            return;
        }
        self.attack_miss_time_ticks = if self.focused && self.attack_down {
            self.attack_miss_time_ticks
                .saturating_sub(client_ticks.min(u32::from(u16::MAX)) as u16)
        } else {
            0
        };
    }

    pub(crate) fn last_input_type(&self) -> LastInputType {
        self.last_input_type
    }

    pub(crate) fn set_narrator_status(&mut self, status: NarratorStatus) {
        self.narrator_status = status;
    }

    pub(crate) fn set_narrator_hotkey_enabled(&mut self, enabled: bool) {
        self.narrator_hotkey_enabled = enabled;
    }

    pub(crate) fn set_narrator_available(&mut self, available: bool) {
        self.narrator_available = available;
    }

    pub(crate) fn set_key_bindings(&mut self, bindings: KeyBindings) {
        self.key_bindings = bindings;
    }

    pub(crate) fn set_toggle_key_options(
        &mut self,
        crouch: bool,
        sprint: bool,
        attack: bool,
        use_item: bool,
    ) {
        self.toggle_crouch = crouch;
        self.toggle_sprint = sprint;
        self.toggle_attack = attack;
        self.toggle_use = use_item;
    }

    /// `ToggleKeyMapping.setDown`: while the matching option is on, only a
    /// press reaches the state and it flips instead of following the key.
    fn toggled_down(&self, needs_toggle: bool, held: bool, pressed: bool, down: bool) -> bool {
        if needs_toggle {
            if pressed {
                !held
            } else {
                held
            }
        } else {
            down
        }
    }

    /// `ToggleKeyMapping.shouldRestoreStateOnScreenClosed`, which only ever
    /// restores a keyboard-bound toggle that a screen released while it was on.
    fn restores_toggle_on_screen_closed(&self, name: &str, needs_toggle: bool, flag: bool) -> bool {
        needs_toggle && flag && matches!(self.key_bindings.input(name), BoundInput::Key(_))
    }

    pub(crate) fn key_bindings(&self) -> &KeyBindings {
        &self.key_bindings
    }

    /// Whether `code` is the physical key `name` currently sits on.
    pub(crate) fn binding_matches(&self, name: &str, code: KeyCode) -> bool {
        self.key_bindings.input(name) == BoundInput::Key(code)
    }

    /// Whether `bound` is the keyboard or mouse input `name` currently sits on.
    pub(crate) fn binding_matches_input(&self, name: &str, bound: BoundInput) -> bool {
        self.key_bindings.input(name) == bound
    }

    fn binding_down(&self, name: &str) -> bool {
        match self.key_bindings.input(name) {
            BoundInput::Key(code) => self.pressed_keys.contains(&code),
            BoundInput::Mouse(MouseButton::Left) => self.mouse_left_down,
            BoundInput::Mouse(MouseButton::Right) => self.mouse_right_down,
            BoundInput::Mouse(MouseButton::Middle) => self.mouse_middle_down,
            BoundInput::Mouse(_) | BoundInput::Unbound => false,
        }
    }

    /// Vanilla `options.keySpectatorOutlines.isDown()` (unbound by default,
    /// live once `options.txt` binds it), synchronized into the world store
    /// for `Minecraft.shouldEntityAppearGlowing`.
    pub(crate) fn spectator_outlines_key_down(&self) -> bool {
        self.binding_down("key.spectatorOutlines")
    }

    pub(crate) fn take_narrator_status_requests(&mut self) -> Vec<NarratorStatusRequest> {
        std::mem::take(&mut self.narrator_status_requests)
    }

    /// `TextInputManager`'s `textInputEnabled`.
    ///
    /// Vanilla starts text input when an `EditBox`/`MultiLineEditBox` takes
    /// focus and stops it when focus leaves or the screen closes, and
    /// `TextInputManager.tick` disables the IME on every tick the window is
    /// focused outside text input. So the IME is on exactly while something
    /// would receive a commit -- which, here, is exactly the set of consumers
    /// `handle_text_input_with_item_runtime` walks. Leaving it on all the time
    /// is not a harmless superset: an input method grabs every letter key, so
    /// `T` composes a preedit instead of opening chat.
    pub(crate) fn text_input_is_active(&self, world: &WorldStore) -> bool {
        if !self.focused {
            return false;
        }
        match self.current_screen() {
            Some(Screen::Social(_)) => self.social_interactions_search_is_focused(),
            Some(Screen::GameRules(_)) => self.game_rules_text_is_focused(),
            Some(
                Screen::StructureBlock(_)
                | Screen::JigsawBlock(_)
                | Screen::TestInstanceBlock(_)
                | Screen::Sign(_)
                | Screen::Dialog(_)
                | Screen::DialogWarning(_)
                | Screen::DialogLink(_)
                | Screen::DialogWaiting(_),
            ) => true,
            Some(Screen::TestBlock(editor)) => editor.message_is_editable(),
            Some(Screen::Book(book)) => book.input.is_some(),
            Some(Screen::CommandEntry(_)) => self.command_entry_is_active(),
            Some(Screen::DebugOptions(_)) => self.debug_options_screen_search_is_focused(),
            Some(Screen::Creative(state)) => world
                .creative_inventory_snapshot()
                .is_some_and(|snapshot| state.search_is_active(snapshot)),
            Some(Screen::Container(_)) => {
                inventory::recipe_book_search_is_active(self, world)
                    || inventory::anvil_screen_is_open(world)
            }
            Some(
                Screen::CommandConfirmation(_)
                | Screen::ChatRestrictions(_)
                | Screen::Pause(_)
                | Screen::GameModeSwitcher(_)
                | Screen::Disconnected(_)
                | Screen::Death(_)
                | Screen::GameEvent(_)
                | Screen::Advancements(_)
                | Screen::Stats(_),
            )
            | None => false,
        }
    }

    /// `KeyboardHandler.keyPress`'s pre-screen press block
    /// on a press, the fullscreen and
    /// screenshot keys are answered before any screen sees the event and
    /// `return` without recording the key. The only screen that defers them is
    /// `KeyBindsScreen`, whose 20 ms rebind guard has no equivalent here
    /// (key-binds UI is a non-goal), so these two are unconditional. Vanilla
    /// validates only the callback's window handle; the platform filters
    /// ordinary unfocused input before a key event reaches this owner.
    pub(crate) fn handle_global_key_press(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
    ) -> bool {
        let PhysicalKey::Code(code) = physical_key else {
            return false;
        };
        let is_fullscreen = self.binding_matches("key.fullscreen", code);
        let is_screenshot = self.binding_matches("key.screenshot", code);
        if !is_fullscreen && !is_screenshot {
            return false;
        }
        if !matches!(state, ElementState::Pressed) {
            self.pressed_keys.remove(&code);
            self.physical_pressed_keys.remove(&code);
            return false;
        }
        // Ignore OS key repeats for fullscreen/screenshot hotkeys while still
        // swallowing them before screen dispatch.
        self.pressed_keys.insert(code);
        if !self.physical_pressed_keys.insert(code) {
            return true;
        }
        if is_fullscreen {
            self.toggle_fullscreen();
            return true;
        }
        if is_screenshot {
            self.screenshot_requests = self.screenshot_requests.saturating_add(1);
            return true;
        }
        false
    }

    pub(crate) fn set_fullscreen(&mut self, fullscreen: bool) {
        self.fullscreen = fullscreen;
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
    pub(crate) fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }

    pub(crate) fn set_mouse_sensitivity(&mut self, sensitivity: f64) {
        self.mouse_sensitivity = MouseSensitivity(sensitivity);
    }

    pub(crate) fn set_mouse_scroll_options(&mut self, discrete: bool, sensitivity: f64) {
        self.discrete_mouse_scroll = discrete;
        self.mouse_wheel_sensitivity = MouseWheelSensitivity(sensitivity);
    }

    pub(crate) fn scale_scroll_delta(&self, delta: MouseScrollDelta) -> MouseScrollDelta {
        let scale = |value: f64| {
            let value = if self.discrete_mouse_scroll && value != 0.0 {
                value.signum()
            } else {
                value
            };
            value * self.mouse_wheel_sensitivity.0
        };
        match delta {
            MouseScrollDelta::LineDelta(x, y) => {
                MouseScrollDelta::LineDelta(scale(f64::from(x)) as f32, scale(f64::from(y)) as f32)
            }
            MouseScrollDelta::PixelDelta(position) if self.discrete_mouse_scroll => {
                MouseScrollDelta::LineDelta(scale(position.x) as f32, scale(position.y) as f32)
            }
            MouseScrollDelta::PixelDelta(position) => {
                MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                    position.x * self.mouse_wheel_sensitivity.0,
                    position.y * self.mouse_wheel_sensitivity.0,
                ))
            }
        }
    }

    pub(crate) fn set_invert_mouse(&mut self, invert_x: bool, invert_y: bool) {
        self.invert_mouse_x = invert_x;
        self.invert_mouse_y = invert_y;
    }

    pub(crate) fn set_mouse_look_live(&mut self, live: bool) {
        self.mouse_look_live = live;
    }

    pub(crate) fn set_qol_extensions(
        &mut self,
        extensions: crate::qol_extensions::ClientExtensionSet,
    ) {
        self.qol_extensions = extensions;
    }

    fn effective_zoom_scale(&self) -> f32 {
        self.zoom_scale.unwrap_or(0.25).clamp(0.10, 0.50)
    }

    pub(crate) fn is_zooming(&self) -> bool {
        self.qol_extensions.zoom
            && self.focused
            && self.mouse_look_live
            && self.pressed_keys.contains(&KeyCode::KeyC)
            && !self.debug_modifier_down
    }

    pub(crate) fn target_zoom_factor(&self) -> f32 {
        if self.is_zooming() {
            self.effective_zoom_scale()
        } else {
            1.0
        }
    }

    pub(crate) fn adjust_zoom_wheel(&mut self, steps: i32) -> bool {
        if !self.is_zooming() || steps == 0 {
            return false;
        }
        self.zoom_scale =
            Some((self.effective_zoom_scale() * 0.85_f32.powi(steps)).clamp(0.10, 0.50));
        true
    }

    /// Vanilla `MouseHandler.handleAccumulatedMovement`/`turnPlayer`: the
    /// wall-clock gap (`mousea`) is sampled every call, but only a grabbed
    /// mouse advances the smooth-camera accumulators or resets them on the
    /// non-smooth branches; ungrabbed frames leave the smooths frozen.
    pub(crate) fn prepare_mouse_turn(&mut self, world: &WorldStore, now: Instant) {
        let mousea = self
            .last_turn_sample_at
            .and_then(|last| now.checked_duration_since(last))
            .unwrap_or_default()
            .as_secs_f64();
        self.last_turn_sample_at = Some(now);
        if !self.mouse_look_live {
            self.pending_turn_delta = None;
            return;
        }
        let zoom_sens = if self.is_zooming() {
            f64::from(self.effective_zoom_scale())
        } else {
            1.0
        };
        let scaled = self.mouse_sensitivity.0 * 0.6 + 0.2;
        let sensitivity_modifier = scaled * scaled * scaled * zoom_sens;
        let sens = sensitivity_modifier * 8.0;
        let (x, y) = if self.smooth_camera {
            (
                self.smooth_turn_x
                    .new_delta_value(self.mouse_delta_x * sens, mousea * sens),
                self.smooth_turn_y
                    .new_delta_value(self.mouse_delta_y * sens, mousea * sens),
            )
        } else if self.camera_type.is_first_person() && world.local_player_is_scoping() {
            self.smooth_turn_x.reset();
            self.smooth_turn_y.reset();
            (
                self.mouse_delta_x * sensitivity_modifier,
                self.mouse_delta_y * sensitivity_modifier,
            )
        } else {
            self.smooth_turn_x.reset();
            self.smooth_turn_y.reset();
            (self.mouse_delta_x * sens, self.mouse_delta_y * sens)
        };
        self.pending_turn_delta = Some((
            if self.invert_mouse_x { -x } else { x },
            if self.invert_mouse_y { -y } else { y },
        ));
    }

    pub(crate) fn clear_accumulated_mouse_turn(&mut self) {
        self.mouse_delta_x = 0.0;
        self.mouse_delta_y = 0.0;
        self.pending_turn_delta = None;
    }

    pub(crate) fn mouse_turn_delta(&self, world: &WorldStore) -> (f64, f64) {
        if let Some(pending) = self.pending_turn_delta {
            return pending;
        }
        let zoom_sens = if self.is_zooming() {
            f64::from(self.effective_zoom_scale())
        } else {
            1.0
        };
        let scaled = self.mouse_sensitivity.0 * 0.6 + 0.2;
        let sensitivity_modifier = scaled * scaled * scaled * zoom_sens;
        let scale = if self.camera_type.is_first_person() && world.local_player_is_scoping() {
            sensitivity_modifier
        } else {
            sensitivity_modifier * 8.0
        };
        let x = self.mouse_delta_x * scale;
        let y = self.mouse_delta_y * scale;
        (
            if self.invert_mouse_x { -x } else { x },
            if self.invert_mouse_y { -y } else { y },
        )
    }

    pub(crate) fn pause_screen_disconnect_requested_or_local_disconnect(&mut self) -> bool {
        std::mem::take(&mut self.local_disconnect_requested)
    }

    pub(crate) fn mark_local_disconnect_requested(&mut self) {
        self.local_disconnect_requested = true;
    }

    pub(crate) fn take_fullscreen_requests(&mut self) -> Vec<bool> {
        std::mem::take(&mut self.fullscreen_requests)
    }

    pub(crate) fn take_screenshot_requests(&mut self) -> u32 {
        std::mem::take(&mut self.screenshot_requests)
    }

    pub(crate) fn push_screenshot_saved_message(
        world: Option<&mut WorldStore>,
        name: &str,
        error: Option<&str>,
    ) {
        let Some(world) = world else { return };
        let runs = match error {
            None => vec![
                StyledTextRun {
                    text: "Saved screenshot as ".to_string(),
                    style: ComponentStyle::default(),
                },
                StyledTextRun {
                    text: name.to_string(),
                    style: ComponentStyle {
                        underlined: Some(true),
                        ..ComponentStyle::default()
                    },
                },
            ],
            Some(error) => vec![StyledTextRun {
                text: format!("Couldn't save screenshot: {error}"),
                style: ComponentStyle::default(),
            }],
        };
        world.push_styled_client_system_chat_message(runs);
    }

    fn toggle_fullscreen(&mut self) {
        self.fullscreen = !self.fullscreen;
        self.fullscreen_requests.push(self.fullscreen);
        self.debug_client_option_writes.push(ClientOptionWrite {
            key: "fullscreen",
            value: ClientOptionValue::Boolean(self.fullscreen),
        });
    }

    pub(crate) fn debug_options_screen_search_is_focused(&self) -> bool {
        self.screen
            .debug_options()
            .is_some_and(|screen| screen.focus == DebugOptionsScreenFocus::Search)
    }

    fn text_entry_consumes_input(&self, _world: &WorldStore) -> bool {
        self.screen.chat_entry().is_some()
            || self.book_input_state().is_some()
            || self.sign_editor_is_open()
            || self.debug_options_screen_search_is_focused()
            || self.structure_block_editor_is_active()
            || self.jigsaw_block_editor_is_active()
            || self.test_block_editor_is_active()
            || self.test_instance_block_editor_is_active()
            || self.game_rules_screen().is_some()
            || self.social_interactions_runtime().is_some()
    }

    fn handle_narrator_hotkey(&mut self, world: &WorldStore) {
        if !self.narrator_hotkey_enabled
            || !self.narrator_available
            || !self.control_down()
            || self.text_entry_consumes_input(world)
        {
            return;
        }
        let was_disabled = self.narrator_status == NarratorStatus::Off;
        self.narrator_status = self.narrator_status.next();
        self.narrator_status_requests.push(NarratorStatusRequest {
            status: self.narrator_status,
            schedule_narration: was_disabled,
        });
        self.debug_client_option_writes.push(ClientOptionWrite {
            key: "narrator",
            value: ClientOptionValue::EnumId(self.narrator_status.id()),
        });
    }

    fn synchronize_last_input_type_screen_edge(&mut self) {
        let open = self.screen.is_open();
        if open && !self.last_input_type_screen_was_open {
            self.last_input_type = LastInputType::None;
        }
        if !open && self.last_input_type_screen_was_open {
            self.restore_toggle_states_on_screen_closed();
        }
        self.last_input_type_screen_was_open = open;
    }

    fn restore_toggle_states_on_screen_closed(&mut self) {
        if self.restores_toggle_on_screen_closed(
            "key.sneak",
            self.toggle_crouch,
            self.sneak_released_by_screen_when_down,
        ) {
            self.sneak = true;
        }
        if self.restores_toggle_on_screen_closed(
            "key.sprint",
            self.toggle_sprint,
            self.sprint_released_by_screen_when_down,
        ) {
            self.sprint_key_down = true;
        }
        if self.restores_toggle_on_screen_closed(
            "key.attack",
            self.toggle_attack,
            self.attack_released_by_screen_when_down,
        ) {
            self.attack_down = true;
            self.destroy_block_held = true;
        }
        self.sneak_released_by_screen_when_down = false;
        self.sprint_released_by_screen_when_down = false;
        self.attack_released_by_screen_when_down = false;
    }

    /// `KeyMapping.resetToggleKeys`: `LocalPlayer.respawn` drops the down
    /// state of every `ToggleKeyMapping` (`reset()` is a plain
    /// `setDown(false)`), so a toggled sneak/sprint/attack/use never survives
    /// into the respawned player.
    fn reset_toggle_keys(&mut self) {
        self.sneak = false;
        self.sprint_key_down = false;
        self.attack_down = false;
        self.use_down = false;
        self.destroy_block_held = false;
        self.use_item_held = false;
        self.mouse_left_down = false;
        self.mouse_right_down = false;
    }

    /// Every `PerformRespawn` producer (death/game-event screen, direct key,
    /// control request, immediate-respawn combat kill) funnels through
    /// `queue_perform_respawn_command`, so the queued-command counter is the
    /// one observation point for vanilla's respawn-time toggle reset.
    pub(crate) fn sync_respawn_toggle_reset(&mut self, counters: &NetCounters) {
        if counters.perform_respawn_commands_queued > self.last_seen_perform_respawn_commands {
            self.reset_toggle_keys();
        }
        self.last_seen_perform_respawn_commands = counters.perform_respawn_commands_queued;
    }

    fn note_key_input_type(&mut self, code: KeyCode) {
        if !self.screen.is_open() {
            return;
        }
        match code {
            KeyCode::Tab => self.last_input_type = LastInputType::KeyboardTab,
            KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::ArrowLeft | KeyCode::ArrowRight => {
                self.last_input_type = LastInputType::KeyboardArrow
            }
            _ => {}
        }
    }

    pub(crate) fn note_mouse_button_input_type(&mut self) {
        if self.screen.is_open() {
            self.last_input_type = LastInputType::Mouse;
        }
    }

    pub(crate) fn new(focused: bool) -> Self {
        Self {
            focused,
            chat_input_inner_width: 1276,
            last_active_at: Some(bbb_client_clock::now()),
            debug_pause_on_lost_focus: true,
            debug_fog_enabled: true,
            debug_smart_cull_enabled: true,
            debug_query_transaction_id: -1,
            ..Self::default()
        }
    }

    pub(crate) fn set_save_chat_drafts(&mut self, enabled: bool) {
        self.save_chat_drafts = enabled;
    }

    fn close_chat_screen(&mut self, reason: ChatScreenExitReason) -> bool {
        let Some(entry) = self.take_chat_screen() else {
            return false;
        };
        self.finalize_chat_entry(entry, reason);
        true
    }

    fn take_chat_screen(&mut self) -> Option<ChatEntryState> {
        self.screen
            .chat_entry()
            .is_some_and(|entry| matches!(entry.owner, CommandEntryKind::Chat(_)))
            .then(|| self.screen.take_chat_entry())
            .flatten()
    }

    fn finalize_chat_entry(&mut self, entry: ChatEntryState, reason: ChatScreenExitReason) {
        let CommandEntryKind::Chat(screen) = &entry.owner else {
            return;
        };
        self.reset_chat_scroll();
        let discard = (reason != ChatScreenExitReason::Interrupted
            && (reason != ChatScreenExitReason::Intentional || !self.save_chat_drafts))
            || is_java_blank(&entry.edit.text());
        if discard {
            self.latest_chat_draft = None;
        } else if !screen.is_draft {
            self.latest_chat_draft = Some(ChatDraftState {
                method: if entry.starts_with_slash() {
                    ChatMethod::Command
                } else {
                    ChatMethod::Message
                },
                text: entry.edit.value().to_vec(),
            });
        }
    }

    pub(crate) fn interrupt_chat_screen(&mut self) -> bool {
        if self.close_chat_screen(ChatScreenExitReason::Interrupted) {
            return true;
        }
        if !self.close_chat_restrictions_screen() {
            return false;
        }
        self.close_chat_screen(ChatScreenExitReason::Interrupted);
        true
    }

    pub(crate) fn open_chat_restrictions_screen(&mut self) -> bool {
        if !self
            .screen
            .chat_entry()
            .is_some_and(|entry| matches!(entry.owner, CommandEntryKind::Chat(_)))
        {
            return false;
        }
        self.reset_chat_scroll();
        self.next_chat_screen_revision = self.next_chat_screen_revision.saturating_add(1);
        self.screen
            .open_child(Screen::ChatRestrictions(ChatRestrictionsScreenState {
                generation: self.next_chat_screen_revision,
            }));
        true
    }

    pub(crate) fn close_chat_restrictions_screen(&mut self) -> bool {
        self.screen
            .close_if(|screen| matches!(screen, Screen::ChatRestrictions(_)))
            .is_some()
    }

    pub(crate) fn chat_restrictions_screen_is_open(&self) -> bool {
        self.screen.chat_restrictions().is_some()
    }

    pub(crate) fn chat_restrictions_screen_generation(&self) -> Option<u64> {
        self.screen
            .chat_restrictions()
            .map(|screen| screen.generation)
    }

    fn queue_local_block_attack_effect(&mut self, effect: LocalBlockAttackEffect) {
        self.pending_local_player_action_particles
            .push(LocalPlayerActionParticleState::BlockAttack(effect));
    }

    fn queue_local_destroy_hit_particles(&mut self, state: LocalDestroyHitParticleState) {
        self.pending_local_player_action_particles
            .push(LocalPlayerActionParticleState::DestroyHit(Box::new(state)));
    }

    fn queue_local_interaction_particle_action(
        &mut self,
        action: bbb_client_world::LocalInteractionParticleAction,
    ) {
        self.pending_local_player_action_particles
            .push(LocalPlayerActionParticleState::Interaction(action));
    }

    pub(crate) fn take_pending_local_player_action_particles(
        &mut self,
    ) -> Vec<LocalPlayerActionParticleState> {
        std::mem::take(&mut self.pending_local_player_action_particles)
    }

    pub(crate) fn set_client_text_filtering_enabled(&mut self, enabled: bool) {
        self.client_text_filtering_enabled = enabled;
    }

    pub(crate) fn client_text_filtering_enabled(&self) -> bool {
        self.client_text_filtering_enabled
    }

    /// Release only held gameplay bindings, matching
    /// `KeyMapping.releaseAll`. Screen-owned focus, text, hover and drag
    /// state remain with the open screen; clearing those after every input
    /// batch makes a focused `EditBox` lose ownership before its next
    /// `CharacterEvent` arrives.
    fn release_gameplay_pressed(&mut self) {
        self.sneak_released_by_screen_when_down |= self.toggle_crouch && self.sneak;
        self.sprint_released_by_screen_when_down |= self.toggle_sprint && self.sprint_key_down;
        self.attack_released_by_screen_when_down |= self.toggle_attack && self.attack_down;
        self.forward = false;
        self.backward = false;
        self.left = false;
        self.right = false;
        self.jump = false;
        self.sneak = false;
        self.sprint_key_down = false;
        self.sprint_start_requested = false;
        self.attack_down = false;
        self.use_down = false;
        self.destroy_block_held = false;
        self.use_item_held = false;
        self.pressed_keys
            .retain(|code| is_tracked_modifier_key(*code));
        self.boat_paddles_active = false;
        self.mouse_left_down = false;
        self.mouse_right_down = false;
        self.mouse_middle_down = false;
        self.attack_miss_time_ticks = 0;
        self.use_item_repeat_delay_ticks = 0;
        self.mouse_delta_x = 0.0;
        self.mouse_delta_y = 0.0;
        self.scroll_accumulated_x = 0.0;
        self.scroll_accumulated_y = 0.0;
        #[cfg(test)]
        {
            self.local_player_movement_tick_accumulator_seconds = 0.0;
        }
        self.last_vehicle_command_at = None;
        self.riding_jump_charge_seconds = None;
        self.clear_sprint_trigger();
        self.creative_flight_jump_trigger_ticks = 0;
        self.creative_flight_jump_trigger_elapsed_seconds = 0.0;
    }

    /// Tear down screen-local transient interaction as well as gameplay
    /// bindings. Terminal screen replacement uses this stronger reset; a
    /// normal `release_active_input` deliberately does not.
    fn clear_pressed(&mut self) {
        self.cancel_chat_input_drag();
        self.release_gameplay_pressed();
        self.bundle_scroll_accumulated_x = 0.0;
        self.bundle_scroll_accumulated_y = 0.0;
        if let Some(container) = self.screen.container_mut() {
            container.clear_pressed();
        }
        self.inventory_cursor_position = None;
        self.recipe_book_search_focused_preference = false;
        self.recipe_book_overlay_preference = None;
        if let Some(creative) = self.screen.creative_mut() {
            creative.cancel_primary_interaction();
        }
        for screen in self.screen.iter_mut() {
            if let Screen::Dialog(dialog) = screen {
                dialog.dialog_last_click_button = None;
                dialog.dialog_last_click_at = None;
                dialog.dialog_last_click_screen_revision = None;
            }
        }
        // Vanilla `KeyMapping.releaseAll` never touches the IME composition:
        // pre-edit state is cleared only by `Minecraft.onTextInputFocusChange`
        // (the focus handoff in `sync_ime_preedit_focus`) or by a newer IME
        // event, and the per-element projections already reject stale state
        // via the dialog screen revision / sign editor signature.
        if let Some(editor) = self.screen.sign_editor_mut() {
            editor.cursor_position = None;
        }
        if let Some(screen) = self.screen.advancements_mut() {
            screen.clear_pressed();
        }
        self.reset_debug_crash_hold();
    }

    fn clear_modifiers(&mut self) {
        self.shift_left_down = false;
        self.shift_right_down = false;
        self.control_left_down = false;
        self.control_right_down = false;
        self.pressed_keys
            .retain(|code| !is_tracked_modifier_key(*code));
        self.physical_pressed_keys.clear();
    }

    pub(crate) fn has_active_gameplay_input(&self) -> bool {
        self.forward
            || self.backward
            || self.left
            || self.right
            || self.jump
            || self.sneak
            || self.sprint_key_down
            || self.sprint_start_requested
            || self.attack_down
            || self.use_down
            || self.destroy_block_held
            || self.use_item_held
            || self
                .pressed_keys
                .iter()
                .any(|code| !is_tracked_modifier_key(*code))
            || self.boat_paddles_active
            || self.mouse_left_down
            || self.mouse_right_down
            || self.mouse_middle_down
            || self.riding_jump_charge_seconds.is_some()
    }

    fn clear_debug_key_state(&mut self) {
        self.debug_modifier_down = false;
        self.debug_modifier_used = false;
        self.screen
            .close_if(|screen| matches!(screen, Screen::GameModeSwitcher(_)));
        self.reset_debug_crash_hold();
    }

    fn set_shift_key(&mut self, code: KeyCode, pressed: bool) {
        match code {
            KeyCode::ShiftLeft => self.shift_left_down = pressed,
            KeyCode::ShiftRight => self.shift_right_down = pressed,
            _ => {}
        }
    }

    pub(crate) fn shift_down(&self) -> bool {
        self.shift_left_down || self.shift_right_down
    }

    /// `InputWithModifiers.isCopy`/`isPaste`/`isCut`/`isSelectAll` all share
    /// this modifier shape, so a text field answers all four the same way. `InputQuirks`
    /// replaces Ctrl with Command/Super on macOS (`EDIT_SHORTCUT_KEY_MODIFIER`
    /// is 8 on macOS and 2 elsewhere), and all four reject Shift and Alt.
    pub(crate) fn text_field_modifiers(&self) -> text_field::TextFieldModifiers {
        text_field::TextFieldModifiers {
            shift: self.shift_down(),
            word: if cfg!(target_os = "macos") {
                self.super_down()
            } else if cfg!(target_arch = "wasm32") {
                self.control_down() || self.super_down()
            } else {
                self.control_down()
            },
            shortcut: self.text_edit_shortcut_down(),
        }
    }

    pub(crate) fn text_edit_shortcut_down(&self) -> bool {
        let shortcut_down = if cfg!(target_os = "macos") {
            self.super_down()
        } else if cfg!(target_arch = "wasm32") {
            self.control_down() || self.super_down()
        } else {
            self.control_down()
        };
        shortcut_down && !self.shift_down() && !self.alt_down()
    }

    pub(crate) fn super_down(&self) -> bool {
        self.pressed_keys.contains(&KeyCode::SuperLeft)
            || self.pressed_keys.contains(&KeyCode::SuperRight)
    }

    pub(crate) fn alt_down(&self) -> bool {
        self.pressed_keys.contains(&KeyCode::AltLeft)
            || self.pressed_keys.contains(&KeyCode::AltRight)
    }

    pub(crate) fn physical_key_down(&self, code: KeyCode) -> bool {
        self.physical_pressed_keys.contains(&code)
    }

    pub(crate) fn set_physical_key_down(&mut self, code: KeyCode, pressed: bool) {
        if pressed {
            self.physical_pressed_keys.insert(code);
            if is_tracked_modifier_key(code) {
                self.pressed_keys.insert(code);
            }
        } else {
            self.physical_pressed_keys.remove(&code);
            if is_tracked_modifier_key(code) {
                self.pressed_keys.remove(&code);
            }
        }
    }

    pub(crate) fn player_list_key_down(&self) -> bool {
        self.binding_down("key.playerlist")
    }

    pub(crate) fn hide_gui(&self) -> bool {
        self.hide_gui
    }

    pub(crate) fn synchronize_camera_post_effect(
        &mut self,
        client_level_generation: u64,
        set_camera_updates_applied: usize,
        effect_id: Option<&'static str>,
    ) {
        let selection = CameraPostEffectSelection {
            client_level_generation,
            set_camera_updates_applied,
            effect_id,
        };
        if self.camera_post_effect_selection == Some(selection) {
            return;
        }
        self.camera_post_effect_selection = Some(selection);
        self.camera_post_effect_id = effect_id;
        self.camera_post_effect_active = effect_id.is_some();
    }

    pub(crate) fn camera_type(&self) -> CameraType {
        self.camera_type
    }

    pub(crate) fn camera_post_effect_id(&self) -> Option<&'static str> {
        self.camera_post_effect_id
    }

    pub(crate) fn camera_post_effect_active(&self) -> bool {
        self.camera_post_effect_active
    }

    fn cycle_camera_type(&mut self, current_camera_effect_id: Option<&'static str>) {
        let previous = self.camera_type;
        self.camera_type = self.camera_type.cycle();
        if previous.is_first_person() == self.camera_type.is_first_person() {
            return;
        }
        self.camera_post_effect_id = if self.camera_type.is_first_person() {
            current_camera_effect_id
        } else {
            None
        };
        self.camera_post_effect_active = self.camera_post_effect_id.is_some();
    }

    fn toggle_camera_post_effect(&mut self) {
        self.camera_post_effect_active = !self.camera_post_effect_active;
    }

    fn set_key_down(&mut self, code: KeyCode, pressed: bool) {
        if pressed {
            self.pressed_keys.insert(code);
            self.physical_pressed_keys.insert(code);
        } else {
            self.pressed_keys.remove(&code);
            self.physical_pressed_keys.remove(&code);
            if code == KeyCode::KeyC {
                self.zoom_scale = None;
            }
        }
    }

    fn set_mouse_button_down(&mut self, button: MouseButton, pressed: bool) {
        match button {
            MouseButton::Left => self.mouse_left_down = pressed,
            MouseButton::Right => self.mouse_right_down = pressed,
            MouseButton::Middle => self.mouse_middle_down = pressed,
            _ => {}
        }
    }

    pub(crate) fn item_model_keybind_context(&self) -> ItemModelKeybindContext {
        ItemModelKeybindContext {
            forward: self.binding_down("key.forward"),
            left: self.binding_down("key.left"),
            backward: self.binding_down("key.back"),
            right: self.binding_down("key.right"),
            jump: self.binding_down("key.jump"),
            sneak: self.sneak,
            sprint: self.sprint_key_down,
            attack: self.attack_down,
            use_item: self.use_down,
            pick_item: self.binding_down("key.pickItem"),
            inventory: self.binding_down("key.inventory"),
            swap_offhand: self.binding_down("key.swapOffhand"),
            drop: self.binding_down("key.drop"),
            chat: self.binding_down("key.chat"),
            command: self.binding_down("key.command"),
            player_list: self.binding_down("key.playerlist"),
            social_interactions: self.binding_down("key.socialInteractions"),
            screenshot: self.binding_down("key.screenshot"),
            toggle_perspective: self.binding_down("key.togglePerspective"),
            fullscreen: self.binding_down("key.fullscreen"),
            advancements: self.binding_down("key.advancements"),
            quick_actions: self.binding_down("key.quickActions"),
            toggle_gui: self.binding_down("key.toggleGui"),
            smooth_camera: self.binding_down("key.smoothCamera"),
            spectator_outlines: self.binding_down("key.spectatorOutlines"),
            toggle_spectator_shader_effects: self.binding_down("key.toggleSpectatorShaderEffects"),
            save_toolbar_activator: self.binding_down("key.saveToolbarActivator"),
            load_toolbar_activator: self.binding_down("key.loadToolbarActivator"),
            spectator_hotbar: self.binding_down("key.spectatorHotbar"),
            hotbar: std::array::from_fn(|slot| self.binding_down(HOTBAR_BINDING_NAMES[slot])),
        }
    }

    fn set_control_key(&mut self, code: KeyCode, pressed: bool) {
        match code {
            KeyCode::ControlLeft => self.control_left_down = pressed,
            KeyCode::ControlRight => self.control_right_down = pressed,
            _ => {}
        }
    }

    pub(crate) fn control_down(&self) -> bool {
        self.control_left_down || self.control_right_down
    }

    pub(crate) fn command_entry_is_active(&self) -> bool {
        self.screen.chat_entry().is_some()
    }

    pub(crate) fn command_suggestion_hud_state(
        &mut self,
        world: &WorldStore,
        measurer: &impl CommandSuggestionTextMeasurer,
        surface_size: PhysicalSize<u32>,
    ) -> Option<CommandSuggestionHudState> {
        let entry = self.screen.chat_entry_mut()?;
        sync_chat_suggestion_list(entry, world);
        update_chat_suggestion_list_rect(entry, measurer, surface_size);
        let syntax = entry.command_completion.as_ref().unwrap().syntax.clone();
        Some(chat_suggestion_hud_state(entry, syntax))
    }

    pub(crate) fn command_suggestion_hud_state_ref(
        &self,
        world: &WorldStore,
        measurer: &impl CommandSuggestionTextMeasurer,
        surface_size: PhysicalSize<u32>,
    ) -> Option<CommandSuggestionHudState> {
        let entry = self.screen.chat_entry()?;
        let revision = world.command_completion_revision();
        if let Some(cached) = entry.command_completion.as_ref().filter(|cached| {
            cached.suggestions_synced
                && cached.contents.as_slice() == entry.edit.value()
                && cached.cursor == entry.edit.cursor()
                && cached.suggestion_cursor == entry.suggestion_cursor
                && cached.revision == revision
        }) {
            let rect = compute_chat_suggestion_list_rect(entry, measurer, surface_size);
            return Some(chat_suggestion_hud_state_with_rect(
                entry,
                cached.syntax.clone(),
                rect,
            ));
        }
        let mut temp = entry.clone();
        sync_chat_suggestion_list(&mut temp, world);
        update_chat_suggestion_list_rect(&mut temp, measurer, surface_size);
        let syntax = temp.command_completion.as_ref().unwrap().syntax.clone();
        Some(chat_suggestion_hud_state(&temp, syntax))
    }

    pub(crate) fn command_suggestions_active(&self) -> bool {
        self.screen.chat_entry().is_some()
    }

    pub(crate) fn chat_scroll(&self) -> bbb_client_render::HudChatScroll {
        self.chat_scroll
    }

    pub(crate) fn scroll_chat_lines(&mut self, delta: i32) {
        self.chat_scroll = self.chat_scroll.scrolled(self.chat_scroll_page, delta);
    }

    /// `ChatComponent.resetChatScroll`, which `ChatScreen.removed` runs whenever
    /// another screen replaces the chat.
    fn reset_chat_scroll(&mut self) {
        self.chat_scroll = bbb_client_render::HudChatScroll::default();
    }

    /// Lines the renderer appended to the wrapped chat; they scroll the page
    /// only while `ChatScreen` is open (397-399).
    pub(crate) fn follow_chat_appended_lines(
        &mut self,
        appended: &bbb_client_render::HudChatAppendedLines,
    ) {
        if self.chat_entry_is_active() {
            self.chat_scroll = self
                .chat_scroll
                .followed(self.chat_scroll_page.lines_per_page, appended);
        }
    }

    /// The renderer's latest wrapped chat page, the bound every later scroll uses.
    pub(crate) fn set_chat_scroll_page(&mut self, page: bbb_client_render::HudChatScrollPage) {
        self.chat_scroll_page = page;
    }

    pub(crate) fn chat_lines_per_page(&self) -> i32 {
        i32::try_from(self.chat_scroll_page.lines_per_page).unwrap_or(i32::MAX)
    }

    #[cfg(test)]
    pub(crate) fn replace_chat_entry_text(&mut self, text: String) -> bool {
        self.replace_chat_entry_java_text_with_measurer(text.into(), &FallbackTextMeasurer)
    }

    pub(crate) fn replace_chat_entry_java_text_with_measurer(
        &mut self,
        text: bbb_protocol::JavaString,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) -> bool {
        let layout = self.chat_input_layout(measurer);
        let Some(entry) = self.screen.chat_entry_mut() else {
            return false;
        };
        let layout = entry.edit_layout(&layout);
        entry.edit.set_value(text.units(), layout);
        if let CommandEntryKind::Chat(screen) = &mut entry.owner {
            screen.is_draft = false;
        }
        entry.suggestion_cursor = entry.edit.cursor();
        entry.invalidate_suggestion_list();
        entry.suggestion_narration = None;
        entry.hidden_suggestion_id = None;
        entry.last_suggestion_request_text = None;
        true
    }

    pub(crate) fn handle_command_suggestion_cursor_moved(
        &mut self,
        world: &WorldStore,
        cursor_position: Option<PhysicalPosition<f64>>,
    ) -> bool {
        let Some(entry) = self.screen.chat_entry_mut() else {
            return false;
        };
        sync_chat_suggestion_list(entry, world);
        let previous = entry.mouse_position;
        entry.mouse_position = cursor_position.and_then(physical_position_floor_i32);
        if previous == entry.mouse_position {
            return true;
        }
        let Some((mouse_x, mouse_y)) = entry.mouse_position else {
            return true;
        };
        let Some(list) = entry.suggestion_list.as_ref() else {
            return true;
        };
        let Some(line) = list
            .rect
            .and_then(|rect| rect.hovered_line(mouse_x, mouse_y))
        else {
            return true;
        };
        let selected = list.offset.saturating_add(line);
        select_chat_suggestion(entry, selected, true);
        true
    }

    pub(crate) fn handle_command_suggestion_mouse_input(
        &mut self,
        world: &WorldStore,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) -> bool {
        let layout = self.chat_input_layout(measurer);
        let Some(entry) = self.screen.chat_entry_mut() else {
            return false;
        };
        sync_chat_suggestion_list(entry, world);
        entry.mouse_position = cursor_position.and_then(physical_position_floor_i32);
        if !matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
            return false;
        }
        let Some((mouse_x, mouse_y)) = entry.mouse_position else {
            return false;
        };
        let Some(list) = entry.suggestion_list.as_ref() else {
            return false;
        };
        let Some(rect) = list.rect.filter(|rect| rect.contains(mouse_x, mouse_y)) else {
            return false;
        };
        let line = usize::try_from((mouse_y - rect.y) / COMMAND_SUGGESTION_LINE_HEIGHT)
            .unwrap_or_default();
        let selected = list.offset.saturating_add(line);
        if selected >= list.sorted_indices.len() {
            return false;
        }
        select_chat_suggestion(entry, selected, true);
        use_selected_chat_suggestion(entry, &layout);
        true
    }

    pub(crate) fn handle_command_suggestion_mouse_wheel(
        &mut self,
        delta: MouseScrollDelta,
    ) -> bool {
        let Some(entry) = self.screen.chat_entry_mut() else {
            return false;
        };
        let line_limit = entry.suggestion_line_limit();
        let Some((mouse_x, mouse_y)) = entry.mouse_position else {
            return false;
        };
        let Some(list) = entry.suggestion_list.as_mut() else {
            return false;
        };
        let Some(_rect) = list.rect.filter(|rect| rect.contains(mouse_x, mouse_y)) else {
            return false;
        };
        let scroll = scroll_delta_lines(delta).1.clamp(-1.0, 1.0);
        let max_offset = list.sorted_indices.len().saturating_sub(line_limit);
        list.offset =
            ((list.offset as f64 - scroll) as isize).clamp(0, max_offset as isize) as usize;
        true
    }

    pub(crate) fn command_suggestion_narration(&self) -> Option<&CommandSuggestionNarrationState> {
        self.screen
            .chat_entry()
            .and_then(|entry| entry.suggestion_narration.as_ref())
    }

    pub(crate) fn chat_entry_is_active(&self) -> bool {
        self.screen
            .chat_entry()
            .is_some_and(|entry| matches!(entry.owner, CommandEntryKind::Chat(_)))
    }

    pub(crate) fn chat_screen_revision(&self) -> u64 {
        if let Some(screen) = self.screen.chat_restrictions() {
            return screen.generation;
        }
        self.screen
            .chat_entry()
            .and_then(|entry| match &entry.owner {
                CommandEntryKind::Chat(screen) => Some(screen.revision),
                CommandEntryKind::CommandBlock(_) | CommandEntryKind::MinecartCommandBlock(_) => {
                    None
                }
            })
            .unwrap_or_default()
    }

    pub(crate) fn chat_screen_status(
        &self,
        visibility: bbb_client_world::ChatVisibilityState,
    ) -> Option<crate::control::ChatScreenStatus> {
        let (entry, generation, screen) =
            if let Some(restrictions) = self.screen.chat_restrictions() {
                let Some(Screen::CommandEntry(previous)) = self.screen.parent() else {
                    return None;
                };
                (
                    previous,
                    restrictions.generation,
                    crate::control::ChatScreenControlKind::Restrictions,
                )
            } else {
                let entry = self.screen.chat_entry()?;
                let CommandEntryKind::Chat(chat) = &entry.owner else {
                    return None;
                };
                (
                    entry,
                    chat.revision,
                    crate::control::ChatScreenControlKind::Chat,
                )
            };
        let CommandEntryKind::Chat(chat) = &entry.owner else {
            return None;
        };
        Some(crate::control::ChatScreenStatus {
            generation,
            screen,
            method: match chat.method {
                ChatMethod::Message => crate::control::ChatMethodControl::Message,
                ChatMethod::Command => crate::control::ChatMethodControl::Command,
            },
            input_utf16_units: entry.edit.value().len(),
            input_text: entry.edit.text(),
            draft: chat.is_draft,
            history_position: chat.history_pos,
            recent_count: self.recent_chat.len(),
            has_latest_draft: self.latest_chat_draft.is_some(),
            save_chat_drafts: self.save_chat_drafts,
            can_send_messages: visibility == bbb_client_world::ChatVisibilityState::Full,
            can_send_commands: visibility != bbb_client_world::ChatVisibilityState::Hidden,
        })
    }

    pub(crate) fn insert_chat_screen_text(
        &mut self,
        text: &bbb_protocol::JavaString,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) -> bool {
        let layout = self.chat_input_layout(measurer);
        let Some(entry) = self.screen.chat_entry_mut() else {
            return false;
        };
        if !matches!(entry.owner, CommandEntryKind::Chat(_)) {
            return false;
        }
        let edited = entry
            .edit
            .insert_units(text.units().iter().copied(), Some(&layout));
        update_chat_entry_after_text_change(self, counters, world, net_commands, edited);
        true
    }
}

impl ClientInputState {
    pub(crate) fn is_focused(&self) -> bool {
        self.focused
    }

    fn advance_creative_flight_jump_trigger(&mut self, dt_seconds: f64) {
        if self.creative_flight_jump_trigger_ticks == 0 {
            self.creative_flight_jump_trigger_elapsed_seconds = 0.0;
            return;
        }

        self.creative_flight_jump_trigger_elapsed_seconds += dt_seconds.max(0.0);
        while self.creative_flight_jump_trigger_elapsed_seconds + f64::EPSILON
            >= CREATIVE_FLIGHT_TICK_SECONDS
            && self.creative_flight_jump_trigger_ticks > 0
        {
            self.creative_flight_jump_trigger_elapsed_seconds -= CREATIVE_FLIGHT_TICK_SECONDS;
            self.creative_flight_jump_trigger_ticks -= 1;
        }

        if self.creative_flight_jump_trigger_ticks == 0 {
            self.creative_flight_jump_trigger_elapsed_seconds = 0.0;
        }
    }

    fn prime_sprint_trigger(&mut self) {
        self.sprint_trigger_ticks = SPRINT_TRIGGER_TICKS;
        self.sprint_trigger_elapsed_seconds = 0.0;
    }

    fn clear_sprint_trigger(&mut self) {
        self.sprint_trigger_ticks = 0;
        self.sprint_trigger_elapsed_seconds = 0.0;
    }

    fn advance_sprint_trigger(&mut self, dt_seconds: f64) {
        if self.sprint_trigger_ticks == 0 {
            self.sprint_trigger_elapsed_seconds = 0.0;
            return;
        }

        self.sprint_trigger_elapsed_seconds += dt_seconds.max(0.0);
        while self.sprint_trigger_elapsed_seconds + f64::EPSILON >= SPRINT_TRIGGER_TICK_SECONDS
            && self.sprint_trigger_ticks > 0
        {
            self.sprint_trigger_elapsed_seconds -= SPRINT_TRIGGER_TICK_SECONDS;
            self.sprint_trigger_ticks -= 1;
        }

        if self.sprint_trigger_ticks == 0 {
            self.sprint_trigger_elapsed_seconds = 0.0;
        }
    }
}

pub(crate) fn release_active_input(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    input.cancel_chat_input_drag();
    let before = player_input_from_state(input);
    let had_boat_paddles = input.boat_paddles_active
        || movement::paddle_boat_state_from_input(input) != (false, false);
    let riding_jump_charge_seconds = input.riding_jump_charge_seconds.take();
    input.release_gameplay_pressed();
    let after = player_input_from_state(input);
    if after != before {
        queue_player_input_command(counters, net_commands, after);
    }
    movement::queue_released_riding_jump_command(
        riding_jump_charge_seconds,
        world,
        counters,
        net_commands,
    );
    mouse::abort_destroy_block(counters, world, net_commands, ProtocolDirection::Down);
    if world.local_player().interaction.using_item {
        queue_zero_pos_player_action_command(
            counters,
            net_commands,
            PlayerActionKind::ReleaseUseItem,
        );
        world.release_local_using_item();
    }
    if had_boat_paddles && world.local_player_root_boat_vehicle_id().is_some() {
        queue_paddle_boat_command(counters, net_commands, false, false);
    }
}

pub(crate) fn handle_focus_change(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    focused: bool,
) {
    if !focused {
        release_active_input(input, world, counters, net_commands);
        input.clear_modifiers();
        input.clear_debug_key_state();
    }
    input.focused = focused;
}

fn physical_position_floor_i32(position: PhysicalPosition<f64>) -> Option<(i32, i32)> {
    (position.x.is_finite() && position.y.is_finite())
        .then(|| (position.x.floor() as i32, position.y.floor() as i32))
}

/// Vanilla `Minecraft.handleKeybinds` hotbar branch: with infinite materials,
/// no open screen, and a preset activator held, the hotbar key routes to
/// `CreativeModeInventoryScreen.handleHotbarLoadOrSave` instead of slot
/// selection, and load wins when both activators are down. Without infinite
/// materials the key falls through to normal slot selection. The no-screen
/// condition is met structurally: this runs after the container/screen early
/// returns.
fn handle_creative_hotbar_preset_key(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    slot: u8,
) -> bool {
    let load_pressed = input.binding_down("key.loadToolbarActivator");
    let save_pressed = input.binding_down("key.saveToolbarActivator");
    if !load_pressed && !save_pressed {
        return false;
    }
    let Some(store) = input.hotbar_preset_store.as_mut() else {
        return false;
    };
    let index = usize::from(slot);
    if load_pressed {
        let items = store.preset(index, world);
        let commands = match world.creative_hotbar_preset_commands(&items) {
            Ok(commands) => commands,
            Err(CreativeHotbarPresetError::NoInfiniteMaterials) => return false,
        };
        match queue_set_creative_mode_slot_commands_atomically(counters, net_commands, &commands) {
            Ok(()) => {
                let applied = world.load_local_creative_hotbar_preset(&items);
                debug_assert!(
                    applied.is_ok(),
                    "hotbar preset permissions changed after queue admission"
                );
            }
            Err(err) => {
                tracing::warn!(?err, "failed to queue hotbar preset load commands");
            }
        }
        true
    } else {
        let items = match world.creative_hotbar_preset_snapshot() {
            Ok(items) => items,
            Err(CreativeHotbarPresetError::NoInfiniteMaterials) => return false,
        };
        if let Err(err) = store.save_preset(index, items, world) {
            tracing::warn!(?err, "failed to save hotbar preset");
        }
        world.set_creative_hotbar_preset_saved_overlay(index);
        true
    }
}

fn is_tracked_modifier_key(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::AltLeft | KeyCode::AltRight | KeyCode::SuperLeft | KeyCode::SuperRight
    )
}

fn maybe_apply_forward_sprint_trigger(input: &mut ClientInputState, world: &WorldStore) {
    movement::maybe_request_double_tap_sprint(input, world);
}

fn maybe_toggle_creative_flight(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    before: PlayerInput,
    after: PlayerInput,
) -> bool {
    if world.local_player_is_spectator() {
        return false;
    }
    if !after.jump || before.jump || !world.local_player().abilities.is_some_and(|a| a.can_fly) {
        return false;
    }

    if world.local_player_root_vehicle_id().is_some()
        && world.local_player_rideable_jumping_vehicle_id().is_none()
    {
        input.creative_flight_jump_trigger_ticks = 0;
        input.creative_flight_jump_trigger_elapsed_seconds = 0.0;
        return false;
    }

    if input.creative_flight_jump_trigger_ticks == 0 {
        input.creative_flight_jump_trigger_ticks = CREATIVE_FLIGHT_JUMP_TRIGGER_TICKS;
        input.creative_flight_jump_trigger_elapsed_seconds = 0.0;
        return false;
    }

    // Vanilla `LocalPlayer.aiStep` (`LocalPlayer.java:837-846`) blocks double-tap
    // creative flight while swimming, and triggers `jumpFromGround()` when
    // toggling flight on while grounded so the post-`aiStep` ground check does
    // not immediately turn flight back off.
    if world.local_player().pose.is_some_and(|pose| pose.swimming) {
        return false;
    }

    let flying = !world.local_player().abilities.is_some_and(|a| a.flying);
    let was_on_ground = world.local_player().pose.is_some_and(|pose| pose.on_ground);
    let toggled = queue_player_abilities_command(counters, world, net_commands, flying);
    if toggled {
        if flying && was_on_ground {
            let sprinting = world.local_player().sprinting;
            world.local_player_jump_from_ground_for_creative_flight(sprinting);
        }
        input.creative_flight_jump_trigger_ticks = 0;
        input.creative_flight_jump_trigger_elapsed_seconds = 0.0;
    }
    toggled
}

fn should_queue_start_fall_flying(
    world: &WorldStore,
    before: PlayerInput,
    after: PlayerInput,
) -> bool {
    after.jump && !before.jump && world.local_player_can_start_fall_flying()
}

#[cfg(test)]
pub(crate) fn handle_text_input(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    text: &str,
) {
    handle_text_input_with_item_runtime(
        input,
        counters,
        world,
        net_commands,
        None,
        text,
        &FallbackTextMeasurer,
    );
}

fn player_input_from_state(input: &ClientInputState) -> PlayerInput {
    PlayerInput {
        forward: input.forward,
        backward: input.backward,
        left: input.left,
        right: input.right,
        jump: input.jump,
        shift: input.sneak,
        sprint: input.sprint_key_down,
    }
}

pub(crate) const PIXELS_PER_SCROLL_LINE: f64 = if cfg!(target_arch = "wasm32") {
    100.0
} else {
    40.0
};

pub(crate) fn scroll_delta_lines(delta: MouseScrollDelta) -> (f64, f64) {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => (f64::from(x), f64::from(y)),
        MouseScrollDelta::PixelDelta(position) => (
            position.x / PIXELS_PER_SCROLL_LINE,
            position.y / PIXELS_PER_SCROLL_LINE,
        ),
    }
}

pub(crate) fn key_code_matches_opening_text(code: KeyCode, text: &str) -> bool {
    let mut chars = text.chars();
    let Some(ch) = chars.next() else {
        return false;
    };
    if chars.next().is_some() {
        return false;
    }
    let ch = ch.to_ascii_lowercase();
    match code {
        KeyCode::KeyA => ch == 'a',
        KeyCode::KeyB => ch == 'b',
        KeyCode::KeyC => ch == 'c',
        KeyCode::KeyD => ch == 'd',
        KeyCode::KeyE => ch == 'e',
        KeyCode::KeyF => ch == 'f',
        KeyCode::KeyG => ch == 'g',
        KeyCode::KeyH => ch == 'h',
        KeyCode::KeyI => ch == 'i',
        KeyCode::KeyJ => ch == 'j',
        KeyCode::KeyK => ch == 'k',
        KeyCode::KeyL => ch == 'l',
        KeyCode::KeyM => ch == 'm',
        KeyCode::KeyN => ch == 'n',
        KeyCode::KeyO => ch == 'o',
        KeyCode::KeyP => ch == 'p',
        KeyCode::KeyQ => ch == 'q',
        KeyCode::KeyR => ch == 'r',
        KeyCode::KeyS => ch == 's',
        KeyCode::KeyT => ch == 't',
        KeyCode::KeyU => ch == 'u',
        KeyCode::KeyV => ch == 'v',
        KeyCode::KeyW => ch == 'w',
        KeyCode::KeyX => ch == 'x',
        KeyCode::KeyY => ch == 'y',
        KeyCode::KeyZ => ch == 'z',
        KeyCode::Digit0 => matches!(ch, '0' | ')'),
        KeyCode::Digit1 => matches!(ch, '1' | '!'),
        KeyCode::Digit2 => matches!(ch, '2' | '@'),
        KeyCode::Digit3 => matches!(ch, '3' | '#'),
        KeyCode::Digit4 => matches!(ch, '4' | '$'),
        KeyCode::Digit5 => matches!(ch, '5' | '%'),
        KeyCode::Digit6 => matches!(ch, '6' | '^'),
        KeyCode::Digit7 => matches!(ch, '7' | '&'),
        KeyCode::Digit8 => matches!(ch, '8' | '*'),
        KeyCode::Digit9 => matches!(ch, '9' | '('),
        KeyCode::Slash => matches!(ch, '/' | '?'),
        KeyCode::Backslash => matches!(ch, '\\' | '|'),
        KeyCode::Minus => matches!(ch, '-' | '_'),
        KeyCode::Equal => matches!(ch, '=' | '+'),
        KeyCode::BracketLeft => matches!(ch, '[' | '{'),
        KeyCode::BracketRight => matches!(ch, ']' | '}'),
        KeyCode::Semicolon => matches!(ch, ';' | ':'),
        KeyCode::Quote => matches!(ch, '\'' | '"'),
        KeyCode::Comma => matches!(ch, ',' | '<'),
        KeyCode::Period => matches!(ch, '.' | '>'),
        KeyCode::Backquote => matches!(ch, '`' | '~'),
        KeyCode::Space => ch == ' ',
        _ => false,
    }
}

#[cfg(test)]
mod tests;
