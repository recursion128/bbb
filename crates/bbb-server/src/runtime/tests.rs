use std::collections::BTreeSet;

use bbb_protocol::NbtSummaryValue;
use bbb_server_admin::AdminCall;
use bbb_server_core::admin::{AdminOutcome, AdminRequest, AdminResult};
use bbb_server_core::mailbox::WorldCommandReceiver;
use bbb_server_core::world::{ClientboundPlayPacket, WorldCloseReason, WorldEvent};
use bbb_server_net::ConnectionCommandSender;
use bbb_server_types::save::Java26LevelKey;
use clap::Parser;

use super::packet_encoder::{encode_world_packet, world_packet_kind};
use super::*;

#[tokio::test]
async fn a_closed_driver_drains_accepted_input_before_disconnect_despite_world_backpressure() {
    use bbb_protocol::{
        frame::encode_packet,
        packets::{
            encode_configuration_finish, encode_handshake, encode_login_acknowledged,
            encode_login_hello, encode_select_known_packs, ClientIntent,
        },
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    for fail_route in [false, true] {
        let listener = ServerListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let mut client = tokio::net::TcpStream::connect(address).await.unwrap();
        let (remote, handle, driver) = listener
            .accept(
                Arc::new(ServerConnectionConfig::new(serde_json::json!({}))),
                bbb_server_net::ConnectionLimits::default(),
            )
            .await
            .unwrap();
        let route = handle.command_sender();
        let mut limits = WorldMailboxLimits::default();
        limits.network.entries = 1;
        let (sender, receiver) = world_command_mailbox(limits).unwrap();
        sender
            .try_send(WorldCommand::PlayPacket {
                connection_id: ConnectionId(99),
                packet_id: 1,
                payload: vec![9],
            })
            .unwrap();
        let held = receiver.drain();
        let (_shutdown, shutdown) = watch::channel(false);
        let permit = Arc::new(Semaphore::new(1)).acquire_owned().await.unwrap();
        let ip_permit = IpConnectionLimiter::new(1, 1, Duration::from_secs(10), 4)
            .try_acquire(remote.ip(), Instant::now())
            .unwrap();
        let task = tokio::spawn(run_connection(
            ConnectionId(1),
            remote,
            handle,
            driver,
            sender,
            shutdown,
            super::ConnectionPermits {
                _connection: permit,
                _ip: ip_permit,
            },
        ));

        for packet in [
            encode_handshake("localhost", address.port(), ClientIntent::Login).unwrap(),
            encode_login_hello("drained-player", uuid::Uuid::nil()).unwrap(),
        ] {
            client
                .write_all(&encode_packet(packet.0, &packet.1))
                .await
                .unwrap();
        }
        let mut finished = [0; 4096];
        assert!(
            timeout(Duration::from_secs(5), client.read(&mut finished))
                .await
                .unwrap()
                .unwrap()
                > 0
        );
        for packet in [
            encode_login_acknowledged(),
            encode_select_known_packs(&[]).unwrap(),
            encode_configuration_finish(),
            (7, b"first accepted input".to_vec()),
            (8, b"second accepted input".to_vec()),
        ] {
            client
                .write_all(&encode_packet(packet.0, &packet.1))
                .await
                .unwrap();
        }
        client.shutdown().await.unwrap();
        timeout(Duration::from_secs(5), client.read_to_end(&mut Vec::new()))
            .await
            .unwrap()
            .unwrap();
        if fail_route {
            let routes = Mutex::new(BTreeMap::from([(ConnectionId(1), route)]));
            let (gateway, _receiver) = test_gateway();
            let stats = dispatch_world_events(
                vec![WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: invalid_transfer(),
                }],
                &routes,
                None,
                &mut TickRequests::default(),
                &gateway,
                Duration::from_millis(25),
            )
            .await
            .unwrap();
            assert!(stats.unavailable.contains(&ConnectionId(1)));
        }
        assert!(
            receiver.drain().is_empty(),
            "Disconnect cannot overtake quota-blocked input"
        );
        assert!(!task.is_finished());
        drop(held);
        let mut received = Vec::new();
        timeout(Duration::from_secs(5), async {
            while received.len() < 4 {
                for envelope in receiver.drain() {
                    received.push(match envelope.command() {
                        WorldCommand::Join { profile, .. } => {
                            ("join", profile.name.as_bytes().to_vec())
                        }
                        WorldCommand::PlayPacket {
                            packet_id: 7,
                            payload,
                            ..
                        } => ("play:7", payload.clone()),
                        WorldCommand::PlayPacket {
                            packet_id: 8,
                            payload,
                            ..
                        } => ("play:8", payload.clone()),
                        WorldCommand::Disconnect { .. } => ("disconnect", Vec::new()),
                        other => panic!("unexpected command {other:?}"),
                    });
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            received,
            vec![
                ("join", b"drained-player".to_vec()),
                ("play:7", b"first accepted input".to_vec()),
                ("play:8", b"second accepted input".to_vec()),
                ("disconnect", Vec::new())
            ]
        );
        assert!(task.await.unwrap().outcome.is_ok());
        assert!(
            receiver.drain().is_empty(),
            "the lifecycle submits one Disconnect"
        );
    }
}

async fn accept_unpolled_route(
    runtime: &ServerRuntime,
    config: &Arc<ServerConnectionConfig>,
    limits: bbb_server_net::ConnectionLimits,
) -> (
    tokio::net::TcpStream,
    ConnectionCommandSender,
    ServerConnectionDriver,
) {
    let address = runtime.local_addr().unwrap();
    let (client, accepted) = tokio::join!(
        tokio::net::TcpStream::connect(address),
        runtime.listener.accept(Arc::clone(config), limits),
    );
    let (_, handle, driver) = accepted.unwrap();
    (client.unwrap(), handle.command_sender(), driver)
}

async fn route_test_runtime() -> (ServerRuntime, Arc<ServerConnectionConfig>) {
    let options =
        ServerOptions::try_parse_from(["bbb-server", "--bind", "127.0.0.1:0", "--no-mcp"]).unwrap();
    let runtime = ServerRuntime::bind(options).await.unwrap();
    let config = runtime.connection_config(&StatusView::from_options(&runtime.options), None);
    (runtime, config)
}

fn route_limits(entries: usize, bytes: usize) -> bbb_server_net::ConnectionLimits {
    bbb_server_net::ConnectionLimits {
        event_entries: 1,
        event_bytes: 1024,
        command_entries: entries,
        command_bytes: bytes,
    }
}

#[rustfmt::skip]
fn enable_moon_pack(save: &mut Java26CreatedSave) {
    let NbtSummaryValue::Compound(data) = &mut save.level.entries_mut()[0].value else {
        panic!("created level data is a compound")
    };
    let packs = data.iter_mut().find(|entry| entry.name == "DataPacks").unwrap();
    let NbtSummaryValue::Compound(packs) = &mut packs.value else { unreachable!() };
    let enabled = packs.iter_mut().find(|entry| entry.name == "Enabled").unwrap();
    let NbtSummaryValue::List(enabled) = &mut enabled.value else { unreachable!() };
    enabled.push(NbtSummaryValue::string("file/moon"));
}

#[rustfmt::skip]
fn write_custom_flat_pack(root: &Path) {
    let pack = root.join("datapacks/moon");
    std::fs::create_dir_all(pack.join("data/example/dimension")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"moon","min_format":101,"max_format":101}}"#).unwrap();
    std::fs::write(pack.join("data/example/dimension/moon.json"), r#"{"type":"minecraft:overworld","generator":{"type":"minecraft:flat","settings":{"biome":"minecraft:plains","features":false,"lakes":false,"layers":[{"block":"minecraft:stone","height":1}],"structure_overrides":[]}}}"#).unwrap();
}

fn retained_command_bytes(command: &ConnectionCommand) -> usize {
    match command {
        ConnectionCommand::SendPlayPacket { payload, .. } => {
            std::mem::size_of::<ConnectionCommand>() + payload.len()
        }
        ConnectionCommand::SendPreparedPlayPacket(prepared) => {
            std::mem::size_of::<ConnectionCommand>() + prepared.framed().len()
        }
        ConnectionCommand::StartConfiguration { .. } | ConnectionCommand::Close => {
            std::mem::size_of::<ConnectionCommand>()
        }
    }
}

fn test_gateway() -> (Arc<RuntimeGateway>, WorldCommandReceiver) {
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    (RuntimeGateway::new(sender, 40), receiver)
}

fn admin_outcome(operation_id: u64) -> AdminOutcome {
    AdminOutcome {
        operation_id,
        request_id: "route-barrier".to_string(),
        principal: "loopback".to_string(),
        scope: AdminScope::Manage,
        action: "allowlist.clear",
        tick: 7,
        replayed: false,
        result: Ok(AdminResult::Sent(true)),
    }
}

fn invalid_transfer() -> ClientboundPlayPacket {
    ClientboundPlayPacket::Transfer(bbb_protocol::packets::Transfer {
        host: "a".repeat(32_768).into(),
        port: 25_565,
    })
}

#[tokio::test]
async fn a_full_route_does_not_block_later_events_for_another_route() {
    let (runtime, config) = route_test_runtime().await;
    let limits = route_limits(1, 1024);
    let (_slow_client, slow_route, _slow_driver) =
        accept_unpolled_route(&runtime, &config, limits).await;
    let (_fast_client, fast_route, fast_driver) =
        accept_unpolled_route(&runtime, &config, limits).await;
    slow_route.try_send(ConnectionCommand::Close).unwrap();
    let routes = Mutex::new(BTreeMap::from([
        (ConnectionId(1), slow_route),
        (ConnectionId(2), fast_route),
    ]));
    let mut requests = TickRequests::default();
    let (gateway, _receiver) = test_gateway();
    let mut dispatch = Box::pin(dispatch_world_events(
        vec![
            WorldEvent::Broadcast {
                recipients: vec![ConnectionId(1), ConnectionId(2)],
                packet: ClientboundPlayPacket::ChunkBatchStart,
            },
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::ChunkBatchStart,
            },
        ],
        &routes,
        config.compression_threshold,
        &mut requests,
        &gateway,
        Duration::from_millis(50),
    ));

    let mut fast_driver = Box::pin(fast_driver.run());
    tokio::select! {
        result = &mut dispatch => panic!("slow route did not remain blocked: {result:?}"),
        outcome = &mut fast_driver => assert!(matches!(
            outcome.unwrap().reason,
            bbb_server_net::ConnectionCloseReason::ProtocolViolation(_),
        )),
    }
    let stats = dispatch.await.unwrap();
    assert_eq!(
        stats.handed_off.get(&ConnectionId(2)),
        Some(&vec!["chunk_batch_start"])
    );
    assert_eq!(stats.failure_packets[&ConnectionId(1)], "timeout");
}

#[tokio::test]
async fn admin_completion_waits_for_prior_network_handoff() {
    let (runtime, config) = route_test_runtime().await;
    let limits = route_limits(1, 1024);
    let (_client, route, _driver) = accept_unpolled_route(&runtime, &config, limits).await;
    route.try_send(ConnectionCommand::Close).unwrap();
    let routes = Mutex::new(BTreeMap::from([(ConnectionId(1), route)]));
    let mut requests = TickRequests::default();
    let (gateway, _receiver) = test_gateway();
    let pending = gateway
        .submit(AdminCall {
            principal: "loopback".to_string(),
            scope: AdminScope::Manage,
            request_id: "route-barrier".to_string(),
            request: AdminRequest::AllowlistClear,
        })
        .unwrap();
    let mut answer = pending.answer;
    let mut dispatch = Box::pin(dispatch_world_events(
        vec![
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::ChunkBatchStart,
            },
            WorldEvent::Admin(admin_outcome(pending.operation_id)),
        ],
        &routes,
        config.compression_threshold,
        &mut requests,
        &gateway,
        Duration::from_millis(25),
    ));

    tokio::select! {
        result = &mut answer => panic!("admin completed before network barrier: {result:?}"),
        result = &mut dispatch => panic!("dispatch finished before its full route timed out: {result:?}"),
        _ = tokio::task::yield_now() => {}
    }
    assert!(answer.try_recv().is_err());
    let stats = dispatch.await.unwrap();
    assert_eq!(answer.await.unwrap().operation_id, pending.operation_id);
    assert_eq!(stats.unavailable, BTreeSet::from([ConnectionId(1)]));
}

#[tokio::test]
async fn encode_failure_flushes_the_valid_prefix_before_closing() {
    let (runtime, config) = route_test_runtime().await;
    let limits = route_limits(4, 4096);
    let (_client, route, _driver) = accept_unpolled_route(&runtime, &config, limits).await;
    let routes = Mutex::new(BTreeMap::from([(ConnectionId(1), route)]));
    let mut requests = TickRequests::default();
    let (gateway, _receiver) = test_gateway();

    let stats = dispatch_world_events(
        vec![
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::ChunkBatchStart,
            },
            WorldEvent::Broadcast {
                recipients: vec![ConnectionId(1)],
                packet: invalid_transfer(),
            },
        ],
        &routes,
        config.compression_threshold,
        &mut requests,
        &gateway,
        Duration::from_millis(25),
    )
    .await
    .unwrap();
    assert_eq!(
        stats.handed_off.get(&ConnectionId(1)),
        Some(&vec!["chunk_batch_start"])
    );
    assert_eq!(stats.failure_packets[&ConnectionId(1)], "transfer");
    assert_eq!(stats.unavailable, BTreeSet::from([ConnectionId(1)]));
}

