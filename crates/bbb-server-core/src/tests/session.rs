//! The session surface: login and join, keep-alive, permission level, the
//! packets this world accepts without acting on, and disconnect.

use super::*;

mod respawn;

/// `PlayerList.placeNewPlayer` sends the status MOTD after the spawn teleport
/// (`PlayerList.java:201-205`).
#[test]
fn the_join_sends_the_server_data_after_the_spawn_teleport() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        motd: "Welcome".to_string(),
        ..creative_fixture_config()
    })
    .unwrap();
    let (joined, _) = join_loaded_ticks(&sender, &receiver, &mut world, 1, "alice", 1);

    let index = |wanted: fn(&ClientboundPlayPacket) -> bool| {
        joined.events.iter().position(|event| {
            matches!(event, WorldEvent::Send { connection_id: ConnectionId(1), packet } if wanted(packet))
        })
    };
    let teleport = index(|packet| matches!(packet, ClientboundPlayPacket::PlayerPosition(_)));
    let server_data = index(|packet| {
        matches!(packet, ClientboundPlayPacket::ServerData(data)
            if data.motd.text() == "Welcome" && data.icon_bytes.is_none())
    });
    assert!(
        teleport.is_some() && server_data > teleport,
        "{teleport:?} {server_data:?}"
    );
}

#[test]
fn session_housekeeping_packets_are_accepted_without_a_world_change() {
    // `ServerCommonPacketListenerImpl` treats these as no-ops (or, for the
    // resource pack, as a no-op while no pack is required). Closing the
    // connection instead would disconnect any client that changes its options
    // or answers a ping mid-session.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    for packet in [
        PlayServerbound::ClientInformation(ClientInformation::default()),
        PlayServerbound::CustomPayload(CustomPayload {
            id: "minecraft:brand".to_string(),
            payload: CustomPayloadBody::Brand {
                brand: "bbb".to_string(),
            },
        }),
        PlayServerbound::Pong { id: 7 },
        PlayServerbound::ResourcePack {
            id: Uuid::from_u128(9),
            action: ResourcePackResponseAction::Declined,
        },
        PlayServerbound::CustomClickAction(CustomClickAction {
            id: "bbb:noop".to_string(),
            payload: None,
        }),
    ] {
        let result = play(&sender, &receiver, &mut world, 1, packet.clone());
        assert!(result.events.is_empty(), "{packet:?} produced {result:?}");
        assert_eq!(result.snapshot.players.len(), 1, "{packet:?} closed play");
    }
}

#[test]
fn a_ping_request_is_echoed_straight_back_to_the_sender() {
    // `ServerGamePacketListenerImpl.handlePingRequest` answers on the
    // connection with the client's own timestamp.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    settle_joined_bodies(&mut world);

    let result = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PingRequest { time: -42 },
    );

    assert_eq!(
        result.events,
        vec![WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::PongResponse(PongResponse { time: -42 }),
        }]
    );
}

#[test]
fn the_login_reports_the_sea_level_the_loaded_generator_answers() {
    // `ServerPlayer.createCommonSpawnInfo` (:2151-2162) takes it from
    // `level.getSeaLevel()`, which forwards to the chunk generator
    // (ServerLevel.java:1857-1858). `FlatLevelSource.getSeaLevel`
    // (FlatLevelSource.java:150-152) is -63 whatever the layers are, so a flat
    // world that reported its own floor would be answering a question about a
    // generator it is not running.
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    let tick = world.tick(receiver.drain());
    let login = tick
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Login(login),
                ..
            } => Some(login),
            _ => None,
        })
        .expect("the join opens with the login");
    assert_eq!(
        login.common_spawn_info.sea_level,
        default_overworld_level_source().unwrap().sea_level()
    );
    assert_eq!(login.common_spawn_info.sea_level, -63);
    assert!(login.common_spawn_info.is_flat);
}

#[test]
fn a_restored_world_hashes_its_seed_in_common_spawn_info() {
    let mut candidate = restore_candidate(0, None, Vec::new());
    candidate.world_gen_settings.seed = 42;
    let mut world = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();

    let tick = world.tick(receiver.drain());
    let spawn = tick
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Login(login),
                ..
            } => Some(&login.common_spawn_info),
            _ => None,
        })
        .expect("the join opens with the login");
    assert_eq!(
        spawn.seed,
        bbb_server_worldgen::noise::biome::biome_zoom_seed(42)
    );
    assert!(spawn.is_flat);
}

#[test]
fn the_join_difficulty_announcement_follows_the_configured_difficulty() {
    // `DedicatedServer.forceDifficulty` applies `server.properties` at startup,
    // and `PlayerList.placeNewPlayer` announces the level data to each joining
    // player. A fresh level is never locked.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        difficulty: Difficulty::Hard,
        ..creative_fixture_config()
    })
    .unwrap();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();

    let tick = world.tick(receiver.drain());

    assert!(matches!(
        &tick.events[1],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::ChangeDifficulty(update),
            ..
        } if update.difficulty == Difficulty::Hard && !update.locked
    ));
}

#[test]
fn datapack_join_syncs_recipe_catalog_book_and_seen_highlights() {
    let (sender, receiver) = mailbox();
    let startup = Arc::new(
        bbb_server_datapack::startup::DatapackStartup::load(
            &bbb_server_datapack::startup::DatapackStartupOptions {
                datapacks_dir: None,
                saved_data_packs: None,
                safe_mode: false,
                limits: DatapackLoadLimits::default(),
            },
        )
        .unwrap(),
    );
    let recipe_id = "minecraft:flint_and_steel";
    let display = startup.recipe_displays.displays_for_recipe(recipe_id)[0].id;
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(Arc::clone(&startup)),
        ..creative_fixture_config()
    })
    .unwrap();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let joined = world.tick(receiver.drain());

    assert!(matches!(
        &joined.events[3],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::SetHeldSlot(update),
            ..
        } if update.slot == 0
    ));
    assert!(matches!(
        &joined.events[4],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::UpdateRecipes(update),
            ..
        } if update == startup.recipe_displays.update_recipes()
    ));
    assert!(matches!(
        &joined.events[5],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::EntityEvent(_),
            ..
        }
    ));
    assert!(matches!(
        &joined.events[6],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::Commands(_),
            ..
        }
    ));
    assert!(matches!(
        &joined.events[7],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::RecipeBookSettings(settings),
            ..
        } if *settings == Default::default()
    ));
    assert!(matches!(
        &joined.events[8],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::RecipeBookAdd(add),
            ..
        } if add.replace && add.entries.is_empty()
    ));

    world.award_player_recipe_for_test(ConnectionId(1), recipe_id);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::RecipeBookChangeSettings(RecipeBookChangeSettingsCommand {
            book_type: RecipeBookType::Crafting,
            open: true,
            filtering: true,
        }),
    );
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
    let settings = rejoined.events.iter().find_map(|event| match event {
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::RecipeBookSettings(settings),
        } => Some(settings),
        _ => None,
    });
    assert!(matches!(
        settings,
        Some(settings) if settings.crafting.open && settings.crafting.filtering
    ));
    let add = rejoined
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::RecipeBookAdd(add),
            } => Some(add),
            _ => None,
        })
        .expect("the restored recipe book is sent");
    assert!(add.replace);
    let mut expected_displays = joined
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::RecipeBookAdd(add),
                ..
            } => Some(add.entries.iter().map(|entry| entry.contents.id)),
            _ => None,
        })
        .flatten()
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        !expected_displays.is_empty(),
        "initial stone inventory awards recipes"
    );
    expected_displays.insert(display);
    assert_eq!(
        add.entries
            .iter()
            .map(|entry| entry.contents.id)
            .collect::<std::collections::BTreeSet<_>>(),
        expected_displays
    );
    assert_eq!(add.entries.len(), expected_displays.len());
    let crafted = add
        .entries
        .iter()
        .find(|entry| entry.contents.id == display)
        .unwrap();
    assert!(!crafted.notification);
    assert!(crafted.highlight);
    assert!(world.player_recipe_is_highlighted_for_test(ConnectionId(2), recipe_id));

    // The first player tick's creative ranges (ServerPlayer.java:610-627) sync next tick.
    world.tick(Vec::new());
    let seen = play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::RecipeBookSeenRecipe(RecipeBookSeenRecipeCommand { recipe: display }),
    );
    assert!(seen.events.is_empty());
    assert!(!world.player_recipe_is_highlighted_for_test(ConnectionId(2), recipe_id));
}

