use super::*;

#[test]
fn movement_key_changes_queue_player_input_commands() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
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

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );
    assert!(rx.try_recv().is_err());
    assert_eq!(counters.player_input_commands_queued, 1);

    handle_key_input_without_world(
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
fn release_active_input_keeps_shift_modifier_for_inventory_clicks() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    );
    assert!(input.shift_down());
    assert!(input.sneak);

    release_active_input(&mut input, &mut world, &mut counters, &None);

    assert!(input.shift_down());
    assert!(!input.sneak);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Released,
    );
    assert!(!input.shift_down());
}

#[test]
fn item_model_keybind_context_tracks_default_key_and_mouse_state() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::Digit5),
        ElementState::Pressed,
    );

    let context = input.item_model_keybind_context();
    assert!(context.keybind_down("key.forward"));
    assert!(context.keybind_down("key.hotbar.5"));
    assert!(!context.keybind_down("key.use"));

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Released,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::Digit5),
        ElementState::Released,
    );

    let context = input.item_model_keybind_context();
    assert!(!context.keybind_down("key.forward"));
    assert!(!context.keybind_down("key.hotbar.5"));

    let mut world = WorldStore::new();
    handle_mouse_input_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &None,
        MouseButton::Right,
        ElementState::Pressed,
        1.0,
    );
    assert!(input.item_model_keybind_context().keybind_down("key.use"));
    assert!(!input
        .item_model_keybind_context()
        .keybind_down("key.attack"));

    handle_mouse_input_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &None,
        MouseButton::Right,
        ElementState::Released,
        1.0,
    );
    assert!(!input.item_model_keybind_context().keybind_down("key.use"));
}

#[test]
fn item_model_keybind_context_tracks_non_debug_default_keymappings() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    for (code, keybind) in [
        (KeyCode::F2, "key.screenshot"),
        (KeyCode::F5, "key.togglePerspective"),
        (KeyCode::F11, "key.fullscreen"),
        (KeyCode::KeyG, "key.quickActions"),
        (KeyCode::F1, "key.toggleGui"),
        (KeyCode::F4, "key.toggleSpectatorShaderEffects"),
        (KeyCode::KeyC, "key.saveToolbarActivator"),
        (KeyCode::KeyX, "key.loadToolbarActivator"),
    ] {
        handle_key_input_without_world(
            &mut input,
            &mut counters,
            &None,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );
        assert!(input.item_model_keybind_context().keybind_down(keybind));
        handle_key_input_without_world(
            &mut input,
            &mut counters,
            &None,
            PhysicalKey::Code(code),
            ElementState::Released,
        );
        assert!(!input.item_model_keybind_context().keybind_down(keybind));
    }

    // Screen-opening bindings release all active input after their key press,
    // so exercise their raw keymapping projection without opening the screen.
    input.set_key_down(KeyCode::KeyP, true);
    assert!(input
        .item_model_keybind_context()
        .keybind_down("key.socialInteractions"));
    input.set_key_down(KeyCode::KeyP, false);
    assert!(!input
        .item_model_keybind_context()
        .keybind_down("key.socialInteractions"));

    input.set_key_down(KeyCode::KeyL, true);
    assert!(input
        .item_model_keybind_context()
        .keybind_down("key.advancements"));
    input.set_key_down(KeyCode::KeyL, false);
    assert!(!input
        .item_model_keybind_context()
        .keybind_down("key.advancements"));

    let mut world = WorldStore::new();
    handle_mouse_input_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &None,
        MouseButton::Middle,
        ElementState::Pressed,
        1.0,
    );
    let context = input.item_model_keybind_context();
    assert!(context.keybind_down("key.pickItem"));
    assert!(context.keybind_down("key.spectatorHotbar"));
}

#[test]
fn toggle_gui_key_flips_hide_gui_once_per_press_edge() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    assert!(!input.hide_gui());

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::F1),
        ElementState::Pressed,
    );
    assert!(input.hide_gui());
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::F1),
        ElementState::Pressed,
    );
    assert!(input.hide_gui());
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::F1),
        ElementState::Released,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::F1),
        ElementState::Pressed,
    );
    assert!(!input.hide_gui());
}

