mod management;
mod packet_fuzz;
mod world_saves;

use std::{
    collections::VecDeque,
    net::SocketAddr,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use bbb_facts::sound_sources::SoundSource;
use bbb_protocol::{
    attribute_types::{
        VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID,
        VANILLA_ATTRIBUTE_ENTITY_INTERACTION_RANGE_ID, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
    },
    codec::offline_player_uuid,
    difficulties::Difficulty,
    entity_data_serializers::{
        VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID, VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
    },
    entity_synched_data::{
        VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID, VANILLA_LIVING_ENTITY_HEALTH_DATA_ID,
    },
    frame::{decode_packet_body, encode_packet_with_compression, split_packet, try_read_frame},
    game_types::GameType,
    packets::{
        decode_configuration_clientbound, decode_login_clientbound, decode_play_clientbound,
        decode_status_response, encode_client_information,
        encode_configuration_brand_custom_payload, encode_configuration_finish, encode_handshake,
        encode_login_acknowledged, encode_login_hello, encode_play_accept_teleportation,
        encode_play_chat_message, encode_play_cookie_response, encode_play_move_player_pos_rot,
        encode_play_player_action, encode_play_player_loaded, encode_play_set_carried_item,
        encode_play_set_creative_mode_slot, encode_play_use_item_on, encode_select_known_packs,
        encode_status_request, AttributeModifier, AttributeSnapshot, BlockHitResult, BlockPos,
        ChatMessage, ClientIntent, ConfigurationClientbound, DataComponentPatchSummary, Direction,
        EntityDataValue, EntityDataValueKind, InteractionHand, ItemStackSummary, LoginClientbound,
        PlayClientbound, PlayerAbilities, PlayerAction, PlayerActionKind, RegistryData,
        RegistryTags, SetCreativeModeSlot, UpdateTags, UseItemOn,
    },
    registry_data_types::VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES,
    NbtSummaryValue, PROTOCOL_VERSION,
};
use bytes::BytesMut;
use clap::Parser;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::oneshot,
    task::JoinHandle,
};

use crate::{ServerOptions, ServerReport, ServerRuntime, ServerRuntimeError};

pub(super) fn options_with(extra: &[&str]) -> ServerOptions {
    let mut args = vec![
        "bbb-server",
        // The fixtures below count chunks at the world's own radius and
        // drive an empty server that must keep ticking.
        "--view-distance",
        "2",
        "--simulation-distance",
        "2",
        "--pause-when-empty-seconds",
        "0",
        "--bind",
        "127.0.0.1:0",
        // Every test runtime needs its own management port, or two of them
        // racing for the default would look like a listener that died.
        "--mcp-bind",
        "127.0.0.1:0",
        "--motd",
        "runtime fixture",
        // The fixture world this crate's tests drive is the creative one
        // `ServerWorldConfig::default` describes; the flag's own default is
        // vanilla's `gamemode=survival`, which is a different question from
        // what these tests are for.
        "--gamemode",
        "creative",
    ];
    args.extend_from_slice(extra);
    ServerOptions::try_parse_from(args).unwrap()
}

pub(super) fn test_options() -> ServerOptions {
    options_with(&[])
}

