use std::{collections::HashSet, time::Instant};

use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::packets::{
    BlockPos as ProtocolBlockPos, Direction as ProtocolDirection, InteractionHand,
    ItemStackSummary, PlayerActionKind, PlayerCommandAction, PlayerInput, SignUpdate,
};
use bbb_world::{BlockPos, LocalPlayerInputState, LocalPlayerPoseState, WorldStore};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton},
    keyboard::{KeyCode, PhysicalKey},
};

mod bundle;
mod commands;
mod inventory;
mod mouse;
mod movement;
mod text_edit;

use crate::crosshair::protocol_block_pos_from_world;
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
pub(crate) use inventory::{
    anvil_rename_entry_consumes_key, handle_inventory_cursor_moved, handle_inventory_key_input,
    handle_inventory_mouse_input, handle_inventory_mouse_wheel, handle_inventory_text_input,
    inventory_screen_layout, inventory_screen_selected_hotbar_slot_id, recipe_book_button_position,
    recipe_book_main_gui_offset, sync_beacon_effect_selection_state,
    sync_loom_pattern_state_for_hud, sync_stonecutter_recipe_scroll_state,
    InventoryScreenBackground, InventorySlotLayout, RECIPE_BOOK_BUTTON_HEIGHT,
    RECIPE_BOOK_BUTTON_WIDTH,
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
const CHAT_ENTRY_MAX_LENGTH: usize = 256;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BookScreenClickTarget {
    Done,
    PreviousPage,
    NextPage,
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

impl ClientInputState {
    pub(crate) fn new(focused: bool) -> Self {
        Self {
            focused,
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

    pub(crate) fn loom_selected_pattern_index(&self) -> Option<i32> {
        self.loom_selected_pattern_index
    }

    pub(crate) fn beacon_effect_selection(&self) -> (Option<i32>, Option<i32>) {
        (self.beacon_primary_effect, self.beacon_secondary_effect)
    }

    pub(crate) fn anvil_rename_text(&self) -> &str {
        &self.anvil_rename_text
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
            && queue_container_close_command(counters, world, net_commands)
        {
            return;
        }
        if matches!(code, KeyCode::KeyE)
            && !anvil_rename_entry_consumes_key(world, code)
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

    if world.current_book().is_some() {
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