#[test]
fn difficulty_requests_below_the_gamemaster_level_change_nothing() {
    // `handleChangeDifficulty` and `handleLockDifficulty`
    // (ServerGamePacketListenerImpl.java:2063-2096) gate on
    // `Permissions.COMMANDS_GAMEMASTER`, and bbb has no operator surface, so
    // both requests are silently refused. A granted request would broadcast
    // through `MinecraftServer.sendDifficultyUpdate`, so "no events" is the
    // whole observable difference.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        difficulty: Difficulty::Peaceful,
        ..creative_fixture_config()
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    settle_joined_bodies(&mut world);

    for packet in [
        PlayServerbound::ChangeDifficulty {
            difficulty: Difficulty::Hard,
        },
        PlayServerbound::LockDifficulty { locked: true },
    ] {
        let result = play(&sender, &receiver, &mut world, 1, packet.clone());
        assert!(result.events.is_empty(), "{packet:?} produced {result:?}");
        assert_eq!(result.snapshot.players.len(), 2, "{packet:?} closed play");
    }

    // A player joining afterwards still sees the configured difficulty, which
    // proves the refusal did not quietly change server state either.
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(3),
            profile: profile("carol", 3),
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    assert!(tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(3),
            packet: ClientboundPlayPacket::ChangeDifficulty(update),
        } if update.difficulty == Difficulty::Peaceful && !update.locked
    )));
}

/// Every inbound packet whose vanilla handler is gated on a permission or a
/// game mode no session on this server holds, with the shape each one refuses
/// in. Only the two command-block editors answer; the rest are silent.
fn operator_gated_packets() -> Vec<(&'static str, PlayServerbound, bool)> {
    let pos = BlockPos { x: 1, y: 63, z: 1 };
    vec![
        (
            "block entity tag query",
            PlayServerbound::BlockEntityTagQuery(BlockEntityTagQuery {
                transaction_id: 7,
                pos,
            }),
            false,
        ),
        (
            "entity tag query",
            PlayServerbound::EntityTagQuery(EntityTagQuery {
                transaction_id: 8,
                entity_id: 1,
            }),
            false,
        ),
        (
            "set command block",
            PlayServerbound::SetCommandBlock(SetCommandBlock {
                pos,
                command: "say hello".into(),
                mode: CommandBlockMode::Redstone,
                track_output: true,
                conditional: false,
                automatic: false,
            }),
            true,
        ),
        (
            "set command minecart",
            PlayServerbound::SetCommandMinecart(SetCommandMinecart {
                entity_id: 1,
                command: "say hello".into(),
                track_output: true,
            }),
            true,
        ),
        (
            "set jigsaw block",
            PlayServerbound::SetJigsawBlock(SetJigsawBlock {
                pos,
                name: "minecraft:bottom".to_string(),
                target: "minecraft:top".to_string(),
                pool: "minecraft:empty".to_string(),
                final_state: "minecraft:air".into(),
                joint: JigsawJoint::Aligned,
                selection_priority: 0,
                placement_priority: 0,
            }),
            false,
        ),
        (
            "jigsaw generate",
            PlayServerbound::JigsawGenerate(JigsawGenerate {
                pos,
                levels: 3,
                keep_jigsaws: true,
            }),
            false,
        ),
        (
            "set structure block",
            PlayServerbound::SetStructureBlock(SetStructureBlock {
                pos,
                update: StructureBlockUpdateKind::SaveArea,
                mode: StructureBlockMode::Save,
                name: "bbb:fixture".to_string(),
                offset: BlockPos { x: 0, y: 1, z: 0 },
                size: Vec3i { x: 2, y: 2, z: 2 },
                mirror: StructureBlockMirror::None,
                rotation: StructureBlockRotation::None,
                data: String::new().into(),
                ignore_entities: true,
                strict: false,
                show_air: false,
                show_bounding_box: true,
                integrity: 1.0,
                seed: 0,
            }),
            false,
        ),
        (
            "set test block",
            PlayServerbound::SetTestBlock(SetTestBlock {
                pos,
                mode: TestBlockMode::Accept,
                message: "ok".into(),
            }),
            false,
        ),
        (
            "test instance block action",
            PlayServerbound::TestInstanceBlockAction(TestInstanceBlockAction {
                pos,
                action: TestInstanceBlockActionKind::Run,
                test: None,
                size: Vec3i { x: 1, y: 1, z: 1 },
                rotation: TestInstanceBlockRotation::None,
                ignore_entities: false,
            }),
            false,
        ),
        (
            "change game mode",
            PlayServerbound::ChangeGameMode(ChangeGameModeCommand {
                game_mode: GameType::Survival,
            }),
            false,
        ),
        (
            "set game rule",
            PlayServerbound::SetGameRule(SetGameRule {
                entries: vec![SetGameRuleEntry {
                    rule: "minecraft:advance_time".to_string(),
                    value: "false".to_string(),
                }],
            }),
            false,
        ),
        (
            "spectate entity",
            PlayServerbound::SpectateEntity(SpectateEntity { entity_id: 1 }),
            false,
        ),
        (
            "teleport to entity",
            PlayServerbound::TeleportToEntity(TeleportToEntity {
                uuid: Uuid::from_u128(1),
            }),
            false,
        ),
    ]
}

#[test]
fn acknowledging_messages_the_window_never_tracked_ends_the_session() {
    // `LastSeenMessagesValidator.applyOffset` (:31-38): the window starts full
    // of empty entries and this server never adds to it, so the only offset it
    // can afford is zero.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let accepted = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ChatAck { offset: 0 },
    );
    assert!(accepted.events.is_empty(), "{accepted:?}");

    // An offset past the window and one that is not an offset at all. Each
    // ends its own session, so each gets one.
    for offset in [1, -1] {
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let refused = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::ChatAck { offset },
        );
        assert!(
            refused.events.iter().any(|event| matches!(
                event,
                WorldEvent::Close {
                    reason: WorldCloseReason::ChatValidationFailed,
                    ..
                }
            )),
            "offset {offset} produced {refused:?}"
        );
    }
}

#[test]
fn a_living_sessions_respawn_is_dropped_and_the_time_counters_are_reported() {
    // `handleClientCommand` (:1859-1891): a player whose health is above zero
    // returns before the respawn can do anything (`:1872-1874`), the statistics
    // answer carries whatever the counter marked dirty, and the game rule
    // readout is gamemaster-gated like every other one.
    //
    // What is dirty for a session that has only stood still is the five
    // per-tick counters `ServerPlayer.doTick` awards (`ServerPlayer.java:649-661`),
    // minus the crouch one: `TIME_SINCE_REST` climbs because nothing here
    // sleeps, and `TIME_SINCE_DEATH` because the session is alive.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    for action in [
        ClientCommandAction::PerformRespawn,
        ClientCommandAction::RequestGameRuleValues,
    ] {
        let result = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::ClientCommand(action),
        );
        assert!(result.events.is_empty(), "{action:?} produced {result:?}");
    }

    let stats = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientCommand(ClientCommandAction::RequestStats),
    );
    let [WorldEvent::Send {
        connection_id: ConnectionId(1),
        packet: ClientboundPlayPacket::AwardStats(update),
    }] = stats.events.as_slice()
    else {
        panic!("{stats:?}");
    };
    let reported: std::collections::BTreeMap<String, i32> = update
        .stats
        .iter()
        .map(|update| {
            let key = bbb_server_types::stats::StatKey::new(update.stat_type_id, update.value_id);
            let (stat_type, value) = key
                .resource_ids()
                .expect("a reported statistic is nameable");
            (format!("{stat_type}/{value}"), update.amount)
        })
        .collect();
    let ticks = reported
        .get("minecraft:custom/minecraft:play_time")
        .copied()
        .expect("play_time is awarded every tick");
    assert!(ticks > 0, "{reported:?}");
    assert_eq!(
        reported,
        [
            ("minecraft:custom/minecraft:play_time", ticks),
            ("minecraft:custom/minecraft:total_world_time", ticks),
            ("minecraft:custom/minecraft:time_since_death", ticks),
            ("minecraft:custom/minecraft:time_since_rest", ticks),
        ]
        .into_iter()
        .map(|(name, amount)| (name.to_string(), amount))
        .collect::<std::collections::BTreeMap<_, _>>(),
        "a standing session counts exactly the four unconditional time stats"
    );
    // The dirty set is drained by the send (`ServerStatsCounter.java:98-102`),
    // so an immediate second request answers with only what the ticks between
    // the two moved.
    let again = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientCommand(ClientCommandAction::RequestStats),
    );
    let [WorldEvent::Send {
        packet: ClientboundPlayPacket::AwardStats(update),
        ..
    }] = again.events.as_slice()
    else {
        panic!("{again:?}");
    };
    assert_eq!(update.stats.len(), 4, "{update:?}");
    assert!(
        update.stats.iter().all(|update| update.amount > ticks),
        "the counters kept climbing between the two requests: {update:?}"
    );
}

#[test]
fn the_operator_gated_packets_are_all_refused() {
    // No operator list is read, so `Player.canUseGameMasterBlocks` and
    // `Player.isSpectator` are false for every session this world accepts and
    // each handler stops at its own gate.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    for (what, packet, answers) in operator_gated_packets() {
        let result = play(&sender, &receiver, &mut world, 1, packet);
        if answers {
            // `advMode.notAllowed` (ServerGamePacketListenerImpl.java:606-607
            // behind `handleSetCommandBlock`'s gate, :665-666 behind
            // `handleSetCommandMinecart`'s), to the sender and to nobody else.
            assert!(
                matches!(
                    result.events.as_slice(),
                    [WorldEvent::Send {
                        connection_id: ConnectionId(1),
                        packet: ClientboundPlayPacket::SystemChat(message),
                    }] if !message.overlay
                ),
                "{what} produced {result:?}"
            );
        } else {
            assert!(result.events.is_empty(), "{what} produced {result:?}");
        }
    }
}

