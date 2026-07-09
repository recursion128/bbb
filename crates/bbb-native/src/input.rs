use std::{
    collections::{BTreeMap, HashSet},
    path::PathBuf,
    time::{Duration, Instant},
};

use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::{
    entity_types::vanilla_entity_resource_id_for_type_id,
    packets::{
        BlockEntityTagQuery, BlockPos as ProtocolBlockPos, ChangeGameModeCommand,
        Direction as ProtocolDirection, EntityDataValueKind, EntityTagQuery, GameType,
        InteractionHand, ItemStackSummary, PlayerActionKind, PlayerCommandAction, PlayerInput,
        RecipeBookType, SeenAdvancements, SignUpdate,
    },
    ComponentClickEvent, ComponentStyle, StyledTextRun, MC_BUILD_TIME, MC_DATA_PACK_FORMAT,
    MC_DATA_VERSION, MC_DATA_VERSION_SERIES, MC_RESOURCE_PACK_FORMAT, MC_STABLE, MC_VERSION,
    PROTOCOL_VERSION,
};
use bbb_world::{
    BlockPos, EntityState, EntityVec3, LocalPlayerInputState, LocalPlayerPoseState,
    TagQueryResponseState, WorldStore,
};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::debug_entries::{
    DebugScreenEntryId, DebugScreenEntryList, DebugScreenEntryStatus, DebugScreenProfile,
};

mod bundle;
mod commands;
mod inventory;
mod mouse;
mod movement;
mod text_edit;

use crate::camera_pose::camera_pose_from_world;
use crate::crosshair::protocol_block_pos_from_world;
use crate::crosshair::{crosshair_target_from_camera_at_partial_tick, CrosshairTarget};
use crate::terrain_runtime::TerrainUploadState;
use bbb_item_model::{ItemModelKeybindContext, NativeItemRuntime};
pub(crate) use bundle::select_bundle_item;
use commands::*;
pub(crate) use commands::{
    queue_block_entity_tag_query_command, queue_change_difficulty_command,
    queue_change_game_mode_command, queue_chat_command, queue_chat_message_command,
    queue_command_suggestion_request, queue_container_button_click_command,
    queue_container_click_command, queue_container_close_request_command,
    queue_container_slot_state_changed_command, queue_edit_book_command,
    queue_entity_tag_query_command, queue_lock_difficulty_command, queue_perform_respawn_command,
    queue_place_recipe_command, queue_player_abilities_command,
    queue_recipe_book_change_settings_command, queue_recipe_book_seen_recipe_command,
    queue_rename_item_command, queue_request_game_rule_values_command, queue_request_stats_command,
    queue_seen_advancements_command, queue_select_trade_command, queue_set_beacon_command,
    queue_set_creative_mode_slot_command, queue_sign_update_command, queue_spectate_entity_command,
    queue_teleport_to_entity_command, queue_vehicle_move_command, select_hotbar_slot,
};
#[cfg(test)]
pub(crate) use inventory::handle_inventory_mouse_input;
pub(crate) use inventory::{
    anvil_rename_entry_consumes_key, handle_inventory_cursor_moved, handle_inventory_key_input,
    handle_inventory_mouse_input_with_item_runtime, handle_inventory_mouse_wheel,
    handle_inventory_text_input, inventory_screen_layout, inventory_screen_layout_for_surface,
    inventory_screen_selected_hotbar_slot_id, maybe_close_narrow_recipe_book,
    recipe_book_button_position, recipe_book_main_gui_offset,
    recipe_book_search_entry_consumes_key, recipe_book_tab_count_for_background,
    recipe_book_type_for_background, recipe_book_type_settings, recipe_book_visible_tab_indices,
    sync_beacon_effect_selection_state, sync_loom_pattern_state_for_hud,
    sync_stonecutter_recipe_scroll_state, InventoryScreenBackground, InventorySlotLayout,
    RECIPE_BOOK_BUTTON_HEIGHT, RECIPE_BOOK_BUTTON_WIDTH, RECIPE_BOOK_FILTER_BUTTON_HEIGHT,
    RECIPE_BOOK_FILTER_BUTTON_WIDTH, RECIPE_BOOK_FILTER_BUTTON_X, RECIPE_BOOK_FILTER_BUTTON_Y,
    RECIPE_BOOK_PAGE_BACKWARD_BUTTON_X, RECIPE_BOOK_PAGE_BUTTON_HEIGHT,
    RECIPE_BOOK_PAGE_BUTTON_WIDTH, RECIPE_BOOK_PAGE_BUTTON_Y, RECIPE_BOOK_PAGE_FORWARD_BUTTON_X,
    RECIPE_BOOK_RECIPE_BUTTON_COLUMNS, RECIPE_BOOK_RECIPE_BUTTON_SIZE, RECIPE_BOOK_RECIPE_BUTTON_X,
    RECIPE_BOOK_RECIPE_BUTTON_Y, RECIPE_BOOK_SEARCH_BOX_HEIGHT, RECIPE_BOOK_SEARCH_BOX_WIDTH,
    RECIPE_BOOK_SEARCH_BOX_X, RECIPE_BOOK_SEARCH_BOX_Y, RECIPE_BOOK_SEARCH_TEXT_X_OFFSET,
    RECIPE_BOOK_SEARCH_TEXT_Y_OFFSET, RECIPE_BOOK_SELECTED_TAB_X_OFFSET, RECIPE_BOOK_TAB_HEIGHT,
    RECIPE_BOOK_TAB_STRIDE_Y, RECIPE_BOOK_TAB_WIDTH, RECIPE_BOOK_TAB_X, RECIPE_BOOK_TAB_Y,
};
pub(crate) use mouse::{
    advance_destroying_block_at_partial_tick, advance_using_item_at_partial_tick,
    handle_mouse_input_at_partial_tick, handle_mouse_motion, handle_mouse_wheel,
};
pub(crate) use movement::advance_player_input;

const CREATIVE_FLIGHT_JUMP_TRIGGER_TICKS: u8 = 7;
const CREATIVE_FLIGHT_TICK_SECONDS: f64 = 0.05;
const SPRINT_TRIGGER_TICKS: u8 = 7;
const SPRINT_TRIGGER_TICK_SECONDS: f64 = 0.05;
const DEBUG_CRASH_TIME: Duration = Duration::from_secs(10);
const DEBUG_CRASH_REPORT_INTERVAL: Duration = Duration::from_secs(1);
const CHAT_ENTRY_MAX_LENGTH: usize = 256;
pub(crate) const DEBUG_DYNAMIC_TEXTURE_DUMP_RELATIVE_PATH: &str = "screenshots/debug";
pub(crate) const DEBUG_PROFILING_RESULTS_RELATIVE_DIR: &str = "debug/profiling";
const VANILLA_DEBUG_FEEDBACK_COLOR: u32 = 0xFFFF55;
const DEBUG_OPTIONS_HEADER_HEIGHT: i32 = 61;
const DEBUG_OPTIONS_FOOTER_HEIGHT: i32 = 33;
const DEBUG_OPTIONS_ROW_WIDTH: i32 = 350;
const DEBUG_OPTIONS_ROW_HEIGHT: i32 = 20;
const DEBUG_OPTIONS_STATUS_BUTTON_WIDTH: i32 = 60;
const DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH: i32 = 120;
const DEBUG_OPTIONS_DONE_BUTTON_WIDTH: i32 = 60;
const DEBUG_OPTIONS_FOOTER_BUTTON_SPACING: i32 = 8;
const DEBUG_OPTIONS_SEARCH_MAX_LENGTH: usize = 32;
const ENTITY_SHARED_FLAGS_DATA_ID: u8 = 0;
const ENTITY_AIR_SUPPLY_DATA_ID: u8 = 1;
const ENTITY_CUSTOM_NAME_DATA_ID: u8 = 2;
const ENTITY_CUSTOM_NAME_VISIBLE_DATA_ID: u8 = 3;
const ENTITY_SILENT_DATA_ID: u8 = 4;
const ENTITY_NO_GRAVITY_DATA_ID: u8 = 5;
const ENTITY_TICKS_FROZEN_DATA_ID: u8 = 7;
const ENTITY_SHARED_FLAG_GLOWING: i8 = 1 << 6;
const ENTITY_DEFAULT_AIR_SUPPLY: i32 = 300;
const ENTITY_DEFAULT_FALL_DISTANCE: f64 = 0.0;
const ENTITY_DEFAULT_FIRE_TICKS: i16 = 0;
const ENTITY_DEFAULT_INVULNERABLE: bool = false;
const ENTITY_DEFAULT_PORTAL_COOLDOWN: i32 = 0;
const SIGN_LINE_MAX_LENGTH: usize = 384;
const BOOK_SCREEN_WIDTH: i32 = 192;
const BOOK_SCREEN_HEIGHT: i32 = 192;
const BOOK_PAGE_BUTTON_Y: i32 = 157;
const BOOK_PAGE_BACK_BUTTON_X: i32 = 43;
const BOOK_PAGE_FORWARD_BUTTON_X: i32 = 116;
const BOOK_PAGE_BUTTON_WIDTH: i32 = 23;
const BOOK_PAGE_BUTTON_HEIGHT: i32 = 13;
const BOOK_MENU_BUTTON_Y: i32 = 194;
const BOOK_MENU_DONE_BUTTON_X: i32 = -4;
const BOOK_MENU_BUTTON_WIDTH: i32 = 200;
const BOOK_MENU_BUTTON_HEIGHT: i32 = 20;
const DEBUG_GAME_MODE_SWITCHER_SLOT_AREA: i32 = 26;
const DEBUG_GAME_MODE_SWITCHER_SLOT_PADDED: i32 = 31;
const DEBUG_GAME_MODE_SWITCHER_ALL_SLOTS_WIDTH: i32 = 4 * DEBUG_GAME_MODE_SWITCHER_SLOT_PADDED - 5;
const DEBUG_GAME_MODE_SWITCHER_SLOT_Y_OFFSET: i32 = 31;
const ADVANCEMENTS_FOOTER_HEIGHT: i32 = 33;
const ADVANCEMENTS_WINDOW_WIDTH: i32 = 252;
const ADVANCEMENTS_WINDOW_HEIGHT: i32 = 140;
const ADVANCEMENTS_DONE_BUTTON_WIDTH: i32 = 200;
const ADVANCEMENTS_DONE_BUTTON_HEIGHT: i32 = 20;
const ADVANCEMENTS_TAB_ABOVE_WIDTH: i32 = 28;
const ADVANCEMENTS_TAB_ABOVE_HEIGHT: i32 = 32;
const ADVANCEMENTS_TAB_ABOVE_MAX: usize = 8;
const ADVANCEMENTS_TAB_BELOW_WIDTH: i32 = 28;
const ADVANCEMENTS_TAB_BELOW_HEIGHT: i32 = 32;
const ADVANCEMENTS_TAB_BELOW_MAX: usize = 8;
const ADVANCEMENTS_TAB_LEFT_WIDTH: i32 = 32;
const ADVANCEMENTS_TAB_LEFT_HEIGHT: i32 = 28;
const ADVANCEMENTS_TAB_LEFT_MAX: usize = 5;
const ADVANCEMENTS_TAB_RIGHT_WIDTH: i32 = 32;
const ADVANCEMENTS_TAB_RIGHT_HEIGHT: i32 = 28;
const ADVANCEMENTS_TAB_RIGHT_MAX: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BookScreenClickTarget {
    Done,
    PreviousPage,
    NextPage,
}

pub(crate) trait DebugClipboard {
    fn set_debug_clipboard_text(&mut self, text: &str) -> bool;
}

