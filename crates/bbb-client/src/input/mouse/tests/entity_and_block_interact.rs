use super::*;

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
fn right_mouse_press_on_entity_queues_interact_with_relative_hit_location() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.sneak = true;
    let mut world = world_with_crosshair_entity(123);
    assert!(world.set_local_selected_hotbar_slot(4));
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
    assert!(input.use_item_held);
    assert_eq!(
        input.use_item_repeat_delay_ticks,
        USE_ITEM_REPEAT_DELAY_TICKS
    );
    assert_eq!(counters.interact_entity_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::InteractEntity(InteractEntityCommand {
            packet: InteractEntity {
                entity_id: 123,
                hand: InteractionHand::MainHand,
                location: ProtocolVec3d {
                    x: 0.0,
                    y: 0.6200000047683716,
                    z: -0.49000000953674316,
                },
                using_secondary_action: true,
            },
            selected_hotbar_slot: 4,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::InteractEntity(InteractEntityCommand {
            packet: InteractEntity {
                entity_id: 123,
                hand: InteractionHand::OffHand,
                location: ProtocolVec3d {
                    x: 0.0,
                    y: 0.6200000047683716,
                    z: -0.49000000953674316,
                },
                using_secondary_action: true,
            },
            selected_hotbar_slot: 4,
        })
    );
}

#[test]
fn right_mouse_press_on_permitted_command_minecart_opens_editor_and_still_interacts() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_command_minecart(123);
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
    );

    assert!(input
        .command_block_editor_hud_state()
        .is_some_and(|state| state.block.is_none()));

    assert_eq!(counters.interact_entity_commands_queued, 1);
    let NetCommand::InteractEntity(interact) = rx.try_recv().unwrap() else {
        panic!("expected command-minecart interaction");
    };
    assert_eq!(interact.packet.entity_id, 123);
    assert_eq!(interact.packet.hand, InteractionHand::MainHand);
    assert_eq!(interact.selected_hotbar_slot, 0);
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
}

#[test]
fn right_mouse_press_on_minecart_success_swings_main_and_stops() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_entity(123);
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(99, 1),
    });
    let mut counters = NetCounters::default();
    let mut first_person_hand_presentation =
        FirstPersonHandPresentation::fully_raised_for_test(&world);
    let mut random = LevelEventSoundRandomState::default();
    let mut audio_events = None;

    handle_mouse_input_at_partial_tick_with_effects_and_presentation(
        &mut input,
        &mut first_person_hand_presentation,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
        1.0,
        &mut random,
        &mut audio_events,
    );

    assert_eq!(counters.interact_entity_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::InteractEntity(InteractEntityCommand {
            packet: InteractEntity {
                entity_id: 123,
                hand: InteractionHand::MainHand,
                location: ProtocolVec3d {
                    x: 0.0,
                    y: 0.6200000047683716,
                    z: -0.49000000953674316,
                },
                using_secondary_action: false,
            },
            selected_hotbar_slot: 0,
        })
    );
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert_eq!(
        first_person_hand_presentation.interpolated_inverse_height(
            InteractionHand::MainHand,
            None,
            1.0,
        ),
        0.0,
        "entity success never invokes ItemInHandRenderer.itemUsed",
    );
}

#[test]
fn right_mouse_press_on_default_interaction_entity_consumes_without_swing() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_entity_type(123, VANILLA_ENTITY_TYPE_INTERACTION_ID);
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

    assert_eq!(counters.interact_entity_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 0);
    let NetCommand::InteractEntity(command) = rx.try_recv().unwrap() else {
        panic!("expected interaction-entity command");
    };
    assert_eq!(command.packet.entity_id, 123);
    assert_eq!(command.packet.hand, InteractionHand::MainHand);
    assert!(rx.try_recv().is_err());
}

