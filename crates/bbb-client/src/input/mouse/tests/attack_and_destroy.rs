use super::*;

#[test]
fn left_mouse_press_on_entity_queues_attack_and_swing() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = local_attack_world_with_player_and_target(123);
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
        NetCommand::AttackEntity(AttackEntityCommand {
            packet: AttackEntity { entity_id: 123 },
            selected_hotbar_slot: 0,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    world.advance_entity_client_animations(2);
    let swing = world.local_player_attack_swing(1.0).unwrap();
    assert!(!swing.off_hand);
    assert!(
        swing.attack_anim > 0.0,
        "LocalPlayer.swing arms the local animation before the server omits its sender echo"
    );
}

#[test]
fn left_mouse_press_on_extended_attack_range_entity_queues_attack_and_swing() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_entity_at(123, [0.0, 1.0, 4.0]);
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack_with_attack_range(42, 1, 0.0, 4.5),
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

    assert_eq!(world.local_player().interaction.destroying_block, None);
    assert_eq!(counters.attack_entity_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::AttackEntity(AttackEntityCommand {
            packet: AttackEntity { entity_id: 123 },
            selected_hotbar_slot: 0,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
}

#[test]
fn left_mouse_press_on_entity_outside_selected_attack_range_swings_without_attack() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_entity(123);
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack_with_attack_range(42, 1, 0.0, 1.0),
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

    assert_eq!(world.local_player().interaction.destroying_block, None);
    assert!(input.destroy_block_held);
    assert_eq!(counters.attack_entity_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn left_mouse_press_with_piercing_weapon_queues_stab_and_swing_on_entity() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_entity(123);
    set_selected_piercing_weapon(&mut world, 42);
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );

    assert!(!input.destroy_block_held);
    assert_eq!(world.local_player().interaction.destroying_block, None);
    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(counters.attack_entity_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::Stab,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
}

#[test]
fn spectator_left_mouse_press_on_entity_queues_spectate_only() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_entity(123);
    set_local_spectator(&mut world);
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );

    assert!(!input.destroy_block_held);
    assert_eq!(world.local_player().interaction.destroying_block, None);
    assert_eq!(counters.spectate_entity_commands_queued, 1);
    assert_eq!(counters.attack_entity_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SpectateEntity(SpectateEntity { entity_id: 123 })
    );
}

#[test]
fn spectator_left_mouse_press_with_piercing_weapon_queues_spectate_only() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_entity(123);
    set_selected_piercing_weapon(&mut world, 42);
    set_local_spectator(&mut world);
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );

    assert!(!input.destroy_block_held);
    assert_eq!(counters.spectate_entity_commands_queued, 1);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.attack_entity_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SpectateEntity(SpectateEntity { entity_id: 123 })
    );
}

#[test]
fn left_mouse_press_while_using_item_does_not_attack_or_swing() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_entity(123);
    world.set_local_using_item(true);
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );

    assert!(!input.destroy_block_held);
    assert!(world.local_player().interaction.using_item);
    assert_eq!(counters.attack_entity_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn local_attack_entity_carries_selected_slot_and_resets_both_strength_clocks() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = local_attack_world_with_player_and_target(123);
    assert!(world.set_local_selected_hotbar_slot(4));
    world.advance_client_level_entity_ticks_with_observation(6);
    assert_eq!(world.local_player_attack_strength_ticks(), 6);
    assert_eq!(world.local_player_item_swap_ticks(), 6);
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );

    assert_eq!(world.local_player_attack_strength_ticks(), 0);
    assert_eq!(world.local_player_item_swap_ticks(), 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::AttackEntity(AttackEntityCommand {
            packet: AttackEntity { entity_id: 123 },
            selected_hotbar_slot: 4,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
}

#[test]
fn local_attack_sprint_predicts_vehicle_knockback_and_sends_stop_on_tick() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut world = local_attack_world_with_player_and_target(123);
    world.set_local_player_pose(LocalPlayerPoseState {
        delta_movement: ProtocolVec3d {
            x: 0.2,
            y: 0.4,
            z: 0.3,
        },
        ..LocalPlayerPoseState::default()
    });
    assert!(world.reconcile_local_player_sprinting(input.local_player_input(&world), true));
    input.last_sent_sprinting = true;
    input.sprint_level_generation = world.client_level_generation();
    input.sprint_level_load_start = world.level_load().starts;
    world.advance_client_level_entity_ticks_with_observation(6);
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );

    assert!(!world.local_player_is_sprinting());
    let target = world.probe_entity_transform(123).unwrap();
    assert_f64_near(target.delta_movement.y, 0.1);
    assert_f64_near(target.delta_movement.z, 0.5);
    let player = world.local_player_pose().unwrap();
    assert_f64_near(player.delta_movement.x, 0.12);
    assert_f64_near(player.delta_movement.y, 0.4);
    assert_f64_near(player.delta_movement.z, 0.18);
    assert_eq!(world.local_player_attack_strength_ticks(), 0);
    assert_eq!(world.local_player_item_swap_ticks(), 0);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::AttackEntity(_)
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );

    crate::input::advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        std::time::Instant::now(),
        1,
    );

    assert!(!world.local_player_is_sprinting());
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 42,
            action: PlayerCommandAction::StopSprinting,
            data: 0,
        })
    );
}

