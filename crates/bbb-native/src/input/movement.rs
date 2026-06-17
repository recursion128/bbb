use std::time::{Duration, Instant};

use bbb_control::NetCounters;
use bbb_net::{NetCommand, PlayerMoveCommand};
use bbb_world::{LocalPlayerInputState, LocalPlayerPoseState, WorldStore};
use tokio::sync::mpsc;

use super::ClientInputState;

const MOVE_COMMAND_INTERVAL: Duration = Duration::from_millis(50);
const MOVE_COMMAND_POSITION_REMINDER_INTERVAL: Duration = Duration::from_secs(1);
const MOVE_COMMAND_POSITION_THRESHOLD: f64 = 2.0E-4;
const MOVE_COMMAND_POSITION_THRESHOLD_SQUARED: f64 =
    MOVE_COMMAND_POSITION_THRESHOLD * MOVE_COMMAND_POSITION_THRESHOLD;

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
    if !command_due {
        return;
    }

    let last_pose = input.last_move_command_pose;
    let force_position = last_pose.is_some()
        && elapsed_since_last
            .is_some_and(|elapsed| elapsed >= MOVE_COMMAND_POSITION_REMINDER_INTERVAL);
    let (send_position, send_rotation) = match last_pose {
        Some(last_pose) => {
            let send_position = position_delta_squared(last_pose.position, pose.position)
                > MOVE_COMMAND_POSITION_THRESHOLD_SQUARED
                || force_position;
            let send_rotation = last_pose.y_rot != pose.y_rot || last_pose.x_rot != pose.x_rot;
            let send_status = last_pose.on_ground != pose.on_ground
                || last_pose.horizontal_collision != pose.horizontal_collision;
            if !send_position && !send_rotation && !send_status {
                return;
            }
            (send_position, send_rotation)
        }
        None => (true, true),
    };

    let mut command_state = last_pose.map_or_else(
        || pose.position_state(),
        LocalPlayerPoseState::position_state,
    );
    command_state.delta_movement = pose.delta_movement;
    if send_position {
        command_state.position = pose.position;
    }
    if send_rotation {
        command_state.y_rot = pose.y_rot;
        command_state.x_rot = pose.x_rot;
    }

    let command = NetCommand::MovePlayer(PlayerMoveCommand {
        state: command_state,
        on_ground: pose.on_ground,
        horizontal_collision: pose.horizontal_collision,
        force_position,
    });
    if tx.try_send(command).is_ok() {
        let mut remembered_pose = last_pose.unwrap_or(pose);
        remembered_pose.delta_movement = pose.delta_movement;
        if send_position {
            remembered_pose.position = pose.position;
        }
        if send_rotation {
            remembered_pose.y_rot = pose.y_rot;
            remembered_pose.x_rot = pose.x_rot;
        }
        remembered_pose.on_ground = pose.on_ground;
        remembered_pose.horizontal_collision = pose.horizontal_collision;
        remembered_pose.last_teleport_id = pose.last_teleport_id;

        input.last_move_command_at = Some(now);
        input.last_move_command_pose = Some(remembered_pose);
        counters.player_move_commands_queued += 1;
    }
}