#[test]
fn a_debug_subscription_request_has_no_immediate_answer() {
    // `ServerPlayer.requestDebugSubscriptions` (:2265-2267) only stores the
    // request; permission-filtered publication happens from the level tick.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let requested = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::DebugSubscriptionRequest {
            subscription_ids: vec![0],
        },
    );
    assert!(requested.events.is_empty(), "{requested:?}");
}

#[test]
fn the_join_announces_the_session_permission_level() {
    // `PlayerList.sendPlayerPermissionLevel` (PlayerList.java:559-572): the
    // level goes to its own player as an entity event naming that player, and
    // to nobody else. Vanilla's `PermissionLevel.ALL` event id is 24
    // (:561-567) and it is the only level this server hands out.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(2),
            profile: profile("bob", 2),
        },
    )
    .unwrap();
    let joined = world.tick(receiver.drain());
    let bob = entity_id_of(&world, 2);
    let announcements = joined
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::EntityEvent(update),
            } => Some((*connection_id, *update)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        announcements,
        vec![(
            ConnectionId(2),
            EntityEvent {
                entity_id: bob,
                event_id: 24,
            }
        )],
        "{joined:?}"
    );
}

#[test]
fn riding_and_inventory_commands_are_accepted_without_state_to_act_on() {
    // `STOP_SLEEPING`, both riding-jump actions and `OPEN_INVENTORY` all guard
    // on state this world does not carry — a bed, a ridden `PlayerRideableJumping`,
    // a vehicle with a custom screen. Vanilla runs the same guard and does
    // nothing for a standing player, so accepting the packet silently is the
    // aligned behavior, not a gap.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let actor = world.publication().players[0].entity_id;

    for action in [
        PlayerCommandAction::StopSleeping,
        PlayerCommandAction::StartRidingJump,
        PlayerCommandAction::StopRidingJump,
        PlayerCommandAction::OpenInventory,
    ] {
        let result = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::PlayerCommand(PlayerCommand {
                entity_id: actor,
                action,
                data: 1,
            }),
        );
        assert!(result.events.is_empty(), "{action:?} produced {result:?}");
        assert_eq!(result.snapshot.players.len(), 1, "{action:?} closed play");
    }
}

#[test]
fn the_flight_request_is_answered_with_silence() {
    // `handlePlayerAbilities` (:2047-2050) is `flying = requested && mayfly`
    // and echoes nothing: the client already knows what it asked for, so the
    // only thing a client can observe is that nothing comes back and the
    // session survives. The clamp itself is pinned where it is decided, on
    // `PlayerAbilities::for_game_type`.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    for flying in [true, false] {
        let result = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::PlayerAbilities { flying },
        );
        assert!(result.events.is_empty(), "flying={flying} produced events");
        assert_eq!(result.snapshot.players.len(), 1);
    }
}

#[test]
fn an_answer_to_a_query_the_server_never_asked_closes_the_connection() {
    // `ServerCommonPacketListenerImpl.handleCookieResponse` disconnects
    // unconditionally, and bbb requests no configuration switch, so an
    // acknowledgement is equally unsolicited.
    for (packet, reason) in [
        (
            PlayServerbound::CookieResponse {
                key: "minecraft:test".to_string(),
                payload: None,
            },
            WorldCloseReason::UnexpectedQuery,
        ),
        (
            PlayServerbound::ConfigurationAcknowledged,
            WorldCloseReason::PacketOutsideActiveContract,
        ),
    ] {
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

        let result = play(&sender, &receiver, &mut world, 1, packet.clone());

        assert!(
            result.events.contains(&WorldEvent::Close {
                connection_id: ConnectionId(1),
                reason,
            }),
            "{packet:?} produced {:?}",
            result.events
        );
        assert!(result.snapshot.players.is_empty(), "{packet:?}");
    }
}

#[test]
fn the_tab_list_hat_flag_follows_the_declared_skin_parts() {
    // `ClientboundPlayerInfoUpdatePacket` reads `isModelPartShown(HAT)`, and
    // `handleClientInformation` re-broadcasts the entry only when that flag
    // flips.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: ClientInformation {
                displayed_skin_parts: 0,
                ..ClientInformation::default()
            },
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
            context: JoinContext::default(),
        },
    )
    .unwrap();
    let joined = world.tick(receiver.drain());
    assert_eq!(info_entry_hats(&joined.events), vec![false]);
    // The first player tick's creative ranges (ServerPlayer.java:610-627) sync next tick.
    world.tick(Vec::new());

    // Same options again: no flip, so no broadcast.
    let unchanged = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientInformation(ClientInformation {
            displayed_skin_parts: 0,
            ..ClientInformation::default()
        }),
    );
    assert!(unchanged.events.is_empty(), "{:?}", unchanged.events);

    // Showing the hat re-broadcasts the entry with the single hat action.
    let shown = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientInformation(ClientInformation::default()),
    );
    let WorldEvent::Send {
        packet: ClientboundPlayPacket::PlayerInfoUpdate(update),
        ..
    } = shown
        .events
        .first()
        .expect("a hat change broadcasts the entry")
    else {
        panic!("expected a player info update, got {:?}", shown.events);
    };
    assert_eq!(update.actions, vec![PlayerInfoAction::UpdateHat]);
    assert_eq!(
        update
            .entries
            .iter()
            .map(|entry| entry.show_hat)
            .collect::<Vec<_>>(),
        vec![true]
    );
}

fn info_entry_hats(events: &[WorldEvent]) -> Vec<bool> {
    events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerInfoUpdate(update),
                ..
            } => Some(update.entries.iter().map(|entry| entry.show_hat)),
            _ => None,
        })
        .flatten()
        .collect()
}

#[test]
fn disconnect_tells_survivors_to_remove_the_player() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    for connection_id in [ConnectionId(1), ConnectionId(2)] {
        world.reset_pending_chunks_for_test(connection_id, &[]);
        world.reconcile_player_tracking_for_test(connection_id);
    }

    // `removePlayerFromWorld` (:1450-1459) announces the departure, then
    // `PlayerList.remove` removes the tracked body, then the waypoint, then
    // the tab-list entry (`PlayerList.java:318-331`). The closing connection
    // gets none of them.
    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(1),
        },
    )
    .unwrap();
    let removed = world.tick(receiver.drain());
    assert!(matches!(
        &removed.events[..],
        [
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::SystemChat(left),
            },
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::RemoveEntities(entities),
            },
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::TrackedWaypoint(waypoint),
            },
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::PlayerInfoRemove(info),
            },
        ] if info.profile_ids == [Uuid::from_u128(1)]
            && entities.entity_ids == [1]
            && waypoint.operation == bbb_protocol::packets::WaypointOperation::Untrack
            && !left.overlay
            && left.content.text().contains("multiplayer.player.left")
    ));
    assert_eq!(removed.snapshot.players.len(), 1);

    // The keepalive-timeout path emits the same pair to survivors. Challenges
    // are gated by wall-clock interval (`keep_alive_interval_ticks * 50 ms`).
    let config = ServerWorldConfig {
        keep_alive_interval_ticks: 5,
        ..creative_fixture_config()
    };
    let mut world = ServerWorld::new(config).unwrap();
    join_loaded(&sender, &receiver, &mut world, 3, "carol", 3);
    join_loaded(&sender, &receiver, &mut world, 4, "dave", 4);
    // First 250 ms: keepalive challenges go out. Second 250 ms: both
    // unanswered -> both time out. No survivors remain, so no removal packets
    // are emitted.
    crate::world::idle_timeout::advance_clock_for_test(250);
    world.tick(Vec::new());
    crate::world::idle_timeout::advance_clock_for_test(250);
    let timed_out = world.tick(Vec::new());
    assert_eq!(
        timed_out
            .events
            .iter()
            .filter(|event| matches!(event, WorldEvent::Close { .. }))
            .count(),
        2
    );
    assert!(timed_out.snapshot.players.is_empty());
}