#[test]
fn right_mouse_press_on_empty_item_frame_uses_populated_offhand() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_entity_type(123, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID);
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(99, 1),
    });
    let mut counters = NetCounters::default();
    let player_pose = world.local_player_pose();

    assert!(start_use_item(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(CrosshairTarget::Entity(CrosshairEntityHit {
            entity_id: 123,
            location: ProtocolVec3d {
                x: 0.0,
                y: 1.62,
                z: 3.0,
            },
            relative_location: ProtocolVec3d {
                x: 0.0,
                y: 0.12,
                z: 0.0,
            },
        })),
        player_pose,
    ));

    assert_eq!(counters.interact_entity_commands_queued, 2);
    for expected_hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
        let NetCommand::InteractEntity(command) = rx.try_recv().unwrap() else {
            panic!("expected item-frame interaction");
        };
        assert_eq!(command.packet.entity_id, 123);
        assert_eq!(command.packet.hand, expected_hand);
    }
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::OffHand)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn right_mouse_press_on_default_mob_pass_tries_both_hands_in_order() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_entity_type(123, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    let mut counters = NetCounters::default();

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
    );

    assert_eq!(counters.interact_entity_commands_queued, 2);
    for expected_hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
        let NetCommand::InteractEntity(command) = rx.try_recv().unwrap() else {
            panic!("expected default-mob interaction");
        };
        assert_eq!(command.packet.entity_id, 123);
        assert_eq!(command.packet.hand, expected_hand);
    }
}

#[test]
fn spawn_egg_success_on_default_mob_stops_without_client_swing() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_entity_type(123, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_CHICKEN_SPAWN_EGG_ID, 1),
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

    assert!(!world.local_player().interaction.using_item);
    assert_eq!(counters.interact_entity_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 0);
    let NetCommand::InteractEntity(command) = rx.try_recv().unwrap() else {
        panic!("expected spawn-egg mob interaction");
    };
    assert_eq!(command.packet.entity_id, 123);
    assert_eq!(command.packet.hand, InteractionHand::MainHand);
}

#[test]
fn no_target_ender_eye_and_trident_start_using_without_client_swing() {
    for item_id in [VANILLA_ITEM_ENDER_EYE_ID, VANILLA_ITEM_TRIDENT_ID] {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(item_id, 1),
        });
        let mut counters = NetCounters::default();
        let player_pose = world.local_player_pose();

        assert!(start_use_item(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            None,
            player_pose,
        ));

        assert!(world.local_player().interaction.using_item);
        assert_eq!(counters.use_item_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 0);
        assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItem(_))));
        assert!(rx.try_recv().is_err());
    }
}

#[test]
fn no_target_boat_use_raycast_any_fluid_swings_without_local_shrink() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    assert!(world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
        block_state_id: test_block_state_id("minecraft:water", &[("level", "0")]),
    }));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_OAK_BOAT_ID, 1),
    });
    let held_before = world.local_item_in_hand(InteractionHand::MainHand).cloned();
    let mut counters = NetCounters::default();
    let player_pose = world.local_player_pose();

    assert!(start_use_item(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        player_pose,
    ));

    assert!(!world.local_player().interaction.using_item);
    assert_eq!(
        world.local_item_in_hand(InteractionHand::MainHand),
        held_before.as_ref()
    );
    assert_eq!(counters.use_item_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItem(_))));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn no_target_bottle_use_fills_source_water_with_local_sound_and_swing() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    assert!(world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
        block_state_id: test_block_state_id("minecraft:water", &[("level", "0")]),
    }));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_GLASS_BOTTLE_ID, 1),
    });
    let mut counters = NetCounters::default();
    let player_pose = world.local_player_pose();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_seed_world = WorldStore::new();
    let expected_seed = expected_seed_world.next_sound_seed();
    let mut audio = RecordingAudioSink::default();
    let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);
    let mut first_person_hand_presentation =
        FirstPersonHandPresentation::fully_raised_for_test(&world);

    assert!(start_use_item_with_effects(
        &mut input,
        &mut first_person_hand_presentation,
        &mut world,
        &mut counters,
        &commands,
        None,
        player_pose,
        &mut random,
        &mut audio_events,
    ));

    let filled = world
        .local_item_in_hand(InteractionHand::MainHand)
        .expect("filled bottle");
    let water_potion_id =
        bbb_protocol::potion_types::vanilla_potion_for_resource_id("minecraft:water")
            .expect("generated water potion")
            .id;
    assert_eq!(filled.item_id, Some(VANILLA_ITEM_POTION_ID));
    assert_eq!(filled.count, 1);
    assert_eq!(
        filled.component_patch.potion_contents.potion_id,
        Some(water_potion_id)
    );
    assert_eq!(counters.use_item_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItem(_))));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(rx.try_recv().is_err());
    assert_eq!(audio.positioned_sounds.len(), 1);
    let sound = &audio.positioned_sounds[0];
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:item.bottle.fill")
    );
    assert_eq!(sound.source, "neutral");
    assert_eq!(sound.volume, 1.0);
    assert_eq!(sound.pitch, 1.0);
    assert_eq!(sound.seed, expected_seed);
    assert_eq!(sound.position, player_pose.expect("player pose").position);
    assert_eq!(
        first_person_hand_presentation.interpolated_inverse_height(
            InteractionHand::MainHand,
            None,
            1.0,
        ),
        1.0,
        "empty-space item success always invokes ItemInHandRenderer.itemUsed",
    );
}

