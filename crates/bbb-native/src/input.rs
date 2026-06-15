use std::time::Instant;

use bbb_control::{NetCounters, PlayerPose};
use bbb_net::NetCommand;
use bbb_protocol::packets::{
    Direction as ProtocolDirection, PlayerActionKind, PlayerCommandAction, PlayerInput,
};
use bbb_world::WorldStore;
use tokio::sync::mpsc;
use winit::{
    event::ElementState,
    keyboard::{KeyCode, PhysicalKey},
};

use crate::crosshair::CrosshairBlockHit;

mod commands;
mod mouse;
mod movement;

pub(crate) use commands::queue_vehicle_move_command;
use commands::*;
pub(crate) use mouse::{handle_mouse_input, handle_mouse_motion, handle_mouse_wheel};
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
    mouse_delta_x: f64,
    mouse_delta_y: f64,
    scroll_accumulated_x: f64,
    scroll_accumulated_y: f64,
    last_step: Option<Instant>,
    last_move_command_at: Option<Instant>,
    last_move_command_pose: Option<PlayerPose>,
    destroying_block: Option<CrosshairBlockHit>,
    using_item: bool,
    prediction_sequence: i32,
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
        self.mouse_delta_x = 0.0;
        self.mouse_delta_y = 0.0;
        self.scroll_accumulated_x = 0.0;
        self.scroll_accumulated_y = 0.0;
    }

    fn next_prediction_sequence(&mut self) -> i32 {
        self.prediction_sequence = if self.prediction_sequence == i32::MAX {
            1
        } else {
            self.prediction_sequence + 1
        };
        self.prediction_sequence
    }
}

pub(crate) fn handle_focus_change(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    focused: bool,
) {
    input.focused = focused;
    if !focused {
        let before = player_input_from_state(input);
        input.clear_pressed();
        let after = player_input_from_state(input);
        if after != before {
            queue_player_input_command(counters, net_commands, after);
        }
        if let Some(hit) = input.destroying_block.take() {
            queue_player_action_command(
                counters,
                net_commands,
                PlayerActionKind::AbortDestroyBlock,
                hit.pos,
                ProtocolDirection::Down,
                0,
            );
        }
        if input.using_item {
            input.using_item = false;
            queue_zero_pos_player_action_command(
                counters,
                net_commands,
                PlayerActionKind::ReleaseUseItem,
            );
        }
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
    let pressed = matches!(state, ElementState::Pressed);
    let PhysicalKey::Code(code) = physical_key else {
        return;
    };

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
                queue_sprint_command(counters, net_commands, after.sprint);
            }
        }
    }
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
