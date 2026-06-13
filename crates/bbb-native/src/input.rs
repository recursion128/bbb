use std::time::Instant;

use bbb_control::{NetCounters, PlayerPose};
use bbb_net::NetCommand;
use bbb_protocol::packets::{
    Direction as ProtocolDirection, PlayerActionKind, PlayerCommandAction, PlayerInput,
};
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
pub(crate) use mouse::{handle_mouse_input, handle_mouse_motion};
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
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    physical_key: PhysicalKey,
    state: ElementState,
) {
    let pressed = matches!(state, ElementState::Pressed);
    let PhysicalKey::Code(code) = physical_key else {
        return;
    };

    if pressed {
        if let Some(slot) = hotbar_slot_for_key(code) {
            select_hotbar_slot(counters, net_commands, slot);
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
mod tests {
    use super::*;
    use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, PlayerAction, PlayerCommand};
    use bbb_world::BlockPos;

    #[test]
    fn prediction_sequence_starts_at_one_and_wraps_positive() {
        let mut input = ClientInputState::new(true);

        assert_eq!(input.next_prediction_sequence(), 1);
        assert_eq!(input.next_prediction_sequence(), 2);

        input.prediction_sequence = i32::MAX;
        assert_eq!(input.next_prediction_sequence(), 1);
    }

    #[test]
    fn digit_key_selects_hotbar_slot_and_queues_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::Digit5),
            ElementState::Pressed,
        );

        assert_eq!(counters.selected_hotbar_slot, 4);
        assert_eq!(counters.held_slot_commands_queued, 1);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(4));
    }

    #[test]
    fn drop_key_queues_drop_item_action() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::KeyQ),
            ElementState::Pressed,
        );

        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::DropItem,
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                direction: ProtocolDirection::Down,
                sequence: 0,
            })
        );
    }

    #[test]
    fn control_drop_key_queues_drop_all_items_action() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.sprint = true;
        let mut counters = NetCounters::default();

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::KeyQ),
            ElementState::Pressed,
        );

        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::DropAllItems,
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                direction: ProtocolDirection::Down,
                sequence: 0,
            })
        );
    }

    #[test]
    fn swap_offhand_key_queues_swap_action() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::KeyF),
            ElementState::Pressed,
        );

        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::SwapItemWithOffhand,
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                direction: ProtocolDirection::Down,
                sequence: 0,
            })
        );
    }

    #[test]
    fn inventory_key_queues_open_inventory_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters {
            player_entity_id: Some(77),
            ..NetCounters::default()
        };

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::KeyE),
            ElementState::Pressed,
        );

        assert_eq!(counters.player_command_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerCommand(PlayerCommand {
                entity_id: 77,
                action: PlayerCommandAction::OpenInventory,
                data: 0,
            })
        );
    }

    #[test]
    fn movement_key_changes_queue_player_input_commands() {
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::KeyW),
            ElementState::Pressed,
        );

        assert_eq!(counters.player_input_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerInput(PlayerInput {
                forward: true,
                ..PlayerInput::default()
            })
        );

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::KeyW),
            ElementState::Pressed,
        );
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.player_input_commands_queued, 1);

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::KeyW),
            ElementState::Released,
        );

        assert_eq!(counters.player_input_commands_queued, 2);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerInput(PlayerInput::default())
        );
    }

    #[test]
    fn sprint_key_queues_player_input_and_sprint_commands() {
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters {
            player_entity_id: Some(77),
            ..NetCounters::default()
        };

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::ControlLeft),
            ElementState::Pressed,
        );

        assert_eq!(counters.player_input_commands_queued, 1);
        assert_eq!(counters.player_command_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerInput(PlayerInput {
                sprint: true,
                ..PlayerInput::default()
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerCommand(PlayerCommand {
                entity_id: 77,
                action: PlayerCommandAction::StartSprinting,
                data: 0,
            })
        );

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::ControlLeft),
            ElementState::Released,
        );

        assert_eq!(counters.player_input_commands_queued, 2);
        assert_eq!(counters.player_command_commands_queued, 2);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerInput(PlayerInput::default())
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerCommand(PlayerCommand {
                entity_id: 77,
                action: PlayerCommandAction::StopSprinting,
                data: 0,
            })
        );
    }

    #[test]
    fn sprint_key_without_player_entity_id_only_queues_input() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::ControlLeft),
            ElementState::Pressed,
        );

        assert_eq!(counters.player_input_commands_queued, 1);
        assert_eq!(counters.player_command_commands_queued, 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerInput(PlayerInput {
                sprint: true,
                ..PlayerInput::default()
            })
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn focus_loss_clears_pressed_input_and_queues_release() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.forward = true;
        input.jump = true;
        input.sprint = true;
        let mut counters = NetCounters::default();

        handle_focus_change(&mut input, &mut counters, &commands, false);

        assert!(!input.focused);
        assert_eq!(player_input_from_state(&input), PlayerInput::default());
        assert_eq!(counters.player_input_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerInput(PlayerInput::default())
        );
    }

    #[test]
    fn focus_loss_aborts_destroying_block() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.destroying_block = Some(CrosshairBlockHit {
            pos: BlockPos { x: 4, y: 70, z: -6 },
            face: ProtocolDirection::North,
            cursor: [0.5, 0.5, 0.0],
            inside: false,
        });
        let mut counters = NetCounters::default();

        handle_focus_change(&mut input, &mut counters, &commands, false);

        assert!(input.destroying_block.is_none());
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::AbortDestroyBlock,
                pos: ProtocolBlockPos { x: 4, y: 70, z: -6 },
                direction: ProtocolDirection::Down,
                sequence: 0,
            })
        );
    }

    #[test]
    fn focus_loss_releases_using_item() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.using_item = true;
        let mut counters = NetCounters::default();

        handle_focus_change(&mut input, &mut counters, &commands, false);

        assert!(!input.using_item);
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::ReleaseUseItem,
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                direction: ProtocolDirection::Down,
                sequence: 0,
            })
        );
    }
}