#[test]
fn right_mouse_riptide_release_queues_action_applies_motion_and_suppresses_local_sound() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    assert!(world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
        block_state_id: test_block_state_id("minecraft:water", &[("level", "0")]),
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        on_ground: true,
        y_rot: 90.0,
        x_rot: -30.0,
        ..LocalPlayerPoseState::default()
    });
    world.record_registry_data(RegistryData {
        registry: "minecraft:enchantment".to_string(),
        entries: vec![RegistryDataEntry {
            id: "minecraft:riptide".to_string(),
            raw_data: None,
        }],
        raw_payload_len: 0,
    });
    let mut trident = item_stack(VANILLA_ITEM_TRIDENT_ID, 1);
    trident
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID);
    trident
        .component_patch
        .enchantments
        .push(ItemEnchantmentSummary {
            holder_id: 0,
            level: 1,
        });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: trident,
    });
    let mut counters = NetCounters::default();
    let player_pose = world.local_player_pose();
    assert!(start_use_item(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        player_pose,
    ));
    assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItem(_))));
    world.advance_local_using_item_ticks(10);

    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut audio = RecordingAudioSink::default();
    let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);
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

    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::ReleaseUseItem,
            ..
        })
    ));
    assert!(rx.try_recv().is_err());
    assert_eq!(counters.player_action_commands_queued, 1);
    assert!(!world.local_player().interaction.using_item);
    assert_eq!(
        world
            .local_player_auto_spin_attack()
            .map(|state| state.ticks_remaining),
        Some(20)
    );
    assert!(world
        .local_player_pose()
        .is_some_and(|pose| pose.delta_movement.x < -1.0 && pose.position.y > 1.0));
    assert!(audio.positioned_sounds.is_empty());
    assert!(audio.entity_sounds.is_empty());
}

#[test]
fn dye_success_on_sheep_swings_main_hand_and_stops_before_offhand() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_entity_type(123, VANILLA_ENTITY_TYPE_SHEEP_ID);
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:sheep_food".to_string(),
                entries: Vec::new(),
            }],
        }],
    });
    world.set_default_item_component_codec_values(BTreeMap::from([(
        VANILLA_ITEM_RED_DYE_ID,
        BTreeMap::from([(VANILLA_DATA_COMPONENT_DYE_TYPE_ID, serde_json::json!("red"))]),
    )]));
    world.rebuild_item_component_defaults().unwrap();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_RED_DYE_ID, 1),
    });
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

    assert_eq!(counters.interact_entity_commands_queued, 1);
    let NetCommand::InteractEntity(command) = rx.try_recv().unwrap() else {
        panic!("expected Sheep dye interaction");
    };
    assert_eq!(command.packet.entity_id, 123);
    assert_eq!(command.packet.hand, InteractionHand::MainHand);
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn spectator_right_mouse_press_on_entity_tries_both_hands_in_order() {
    let (tx, mut rx) = mpsc::channel(2);
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
        MouseButton::Right,
        ElementState::Pressed,
    );

    assert_eq!(counters.interact_entity_commands_queued, 2);
    for expected_hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
        let NetCommand::InteractEntity(command) = rx.try_recv().unwrap() else {
            panic!("expected spectator entity interaction");
        };
        assert_eq!(command.packet.entity_id, 123);
        assert_eq!(command.packet.hand, expected_hand);
        assert_eq!(
            command.packet.location,
            ProtocolVec3d {
                x: 0.0,
                y: 0.6200000047683716,
                z: -0.49000000953674316,
            }
        );
        assert!(!command.packet.using_secondary_action);
        assert_eq!(command.selected_hotbar_slot, 0);
    }
}