#[test]
fn local_attack_held_sprint_key_restarts_before_outbound_stop_is_sent() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    input.sprint_key_down = true;
    input.control_left_down = true;
    let mut world = local_attack_world_with_player_and_target(123);
    assert!(world.reconcile_local_player_sprinting(input.local_player_input(&world), true));
    input.last_sent_sprinting = true;
    input.sprint_level_generation = world.client_level_generation();
    input.sprint_level_load_start = world.level_load().starts;
    world.advance_client_level_entity_ticks_with_observation(6);
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );
    assert!(!world.local_player_is_sprinting());
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::AttackEntity(_)
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );

    crate::input::advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        std::time::Instant::now(),
        1,
    );

    assert!(world.local_player_is_sprinting());
    while let Ok(command) = rx.try_recv() {
        assert!(!matches!(
            command,
            NetCommand::PlayerCommand(PlayerCommand {
                action: PlayerCommandAction::StopSprinting,
                ..
            })
        ));
    }
}

#[test]
fn local_attack_charge_and_feature_gates_emit_no_action_or_swing() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = local_attack_world_with_player_and_target(123);
    world.set_default_item_component_codec_values(BTreeMap::from([(
        42,
        BTreeMap::from([(
            VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
            serde_json::json!(1.0),
        )]),
    )]));
    world.rebuild_item_component_defaults().unwrap();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    world.advance_client_level_entity_ticks_with_observation(5);
    assert_eq!(world.local_player_attack_strength_ticks(), 4);
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );
    assert_eq!(counters.attack_entity_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    world.advance_client_level_entity_ticks_with_observation(1);
    world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec!["minecraft:trade_rebalance".to_string()],
    });
    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );
    assert_eq!(counters.attack_entity_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec!["minecraft:vanilla".to_string()],
    });
    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );
    assert_eq!(counters.attack_entity_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::AttackEntity(_)
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
}

#[test]
fn closing_a_screen_allows_the_next_attack_when_the_button_is_up() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = local_attack_world_with_player_and_target(123);
    assert_eq!(
        world.apply_remove_entities(RemoveEntities {
            entity_ids: vec![123],
        }),
        1
    );
    world.advance_client_level_entity_ticks_with_observation(6);
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );
    assert_eq!(input.attack_miss_time_ticks, 10);
    assert_eq!(world.local_player_attack_strength_ticks(), 0);
    assert_eq!(world.local_player_item_swap_ticks(), 0);
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
        ElementState::Pressed,
    );
    assert_eq!(counters.swing_commands_queued, 1);
    assert!(rx.try_recv().is_err());

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &None,
        MouseButton::Left,
        ElementState::Released,
    );
    input.open_debug_pause_screen_with_menu(&world, &mut Default::default(), &None);
    crate::input::advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &None,
        std::time::Instant::now(),
        5,
    );
    assert_eq!(input.attack_miss_time_ticks, 10_000);
    input.close_debug_pause_screen();
    crate::input::advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &None,
        std::time::Instant::now(),
        5,
    );
    assert_eq!(input.attack_miss_time_ticks, 0);

    world.apply_game_event(ProtocolGameEvent {
        event_id: 3,
        param: 1.0,
    });
    assert!(world.local_player_is_creative());
    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );
    assert_eq!(input.attack_miss_time_ticks, 0);
    assert_eq!(counters.swing_commands_queued, 2);
}

