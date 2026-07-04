use super::*;

#[test]
fn transfer_event_updates_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Transfer(bbb_protocol::packets::Transfer {
        host: "next.example.com".to_string(),
        port: 25566,
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(
        world.last_transfer(),
        Some(&bbb_world::TransferTargetState {
            host: "next.example.com".to_string(),
            port: 25566,
        })
    );
    assert_eq!(world.counters().transfer_packets, 1);
}

#[test]
fn cookie_events_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::StoreCookie {
        key: "bbb:session".to_string(),
        payload_len: 3,
        stored_cookie_count: 1,
    })
    .unwrap();
    tx.try_send(NetEvent::CookieRequest {
        key: "bbb:session".to_string(),
        response_payload_present: true,
    })
    .unwrap();
    tx.try_send(NetEvent::CookieRequest {
        key: "bbb:missing".to_string(),
        response_payload_present: false,
    })
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );
    assert_eq!(world.last_cookie_key(), Some("bbb:missing"));
    assert_eq!(world.stored_cookie_count(), 1);
    let world_counters = world.counters();
    assert_eq!(world_counters.store_cookie_packets, 1);
    assert_eq!(world_counters.stored_cookie_bytes, 3);
    assert_eq!(world_counters.cookie_request_packets, 2);
    assert_eq!(world_counters.cookie_response_hits, 1);
    assert_eq!(world_counters.cookie_response_misses, 1);
}

#[test]
fn custom_report_details_event_updates_world_counters() {
    let details = BTreeMap::from([
        ("Region".to_string(), "local".to_string()),
        ("Server".to_string(), "bbb test shard".to_string()),
    ]);
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::CustomReportDetails(
        bbb_protocol::packets::CustomReportDetails {
            details: details.clone(),
        },
    ))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(world.custom_report_details(), &details);
    assert_eq!(world.counters().custom_report_detail_packets, 1);
    assert_eq!(world.counters().custom_report_details_tracked, 2);
}

