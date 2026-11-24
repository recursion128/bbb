use super::*;

#[test]
fn block_success_item_used_requires_client_swing_and_nonempty_count_change() {
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_STONE_ID, 1),
    });
    let pre_use_stack = item_stack(VANILLA_ITEM_STONE_ID, 2);
    let success = LocalInteractionSuccess {
        swing_source: LocalInteractionSwingSource::Client,
        was_item_interaction: true,
        starts_using_item: false,
        extra_swing: None,
        sound_action: None,
        particle_action: None,
    };

    assert!(block_success_uses_first_person_item(
        Some(&pre_use_stack),
        &world,
        InteractionHand::MainHand,
        success,
    ));
    assert!(!block_success_uses_first_person_item(
        world.local_item_in_hand(InteractionHand::MainHand),
        &world,
        InteractionHand::MainHand,
        success,
    ));
    assert!(!block_success_uses_first_person_item(
        Some(&pre_use_stack),
        &world,
        InteractionHand::MainHand,
        LocalInteractionSuccess {
            swing_source: LocalInteractionSwingSource::Server,
            ..success
        },
    ));
    assert!(!block_success_uses_first_person_item(
        None,
        &world,
        InteractionHand::MainHand,
        success,
    ));
}

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
fn sign_editor_mouse_press_does_not_queue_gameplay_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_open_sign_editor(&mut input);
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
    assert_eq!(counters.swing_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.attack_entity_commands_queued, 0);
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
    assert!(world.set_local_selected_hotbar_slot(4));
    install_consumable(&mut world, 4, 42, 1.6);
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
        NetCommand::UseItem(UseItemCommand {
            packet: UseItem {
                hand: InteractionHand::MainHand,
                sequence: 1,
                y_rot: 45.0,
                x_rot: -20.0,
            },
            selected_hotbar_slot: 4,
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
fn unavailable_item_use_command_keeps_sequence_and_world_unchanged() {
    let assert_noop = |commands: &Option<mpsc::Sender<NetCommand>>| {
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState::default());
        install_consumable(&mut world, 0, 42, 1.6);
        let item_before = world.local_item_in_hand(InteractionHand::MainHand).cloned();
        let mut counters = NetCounters::default();
        let player_pose = world.local_player_pose();

        assert!(start_use_item(
            &mut input,
            &mut world,
            &mut counters,
            commands,
            None,
            player_pose,
        ));

        assert_eq!(world.local_prediction_sequence(), 0);
        assert_eq!(
            world.local_item_in_hand(InteractionHand::MainHand),
            item_before.as_ref()
        );
        assert!(!world.local_player().interaction.using_item);
        assert!(world.local_block_predictions().is_empty());
        assert!(input.pending_local_player_action_particles.is_empty());
        assert_eq!(counters.use_item_commands_queued, 0);
    };

    assert_noop(&None);

    let (full_tx, mut full_rx) = mpsc::channel(1);
    full_tx
        .try_send(NetCommand::Swing(InteractionHand::OffHand))
        .unwrap();
    assert_noop(&Some(full_tx));
    assert_eq!(
        full_rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::OffHand)
    );

    let (closed_tx, closed_rx) = mpsc::channel(1);
    drop(closed_rx);
    assert_noop(&Some(closed_tx));
}

#[test]
fn unavailable_block_use_command_keeps_prediction_and_item_unchanged() {
    let assert_noop = |commands: &Option<mpsc::Sender<NetCommand>>| {
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_lodestone();
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(VANILLA_ITEM_COMPASS_ID, 1),
        });
        let item_before = world.local_item_in_hand(InteractionHand::MainHand).cloned();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert_eq!(world.local_prediction_sequence(), 0);
        assert_eq!(
            world.local_item_in_hand(InteractionHand::MainHand),
            item_before.as_ref()
        );
        assert!(world.local_block_predictions().is_empty());
        assert!(input.pending_local_player_action_particles.is_empty());
        assert_eq!(counters.use_item_on_commands_queued, 0);
        assert_eq!(counters.swing_commands_queued, 0);
    };

    assert_noop(&None);

    let (full_tx, mut full_rx) = mpsc::channel(1);
    full_tx
        .try_send(NetCommand::Swing(InteractionHand::OffHand))
        .unwrap();
    assert_noop(&Some(full_tx));
    assert_eq!(
        full_rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::OffHand)
    );

    let (closed_tx, closed_rx) = mpsc::channel(1);
    drop(closed_rx);
    assert_noop(&Some(closed_tx));
}

#[test]
fn unavailable_block_use_command_does_not_open_prediction_owned_editor() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetCommand::Swing(InteractionHand::OffHand))
        .unwrap();
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_command_block();
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
    );

    assert_eq!(world.local_prediction_sequence(), 0);

    assert!(!input
        .command_block_editor_hud_state()
        .is_some_and(|state| state.block.is_some()));
    assert_eq!(counters.use_item_on_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::OffHand)
    );
}

