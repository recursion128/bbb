//! Inventory and menu slots: the baseline a join publishes, the crafting grid a
//! close returns, and the slots a click the client mispredicted comes back as.

use super::*;

fn inventory_test_stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

#[test]
fn departed_rejoin_restores_latest_inventory_and_fall_flying() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let dirt = inventory_test_stack(bbb_protocol::item_types::VANILLA_ITEM_DIRT_ID, 7);
    world.set_inventory_slot_for_test(ConnectionId(1), 5, dirt.clone());
    world.set_player_fall_flying_for_test(ConnectionId(1), true);
    super::player_fall_flying::wear_an_elytra_in_the_air(&sender, &receiver, &mut world, 1);
    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(1),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(2),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let rejoined = world.tick(receiver.drain());
    assert_eq!(rejoined.snapshot.players[0].inventory.slots[5], dirt);
    assert!(rejoined.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::SetEntityData(update),
        } if update.values.iter().any(|value| matches!(
            value.value,
            EntityDataValueKind::Byte(flags)
                if flags & bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING != 0
        ))
    )));
    let patch = world
        .save_request(1_786_000_000_010, 10, RegionCompression::Zlib)
        .unwrap()
        .player_patches
        .remove(0);
    assert!(patch.created.fall_flying);
}

/// `ServerPlayer.restoreFrom` copies the ender chest at
/// `ServerPlayer.java:1611`, outside both the `keepEverything` branch and the
/// `KEEP_INVENTORY` one, so it crosses a death the inventory does not.
#[test]
fn death_respawn_keeps_the_ender_chest_whatever_the_gamerule_says() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let stored = inventory_test_stack(bbb_protocol::item_types::VANILLA_ITEM_DIRT_ID, 3);
    world.set_inventory_slot_for_test(ConnectionId(1), 5, stored.clone());
    world.set_ender_chest_slot_for_test(ConnectionId(1), 5, stored.clone());
    world.set_player_dead_for_test(ConnectionId(1));
    let respawned = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
    );
    assert_eq!(
        respawned.snapshot.players[0].inventory.slots[5],
        ItemStackSummary::empty()
    );
    assert_eq!(world.ender_chest_slot_for_test(ConnectionId(1), 5), stored);
}

#[test]
fn death_respawn_clears_or_keeps_inventory_but_always_clears_fall_flying() {
    for keep_inventory in [false, true] {
        let (sender, receiver) = mailbox();
        let config = if keep_inventory {
            ServerWorldConfig {
                game_type: GameType::Survival,
                ..operator_config(&[("alice", 1)])
            }
        } else {
            creative_fixture_config()
        };
        let mut world = ServerWorld::new(config).unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        if keep_inventory {
            chat_command(
                &sender,
                &receiver,
                &mut world,
                1,
                "gamerule keep_inventory true",
            );
        }
        let held = inventory_test_stack(bbb_protocol::item_types::VANILLA_ITEM_DIRT_ID, 3);
        world.set_inventory_slot_for_test(ConnectionId(1), 5, held.clone());
        world.set_player_fall_flying_for_test(ConnectionId(1), true);
        world.set_player_dead_for_test(ConnectionId(1));
        let respawned = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
        );
        assert_eq!(
            respawned.snapshot.players[0].inventory.slots[5],
            if keep_inventory {
                held
            } else {
                ItemStackSummary::empty()
            }
        );
        let patch = world
            .save_request(1_786_000_000_020, 20, RegionCompression::Zlib)
            .unwrap()
            .player_patches
            .remove(0);
        assert!(!patch.created.fall_flying);
    }
}

