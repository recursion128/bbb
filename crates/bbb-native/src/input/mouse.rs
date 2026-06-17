use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::packets::{Direction as ProtocolDirection, InteractionHand, PlayerActionKind};
use bbb_world::{LocalDestroyBlockFinished, WorldStore};
use tokio::sync::mpsc;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};

use crate::camera_pose::camera_pose_from_world;
use crate::crosshair::{
    crosshair_target_from_camera_at_partial_tick, CrosshairBlockHit, CrosshairTarget,
};

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

#[cfg(test)]
fn handle_mouse_input(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    button: MouseButton,
    state: ElementState,
) {
    handle_mouse_input_at_partial_tick(input, world, counters, net_commands, button, state, 1.0);
}

pub(crate) fn handle_mouse_input_at_partial_tick(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    button: MouseButton,
    state: ElementState,
    entity_partial_tick: f32,
) {
    if !input.focused {
        return;
    }
    let player_pose = world.local_player_pose();
    let camera_target = match (button, state) {
        (MouseButton::Left | MouseButton::Right | MouseButton::Middle, ElementState::Pressed) => {
            crosshair_target_from_camera_at_partial_tick(
                world,
                camera_pose_from_world(world),
                entity_partial_tick,
            )
        }
        _ => None,
    };
    match (button, state) {
        (MouseButton::Left, ElementState::Pressed) => {
            input.destroy_block_held = true;
            match camera_target {
                Some(CrosshairTarget::Entity(hit)) => {
                    queue_attack_entity_command(counters, net_commands, hit.entity_id);
                }
                Some(CrosshairTarget::Block(hit)) => {
                    start_destroy_block(counters, world, net_commands, hit);
                }
                None => {}
            }
            queue_swing_command(counters, net_commands, InteractionHand::MainHand);
        }
        (MouseButton::Left, ElementState::Released) => {
            input.destroy_block_held = false;
            abort_destroy_block(counters, world, net_commands, ProtocolDirection::Down);
        }
        (MouseButton::Right, ElementState::Pressed) => match camera_target {
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
                let sequence = world.next_local_prediction_sequence();
                queue_use_item_on_command(counters, net_commands, hit, sequence);
            }
            None => {
                if let Some(pose) = player_pose {
                    let sequence = world.next_local_prediction_sequence();
                    let using_item = queue_use_item_command(
                        counters,
                        net_commands,
                        InteractionHand::MainHand,
                        pose.y_rot,
                        pose.x_rot,
                        sequence,
                    );
                    world.set_local_using_item(using_item);
                }
            }
        },
        (MouseButton::Right, ElementState::Released) => {
            if world.take_local_using_item() {
                queue_zero_pos_player_action_command(
                    counters,
                    net_commands,
                    PlayerActionKind::ReleaseUseItem,
                );
            }
        }
        (MouseButton::Middle, ElementState::Pressed) => match camera_target {
            Some(CrosshairTarget::Entity(hit)) => {
                queue_pick_item_from_entity_command(
                    counters,
                    net_commands,
                    hit.entity_id,
                    input.sprint,
                );
            }
            Some(CrosshairTarget::Block(hit)) => {
                queue_pick_item_from_block_command(counters, net_commands, hit.pos, input.sprint);
            }
            None => {}
        },
        _ => {}
    }
}

pub(crate) fn advance_destroying_block_at_partial_tick(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    entity_partial_tick: f32,
    destroy_ticks: u32,
) {
    if !input.focused || !input.destroy_block_held {
        return;
    }
    if world.tick_local_destroy_delay() {
        return;
    }

    let camera_target = crosshair_target_from_camera_at_partial_tick(
        world,
        camera_pose_from_world(world),
        entity_partial_tick,
    );
    match camera_target {
        Some(CrosshairTarget::Block(hit)) => {
            continue_destroy_block(counters, world, net_commands, hit, destroy_ticks)
        }
        _ => {
            abort_destroy_block(counters, world, net_commands, ProtocolDirection::Down);
        }
    }
}