#[test]
fn local_attack_boat_directional_input_blocks_before_spectator_or_item_routing() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut world = local_attack_world_with_player_and_target(123);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 50,
        passenger_ids: vec![42],
    }));
    assert_eq!(
        world.local_player_first_passenger_root_boat_vehicle_id(),
        Some(50)
    );
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );

    assert_eq!(counters.attack_entity_commands_queued, 0);
    assert_eq!(counters.spectate_entity_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn right_mouse_press_boat_directional_input_blocks_item_use() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut world = local_attack_world_with_player_and_target(123);
    world.apply_remove_entities(RemoveEntities {
        entity_ids: vec![123],
    });
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 50,
        passenger_ids: vec![42],
    }));
    install_consumable(&mut world, 0, 42, 1.6);
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
    );

    assert_eq!(
        input.use_item_repeat_delay_ticks,
        USE_ITEM_REPEAT_DELAY_TICKS
    );
    assert!(!world.local_player().interaction.using_item);
    assert_eq!(counters.use_item_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn local_attack_piercing_weapon_resets_only_attack_strength() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = local_attack_world_with_player_and_target(123);
    set_selected_piercing_weapon(&mut world, 42);
    world.advance_client_level_entity_ticks_with_observation(6);
    assert_eq!(world.local_player_attack_strength_ticks(), 5);
    assert_eq!(world.local_player_item_swap_ticks(), 5);
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );

    assert_eq!(world.local_player_attack_strength_ticks(), 0);
    assert_eq!(world.local_player_item_swap_ticks(), 5);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::Stab,
            ..
        })
    ));
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
fn repeated_survival_press_on_same_target_keeps_prediction_and_only_swings_again() {
    let (tx, mut rx) = mpsc::channel(3);
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
    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );

    assert_eq!(world.local_prediction_sequence(), 1);
    assert_eq!(
        world.local_player().interaction.destroying_block,
        Some(CROSSHAIR_BLOCK_POS)
    );
    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 2);
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            sequence: 1,
            ..
        }))
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
}

#[test]
fn adventure_mode_block_destroy_requires_matching_can_break_predicate() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    set_local_adventure(&mut world);
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(1, 1),
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

    assert_eq!(world.local_player().interaction.destroying_block, None);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );

    input.destroy_block_held = false;
    set_selected_can_break(&mut world, "minecraft:grass_block");
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
    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 2);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            ..
        })
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn survival_instant_left_mouse_press_predicts_destroy_without_delay() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    assert!(world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
        block_state_id: test_block_state_id("minecraft:slime_block", &[]),
    }));
    assert!(world.local_destroy_block_is_immediate(CROSSHAIR_BLOCK_POS));
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
    assert_eq!(
        world.local_player().interaction.destroying_block_stage,
        None
    );
    assert_eq!(world.local_player().interaction.destroy_delay_ticks, 0);
    assert_eq!(
        world
            .probe_block(CROSSHAIR_BLOCK_POS)
            .unwrap()
            .block_state_id,
        bbb_client_world::vanilla_air_block_state_id()
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
fn survival_start_runs_redstone_attack_prediction_before_start_command() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    let unlit = test_block_state_id("minecraft:redstone_ore", &[("lit", "false")]);
    let lit = test_block_state_id("minecraft:redstone_ore", &[("lit", "true")]);
    assert!(world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
        block_state_id: unlit,
    }));
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
        world
            .probe_block(CROSSHAIR_BLOCK_POS)
            .unwrap()
            .block_state_id,
        lit
    );
    assert_eq!(world.local_block_predictions().len(), 1);
    assert_eq!(world.local_block_predictions()[0].sequence, 1);
    assert_eq!(
        world.local_block_predictions()[0].server_block_state_id,
        unlit
    );
    assert_eq!(
        world.local_block_predictions()[0].predicted_block_state_id,
        lit
    );
    assert_eq!(
        input.pending_local_player_action_particles,
        [
            crate::particle_runtime::LocalPlayerActionParticleState::BlockAttack(
                bbb_client_world::LocalBlockAttackEffect::RedstoneOre {
                    pos: CROSSHAIR_BLOCK_POS,
                },
            )
        ]
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
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );

    advance_destroying_block_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        1.0,
        1,
    );

    assert_eq!(input.pending_local_player_action_particles.len(), 2);
    assert!(matches!(
        input.pending_local_player_action_particles[0],
        crate::particle_runtime::LocalPlayerActionParticleState::BlockAttack(_)
    ));
    assert!(matches!(
        input.pending_local_player_action_particles[1],
        crate::particle_runtime::LocalPlayerActionParticleState::DestroyHit(_)
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn right_click_lever_queues_vanilla_client_particle_and_swing() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    let lever = test_block_state_id(
        "minecraft:lever",
        &[
            ("face", "ceiling"),
            ("facing", "east"),
            ("powered", "false"),
        ],
    );
    assert!(world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
        block_state_id: lever,
    }));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_STONE_ID, 1),
    });
    world.apply_player_abilities(PlayerAbilities {
        invulnerable: false,
        flying: false,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    let mut counters = NetCounters::default();
    let mut first_person_hand_presentation =
        FirstPersonHandPresentation::fully_raised_for_test(&world);
    let mut random = LevelEventSoundRandomState::default();
    let mut audio_events = None;

    let player_pose = world.local_player_pose();
    assert!(start_use_item_with_effects(
        &mut input,
        &mut first_person_hand_presentation,
        &mut world,
        &mut counters,
        &commands,
        Some(CrosshairTarget::Block(CrosshairBlockHit {
            pos: CROSSHAIR_BLOCK_POS,
            face: ProtocolDirection::North,
            cursor: [0.5; 3],
            inside: false,
        })),
        player_pose,
        &mut random,
        &mut audio_events,
    ));

    assert_eq!(
        input.pending_local_player_action_particles,
        [
            crate::particle_runtime::LocalPlayerActionParticleState::Interaction(
                bbb_client_world::LocalInteractionParticleAction::LeverActivation {
                    pos: CROSSHAIR_BLOCK_POS,
                    facing: ProtocolDirection::East,
                    face: bbb_client_world::LocalLeverAttachFace::Ceiling,
                },
            )
        ]
    );
    assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItemOn(_))));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert_eq!(counters.use_item_on_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    assert!(rx.try_recv().is_err());
    assert_eq!(
        first_person_hand_presentation.interpolated_inverse_height(
            InteractionHand::MainHand,
            None,
            1.0,
        ),
        1.0,
        "creative block success invokes ItemInHandRenderer.itemUsed",
    );
}