#[test]
fn right_mouse_press_on_entity_outside_world_border_does_not_interact() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_entity(123);
    set_world_border_excluding_crosshair_block(&mut world);
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
    assert!(input.use_item_held);
    assert_eq!(
        input.use_item_repeat_delay_ticks,
        USE_ITEM_REPEAT_DELAY_TICKS
    );
    assert_eq!(counters.interact_entity_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn right_mouse_press_on_default_pass_block_queues_both_hands() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    assert!(world.set_local_selected_hotbar_slot(4));
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
    assert!(input.use_item_held);
    assert_eq!(
        input.use_item_repeat_delay_ticks,
        USE_ITEM_REPEAT_DELAY_TICKS
    );
    assert_eq!(counters.use_item_on_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::UseItemOn(UseItemOnCommand {
            packet: UseItemOn {
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
            },
            selected_hotbar_slot: 4,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::UseItemOn(UseItemOnCommand {
            packet: UseItemOn {
                hand: InteractionHand::OffHand,
                hit: ProtocolBlockHitResult {
                    pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                    direction: ProtocolDirection::North,
                    cursor_x: 0.0,
                    cursor_y: 0.62,
                    cursor_z: 0.0,
                    inside: false,
                    world_border_hit: false,
                },
                sequence: 2,
            },
            selected_hotbar_slot: 4,
        })
    );
}

#[test]
fn right_mouse_press_locks_compass_without_local_sound_and_preserves_drop_swing() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_lodestone();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_COMPASS_ID, 1),
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

    assert_eq!(counters.use_item_on_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItemOn(_))));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    let locked = world.local_item_in_hand(InteractionHand::MainHand).unwrap();
    assert!(locked
        .component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID));
    assert_eq!(
        locked.component_patch.lodestone_target,
        Some(LodestoneTargetSummary {
            dimension: "minecraft:overworld".to_string(),
            pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
        })
    );
    assert_eq!(locked.component_patch.lodestone_tracked, Some(true));
    assert!(
        world.client_audio().last_sound.is_none(),
        "CompassItem passes null as the excluded player, so ClientLevel waits for the server sound packet"
    );

    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_lodestone();
    for slot in 0..36 {
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot,
            item: item_stack(VANILLA_ITEM_STONE_ID, 1),
        });
    }
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_COMPASS_ID, 2),
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
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .count,
        1
    );
    assert_eq!(counters.use_item_on_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 2);
    assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItemOn(_))));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(world.client_audio().last_sound.is_none());
}

#[test]
fn right_mouse_ignition_plays_flint_sound_and_suppresses_fire_charge_sound() {
    let target = BlockPos { x: 0, y: 1, z: 2 };
    let mut expected_pitch_random = LevelEventSoundRandomState::with_seed(0);
    let expected_pitch = expected_pitch_random.next_float() * 0.4 + 0.8;
    let mut expected_seed_world = WorldStore::new();
    let expected_seed = expected_seed_world.next_sound_seed();

    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_ignitable_block();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_FLINT_AND_STEEL_ID, 1),
    });
    let mut direct_prediction = world.clone();
    let direct_result = direct_prediction.predict_local_block_use(
        CROSSHAIR_BLOCK_POS,
        ProtocolDirection::North,
        InteractionHand::MainHand,
        false,
        1,
    );
    assert!(
        matches!(
            direct_result,
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
        ),
        "{direct_result:?}"
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

    assert_eq!(counters.use_item_on_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItemOn(_))));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert_eq!(audio.positioned_sounds.len(), 1);
    let sound = &audio.positioned_sounds[0];
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:item.flintandsteel.use")
    );
    assert_eq!(sound.source, "block");
    assert_eq!(
        sound.position,
        ProtocolVec3d {
            x: 0.5,
            y: 1.5,
            z: 2.5,
        }
    );
    assert_eq!(sound.volume, 1.0);
    assert_eq!(sound.pitch, expected_pitch);
    assert_eq!(sound.seed, expected_seed);
    assert_eq!(world.client_audio().last_sound.as_ref(), Some(sound));
    assert_eq!(
        world.probe_block(target).unwrap().block_state_id,
        test_block_state_id(
            "minecraft:fire",
            &[
                ("age", "0"),
                ("east", "false"),
                ("north", "false"),
                ("south", "true"),
                ("up", "false"),
                ("west", "false"),
            ],
        )
    );

    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_ignitable_block();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_FIRE_CHARGE_ID, 2),
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

    assert_eq!(counters.use_item_on_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItemOn(_))));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(audio.positioned_sounds.is_empty());
    assert!(world.client_audio().last_sound.is_none());

    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .count,
        1
    );
}

