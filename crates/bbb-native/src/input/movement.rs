use std::time::{Duration, Instant};

use bbb_control::NetCounters;
use bbb_net::{NetCommand, PlayerMoveCommand};
use bbb_protocol::packets::PlayerCommandAction;
use bbb_world::{LocalPlayerInputState, LocalPlayerPoseState, WorldStore};
use tokio::sync::mpsc;

use super::{
    commands::{
        queue_paddle_boat_command, queue_player_abilities_command, queue_player_command_action,
        queue_player_input_command, queue_sprint_command, queue_vehicle_move_command,
    },
    effective_sprinting_from_state, player_input_from_state, ClientInputState,
};

const MOVE_COMMAND_INTERVAL: Duration = Duration::from_millis(50);
const MOVE_COMMAND_POSITION_THRESHOLD: f64 = 2.0E-4;
const MOVE_COMMAND_POSITION_THRESHOLD_SQUARED: f64 =
    MOVE_COMMAND_POSITION_THRESHOLD * MOVE_COMMAND_POSITION_THRESHOLD;
const MOVE_COMMAND_POSITION_REMINDER_TICKS: u32 = 20;
const LOCAL_PLAYER_MOVEMENT_TICK_SECONDS: f64 = 0.05;
const RIDING_JUMP_TICK_SECONDS: f64 = 0.05;

