use bbb_pack::KnownPackDataSource;
use bbb_protocol::packets::{
    CustomPayload, CustomReportDetails, KnownPack, PlayClientbound, PlayerPositionState,
    PlayerPositionUpdate, RegistryData, ResourcePackPop, ResourcePackPush,
    ResourcePackResponseAction, ServerLinks, ShowDialog, Transfer, UpdateEnabledFeatures,
    UpdateTags,
};
use std::time::Instant;
use tokio::sync::oneshot;

use super::ConnectionState;

/// Events surfaced by the online event stream.
///
/// Clientbound play packets travel as [`NetEvent::Play`] and are applied
/// through `WorldStore::apply_play_packet`; dedicated variants exist only for
/// connection lifecycle, login/configuration-state packets, and events whose
/// response or payload requires connection or canonical world state.
#[derive(Debug)]
pub enum NetEvent {
    Connected,
    Disconnected {
        reason: Option<String>,
    },
    StateChanged {
        state: ConnectionState,
    },
    /// RSA-4096 public keys acquired from the Minecraft Services key set.
    /// World state uses them to validate player profile keys before retaining
    /// remote chat sessions.
    ServicesProfileKeys {
        public_keys: Vec<Vec<u8>>,
    },
    StartConfiguration {
        pending_chat_acknowledgement:
            oneshot::Sender<Option<bbb_protocol::packets::ChatAcknowledgement>>,
    },
    /// A position correction that must be canonically applied before the
    /// connection can echo the resulting player transform to the server.
    PlayerPosition {
        update: PlayerPositionUpdate,
        acknowledged_state: oneshot::Sender<Option<PlayerPositionState>>,
    },
    CompressionSet {
        threshold: i32,
    },
    KeepAlive {
        id: i64,
        expires_at: Instant,
    },
    PacketSeen {
        state: ConnectionState,
        packet_id: i32,
        len: usize,
    },
    UnsupportedPacket {
        state: ConnectionState,
        packet_id: i32,
        len: usize,
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
    ResourcePackResponse {
        id: uuid::Uuid,
        action: ResourcePackResponseAction,
    },
    SelectKnownPacks {
        known_packs: Vec<KnownPack>,
        selected_packs: Vec<KnownPack>,
        data_source: KnownPackDataSource,
    },
    CodeOfConduct {
        text: String,
    },
    // Configuration-state packets with world-owned application.
    RegistryData(RegistryData),
    ResetChat,
    UpdateEnabledFeatures(UpdateEnabledFeatures),
    UpdateTags(UpdateTags),
    CustomPayload(CustomPayload),
    CustomReportDetails(CustomReportDetails),
    ServerLinks(ServerLinks),
    Transfer(Transfer),
    ShowDialog(ShowDialog),
    ClearDialog,
    ResourcePackPush(ResourcePackPush),
    ResourcePackPop(ResourcePackPop),
    /// A decoded clientbound play packet; applied to canonical world state via
    /// `WorldStore::apply_play_packet`.
    Play(PlayClientbound),
}
