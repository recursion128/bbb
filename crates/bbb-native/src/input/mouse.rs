use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::packets::{Direction as ProtocolDirection, InteractionHand, PlayerActionKind};
use bbb_world::WorldStore;
use tokio::sync::mpsc;
use winit::event::{ElementState, MouseButton};

use crate::crosshair::crosshair_block_hit_from_world;

use super::{
    commands::{
        queue_pick_item_from_block_command, queue_player_action_command, queue_swing_command,
        queue_use_item_command, queue_use_item_on_command, queue_zero_pos_player_action_command,
    },
    ClientInputState,
};

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

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_control::PlayerPose;
    use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, PlayerAction, UseItem};
    use bbb_world::BlockPos;

    use crate::crosshair::CrosshairBlockHit;

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
}