#[test]
fn award_stats_event_updates_world_state_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::AwardStats(AwardStats {
        stats: vec![
            StatUpdate {
                stat_type_id: 8,
                value_id: 10,
                amount: 3,
            },
            StatUpdate {
                stat_type_id: 0,
                value_id: 4,
                amount: 11,
            },
        ],
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(world.stat_value(8, 10), Some(3));
    assert_eq!(world.stat_value(0, 4), Some(11));
    assert_eq!(world.counters().award_stats_packets, 1);
    assert_eq!(world.counters().award_stats_entries_received, 2);
    assert_eq!(world.counters().last_award_stats_entry_count, 2);
    assert_eq!(world.counters().stats_tracked, 2);
    assert_eq!(
        world.last_stats_update(),
        Some(&bbb_world::StatsUpdateState {
            entries: vec![
                bbb_world::StatValueState {
                    stat_type_id: 8,
                    value_id: 10,
                    amount: 3,
                },
                bbb_world::StatValueState {
                    stat_type_id: 0,
                    value_id: 4,
                    amount: 11,
                },
            ],
        })
    );
}

#[test]
fn configuration_state_events_update_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::UpdateEnabledFeatures(
        bbb_protocol::packets::UpdateEnabledFeatures {
            features: vec![
                "minecraft:minecart_improvements".to_string(),
                "minecraft:vanilla".to_string(),
            ],
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::SelectKnownPacks {
        known_packs: vec![bbb_protocol::packets::KnownPack {
            namespace: "minecraft".to_string(),
            id: "core".to_string(),
            version: "26.1".to_string(),
        }],
        selected_packs: Vec::new(),
    })
    .unwrap();
    tx.try_send(NetEvent::ResetChat).unwrap();
    tx.try_send(NetEvent::CodeOfConduct {
        text: "Keep the server friendly.".to_string(),
    })
    .unwrap();

    let mut world = WorldStore::new();
    let _ = world.apply_player_chat(PlayerChat {
        global_index: 0,
        sender: Uuid::from_u128(1),
        index: 0,
        signature: Some(MessageSignature {
            bytes: vec![7; 256],
        }),
        body: SignedMessageBody {
            content: "previous".to_string(),
            timestamp_millis: 1,
            salt: 2,
            last_seen: Vec::new(),
        },
        unsigned_content: None,
        filter_mask: FilterMask {
            kind: FilterMaskKind::PassThrough,
            mask_words: Vec::new(),
        },
        chat_type: ChatTypeBound {
            chat_type: ChatTypeHolder::Registry { id: 0 },
            name: "Alice".to_string(),
            target_name: None,
        },
    });
    assert_eq!(world.counters().chat_messages_tracked, 1);
    assert_eq!(world.counters().chat_signature_cache_entries, 1);
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        4
    );
    assert_eq!(
        world.enabled_feature_list(),
        vec![
            "minecraft:minecart_improvements".to_string(),
            "minecraft:vanilla".to_string(),
        ]
    );
    assert!(world.is_feature_enabled("minecraft:vanilla"));
    assert_eq!(world.counters().update_enabled_features_packets, 1);
    assert_eq!(world.counters().enabled_features_tracked, 2);
    assert_eq!(world.counters().enabled_features_ignored, 0);
    assert_eq!(world.known_packs().offered.len(), 1);
    assert_eq!(world.known_packs().offered[0].namespace, "minecraft");
    assert_eq!(world.known_packs().offered[0].id, "core");
    assert_eq!(world.known_packs().offered[0].version, "26.1");
    assert!(world.known_packs().selected.is_empty());
    assert_eq!(world.counters().select_known_packs_packets, 1);
    assert_eq!(world.counters().known_packs_offered, 1);
    assert_eq!(world.counters().known_packs_selected, 0);
    assert!(world.client_chat().messages.is_empty());
    assert!(world.client_chat().deleted_messages.is_empty());
    assert_eq!(world.client_chat().expected_player_chat_global_index, 0);
    assert_eq!(world.counters().reset_chat_packets, 1);
    assert_eq!(world.counters().chat_messages_tracked, 0);
    assert_eq!(world.counters().deleted_chat_messages_tracked, 0);
    assert_eq!(world.counters().chat_signature_cache_entries, 0);
    assert_eq!(
        world.last_code_of_conduct(),
        Some(&bbb_world::CodeOfConductState {
            text: "Keep the server friendly.".to_string(),
            text_hash: bbb_world::code_of_conduct_text_hash("Keep the server friendly."),
        })
    );
    assert_eq!(world.counters().code_of_conduct_packets, 1);
    assert_eq!(
        world.counters().last_code_of_conduct_len,
        "Keep the server friendly.".len()
    );
}

#[test]
fn registry_data_event_updates_world_state_and_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::RegistryData(RegistryData {
        registry: "minecraft:chat_type".to_string(),
        raw_payload_len: 96,
        entries: vec![
            RegistryDataEntry {
                id: "minecraft:chat".to_string(),
                raw_data: Some(vec![10; 24]),
            },
            RegistryDataEntry {
                id: "minecraft:raw".to_string(),
                raw_data: None,
            },
        ],
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    let packet = &world.registries().registries[0];
    assert_eq!(packet.name, "minecraft:chat_type");
    assert_eq!(packet.raw_payload_len, 96);
    assert_eq!(
        packet.entries,
        vec![
            RegistryPacketEntry::with_raw_data("minecraft:chat", vec![10; 24]),
            RegistryPacketEntry::stub("minecraft:raw"),
        ]
    );
    assert_eq!(world.counters().registries_seen, 1);
    assert_eq!(world.counters().registry_entries_seen, 2);
    assert_eq!(world.counters().registry_entries_with_data, 1);
    assert_eq!(world.counters().registry_entry_stubs, 1);
    assert_eq!(world.counters().registry_entry_payload_bytes, 24);
    assert_eq!(world.counters().registry_content_registries_tracked, 1);
    assert_eq!(world.counters().registry_content_packets_tracked, 1);
    assert_eq!(world.counters().registry_content_entries_tracked, 2);
    assert_eq!(world.counters().registry_duplicate_entries, 0);
    assert_eq!(world.counters().registry_duplicate_entry_ids_tracked, 0);
    assert_eq!(
        world.counters().last_registry_data_registry.as_deref(),
        Some("minecraft:chat_type")
    );
    assert_eq!(world.counters().last_registry_data_entry_count, 2);
}

#[test]
fn update_tags_event_updates_world_state_and_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::UpdateTags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:logs".to_string(),
                entries: vec![5, 6, 7],
            }],
        }],
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(
        world.registry_tags("minecraft:item").unwrap().tags["minecraft:logs"],
        vec![5, 6, 7]
    );
    assert_eq!(world.counters().update_tags_packets, 1);
    assert_eq!(world.counters().last_update_tags_registry_count, 1);
    assert_eq!(world.counters().last_update_tags_total_tag_count, 1);
    assert_eq!(world.counters().last_update_tags_total_value_count, 3);
    assert_eq!(world.counters().tag_registries_tracked, 1);
    assert_eq!(world.counters().tags_tracked, 1);
    assert_eq!(world.counters().tag_entries_tracked, 3);
}