#[test]
fn left_mouse_press_with_piercing_weapon_queues_stab_and_swing_on_block() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    set_selected_piercing_weapon(&mut world, 42);
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );

    assert!(!input.destroy_block_held);
    assert_eq!(world.local_player().interaction.destroying_block, None);
    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::Stab,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
}

#[test]
fn left_mouse_press_on_block_outside_world_border_swings_without_destroying() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    set_world_border_excluding_crosshair_block(&mut world);
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
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn survival_same_target_continues_after_world_border_moves_past_block() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.destroy_block_held = true;
    let mut world = world_with_crosshair_block();
    world.set_local_destroying_block_hit(CROSSHAIR_BLOCK_POS, ProtocolDirection::North);
    set_world_border_excluding_crosshair_block(&mut world);
    let mut counters = NetCounters::default();

    advance_destroying_block_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        1.0,
        1,
    );

    assert_eq!(
        world.local_player().interaction.destroying_block,
        Some(CROSSHAIR_BLOCK_POS)
    );
    assert!(world.local_player().interaction.destroying_block_progress > 0);
    assert_eq!(world.local_player().interaction.destroying_block_ticks, 1);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
}

#[test]
fn survival_same_target_air_update_clears_without_hit_effect_or_swing() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.destroy_block_held = true;
    let mut world = world_with_crosshair_block();
    world.set_local_destroying_block_hit(CROSSHAIR_BLOCK_POS, ProtocolDirection::North);
    assert!(world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
        block_state_id: bbb_client_world::vanilla_air_block_state_id(),
    }));
    let mut counters = NetCounters::default();

    let mut audio_events = None;
    assert!(!continue_destroy_block_tick(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        &mut audio_events,
        CrosshairBlockHit {
            pos: CROSSHAIR_BLOCK_POS,
            face: ProtocolDirection::North,
            cursor: [0.5; 3],
            inside: false,
        },
    ));

    assert_eq!(world.local_player().interaction.destroying_block, None);
    assert!(input.pending_local_player_action_particles.is_empty());
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn full_or_closed_start_queue_keeps_prediction_and_destroy_state_unchanged() {
    for closed in [false, true] {
        let (tx, rx) = mpsc::channel(1);
        let mut rx = Some(rx);
        if closed {
            rx.take();
        } else {
            tx.try_send(NetCommand::Swing(InteractionHand::OffHand))
                .unwrap();
        }
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

        assert!(input.destroy_block_held);
        assert_eq!(world.local_prediction_sequence(), 0);
        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert!(world.local_block_predictions().is_empty());
        assert!(input.pending_local_player_action_particles.is_empty());
        assert_eq!(counters.player_action_commands_queued, 0);
        assert_eq!(counters.swing_commands_queued, 0);
        if let Some(rx) = rx.as_mut() {
            assert_eq!(
                rx.try_recv().unwrap(),
                NetCommand::Swing(InteractionHand::OffHand)
            );
        }
    }
}