#[test]
fn join_publishes_complete_inventory_baseline_before_first_chunk() {
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(7),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    let tick = world.tick(receiver.drain());

    assert_eq!(tick.tick, 1);
    assert_eq!(tick.processed_commands, 1);
    assert!(matches!(
        &tick.events[0],
        WorldEvent::Send {
            connection_id: ConnectionId(7),
            packet: ClientboundPlayPacket::Login(_),
        }
    ));
    // The world difficulty and the player's abilities follow the login and
    // precede the held slot (`PlayerList.java:181-183`).
    assert!(matches!(
        &tick.events[1],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::ChangeDifficulty(update),
            ..
        } if update.difficulty == Difficulty::Easy && !update.locked
    ));
    assert!(matches!(
        &tick.events[2],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::PlayerAbilities(update),
            ..
        } if *update == PlayerAbilities::for_game_type(GameType::Creative)
    ));
    assert!(matches!(
        &tick.events[3],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::SetHeldSlot(update),
            ..
        } if update.slot == 0
    ));
    // `PlayerList.sendPlayerPermissionLevel` (PlayerList.java:188, 559-572).
    assert!(matches!(
        &tick.events[4],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::EntityEvent(update),
            ..
        } if update.event_id == 24
    ));
    // `sendPlayerPermissionLevel` ends with `sendCommands`
    // (PlayerList.java:571), so the tree travels with the level it belongs to.
    assert!(matches!(
        &tick.events[5],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::Commands(tree),
            ..
        } if *tree == crate::commands::tree::usable_command_tree(CommandPermission::All)
    ));
    // `sendLevelInfo` opens with the border (`PlayerList.java:645-646`).
    assert!(matches!(
        &tick.events[6],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::InitializeBorder(_),
            ..
        }
    ));
    // Full world-clock sync follows the login (`PlayerList.java:647`); the bare
    // test config still publishes both built-in clock entries.
    //
    // The game time it carries is zero rather than one: `createFullSyncPacket`
    // reads `overworld().getGameTime()` (`ServerClockManager.java:129-134`),
    // and a join is handled by `processPacketsAndTick` (`:1005`) before
    // `tickServer` (`:1007`) reaches `ServerLevel.tickTime`, so a joining
    // session is told the time the world ended its previous tick at.
    assert!(matches!(
        &tick.events[7],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::SetTime(update),
            ..
        } if update.game_time == 0
            && update.clock_updates.len() == 2
            && update.clock_updates[0].clock_id == 0
            && update.clock_updates[0].total_ticks == 0
            && update.clock_updates[1].clock_id == 1
            && update.clock_updates[1].total_ticks == 0
    ));
    // `:648`: the server's respawn data.
    assert!(matches!(
        &tick.events[8],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::SetDefaultSpawnPosition(_),
            ..
        }
    ));
    // `sendLevelInfo` closes with the chunk-load announcement
    // (`PlayerList.java:655`) and the tick state
    // (`ServerTickRateManager.updateJoiningPlayer`,
    // ServerTickRateManager.java:121-124).
    assert!(matches!(
        &tick.events[9],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::GameEvent(update),
            ..
        } if update.event_id == 13 && update.param == 0.0
    ));
    assert!(matches!(
        &tick.events[10],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::TickingState(update),
            ..
        } if update.tick_rate == 20.0 && !update.frozen
    ));
    assert!(matches!(
        &tick.events[11],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::TickingStep(update),
            ..
        } if update.tick_steps == 0
    ));
    assert!(matches!(
        &tick.events[12],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::PlayerPosition(_),
            ..
        }
    ));
    // PlayerList.java:202-205: the status MOTD behind the spawn teleport.
    assert!(matches!(
        &tick.events[13],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::ServerData(_),
            ..
        }
    ));
    assert!(matches!(
        &tick.events[14],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::SetChunkCacheCenter(_),
            ..
        }
    ));
    // PlayerList.java:210-215 initializes the inventory after publishing the player.
    assert!(matches!(
        &tick.events[16],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::ContainerSetContent(update),
            ..
        } if update.container_id == 0
            && update.state_id == 1
            && update.items.len() == 46
            && update.items.iter().all(|item| *item == ItemStackSummary::empty())
            && update.carried_item == ItemStackSummary::empty()
    ));
    // The initializing PlayerInfoUpdate announces the new player to everyone
    // (including itself, PlayerList.java:210) before entity pairing begins.
    assert!(matches!(
        &tick.events[15],
        WorldEvent::Send {
            connection_id: ConnectionId(7),
            packet: ClientboundPlayPacket::PlayerInfoUpdate(update),
        } if update.entries.len() == 1 && update.entries[0].profile_id == Uuid::from_u128(1)
    ));
    assert!(matches!(
        &tick.events[17],
        WorldEvent::Send {
            connection_id: ConnectionId(7),
            packet: ClientboundPlayPacket::SetEntityData(update),
        } if update.id == world.player_entity_id_for_test(ConnectionId(7))
            && update.values == vec![bbb_protocol::packets::EntityDataValue {
                data_id: bbb_protocol::entity_synched_data::VANILLA_LIVING_ENTITY_HEALTH_DATA_ID,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: bbb_protocol::packets::EntityDataValueKind::Float(20.0),
            }, bbb_protocol::packets::EntityDataValue {
                data_id: bbb_protocol::entity_synched_data::VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: bbb_protocol::packets::EntityDataValueKind::Byte(127),
            }]
    ));
    // The join itself sends no chunk: it marks the whole tracking view pending,
    // and the tick's chunk-sender step releases what the start rate allows
    // inside one acknowledgeable batch.
    assert!(matches!(
        &tick.events[18],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::ChunkBatchStart,
            ..
        }
    ));
    assert!(tick.events[19..28].iter().all(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::LevelChunkWithLight(_),
            ..
        }
    )));
    assert!(matches!(
        &tick.events[28],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::ChunkBatchFinished { batch_size: 9 },
            ..
        }
    ));
    // `ServerPlayer.doTick`'s health arm (ServerPlayer.java:671-678) fires on
    // the first tick of every session, `lastSentHealth` starting at a value no
    // health can equal (`:238`).
    assert!(matches!(
        &tick.events[29],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::SetHealth(_),
            ..
        }
    ));
    // `doTick`'s experience arm right after it (`:710-713`), for the same
    // reason: `lastSentExp` starts at a total no session can hold (`:241`).
    assert!(matches!(
        &tick.events[30],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::SetExperience(_),
            ..
        }
    ));
    assert_eq!(tick.events.len(), 31);
    assert_eq!(tick.snapshot.players.len(), 1);
    assert_eq!(tick.snapshot.players[0].name, "alice");
    assert_eq!(tick.snapshot.players[0].inventory.slots.len(), 43);
    assert_eq!(tick.snapshot.players[0].inventory.slots[0].count, 0);
}

