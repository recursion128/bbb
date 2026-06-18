use std::time::Instant;

use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::packets::{
    Direction as ProtocolDirection, PlayerActionKind, PlayerCommandAction, PlayerInput,
};
use bbb_world::{LocalPlayerPoseState, WorldStore};
use tokio::sync::mpsc;
use winit::{
    event::ElementState,
    keyboard::{KeyCode, PhysicalKey},
};

mod bundle;
mod commands;
mod inventory;
mod mouse;
mod movement;

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
    queue_rename_item_command, queue_seen_advancements_command, queue_select_trade_command,
    queue_set_beacon_command, queue_sign_update_command, queue_spectate_entity_command,
    queue_teleport_to_entity_command, queue_vehicle_move_command, select_hotbar_slot,
};
pub(crate) use inventory::{
    handle_inventory_cursor_moved, handle_inventory_key_input, handle_inventory_mouse_input,
    handle_inventory_mouse_wheel, inventory_screen_layout, InventoryScreenBackground,
};
pub(crate) use mouse::{
    advance_destroying_block_at_partial_tick, advance_using_item_at_partial_tick,
    handle_mouse_input_at_partial_tick, handle_mouse_motion, handle_mouse_wheel,
};
pub(crate) use movement::advance_player_input;

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
    use_item_repeat_delay_ticks: u8,
    mouse_delta_x: f64,
    mouse_delta_y: f64,
    scroll_accumulated_x: f64,
    scroll_accumulated_y: f64,
    bundle_scroll_accumulated_x: f64,
    bundle_scroll_accumulated_y: f64,
    inventory_hovered_slot: Option<i16>,
    inventory_last_click_slot: Option<i16>,
    inventory_last_click_button_num: Option<i8>,
    inventory_last_click_at: Option<Instant>,
    inventory_quick_craft_button_num: Option<i8>,
    inventory_quick_craft_slots: Vec<i16>,
    chat_entry: Option<ChatEntryState>,
    last_step: Option<Instant>,
    last_move_command_at: Option<Instant>,
    last_move_position_command_at: Option<Instant>,
    last_move_command_pose: Option<LocalPlayerPoseState>,
    last_paddle_boat_command_at: Option<Instant>,
    riding_jump_charge_seconds: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ChatEntryState {
    text: String,
    last_suggestion_request_text: Option<String>,
    suppress_open_key_commit: bool,
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
        self.use_item_repeat_delay_ticks = 0;
        self.mouse_delta_x = 0.0;
        self.mouse_delta_y = 0.0;
        self.scroll_accumulated_x = 0.0;
        self.scroll_accumulated_y = 0.0;
        self.bundle_scroll_accumulated_x = 0.0;
        self.bundle_scroll_accumulated_y = 0.0;
        self.inventory_hovered_slot = None;
        self.inventory_last_click_slot = None;
        self.inventory_last_click_button_num = None;
        self.inventory_last_click_at = None;
        self.inventory_quick_craft_button_num = None;
        self.inventory_quick_craft_slots.clear();
        self.chat_entry = None;
        self.last_paddle_boat_command_at = None;
        self.riding_jump_charge_seconds = None;
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

    fn shift_down(&self) -> bool {
        self.shift_left_down || self.shift_right_down
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

    pub(crate) fn chat_entry_is_active(&self) -> bool {
        self.chat_entry.is_some()
    }

    pub(crate) fn inventory_hovered_slot(&self) -> Option<i16> {
        self.inventory_hovered_slot
    }
}

pub(crate) fn release_active_input(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    let before = player_input_from_state(input);
    input.clear_pressed();
    let after = player_input_from_state(input);
    if after != before {
        queue_player_input_command(counters, net_commands, after);
        if before.sprint != after.sprint {
            queue_sprint_command(counters, world, net_commands, after.sprint);
        }
    }
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
    input.focused = focused;
    if !focused {
        release_active_input(input, world, counters, net_commands);
        input.clear_modifiers();
    }
}

pub(crate) fn handle_key_input(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
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
    if matches!(code, KeyCode::ShiftLeft | KeyCode::ShiftRight) {
        input.set_shift_key(code, pressed);
    }
    if matches!(code, KeyCode::ControlLeft | KeyCode::ControlRight) {
        input.set_control_key(code, pressed);
    }

    if input.command_entry_is_active() {
        handle_chat_entry_key(input, counters, world, net_commands, code, pressed);
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
        if matches!(code, KeyCode::Escape | KeyCode::KeyE)
            && queue_container_close_command(counters, world, net_commands)
        {
            return;
        }
    }
    if inventory_screen_layout(world).is_some() {
        if pressed {
            handle_inventory_key_input(input, world, counters, net_commands, code);
        }
        return;
    }
    if world.open_container_id().is_some() {
        return;
    }

    if pressed {
        if let Some(slot) = hotbar_slot_for_key(code) {
            select_hotbar_slot(counters, world, net_commands, slot);
            return;
        }
        match code {
            KeyCode::KeyQ => {
                let action = if input.sprint {
                    PlayerActionKind::DropAllItems
                } else {
                    PlayerActionKind::DropItem
                };
                queue_zero_pos_player_action_command(counters, net_commands, action);
                return;
            }
            KeyCode::KeyF => {
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
    let handled = match code {
        KeyCode::KeyW | KeyCode::ArrowUp => {
            input.forward = pressed;
            true
        }
        KeyCode::KeyS | KeyCode::ArrowDown => {
            input.backward = pressed;
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
            true
        }
        KeyCode::ControlLeft | KeyCode::ControlRight => {
            input.sprint = pressed;
            true
        }
        _ => false,
    };
    if handled {
        let after = player_input_from_state(input);
        if after != before {
            queue_player_input_command(counters, net_commands, after);
            if before.sprint != after.sprint {
                queue_sprint_command(counters, world, net_commands, after.sprint);
            }
            if should_queue_start_fall_flying(world, before, after) {
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

pub(crate) fn handle_text_input(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    text: &str,
) {
    if !input.focused {
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
    for ch in text.chars().filter(|ch| is_chat_text_char(*ch)) {
        entry.text.push(ch);
    }
    if entry.text.is_empty() {
        input.chat_entry = None;
        return;
    }

    if entry.text.starts_with('/') {
        maybe_queue_command_suggestion_request(input, counters, world, net_commands);
    }
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
        KeyCode::Backspace => {
            if let Some(entry) = &mut input.chat_entry {
                entry.text.pop();
                if entry.text.is_empty() {
                    input.chat_entry = None;
                } else if entry.text.starts_with('/') {
                    maybe_queue_command_suggestion_request(input, counters, world, net_commands);
                }
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
    !ch.is_control() && ch != '\u{7f}'
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

#[cfg(test)]
mod tests;