pub(super) fn temp_test_dir(prefix: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "bbb-server-{prefix}-{}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}

pub(super) struct RunningServer {
    pub(super) address: SocketAddr,
    pub(super) management: Option<SocketAddr>,
    shutdown_tx: oneshot::Sender<()>,
    task: JoinHandle<Result<ServerReport, ServerRuntimeError>>,
}

impl RunningServer {
    pub(super) async fn start(options: ServerOptions) -> Self {
        let runtime = ServerRuntime::bind(options).await.unwrap();
        let address = runtime.local_addr().unwrap();
        let management = runtime.management_addr().unwrap();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(runtime.run_until(async {
            let _ = shutdown_rx.await;
        }));
        Self {
            address,
            management,
            shutdown_tx,
            task,
        }
    }

    pub(super) fn management_addr(&self) -> SocketAddr {
        self.management.expect("an endpoint")
    }

    pub(super) async fn stop(self) -> ServerReport {
        self.stop_within(Duration::from_secs(5)).await
    }

    pub(super) async fn stop_within(self, budget: Duration) -> ServerReport {
        let _ = self.shutdown_tx.send(());
        tokio::time::timeout(budget, self.task)
            .await
            .expect("the runtime shuts down")
            .unwrap()
            .unwrap()
    }

    pub(super) async fn wait_stopped(self) -> ServerReport {
        let Self {
            shutdown_tx: _keep_signal_open,
            task,
            ..
        } = self;
        tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .expect("the runtime stops on its own")
            .unwrap()
            .unwrap()
    }
}

pub(super) async fn probe_status(address: SocketAddr) -> (i32, serde_json::Value) {
    let mut client = TestClient::connect(address).await;
    client
        .send(encode_handshake("localhost", address.port(), ClientIntent::Status).unwrap())
        .await;
    client.send(encode_status_request()).await;
    let (packet_id, payload) = client.read().await;
    let status = serde_json::from_str(&decode_status_response(&payload).unwrap()).unwrap();
    (packet_id, status)
}

pub(super) struct TestClient {
    stream: TcpStream,
    buffered: BytesMut,
    pending_play: VecDeque<PlayClientbound>,
    compression_threshold: Option<i32>,
    /// Where the join burst put this session, which
    /// `PlayerSpawnFinder.findSpawn` picks off a thread-local generator
    /// (PlayerSpawnFinder.java:47) rather than from anything a test can pin,
    /// and the chunk that column falls in.
    spawn: (f64, f64, f64),
    spawn_chunk: (i32, i32),
}

impl TestClient {
    pub(super) async fn connect(address: SocketAddr) -> Self {
        Self {
            stream: TcpStream::connect(address).await.unwrap(),
            buffered: BytesMut::new(),
            pending_play: VecDeque::new(),
            compression_threshold: None,
            spawn: (0.0, 0.0, 0.0),
            spawn_chunk: (0, 0),
        }
    }

    pub(super) async fn send(&mut self, packet: (i32, Vec<u8>)) {
        let frame = encode_packet_with_compression(packet.0, &packet.1, self.compression_threshold)
            .unwrap();
        self.stream.write_all(&frame).await.unwrap();
    }

    pub(super) async fn read(&mut self) -> (i32, Vec<u8>) {
        loop {
            if let Some(frame) = try_read_frame(&mut self.buffered).unwrap() {
                let body = decode_packet_body(&frame, self.compression_threshold).unwrap();
                let (packet_id, payload) = split_packet(&body).unwrap();
                return (packet_id, payload.to_vec());
            }
            let mut bytes = [0_u8; 4096];
            let read = self.stream.read(&mut bytes).await.unwrap();
            assert_ne!(read, 0, "server closed before the expected packet");
            self.buffered.extend_from_slice(&bytes[..read]);
        }
    }

    pub(super) async fn read_play_raw(&mut self) -> PlayClientbound {
        self.read_play_raw_within(Duration::from_secs(2)).await
    }

    pub(super) async fn read_play_raw_within(&mut self, budget: Duration) -> PlayClientbound {
        if let Some(packet) = self.pending_play.pop_front() {
            return packet;
        }
        loop {
            let (packet_id, payload) = tokio::time::timeout(budget, self.read())
                .await
                .expect("server did not send the expected play packet");
            // A slow tick budget lets the 15 s keep-alive land mid-sequence;
            // answer it like a client so the session survives the test.
            match decode_play_clientbound(packet_id, &payload).unwrap() {
                PlayClientbound::KeepAlive { id } => {
                    self.send(bbb_protocol::packets::serverbound::encode_play_keep_alive(
                        id,
                    ))
                    .await;
                }
                packet => return packet,
            }
        }
    }

    pub(super) async fn expect_eof(&mut self) {
        loop {
            assert!(
                try_read_frame(&mut self.buffered).unwrap().is_none(),
                "server sent a packet after the terminal disconnect"
            );
            let mut bytes = [0_u8; 4096];
            let read = tokio::time::timeout(Duration::from_secs(2), self.stream.read(&mut bytes))
                .await
                .expect("server did not close after the terminal disconnect")
                .unwrap();
            if read == 0 {
                assert!(
                    self.buffered.is_empty(),
                    "server closed with a partial packet after the disconnect"
                );
                return;
            }
            self.buffered.extend_from_slice(&bytes[..read]);
        }
    }

    pub(super) async fn wait_closed(&mut self) {
        let mut bytes = [0_u8; 4096];
        loop {
            match tokio::time::timeout(Duration::from_secs(5), self.stream.read(&mut bytes))
                .await
                .expect("server closed connection within timeout")
            {
                Ok(0) | Err(_) => return,
                Ok(_) => {}
            }
        }
    }

    /// The next play packet that carries world semantics.
    ///
    /// Chunk delivery is paced independently of everything else
    /// (`PlayerChunkSender.sendNextChunks` runs on its own tick step), so its
    /// packets can land between any two packets a test cares about. The
    /// bracketing itself is asserted with `read_play_raw`.
    ///
    /// `ClientboundSetHealthPacket` is skipped for the same reason:
    /// `ServerPlayer.doTick` (ServerPlayer.java:671-678) sends it on the first
    /// tick of every session and whenever the health moves afterwards, on its
    /// own step rather than in answer to anything a test sent.
    /// Advancement deltas also flush on tick (ServerPlayer.java:607-607).
    /// `ClientboundSetExperiencePacket` comes off the same tick step
    /// (ServerPlayer.java:710-713) and is skipped for the same reason.
    pub(super) async fn read_play(&mut self) -> PlayClientbound {
        loop {
            let packet = self.read_play_raw().await;
            if !matches!(
                packet,
                PlayClientbound::ChunkBatchStart
                    | PlayClientbound::ChunkBatchFinished { .. }
                    | PlayClientbound::LevelChunkWithLight(_)
                    | PlayClientbound::LightUpdate(_)
                    | PlayClientbound::SetHealth(_)
                    | PlayClientbound::SetExperience(_)
                    | PlayClientbound::UpdateAdvancements(_)
                    // Every session unlocks the crafting table on its first
                    // tick: `minecraft:recipes/decorations/crafting_table`
                    // carries an `unlock_right_away` criterion on
                    // `minecraft:tick` whose reward is that recipe, so the
                    // book add rides the join burst like the advancement
                    // progress beside it.
                    | PlayClientbound::RecipeBookAdd(_)
                    // `MinecraftServer.java:1095-1099`: the clock every twenty ticks.
                    | PlayClientbound::SetTime(_)
                    // `ServerEntity.sendChanges` force-syncs tracked positions on its own cadence.
                    | PlayClientbound::EntityPositionSync(_)
            ) {
                return packet;
            }
        }
    }

    /// The chunks of the batches up to the one completing the 3x3 around the
    /// spawn chunk, each batch acknowledged as a client would. A batch carries
    /// only columns that are already FULL (`ChunkHolder.getChunkToSend`), so
    /// the view can take several.
    async fn read_spawn_chunks(&mut self) -> Vec<(i32, i32)> {
        let (spawn_x, spawn_z) = self.spawn_chunk;
        let mut positions = Vec::new();
        while (-1..=1)
            .any(|dx| (-1..=1).any(|dz| !positions.contains(&(spawn_x + dx, spawn_z + dz))))
        {
            match self.read_play_raw().await {
                PlayClientbound::LevelChunkWithLight(chunk) => {
                    assert!(chunk.chunk_data.section_data.len() > 24);
                    positions.push((chunk.x, chunk.z));
                }
                PlayClientbound::ChunkBatchFinished { .. } => {
                    self.send(
                        bbb_protocol::packets::serverbound::encode_play_chunk_batch_received(25.0),
                    )
                    .await;
                }
                _ => {}
            }
        }
        positions
    }

    async fn read_initial_entity_sync(&mut self, entity_id: i32, restored: bool) {
        // Initial client options dirty the skin mask; the tracker sends it to self
        // (`ServerPlayer.java:1834-1843`, `ServerEntity.java:336-342`).
        // The first tick instantiates both interaction ranges, with a creative
        // player's modifiers (`ServerPlayer.java:610-627`). A fresh player does
        // not spuriously instantiate default movement speed when frost ticks are
        // zero, whereas a restored profile applies `abilities.walking_speed`
        // (`Player.java:640`).
        let mut pending = std::mem::take(&mut self.pending_play);
        let mut saw_skin = false;
        let mut saw_attributes = false;
        while !saw_skin || !saw_attributes {
            let packet = self.read_play_raw().await;
            if let PlayClientbound::SetEntityData(update) = &packet {
                if update.id == entity_id {
                    assert!(!saw_skin);
                    // The fresh player's `setHealth(getMaxHealth())` is dirty too.
                    assert_eq!(
                        update.values,
                        vec![
                            EntityDataValue {
                                data_id: VANILLA_LIVING_ENTITY_HEALTH_DATA_ID,
                                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                                value: EntityDataValueKind::Float(20.0),
                            },
                            EntityDataValue {
                                data_id: VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
                                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                                value: EntityDataValueKind::Byte(127),
                            },
                        ]
                    );
                    saw_skin = true;
                    continue;
                }
            }
            if let PlayClientbound::UpdateAttributes(update) = &packet {
                if update.entity_id == entity_id {
                    assert!(!saw_attributes);
                    let creative = update
                        .attributes
                        .first()
                        .is_some_and(|attribute| !attribute.modifiers.is_empty());
                    let range = |attribute_id, base, id: &str, amount| AttributeSnapshot {
                        attribute_id,
                        base,
                        modifiers: if creative {
                            vec![AttributeModifier {
                                id: id.into(),
                                amount,
                                operation_id: 0,
                            }]
                        } else {
                            Vec::new()
                        },
                    };
                    let mut expected = vec![
                        range(
                            VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID,
                            4.5,
                            "minecraft:creative_mode_block_range",
                            0.5,
                        ),
                        range(
                            VANILLA_ATTRIBUTE_ENTITY_INTERACTION_RANGE_ID,
                            3.0,
                            "minecraft:creative_mode_entity_range",
                            2.0,
                        ),
                    ];
                    if restored {
                        expected.push(AttributeSnapshot {
                            attribute_id: VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                            base: f64::from(0.1_f32),
                            modifiers: Vec::new(),
                        });
                    }
                    assert_eq!(update.attributes, expected);
                    saw_attributes = true;
                    continue;
                }
            }
            pending.push_back(packet);
        }
        self.pending_play = pending;
    }

    async fn read_play_without_waypoints(&mut self) -> PlayClientbound {
        loop {
            let packet = self.read_play().await;
            if !matches!(packet, PlayClientbound::Waypoint(_)) {
                return packet;
            }
        }
    }

    /// `read_play_without_waypoints` past the other player's tracker updates.
    ///
    /// `ServerEntity.sendChanges` sends a body's rotation and head turn on its
    /// own cadence, ticks after the move that changed them, so they can land
    /// between any two packets a test sent once a second player is paired.
    async fn read_play_settled(&mut self) -> PlayClientbound {
        loop {
            match self.read_play_without_waypoints().await {
                PlayClientbound::RotateHead(_) | PlayClientbound::MoveEntity(_) => {}
                packet => return packet,
            }
        }
    }

    /// `read_play_settled` with each `SetEquipment` set aside:
    /// `LivingEntity.detectEquipmentUpdates` (`LivingEntity.java:2719`) sends
    /// it from the tracked player's own tick, wherever the tick boundaries
    /// fall between the packets a test sent.
    async fn read_play_noting_equipment(
        &mut self,
        equipment: &mut Vec<bbb_protocol::packets::SetEquipment>,
    ) -> PlayClientbound {
        loop {
            match self.read_play_settled().await {
                PlayClientbound::SetEquipment(update) => equipment.push(update),
                packet => return packet,
            }
        }
    }
}

#[test]
fn command_line_defaults_are_offline_headless_and_bounded() {
    let options = ServerOptions::try_parse_from(["bbb-server"]).unwrap();
    assert_eq!(options.bind.to_string(), "127.0.0.1:25565");
    assert_eq!(options.max_players(), 20);
    assert_eq!(options.max_connections, 128);
    assert_eq!(options.compression_threshold, 256);
    assert!(
        ServerOptions::try_parse_from(["bbb-server", "--compression-threshold", "-2"]).is_err()
    );
}

#[tokio::test]
async fn runtime_serves_bbb_status_and_stops_gracefully() {
    let server = RunningServer::start(test_options()).await;
    let (packet_id, status) = probe_status(server.address).await;
    assert_eq!(
        packet_id,
        bbb_protocol::ids::status::CLIENTBOUND_STATUS_RESPONSE
    );
    assert_eq!(status["version"]["protocol"], PROTOCOL_VERSION);
    assert_eq!(status["description"]["text"], "runtime fixture");

    let report = server.stop().await;
    assert_eq!(report.accepted_connections, 1);
    assert_eq!(report.completed_connections, 1);
}

#[tokio::test]
async fn runtime_serves_the_flat_play_fixture_and_allows_reconnect() {
    let server = RunningServer::start(test_options()).await;
    let mut client = join_play_client(server.address, "fixture-player").await;
    // The join announces the player and then sends no chunk of its own: the
    // tick's chunk sender releases the tracking view inside acknowledgeable
    // batches (`PlayerChunkSender.sendNextChunks`), which is the only shape a
    // client can pace its own chunk rate against.
    assert!(matches!(
        client.read_play_raw().await,
        PlayClientbound::PlayerInfoUpdate(_)
    ));
    let packet = client.read_play_raw().await;
    assert!(
        matches!(packet, PlayClientbound::ChunkBatchStart),
        "{packet:?}"
    );
    let positions = client.read_spawn_chunks().await;
    let mut unique = positions.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), positions.len(), "{positions:?}");
    drop(client);

    tokio::time::sleep(Duration::from_millis(100)).await;
    let _rejoined =
        join_with_intent(server.address, "fixture-player", ClientIntent::Login, true).await;

    let report = server.stop().await;
    assert_eq!(report.accepted_connections, 2);
    assert_eq!(report.completed_connections, 2);
}