#[test]
fn default_block_pass_falls_through_to_same_hand_item_use() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    install_consumable(&mut world, 0, VANILLA_ITEM_APPLE_ID, 1.6);
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
    assert_eq!(counters.use_item_on_commands_queued, 1);
    assert_eq!(counters.use_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::UseItemOn(UseItemOnCommand {
            packet: UseItemOn {
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
            },
            selected_hotbar_slot: 0,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::UseItem(UseItemCommand {
            packet: UseItem {
                hand: InteractionHand::MainHand,
                sequence: 2,
                y_rot: 0.0,
                x_rot: 0.0,
            },
            selected_hotbar_slot: 0,
        })
    );
}

#[test]
fn right_mouse_press_on_permitted_command_block_opens_editor_and_still_uses_block() {
    let (tx, mut rx) = mpsc::channel(2);
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

    assert!(input
        .command_block_editor_hud_state()
        .is_some_and(|state| state.block.is_some()));
    assert!(
        !input
            .command_block_editor_hud_state()
            .unwrap()
            .block
            .unwrap()
            .controls_enabled
    );

    assert_eq!(counters.use_item_on_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::UseItemOn(UseItemOnCommand {
            packet: UseItemOn {
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
            },
            selected_hotbar_slot: 0,
        })
    );
}

#[test]
fn right_mouse_press_on_block_outside_world_border_does_not_queue_use_item_on() {
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
        MouseButton::Right,
        ElementState::Pressed,
    );

    assert!(input.use_item_held);
    assert_eq!(
        input.use_item_repeat_delay_ticks,
        USE_ITEM_REPEAT_DELAY_TICKS
    );
    assert!(!world.local_player().interaction.using_item);
    assert_eq!(counters.use_item_on_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn right_mouse_press_on_block_tries_main_before_populated_offhand() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
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

    assert_eq!(counters.use_item_on_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::UseItemOn(UseItemOnCommand {
            packet: UseItemOn {
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
            },
            selected_hotbar_slot: 0,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::UseItemOn(UseItemOnCommand {
            packet: UseItemOn {
                hand: InteractionHand::OffHand,
                hit: ProtocolBlockHitResult {
                    pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                    direction: ProtocolDirection::North,
                    cursor_x: 0.0,
                    cursor_y: 0.62,
                    cursor_z: 0.0,
                    inside: false,
                    world_border_hit: false,
                },
                sequence: 2,
            },
            selected_hotbar_slot: 0,
        })
    );
}

#[test]
fn firework_use_on_swings_main_hand_and_stops_before_offhand() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_FIREWORK_ROCKET_ID, 1),
    });
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

    assert_eq!(counters.use_item_on_commands_queued, 1);
    let NetCommand::UseItemOn(command) = rx.try_recv().unwrap() else {
        panic!("expected firework use-on command");
    };
    assert_eq!(command.packet.hand, InteractionHand::MainHand);
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn bone_meal_full_water_growth_swings_main_hand_without_local_shrink() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    assert!(world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
        block_state_id: test_block_state_id("minecraft:stone", &[]),
    }));
    let water_pos = BlockPos { x: 0, y: 1, z: 2 };
    let water_state = test_block_state_id("minecraft:water", &[("level", "0")]);
    assert!(world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos {
            x: water_pos.x,
            y: water_pos.y,
            z: water_pos.z,
        },
        block_state_id: water_state,
    }));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_BONE_MEAL_ID, 3),
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(VANILLA_ITEM_STONE_ID, 1),
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

    assert_eq!(counters.use_item_on_commands_queued, 1);
    let NetCommand::UseItemOn(command) = rx.try_recv().unwrap() else {
        panic!("expected bone-meal use-on command");
    };
    assert_eq!(command.packet.hand, InteractionHand::MainHand);
    assert_eq!(counters.swing_commands_queued, 1);
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
        3
    );
    assert_eq!(
        world.probe_block(water_pos).unwrap().block_state_id,
        water_state
    );
    assert!(world.local_block_predictions().is_empty());
}