#[test]
fn server_links_event_updates_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::ServerLinks(ServerLinks {
        links: vec![
            ServerLinkEntry {
                link_type: ServerLinkType::Known(ServerLinkKnownType::Support),
                url: "https://example.invalid/support".to_string(),
            },
            ServerLinkEntry {
                link_type: ServerLinkType::Custom {
                    label: "Rules".to_string(),
                },
                url: "http://example.invalid/rules".to_string(),
            },
            ServerLinkEntry {
                link_type: ServerLinkType::Known(ServerLinkKnownType::Website),
                url: "ftp://example.invalid/file".to_string(),
            },
        ],
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(
        world.server_links(),
        &[
            bbb_world::ServerLinkState {
                label: "known_server_link.support".to_string(),
                url: "https://example.invalid/support".to_string(),
                known_type: Some("support".to_string()),
            },
            bbb_world::ServerLinkState {
                label: "Rules".to_string(),
                url: "http://example.invalid/rules".to_string(),
                known_type: None,
            },
        ]
    );
    let world_counters = world.counters();
    assert_eq!(world_counters.server_link_packets, 1);
    assert_eq!(world_counters.server_link_invalid_entries, 1);
    assert_eq!(world_counters.server_links_tracked, 2);
}

#[test]
fn client_ui_events_update_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::LowDiskSpaceWarning))
        .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::MountScreenOpen(
        MountScreenOpen {
            container_id: 11,
            inventory_columns: 5,
            entity_id: 42,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::OpenBook(OpenBook {
        hand: InteractionHand::OffHand,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::OpenSignEditor(
        OpenSignEditor {
            pos: ProtocolBlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            is_front_text: false,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::PongResponse(
        PongResponse { time: 123456789 },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        5
    );
    assert_eq!(world.low_disk_space_warning_count(), 1);
    assert_eq!(
        world.last_mount_screen(),
        Some(&bbb_world::MountScreenState {
            container_id: 11,
            inventory_columns: 5,
            entity_id: 42,
        })
    );
    assert_eq!(
        world.last_open_book(),
        Some(&bbb_world::OpenBookState {
            hand: "off_hand".to_string(),
        })
    );
    assert_eq!(
        world.last_open_sign_editor(),
        Some(&bbb_world::OpenSignEditorState {
            pos: BlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            is_front_text: false,
        })
    );
    assert_eq!(
        world.last_pong_response(),
        Some(&bbb_world::PongResponseState { time: 123456789 })
    );

    let world_counters = world.counters();
    assert_eq!(world_counters.low_disk_space_warnings, 1);
    assert_eq!(world_counters.mount_screen_open_packets, 1);
    assert_eq!(world_counters.open_book_packets, 1);
    assert_eq!(world_counters.open_sign_editor_packets, 1);
    assert_eq!(world_counters.pong_response_packets, 1);
}

#[test]
fn open_book_event_uses_held_written_book_for_active_screen() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::SetPlayerInventory(
        SetPlayerInventory {
            slot: 40,
            item: written_book_stack(vec!["Native first", "Native second"]),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::OpenBook(OpenBook {
        hand: InteractionHand::OffHand,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    assert_eq!(
        world.last_open_book(),
        Some(&bbb_world::OpenBookState {
            hand: "off_hand".to_string(),
        })
    );
    assert_eq!(
        world.current_book(),
        Some(&bbb_world::BookScreenState {
            hand: "off_hand".to_string(),
            pages: vec!["Native first".to_string(), "Native second".to_string()],
            current_page: 0,
        })
    );
}

#[test]
fn map_item_data_event_updates_world_state() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::MapItemData(MapItemData {
        map_id: 42,
        scale: 2,
        locked: true,
        decorations: Some(vec![MapDecoration {
            type_id: 4,
            x: -20,
            y: 30,
            rot: 7,
            name: Some("Village".to_string()),
        }]),
        color_patch: Some(MapColorPatch {
            start_x: 3,
            start_y: 4,
            width: 2,
            height: 2,
            colors: vec![1, 2, 3, 4],
        }),
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    let map = world.map_item(42).expect("map state is tracked");
    assert_eq!(map.scale, 2);
    assert!(map.locked);
    assert_eq!(map.decorations.len(), 1);
    assert_eq!(map.decorations[0].name.as_deref(), Some("Village"));
    assert_eq!(map.colors[3 + 4 * 128], 1);
    assert_eq!(map.colors[4 + 5 * 128], 4);

    let world_counters = world.counters();
    assert_eq!(world_counters.map_item_data_packets, 1);
    assert_eq!(world_counters.maps_tracked, 1);
    assert_eq!(world_counters.map_decorations_tracked, 1);
    assert_eq!(world_counters.map_color_patches_applied, 1);
    assert_eq!(world_counters.map_color_patches_ignored, 0);
    assert_eq!(
        world.last_map_color_patch(),
        Some(&bbb_world::LastMapColorPatchState {
            map_id: 42,
            start_x: 3,
            start_y: 4,
            width: 2,
            height: 2,
        })
    );
}

#[test]
fn take_item_entity_event_updates_world_counter() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::TakeItemEntity(
        bbb_protocol::packets::TakeItemEntity {
            item_id: 10,
            player_id: 20,
            amount: 3,
        },
    )))
    .unwrap();
    drop(tx);

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(world.counters().take_item_entities_received, 1);
    assert_eq!(world.counters().take_item_entities_applied, 0);
    assert_eq!(world.counters().take_item_entities_ignored, 1);
    assert_eq!(world.counters().item_entity_stack_shrinks, 0);
    assert_eq!(world.counters().take_item_entities_removed, 0);
}

#[test]
fn take_item_entity_event_emits_pickup_sounds() {
    let (tx, mut rx) = mpsc::channel(7);
    let add_entity_at = |id: i32, entity_type_id: i32, position: ProtocolVec3d| {
        let mut entity = protocol_add_entity_with_type(id, entity_type_id);
        entity.position = position;
        PlayClientbound::AddEntity(entity)
    };
    for packet in [
        add_entity_at(
            10,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            ProtocolVec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
        ),
        add_entity_at(
            20,
            VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
            ProtocolVec3d {
                x: 4.0,
                y: 70.0,
                z: 8.0,
            },
        ),
        add_entity_at(
            30,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            ProtocolVec3d {
                x: -6.0,
                y: 65.0,
                z: 9.0,
            },
        ),
        PlayClientbound::TakeItemEntity(bbb_protocol::packets::TakeItemEntity {
            item_id: 10,
            player_id: 99,
            amount: 1,
        }),
        PlayClientbound::TakeItemEntity(bbb_protocol::packets::TakeItemEntity {
            item_id: 20,
            player_id: 99,
            amount: 1,
        }),
        PlayClientbound::TakeItemEntity(bbb_protocol::packets::TakeItemEntity {
            item_id: 30,
            player_id: 99,
            amount: 1,
        }),
        PlayClientbound::TakeItemEntity(bbb_protocol::packets::TakeItemEntity {
            item_id: 404,
            player_id: 99,
            amount: 1,
        }),
    ] {
        tx.try_send(NetEvent::Play(packet)).unwrap();
    }
    drop(tx);

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_item_pitch =
        (expected_random.next_float() - expected_random.next_float()) * 1.4 + 2.0;
    let expected_orb_pitch =
        (expected_random.next_float() - expected_random.next_float()) * 0.35 + 0.9;
    let expected_zombie_pitch =
        (expected_random.next_float() - expected_random.next_float()) * 1.4 + 2.0;

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        7
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 3);
    let expected = [
        (
            "minecraft:entity.item.pickup",
            AudioCategory::Players,
            [1.0, 64.0, -2.0],
            0.2,
            expected_item_pitch,
        ),
        (
            "minecraft:entity.experience_orb.pickup",
            AudioCategory::Players,
            [4.0, 70.0, 8.0],
            0.1,
            expected_orb_pitch,
        ),
        (
            "minecraft:entity.item.pickup",
            AudioCategory::Players,
            [-6.0, 65.0, 9.0],
            0.2,
            expected_zombie_pitch,
        ),
    ];
    for (command, (event_id, category, position, volume, pitch)) in
        audio.commands.iter().zip(expected)
    {
        let AudioCommand::PlayPositionedSound(command) = command else {
            panic!("expected positioned sound, got {command:?}");
        };
        assert_eq!(command.sound.event_id, event_id);
        assert_eq!(command.category, category);
        assert_eq!(command.position, position);
        assert_close(command.packet_volume, volume);
        assert_close(command.packet_pitch, pitch);
        assert_eq!(command.seed, 0);
        assert!(!command.distance_delay);
    }
    assert_eq!(world.counters().take_item_entities_received, 4);
    assert_eq!(world.counters().take_item_entities_applied, 3);
    assert_eq!(world.counters().take_item_entities_ignored, 1);
    assert_eq!(
        world.last_sound().unwrap().sound.location.as_deref(),
        Some("minecraft:entity.item.pickup")
    );
}

#[test]
fn take_item_entity_event_emits_pickup_particle_state_before_removal() {
    let (tx, mut rx) = mpsc::channel(5);
    let mut item = protocol_add_entity_with_type(10, VANILLA_ENTITY_TYPE_ITEM_ID);
    item.position = ProtocolVec3d {
        x: 1.0,
        y: 64.0,
        z: -2.0,
    };
    item.delta_movement = ProtocolVec3d {
        x: 0.1,
        y: 0.2,
        z: -0.3,
    };
    let mut target = protocol_add_entity_with_type(20, VANILLA_ENTITY_TYPE_PLAYER_ID);
    target.position = ProtocolVec3d {
        x: 4.0,
        y: 70.0,
        z: 8.0,
    };
    for packet in [
        PlayClientbound::AddEntity(item),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 10,
            values: vec![EntityDataValue {
                data_id: 8,
                serializer_id: 7,
                value: EntityDataValueKind::ItemStack(item_stack(42, 5)),
            }],
        }),
        PlayClientbound::AddEntity(target),
        PlayClientbound::TakeItemEntity(bbb_protocol::packets::TakeItemEntity {
            item_id: 10,
            player_id: 20,
            amount: 5,
        }),
    ] {
        tx.try_send(NetEvent::Play(packet)).unwrap();
    }
    drop(tx);

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut random,
        ),
        4
    );

    assert_eq!(particles.take_item_entity_pickup_states.len(), 1);
    let state = &particles.take_item_entity_pickup_states[0];
    assert_eq!(state.item_entity_id, 10);
    assert_eq!(state.item_entity_type_id, VANILLA_ENTITY_TYPE_ITEM_ID);
    assert_eq!(state.item_position.x, 1.0);
    assert_eq!(state.item_position.y, 64.0);
    assert_eq!(state.item_position.z, -2.0);
    assert_eq!(state.item_delta_movement.x, 0.1);
    assert_eq!(state.item_delta_movement.y, 0.2);
    assert_eq!(state.item_delta_movement.z, -0.3);
    assert_eq!(state.item_age_ticks, 1.0);
    assert_eq!(state.item_light.block, 15);
    assert_eq!(state.item_light.sky, 15);
    assert_eq!(state.target_entity_id, 20);
    assert_eq!(state.target_position.x, 4.0);
    assert_eq!(state.target_position.y, 70.0);
    assert_eq!(state.target_position.z, 8.0);
    assert_close(state.target_eye_height, 1.62);
    assert_eq!(state.item_stack, Some(item_stack(42, 5)));
    assert_eq!(state.experience_orb_icon, None);
    assert!(world.probe_entity(10).is_none());
    assert_eq!(world.counters().take_item_entities_removed, 1);
}

#[test]
fn clear_titles_event_updates_world_counters() {
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::SetTitlesAnimation(
        bbb_protocol::packets::SetTitlesAnimation {
            fade_in: 5,
            stay: 40,
            fade_out: 15,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetTitleText(
        bbb_protocol::packets::SetTitleText {
            content: "Quest complete".to_string(),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetSubtitleText(
        bbb_protocol::packets::SetSubtitleText {
            content: "Return to camp".to_string(),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ClearTitles(
        bbb_protocol::packets::ClearTitles { reset_times: false },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ClearTitles(
        bbb_protocol::packets::ClearTitles { reset_times: true },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        5
    );
    assert_eq!(world.title().title.as_deref(), None);
    assert_eq!(world.title().subtitle.as_deref(), None);
    assert_eq!(world.title().fade_in, 10);
    assert_eq!(world.title().stay, 70);
    assert_eq!(world.title().fade_out, 20);
    assert_eq!(world.title().title_time, 0);
    let world_counters = world.counters();
    assert_eq!(world_counters.clear_titles_packets, 2);
    assert_eq!(world_counters.title_text_packets, 1);
    assert_eq!(world_counters.subtitle_text_packets, 1);
    assert_eq!(world_counters.titles_animation_packets, 1);
}

#[test]
fn command_suggestions_event_updates_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::CommandSuggestions(
        bbb_protocol::packets::CommandSuggestions {
            id: 7,
            start: 1,
            length: 4,
            suggestions: vec![
                bbb_protocol::packets::CommandSuggestion {
                    text: "give".to_string(),
                    tooltip: Some("Run give".to_string()),
                },
                bbb_protocol::packets::CommandSuggestion {
                    text: "gamemode".to_string(),
                    tooltip: None,
                },
            ],
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(world.counters().command_suggestion_packets, 1);
    assert_eq!(world.counters().command_suggestion_entries_tracked, 2);

    let result = world.command_suggestions_by_id(7).unwrap();
    assert_eq!(result.start, 1);
    assert_eq!(result.length, 4);
    assert_eq!(result.suggestions.len(), 2);
    assert_eq!(result.suggestions[0].text, "give");
    assert_eq!(result.suggestions[0].tooltip.as_deref(), Some("Run give"));
    assert_eq!(world.last_command_suggestions(), Some(result));
}

#[test]
fn commands_event_updates_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::Commands(
        command_tree_packet("say"),
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(world.counters().command_tree_packets, 1);
    assert_eq!(world.counters().command_nodes_tracked, 3);
    assert_eq!(world.counters().command_literal_nodes_tracked, 1);
    assert_eq!(world.counters().command_argument_nodes_tracked, 1);
    assert_eq!(world.counters().command_redirect_nodes_tracked, 0);
    assert_eq!(world.counters().command_executable_nodes_tracked, 1);
    assert_eq!(world.counters().command_restricted_nodes_tracked, 1);

    let commands = world.commands();
    assert_eq!(commands.root_index, 0);
    assert_eq!(commands.nodes[1].name.as_deref(), Some("say"));
    assert_eq!(commands.nodes[2].name.as_deref(), Some("message"));
    assert_eq!(
        commands.nodes[2].parser.as_ref().unwrap().name,
        "brigadier:string"
    );
    assert_eq!(
        commands.nodes[2].suggestions.as_deref(),
        Some("minecraft:ask_server")
    );
}

#[test]
fn client_chat_events_update_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(3);
    let sender = Uuid::from_u128(0x1234);
    let signature = MessageSignature {
        bytes: vec![9; 256],
    };
    let expected_signature_checksum = signature.checksum();
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerChat(PlayerChat {
        global_index: 0,
        sender,
        index: 2,
        signature: Some(signature),
        body: SignedMessageBody {
            content: "hello".to_string(),
            timestamp_millis: 1,
            salt: 2,
            last_seen: Vec::new(),
        },
        unsigned_content: Some("unsigned hello".to_string()),
        filter_mask: FilterMask {
            kind: FilterMaskKind::PartiallyFiltered,
            mask_words: vec![1],
        },
        chat_type: protocol_chat_type("Alice"),
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DeleteChat(DeleteChat {
        message_signature: PackedMessageSignature {
            cache_id: Some(0),
            full_signature: None,
        },
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DisguisedChat(
        DisguisedChat {
            message: "server notice".to_string(),
            chat_type: protocol_chat_type("Server"),
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );
    assert_eq!(world.client_chat().messages.len(), 2);
    assert_eq!(world.client_chat().deleted_messages.len(), 1);
    let player_chat = &world.client_chat().messages[0];
    assert_eq!(player_chat.kind, bbb_world::ChatMessageKind::Player);
    assert_eq!(player_chat.content, "hello");
    assert_eq!(player_chat.sender, Some(sender));
    assert_eq!(player_chat.sender_name, "Alice");
    assert_eq!(player_chat.global_index, Some(0));
    assert_eq!(player_chat.message_index, Some(2));
    assert_eq!(player_chat.chat_type.registry_id, Some(0));
    assert_eq!(
        player_chat
            .signature
            .as_ref()
            .map(|signature| signature.checksum),
        Some(expected_signature_checksum)
    );
    assert_eq!(
        player_chat.unsigned_content.as_deref(),
        Some("unsigned hello")
    );
    assert_eq!(player_chat.filter_mask, "partially_filtered");
    assert_eq!(
        player_chat.validation_state,
        bbb_world::ChatValidationState::Unchecked
    );
    let disguised_chat = &world.client_chat().messages[1];
    assert_eq!(disguised_chat.kind, bbb_world::ChatMessageKind::Disguised);
    assert_eq!(disguised_chat.content, "server notice");
    let deleted_chat = &world.client_chat().deleted_messages[0];
    assert_eq!(
        deleted_chat
            .signature
            .as_ref()
            .map(|signature| signature.checksum),
        Some(expected_signature_checksum)
    );
    assert_eq!(deleted_chat.cache_id, Some(0));
    assert!(deleted_chat.resolved);
    let world_counters = world.counters();
    assert_eq!(world_counters.player_chat_packets, 1);
    assert_eq!(world_counters.disguised_chat_packets, 1);
    assert_eq!(world_counters.delete_chat_packets, 1);
    assert_eq!(world_counters.chat_messages_tracked, 2);
    assert_eq!(world_counters.deleted_chat_messages_tracked, 1);
    assert_eq!(world_counters.chat_signature_cache_entries, 1);
    assert_eq!(world_counters.player_chat_unsigned_content_packets, 1);
    assert_eq!(world_counters.player_chat_filtered_packets, 1);
}

#[test]
fn player_chat_events_queue_vanilla_threshold_acknowledgement() {
    let (tx, mut rx) = mpsc::channel(80);
    for index in 0..65 {
        tx.try_send(NetEvent::Play(PlayClientbound::PlayerChat(
            player_chat_with_signature(
                index,
                MessageSignature {
                    bytes: vec![index as u8; 256],
                },
            ),
        )))
        .unwrap();
    }

    let (command_tx, mut command_rx) = mpsc::channel(2);
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &Some(command_tx)),
        65
    );
    assert_eq!(world.counters().player_chat_packets, 65);
    assert_eq!(world.counters().player_chat_acknowledgement_packets, 1);
    assert_eq!(
        world.counters().player_chat_acknowledgement_pending_offset,
        0
    );
    assert_eq!(counters.chat_acknowledgement_commands_queued, 1);
    assert_eq!(
        command_rx.try_recv().unwrap(),
        NetCommand::ChatAcknowledgement(bbb_protocol::packets::ChatAcknowledgement { offset: 65 })
    );
    assert!(command_rx.try_recv().is_err());
}

#[test]
fn client_feature_events_update_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::CustomChatCompletions(
        CustomChatCompletions {
            action: CustomChatCompletionsAction::Set,
            entries: vec!["/warp".to_string(), "/spawn".to_string()],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::PlaceGhostRecipe(
        PlaceGhostRecipe {
            container_id: 9,
            recipe_display_type: RecipeDisplayType::Stonecutter,
            recipe_display_body: vec![1, 2, 3],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::UpdateAdvancements(
        UpdateAdvancements {
            reset: true,
            added: vec![AdvancementSummary {
                id: "minecraft:story/root".to_string(),
                parent: None,
                display: None,
                requirements: Vec::new(),
                sends_telemetry_event: false,
            }],
            removed: Vec::new(),
            progress: Vec::new(),
            show_advancements: false,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SelectAdvancementsTab(
        SelectAdvancementsTab {
            tab: Some("minecraft:story/root".to_string()),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::TagQuery(TagQuery {
        transaction_id: 12,
        tag_present: true,
        raw_nbt: vec![10, 0],
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        5
    );
    assert_eq!(
        world.last_custom_chat_completion_update(),
        Some(&bbb_world::CustomChatCompletionUpdateState {
            action: "set".to_string(),
            entries: 2,
        })
    );
    assert!(world.custom_chat_completions().contains("/warp"));
    assert!(world.custom_chat_completions().contains("/spawn"));
    assert_eq!(world.counters().custom_chat_completion_packets, 1);
    assert_eq!(world.counters().custom_chat_completions_tracked, 2);
    assert_eq!(
        world.last_ghost_recipe(),
        Some(&bbb_world::GhostRecipeState {
            container_id: 9,
            recipe_display_type_id: 3,
            recipe_display_type: "stonecutter".to_string(),
            recipe_display_body_len: 3,
        })
    );
    assert_eq!(world.counters().ghost_recipe_packets, 1);
    assert_eq!(
        world.selected_advancements_tab(),
        Some("minecraft:story/root")
    );
    assert_eq!(world.counters().select_advancements_tab_packets, 1);
    assert_eq!(
        world.last_tag_query(),
        Some(&bbb_world::TagQueryResponseState {
            transaction_id: 12,
            tag_present: true,
            raw_nbt: vec![10, 0],
        })
    );
    assert_eq!(world.counters().tag_query_packets, 1);
}

#[test]
fn inventory_events_update_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(8);
    tx.try_send(NetEvent::Play(PlayClientbound::OpenScreen(OpenScreen {
        container_id: 7,
        menu_type_id: 18,
        title: "Inventory".to_string(),
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ContainerSetContent(
        ContainerSetContent {
            container_id: 7,
            state_id: 1,
            items: vec![item_stack(42, 1), item_stack(43, 2)],
            carried_item: item_stack(99, 1),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ContainerSetSlot(
        ContainerSetSlot {
            container_id: 7,
            state_id: 2,
            slot: 1,
            item: item_stack(44, 3),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ContainerSetData(
        ContainerSetData {
            container_id: 7,
            id: 3,
            value: 11,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetPlayerInventory(
        SetPlayerInventory {
            slot: 5,
            item: item_stack(12, 1),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetCursorItem(
        SetCursorItem {
            item: item_stack(100, 4),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ContainerClose(
        ContainerClose { container_id: 7 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ContainerClose(
        ContainerClose { container_id: 99 },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        8
    );

    assert!(world.inventory().open_container.is_none());
    assert_eq!(world.inventory().cursor_item, item_stack(100, 4));
    assert_eq!(world.inventory().player_slots.len(), 1);
    assert_eq!(world.inventory().player_slots[0].slot, 5);
    assert_eq!(world.inventory().player_slots[0].item, item_stack(12, 1));

    let world_counters = world.counters();
    assert_eq!(world_counters.container_open_updates_received, 1);
    assert_eq!(world_counters.container_content_updates_received, 1);
    assert_eq!(world_counters.container_slot_updates_received, 1);
    assert_eq!(world_counters.container_data_updates_received, 1);
    assert_eq!(world_counters.container_close_updates_received, 2);
    assert_eq!(world_counters.container_close_updates_applied, 1);
    assert_eq!(world_counters.container_close_updates_ignored, 1);
    assert_eq!(world_counters.inventory_slot_updates_received, 1);
    assert_eq!(world_counters.inventory_slots_tracked, 1);
    assert_eq!(world_counters.cursor_item_updates_received, 1);
}