#[tokio::test]
async fn runtime_waits_out_a_transient_one_entry_outbound_mailbox() {
    let mut options = test_options();
    options.connection_command_entries = 1;
    let server = RunningServer::start(options).await;

    let mut client = join_play_client(server.address, "one-entry").await;
    assert!(matches!(
        client.read_play_raw().await,
        PlayClientbound::PlayerInfoUpdate(_)
    ));
    let packet = client.read_play_raw().await;
    assert!(
        matches!(packet, PlayClientbound::ChunkBatchStart),
        "{packet:?}"
    );
    client.read_spawn_chunks().await;

    client
        .send(encode_play_chat_message(&ChatMessage::unsigned(
            "still connected",
            42,
            7,
        )))
        .await;
    assert_chat_line(client.read_play().await, "one-entry", "still connected");
    drop(client);

    let report = server.stop().await;
    assert_eq!(report.accepted_connections, 1);
    assert_eq!(report.completed_connections, 1);
}

#[tokio::test]
async fn runtime_registry_publication_replay_is_complete() {
    let server = RunningServer::start(test_options()).await;
    let _client = join_play_client(server.address, "registry-replay").await;
    let report = server.stop().await;
    assert_eq!(report.accepted_connections, 1);
    assert_eq!(report.rejected_connections, 0);
}