#[test]
fn admitted_block_use_does_not_start_unadmitted_item_fallback() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_WATER_BUCKET_ID, 1),
    });
    let item_before = world.local_item_in_hand(InteractionHand::MainHand).cloned();
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
    );

    assert_eq!(world.local_prediction_sequence(), 1);
    assert_eq!(
        world.local_item_in_hand(InteractionHand::MainHand),
        item_before.as_ref()
    );
    assert!(world.local_block_predictions().is_empty());
    assert_eq!(counters.use_item_on_commands_queued, 1);
    assert_eq!(counters.use_item_commands_queued, 0);
    assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItemOn(_))));
    assert!(rx.try_recv().is_err());
}

#[test]
fn right_mouse_bucket_prediction_uses_the_queued_use_item_sequence() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_WATER_BUCKET_ID, 1),
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

    let NetCommand::UseItemOn(use_on) = rx.try_recv().unwrap() else {
        panic!("expected initial use-item-on packet");
    };
    assert_eq!(use_on.packet.sequence, 1);
    let NetCommand::UseItem(use_item) = rx.try_recv().unwrap() else {
        panic!("expected bucket use-item packet");
    };
    assert_eq!(use_item.packet.sequence, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert_eq!(world.local_block_predictions().len(), 1);
    assert_eq!(world.local_block_predictions()[0].sequence, 2);
    assert_eq!(
        world.local_block_predictions()[0].pos,
        BlockPos { x: 0, y: 1, z: 2 }
    );
    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_BUCKET_ID)
    );
    assert_eq!(counters.use_item_on_commands_queued, 1);
    assert_eq!(counters.use_item_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
}

#[test]
fn right_mouse_instrument_use_starts_main_hand_cooldown_and_entity_sound() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.apply_login(&PlayLogin {
        player_id: 42,
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
        enforces_secure_chat: true,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 1.25,
            y: 64.0,
            z: -3.5,
        },
        y_rot: 45.0,
        x_rot: -20.0,
        ..LocalPlayerPoseState::default()
    });
    let mut horn = item_stack(VANILLA_ITEM_GOAT_HORN_ID, 1);
    horn.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID];
    horn.component_patch.instrument.direct = Some(Box::new(InstrumentSummary {
        sound_event: bbb_protocol::packets::SoundEventSummary {
            registry_id: None,
            sound_id: Some("example:test_horn".to_string()),
            fixed_range_bits: Some(24.0_f32.to_bits()),
        },
        use_duration_bits: 3.25_f32.to_bits(),
        range_bits: 96.0_f32.to_bits(),
        description: "Test horn".to_string(),
        description_styled: None,
        description_nbt: None,
    }));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: horn,
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(VANILLA_ITEM_STONE_ID, 1),
    });
    let mut expected_seed_world = WorldStore::new();
    let expected_seed = expected_seed_world.next_sound_seed();
    let mut counters = NetCounters::default();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut audio = RecordingAudioSink::default();
    let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

    handle_mouse_input_at_partial_tick_with_effects(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
        1.0,
        &mut random,
        &mut audio_events,
    );

    assert!(world.local_player().interaction.using_item);
    assert_eq!(
        world.local_player().interaction.using_item_hand,
        Some(InteractionHand::MainHand)
    );
    assert_eq!(
        world
            .cooldown("minecraft:goat_horn")
            .map(|cooldown| cooldown.duration),
        Some(65)
    );
    assert_eq!(counters.use_item_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::UseItem(UseItemCommand {
            packet: UseItem {
                hand: InteractionHand::MainHand,
                sequence: 1,
                y_rot: 45.0,
                x_rot: -20.0,
            },
            selected_hotbar_slot: 0,
        })
    );
    assert!(rx.try_recv().is_err());
    assert!(audio.positioned_sounds.is_empty());
    assert_eq!(audio.entity_sounds.len(), 1);
    let (sound, position) = &audio.entity_sounds[0];
    assert_eq!(sound.sound.kind, "direct");
    assert_eq!(sound.sound.location.as_deref(), Some("example:test_horn"));
    assert_eq!(sound.sound.fixed_range, Some(24.0));
    assert_eq!(sound.source, "record");
    assert_eq!(sound.entity_id, 42);
    assert_eq!(sound.volume, 6.0);
    assert_eq!(sound.pitch, 1.0);
    assert_eq!(sound.seed, expected_seed);
    assert_eq!(*position, Some([1.25, 64.0, -3.5]));
    assert_eq!(world.last_sound_entity(), Some(sound));
}