#[test]
fn closing_the_menu_returns_the_crafting_grid_through_the_inventory() {
    // `handleContainerClose` (:1909-1912) reads no field of the packet and
    // closes whatever is open, which here is always the player inventory.
    // `InventoryMenu.removed` (InventoryMenu.java:88-94) clears the result and
    // hands the grid back, and the hand-back is the one vanilla path that
    // reports single inventory slots to the client.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);

    let grid_stone = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 5,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    // Menu slot 0, the result, is the one slot the creative write refuses
    // (`ServerGamePacketListenerImpl.java:2010`).
    let refused = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 0,
            item: grid_stone.clone(),
        }),
    );
    assert!(refused.events.is_empty(), "{refused:?}");
    // Menu slot 1 is the first crafting slot, and the write publishes the
    // recomputed result and nothing else: the write mirrors the grid slot
    // itself, so the `broadcastChanges` after it reports nothing.
    let written = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 1,
            item: grid_stone.clone(),
        }),
    );
    assert!(
        matches!(
            written.events.as_slice(),
            [WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::ContainerSetSlot(update),
            }] if update.slot == 0 && update.item == ItemStackSummary::empty()
        ),
        "{written:?}"
    );

    let closed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClose(ContainerCloseRequest { container_id: 0 }),
    );
    // The fixture stack of 64 already fills hotbar slot 0, and that is the
    // held slot `getSlotWithRemainingSpace` tries first, so the grid stack
    // opens the next free storage slot instead of merging.
    // Three packets, and each has a different owner. The hand-back reports
    // itself through `SetPlayerInventory`; the same tick's
    // `ServerPlayer.tick` broadcast then notices that the grid slot the
    // creative write had mirrored is now empty and that the inventory slot the
    // stack landed in no longer matches what the join published.
    assert!(
        matches!(
            closed.events.as_slice(),
            [
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::SetPlayerInventory(update),
                },
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::ContainerSetSlot(emptied),
                },
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::ContainerSetSlot(landed),
                },
            ] if update.slot == 1
                && update.item == grid_stone
                && emptied.slot == 1
                && emptied.item == ItemStackSummary::empty()
                && landed.slot == 37
                && landed.item == grid_stone
        ),
        "{closed:?}"
    );
    assert_eq!(closed.snapshot.players[0].inventory.slots[1], grid_stone);

    // Closing again has nothing left to return, and the result slot the
    // creative write refused never held anything to begin with.
    let reclosed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClose(ContainerCloseRequest { container_id: 0 }),
    );
    assert!(reclosed.events.is_empty(), "{reclosed:?}");
}

#[test]
fn a_click_the_client_did_not_predict_comes_back_as_the_slots_it_changed() {
    // `handleContainerClick` (:1915-1951) runs the click with remote updates
    // suppressed, stamps the client's own prediction into the remote slots and
    // only then broadcasts. This click predicts nothing, so both halves of the
    // pickup come back: the emptied slot and the loaded cursor.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);

    // Menu slot 36 is the first hotbar slot, which the fixture stocks. The
    // join's `sendAllDataToRemote` is the only thing that has stamped a state
    // id so far, so the client's copy of it is 1.
    let state_id = 1;
    let clicked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: 0,
            state_id,
            slot_num: 36,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );

    let fixture_stone = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 64,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    assert!(
        matches!(
            clicked.events.as_slice(),
            [
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::ContainerSetSlot(slot),
                },
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::SetCursorItem(cursor),
                },
            ] if slot.container_id == 0
                && slot.slot == 36
                && slot.item == ItemStackSummary::empty()
                && cursor.item == fixture_stone
        ),
        "{clicked:?}"
    );
    assert_eq!(
        clicked.snapshot.players[0].inventory.slots[0],
        ItemStackSummary::empty()
    );

    // A click that names a container this world never opened is ignored
    // outright, and so is one whose slot the menu does not have.
    for (container_id, slot_num) in [(7, 36), (0, MENU_SLOT_COUNT)] {
        let ignored = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::ContainerClick(ContainerClick {
                container_id,
                state_id,
                slot_num,
                button_num: 0,
                input: ContainerInput::Pickup,
                changed_slots: BTreeMap::new(),
                carried_item: HashedStack::empty(),
            }),
        );
        assert!(ignored.events.is_empty(), "{container_id}/{slot_num}");
    }

    // A stale state id means the two sides no longer agree on where the menu
    // is, so vanilla stops trusting the prediction and resends the whole menu
    // (:1943-1947) instead of the slots the click changed. Cloning with a
    // loaded cursor changes nothing (:512-518), so the resend is the only
    // thing this click produces.
    let resynced = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: state_id - 1,
            slot_num: 9,
            button_num: 0,
            input: ContainerInput::Clone,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );
    assert!(
        matches!(
            resynced.events.as_slice(),
            [WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::ContainerSetContent(content),
            }] if content.container_id == 0
                && content.items.len() == usize::try_from(MENU_SLOT_COUNT).unwrap()
                && content.carried_item == fixture_stone
        ),
        "{resynced:?}"
    );

    // Closing the menu hands the cursor back rather than losing it.
    let closed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClose(ContainerCloseRequest { container_id: 0 }),
    );
    assert_eq!(closed.snapshot.players[0].inventory.slots[0], fixture_stone);
}

#[test]
fn the_button_and_slot_state_of_a_menu_this_world_never_opens_change_nothing() {
    // `clickMenuButton` is false for every menu that does not override it
    // (AbstractContainerMenu.java:301-303), and
    // `handleContainerSlotStateChanged` (:1033-1040) only acts on a
    // `CrafterMenu`. The player inventory is neither, so both packets are
    // accepted and leave the session exactly as it was.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    for packet in [
        PlayServerbound::ContainerButtonClick(ContainerButtonClick {
            container_id: 0,
            button_id: 3,
        }),
        PlayServerbound::ContainerSlotStateChanged(ContainerSlotStateChanged {
            slot_id: 1,
            container_id: 0,
            new_state: true,
        }),
    ] {
        let result = play(&sender, &receiver, &mut world, 1, packet.clone());
        assert!(result.events.is_empty(), "{packet:?} produced {result:?}");
        assert_eq!(result.snapshot.players.len(), 1, "{packet:?} closed play");
    }
}