#[test]
fn camera_post_effect_selection_and_f4_edges_preserve_vanilla_active_lifecycle() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    input.synchronize_camera_post_effect(7, 1, Some("minecraft:creeper"));
    assert!(input.camera_post_effect_active());
    assert_eq!(input.camera_post_effect_id(), Some("minecraft:creeper"));

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Pressed,
    );
    assert!(!input.camera_post_effect_active());
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Pressed,
    );
    assert!(
        !input.camera_post_effect_active(),
        "a held key must not consume a second click"
    );
    input.synchronize_camera_post_effect(7, 1, Some("minecraft:creeper"));
    assert!(
        !input.camera_post_effect_active(),
        "same-selection extraction must preserve the disabled state"
    );
    assert!(!input.camera_post_effect_active());
    assert_eq!(input.camera_post_effect_id(), Some("minecraft:creeper"));

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Released,
    );
    input.synchronize_camera_post_effect(7, 2, Some("minecraft:creeper"));
    assert!(
        input.camera_post_effect_active(),
        "a repeated applied SetCamera selection reenables its effect"
    );
    input.synchronize_camera_post_effect(7, 3, None);
    assert!(
        !input.camera_post_effect_active(),
        "clearing the selected effect clears active state"
    );
    assert_eq!(input.camera_post_effect_id(), None);

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Pressed,
    );
    assert!(
        input.camera_post_effect_active(),
        "F4 remains independent even when no effect id is selected"
    );
    input.synchronize_camera_post_effect(8, 0, None);
    assert!(
        !input.camera_post_effect_active(),
        "a new level selection token clears an absent effect"
    );
}

#[test]
fn camera_perspective_cycles_and_applies_only_first_person_post_effect_boundaries() {
    let mut input = ClientInputState::new(true);
    input.synchronize_camera_post_effect(7, 1, Some("minecraft:spider"));
    assert_eq!(input.camera_type(), CameraType::FirstPerson);

    input.cycle_camera_type(Some("minecraft:spider"));
    assert_eq!(input.camera_type(), CameraType::ThirdPersonBack);
    assert_eq!(input.camera_post_effect_id(), None);
    assert!(!input.camera_post_effect_active());

    input.cycle_camera_type(Some("minecraft:spider"));
    assert_eq!(input.camera_type(), CameraType::ThirdPersonFront);
    assert_eq!(input.camera_post_effect_id(), None);

    input.synchronize_camera_post_effect(7, 2, Some("minecraft:creeper"));
    assert_eq!(input.camera_post_effect_id(), Some("minecraft:creeper"));
    assert!(
        input.camera_post_effect_active(),
        "SetCamera must refresh an effect even while detached"
    );

    input.cycle_camera_type(Some("minecraft:invert"));
    assert_eq!(input.camera_type(), CameraType::FirstPerson);
    assert_eq!(input.camera_post_effect_id(), Some("minecraft:invert"));
    assert!(input.camera_post_effect_active());

    let publication = &input;
    assert_eq!(publication.camera_type(), CameraType::FirstPerson);
    assert_eq!(
        publication.camera_post_effect_id(),
        Some("minecraft:invert")
    );
}

#[test]
fn camera_perspective_f5_consumes_one_cycle_per_press_edge() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let net_commands = None;

    for state in [ElementState::Pressed, ElementState::Pressed] {
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &net_commands,
            PhysicalKey::Code(KeyCode::F5),
            state,
        );
    }
    assert_eq!(input.camera_type(), CameraType::ThirdPersonBack);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        PhysicalKey::Code(KeyCode::F5),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        PhysicalKey::Code(KeyCode::F5),
        ElementState::Pressed,
    );
    assert_eq!(input.camera_type(), CameraType::ThirdPersonFront);
}

#[test]
fn item_model_keybind_context_accepts_vanilla_default_unbound_keymappings() {
    // Vanilla Options registers these with InputConstants.UNKNOWN, so
    // IsKeybindDown can decode the key names but KeyMapping.isDown() is false
    // under the default keymap.
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    for code in [KeyCode::F4, KeyCode::F5] {
        handle_key_input_without_world(
            &mut input,
            &mut counters,
            &None,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );
    }

    let context = input.item_model_keybind_context();
    assert!(!context.keybind_down("key.smoothCamera"));
    assert!(!context.keybind_down("key.spectatorOutlines"));
}