#[tokio::test]
async fn oversized_command_flushes_the_valid_prefix_before_closing() {
    let (runtime, config) = route_test_runtime().await;
    let valid = encode_world_packet(
        ClientboundPlayPacket::ChunkBatchStart,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
    .unwrap();
    let valid_bytes = retained_command_bytes(&valid);
    let oversized_packet = ClientboundPlayPacket::Transfer(bbb_protocol::packets::Transfer {
        host: "a".repeat(200).into(),
        port: 25_565,
    });
    assert!(
        retained_command_bytes(
            &encode_world_packet(
                oversized_packet.clone(),
                bbb_protocol::content_registry::ContentRegistry::vanilla(),
            )
            .unwrap()
        ) > valid_bytes
    );
    let limits = route_limits(4, valid_bytes);
    let (_client, route, _driver) = accept_unpolled_route(&runtime, &config, limits).await;
    let routes = Mutex::new(BTreeMap::from([(ConnectionId(1), route)]));
    let mut requests = TickRequests::default();
    let (gateway, _receiver) = test_gateway();
    let stats = dispatch_world_events(
        vec![
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::ChunkBatchStart,
            },
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: oversized_packet,
            },
        ],
        &routes,
        config.compression_threshold,
        &mut requests,
        &gateway,
        Duration::from_millis(25),
    )
    .await
    .unwrap();
    assert_eq!(
        stats.handed_off.get(&ConnectionId(1)),
        Some(&vec!["chunk_batch_start"])
    );
    assert_eq!(stats.failure_packets[&ConnectionId(1)], "transfer");
    assert_eq!(stats.unavailable, BTreeSet::from([ConnectionId(1)]));
}

