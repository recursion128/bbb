use std::time::{Duration, Instant};

use bbb_control::NetCounters;
use bbb_net::{NetCommand, PlayerMoveCommand};
use bbb_protocol::packets::Vec3d as ProtocolVec3d;
use bbb_world::{LocalPlayerPoseState, WorldStore};
use tokio::sync::mpsc;

use super::ClientInputState;

const INPUT_MOUSE_SENSITIVITY_DEGREES: f32 = 0.12;
const INPUT_WALK_SPEED_BLOCKS_PER_SECOND: f64 = 4.317;
const INPUT_SPRINT_SPEED_BLOCKS_PER_SECOND: f64 = 5.612;
const MOVE_COMMAND_INTERVAL: Duration = Duration::from_millis(50);

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

    let Some(current_pose) = world.local_player_pose() else {
        input.mouse_delta_x = 0.0;
        input.mouse_delta_y = 0.0;
        return;
    };

    let pose = integrate_player_input_pose(current_pose, input, dt_seconds);
    input.mouse_delta_x = 0.0;
    input.mouse_delta_y = 0.0;
    world.set_local_player_pose(pose);
    maybe_queue_player_move_command(input, counters, net_commands, pose, now);
}

fn integrate_player_input_pose(
    mut pose: LocalPlayerPoseState,
    input: &ClientInputState,
    dt_seconds: f64,
) -> LocalPlayerPoseState {
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
    pose.delta_movement = ProtocolVec3d {
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
    pose: LocalPlayerPoseState,
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
        state: pose.position_state(),
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

    #[test]
    fn player_input_moves_forward_with_minecraft_yaw() {
        let mut input = ClientInputState::new(true);
        input.forward = true;
        let pose = integrate_player_input_pose(
            LocalPlayerPoseState {
                position: vec3(0.0, 64.0, 0.0),
                y_rot: 0.0,
                ..LocalPlayerPoseState::default()
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
            LocalPlayerPoseState {
                position: vec3(0.0, 64.0, 0.0),
                ..LocalPlayerPoseState::default()
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