#[tokio::test]
async fn runtime_synchronizes_chat_blocks_and_inventory_for_two_clients() {
    let server = RunningServer::start(test_options()).await;

    let mut alice = join_play_client(server.address, "alice").await;
    let mut bob = join_play_client(server.address, "bob").await;
    // `ChunkMap.isChunkTracked` skips chunks still pending send, so bob's join
    // move can land before his chunks go out; resend his position like a
    // client's periodic move so `ChunkMap.move` pairs alice for him.
    let (bob_x, bob_y, bob_z) = bob.spawn;
    bob.send(encode_play_move_player_pos_rot(
        bob_x + 1.0,
        bob_y + 1.0,
        bob_z,
        90.0,
        0.0,
        true,
        false,
    ))
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Player visibility handshake (`PlayerList.java:207-212`). Alice's own
    // tab-list entry was announced when she joined and nobody else existed
    // yet; bob's join sends her his entry plus the AddEntity pairing, and
    // sends bob both entries plus alice's spawn. The initializing
    // PlayerInfoUpdate always precedes the matching AddEntity
    // (`ClientPacketListener.java:592-599`), so assert that order on the wire.
    // Both clients see the same two entries in the same order: alice's entry
    // (her own join announcement for her, the existing-player list for bob)
    // then bob's announcement, which every player receives. Alice also hears
    // bob arrive (`PlayerList.java:193-200`), which bob does not: the list the
    // announcement goes to does not contain him yet.
    for (client, paired_entity, paired_name, hears_the_arrival) in
        [(&mut alice, 2, "bob", true), (&mut bob, 1, "alice", false)]
    {
        assert!(matches!(
            client.read_play_without_waypoints().await,
            PlayClientbound::PlayerInfoUpdate(ref update)
                if info_entry_names(update) == ["alice"]
        ));
        if hears_the_arrival {
            let packet = client.read_play_without_waypoints().await;
            assert!(
                matches!(
                    packet,
                    PlayClientbound::SystemChat(ref chat)
                        if chat.content.text().contains("multiplayer.player.joined")
                ),
                "{packet:?}"
            );
        }
        assert!(matches!(
            client.read_play_without_waypoints().await,
            PlayClientbound::PlayerInfoUpdate(ref update)
                if info_entry_names(update) == ["bob"]
        ));
        // `ServerEntity.addPairing` wraps spawn, data, attributes and equipment
        // in one clientbound bundle (`ServerEntity.java:262-266`).
        let bundle_start = client.read_play_without_waypoints().await;
        assert!(
            matches!(bundle_start, PlayClientbound::BundleDelimiter),
            "expected player-pairing bundle, got {bundle_start:?}"
        );
        let mut saw_pairing = false;
        loop {
            match client.read_play().await {
                PlayClientbound::BundleDelimiter => break,
                PlayClientbound::AddEntity(update) => {
                    assert_eq!(update.id, paired_entity);
                    assert_eq!(update.uuid, offline_player_uuid(paired_name));
                    saw_pairing = true;
                }
                PlayClientbound::SetEntityData(_)
                | PlayClientbound::UpdateAttributes(_)
                | PlayClientbound::SetEquipment(_)
                | PlayClientbound::SetPassengers(_)
                | PlayClientbound::SetEntityLink(_) => {}
                packet => panic!("unexpected player-pairing packet {packet:?}"),
            }
        }
        assert!(saw_pairing);
        if hears_the_arrival {
            let first_tracking_update = client.read_play_without_waypoints().await;
            assert!(
                matches!(
                    first_tracking_update,
                    PlayClientbound::MoveEntity(update)
                        if update.id == paired_entity
                            && update.has_position
                            && (update.delta_x, update.delta_y, update.delta_z) == (0, 0, 0)
                ),
                "expected first tracking move for entity {paired_entity}, got {first_tracking_update:?}"
            );
            client.read_initial_entity_sync(paired_entity, false).await;
        }
    }

    alice
        .send(encode_play_chat_message(&ChatMessage::unsigned(
            "hello fixture",
            42,
            7,
        )))
        .await;
    for client in [&mut alice, &mut bob] {
        assert_chat_line(client.read_play_settled().await, "alice", "hello fixture");
    }

    // The cells alice reaches are hers relative to the column the spawn finder
    // put her in; the fixture's floor is the same y everywhere.
    let (alice_x, _, alice_z) = alice.spawn;
    let floor = |dx: i32, dz: i32| BlockPos {
        x: alice_x.floor() as i32 + dx,
        y: 63,
        z: alice_z.floor() as i32 + dz,
    };
    let broken = floor(1, 1);
    alice
        .send(encode_play_player_action(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: broken,
            direction: Direction::Up,
            sequence: 1,
        }))
        .await;
    // `Block.playerWillDestroy` (`Block.java:518-526`) runs before
    // `Level.removeBlock` (`ServerPlayerGameMode.java:279-280`), so the break
    // particles reach the wire ahead of the block update -- and they reach
    // bob only: `spawnDestroyParticles` hands the breaking player to
    // `levelEvent` (`Block.java:515`) and `PlayerList.broadcast` drops that one
    // by identity (`PlayerList.java:608`).
    let probe = bob.read_play_settled().await;
    assert!(
        matches!(
            probe,
            PlayClientbound::LevelEvent(update)
                if update.event_type == 2001 && update.pos == broken && !update.global
        ),
        "bob got {probe:?}"
    );
    for client in [&mut alice, &mut bob] {
        assert!(matches!(
            client.read_play_settled().await,
            PlayClientbound::BlockUpdate(update)
                if update.pos == broken && update.block_state_id == air_state_id()
        ));
    }
    assert!(matches!(
        alice.read_play_settled().await,
        PlayClientbound::BlockChangedAck(update) if update.sequence == 1
    ));

    let place_on = |pos: BlockPos, sequence: i32| {
        encode_play_use_item_on(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos,
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y: 1.0,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence,
        })
    };

    let clicked = floor(2, 1);
    let placed = BlockPos { y: 64, ..clicked };
    // A join hands out nothing (Inventory.java:55), so the stack bob places
    // from is one he asks the creative menu for first.
    bob.send(
        encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
                count: 64,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        })
        .unwrap(),
    )
    .await;
    bob.send(place_on(clicked, 2)).await;
    // `BlockItem.place` plays the place sound as soon as the block is set
    // (`BlockItem.java:86-87`), while the block change itself rides
    // `ChunkHolder.broadcastChanges` in the chunk map's own tick
    // (`ServerChunkCache.java:362`), so a watching session hears the placement
    // one packet before it sees it. The placing session is excluded by the
    // `player` argument at `:87` and so reads only the block change.
    let mut bob_equipment = Vec::new();
    let alice_sound = alice.read_play_noting_equipment(&mut bob_equipment).await;
    assert!(
        matches!(&alice_sound, PlayClientbound::Sound(sound)
            if sound.source == SoundSource::Blocks && sound.pitch == 0.8),
        "expected the place sound, got {alice_sound:?}"
    );
    let alice_update = alice.read_play_noting_equipment(&mut bob_equipment).await;
    assert!(
        matches!(alice_update, PlayClientbound::BlockUpdate(update)
            if update.pos == placed && update.block_state_id == stone_state_id()),
        "expected placed block, got {alice_update:?}"
    );
    let bob_updates = [
        bob.read_play_settled().await,
        bob.read_play_settled().await,
        bob.read_play_settled().await,
    ];
    assert_eq!(
        bob_updates
            .iter()
            .filter(|packet| matches!(
                packet,
                PlayClientbound::BlockUpdate(update)
                    if update.pos == placed && update.block_state_id == stone_state_id()
            ))
            .count(),
        2
    );
    assert!(matches!(
        bob.read_play_settled().await,
        PlayClientbound::BlockChangedAck(update) if update.sequence == 2
    ));

    let replacement = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 32,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    alice.send(encode_play_set_carried_item(1)).await;
    alice
        .send(
            encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
                slot_num: 37,
                item: replacement,
            })
            .unwrap(),
        )
        .await;
    // `handleSetCreativeModeSlot` answers with silence (:2013-2015), so the
    // write is proved by what alice can do with it rather than by an echo:
    // she now holds the stack the creative slot put in her second hotbar slot,
    // and placing from it is the next thing she hears about. A server that had
    // echoed the write would have this assertion read that echo instead.
    let second_clicked = floor(3, 1);
    let second_placed = BlockPos {
        y: 64,
        ..second_clicked
    };
    alice.send(place_on(second_clicked, 3)).await;
    // The placing session hears the same three updates bob heard above: the
    // two `handleUseItemOn` sends by hand (`ServerGamePacketListenerImpl.java:1384-1385`,
    // the clicked position and the one the block landed on) plus the level's
    // own broadcast of the changed cell, which
    // `ChunkHolder.broadcastChanges` sends to every player tracking the chunk
    // and excludes nobody -- the placer included.
    let alice_updates = [
        alice.read_play_noting_equipment(&mut bob_equipment).await,
        alice.read_play_noting_equipment(&mut bob_equipment).await,
        alice.read_play_noting_equipment(&mut bob_equipment).await,
    ];
    assert_eq!(
        alice_updates
            .iter()
            .filter(|packet| matches!(
                packet,
                PlayClientbound::BlockUpdate(update)
                    if update.pos == second_placed && update.block_state_id == stone_state_id()
            ))
            .count(),
        2
    );
    assert!(matches!(
        alice.read_play_noting_equipment(&mut bob_equipment).await,
        PlayClientbound::BlockChangedAck(update) if update.sequence == 3
    ));
    // Bob's creative write filled his main hand; the placement, a creative
    // one, left it as it was.
    assert_eq!(
        bob_equipment
            .iter()
            .map(|update| (
                update.entity_id,
                update
                    .slots
                    .iter()
                    .map(|slot| (slot.slot, slot.item.item_id, slot.item.count))
                    .collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>(),
        [(
            2,
            vec![(
                bbb_protocol::equipment_slots::EquipmentSlot::MainHand,
                Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
                64
            )]
        )]
    );

    let report = server.stop().await;
    assert_eq!(report.accepted_connections, 2);
}

pub(super) async fn join_play_client(address: SocketAddr, username: &str) -> TestClient {
    join_with_intent(address, username, ClientIntent::Login, false).await
}

#[tokio::test]
async fn transfer_requires_explicit_acceptance_and_omits_join_server_data() {
    let disabled = RunningServer::start(test_options()).await;
    let mut client = TestClient::connect(disabled.address).await;
    client
        .send(
            encode_handshake("localhost", disabled.address.port(), ClientIntent::Transfer).unwrap(),
        )
        .await;
    let (id, payload) = client.read().await;
    assert!(matches!(decode_login_clientbound(id, &payload).unwrap(),
        LoginClientbound::Disconnect { raw_reason, .. } if raw_reason.contains("multiplayer.disconnect.transfers_disabled")));
    disabled.stop().await;

    let enabled = RunningServer::start(options_with(&["--accepts-transfers", "true"])).await;
    let _joined =
        join_with_intent(enabled.address, "traveler", ClientIntent::Transfer, false).await;
    enabled.stop().await;
}

async fn configure_client(
    address: SocketAddr,
    username: &str,
    intent: ClientIntent,
) -> (TestClient, Vec<RegistryData>) {
    let mut client = TestClient::connect(address).await;
    client
        .send(encode_handshake("localhost", address.port(), intent).unwrap())
        .await;
    client
        .send(encode_login_hello(username, offline_player_uuid(username)).unwrap())
        .await;

    let (packet_id, payload) = client.read().await;
    assert_eq!(
        decode_login_clientbound(packet_id, &payload).unwrap(),
        LoginClientbound::SetCompression { threshold: 256 }
    );
    client.compression_threshold = Some(256);
    let (packet_id, payload) = client.read().await;
    assert!(matches!(
        decode_login_clientbound(packet_id, &payload).unwrap(),
        LoginClientbound::LoginFinished { .. }
    ));
    client.send(encode_login_acknowledged()).await;
    client
        .send(encode_configuration_brand_custom_payload("bbb-runtime-test").unwrap())
        .await;
    client
        .send(encode_client_information(&Default::default()).unwrap())
        .await;

    let (packet_id, payload) = client.read().await;
    assert!(matches!(
        decode_configuration_clientbound(packet_id, &payload).unwrap(),
        ConfigurationClientbound::CustomPayload(_)
    ));
    let (packet_id, payload) = client.read().await;
    assert!(matches!(
        decode_configuration_clientbound(packet_id, &payload).unwrap(),
        ConfigurationClientbound::UpdateEnabledFeatures(_)
    ));
    let (packet_id, payload) = client.read().await;
    assert_eq!(
        decode_configuration_clientbound(packet_id, &payload).unwrap(),
        ConfigurationClientbound::SelectKnownPacks {
            known_packs: vec![bbb_server_net::vanilla_core_known_pack()],
        }
    );
    client.send(encode_select_known_packs(&[]).unwrap()).await;
    let mut registries = Vec::new();
    for expected in VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES {
        let (packet_id, payload) = client.read().await;
        let ConfigurationClientbound::RegistryData(registry) =
            decode_configuration_clientbound(packet_id, &payload).unwrap()
        else {
            panic!("expected registry data for {}", expected.resource_id)
        };
        assert_eq!(registry.registry, expected.resource_id);
        assert!(registry
            .entries
            .iter()
            .all(|entry| entry.raw_data.as_ref().is_some_and(|raw| !raw.is_empty())));
        if expected.validator == Some("RegistryValidator.nonEmpty ()") {
            assert!(
                !registry.entries.is_empty(),
                "{} must pass RegistryValidator.nonEmpty()",
                registry.registry
            );
        }
        registries.push(registry);
    }
    let (packet_id, payload) = client.read().await;
    let ConfigurationClientbound::UpdateTags(tags) =
        decode_configuration_clientbound(packet_id, &payload).unwrap()
    else {
        panic!("expected the registry tag transaction")
    };
    validate_registry_transaction(&registries, &tags);
    let (packet_id, payload) = client.read().await;
    assert_eq!(
        decode_configuration_clientbound(packet_id, &payload).unwrap(),
        ConfigurationClientbound::Finish
    );
    client.send(encode_configuration_finish()).await;
    (client, registries)
}

async fn join_with_intent(
    address: SocketAddr,
    username: &str,
    intent: ClientIntent,
    restored: bool,
) -> TestClient {
    let (mut client, registries) = configure_client(address, username, intent).await;
    // The fixed head of the join burst. What follows it — the player-visibility
    // packets and the batched chunk — depends on who else is online, so callers
    // assert those themselves.
    // The tick that places a player also generates the terrain around them,
    // and the whole burst waits for that tick to end; on a loaded machine the
    // first join's tick can run for seconds.
    let mut packets = vec![client.read_play_raw_within(Duration::from_secs(10)).await];
    for _ in 1..16 {
        packets.push(client.read_play_raw().await);
    }
    let PlayClientbound::Login(login) = &packets[0] else {
        panic!("expected login after configuration, got {:?}", packets[0])
    };
    validate_login_dimension_type(login.common_spawn_info.dimension_type_id, &registries);
    // `PlayerList.placeNewPlayer` sends difficulty, abilities, held slot and
    // the recipe catalog in this order (`PlayerList.java:181-188`).
    assert!(matches!(
        packets[1],
        PlayClientbound::ChangeDifficulty(update)
            if update.difficulty == Difficulty::Easy && !update.locked
    ));
    assert!(matches!(
        packets[2],
        PlayClientbound::PlayerAbilities(update)
            if update == PlayerAbilities::for_game_type(GameType::Creative)
    ));
    assert!(matches!(
        packets[3],
        PlayClientbound::SetHeldSlot(update) if update.slot == 0
    ));
    assert!(matches!(packets[4], PlayClientbound::UpdateRecipes(_)));
    // `PlayerList.sendPlayerPermissionLevel` (PlayerList.java:188, 559-572):
    // the session hears its own level and nobody else's. No operator list is
    // read, so it is always the baseline one.
    assert!(matches!(
        packets[5],
        PlayClientbound::EntityEvent(update) if update.event_id == 24
    ));
    // `sendPlayerPermissionLevel` closes with `sendCommands`
    // (PlayerList.java:571), so the session learns the tree together with the
    // level that filtered it. At the baseline that is every command registered
    // at `Commands.LEVEL_ALL` — `/me`, `/help`, `/list`, `/msg` with its two
    // aliases, `/random`'s two draw arms, `/teammsg` with its one, and `/trigger`
    // (TriggerCommand.java:31-67) — and nothing gamemaster-gated: no `/say`,
    // no `/kick`, no `/stop`, and not
    // `/random reset`, whose own `requires` asks for `LEVEL_GAMEMASTERS`
    // (RandomCommand.java:38-44). `ClientboundCommandsPacket.enumerateNodes`
    // numbers the filtered tree breadth-first (ClientboundCommandsPacket.java:81-98).
    let PlayClientbound::Commands(tree) = &packets[6] else {
        panic!("expected the command tree, got {:?}", packets[6])
    };
    assert_eq!(
        tree.nodes
            .iter()
            .filter_map(|node| node.name.as_deref())
            .collect::<Vec<_>>(),
        vec![
            "me",
            "help",
            "list",
            "msg",
            "tell",
            "w",
            "random",
            "teammsg",
            "tm",
            "trigger",
            "action",
            "command",
            "uuids",
            "targets",
            "value",
            "roll",
            "message",
            "objective",
            "message",
            "range",
            "range",
            "add",
            "set",
            "value",
            "value",
        ]
    );
    assert!(matches!(packets[7], PlayClientbound::RecipeBookSettings(_)));
    assert!(matches!(
        packets[8],
        PlayClientbound::RecipeBookAdd(ref add) if add.replace
    ));
    // `PlayerList.sendLevelInfo` (`PlayerList.java:644-657`): the border,
    // the full world-clock sync (the fixture runtime wires no datapack
    // surface, so the clock map is empty) and the default spawn.
    assert!(matches!(packets[9], PlayClientbound::InitializeBorder(_)));
    assert!(matches!(packets[10], PlayClientbound::SetTime(_)));
    assert!(matches!(
        packets[11],
        PlayClientbound::SetDefaultSpawnPosition(_)
    ));
    // `PlayerList.sendLevelInfo` closes with `LEVEL_CHUNKS_LOAD_START`
    // (PlayerList.java:655) and then `updateJoiningPlayer`
    // (`ServerTickRateManager.java:121-124`), which tells the arriving session
    // the tick rate, the freeze flag and any step still owed.
    assert!(matches!(
        packets[12],
        PlayClientbound::GameEvent(update) if update.event_id == 13 && update.param == 0.0
    ));
    assert!(matches!(
        packets[13],
        PlayClientbound::TickingState(update)
            if update.tick_rate == 20.0 && !update.frozen
    ));
    assert!(matches!(
        packets[14],
        PlayClientbound::TickingStep(update) if update.tick_steps == 0
    ));
    let PlayClientbound::PlayerPosition(position) = packets[15] else {
        panic!("expected absolute spawn position after play login")
    };
    // The floor the fixture's flat generator lays: see
    // `the_default_spawn_stands_on_the_fixture_floor` in bbb-server-world.
    assert_eq!(position.position.y, 64.0);
    // `PlayerSpawnFinder.findSpawn` (PlayerSpawnFinder.java:50-87) walks the
    // `respawn_radius` square around the suggestion from an offset it draws off
    // a thread-local generator (`:47`), so which column a session lands in is
    // not a contract. What is, is that the point is a `Vec3.atBottomCenterOf`
    // (`:80`) inside that square: the rule's default radius is ten
    // (GameRules.java:76) and the fixture's own spawn is the origin column.
    let radius = 10.0;
    for axis in [position.position.x, position.position.z] {
        assert!(
            (axis - 0.5).abs() <= radius,
            "{axis} is outside the respawn-radius square"
        );
        assert_eq!(axis.fract().abs(), 0.5, "{axis} is not a block centre");
    }
    // The centre follows the column the finder chose, through
    // `SectionPos.blockToSectionCoord(double)` (SectionPos.java:84-86).
    let center = (
        (position.position.x.floor() as i32) >> 4,
        (position.position.z.floor() as i32) >> 4,
    );
    client.spawn = (
        position.position.x,
        position.position.y,
        position.position.z,
    );
    client.spawn_chunk = center;
    let mut saw_center = false;
    let mut saw_player_info = false;
    let mut joining_packets = VecDeque::new();
    let menu = loop {
        match client.read_play_raw().await {
            // `PlayerList.java:202-205`, right behind the spawn teleport.
            PlayClientbound::ServerData(data) if !saw_center => {
                assert_eq!(intent, ClientIntent::Login);
                assert!(data.icon_bytes.is_none());
            }
            PlayClientbound::SetChunkCacheCenter(update) => {
                assert_eq!((update.chunk_x, update.chunk_z), center);
                saw_center = true;
            }
            PlayClientbound::Waypoint(_) if saw_center => {}
            packet @ PlayClientbound::PlayerInfoUpdate(_) if saw_center => {
                saw_player_info = true;
                joining_packets.push_back(packet);
            }
            PlayClientbound::RecipeBookAdd(add) if saw_player_info => {
                assert!(!add.replace);
            }
            packet @ (PlayClientbound::BundleDelimiter
            | PlayClientbound::AddEntity(_)
            | PlayClientbound::SetEntityData(_)
            | PlayClientbound::UpdateAttributes(_)
            | PlayClientbound::SetEquipment(_)
            | PlayClientbound::SetPassengers(_)
            | PlayClientbound::SetEntityLink(_))
                if saw_player_info =>
            {
                joining_packets.push_back(packet);
            }
            // `ServerPlayer.initInventoryMenu` at the end of join
            // (`PlayerList.java:215`) publishes the always-open inventory menu.
            PlayClientbound::ContainerSetContent(menu) if saw_player_info => break menu,
            packet => panic!(
                "expected chunk centre, player visibility, then inventory menu; got {packet:?}"
            ),
        }
    };
    // PlayerList.java:207-215: visibility precedes menu initialization; callers
    // still inspect those visibility packets in their original order.
    client.pending_play = joining_packets;
    assert_eq!(menu.container_id, 0);
    assert_eq!(menu.state_id, 1);
    assert_eq!(menu.items.len(), 46);
    // A fresh session's inventory is the one `Player` is built with
    // (Player.java:174-180), whose slots are `ItemStack.EMPTY`
    // (Inventory.java:55): nothing is handed out at join.
    assert!(
        menu.items
            .iter()
            .all(|item| *item == ItemStackSummary::empty()),
        "{:?}",
        menu.items
    );
    assert_eq!(menu.carried_item, ItemStackSummary::empty());
    client
        .send(encode_play_accept_teleportation(position.id))
        .await;
    // One block over and one up from wherever the finder put the session, so
    // every caller downstream reaches the same cells relative to its own spawn.
    let (spawn_x, spawn_y, spawn_z) = client.spawn;
    client
        .send(encode_play_move_player_pos_rot(
            spawn_x + 1.0,
            spawn_y + 1.0,
            spawn_z,
            90.0,
            0.0,
            true,
            false,
        ))
        .await;
    client.send(encode_play_player_loaded()).await;
    client
        .read_initial_entity_sync(login.player_id, restored)
        .await;
    client
}

fn validate_registry_transaction(registries: &[RegistryData], tags: &UpdateTags) {
    assert_eq!(
        registries.len(),
        VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES.len()
    );
    assert_eq!(
        tags.registries
            .iter()
            .map(|registry| registry.registry.as_str())
            .collect::<Vec<_>>(),
        [
            "minecraft:worldgen/biome",
            "minecraft:painting_variant",
            "minecraft:damage_type",
            "minecraft:banner_pattern",
            "minecraft:enchantment",
            "minecraft:instrument",
            "minecraft:dialog",
            "minecraft:timeline",
            "minecraft:block",
            "minecraft:item",
            "minecraft:entity_type",
            "minecraft:fluid",
            "minecraft:game_event",
            "minecraft:point_of_interest_type",
            "minecraft:potion",
        ]
    );
    for registry in &tags.registries {
        assert!(
            !registry.tags.is_empty(),
            "{} has no wire tags",
            registry.registry
        );
        let holder_count = registry_holder_count(registry, registries);
        for tag in &registry.tags {
            for &holder_id in &tag.entries {
                assert!(
                    holder_id >= 0 && usize::try_from(holder_id).unwrap() < holder_count,
                    "{} tag {} has out-of-range holder id {holder_id}/{holder_count}",
                    registry.registry,
                    tag.tag
                );
            }
        }
    }
}

fn registry_holder_count(registry: &RegistryTags, published: &[RegistryData]) -> usize {
    if let Some(published) = published
        .iter()
        .find(|candidate| candidate.registry == registry.registry)
    {
        return published.entries.len();
    }
    bbb_server_datapack::tag_loader::network_safe_static_registry_elements()
        .into_iter()
        .find(|(id, _)| *id == registry.registry)
        .map(|(_, elements)| elements.len())
        .unwrap_or_else(|| panic!("tag packet named unknown registry {}", registry.registry))
}

fn validate_login_dimension_type(dimension_type_id: i32, registries: &[RegistryData]) {
    let dimension_types = registries
        .iter()
        .find(|registry| registry.registry == "minecraft:dimension_type")
        .expect("the synchronized catalog contains dimension types");
    let entry = dimension_types
        .entries
        .get(usize::try_from(dimension_type_id).expect("dimension type id is non-negative"))
        .expect("login dimension type id resolves in the published registry");
    assert_eq!(entry.id, "minecraft:overworld");
    let NbtSummaryValue::Compound(fields) = bbb_protocol::decode_nbt_summary(
        entry
            .raw_data
            .as_ref()
            .expect("known-packs are empty, so dimension type contents are present"),
    )
    .unwrap()
    .expect("dimension type payload is not TAG_End") else {
        panic!("published dimension type is not a compound")
    };
    let field = |name: &str| {
        fields
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| &entry.value)
    };
    assert_eq!(field("min_y"), Some(&NbtSummaryValue::Int(-64)));
    assert_eq!(field("height"), Some(&NbtSummaryValue::Int(384)));
    assert_eq!(field("logical_height"), Some(&NbtSummaryValue::Int(384)));
    assert_eq!(
        field("coordinate_scale"),
        Some(&NbtSummaryValue::Double(1.0_f64.to_bits()))
    );
    assert!(matches!(
        field("ambient_light"),
        Some(NbtSummaryValue::Float(_))
    ));
    assert!(matches!(
        field("attributes"),
        Some(NbtSummaryValue::Compound(attributes)) if !attributes.is_empty()
    ));
    assert!(field("has_fixed_time").is_none());
    assert!(field("skybox").is_none());
}