struct DebugNetContext<'a> {
    counters: &'a mut NetCounters,
    net_commands: &'a Option<mpsc::Sender<NetCommand>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DebugGameModeSwitcherState {
    selected: GameType,
    first_mouse_position: Option<(i32, i32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DebugPauseScreenState {
    pub(crate) show_pause_menu: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct DebugOptionsScreenState {
    search_text: String,
    search_cursor: usize,
    search_selection: usize,
    scroll_row: usize,
    cursor_position: Option<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DebugOptionsScreenHudState {
    pub(crate) search_text: String,
    pub(crate) search_cursor: usize,
    pub(crate) search_selection: usize,
    pub(crate) rows: Vec<DebugOptionsScreenHudRow>,
    pub(crate) tooltip: Option<DebugOptionsScreenTooltip>,
    pub(crate) scroll_row: usize,
    pub(crate) total_rows: usize,
    pub(crate) visible_rows: usize,
    pub(crate) default_profile_active: bool,
    pub(crate) performance_profile_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DebugOptionsScreenHudRow {
    Category {
        label: String,
    },
    Entry {
        entry: DebugScreenEntryId,
        path: String,
        status: DebugScreenEntryStatus,
        allowed: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DebugOptionsScreenTooltip {
    pub(crate) text: String,
    pub(crate) x: i32,
    pub(crate) y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DebugOptionsScreenRow {
    Category { label: &'static str },
    Entry(DebugScreenEntryId),
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

#[derive(Debug, Clone, Default)]
pub(crate) struct ClientInputState {
    focused: bool,
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    jump: bool,
    sneak: bool,
    sprint: bool,
    destroy_block_held: bool,
    use_item_held: bool,
    shift_left_down: bool,
    shift_right_down: bool,
    control_left_down: bool,
    control_right_down: bool,
    pressed_keys: HashSet<KeyCode>,
    mouse_left_down: bool,
    mouse_right_down: bool,
    mouse_middle_down: bool,
    use_item_repeat_delay_ticks: u8,
    mouse_delta_x: f64,
    mouse_delta_y: f64,
    scroll_accumulated_x: f64,
    scroll_accumulated_y: f64,
    bundle_scroll_accumulated_x: f64,
    bundle_scroll_accumulated_y: f64,
    inventory_hovered_slot: Option<i16>,
    inventory_cursor_position: Option<(i32, i32)>,
    inventory_last_click_slot: Option<i16>,
    inventory_last_click_button_num: Option<i8>,
    inventory_last_click_at: Option<Instant>,
    inventory_quick_craft_button_num: Option<i8>,
    inventory_quick_craft_slots: Vec<i16>,
    stonecutter_recipe_scroll_row: i32,
    stonecutter_recipe_scroll_input_item_id: Option<i32>,
    stonecutter_recipe_scrolling: bool,
    loom_pattern_scroll_row: i32,
    loom_pattern_scrolling: bool,
    loom_pattern_selection_container_id: Option<i32>,
    loom_pattern_selection_dirty: bool,
    loom_selected_pattern_index: Option<i32>,
    beacon_effect_selection_container_id: Option<i32>,
    beacon_effect_selection_dirty: bool,
    beacon_primary_effect: Option<i32>,
    beacon_secondary_effect: Option<i32>,
    anvil_rename_input: Option<AnvilRenameInputSignature>,
    anvil_rename_text: String,
    anvil_rename_cursor: usize,
    anvil_rename_selection: usize,
    anvil_rename_hover_name: String,
    recipe_book_search_text: String,
    recipe_book_search_cursor: usize,
    recipe_book_search_selection: usize,
    recipe_book_search_focused: bool,
    recipe_book_search_suppress_open_key_commit: bool,
    recipe_book_crafting_tab_index: usize,
    recipe_book_furnace_tab_index: usize,
    recipe_book_blast_furnace_tab_index: usize,
    recipe_book_smoker_tab_index: usize,
    recipe_book_crafting_page: usize,
    recipe_book_furnace_page: usize,
    recipe_book_blast_furnace_page: usize,
    recipe_book_smoker_page: usize,
    recipe_book_overlay: Option<RecipeBookOverlayHudState>,
    recipe_book_last_placed_recipe: Option<(i32, i32)>,
    advancement_scroll_deltas: BTreeMap<String, (f64, f64)>,
    advancement_hover_fade: f32,
    advancement_mouse_left_down: bool,
    advancement_is_scrolling: bool,
    debug_entries: DebugScreenEntryList,
    debug_profile_store_path: Option<PathBuf>,
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
    debug_fog_enabled: bool,
    debug_smart_cull_enabled: bool,
    debug_wireframe_enabled: bool,
    debug_frustum_requests: Vec<DebugFrustumRequest>,
    debug_feature_count_requests: Vec<DebugFeatureCountRequest>,
    debug_pause_on_lost_focus: bool,
    debug_resource_pack_reload_requests: u32,
    debug_dynamic_texture_dump_requests: u32,
    debug_profiling_recording: bool,
    debug_profiling_toggle_requests: Vec<DebugProfilingToggleRequest>,
    debug_profiler_chart_navigation_requests: Vec<u8>,
    debug_options_screen_requests: u32,
    debug_pause_without_menu_requests: u32,
    debug_options_screen: Option<DebugOptionsScreenState>,
    debug_pause_screen: Option<DebugPauseScreenState>,
    debug_game_mode_switcher: Option<DebugGameModeSwitcherState>,
    debug_recreate_server_query_requests: Vec<DebugRecreateServerQueryRequest>,
    pending_debug_recreate_server_query: Option<PendingDebugRecreateServerQuery>,
    debug_query_transaction_id: i32,
    debug_crash_started_at: Option<Instant>,
    debug_crash_last_reported_at: Option<Instant>,
    debug_crash_report_count: u32,
    sign_editor: Option<SignEditorInputState>,
    dismissed_sign_editor: Option<SignEditorInputSignature>,
    merchant_trade_scrolling: bool,
    chat_entry: Option<ChatEntryState>,
    last_step: Option<Instant>,
    local_player_movement_tick_accumulator_seconds: f64,
    last_move_command_at: Option<Instant>,
    move_position_reminder_ticks: u32,
    last_move_command_pose: Option<LocalPlayerPoseState>,
    last_paddle_boat_command_at: Option<Instant>,
    riding_jump_charge_seconds: Option<f64>,
    sprint_trigger_ticks: u8,
    sprint_trigger_elapsed_seconds: f64,
    creative_flight_jump_trigger_ticks: u8,
    creative_flight_jump_trigger_elapsed_seconds: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ChatEntryState {
    text: String,
    cursor: usize,
    selection: usize,
    last_suggestion_request_text: Option<String>,
    suppress_open_key_commit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AnvilRenameInputSignature {
    container_id: i32,
    item: ItemStackSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SignEditorInputSignature {
    open_count: usize,
    pos: ProtocolBlockPos,
    is_front_text: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SignEditorInputState {
    signature: SignEditorInputSignature,
    lines: [String; 4],
    line: usize,
    cursor: usize,
    selection: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SignEditorHudState {
    pub(crate) pos: BlockPos,
    pub(crate) is_front_text: bool,
    pub(crate) lines: [String; 4],
    pub(crate) line: usize,
    pub(crate) cursor: usize,
    pub(crate) selection: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RecipeBookSearchHudState {
    pub(crate) text: String,
    pub(crate) cursor: usize,
    pub(crate) selection: usize,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DebugRecreateServerQueryRequest {
    BlockEntityTag {
        transaction_id: i32,
        pos: ProtocolBlockPos,
    },
    EntityTag {
        transaction_id: i32,
        entity_id: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DebugRecreateTarget {
    Block(BlockPos),
    Entity(i32),
}

#[derive(Debug, Clone, PartialEq)]
struct PendingDebugRecreateServerQuery {
    transaction_id: i32,
    target: PendingDebugRecreateServerQueryTarget,
}

#[derive(Debug, Clone, PartialEq)]
enum PendingDebugRecreateServerQueryTarget {
    Block {
        pos: BlockPos,
        description: String,
    },
    Entity {
        entity_type: String,
        position: [f64; 3],
    },
}

impl ClientInputState {
    pub(crate) fn new(focused: bool) -> Self {
        Self {
            focused,
            debug_pause_on_lost_focus: true,
            debug_fog_enabled: true,
            debug_smart_cull_enabled: true,
            debug_query_transaction_id: -1,
            ..Self::default()
        }
    }

    fn clear_pressed(&mut self) {
        self.forward = false;
        self.backward = false;
        self.left = false;
        self.right = false;
        self.jump = false;
        self.sneak = false;
        self.sprint = false;
        self.destroy_block_held = false;
        self.use_item_held = false;
        self.pressed_keys.clear();
        self.mouse_left_down = false;
        self.mouse_right_down = false;
        self.mouse_middle_down = false;
        self.use_item_repeat_delay_ticks = 0;
        self.mouse_delta_x = 0.0;
        self.mouse_delta_y = 0.0;
        self.scroll_accumulated_x = 0.0;
        self.scroll_accumulated_y = 0.0;
        self.bundle_scroll_accumulated_x = 0.0;
        self.bundle_scroll_accumulated_y = 0.0;
        self.inventory_hovered_slot = None;
        self.inventory_cursor_position = None;
        self.inventory_last_click_slot = None;
        self.inventory_last_click_button_num = None;
        self.inventory_last_click_at = None;
        self.inventory_quick_craft_button_num = None;
        self.inventory_quick_craft_slots.clear();
        self.loom_pattern_scrolling = false;
        self.merchant_trade_scrolling = false;
        self.stonecutter_recipe_scrolling = false;
        self.anvil_rename_input = None;
        self.anvil_rename_text.clear();
        self.anvil_rename_cursor = 0;
        self.anvil_rename_selection = 0;
        self.anvil_rename_hover_name.clear();
        self.recipe_book_search_focused = false;
        self.recipe_book_search_suppress_open_key_commit = false;
        self.recipe_book_overlay = None;
        self.advancement_hover_fade = 0.0;
        self.advancement_mouse_left_down = false;
        self.advancement_is_scrolling = false;
        self.reset_debug_crash_hold();
        self.chat_entry = None;
        self.local_player_movement_tick_accumulator_seconds = 0.0;
        self.last_paddle_boat_command_at = None;
        self.riding_jump_charge_seconds = None;
        self.clear_sprint_trigger();
        self.creative_flight_jump_trigger_ticks = 0;
        self.creative_flight_jump_trigger_elapsed_seconds = 0.0;
    }

    fn clear_modifiers(&mut self) {
        self.shift_left_down = false;
        self.shift_right_down = false;
        self.control_left_down = false;
        self.control_right_down = false;
    }

    fn clear_debug_key_state(&mut self) {
        self.debug_modifier_down = false;
        self.debug_modifier_used = false;
        self.debug_game_mode_switcher = None;
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

    fn set_key_down(&mut self, code: KeyCode, pressed: bool) {
        if pressed {
            self.pressed_keys.insert(code);
        } else {
            self.pressed_keys.remove(&code);
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
            forward: self.pressed_keys.contains(&KeyCode::KeyW),
            left: self.pressed_keys.contains(&KeyCode::KeyA),
            backward: self.pressed_keys.contains(&KeyCode::KeyS),
            right: self.pressed_keys.contains(&KeyCode::KeyD),
            jump: self.pressed_keys.contains(&KeyCode::Space),
            sneak: self.pressed_keys.contains(&KeyCode::ShiftLeft),
            sprint: self.pressed_keys.contains(&KeyCode::ControlLeft),
            attack: self.mouse_left_down,
            use_item: self.mouse_right_down,
            pick_item: self.mouse_middle_down,
            inventory: self.pressed_keys.contains(&KeyCode::KeyE),
            swap_offhand: self.pressed_keys.contains(&KeyCode::KeyF),
            drop: self.pressed_keys.contains(&KeyCode::KeyQ),
            chat: self.pressed_keys.contains(&KeyCode::KeyT),
            command: self.pressed_keys.contains(&KeyCode::Slash),
            player_list: self.pressed_keys.contains(&KeyCode::Tab),
            social_interactions: self.pressed_keys.contains(&KeyCode::KeyP),
            screenshot: self.pressed_keys.contains(&KeyCode::F2),
            toggle_perspective: self.pressed_keys.contains(&KeyCode::F5),
            fullscreen: self.pressed_keys.contains(&KeyCode::F11),
            advancements: self.pressed_keys.contains(&KeyCode::KeyL),
            quick_actions: self.pressed_keys.contains(&KeyCode::KeyG),
            toggle_gui: self.pressed_keys.contains(&KeyCode::F1),
            toggle_spectator_shader_effects: self.pressed_keys.contains(&KeyCode::F4),
            save_toolbar_activator: self.pressed_keys.contains(&KeyCode::KeyC),
            load_toolbar_activator: self.pressed_keys.contains(&KeyCode::KeyX),
            spectator_hotbar: self.mouse_middle_down,
            hotbar: [
                self.pressed_keys.contains(&KeyCode::Digit1),
                self.pressed_keys.contains(&KeyCode::Digit2),
                self.pressed_keys.contains(&KeyCode::Digit3),
                self.pressed_keys.contains(&KeyCode::Digit4),
                self.pressed_keys.contains(&KeyCode::Digit5),
                self.pressed_keys.contains(&KeyCode::Digit6),
                self.pressed_keys.contains(&KeyCode::Digit7),
                self.pressed_keys.contains(&KeyCode::Digit8),
                self.pressed_keys.contains(&KeyCode::Digit9),
            ],
        }
    }

    fn set_control_key(&mut self, code: KeyCode, pressed: bool) {
        match code {
            KeyCode::ControlLeft => self.control_left_down = pressed,
            KeyCode::ControlRight => self.control_right_down = pressed,
            _ => {}
        }
    }

    fn control_down(&self) -> bool {
        self.control_left_down || self.control_right_down
    }

    pub(crate) fn command_entry_is_active(&self) -> bool {
        self.chat_entry_is_active()
    }

    pub(crate) fn sign_editor_is_active_or_pending(&self, world: &WorldStore) -> bool {
        self.sign_editor.is_some()
            || sign_editor_signature_from_world(world)
                .is_some_and(|signature| Some(&signature) != self.dismissed_sign_editor.as_ref())
    }

    pub(crate) fn sign_editor_hud_state(&self, world: &WorldStore) -> Option<SignEditorHudState> {
        let current_signature = sign_editor_signature_from_world(world);
        if let Some(editor) = &self.sign_editor {
            if current_signature.as_ref() == Some(&editor.signature) {
                return Some(sign_editor_hud_state_from_editor(editor));
            }
        }

        let signature = current_signature?;
        if self.dismissed_sign_editor.as_ref() == Some(&signature) {
            return None;
        }
        let lines = sign_editor_initial_lines(world);
        let cursor = sign_line_char_len(&lines[0]);
        Some(SignEditorHudState {
            pos: world_block_pos_from_protocol(signature.pos),
            is_front_text: signature.is_front_text,
            lines,
            line: 0,
            cursor,
            selection: cursor,
        })
    }

    pub(crate) fn chat_entry_is_active(&self) -> bool {
        self.chat_entry.is_some()
    }

    pub(crate) fn inventory_hovered_slot(&self) -> Option<i16> {
        self.inventory_hovered_slot
    }

    pub(crate) fn inventory_cursor_position(&self) -> Option<(i32, i32)> {
        self.inventory_cursor_position
    }

    pub(crate) fn inventory_quick_craft_button_num(&self) -> Option<i8> {
        self.inventory_quick_craft_button_num
    }

    pub(crate) fn inventory_quick_craft_slots(&self) -> &[i16] {
        &self.inventory_quick_craft_slots
    }

    pub(crate) fn stonecutter_recipe_scroll_row(&self) -> i32 {
        self.stonecutter_recipe_scroll_row
    }

    pub(crate) fn loom_pattern_scroll_row(&self) -> i32 {
        self.loom_pattern_scroll_row
    }

    pub(crate) fn advancement_scroll_delta(
        &self,
        selected_tab: Option<&str>,
    ) -> Option<(f64, f64)> {
        selected_tab.and_then(|tab| self.advancement_scroll_deltas.get(tab).copied())
    }

    pub(crate) fn update_advancement_hover_fade(&mut self, hovering: bool) {
        if hovering {
            self.advancement_hover_fade = (self.advancement_hover_fade + 0.02).clamp(0.0, 0.3);
        } else {
            self.advancement_hover_fade = (self.advancement_hover_fade - 0.04).clamp(0.0, 1.0);
        }
    }

    pub(crate) fn reset_advancement_hover_fade(&mut self) {
        self.advancement_hover_fade = 0.0;
    }

    pub(crate) fn advancement_hover_fade(&self) -> f32 {
        self.advancement_hover_fade
    }

    pub(crate) fn debug_overlay_visible(&self) -> bool {
        self.debug_entries.is_overlay_visible()
    }

    pub(crate) fn debug_profiler_chart_visible(&self) -> bool {
        self.debug_overlay_visible() && self.debug_profiler_chart_visible
    }

    pub(crate) fn debug_fps_charts_visible(&self) -> bool {
        self.debug_overlay_visible() && self.debug_fps_charts_visible
    }

    pub(crate) fn debug_network_charts_visible(&self) -> bool {
        self.debug_overlay_visible() && self.debug_network_charts_visible
    }

    pub(crate) fn debug_lightmap_texture_visible(&self) -> bool {
        self.debug_overlay_visible() && self.debug_lightmap_texture_visible
    }

    #[cfg(test)]
    pub(crate) fn debug_entity_hitboxes_visible(&self) -> bool {
        self.debug_screen_entry_enabled(DebugScreenEntryId::EntityHitboxes, false)
    }

    #[cfg(test)]
    pub(crate) fn debug_chunk_borders_visible(&self) -> bool {
        self.debug_screen_entry_enabled(DebugScreenEntryId::ChunkBorders, false)
    }

    pub(crate) fn debug_entity_hitboxes_visible_for_world(&self, world: &WorldStore) -> bool {
        self.debug_screen_entry_enabled(
            DebugScreenEntryId::EntityHitboxes,
            world.local_player_has_reduced_debug_info(),
        )
    }

    pub(crate) fn debug_chunk_borders_visible_for_world(&self, world: &WorldStore) -> bool {
        self.debug_screen_entry_enabled(
            DebugScreenEntryId::ChunkBorders,
            world.local_player_has_reduced_debug_info(),
        )
    }

    pub(crate) fn debug_screen_entry_enabled(
        &self,
        entry: DebugScreenEntryId,
        reduced_debug_info: bool,
    ) -> bool {
        self.debug_entries
            .is_currently_enabled(entry, reduced_debug_info)
    }

    #[cfg(test)]
    pub(crate) fn set_debug_screen_entry_status(
        &mut self,
        entry: DebugScreenEntryId,
        status: DebugScreenEntryStatus,
    ) {
        self.set_debug_screen_entry_status_inner(entry, status);
    }

    #[cfg(test)]
    pub(crate) fn debug_screen_entry_status(
        &self,
        entry: DebugScreenEntryId,
    ) -> DebugScreenEntryStatus {
        self.debug_entries.status(entry)
    }

    pub(crate) fn load_debug_screen_profile(&mut self, profile: DebugScreenProfile) {
        self.debug_entries.load_profile(profile);
        self.persist_debug_screen_entries();
    }

    pub(crate) fn set_debug_screen_entries(&mut self, entries: DebugScreenEntryList) {
        self.debug_entries = entries;
    }

    pub(crate) fn set_debug_profile_store_path(&mut self, path: PathBuf) {
        self.debug_profile_store_path = Some(path);
    }

    fn toggle_debug_screen_entry_status(&mut self, entry: DebugScreenEntryId) -> bool {
        let enabled = self.debug_entries.toggle_status(entry);
        self.persist_debug_screen_entries();
        enabled
    }

    fn set_debug_screen_entry_status_inner(
        &mut self,
        entry: DebugScreenEntryId,
        status: DebugScreenEntryStatus,
    ) {
        self.debug_entries.set_status(entry, status);
        self.persist_debug_screen_entries();
    }

    fn persist_debug_screen_entries(&self) {
        let Some(path) = &self.debug_profile_store_path else {
            return;
        };
        if let Err(err) = self.debug_entries.save_to_debug_profile_file(path) {
            tracing::warn!(?err, path = %path.display(), "failed to save debug profile store");
        }
    }

    pub(crate) fn debug_advanced_item_tooltips(&self) -> bool {
        self.debug_advanced_item_tooltips
    }

    pub(crate) fn set_debug_advanced_item_tooltips(&mut self, enabled: bool) {
        self.debug_advanced_item_tooltips = enabled;
    }

    pub(crate) fn debug_show_local_server_entity_hit_boxes(&self) -> bool {
        self.debug_show_local_server_entity_hit_boxes
    }

    pub(crate) fn set_debug_show_local_server_entity_hit_boxes(&mut self, enabled: bool) {
        self.debug_show_local_server_entity_hit_boxes = enabled;
    }

    pub(crate) fn set_debug_hotkeys_enabled(&mut self, enabled: bool) {
        self.debug_hotkeys_enabled = enabled;
    }

    pub(crate) fn set_debug_feature_count_enabled(&mut self, enabled: bool) {
        self.debug_feature_count_enabled = enabled;
    }

    pub(crate) fn debug_fog_enabled(&self) -> bool {
        self.debug_fog_enabled
    }

    pub(crate) fn debug_smart_cull_enabled(&self) -> bool {
        self.debug_smart_cull_enabled
    }

    pub(crate) fn debug_pause_on_lost_focus(&self) -> bool {
        self.debug_pause_on_lost_focus
    }

    pub(crate) fn take_debug_resource_pack_reload_requests(&mut self) -> u32 {
        std::mem::take(&mut self.debug_resource_pack_reload_requests)
    }

    pub(crate) fn take_debug_dynamic_texture_dump_requests(&mut self) -> u32 {
        std::mem::take(&mut self.debug_dynamic_texture_dump_requests)
    }

    pub(crate) fn take_debug_profiling_toggle_requests(
        &mut self,
    ) -> Vec<DebugProfilingToggleRequest> {
        std::mem::take(&mut self.debug_profiling_toggle_requests)
    }

    pub(crate) fn take_debug_profiler_chart_navigation_requests(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.debug_profiler_chart_navigation_requests)
    }

    pub(crate) fn take_debug_options_screen_requests(&mut self) -> u32 {
        std::mem::take(&mut self.debug_options_screen_requests)
    }

    pub(crate) fn take_debug_pause_without_menu_requests(&mut self) -> u32 {
        std::mem::take(&mut self.debug_pause_without_menu_requests)
    }

    pub(crate) fn take_debug_frustum_requests(&mut self) -> Vec<DebugFrustumRequest> {
        std::mem::take(&mut self.debug_frustum_requests)
    }

    pub(crate) fn take_debug_feature_count_requests(&mut self) -> Vec<DebugFeatureCountRequest> {
        std::mem::take(&mut self.debug_feature_count_requests)
    }

    pub(crate) fn open_debug_options_screen(&mut self) {
        self.debug_pause_screen = None;
        self.debug_game_mode_switcher = None;
        self.debug_options_screen = Some(DebugOptionsScreenState::default());
    }

    pub(crate) fn close_debug_options_screen(&mut self) {
        self.debug_options_screen = None;
    }

    pub(crate) fn debug_options_screen_is_open(&self) -> bool {
        self.debug_options_screen.is_some()
    }

    pub(crate) fn debug_options_screen_hud_state(
        &self,
        surface_size: PhysicalSize<u32>,
        reduced_debug_info: bool,
    ) -> Option<DebugOptionsScreenHudState> {
        let screen = self.debug_options_screen.as_ref()?;
        let rows = debug_options_screen_rows(&screen.search_text);
        let total_rows = rows.len();
        let visible_rows = debug_options_visible_row_count(surface_size);
        let scroll_row = screen
            .scroll_row
            .min(debug_options_max_scroll_row(total_rows, visible_rows));
        let rows = rows
            .into_iter()
            .skip(scroll_row)
            .take(visible_rows)
            .map(|row| match row {
                DebugOptionsScreenRow::Category { label } => DebugOptionsScreenHudRow::Category {
                    label: label.to_string(),
                },
                DebugOptionsScreenRow::Entry(entry) => DebugOptionsScreenHudRow::Entry {
                    entry,
                    path: entry.path().to_string(),
                    status: self.debug_entries.status(entry),
                    allowed: entry.is_allowed(reduced_debug_info),
                },
            })
            .collect();
        let tooltip = screen
            .cursor_position
            .and_then(|(mouse_x, mouse_y)| {
                debug_options_not_allowed_tooltip_at(
                    &screen.search_text,
                    scroll_row,
                    mouse_x,
                    mouse_y,
                    surface_size,
                    reduced_debug_info,
                )
            })
            .map(|(x, y)| DebugOptionsScreenTooltip {
                text: "Not visible when debug info is reduced".to_string(),
                x,
                y,
            });
        Some(DebugOptionsScreenHudState {
            search_text: screen.search_text.clone(),
            search_cursor: screen
                .search_cursor
                .min(text_edit::char_len(&screen.search_text)),
            search_selection: screen
                .search_selection
                .min(text_edit::char_len(&screen.search_text)),
            rows,
            tooltip,
            scroll_row,
            total_rows,
            visible_rows,
            default_profile_active: !self
                .debug_entries
                .is_using_profile(DebugScreenProfile::Default),
            performance_profile_active: !self
                .debug_entries
                .is_using_profile(DebugScreenProfile::Performance),
        })
    }

    pub(crate) fn handle_debug_options_screen_key(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
    ) -> bool {
        if self.debug_options_screen.is_none() {
            return false;
        }
        let PhysicalKey::Code(code) = physical_key else {
            return true;
        };
        let pressed = matches!(state, ElementState::Pressed);
        if matches!(code, KeyCode::ShiftLeft | KeyCode::ShiftRight) {
            self.set_shift_key(code, pressed);
        }
        if matches!(code, KeyCode::ControlLeft | KeyCode::ControlRight) {
            self.set_control_key(code, pressed);
        }
        if matches!(code, KeyCode::F3) || self.debug_modifier_down {
            return false;
        }
        if !pressed {
            return true;
        }
        match code {
            KeyCode::Escape => {
                self.set_key_down(code, false);
                self.close_debug_options_screen();
            }
            KeyCode::KeyA if self.control_down() && !self.shift_down() => {
                if let Some(screen) = self.debug_options_screen.as_mut() {
                    select_debug_options_search_text(screen);
                }
            }
            KeyCode::ArrowLeft => {
                let control_down = self.control_down();
                let shift_down = self.shift_down();
                if let Some(screen) = self.debug_options_screen.as_mut() {
                    let cursor = if control_down {
                        text_edit::word_position(&screen.search_text, screen.search_cursor, -1)
                    } else {
                        screen.search_cursor.saturating_sub(1)
                    };
                    move_debug_options_search_cursor(screen, cursor, shift_down);
                }
            }
            KeyCode::ArrowRight => {
                let control_down = self.control_down();
                let shift_down = self.shift_down();
                if let Some(screen) = self.debug_options_screen.as_mut() {
                    let cursor = if control_down {
                        text_edit::word_position(&screen.search_text, screen.search_cursor, 1)
                    } else {
                        (screen.search_cursor + 1).min(text_edit::char_len(&screen.search_text))
                    };
                    move_debug_options_search_cursor(screen, cursor, shift_down);
                }
            }
            KeyCode::Home => {
                let shift_down = self.shift_down();
                if let Some(screen) = self.debug_options_screen.as_mut() {
                    move_debug_options_search_cursor(screen, 0, shift_down);
                }
            }
            KeyCode::End => {
                let shift_down = self.shift_down();
                if let Some(screen) = self.debug_options_screen.as_mut() {
                    let cursor = text_edit::char_len(&screen.search_text);
                    move_debug_options_search_cursor(screen, cursor, shift_down);
                }
            }
            KeyCode::Backspace => {
                let control_down = self.control_down();
                if let Some(screen) = self.debug_options_screen.as_mut() {
                    let before = screen.search_text.clone();
                    let deleted_selection = delete_debug_options_search_selection(screen);
                    if !deleted_selection && control_down {
                        text_edit::remove_word_before_cursor(
                            &mut screen.search_text,
                            &mut screen.search_cursor,
                        );
                        screen.search_selection = screen.search_cursor;
                    } else if !deleted_selection {
                        remove_debug_options_search_char_before_cursor(
                            &mut screen.search_text,
                            &mut screen.search_cursor,
                        );
                        screen.search_selection = screen.search_cursor;
                    }
                    if screen.search_text != before {
                        screen.scroll_row = 0;
                    }
                }
            }
            KeyCode::Delete => {
                let control_down = self.control_down();
                if let Some(screen) = self.debug_options_screen.as_mut() {
                    let before = screen.search_text.clone();
                    let deleted_selection = delete_debug_options_search_selection(screen);
                    if !deleted_selection && control_down {
                        text_edit::remove_word_at_cursor(
                            &mut screen.search_text,
                            screen.search_cursor,
                        );
                        screen.search_selection = screen.search_cursor;
                    } else if !deleted_selection {
                        remove_debug_options_search_char_at_cursor(
                            &mut screen.search_text,
                            screen.search_cursor,
                        );
                        screen.search_selection = screen.search_cursor;
                    }
                    if screen.search_text != before {
                        screen.scroll_row = 0;
                    }
                }
            }
            _ => {}
        }
        true
    }

    pub(crate) fn handle_debug_options_screen_text_input(&mut self, text: &str) -> bool {
        let Some(screen) = self.debug_options_screen.as_mut() else {
            return false;
        };
        insert_debug_options_search_text(screen, text);
        true
    }

    pub(crate) fn handle_debug_options_screen_cursor_moved(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
    ) -> bool {
        let Some(screen) = self.debug_options_screen.as_mut() else {
            return false;
        };
        screen.cursor_position = cursor_position.and_then(physical_position_floor_i32);
        true
    }

    pub(crate) fn handle_debug_options_screen_mouse_input(
        &mut self,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        reduced_debug_info: bool,
    ) -> bool {
        if self.debug_options_screen.is_none() {
            return false;
        }
        if let Some(screen) = self.debug_options_screen.as_mut() {
            screen.cursor_position = cursor_position.and_then(physical_position_floor_i32);
        }
        if !matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
            return true;
        }
        let Some((mouse_x, mouse_y)) = cursor_position.and_then(physical_position_floor_i32) else {
            return true;
        };
        if let Some(profile) = debug_options_profile_button_at(mouse_x, mouse_y, surface_size) {
            if !self.debug_entries.is_using_profile(profile) {
                self.load_debug_screen_profile(profile);
                if let Some(screen) = self.debug_options_screen.as_mut() {
                    screen.scroll_row = 0;
                }
            }
            return true;
        }
        if debug_options_done_button_contains(mouse_x, mouse_y, surface_size) {
            self.close_debug_options_screen();
            return true;
        }
        if let Some((entry, status)) =
            self.debug_options_status_button_at(mouse_x, mouse_y, surface_size, reduced_debug_info)
        {
            self.set_debug_screen_entry_status_inner(entry, status);
        }
        true
    }

    pub(crate) fn handle_debug_options_screen_mouse_wheel(
        &mut self,
        delta: MouseScrollDelta,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let Some(screen) = self.debug_options_screen.as_mut() else {
            return false;
        };
        let rows = debug_options_screen_rows(&screen.search_text);
        let visible_rows = debug_options_visible_row_count(surface_size);
        let max_scroll = debug_options_max_scroll_row(rows.len(), visible_rows);
        let delta_rows = debug_options_scroll_delta_rows(delta);
        if delta_rows < 0 {
            screen.scroll_row = screen
                .scroll_row
                .saturating_add(delta_rows.unsigned_abs() as usize);
        } else if delta_rows > 0 {
            screen.scroll_row = screen.scroll_row.saturating_sub(delta_rows as usize);
        }
        screen.scroll_row = screen.scroll_row.min(max_scroll);
        true
    }

    pub(crate) fn open_debug_pause_screen_without_menu(&mut self) {
        self.debug_options_screen = None;
        self.debug_pause_screen = Some(DebugPauseScreenState {
            show_pause_menu: false,
        });
    }

    pub(crate) fn close_debug_pause_screen(&mut self) {
        self.debug_pause_screen = None;
    }

    pub(crate) fn debug_pause_screen(&self) -> Option<DebugPauseScreenState> {
        self.debug_pause_screen
    }

    pub(crate) fn debug_pause_screen_is_open(&self) -> bool {
        self.debug_pause_screen.is_some()
    }

    pub(crate) fn handle_debug_pause_screen_key(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
    ) -> bool {
        if self.debug_pause_screen.is_none() {
            return false;
        }
        let PhysicalKey::Code(code) = physical_key else {
            return true;
        };
        let pressed = matches!(state, ElementState::Pressed);
        if matches!(code, KeyCode::ShiftLeft | KeyCode::ShiftRight) {
            self.set_shift_key(code, pressed);
        }
        if matches!(code, KeyCode::ControlLeft | KeyCode::ControlRight) {
            self.set_control_key(code, pressed);
        }
        if matches!(state, ElementState::Pressed) && matches!(code, KeyCode::Escape) {
            self.set_key_down(code, false);
            self.close_debug_pause_screen();
            return true;
        }
        if matches!(code, KeyCode::F3) || self.debug_modifier_down {
            return false;
        }
        self.set_key_down(code, false);
        true
    }

    pub(crate) fn debug_game_mode_switcher_selected(&self) -> Option<GameType> {
        self.debug_game_mode_switcher
            .as_ref()
            .map(|switcher| switcher.selected)
    }

    pub(crate) fn debug_game_mode_switcher_is_open(&self) -> bool {
        self.debug_game_mode_switcher.is_some()
    }

    pub(crate) fn handle_debug_game_mode_switcher_cursor_moved(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let Some(switcher) = self.debug_game_mode_switcher.as_mut() else {
            return false;
        };
        let Some((mouse_x, mouse_y)) = cursor_position.and_then(debug_game_mode_switcher_mouse_pos)
        else {
            return true;
        };
        let mouse_pos = (mouse_x, mouse_y);
        let Some(first_mouse_position) = switcher.first_mouse_position else {
            switcher.first_mouse_position = Some(mouse_pos);
            return true;
        };
        if first_mouse_position == mouse_pos {
            return true;
        }
        if let Some(hovered) = debug_game_mode_switcher_hovered_game_type(mouse_pos, surface_size) {
            switcher.selected = hovered;
        }
        true
    }

    fn debug_options_status_button_at(
        &self,
        mouse_x: i32,
        mouse_y: i32,
        surface_size: PhysicalSize<u32>,
        _reduced_debug_info: bool,
    ) -> Option<(DebugScreenEntryId, DebugScreenEntryStatus)> {
        let screen = self.debug_options_screen.as_ref()?;
        let row_index = debug_options_row_index_at(mouse_x, mouse_y, surface_size)?;
        let rows = debug_options_screen_rows(&screen.search_text);
        let visible_rows = debug_options_visible_row_count(surface_size);
        let scroll_row = screen
            .scroll_row
            .min(debug_options_max_scroll_row(rows.len(), visible_rows));
        let DebugOptionsScreenRow::Entry(entry) = *rows.get(scroll_row + row_index)? else {
            return None;
        };
        let buttons_start_x = debug_options_content_x(surface_size) + DEBUG_OPTIONS_ROW_WIDTH
            - DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 3;
        if mouse_x < buttons_start_x {
            return None;
        }
        let button_index =
            ((mouse_x - buttons_start_x) / DEBUG_OPTIONS_STATUS_BUTTON_WIDTH).clamp(0, 2);
        let status = match button_index {
            0 => DebugScreenEntryStatus::Never,
            1 => DebugScreenEntryStatus::InOverlay,
            _ => DebugScreenEntryStatus::AlwaysOn,
        };
        Some((entry, status))
    }

    pub(crate) fn take_debug_recreate_server_query_requests(
        &mut self,
    ) -> Vec<DebugRecreateServerQueryRequest> {
        std::mem::take(&mut self.debug_recreate_server_query_requests)
    }

    pub(crate) fn consume_debug_recreate_server_query_response(
        &mut self,
        world: &mut WorldStore,
        clipboard: &mut dyn DebugClipboard,
    ) -> bool {
        let Some(pending) = self.pending_debug_recreate_server_query.clone() else {
            return false;
        };
        let Some(response) = world.last_tag_query().cloned() else {
            return false;
        };
        if response.transaction_id != pending.transaction_id {
            return false;
        }

        self.pending_debug_recreate_server_query = None;
        let copy = match debug_copy_recreate_server_response_command(&pending.target, &response) {
            Ok(copy) => copy,
            Err(_) => {
                push_debug_feedback_chat_message(
                    Some(world),
                    "Failed to decode server-side recreate data",
                );
                return true;
            }
        };
        let Some(copy) = copy else {
            return true;
        };
        if clipboard.set_debug_clipboard_text(&copy.command) {
            push_debug_feedback_chat_message(Some(world), copy.feedback_message);
        }
        true
    }

    #[cfg(test)]
    pub(crate) fn handle_debug_overlay_key(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
        world: Option<&mut WorldStore>,
        terrain_upload: Option<&mut TerrainUploadState>,
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
        terrain_upload: Option<&mut TerrainUploadState>,
        clipboard: Option<&mut dyn DebugClipboard>,
    ) -> bool {
        self.handle_debug_overlay_key_inner(
            physical_key,
            state,
            world,
            terrain_upload,
            clipboard,
            None,
        )
    }

    pub(crate) fn handle_debug_overlay_key_with_clipboard_and_net(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
        world: Option<&mut WorldStore>,
        terrain_upload: Option<&mut TerrainUploadState>,
        clipboard: Option<&mut dyn DebugClipboard>,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) -> bool {
        self.handle_debug_overlay_key_inner(
            physical_key,
            state,
            world,
            terrain_upload,
            clipboard,
            Some(DebugNetContext {
                counters,
                net_commands,
            }),
        )
    }

    fn handle_debug_overlay_key_inner(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
        mut world: Option<&mut WorldStore>,
        terrain_upload: Option<&mut TerrainUploadState>,
        clipboard: Option<&mut dyn DebugClipboard>,
        mut net_context: Option<DebugNetContext<'_>>,
    ) -> bool {
        if !self.focused {
            return false;
        }
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
                    if let Some(switcher) = self.debug_game_mode_switcher.take() {
                        self.debug_modifier_used = false;
                        if let (Some(world), Some(context)) =
                            (world.as_deref_mut(), net_context.as_mut())
                        {
                            queue_debug_game_mode_switcher_selection(
                                world,
                                context,
                                switcher.selected,
                            );
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
            && self.handle_debug_overlay_modifier_key(
                code,
                world,
                terrain_upload,
                clipboard,
                net_context.as_mut(),
            )
        {
            self.debug_modifier_used = true;
            return true;
        }

        if self.debug_game_mode_switcher.is_some() {
            return true;
        }

        false
    }

    fn handle_debug_overlay_modifier_key(
        &mut self,
        code: KeyCode,
        mut world: Option<&mut WorldStore>,
        mut terrain_upload: Option<&mut TerrainUploadState>,
        mut clipboard: Option<&mut dyn DebugClipboard>,
        net_context: Option<&mut DebugNetContext<'_>>,
    ) -> bool {
        if self.debug_hotkeys_enabled && self.handle_shared_debug_hotkey(code, world.as_deref_mut())
        {
            return true;
        }
        if self.debug_feature_count_enabled && self.handle_debug_feature_count_key(code) {
            return true;
        }
        match code {
            KeyCode::Escape => {
                self.debug_pause_without_menu_requests =
                    self.debug_pause_without_menu_requests.saturating_add(1);
                true
            }
            KeyCode::KeyA => {
                let Some(terrain_upload) = terrain_upload.as_deref_mut() else {
                    return false;
                };
                terrain_upload.request_reload_all_chunks();
                push_debug_feedback_chat_message(world.as_deref_mut(), "Reloading all chunks");
                true
            }
            KeyCode::Digit1 => {
                self.toggle_debug_profiler_chart();
                true
            }
            KeyCode::Digit2 => {
                self.toggle_debug_fps_charts();
                true
            }
            KeyCode::Digit3 => {
                self.toggle_debug_network_charts();
                true
            }
            KeyCode::Digit4 => {
                self.toggle_debug_lightmap_texture();
                true
            }
            KeyCode::KeyD => {
                let Some(world) = world.as_deref_mut() else {
                    return false;
                };
                world.clear_client_chat_display_messages();
                true
            }
            KeyCode::KeyB => {
                if !Self::debug_world_status_toggles_allowed(world.as_deref()) {
                    return false;
                }
                let enabled =
                    self.toggle_debug_screen_entry_status(DebugScreenEntryId::EntityHitboxes);
                push_debug_feedback_chat_message(
                    world.as_deref_mut(),
                    if enabled {
                        "Hitboxes: shown"
                    } else {
                        "Hitboxes: hidden"
                    },
                );
                true
            }
            KeyCode::KeyG => {
                if !Self::debug_world_status_toggles_allowed(world.as_deref()) {
                    return false;
                }
                let enabled =
                    self.toggle_debug_screen_entry_status(DebugScreenEntryId::ChunkBorders);
                push_debug_feedback_chat_message(
                    world.as_deref_mut(),
                    if enabled {
                        "Chunk borders: shown"
                    } else {
                        "Chunk borders: hidden"
                    },
                );
                true
            }
            KeyCode::KeyH => {
                self.debug_advanced_item_tooltips = !self.debug_advanced_item_tooltips;
                push_debug_feedback_chat_message(
                    world.as_deref_mut(),
                    if self.debug_advanced_item_tooltips {
                        "Advanced tooltips: shown"
                    } else {
                        "Advanced tooltips: hidden"
                    },
                );
                true
            }
            KeyCode::KeyP => {
                self.debug_pause_on_lost_focus = !self.debug_pause_on_lost_focus;
                push_debug_feedback_chat_message(
                    world.as_deref_mut(),
                    if self.debug_pause_on_lost_focus {
                        "Pause on lost focus: enabled"
                    } else {
                        "Pause on lost focus: disabled"
                    },
                );
                true
            }
            KeyCode::KeyN => {
                let Some(world) = world.as_deref_mut() else {
                    return true;
                };
                if !world.local_player_has_gamemaster_permission() {
                    push_debug_feedback_chat_message(
                        Some(world),
                        "Unable to switch game mode; no permission",
                    );
                } else if let Some(context) = net_context {
                    queue_change_game_mode_command(
                        context.counters,
                        context.net_commands,
                        ChangeGameModeCommand {
                            game_mode: debug_spectate_target_game_mode(world),
                        },
                    );
                }
                true
            }
            KeyCode::F4 => {
                if let Some(switcher) = self.debug_game_mode_switcher.as_mut() {
                    switcher.selected = next_debug_game_mode_icon(switcher.selected);
                    switcher.first_mouse_position = None;
                    return true;
                }
                let Some(world) = world.as_deref_mut() else {
                    return false;
                };
                if !self.debug_game_mode_switcher_can_open(world) {
                    return false;
                }
                if !world.local_player_has_gamemaster_permission() {
                    push_debug_feedback_chat_message(
                        Some(world),
                        "Unable to open game mode switcher; no permission",
                    );
                } else {
                    self.debug_game_mode_switcher = Some(DebugGameModeSwitcherState {
                        selected: default_debug_game_mode_switcher_selection(world),
                        first_mouse_position: None,
                    });
                }
                true
            }
            KeyCode::KeyV => {
                if let Some(world) = world.as_deref_mut() {
                    push_debug_version_chat_messages(world);
                }
                true
            }
            KeyCode::KeyT => {
                self.debug_resource_pack_reload_requests =
                    self.debug_resource_pack_reload_requests.saturating_add(1);
                push_debug_feedback_chat_message(world.as_deref_mut(), "Reloaded resource packs");
                true
            }
            KeyCode::KeyS => {
                self.debug_dynamic_texture_dump_requests =
                    self.debug_dynamic_texture_dump_requests.saturating_add(1);
                push_debug_dynamic_texture_dump_feedback(world.as_deref_mut());
                true
            }
            KeyCode::KeyI => {
                let pull_from_server = !self.shift_down();
                let add_nbt = world
                    .as_deref()
                    .is_some_and(WorldStore::local_player_has_gamemaster_permission);
                if add_nbt && pull_from_server {
                    if let Some(world) = world.as_deref() {
                        if let Some(target) = debug_recreate_target(world) {
                            let transaction_id = self.next_debug_query_transaction_id();
                            if let Some(pending) =
                                pending_debug_recreate_server_query(world, target, transaction_id)
                            {
                                self.pending_debug_recreate_server_query = Some(pending);
                                self.debug_recreate_server_query_requests.push(
                                    debug_recreate_server_query_request(target, transaction_id),
                                );
                            }
                        }
                    }
                } else {
                    let command = world
                        .as_deref()
                        .and_then(|world| debug_copy_recreate_command(world, add_nbt));
                    if let (Some(copy), Some(clipboard)) = (command, clipboard.as_deref_mut()) {
                        if clipboard.set_debug_clipboard_text(&copy.command) {
                            push_debug_feedback_chat_message(
                                world.as_deref_mut(),
                                copy.feedback_message,
                            );
                        }
                    }
                }
                true
            }
            KeyCode::KeyL => {
                if self.debug_profiling_recording {
                    self.debug_profiling_recording = false;
                    self.debug_profiling_toggle_requests
                        .push(DebugProfilingToggleRequest::Stop);
                    push_debug_profiling_stop_feedback(world.as_deref_mut());
                } else {
                    self.debug_profiling_recording = true;
                    self.debug_profiling_toggle_requests
                        .push(DebugProfilingToggleRequest::Start);
                    push_debug_feedback_chat_message(
                        world.as_deref_mut(),
                        "Profiling started for 10 seconds. Use F3 + L to stop early",
                    );
                }
                true
            }
            KeyCode::F6 => {
                self.debug_options_screen_requests =
                    self.debug_options_screen_requests.saturating_add(1);
                true
            }
            KeyCode::KeyC => {
                if let (Some(command), Some(clipboard)) = (
                    world.as_deref().and_then(debug_copy_location_command),
                    clipboard.as_deref_mut(),
                ) {
                    if clipboard.set_debug_clipboard_text(&command) {
                        push_debug_feedback_chat_message(
                            world.as_deref_mut(),
                            "Copied location to clipboard",
                        );
                    }
                }
                // Vanilla shares C between copy-location and the manual crash key,
                // so F3+C still counts as a debug action even when no copy happens.
                true
            }
            _ => false,
        }
    }

    fn handle_shared_debug_hotkey(
        &mut self,
        code: KeyCode,
        mut world: Option<&mut WorldStore>,
    ) -> bool {
        match code {
            KeyCode::KeyE => {
                let Some(world) = world.as_deref_mut() else {
                    return false;
                };
                if world.local_player_id().is_none() {
                    return false;
                }
                let enabled =
                    self.toggle_debug_screen_entry_status(DebugScreenEntryId::ChunkSectionPaths);
                push_debug_feedback_chat_message(
                    Some(world),
                    if enabled {
                        "SectionPath: shown"
                    } else {
                        "SectionPath: hidden"
                    },
                );
                true
            }
            KeyCode::KeyF => {
                self.debug_fog_enabled = !self.debug_fog_enabled;
                push_debug_feedback_chat_message(
                    world.as_deref_mut(),
                    if self.debug_fog_enabled {
                        "Fog: enabled"
                    } else {
                        "Fog: disabled"
                    },
                );
                true
            }
            KeyCode::KeyL => {
                self.debug_smart_cull_enabled = !self.debug_smart_cull_enabled;
                push_debug_feedback_chat_message(
                    world.as_deref_mut(),
                    if self.debug_smart_cull_enabled {
                        "SmartCull: enabled"
                    } else {
                        "SmartCull: disabled"
                    },
                );
                true
            }
            KeyCode::KeyO => {
                let Some(world) = world.as_deref_mut() else {
                    return false;
                };
                if world.local_player_id().is_none() {
                    return false;
                }
                let enabled =
                    self.toggle_debug_screen_entry_status(DebugScreenEntryId::ChunkSectionOctree);
                push_debug_feedback_chat_message(
                    Some(world),
                    if enabled {
                        "Frustum culling Octree: enabled"
                    } else {
                        "Frustum culling Octree: disabled"
                    },
                );
                true
            }
            KeyCode::KeyU => {
                let (request, feedback) = if self.shift_down() {
                    (DebugFrustumRequest::Kill, "Killed frustum")
                } else {
                    (DebugFrustumRequest::Capture, "Captured frustum")
                };
                self.debug_frustum_requests.push(request);
                push_debug_feedback_chat_message(world.as_deref_mut(), feedback);
                true
            }
            KeyCode::KeyV => {
                let Some(world) = world.as_deref_mut() else {
                    return false;
                };
                if world.local_player_id().is_none() {
                    return false;
                }
                let enabled = self
                    .toggle_debug_screen_entry_status(DebugScreenEntryId::ChunkSectionVisibility);
                push_debug_feedback_chat_message(
                    Some(world),
                    if enabled {
                        "SectionVisibility: enabled"
                    } else {
                        "SectionVisibility: disabled"
                    },
                );
                true
            }
            KeyCode::KeyW => {
                self.debug_wireframe_enabled = !self.debug_wireframe_enabled;
                push_debug_feedback_chat_message(
                    world.as_deref_mut(),
                    if self.debug_wireframe_enabled {
                        "WireFrame: enabled"
                    } else {
                        "WireFrame: disabled"
                    },
                );
                true
            }
            _ => false,
        }
    }

    fn handle_debug_feature_count_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::KeyL => {
                self.debug_feature_count_requests
                    .push(DebugFeatureCountRequest::Log);
                true
            }
            KeyCode::KeyR => {
                self.debug_feature_count_requests
                    .push(DebugFeatureCountRequest::Clear);
                true
            }
            _ => false,
        }
    }

    fn reset_debug_crash_hold(&mut self) {
        self.debug_crash_started_at = None;
        self.debug_crash_last_reported_at = None;
        self.debug_crash_report_count = 0;
    }

    fn advance_debug_crash_hold(&mut self, world: &mut WorldStore, now: Instant) {
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
            push_debug_feedback_chat_message(
                Some(world),
                &format!("Crashing in {remaining_seconds}..."),
            );
        }

        self.debug_crash_last_reported_at = Some(now);
        self.debug_crash_report_count = self.debug_crash_report_count.saturating_add(1);
    }

    fn debug_world_status_toggles_allowed(world: Option<&WorldStore>) -> bool {
        world.is_some_and(|world| {
            world.local_player_id().is_some() && !world.local_player_has_reduced_debug_info()
        })
    }

    fn debug_game_mode_switcher_can_open(&self, world: &WorldStore) -> bool {
        world.level_info().is_some() && !self.debug_game_mode_switcher_blocked_by_screen(world)
    }

    fn debug_game_mode_switcher_blocked_by_screen(&self, world: &WorldStore) -> bool {
        self.command_entry_is_active()
            || self.sign_editor_is_active_or_pending(world)
            || world.open_container_id().is_some()
            || world.current_dialog().is_some()
            || world.current_book().is_some()
            || world.advancements_screen_is_open()
    }

    fn next_debug_query_transaction_id(&mut self) -> i32 {
        self.debug_query_transaction_id = self.debug_query_transaction_id.saturating_add(1);
        self.debug_query_transaction_id
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

    fn record_debug_profiler_chart_navigation_key(&mut self, code: KeyCode) {
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

fn debug_options_screen_rows(search: &str) -> Vec<DebugOptionsScreenRow> {
    let mut rows = Vec::new();
    let mut current_category = None;
    for entry in DebugScreenEntryId::all_debug_options_ordered()
        .iter()
        .copied()
        .filter(|entry| entry.path().contains(search))
    {
        let category = entry.category_label();
        if current_category != Some(category) {
            rows.push(DebugOptionsScreenRow::Category { label: category });
            current_category = Some(category);
        }
        rows.push(DebugOptionsScreenRow::Entry(entry));
    }
    rows
}

fn insert_debug_options_search_text(screen: &mut DebugOptionsScreenState, text: &str) {
    let before = screen.search_text.clone();
    delete_debug_options_search_selection(screen);
    screen.search_cursor = screen
        .search_cursor
        .min(text_edit::char_len(&screen.search_text));
    let mut remaining = DEBUG_OPTIONS_SEARCH_MAX_LENGTH
        .saturating_sub(debug_options_search_len(&screen.search_text));
    for ch in text.chars().filter(|ch| is_chat_text_char(*ch)) {
        let len = ch.len_utf16();
        if len > remaining {
            break;
        }
        let insert_at = text_edit::byte_index(&screen.search_text, screen.search_cursor);
        screen.search_text.insert(insert_at, ch);
        screen.search_cursor += 1;
        remaining -= len;
    }
    screen.search_selection = screen.search_cursor;
    if screen.search_text != before {
        screen.scroll_row = 0;
    }
}

fn move_debug_options_search_cursor(
    screen: &mut DebugOptionsScreenState,
    cursor: usize,
    keep_selection: bool,
) {
    screen.search_cursor = cursor.min(text_edit::char_len(&screen.search_text));
    if !keep_selection {
        screen.search_selection = screen.search_cursor;
    }
}

fn select_debug_options_search_text(screen: &mut DebugOptionsScreenState) {
    screen.search_selection = 0;
    screen.search_cursor = text_edit::char_len(&screen.search_text);
}

fn delete_debug_options_search_selection(screen: &mut DebugOptionsScreenState) -> bool {
    if screen.search_selection == screen.search_cursor {
        return false;
    }
    let start = screen.search_selection.min(screen.search_cursor);
    let end = screen.search_selection.max(screen.search_cursor);
    let start_byte = text_edit::byte_index(&screen.search_text, start);
    let end_byte = text_edit::byte_index(&screen.search_text, end);
    screen.search_text.replace_range(start_byte..end_byte, "");
    screen.search_cursor = start;
    screen.search_selection = start;
    true
}

fn remove_debug_options_search_char_before_cursor(current: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let start = text_edit::byte_index(current, *cursor - 1);
    let end = text_edit::byte_index(current, *cursor);
    current.replace_range(start..end, "");
    *cursor -= 1;
}

fn remove_debug_options_search_char_at_cursor(current: &mut String, cursor: usize) {
    if cursor >= text_edit::char_len(current) {
        return;
    }
    let start = text_edit::byte_index(current, cursor);
    let end = text_edit::byte_index(current, cursor + 1);
    current.replace_range(start..end, "");
}

fn debug_options_search_len(text: &str) -> usize {
    text.encode_utf16().count()
}

fn debug_options_visible_row_count(surface_size: PhysicalSize<u32>) -> usize {
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    ((height - DEBUG_OPTIONS_HEADER_HEIGHT - DEBUG_OPTIONS_FOOTER_HEIGHT)
        / DEBUG_OPTIONS_ROW_HEIGHT)
        .max(0) as usize
}

fn debug_options_max_scroll_row(total_rows: usize, visible_rows: usize) -> usize {
    total_rows.saturating_sub(visible_rows)
}

fn debug_options_content_x(surface_size: PhysicalSize<u32>) -> i32 {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    width / 2 - DEBUG_OPTIONS_ROW_WIDTH / 2
}

fn debug_options_row_index_at(
    mouse_x: i32,
    mouse_y: i32,
    surface_size: PhysicalSize<u32>,
) -> Option<usize> {
    let x = debug_options_content_x(surface_size);
    if mouse_x < x || mouse_x >= x + DEBUG_OPTIONS_ROW_WIDTH {
        return None;
    }
    if mouse_y < DEBUG_OPTIONS_HEADER_HEIGHT {
        return None;
    }
    let row_index = (mouse_y - DEBUG_OPTIONS_HEADER_HEIGHT) / DEBUG_OPTIONS_ROW_HEIGHT;
    (row_index >= 0 && (row_index as usize) < debug_options_visible_row_count(surface_size))
        .then_some(row_index as usize)
}

fn debug_options_not_allowed_tooltip_at(
    search_text: &str,
    scroll_row: usize,
    mouse_x: i32,
    mouse_y: i32,
    surface_size: PhysicalSize<u32>,
    reduced_debug_info: bool,
) -> Option<(i32, i32)> {
    let row_index = debug_options_row_index_at(mouse_x, mouse_y, surface_size)?;
    let rows = debug_options_screen_rows(search_text);
    let visible_rows = debug_options_visible_row_count(surface_size);
    let scroll_row = scroll_row.min(debug_options_max_scroll_row(rows.len(), visible_rows));
    let DebugOptionsScreenRow::Entry(entry) = *rows.get(scroll_row + row_index)? else {
        return None;
    };
    if entry.is_allowed(reduced_debug_info) {
        return None;
    }
    let buttons_start_x = debug_options_content_x(surface_size) + DEBUG_OPTIONS_ROW_WIDTH
        - DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 3;
    (mouse_x < buttons_start_x).then_some((mouse_x, mouse_y))
}

fn debug_options_footer_button_y(surface_size: PhysicalSize<u32>) -> i32 {
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    height - DEBUG_OPTIONS_FOOTER_HEIGHT + 6
}

fn debug_options_footer_button_xs(surface_size: PhysicalSize<u32>) -> (i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let total_width = DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH * 2
        + DEBUG_OPTIONS_DONE_BUTTON_WIDTH
        + DEBUG_OPTIONS_FOOTER_BUTTON_SPACING * 2;
    let default_x = width / 2 - total_width / 2;
    let performance_x =
        default_x + DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH + DEBUG_OPTIONS_FOOTER_BUTTON_SPACING;
    let done_x =
        performance_x + DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH + DEBUG_OPTIONS_FOOTER_BUTTON_SPACING;
    (default_x, performance_x, done_x)
}

fn debug_options_profile_button_at(
    mouse_x: i32,
    mouse_y: i32,
    surface_size: PhysicalSize<u32>,
) -> Option<DebugScreenProfile> {
    let y = debug_options_footer_button_y(surface_size);
    if mouse_y < y || mouse_y >= y + DEBUG_OPTIONS_ROW_HEIGHT {
        return None;
    }
    let (default_x, performance_x, _) = debug_options_footer_button_xs(surface_size);
    if mouse_x >= default_x && mouse_x < default_x + DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH {
        return Some(DebugScreenProfile::Default);
    }
    if mouse_x >= performance_x && mouse_x < performance_x + DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH {
        return Some(DebugScreenProfile::Performance);
    }
    None
}

fn debug_options_done_button_contains(
    mouse_x: i32,
    mouse_y: i32,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let y = debug_options_footer_button_y(surface_size);
    let (_, _, done_x) = debug_options_footer_button_xs(surface_size);
    mouse_x >= done_x
        && mouse_x < done_x + DEBUG_OPTIONS_DONE_BUTTON_WIDTH
        && mouse_y >= y
        && mouse_y < y + DEBUG_OPTIONS_ROW_HEIGHT
}

fn debug_options_scroll_delta_rows(delta: MouseScrollDelta) -> i32 {
    match delta {
        MouseScrollDelta::LineDelta(_, y) => y.round() as i32,
        MouseScrollDelta::PixelDelta(position) => {
            (position.y / f64::from(DEBUG_OPTIONS_ROW_HEIGHT))
                .round()
                .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
        }
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

fn debug_game_mode_switcher_mouse_pos(position: PhysicalPosition<f64>) -> Option<(i32, i32)> {
    (position.x.is_finite() && position.y.is_finite())
        .then(|| (position.x.floor() as i32, position.y.floor() as i32))
}

fn debug_game_mode_switcher_hovered_game_type(
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

fn push_debug_feedback_chat_message(world: Option<&mut WorldStore>, message: &str) {
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
                        path: DEBUG_DYNAMIC_TEXTURE_DUMP_RELATIVE_PATH.to_string(),
                    }),
                    ..ComponentStyle::default()
                },
            },
        ],
    );
}

fn push_debug_profiling_stop_feedback(world: Option<&mut WorldStore>) {
    push_debug_feedback_chat_runs(
        world,
        vec![
            StyledTextRun {
                text: "Profiling ended. Results folder ".to_string(),
                style: ComponentStyle::default(),
            },
            StyledTextRun {
                text: DEBUG_PROFILING_RESULTS_RELATIVE_DIR.to_string(),
                style: ComponentStyle {
                    underlined: Some(true),
                    click_event: Some(ComponentClickEvent::OpenFile {
                        path: DEBUG_PROFILING_RESULTS_RELATIVE_DIR.to_string(),
                    }),
                    ..ComponentStyle::default()
                },
            },
        ],
    );
}

fn push_debug_feedback_chat_runs(world: Option<&mut WorldStore>, body_runs: Vec<StyledTextRun>) {
    if let Some(world) = world {
        world.push_styled_client_system_chat_message(debug_feedback_styled_runs(body_runs));
    }
}

fn debug_feedback_styled_runs(body_runs: Vec<StyledTextRun>) -> Vec<StyledTextRun> {
    let mut runs = vec![
        StyledTextRun {
            text: "[Debug]:".to_string(),
            style: ComponentStyle {
                bold: Some(true),
                color: Some(VANILLA_DEBUG_FEEDBACK_COLOR),
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

pub(crate) fn queue_debug_recreate_server_query_request(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    request: DebugRecreateServerQueryRequest,
) {
    match request {
        DebugRecreateServerQueryRequest::BlockEntityTag {
            transaction_id,
            pos,
        } => queue_block_entity_tag_query_command(
            counters,
            net_commands,
            BlockEntityTagQuery {
                transaction_id,
                pos,
            },
        ),
        DebugRecreateServerQueryRequest::EntityTag {
            transaction_id,
            entity_id,
        } => queue_entity_tag_query_command(
            counters,
            net_commands,
            EntityTagQuery {
                transaction_id,
                entity_id,
            },
        ),
    }
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

struct DebugRecreateCopy {
    command: String,
    feedback_message: &'static str,
}

fn debug_copy_recreate_command(world: &WorldStore, add_nbt: bool) -> Option<DebugRecreateCopy> {
    match debug_recreate_target(world)? {
        DebugRecreateTarget::Block(pos) => debug_copy_recreate_block_command(world, pos, add_nbt),
        DebugRecreateTarget::Entity(entity_id) => {
            debug_copy_recreate_entity_command(world, entity_id, add_nbt)
        }
    }
}

fn debug_copy_recreate_server_response_command(
    target: &PendingDebugRecreateServerQueryTarget,
    response: &TagQueryResponseState,
) -> bbb_world::Result<Option<DebugRecreateCopy>> {
    match target {
        PendingDebugRecreateServerQueryTarget::Block { pos, description } => {
            debug_copy_recreate_server_block_command(*pos, description, response)
        }
        PendingDebugRecreateServerQueryTarget::Entity {
            entity_type,
            position,
        } => debug_copy_recreate_server_entity_command(entity_type, *position, response),
    }
}

fn debug_recreate_target(world: &WorldStore) -> Option<DebugRecreateTarget> {
    if world.local_player_id().is_none() || world.local_player_has_reduced_debug_info() {
        return None;
    }
    let target =
        crosshair_target_from_camera_at_partial_tick(world, camera_pose_from_world(world), 1.0)?;
    match target {
        CrosshairTarget::Block(hit) => Some(DebugRecreateTarget::Block(hit.pos)),
        CrosshairTarget::Entity(hit) => Some(DebugRecreateTarget::Entity(hit.entity_id)),
    }
}

fn debug_recreate_server_query_request(
    target: DebugRecreateTarget,
    transaction_id: i32,
) -> DebugRecreateServerQueryRequest {
    match target {
        DebugRecreateTarget::Block(pos) => DebugRecreateServerQueryRequest::BlockEntityTag {
            transaction_id,
            pos: protocol_block_pos_from_world(pos),
        },
        DebugRecreateTarget::Entity(entity_id) => DebugRecreateServerQueryRequest::EntityTag {
            transaction_id,
            entity_id,
        },
    }
}

fn pending_debug_recreate_server_query(
    world: &WorldStore,
    target: DebugRecreateTarget,
    transaction_id: i32,
) -> Option<PendingDebugRecreateServerQuery> {
    let target = match target {
        DebugRecreateTarget::Block(pos) => {
            let block = world.probe_block(pos)?;
            let block_name = block.block_name.as_deref()?;
            PendingDebugRecreateServerQueryTarget::Block {
                pos,
                description: debug_block_state_description(block_name, &block.block_properties),
            }
        }
        DebugRecreateTarget::Entity(entity_id) => {
            let entity = world.probe_entity(entity_id)?;
            let entity_type = vanilla_entity_resource_id_for_type_id(entity.entity_type_id)?;
            PendingDebugRecreateServerQueryTarget::Entity {
                entity_type: entity_type.to_string(),
                position: [entity.position.x, entity.position.y, entity.position.z],
            }
        }
    };
    Some(PendingDebugRecreateServerQuery {
        transaction_id,
        target,
    })
}

fn debug_copy_recreate_block_command(
    world: &WorldStore,
    pos: BlockPos,
    add_nbt: bool,
) -> Option<DebugRecreateCopy> {
    let block = world.probe_block(pos)?;
    let block_name = block.block_name.as_deref()?;
    let description = debug_block_state_description(block_name, &block.block_properties);
    let mut command = format!("/setblock {} {} {} {}", pos.x, pos.y, pos.z, description);
    if add_nbt {
        if let Some(snbt) = debug_local_block_entity_compact_snbt(world, pos) {
            command.push_str(&snbt);
        }
    }
    Some(DebugRecreateCopy {
        command,
        feedback_message: "Copied client-side block data to clipboard",
    })
}

fn debug_local_block_entity_compact_snbt(world: &WorldStore, pos: BlockPos) -> Option<String> {
    let raw_nbt = world.block_entity_raw_nbt_at(pos)?;
    let response = TagQueryResponseState {
        transaction_id: 0,
        tag_present: raw_nbt.first().is_some_and(|tag_id| *tag_id != 0),
        raw_nbt: raw_nbt.to_vec(),
    };
    response.compact_snbt().ok().flatten()
}

fn debug_copy_recreate_server_block_command(
    pos: BlockPos,
    description: &str,
    response: &TagQueryResponseState,
) -> bbb_world::Result<Option<DebugRecreateCopy>> {
    let mut command = format!("/setblock {} {} {} {}", pos.x, pos.y, pos.z, description);
    if let Some(snbt) = response.compact_snbt()? {
        command.push_str(&snbt);
    }
    Ok(Some(DebugRecreateCopy {
        command,
        feedback_message: "Copied server-side block data to clipboard",
    }))
}

fn debug_copy_recreate_entity_command(
    world: &WorldStore,
    entity_id: i32,
    add_nbt: bool,
) -> Option<DebugRecreateCopy> {
    let entity = world.probe_entity(entity_id)?;
    let entity_type = vanilla_entity_resource_id_for_type_id(entity.entity_type_id)?;
    let mut command = format!(
        "/summon {} {:.2} {:.2} {:.2}",
        entity_type, entity.position.x, entity.position.y, entity.position.z
    );
    if add_nbt {
        if let Some(snbt) = debug_local_entity_pretty_snbt(&entity) {
            command.push(' ');
            command.push_str(&snbt);
        }
    }
    Some(DebugRecreateCopy {
        command,
        feedback_message: "Copied client-side entity data to clipboard",
    })
}

fn debug_local_entity_pretty_snbt(entity: &EntityState) -> Option<String> {
    let mut fields = Vec::new();
    if let Some(motion) = debug_snbt_vec3d("Motion", entity.delta_movement) {
        fields.push(motion);
    }
    if let Some(rotation) = debug_snbt_rotation(entity.y_rot, entity.x_rot) {
        fields.push(rotation);
    }
    fields.push(format!(
        "fall_distance: {}",
        debug_snbt_double(ENTITY_DEFAULT_FALL_DISTANCE)?
    ));
    fields.push(format!("Fire: {ENTITY_DEFAULT_FIRE_TICKS}s"));
    let air = debug_entity_data_int_present(entity, ENTITY_AIR_SUPPLY_DATA_ID)
        .unwrap_or(ENTITY_DEFAULT_AIR_SUPPLY);
    fields.push(format!("Air: {}s", air as i16));
    let on_ground = entity.on_ground.unwrap_or(false);
    fields.push(format!("OnGround: {}b", if on_ground { 1 } else { 0 }));
    fields.push(format!(
        "Invulnerable: {}b",
        if ENTITY_DEFAULT_INVULNERABLE { 1 } else { 0 }
    ));
    fields.push(format!("PortalCooldown: {ENTITY_DEFAULT_PORTAL_COOLDOWN}"));
    if let Some(custom_name) =
        debug_entity_data_optional_component_present(entity, ENTITY_CUSTOM_NAME_DATA_ID)
    {
        fields.push(format!("CustomName: {}", debug_snbt_string(custom_name)));
    }
    if debug_entity_data_bool_present(entity, ENTITY_CUSTOM_NAME_VISIBLE_DATA_ID)
        .is_some_and(|visible| visible)
    {
        fields.push("CustomNameVisible: 1b".to_string());
    }
    if debug_entity_data_bool_present(entity, ENTITY_SILENT_DATA_ID).is_some_and(|silent| silent) {
        fields.push("Silent: 1b".to_string());
    }
    if debug_entity_data_bool_present(entity, ENTITY_NO_GRAVITY_DATA_ID)
        .is_some_and(|no_gravity| no_gravity)
    {
        fields.push("NoGravity: 1b".to_string());
    }
    if debug_entity_data_byte_present(entity, ENTITY_SHARED_FLAGS_DATA_ID)
        .is_some_and(|flags| flags & ENTITY_SHARED_FLAG_GLOWING != 0)
    {
        fields.push("Glowing: 1b".to_string());
    }
    if let Some(ticks_frozen) = debug_entity_data_int_present(entity, ENTITY_TICKS_FROZEN_DATA_ID) {
        if ticks_frozen > 0 {
            fields.push(format!("TicksFrozen: {ticks_frozen}"));
        }
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

fn debug_entity_data_byte_present(entity: &EntityState, data_id: u8) -> Option<i8> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Byte(value) => Some(*value),
            _ => None,
        })
}

fn debug_entity_data_int_present(entity: &EntityState, data_id: u8) -> Option<i32> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Int(value) => Some(*value),
            _ => None,
        })
}

fn debug_entity_data_bool_present(entity: &EntityState, data_id: u8) -> Option<bool> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Boolean(value) => Some(*value),
            _ => None,
        })
}

fn debug_entity_data_optional_component_present(entity: &EntityState, data_id: u8) -> Option<&str> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::OptionalComponent(Some(value)) => Some(value.as_str()),
            _ => None,
        })
}

fn debug_snbt_vec3d(name: &str, value: EntityVec3) -> Option<String> {
    Some(format!(
        "{}: [{}, {}, {}]",
        name,
        debug_snbt_double(value.x)?,
        debug_snbt_double(value.y)?,
        debug_snbt_double(value.z)?
    ))
}

fn debug_snbt_rotation(y_rot: f32, x_rot: f32) -> Option<String> {
    Some(format!(
        "Rotation: [{}, {}]",
        debug_snbt_float(y_rot)?,
        debug_snbt_float(x_rot)?
    ))
}

fn debug_snbt_double(value: f64) -> Option<String> {
    if !value.is_finite() {
        return None;
    }
    let value = if value == 0.0 { 0.0 } else { value };
    Some(debug_snbt_number_text(value.to_string(), 'd'))
}

fn debug_snbt_float(value: f32) -> Option<String> {
    if !value.is_finite() {
        return None;
    }
    let value = if value == 0.0 { 0.0 } else { value };
    Some(debug_snbt_number_text(value.to_string(), 'f'))
}

fn debug_snbt_string(value: &str) -> String {
    let mut out = String::new();
    let mut quote = None;
    let mut body = String::new();
    for c in value.chars() {
        match c {
            '\\' => body.push_str("\\\\"),
            '"' | '\'' => {
                if quote.is_none() {
                    quote = Some(if c == '"' { '\'' } else { '"' });
                }
                if quote == Some(c) {
                    body.push('\\');
                }
                body.push(c);
            }
            '\u{0008}' => body.push_str("\\b"),
            '\t' => body.push_str("\\t"),
            '\n' => body.push_str("\\n"),
            '\u{000c}' => body.push_str("\\f"),
            '\r' => body.push_str("\\r"),
            c if c < ' ' => body.push_str(&format!("\\x{:02x}", c as u8)),
            _ => body.push(c),
        }
    }
    let quote = quote.unwrap_or('"');
    out.push(quote);
    out.push_str(&body);
    out.push(quote);
    out
}

fn debug_snbt_number_text(mut text: String, suffix: char) -> String {
    if !text.contains('.') && !text.contains('e') && !text.contains('E') {
        text.push_str(".0");
    }
    text.push(suffix);
    text
}

fn debug_copy_recreate_server_entity_command(
    entity_type: &str,
    position: [f64; 3],
    response: &TagQueryResponseState,
) -> bbb_world::Result<Option<DebugRecreateCopy>> {
    let command = if let Some(snbt) = response.pretty_snbt_without_root_keys(&["UUID", "Pos"])? {
        format!(
            "/summon {} {:.2} {:.2} {:.2} {}",
            entity_type, position[0], position[1], position[2], snbt
        )
    } else {
        format!(
            "/summon {} {:.2} {:.2} {:.2}",
            entity_type, position[0], position[1], position[2]
        )
    };
    Ok(Some(DebugRecreateCopy {
        command,
        feedback_message: "Copied server-side entity data to clipboard",
    }))
}

fn debug_block_state_description(
    block_name: &str,
    properties: &BTreeMap<String, String>,
) -> String {
    if properties.is_empty() {
        return block_name.to_string();
    }
    let properties = properties
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("{block_name}[{properties}]")
}

impl ClientInputState {
    pub(crate) fn loom_selected_pattern_index(&self) -> Option<i32> {
        self.loom_selected_pattern_index
    }

    pub(crate) fn beacon_effect_selection(&self) -> (Option<i32>, Option<i32>) {
        (self.beacon_primary_effect, self.beacon_secondary_effect)
    }

    pub(crate) fn anvil_rename_text(&self) -> &str {
        &self.anvil_rename_text
    }

    pub(crate) fn recipe_book_search_hud_state(&self) -> RecipeBookSearchHudState {
        RecipeBookSearchHudState {
            text: self.recipe_book_search_text.clone(),
            cursor: self.recipe_book_search_cursor,
            selection: self.recipe_book_search_selection,
            focused: self.recipe_book_search_focused,
        }
    }

    pub(crate) fn recipe_book_tab_selection_hud_state(&self) -> RecipeBookTabSelectionHudState {
        RecipeBookTabSelectionHudState {
            crafting: self.recipe_book_crafting_tab_index,
            furnace: self.recipe_book_furnace_tab_index,
            blast_furnace: self.recipe_book_blast_furnace_tab_index,
            smoker: self.recipe_book_smoker_tab_index,
        }
    }

    pub(crate) fn recipe_book_page_hud_state(&self) -> RecipeBookPageHudState {
        RecipeBookPageHudState {
            crafting: self.recipe_book_crafting_page,
            furnace: self.recipe_book_furnace_page,
            blast_furnace: self.recipe_book_blast_furnace_page,
            smoker: self.recipe_book_smoker_page,
        }
    }

    pub(crate) fn recipe_book_overlay_hud_state(&self) -> Option<RecipeBookOverlayHudState> {
        self.recipe_book_overlay
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
    let before = player_input_from_state(input);
    let before_sprinting = effective_sprinting_from_state(world, input);
    let riding_jump_charge_seconds = input.riding_jump_charge_seconds.take();
    input.clear_pressed();
    let after = player_input_from_state(input);
    let after_sprinting = effective_sprinting_from_state(world, input);
    if after != before {
        queue_player_input_command(counters, net_commands, after);
        if before_sprinting != after_sprinting {
            queue_sprint_command(counters, world, net_commands, after_sprinting);
        }
    }
    movement::queue_released_riding_jump_command(
        riding_jump_charge_seconds,
        world,
        counters,
        net_commands,
    );
    if let Some(pos) = world.take_local_destroying_block() {
        queue_player_action_command(
            counters,
            net_commands,
            PlayerActionKind::AbortDestroyBlock,
            pos,
            ProtocolDirection::Down,
            0,
        );
    }
    if world.take_local_using_item() {
        queue_zero_pos_player_action_command(
            counters,
            net_commands,
            PlayerActionKind::ReleaseUseItem,
        );
    }
    if world.local_player_root_boat_vehicle_id().is_some() {
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

fn handle_book_screen_key(world: &mut WorldStore, code: KeyCode, pressed: bool) -> bool {
    if world.current_book().is_none() {
        return false;
    }
    if !pressed {
        return true;
    }
    match code {
        KeyCode::Escape | KeyCode::KeyE => {
            world.close_current_book();
        }
        KeyCode::PageUp => {
            world.turn_current_book_page(-1);
        }
        KeyCode::PageDown => {
            world.turn_current_book_page(1);
        }
        _ => {}
    }
    true
}

fn handle_advancements_screen_key(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    code: KeyCode,
    pressed: bool,
) -> bool {
    if !world.advancements_screen_is_open() {
        return false;
    }
    if pressed && matches!(code, KeyCode::Escape | KeyCode::KeyL) {
        close_advancements_screen_and_queue(input, counters, world, net_commands);
    }
    true
}

fn open_advancements_screen(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    if world.open_advancements_screen() {
        release_active_input(input, world, counters, net_commands);
        if let Some(tab) = world.ensure_advancements_screen_selected_tab() {
            queue_seen_advancements_command(
                counters,
                net_commands,
                SeenAdvancements::OpenedTab { tab },
            );
        }
    }
}

fn close_advancements_screen_and_queue(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    if world.close_advancements_screen() {
        release_active_input(input, world, counters, net_commands);
        queue_seen_advancements_command(counters, net_commands, SeenAdvancements::ClosedScreen);
    }
}

pub(crate) fn handle_advancements_screen_mouse_input(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    button: MouseButton,
    state: ElementState,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    if !world.advancements_screen_is_open() {
        return false;
    }
    input.inventory_cursor_position =
        advancements_screen_cursor_position(cursor_position, surface_size);
    input.inventory_hovered_slot = None;
    if matches!(state, ElementState::Released) {
        input.advancement_is_scrolling = false;
        if matches!(button, MouseButton::Left) {
            input.advancement_mouse_left_down = false;
        }
        return true;
    }
    if !matches!(button, MouseButton::Left) {
        return true;
    }
    input.advancement_mouse_left_down = true;
    input.advancement_is_scrolling = false;
    if let Some(tab) = advancements_tab_at_position(world, cursor_position, surface_size) {
        if let Some(tab) = world.select_advancements_root_tab(&tab) {
            queue_seen_advancements_command(
                counters,
                net_commands,
                SeenAdvancements::OpenedTab { tab },
            );
        }
    } else if advancements_done_button_contains(cursor_position, surface_size) {
        close_advancements_screen_and_queue(input, counters, world, net_commands);
    }
    true
}

pub(crate) fn handle_advancements_screen_cursor_moved(
    input: &mut ClientInputState,
    world: &WorldStore,
    previous_position: Option<PhysicalPosition<f64>>,
    cursor_position: Option<PhysicalPosition<f64>>,
) -> bool {
    if !world.advancements_screen_is_open() {
        return false;
    }
    input.inventory_cursor_position = cursor_position.and_then(physical_position_floor_i32);
    input.inventory_hovered_slot = None;
    if !input.advancement_mouse_left_down {
        return true;
    }
    let (Some(previous), Some(current)) = (previous_position, cursor_position) else {
        return true;
    };
    if !input.advancement_is_scrolling {
        input.advancement_is_scrolling = true;
        return true;
    }
    let Some(tab) = world.selected_advancements_tab() else {
        return true;
    };
    let entry = input
        .advancement_scroll_deltas
        .entry(tab.to_string())
        .or_default();
    entry.0 += current.x - previous.x;
    entry.1 += current.y - previous.y;
    true
}

pub(crate) fn handle_advancements_screen_mouse_wheel(
    input: &mut ClientInputState,
    world: &WorldStore,
    delta: MouseScrollDelta,
) -> bool {
    if !world.advancements_screen_is_open() {
        return false;
    }
    let Some(tab) = world.selected_advancements_tab() else {
        return true;
    };
    let Some((wheel_x, wheel_y)) = advancement_wheel_steps_from_scroll(input, delta) else {
        return true;
    };
    let entry = input
        .advancement_scroll_deltas
        .entry(tab.to_string())
        .or_default();
    entry.0 += f64::from(wheel_x) * 16.0;
    entry.1 += f64::from(wheel_y) * 16.0;
    true
}

fn advancement_wheel_steps_from_scroll(
    input: &mut ClientInputState,
    delta: MouseScrollDelta,
) -> Option<(i32, i32)> {
    let (x, y) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (f64::from(x), f64::from(y)),
        MouseScrollDelta::PixelDelta(pos) => (pos.x, pos.y),
    };
    if input.scroll_accumulated_x != 0.0
        && advancement_scroll_signum(x) != advancement_scroll_signum(input.scroll_accumulated_x)
    {
        input.scroll_accumulated_x = 0.0;
    }
    if input.scroll_accumulated_y != 0.0
        && advancement_scroll_signum(y) != advancement_scroll_signum(input.scroll_accumulated_y)
    {
        input.scroll_accumulated_y = 0.0;
    }

    input.scroll_accumulated_x += x;
    input.scroll_accumulated_y += y;
    let wheel_x = input.scroll_accumulated_x as i32;
    let wheel_y = input.scroll_accumulated_y as i32;
    if wheel_x == 0 && wheel_y == 0 {
        return None;
    }

    input.scroll_accumulated_x -= f64::from(wheel_x);
    input.scroll_accumulated_y -= f64::from(wheel_y);
    Some((wheel_x, wheel_y))
}

fn advancements_screen_cursor_position(
    cursor_position: Option<PhysicalPosition<f64>>,
    _surface_size: PhysicalSize<u32>,
) -> Option<(i32, i32)> {
    cursor_position.and_then(physical_position_floor_i32)
}

fn physical_position_floor_i32(position: PhysicalPosition<f64>) -> Option<(i32, i32)> {
    (position.x.is_finite() && position.y.is_finite())
        .then(|| (position.x.floor() as i32, position.y.floor() as i32))
}

fn advancement_scroll_signum(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}

fn advancements_tab_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<String> {
    let cursor = cursor_position?;
    let tabs = world.advancement_root_tabs();
    if tabs.len() <= 1 {
        return None;
    }
    let window_x =
        (f64::from(surface_size.width.max(1)) - f64::from(ADVANCEMENTS_WINDOW_WIDTH)) * 0.5;
    let window_y =
        (f64::from(surface_size.height.max(1)) - f64::from(ADVANCEMENTS_WINDOW_HEIGHT)) * 0.5;
    tabs.into_iter().find_map(|tab| {
        let (x, y, width, height) = advancements_tab_bounds(tab.display_index, window_x, window_y)?;
        (cursor.x > x && cursor.x < x + width && cursor.y > y && cursor.y < y + height)
            .then_some(tab.id)
    })
}

fn advancements_tab_bounds(
    display_index: usize,
    window_x: f64,
    window_y: f64,
) -> Option<(f64, f64, f64, f64)> {
    let (x, y, width, height) = if display_index < ADVANCEMENTS_TAB_ABOVE_MAX {
        let index = i32::try_from(display_index).ok()?;
        (
            (ADVANCEMENTS_TAB_ABOVE_WIDTH + 4) * index,
            -ADVANCEMENTS_TAB_ABOVE_HEIGHT + 4,
            ADVANCEMENTS_TAB_ABOVE_WIDTH,
            ADVANCEMENTS_TAB_ABOVE_HEIGHT,
        )
    } else if display_index < ADVANCEMENTS_TAB_ABOVE_MAX + ADVANCEMENTS_TAB_BELOW_MAX {
        let index = i32::try_from(display_index - ADVANCEMENTS_TAB_ABOVE_MAX).ok()?;
        (
            (ADVANCEMENTS_TAB_BELOW_WIDTH + 4) * index,
            136,
            ADVANCEMENTS_TAB_BELOW_WIDTH,
            ADVANCEMENTS_TAB_BELOW_HEIGHT,
        )
    } else if display_index
        < ADVANCEMENTS_TAB_ABOVE_MAX + ADVANCEMENTS_TAB_BELOW_MAX + ADVANCEMENTS_TAB_LEFT_MAX
    {
        let index =
            i32::try_from(display_index - ADVANCEMENTS_TAB_ABOVE_MAX - ADVANCEMENTS_TAB_BELOW_MAX)
                .ok()?;
        (
            -ADVANCEMENTS_TAB_LEFT_WIDTH + 4,
            ADVANCEMENTS_TAB_LEFT_HEIGHT * index,
            ADVANCEMENTS_TAB_LEFT_WIDTH,
            ADVANCEMENTS_TAB_LEFT_HEIGHT,
        )
    } else if display_index
        < ADVANCEMENTS_TAB_ABOVE_MAX
            + ADVANCEMENTS_TAB_BELOW_MAX
            + ADVANCEMENTS_TAB_LEFT_MAX
            + ADVANCEMENTS_TAB_RIGHT_MAX
    {
        let index = i32::try_from(
            display_index
                - ADVANCEMENTS_TAB_ABOVE_MAX
                - ADVANCEMENTS_TAB_BELOW_MAX
                - ADVANCEMENTS_TAB_LEFT_MAX,
        )
        .ok()?;
        (
            248,
            ADVANCEMENTS_TAB_RIGHT_HEIGHT * index,
            ADVANCEMENTS_TAB_RIGHT_WIDTH,
            ADVANCEMENTS_TAB_RIGHT_HEIGHT,
        )
    } else {
        return None;
    };
    Some((
        window_x + f64::from(x),
        window_y + f64::from(y),
        f64::from(width),
        f64::from(height),
    ))
}

fn advancements_done_button_contains(
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(cursor) = cursor_position else {
        return false;
    };
    let x =
        (f64::from(surface_size.width.max(1)) - f64::from(ADVANCEMENTS_DONE_BUTTON_WIDTH)) * 0.5;
    let y = f64::from(surface_size.height.max(1)) - f64::from(ADVANCEMENTS_FOOTER_HEIGHT)
        + f64::from(ADVANCEMENTS_FOOTER_HEIGHT - ADVANCEMENTS_DONE_BUTTON_HEIGHT) * 0.5;
    cursor.x >= x
        && cursor.x < x + f64::from(ADVANCEMENTS_DONE_BUTTON_WIDTH)
        && cursor.y >= y
        && cursor.y < y + f64::from(ADVANCEMENTS_DONE_BUTTON_HEIGHT)
}

pub(crate) fn handle_book_screen_mouse_input(
    world: &mut WorldStore,
    button: MouseButton,
    state: ElementState,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    if world.current_book().is_none() {
        return false;
    }
    if !matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
        return true;
    }
    match book_screen_click_target_at_position(world, cursor_position, surface_size) {
        Some(BookScreenClickTarget::Done) => {
            world.close_current_book();
        }
        Some(BookScreenClickTarget::PreviousPage) => {
            world.turn_current_book_page(-1);
        }
        Some(BookScreenClickTarget::NextPage) => {
            world.turn_current_book_page(1);
        }
        None => {}
    }
    true
}

fn book_screen_click_target_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<BookScreenClickTarget> {
    let book = world.current_book()?;
    let cursor = cursor_position?;
    let (origin_x, origin_y) = book_screen_origin(surface_size);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    if y >= f64::from(BOOK_MENU_BUTTON_Y)
        && y < f64::from(BOOK_MENU_BUTTON_Y + BOOK_MENU_BUTTON_HEIGHT)
        && x >= f64::from(BOOK_MENU_DONE_BUTTON_X)
        && x < f64::from(BOOK_MENU_DONE_BUTTON_X + BOOK_MENU_BUTTON_WIDTH)
    {
        return Some(BookScreenClickTarget::Done);
    }
    if y < f64::from(BOOK_PAGE_BUTTON_Y)
        || y >= f64::from(BOOK_PAGE_BUTTON_Y + BOOK_PAGE_BUTTON_HEIGHT)
    {
        return None;
    }
    if book.current_page > 0
        && x >= f64::from(BOOK_PAGE_BACK_BUTTON_X)
        && x < f64::from(BOOK_PAGE_BACK_BUTTON_X + BOOK_PAGE_BUTTON_WIDTH)
    {
        return Some(BookScreenClickTarget::PreviousPage);
    }
    if book.current_page + 1 < book.pages.len()
        && x >= f64::from(BOOK_PAGE_FORWARD_BUTTON_X)
        && x < f64::from(BOOK_PAGE_FORWARD_BUTTON_X + BOOK_PAGE_BUTTON_WIDTH)
    {
        return Some(BookScreenClickTarget::NextPage);
    }
    None
}

fn book_screen_origin(surface_size: PhysicalSize<u32>) -> (f64, f64) {
    (
        (f64::from(surface_size.width.max(1)) - f64::from(BOOK_SCREEN_WIDTH)) * 0.5,
        (f64::from(surface_size.height.max(1)) - f64::from(BOOK_SCREEN_HEIGHT)) * 0.5,
    )
}

#[cfg(test)]
pub(crate) fn handle_key_input(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    physical_key: PhysicalKey,
    state: ElementState,
) {
    handle_key_input_with_item_runtime(
        input,
        counters,
        world,
        net_commands,
        None,
        physical_key,
        state,
        PhysicalSize::new(1280, 720),
    );
}

pub(crate) fn handle_key_input_with_item_runtime(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    physical_key: PhysicalKey,
    state: ElementState,
    surface_size: PhysicalSize<u32>,
) {
    if !input.focused {
        return;
    }

    let pressed = matches!(state, ElementState::Pressed);
    let PhysicalKey::Code(code) = physical_key else {
        return;
    };
    input.set_key_down(code, pressed);
    if matches!(code, KeyCode::ShiftLeft | KeyCode::ShiftRight) {
        input.set_shift_key(code, pressed);
    }
    if matches!(code, KeyCode::ControlLeft | KeyCode::ControlRight) {
        input.set_control_key(code, pressed);
    }

    if input.handle_debug_pause_screen_key(physical_key, state) {
        return;
    }

    if input.handle_debug_overlay_key_with_clipboard_and_net(
        physical_key,
        state,
        Some(world),
        None,
        None,
        counters,
        net_commands,
    ) {
        return;
    }

    if input.debug_pause_screen_is_open() {
        return;
    }

    if handle_sign_editor_key(input, counters, world, net_commands, code, pressed) {
        return;
    }

    if input.command_entry_is_active() {
        handle_chat_entry_key(input, counters, world, net_commands, code, pressed);
        return;
    }

    if handle_book_screen_key(world, code, pressed) {
        return;
    }

    if handle_advancements_screen_key(input, counters, world, net_commands, code, pressed) {
        return;
    }

    if world.current_dialog().is_some() {
        return;
    }

    if world.local_player_is_dead() {
        if pressed && matches!(code, KeyCode::Enter | KeyCode::Space) {
            queue_perform_respawn_command(counters, net_commands);
        }
        return;
    }

    if world.local_player_is_sleeping() {
        if pressed && matches!(code, KeyCode::Escape | KeyCode::Enter | KeyCode::Space) {
            queue_player_command_action(
                counters,
                world,
                net_commands,
                PlayerCommandAction::StopSleeping,
                0,
            );
        }
        return;
    }

    if pressed {
        if matches!(code, KeyCode::Escape)
            && maybe_close_narrow_recipe_book(input, world, counters, net_commands, surface_size)
        {
            return;
        }
        if matches!(code, KeyCode::Escape)
            && queue_container_close_command(counters, world, net_commands)
        {
            return;
        }
        if matches!(code, KeyCode::KeyE)
            && !anvil_rename_entry_consumes_key(world, code)
            && !recipe_book_search_entry_consumes_key(input, world, code)
            && queue_container_close_command(counters, world, net_commands)
        {
            return;
        }
    }
    if inventory_screen_layout(world).is_some() {
        if pressed {
            handle_inventory_key_input(input, world, counters, net_commands, item_runtime, code);
        }
        return;
    }
    if world.open_container_id().is_some() {
        return;
    }

    if pressed {
        input.record_debug_profiler_chart_navigation_key(code);
        if let Some(slot) = hotbar_slot_for_key(code) {
            if world.local_player_is_spectator() {
                return;
            }
            select_hotbar_slot(counters, world, net_commands, slot);
            return;
        }
        match code {
            KeyCode::KeyQ => {
                if world.local_player_is_spectator() {
                    return;
                }
                let drop_all = input.control_down();
                let action = if drop_all {
                    PlayerActionKind::DropAllItems
                } else {
                    PlayerActionKind::DropItem
                };
                let dropped_item = world.drop_local_selected_hotbar_item(drop_all);
                queue_zero_pos_player_action_command(counters, net_commands, action);
                if dropped_item {
                    queue_swing_command(counters, net_commands, InteractionHand::MainHand);
                }
                return;
            }
            KeyCode::KeyF => {
                if world.local_player_is_spectator() {
                    return;
                }
                queue_zero_pos_player_action_command(
                    counters,
                    net_commands,
                    PlayerActionKind::SwapItemWithOffhand,
                );
                return;
            }
            KeyCode::KeyE => {
                if world
                    .local_player_server_controlled_inventory_vehicle_id()
                    .is_some()
                {
                    queue_player_command_action(
                        counters,
                        world,
                        net_commands,
                        PlayerCommandAction::OpenInventory,
                        0,
                    );
                } else {
                    world.open_local_inventory();
                }
                return;
            }
            KeyCode::KeyL => {
                open_advancements_screen(input, counters, world, net_commands);
                return;
            }
            KeyCode::KeyT => {
                open_chat_entry(input, world, counters, net_commands, true);
                return;
            }
            _ => {}
        }
    }

    let before = player_input_from_state(input);
    let before_sprinting = effective_sprinting_from_state(world, input);
    let handled = match code {
        KeyCode::KeyW | KeyCode::ArrowUp => {
            input.forward = pressed;
            if pressed && !before.forward {
                maybe_apply_forward_sprint_trigger(input, world);
            }
            true
        }
        KeyCode::KeyS | KeyCode::ArrowDown => {
            input.backward = pressed;
            if pressed {
                input.clear_sprint_trigger();
            }
            true
        }
        KeyCode::KeyA | KeyCode::ArrowLeft => {
            input.left = pressed;
            true
        }
        KeyCode::KeyD | KeyCode::ArrowRight => {
            input.right = pressed;
            true
        }
        KeyCode::Space => {
            input.jump = pressed;
            true
        }
        KeyCode::ShiftLeft | KeyCode::ShiftRight => {
            input.sneak = input.shift_down();
            if input.sneak {
                input.clear_sprint_trigger();
            }
            true
        }
        KeyCode::ControlLeft | KeyCode::ControlRight => {
            input.sprint = pressed;
            if pressed {
                input.clear_sprint_trigger();
            }
            true
        }
        _ => false,
    };
    if handled {
        let after = player_input_from_state(input);
        let after_sprinting = effective_sprinting_from_state(world, input);
        if after != before {
            queue_player_input_command(counters, net_commands, after);
            if before_sprinting != after_sprinting {
                queue_sprint_command(counters, world, net_commands, after_sprinting);
            }
            let just_toggled_creative_flight =
                maybe_toggle_creative_flight(input, counters, world, net_commands, before, after);
            if should_queue_start_fall_flying(world, before, after) {
                if !just_toggled_creative_flight {
                    queue_player_command_action(
                        counters,
                        world,
                        net_commands,
                        PlayerCommandAction::StartFallFlying,
                        0,
                    );
                }
            }
        }
    }
}

fn maybe_apply_forward_sprint_trigger(input: &mut ClientInputState, world: &WorldStore) {
    if input.sprint {
        input.clear_sprint_trigger();
        return;
    }
    let mut sprint_candidate = local_player_input_from_state(input);
    sprint_candidate.sprint = true;
    if !world.local_player_effective_sprint(sprint_candidate) {
        return;
    }
    if input.sprint_trigger_ticks > 0 {
        input.sprint = true;
        input.clear_sprint_trigger();
    } else {
        input.prime_sprint_trigger();
    }
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

    let flying = !world.local_player().abilities.is_some_and(|a| a.flying);
    let toggled = queue_player_abilities_command(counters, world, net_commands, flying);
    if toggled {
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
    after.jump
        && !before.jump
        && world.local_player_root_vehicle_id().is_none()
        && world
            .local_player_pose()
            .is_some_and(|pose| !pose.on_ground)
        && world.local_player_has_equipped_elytra()
}

#[cfg(test)]
pub(crate) fn handle_text_input(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    text: &str,
) {
    handle_text_input_with_item_runtime(input, counters, world, net_commands, None, text);
}

pub(crate) fn handle_text_input_with_item_runtime(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    text: &str,
) {
    if !input.focused {
        return;
    }

    if handle_sign_editor_text(input, counters, world, net_commands, text) {
        return;
    }

    if world.current_dialog().is_some()
        || world.current_book().is_some()
        || world.advancements_screen_is_open()
    {
        return;
    }

    if handle_inventory_text_input(input, world, counters, net_commands, item_runtime, text) {
        return;
    }

    if input.chat_entry.is_none() {
        if !text.starts_with('/') {
            return;
        }
        open_chat_entry(input, world, counters, net_commands, false);
    }

    let Some(entry) = &mut input.chat_entry else {
        return;
    };
    if entry.suppress_open_key_commit {
        entry.suppress_open_key_commit = false;
        if matches!(text, "t" | "T") {
            return;
        }
    }
    let before = entry.text.clone();
    insert_chat_entry_text(entry, text);
    update_chat_entry_after_text_change(input, counters, world, net_commands, before.as_str());
}

fn open_chat_entry(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    suppress_open_key_commit: bool,
) {
    release_active_input(input, world, counters, net_commands);
    input.chat_entry = Some(ChatEntryState {
        text: String::new(),
        cursor: 0,
        selection: 0,
        last_suggestion_request_text: None,
        suppress_open_key_commit,
    });
}

fn handle_chat_entry_key(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    code: KeyCode,
    pressed: bool,
) {
    if !pressed {
        return;
    }

    match code {
        KeyCode::Enter | KeyCode::NumpadEnter => {
            submit_chat_entry(input, counters, world, net_commands)
        }
        KeyCode::Escape => input.chat_entry = None,
        KeyCode::Tab => {
            apply_latest_command_suggestion(input, world);
        }
        KeyCode::KeyA if input.control_down() && !input.shift_down() => {
            if let Some(entry) = &mut input.chat_entry {
                select_chat_entry_text(entry);
            }
        }
        KeyCode::ArrowLeft => {
            let control_down = input.control_down();
            if let Some(entry) = &mut input.chat_entry {
                let cursor = if control_down {
                    text_edit::word_position(&entry.text, entry.cursor, -1)
                } else {
                    entry.cursor.saturating_sub(1)
                };
                set_chat_entry_cursor(entry, cursor);
            }
        }
        KeyCode::ArrowRight => {
            let control_down = input.control_down();
            if let Some(entry) = &mut input.chat_entry {
                let cursor = if control_down {
                    text_edit::word_position(&entry.text, entry.cursor, 1)
                } else {
                    (entry.cursor + 1).min(text_edit::char_len(&entry.text))
                };
                set_chat_entry_cursor(entry, cursor);
            }
        }
        KeyCode::Home => {
            if let Some(entry) = &mut input.chat_entry {
                set_chat_entry_cursor(entry, 0);
            }
        }
        KeyCode::End => {
            if let Some(entry) = &mut input.chat_entry {
                set_chat_entry_cursor(entry, text_edit::char_len(&entry.text));
            }
        }
        KeyCode::Backspace => {
            let control_down = input.control_down();
            if let Some(entry) = &mut input.chat_entry {
                if entry.text.is_empty() {
                    input.chat_entry = None;
                    return;
                }
                let before = entry.text.clone();
                let deleted_selection = delete_chat_entry_selection(entry);
                if !deleted_selection && control_down {
                    text_edit::remove_word_before_cursor(&mut entry.text, &mut entry.cursor);
                    entry.selection = entry.cursor;
                } else if !deleted_selection {
                    remove_chat_entry_char_before_cursor(&mut entry.text, &mut entry.cursor);
                    entry.selection = entry.cursor;
                }
                update_chat_entry_after_text_change(
                    input,
                    counters,
                    world,
                    net_commands,
                    before.as_str(),
                );
            }
        }
        KeyCode::Delete => {
            let control_down = input.control_down();
            if let Some(entry) = &mut input.chat_entry {
                let before = entry.text.clone();
                let deleted_selection = delete_chat_entry_selection(entry);
                if !deleted_selection && control_down {
                    text_edit::remove_word_at_cursor(&mut entry.text, entry.cursor);
                    entry.selection = entry.cursor;
                } else if !deleted_selection {
                    remove_chat_entry_char_at_cursor(&mut entry.text, entry.cursor);
                    entry.selection = entry.cursor;
                }
                update_chat_entry_after_text_change(
                    input,
                    counters,
                    world,
                    net_commands,
                    before.as_str(),
                );
            }
        }
        _ => {}
    }
}

fn apply_latest_command_suggestion(input: &mut ClientInputState, world: &WorldStore) {
    let Some(current_text) = input.chat_entry.as_ref().map(|entry| entry.text.as_str()) else {
        return;
    };
    let Some(updated_text) = latest_command_suggestion_text(world, current_text) else {
        return;
    };
    if let Some(entry) = &mut input.chat_entry {
        entry.text = updated_text;
        set_chat_entry_cursor(entry, text_edit::char_len(&entry.text));
        entry.last_suggestion_request_text = Some(entry.text.clone());
    }
}

fn latest_command_suggestion_text(world: &WorldStore, current_text: &str) -> Option<String> {
    let suggestions = world.command_suggestions();
    let request = suggestions.last_request.as_ref()?;
    if request.command != current_text {
        return None;
    }

    let result = suggestions
        .last_id
        .and_then(|id| suggestions.by_id.get(&id))?;
    if result.id != request.id {
        return None;
    }
    let suggestion = result.suggestions.first()?;
    apply_command_suggestion_text(current_text, result.start, result.length, &suggestion.text)
}

fn apply_command_suggestion_text(
    current_text: &str,
    start: i32,
    length: i32,
    suggestion: &str,
) -> Option<String> {
    let start = usize::try_from(start).ok()?;
    let length = usize::try_from(length).ok()?;
    let end = start.checked_add(length)?;
    if end > current_text.len()
        || !current_text.is_char_boundary(start)
        || !current_text.is_char_boundary(end)
    {
        return None;
    }

    let mut updated = String::with_capacity(current_text.len() - length + suggestion.len());
    updated.push_str(&current_text[..start]);
    updated.push_str(suggestion);
    updated.push_str(&current_text[end..]);
    Some(updated)
}

fn update_chat_entry_after_text_change(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    before: &str,
) {
    let Some(entry) = &mut input.chat_entry else {
        return;
    };
    if entry.text.is_empty() {
        input.chat_entry = None;
        return;
    }
    if entry.text == before {
        return;
    }
    if entry.text.starts_with('/') {
        maybe_queue_command_suggestion_request(input, counters, world, net_commands);
    }
}

fn insert_chat_entry_text(entry: &mut ChatEntryState, text: &str) {
    delete_chat_entry_selection(entry);
    let current = &mut entry.text;
    entry.cursor = entry.cursor.min(text_edit::char_len(current));
    let mut remaining = CHAT_ENTRY_MAX_LENGTH.saturating_sub(text_edit::char_len(current));
    for ch in text.chars().filter(|ch| is_chat_text_char(*ch)) {
        if remaining == 0 {
            break;
        }
        let insert_at = text_edit::byte_index(current, entry.cursor);
        current.insert(insert_at, ch);
        entry.cursor += 1;
        remaining -= 1;
    }
    entry.selection = entry.cursor;
}

fn set_chat_entry_cursor(entry: &mut ChatEntryState, cursor: usize) {
    entry.cursor = cursor.min(text_edit::char_len(&entry.text));
    entry.selection = entry.cursor;
}

fn select_chat_entry_text(entry: &mut ChatEntryState) {
    entry.selection = 0;
    entry.cursor = text_edit::char_len(&entry.text);
}

fn delete_chat_entry_selection(entry: &mut ChatEntryState) -> bool {
    if entry.selection == entry.cursor {
        return false;
    }
    let start = entry.selection.min(entry.cursor);
    let end = entry.selection.max(entry.cursor);
    let start_byte = text_edit::byte_index(&entry.text, start);
    let end_byte = text_edit::byte_index(&entry.text, end);
    entry.text.replace_range(start_byte..end_byte, "");
    entry.cursor = start;
    entry.selection = start;
    true
}

fn remove_chat_entry_char_before_cursor(current: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let start = text_edit::byte_index(current, *cursor - 1);
    let end = text_edit::byte_index(current, *cursor);
    current.replace_range(start..end, "");
    *cursor -= 1;
}

fn remove_chat_entry_char_at_cursor(current: &mut String, cursor: usize) {
    if cursor >= text_edit::char_len(current) {
        return;
    }
    let start = text_edit::byte_index(current, cursor);
    let end = text_edit::byte_index(current, cursor + 1);
    current.replace_range(start..end, "");
}

fn submit_chat_entry(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    let Some(entry) = input.chat_entry.take() else {
        return;
    };
    let Some(command) = normalize_command_entry(&entry.text) else {
        let Some(message) = normalize_chat_entry(&entry.text) else {
            return;
        };
        queue_chat_message_command(counters, world, net_commands, message);
        return;
    };
    queue_chat_command(counters, world, net_commands, command);
}

fn normalize_command_entry(text: &str) -> Option<String> {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let command = normalized.strip_prefix('/')?;
    if command.is_empty() {
        return None;
    }
    Some(command.to_string())
}

fn normalize_chat_entry(text: &str) -> Option<String> {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() || normalized.starts_with('/') {
        return None;
    }
    Some(normalized)
}

fn maybe_queue_command_suggestion_request(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    let Some(entry) = &mut input.chat_entry else {
        return;
    };
    if entry.last_suggestion_request_text.as_deref() == Some(entry.text.as_str()) {
        return;
    }
    let text = entry.text.clone();
    entry.last_suggestion_request_text = Some(text.clone());
    let request = world.next_command_suggestion_request(text);
    queue_command_suggestion_request(counters, net_commands, request.id, request.command);
}

fn is_chat_text_char(ch: char) -> bool {
    ch >= ' ' && ch != '\u{7f}' && ch != '\u{a7}'
}

fn handle_sign_editor_text(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    text: &str,
) -> bool {
    sync_sign_editor_input(input, counters, world, net_commands);
    let Some(editor) = &mut input.sign_editor else {
        return false;
    };

    insert_sign_line_text(editor, text);
    true
}

fn handle_sign_editor_key(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    code: KeyCode,
    pressed: bool,
) -> bool {
    sync_sign_editor_input(input, counters, world, net_commands);
    if input.sign_editor.is_none() {
        return false;
    }
    if !pressed {
        return true;
    }

    match code {
        KeyCode::Escape => submit_sign_editor(input, counters, net_commands),
        KeyCode::KeyA if input.control_down() => {
            if let Some(editor) = &mut input.sign_editor {
                select_sign_editor_line(editor);
            }
        }
        KeyCode::ArrowUp => {
            if let Some(editor) = &mut input.sign_editor {
                set_sign_editor_line(editor, (editor.line + 3) % editor.lines.len());
            }
        }
        KeyCode::ArrowDown | KeyCode::Enter | KeyCode::NumpadEnter => {
            if let Some(editor) = &mut input.sign_editor {
                set_sign_editor_line(editor, (editor.line + 1) % editor.lines.len());
            }
        }
        KeyCode::ArrowLeft => {
            let control_down = input.control_down();
            if let Some(editor) = &mut input.sign_editor {
                let cursor = if control_down {
                    text_edit::word_position(&editor.lines[editor.line], editor.cursor, -1)
                } else {
                    editor.cursor.saturating_sub(1)
                };
                set_sign_editor_cursor(editor, cursor);
            }
        }
        KeyCode::ArrowRight => {
            let control_down = input.control_down();
            if let Some(editor) = &mut input.sign_editor {
                let cursor = if control_down {
                    text_edit::word_position(&editor.lines[editor.line], editor.cursor, 1)
                } else {
                    (editor.cursor + 1).min(sign_line_char_len(&editor.lines[editor.line]))
                };
                set_sign_editor_cursor(editor, cursor);
            }
        }
        KeyCode::Home => {
            if let Some(editor) = &mut input.sign_editor {
                set_sign_editor_cursor(editor, 0);
            }
        }
        KeyCode::End => {
            if let Some(editor) = &mut input.sign_editor {
                set_sign_editor_cursor(editor, sign_line_char_len(&editor.lines[editor.line]));
            }
        }
        KeyCode::Backspace => {
            let control_down = input.control_down();
            if let Some(editor) = &mut input.sign_editor {
                if delete_sign_selection(editor) {
                    return true;
                } else if control_down {
                    text_edit::remove_word_before_cursor(
                        &mut editor.lines[editor.line],
                        &mut editor.cursor,
                    );
                    editor.selection = editor.cursor;
                } else {
                    remove_sign_char_before_cursor(
                        &mut editor.lines[editor.line],
                        &mut editor.cursor,
                    );
                    editor.selection = editor.cursor;
                }
            }
        }
        KeyCode::Delete => {
            let control_down = input.control_down();
            if let Some(editor) = &mut input.sign_editor {
                if delete_sign_selection(editor) {
                    return true;
                } else if control_down {
                    text_edit::remove_word_at_cursor(&mut editor.lines[editor.line], editor.cursor);
                    editor.selection = editor.cursor;
                } else {
                    remove_sign_char_at_cursor(&mut editor.lines[editor.line], editor.cursor);
                    editor.selection = editor.cursor;
                }
            }
        }
        _ => {}
    }
    true
}

fn sync_sign_editor_input(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) -> bool {
    let Some(signature) = sign_editor_signature_from_world(world) else {
        input.sign_editor = None;
        return false;
    };
    if input
        .sign_editor
        .as_ref()
        .is_some_and(|editor| editor.signature == signature)
    {
        return false;
    }
    if input.dismissed_sign_editor.as_ref() == Some(&signature) {
        input.sign_editor = None;
        return false;
    }

    let lines = sign_editor_initial_lines(world);
    let cursor = sign_line_char_len(&lines[0]);
    input.sign_editor = Some(SignEditorInputState {
        signature,
        lines,
        line: 0,
        cursor,
        selection: cursor,
    });
    release_active_input(input, world, counters, net_commands);
    true
}

fn sign_editor_signature_from_world(world: &WorldStore) -> Option<SignEditorInputSignature> {
    let editor = world.last_open_sign_editor()?;
    let open_count = world.counters().open_sign_editor_packets;
    if open_count == 0 {
        return None;
    }
    Some(SignEditorInputSignature {
        open_count,
        pos: protocol_block_pos_from_world(editor.pos),
        is_front_text: editor.is_front_text,
    })
}

fn sign_editor_initial_lines(world: &WorldStore) -> [String; 4] {
    world
        .last_open_sign_editor()
        .and_then(|editor| world.sign_text_lines(editor.pos, editor.is_front_text))
        .unwrap_or_else(|| std::array::from_fn(|_| String::new()))
}

fn sign_editor_hud_state_from_editor(editor: &SignEditorInputState) -> SignEditorHudState {
    SignEditorHudState {
        pos: world_block_pos_from_protocol(editor.signature.pos),
        is_front_text: editor.signature.is_front_text,
        lines: editor.lines.clone(),
        line: editor.line,
        cursor: editor.cursor,
        selection: editor.selection,
    }
}

fn world_block_pos_from_protocol(pos: ProtocolBlockPos) -> BlockPos {
    BlockPos {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

fn submit_sign_editor(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    let Some(editor) = input.sign_editor.take() else {
        return;
    };
    input.dismissed_sign_editor = Some(editor.signature.clone());
    queue_sign_update_command(
        counters,
        net_commands,
        SignUpdate {
            pos: editor.signature.pos,
            is_front_text: editor.signature.is_front_text,
            lines: editor.lines,
        },
    );
}

fn set_sign_editor_line(editor: &mut SignEditorInputState, line: usize) {
    editor.line = line;
    set_sign_editor_cursor(editor, sign_line_char_len(&editor.lines[editor.line]));
}

fn insert_sign_line_text(editor: &mut SignEditorInputState, text: &str) {
    delete_sign_selection(editor);
    let current = &mut editor.lines[editor.line];
    editor.cursor = editor.cursor.min(sign_line_char_len(current));
    let mut remaining = SIGN_LINE_MAX_LENGTH.saturating_sub(sign_line_len(current));
    for ch in text.chars().filter(|ch| is_sign_text_char(*ch)) {
        let len = ch.len_utf16();
        if len > remaining {
            break;
        }
        let insert_at = sign_line_byte_index(current, editor.cursor);
        current.insert(insert_at, ch);
        editor.cursor += 1;
        remaining -= len;
    }
    editor.selection = editor.cursor;
}

fn set_sign_editor_cursor(editor: &mut SignEditorInputState, cursor: usize) {
    editor.cursor = cursor.min(sign_line_char_len(&editor.lines[editor.line]));
    editor.selection = editor.cursor;
}

fn select_sign_editor_line(editor: &mut SignEditorInputState) {
    editor.selection = 0;
    editor.cursor = sign_line_char_len(&editor.lines[editor.line]);
}

fn delete_sign_selection(editor: &mut SignEditorInputState) -> bool {
    if editor.selection == editor.cursor {
        return false;
    }
    let start = editor.selection.min(editor.cursor);
    let end = editor.selection.max(editor.cursor);
    let line = &mut editor.lines[editor.line];
    let start_byte = sign_line_byte_index(line, start);
    let end_byte = sign_line_byte_index(line, end);
    line.replace_range(start_byte..end_byte, "");
    editor.cursor = start;
    editor.selection = start;
    true
}

fn is_sign_text_char(ch: char) -> bool {
    ch != '\u{a7}' && ch >= ' ' && ch != '\u{7f}'
}

fn sign_line_len(text: &str) -> usize {
    text.encode_utf16().count()
}

fn sign_line_char_len(text: &str) -> usize {
    text.chars().count()
}

fn sign_line_byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(index, _)| index)
}

fn remove_sign_char_before_cursor(current: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let start = sign_line_byte_index(current, *cursor - 1);
    let end = sign_line_byte_index(current, *cursor);
    current.replace_range(start..end, "");
    *cursor -= 1;
}

fn remove_sign_char_at_cursor(current: &mut String, cursor: usize) {
    if cursor >= sign_line_char_len(current) {
        return;
    }
    let start = sign_line_byte_index(current, cursor);
    let end = sign_line_byte_index(current, cursor + 1);
    current.replace_range(start..end, "");
}

fn player_input_from_state(input: &ClientInputState) -> PlayerInput {
    PlayerInput {
        forward: input.forward,
        backward: input.backward,
        left: input.left,
        right: input.right,
        jump: input.jump,
        shift: input.sneak,
        sprint: input.sprint,
    }
}

fn local_player_input_from_state(input: &ClientInputState) -> LocalPlayerInputState {
    LocalPlayerInputState {
        focused: input.focused,
        forward: input.forward,
        backward: input.backward,
        left: input.left,
        right: input.right,
        jump: input.jump,
        sneak: input.sneak,
        sprint: input.sprint,
        mouse_delta_x: input.mouse_delta_x,
        mouse_delta_y: input.mouse_delta_y,
    }
}

fn effective_sprinting_from_state(world: &WorldStore, input: &ClientInputState) -> bool {
    world.local_player_effective_sprint(local_player_input_from_state(input))
}

#[cfg(test)]
mod tests;