#[test]
fn spectator_left_mouse_press_on_block_does_not_start_destroy_or_swing() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    set_local_spectator(&mut world);
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );

    assert!(!input.destroy_block_held);
    assert_eq!(world.local_player().interaction.destroying_block, None);
    assert_eq!(counters.spectate_entity_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn left_mouse_press_while_using_item_does_not_start_destroy() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    world.set_local_using_item(true);
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );

    assert!(!input.destroy_block_held);
    assert_eq!(world.local_player().interaction.destroying_block, None);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 0);
    assert!(rx.try_recv().is_err());
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
        None,
        1.0,
        0,
    );

    assert_eq!(
        world.local_player().interaction.destroying_block,
        Some(old_pos)
    );
    assert_eq!(counters.player_action_commands_queued, 0);

    advance_destroying_block_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
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
fn retarget_reserves_abort_and_start_before_changing_the_old_target() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetCommand::Swing(InteractionHand::OffHand))
        .unwrap();
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
        None,
        1.0,
        1,
    );

    assert_eq!(
        world.local_player().interaction.destroying_block,
        Some(old_pos)
    );
    assert_eq!(
        world.local_player().interaction.destroying_block_face,
        Some(ProtocolDirection::South)
    );
    assert_eq!(world.local_prediction_sequence(), 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::OffHand)
    );
}

