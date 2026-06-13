use std::time::{Duration, Instant};

use bbb_control::{NetCounters, NetVec3, PlayerPose};
use bbb_net::{NetCommand, PlayerMoveCommand};
use bbb_protocol::packets::{
    Direction as ProtocolDirection, InteractionHand, PlayerActionKind, PlayerCommandAction,
    PlayerInput,
};
use bbb_world::WorldStore;
use tokio::sync::mpsc;
use winit::{
    event::{ElementState, MouseButton},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::crosshair::{crosshair_block_hit_from_world, CrosshairBlockHit};
use crate::runtime::player_position_state_from_pose;

mod commands;

pub(crate) use commands::queue_vehicle_move_command;
use commands::*;

const INPUT_MOUSE_SENSITIVITY_DEGREES: f32 = 0.12;
const INPUT_WALK_SPEED_BLOCKS_PER_SECOND: f64 = 4.317;
const INPUT_SPRINT_SPEED_BLOCKS_PER_SECOND: f64 = 5.612;
const MOVE_COMMAND_INTERVAL: Duration = Duration::from_millis(50);

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

pub(crate) fn handle_mouse_motion(input: &mut ClientInputState, delta: (f64, f64)) {
    if !input.focused {
        return;
    }
    input.mouse_delta_x += delta.0;
    input.mouse_delta_y += delta.1;
}

pub(crate) fn handle_mouse_input(
    input: &mut ClientInputState,
    world: &WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    button: MouseButton,
    state: ElementState,
) {
    if !input.focused {
        return;
    }
    match (button, state) {
        (MouseButton::Left, ElementState::Pressed) => {
            if let Some(hit) = crosshair_block_hit_from_world(world, counters.player_pose) {
                let sequence = input.next_prediction_sequence();
                queue_player_action_command(
                    counters,
                    net_commands,
                    PlayerActionKind::StartDestroyBlock,
                    hit.pos,
                    hit.face,
                    sequence,
                );
                input.destroying_block = Some(hit);
            }
            queue_swing_command(counters, net_commands, InteractionHand::MainHand);
        }
        (MouseButton::Left, ElementState::Released) => {
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
        }
        (MouseButton::Right, ElementState::Pressed) => {
            if let Some(hit) = crosshair_block_hit_from_world(world, counters.player_pose) {
                let sequence = input.next_prediction_sequence();
                queue_use_item_on_command(counters, net_commands, hit, sequence);
            } else if let Some(pose) = counters.player_pose {
                let sequence = input.next_prediction_sequence();
                input.using_item = queue_use_item_command(
                    counters,
                    net_commands,
                    InteractionHand::MainHand,
                    pose,
                    sequence,
                );
            }
        }
        (MouseButton::Right, ElementState::Released) => {
            if input.using_item {
                input.using_item = false;
                queue_zero_pos_player_action_command(
                    counters,
                    net_commands,
                    PlayerActionKind::ReleaseUseItem,
                );
            }
        }
        (MouseButton::Middle, ElementState::Pressed) => {
            if let Some(hit) = crosshair_block_hit_from_world(world, counters.player_pose) {
                queue_pick_item_from_block_command(counters, net_commands, hit.pos, input.sprint);
            }
        }
        _ => {}
    }
}

pub(crate) fn advance_player_input(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    now: Instant,
) {
    let dt_seconds = input
        .last_step
        .and_then(|last| now.checked_duration_since(last))
        .unwrap_or_default()
        .as_secs_f64()
        .min(0.25);
    input.last_step = Some(now);

    let Some(current_pose) = counters.player_pose else {
        input.mouse_delta_x = 0.0;
        input.mouse_delta_y = 0.0;
        return;
    };

    let pose = integrate_player_input_pose(current_pose, input, dt_seconds);
    input.mouse_delta_x = 0.0;
    input.mouse_delta_y = 0.0;
    counters.player_pose = Some(pose);
    maybe_queue_player_move_command(input, counters, net_commands, pose, now);
}

fn integrate_player_input_pose(
    mut pose: PlayerPose,
    input: &ClientInputState,
    dt_seconds: f64,
) -> PlayerPose {
    if input.focused {
        pose.y_rot =
            wrap_degrees(pose.y_rot + input.mouse_delta_x as f32 * INPUT_MOUSE_SENSITIVITY_DEGREES);
        pose.x_rot = (pose.x_rot + input.mouse_delta_y as f32 * INPUT_MOUSE_SENSITIVITY_DEGREES)
            .clamp(-90.0, 90.0);
    }

    let forward_input = axis(input.forward, input.backward);
    let strafe_input = axis(input.right, input.left);
    let vertical_input = axis(input.jump, input.sneak);
    let speed = if input.sprint {
        INPUT_SPRINT_SPEED_BLOCKS_PER_SECOND
    } else {
        INPUT_WALK_SPEED_BLOCKS_PER_SECOND
    };
    let yaw = f64::from(pose.y_rot).to_radians();
    let forward = (-yaw.sin(), yaw.cos());
    let right = (-yaw.cos(), -yaw.sin());
    let mut move_x = forward.0 * forward_input + right.0 * strafe_input;
    let mut move_z = forward.1 * forward_input + right.1 * strafe_input;
    let horizontal_len = (move_x * move_x + move_z * move_z).sqrt();
    if horizontal_len > f64::EPSILON {
        move_x /= horizontal_len;
        move_z /= horizontal_len;
    }

    pose.position.x += move_x * speed * dt_seconds;
    pose.position.y += vertical_input * speed * dt_seconds;
    pose.position.z += move_z * speed * dt_seconds;
    pose.delta_movement = NetVec3 {
        x: move_x * speed / 20.0,
        y: vertical_input * speed / 20.0,
        z: move_z * speed / 20.0,
    };
    pose
}

fn maybe_queue_player_move_command(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    pose: PlayerPose,
    now: Instant,
) {
    let Some(tx) = net_commands else {
        return;
    };
    let command_due = input
        .last_move_command_at
        .and_then(|last| now.checked_duration_since(last))
        .map_or(true, |elapsed| elapsed >= MOVE_COMMAND_INTERVAL);
    if !command_due || input.last_move_command_pose == Some(pose) {
        return;
    }

    let command = NetCommand::MovePlayer(PlayerMoveCommand {
        state: player_position_state_from_pose(pose),
        on_ground: pose.delta_movement.y.abs() <= f64::EPSILON,
        horizontal_collision: false,
    });
    if tx.try_send(command).is_ok() {
        input.last_move_command_at = Some(now);
        input.last_move_command_pose = Some(pose);
        counters.player_move_commands_queued += 1;
    }
}

fn axis(positive: bool, negative: bool) -> f64 {
    match (positive, negative) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    }
}