/// The player names carried by an initializing `PlayerInfoUpdate`, in entry
/// order. Every entry the join path sends includes the ADD_PLAYER action, so a
/// missing profile is a real defect rather than a shape this helper tolerates.
fn info_entry_names(update: &bbb_protocol::packets::PlayerInfoUpdate) -> Vec<&str> {
    update
        .entries
        .iter()
        .map(|entry| {
            entry
                .profile
                .as_ref()
                .map(|profile| profile.name.as_str())
                .expect("initializing player info entries carry a profile")
        })
        .collect()
}

fn air_state_id() -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_AIR_ID,
    )
    .unwrap()
}

fn stone_state_id() -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
    )
    .unwrap()
}

#[tokio::test]
async fn runtime_rejects_invalid_programmatic_configuration_before_ready() {
    let mut options = test_options();
    options.max_players = Some(0);
    assert!(ServerRuntime::bind(options).await.is_err());

    let mut options = test_options();
    options.motd = Some("x".repeat(32_768));
    assert!(ServerRuntime::bind(options).await.is_err());
}

#[test]
fn headless_manifest_excludes_client_and_presentation_crates() {
    let manifest = include_str!("../Cargo.toml");
    for forbidden in [
        "bbb-client",
        "bbb-client-assets",
        "bbb-client-audio",
        "bbb-client-clock",
        "bbb-client-net",
        "bbb-client-render",
        "bbb-client-world",
        "winit",
        "wgpu",
    ] {
        assert!(
            !manifest.lines().any(|line| {
                let line = line.trim_start();
                line.starts_with(forbidden)
                    && line[forbidden.len()..]
                        .starts_with(|ch: char| ch.is_whitespace() || ch == '=')
            }),
            "bbb-server must not depend on {forbidden}"
        );
    }
}