#[test]
fn right_mouse_equippable_swap_mutates_inventory_and_stops_before_offhand() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState {
        y_rot: 45.0,
        x_rot: -20.0,
        ..LocalPlayerPoseState::default()
    });
    let sound = || SoundEventSummary {
        registry_id: Some(0),
        sound_id: None,
        fixed_range_bits: None,
    };
    let mut held = item_stack(VANILLA_ITEM_STONE_ID, 2);
    held.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID];
    held.component_patch.equippable = Some(EquippableSummary {
        slot_id: VANILLA_EQUIPMENT_SLOT_CHEST_ID,
        equip_sound: sound(),
        asset_id: None,
        camera_overlay: None,
        allowed_entities: None,
        dispensable: true,
        swappable: true,
        damage_on_hurt: true,
        equip_on_interact: false,
        can_be_sheared: false,
        shearing_sound: sound(),
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: held,
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
        item: item_stack(VANILLA_ITEM_SHIELD_ID, 1),
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(VANILLA_ITEM_SHIELD_ID, 1),
    });
    crate::item_registry_test_fixture::install_max_stack_size_default(
        &mut world,
        VANILLA_ITEM_SHIELD_ID,
        1,
    );
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
    );

    assert_eq!(counters.use_item_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::UseItem(UseItemCommand {
            packet: UseItem {
                hand: InteractionHand::MainHand,
                sequence: 1,
                y_rot: 45.0,
                x_rot: -20.0,
            },
            selected_hotbar_slot: 0,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(rx.try_recv().is_err());
    assert_eq!(
        world
            .inventory()
            .player_slots
            .iter()
            .find(|slot| slot.slot == 0)
            .unwrap()
            .item
            .count,
        1
    );
    assert_eq!(
        world
            .inventory()
            .player_slots
            .iter()
            .find(|slot| slot.slot == 1)
            .unwrap()
            .item
            .item_id,
        Some(VANILLA_ITEM_SHIELD_ID)
    );
    assert_eq!(
        world
            .inventory()
            .player_slots
            .iter()
            .find(|slot| slot.slot == VANILLA_PLAYER_INVENTORY_CHEST_SLOT)
            .unwrap()
            .item
            .item_id,
        Some(VANILLA_ITEM_STONE_ID)
    );
    assert_eq!(
        world
            .inventory()
            .player_slots
            .iter()
            .find(|slot| slot.slot == VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT)
            .unwrap()
            .item
            .item_id,
        Some(VANILLA_ITEM_SHIELD_ID)
    );
}

#[test]
fn right_mouse_projectile_throw_consumes_cools_down_and_suppresses_local_sound() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState {
        y_rot: 45.0,
        x_rot: -20.0,
        ..LocalPlayerPoseState::default()
    });
    let mut pearl = item_stack(VANILLA_ITEM_ENDER_PEARL_ID, 2);
    pearl.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID];
    pearl.component_patch.use_cooldown_seconds_bits = Some(0.35_f32.to_bits());
    pearl.component_patch.use_cooldown_group = Some("example:shared_throw".to_string());
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: pearl,
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(VANILLA_ITEM_STONE_ID, 1),
    });
    let mut counters = NetCounters::default();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut audio = RecordingAudioSink::default();
    let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

    handle_mouse_input_at_partial_tick_with_effects(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
        1.0,
        &mut random,
        &mut audio_events,
    );

    assert_eq!(counters.use_item_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::UseItem(UseItemCommand {
            packet: UseItem {
                hand: InteractionHand::MainHand,
                sequence: 1,
                y_rot: 45.0,
                x_rot: -20.0,
            },
            selected_hotbar_slot: 0,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(rx.try_recv().is_err());
    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .count,
        1
    );
    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::OffHand)
            .unwrap()
            .item_id,
        Some(VANILLA_ITEM_STONE_ID)
    );
    assert_eq!(
        world
            .cooldown("example:shared_throw")
            .map(|cooldown| cooldown.duration),
        Some(7)
    );
    assert!(audio.positioned_sounds.is_empty());
    assert!(audio.entity_sounds.is_empty());
    assert!(world.client_audio().last_sound.is_none());
}

#[test]
fn right_mouse_bow_use_and_release_follow_ammo_threshold_and_suppressed_sound() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState {
        y_rot: 20.0,
        x_rot: -10.0,
        ..LocalPlayerPoseState::default()
    });
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:arrows".to_string(),
                entries: vec![VANILLA_ITEM_ARROW_ID],
            }],
        }],
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_BOW_ID, 1),
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(VANILLA_ITEM_ARROW_ID, 4),
    });
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::default();
    let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

    handle_mouse_input_at_partial_tick_with_effects(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
        1.0,
        &mut random,
        &mut audio_events,
    );
    assert!(world.local_player().interaction.using_item);
    assert_eq!(
        world.local_player().interaction.using_item_hand,
        Some(InteractionHand::MainHand)
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::UseItem(UseItemCommand {
            packet: UseItem {
                hand: InteractionHand::MainHand,
                sequence: 1,
                y_rot: 20.0,
                x_rot: -10.0,
            },
            selected_hotbar_slot: 0,
        })
    );

    world.advance_local_using_item_ticks(3);
    handle_mouse_input_at_partial_tick_with_effects(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Released,
        1.0,
        &mut random,
        &mut audio_events,
    );

    assert!(!world.local_player().interaction.using_item);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::ReleaseUseItem,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
    assert!(rx.try_recv().is_err());
    assert_eq!(counters.use_item_commands_queued, 1);
    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 0);
    assert_eq!(
        world
            .inventory()
            .player_slots
            .iter()
            .find(|slot| slot.slot == 9)
            .map(|slot| slot.item.count),
        Some(4)
    );
    assert!(audio.positioned_sounds.is_empty());
    assert!(audio.entity_sounds.is_empty());
}