#[test]
fn water_potion_mud_conversion_mutates_once_and_suppresses_local_sounds() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    assert!(world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos {
            x: CROSSHAIR_BLOCK_POS.x,
            y: CROSSHAIR_BLOCK_POS.y,
            z: CROSSHAIR_BLOCK_POS.z,
        },
        block_state_id: test_block_state_id("minecraft:dirt", &[]),
    }));
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:block".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:convertable_to_mud".to_string(),
                entries: vec![VANILLA_BLOCK_DIRT_ID],
            }],
        }],
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: water_potion_item_stack(1),
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(VANILLA_ITEM_STONE_ID, 1),
    });
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut audio = RecordingAudioSink::default();
    let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);
    let mut counters = NetCounters::default();

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

    assert_eq!(counters.use_item_on_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    let NetCommand::UseItemOn(command) = rx.try_recv().unwrap() else {
        panic!("expected potion use-on command");
    };
    assert_eq!(command.packet.hand, InteractionHand::MainHand);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(rx.try_recv().is_err());
    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .item_id,
        Some(VANILLA_ITEM_GLASS_BOTTLE_ID)
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
            .probe_block(CROSSHAIR_BLOCK_POS)
            .unwrap()
            .block_state_id,
        test_block_state_id("minecraft:mud", &[])
    );
    assert_eq!(world.local_block_predictions().len(), 1);
    assert!(audio.positioned_sounds.is_empty());
    assert!(world.client_audio().last_sound.is_none());
}

#[test]
fn brush_use_on_starts_main_hand_use_without_swing_or_offhand() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_BRUSH_ID, 1),
    });
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
    assert_eq!(
        world.local_player().interaction.using_item_hand,
        Some(InteractionHand::MainHand)
    );
    assert_eq!(counters.use_item_on_commands_queued, 1);
    assert_eq!(counters.use_item_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 0);
    let NetCommand::UseItemOn(command) = rx.try_recv().unwrap() else {
        panic!("expected brush use-on command");
    };
    assert_eq!(command.packet.hand, InteractionHand::MainHand);
    assert!(rx.try_recv().is_err());
}

#[test]
fn spectator_right_mouse_press_on_block_keeps_main_hand_when_hotbar_is_empty() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(99, 1),
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

    assert!(!world.local_player().interaction.using_item);
    assert!(input.use_item_held);
    assert_eq!(counters.use_item_on_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::UseItemOn(UseItemOnCommand {
            packet: UseItemOn {
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
            },
            selected_hotbar_slot: 0,
        })
    );
}

#[test]
fn right_mouse_press_on_block_keeps_main_hand_when_both_hands_have_items() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_crosshair_block();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
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

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::UseItemOn(UseItemOnCommand {
            packet: UseItemOn {
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
            },
            selected_hotbar_slot: 0,
        })
    );
}

