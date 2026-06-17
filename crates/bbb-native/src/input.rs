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
mod mouse;
mod movement;

pub(crate) use bundle::select_bundle_item;
use commands::*;
pub(crate) use commands::{
    queue_block_entity_tag_query_command, queue_change_difficulty_command,
    queue_change_game_mode_command, queue_chat_command, queue_command_suggestion_request,
    queue_container_button_click_command, queue_container_click_command,
    queue_container_close_request_command, queue_container_slot_state_changed_command,
    queue_edit_book_command, queue_entity_tag_query_command, queue_lock_difficulty_command,
    queue_place_recipe_command, queue_player_abilities_command,
    queue_recipe_book_change_settings_command, queue_recipe_book_seen_recipe_command,
    queue_rename_item_command, queue_seen_advancements_command, queue_select_trade_command,
    queue_set_beacon_command, queue_sign_update_command, queue_spectate_entity_command,
    queue_teleport_to_entity_command, queue_vehicle_move_command, select_hotbar_slot,
};
pub(crate) use mouse::{
    advance_destroying_block_at_partial_tick, handle_mouse_input_at_partial_tick,
    handle_mouse_motion, handle_mouse_wheel,
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
    mouse_delta_x: f64,
    mouse_delta_y: f64,
    scroll_accumulated_x: f64,
    scroll_accumulated_y: f64,
    bundle_scroll_accumulated_x: f64,
    bundle_scroll_accumulated_y: f64,
    command_entry: Option<CommandEntryState>,
    last_step: Option<Instant>,
    last_move_command_at: Option<Instant>,
    last_move_command_pose: Option<LocalPlayerPoseState>,
    last_paddle_boat_command_at: Option<Instant>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct CommandEntryState {
    text: String,
    last_suggestion_request_text: Option<String>,
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
        self.mouse_delta_x = 0.0;
        self.mouse_delta_y = 0.0;
        self.scroll_accumulated_x = 0.0;
        self.scroll_accumulated_y = 0.0;
        self.bundle_scroll_accumulated_x = 0.0;
        self.bundle_scroll_accumulated_y = 0.0;
        self.command_entry = None;
        self.last_paddle_boat_command_at = None;
    }

    pub(crate) fn command_entry_is_active(&self) -> bool {
        self.command_entry.is_some()
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

    if input.command_entry_is_active() {
        handle_command_entry_key(input, counters, world, net_commands, code, pressed);
        return;
    }

    if pressed {
        if matches!(code, KeyCode::Escape | KeyCode::KeyE)
            && queue_container_close_command(counters, world, net_commands)
        {
            return;
        }
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
                queue_player_command_action(
                    counters,
                    world,
                    net_commands,
                    PlayerCommandAction::OpenInventory,
                    0,
                );
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
            input.sneak = pressed;
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
        }
    }
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

    if input.command_entry.is_none() {
        if !text.starts_with('/') {
            return;
        }
        release_active_input(input, world, counters, net_commands);
        input.command_entry = Some(CommandEntryState {
            text: String::new(),
            last_suggestion_request_text: None,
        });
    }

    let Some(entry) = &mut input.command_entry else {
        return;
    };
    for ch in text.chars().filter(|ch| is_command_text_char(*ch)) {
        entry.text.push(ch);
    }
    if entry.text.is_empty() || !entry.text.starts_with('/') {
        input.command_entry = None;
        return;
    }

    maybe_queue_command_suggestion_request(input, counters, world, net_commands);
}

fn handle_command_entry_key(
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
            submit_command_entry(input, counters, net_commands)
        }
        KeyCode::Escape => input.command_entry = None,
        KeyCode::Backspace => {
            if let Some(entry) = &mut input.command_entry {
                entry.text.pop();
                if entry.text.is_empty() {
                    input.command_entry = None;
                } else {
                    maybe_queue_command_suggestion_request(input, counters, world, net_commands);
                }
            }
        }
        _ => {}
    }
}

fn submit_command_entry(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    let Some(entry) = input.command_entry.take() else {
        return;
    };
    let Some(command) = normalize_command_entry(&entry.text) else {
        return;
    };
    queue_chat_command(counters, net_commands, command);
}

fn normalize_command_entry(text: &str) -> Option<String> {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let command = normalized.strip_prefix('/')?;
    if command.is_empty() {
        return None;
    }
    Some(command.to_string())
}

fn maybe_queue_command_suggestion_request(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    let Some(entry) = &mut input.command_entry else {
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

fn is_command_text_char(ch: char) -> bool {
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
