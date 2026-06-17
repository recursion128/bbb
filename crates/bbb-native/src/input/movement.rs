use std::time::{Duration, Instant};

use bbb_control::NetCounters;
use bbb_net::{NetCommand, PlayerMoveCommand};
use bbb_protocol::packets::PlayerCommandAction;
use bbb_world::{LocalPlayerInputState, LocalPlayerPoseState, WorldStore};
use tokio::sync::mpsc;

use super::{
    commands::{
        queue_paddle_boat_command, queue_player_command_action, queue_vehicle_move_command,
    },
    ClientInputState,
};

const MOVE_COMMAND_INTERVAL: Duration = Duration::from_millis(50);
const MOVE_COMMAND_POSITION_REMINDER_INTERVAL: Duration = Duration::from_secs(1);
const MOVE_COMMAND_POSITION_THRESHOLD: f64 = 2.0E-4;
const MOVE_COMMAND_POSITION_THRESHOLD_SQUARED: f64 =
    MOVE_COMMAND_POSITION_THRESHOLD * MOVE_COMMAND_POSITION_THRESHOLD;
const RIDING_JUMP_TICK_SECONDS: f64 = 0.05;

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

    if world.local_player_root_vehicle_id().is_some() {
        let input_state = input.local_player_input();
        let before_pose = world.local_player_pose();
        if let Some(pose) = world.advance_local_player_look_input(input_state) {
            if Some(pose) != before_pose {
                maybe_queue_player_move_command(input, counters, net_commands, pose, now);
            }
        }
        maybe_queue_riding_jump_command(input, world, counters, net_commands, dt_seconds);
        let boat_report = world.advance_local_boat_vehicle_input(input_state, dt_seconds);
        maybe_queue_boat_commands(input, world, counters, net_commands, now, boat_report);
        input.mouse_delta_x = 0.0;
        input.mouse_delta_y = 0.0;
        return;
    }
    input.last_paddle_boat_command_at = None;
    input.riding_jump_charge_seconds = None;

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

fn maybe_queue_riding_jump_command(
    input: &mut ClientInputState,
    world: &WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    dt_seconds: f64,
) {
    if world.local_player_rideable_jumping_vehicle_id().is_none() {
        input.riding_jump_charge_seconds = None;
        return;
    }

    if let Some(charge_seconds) = &mut input.riding_jump_charge_seconds {
        *charge_seconds += dt_seconds.max(0.0);
    }

    if input.focused && input.jump {
        input.riding_jump_charge_seconds.get_or_insert(0.0);
        return;
    }

    let Some(charge_seconds) = input.riding_jump_charge_seconds.take() else {
        return;
    };
    let jump_data = riding_jump_command_data(charge_seconds);
    if jump_data > 0 {
        queue_player_command_action(
            counters,
            world,
            net_commands,
            PlayerCommandAction::StartRidingJump,
            jump_data,
        );
    }
}

fn maybe_queue_boat_commands(
    input: &mut ClientInputState,
    world: &WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    now: Instant,
    boat_report: Option<bbb_world::VehicleMoveReport>,
) {
    if world.local_player_root_boat_vehicle_id().is_none() {
        input.last_paddle_boat_command_at = None;
        return;
    }
    let elapsed_since_last = input
        .last_paddle_boat_command_at
        .and_then(|last| now.checked_duration_since(last));
    let command_due = elapsed_since_last.map_or(true, |elapsed| elapsed >= MOVE_COMMAND_INTERVAL);
    if !command_due {
        return;
    }

    let (left, right) = paddle_boat_state_from_input(input);
    queue_paddle_boat_command(counters, net_commands, left, right);
    if let Some(report) = boat_report {
        queue_vehicle_move_command(counters, net_commands, report);
    }
    input.last_paddle_boat_command_at = Some(now);
}

fn paddle_boat_state_from_input(input: &ClientInputState) -> (bool, bool) {
    let left = (input.right && !input.left) || input.forward;
    let right = (input.left && !input.right) || input.forward;
    (left, right)
}