#[test]
fn right_mouse_crossbow_draw_uses_held_firework_without_local_loading_side_effects() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState {
        y_rot: 20.0,
        x_rot: -10.0,
        ..LocalPlayerPoseState::default()
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_CROSSBOW_ID, 1),
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(VANILLA_ITEM_FIREWORK_ROCKET_ID, 1),
    });
    let mut counters = NetCounters::default();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_seed_world = WorldStore::new();
    let expected_next_seed = expected_seed_world.next_sound_seed();
    let mut audio = RecordingAudioSink::default();
    let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

    handle_mouse_input_at_partial_tick_with_effects(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
        1.0,
        &mut random,
        &mut audio_events,
    );
    assert!(world.local_player().interaction.using_item);
    assert_eq!(
        world.local_player().interaction.using_item_hand,
        Some(InteractionHand::MainHand)
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::UseItem(UseItemCommand {
            packet: UseItem {
                hand: InteractionHand::MainHand,
                sequence: 1,
                y_rot: 20.0,
                x_rot: -10.0,
            },
            selected_hotbar_slot: 0,
        })
    );

    advance_local_using_item_ticks_with_effects(
        &mut world,
        &mut counters,
        &commands,
        30,
        &mut random,
        &mut audio_events,
    );
    assert_eq!(world.local_player().interaction.using_item_ticks, 30);
    assert!(world
        .local_item_in_hand(InteractionHand::MainHand)
        .unwrap()
        .component_patch
        .charged_projectiles_items
        .is_empty());

    handle_mouse_input_at_partial_tick_with_effects(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Released,
        1.0,
        &mut random,
        &mut audio_events,
    );

    assert!(!world.local_player().interaction.using_item);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::ReleaseUseItem,
            ..
        })
    ));
    assert!(rx.try_recv().is_err());
    assert_eq!(counters.use_item_commands_queued, 1);
    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 0);
    assert!(audio.positioned_sounds.is_empty());
    assert!(audio.entity_sounds.is_empty());
    assert_eq!(random.next_float(), expected_random.next_float());
    assert_eq!(world.next_sound_seed(), expected_next_seed);
}

#[test]
fn charged_crossbow_use_stays_server_owned_without_starting_or_swinging() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState::default());
    let mut crossbow = item_stack(VANILLA_ITEM_CROSSBOW_ID, 1);
    crossbow
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID);
    crossbow
        .component_patch
        .charged_projectiles_items
        .push(ItemStackTemplateSummary {
            item_id: VANILLA_ITEM_ARROW_ID,
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: crossbow.clone(),
    });
    let mut counters = NetCounters::default();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_seed_world = WorldStore::new();
    let expected_next_seed = expected_seed_world.next_sound_seed();
    let mut audio = RecordingAudioSink::default();
    let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

    handle_mouse_input_at_partial_tick_with_effects(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
        1.0,
        &mut random,
        &mut audio_events,
    );

    assert!(matches!(rx.try_recv().unwrap(), NetCommand::UseItem(_)));
    assert!(rx.try_recv().is_err());
    assert!(!world.local_player().interaction.using_item);
    assert_eq!(
        world.local_item_in_hand(InteractionHand::MainHand),
        Some(&crossbow)
    );
    assert_eq!(counters.use_item_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 0);
    assert!(audio.positioned_sounds.is_empty());
    assert!(audio.entity_sounds.is_empty());
    assert_eq!(random.next_float(), expected_random.next_float());
    assert_eq!(world.next_sound_seed(), expected_next_seed);
}

