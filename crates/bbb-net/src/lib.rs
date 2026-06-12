use std::{net::ToSocketAddrs, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use bbb_protocol::{
    codec::offline_player_uuid,
    frame::{
        decode_packet_body, encode_packet, encode_packet_with_compression, split_packet,
        try_read_frame,
    },
    ids,
    packets::{
        self, AddEntity, BlockChangedAck, BlockUpdate, ChunksBiomes, ClientIntent,
        ConfigurationClientbound, ContainerClose, ContainerSetContent, ContainerSetData,
        ContainerSetSlot, EntityMove, EntityPositionSync, ForgetLevelChunk, GameEvent,
        InteractionHand, LevelChunkWithLight, LightUpdate, LoginClientbound, PickItemFromBlock,
        PlayClientbound, PlayLogin, PlayTime, PlayerAbilities, PlayerAction, PlayerCommand,
        PlayerExperience, PlayerHealth, PlayerInput, PlayerPositionState, PlayerPositionUpdate,
        RemoveEntities, Respawn, RotateHead, SectionBlocksUpdate, SetChunkCacheCenter,
        SetChunkCacheRadius, SetCursorItem, SetDefaultSpawnPosition, SetEntityData,
        SetEntityMotion, SetEquipment, SetHeldSlot, SetPlayerInventory, SetSimulationDistance,
        SystemChat, TeleportEntity, UseItem, UseItemOn,
    },
};
use bbb_world::{
    BlockPos, BlockProbe, ChunkColumn, ChunkPos, ChunkState, WorldCounters, WorldStore,
};
use bytes::BytesMut;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
    time::{interval, timeout, Interval, MissedTickBehavior},
};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionOptions {
    pub address: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub profile_id: Uuid,
    pub timeout: Duration,
}