fn assert_sprint_key_on_mount_only_queues_raw_player_input(entity_type_id: i32) {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_vehicle(77, 10, entity_type_id);
    set_local_player_sprint_test_pose(&mut world);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            sprint: true,
            ..PlayerInput::default()
        })
    );
    assert!(rx.try_recv().is_err());
}

fn assert_sprint_key_on_camel_type_queues_start_sprinting(entity_type_id: i32) {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_vehicle(77, 10, entity_type_id);
    set_local_player_sprint_test_pose(&mut world);
    world.apply_player_health(PlayerHealth {
        health: 20.0,
        food: 6,
        saturation: 0.0,
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Instant::now(),
        1,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            sprint: true,
            ..PlayerInput::default()
        })
    );
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::MovePlayer(PlayerMoveCommand {
            force_rotation_only: true,
            ..
        })
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StartSprinting,
            data: 0,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sprint_key_with_forward_input_queues_player_input_and_sprint_commands() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_local_player_sprint_test_pose(&mut world);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Instant::now(),
        1,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
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
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::MovePlayer(_)));
    assert!(world.local_player_is_sprinting());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );

    assert_eq!(counters.player_input_commands_queued, 2);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            ..PlayerInput::default()
        })
    );
    advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Instant::now(),
        1,
    );
    assert!(world.local_player_is_sprinting());
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::MovePlayer(_)));
    assert!(rx.try_recv().is_err());
}

#[test]
fn double_tap_forward_within_sprint_window_queues_start_sprinting() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_local_player_sprint_test_pose(&mut world);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );
    advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Instant::now(),
        1,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );
    advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Instant::now(),
        1,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            ..PlayerInput::default()
        })
    );
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::MovePlayer(_)));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
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
    assert!(world.local_player_is_sprinting());
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::MovePlayer(_)));
    assert!(rx.try_recv().is_err());
}

#[test]
fn double_tap_forward_after_sprint_window_expires_does_not_start_sprinting() {
    let (tx, mut rx) = mpsc::channel(12);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_local_player_sprint_test_pose(&mut world);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Released,
    );

    let start = Instant::now();
    advance_player_input(&mut input, &mut world, &mut counters, &commands, start);
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_millis(250),
    );
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_millis(350),
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );
    advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Instant::now(),
        1,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_command_commands_queued, 0);
    let queued = std::iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>();
    let raw_inputs = queued
        .iter()
        .filter_map(|command| match command {
            NetCommand::PlayerInput(input) => Some(*input),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        raw_inputs,
        vec![
            PlayerInput {
                forward: true,
                ..PlayerInput::default()
            },
            PlayerInput::default(),
            PlayerInput {
                forward: true,
                ..PlayerInput::default()
            },
        ]
    );
    assert!(!queued.iter().any(|command| matches!(
        command,
        NetCommand::PlayerCommand(PlayerCommand {
            action: PlayerCommandAction::StartSprinting,
            ..
        })
    )));
    assert!(!world.local_player_is_sprinting());
}

#[test]
fn sprint_key_without_forward_input_does_not_start_sprinting_until_forward_pressed() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_local_player_sprint_test_pose(&mut world);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Instant::now(),
        1,
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
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::MovePlayer(_)));
    assert!(rx.try_recv().is_err());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Instant::now(),
        1,
    );

    assert_eq!(counters.player_input_commands_queued, 2);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
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
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::MovePlayer(_)));
    assert!(rx.try_recv().is_err());
}

#[test]
fn sprint_key_with_low_food_only_queues_raw_player_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    world.apply_player_health(PlayerHealth {
        health: 20.0,
        food: 6,
        saturation: 0.0,
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            sprint: true,
            ..PlayerInput::default()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sprint_key_while_using_slow_item_only_queues_raw_player_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: test_item_stack(42, 1),
    });
    world.set_local_using_item(true);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            sprint: true,
            ..PlayerInput::default()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sprint_key_on_horse_mount_only_queues_raw_player_input() {
    assert_sprint_key_on_mount_only_queues_raw_player_input(VANILLA_ENTITY_TYPE_HORSE_ID);
}