#[test]
fn fishing_rod_cast_and_retrieve_swing_without_local_bobber_or_sound() {
    for retrieving in [false, true] {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.apply_login(&PlayLogin {
            player_id: 42,
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
            enforces_secure_chat: true,
        });
        world.set_local_player_pose(LocalPlayerPoseState {
            y_rot: 20.0,
            x_rot: -10.0,
            ..LocalPlayerPoseState::default()
        });
        let rod = item_stack(VANILLA_ITEM_FISHING_ROD_ID, 1);
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: rod.clone(),
        });
        if retrieving {
            world.apply_add_entity(AddEntity {
                id: 88,
                uuid: Uuid::from_u128(88),
                entity_type_id: VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
                position: ProtocolVec3d::default(),
                delta_movement: ProtocolVec3d::default(),
                x_rot: 0.0,
                y_rot: 0.0,
                y_head_rot: 0.0,
                data: 42,
            });
        }
        assert_eq!(
            world.local_player_fishing_bobber_id(),
            retrieving.then_some(88)
        );
        let mut counters = NetCounters::default();
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut audio = RecordingAudioSink::default();
        let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

        handle_mouse_input_at_partial_tick_with_effects(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
            1.0,
            &mut random,
            &mut audio_events,
        );

        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItem(UseItemCommand {
                packet: UseItem {
                    hand: InteractionHand::MainHand,
                    sequence: 1,
                    y_rot: 20.0,
                    x_rot: -10.0,
                },
                selected_hotbar_slot: 0,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert!(rx.try_recv().is_err());
        assert!(!world.local_player().interaction.using_item);
        assert_eq!(
            world.local_item_in_hand(InteractionHand::MainHand),
            Some(&rod)
        );
        assert_eq!(
            world.local_player_fishing_bobber_id(),
            retrieving.then_some(88)
        );
        assert_eq!(counters.use_item_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        assert!(audio.positioned_sounds.is_empty());
        assert!(audio.entity_sounds.is_empty());
        assert!(world.client_audio().last_sound.is_none());
    }
}

#[test]
fn firework_rocket_passes_on_ground_and_swings_without_local_launch_while_flying() {
    for fall_flying in [false, true] {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.apply_login(&PlayLogin {
            player_id: 42,
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
            enforces_secure_chat: true,
        });
        world.apply_add_entity(AddEntity {
            id: 42,
            uuid: Uuid::from_u128(42),
            entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            position: ProtocolVec3d::default(),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        if fall_flying {
            let accessors = vanilla_entity_metadata_entity_accessors().unwrap();
            assert!(world.apply_set_entity_data(SetEntityData {
                id: 42,
                values: vec![EntityDataValue {
                    data_id: accessors.shared_flags_data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    value: EntityDataValueKind::Byte(
                        accessors.shared_flags_default | accessors.fall_flying_mask,
                    ),
                }],
            }));
        }
        world.set_local_player_pose(LocalPlayerPoseState::default());
        let rocket = item_stack(VANILLA_ITEM_FIREWORK_ROCKET_ID, 2);
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: rocket.clone(),
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: item_stack(VANILLA_ITEM_SPYGLASS_ID, 1),
        });
        let entity_count = world.entity_count();
        let mut counters = NetCounters::default();
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut audio = RecordingAudioSink::default();
        let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

        handle_mouse_input_at_partial_tick_with_effects(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
            1.0,
            &mut random,
            &mut audio_events,
        );

        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItem(UseItemCommand {
                packet: UseItem {
                    hand: InteractionHand::MainHand,
                    sequence: 1,
                    y_rot: 0.0,
                    x_rot: 0.0,
                },
                selected_hotbar_slot: 0,
            })
        );
        if fall_flying {
            assert_eq!(
                rx.try_recv().unwrap(),
                NetCommand::Swing(InteractionHand::MainHand)
            );
        } else {
            assert_eq!(
                rx.try_recv().unwrap(),
                NetCommand::UseItem(UseItemCommand {
                    packet: UseItem {
                        hand: InteractionHand::OffHand,
                        sequence: 2,
                        y_rot: 0.0,
                        x_rot: 0.0,
                    },
                    selected_hotbar_slot: 0,
                })
            );
        }
        assert!(rx.try_recv().is_err());
        assert_eq!(
            counters.use_item_commands_queued,
            if fall_flying { 1 } else { 2 }
        );
        assert_eq!(counters.swing_commands_queued, usize::from(fall_flying));
        assert_eq!(
            world.local_player().interaction.using_item_hand,
            (!fall_flying).then_some(InteractionHand::OffHand)
        );
        assert_eq!(
            world.local_item_in_hand(InteractionHand::MainHand),
            Some(&rocket)
        );
        assert_eq!(world.entity_count(), entity_count);
        assert_eq!(audio.positioned_sounds.len(), usize::from(!fall_flying));
        assert!(audio.entity_sounds.is_empty());
    }
}

#[test]
fn empty_map_swings_without_local_conversion_sound_or_offhand_fallback() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState::default());
    let map = item_stack(VANILLA_ITEM_MAP_ID, 2);
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: map.clone(),
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(VANILLA_ITEM_SPYGLASS_ID, 1),
    });
    let mut counters = NetCounters::default();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut audio = RecordingAudioSink::default();
    let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

    handle_mouse_input_at_partial_tick_with_effects(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
        1.0,
        &mut random,
        &mut audio_events,
    );

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::UseItem(UseItemCommand {
            packet: UseItem {
                hand: InteractionHand::MainHand,
                sequence: 1,
                y_rot: 0.0,
                x_rot: 0.0,
            },
            selected_hotbar_slot: 0,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(rx.try_recv().is_err());
    assert_eq!(counters.use_item_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    assert!(!world.local_player().interaction.using_item);
    assert_eq!(
        world.local_item_in_hand(InteractionHand::MainHand),
        Some(&map)
    );
    assert!(world.map_items().is_empty());
    assert!(audio.positioned_sounds.is_empty());
    assert!(audio.entity_sounds.is_empty());
}

#[test]
fn food_on_a_stick_passes_to_offhand_without_local_durability_or_boost() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState::default());
    let on_a_stick = item_stack(VANILLA_ITEM_CARROT_ON_A_STICK_ID, 1);
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: on_a_stick.clone(),
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(VANILLA_ITEM_SPYGLASS_ID, 1),
    });
    let mut counters = NetCounters::default();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut audio = RecordingAudioSink::default();
    let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

    handle_mouse_input_at_partial_tick_with_effects(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
        1.0,
        &mut random,
        &mut audio_events,
    );

    for (sequence, hand) in [
        (1, InteractionHand::MainHand),
        (2, InteractionHand::OffHand),
    ] {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItem(UseItemCommand {
                packet: UseItem {
                    hand,
                    sequence,
                    y_rot: 0.0,
                    x_rot: 0.0,
                },
                selected_hotbar_slot: 0,
            })
        );
    }
    assert!(rx.try_recv().is_err());
    assert_eq!(counters.use_item_commands_queued, 2);
    assert_eq!(counters.swing_commands_queued, 0);
    assert_eq!(
        world.local_player().interaction.using_item_hand,
        Some(InteractionHand::OffHand)
    );
    assert_eq!(
        world.local_item_in_hand(InteractionHand::MainHand),
        Some(&on_a_stick)
    );
    assert_eq!(audio.positioned_sounds.len(), 1);
    assert!(audio.entity_sounds.is_empty());
}