impl ClientInputState {
    pub(crate) fn local_player_input(&self) -> LocalPlayerInputState {
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

    pub(crate) fn riding_jump_scale(&self) -> Option<f32> {
        self.riding_jump_charge_seconds.map(riding_jump_scale)
    }
}

#[cfg(test)]
pub(crate) fn advance_player_input(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    now: Instant,
) {
    // Legacy fixed-clock fixture adapter. Production receives the authoritative
    // client tick count from runtime through `advance_player_input_for_client_ticks`.
    let dt_seconds = input
        .last_step
        .and_then(|last| now.checked_duration_since(last))
        .unwrap_or_default()
        .as_secs_f64()
        .min(0.25);
    input.last_step = Some(now);
    input.local_player_movement_tick_accumulator_seconds =
        (input.local_player_movement_tick_accumulator_seconds + dt_seconds.max(0.0))
            .min(0.25 + LOCAL_PLAYER_MOVEMENT_TICK_SECONDS);
    let client_ticks = (input.local_player_movement_tick_accumulator_seconds
        / LOCAL_PLAYER_MOVEMENT_TICK_SECONDS)
        .floor() as u32;
    input.local_player_movement_tick_accumulator_seconds -=
        f64::from(client_ticks) * LOCAL_PLAYER_MOVEMENT_TICK_SECONDS;
    advance_player_input_for_client_ticks(input, world, counters, net_commands, now, client_ticks);
}

pub(crate) fn advance_player_input_for_client_ticks(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    now: Instant,
    client_ticks: u32,
) {
    input.expire_spectator_menu_at(now);
    input.advance_attack_miss_time(world, client_ticks);
    // Vanilla calls LocalPlayer.tick from Minecraft.tick, so movement constants
    // remain per-tick (0.05 s) even when synchronized tick rate slows wall time.
    // Preserve bbb's existing five-tick catch-up bound for a stalled frame.
    let client_ticks = client_ticks.min(5);
    let tutorial_input = input.local_player_input();
    world.tutorial_on_mouse(tutorial_input.mouse_delta_x, tutorial_input.mouse_delta_y);
    for _ in 0..client_ticks {
        world.tutorial_on_input(
            tutorial_input.focused && tutorial_input.forward,
            tutorial_input.focused && tutorial_input.backward,
            tutorial_input.focused && tutorial_input.left,
            tutorial_input.focused && tutorial_input.right,
            tutorial_input.focused && tutorial_input.jump,
        );
    }
    let dt_seconds = f64::from(client_ticks) * LOCAL_PLAYER_MOVEMENT_TICK_SECONDS;
    input.advance_debug_crash_hold(world, now);
    input.advance_sprint_trigger(dt_seconds);
    input.advance_creative_flight_jump_trigger(dt_seconds);
    resolve_pending_attack_sprint_stop(input, world, counters, net_commands, client_ticks);
    maybe_enable_spectator_flying(counters, world, net_commands);

    if world.local_player_root_vehicle_id().is_some() {
        for _ in 0..client_ticks {
            world.advance_local_player_underwater_audio_tick();
        }
        let input_state = input.local_player_input();
        if let Some(pose) = world.advance_local_player_look_input(input_state) {
            let rotation_command_count =
                client_ticks.max(u32::from(input.last_move_command_at.is_none()));
            queue_passenger_rotation_commands(
                input,
                counters,
                net_commands,
                pose,
                now,
                rotation_command_count,
            );
        }
        maybe_queue_riding_jump_command(input, world, counters, net_commands, dt_seconds);
        let boat_vehicle = world
            .local_player_first_passenger_root_boat_vehicle_id()
            .is_some();
        let vehicle_report = world
            .advance_local_boat_vehicle_input(input_state, client_ticks)
            .or_else(|| world.advance_local_abstract_horse_vehicle_input(input_state, client_ticks))
            .or_else(|| world.advance_local_nautilus_vehicle_input(input_state, client_ticks))
            .or_else(|| world.advance_local_item_based_steering_vehicle_input(client_ticks))
            .or_else(|| world.advance_local_happy_ghast_vehicle_input(input_state, client_ticks));
        if client_ticks > 0 || boat_vehicle && input.last_vehicle_command_at.is_none() {
            maybe_queue_vehicle_commands(input, world, counters, net_commands, now, vehicle_report);
        }
        input.mouse_delta_x = 0.0;
        input.mouse_delta_y = 0.0;
        return;
    }
    input.last_vehicle_command_at = None;
    input.riding_jump_charge_seconds = None;

    let Some(pose) = advance_unmounted_local_player_input(input, world, client_ticks) else {
        input.mouse_delta_x = 0.0;
        input.mouse_delta_y = 0.0;
        return;
    };
    maybe_disable_grounded_flying(counters, world, net_commands);

    input.mouse_delta_x = 0.0;
    input.mouse_delta_y = 0.0;
    if world.local_player().camera.follows_player
        && (client_ticks > 0 || input.last_move_command_at.is_none())
    {
        maybe_queue_player_move_command(input, counters, net_commands, pose, now);
    }
}

fn advance_unmounted_local_player_input(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    client_ticks: u32,
) -> Option<LocalPlayerPoseState> {
    let input_state = input.local_player_input();
    let mut pose = world.advance_local_player_look_input(input_state)?;
    let mut movement_input = input_state;
    movement_input.mouse_delta_x = 0.0;
    movement_input.mouse_delta_y = 0.0;

    for _ in 0..client_ticks {
        world.advance_local_player_underwater_audio_tick();
        pose =
            world.advance_local_player_input(movement_input, LOCAL_PLAYER_MOVEMENT_TICK_SECONDS)?;
    }

    Some(pose)
}

fn resolve_pending_attack_sprint_stop(
    input: &mut ClientInputState,
    world: &WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    client_ticks: u32,
) {
    if client_ticks == 0 || !input.pending_attack_sprint_stop {
        return;
    }
    input.pending_attack_sprint_stop = false;

    // Player.attack clears the sprinting entity flag. On the next LocalPlayer
    // tick, holding the dedicated sprint key can set it again before
    // sendIsSprintingIfNeeded observes any transition, so no stop command is
    // emitted in that case. Double-tap sprint has no held key and remains
    // stopped.
    if input.control_down() && effective_sprinting_from_state(world, input) {
        return;
    }

    input.sprint = false;
    queue_player_input_command(counters, net_commands, player_input_from_state(input));
    queue_sprint_command(counters, world, net_commands, false);
}

fn maybe_enable_spectator_flying(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) -> bool {
    if !world.local_player_is_spectator() {
        return false;
    }
    let Some(abilities) = world.local_player().abilities else {
        return false;
    };
    if !abilities.can_fly || abilities.flying {
        return false;
    }
    queue_player_abilities_command(counters, world, net_commands, true)
}

fn maybe_disable_grounded_flying(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) -> bool {
    if world.local_player_is_spectator()
        || !world.local_player_pose().is_some_and(|pose| pose.on_ground)
        || !world
            .local_player()
            .abilities
            .is_some_and(|abilities| abilities.flying)
    {
        return false;
    }
    queue_player_abilities_command(counters, world, net_commands, false)
}

fn maybe_queue_riding_jump_command(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    dt_seconds: f64,
) {
    if world.local_player_rideable_jumping_vehicle_id().is_none()
        || world
            .local_player_rideable_jumping_vehicle_cooldown_ticks()
            .is_none_or(|cooldown| cooldown > 0)
    {
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
    queue_released_riding_jump_command(Some(charge_seconds), world, counters, net_commands);
}

pub(super) fn queue_released_riding_jump_command(
    charge_seconds: Option<f64>,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    if world.local_player_rideable_jumping_vehicle_id().is_none()
        || world
            .local_player_rideable_jumping_vehicle_cooldown_ticks()
            .is_none_or(|cooldown| cooldown > 0)
    {
        return;
    }
    let Some(charge_seconds) = charge_seconds else {
        return;
    };
    let jump_data = riding_jump_command_data(charge_seconds);
    let _ = world.apply_local_player_riding_jump(jump_data);
    queue_player_command_action(
        counters,
        world,
        net_commands,
        PlayerCommandAction::StartRidingJump,
        jump_data,
    );
}

fn maybe_queue_vehicle_commands(
    input: &mut ClientInputState,
    world: &WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    now: Instant,
    vehicle_report: Option<bbb_world::VehicleMoveReport>,
) {
    let boat_vehicle = world
        .local_player_first_passenger_root_boat_vehicle_id()
        .is_some();
    if !boat_vehicle && vehicle_report.is_none() {
        input.last_vehicle_command_at = None;
        return;
    }
    let elapsed_since_last = input
        .last_vehicle_command_at
        .and_then(|last| now.checked_duration_since(last));
    let command_due = elapsed_since_last.map_or(true, |elapsed| elapsed >= MOVE_COMMAND_INTERVAL);
    if !command_due {
        return;
    }

    if boat_vehicle {
        let (left, right) = paddle_boat_state_from_input(input);
        queue_paddle_boat_command(counters, net_commands, left, right);
    }
    if let Some(report) = vehicle_report {
        queue_vehicle_move_command(counters, net_commands, report);
    }
    input.last_vehicle_command_at = Some(now);
}

fn queue_passenger_rotation_commands(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    pose: LocalPlayerPoseState,
    now: Instant,
    command_count: u32,
) {
    let Some(tx) = net_commands else {
        return;
    };

    let mut command_state = pose.position_state();
    command_state.delta_movement = pose.delta_movement;
    let command = PlayerMoveCommand {
        state: command_state,
        on_ground: pose.on_ground,
        horizontal_collision: pose.horizontal_collision,
        force_position: false,
        force_pos_rot: false,
        force_rotation_only: true,
    };
    // Vanilla's passenger branch sends one Rot packet per LocalPlayer tick and
    // bypasses sendPosition entirely. In particular, these packets must not
    // advance the unmounted xLast/yLast/zLast or rotation/status baselines.
    for _ in 0..command_count {
        if tx.try_send(NetCommand::MovePlayer(command)).is_err() {
            break;
        }
        input.last_move_command_at = Some(now);
        counters.player_move_commands_queued += 1;
    }
}

fn paddle_boat_state_from_input(input: &ClientInputState) -> (bool, bool) {
    let left = (input.right && !input.left) || input.forward;
    let right = (input.left && !input.right) || input.forward;
    (left, right)
}

fn riding_jump_command_data(charge_seconds: f64) -> i32 {
    (riding_jump_scale(charge_seconds) * 100.0).floor() as i32
}

fn riding_jump_scale(charge_seconds: f64) -> f32 {
    let ticks = (charge_seconds.max(0.0) / RIDING_JUMP_TICK_SECONDS).floor() as i32;
    if ticks <= 0 {
        return 0.0;
    }
    if ticks < 10 {
        ticks as f32 * 0.1
    } else {
        0.8 + 2.0 / (ticks - 9) as f32 * 0.1
    }
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
    input.move_position_reminder_ticks = input.move_position_reminder_ticks.saturating_add(1);

    let last_pose = input.last_move_command_pose;
    let force_position = last_pose.is_some()
        && input.move_position_reminder_ticks >= MOVE_COMMAND_POSITION_REMINDER_TICKS;
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
        force_pos_rot: false,
        force_rotation_only: false,
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
        if send_position {
            input.move_position_reminder_ticks = 0;
        }
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
    use std::collections::BTreeMap;

    use super::*;
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_CAMEL_ID, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
        VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_MINECART_ID,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID, VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_STRIDER_ID,
    };
    use bbb_protocol::item_types::{
        VANILLA_ITEM_CARROT_ON_A_STICK_ID, VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID,
    };
    use bbb_protocol::packets::{
        AddEntity, CommonPlayerSpawnInfo, EntityMove, EquipmentSlot, EquipmentSlotUpdate,
        GameEvent as ProtocolGameEvent, ItemStackSummary, PaddleBoat, PlayLogin, PlayTime,
        PlayerAbilities, PlayerAbilitiesCommand, SetCamera, SetEquipment, SetPassengers,
        SetPlayerInventory, Vec3d as ProtocolVec3d,
    };
    use bbb_world::ItemEquipmentSlot;
    use uuid::Uuid;

    const SADDLE_ITEM_ID: i32 = 8_902;

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
    fn client_tick_catch_up_preserves_each_tutorial_movement_sample() {
        let mut input = ClientInputState::new(true);
        input.forward = true;
        input.mouse_delta_x = 1.0;
        let mut world = WorldStore::new();
        world.apply_login(&protocol_play_login(99));
        world.advance_tutorial_ticks(1);
        world.advance_tutorial_ticks(100);
        let mut counters = NetCounters::default();
        let commands = None;

        advance_player_input_for_client_ticks(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            Instant::now(),
            5,
        );
        world.advance_tutorial_ticks(5);

        assert_eq!(
            world
                .active_tutorial_toast()
                .and_then(|toast| toast.progress_millionths),
            Some(125_000)
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
    fn advance_player_input_does_not_report_unmounted_movement_for_remote_camera() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.forward = true;
        let mut world = WorldStore::new();
        world.apply_login(&protocol_play_login(99));
        world.apply_add_entity(protocol_add_entity_with_type(123, 0));
        assert!(world.apply_set_camera(SetCamera { camera_id: 123 }));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.0, 64.0, 0.0),
            ..LocalPlayerPoseState::default()
        });
        let mut counters = NetCounters::default();