#[tokio::test]
async fn a_refused_session_is_told_why_before_the_connection_ends() {
    // `ServerCommonPacketListenerImpl.handleCookieResponse` (:113-116)
    // disconnects with `DISCONNECT_UNEXPECTED_QUERY` (:38), and
    // `disconnect` (:141-151) sends that reason before it closes. A client
    // that only saw the socket die would have nothing to show the player.
    let server = RunningServer::start(test_options()).await;

    let mut client = join_play_client(server.address, "refused-player").await;
    client
        .send(encode_play_cookie_response("bbb:probe", None).unwrap())
        .await;

    let mut reason = None;
    for _ in 0..16 {
        if let PlayClientbound::Disconnect(disconnect) = client.read_play().await {
            reason = Some(disconnect.reason);
            break;
        }
    }
    assert_eq!(
        reason.as_deref(),
        Some("multiplayer.disconnect.unexpected_query_response")
    );
    client.expect_eof().await;
    drop(client);

    server.stop().await;
}

/// Readiness is announced from the place that has a world to serve, and the
/// stop handlers are installed before anything can be told the server is up.
///
/// The window this closes is real: while `server.ready` was logged straight
/// after `ServerRuntime::bind`, a supervisor that read it and sent `SIGTERM`
/// hit a process whose world was not open yet and whose shutdown future had
/// never been polled, so the signal took its default disposition and the save
/// that a stop is supposed to flush never happened. There is no unit test for a
/// race, so what is pinned instead is the ordering that removes it.
///
/// Vanilla's own readiness line has the same meaning: `DedicatedServer.initServer`
/// prints it after `loadLevel` returns (`DedicatedServer.java:282`, `:285`).
#[test]
fn readiness_is_announced_after_the_world_is_open_and_the_handlers_are_installed() {
    let main = include_str!("main.rs");
    let install = main
        .find("ShutdownSignals::install()")
        .expect("main installs the shutdown handlers");
    let bind = main
        .find("ServerRuntime::bind(")
        .expect("main binds the runtime");
    assert!(
        install < bind,
        "the handlers have to be installed before the sockets are bound"
    );
    assert!(
        !main.contains("server.ready"),
        "readiness belongs to the runtime, which is what knows the world is open"
    );

    let runtime = include_str!("runtime.rs");
    let ready = runtime
        .find(r#"event = "server.ready""#)
        .expect("the runtime announces readiness");
    let restore = runtime
        .find("ServerWorld::restore(")
        .expect("the runtime opens the world");
    assert!(
        restore < ready,
        "readiness has to follow the world it is readiness for"
    );
}

/// A plain chat line: an unsigned player chat carrying the message, bound to
/// a chat type over the sender's display name.
fn assert_chat_line(packet: PlayClientbound, sender: &str, message: &str) {
    let PlayClientbound::PlayerChat(chat) = packet else {
        panic!("expected a chat line, got {packet:?}");
    };
    assert_eq!(chat.signature, None, "{chat:?}");
    assert_eq!(chat.body.content, message, "{chat:?}");
    assert_eq!(chat.chat_type.name, sender, "{chat:?}");
}