fn riding_jump_command_data(charge_seconds: f64) -> i32 {
    let ticks = (charge_seconds.max(0.0) / RIDING_JUMP_TICK_SECONDS).floor() as i32;
    if ticks <= 0 {
        return 0;
    }
    let scale = if ticks < 10 {
        ticks as f32 * 0.1
    } else {
        0.8 + 2.0 / (ticks - 9) as f32 * 0.1
    };
    (scale * 100.0).floor() as i32
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
    use bbb_protocol::packets::{
        AddEntity, CommonPlayerSpawnInfo, PaddleBoat, PlayLogin, SetPassengers,
        Vec3d as ProtocolVec3d,
    };
    use uuid::Uuid;

    const VANILLA_26_1_MINECART_ENTITY_TYPE_ID: i32 = 85;
    const VANILLA_26_1_HORSE_ENTITY_TYPE_ID: i32 = 66;
    const VANILLA_26_1_OAK_BOAT_ENTITY_TYPE_ID: i32 = 89;

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

    #[test]
    fn mounted_boat_input_queues_paddle_and_vehicle_move_instead_of_player_walk() {
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        input.left = true;
        let mut world = world_with_local_vehicle(VANILLA_26_1_OAK_BOAT_ENTITY_TYPE_ID);
        let initial_pose = world.local_player_pose().unwrap();
        let mut counters = NetCounters::default();

        advance_player_input(&mut input, &mut world, &mut counters, &commands, start);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PaddleBoat(PaddleBoat {
                left: false,
                right: true,
            })
        );
        let first_vehicle = match rx.try_recv().unwrap() {
            NetCommand::MoveVehicle(command) => command,
            other => panic!("expected vehicle move command, got {other:?}"),
        };
        assert_eq!(first_vehicle.position.x, 1.0);
        assert_eq!(first_vehicle.position.y, 64.0);
        assert_eq!(first_vehicle.position.z, -2.0);
        assert_eq!(world.local_player_pose(), Some(initial_pose));
        assert_eq!(counters.paddle_boat_commands_queued, 1);
        assert_eq!(counters.move_vehicle_commands_queued, 1);
        assert_eq!(counters.player_move_commands_queued, 0);

        input.left = false;
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
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PaddleBoat(PaddleBoat {
                left: true,
                right: true,
            })
        );
        let second_vehicle = match rx.try_recv().unwrap() {
            NetCommand::MoveVehicle(command) => command,
            other => panic!("expected vehicle move command, got {other:?}"),
        };
        assert!(second_vehicle.position.z > first_vehicle.position.z);
        assert_eq!(counters.paddle_boat_commands_queued, 2);
        assert_eq!(counters.move_vehicle_commands_queued, 2);
        assert_eq!(counters.player_move_commands_queued, 0);
    }

    #[test]
    fn mounted_mouse_motion_updates_look_and_queues_riding_rotation() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        input.mouse_delta_x = 100.0;
        input.mouse_delta_y = -50.0;
        let mut world = world_with_local_vehicle(VANILLA_26_1_MINECART_ENTITY_TYPE_ID);
        let initial_pose = world.local_player_pose().unwrap();
        let mut counters = NetCounters::default();

        advance_player_input(&mut input, &mut world, &mut counters, &commands, start);

        let command = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected player rotation command, got {other:?}"),
        };
        assert_eq!(command.state.position, initial_pose.position);
        assert_eq!(command.state.y_rot, 12.0);
        assert_eq!(command.state.x_rot, -6.0);
        assert_eq!(
            world.local_player_pose().unwrap().position,
            initial_pose.position
        );
        assert_eq!(counters.player_move_commands_queued, 1);
        assert_eq!(counters.paddle_boat_commands_queued, 0);
        assert_eq!(counters.move_vehicle_commands_queued, 0);
    }

    #[test]
    fn mounted_non_boat_input_does_not_queue_paddle_or_vehicle_move() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        input.forward = true;
        let mut world = world_with_local_vehicle(VANILLA_26_1_MINECART_ENTITY_TYPE_ID);
        let initial_pose = world.local_player_pose().unwrap();
        let mut counters = NetCounters::default();

        advance_player_input(&mut input, &mut world, &mut counters, &commands, start);

        assert!(rx.try_recv().is_err());
        assert_eq!(world.local_player_pose(), Some(initial_pose));
        assert_eq!(counters.paddle_boat_commands_queued, 0);
        assert_eq!(counters.move_vehicle_commands_queued, 0);
        assert_eq!(counters.player_move_commands_queued, 0);
    }

    #[test]
    fn mounted_jumpable_vehicle_release_queues_riding_jump_command() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        input.jump = true;
        let mut world = world_with_local_vehicle(VANILLA_26_1_HORSE_ENTITY_TYPE_ID);
        let mut counters = NetCounters::default();

        advance_player_input(&mut input, &mut world, &mut counters, &commands, start);
        assert!(rx.try_recv().is_err());

        input.jump = false;
        advance_player_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            start + Duration::from_millis(100),
        );

        let command = match rx.try_recv().unwrap() {
            NetCommand::PlayerCommand(command) => command,
            other => panic!("expected riding jump player command, got {other:?}"),
        };
        assert_eq!(command.entity_id, 99);
        assert_eq!(command.action, PlayerCommandAction::StartRidingJump);
        assert_eq!(command.data, 20);
        assert_eq!(counters.player_command_commands_queued, 1);
    }

    #[test]
    fn mounted_non_jumpable_vehicle_release_does_not_queue_riding_jump() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        input.jump = true;
        let mut world = world_with_local_vehicle(VANILLA_26_1_MINECART_ENTITY_TYPE_ID);
        let mut counters = NetCounters::default();

        advance_player_input(&mut input, &mut world, &mut counters, &commands, start);
        input.jump = false;
        advance_player_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            start + Duration::from_millis(100),
        );

        assert!(rx.try_recv().is_err());
        assert_eq!(counters.player_command_commands_queued, 0);
    }

    #[test]
    fn riding_jump_command_data_matches_vanilla_charge_shape() {
        assert_eq!(
            riding_jump_command_data(Duration::from_millis(49).as_secs_f64()),
            0
        );
        assert_eq!(
            riding_jump_command_data(Duration::from_millis(50).as_secs_f64()),
            10
        );
        assert_eq!(
            riding_jump_command_data(Duration::from_millis(450).as_secs_f64()),
            90
        );
        assert_eq!(
            riding_jump_command_data(Duration::from_millis(500).as_secs_f64()),
            100
        );
        assert_eq!(
            riding_jump_command_data(Duration::from_millis(550).as_secs_f64()),
            90
        );
    }

    fn vec3(x: f64, y: f64, z: f64) -> ProtocolVec3d {
        ProtocolVec3d { x, y, z }
    }

    fn world_with_local_vehicle(entity_type_id: i32) -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_login(&protocol_play_login(99));
        world.apply_add_entity(protocol_add_entity_with_type(10, entity_type_id));
        assert!(world.apply_set_passengers(SetPassengers {
            vehicle_id: 10,
            passenger_ids: vec![99],
        }));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.0, 64.0, 0.0),
            ..LocalPlayerPoseState::default()
        });
        world
    }

    fn protocol_play_login(player_id: i32) -> PlayLogin {
        PlayLogin {
            player_id,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: false,
        }
    }

    fn protocol_add_entity_with_type(id: i32, entity_type_id: i32) -> AddEntity {
        AddEntity {
            id,
            uuid: Uuid::from_u128(id as u128),
            entity_type_id,
            position: vec3(1.0, 64.0, -2.0),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        }
    }

    fn assert_f64_near(actual: f64, expected: f64, epsilon: f64) {
        assert!(
            (actual - expected).abs() <= epsilon,
            "expected {actual} to be within {epsilon} of {expected}"
        );
    }
}