#[test]
fn offhand_bundle_use_tick_drops_content_with_main_hand_swing_and_sound() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let position = ProtocolVec3d {
        x: 1.5,
        y: 64.0,
        z: -2.25,
    };
    world.set_local_player_pose(LocalPlayerPoseState {
        position,
        y_rot: 20.0,
        x_rot: -10.0,
        ..LocalPlayerPoseState::default()
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: ProtocolItemStackSummary {
            item_id: Some(VANILLA_ITEM_BUNDLE_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID],
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: VANILLA_ITEM_STONE_ID,
                    count: 3,
                    component_patch: DataComponentPatchSummary::default().into(),
                }],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    });
    let mut counters = NetCounters::default();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_pitch = 0.8 + expected_random.next_float() * 0.4;
    let mut expected_seed_world = WorldStore::new();
    let expected_sound_seed = expected_seed_world.next_sound_seed();
    let mut audio = RecordingAudioSink::default();
    let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

    handle_mouse_input_at_partial_tick_with_effects(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
        1.0,
        &mut random,
        &mut audio_events,
    );
    assert!(world.local_player().interaction.using_item);
    assert_eq!(
        world.local_player().interaction.using_item_hand,
        Some(InteractionHand::OffHand)
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::UseItem(UseItemCommand {
            packet: UseItem {
                hand: InteractionHand::OffHand,
                sequence: 1,
                y_rot: 20.0,
                x_rot: -10.0,
            },
            selected_hotbar_slot: 0,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::OffHand)
    );

    advance_local_using_item_ticks_with_effects(
        &mut world,
        &mut counters,
        &commands,
        1,
        &mut random,
        &mut audio_events,
    );

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(rx.try_recv().is_err());
    assert_eq!(counters.use_item_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 2);
    assert_eq!(world.local_player().interaction.using_item_ticks, 1);
    let bundle = world
        .local_item_in_hand(InteractionHand::OffHand)
        .expect("offhand bundle");
    assert!(bundle.component_patch.bundle_contents_items.is_empty());
    assert!(!bundle
        .component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID));
    assert_eq!(audio.positioned_sounds.len(), 1);
    assert_eq!(
        audio.positioned_sounds[0].sound.location.as_deref(),
        Some("minecraft:item.bundle.remove_one")
    );
    assert_eq!(audio.positioned_sounds[0].source, "player");
    assert_eq!(audio.positioned_sounds[0].position, position);
    assert_eq!(audio.positioned_sounds[0].volume, 0.8);
    assert_eq!(audio.positioned_sounds[0].pitch, expected_pitch);
    assert_eq!(audio.positioned_sounds[0].seed, expected_sound_seed);
    assert!(audio.entity_sounds.is_empty());
}

#[test]
fn projectile_weapon_without_arrow_falls_through_to_offhand_spyglass_use() {
    for weapon_item_id in [VANILLA_ITEM_BOW_ID, VANILLA_ITEM_CROSSBOW_ID] {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState::default());
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:arrows".to_string(),
                    entries: vec![VANILLA_ITEM_ARROW_ID],
                }],
            }],
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(weapon_item_id, 1),
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: item_stack(VANILLA_ITEM_SPYGLASS_ID, 1),
        });
        let mut counters = NetCounters::default();
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut audio = RecordingAudioSink::default();
        let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

        handle_mouse_input_at_partial_tick_with_effects(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
            1.0,
            &mut random,
            &mut audio_events,
        );

        for (sequence, hand) in [
            (1, InteractionHand::MainHand),
            (2, InteractionHand::OffHand),
        ] {
            assert_eq!(
                rx.try_recv().unwrap(),
                NetCommand::UseItem(UseItemCommand {
                    packet: UseItem {
                        hand,
                        sequence,
                        y_rot: 0.0,
                        x_rot: 0.0,
                    },
                    selected_hotbar_slot: 0,
                })
            );
        }
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.use_item_commands_queued, 2);
        assert_eq!(counters.swing_commands_queued, 0);
        assert_eq!(
            world.local_player().interaction.using_item_hand,
            Some(InteractionHand::OffHand)
        );
        assert_eq!(
            audio.positioned_sounds[0].sound.location.as_deref(),
            Some("minecraft:item.spyglass.use")
        );
    }
}