#[tokio::test]
async fn terminal_reason_and_close_reject_every_following_send() {
    let (runtime, config) = route_test_runtime().await;
    let limits = route_limits(4, 4096);
    let (_client, route, _driver) = accept_unpolled_route(&runtime, &config, limits).await;
    let routes = Mutex::new(BTreeMap::from([(ConnectionId(1), route)]));
    let mut requests = TickRequests::default();
    let (gateway, _receiver) = test_gateway();
    let stats = dispatch_world_events(
        vec![
            WorldEvent::Close {
                connection_id: ConnectionId(1),
                reason: WorldCloseReason::ServerShutdown,
            },
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::ChunkBatchStart,
            },
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::ChunkBatchStart,
            },
        ],
        &routes,
        config.compression_threshold,
        &mut requests,
        &gateway,
        Duration::from_millis(25),
    )
    .await
    .unwrap();
    assert_eq!(
        stats.handed_off.get(&ConnectionId(1)),
        Some(&vec!["disconnect", "close"])
    );
    assert_eq!(
        stats.rejected_after_terminal,
        BTreeSet::from([ConnectionId(1)])
    );
}

#[tokio::test]
async fn outbound_failures_close_the_failed_routes() {
    let (runtime, config) = route_test_runtime().await;
    let (gateway, _receiver) = test_gateway();

    let closed_limits = route_limits(2, 1024);
    let (_closed_client, closed_route, closed_driver) =
        accept_unpolled_route(&runtime, &config, closed_limits).await;
    drop(closed_driver);
    let oversized_limits = route_limits(2, 1);
    let (_oversized_client, oversized_route, _oversized_driver) =
        accept_unpolled_route(&runtime, &config, oversized_limits).await;
    let timeout_limits = route_limits(1, 1024);
    let (_timeout_client, timeout_route, _timeout_driver) =
        accept_unpolled_route(&runtime, &config, timeout_limits).await;
    timeout_route.try_send(ConnectionCommand::Close).unwrap();
    let encode_limits = route_limits(2, 1024);
    let (_encode_client, encode_route, _encode_driver) =
        accept_unpolled_route(&runtime, &config, encode_limits).await;

    for (connection_id, route, packet, expected_failure) in [
        (
            ConnectionId(1),
            closed_route,
            ClientboundPlayPacket::ChunkBatchStart,
            "chunk_batch_start",
        ),
        (
            ConnectionId(2),
            oversized_route,
            ClientboundPlayPacket::ChunkBatchStart,
            "chunk_batch_start",
        ),
        (
            ConnectionId(3),
            timeout_route,
            ClientboundPlayPacket::ChunkBatchStart,
            "timeout",
        ),
        (
            ConnectionId(4),
            encode_route,
            invalid_transfer(),
            "transfer",
        ),
    ] {
        let routes = Mutex::new(BTreeMap::from([(connection_id, route)]));
        let mut requests = TickRequests::default();
        let stats = dispatch_world_events(
            vec![WorldEvent::Send {
                connection_id,
                packet,
            }],
            &routes,
            config.compression_threshold,
            &mut requests,
            &gateway,
            Duration::from_millis(20),
        )
        .await
        .unwrap();
        assert_eq!(stats.unavailable, BTreeSet::from([connection_id]));
        assert_eq!(stats.failure_packets[&connection_id], expected_failure);
        assert!(routes.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn staging_does_not_spend_the_per_handoff_timeout() {
    let (runtime, config) = route_test_runtime().await;
    let limits = route_limits(2, 4096);
    let (_client, route, _driver) = accept_unpolled_route(&runtime, &config, limits).await;
    let routes = Mutex::new(BTreeMap::from([(ConnectionId(1), route)]));
    let mut dispatch = RouteDispatch::new(
        &routes,
        config.compression_threshold,
        Duration::from_millis(25),
    );
    dispatch.stage_send(ConnectionId(1), ClientboundPlayPacket::ChunkBatchStart);
    tokio::time::sleep(Duration::from_millis(30)).await;
    dispatch.flush().await.unwrap();
    dispatch.senders[&ConnectionId(1)]
        .try_send(ConnectionCommand::Close)
        .unwrap();
    dispatch.stage_send(ConnectionId(1), ClientboundPlayPacket::ChunkBatchStart);
    tokio::time::sleep(Duration::from_millis(30)).await;
    dispatch.flush().await.unwrap();
    assert_eq!(dispatch.handed_off[&ConnectionId(1)], ["chunk_batch_start"]);
    assert_eq!(dispatch.failure_packets[&ConnectionId(1)], "timeout");
    assert!(dispatch.is_unavailable(ConnectionId(1)));
}

#[tokio::test]
async fn route_tasks_move_payloads_and_hold_only_one_encoded_command() {
    let (runtime, config) = route_test_runtime().await;
    let limits = route_limits(4, 4096);
    let (_client, route, _driver) = accept_unpolled_route(&runtime, &config, limits).await;
    let (_second_client, second_route, _second_driver) =
        accept_unpolled_route(&runtime, &config, limits).await;
    let routes = Mutex::new(BTreeMap::from([
        (ConnectionId(1), route),
        (ConnectionId(2), second_route),
    ]));
    let mut dispatch = RouteDispatch::new(
        &routes,
        config.compression_threshold,
        Duration::from_millis(25),
    );
    let packet = ClientboundPlayPacket::Transfer(bbb_protocol::packets::Transfer {
        host: "moved-payload".repeat(8).into(),
        port: 25_565,
    });
    dispatch.stage_send(ConnectionId(1), packet);
    dispatch.stage_send(ConnectionId(1), ClientboundPlayPacket::ChunkBatchStart);
    dispatch.stage_send(ConnectionId(1), ClientboundPlayPacket::ChunkBatchStart);
    let PendingRouteEvent::Send(packet) = &dispatch.pending_routes[&ConnectionId(1)].events[0]
    else {
        panic!("first raw route event was not staged")
    };
    let ClientboundPlayPacket::Transfer(transfer) = &**packet else {
        panic!("first raw route event was not the moved transfer")
    };
    assert_eq!(transfer.host, "moved-payload".repeat(8));
    assert!(std::mem::size_of::<PendingRouteEvent>() <= std::mem::size_of::<WorldEvent>());

    dispatch.stage_broadcast(
        vec![ConnectionId(1), ConnectionId(2)],
        ClientboundPlayPacket::Transfer(bbb_protocol::packets::Transfer {
            host: "shared-broadcast".repeat(8).into(),
            port: 25_565,
        }),
    );
    let PendingRouteEvent::Shared(shared) = &dispatch.pending_routes[&ConnectionId(1)].events[3]
    else {
        panic!("broadcast was not staged on the first route")
    };
    let shared = Arc::clone(shared);
    let PendingRouteEvent::Shared(second) = &dispatch.pending_routes[&ConnectionId(2)].events[0]
    else {
        panic!("broadcast was not staged on the second route")
    };
    assert!(Arc::ptr_eq(&shared, second));
    assert!(shared.encoded.get().is_none());
    dispatch.flush().await.unwrap();
    assert!(shared.packet.lock().unwrap().is_none());
    let ConnectionCommand::SendPreparedPlayPacket(first) = shared.command().await.unwrap() else {
        unreachable!()
    };
    let ConnectionCommand::SendPreparedPlayPacket(second) = shared.command().await.unwrap() else {
        unreachable!()
    };
    assert!(!first.framed().is_empty());
    assert!(Arc::ptr_eq(first.framed_shared(), second.framed_shared()));
    assert_eq!(dispatch.max_encoded_in_flight[&ConnectionId(1)], 1);
    assert_eq!(
        dispatch.handed_off[&ConnectionId(1)],
        [
            "transfer",
            "chunk_batch_start",
            "chunk_batch_start",
            "transfer"
        ]
    );
    assert_eq!(dispatch.handed_off[&ConnectionId(2)], ["transfer"]);
}

#[test]
fn ephemeral_datapack_startup_loads_the_builtin_vanilla_surface() {
    let options = ServerOptions::try_parse_from(["bbb-server"]).unwrap();
    let startup = load_datapack_startup(
        &options,
        None,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
    .unwrap();
    assert_eq!(startup.selection.ordered_ids, vec!["vanilla".to_string()]);
    assert!(!startup.recipes.recipes.is_empty());
    assert!(!startup.structures.templates.is_empty());
}

#[test]
fn safe_mode_forces_the_vanilla_only_selection() {
    let options = ServerOptions::try_parse_from(["bbb-server", "--safe-mode"]).unwrap();
    let startup = load_datapack_startup(
        &options,
        None,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
    .unwrap();
    assert_eq!(startup.selection.ordered_ids, vec!["vanilla".to_string()]);
}

#[test]
fn saved_pack_selection_builds_custom_level_geometry_before_deep_open() {
    let root = std::env::temp_dir().join(format!(
        "bbb-server-custom-level-bootstrap-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    write_custom_flat_pack(&root);
    let options = ServerOptions::try_parse_from([
        "bbb-server",
        "--world",
        root.to_str().expect("test path is UTF-8"),
    ])
    .unwrap();
    let mut save = created_world(&root, &options).unwrap();
    enable_moon_pack(&mut save);
    let storage = Java26WorldStorage::open(
        WorldSavePaths::new(&root).unwrap(),
        WorldSaveLimits::default(),
    )
    .unwrap();
    storage
        .create_world(&save, RegionCompression::Zlib, 1)
        .unwrap();
    storage.close().unwrap();

    let (storage, bootstrap) = open_world_storage(
        &root,
        &options,
        Arc::clone(bbb_protocol::content_registry::ContentRegistry::vanilla()),
    )
    .unwrap();
    let startup = load_datapack_startup(
        &options,
        Some(bootstrap.level()),
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
    .unwrap();
    let key = Java26LevelKey::parse("example:moon").unwrap();
    assert!(startup
        .registries
        .registry("minecraft:dimension")
        .unwrap()
        .elements
        .iter()
        .any(|element| element.id == key.resource_id()));
    let candidate = bbb_server_core::world_creation::load_candidate_from_bootstrap(
        &storage,
        bootstrap,
        &startup.registries,
    )
    .unwrap();
    assert!(candidate.dimensions.contains_key(&key));
    let world = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(startup),
            ..ServerWorldConfig::default()
        },
        &candidate,
    )
    .unwrap();
    assert!(!world
        .unserved_levels()
        .iter()
        .any(|(level, _)| *level == key.resource_id()));
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn encode_world_packet_debug_updates_round_trip() {
    use bbb_protocol::{
        debug_subscription_types::{
            VANILLA_DEBUG_POI_TYPES, VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENT_LISTENERS_ID,
            VANILLA_DEBUG_SUBSCRIPTION_POIS_ID, VANILLA_DEBUG_SUBSCRIPTION_RAIDS_ID,
        },
        packets::{
            BlockPos, ChunkPos, DebugBlockValue, DebugChunkValue, DebugGameEventListenerInfo,
            DebugPoiInfo, DebugSubscriptionUpdate, DebugSubscriptionValue, PlayClientbound,
        },
    };

    let block_updates = [
        (
            VANILLA_DEBUG_SUBSCRIPTION_POIS_ID,
            Some(DebugSubscriptionValue::Poi(DebugPoiInfo {
                pos: BlockPos {
                    x: -11,
                    y: -32,
                    z: 17,
                },
                poi_type_holder_id: VANILLA_DEBUG_POI_TYPES[0].holder_id,
                free_ticket_count: 1,
            })),
        ),
        (
            VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENT_LISTENERS_ID,
            Some(DebugSubscriptionValue::GameEventListener(
                DebugGameEventListenerInfo {
                    listener_radius: 16,
                },
            )),
        ),
        (VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENT_LISTENERS_ID, None),
    ]
    .into_iter()
    .map(|(subscription_id, value)| {
        let update = DebugBlockValue {
            pos: BlockPos { x: -7, y: 64, z: 3 },
            update: DebugSubscriptionUpdate {
                subscription_id,
                value,
            },
        };
        (
            ClientboundPlayPacket::DebugBlockValue(Arc::new(update.clone())),
            PlayClientbound::DebugBlockValue(update),
        )
    });
    let chunk_updates = [
        Some(DebugSubscriptionValue::Raids(vec![BlockPos {
            x: 2,
            y: 70,
            z: -9,
        }])),
        None,
    ]
    .into_iter()
    .map(|value| {
        let update = DebugChunkValue {
            pos: ChunkPos { x: -3, z: -4 },
            update: DebugSubscriptionUpdate {
                subscription_id: VANILLA_DEBUG_SUBSCRIPTION_RAIDS_ID,
                value,
            },
        };
        (
            ClientboundPlayPacket::DebugChunkValue(Arc::new(update.clone())),
            PlayClientbound::DebugChunkValue(update),
        )
    });
    for (packet, expected) in block_updates.chain(chunk_updates) {
        let ConnectionCommand::SendPlayPacket { packet_id, payload } = encode_world_packet(
            packet,
            bbb_protocol::content_registry::ContentRegistry::vanilla(),
        )
        .unwrap() else {
            panic!("debug update must produce one play packet");
        };
        assert_eq!(
            bbb_protocol::packets::decode_play_clientbound(packet_id, &payload).unwrap(),
            expected
        );
    }
}

#[test]
fn encode_world_packet_light_update_uses_the_26_1_wire_shape() {
    use bbb_protocol::packets::{LightUpdate, LightUpdateData, PlayClientbound};

    let update = LightUpdate {
        chunk_x: -3,
        chunk_z: -9,
        light_data: LightUpdateData {
            sky_y_mask: vec![0b10],
            block_y_mask: vec![0b100],
            empty_sky_y_mask: vec![0b1000],
            empty_block_y_mask: vec![0b1],
            sky_updates: vec![vec![0x12; 2048]],
            block_updates: vec![vec![0x34; 2048]],
        },
    };
    let packet = ClientboundPlayPacket::LightUpdate(Arc::new(update.clone()));
    assert_eq!(world_packet_kind(&packet), "light_update");
    let ConnectionCommand::SendPlayPacket { packet_id, payload } = encode_world_packet(
        packet,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
    .unwrap() else {
        panic!("world packet encoder must produce one play packet");
    };
    assert_eq!(packet_id, bbb_protocol::ids::play::CLIENTBOUND_LIGHT_UPDATE);
    assert_eq!(
        bbb_protocol::packets::decode_play_clientbound(packet_id, &payload).unwrap(),
        PlayClientbound::LightUpdate(update)
    );
}

#[test]
fn encode_world_packet_projectile_power_uses_the_26_1_wire_shape() {
    let update = bbb_protocol::packets::ProjectilePower {
        entity_id: 41,
        acceleration_power: 0.125,
    };
    let ConnectionCommand::SendPlayPacket { packet_id, payload } = encode_world_packet(
        ClientboundPlayPacket::ProjectilePower(update),
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
    .unwrap() else {
        panic!("world packet encoder must produce one play packet");
    };
    assert_eq!(
        packet_id,
        bbb_protocol::packet_catalog::play::CLIENTBOUND_PROJECTILE_POWER
    );
    assert_eq!(
        bbb_protocol::packets::decode_play_clientbound(packet_id, &payload).unwrap(),
        bbb_protocol::packets::PlayClientbound::ProjectilePower(update)
    );
}

#[test]
fn encode_world_packet_sound_entity_uses_the_26_1_wire_shape() {
    let update = bbb_protocol::packets::SoundEntityEvent {
        sound: bbb_protocol::packets::SoundEventHolder::Reference { registry_id: 41 },
        source: bbb_protocol::packets::SoundSource::Blocks,
        entity_id: 123,
        volume: 1.0,
        pitch: 0.75,
        seed: -9,
    };
    let packet = ClientboundPlayPacket::SoundEntity(update.clone());
    assert_eq!(world_packet_kind(&packet), "sound_entity");
    let ConnectionCommand::SendPlayPacket { packet_id, payload } = encode_world_packet(
        packet,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
    .unwrap() else {
        panic!("world packet encoder must produce one play packet");
    };
    assert_eq!(
        packet_id,
        bbb_protocol::packet_catalog::play::CLIENTBOUND_SOUND_ENTITY
    );
    assert_eq!(
        bbb_protocol::packets::decode_play_clientbound(packet_id, &payload).unwrap(),
        bbb_protocol::packets::PlayClientbound::SoundEntity(update)
    );
}

#[test]
fn encode_world_packet_boss_event_uses_the_26_1_wire_shape() {
    let update = bbb_protocol::packets::BossEvent {
        id: "00000000-0000-0000-0000-000000001234".parse().unwrap(),
        operation: bbb_protocol::packets::BossEventOperation::UpdateProperties {
            flags: bbb_protocol::packets::BossEventFlags {
                darken_screen: true,
                play_music: false,
                create_world_fog: true,
            },
        },
    };
    let packet = ClientboundPlayPacket::BossEvent(update.clone());
    assert_eq!(world_packet_kind(&packet), "boss_event");
    let ConnectionCommand::SendPlayPacket { packet_id, payload } = encode_world_packet(
        packet,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
    .unwrap() else {
        panic!("world packet encoder must produce one play packet");
    };
    assert_eq!(
        packet_id,
        bbb_protocol::packet_catalog::play::CLIENTBOUND_BOSS_EVENT
    );
    assert_eq!(
        bbb_protocol::packets::decode_play_clientbound(packet_id, &payload).unwrap(),
        bbb_protocol::packets::PlayClientbound::BossEvent(update)
    );
}

#[test]
fn encode_world_packet_map_item_data_uses_the_26_1_wire_shape() {
    let update = bbb_protocol::packets::MapItemData {
        map_id: 42,
        scale: 2,
        locked: true,
        decorations: Some(Vec::new()),
        color_patch: None,
    };
    let packet = ClientboundPlayPacket::MapItemData(update.clone());
    assert_eq!(world_packet_kind(&packet), "map_item_data");
    let ConnectionCommand::SendPlayPacket { packet_id, payload } = encode_world_packet(
        packet,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
    .unwrap() else {
        panic!("world packet encoder must produce one play packet");
    };
    assert_eq!(
        packet_id,
        bbb_protocol::packet_catalog::play::CLIENTBOUND_MAP_ITEM_DATA
    );
    assert_eq!(
        bbb_protocol::packets::decode_play_clientbound(packet_id, &payload).unwrap(),
        bbb_protocol::packets::PlayClientbound::MapItemData(update)
    );
}

#[test]
fn encode_world_packet_tracked_waypoint_uses_the_26_1_wire_shape() {
    use bbb_protocol::packets::{
        TrackedWaypoint, TrackedWaypointPacket, WaypointData, WaypointIcon, WaypointIdentifier,
        WaypointOperation, WaypointVec3i,
    };

    let update = TrackedWaypointPacket {
        operation: WaypointOperation::Track,
        waypoint: TrackedWaypoint {
            identifier: WaypointIdentifier::Uuid(
                "00000000-0000-0000-0000-000000001234".parse().unwrap(),
            ),
            icon: WaypointIcon {
                style: "minecraft:default".to_string(),
                color_rgb: Some(0x12_34_56),
            },
            data: WaypointData::Position(WaypointVec3i { x: -3, y: 64, z: 9 }),
        },
    };
    let packet = ClientboundPlayPacket::TrackedWaypoint(update.clone());
    assert_eq!(world_packet_kind(&packet), "tracked_waypoint");
    let ConnectionCommand::SendPlayPacket { packet_id, payload } = encode_world_packet(
        packet,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
    .unwrap() else {
        panic!("world packet encoder must produce one play packet");
    };
    assert_eq!(packet_id, bbb_protocol::ids::play::CLIENTBOUND_WAYPOINT);
    assert_eq!(
        bbb_protocol::packets::decode_play_clientbound(packet_id, &payload).unwrap(),
        bbb_protocol::packets::PlayClientbound::Waypoint(update)
    );
}

/// `Main.java:245` creates the level with the `hardcore` property; a save that
/// already exists is opened with its own.
#[test]
fn the_hardcore_option_creates_a_hardcore_world_only() {
    let root = std::env::temp_dir().join(format!(
        "bbb-server-hardcore-creation-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let open = |hardcore: bool| {
        let mut arguments = vec![
            "bbb-server",
            "--world",
            root.to_str().expect("test path is UTF-8"),
        ];
        if hardcore {
            arguments.push("--hardcore");
        }
        let options = ServerOptions::try_parse_from(arguments).unwrap();
        let (storage, bootstrap) = open_world_storage(
            &root,
            &options,
            Arc::clone(bbb_protocol::content_registry::ContentRegistry::vanilla()),
        )
        .unwrap();
        let hardcore = bootstrap.level().difficulty_settings().hardcore();
        storage.close().unwrap();
        hardcore
    };
    assert!(open(true));
    std::fs::remove_dir_all(&root).unwrap();
    std::fs::create_dir_all(&root).unwrap();
    assert!(!open(false));
    assert!(!open(true), "an existing save keeps its own flag");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn ip_connection_limiter_enforces_active_cap_rate_window_and_tracks_release() {
    let limiter = IpConnectionLimiter::new(2, 3, Duration::from_secs(10), 4);
    let ip_a: IpAddr = "198.51.100.10".parse().unwrap();
    let ip_b: IpAddr = "198.51.100.11".parse().unwrap();
    let start = Instant::now();

    let p1 = limiter.try_acquire(ip_a, start).unwrap();
    let p2 = limiter.try_acquire(ip_a, start).unwrap();
    assert_eq!(
        limiter.try_acquire(ip_a, start).err(),
        Some(IpLimitRejection::ActiveLimit)
    );
    // Another IP is unaffected by ip_a's active cap.
    let _other = limiter.try_acquire(ip_b, start).unwrap();

    drop(p1);
    let p3 = limiter.try_acquire(ip_a, start).unwrap();
    drop(p2);
    drop(p3);
    // Three attempts already recorded inside the 10-second window.
    assert_eq!(
        limiter.try_acquire(ip_a, start).err(),
        Some(IpLimitRejection::RateLimit)
    );
    // After the window expires, attempts succeed again.
    let _later = limiter
        .try_acquire(ip_a, start + Duration::from_secs(11))
        .unwrap();
}

#[test]
fn status_view_sample_shuffles_and_caps_online_players() {
    let players = (0..20)
        .map(|i| (format!("uuid-{i}"), format!("player-{i}")))
        .collect::<Vec<_>>();
    let view = StatusView {
        motd: String::new(),
        max_players: 20,
        hide_online_players: false,
        status_replies: true,
        accepts_transfers: false,
        players,
    };
    let sample_a = view.sample_with_seed(1);
    let sample_b = view.sample_with_seed(42);
    assert_eq!(sample_a.len(), StatusView::MAX_SAMPLE);
    assert_eq!(sample_b.len(), StatusView::MAX_SAMPLE);
    assert_ne!(sample_a, sample_b);
}

#[test]
fn mod_pack_singular_cli_alias_is_accepted() {
    let options =
        ServerOptions::try_parse_from(["bbb-server", "--mod-pack", "packs/example"]).unwrap();
    assert_eq!(
        options.mod_packs,
        vec![std::path::PathBuf::from("packs/example")]
    );
}

#[tokio::test]
async fn max_connections_per_ip_option_caps_simultaneous_connections_per_address() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let options = crate::tests::options_with(&["--max-connections-per-ip", "2"]);
    let server = crate::tests::RunningServer::start(options).await;
    let addr = server.address;

    let first = tokio::net::TcpStream::connect(addr).await.unwrap();
    let second = tokio::net::TcpStream::connect(addr).await.unwrap();
    // Wait briefly so both active permits are registered before the 3rd attempt.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut third = tokio::net::TcpStream::connect(addr).await.unwrap();
    let handshake = bbb_protocol::packets::encode_handshake(
        "localhost",
        addr.port(),
        bbb_protocol::packets::ClientIntent::Status,
    )
    .unwrap();
    let _ = third
        .write_all(&bbb_protocol::frame::encode_packet(
            handshake.0,
            &handshake.1,
        ))
        .await;
    let status_req = bbb_protocol::packets::encode_status_request();
    let _ = third
        .write_all(&bbb_protocol::frame::encode_packet(
            status_req.0,
            &status_req.1,
        ))
        .await;
    let mut buf = [0_u8; 64];
    let read = tokio::time::timeout(Duration::from_secs(2), third.read(&mut buf)).await;
    match read {
        Ok(Ok(0)) | Ok(Err(_)) => {}
        other => panic!("expected 3rd connection past per-ip limit 2 to close, got {other:?}"),
    }

    drop(first);
    drop(second);
    tokio::time::sleep(Duration::from_millis(50)).await;
    let (id, _) = crate::tests::probe_status(addr).await;
    assert_eq!(id, bbb_protocol::ids::status::CLIENTBOUND_STATUS_RESPONSE);

    server.stop().await;
}
