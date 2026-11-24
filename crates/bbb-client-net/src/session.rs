use std::{collections::BTreeMap, error::Error, fmt, net::Ipv6Addr, sync::Arc};

use anyhow::{bail, ensure, Context, Result};
use bbb_protocol::{
    code_of_conduct_text_hash,
    content_registry::{ContentRegistry, BBB_CONTENT_REGISTRY_CHANNEL},
    packets::{
        self, ClientIntent, ConfigurationClientbound, CustomPayload, CustomPayloadBody, KnownPack,
        LoginClientbound, PlayClientbound, PlayerPositionState, PlayerRotationUpdate, Transfer,
    },
    resource_pack_response_actions::ResourcePackResponseAction,
};
use uuid::Uuid;

use crate::{
    chunk_batch::ChunkBatchSizeCalculator,
    connection::RawConnection,
    login::ensure_offline_login,
    login::send_login_hello,
    resource_pack::response_action_for_push_with_policy,
    types::{
        AcceptTeleportCommand, ConnectionOptions, ConnectionState, DisconnectReason,
        ServerResourcePackPolicy,
    },
};

#[derive(Debug)]
pub struct RemoteDisconnectError {
    pub diagnostic: String,
    pub reason: DisconnectReason,
}

impl fmt::Display for RemoteDisconnectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.diagnostic)
    }
}

impl Error for RemoteDisconnectError {}

pub fn remote_disconnect_error(
    phase: &'static str,
    summary: String,
    styled: Vec<bbb_protocol::StyledTextRun>,
) -> anyhow::Error {
    RemoteDisconnectError {
        diagnostic: format!("{phase} disconnected: {summary}"),
        reason: DisconnectReason::from_styled(styled),
    }
    .into()
}