#[test]
fn world_time_syncs_full_at_join_and_empty_every_twenty_ticks() {
    let (sender, receiver) = mailbox();
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        },
    )
    .unwrap();
    let config = ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..creative_fixture_config()
    };
    let mut world = ServerWorld::new(config).unwrap();

    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(3),
            profile: profile("carol", 3),
        },
    )
    .unwrap();
    let first = world.tick(receiver.drain());
    let join_sync = first
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SetTime(update),
                ..
            } => Some(update),
            _ => None,
        })
        .expect("join sends the full clock sync");
    // The join is answered before the level ticks (`MinecraftServer.java:1005`
    // ahead of `:1007`), so the world time it carries is the one the previous
    // tick left -- zero on a world that has never ticked.
    assert_eq!(join_sync.game_time, 0);
    // The vanilla 26.1 built-in world_clock registry loads minecraft:overworld
    // (id 0) and minecraft:the_end (id 1), sorted by element id.
    assert_eq!(join_sync.clock_updates.len(), 2);
    assert_eq!(join_sync.clock_updates[0].clock_id, 0);
    assert_eq!(join_sync.clock_updates[0].total_ticks, 0);
    assert_eq!(join_sync.clock_updates[0].rate, 1.0);
    assert_eq!(join_sync.clock_updates[1].clock_id, 1);

    for _ in 0..18 {
        let result = world.tick(Vec::new());
        assert!(
            !result.events.iter().any(|event| matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::SetTime(_),
                    ..
                }
            )),
            "no time sync before the 20-tick cadence"
        );
    }
    // `forceGameTimeSynchronization` is a `tickChildren` arm
    // (`MinecraftServer.java:1095-1098`) that runs ahead of the levels arm
    // (`:1101`), so the twentieth tick broadcasts the game time the nineteenth
    // left behind.
    let twentieth = world.tick(Vec::new());
    assert!(twentieth.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(3),
            packet: ClientboundPlayPacket::SetTime(update),
        } if update.game_time == 19 && update.clock_updates.is_empty()
    )));

    // A second join observes the advanced clocks in its full sync.
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(4),
            profile: profile("dave", 4),
        },
    )
    .unwrap();
    let later = world.tick(receiver.drain());
    let later_sync = later
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(4),
                packet: ClientboundPlayPacket::SetTime(update),
            } => Some(update),
            _ => None,
        })
        .expect("second join sends the full clock sync");
    assert_eq!(later_sync.game_time, 20);
    // The second join runs during tick 21 command processing, before that
    // tick's clock advance, so the clocks still read 20.
    assert_eq!(later_sync.clock_updates[0].total_ticks, 20);
    assert_eq!(later_sync.clock_updates[1].total_ticks, 20);
}

#[test]
fn invalid_movement_and_keep_alive_fail_closed() {
    let config = ServerWorldConfig {
        keep_alive_interval_ticks: 2,
        ..creative_fixture_config()
    };
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let mut world = ServerWorld::new(config).unwrap();
    world.tick(receiver.drain());
    crate::world::idle_timeout::advance_clock_for_test(100);
    let keep_alive = world.tick(receiver.drain());
    let challenges: Vec<_> = keep_alive
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::KeepAlive { id },
            } => Some((*connection_id, *id)),
            _ => None,
        })
        .collect();
    assert_eq!(challenges, vec![(ConnectionId(1), 2)]);
    assert!(!keep_alive
        .events
        .iter()
        .any(|event| matches!(event, WorldEvent::Close { .. })));

    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::KeepAlive { id: 3 },
        },
    )
    .unwrap();
    let mismatch = world.tick(receiver.drain());
    assert_eq!(
        mismatch.events,
        vec![WorldEvent::Close {
            connection_id: ConnectionId(1),
            reason: WorldCloseReason::KeepAliveMismatch,
        }]
    );
    assert!(mismatch.snapshot.players.is_empty());

    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(2),
            profile: profile("bob", 2),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(2),
            packet: PlayServerbound::MovePlayer(MovePlayer {
                position: Some(Vec3d {
                    x: f64::NAN,
                    y: 65.0,
                    z: 0.0,
                }),
                rotation: None,
                on_ground: false,
                horizontal_collision: false,
            }),
        },
    )
    .unwrap();
    let invalid = world.tick(receiver.drain());
    assert!(matches!(
        invalid.events.as_slice(),
        [WorldEvent::Close {
            connection_id: ConnectionId(2),
            reason: WorldCloseReason::InvalidMovement,
        }]
    ));
}

/// `handleKeepAlive` (ServerCommonPacketListenerImpl.java:80-88) smooths the
/// round trip into the latency the player list carries to a later joiner.
#[test]
fn keep_alive_round_trip_sets_the_player_list_latency() {
    let config = ServerWorldConfig {
        keep_alive_interval_ticks: 2,
        ..creative_fixture_config()
    };
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let mut world = ServerWorld::new(config).unwrap();
    world.tick(receiver.drain());
    crate::world::idle_timeout::advance_clock_for_test(100);
    world.tick(receiver.drain());
    crate::world::idle_timeout::advance_clock_for_test(100);
    for command in [
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::KeepAlive { id: 2 },
        },
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(2),
            profile: profile("bob", 2),
        },
    ] {
        enqueue(&sender, command).unwrap();
    }
    let joined = world.tick(receiver.drain());
    let latency = joined.events.iter().find_map(|event| match event {
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::PlayerInfoUpdate(update),
        } => update
            .entries
            .iter()
            .find(|entry| entry.profile_id == Uuid::from_u128(1))
            .map(|entry| entry.latency),
        _ => None,
    });
    assert_eq!(latency, Some(25));
}

#[test]
fn keep_alive_ack_rearms_and_unimplemented_packets_are_attributable() {
    let config = ServerWorldConfig {
        keep_alive_interval_ticks: 2,
        ..creative_fixture_config()
    };
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let mut world = ServerWorld::new(config).unwrap();
    world.tick(receiver.drain());
    crate::world::idle_timeout::advance_clock_for_test(100);
    world.tick(receiver.drain());
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::KeepAlive { id: 2 },
        },
    )
    .unwrap();
    assert!(world.tick(receiver.drain()).events.is_empty());
    crate::world::idle_timeout::advance_clock_for_test(100);
    assert!(matches!(
        world.tick(receiver.drain()).events.as_slice(),
        [WorldEvent::Send {
            packet: ClientboundPlayPacket::KeepAlive { id: 4 },
            ..
        }]
    ));
    world.tick(receiver.drain());
    crate::world::idle_timeout::advance_clock_for_test(100);
    assert!(matches!(
        world.tick(receiver.drain()).events.as_slice(),
        [WorldEvent::Close {
            reason: WorldCloseReason::KeepAliveTimeout,
            ..
        }]
    ));
}

#[test]
fn keep_alive_uses_wall_clock_during_tick_sprint() {
    let config = ServerWorldConfig {
        keep_alive_interval_ticks: 2,
        ..creative_fixture_config()
    };
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let mut world = ServerWorld::new(config).unwrap();
    world.tick(receiver.drain());
    // Fifty fast ticks with zero wall-clock progress must neither emit a
    // challenge nor time the player out.
    for _ in 0..50 {
        let tick = world.tick(Vec::new());
        assert!(!tick.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::KeepAlive { .. },
                ..
            } | WorldEvent::Close { .. }
        )));
    }
    crate::world::idle_timeout::advance_clock_for_test(100);
    let challenged = world.tick(Vec::new());
    assert!(challenged.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::KeepAlive { .. },
            ..
        }
    )));
    // Fifty more sprint ticks with no wall-clock progress still do not time out.
    for _ in 0..50 {
        let tick = world.tick(Vec::new());
        assert!(!tick
            .events
            .iter()
            .any(|event| matches!(event, WorldEvent::Close { .. })));
    }
    crate::world::idle_timeout::advance_clock_for_test(100);
    let timed_out = world.tick(Vec::new());
    assert!(matches!(
        timed_out.events.as_slice(),
        [WorldEvent::Close {
            connection_id: ConnectionId(1),
            reason: WorldCloseReason::KeepAliveTimeout,
        }]
    ));
}