fn wrap_degrees(degrees: f32) -> f32 {
    (degrees + 180.0).rem_euclid(360.0) - 180.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        BlockHitResult as ProtocolBlockHitResult, BlockPos as ProtocolBlockPos, PickItemFromBlock,
        PlayerAction, PlayerCommand, UseItem, UseItemOn,
    };
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

    #[test]
    fn left_mouse_press_queues_main_hand_swing() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let world = WorldStore::new();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );
        assert!(input.destroying_block.is_none());

        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );

        handle_mouse_input(
            &mut input,
            &world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Released,
        );
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.swing_commands_queued, 1);
    }

    #[test]
    fn unfocused_mouse_press_does_not_queue_swing() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(false);
        let world = WorldStore::new();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert_eq!(counters.swing_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn queues_start_destroy_block_for_crosshair_hit() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();
        let hit = CrosshairBlockHit {
            pos: BlockPos { x: 1, y: 64, z: -2 },
            face: ProtocolDirection::West,
            cursor: [0.0, 0.5, 0.5],
            inside: false,
        };

        queue_player_action_command(
            &mut counters,
            &commands,
            PlayerActionKind::StartDestroyBlock,
            hit.pos,
            hit.face,
            3,
        );

        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                pos: ProtocolBlockPos { x: 1, y: 64, z: -2 },
                direction: ProtocolDirection::West,
                sequence: 3,
            })
        );
    }

    #[test]
    fn left_mouse_release_aborts_destroying_block() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.destroying_block = Some(CrosshairBlockHit {
            pos: BlockPos { x: 2, y: 65, z: -3 },
            face: ProtocolDirection::East,
            cursor: [1.0, 0.5, 0.5],
            inside: false,
        });
        let world = WorldStore::new();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Released,
        );

        assert!(input.destroying_block.is_none());
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::AbortDestroyBlock,
                pos: ProtocolBlockPos { x: 2, y: 65, z: -3 },
                direction: ProtocolDirection::Down,
                sequence: 0,
            })
        );
    }

    #[test]
    fn queues_use_item_on_for_crosshair_hit() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();
        let hit = CrosshairBlockHit {
            pos: BlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            face: ProtocolDirection::South,
            cursor: [0.25, 0.5, 0.75],
            inside: false,
        };

        queue_use_item_on_command(&mut counters, &commands, hit, 5);

        assert_eq!(counters.use_item_on_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItemOn(UseItemOn {
                hand: InteractionHand::MainHand,
                hit: ProtocolBlockHitResult {
                    pos: ProtocolBlockPos {
                        x: -5,
                        y: 70,
                        z: 12
                    },
                    direction: ProtocolDirection::South,
                    cursor_x: 0.25,
                    cursor_y: 0.5,
                    cursor_z: 0.75,
                    inside: false,
                    world_border_hit: false,
                },
                sequence: 5,
            })
        );
    }

    #[test]
    fn queues_pick_item_from_block_for_crosshair_hit() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();

        queue_pick_item_from_block_command(
            &mut counters,
            &commands,
            BlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            true,
        );

        assert_eq!(counters.pick_item_from_block_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PickItemFromBlock(PickItemFromBlock {
                pos: ProtocolBlockPos {
                    x: -5,
                    y: 70,
                    z: 12,
                },
                include_data: true,
            })
        );
    }

    #[test]
    fn right_mouse_press_without_block_queues_use_item() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let world = WorldStore::new();
        let mut counters = NetCounters {
            player_pose: Some(PlayerPose {
                y_rot: 45.0,
                x_rot: -20.0,
                ..PlayerPose::default()
            }),
            ..NetCounters::default()
        };

        handle_mouse_input(
            &mut input,
            &world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(input.using_item);
        assert_eq!(counters.use_item_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItem(UseItem {
                hand: InteractionHand::MainHand,
                sequence: 1,
                y_rot: 45.0,
                x_rot: -20.0,
            })
        );

        handle_mouse_input(
            &mut input,
            &world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Released,
        );

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

    #[test]
    fn player_input_moves_forward_with_minecraft_yaw() {
        let mut input = ClientInputState::new(true);
        input.forward = true;
        let pose = integrate_player_input_pose(
            PlayerPose {
                position: vec3(0.0, 64.0, 0.0),
                y_rot: 0.0,
                ..PlayerPose::default()
            },
            &input,
            1.0,
        );

        assert_f64_near(pose.position.x, 0.0, 0.000001);
        assert_f64_near(pose.position.y, 64.0, 0.000001);
        assert_f64_near(
            pose.position.z,
            INPUT_WALK_SPEED_BLOCKS_PER_SECOND,
            0.000001,
        );
        assert_f64_near(
            pose.delta_movement.z,
            INPUT_WALK_SPEED_BLOCKS_PER_SECOND / 20.0,
            0.000001,
        );
    }

    #[test]
    fn player_input_rotates_and_clamps_pitch() {
        let mut input = ClientInputState::new(true);
        input.mouse_delta_x = 100.0;
        input.mouse_delta_y = 1000.0;
        let pose = integrate_player_input_pose(
            PlayerPose {
                position: vec3(0.0, 64.0, 0.0),
                ..PlayerPose::default()
            },
            &input,
            0.0,
        );

        assert_eq!(pose.y_rot, 12.0);
        assert_eq!(pose.x_rot, 90.0);
    }

    #[test]
    fn advance_player_input_queues_move_commands_at_tick_interval() {
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters {
            player_pose: Some(PlayerPose {
                position: vec3(0.0, 64.0, 0.0),
                ..PlayerPose::default()
            }),
            ..NetCounters::default()
        };

        advance_player_input(&mut input, &mut counters, &commands, start);
        let first = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected move command, got {other:?}"),
        };
        assert_f64_near(first.state.position.y, 64.0, 0.000001);
        assert!(first.on_ground);
        assert_eq!(counters.player_move_commands_queued, 1);

        input.forward = true;
        advance_player_input(
            &mut input,
            &mut counters,
            &commands,
            start + Duration::from_millis(25),
        );
        assert!(rx.try_recv().is_err());

        advance_player_input(
            &mut input,
            &mut counters,
            &commands,
            start + Duration::from_millis(50),
        );
        let second = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected move command, got {other:?}"),
        };
        assert!(second.state.position.z > 0.0);
        assert_eq!(counters.player_move_commands_queued, 2);
    }

    fn vec3(x: f64, y: f64, z: f64) -> NetVec3 {
        NetVec3 { x, y, z }
    }

    fn assert_f64_near(actual: f64, expected: f64, epsilon: f64) {
        assert!(
            (actual - expected).abs() <= epsilon,
            "expected {actual} to be within {epsilon} of {expected}"
        );
    }
}