fn continue_destroy_block(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    hit: CrosshairBlockHit,
    destroy_ticks: u32,
) {
    if world.local_player().interaction.destroying_block == Some(hit.pos)
        && world.local_destroying_block_matches_current_item()
    {
        world.update_local_destroying_block_face(hit.face);
        for _ in 0..destroy_ticks {
            if let Some(finished) = world.advance_local_destroying_block_tick() {
                stop_destroy_block(counters, net_commands, finished);
                break;
            }
        }
        return;
    }

    if let Some(old_pos) = world.take_local_destroying_block() {
        queue_player_action_command(
            counters,
            net_commands,
            PlayerActionKind::AbortDestroyBlock,
            old_pos,
            hit.face,
            0,
        );
    }
    start_destroy_block(counters, world, net_commands, hit);
}

fn start_destroy_block(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    hit: CrosshairBlockHit,
) {
    let sequence = world.next_local_prediction_sequence();
    queue_player_action_command(
        counters,
        net_commands,
        PlayerActionKind::StartDestroyBlock,
        hit.pos,
        hit.face,
        sequence,
    );
    if local_player_instabuild(world) || world.local_destroy_block_is_immediate(hit.pos) {
        world.predict_local_destroy_block(hit.pos, sequence);
        world.set_local_destroy_delay_ticks(5);
        return;
    }
    world.set_local_destroying_block_hit(hit.pos, hit.face);
}

fn stop_destroy_block(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    finished: LocalDestroyBlockFinished,
) {
    queue_player_action_command(
        counters,
        net_commands,
        PlayerActionKind::StopDestroyBlock,
        finished.pos,
        finished.face,
        finished.sequence,
    );
}

fn abort_destroy_block(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    direction: ProtocolDirection,
) {
    if let Some(pos) = world.take_local_destroying_block() {
        queue_player_action_command(
            counters,
            net_commands,
            PlayerActionKind::AbortDestroyBlock,
            pos,
            direction,
            0,
        );
    }
}