#[tokio::test]
async fn mailbox_preserves_global_order_and_isolates_class_backpressure() {
    let limits = WorldMailboxLimits {
        network: MailboxClassLimits {
            entries: 1,
            bytes: 4096,
        },
        admin: MailboxClassLimits {
            entries: 1,
            bytes: 4096,
        },
        lifecycle: MailboxClassLimits {
            entries: 1,
            bytes: 4096,
        },
    };
    let (sender, receiver) = world_command_mailbox(limits).unwrap();
    assert_eq!(
        sender
            .try_send(WorldCommand::Disconnect {
                connection_id: ConnectionId(1),
            })
            .unwrap(),
        1
    );
    assert!(matches!(
        sender.try_send(WorldCommand::Disconnect {
            connection_id: ConnectionId(2),
        }),
        Err(WorldCommandSendError::Full(command))
            if matches!(
                &*command,
                WorldCommand::Disconnect {
                    connection_id: ConnectionId(2)
                }
            )
    ));
    assert_eq!(sender.try_send(WorldCommand::Shutdown).unwrap(), 2);
    let drained = receiver.drain();
    assert_eq!(
        drained
            .iter()
            .map(CommandEnvelope::sequence)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(
        drained
            .iter()
            .map(CommandEnvelope::producer_class)
            .collect::<Vec<_>>(),
        vec![WorldProducerClass::Network, WorldProducerClass::Lifecycle,]
    );

    assert!(matches!(
        sender.try_send(WorldCommand::Disconnect {
            connection_id: ConnectionId(3),
        }),
        Err(WorldCommandSendError::Full(_))
    ));
    drop(drained);

    sender
        .try_send(WorldCommand::Disconnect {
            connection_id: ConnectionId(3),
        })
        .unwrap();
    let waiting_sender = sender.clone();
    let waiting = tokio::spawn(async move {
        waiting_sender
            .send(WorldCommand::Disconnect {
                connection_id: ConnectionId(4),
            })
            .await
            .unwrap()
    });
    tokio::task::yield_now().await;
    assert!(!waiting.is_finished());
    receiver.drain();
    assert_eq!(waiting.await.unwrap(), 4);
    assert_eq!(receiver.drain()[0].sequence(), 4);

    let tiny = WorldMailboxLimits {
        network: MailboxClassLimits {
            entries: 1,
            bytes: 1,
        },
        ..limits
    };
    let (tiny_sender, _) = world_command_mailbox(tiny).unwrap();
    assert!(matches!(
        tiny_sender.try_send(WorldCommand::Disconnect {
            connection_id: ConnectionId(5),
        }),
        Err(WorldCommandSendError::Oversized { max: 1, .. })
    ));
}

#[test]
fn wire_play_payloads_remain_budgeted_until_the_tick_consumes_them() {
    let limits = WorldMailboxLimits {
        network: MailboxClassLimits {
            entries: 2,
            bytes: 2 * std::mem::size_of::<WorldCommand>() + 4096,
        },
        ..WorldMailboxLimits::default()
    };
    let (sender, receiver) = world_command_mailbox(limits).unwrap();
    let (packet_id, payload) = bbb_protocol::packets::encode_play_custom_payload(
        &bbb_protocol::packets::ServerboundCustomPayload::Unknown {
            id: "bbb:budget".to_string(),
            raw_payload: vec![0; 32_767],
        },
    )
    .unwrap();
    assert!(matches!(
        sender.try_send(WorldCommand::PlayPacket {
            connection_id: ConnectionId(1),
            packet_id,
            payload,
        }),
        Err(WorldCommandSendError::Oversized { .. })
    ));

    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    for slot in [1, 3] {
        let (packet_id, payload) = bbb_protocol::packets::encode_play_set_carried_item(slot);
        sender
            .try_send(WorldCommand::PlayPacket {
                connection_id: ConnectionId(1),
                packet_id,
                payload,
            })
            .unwrap();
    }
    let pending = receiver.drain();
    assert!(matches!(
        sender.try_send(WorldCommand::Disconnect {
            connection_id: ConnectionId(1),
        }),
        Err(WorldCommandSendError::Full(_))
    ));
    let tick = world.tick(pending);
    assert_eq!(snapshot_player(&tick, 1).inventory.selected_slot, 3);
    let (packet_id, _) = bbb_protocol::packets::encode_play_set_carried_item(0);
    sender
        .try_send(WorldCommand::PlayPacket {
            connection_id: ConnectionId(1),
            packet_id,
            payload: Vec::new(),
        })
        .unwrap();
    let tick = world.tick(receiver.drain());
    assert!(tick.snapshot.players.is_empty());
    assert!(tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Close {
            connection_id: ConnectionId(1),
            reason: WorldCloseReason::PacketOutsideActiveContract,
        }
    )));
}

#[test]
fn shutdown_and_duplicate_profile_are_tick_owned() {
    let config = ServerWorldConfig {
        max_players: 2,
        ..creative_fixture_config()
    };
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(2),
            profile: profile("renamed", 1),
        },
    )
    .unwrap();
    let mut world = ServerWorld::new(config).unwrap();
    let joined = world.tick(receiver.drain());
    // `PlayerList.disconnectAllPlayersWithProfile` (PlayerList.java:364-383):
    // the session already online under the profile is the one that goes.
    assert!(joined.events.iter().any(|event| matches!(
        event,
        WorldEvent::Close {
            connection_id: ConnectionId(1),
            reason: WorldCloseReason::DuplicatePlayer,
        }
    )));

    sender.try_send(WorldCommand::Shutdown).unwrap();
    let stopped = world.tick(receiver.drain());
    assert!(stopped.snapshot.stopping);
    assert!(stopped.snapshot.players.is_empty());
    assert!(
        matches!(
            stopped.events.as_slice(),
            [WorldEvent::Close {
                connection_id: ConnectionId(2),
                reason: WorldCloseReason::ServerShutdown,
            }]
        ),
        "{:?}",
        stopped.events
    );
}

/// A second login under an online profile kicks the first session with
/// `multiplayer.disconnect.duplicate_login` and admits the newcomer, which
/// loads what the kicked session's removal saved.
#[test]
fn a_duplicate_login_replaces_the_session_already_online() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let moved = Vec3d {
        x: 3.5,
        y: world
            .player_movement_validation_for_test(ConnectionId(1))
            .0
            .y,
        z: -2.5,
    };
    world.set_player_position_for_test(ConnectionId(1), moved);

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
    let joined = world.tick(receiver.drain());
    let closed = joined
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Close {
                connection_id,
                reason,
            } => Some((*connection_id, reason.reason_component())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(closed.len(), 1, "{closed:?}");
    assert_eq!(closed[0].0, ConnectionId(1));
    assert!(format!("{:?}", closed[0].1).contains("multiplayer.disconnect.duplicate_login"));
    let position = world.player_movement_validation_for_test(ConnectionId(2)).0;
    assert_eq!((position.x, position.z), (moved.x, moved.z));
}

/// `ServerCommonPacketListenerImpl.handleResourcePackResponse` (`:105-111`):
/// all terminal failure/discard statuses end play when the pack is required,
/// while progress/success statuses keep the session alive.
#[test]
fn declining_a_required_pack_in_play_disconnects() {
    use bbb_protocol::resource_pack_response_actions::ResourcePackResponseAction;
    let closes = |tick: &TickResult| {
        tick.events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Close { reason, .. } => Some(reason.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let response = |action| PlayServerbound::ResourcePack {
        id: Uuid::from_u128(3),
        action,
    };
    {
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(ServerWorldConfig {
            resource_pack_required: true,
            ..creative_fixture_config()
        })
        .unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        for ok_action in [
            ResourcePackResponseAction::Accepted,
            ResourcePackResponseAction::Downloaded,
            ResourcePackResponseAction::SuccessfullyLoaded,
        ] {
            let tick = play(&sender, &receiver, &mut world, 1, response(ok_action));
            assert!(closes(&tick).is_empty(), "{ok_action:?} closed play");
        }
    }
    for fail_action in [
        ResourcePackResponseAction::Declined,
        ResourcePackResponseAction::FailedDownload,
        ResourcePackResponseAction::InvalidUrl,
        ResourcePackResponseAction::FailedReload,
        ResourcePackResponseAction::Discarded,
    ] {
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(ServerWorldConfig {
            resource_pack_required: true,
            ..creative_fixture_config()
        })
        .unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let tick = play(&sender, &receiver, &mut world, 1, response(fail_action));
        assert_eq!(
            closes(&tick),
            [WorldCloseReason::RequiredResourcePackDeclined],
            "expected disconnect for {fail_action:?}"
        );
    }
}

#[test]
fn multi_player_broadcast_emits_real_world_event_broadcast() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    settle_joined_bodies(&mut world);
    let pos = BlockPos { x: 2, y: 65, z: 2 };
    let stone = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
    )
    .unwrap();
    world.set_block(
        Java26BuiltinDimension::Overworld,
        pos,
        stone,
        SET_BLOCK_UPDATE_CLIENTS,
    );
    let tick = world.tick(receiver.drain());
    assert!(
        tick.events.iter().any(|event| matches!(
            event,
            WorldEvent::Broadcast {
                recipients,
                packet: ClientboundPlayPacket::BlockUpdate(update),
            } if update.pos == pos
                && recipients.contains(&ConnectionId(1))
                && recipients.contains(&ConnectionId(2))
        )),
        "expected WorldEvent::Broadcast for multi-player block update: {:?}",
        tick.events
    );
}

#[test]
fn menu_vehicle_and_book_requests_find_nothing_to_act_on() {
    // The vanilla handlers behind these six all begin by asking for something
    // this session does not have open: an `AnvilMenu`, a `MerchantMenu`, a
    // `BeaconMenu` (`handleRenameItem` :761-771, `handleSelectTrade` :962-973,
    // `handleSetBeaconPacket` :774-784), a ridden vehicle (`handleMoveVehicle`
    // :439-482, `handlePaddleBoat` :1436-1441) or a published recipe display
    // (`handleRecipeBookSeenRecipePacket` :556-562). Each is accepted and
    // answers nothing.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    for packet in [
        PlayServerbound::RenameItem(RenameItem {
            name: "bbb anvil".to_string(),
        }),
        PlayServerbound::SelectTrade(SelectTradeCommand { item: 0 }),
        PlayServerbound::SetBeacon(SetBeacon {
            primary_effect: Some(bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_SPEED_ID),
            secondary_effect: None,
        }),
        PlayServerbound::PaddleBoat(PaddleBoat {
            left: true,
            right: true,
        }),
        PlayServerbound::RecipeBookSeenRecipe(RecipeBookSeenRecipeCommand {
            recipe: RecipeDisplayId { index: 0 },
        }),
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: Vec3d {
                x: 8.5,
                y: 65.0,
                z: 8.5,
            },
            y_rot: 90.0,
            x_rot: 0.0,
            on_ground: true,
        }),
        // `handleSignUpdate` (:2028-2044) needs a `SignBlockEntity` at the
        // position, `handlePlaceRecipe` (:1954-1981) a published recipe
        // display, and `handleBundleItemSelectedPacket` (:565-568) writes a
        // selection `BundleContents.STREAM_CODEC` drops and `equals` ignores.
        PlayServerbound::SignUpdate(SignUpdate {
            pos: BlockPos { x: 0, y: 64, z: 0 },
            is_front_text: true,
            lines: [
                "bbb".to_string(),
                String::new(),
                String::new(),
                String::new(),
            ],
        }),
        PlayServerbound::PlaceRecipe(PlaceRecipeCommand {
            container_id: MENU_CONTAINER_ID,
            recipe_index: 0,
            use_max_items: true,
        }),
        PlayServerbound::SelectBundleItem(SelectBundleItem {
            slot_id: 38,
            selected_item_index: 0,
        }),
    ] {
        let result = play(&sender, &receiver, &mut world, 1, packet.clone());
        assert!(result.events.is_empty(), "{packet:?} produced {result:?}");
    }
}