#[derive(Debug, Clone, PartialEq)]
// Keep PlayClientbound inline on the hot packet decode path.
#[allow(clippy::large_enum_variant)]
pub enum DecodedClientboundPacket {
    Login(LoginClientbound),
    Configuration(ConfigurationClientbound),
    Play(PlayClientbound),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginPacketStep {
    CompressionSet {
        threshold: i32,
    },
    CookieRequest {
        key: String,
        response_payload_present: bool,
    },
    EnteredConfiguration,
    Unknown {
        packet_id: i32,
        len: usize,
    },
    Handled,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigurationPacketStep {
    Finish,
    CustomPayload(CustomPayload),
    ResourcePackPush {
        update: packets::ResourcePackPush,
        action: Option<ResourcePackResponseAction>,
    },
    SelectKnownPacks {
        known_packs: Vec<KnownPack>,
    },
    CookieRequest {
        key: String,
        response_payload_present: bool,
    },
    StoreCookie {
        key: String,
        payload_len: usize,
        stored_cookie_count: usize,
    },
    Transfer(Transfer),
    CodeOfConduct {
        text: String,
    },
    Unknown {
        packet_id: i32,
        len: usize,
    },
    PassThrough(ConfigurationClientbound),
    Handled,
}

#[derive(Debug, Clone, PartialEq)]
// Keep PlayClientbound inline on the hot packet step path.
#[allow(clippy::large_enum_variant)]
pub enum PlayPacketStep {
    ChunkBatchStart,
    ChunkBatchFinished {
        batch_size: i32,
    },
    CookieRequest {
        key: String,
        response_payload_present: bool,
    },
    StoreCookie {
        key: String,
        payload_len: usize,
        stored_cookie_count: usize,
    },
    StartConfiguration,
    ResourcePackPush {
        update: packets::ResourcePackPush,
        action: Option<ResourcePackResponseAction>,
    },
    Transfer(Transfer),
    PlayerPosition(packets::PlayerPositionUpdate),
    PlayerRotation(PlayerRotationUpdate),
    SetHeldSlot(packets::SetHeldSlot),
    Unknown {
        packet_id: i32,
        len: usize,
    },
    PassThrough(PlayClientbound),
    Handled,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayerMovementFlags {
    pub on_ground: bool,
    pub horizontal_collision: bool,
}

const MAX_SERVER_COOKIES: usize = 256;
const MAX_SERVER_COOKIES_TOTAL_BYTES: usize = 256 * 1024;

/// Shared Login/Configuration/Play wire protocol session state machine used by
/// both the runtime event stream and the headless differential probe.
pub struct ClientProtocolSession {
    pub conn: RawConnection,
    pub state: ConnectionState,
    pub player_position_state: PlayerPositionState,
    pub player_movement_flags: PlayerMovementFlags,
    pub chunk_batch_size: ChunkBatchSizeCalculator,
    pub pending_teleports: u32,
    pub server_cookies: BTreeMap<String, Vec<u8>>,
    pub seen_code_of_conduct: bool,
    pub accepted_code_of_conduct_hash: Option<i32>,
    pub client_information: packets::ClientInformation,
    pub content_registry: Arc<ContentRegistry>,
    #[cfg(not(target_arch = "wasm32"))]
    pub play_tick: Option<tokio::time::Interval>,
}

impl ClientProtocolSession {
    pub fn new(conn: RawConnection) -> Self {
        Self {
            conn,
            state: ConnectionState::Login,
            player_position_state: PlayerPositionState::default(),
            player_movement_flags: PlayerMovementFlags::default(),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            pending_teleports: 0,
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        }
    }

    pub fn from_connection_options(conn: RawConnection, options: &ConnectionOptions) -> Self {
        let mut session = Self::new(conn);
        session.accepted_code_of_conduct_hash = options.accepted_code_of_conduct_hash;
        session.client_information = options.client_information.clone();
        session
    }

    pub fn content_registry(&self) -> &Arc<ContentRegistry> {
        &self.content_registry
    }

    pub fn reset_content_registry(&mut self) {
        self.content_registry = Arc::clone(ContentRegistry::vanilla());
    }

    pub async fn start_login(
        &mut self,
        host: &str,
        port: u16,
        intent: ClientIntent,
        username: &str,
        profile_id: Uuid,
    ) -> Result<()> {
        let (host, port) = self
            .conn
            .srv_redirect
            .as_ref()
            .map_or((host, port), |(host, port)| (host.as_str(), *port));
        let (id, payload) = packets::encode_handshake(host, port, intent)?;
        self.conn.send_packet(id, &payload).await?;
        send_login_hello(&mut self.conn, username, profile_id).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn read_packet_or_send_play_tick(&mut self) -> Result<(i32, Vec<u8>)> {
        crate::driver::read_packet_or_send_play_tick(
            &mut self.conn,
            self.state,
            &mut self.play_tick,
        )
        .await
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn read_packet_or_eof_or_send_play_tick(&mut self) -> Result<Option<(i32, Vec<u8>)>> {
        crate::driver::read_packet_or_eof_or_send_play_tick(
            &mut self.conn,
            self.state,
            &mut self.play_tick,
        )
        .await
    }

    pub fn decode_play_packet(&self, packet_id: i32, payload: &[u8]) -> Result<PlayClientbound> {
        packets::decode_play_clientbound_with_content_registry(
            packet_id,
            payload,
            &self.content_registry,
        )
        .map_err(Into::into)
    }

    pub fn decode_packet(
        &self,
        packet_id: i32,
        payload: &[u8],
    ) -> Result<DecodedClientboundPacket> {
        match self.state {
            ConnectionState::Login => Ok(DecodedClientboundPacket::Login(
                packets::decode_login_clientbound(packet_id, payload)?,
            )),
            ConnectionState::Configuration => Ok(DecodedClientboundPacket::Configuration(
                packets::decode_configuration_clientbound(packet_id, payload)?,
            )),
            ConnectionState::Play => Ok(DecodedClientboundPacket::Play(
                self.decode_play_packet(packet_id, payload)?,
            )),
            ConnectionState::Handshake | ConnectionState::Status => {
                bail!(
                    "cannot decode packet while connection state is {:?}",
                    self.state
                )
            }
        }
    }

    pub async fn step_login_packet(&mut self, packet: LoginClientbound) -> Result<LoginPacketStep> {
        ensure!(
            self.state == ConnectionState::Login,
            "received login packet while connection state is {:?}",
            self.state
        );
        match packet {
            LoginClientbound::Disconnect {
                reason,
                styled_reason,
                ..
            } => Err(remote_disconnect_error("login", reason, styled_reason)),
            LoginClientbound::EncryptionRequest(_) => {
                Err(ensure_offline_login(&self.conn).unwrap_err())
            }
            LoginClientbound::SetCompression { threshold } => {
                self.conn.set_compression_threshold(threshold);
                Ok(LoginPacketStep::CompressionSet { threshold })
            }
            LoginClientbound::CustomQuery { transaction_id } => {
                let (id, response) = packets::encode_login_custom_query_answer(transaction_id);
                self.conn.send_packet(id, &response).await?;
                Ok(LoginPacketStep::Handled)
            }
            LoginClientbound::CookieRequest(request) => {
                let payload = self.server_cookies.get(&request.key).map(Vec::as_slice);
                let payload_present = payload.is_some();
                let (id, response) = packets::encode_login_cookie_response(&request.key, payload)?;
                self.conn.send_packet(id, &response).await?;
                Ok(LoginPacketStep::CookieRequest {
                    key: request.key,
                    response_payload_present: payload_present,
                })
            }
            LoginClientbound::LoginFinished { .. } => {
                let (id, payload) = packets::encode_login_acknowledged();
                self.conn.send_packet(id, &payload).await?;
                self.state = ConnectionState::Configuration;
                self.seen_code_of_conduct = false;
                self.reset_content_registry();

                let (id, payload) =
                    packets::encode_configuration_brand_custom_payload("bbb-client")?;
                self.conn.send_packet(id, &payload).await?;

                let (id, payload) = packets::encode_client_information(&self.client_information)?;
                self.conn.send_packet(id, &payload).await?;
                Ok(LoginPacketStep::EnteredConfiguration)
            }
            LoginClientbound::Unknown { packet_id, len } => {
                Ok(LoginPacketStep::Unknown { packet_id, len })
            }
        }
    }

    pub async fn step_configuration_packet(
        &mut self,
        packet: ConfigurationClientbound,
        resource_pack_policy: ServerResourcePackPolicy,
    ) -> Result<ConfigurationPacketStep> {
        match packet {
            ConfigurationClientbound::Finish => Ok(ConfigurationPacketStep::Finish),
            ConfigurationClientbound::Disconnect {
                reason,
                styled_reason,
                ..
            } => Err(remote_disconnect_error(
                "configuration",
                reason,
                styled_reason,
            )),
            ConfigurationClientbound::CustomPayload(update) => {
                if update.id == BBB_CONTENT_REGISTRY_CHANNEL {
                    if let CustomPayloadBody::Unknown { raw_payload } = &update.payload {
                        let decoded = ContentRegistry::decode_wire(raw_payload)
                            .context("decode bbb:content_registry configuration payload")?;
                        self.content_registry = Arc::new(decoded);
                    }
                }
                Ok(ConfigurationPacketStep::CustomPayload(update))
            }
            ConfigurationClientbound::KeepAlive { id } => {
                let (id, payload) = packets::encode_configuration_keep_alive(id);
                self.conn.send_packet(id, &payload).await?;
                Ok(ConfigurationPacketStep::Handled)
            }
            ConfigurationClientbound::Ping { id } => {
                let (id, payload) = packets::encode_configuration_pong(id);
                self.conn.send_packet(id, &payload).await?;
                Ok(ConfigurationPacketStep::Handled)
            }
            ConfigurationClientbound::ResourcePackPush(update) => {
                let pack_id = update.id;
                let action = response_action_for_push_with_policy(&update, resource_pack_policy);
                if let Some(action) = action {
                    let (id, payload) =
                        packets::encode_configuration_resource_pack_response(pack_id, action);
                    self.conn.send_packet(id, &payload).await?;
                }
                Ok(ConfigurationPacketStep::ResourcePackPush { update, action })
            }
            ConfigurationClientbound::SelectKnownPacks { known_packs } => {
                Ok(ConfigurationPacketStep::SelectKnownPacks { known_packs })
            }
            ConfigurationClientbound::CookieRequest(request) => {
                let payload = self.server_cookies.get(&request.key).map(Vec::as_slice);
                let payload_present = payload.is_some();
                let (id, response) =
                    packets::encode_configuration_cookie_response(&request.key, payload)?;
                self.conn.send_packet(id, &response).await?;
                Ok(ConfigurationPacketStep::CookieRequest {
                    key: request.key,
                    response_payload_present: payload_present,
                })
            }
            ConfigurationClientbound::StoreCookie(cookie) => {
                let (key, payload_len, stored_cookie_count) =
                    self.store_cookie(cookie.key, cookie.payload);
                Ok(ConfigurationPacketStep::StoreCookie {
                    key,
                    payload_len,
                    stored_cookie_count,
                })
            }
            ConfigurationClientbound::Transfer(update) => {
                Ok(ConfigurationPacketStep::Transfer(update))
            }
            ConfigurationClientbound::CodeOfConduct { text } => {
                if self.seen_code_of_conduct {
                    bail!("server sent duplicate Code of Conduct");
                }
                self.seen_code_of_conduct = true;
                if self.accepted_code_of_conduct_hash == Some(code_of_conduct_text_hash(&text)) {
                    let (id, payload) = packets::encode_configuration_accept_code_of_conduct();
                    self.conn.send_packet(id, &payload).await?;
                }
                Ok(ConfigurationPacketStep::CodeOfConduct { text })
            }
            ConfigurationClientbound::Unknown { packet_id, len } => {
                Ok(ConfigurationPacketStep::Unknown { packet_id, len })
            }
            other => Ok(ConfigurationPacketStep::PassThrough(other)),
        }
    }

    pub async fn send_select_known_packs(&mut self, selected_packs: &[KnownPack]) -> Result<()> {
        let (id, payload) = packets::encode_select_known_packs(selected_packs)?;
        self.conn.send_packet(id, &payload).await
    }

    pub async fn finish_configuration(&mut self) -> Result<()> {
        if !matches!(self.state, ConnectionState::Configuration) {
            bail!(
                "cannot finish configuration while connection state is {:?}",
                self.state
            );
        }

        let (id, payload) = packets::encode_configuration_finish();
        self.conn.send_packet(id, &payload).await?;
        self.state = ConnectionState::Play;
        self.conn.enter_play_phase();
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn start_play_tick(&mut self) {
        self.play_tick = Some(crate::connection::play_tick_interval());
    }

    pub async fn respond_play_keep_alive(&mut self, id: i64) -> Result<()> {
        let (id, payload) = packets::encode_play_keep_alive(id);
        self.conn.send_packet(id, &payload).await
    }

    pub async fn respond_play_ping(&mut self, id: i32) -> Result<()> {
        let (id, payload) = packets::encode_play_pong(id);
        self.conn.send_packet(id, &payload).await
    }

    pub async fn on_play_chunk_batch_finished(&mut self, batch_size: i32) -> Result<()> {
        let desired_chunks_per_tick = self.chunk_batch_size.on_batch_finished(batch_size);
        let (id, payload) = packets::encode_play_chunk_batch_received(desired_chunks_per_tick);
        self.conn.send_packet(id, &payload).await
    }

    pub async fn respond_play_cookie_request(&mut self, key: &str) -> Result<bool> {
        let payload = self.server_cookies.get(key).map(Vec::as_slice);
        let payload_present = payload.is_some();
        let (id, response) = packets::encode_play_cookie_response(key, payload)?;
        self.conn.send_packet(id, &response).await?;
        Ok(payload_present)
    }

    pub fn store_cookie(&mut self, key: String, payload: Vec<u8>) -> (String, usize, usize) {
        let payload_len = payload.len();
        if payload_len <= MAX_SERVER_COOKIES_TOTAL_BYTES {
            self.server_cookies.remove(&key);
            let mut total_bytes: usize = self.server_cookies.values().map(Vec::len).sum();
            while !self.server_cookies.is_empty()
                && (self.server_cookies.len() >= MAX_SERVER_COOKIES
                    || total_bytes.saturating_add(payload_len) > MAX_SERVER_COOKIES_TOTAL_BYTES)
            {
                let Some((_, removed)) = self.server_cookies.pop_first() else {
                    break;
                };
                total_bytes = total_bytes.saturating_sub(removed.len());
            }
            self.server_cookies.insert(key.clone(), payload);
        }
        (key, payload_len, self.server_cookies.len())
    }

    pub async fn respond_play_resource_pack_push(
        &mut self,
        update: &packets::ResourcePackPush,
        policy: ServerResourcePackPolicy,
    ) -> Result<Option<ResourcePackResponseAction>> {
        let action = response_action_for_push_with_policy(update, policy);
        if let Some(action) = action {
            let (id, payload) = packets::encode_play_resource_pack_response(update.id, action);
            self.conn.send_packet(id, &payload).await?;
        }
        Ok(action)
    }

    pub async fn step_play_packet(
        &mut self,
        packet: PlayClientbound,
        resource_pack_policy: ServerResourcePackPolicy,
    ) -> Result<PlayPacketStep> {
        match packet {
            PlayClientbound::BundleDelimiter => Ok(PlayPacketStep::Handled),
            PlayClientbound::KeepAlive { id } => {
                // Vanilla answers keep-alives on the connection thread
                // (ClientCommonPacketListenerImpl.handleKeepAlive has no
                // ensureRunningOnSameThread), so a stalled render loop must
                // not cost the session its liveness.
                self.respond_play_keep_alive(id).await?;
                Ok(PlayPacketStep::Handled)
            }
            PlayClientbound::Ping { id } => {
                self.respond_play_ping(id).await?;
                Ok(PlayPacketStep::Handled)
            }
            PlayClientbound::ChunkBatchStart => {
                self.chunk_batch_size.on_batch_start();
                Ok(PlayPacketStep::ChunkBatchStart)
            }
            PlayClientbound::ChunkBatchFinished { batch_size } => {
                self.on_play_chunk_batch_finished(batch_size).await?;
                Ok(PlayPacketStep::ChunkBatchFinished { batch_size })
            }
            PlayClientbound::CookieRequest(request) => {
                let response_payload_present =
                    self.respond_play_cookie_request(&request.key).await?;
                Ok(PlayPacketStep::CookieRequest {
                    key: request.key,
                    response_payload_present,
                })
            }
            PlayClientbound::StoreCookie(cookie) => {
                let (key, payload_len, stored_cookie_count) =
                    self.store_cookie(cookie.key, cookie.payload);
                Ok(PlayPacketStep::StoreCookie {
                    key,
                    payload_len,
                    stored_cookie_count,
                })
            }
            PlayClientbound::StartConfiguration => Ok(PlayPacketStep::StartConfiguration),
            PlayClientbound::Disconnect(disconnect) => Err(remote_disconnect_error(
                "play",
                disconnect.reason,
                disconnect.styled_reason,
            )),
            PlayClientbound::ResourcePackPush(update) => {
                let action = self
                    .respond_play_resource_pack_push(&update, resource_pack_policy)
                    .await?;
                Ok(PlayPacketStep::ResourcePackPush { update, action })
            }
            PlayClientbound::Transfer(update) => Ok(PlayPacketStep::Transfer(update)),
            PlayClientbound::PlayerPosition(update) => Ok(PlayPacketStep::PlayerPosition(update)),
            PlayClientbound::PlayerRotation(update) => Ok(PlayPacketStep::PlayerRotation(update)),
            PlayClientbound::SetHeldSlot(update) => {
                self.observe_play_set_held_slot(update.slot);
                Ok(PlayPacketStep::SetHeldSlot(update))
            }
            PlayClientbound::Unknown { packet_id, len } => {
                Ok(PlayPacketStep::Unknown { packet_id, len })
            }
            other => Ok(PlayPacketStep::PassThrough(other)),
        }
    }

    pub fn observe_play_set_held_slot(&mut self, slot: i32) {
        if (0..=8).contains(&slot) {
            self.conn
                .remember_serverbound_carried_hotbar_slot(slot as u8);
        }
    }

    pub fn record_player_movement_flags(&mut self, on_ground: bool, horizontal_collision: bool) {
        self.player_movement_flags = PlayerMovementFlags {
            on_ground,
            horizontal_collision,
        };
    }

    pub async fn send_play_chat_acknowledgement(
        &mut self,
        command: packets::ChatAcknowledgement,
    ) -> Result<()> {
        let (id, payload) = packets::encode_play_chat_acknowledgement(command);
        self.conn.send_packet(id, &payload).await
    }

    pub async fn send_play_move_vehicle(
        &mut self,
        command: packets::MoveVehicleCommand,
    ) -> Result<()> {
        let (id, payload) = packets::encode_play_move_vehicle(command);
        self.conn.send_packet(id, &payload).await
    }

    pub async fn send_play_player_pos_rot(
        &mut self,
        state: PlayerPositionState,
        on_ground: bool,
        horizontal_collision: bool,
    ) -> Result<()> {
        self.player_position_state = state;
        self.record_player_movement_flags(on_ground, horizontal_collision);
        let (id, payload) = packets::encode_play_move_player_pos_rot(
            state.position.x,
            state.position.y,
            state.position.z,
            state.y_rot,
            state.x_rot,
            on_ground,
            horizontal_collision,
        );
        self.conn.send_packet(id, &payload).await
    }

    pub async fn send_play_player_loaded(&mut self) -> Result<()> {
        let (id, payload) = packets::encode_play_player_loaded();
        self.conn.send_packet(id, &payload).await
    }

    /// Sends `ServerboundMovePlayerPacket.Rot(y_rot, x_rot, false, false)` after
    /// the main thread applies `ClientboundPlayerRotationPacket` to canonical
    /// player rotation (Vanilla 26.1 `ClientPacketListener.handleRotatePlayer`).
    pub async fn acknowledge_player_rotation(&mut self, y_rot: f32, x_rot: f32) -> Result<()> {
        self.player_position_state.y_rot = y_rot;
        self.player_position_state.x_rot = x_rot;
        self.record_player_movement_flags(false, false);
        let (id, payload) = packets::encode_play_move_player_rot(y_rot, x_rot, false, false);
        self.conn.send_packet(id, &payload).await
    }

    pub async fn send_teleport_acknowledgement(&mut self, teleport_id: i32) -> Result<()> {
        let (id, payload) = packets::encode_play_accept_teleportation(teleport_id);
        self.conn.send_packet(id, &payload).await?;
        self.send_play_player_pos_rot(self.player_position_state, false, false)
            .await
    }

    pub async fn accept_teleport(&mut self, command: AcceptTeleportCommand) -> Result<()> {
        self.player_position_state = command.state;
        self.send_teleport_acknowledgement(command.teleport_id)
            .await?;
        self.pending_teleports = self.pending_teleports.saturating_sub(1);
        Ok(())
    }

    pub async fn acknowledge_start_configuration_with_chat(
        &mut self,
        pending_chat_acknowledgement: Option<packets::ChatAcknowledgement>,
    ) -> Result<()> {
        if let Some(command) = pending_chat_acknowledgement {
            self.send_play_chat_acknowledgement(command).await?;
        }
        self.acknowledge_start_configuration().await
    }

    pub async fn acknowledge_start_configuration(&mut self) -> Result<()> {
        let (id, payload) = packets::encode_play_configuration_acknowledged();
        self.conn.send_packet(id, &payload).await?;
        self.conn.exit_play_phase();
        self.state = ConnectionState::Configuration;
        self.seen_code_of_conduct = false;
        // Reconfiguration abandons the play session's pending teleports, movement flags, and synced content registry.
        self.pending_teleports = 0;
        self.player_movement_flags = PlayerMovementFlags::default();
        self.reset_content_registry();
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.play_tick = None;
        }
        Ok(())
    }

    pub async fn reconnect_for_transfer(
        &mut self,
        options: &ConnectionOptions,
        transfer: Transfer,
    ) -> Result<()> {
        let port = u16::try_from(transfer.port)
            .with_context(|| format!("invalid transfer port {}", transfer.port))?;
        let host = transfer.host.try_to_string()?;
        let address = transfer_socket_address(&host, port);
        let (host, port) = crate::types::split_host_port(&address)?;
        // Vanilla `ClientCommonPacketListenerImpl.handleTransfer` disconnects and
        // closes the old connection before connecting to the transfer target.
        self.conn.shutdown().await;
        let conn = RawConnection::connect_with_gateway(
            &address,
            options.websocket_gateway.as_deref(),
            options.timeout,
            None,
        )
        .await?;

        // Session cookies intentionally remain on the context, matching the
        // vanilla TransferState carry behavior, while ContentRegistry resets.
        self.conn = conn;
        self.state = ConnectionState::Login;
        self.player_position_state = PlayerPositionState::default();
        self.player_movement_flags = PlayerMovementFlags::default();
        self.chunk_batch_size = ChunkBatchSizeCalculator::new();
        self.pending_teleports = 0;
        self.seen_code_of_conduct = false;
        self.reset_content_registry();
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.play_tick = None;
        }

        self.start_login(
            &host,
            port,
            ClientIntent::Transfer,
            &options.username,
            options.profile_id,
        )
        .await
    }
}

fn transfer_socket_address(host: &str, port: u16) -> String {
    if host.parse::<Ipv6Addr>().is_ok() {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use tokio::net::{TcpListener, TcpStream};

    #[tokio::test]
    async fn store_cookie_enforces_entry_and_byte_caps_and_reconfig_exits_play_phase() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let client_stream = TcpStream::connect(addr).await.unwrap();
        let (_server_stream, _) = listener.accept().await.unwrap();
        let mut session =
            ClientProtocolSession::new(RawConnection::from_stream(client_stream, None));

        // Entry cap: storing more than MAX_SERVER_COOKIES evicts old keys.
        for i in 0..(MAX_SERVER_COOKIES + 20) {
            session.store_cookie(format!("bbb:key_{i:04}"), vec![1; 16]);
        }
        assert!(session.server_cookies.len() <= MAX_SERVER_COOKIES);

        // Byte cap: total stored bytes never exceed MAX_SERVER_COOKIES_TOTAL_BYTES.
        for i in 0..80 {
            session.store_cookie(format!("bbb:big_{i:04}"), vec![7; 5120]);
        }
        let total_bytes: usize = session.server_cookies.values().map(Vec::len).sum();
        assert!(total_bytes <= MAX_SERVER_COOKIES_TOTAL_BYTES);

        // Reconfiguration exits play phase so later configuration sends aren't counted as play ids.
        session.conn.enter_play_phase();
        session.acknowledge_start_configuration().await.unwrap();
        let count_after_ack = session.conn.serverbound_play_ids().len();
        session.conn.send_packet(0x77, b"cfg").await.unwrap();
        assert_eq!(session.conn.serverbound_play_ids().len(), count_after_ack);
    }
}