impl ConnectionOptions {
    pub fn offline(address: impl Into<String>, username: impl Into<String>) -> Result<Self> {
        let address = address.into();
        let username = username.into();
        let (host, port) = split_host_port(&address)?;
        let profile_id = offline_player_uuid(&username);
        Ok(Self {
            address,
            host,
            port,
            username,
            profile_id,
            timeout: Duration::from_secs(20),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    Handshake,
    Status,
    Login,
    Configuration,
    Play,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusPing {
    pub json: String,
    pub latency_ms: u128,
}

#[derive(Debug, Clone)]
pub enum NetEvent {
    Connected,
    Disconnected {
        reason: Option<String>,
    },
    StateChanged {
        state: ConnectionState,
    },
    CompressionSet {
        threshold: i32,
    },
    PacketSeen {
        state: ConnectionState,
        packet_id: i32,
        len: usize,
    },
    ContainerClose(ContainerClose),
    ContainerSetContent(ContainerSetContent),
    ContainerSetData(ContainerSetData),
    ContainerSetSlot(ContainerSetSlot),
    SetCursorItem(SetCursorItem),
    SetPlayerInventory(SetPlayerInventory),
    AddEntity(AddEntity),
    MoveEntity(EntityMove),
    EntityPositionSync(EntityPositionSync),
    RemoveEntities(RemoveEntities),
    RotateHead(RotateHead),
    SetEntityData(SetEntityData),
    SetEntityMotion(SetEntityMotion),
    SetEquipment(SetEquipment),
    TeleportEntity(TeleportEntity),
    RegistryData {
        registry: String,
        raw_payload_len: usize,
    },
    Login(PlayLogin),
    Respawn(Respawn),
    PlayerPosition(PlayerPositionUpdate),
    PlayerAbilities(PlayerAbilities),
    PlayerHealth(PlayerHealth),
    PlayerExperience(PlayerExperience),
    HeldSlot(SetHeldSlot),
    SetDefaultSpawnPosition(SetDefaultSpawnPosition),
    SetSimulationDistance(SetSimulationDistance),
    SystemChat(SystemChat),
    GameEvent(GameEvent),
    SetTime(PlayTime),
    BlockUpdate(BlockUpdate),
    SectionBlocksUpdate(SectionBlocksUpdate),
    SetChunkCacheCenter(SetChunkCacheCenter),
    SetChunkCacheRadius(SetChunkCacheRadius),
    ForgetLevelChunk(ForgetLevelChunk),
    LevelChunkWithLight(LevelChunkWithLight),
    LightUpdate(LightUpdate),
    ChunksBiomes(ChunksBiomes),
    BlockChangedAck(BlockChangedAck),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerMoveCommand {
    pub state: PlayerPositionState,
    pub on_ground: bool,
    pub horizontal_collision: bool,
}

impl PlayerMoveCommand {
    fn encode_packet(self) -> (i32, Vec<u8>) {
        packets::encode_play_move_player_pos_rot(
            self.state.position.x,
            self.state.position.y,
            self.state.position.z,
            self.state.y_rot,
            self.state.x_rot,
            self.on_ground,
            self.horizontal_collision,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NetCommand {
    MovePlayer(PlayerMoveCommand),
    PlayerAction(PlayerAction),
    PlayerCommand(PlayerCommand),
    PlayerInput(PlayerInput),
    SetHeldSlot(u8),
    Swing(InteractionHand),
    UseItemOn(UseItemOn),
    UseItem(UseItem),
    PickItemFromBlock(PickItemFromBlock),
    Disconnect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeReport {
    pub reached_state: ConnectionState,
    pub compression_threshold: Option<i32>,
    pub packets_seen: usize,
    pub registries_seen: usize,
    pub first_chunk: Option<ChunkPos>,
    pub first_chunk_summary: Option<ChunkProbeSummary>,
    pub first_chunk_center_block: Option<BlockProbe>,
    pub world_counters: WorldCounters,
    #[serde(skip)]
    pub world: WorldStore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkProbeSummary {
    pub pos: ChunkPos,
    pub state: ChunkState,
    pub heightmaps: usize,
    pub sections: usize,
    pub block_entities: usize,
    pub sky_light_arrays: usize,
    pub block_light_arrays: usize,
}

impl ChunkProbeSummary {
    fn from_column(column: &ChunkColumn) -> Self {
        Self {
            pos: column.pos,
            state: column.state,
            heightmaps: column.heightmaps.len(),
            sections: column.sections.len(),
            block_entities: column.block_entities.len(),
            sky_light_arrays: column.light.sky_updates.len(),
            block_light_arrays: column.light.block_updates.len(),
        }
    }
}

pub async fn status_ping(address: &str) -> Result<StatusPing> {
    let (host, port) = split_host_port(address)?;
    let mut conn = RawConnection::connect(address, None).await?;

    let (id, payload) = packets::encode_handshake(&host, port, ClientIntent::Status);
    conn.send_packet(id, &payload).await?;
    let (id, payload) = packets::encode_status_request();
    conn.send_packet(id, &payload).await?;

    let started = std::time::Instant::now();
    let (packet_id, payload) = conn.read_packet().await?;
    if packet_id != ids::status::CLIENTBOUND_STATUS_RESPONSE {
        bail!("expected status response packet, got {packet_id}");
    }
    let json = packets::decode_status_response(&payload)?;

    let ping_time = started.elapsed().as_millis() as i64;
    let (id, payload) = packets::encode_ping_request(ping_time);
    conn.send_packet(id, &payload).await?;
    let (packet_id, payload) = conn.read_packet().await?;
    if packet_id != ids::status::CLIENTBOUND_PONG_RESPONSE {
        bail!("expected pong response packet, got {packet_id}");
    }
    let _ = packets::decode_pong_response(&payload)?;

    Ok(StatusPing {
        json,
        latency_ms: started.elapsed().as_millis(),
    })
}

pub async fn run_offline_probe(options: ConnectionOptions) -> Result<ProbeReport> {
    timeout(options.timeout, run_offline_probe_inner(options))
        .await
        .context("offline probe timed out")?
}

pub async fn run_offline_event_stream(
    options: ConnectionOptions,
    events: mpsc::Sender<NetEvent>,
    mut commands: mpsc::Receiver<NetCommand>,
) -> Result<()> {
    let mut conn = timeout(
        options.timeout,
        RawConnection::connect(&options.address, None),
    )
    .await
    .context("offline connect timed out")??;
    let mut state = ConnectionState::Login;
    let mut player_loaded_sent = false;
    let mut player_position_state = PlayerPositionState::default();
    let mut player_was_dead = false;
    let mut play_tick = None;

    emit(&events, NetEvent::Connected).await?;
    emit(&events, NetEvent::StateChanged { state }).await?;

    let (id, payload) = packets::encode_handshake(&options.host, options.port, ClientIntent::Login);
    conn.send_packet(id, &payload).await?;
    let (id, payload) = packets::encode_login_hello(&options.username, options.profile_id);
    conn.send_packet(id, &payload).await?;

    loop {
        let drive = read_packet_or_drive_connection(
            &mut conn,
            state,
            &mut play_tick,
            &mut commands,
            &mut player_position_state,
        )
        .await?;
        let ConnectionDrive::Packet(packet_id, payload) = drive else {
            return Ok(());
        };
        tracing::debug!(?state, packet_id, len = payload.len(), "clientbound packet");
        emit_best_effort(
            &events,
            NetEvent::PacketSeen {
                state,
                packet_id,
                len: payload.len(),
            },
        )?;

        match state {
            ConnectionState::Login => match packets::decode_login_clientbound(packet_id, &payload)?
            {
                LoginClientbound::Disconnect { raw_json } => {
                    bail!("login disconnected: {raw_json}")
                }
                LoginClientbound::EncryptionRequest => {
                    bail!("server requested encryption; offline-mode event stream cannot continue")
                }
                LoginClientbound::SetCompression { threshold } => {
                    conn.compression_threshold = Some(threshold);
                    emit(&events, NetEvent::CompressionSet { threshold }).await?;
                }
                LoginClientbound::CustomQuery { transaction_id } => {
                    let mut response = bbb_protocol::codec::Encoder::new();
                    response.write_var_i32(transaction_id);
                    response.write_bool(false);
                    conn.send_packet(
                        ids::login::SERVERBOUND_CUSTOM_QUERY_ANSWER,
                        &response.into_inner(),
                    )
                    .await?;
                }
                LoginClientbound::LoginFinished { .. } => {
                    let (id, payload) = packets::encode_login_acknowledged();
                    conn.send_packet(id, &payload).await?;
                    state = ConnectionState::Configuration;
                    emit(&events, NetEvent::StateChanged { state }).await?;

                    let (id, payload) = packets::encode_client_information_default();
                    conn.send_packet(id, &payload).await?;
                }
            },
            ConnectionState::Configuration => {
                match packets::decode_configuration_clientbound(packet_id, &payload)? {
                    ConfigurationClientbound::Finish => {
                        let (id, payload) = packets::encode_configuration_finish();
                        conn.send_packet(id, &payload).await?;
                        state = ConnectionState::Play;
                        emit(&events, NetEvent::StateChanged { state }).await?;
                        let (id, payload) = packets::encode_play_client_information_default();
                        conn.send_packet(id, &payload).await?;
                        play_tick = Some(play_tick_interval());
                    }
                    ConfigurationClientbound::KeepAlive { id } => {
                        let (id, payload) = packets::encode_configuration_keep_alive(id);
                        conn.send_packet(id, &payload).await?;
                    }
                    ConfigurationClientbound::Ping { id } => {
                        let (id, payload) = packets::encode_configuration_pong(id);
                        conn.send_packet(id, &payload).await?;
                    }
                    ConfigurationClientbound::RegistryData {
                        registry,
                        raw_payload_len,
                    } => {
                        emit(
                            &events,
                            NetEvent::RegistryData {
                                registry,
                                raw_payload_len,
                            },
                        )
                        .await?;
                    }
                    ConfigurationClientbound::SelectKnownPacks { .. } => {
                        let (id, payload) = packets::encode_select_known_packs_empty();
                        conn.send_packet(id, &payload).await?;
                    }
                    ConfigurationClientbound::Unknown { .. } => {}
                }
            }
            ConnectionState::Play => match packets::decode_play_clientbound(packet_id, &payload)? {
                PlayClientbound::AddEntity(entity) => {
                    emit(&events, NetEvent::AddEntity(entity)).await?;
                }
                PlayClientbound::MoveEntity(update) => {
                    emit(&events, NetEvent::MoveEntity(update)).await?;
                }
                PlayClientbound::KeepAlive { id } => {
                    let (id, payload) = packets::encode_play_keep_alive(id);
                    conn.send_packet(id, &payload).await?;
                }
                PlayClientbound::ChunkBatchStart => {}
                PlayClientbound::ChunkBatchFinished { .. } => {
                    let (id, payload) = packets::encode_play_chunk_batch_received(9.0);
                    conn.send_packet(id, &payload).await?;
                }
                PlayClientbound::ContainerClose(update) => {
                    emit(&events, NetEvent::ContainerClose(update)).await?;
                }
                PlayClientbound::ContainerSetContent(update) => {
                    emit(&events, NetEvent::ContainerSetContent(update)).await?;
                }
                PlayClientbound::ContainerSetData(update) => {
                    emit(&events, NetEvent::ContainerSetData(update)).await?;
                }
                PlayClientbound::ContainerSetSlot(update) => {
                    emit(&events, NetEvent::ContainerSetSlot(update)).await?;
                }
                PlayClientbound::Disconnect(disconnect) => {
                    bail!("play disconnected: {}", disconnect.reason)
                }
                PlayClientbound::Ping { id } => {
                    let (id, payload) = packets::encode_play_pong(id);
                    conn.send_packet(id, &payload).await?;
                }
                PlayClientbound::StartConfiguration => {
                    let (id, payload) = packets::encode_play_configuration_acknowledged();
                    conn.send_packet(id, &payload).await?;
                    state = ConnectionState::Configuration;
                    play_tick = None;
                    emit(&events, NetEvent::StateChanged { state }).await?;
                }
                PlayClientbound::Login(login) => {
                    emit(&events, NetEvent::Login(login)).await?;
                }
                PlayClientbound::Respawn(respawn) => {
                    player_was_dead = false;
                    emit(&events, NetEvent::Respawn(respawn)).await?;
                }
                PlayClientbound::SetHealth(health) => {
                    maybe_send_perform_respawn(&mut conn, health, &mut player_was_dead).await?;
                    emit(&events, NetEvent::PlayerHealth(health)).await?;
                }
                PlayClientbound::SetExperience(experience) => {
                    emit(&events, NetEvent::PlayerExperience(experience)).await?;
                }
                PlayClientbound::SetHeldSlot(slot) => {
                    emit(&events, NetEvent::HeldSlot(slot)).await?;
                }
                PlayClientbound::SetCursorItem(update) => {
                    emit(&events, NetEvent::SetCursorItem(update)).await?;
                }
                PlayClientbound::SetPlayerInventory(update) => {
                    emit(&events, NetEvent::SetPlayerInventory(update)).await?;
                }
                PlayClientbound::GameEvent(event) => {
                    emit(&events, NetEvent::GameEvent(event)).await?;
                }
                PlayClientbound::SetTime(time) => {
                    emit(&events, NetEvent::SetTime(time)).await?;
                }
                PlayClientbound::BlockChangedAck(ack) => {
                    emit(&events, NetEvent::BlockChangedAck(ack)).await?;
                }
                PlayClientbound::PlayerPosition(update) => {
                    player_position_state = update.apply_to_state(player_position_state);
                    emit(&events, NetEvent::PlayerPosition(update)).await?;
                    let (id, payload) = packets::encode_play_accept_teleportation(update.id);
                    conn.send_packet(id, &payload).await?;
                    let (id, payload) = packets::encode_play_move_player_pos_rot(
                        player_position_state.position.x,
                        player_position_state.position.y,
                        player_position_state.position.z,
                        player_position_state.y_rot,
                        player_position_state.x_rot,
                        false,
                        false,
                    );
                    conn.send_packet(id, &payload).await?;
                    if !player_loaded_sent {
                        let (id, payload) = packets::encode_play_player_loaded();
                        conn.send_packet(id, &payload).await?;
                        player_loaded_sent = true;
                    }
                }
                PlayClientbound::EntityPositionSync(update) => {
                    emit(&events, NetEvent::EntityPositionSync(update)).await?;
                }
                PlayClientbound::RemoveEntities(update) => {
                    emit(&events, NetEvent::RemoveEntities(update)).await?;
                }
                PlayClientbound::RotateHead(update) => {
                    emit(&events, NetEvent::RotateHead(update)).await?;
                }
                PlayClientbound::SetEntityMotion(update) => {
                    emit(&events, NetEvent::SetEntityMotion(update)).await?;
                }
                PlayClientbound::SetEquipment(update) => {
                    emit(&events, NetEvent::SetEquipment(update)).await?;
                }
                PlayClientbound::SetEntityData(update) => {
                    emit(&events, NetEvent::SetEntityData(update)).await?;
                }
                PlayClientbound::TeleportEntity(update) => {
                    emit(&events, NetEvent::TeleportEntity(update)).await?;
                }
                PlayClientbound::PlayerAbilities(abilities) => {
                    emit(&events, NetEvent::PlayerAbilities(abilities)).await?;
                }
                PlayClientbound::SetDefaultSpawnPosition(spawn) => {
                    emit(&events, NetEvent::SetDefaultSpawnPosition(spawn)).await?;
                }
                PlayClientbound::SetSimulationDistance(distance) => {
                    emit(&events, NetEvent::SetSimulationDistance(distance)).await?;
                }
                PlayClientbound::SystemChat(chat) => {
                    emit(&events, NetEvent::SystemChat(chat)).await?;
                }
                PlayClientbound::LevelChunkWithLight(chunk) => {
                    emit(&events, NetEvent::LevelChunkWithLight(chunk)).await?;
                }
                PlayClientbound::LightUpdate(update) => {
                    emit(&events, NetEvent::LightUpdate(update)).await?;
                }
                PlayClientbound::ChunksBiomes(update) => {
                    emit(&events, NetEvent::ChunksBiomes(update)).await?;
                }
                PlayClientbound::ForgetLevelChunk(update) => {
                    emit(&events, NetEvent::ForgetLevelChunk(update)).await?;
                }
                PlayClientbound::BlockUpdate(update) => {
                    emit(&events, NetEvent::BlockUpdate(update)).await?;
                }
                PlayClientbound::SectionBlocksUpdate(update) => {
                    emit(&events, NetEvent::SectionBlocksUpdate(update)).await?;
                }
                PlayClientbound::SetChunkCacheCenter(update) => {
                    emit(&events, NetEvent::SetChunkCacheCenter(update)).await?;
                }
                PlayClientbound::SetChunkCacheRadius(update) => {
                    emit(&events, NetEvent::SetChunkCacheRadius(update)).await?;
                }
                PlayClientbound::Unknown { .. } => {}
            },
            ConnectionState::Handshake | ConnectionState::Status => {
                unreachable!("event stream starts at login")
            }
        }
    }
}

async fn run_offline_probe_inner(options: ConnectionOptions) -> Result<ProbeReport> {
    let mut conn = RawConnection::connect(&options.address, None).await?;
    let mut state = ConnectionState::Login;
    let mut packets_seen = 0usize;
    let mut world = WorldStore::new();
    let mut player_loaded_sent = false;
    let mut player_position_state = PlayerPositionState::default();
    let mut player_was_dead = false;
    let mut play_tick = None;

    let (id, payload) = packets::encode_handshake(&options.host, options.port, ClientIntent::Login);
    conn.send_packet(id, &payload).await?;
    let (id, payload) = packets::encode_login_hello(&options.username, options.profile_id);
    conn.send_packet(id, &payload).await?;

    let first_chunk = loop {
        let (packet_id, payload) =
            read_packet_or_send_play_tick(&mut conn, state, &mut play_tick).await?;
        packets_seen += 1;
        tracing::debug!(?state, packet_id, len = payload.len(), "clientbound packet");

        match state {
            ConnectionState::Login => match packets::decode_login_clientbound(packet_id, &payload)?
            {
                LoginClientbound::Disconnect { raw_json } => {
                    bail!("login disconnected: {raw_json}")
                }
                LoginClientbound::EncryptionRequest => {
                    bail!("server requested encryption; offline-mode probe cannot continue")
                }
                LoginClientbound::SetCompression { threshold } => {
                    conn.compression_threshold = Some(threshold);
                }
                LoginClientbound::CustomQuery { transaction_id } => {
                    let mut response = bbb_protocol::codec::Encoder::new();
                    response.write_var_i32(transaction_id);
                    response.write_bool(false);
                    conn.send_packet(
                        ids::login::SERVERBOUND_CUSTOM_QUERY_ANSWER,
                        &response.into_inner(),
                    )
                    .await?;
                }
                LoginClientbound::LoginFinished { .. } => {
                    let (id, payload) = packets::encode_login_acknowledged();
                    conn.send_packet(id, &payload).await?;
                    state = ConnectionState::Configuration;

                    let (id, payload) = packets::encode_client_information_default();
                    conn.send_packet(id, &payload).await?;
                }
            },
            ConnectionState::Configuration => {
                match packets::decode_configuration_clientbound(packet_id, &payload)? {
                    ConfigurationClientbound::Finish => {
                        let (id, payload) = packets::encode_configuration_finish();
                        conn.send_packet(id, &payload).await?;
                        state = ConnectionState::Play;
                        let (id, payload) = packets::encode_play_client_information_default();
                        conn.send_packet(id, &payload).await?;
                        play_tick = Some(play_tick_interval());
                    }
                    ConfigurationClientbound::KeepAlive { id } => {
                        let (id, payload) = packets::encode_configuration_keep_alive(id);
                        conn.send_packet(id, &payload).await?;
                    }
                    ConfigurationClientbound::Ping { id } => {
                        let (id, payload) = packets::encode_configuration_pong(id);
                        conn.send_packet(id, &payload).await?;
                    }
                    ConfigurationClientbound::RegistryData {
                        registry,
                        raw_payload_len,
                    } => {
                        world.record_registry(registry, raw_payload_len);
                    }
                    ConfigurationClientbound::SelectKnownPacks { .. } => {
                        let (id, payload) = packets::encode_select_known_packs_empty();
                        conn.send_packet(id, &payload).await?;
                    }
                    ConfigurationClientbound::Unknown { .. } => {}
                }
            }
            ConnectionState::Play => match packets::decode_play_clientbound(packet_id, &payload)? {
                PlayClientbound::AddEntity(entity) => {
                    world.apply_add_entity(entity);
                }
                PlayClientbound::MoveEntity(update) => {
                    world.apply_entity_move(update);
                }
                PlayClientbound::KeepAlive { id } => {
                    let (id, payload) = packets::encode_play_keep_alive(id);
                    conn.send_packet(id, &payload).await?;
                }
                PlayClientbound::ChunkBatchStart => {}
                PlayClientbound::ChunkBatchFinished { .. } => {
                    let (id, payload) = packets::encode_play_chunk_batch_received(9.0);
                    conn.send_packet(id, &payload).await?;
                }
                PlayClientbound::ContainerClose(update) => {
                    world.apply_container_close(update);
                }
                PlayClientbound::ContainerSetContent(update) => {
                    world.apply_container_set_content(update);
                }
                PlayClientbound::ContainerSetData(update) => {
                    world.apply_container_set_data(update);
                }
                PlayClientbound::ContainerSetSlot(update) => {
                    world.apply_container_set_slot(update);
                }
                PlayClientbound::Disconnect(disconnect) => {
                    bail!("play disconnected: {}", disconnect.reason)
                }
                PlayClientbound::Ping { id } => {
                    let (id, payload) = packets::encode_play_pong(id);
                    conn.send_packet(id, &payload).await?;
                }
                PlayClientbound::StartConfiguration => {
                    let (id, payload) = packets::encode_play_configuration_acknowledged();
                    conn.send_packet(id, &payload).await?;
                    state = ConnectionState::Configuration;
                    play_tick = None;
                }
                PlayClientbound::Login(login) => {
                    world.apply_login(&login);
                }
                PlayClientbound::Respawn(respawn) => {
                    player_was_dead = false;
                    world.apply_respawn(&respawn);
                }
                PlayClientbound::SetHealth(health) => {
                    maybe_send_perform_respawn(&mut conn, health, &mut player_was_dead).await?;
                }
                PlayClientbound::EntityPositionSync(update) => {
                    world.apply_entity_position_sync(update);
                }
                PlayClientbound::RemoveEntities(update) => {
                    world.apply_remove_entities(update);
                }
                PlayClientbound::RotateHead(update) => {
                    world.apply_rotate_head(update);
                }
                PlayClientbound::SetEntityMotion(update) => {
                    world.apply_set_entity_motion(update);
                }
                PlayClientbound::SetEquipment(update) => {
                    world.apply_set_equipment(update);
                }
                PlayClientbound::SetEntityData(update) => {
                    world.apply_set_entity_data(update);
                }
                PlayClientbound::TeleportEntity(update) => {
                    world.apply_teleport_entity(update);
                }
                PlayClientbound::PlayerAbilities(_) => {}
                PlayClientbound::SetExperience(_) => {}
                PlayClientbound::SetHeldSlot(_) => {}
                PlayClientbound::SetCursorItem(update) => {
                    world.apply_set_cursor_item(update);
                }
                PlayClientbound::SetPlayerInventory(update) => {
                    world.apply_set_player_inventory(update);
                }
                PlayClientbound::SetDefaultSpawnPosition(_) => {}
                PlayClientbound::SetSimulationDistance(_) => {}
                PlayClientbound::SystemChat(_) => {}
                PlayClientbound::BlockChangedAck(_) => {}
                PlayClientbound::GameEvent(_) | PlayClientbound::SetTime(_) => {}
                PlayClientbound::PlayerPosition(update) => {
                    player_position_state = update.apply_to_state(player_position_state);
                    let (id, payload) = packets::encode_play_accept_teleportation(update.id);
                    conn.send_packet(id, &payload).await?;
                    let (id, payload) = packets::encode_play_move_player_pos_rot(
                        player_position_state.position.x,
                        player_position_state.position.y,
                        player_position_state.position.z,
                        player_position_state.y_rot,
                        player_position_state.x_rot,
                        false,
                        false,
                    );
                    conn.send_packet(id, &payload).await?;
                    if !player_loaded_sent {
                        let (id, payload) = packets::encode_play_player_loaded();
                        conn.send_packet(id, &payload).await?;
                        player_loaded_sent = true;
                    }
                }
                PlayClientbound::LevelChunkWithLight(chunk) => {
                    let pos = world.insert_level_chunk_with_light(chunk)?;
                    break pos;
                }
                PlayClientbound::LightUpdate(update) => {
                    world.apply_light_update(update)?;
                }
                PlayClientbound::ChunksBiomes(update) => {
                    world.apply_biome_update(update)?;
                }
                PlayClientbound::ForgetLevelChunk(update) => {
                    world.forget_chunk(ChunkPos {
                        x: update.pos.x,
                        z: update.pos.z,
                    });
                }
                PlayClientbound::BlockUpdate(update) => {
                    world.apply_block_update(update);
                }
                PlayClientbound::SectionBlocksUpdate(update) => {
                    world.apply_section_blocks_update(update);
                }
                PlayClientbound::SetChunkCacheCenter(_)
                | PlayClientbound::SetChunkCacheRadius(_) => {}
                PlayClientbound::Unknown { .. } => {}
            },
            ConnectionState::Handshake | ConnectionState::Status => {
                unreachable!("probe starts at login")
            }
        }
    };

    let first_chunk_summary = world
        .probe_chunk(first_chunk)
        .map(ChunkProbeSummary::from_column);
    let first_chunk_center_block = world.probe_block(BlockPos {
        x: first_chunk.x * 16 + 8,
        y: 64,
        z: first_chunk.z * 16 + 8,
    });
    let world_counters = world.counters();

    Ok(ProbeReport {
        reached_state: state,
        compression_threshold: conn.compression_threshold,
        packets_seen,
        registries_seen: world_counters.registries_seen,
        first_chunk: Some(first_chunk),
        first_chunk_summary,
        first_chunk_center_block,
        world_counters,
        world,
    })
}

async fn read_packet_or_send_play_tick(
    conn: &mut RawConnection,
    state: ConnectionState,
    play_tick: &mut Option<Interval>,
) -> Result<(i32, Vec<u8>)> {
    if !matches!(state, ConnectionState::Play) {
        return conn.read_packet().await;
    }

    let Some(tick) = play_tick.as_mut() else {
        return conn.read_packet().await;
    };

    loop {
        tokio::select! {
            packet = conn.read_packet() => return packet,
            _ = tick.tick() => {
                let (id, payload) = packets::encode_play_client_tick_end();
                conn.send_packet(id, &payload).await?;
            }
        }
    }
}

#[derive(Debug)]
enum ConnectionDrive {
    Packet(i32, Vec<u8>),
    Disconnect,
}

async fn read_packet_or_drive_connection(
    conn: &mut RawConnection,
    state: ConnectionState,
    play_tick: &mut Option<Interval>,
    commands: &mut mpsc::Receiver<NetCommand>,
    player_position_state: &mut PlayerPositionState,
) -> Result<ConnectionDrive> {
    if !matches!(state, ConnectionState::Play) || play_tick.is_none() {
        return read_packet_or_disconnect_command(conn, commands).await;
    }
    let tick = play_tick.as_mut().expect("play tick checked above");

    loop {
        tokio::select! {
            packet = conn.read_packet() => {
                let (packet_id, payload) = packet?;
                return Ok(ConnectionDrive::Packet(packet_id, payload));
            }
            _ = tick.tick() => {
                let (id, payload) = packets::encode_play_client_tick_end();
                conn.send_packet(id, &payload).await?;
            }
            command = commands.recv() => {
                match command {
                    Some(NetCommand::MovePlayer(command)) => {
                        send_player_move_command(conn, command, player_position_state).await?;
                    }
                    Some(NetCommand::PlayerAction(action)) => {
                        send_player_action(conn, action).await?;
                    }
                    Some(NetCommand::PlayerCommand(command)) => {
                        send_player_command(conn, command).await?;
                    }
                    Some(NetCommand::PlayerInput(input)) => {
                        send_player_input_command(conn, input).await?;
                    }
                    Some(NetCommand::SetHeldSlot(slot)) => {
                        send_set_held_slot_command(conn, slot).await?;
                    }
                    Some(NetCommand::Swing(hand)) => {
                        send_swing_command(conn, hand).await?;
                    }
                    Some(NetCommand::UseItemOn(packet)) => {
                        send_use_item_on(conn, packet).await?;
                    }
                    Some(NetCommand::UseItem(packet)) => {
                        send_use_item(conn, packet).await?;
                    }
                    Some(NetCommand::PickItemFromBlock(packet)) => {
                        send_pick_item_from_block(conn, packet).await?;
                    }
                    Some(NetCommand::Disconnect) | None => {
                        return Ok(ConnectionDrive::Disconnect);
                    }
                }
            }
        }
    }
}

async fn read_packet_or_disconnect_command(
    conn: &mut RawConnection,
    commands: &mut mpsc::Receiver<NetCommand>,
) -> Result<ConnectionDrive> {
    loop {
        tokio::select! {
            packet = conn.read_packet() => {
                let (packet_id, payload) = packet?;
                return Ok(ConnectionDrive::Packet(packet_id, payload));
            }
            command = commands.recv() => {
                match command {
                    Some(NetCommand::MovePlayer(_)) => {}
                    Some(NetCommand::PlayerAction(_)) => {}
                    Some(NetCommand::PlayerCommand(_)) => {}
                    Some(NetCommand::PlayerInput(_)) => {}
                    Some(NetCommand::SetHeldSlot(_)) => {}
                    Some(NetCommand::Swing(_)) => {}
                    Some(NetCommand::UseItemOn(_)) => {}
                    Some(NetCommand::UseItem(_)) => {}
                    Some(NetCommand::PickItemFromBlock(_)) => {}
                    Some(NetCommand::Disconnect) | None => {
                        return Ok(ConnectionDrive::Disconnect);
                    }
                }
            }
        }
    }
}

async fn send_player_move_command(
    conn: &mut RawConnection,
    command: PlayerMoveCommand,
    player_position_state: &mut PlayerPositionState,
) -> Result<()> {
    let (id, payload) = command.encode_packet();
    conn.send_packet(id, &payload).await?;
    *player_position_state = command.state;
    Ok(())
}

async fn send_player_action(conn: &mut RawConnection, action: PlayerAction) -> Result<()> {
    let (id, payload) = packets::encode_play_player_action(action);
    conn.send_packet(id, &payload).await
}

async fn send_player_command(conn: &mut RawConnection, command: PlayerCommand) -> Result<()> {
    let (id, payload) = packets::encode_play_player_command(command);
    conn.send_packet(id, &payload).await
}

async fn send_player_input_command(conn: &mut RawConnection, input: PlayerInput) -> Result<()> {
    let (id, payload) = packets::encode_play_player_input(input);
    conn.send_packet(id, &payload).await
}

async fn send_set_held_slot_command(conn: &mut RawConnection, slot: u8) -> Result<()> {
    let (id, payload) = packets::encode_play_set_carried_item(i16::from(slot.min(8)));
    conn.send_packet(id, &payload).await
}

async fn send_swing_command(conn: &mut RawConnection, hand: InteractionHand) -> Result<()> {
    let (id, payload) = packets::encode_play_swing(hand);
    conn.send_packet(id, &payload).await
}

async fn send_use_item_on(conn: &mut RawConnection, packet: UseItemOn) -> Result<()> {
    let (id, payload) = packets::encode_play_use_item_on(packet);
    conn.send_packet(id, &payload).await
}

async fn send_use_item(conn: &mut RawConnection, packet: UseItem) -> Result<()> {
    let (id, payload) = packets::encode_play_use_item(packet);
    conn.send_packet(id, &payload).await
}

async fn send_pick_item_from_block(
    conn: &mut RawConnection,
    packet: PickItemFromBlock,
) -> Result<()> {
    let (id, payload) = packets::encode_play_pick_item_from_block(packet);
    conn.send_packet(id, &payload).await
}

async fn maybe_send_perform_respawn(
    conn: &mut RawConnection,
    health: PlayerHealth,
    player_was_dead: &mut bool,
) -> Result<()> {
    let is_dead = health.health <= 0.0;
    if is_dead && !*player_was_dead {
        let (id, payload) = packets::encode_play_perform_respawn();
        conn.send_packet(id, &payload).await?;
    }
    *player_was_dead = is_dead;
    Ok(())
}

fn play_tick_interval() -> Interval {
    let mut tick = interval(Duration::from_millis(50));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    tick
}

struct RawConnection {
    stream: TcpStream,
    read_buf: BytesMut,
    compression_threshold: Option<i32>,
}

impl RawConnection {
    async fn connect(address: &str, compression_threshold: Option<i32>) -> Result<Self> {
        let stream = TcpStream::connect(address)
            .await
            .with_context(|| format!("connect {address}"))?;
        stream.set_nodelay(true).ok();
        Ok(Self {
            stream,
            read_buf: BytesMut::with_capacity(8192),
            compression_threshold,
        })
    }

    async fn send_packet(&mut self, packet_id: i32, payload: &[u8]) -> Result<()> {
        let packet =
            encode_packet_with_compression(packet_id, payload, self.compression_threshold)?;
        self.stream.write_all(&packet).await?;
        Ok(())
    }

    async fn read_packet(&mut self) -> Result<(i32, Vec<u8>)> {
        loop {
            if let Some(frame) = try_read_frame(&mut self.read_buf)? {
                let body = decode_packet_body(&frame, self.compression_threshold)?;
                let (packet_id, payload) = split_packet(&body)?;
                return Ok((packet_id, payload.to_vec()));
            }

            let mut temp = [0u8; 4096];
            let read = self.stream.read(&mut temp).await?;
            if read == 0 {
                return Err(anyhow!("connection closed"));
            }
            self.read_buf.extend_from_slice(&temp[..read]);
        }
    }
}

fn split_host_port(address: &str) -> Result<(String, u16)> {
    if let Some((host, port)) = address.rsplit_once(':') {
        let port = port.parse::<u16>()?;
        return Ok((host.to_string(), port));
    }

    let mut addrs = (address, 25565).to_socket_addrs()?;
    let first = addrs
        .next()
        .ok_or_else(|| anyhow!("could not resolve {address}:25565"))?;
    Ok((address.to_string(), first.port()))
}

async fn emit(events: &mpsc::Sender<NetEvent>, event: NetEvent) -> Result<()> {
    events
        .send(event)
        .await
        .map_err(|_| anyhow!("net event receiver dropped"))
}

fn emit_best_effort(events: &mpsc::Sender<NetEvent>, event: NetEvent) -> Result<()> {
    if events.capacity() <= 1024 {
        return Ok(());
    }

    match events.try_send(event) {
        Ok(()) | Err(mpsc::error::TrySendError::Full(_)) => Ok(()),
        Err(mpsc::error::TrySendError::Closed(_)) => Err(anyhow!("net event receiver dropped")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{codec::Decoder, ids, packets::Vec3d};

    #[tokio::test]
    async fn drive_connection_disconnects_when_command_channel_closes_before_play() {
        let (mut conn, server) = raw_connection_pair().await;
        let (tx, mut commands) = mpsc::channel(1);
        drop(tx);
        let mut play_tick = None;
        let mut player_position_state = PlayerPositionState::default();

        let result = timeout(
            Duration::from_secs(1),
            read_packet_or_drive_connection(
                &mut conn,
                ConnectionState::Login,
                &mut play_tick,
                &mut commands,
                &mut player_position_state,
            ),
        )
        .await
        .expect("drive should not hang")
        .unwrap();

        assert!(matches!(result, ConnectionDrive::Disconnect));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn drive_connection_honors_disconnect_command() {
        let (mut conn, server) = raw_connection_pair().await;
        let (tx, mut commands) = mpsc::channel(1);
        tx.send(NetCommand::Disconnect).await.unwrap();
        let mut play_tick = None;
        let mut player_position_state = PlayerPositionState::default();

        let result = timeout(
            Duration::from_secs(1),
            read_packet_or_drive_connection(
                &mut conn,
                ConnectionState::Configuration,
                &mut play_tick,
                &mut commands,
                &mut player_position_state,
            ),
        )
        .await
        .expect("drive should not hang")
        .unwrap();

        assert!(matches!(result, ConnectionDrive::Disconnect));
        server.await.unwrap();
    }

    #[test]
    fn player_move_command_encodes_pos_rot_packet() {
        let command = PlayerMoveCommand {
            state: PlayerPositionState {
                position: Vec3d {
                    x: 1.25,
                    y: 64.5,
                    z: -8.75,
                },
                delta_movement: Vec3d {
                    x: 0.1,
                    y: 0.0,
                    z: -0.2,
                },
                y_rot: 90.0,
                x_rot: -15.0,
            },
            on_ground: true,
            horizontal_collision: true,
        };

        let (packet_id, payload) = command.encode_packet();

        assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_POS_ROT);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_f64().unwrap(), 1.25);
        assert_eq!(decoder.read_f64().unwrap(), 64.5);
        assert_eq!(decoder.read_f64().unwrap(), -8.75);
        assert_eq!(decoder.read_f32().unwrap(), 90.0);
        assert_eq!(decoder.read_f32().unwrap(), -15.0);
        assert_eq!(decoder.read_u8().unwrap(), 0b11);
        assert!(decoder.is_empty());
    }

    #[tokio::test]
    async fn send_player_action_encodes_player_action_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("player action should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_PLAYER_ACTION);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), 0);
            let pos = decoder.read_i64().unwrap();
            assert_eq!(
                bbb_protocol::packets::BlockPos {
                    x: (pos >> 38) as i32,
                    y: ((pos << 52) >> 52) as i32,
                    z: ((pos << 26) >> 38) as i32,
                },
                bbb_protocol::packets::BlockPos { x: 1, y: 64, z: -2 }
            );
            assert_eq!(decoder.read_u8().unwrap(), 4);
            assert_eq!(decoder.read_var_i32().unwrap(), 9);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_player_action(
            &mut conn,
            PlayerAction {
                action: bbb_protocol::packets::PlayerActionKind::StartDestroyBlock,
                pos: bbb_protocol::packets::BlockPos { x: 1, y: 64, z: -2 },
                direction: bbb_protocol::packets::Direction::West,
                sequence: 9,
            },
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_player_command_encodes_player_command_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("player command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_PLAYER_COMMAND);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), 123);
            assert_eq!(decoder.read_var_i32().unwrap(), 2);
            assert_eq!(decoder.read_var_i32().unwrap(), 0);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_player_command(
            &mut conn,
            PlayerCommand {
                entity_id: 123,
                action: bbb_protocol::packets::PlayerCommandAction::StopSprinting,
                data: 0,
            },
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_player_input_command_encodes_input_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("player input command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_PLAYER_INPUT);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_u8().unwrap(), 0b0111_0001);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_player_input_command(
            &mut conn,
            PlayerInput {
                forward: true,
                backward: false,
                left: false,
                right: false,
                jump: true,
                shift: true,
                sprint: true,
            },
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_set_held_slot_command_encodes_carried_item_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("held-slot command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_SET_CARRIED_ITEM);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_i16().unwrap(), 8);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_set_held_slot_command(&mut conn, 12).await.unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_swing_command_encodes_swing_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("swing command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_SWING);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), 0);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_swing_command(&mut conn, InteractionHand::MainHand)
            .await
            .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_use_item_on_encodes_use_item_on_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("use item on command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_USE_ITEM_ON);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), 0);
            let pos = decoder.read_i64().unwrap();
            assert_eq!(
                bbb_protocol::packets::BlockPos {
                    x: (pos >> 38) as i32,
                    y: ((pos << 52) >> 52) as i32,
                    z: ((pos << 26) >> 38) as i32,
                },
                bbb_protocol::packets::BlockPos {
                    x: -5,
                    y: 70,
                    z: 12
                }
            );
            assert_eq!(decoder.read_var_i32().unwrap(), 3);
            assert_eq!(decoder.read_f32().unwrap(), 0.25);
            assert_eq!(decoder.read_f32().unwrap(), 0.5);
            assert_eq!(decoder.read_f32().unwrap(), 0.75);
            assert!(!decoder.read_bool().unwrap());
            assert!(!decoder.read_bool().unwrap());
            assert_eq!(decoder.read_var_i32().unwrap(), 4);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_use_item_on(
            &mut conn,
            bbb_protocol::packets::UseItemOn {
                hand: InteractionHand::MainHand,
                hit: bbb_protocol::packets::BlockHitResult {
                    pos: bbb_protocol::packets::BlockPos {
                        x: -5,
                        y: 70,
                        z: 12,
                    },
                    direction: bbb_protocol::packets::Direction::South,
                    cursor_x: 0.25,
                    cursor_y: 0.5,
                    cursor_z: 0.75,
                    inside: false,
                    world_border_hit: false,
                },
                sequence: 4,
            },
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_use_item_encodes_use_item_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("use item command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_USE_ITEM);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), 1);
            assert_eq!(decoder.read_var_i32().unwrap(), 8);
            assert_eq!(decoder.read_f32().unwrap(), 45.0);
            assert_eq!(decoder.read_f32().unwrap(), -20.0);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_use_item(
            &mut conn,
            bbb_protocol::packets::UseItem {
                hand: InteractionHand::OffHand,
                sequence: 8,
                y_rot: 45.0,
                x_rot: -20.0,
            },
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_pick_item_from_block_encodes_pick_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("pick item from block command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_PICK_ITEM_FROM_BLOCK);
            let mut decoder = Decoder::new(&payload);
            let pos = decoder.read_i64().unwrap();
            assert_eq!(
                bbb_protocol::packets::BlockPos {
                    x: (pos >> 38) as i32,
                    y: ((pos << 52) >> 52) as i32,
                    z: ((pos << 26) >> 38) as i32,
                },
                bbb_protocol::packets::BlockPos {
                    x: -5,
                    y: 70,
                    z: 12
                }
            );
            assert!(decoder.read_bool().unwrap());
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_pick_item_from_block(
            &mut conn,
            bbb_protocol::packets::PickItemFromBlock {
                pos: bbb_protocol::packets::BlockPos {
                    x: -5,
                    y: 70,
                    z: 12,
                },
                include_data: true,
            },
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn death_health_sends_respawn_command_once_until_alive() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("respawn command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_CLIENT_COMMAND);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), 0);
            assert!(decoder.is_empty());

            assert!(
                timeout(Duration::from_millis(100), conn.read_packet())
                    .await
                    .is_err(),
                "second dead health packet must not send another respawn"
            );
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();
        let mut player_was_dead = false;

        maybe_send_perform_respawn(
            &mut conn,
            PlayerHealth {
                health: 20.0,
                food: 20,
                saturation: 5.0,
            },
            &mut player_was_dead,
        )
        .await
        .unwrap();
        assert!(!player_was_dead);

        maybe_send_perform_respawn(
            &mut conn,
            PlayerHealth {
                health: 0.0,
                food: 18,
                saturation: 0.0,
            },
            &mut player_was_dead,
        )
        .await
        .unwrap();
        assert!(player_was_dead);

        maybe_send_perform_respawn(
            &mut conn,
            PlayerHealth {
                health: 0.0,
                food: 18,
                saturation: 0.0,
            },
            &mut player_was_dead,
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    async fn raw_connection_pair() -> (RawConnection, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_millis(50)).await;
        });
        let conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();
        (conn, server)
    }
}

#[allow(dead_code)]
fn _keep_encode_packet_reachable(packet_id: i32, payload: &[u8]) -> Vec<u8> {
    encode_packet(packet_id, payload)
}