#[test]
fn a_vehicle_move_that_is_not_a_number_ends_the_session() {
    // `containsInvalidValues` (:426-428) as `handleMoveVehicle` (:441-442)
    // calls it: the position must be a number and both rotations finite, and
    // the reader's answer to anything else is the disconnect — before it ever
    // looks for a vehicle.
    for update in [
        MoveVehicleCommand {
            position: Vec3d {
                x: f64::NAN,
                y: 65.0,
                z: 8.5,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        },
        MoveVehicleCommand {
            position: Vec3d {
                x: 8.5,
                y: 65.0,
                z: 8.5,
            },
            y_rot: f32::INFINITY,
            x_rot: 0.0,
            on_ground: true,
        },
        MoveVehicleCommand {
            position: Vec3d {
                x: 8.5,
                y: 65.0,
                z: 8.5,
            },
            y_rot: 0.0,
            x_rot: f32::NAN,
            on_ground: false,
        },
    ] {
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let refused = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::MoveVehicle(update),
        );
        assert!(
            refused.events.iter().any(|event| matches!(
                event,
                WorldEvent::Close {
                    reason: WorldCloseReason::InvalidVehicleMovement,
                    ..
                }
            )),
            "{update:?} produced {refused:?}"
        );
    }
}

#[test]
fn joining_without_item_tags_skips_crafting_catalog_install() {
    use crate::inventory::PlayerInventoryMenu;
    use bbb_protocol::item_types::{VANILLA_ITEM_FLINT_ID, VANILLA_ITEM_IRON_INGOT_ID};
    let mut startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        },
    )
    .unwrap();
    startup
        .tags
        .registries
        .retain(|registry| registry.registry_id != "minecraft:item");
    assert!(startup.tags.registry("minecraft:item").is_none());
    let mut inventory = PlayerInventoryMenu::empty();
    let startup = std::sync::Arc::new(startup);
    ServerWorld::install_join_crafting_catalog(
        crate::inventory::test_item_resolver(),
        None,
        &mut inventory,
        Some(&startup),
        ServerWorld::crafting_item_tags_of(Some(&*startup)).as_ref(),
    )
    .expect("fixture crafting catalog");
    assert!(inventory
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            crate::tests::native_stack(ItemStackSummary {
                item_id: Some(VANILLA_ITEM_IRON_INGOT_ID),
                count: 1,
                component_patch: Default::default(),
            }),
        )
        .expect("the fixture item is accepted"));
    assert!(inventory
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            2,
            crate::tests::native_stack(ItemStackSummary {
                item_id: Some(VANILLA_ITEM_FLINT_ID),
                count: 1,
                component_patch: Default::default(),
            }),
        )
        .expect("the fixture item is accepted"));
    // Flint-and-steel matches without tags; skip (not an empty index) is what
    // leaves this result unmatched.
    assert!(inventory.slot_item(0).item_id().is_none());
}

#[test]
fn join_catalog_install_keeps_the_restored_recipe_book() {
    use bbb_protocol::item_types::{
        VANILLA_ITEM_FLINT_AND_STEEL_ID, VANILLA_ITEM_FLINT_ID, VANILLA_ITEM_IRON_INGOT_ID,
    };
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.award_player_recipe_for_test(ConnectionId(1), "minecraft:flint_and_steel");
    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(1),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    join_loaded(&sender, &receiver, &mut world, 2, "alice", 1);
    assert!(world.player_knows_recipe_for_test(ConnectionId(2), "minecraft:flint_and_steel"));

    for (slot_num, item_id) in [(1, VANILLA_ITEM_IRON_INGOT_ID), (2, VANILLA_ITEM_FLINT_ID)] {
        play(
            &sender,
            &receiver,
            &mut world,
            2,
            PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
                slot_num,
                item: ItemStackSummary {
                    item_id: Some(item_id),
                    count: 1,
                    component_patch: Default::default(),
                },
            }),
        );
    }
    assert_eq!(
        world
            .player_crafting_result_for_test(ConnectionId(2))
            .item_id,
        Some(VANILLA_ITEM_FLINT_AND_STEEL_ID)
    );
}

#[test]
fn recipe_book_settings_are_kept_per_book_type_and_answer_nothing() {
    // `RecipeBook.setBookSetting` (RecipeBook.java:32-35) writes both halves of
    // the named type's pair and leaves the other three alone, and nothing on
    // the wire says so.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let result = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::RecipeBookChangeSettings(RecipeBookChangeSettingsCommand {
            book_type: RecipeBookType::Smoker,
            open: true,
            filtering: true,
        }),
    );
    assert!(result.events.is_empty(), "{result:?}");

    let settings = world.player_recipe_book_settings(ConnectionId(1));
    assert_eq!(
        settings[RecipeBookType::Smoker.id() as usize],
        RecipeBookTypeSettings {
            open: true,
            filtering: true,
        }
    );
    assert_eq!(
        settings[RecipeBookType::Crafting.id() as usize],
        RecipeBookTypeSettings::default()
    );

    // The second write of the same type replaces the pair rather than merging
    // it, which is what makes closing the book readable.
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::RecipeBookChangeSettings(RecipeBookChangeSettingsCommand {
            book_type: RecipeBookType::Smoker,
            open: false,
            filtering: false,
        }),
    );
    assert_eq!(
        world.player_recipe_book_settings(ConnectionId(1))[RecipeBookType::Smoker.id() as usize],
        RecipeBookTypeSettings::default()
    );
}

#[test]
fn opening_an_advancement_tab_answers_only_when_the_selection_changed() {
    // `handleSeenAdvancements` (:577-586) and `setSelectedTab` (:273-285): the
    // id must name a loaded advancement, the holder must be a root with a
    // display to be kept, and the client is told whenever the selection moved.
    let (sender, receiver) = mailbox();
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        },
    )
    .unwrap();
    let config = ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..creative_fixture_config()
    };
    let mut world = ServerWorld::new(config).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let opened_tab = |world: &mut ServerWorld, tab: &str| {
        play(
            &sender,
            &receiver,
            world,
            1,
            PlayServerbound::SeenAdvancements(SeenAdvancements::OpenedTab {
                tab: tab.to_string(),
            }),
        )
        .events
        .into_iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SelectAdvancementsTab(update),
                ..
            } => Some(update),
            _ => None,
        })
        .collect::<Vec<_>>()
    };

    // A root with a display is kept, and the change is answered.
    assert_eq!(
        opened_tab(&mut world, "minecraft:story/root"),
        vec![SelectAdvancementsTab {
            tab: Some("minecraft:story/root".to_string()),
        }]
    );

    // The same tab again changes nothing, so nothing is said.
    assert!(opened_tab(&mut world, "minecraft:story/root").is_empty());

    // An id that names no advancement is dropped before `setSelectedTab` runs.
    assert!(opened_tab(&mut world, "bbb:not_an_advancement").is_empty());

    // A loaded advancement that is not a root clears the selection instead of
    // taking it, and the client is told that too.
    assert_eq!(
        opened_tab(&mut world, "minecraft:story/mine_stone"),
        vec![SelectAdvancementsTab { tab: None }]
    );
    // ...and the selection is now clear, so re-clearing it says nothing.
    assert!(opened_tab(&mut world, "minecraft:story/mine_stone").is_empty());

    // `CLOSED_SCREEN` is not a branch the handler has at all.
    let closed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SeenAdvancements(SeenAdvancements::ClosedScreen),
    );
    assert!(closed.events.is_empty(), "{closed:?}");
}

