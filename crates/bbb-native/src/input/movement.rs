use std::time::{Duration, Instant};

use bbb_control::NetCounters;
use bbb_net::{NetCommand, PlayerMoveCommand};
use bbb_world::{LocalPlayerInputState, LocalPlayerPoseState, WorldStore};
use tokio::sync::mpsc;

use super::ClientInputState;

const MOVE_COMMAND_INTERVAL: Duration = Duration::from_millis(50);
const MOVE_COMMAND_POSITION_REMINDER_INTERVAL: Duration = Duration::from_secs(1);

impl ClientInputState {
    fn local_player_input(&self) -> LocalPlayerInputState {
        LocalPlayerInputState {
            focused: self.focused,
            forward: self.forward,
            backward: self.backward,
            left: self.left,
            right: self.right,
            jump: self.jump,
            sneak: self.sneak,
            sprint: self.sprint,
            mouse_delta_x: self.mouse_delta_x,
            mouse_delta_y: self.mouse_delta_y,
        }
    }
}

pub(crate) fn advance_player_input(
    input: &mut ClientInputState,
    world: &mut WorldStore,
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

    let Some(pose) = world.advance_local_player_input(input.local_player_input(), dt_seconds)
    else {
        input.mouse_delta_x = 0.0;
        input.mouse_delta_y = 0.0;
        return;
    };

    input.mouse_delta_x = 0.0;
    input.mouse_delta_y = 0.0;
    maybe_queue_player_move_command(input, counters, net_commands, pose, now);
}

fn maybe_queue_player_move_command(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    pose: LocalPlayerPoseState,
    now: Instant,
) {
    let Some(tx) = net_commands else {
        return;
    };
    let elapsed_since_last = input
        .last_move_command_at
        .and_then(|last| now.checked_duration_since(last));
    let command_due = elapsed_since_last.map_or(true, |elapsed| elapsed >= MOVE_COMMAND_INTERVAL);
    let pose_changed = input.last_move_command_pose != Some(pose);
    let force_position = !pose_changed
        && elapsed_since_last
            .is_some_and(|elapsed| elapsed >= MOVE_COMMAND_POSITION_REMINDER_INTERVAL);
    if !command_due || (!pose_changed && !force_position) {
        return;
    }

    let command = NetCommand::MovePlayer(PlayerMoveCommand {
        state: pose.position_state(),
        on_ground: pose.delta_movement.y.abs() <= f64::EPSILON,
        horizontal_collision: false,
        force_position,
    });
    if tx.try_send(command).is_ok() {
        input.last_move_command_at = Some(now);
        input.last_move_command_pose = Some(pose);
        counters.player_move_commands_queued += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::Vec3d as ProtocolVec3d;

    #[test]
    fn local_input_projection_includes_focus_keys_and_mouse_delta() {
        let mut input = ClientInputState::new(true);
        input.forward = true;
        input.left = true;
        input.jump = true;
        input.sprint = true;
        input.mouse_delta_x = 100.0;
        input.mouse_delta_y = 1000.0;

        assert_eq!(
            input.local_player_input(),
            LocalPlayerInputState {
                focused: true,
                forward: true,
                backward: false,
                left: true,
                right: false,
                jump: true,
                sneak: false,
                sprint: true,
                mouse_delta_x: 100.0,
                mouse_delta_y: 1000.0,
            }
        );
    }

    #[test]
    fn advance_player_input_queues_move_commands_at_tick_interval() {
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.0, 64.0, 0.0),
            ..LocalPlayerPoseState::default()
        });
        let mut counters = NetCounters::default();

        advance_player_input(&mut input, &mut world, &mut counters, &commands, start);
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
            &mut world,
            &mut counters,
            &commands,
            start + Duration::from_millis(25),
        );
        assert!(rx.try_recv().is_err());

        advance_player_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            start + Duration::from_millis(50),
        );
        let second = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected move command, got {other:?}"),
        };
        assert!(second.state.position.z > 0.0);
        let world_pose = world.local_player_pose().unwrap();
        assert_f64_near(world_pose.position.z, second.state.position.z, 0.000001);
        assert_eq!(counters.player_move_commands_queued, 2);
    }

    #[test]
    fn advance_player_input_forces_position_after_vanilla_reminder_interval() {
        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.0, 64.0, 0.0),
            ..LocalPlayerPoseState::default()
        });
        let mut counters = NetCounters::default();

        advance_player_input(&mut input, &mut world, &mut counters, &commands, start);
        let first = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected move command, got {other:?}"),
        };
        assert!(!first.force_position);

        advance_player_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            start + MOVE_COMMAND_POSITION_REMINDER_INTERVAL - Duration::from_millis(1),
        );
        assert!(rx.try_recv().is_err());

        advance_player_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            start + MOVE_COMMAND_POSITION_REMINDER_INTERVAL,
        );
        let reminder = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected move command, got {other:?}"),
        };
        assert!(reminder.force_position);
        assert_eq!(reminder.state, first.state);
        assert_eq!(counters.player_move_commands_queued, 2);
    }

    fn vec3(x: f64, y: f64, z: f64) -> ProtocolVec3d {
        ProtocolVec3d { x, y, z }
    }

    fn assert_f64_near(actual: f64, expected: f64, epsilon: f64) {
        assert!(
            (actual - expected).abs() <= epsilon,
            "expected {actual} to be within {epsilon} of {expected}"
        );
    }
}