#[test]
fn sprint_key_on_boat_mount_only_queues_raw_player_input() {
    assert_sprint_key_on_mount_only_queues_raw_player_input(VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
}

#[test]
fn sprint_key_on_camel_mount_queues_player_input_and_sprint_command_with_low_food() {
    for entity_type_id in [
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
    ] {
        assert_sprint_key_on_camel_type_queues_start_sprinting(entity_type_id);
    }
}

#[test]
fn sprint_key_camel_mount_focus_loss_queues_stop_sprinting() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    input.sprint_key_down = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_vehicle(77, 10, VANILLA_ENTITY_TYPE_CAMEL_ID);
    set_local_player_sprint_test_pose(&mut world);
    assert!(world.reconcile_local_player_sprinting(input.local_player_input(&world), true));
    input.last_sent_sprinting = true;
    input.sprint_level_generation = world.client_level_generation();
    input.sprint_level_load_start = world.level_load().starts;

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);
    advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Instant::now(),
        1,
    );

    assert!(!input.focused);
    assert_eq!(player_input_from_state(&input), PlayerInput::default());
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::MovePlayer(PlayerMoveCommand {
            force_rotation_only: true,
            ..
        })
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StopSprinting,
            data: 0,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sprint_key_without_local_player_id_only_queues_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
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
fn toggle_crouch_flips_sneak_on_press_and_survives_the_release() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    input.set_toggle_key_options(true, false, false, false);

    let mut press = |input: &mut ClientInputState, state| {
        handle_key_input(
            input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::ShiftLeft),
            state,
        );
    };

    press(&mut input, ElementState::Pressed);
    assert!(input.item_model_keybind_context().sneak);
    press(&mut input, ElementState::Released);
    assert!(input.item_model_keybind_context().sneak);
    press(&mut input, ElementState::Pressed);
    assert!(!input.item_model_keybind_context().sneak);

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            shift: true,
            ..PlayerInput::default()
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn closing_a_screen_restores_a_keyboard_bound_toggle_it_released() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let net_commands = None;
    let mut world = world_with_debug_player(false);
    let now = Instant::now();
    input.set_toggle_key_options(true, false, false, false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    );
    assert!(input.item_model_keybind_context().sneak);

    open_test_stats_screen(&mut input);
    crate::input::release_active_input(&mut input, &mut world, &mut counters, &net_commands);
    crate::input::movement::advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &net_commands,
        now,
        0,
    );
    assert!(!input.item_model_keybind_context().sneak);

    input.screen.close();
    crate::input::movement::advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &net_commands,
        now,
        0,
    );
    assert!(input.item_model_keybind_context().sneak);
}

#[test]
fn a_queued_respawn_resets_every_toggle_key_like_local_player_respawn() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    input.set_toggle_key_options(true, true, false, false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );
    assert!(input.item_model_keybind_context().sneak);
    assert!(input.item_model_keybind_context().sprint);

    input.sync_respawn_toggle_reset(&counters);
    assert!(input.item_model_keybind_context().sneak);

    crate::input::queue_perform_respawn_command(&mut counters, &commands);
    input.sync_respawn_toggle_reset(&counters);
    assert!(!input.item_model_keybind_context().sneak);
    assert!(!input.item_model_keybind_context().sprint);

    while rx.try_recv().is_ok() {}
}

#[test]
fn a_rebound_jump_key_moves_both_the_action_and_the_key_it_left() {
    let (tx, mut rx) = mpsc::channel(6);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);

    let mut options = crate::options::ClientOptions::defaults(Default::default());
    options.set_dynamic_raw_for_test("key_key.jump", "key.keyboard.z");
    input.set_key_bindings(crate::options::key_bindings::KeyBindings::from_options(
        &options,
    ));

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );
    assert_eq!(counters.player_input_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyZ),
        ElementState::Pressed,
    );
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            jump: true,
            ..PlayerInput::default()
        })
    );
    assert!(input.item_model_keybind_context().jump);
}