#[test]
fn chat_inventory_and_blocks_are_authoritative_for_two_clients_and_reconnect() {
    let (sender, receiver) = mailbox();
    for (connection_id, name, uuid) in [(1, "alice", 1), (2, "bob", 2)] {
        enqueue(
            &sender,
            WorldCommand::Join {
                client_information: Default::default(),
                connection_id: ConnectionId(connection_id),
                profile: profile(name, uuid),
                context: JoinContext::default(),
            },
        )
        .unwrap();
    }
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    let joined = world.tick(receiver.drain());
    let alice_position = joined
        .snapshot
        .players
        .iter()
        .find(|player| player.connection_id == ConnectionId(1))
        .expect("alice joined")
        .position;
    let bob_position = joined
        .snapshot
        .players
        .iter()
        .find(|player| player.connection_id == ConnectionId(2))
        .expect("bob joined")
        .position;
    let teleport_ids = joined
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::PlayerPosition(update),
            } => Some((*connection_id, update.id)),
            _ => None,
        })
        .collect::<Vec<_>>();
    for (connection_id, teleport_id) in teleport_ids {
        enqueue(
            &sender,
            WorldCommand::Play {
                connection_id,
                packet: PlayServerbound::AcceptTeleportation { id: teleport_id },
            },
        )
        .unwrap();
        enqueue(
            &sender,
            WorldCommand::Play {
                connection_id,
                packet: PlayServerbound::PlayerLoaded,
            },
        )
        .unwrap();
    }
    let loaded = world.tick(receiver.drain());
    assert_eq!(
        loaded
            .events
            .iter()
            .flat_map(WorldEvent::deliveries)
            .filter(|(_, packet)| matches!(packet, ClientboundPlayPacket::TrackedWaypoint(_)))
            .count(),
        2
    );
    assert!(loaded.events.iter().all(|event| match event.sent_packet() {
        Some(ClientboundPlayPacket::TrackedWaypoint(_)) => true,
        Some(ClientboundPlayPacket::UpdateAttributes(update)) =>
        // The creative ranges `updatePlayerAttributes` adds (ServerPlayer.java:610-627);
        // `removeFrost` skips materializing default movement speed while frost ticks are 0.
            update.attributes.len() == 2
                && update.attributes.iter().all(|attribute| {
                    use bbb_protocol::attribute_types::{
                        VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID as BLOCK,
                        VANILLA_ATTRIBUTE_ENTITY_INTERACTION_RANGE_ID as ENTITY,
                    };
                    matches!(attribute.attribute_id, BLOCK | ENTITY)
                        && attribute.modifiers.len() == 1
                }),
        _ => false,
    }));
    for connection_id in [ConnectionId(1), ConnectionId(2)] {
        let entity_id = world.player_entity_id_for_test(connection_id);
        assert!(loaded.events.iter().any(|event| matches!(
            event.packet_for(connection_id),
            Some(ClientboundPlayPacket::UpdateAttributes(update)) if update.entity_id == entity_id
        )));
    }
    // The join tick's travel moved each new body by its zero delta and left
    // the ground alone (`Entity.java:757`); the handshake tick's found the
    // floor, and each tracker answers that flip with an absolute sync
    // (ServerEntity.java:120-152).
    let settled = world.tick(Vec::new());
    let settled_deliveries = settled
        .events
        .iter()
        .flat_map(WorldEvent::deliveries)
        .collect::<Vec<_>>();
    assert_eq!(settled_deliveries.len(), 2, "{settled:?}");
    assert!(settled_deliveries.iter().all(|(_, packet)| matches!(
        packet,
        ClientboundPlayPacket::EntityPositionSync(sync) if sync.on_ground
    )));

    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::Chat(ChatMessage::unsigned("hello fixture", 42, 7)),
        },
    )
    .unwrap();
    let chat = world.tick(receiver.drain());
    let chat_deliveries = chat
        .events
        .iter()
        .flat_map(WorldEvent::deliveries)
        .collect::<Vec<_>>();
    assert_eq!(chat_deliveries.len(), 2);
    // An unsigned player chat bound to `minecraft:chat` over alice's name.
    let chat_type = world
        .synchronized_registries()
        .registry_element_index("minecraft:chat_type", "minecraft:chat")
        .unwrap();
    let name = bbb_protocol::RichTextComponent::try_from_component_nbt(
        crate::commands::player_display_name_nbt(&crate::commands::CommandPlayerView {
            name: "alice".into(),
            uuid: Uuid::from_u128(1),
        }),
    )
    .unwrap();
    for (index, connection_id) in [ConnectionId(1), ConnectionId(2)].into_iter().enumerate() {
        let (recipient, ClientboundPlayPacket::PlayerChat(message)) = chat_deliveries[index] else {
            panic!("player chat: {:?}", chat_deliveries[index]);
        };
        assert_eq!(recipient, connection_id);
        assert_eq!(message.global_index, 0);
        assert_eq!(message.sender, Uuid::from_u128(1));
        assert_eq!(message.content, "hello fixture");
        assert_eq!(message.unsigned_content, None);
        assert_eq!(message.chat_type.chat_type_id, chat_type);
        assert_eq!(message.chat_type.name, name);
        assert_eq!(message.chat_type.target_name, None);
    }

    let selected_stone = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 32,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
                slot_num: 37,
                item: selected_stone.clone(),
            }),
        },
    )
    .unwrap();
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::SetCarriedItem { slot: 1 },
        },
    )
    .unwrap();
    let inventory = world.tick(receiver.drain());
    // `handleSetCreativeModeSlot` writes the slot and the menu's remote mirror
    // of it in the same breath (:2013-2015), so the broadcast that follows has
    // no inventory echo to send; the observer still receives the changed hand.
    assert!(
        matches!(inventory.events.as_slice(),
        [WorldEvent::Send { connection_id: ConnectionId(2), packet: ClientboundPlayPacket::SetEquipment(update) }]
        if update.entity_id == entity_id_of(&world, 1)
            && update.slots.len() == 1
            && update.slots[0].slot == bbb_protocol::equipment_slots::EquipmentSlot::MainHand
            && update.slots[0].item == selected_stone),
        "{:?}",
        inventory.events
    );
    assert_eq!(inventory.snapshot.players[0].inventory.selected_slot, 1);
    assert_eq!(
        inventory.snapshot.players[0].inventory.slots[1],
        selected_stone
    );

    let air = world.fixture().air_state_id();
    let surface_blocks = |world: &mut ServerWorld, position: Vec3d| {
        let foot = BlockPos {
            x: position.x.floor() as i32,
            y: position.y.floor() as i32 - 1,
            z: position.z.floor() as i32,
        };
        [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)]
            .into_iter()
            .filter_map(|(dx, dz)| {
                let support = BlockPos {
                    x: foot.x + dx,
                    z: foot.z + dz,
                    ..foot
                };
                let above = BlockPos {
                    y: support.y + 1,
                    ..support
                };
                (live_block(world, support).is_some_and(|state| state != air)
                    && live_block(world, above) == Some(air))
                .then_some(support)
            })
            .collect::<Vec<_>>()
    };
    let broken = *surface_blocks(&mut world, alice_position)
        .first()
        .expect("alice has a reachable surface block");
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                pos: broken,
                direction: Direction::Up,
                sequence: 3,
            }),
        },
    )
    .unwrap();
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::PlayerAction(PlayerAction {
                action: PlayerActionKind::StopDestroyBlock,
                pos: broken,
                direction: Direction::Up,
                sequence: 5,
            }),
        },
    )
    .unwrap();
    let destroyed = world.tick(receiver.drain());
    assert_eq!(
        destroyed
            .events
            .iter()
            .flat_map(WorldEvent::deliveries)
            .filter(|(_, packet)| matches!(
                packet,
                ClientboundPlayPacket::BlockUpdate(update)
                    if update.pos == broken
                        && update.block_state_id == world.fixture().air_state_id()
            ))
            .count(),
        2
    );
    assert!(matches!(
        destroyed.events.last(),
        Some(WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::BlockChangedAck(update),
        }) if update.sequence == 5
    ));

    // Bob places what he holds, and his session joined holding nothing.
    stock_fixture_stack(&mut world, 2);
    let clicked = surface_blocks(&mut world, bob_position)
        .into_iter()
        .find(|pos| *pos != broken)
        .expect("bob has a distinct reachable placement support");
    let placed = BlockPos {
        y: clicked.y + 1,
        ..clicked
    };
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(2),
            packet: PlayServerbound::UseItemOn(UseItemOn {
                hand: InteractionHand::MainHand,
                hit: BlockHitResult {
                    pos: clicked,
                    direction: Direction::Up,
                    cursor_x: 0.5,
                    cursor_y: 1.0,
                    cursor_z: 0.5,
                    inside: false,
                    world_border_hit: false,
                },
                sequence: 4,
            }),
        },
    )
    .unwrap();
    let placed_tick = world.tick(receiver.drain());
    assert_eq!(
        placed_tick
            .events
            .iter()
            .flat_map(WorldEvent::deliveries)
            .filter(|(_, packet)| matches!(
                packet,
                ClientboundPlayPacket::BlockUpdate(update)
                    if update.pos == placed
                        && update.block_state_id == world.fixture().stone_state_id()
            ))
            .count(),
        3
    );
    let acknowledgements = placed_tick
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::BlockChangedAck(update),
            } => Some(update.sequence),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(acknowledgements, [4]);
    assert!(placed_tick.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetEquipment(update) }
        if update.entity_id == entity_id_of(&world, 2)
            && update.slots.len() == 1
            && update.slots[0].slot == bbb_protocol::equipment_slots::EquipmentSlot::MainHand
            && update.slots[0].item == placed_tick.snapshot.players[1].inventory.slots[0])));
    assert_eq!(placed_tick.snapshot.changed_blocks.len(), 2);
    assert_eq!(placed_tick.snapshot.players[1].inventory.slots[0].count, 64);

    // What the first session counted, read off the save projection while it is
    // still online. `play_time` is the per-tick award every standing session
    // gets (`ServerPlayer.doTick`), so it is session progress that exists only
    // in memory until a save writes it.
    let before_leaving = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .expect("an online session projects a save")
        .player_patches
        .into_iter()
        .find(|patch| patch.uuid == Uuid::from_u128(1))
        .expect("alice is online")
        .stats;
    let play_time =
        bbb_server_types::stats::custom_stat(bbb_server_types::stats::CUSTOM_STAT_PLAY_TIME);
    let played = before_leaving.value(play_time);
    assert!(played > 0, "a standing session counts play_time");

    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(1),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    // `PlayerList.remove` counts the departure (`PlayerList.java:301`) and
    // saves the player (`:302`) before dropping the counter (`:326`), so the
    // save this server projects with nobody holding that profile still carries
    // everything the session earned. Nothing about a departure resets a
    // counter, so a stats file that lost its counts is a file this server
    // corrupted.
    let offline = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .expect("a departed session projects a save")
        .player_patches;
    let left = offline
        .iter()
        .find(|patch| patch.uuid == Uuid::from_u128(1))
        .expect("the departed profile owes its full playerdata snapshot a write");
    assert_eq!(left.stats.value(play_time), played);
    assert_eq!(
        left.stats.value(bbb_server_types::stats::custom_stat(
            bbb_server_types::stats::CUSTOM_STAT_LEAVE_GAME
        )),
        1,
    );

    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(3),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let rejoined = world.tick(receiver.drain());
    let rejoined_entity = entity_id_of(&world, 3);
    // `PlayerList.getPlayerStats` (`PlayerList.java:757-763`) builds the
    // returning session's counter off the file the departure wrote, so the
    // second session opens on the first one's totals rather than on zero --
    // and the save it projects therefore cannot overwrite the file with an
    // empty document.
    let after_returning = world
        .save_request(1_786_000_002_000, 202, RegionCompression::Zlib)
        .expect("the returning session projects a save")
        .player_patches
        .into_iter()
        .find(|patch| patch.uuid == Uuid::from_u128(1))
        .expect("alice is online again")
        .stats;
    assert!(
        after_returning.value(play_time) >= played,
        "the returning session opened on the counter the departure left behind"
    );
    assert_eq!(
        after_returning.value(bbb_server_types::stats::custom_stat(
            bbb_server_types::stats::CUSTOM_STAT_LEAVE_GAME
        )),
        1,
    );
    // The profile is online again, so its file is no longer owed a separate
    // write: the online patch is the one that carries it.
    assert!(!world
        .save_request(1_786_000_002_000, 202, RegionCompression::Zlib)
        .expect("the returning session projects a save")
        .departed_player_stats
        .contains_key(&Uuid::from_u128(1)));
    // `PlayerList.placeNewPlayer` (PlayerList.java:193-200) announces the
    // arrival to the list as it stands, so bob hears about alice and alice
    // does not hear about herself.
    assert!(rejoined.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::SystemChat(joined),
        } if joined.content.text().contains("multiplayer.player.joined")
    )));
    assert!(rejoined.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(3),
            packet: ClientboundPlayPacket::ContainerSetContent(_),
        }
    )));
    assert!(rejoined.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(3),
            packet: ClientboundPlayPacket::SetHeldSlot(_),
        }
    )));
    let rejoined_without_waypoints = rejoined
        .events
        .iter()
        .filter(|event| {
            !matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::TrackedWaypoint(_),
                    ..
                }
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    let info_position = |recipient, profile_id| {
        rejoined_without_waypoints
            .iter()
            .position(|event| {
                matches!(event,
                WorldEvent::Send {
                    connection_id,
                    packet: ClientboundPlayPacket::PlayerInfoUpdate(update),
                } if *connection_id == recipient
                    && update.entries.iter().any(|entry| entry.profile_id == profile_id))
            })
            .expect("the player-info entry is present")
    };
    let bob_to_alice = info_position(ConnectionId(3), Uuid::from_u128(2));
    let alice_to_bob = info_position(ConnectionId(2), Uuid::from_u128(1));
    let alice_to_self = info_position(ConnectionId(3), Uuid::from_u128(1));
    let alice_pairing = rejoined_without_waypoints
        .iter()
        .position(|event| {
            matches!(event,
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::AddEntity(joined),
            } if joined.uuid == Uuid::from_u128(1)
                && joined.entity_type_id
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID)
        })
        .expect("bob's delivered chunk admits rejoining alice");
    assert!([bob_to_alice, alice_to_bob, alice_to_self]
        .into_iter()
        .all(|info| info < alice_pairing));
    assert!(matches!(
        &rejoined_without_waypoints[alice_pairing - 1],
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::BundleDelimiter,
        }
    ));
    let pairing_end = rejoined_without_waypoints
        .iter()
        .enumerate()
        .skip(alice_pairing + 1)
        .find_map(|(index, event)| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(2),
                    packet: ClientboundPlayPacket::BundleDelimiter,
                }
            )
            .then_some(index)
        })
        .expect("alice's pairing bundle closes");
    assert!(rejoined_without_waypoints[alice_pairing + 1..pairing_end]
        .iter()
        .all(|event| matches!(
            event,
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::SetEntityData(_)
                    | ClientboundPlayPacket::UpdateAttributes(_)
                    | ClientboundPlayPacket::SetEquipment(_)
                    | ClientboundPlayPacket::SetPassengers(_),
            }
        )));
    let first_move = rejoined_without_waypoints
        .iter()
        .position(|event| {
            matches!(event,
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::MoveEntity(update),
            } if update.id == rejoined_entity
                && update.has_position
                && (update.delta_x, update.delta_y, update.delta_z) == (0, 0, 0))
        })
        .expect("the new tracker publishes its tick-zero position");
    assert!(first_move > alice_pairing);
    let batch_start = rejoined_without_waypoints
        .iter()
        .position(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(3),
                    packet: ClientboundPlayPacket::ChunkBatchStart,
                }
            )
        })
        .expect("the returning player's chunk batch starts");
    let batch_end = rejoined_without_waypoints
        .iter()
        .position(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(3),
                    packet: ClientboundPlayPacket::ChunkBatchFinished { batch_size: 9 },
                }
            )
        })
        .expect("the returning player's chunk batch finishes");
    assert!(rejoined_without_waypoints[batch_start + 1..batch_end]
        .iter()
        .all(|event| matches!(
            event,
            WorldEvent::Send {
                connection_id: ConnectionId(3),
                packet: ClientboundPlayPacket::LevelChunkWithLight(_),
            }
        )));
    assert_eq!(batch_end - batch_start - 1, 9);
    let tail = &rejoined_without_waypoints[batch_end + 1..];
    assert!(matches!(
        &tail[0],
        WorldEvent::Send {
            connection_id: ConnectionId(3),
            packet: ClientboundPlayPacket::BundleDelimiter,
        }
    ));
    let bob_entity = entity_id_of(&world, 2);
    assert!(matches!(&tail[1], WorldEvent::Send {
        connection_id: ConnectionId(3), packet: ClientboundPlayPacket::AddEntity(other),
    } if other.id == bob_entity && other.uuid == Uuid::from_u128(2)));
    assert!(bob_to_alice < batch_end + 2);
    let bob_pairing_end = tail
        .iter()
        .enumerate()
        .skip(2)
        .find_map(|(index, event)| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(3),
                    packet: ClientboundPlayPacket::BundleDelimiter,
                }
            )
            .then_some(index)
        })
        .expect("bob's pairing bundle closes");
    assert!(tail[2..bob_pairing_end].iter().all(|event| match event {
        WorldEvent::Send {
            connection_id: ConnectionId(3),
            packet,
        } => match packet {
            ClientboundPlayPacket::SetEntityData(update) => update.id == bob_entity,
            ClientboundPlayPacket::UpdateAttributes(update) => update.entity_id == bob_entity,
            ClientboundPlayPacket::SetEquipment(update) => update.entity_id == bob_entity,
            ClientboundPlayPacket::SetPassengers(update) => update.vehicle_id == bob_entity,
            _ => false,
        },
        _ => false,
    }));
    let tail = &tail[bob_pairing_end + 1..];
    // The rejoining session is a fresh `ServerPlayer`: its
    // `lastEquipmentItems` start empty (`LivingEntity.java:208`), so its first
    // tick tells the tracker about the restored hand once more, and its
    // `lastSentHealth` starts unset, so that tick announces the health it
    // restored with.
    assert!(matches!(
        &tail[0],
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::SetEquipment(update),
        } if update.entity_id == rejoined_entity
            && update.slots.len() == 1
            && update.slots[0].slot == bbb_protocol::equipment_slots::EquipmentSlot::MainHand
            && update.slots[0].item == selected_stone
    ));
    assert!(matches!(
        &tail[1],
        WorldEvent::Send {
            connection_id: ConnectionId(3),
            packet: ClientboundPlayPacket::SetHealth(_),
        }
    ));
    assert!(matches!(
        &tail[2],
        WorldEvent::Send {
            connection_id: ConnectionId(3),
            packet: ClientboundPlayPacket::SetExperience(_),
        }
    ));
    assert_eq!(tail.len(), 3);
    // Nothing replays the changed blocks: the chunk is encoded from the
    // authoritative column, so both edits are already inside it. That the
    // column is authoritative is what makes the replay redundant, so assert it
    // rather than the packets vanilla does not send.
    assert!(!rejoined.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(3),
            packet: ClientboundPlayPacket::BlockUpdate(_),
        }
    )));
    let changed = &rejoined.snapshot.changed_blocks;
    assert_eq!(changed.len(), 2);
    assert!(
        changed
            .iter()
            .any(|block| block.pos == broken
                && block.block_state_id == world.fixture().air_state_id())
    );
    assert!(changed.iter().any(
        |block| block.pos == placed && block.block_state_id == world.fixture().stone_state_id()
    ));
}