#[test]
fn held_right_mouse_on_block_repeats_use_item_on_after_vanilla_delay() {
    let (tx, mut rx) = mpsc::channel(4);
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
    for (expected_hand, expected_sequence) in [
        (InteractionHand::MainHand, 1),
        (InteractionHand::OffHand, 2),
    ] {
        let NetCommand::UseItemOn(command) = rx.try_recv().unwrap() else {
            panic!("expected block-use command");
        };
        assert_eq!(command.packet.hand, expected_hand);
        assert_eq!(command.packet.sequence, expected_sequence);
        assert_eq!(
            command.packet.hit.pos,
            ProtocolBlockPos { x: 0, y: 1, z: 3 }
        );
    }

    advance_using_item_at_partial_tick(&mut input, &mut world, &mut counters, &commands, 1.0, 3);
    assert!(rx.try_recv().is_err());
    assert_eq!(counters.use_item_on_commands_queued, 2);
    assert_eq!(input.use_item_repeat_delay_ticks, 1);

    advance_using_item_at_partial_tick(&mut input, &mut world, &mut counters, &commands, 1.0, 1);

    assert_eq!(counters.use_item_on_commands_queued, 4);
    assert_eq!(
        input.use_item_repeat_delay_ticks,
        USE_ITEM_REPEAT_DELAY_TICKS
    );
    for (expected_hand, expected_sequence) in [
        (InteractionHand::MainHand, 3),
        (InteractionHand::OffHand, 4),
    ] {
        let NetCommand::UseItemOn(command) = rx.try_recv().unwrap() else {
            panic!("expected repeated block-use command");
        };
        assert_eq!(command.packet.hand, expected_hand);
        assert_eq!(command.packet.sequence, expected_sequence);
        assert_eq!(
            command.packet.hit.pos,
            ProtocolBlockPos { x: 0, y: 1, z: 3 }
        );
    }
}

#[test]
fn right_mouse_release_stops_repeated_block_use() {
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
        MouseButton::Right,
        ElementState::Pressed,
    );
    assert!(rx.try_recv().is_ok());
    assert!(rx.try_recv().is_ok());

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Released,
    );
    assert!(!input.use_item_held);
    assert_eq!(input.use_item_repeat_delay_ticks, 0);

    advance_using_item_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        1.0,
        USE_ITEM_REPEAT_DELAY_TICKS.into(),
    );

    assert_eq!(counters.use_item_on_commands_queued, 2);
    assert!(rx.try_recv().is_err());
}

#[test]
fn held_right_mouse_without_target_does_not_repeat_while_item_is_in_use() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState {
        y_rot: 45.0,
        x_rot: -20.0,
        ..LocalPlayerPoseState::default()
    });
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
    assert!(world.local_player().interaction.using_item);
    assert_eq!(counters.use_item_commands_queued, 1);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::UseItem(_)));

    advance_using_item_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        1.0,
        USE_ITEM_REPEAT_DELAY_TICKS.into(),
    );

    assert_eq!(counters.use_item_commands_queued, 1);
    assert!(rx.try_recv().is_err());
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
        NetCommand::InteractEntity(InteractEntityCommand {
            packet: InteractEntity {
                entity_id: 101,
                hand: InteractionHand::MainHand,
                location: ProtocolVec3d {
                    x: 0.0,
                    y: 0.6200000047683716,
                    z: -0.5,
                },
                using_secondary_action: false,
            },
            selected_hotbar_slot: 0,
        })
    );
}

#[test]
fn swapping_attack_and_use_mouse_bindings_routes_actions_to_rebound_buttons() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut options = crate::options::ClientOptions::defaults(Default::default());
    options.set_dynamic_raw_for_test("key_key.attack", "key.mouse.right");
    options.set_dynamic_raw_for_test("key_key.use", "key.mouse.left");
    input.set_key_bindings(crate::options::key_bindings::KeyBindings::from_options(
        &options,
    ));
    let mut world = world_with_crosshair_block();
    let mut counters = NetCounters::default();

    // Pressing Right (rebound to attack) starts destroying the targeted block.
    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
    );
    assert!(input.destroy_block_held);
    assert!(!input.use_item_held);
    assert!(input.item_model_keybind_context().attack);
    assert!(!input.item_model_keybind_context().use_item);
    assert_eq!(counters.use_item_on_commands_queued, 0);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(action) if action.action == PlayerActionKind::StartDestroyBlock
    ));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::Swing(_)));

    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Released,
    );
    assert!(!input.destroy_block_held);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(action) if action.action == PlayerActionKind::AbortDestroyBlock
    ));

    // Pressing Left (rebound to use) triggers UseItemOn instead of attack.
    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );
    assert!(input.use_item_held);
    assert!(!input.destroy_block_held);
    assert!(input.item_model_keybind_context().use_item);
    assert!(!input.item_model_keybind_context().attack);
    assert!(counters.use_item_on_commands_queued >= 1);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::UseItemOn(_)));
}