fn position_delta_squared(
    from: bbb_protocol::packets::Vec3d,
    to: bbb_protocol::packets::Vec3d,
) -> f64 {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let dz = to.z - from.z;
    dx * dx + dy * dy + dz * dz
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
        assert!(!first.on_ground);
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
    fn move_command_forces_position_after_vanilla_reminder_interval() {
        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        let pose = LocalPlayerPoseState {
            position: vec3(0.0, 64.0, 0.0),
            ..LocalPlayerPoseState::default()
        };
        let mut counters = NetCounters::default();

        maybe_queue_player_move_command(&mut input, &mut counters, &commands, pose, start);
        let first = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected move command, got {other:?}"),
        };
        assert!(!first.force_position);

        maybe_queue_player_move_command(
            &mut input,
            &mut counters,
            &commands,
            pose,
            start + MOVE_COMMAND_POSITION_REMINDER_INTERVAL - Duration::from_millis(1),
        );
        assert!(rx.try_recv().is_err());

        maybe_queue_player_move_command(
            &mut input,
            &mut counters,
            &commands,
            pose,
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

    #[test]
    fn move_command_ignores_below_threshold_position_and_delta_changes() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let pose = LocalPlayerPoseState {
            position: vec3(0.0, 64.0, 0.0),
            ..LocalPlayerPoseState::default()
        };

        maybe_queue_player_move_command(&mut input, &mut counters, &commands, pose, start);
        let first = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected move command, got {other:?}"),
        };

        let below_threshold_pose = LocalPlayerPoseState {
            position: vec3(MOVE_COMMAND_POSITION_THRESHOLD / 2.0, 64.0, 0.0),
            delta_movement: vec3(0.25, 0.0, 0.0),
            ..pose
        };
        maybe_queue_player_move_command(
            &mut input,
            &mut counters,
            &commands,
            below_threshold_pose,
            start + MOVE_COMMAND_INTERVAL,
        );

        assert!(rx.try_recv().is_err());
        assert_eq!(
            input.last_move_command_pose.unwrap().position,
            first.state.position
        );
        assert_eq!(counters.player_move_commands_queued, 1);
    }

    #[test]
    fn move_command_queues_status_only_at_command_interval() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let pose = LocalPlayerPoseState {
            position: vec3(0.0, 64.0, 0.0),
            ..LocalPlayerPoseState::default()
        };

        maybe_queue_player_move_command(&mut input, &mut counters, &commands, pose, start);
        let first = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected move command, got {other:?}"),
        };

        let status_pose = LocalPlayerPoseState {
            on_ground: true,
            horizontal_collision: true,
            ..pose
        };
        maybe_queue_player_move_command(
            &mut input,
            &mut counters,
            &commands,
            status_pose,
            start + MOVE_COMMAND_INTERVAL / 2,
        );
        assert!(rx.try_recv().is_err());

        maybe_queue_player_move_command(
            &mut input,
            &mut counters,
            &commands,
            status_pose,
            start + MOVE_COMMAND_INTERVAL,
        );
        let status_only = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected move command, got {other:?}"),
        };

        assert_eq!(status_only.state, first.state);
        assert!(status_only.on_ground);
        assert!(status_only.horizontal_collision);
        assert!(!status_only.force_position);
        assert_eq!(counters.player_move_commands_queued, 2);
    }

    #[test]
    fn move_command_queues_rotation_only_at_command_interval() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let pose = LocalPlayerPoseState {
            position: vec3(0.0, 64.0, 0.0),
            y_rot: 10.0,
            x_rot: 5.0,
            ..LocalPlayerPoseState::default()
        };

        maybe_queue_player_move_command(&mut input, &mut counters, &commands, pose, start);
        let first = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected move command, got {other:?}"),
        };

        let rotation_pose = LocalPlayerPoseState {
            y_rot: 15.0,
            x_rot: -2.5,
            ..pose
        };
        maybe_queue_player_move_command(
            &mut input,
            &mut counters,
            &commands,
            rotation_pose,
            start + MOVE_COMMAND_INTERVAL,
        );
        let rotation_only = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected move command, got {other:?}"),
        };

        assert_eq!(rotation_only.state.position, first.state.position);
        assert_eq!(rotation_only.state.y_rot, 15.0);
        assert_eq!(rotation_only.state.x_rot, -2.5);
        assert!(!rotation_only.force_position);
        assert_eq!(counters.player_move_commands_queued, 2);
    }

    #[test]
    fn move_command_uses_world_collision_flags() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let now = Instant::now();
        let pose = LocalPlayerPoseState {
            position: vec3(0.0, 64.0, 0.0),
            on_ground: false,
            horizontal_collision: true,
            ..LocalPlayerPoseState::default()
        };

        maybe_queue_player_move_command(&mut input, &mut counters, &commands, pose, now);

        let command = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected move command, got {other:?}"),
        };
        assert!(!command.on_ground);
        assert!(command.horizontal_collision);
        assert_eq!(counters.player_move_commands_queued, 1);
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