fn write_player_craft_slots(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
    slots: &[(i16, i32)],
) -> TickResult {
    let mut last = None;
    for &(slot_num, item_id) in slots {
        last = Some(play(
            sender,
            receiver,
            world,
            connection_id,
            PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
                slot_num,
                item: inventory_test_stack(item_id, 1),
            }),
        ));
    }
    last.expect("at least one crafting-grid write")
}

fn result_slot_syncs(events: &[WorldEvent], connection_id: u64, item_id: i32) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::Send {
                connection_id: ConnectionId(id),
                packet: ClientboundPlayPacket::ContainerSetSlot(slot),
            } if *id == connection_id
                && slot.container_id == 0
                && slot.slot == 0
                && slot.item.item_id == Some(item_id)
        )
    })
}

#[test]
fn joining_with_datapacks_rematches_the_2x2_crafting_grid() {
    use bbb_protocol::item_types::{
        VANILLA_ITEM_FLINT_AND_STEEL_ID, VANILLA_ITEM_FLINT_ID, VANILLA_ITEM_IRON_INGOT_ID,
    };
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let matched = write_player_craft_slots(
        &sender,
        &receiver,
        &mut world,
        1,
        &[(1, VANILLA_ITEM_IRON_INGOT_ID), (2, VANILLA_ITEM_FLINT_ID)],
    );
    assert_eq!(
        world
            .player_crafting_result_for_test(ConnectionId(1))
            .item_id,
        Some(VANILLA_ITEM_FLINT_AND_STEEL_ID)
    );
    assert!(
        result_slot_syncs(&matched.events, 1, VANILLA_ITEM_FLINT_AND_STEEL_ID),
        "{matched:?}"
    );

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 1,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );
    assert!(world.player_knows_recipe_for_test(ConnectionId(1), "minecraft:flint_and_steel"));
}