#[test]
fn keyboard_bindings_for_attack_use_and_pick_item_trigger_gameplay_actions() {
    let (tx, mut rx) = mpsc::channel(16);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut options = crate::options::ClientOptions::defaults(Default::default());
    options.set_dynamic_raw_for_test("key_key.attack", "key.keyboard.z");
    options.set_dynamic_raw_for_test("key_key.use", "key.keyboard.r");
    options.set_dynamic_raw_for_test("key_key.pickItem", "key.keyboard.k");
    input.set_key_bindings(crate::options::key_bindings::KeyBindings::from_options(
        &options,
    ));
    let mut world = world_with_crosshair_block();
    let mut counters = NetCounters::default();

    // Unbound left/right/middle mouse buttons should no longer trigger attack/use/pick.
    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );
    assert!(!input.destroy_block_held);
    assert!(rx.try_recv().is_err());

    // Keyboard attack (KeyZ) starts and stops block destruction.
    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyZ),
        ElementState::Pressed,
    );
    assert!(input.destroy_block_held);
    assert!(input.item_model_keybind_context().attack);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(action) if action.action == PlayerActionKind::StartDestroyBlock
    ));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::Swing(_)));

    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyZ),
        ElementState::Released,
    );
    assert!(!input.destroy_block_held);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(action) if action.action == PlayerActionKind::AbortDestroyBlock
    ));

    // Keyboard pickItem (KeyK) queues PickItemFromBlock.
    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyK),
        ElementState::Pressed,
    );
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::PickItemFromBlock(_)
    ));

    // Keyboard use (KeyR) starts UseItemOn and clears use_item_held on release.
    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyR),
        ElementState::Pressed,
    );
    assert!(input.use_item_held);
    assert!(input.item_model_keybind_context().use_item);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::UseItemOn(_)));
    while rx.try_recv().is_ok() {}

    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyR),
        ElementState::Released,
    );
    assert!(!input.use_item_held);
}

#[test]
fn toggle_attack_latches_destroy_block_across_release_until_second_press() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.set_toggle_key_options(false, false, true, false);
    let mut world = world_with_crosshair_block();
    let mut counters = NetCounters::default();

    // First press latches attack on.
    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );
    assert!(input.destroy_block_held);
    assert!(input.item_model_keybind_context().attack);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(action) if action.action == PlayerActionKind::StartDestroyBlock
    ));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::Swing(_)));

    // Releasing mouse button keeps destroy_block_held latched and does not abort.
    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Released,
    );
    assert!(input.destroy_block_held);
    assert!(input.item_model_keybind_context().attack);
    assert!(rx.try_recv().is_err());

    // Second press unlatches and aborts block destruction.
    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
    );
    assert!(!input.destroy_block_held);
    assert!(!input.item_model_keybind_context().attack);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(action) if action.action == PlayerActionKind::AbortDestroyBlock
    ));
}

#[test]
fn toggle_use_latches_use_item_across_release_until_second_press() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.set_toggle_key_options(false, false, false, true);
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState {
        y_rot: 45.0,
        x_rot: -20.0,
        ..LocalPlayerPoseState::default()
    });
    install_consumable(&mut world, 0, 42, 1.6);
    let mut counters = NetCounters::default();

    // First press latches use on.
    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
    );
    assert!(input.use_item_held);
    assert!(input.item_model_keybind_context().use_item);
    assert!(world.local_player().interaction.using_item);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::UseItem(_)));

    // Releasing right mouse button keeps using item latched.
    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Released,
    );
    assert!(input.use_item_held);
    assert!(input.item_model_keybind_context().use_item);
    assert!(world.local_player().interaction.using_item);
    assert!(rx.try_recv().is_err());

    // Second press unlatches and sends ReleaseUseItem.
    handle_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
    );
    assert!(!input.use_item_held);
    assert!(!input.item_model_keybind_context().use_item);
    assert!(!world.local_player().interaction.using_item);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(action) if action.action == PlayerActionKind::ReleaseUseItem
    ));
}