#[test]
fn every_close_reason_either_carries_its_vanilla_message_or_declares_it_has_none() {
    // `ServerCommonPacketListenerImpl.disconnect` (:141-151) sends the reason
    // before it closes, so a refusal that mirrors a vanilla `disconnect(...)`
    // must carry that call's own component. The ones that mirror a throw carry
    // none: vanilla has no message there either.
    let key = |reason: WorldCloseReason| {
        // The component carries no language file, so its flattened text is
        // the translation key itself.
        reason
            .reason_component()
            .map(|component| component.text().to_string())
    };

    assert_eq!(
        key(WorldCloseReason::DuplicateConnection).as_deref(),
        Some("multiplayer.disconnect.duplicate_login")
    );
    assert_eq!(
        key(WorldCloseReason::DuplicatePlayer).as_deref(),
        Some("multiplayer.disconnect.duplicate_login")
    );
    assert_eq!(
        key(WorldCloseReason::PlayerCapacity).as_deref(),
        Some("multiplayer.disconnect.server_full")
    );
    assert_eq!(
        key(WorldCloseReason::ServerShutdown).as_deref(),
        Some("multiplayer.disconnect.server_shutdown")
    );
    assert_eq!(
        key(WorldCloseReason::ChatValidationFailed).as_deref(),
        Some("multiplayer.disconnect.chat_validation_failed")
    );
    assert_eq!(
        key(WorldCloseReason::InvalidEntityAttacked).as_deref(),
        Some("multiplayer.disconnect.invalid_entity_attacked")
    );
    assert_eq!(
        key(WorldCloseReason::InvalidMovement).as_deref(),
        Some("multiplayer.disconnect.invalid_player_movement")
    );
    assert_eq!(
        key(WorldCloseReason::InvalidVehicleMovement).as_deref(),
        Some("multiplayer.disconnect.invalid_vehicle_movement")
    );
    assert_eq!(
        key(WorldCloseReason::IdleTimeout).as_deref(),
        Some("multiplayer.disconnect.idling")
    );
    assert_eq!(
        key(WorldCloseReason::KeepAliveMismatch).as_deref(),
        Some("disconnect.timeout")
    );
    assert_eq!(
        key(WorldCloseReason::KeepAliveTimeout).as_deref(),
        Some("disconnect.timeout")
    );
    assert_eq!(
        key(WorldCloseReason::UnexpectedQuery).as_deref(),
        Some("multiplayer.disconnect.unexpected_query_response")
    );

    for reason in [
        WorldCloseReason::ChunkGenerationFailed {
            detail: "generation failed".to_string(),
        },
        WorldCloseReason::InvalidBlockInteraction,
        WorldCloseReason::InvalidCommand,
        WorldCloseReason::PacketOutsideActiveContract,
        WorldCloseReason::UnknownConnection,
    ] {
        assert_eq!(key(reason.clone()), None, "{reason:?}");
    }
}

#[test]
fn the_book_editor_saves_pages_and_signs_them_into_a_written_book() {
    // `handleEditBook` (:977-989): a hotbar or off-hand slot holding something
    // with `WRITABLE_BOOK_CONTENT`. Without a title `updateBookContents`
    // (:991-997) replaces the pages in place; with one `signBook` (:999-1011)
    // transmutes the stack into a written book carrying the author's name.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let writable_book = || ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_WRITABLE_BOOK_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let stock = |world: &mut ServerWorld, menu_slot: i16| {
        play(
            &sender,
            &receiver,
            world,
            1,
            PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
                slot_num: menu_slot,
                item: writable_book(),
            }),
        );
    };
    let edited_slot = |world: &mut ServerWorld, slot: usize| {
        play(&sender, &receiver, world, 1, PlayServerbound::ClientTickEnd)
            .snapshot
            .players[0]
            .inventory
            .slots[slot]
            .clone()
    };

    // Menu slot 36 is hotbar slot 0.
    stock(&mut world, 36);
    let saved = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::EditBook(EditBook {
            slot: 0,
            pages: vec!["first".to_string(), "second".to_string()],
            title: None,
        }),
    );
    assert!(
        saved
            .events
            .iter()
            .any(|event| matches!(event, WorldEvent::Send { .. })),
        "{saved:?}"
    );
    let book = edited_slot(&mut world, 0);
    assert_eq!(
        book.item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_WRITABLE_BOOK_ID)
    );
    assert_eq!(
        book.component_patch.writable_book_pages,
        ["first", "second"]
    );
    assert_eq!(
        book.component_patch.writable_book_page_filters,
        [None, None]
    );

    // Signing the same slot.
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::EditBook(EditBook {
            slot: 0,
            pages: vec!["first".to_string(), "second".to_string()],
            title: Some("bbb tales".to_string()),
        }),
    );
    let signed = edited_slot(&mut world, 0);
    assert_eq!(
        signed.item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_WRITTEN_BOOK_ID)
    );
    let written = signed
        .component_patch
        .written_book
        .as_ref()
        .expect("signing writes the written book content");
    assert_eq!(written.title, "bbb tales");
    assert_eq!(written.author, "alice");
    assert_eq!(written.generation, 0);
    assert!(written.resolved);
    assert_eq!(written.pages, ["first", "second"]);
    assert!(signed.component_patch.writable_book_pages.is_empty());
    // The writable content is dropped rather than marked removed: the written
    // book's prototype does not carry it.
    assert!(signed.component_patch.removed_type_ids.is_empty());

    // The write is a server task, not something the request finishes: two
    // edits that arrive in one tick both run at the drain, so the menu
    // broadcast after them finds one changed slot and reports it once. The
    // request itself answers nothing at all.
    stock(&mut world, 36);
    for packet in [
        PlayServerbound::EditBook(EditBook {
            slot: 0,
            pages: vec!["drafted".to_string()],
            title: None,
        }),
        PlayServerbound::EditBook(EditBook {
            slot: 0,
            pages: vec!["drafted".to_string()],
            title: Some("bbb tales".to_string()),
        }),
    ] {
        enqueue(
            &sender,
            WorldCommand::Play {
                connection_id: ConnectionId(1),
                packet,
            },
        )
        .unwrap();
    }
    let batched = world.tick(receiver.drain());
    let slot_updates: Vec<_> = batched
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::ContainerSetSlot(update),
                ..
            } => Some(update),
            _ => None,
        })
        .collect();
    assert_eq!(slot_updates.len(), 1, "{batched:?}");
    assert_eq!(
        slot_updates[0].item.item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_WRITTEN_BOOK_ID)
    );

    // A slot the handler does not accept, and a slot holding no book.
    stock(&mut world, 36);
    for packet in [
        PlayServerbound::EditBook(EditBook {
            slot: 20,
            pages: vec!["ignored".to_string()],
            title: None,
        }),
        PlayServerbound::EditBook(EditBook {
            slot: 1,
            pages: vec!["ignored".to_string()],
            title: None,
        }),
    ] {
        let refused = play(&sender, &receiver, &mut world, 1, packet.clone());
        assert!(refused.events.is_empty(), "{packet:?} produced {refused:?}");
    }
    assert!(edited_slot(&mut world, 0)
        .component_patch
        .writable_book_pages
        .is_empty());
}

#[test]
fn a_signed_command_runs_only_when_its_signatures_name_the_arguments_it_read() {
    // `handleSignedChatCommand` (:1525-1533) and `performSignedChatCommand`
    // (:1535-1550). The signatures never reach the dispatcher: they are matched
    // against `SignableCommand.of` (:1541), and this table's only signable
    // argument is `/say`'s message, which the probe's permission level cannot
    // reach. So a request with no signatures runs the command, and one that
    // signs an argument the parse never read is refused whole.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    // The first player tick's creative ranges (ServerPlayer.java:610-627) sync next tick.
    world.tick(Vec::new());

    let signed = |command: &str, argument_signatures: ArgumentSignatures| {
        PlayServerbound::ChatCommandSigned(ChatCommandSigned {
            command: command.to_string(),
            timestamp_millis: 0,
            salt: 0,
            argument_signatures,
            last_seen_messages: LastSeenMessagesUpdate::default(),
        })
    };
    fn texts(result: &TickResult) -> Vec<String> {
        result
            .events
            .iter()
            .map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::SystemChat(update),
                    ..
                } => update.content.text().to_string(),
                other => panic!("expected a system chat reply, got {other:?}"),
            })
            .collect()
    }

    let listed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        signed("list", ArgumentSignatures::default()),
    );
    assert_eq!(texts(&listed), ["commands.list.players alice"]);

    let refused = play(
        &sender,
        &receiver,
        &mut world,
        1,
        signed(
            "list",
            ArgumentSignatures {
                entries: vec![ArgumentSignature {
                    name: "message".to_string(),
                    signature: MessageSignature {
                        bytes: vec![7; 256],
                    },
                }],
            },
        ),
    );
    assert_eq!(texts(&refused), ["chat.disabled.invalid_command_signature"]);

    // `unpackAndApplyLastSeen` (:1624-1638): a window the validator cannot
    // apply ends the session before the command is even parsed.
    let closed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ChatCommandSigned(ChatCommandSigned {
            command: "list".into(),
            timestamp_millis: 0,
            salt: 0,
            argument_signatures: ArgumentSignatures::default(),
            last_seen_messages: LastSeenMessagesUpdate {
                offset: i32::MAX,
                ..LastSeenMessagesUpdate::default()
            },
        }),
    );
    assert!(
        closed.events.iter().any(|event| matches!(
            event,
            WorldEvent::Close {
                reason: WorldCloseReason::ChatValidationFailed,
                ..
            }
        )),
        "{closed:?}"
    );
}