        advance_player_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            Instant::now(),
        );

        assert!(rx.try_recv().is_err());
        assert_eq!(counters.player_move_commands_queued, 0);
    }

    #[test]
    fn advance_player_input_uses_fixed_movement_tick_accumulator() {
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
        assert!(matches!(rx.try_recv().unwrap(), NetCommand::MovePlayer(_)));
        let initial_pose = world.local_player_pose().unwrap();

        input.forward = true;
        input.mouse_delta_x = 100.0;
        advance_player_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            start + Duration::from_millis(25),
        );
        assert!(rx.try_recv().is_err());
        let half_tick_pose = world.local_player_pose().unwrap();
        assert_eq!(half_tick_pose.position, initial_pose.position);
        assert_eq!(half_tick_pose.y_rot, 12.0);

        advance_player_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            start + Duration::from_millis(50),
        );
        let fixed_tick = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected move command, got {other:?}"),
        };
        assert_ne!(
            world.local_player_pose().unwrap().position,
            initial_pose.position
        );
        assert_eq!(
            fixed_tick.state.position,
            world.local_player_pose().unwrap().position
        );
        assert_eq!(counters.player_move_commands_queued, 2);
    }

    #[test]
    fn synchronized_client_ticks_are_the_only_production_movement_clock() {
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

        advance_player_input_for_client_ticks(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            start,
            0,
        );
        assert!(matches!(rx.try_recv().unwrap(), NetCommand::MovePlayer(_)));
        let initial_pose = world.local_player_pose().unwrap();

        input.forward = true;
        advance_player_input_for_client_ticks(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            start + Duration::from_millis(50),
            0,
        );
        assert_eq!(world.local_player_pose(), Some(initial_pose));
        assert!(rx.try_recv().is_err());

        advance_player_input_for_client_ticks(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            start + Duration::from_millis(100),
            1,
        );
        let moved = world.local_player_pose().unwrap();
        assert!(moved.position.z > initial_pose.position.z);
        assert!(matches!(rx.try_recv().unwrap(), NetCommand::MovePlayer(_)));
        assert_eq!(counters.player_move_commands_queued, 2);
    }

    #[test]
    fn advance_player_input_enables_spectator_flying_when_server_allows() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.apply_game_event(ProtocolGameEvent {
            event_id: 3,
            param: 3.0,
        });
        world.apply_player_abilities(PlayerAbilities {
            invulnerable: false,
            flying: false,
            can_fly: true,
            instabuild: false,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        let mut counters = NetCounters::default();

        advance_player_input(&mut input, &mut world, &mut counters, &commands, start);

        assert!(world.local_player().abilities.unwrap().flying);
        assert_eq!(counters.player_abilities_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAbilities(PlayerAbilitiesCommand { flying: true })
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn advance_player_input_does_not_enable_spectator_flying_without_permission() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.apply_game_event(ProtocolGameEvent {
            event_id: 3,
            param: 3.0,
        });
        world.apply_player_abilities(PlayerAbilities {
            invulnerable: false,
            flying: false,
            can_fly: false,
            instabuild: false,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        let mut counters = NetCounters::default();

        advance_player_input(&mut input, &mut world, &mut counters, &commands, start);

        assert!(!world.local_player().abilities.unwrap().flying);
        assert_eq!(counters.player_abilities_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn advance_player_input_disables_non_spectator_flying_after_landing() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState {
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });
        world.apply_player_abilities(PlayerAbilities {
            invulnerable: true,
            flying: true,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        let mut counters = NetCounters::default();

        advance_player_input(&mut input, &mut world, &mut counters, &commands, start);

        assert!(!world.local_player().abilities.unwrap().flying);
        assert_eq!(counters.player_abilities_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAbilities(PlayerAbilitiesCommand { flying: false })
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn advance_player_input_keeps_spectator_flying_while_grounded() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.apply_game_event(ProtocolGameEvent {
            event_id: 3,
            param: 3.0,
        });
        world.set_local_player_pose(LocalPlayerPoseState {
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });
        world.apply_player_abilities(PlayerAbilities {
            invulnerable: true,
            flying: true,
            can_fly: true,
            instabuild: false,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        let mut counters = NetCounters::default();

        advance_player_input(&mut input, &mut world, &mut counters, &commands, start);

        assert!(world.local_player().abilities.unwrap().flying);
        assert_eq!(counters.player_abilities_commands_queued, 0);
        while let Ok(command) = rx.try_recv() {
            assert!(!matches!(command, NetCommand::PlayerAbilities(_)));
        }
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

        for tick in 1..20 {
            maybe_queue_player_move_command(
                &mut input,
                &mut counters,
                &commands,
                pose,
                start + MOVE_COMMAND_INTERVAL * tick,
            );
            assert!(rx.try_recv().is_err(), "tick {tick}");
        }

        maybe_queue_player_move_command(
            &mut input,
            &mut counters,
            &commands,
            pose,
            start + MOVE_COMMAND_INTERVAL * MOVE_COMMAND_POSITION_REMINDER_TICKS,
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
    fn move_command_position_reminder_is_not_reset_by_rotation_only_commands() {
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut pose = LocalPlayerPoseState {
            position: vec3(0.0, 64.0, 0.0),
            ..LocalPlayerPoseState::default()
        };

        maybe_queue_player_move_command(&mut input, &mut counters, &commands, pose, start);
        let first = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected move command, got {other:?}"),
        };
        assert!(!first.force_position);

        for tick in 1..20 {
            pose.y_rot = tick as f32;
            maybe_queue_player_move_command(
                &mut input,
                &mut counters,
                &commands,
                pose,
                start + MOVE_COMMAND_INTERVAL * tick,
            );
            let rotation_only = match rx.try_recv().unwrap() {
                NetCommand::MovePlayer(command) => command,
                other => panic!("expected move command, got {other:?}"),
            };
            assert_eq!(rotation_only.state.position, first.state.position);
            assert!(!rotation_only.force_position, "tick {tick}");
        }

        pose.y_rot = 20.0;
        maybe_queue_player_move_command(
            &mut input,
            &mut counters,
            &commands,
            pose,
            start + MOVE_COMMAND_INTERVAL * MOVE_COMMAND_POSITION_REMINDER_TICKS,
        );
        let reminder = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected move command, got {other:?}"),
        };
        assert!(reminder.force_position);
        assert_eq!(reminder.state.position, first.state.position);

        pose.y_rot = 21.0;
        maybe_queue_player_move_command(
            &mut input,
            &mut counters,
            &commands,
            pose,
            start + MOVE_COMMAND_INTERVAL * (MOVE_COMMAND_POSITION_REMINDER_TICKS + 1),
        );
        let rotation_after_reminder = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected move command, got {other:?}"),
        };
        assert!(!rotation_after_reminder.force_position);
        assert_eq!(counters.player_move_commands_queued, 22);
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
        assert!(!rotation_only.force_rotation_only);
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
        let (tx, mut rx) = mpsc::channel(6);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        input.left = true;
        let mut world = world_with_local_vehicle(VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
        let initial_pose = world.local_player_pose().unwrap();
        let mut counters = NetCounters::default();

        advance_player_input(&mut input, &mut world, &mut counters, &commands, start);
        let first_rotation = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected passenger rotation command, got {other:?}"),
        };
        assert!(first_rotation.force_rotation_only);
        assert_eq!(first_rotation.state.position, initial_pose.position);
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
        assert_eq!(counters.player_move_commands_queued, 1);

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
        let second_rotation = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected passenger rotation command, got {other:?}"),
        };
        assert!(second_rotation.force_rotation_only);
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
        assert_eq!(counters.player_move_commands_queued, 2);
    }

    #[test]
    fn noncontrolling_boat_passenger_only_reports_player_rotation() {
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.forward = true;
        let mut world = world_with_local_vehicle(VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
        world.apply_add_entity(protocol_add_entity_with_type(123, 0));
        assert!(world.apply_set_passengers(SetPassengers {
            vehicle_id: 10,
            passenger_ids: vec![123, 99],
        }));
        let mut counters = NetCounters::default();

        advance_player_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            Instant::now(),
        );

        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::MovePlayer(command) if command.force_rotation_only
        ));
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.player_move_commands_queued, 1);
        assert_eq!(counters.paddle_boat_commands_queued, 0);
        assert_eq!(counters.move_vehicle_commands_queued, 0);
    }

    #[test]
    fn mounted_mouse_motion_updates_look_and_queues_riding_rotation() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        input.mouse_delta_x = 100.0;
        input.mouse_delta_y = -50.0;
        let mut world = world_with_local_vehicle(VANILLA_ENTITY_TYPE_MINECART_ID);
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
        assert!(command.force_rotation_only);
        assert_eq!(
            world.local_player_pose().unwrap().position,
            initial_pose.position
        );
        assert_eq!(counters.player_move_commands_queued, 1);
        assert_eq!(counters.paddle_boat_commands_queued, 0);
        assert_eq!(counters.move_vehicle_commands_queued, 0);
    }

    #[test]
    fn mounted_non_boat_input_queues_passenger_rotation_only() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        input.forward = true;
        let mut world = world_with_local_vehicle(VANILLA_ENTITY_TYPE_MINECART_ID);
        let initial_pose = world.local_player_pose().unwrap();
        let mut counters = NetCounters::default();

        advance_player_input(&mut input, &mut world, &mut counters, &commands, start);

        let command = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected passenger rotation command, got {other:?}"),
        };
        assert!(command.force_rotation_only);
        assert_eq!(command.state.position, initial_pose.position);
        assert_eq!(command.state.y_rot, initial_pose.y_rot);
        assert_eq!(command.state.x_rot, initial_pose.x_rot);
        assert!(rx.try_recv().is_err());
        assert_eq!(world.local_player_pose(), Some(initial_pose));
        assert_eq!(counters.paddle_boat_commands_queued, 0);
        assert_eq!(counters.move_vehicle_commands_queued, 0);
        assert_eq!(counters.player_move_commands_queued, 1);
    }

    #[test]
    fn passenger_rotation_commands_preserve_each_synchronized_tick_opportunity() {
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        let now = Instant::now();
        let mut input = ClientInputState::new(true);
        let mut world = world_with_local_vehicle(VANILLA_ENTITY_TYPE_MINECART_ID);
        let mut counters = NetCounters::default();

        advance_player_input_for_client_ticks(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            now,
            3,
        );

        for _ in 0..3 {
            assert!(matches!(
                rx.try_recv().unwrap(),
                NetCommand::MovePlayer(command) if command.force_rotation_only
            ));
        }
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.player_move_commands_queued, 3);

        advance_player_input_for_client_ticks(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            now,
            0,
        );
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.player_move_commands_queued, 3);
    }

    #[test]
    fn passenger_rotations_preserve_unmounted_position_send_baseline() {
        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let now = Instant::now();
        let mut input = ClientInputState::new(true);
        let baseline = LocalPlayerPoseState {
            position: vec3(0.0, 64.0, 0.0),
            y_rot: 10.0,
            x_rot: -5.0,
            ..LocalPlayerPoseState::default()
        };
        input.last_move_command_pose = Some(baseline);
        input.move_position_reminder_ticks = 7;
        let mut world = world_with_local_vehicle(VANILLA_ENTITY_TYPE_MINECART_ID);
        let passenger_pose = LocalPlayerPoseState {
            position: vec3(8.0, 65.0, -3.0),
            y_rot: 35.0,
            x_rot: 12.0,
            ..baseline
        };
        world.set_local_player_pose(passenger_pose);
        let mut counters = NetCounters::default();

        queue_passenger_rotation_commands(
            &mut input,
            &mut counters,
            &commands,
            passenger_pose,
            now,
            1,
        );
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::MovePlayer(command) if command.force_rotation_only
        ));
        assert_eq!(input.last_move_command_pose, Some(baseline));
        assert_eq!(input.move_position_reminder_ticks, 7);

        assert!(world.apply_set_passengers(SetPassengers {
            vehicle_id: 10,
            passenger_ids: Vec::new(),
        }));
        maybe_queue_player_move_command(
            &mut input,
            &mut counters,
            &commands,
            passenger_pose,
            now + MOVE_COMMAND_INTERVAL,
        );
        let dismount = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            other => panic!("expected post-dismount position command, got {other:?}"),
        };
        assert_eq!(dismount.state.position, passenger_pose.position);
        assert!(!dismount.force_rotation_only);
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.player_move_commands_queued, 2);
    }

    #[test]
    fn controlled_happy_ghast_queues_vehicle_move_without_boat_paddles() {
        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let now = Instant::now();
        let mut input = ClientInputState::new(true);
        input.forward = true;
        let mut world = world_with_local_vehicle(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID);
        let initial = world.probe_entity(10).unwrap();
        let mut counters = NetCounters::default();

        advance_player_input_for_client_ticks(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            now,
            1,
        );

        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::MovePlayer(command) if command.force_rotation_only
        ));
        let vehicle = match rx.try_recv().unwrap() {
            NetCommand::MoveVehicle(command) => command,
            other => panic!("expected Happy Ghast move command, got {other:?}"),
        };
        assert!(vehicle.position.z > initial.position.z);
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.player_move_commands_queued, 1);
        assert_eq!(counters.move_vehicle_commands_queued, 1);
        assert_eq!(counters.paddle_boat_commands_queued, 0);
    }

    #[test]
    fn controlled_abstract_horse_queues_predicted_vehicle_move_without_boat_paddles() {
        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let now = Instant::now();
        let mut input = ClientInputState::new(true);
        input.forward = true;
        let mut world = world_with_local_vehicle(VANILLA_ENTITY_TYPE_HORSE_ID);
        let initial = world.probe_entity(10).unwrap();
        let mut counters = NetCounters::default();

        advance_player_input_for_client_ticks(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            now,
            1,
        );

        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::MovePlayer(command) if command.force_rotation_only
        ));
        let vehicle = match rx.try_recv().unwrap() {
            NetCommand::MoveVehicle(command) => command,
            other => panic!("expected AbstractHorse move command, got {other:?}"),
        };
        assert!(vehicle.position.z > initial.position.z);
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.player_move_commands_queued, 1);
        assert_eq!(counters.move_vehicle_commands_queued, 1);
        assert_eq!(counters.paddle_boat_commands_queued, 0);
    }

    #[test]
    fn controlled_item_based_steering_mounts_queue_predicted_vehicle_moves() {
        for entity_type_id in [VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_STRIDER_ID] {
            let (tx, mut rx) = mpsc::channel(3);
            let commands = Some(tx);
            let now = Instant::now();
            let mut input = ClientInputState::new(true);
            let mut world = world_with_local_vehicle(entity_type_id);
            let initial = world.probe_entity(10).unwrap();
            let mut counters = NetCounters::default();

            advance_player_input_for_client_ticks(
                &mut input,
                &mut world,
                &mut counters,
                &commands,
                now,
                1,
            );

            assert!(matches!(
                rx.try_recv().unwrap(),
                NetCommand::MovePlayer(command) if command.force_rotation_only
            ));
            let vehicle = match rx.try_recv().unwrap() {
                NetCommand::MoveVehicle(command) => command,
                other => panic!("expected item-steered move command, got {other:?}"),
            };
            assert!(vehicle.position.z > initial.position.z);
            assert!(rx.try_recv().is_err());
            assert_eq!(counters.player_move_commands_queued, 1);
            assert_eq!(counters.move_vehicle_commands_queued, 1);
            assert_eq!(counters.paddle_boat_commands_queued, 0);
        }
    }

    #[test]
    fn riding_jump_scale_matches_vanilla_local_player_curve() {
        let assert_scale = |actual: f32, expected: f32| {
            assert!(
                (actual - expected).abs() < 1.0e-6,
                "expected scale {expected}, got {actual}"
            );
        };
        assert_eq!(riding_jump_scale(-1.0), 0.0);
        assert_eq!(riding_jump_scale(0.0), 0.0);
        assert_scale(riding_jump_scale(0.25), 0.5);
        assert_scale(riding_jump_scale(0.45), 0.9);
        assert_scale(riding_jump_scale(0.50), 1.0);
        assert_scale(riding_jump_scale(0.55), 0.9);
        assert_eq!(riding_jump_command_data(0.25), 50);
        assert_eq!(riding_jump_command_data(0.55), 90);
    }

    #[test]
    fn mounted_jumpable_vehicle_release_queues_riding_jump_command() {
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        input.jump = true;
        let mut world = world_with_local_vehicle(VANILLA_ENTITY_TYPE_HORSE_ID);
        let mut counters = NetCounters::default();

        advance_player_input(&mut input, &mut world, &mut counters, &commands, start);
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::MovePlayer(command) if command.force_rotation_only
        ));
        assert!(rx.try_recv().is_err());

        input.jump = false;
        advance_player_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            start + Duration::from_millis(100),
        );

        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::MovePlayer(command) if command.force_rotation_only
        ));
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::MovePlayer(command) if command.force_rotation_only
        ));
        let command = match rx.try_recv().unwrap() {
            NetCommand::PlayerCommand(command) => command,
            other => panic!("expected riding jump player command, got {other:?}"),
        };
        assert_eq!(command.entity_id, 99);
        assert_eq!(command.action, PlayerCommandAction::StartRidingJump);
        assert_eq!(command.data, 20);
        assert!(matches!(rx.try_recv().unwrap(), NetCommand::MoveVehicle(_)));
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.player_command_commands_queued, 1);
    }

    #[test]
    fn zero_charge_riding_jump_release_still_queues_vanilla_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut world = world_with_local_vehicle(VANILLA_ENTITY_TYPE_HORSE_ID);
        let mut counters = NetCounters::default();

        queue_released_riding_jump_command(Some(0.0), &mut world, &mut counters, &commands);

        let command = match rx.try_recv().unwrap() {
            NetCommand::PlayerCommand(command) => command,
            other => panic!("expected zero-charge riding jump command, got {other:?}"),
        };
        assert_eq!(command.action, PlayerCommandAction::StartRidingJump);
        assert_eq!(command.data, 0);
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.player_command_commands_queued, 1);
    }

    #[test]
    fn camel_jump_release_starts_dash_before_vehicle_report() {
        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let now = Instant::now();
        let mut input = ClientInputState::new(true);
        input.forward = true;
        input.riding_jump_charge_seconds = Some(0.40);
        let mut world = world_with_local_vehicle(VANILLA_ENTITY_TYPE_CAMEL_ID);
        let mut counters = NetCounters::default();

        advance_player_input_for_client_ticks(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            now,
            1,
        );

        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::MovePlayer(command) if command.force_rotation_only
        ));
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerCommand(command)
                if command.action == PlayerCommandAction::StartRidingJump && command.data == 90
        ));
        assert!(matches!(rx.try_recv().unwrap(), NetCommand::MoveVehicle(_)));
        assert!(rx.try_recv().is_err());
        assert_eq!(
            world.local_player_rideable_jumping_vehicle_cooldown_ticks(),
            Some(54)
        );
    }

    #[test]
    fn nautilus_jump_release_starts_dash_before_vehicle_report() {
        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let now = Instant::now();
        let mut input = ClientInputState::new(true);
        input.forward = true;
        input.riding_jump_charge_seconds = Some(0.40);
        let mut world = world_with_local_vehicle(VANILLA_ENTITY_TYPE_NAUTILUS_ID);
        let mut counters = NetCounters::default();

        advance_player_input_for_client_ticks(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            now,
            1,
        );

        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::MovePlayer(command) if command.force_rotation_only
        ));
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerCommand(command)
                if command.action == PlayerCommandAction::StartRidingJump && command.data == 90
        ));
        assert!(matches!(rx.try_recv().unwrap(), NetCommand::MoveVehicle(_)));
        assert!(rx.try_recv().is_err());
        assert_eq!(
            world.local_player_rideable_jumping_vehicle_cooldown_ticks(),
            Some(39)
        );
    }

    #[test]
    fn mounted_non_jumpable_vehicle_release_does_not_queue_riding_jump() {
        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        input.jump = true;
        let mut world = world_with_local_vehicle(VANILLA_ENTITY_TYPE_MINECART_ID);
        let mut counters = NetCounters::default();

        advance_player_input(&mut input, &mut world, &mut counters, &commands, start);
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::MovePlayer(command) if command.force_rotation_only
        ));
        input.jump = false;
        advance_player_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            start + Duration::from_millis(100),
        );

        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::MovePlayer(command) if command.force_rotation_only
        ));
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::MovePlayer(command) if command.force_rotation_only
        ));
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

    #[test]
    fn local_player_underwater_audio_ticks_cover_unmounted_and_mounted_player_ticks() {
        let now = Instant::now();
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let commands = None;
        let mut unmounted = WorldStore::new();
        unmounted.apply_login(&protocol_play_login(99));
        unmounted.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.0, 64.0, 0.0),
            ..LocalPlayerPoseState::default()
        });

        advance_player_input_for_client_ticks(
            &mut input,
            &mut unmounted,
            &mut counters,
            &commands,
            now,
            3,
        );
        let unmounted_ticks = unmounted.take_pending_local_player_underwater_audio_ticks();
        assert_eq!(unmounted_ticks.len(), 3);
        assert_eq!(unmounted_ticks[0].position, vec3(0.0, 64.0, 0.0));
        assert!(unmounted_ticks.iter().all(|tick| !tick.underwater));

        let mut mounted = world_with_local_vehicle(VANILLA_ENTITY_TYPE_MINECART_ID);
        advance_player_input_for_client_ticks(
            &mut input,
            &mut mounted,
            &mut counters,
            &commands,
            now,
            3,
        );
        let mounted_ticks = mounted.take_pending_local_player_underwater_audio_ticks();
        assert_eq!(mounted_ticks.len(), 3);
        assert!(mounted_ticks.iter().all(|tick| !tick.underwater));
    }

    fn vec3(x: f64, y: f64, z: f64) -> ProtocolVec3d {
        ProtocolVec3d { x, y, z }
    }

    fn world_with_local_vehicle(entity_type_id: i32) -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_login(&protocol_play_login(99));
        if entity_type_id == VANILLA_ENTITY_TYPE_CAMEL_ID {
            world.apply_world_time(PlayTime {
                game_time: 100,
                clock_updates: Vec::new(),
            });
        }
        world.apply_add_entity(protocol_add_entity_with_type(10, entity_type_id));
        if matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_HORSE_ID
                | VANILLA_ENTITY_TYPE_CAMEL_ID
                | VANILLA_ENTITY_TYPE_NAUTILUS_ID
                | VANILLA_ENTITY_TYPE_PIG_ID
                | VANILLA_ENTITY_TYPE_STRIDER_ID
        ) {
            world.set_default_item_equipment_slots(BTreeMap::from([(
                SADDLE_ITEM_ID,
                ItemEquipmentSlot::Saddle,
            )]));
            assert!(world.apply_set_equipment(SetEquipment {
                entity_id: 10,
                slots: vec![EquipmentSlotUpdate {
                    slot: EquipmentSlot::Saddle,
                    item: ItemStackSummary {
                        item_id: Some(SADDLE_ITEM_ID),
                        count: 1,
                        component_patch: Default::default(),
                    },
                }],
            }));
            if entity_type_id == VANILLA_ENTITY_TYPE_CAMEL_ID {
                assert!(world.apply_entity_move(EntityMove {
                    id: 10,
                    has_position: false,
                    delta_x: 0,
                    delta_y: 0,
                    delta_z: 0,
                    y_rot: None,
                    x_rot: None,
                    on_ground: true,
                }));
            }
            let steering_item_id = match entity_type_id {
                VANILLA_ENTITY_TYPE_PIG_ID => Some(VANILLA_ITEM_CARROT_ON_A_STICK_ID),
                VANILLA_ENTITY_TYPE_STRIDER_ID => Some(VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID),
                _ => None,
            };
            if let Some(item_id) = steering_item_id {
                assert!(world.apply_set_player_inventory(SetPlayerInventory {
                    slot: 0,
                    item: ItemStackSummary {
                        item_id: Some(item_id),
                        count: 1,
                        component_patch: Default::default(),
                    },
                }));
            }
        } else if entity_type_id == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID {
            assert!(world.apply_set_equipment(SetEquipment {
                entity_id: 10,
                slots: vec![EquipmentSlotUpdate {
                    slot: EquipmentSlot::Body,
                    item: ItemStackSummary {
                        item_id: Some(SADDLE_ITEM_ID),
                        count: 1,
                        component_patch: Default::default(),
                    },
                }],
            }));
        }
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