#[test]
fn knowledge_book_consumes_before_result_and_preserves_hand_fallback() {
    for (recipe_keys, expected_use_item_commands, offhand_started) in [
        (Vec::new(), 2, true),
        (vec!["minecraft:bread".to_string()], 1, false),
    ] {
        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState::default());
        let mut book = item_stack(VANILLA_ITEM_KNOWLEDGE_BOOK_ID, 1);
        book.component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID);
        book.component_patch.recipe_keys = Some(recipe_keys);
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: book,
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: item_stack(VANILLA_ITEM_SPYGLASS_ID, 1),
        });
        let mut counters = NetCounters::default();
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut audio = RecordingAudioSink::default();
        let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

        handle_mouse_input_at_partial_tick_with_effects(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
            1.0,
            &mut random,
            &mut audio_events,
        );

        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItem(UseItemCommand {
                packet: UseItem {
                    hand: InteractionHand::MainHand,
                    sequence: 1,
                    y_rot: 0.0,
                    x_rot: 0.0,
                },
                selected_hotbar_slot: 0,
            })
        );
        if offhand_started {
            assert_eq!(
                rx.try_recv().unwrap(),
                NetCommand::UseItem(UseItemCommand {
                    packet: UseItem {
                        hand: InteractionHand::OffHand,
                        sequence: 2,
                        y_rot: 0.0,
                        x_rot: 0.0,
                    },
                    selected_hotbar_slot: 0,
                })
            );
        } else {
            assert_eq!(
                rx.try_recv().unwrap(),
                NetCommand::Swing(InteractionHand::MainHand)
            );
        }
        assert!(rx.try_recv().is_err());
        assert_eq!(
            counters.use_item_commands_queued,
            expected_use_item_commands
        );
        assert_eq!(
            counters.swing_commands_queued,
            usize::from(!offhand_started)
        );
        assert!(world
            .local_item_in_hand(InteractionHand::MainHand)
            .is_none());
        assert_eq!(
            world.local_player().interaction.using_item_hand,
            offhand_started.then_some(InteractionHand::OffHand)
        );
        assert_eq!(audio.positioned_sounds.len(), usize::from(offhand_started));
    }
}

#[test]
fn right_mouse_spyglass_use_and_release_emit_ordered_local_sounds() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 1.5,
            y: 64.0,
            z: -2.25,
        },
        ..LocalPlayerPoseState::default()
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_SPYGLASS_ID, 1),
    });
    let mut counters = NetCounters::default();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut audio = RecordingAudioSink::default();
    let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

    for state in [ElementState::Pressed, ElementState::Released] {
        handle_mouse_input_at_partial_tick_with_effects(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            state,
            1.0,
            &mut random,
            &mut audio_events,
        );
    }

    assert!(!world.local_player().interaction.using_item);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::UseItem(_)));
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::ReleaseUseItem,
            ..
        })
    ));
    assert!(rx.try_recv().is_err());
    assert_eq!(
        audio
            .positioned_sounds
            .iter()
            .map(|sound| sound.sound.location.as_deref())
            .collect::<Vec<_>>(),
        vec![
            Some("minecraft:item.spyglass.use"),
            Some("minecraft:item.spyglass.stop_using"),
        ]
    );
    assert!(audio.positioned_sounds.iter().all(|sound| {
        sound.source == "player"
            && sound.position
                == ProtocolVec3d {
                    x: 1.5,
                    y: 64.0,
                    z: -2.25,
                }
            && sound.volume == 1.0
            && sound.pitch == 1.0
    }));
    assert_ne!(
        audio.positioned_sounds[0].seed,
        audio.positioned_sounds[1].seed
    );
    assert!(audio.entity_sounds.is_empty());
}

#[test]
fn right_mouse_press_without_target_uses_offhand_when_selected_hotbar_slot_is_empty() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState {
        y_rot: 45.0,
        x_rot: -20.0,
        ..LocalPlayerPoseState::default()
    });
    install_consumable(&mut world, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT, 99, 1.6);
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
        NetCommand::UseItem(UseItemCommand {
            packet: UseItem {
                hand: InteractionHand::OffHand,
                sequence: 1,
                y_rot: 45.0,
                x_rot: -20.0,
            },
            selected_hotbar_slot: 0,
        })
    );
}