#[test]
fn command_suggestions_offer_the_siblings_of_the_node_the_cursor_sits_in() {
    // `handleCustomCommandSuggestions` (:588-601) answers every request. What
    // it answers is `CommandDispatcher.getCompletionSuggestions`
    // (CommandDispatcher.java:287-324): `findSuggestionContext`
    // (CommandContextBuilder.java:127-159) picks whose children are offered,
    // `LiteralCommandNode.listSuggestions` (LiteralCommandNode.java:67-69)
    // filters them by what is left to read, and `SuggestionsBuilder.suggest`
    // (SuggestionsBuilder.java:54-60) drops one equal to that remainder.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let ask = |world: &mut ServerWorld, command: &str| {
        let result = play(
            &sender,
            &receiver,
            world,
            1,
            PlayServerbound::CommandSuggestion(CommandSuggestionRequest {
                id: 7,
                command: command.to_string(),
            }),
        );
        let answers = result
            .events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::CommandSuggestions(update),
                    ..
                } => Some(update.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(answers.len(), 1, "{command:?} produced {result:?}");
        let answer = answers.into_iter().next().expect("just counted");
        assert_eq!(answer.id, 7);
        (
            answer.start,
            answer.length,
            answer
                .suggestions
                .into_iter()
                .map(|suggestion| {
                    assert_eq!(suggestion.tooltip, None);
                    suggestion.text.try_to_string().unwrap()
                })
                .collect::<Vec<_>>(),
        )
    };

    // A partial root literal completes to the one command that starts with it.
    assert_eq!(ask(&mut world, "lis"), (0, 3, vec!["list".to_string()]));
    // The leading slash is skipped but still counted, so the range names an
    // offset into the string the client sent.
    assert_eq!(ask(&mut world, "/lis"), (1, 3, vec!["list".to_string()]));
    // A literal typed in full is dropped rather than echoed, and the cursor
    // sits in that node so its own children are not offered either.
    assert_eq!(ask(&mut world, "list"), (0, 0, Vec::<String>::new()));
    // Past the node, its children are what is offered.
    assert_eq!(ask(&mut world, "list u"), (5, 1, vec!["uuids".to_string()]));
    assert_eq!(ask(&mut world, "list uuids"), (0, 0, Vec::<String>::new()));
    // Nothing matches, and an empty answer carries no offset at all.
    assert_eq!(ask(&mut world, "zzz"), (0, 0, Vec::<String>::new()));
    // Completion does not apply `canUse`: `getCompletionSuggestions`
    // (com/mojang/brigadier/CommandDispatcher.java:299-311) walks the
    // dispatcher's own children without asking, while `parseNodes` does ask.
    // So the four owner-level commands starting with `sa` are all offered to a
    // session that could not run any of them.
    assert_eq!(
        ask(&mut world, "sa"),
        (
            0,
            2,
            vec![
                "save-all".to_string(),
                "save-off".to_string(),
                "save-on".to_string(),
                "say".to_string(),
            ]
        )
    );
}

/// `TransferCommand.transfer` (TransferCommand.java:60-65) sends one
/// `ClientboundTransferPacket` per named player and changes nothing else: the
/// client is what reconnects.
#[test]
fn transfer_sends_one_packet_per_named_player_and_leaves_the_session_open() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);

    let tick = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "transfer example.invalid 25566 @a",
    );
    let mut transfers = tick
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::Transfer(update),
            } => Some((connection_id.0, update.host.clone(), update.port)),
            _ => None,
        })
        .collect::<Vec<_>>();
    transfers.sort();
    assert_eq!(
        transfers,
        vec![
            (1, bbb_protocol::JavaString::from("example.invalid"), 25566),
            (2, bbb_protocol::JavaString::from("example.invalid"), 25566),
        ]
    );
    assert!(
        !tick
            .events
            .iter()
            .any(|event| matches!(event, WorldEvent::Close { .. })),
        "a transfer is not a disconnect"
    );
}

/// `MinecraftServer.buildPlayerStatus` (`MinecraftServer.java:1076`) samples a
/// player whose client disables listing as `ANONYMOUS_PLAYER_PROFILE` (`:217`).
#[test]
fn the_status_sample_anonymises_players_that_disable_listing() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    for (connection_id, name, allows_listing) in [(1, "alice", true), (2, "bob", false)] {
        enqueue(
            &sender,
            WorldCommand::Join {
                client_information: ClientInformation {
                    allows_listing,
                    ..ClientInformation::default()
                },
                context: JoinContext::default(),
                connection_id: ConnectionId(connection_id),
                profile: profile(name, connection_id.into()),
            },
        )
        .unwrap();
    }
    let tick = world.tick(receiver.drain());
    assert_eq!(tick.online_player_count, 2);
    let mut sample = tick.online_players.expect("initial join updates listing");
    sample.sort();
    assert_eq!(
        sample,
        vec![
            (Uuid::nil(), "Anonymous Player".to_string()),
            (Uuid::from_u128(1), "alice".to_string()),
        ]
    );

    let unchanged = world.tick(Vec::new());
    assert_eq!(unchanged.online_player_count, 2);
    assert!(unchanged.online_players.is_none());

    let listed = play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::ClientInformation(ClientInformation {
            allows_listing: true,
            ..ClientInformation::default()
        }),
    );
    let mut updated = listed
        .online_players
        .expect("toggling allows_listing updates listing");
    updated.sort();
    assert_eq!(
        updated,
        vec![
            (Uuid::from_u128(1), "alice".to_string()),
            (Uuid::from_u128(2), "bob".to_string()),
        ]
    );
}

/// `updateAwaitingTeleport` (`ServerGamePacketListenerImpl.java:1206-1223`):
/// a move packet more than 20 ticks after an unanswered teleport gets the
/// teleport again under a new id.
#[test]
fn an_unanswered_teleport_is_resent_after_twenty_ticks() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    let teleports = |tick: &TickResult| {
        tick.events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::PlayerPosition(update),
                } => Some((update.id, update.position)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let first = teleports(&chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "tp @s 3 80 4",
    ));
    assert_eq!(first.len(), 1);
    let look = || {
        PlayServerbound::MovePlayer(MovePlayer {
            position: None,
            rotation: Some(PlayerRotation {
                y_rot: 10.0,
                x_rot: 0.0,
            }),
            on_ground: false,
            horizontal_collision: false,
        })
    };
    assert!(teleports(&play(&sender, &receiver, &mut world, 1, look())).is_empty());
    for _ in 0..20 {
        world.tick(Vec::new());
    }
    let resent = teleports(&play(&sender, &receiver, &mut world, 1, look()));
    assert_eq!(resent.len(), 1);
    assert_ne!(resent[0].0, first[0].0);
    assert_eq!(resent[0].1, first[0].1);
    assert_eq!(
        world.player_movement_validation_for_test(ConnectionId(1)).1,
        Some(resent[0].0)
    );
}

#[test]
fn offline_session_ignores_chat_session_update_and_validates_last_seen_as_unsigned() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let session_update = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ChatSessionUpdate(bbb_protocol::packets::ChatSessionUpdate {
            session_id: Uuid::from_u128(99),
            expires_at_epoch_millis: 1_900_000_000_000,
            public_key: vec![0x30, 0x82, 0x01, 0x22],
            key_signature: vec![0xab; 256],
        }),
    );
    assert!(session_update.events.is_empty(), "{session_update:?}");

    // Signed chat with `IGNORE_CHECKSUM` (`0`) decodes through `Decoder.unsigned`
    // (`SignedMessageChain.java:93-100`, `LastSeenMessages.java:81-83`).
    let signed_chat = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Chat(ChatMessage {
            message: "signed client hello".to_string(),
            timestamp_millis: 100,
            salt: 7,
            signature: Some(MessageSignature {
                bytes: vec![0x5a; 256],
            }),
            last_seen_messages: LastSeenMessagesUpdate {
                offset: 0,
                acknowledged: 0,
                checksum: 0,
            },
        }),
    );
    assert!(
        signed_chat.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::PlayerChat(chat),
            } if chat.content == "signed client hello"
        )),
        "{signed_chat:?}"
    );

    // Invalid `acknowledged` or `checksum` closes with `ChatValidationFailed`
    // (`LastSeenMessagesValidator.java:40-76`).
    for bad_update in [
        LastSeenMessagesUpdate {
            offset: 0,
            acknowledged: 1,
            checksum: 1,
        },
        LastSeenMessagesUpdate {
            offset: 0,
            acknowledged: 0,
            checksum: 2,
        },
    ] {
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let closed = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::Chat(ChatMessage {
                last_seen_messages: bad_update,
                ..ChatMessage::unsigned("bad window", 0, 0)
            }),
        );
        assert!(
            closed.events.iter().any(|event| matches!(
                event,
                WorldEvent::Close {
                    reason: WorldCloseReason::ChatValidationFailed,
                    ..
                }
            )),
            "{bad_update:?} -> {closed:?}"
        );
    }
}
