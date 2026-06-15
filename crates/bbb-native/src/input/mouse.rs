use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::packets::{Direction as ProtocolDirection, InteractionHand, PlayerActionKind};
use bbb_world::WorldStore;
use tokio::sync::mpsc;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};

use crate::crosshair::{crosshair_target_from_world, CrosshairTarget};
use crate::runtime::player_pose_from_local_player_pose;

use super::{
    commands::{
        hotbar_slot_for_scroll, queue_attack_entity_command, queue_interact_entity_command,
        queue_pick_item_from_block_command, queue_pick_item_from_entity_command,
        queue_player_action_command, queue_swing_command, queue_use_item_command,
        queue_use_item_on_command, queue_zero_pos_player_action_command, select_hotbar_slot,
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
    let player_pose = world
        .local_player_pose()
        .map(player_pose_from_local_player_pose);
    match (button, state) {
        (MouseButton::Left, ElementState::Pressed) => {
            match crosshair_target_from_world(world, player_pose) {
                Some(CrosshairTarget::Entity(hit)) => {
                    queue_attack_entity_command(counters, net_commands, hit.entity_id);
                }
                Some(CrosshairTarget::Block(hit)) => {
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
                None => {}
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
            match crosshair_target_from_world(world, player_pose) {
                Some(CrosshairTarget::Entity(hit)) => {
                    queue_interact_entity_command(
                        counters,
                        net_commands,
                        hit.entity_id,
                        InteractionHand::MainHand,
                        hit.relative_location,
                        input.sneak,
                    );
                }
                Some(CrosshairTarget::Block(hit)) => {
                    let sequence = input.next_prediction_sequence();
                    queue_use_item_on_command(counters, net_commands, hit, sequence);
                }
                None => {
                    if let Some(pose) = player_pose {
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
            match crosshair_target_from_world(world, player_pose) {
                Some(CrosshairTarget::Entity(hit)) => {
                    queue_pick_item_from_entity_command(
                        counters,
                        net_commands,
                        hit.entity_id,
                        input.sprint,
                    );
                }
                Some(CrosshairTarget::Block(hit)) => {
                    queue_pick_item_from_block_command(
                        counters,
                        net_commands,
                        hit.pos,
                        input.sprint,
                    );
                }
                None => {}
            }
        }
        _ => {}
    }
}

pub(crate) fn handle_mouse_wheel(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    delta: MouseScrollDelta,
) {
    if !input.focused {
        return;
    }
    let Some(wheel) = wheel_steps_from_scroll(input, delta) else {
        return;
    };
    let current_slot = world.local_player().selected_hotbar_slot;
    if let Some(slot) = hotbar_slot_for_scroll(wheel, current_slot) {
        select_hotbar_slot(counters, world, net_commands, slot);
    }
}

fn wheel_steps_from_scroll(input: &mut ClientInputState, delta: MouseScrollDelta) -> Option<i32> {
    let (x, y) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (f64::from(x), f64::from(y)),
        MouseScrollDelta::PixelDelta(pos) => (pos.x, pos.y),
    };

    if input.scroll_accumulated_x != 0.0
        && scroll_signum(x) != scroll_signum(input.scroll_accumulated_x)
    {
        input.scroll_accumulated_x = 0.0;
    }
    if input.scroll_accumulated_y != 0.0
        && scroll_signum(y) != scroll_signum(input.scroll_accumulated_y)
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
    Some(if wheel_y == 0 { -wheel_x } else { wheel_y })
}

fn scroll_signum(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_control::PlayerPose;
    use bbb_protocol::packets::{
        AddEntity, AttackEntity, BlockPos as ProtocolBlockPos, InteractEntity, PickItemFromEntity,
        PlayerAction, UseItem, Vec3d as ProtocolVec3d,
    };
    use bbb_world::BlockPos;
    use uuid::Uuid;

    use crate::crosshair::CrosshairBlockHit;
    use crate::runtime::local_player_pose_from_player_pose;

    const VANILLA_ENTITY_TYPE_MINECART_ID: i32 = 85;

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
        let mut world = WorldStore::new();
        world.set_local_player_pose(local_player_pose_from_player_pose(PlayerPose {
            y_rot: 45.0,
            x_rot: -20.0,
            ..PlayerPose::default()
        }));
        let mut counters = NetCounters::default();

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
    fn left_mouse_press_on_entity_queues_attack_and_swing() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let world = world_with_crosshair_entity(123);
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
        assert_eq!(counters.attack_entity_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::AttackEntity(AttackEntity { entity_id: 123 })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
    }

    #[test]
    fn right_mouse_press_on_entity_queues_interact_with_relative_hit_location() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.sneak = true;
        let world = world_with_crosshair_entity(123);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(!input.using_item);
        assert_eq!(counters.interact_entity_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::InteractEntity(InteractEntity {
                entity_id: 123,
                hand: InteractionHand::MainHand,
                location: ProtocolVec3d {
                    x: 0.0,
                    y: 0.6200000047683716,
                    z: -0.49000000953674316,
                },
                using_secondary_action: true,
            })
        );
    }

    #[test]
    fn middle_mouse_press_on_entity_queues_pick_entity() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.sprint = true;
        let world = world_with_crosshair_entity(123);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &world,
            &mut counters,
            &commands,
            MouseButton::Middle,
            ElementState::Pressed,
        );

        assert_eq!(counters.pick_item_from_entity_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PickItemFromEntity(PickItemFromEntity {
                entity_id: 123,
                include_data: true,
            })
        );
    }

    #[test]
    fn mouse_wheel_selects_hotbar_slot_updates_world_and_queues_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        assert!(world.set_local_selected_hotbar_slot(4));
        let mut counters = NetCounters::default();

        handle_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, 1.0),
        );

        assert_eq!(world.local_player().selected_hotbar_slot, 3);
        assert_eq!(world.counters().held_slot_packets, 0);
        assert_eq!(counters.selected_hotbar_slot, 3);
        assert_eq!(counters.held_slot_commands_queued, 1);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(3));
    }

    #[test]
    fn mouse_wheel_wraps_hotbar_selection_like_vanilla() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        handle_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, 1.0),
        );
        assert_eq!(world.local_player().selected_hotbar_slot, 8);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(8));

        handle_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, -1.0),
        );
        assert_eq!(world.local_player().selected_hotbar_slot, 0);
        assert_eq!(counters.selected_hotbar_slot, 0);
        assert_eq!(counters.held_slot_commands_queued, 2);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(0));
    }

    #[test]
    fn unfocused_mouse_wheel_does_not_select_or_queue() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(false);
        let mut world = WorldStore::new();
        assert!(world.set_local_selected_hotbar_slot(4));
        let mut counters = NetCounters::default();

        handle_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, 1.0),
        );

        assert_eq!(world.local_player().selected_hotbar_slot, 4);
        assert_eq!(counters.selected_hotbar_slot, 0);
        assert_eq!(counters.held_slot_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn fractional_mouse_wheel_accumulates_before_selecting() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        assert!(world.set_local_selected_hotbar_slot(4));
        let mut counters = NetCounters::default();

        handle_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, 0.5),
        );
        assert_eq!(world.local_player().selected_hotbar_slot, 4);
        assert!(rx.try_recv().is_err());

        handle_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, 0.5),
        );

        assert_eq!(world.local_player().selected_hotbar_slot, 3);
        assert_eq!(counters.selected_hotbar_slot, 3);
        assert_eq!(counters.held_slot_commands_queued, 1);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(3));
    }

    fn world_with_crosshair_entity(entity_id: i32) -> WorldStore {
        let mut world = WorldStore::new();
        world.set_local_player_pose(local_player_pose_from_player_pose(PlayerPose::default()));
        world.apply_add_entity(AddEntity {
            id: entity_id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id: VANILLA_ENTITY_TYPE_MINECART_ID,
            position: ProtocolVec3d {
                x: 0.0,
                y: 1.0,
                z: 3.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        world
    }
}