#[test]
fn right_mouse_press_without_target_or_items_only_starts_repeat_delay() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState::default());
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
    );

    assert!(input.use_item_held);
    assert_eq!(
        input.use_item_repeat_delay_ticks,
        USE_ITEM_REPEAT_DELAY_TICKS
    );
    assert!(!world.local_player().interaction.using_item);
    assert_eq!(counters.use_item_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn right_mouse_press_without_target_honors_offhand_enabled_feature_gate() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState::default());
    install_consumable(&mut world, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT, 99, 1.6);
    world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec!["minecraft:trade_rebalance".to_string()],
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

    assert_eq!(
        input.use_item_repeat_delay_ticks,
        USE_ITEM_REPEAT_DELAY_TICKS
    );
    assert!(!world.local_player().interaction.using_item);
    assert_eq!(counters.use_item_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn full_food_consumable_blocks_main_hand_and_falls_through_to_offhand() {
    fn full_food_world() -> WorldStore {
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState {
            y_rot: 45.0,
            x_rot: -20.0,
            ..LocalPlayerPoseState::default()
        });
        world.set_default_item_component_codec_values(BTreeMap::from([
            (
                42,
                BTreeMap::from([
                    (
                        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
                        serde_json::json!({"consume_seconds": 1.6}),
                    ),
                    (
                        VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
                        serde_json::json!({"nutrition": 4, "saturation": 2.4}),
                    ),
                ]),
            ),
            (
                99,
                BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
                    serde_json::json!({"consume_seconds": 1.6}),
                )]),
            ),
        ]));
        world.rebuild_item_component_defaults().unwrap();
        world.apply_player_health(PlayerHealth {
            health: 20.0,
            food: 20,
            saturation: 5.0,
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 1),
        });
        world
    }

    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = full_food_world();
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
    assert_eq!(counters.use_item_commands_queued, 1);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::UseItem(UseItemCommand {
            packet: UseItem {
                hand: InteractionHand::MainHand,
                sequence: 1,
                ..
            },
            selected_hotbar_slot: 0,
        })
    ));

    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = full_food_world();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(99, 1),
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
    assert_eq!(counters.use_item_commands_queued, 2);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::UseItem(UseItemCommand {
            packet: UseItem {
                hand: InteractionHand::MainHand,
                sequence: 1,
                ..
            },
            selected_hotbar_slot: 0,
        })
    ));
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::UseItem(UseItemCommand {
            packet: UseItem {
                hand: InteractionHand::OffHand,
                sequence: 2,
                ..
            },
            selected_hotbar_slot: 0,
        })
    ));
}

#[test]
fn spectator_right_mouse_press_without_target_does_not_use_item() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState {
        y_rot: 45.0,
        x_rot: -20.0,
        ..LocalPlayerPoseState::default()
    });
    set_local_spectator(&mut world);
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
    );

    assert!(input.use_item_held);
    assert_eq!(
        input.use_item_repeat_delay_ticks,
        USE_ITEM_REPEAT_DELAY_TICKS
    );
    assert!(!world.local_player().interaction.using_item);
    assert_eq!(counters.use_item_commands_queued, 0);

    advance_using_item_at_partial_tick(&mut input, &mut world, &mut counters, &commands, 1.0, 4);

    assert_eq!(counters.use_item_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn custom_mod_item_and_block_do_not_swallow_use_item_or_offhand_fallback() {
    use bbb_protocol::content_registry::{
        ContentRegistryBuilder, ModBlockRegistration, ModItemRegistration,
        BBB_CONTENT_REGISTRY_CHANNEL,
    };
    use bbb_protocol::packets::{CustomPayload, CustomPayloadBody};

    let mut builder = ContentRegistryBuilder::new();
    builder.register_item(ModItemRegistration {
        resource_id: "demo:wand".to_string(),
        max_stack_size: 1,
        is_block_item: false,
    });
    builder.register_block(ModBlockRegistration {
        resource_id: "demo:altar".to_string(),
        state_count: 1,
        light_emission: 0,
        destroy_speed: 1.5,
        full_cube_collision: true,
        dynamic_light_rgb: None,
    });
    let registry = builder.freeze_for_session().unwrap();
    let mod_item_id = registry.item_by_resource_id("demo:wand").unwrap().id;
    let mod_block_state_id = registry
        .block_by_resource_id("demo:altar")
        .unwrap()
        .first_state_id;

    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    world.begin_connection_session(false);
    world.begin_registry_configuration();
    world.apply_custom_payload(CustomPayload {
        id: BBB_CONTENT_REGISTRY_CHANNEL.to_string(),
        payload: CustomPayloadBody::Unknown {
            raw_payload: registry.encode_wire(),
        },
    });
    world.finish_registry_configuration().unwrap();
    assert!(world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
        block_state_id: mod_block_state_id,
    }));
    // Hold vanilla wooden axe in main hand and custom mod item in offhand while
    // targeting the custom mod block: UnsupportedItemUseOn from the tool must
    // not swallow UseItem or OffHand fallback.
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:wooden_axe")
                .unwrap()
                .id,
            1,
        ),
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(mod_item_id, 1),
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

    let mut queued = Vec::new();
    while let Ok(cmd) = rx.try_recv() {
        queued.push(cmd);
    }
    assert!(
        queued.iter().any(|cmd| matches!(
            cmd,
            NetCommand::UseItemOn(cmd) if cmd.packet.hand == InteractionHand::MainHand
        )),
        "expected MainHand UseItemOn against custom block, got {queued:?}"
    );
    assert!(
        queued.iter().any(|cmd| matches!(
            cmd,
            NetCommand::UseItem(cmd) if cmd.packet.hand == InteractionHand::MainHand
        )),
        "expected MainHand UseItem after Pass on custom block, got {queued:?}"
    );
    assert!(
        queued.iter().any(|cmd| matches!(
            cmd,
            NetCommand::UseItem(cmd) if cmd.packet.hand == InteractionHand::OffHand
        )),
        "expected OffHand custom item UseItem fallback, got {queued:?}"
    );
}
