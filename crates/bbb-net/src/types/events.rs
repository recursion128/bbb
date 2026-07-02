use bbb_protocol::packets::{
    CustomPayload, CustomReportDetails, KnownPack, PlayClientbound, RegistryData, ResourcePackPop,
    ResourcePackPush, ResourcePackResponseAction, ServerLinks, ShowDialog, Transfer,
    UpdateEnabledFeatures, UpdateTags,
};
use tokio::sync::oneshot;

use super::ConnectionState;

/// Events surfaced by the online event stream.
///
/// Clientbound play packets travel as [`NetEvent::Play`] and are applied
/// through `WorldStore::apply_play_packet`; dedicated variants exist only for
/// connection lifecycle, login/configuration-state packets, and events whose
/// payload is derived from connection state (cookies, resource-pack
/// responses).
#[derive(Debug)]
pub enum NetEvent {
    Connected,
    Disconnected {
        reason: Option<String>,
    },
    StateChanged {
        state: ConnectionState,
    },
    StartConfiguration {
        pending_chat_acknowledgement:
            oneshot::Sender<Option<bbb_protocol::packets::ChatAcknowledgement>>,
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