#[test]
fn held_left_mouse_retarget_to_block_outside_world_border_keeps_current_destroy_state() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.destroy_block_held = true;
    let mut world = world_with_crosshair_block();
    set_world_border_excluding_crosshair_block(&mut world);
    let old_pos = BlockPos { x: 2, y: 65, z: -3 };
    world.set_local_destroying_block_hit(old_pos, ProtocolDirection::South);
    let mut counters = NetCounters::default();

    advance_destroying_block_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        1.0,
        1,
    );

    assert_eq!(
        world.local_player().interaction.destroying_block,
        Some(old_pos)
    );
    assert_eq!(
        world.local_player().interaction.destroying_block_face,
        Some(ProtocolDirection::South)
    );
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn held_left_mouse_while_using_item_does_not_continue_or_abort_destroy() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.destroy_block_held = true;
    let mut world = world_with_crosshair_block();
    let old_pos = BlockPos { x: 2, y: 65, z: -3 };
    world.set_local_destroying_block_hit(old_pos, ProtocolDirection::South);
    world.set_local_using_item(true);
    let mut counters = NetCounters::default();

    advance_destroying_block_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        1.0,
        1,
    );

    assert!(input.destroy_block_held);
    assert!(world.local_player().interaction.using_item);
    assert_eq!(
        world.local_player().interaction.destroying_block,
        Some(old_pos)
    );
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn held_left_mouse_with_piercing_weapon_does_not_continue_or_abort_destroy() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.destroy_block_held = true;
    let mut world = world_with_crosshair_block();
    let old_pos = BlockPos { x: 2, y: 65, z: -3 };
    world.set_local_destroying_block_hit(old_pos, ProtocolDirection::South);
    set_selected_piercing_weapon(&mut world, 42);
    let mut counters = NetCounters::default();

    advance_destroying_block_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        1.0,
        1,
    );

    assert!(input.destroy_block_held);
    assert_eq!(
        world.local_player().interaction.destroying_block,
        Some(old_pos)
    );
    assert_eq!(
        world.local_player().interaction.destroying_block_face,
        Some(ProtocolDirection::South)
    );
    assert_eq!(world.local_player().interaction.destroying_block_ticks, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Released,
    );

    assert!(!input.destroy_block_held);
    assert_eq!(
        world.local_player().interaction.destroying_block,
        Some(old_pos)
    );
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn held_left_mouse_same_target_restarts_destroy_when_main_hand_item_changes() {
    let (tx, mut rx) = mpsc::channel(3);
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
        None,
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
    assert_eq!(
        world.local_player().interaction.destroying_block_stage,
        None
    );
    assert_eq!(world.local_player().interaction.destroying_block_ticks, 0);
    assert_eq!(counters.player_action_commands_queued, 2);
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(input.pending_local_player_action_particles.len(), 1);
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
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn held_left_mouse_miss_aborts_destroying_block() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.destroy_block_held = true;
    let mut world = WorldStore::new();
    world.set_local_destroying_block_hit(BlockPos { x: 2, y: 65, z: -3 }, ProtocolDirection::South);
    let mut counters = NetCounters::default();

    advance_destroying_block_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        1.0,
        1,
    );

    assert!(input.destroy_block_held);
    assert_eq!(world.local_player().interaction.destroying_block, None);
    assert_eq!(world.local_player().interaction.destroying_block_face, None);
    assert_eq!(
        world.local_player().interaction.destroying_block_stage,
        None
    );
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
    let (tx, mut rx) = mpsc::channel(20);
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
        None,
        1.0,
        18,
    );

    assert_eq!(world.local_player().interaction.destroying_block, None);
    assert_eq!(
        world.local_player().interaction.destroying_block_progress,
        0
    );
    assert_eq!(
        world.local_player().interaction.destroying_block_stage,
        None
    );
    assert_eq!(world.local_player().interaction.destroying_block_ticks, 0);
    assert_eq!(world.local_player().interaction.destroy_delay_ticks, 5);
    assert_eq!(
        world
            .probe_block(CROSSHAIR_BLOCK_POS)
            .unwrap()
            .block_state_id,
        bbb_client_world::vanilla_air_block_state_id()
    );
    assert_eq!(world.local_block_predictions().len(), 1);
    assert_eq!(world.local_block_predictions()[0].sequence, 1);
    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 18);
    for _ in 0..17 {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
    }
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::StopDestroyBlock,
            pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
            direction: ProtocolDirection::North,
            sequence: 1,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(rx.try_recv().is_err());
    assert_eq!(input.pending_local_player_action_particles.len(), 17);
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
        None,
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
    assert_eq!(
        world.local_player().interaction.destroying_block_stage,
        None
    );
    assert_eq!(world.local_player().interaction.destroying_block_ticks, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn completion_and_abort_retry_without_publishing_partial_state_when_queue_is_full() {
    let (tx, mut rx) = mpsc::channel(18);
    let commands = Some(tx.clone());
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
        None,
        1.0,
        17,
    );
    assert_eq!(world.local_player().interaction.destroying_block_ticks, 17);
    tx.try_send(NetCommand::Swing(InteractionHand::OffHand))
        .unwrap();

    advance_destroying_block_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        1.0,
        1,
    );
    assert_eq!(
        world.local_player().interaction.destroying_block,
        Some(CROSSHAIR_BLOCK_POS)
    );
    assert_eq!(world.local_player().interaction.destroying_block_ticks, 17);
    assert_eq!(world.local_prediction_sequence(), 0);
    while rx.try_recv().is_ok() {}

    advance_destroying_block_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        1.0,
        1,
    );
    assert_eq!(world.local_player().interaction.destroying_block, None);
    assert_eq!(world.local_prediction_sequence(), 1);
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::StopDestroyBlock,
            sequence: 1,
            ..
        }))
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );

    world.set_local_destroying_block_hit(CROSSHAIR_BLOCK_POS, ProtocolDirection::North);
    input.destroy_block_held = false;
    for _ in 0..18 {
        tx.try_send(NetCommand::Swing(InteractionHand::OffHand))
            .unwrap();
    }
    advance_destroying_block_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        1.0,
        1,
    );
    assert_eq!(
        world.local_player().interaction.destroying_block,
        Some(CROSSHAIR_BLOCK_POS)
    );
    while rx.try_recv().is_ok() {}
    advance_destroying_block_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        1.0,
        1,
    );
    assert_eq!(world.local_player().interaction.destroying_block, None);
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::AbortDestroyBlock,
            sequence: 0,
            ..
        }))
    ));
}