#[test]
fn joining_with_datapacks_rematches_tagged_2x2_planks() {
    use bbb_protocol::item_types::{VANILLA_ITEM_CRAFTING_TABLE_ID, VANILLA_ITEM_OAK_PLANKS_ID};
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let matched = write_player_craft_slots(
        &sender,
        &receiver,
        &mut world,
        1,
        &[
            (1, VANILLA_ITEM_OAK_PLANKS_ID),
            (2, VANILLA_ITEM_OAK_PLANKS_ID),
            (3, VANILLA_ITEM_OAK_PLANKS_ID),
            (4, VANILLA_ITEM_OAK_PLANKS_ID),
        ],
    );
    assert_eq!(
        world
            .player_crafting_result_for_test(ConnectionId(1))
            .item_id,
        Some(VANILLA_ITEM_CRAFTING_TABLE_ID)
    );
    assert!(
        result_slot_syncs(&matched.events, 1, VANILLA_ITEM_CRAFTING_TABLE_ID),
        "{matched:?}"
    );
}

#[test]
fn joining_without_datapacks_leaves_the_2x2_unmatched() {
    use bbb_protocol::item_types::{VANILLA_ITEM_FLINT_ID, VANILLA_ITEM_IRON_INGOT_ID};
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    write_player_craft_slots(
        &sender,
        &receiver,
        &mut world,
        1,
        &[(1, VANILLA_ITEM_IRON_INGOT_ID), (2, VANILLA_ITEM_FLINT_ID)],
    );
    assert!(world
        .player_crafting_result_for_test(ConnectionId(1))
        .item_id
        .is_none());
}