#[test]
fn jump_key_double_tap_toggles_creative_flight() {
    let (tx, mut rx) = mpsc::channel(6);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_player_abilities(&mut world, false, true);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_abilities_commands_queued, 1);
    assert!(world.local_player().abilities.unwrap().flying);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            jump: true,
            ..PlayerInput::default()
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            jump: true,
            ..PlayerInput::default()
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAbilities(PlayerAbilitiesCommand { flying: true })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn jump_key_double_tap_without_can_fly_only_queues_player_input() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_player_abilities(&mut world, false, false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_abilities_commands_queued, 0);
    assert!(!world.local_player().abilities.unwrap().flying);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(rx.try_recv().is_err());
}

#[test]
fn spectator_jump_key_double_tap_does_not_toggle_flying_off() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_local_spectator(&mut world);
    set_player_abilities(&mut world, true, true);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_abilities_commands_queued, 0);
    assert!(world.local_player().abilities.unwrap().flying);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(rx.try_recv().is_err());
}

#[test]
fn creative_flight_jump_trigger_expires_after_vanilla_window() {
    let (tx, mut rx) = mpsc::channel(6);
    let commands = Some(tx);
    let start = Instant::now();
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_player_abilities(&mut world, false, true);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Released,
    );
    advance_player_input(&mut input, &mut world, &mut counters, &commands, start);
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_millis(250),
    );
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_millis(350),
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_abilities_commands_queued, 0);
    assert!(!world.local_player().abilities.unwrap().flying);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(rx.try_recv().is_err());
}

#[test]
fn creative_flight_toggle_suppresses_same_jump_fall_flying_command() {
    let (tx, mut rx) = mpsc::channel(6);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_player_abilities(&mut world, false, true);
    set_local_player_on_ground(&mut world, true);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Released,
    );

    set_local_player_on_ground(&mut world, false);
    equip_test_glider(&mut world, 0);
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_abilities_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert!(world.local_player().abilities.unwrap().flying);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAbilities(PlayerAbilitiesCommand { flying: true })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn jump_key_queues_start_fall_flying_when_airborne_with_usable_glider() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_local_player_on_ground(&mut world, false);
    equip_test_glider(&mut world, 0);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            jump: true,
            ..PlayerInput::default()
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StartFallFlying,
            data: 0,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn start_fall_flying_gate_rejects_last_durability_glider_and_creative_flight() {
    let before = PlayerInput::default();
    let after = PlayerInput {
        jump: true,
        ..PlayerInput::default()
    };
    let mut world = world_with_local_player_id(77);
    set_local_player_on_ground(&mut world, false);
    equip_test_glider(&mut world, 430);
    assert!(should_queue_start_fall_flying(&world, before, after));

    equip_test_glider(&mut world, 431);
    assert!(!should_queue_start_fall_flying(&world, before, after));

    equip_test_glider(&mut world, 0);
    set_player_abilities(&mut world, true, true);
    assert!(!should_queue_start_fall_flying(&world, before, after));
}

#[test]
fn jump_key_on_ground_with_usable_glider_only_queues_player_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_local_player_on_ground(&mut world, true);
    equip_test_glider(&mut world, 0);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            jump: true,
            ..PlayerInput::default()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn jump_key_airborne_without_usable_glider_only_queues_player_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_local_player_on_ground(&mut world, false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            jump: true,
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
    input.sprint_key_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);

    assert!(!input.focused);
    assert_eq!(player_input_from_state(&input), PlayerInput::default());
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
}

#[test]
fn focus_loss_stops_active_sprinting() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    input.sprint_key_down = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_local_player_sprint_test_pose(&mut world);
    assert!(world.reconcile_local_player_sprinting(input.local_player_input(&world), true));
    input.last_sent_sprinting = true;
    input.sprint_level_generation = world.client_level_generation();
    input.sprint_level_load_start = world.level_load().starts;

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);
    advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Instant::now(),
        1,
    );

    assert!(!input.focused);
    assert_eq!(player_input_from_state(&input), PlayerInput::default());
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 1);
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
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::MovePlayer(_)));
}