#[test]
fn held_left_mouse_same_target_updates_local_destroy_stage_on_client_tick() {
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
        None,
        1.0,
        1,
    );

    assert_eq!(
        world.local_player().interaction.destroying_block,
        Some(CROSSHAIR_BLOCK_POS)
    );
    assert!(world.local_player().interaction.destroying_block_progress > 0);
    assert_eq!(
        world.local_player().interaction.destroying_block_stage,
        Some(0)
    );
    assert_eq!(world.local_player().interaction.destroying_block_ticks, 1);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(input.pending_local_player_action_particles.len(), 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn held_left_mouse_same_target_emits_vanilla_block_hit_sound_every_four_ticks() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.destroy_block_held = true;
    let mut world = world_with_crosshair_block();
    world.set_local_destroying_block_hit(CROSSHAIR_BLOCK_POS, ProtocolDirection::North);
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::default();

    advance_destroying_block_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(&mut audio),
        1.0,
        1,
    );
    advance_destroying_block_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(&mut audio),
        1.0,
        1,
    );

    assert_eq!(audio.positioned_sounds.len(), 1);
    let sound = &audio.positioned_sounds[0];
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:block.grass.hit")
    );
    assert_eq!(sound.source, "block");
    assert_eq!(
        sound.position,
        ProtocolVec3d {
            x: 0.5,
            y: 1.5,
            z: 3.5,
        }
    );
    assert_eq!(sound.volume, 0.25);
    assert_eq!(sound.pitch, 0.5);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 2);
    assert_eq!(input.pending_local_player_action_particles.len(), 2);
    for _ in 0..2 {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
    }
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
    assert_eq!(
        world.local_player().interaction.destroying_block_stage,
        None
    );
    assert_eq!(world.local_player().interaction.destroy_delay_ticks, 5);
    assert_eq!(
        world
            .probe_block(CROSSHAIR_BLOCK_POS)
            .unwrap()
            .block_state_id,
        bbb_client_world::vanilla_air_block_state_id()
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
fn creative_game_master_prediction_requires_permission_but_start_still_sends() {
    for permitted in [false, true] {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_command_block();
        if !permitted {
            assert!(world.apply_entity_event(ProtocolEntityEvent {
                entity_id: 42,
                event_id: 24,
            }));
        }
        let initial_state = world
            .probe_block(CROSSHAIR_BLOCK_POS)
            .expect("command block")
            .block_state_id;
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
        assert_eq!(world.local_player().interaction.destroy_delay_ticks, 5);
        assert_eq!(
            world.local_block_predictions().len(),
            usize::from(permitted)
        );
        assert_eq!(
            world
                .probe_block(CROSSHAIR_BLOCK_POS)
                .expect("predicted command block cell")
                .block_state_id,
            if permitted {
                bbb_client_world::vanilla_air_block_state_id()
            } else {
                initial_state
            }
        );
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        assert!(matches!(
            rx.try_recv(),
            Ok(NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                sequence: 1,
                ..
            }))
        ));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
    }
}

#[test]
fn creative_tool_flag_suppresses_local_destroy_but_still_queues_start() {
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
    let mut stack = item_stack(42, 1);
    stack.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_TOOL_TYPE_ID];
    stack.component_patch.tool = Some(ToolSummary {
        rules: Vec::new(),
        default_mining_speed_bits: 1.0_f32.to_bits(),
        damage_per_block: 2,
        can_destroy_blocks_in_creative: false,
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
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
    assert_eq!(
        world
            .probe_block(CROSSHAIR_BLOCK_POS)
            .unwrap()
            .block_state_id,
        VANILLA_GRASS_BLOCK_STATE_ID
    );
    assert!(world.local_block_predictions().is_empty());
    assert_eq!(world.local_player().interaction.destroy_delay_ticks, 5);
    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            ..
        })
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn creative_debug_stick_sends_start_without_local_destroy_prediction() {
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
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_DEBUG_STICK_ID, 1),
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
    assert_eq!(
        world
            .probe_block(CROSSHAIR_BLOCK_POS)
            .unwrap()
            .block_state_id,
        VANILLA_GRASS_BLOCK_STATE_ID
    );
    assert!(world.local_block_predictions().is_empty());
    assert_eq!(world.local_player().interaction.destroy_delay_ticks, 5);
    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            ..
        })
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(rx.try_recv().is_err());
}
