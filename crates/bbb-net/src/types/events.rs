use bbb_pack::KnownPackDataSource;
use bbb_protocol::packets::{
    CustomPayload, CustomReportDetails, KnownPack, PlayClientbound, PlayerPositionUpdate,
    RegistryData, ResourcePackPop, ResourcePackPush, ResourcePackResponseAction, ServerLinks,
    ShowDialog, Transfer, UpdateEnabledFeatures, UpdateTags,
};

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
    /// Transport security for the play session announced immediately before
    /// its clientbound Login packet. Reconfiguration can begin another play
    /// session on the same encrypted connection, so every Login restates it.
    PlaySessionSecurity {
        encrypted: bool,
    },
    /// RSA-4096 public keys acquired from the Minecraft Services key set.
    /// Property keys validate profile textures; certificate keys validate
    /// player profile keys before retaining remote chat sessions.
    ServicesPublicKeys {
        profile_property_keys: Vec<Vec<u8>>,
        player_certificate_keys: Vec<Vec<u8>>,
    },
    /// The server signalled play-to-configuration re-entry; the canonical
    /// world answers with [`crate::NetCommand::AcknowledgeConfiguration`]
    /// carrying any pending chat acknowledgement, and the connection stays
    /// live (reading packets) until that answer arrives.
    StartConfiguration,
    /// A position correction; the canonical world applies it and answers
    /// with [`crate::NetCommand::AcceptTeleport`] carrying the post-apply
    /// transform.  Outgoing player moves are gated until the accept is on
    /// the wire, so a slow canonical apply can never leak stale positions
    /// or stall packet reading.
    PlayerPosition {
        update: PlayerPositionUpdate,
    },
    CompressionSet {
        threshold: i32,
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