#[test]
fn focus_loss_stops_boat_paddles() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.left = true;
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_boat(77);

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);

    assert!(!input.focused);
    assert_eq!(player_input_from_state(&input), PlayerInput::default());
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.paddle_boat_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PaddleBoat(PaddleBoat {
            left: false,
            right: false,
        })
    );
}

#[test]
fn focus_loss_releases_charged_riding_jump() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.jump = true;
    input.riding_jump_charge_seconds = Some(0.2);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_vehicle(77, 10, VANILLA_ENTITY_TYPE_HORSE_ID);

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);

    assert!(!input.focused);
    assert_eq!(input.riding_jump_charge_seconds, None);
    assert_eq!(player_input_from_state(&input), PlayerInput::default());
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StartRidingJump,
            data: 40,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn focus_loss_aborts_destroying_block() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let pos = BlockPos { x: 4, y: 70, z: -6 };
    let mut world = world_with_local_player_id(77);
    world.apply_add_entity(AddEntity {
        id: 77,
        uuid: Uuid::from_u128(77),
        entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    insert_empty_chunk_for_block(&mut world, pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: pos.x,
            y: pos.y,
            z: pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:oak_log", [("axis", "y")]),
    }));
    world.apply_update_tags(bbb_protocol::packets::UpdateTags {
        registries: vec![bbb_protocol::packets::RegistryTags {
            registry: "minecraft:block".to_string(),
            tags: vec![bbb_protocol::packets::TagNetworkPayload {
                tag: "minecraft:logs".to_string(),
                entries: vec![bbb_protocol::block_types::VANILLA_BLOCK_OAK_LOG_ID],
            }],
        }],
    });
    world.set_tutorial_step(bbb_client_world::TutorialStep::PunchTree);
    world.advance_tutorial_ticks(1);
    world.advance_tutorial_ticks(600);
    world.tutorial_on_destroy_block(pos, 500_000);
    assert_eq!(
        world
            .active_tutorial_toast()
            .and_then(|toast| toast.progress_millionths),
        Some(500_000)
    );
    world.set_local_destroying_block(pos);
    world.advance_client_level_entity_ticks_with_observation(6);
    assert!(world.local_player_attack_strength_ticks() > 0);
    let mut counters = NetCounters::default();

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);

    assert_eq!(world.local_player().interaction.destroying_block, None);
    assert_eq!(world.local_player_attack_strength_ticks(), 0);
    assert_eq!(
        world
            .active_tutorial_toast()
            .and_then(|toast| toast.progress_millionths),
        Some(0),
        "focus loss follows MultiPlayerGameMode.stopDestroyBlock before clearing state"
    );
    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::AbortDestroyBlock,
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
}

#[test]
fn focus_loss_retries_destroy_abort_without_clearing_state_when_queue_is_full() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetCommand::Swing(InteractionHand::OffHand))
        .unwrap();
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let pos = BlockPos { x: 4, y: 70, z: -6 };
    let mut world = world_with_local_player_id(77);
    world.apply_add_entity(AddEntity {
        id: 77,
        uuid: Uuid::from_u128(77),
        entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.set_local_destroying_block(pos);
    world.advance_client_level_entity_ticks_with_observation(6);
    let attack_strength_before = world.local_player_attack_strength_ticks();
    assert!(attack_strength_before > 0);
    let mut counters = NetCounters::default();

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);

    assert_eq!(world.local_player().interaction.destroying_block, Some(pos));
    assert_eq!(
        world.local_player_attack_strength_ticks(),
        attack_strength_before
    );
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::OffHand)
    );

    release_active_input(&mut input, &mut world, &mut counters, &commands);

    assert_eq!(world.local_player().interaction.destroying_block, None);
    assert_eq!(world.local_player_attack_strength_ticks(), 0);
    assert_eq!(counters.player_action_commands_queued, 1);
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::AbortDestroyBlock,
            ..
        }))
    ));
}

#[test]
fn focus_loss_releases_using_item() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: test_item_stack(VANILLA_ITEM_SPYGLASS_ID, 1),
    });
    world.set_local_using_item(true);
    let mut counters = NetCounters::default();

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);

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
    assert_eq!(
        world.take_pending_local_interaction_sound_actions(),
        vec![bbb_client_world::LocalInteractionSoundAction::SpyglassStopUsing]
    );
}