#[test]
fn join_installs_player_attribute_supplier() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let attributes = world.player_attributes_for_test(ConnectionId(1));
    assert_eq!(
        attributes.value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID),
        Some(20.0)
    );
    // `updatePlayerAttributes` (ServerPlayer.java:610-637) materializes both
    // interaction ranges; default movement speed stays uninstantiated in the supplier.
    let packed = attributes.pack();
    assert_eq!(packed.len(), 3);
    let waypoint = packed
        .iter()
        .find(|attribute| {
            attribute.attribute_id
                == bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_WAYPOINT_TRANSMIT_RANGE_ID
        })
        .unwrap();
    assert_eq!(
        waypoint.attribute_id,
        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_WAYPOINT_TRANSMIT_RANGE_ID
    );
    assert_eq!(waypoint.base_value, 60_000_000.0);
    assert!(waypoint.modifiers.is_empty());
    assert_eq!(
        attributes.value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID),
        Some(f64::from(0.1_f32))
    );
}

/// A creative write into the 2x2 grid publishes the recomputed result once.
///
/// `Slot.setByPlayer` reaches `TransientCraftingContainer.setItem`
/// (`TransientCraftingContainer.java:64-67`), whose `slotsChanged` runs
/// `slotChangedCraftingGrid`; the result slot is sent whatever it holds
/// (`CraftingMenu.java:70-72`), and the write's own `setRemoteSlot`
/// (`ServerGamePacketListenerImpl.java:2012-2014`) leaves the grid slot for the
/// `broadcastChanges` after it to find unchanged.
#[test]
fn a_creative_write_into_the_grid_publishes_the_result_slot() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(startup_generation()),
        ..creative_fixture_config()
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let written = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 1,
            item: ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_OAK_PLANKS_ID),
                count: 1,
                component_patch: Default::default(),
            },
        }),
    );
    let slots: Vec<i16> = written
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::ContainerSetSlot(update),
            } => Some(update.slot),
            _ => None,
        })
        .collect();
    // One oak plank is the whole of `minecraft:oak_button`
    // (data/minecraft/recipe/oak_button.json).
    assert_eq!(slots, [0], "{written:?}");
    assert_eq!(
        world
            .player_crafting_result_for_test(ConnectionId(1))
            .item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_OAK_BUTTON_ID)
    );
}

/// `ArmorSlot.mayPlace` over `LivingEntity.isEquippableInSlot`
/// (`LivingEntity.java:3821-3826`): a helmet whose `allowed_entities` leaves the
/// player out does not go onto the head; one naming the player does.
#[test]
fn an_armor_slot_respects_the_stacks_allowed_entities() {
    for (allowed, worn) in [("minecraft:zombie", false), ("minecraft:player", true)] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(datapack_config());
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let given = crate::tests::console_command(
            &sender,
            &receiver,
            &mut world,
            20,
            &format!(
                "give alice diamond_helmet[equippable={{slot:\"head\",allowed_entities:\"{allowed}\"}}]"
            ),
        );
        assert!(crate::tests::console_command_answer(&given).0.is_some());
        let state_id = world.player_inventory_for_test(ConnectionId(1)).state_id();
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::ContainerClick(ContainerClick {
                container_id: 0,
                state_id,
                slot_num: 36,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: BTreeMap::new(),
                carried_item: HashedStack::empty(),
            }),
        );
        let head = world
            .player_inventory_for_test(ConnectionId(1))
            .slot_item(5)
            .item_id();
        assert_eq!(
            head == Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_HELMET_ID),
            worn,
            "{allowed}"
        );
    }
}