fn local_player_instabuild(world: &WorldStore) -> bool {
    world
        .local_player()
        .abilities
        .is_some_and(|abilities| abilities.instabuild)
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
    use bbb_protocol::packets::{
        AddEntity, AttackEntity, BlockHitResult as ProtocolBlockHitResult,
        BlockPos as ProtocolBlockPos, BlockUpdate, InteractEntity,
        ItemStackSummary as ProtocolItemStackSummary, PickItemFromBlock, PickItemFromEntity,
        PlayerAbilities, PlayerAction, SetPlayerInventory as ProtocolSetPlayerInventory, UseItem,
        UseItemOn, Vec3d as ProtocolVec3d,
    };
    use bbb_world::{
        BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, LocalPlayerPoseState,
        PaletteDomain, PaletteKind, PalettedContainerData, WorldDimension,
    };
    use uuid::Uuid;

    const CROSSHAIR_BLOCK_POS: BlockPos = BlockPos { x: 0, y: 1, z: 3 };
    const VANILLA_AIR_BLOCK_STATE_ID: i32 = 0;
    const VANILLA_GRASS_BLOCK_STATE_ID: i32 = 9;
    const VANILLA_ENTITY_TYPE_AXOLOTL_ID: i32 = 7;
    const VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID: i32 = 43;
    const VANILLA_ENTITY_TYPE_MINECART_ID: i32 = 85;

    #[test]
    fn left_mouse_press_queues_main_hand_swing() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );
        assert_eq!(world.local_player().interaction.destroying_block, None);

        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );

        handle_mouse_input(
            &mut input,
            &mut world,
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
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
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
        input.destroy_block_held = true;
        let mut world = WorldStore::new();
        world.set_local_destroying_block(BlockPos { x: 2, y: 65, z: -3 });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Released,
        );

        assert!(!input.destroy_block_held);
        assert_eq!(world.local_player().interaction.destroying_block, None);
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
        world.set_local_player_pose(LocalPlayerPoseState {
            y_rot: 45.0,
            x_rot: -20.0,
            ..LocalPlayerPoseState::default()
        });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(world.local_player().interaction.using_item);
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
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Released,
        );

        assert!(!world.local_player().interaction.using_item);
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
        let mut world = world_with_crosshair_entity(123);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert_eq!(world.local_player().interaction.destroying_block, None);
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
    fn left_mouse_press_on_block_queues_start_destroy_and_swing() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert_eq!(
            world.local_player().interaction.destroying_block,
            Some(CROSSHAIR_BLOCK_POS)
        );
        assert!(input.destroy_block_held);
        assert_eq!(
            world.local_player().interaction.destroying_block_face,
            Some(ProtocolDirection::North)
        );
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                direction: ProtocolDirection::North,
                sequence: 1,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
    }

    #[test]
    fn held_left_mouse_retargets_destroy_block_with_abort_and_start() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.destroy_block_held = true;
        let mut world = world_with_crosshair_block();
        let old_pos = BlockPos { x: 2, y: 65, z: -3 };
        world.set_local_destroying_block_hit(old_pos, ProtocolDirection::South);
        let mut counters = NetCounters::default();

        advance_destroying_block_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            1.0,
            0,
        );

        assert_eq!(
            world.local_player().interaction.destroying_block,
            Some(CROSSHAIR_BLOCK_POS)
        );
        assert_eq!(
            world.local_player().interaction.destroying_block_face,
            Some(ProtocolDirection::North)
        );
        assert_eq!(counters.player_action_commands_queued, 2);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::AbortDestroyBlock,
                pos: ProtocolBlockPos { x: 2, y: 65, z: -3 },
                direction: ProtocolDirection::North,
                sequence: 0,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                direction: ProtocolDirection::North,
                sequence: 1,
            })
        );
    }

    #[test]
    fn held_left_mouse_same_target_restarts_destroy_when_main_hand_item_changes() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.destroy_block_held = true;
        let mut world = world_with_crosshair_block();
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 1),
        });
        world.set_local_destroying_block_hit(CROSSHAIR_BLOCK_POS, ProtocolDirection::North);
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(43, 1),
        });
        let mut counters = NetCounters::default();

        advance_destroying_block_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            1.0,
            1,
        );

        assert_eq!(
            world.local_player().interaction.destroying_block,
            Some(CROSSHAIR_BLOCK_POS)
        );
        assert_eq!(
            world.local_player().interaction.destroying_block_face,
            Some(ProtocolDirection::North)
        );
        assert_eq!(
            world.local_player().interaction.destroying_block_progress,
            0
        );
        assert_eq!(world.local_player().interaction.destroying_block_ticks, 0);
        assert_eq!(counters.player_action_commands_queued, 2);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::AbortDestroyBlock,
                pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                direction: ProtocolDirection::North,
                sequence: 0,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                direction: ProtocolDirection::North,
                sequence: 1,
            })
        );
    }

    #[test]
    fn held_left_mouse_miss_aborts_destroying_block() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.destroy_block_held = true;
        let mut world = WorldStore::new();
        world.set_local_destroying_block_hit(
            BlockPos { x: 2, y: 65, z: -3 },
            ProtocolDirection::South,
        );
        let mut counters = NetCounters::default();

        advance_destroying_block_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            1.0,
            0,
        );

        assert!(input.destroy_block_held);
        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert_eq!(world.local_player().interaction.destroying_block_face, None);
        assert_eq!(counters.player_action_commands_queued, 1);
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
    fn held_left_mouse_same_target_finishes_destroy_with_stop() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.destroy_block_held = true;
        let mut world = world_with_crosshair_block();
        world.set_local_destroying_block_hit(CROSSHAIR_BLOCK_POS, ProtocolDirection::North);
        let mut counters = NetCounters::default();

        advance_destroying_block_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            1.0,
            18,
        );

        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert_eq!(
            world.local_player().interaction.destroying_block_progress,
            0
        );
        assert_eq!(world.local_player().interaction.destroying_block_ticks, 0);
        assert_eq!(world.local_player().interaction.destroy_delay_ticks, 5);
        assert_eq!(
            world
                .probe_block(CROSSHAIR_BLOCK_POS)
                .unwrap()
                .block_state_id,
            VANILLA_AIR_BLOCK_STATE_ID
        );
        assert_eq!(world.local_block_predictions().len(), 1);
        assert_eq!(world.local_block_predictions()[0].sequence, 1);
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::StopDestroyBlock,
                pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                direction: ProtocolDirection::North,
                sequence: 1,
            })
        );
    }

    #[test]
    fn held_left_mouse_same_target_advances_destroy_only_on_client_ticks() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.destroy_block_held = true;
        let mut world = world_with_crosshair_block();
        world.set_local_destroying_block_hit(CROSSHAIR_BLOCK_POS, ProtocolDirection::North);
        let mut counters = NetCounters::default();

        advance_destroying_block_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            1.0,
            0,
        );

        assert_eq!(
            world.local_player().interaction.destroying_block,
            Some(CROSSHAIR_BLOCK_POS)
        );
        assert_eq!(
            world.local_player().interaction.destroying_block_progress,
            0
        );
        assert_eq!(world.local_player().interaction.destroying_block_ticks, 0);
        assert_eq!(counters.player_action_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn creative_left_mouse_press_queues_start_without_destroy_state() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        world.apply_player_abilities(PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert!(input.destroy_block_held);
        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert_eq!(world.local_player().interaction.destroy_delay_ticks, 5);
        assert_eq!(
            world
                .probe_block(CROSSHAIR_BLOCK_POS)
                .unwrap()
                .block_state_id,
            VANILLA_AIR_BLOCK_STATE_ID
        );
        assert_eq!(world.local_block_predictions().len(), 1);
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                direction: ProtocolDirection::North,
                sequence: 1,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
    }

    #[test]
    fn left_mouse_press_uses_active_camera_entity_raycast() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 10.0,
                y: 0.0,
                z: 0.0,
            },
            ..LocalPlayerPoseState::default()
        });
        world.apply_add_entity(AddEntity {
            id: 200,
            uuid: Uuid::from_u128(200),
            entity_type_id: VANILLA_ENTITY_TYPE_AXOLOTL_ID,
            position: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        world.apply_add_entity(AddEntity {
            id: 123,
            uuid: Uuid::from_u128(123),
            entity_type_id: VANILLA_ENTITY_TYPE_MINECART_ID,
            position: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 2.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 200 }));
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert_eq!(world.local_player().interaction.destroying_block, None);
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
        let mut world = world_with_crosshair_entity(123);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(!world.local_player().interaction.using_item);
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
    fn right_mouse_press_on_block_queues_use_item_on() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(!world.local_player().interaction.using_item);
        assert_eq!(counters.use_item_on_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItemOn(UseItemOn {
                hand: InteractionHand::MainHand,
                hit: ProtocolBlockHitResult {
                    pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                    direction: ProtocolDirection::North,
                    cursor_x: 0.0,
                    cursor_y: 0.62,
                    cursor_z: 0.0,
                    inside: false,
                    world_border_hit: false,
                },
                sequence: 1,
            })
        );
    }

    #[test]
    fn right_mouse_press_on_ender_dragon_part_queues_interact_with_part_id() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_ender_dragon();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(!world.local_player().interaction.using_item);
        assert_eq!(counters.interact_entity_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::InteractEntity(InteractEntity {
                entity_id: 101,
                hand: InteractionHand::MainHand,
                location: ProtocolVec3d {
                    x: 0.0,
                    y: 0.6200000047683716,
                    z: -0.5,
                },
                using_secondary_action: false,
            })
        );
    }

    #[test]
    fn middle_mouse_press_on_block_queues_pick_block() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.sprint = true;
        let mut world = world_with_crosshair_block();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Middle,
            ElementState::Pressed,
        );

        assert_eq!(counters.pick_item_from_block_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PickItemFromBlock(PickItemFromBlock {
                pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                include_data: true,
            })
        );
    }

    #[test]
    fn middle_mouse_press_on_entity_queues_pick_entity() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.sprint = true;
        let mut world = world_with_crosshair_entity(123);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
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
        assert_eq!(counters.held_slot_commands_queued, 1);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(3));
    }

    fn world_with_crosshair_entity(entity_id: i32) -> WorldStore {
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState::default());
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

    fn world_with_crosshair_block() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.set_local_player_pose(LocalPlayerPoseState::default());
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    VANILLA_AIR_BLOCK_STATE_ID,
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
            block_state_id: VANILLA_GRASS_BLOCK_STATE_ID,
        }));
        world
    }

    fn single_value_container(
        domain: PaletteDomain,
        entry_count: usize,
        global_id: i32,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
        ProtocolItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }

    fn world_with_ender_dragon() -> WorldStore {
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState::default());
        world.apply_add_entity(AddEntity {
            id: 100,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id: VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
            position: ProtocolVec3d {
                x: 0.0,
                y: 2.0,
                z: 9.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        world.advance_entity_client_animations(1);
        world
    }
}
