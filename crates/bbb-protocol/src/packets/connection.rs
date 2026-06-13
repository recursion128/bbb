use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    client_common::{self, CustomPayload, ShowDialog},
    server_presentation::{self, ResourcePackPop, ResourcePackPush},
    tags::UpdateTags,
};

use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    component::decode_component_summary_from_decoder,
    ids, PROTOCOL_VERSION,
};

const MAX_COOKIE_PAYLOAD_SIZE: usize = 5120;
const MAX_CUSTOM_REPORT_DETAILS: usize = 32;
const MAX_SERVER_LINKS_INITIAL_CAPACITY: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum ClientIntent {
    Status = 1,
    Login = 2,
    Transfer = 3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameProfile {
    pub uuid: Uuid,
    pub name: String,
    pub properties: Vec<GameProfileProperty>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameProfileProperty {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoginClientbound {
    Disconnect { raw_json: String },
    EncryptionRequest,
    LoginFinished { profile: GameProfile },
    SetCompression { threshold: i32 },
    CustomQuery { transaction_id: i32 },
    CookieRequest(CookieRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigurationClientbound {
    Disconnect {
        reason: String,
        raw_reason: Vec<u8>,
    },
    CustomPayload(CustomPayload),
    Finish,
    KeepAlive {
        id: i64,
    },
    Ping {
        id: i32,
    },
    ResetChat,
    RegistryData {
        registry: String,
        raw_payload_len: usize,
    },
    ResourcePackPop(ResourcePackPop),
    ResourcePackPush(ResourcePackPush),
    SelectKnownPacks {
        known_packs: Vec<KnownPack>,
    },
    UpdateEnabledFeatures(UpdateEnabledFeatures),
    CookieRequest(CookieRequest),
    StoreCookie(StoreCookie),
    Transfer(Transfer),
    UpdateTags(UpdateTags),
    CustomReportDetails(CustomReportDetails),
    ServerLinks(ServerLinks),
    ClearDialog,
    ShowDialog(ShowDialog),
    CodeOfConduct {
        text: String,
    },
    Unknown {
        packet_id: i32,
        len: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transfer {
    pub host: String,
    pub port: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CookieRequest {
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreCookie {
    pub key: String,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownPack {
    pub namespace: String,
    pub id: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateEnabledFeatures {
    pub features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomReportDetails {
    pub details: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerLinks {
    pub links: Vec<ServerLinkEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerLinkEntry {
    pub link_type: ServerLinkType,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerLinkType {
    Known(ServerLinkKnownType),
    Custom { label: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerLinkKnownType {
    BugReport,
    CommunityGuidelines,
    Support,
    Status,
    Feedback,
    Community,
    Website,
    Forums,
    News,
    Announcements,
}

impl ServerLinkKnownType {
    pub fn vanilla_name(self) -> &'static str {
        match self {
            Self::BugReport => "report_bug",
            Self::CommunityGuidelines => "community_guidelines",
            Self::Support => "support",
            Self::Status => "status",
            Self::Feedback => "feedback",
            Self::Community => "community",
            Self::Website => "website",
            Self::Forums => "forums",
            Self::News => "news",
            Self::Announcements => "announcements",
        }
    }

    fn from_vanilla_id(id: i32) -> Self {
        match id {
            1 => Self::CommunityGuidelines,
            2 => Self::Support,
            3 => Self::Status,
            4 => Self::Feedback,
            5 => Self::Community,
            6 => Self::Website,
            7 => Self::Forums,
            8 => Self::News,
            9 => Self::Announcements,
            _ => Self::BugReport,
        }
    }
}

pub fn encode_handshake(host: &str, port: u16, intent: ClientIntent) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(PROTOCOL_VERSION);
    out.write_string(host);
    out.write_u16(port);
    out.write_var_i32(intent as i32);
    (
        ids::handshake::SERVERBOUND_CLIENT_INTENTION,
        out.into_inner(),
    )
}

pub fn encode_status_request() -> (i32, Vec<u8>) {
    (ids::status::SERVERBOUND_STATUS_REQUEST, Vec::new())
}

pub fn encode_ping_request(time: i64) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i64(time);
    (ids::status::SERVERBOUND_PING_REQUEST, out.into_inner())
}

pub fn decode_status_response(payload: &[u8]) -> Result<String> {
    Decoder::new(payload).read_string(32767)
}

pub fn decode_pong_response(payload: &[u8]) -> Result<i64> {
    Decoder::new(payload).read_i64()
}

pub fn encode_login_hello(username: &str, profile_id: Uuid) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_string(username);
    out.write_uuid(profile_id);
    (ids::login::SERVERBOUND_HELLO, out.into_inner())
}

pub fn encode_login_acknowledged() -> (i32, Vec<u8>) {
    (ids::login::SERVERBOUND_LOGIN_ACKNOWLEDGED, Vec::new())
}

pub fn decode_login_clientbound(packet_id: i32, payload: &[u8]) -> Result<LoginClientbound> {
    match packet_id {
        ids::login::CLIENTBOUND_LOGIN_DISCONNECT => Ok(LoginClientbound::Disconnect {
            raw_json: Decoder::new(payload).read_string(262144)?,
        }),
        ids::login::CLIENTBOUND_HELLO => Ok(LoginClientbound::EncryptionRequest),
        ids::login::CLIENTBOUND_LOGIN_FINISHED => {
            let mut decoder = Decoder::new(payload);
            Ok(LoginClientbound::LoginFinished {
                profile: decode_game_profile(&mut decoder)?,
            })
        }
        ids::login::CLIENTBOUND_LOGIN_COMPRESSION => Ok(LoginClientbound::SetCompression {
            threshold: Decoder::new(payload).read_var_i32()?,
        }),
        ids::login::CLIENTBOUND_CUSTOM_QUERY => Ok(LoginClientbound::CustomQuery {
            transaction_id: Decoder::new(payload).read_var_i32()?,
        }),
        ids::login::CLIENTBOUND_COOKIE_REQUEST => {
            let mut decoder = Decoder::new(payload);
            Ok(LoginClientbound::CookieRequest(decode_cookie_request(
                &mut decoder,
            )?))
        }
        id => Err(ProtocolError::UnknownPacket { state: "login", id }),
    }
}

pub fn encode_login_cookie_response(key: &str, payload: Option<&[u8]>) -> (i32, Vec<u8>) {
    (
        ids::login::SERVERBOUND_COOKIE_RESPONSE,
        encode_cookie_response_payload(key, payload),
    )
}

pub fn encode_client_information_default() -> (i32, Vec<u8>) {
    (
        ids::configuration::SERVERBOUND_CLIENT_INFORMATION,
        encode_client_information_payload_default(),
    )
}

pub(super) fn encode_client_information_payload_default() -> Vec<u8> {
    let mut out = Encoder::new();
    out.write_string("en_us");
    out.write_i8(10);
    out.write_var_i32(0);
    out.write_bool(true);
    out.write_u8(0x7f);
    out.write_var_i32(1);
    out.write_bool(false);
    out.write_bool(false);
    out.write_var_i32(0);
    out.into_inner()
}

pub fn encode_configuration_finish() -> (i32, Vec<u8>) {
    (
        ids::configuration::SERVERBOUND_FINISH_CONFIGURATION,
        Vec::new(),
    )
}

pub fn encode_configuration_keep_alive(id: i64) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i64(id);
    (ids::configuration::SERVERBOUND_KEEP_ALIVE, out.into_inner())
}

pub fn encode_configuration_cookie_response(key: &str, payload: Option<&[u8]>) -> (i32, Vec<u8>) {
    (
        ids::configuration::SERVERBOUND_COOKIE_RESPONSE,
        encode_cookie_response_payload(key, payload),
    )
}

pub fn encode_configuration_pong(id: i32) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i32(id);
    (ids::configuration::SERVERBOUND_PONG, out.into_inner())
}

pub fn encode_select_known_packs_empty() -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(0);
    (
        ids::configuration::SERVERBOUND_SELECT_KNOWN_PACKS,
        out.into_inner(),
    )
}

pub fn decode_configuration_clientbound(
    packet_id: i32,
    payload: &[u8],
) -> Result<ConfigurationClientbound> {
    match packet_id {
        ids::configuration::CLIENTBOUND_COOKIE_REQUEST => {
            let mut decoder = Decoder::new(payload);
            Ok(ConfigurationClientbound::CookieRequest(
                decode_cookie_request(&mut decoder)?,
            ))
        }
        ids::configuration::CLIENTBOUND_CUSTOM_PAYLOAD => {
            let mut decoder = Decoder::new(payload);
            Ok(ConfigurationClientbound::CustomPayload(
                client_common::decode_custom_payload(&mut decoder)?,
            ))
        }
        ids::configuration::CLIENTBOUND_DISCONNECT => {
            let mut decoder = Decoder::new(payload);
            Ok(ConfigurationClientbound::Disconnect {
                reason: decode_component_summary_from_decoder(&mut decoder)?,
                raw_reason: payload.to_vec(),
            })
        }
        ids::configuration::CLIENTBOUND_FINISH_CONFIGURATION => {
            Ok(ConfigurationClientbound::Finish)
        }
        ids::configuration::CLIENTBOUND_KEEP_ALIVE => Ok(ConfigurationClientbound::KeepAlive {
            id: Decoder::new(payload).read_i64()?,
        }),
        ids::configuration::CLIENTBOUND_PING => Ok(ConfigurationClientbound::Ping {
            id: Decoder::new(payload).read_i32()?,
        }),
        ids::configuration::CLIENTBOUND_RESET_CHAT => Ok(ConfigurationClientbound::ResetChat),
        ids::configuration::CLIENTBOUND_REGISTRY_DATA => {
            let mut decoder = Decoder::new(payload);
            let registry = decoder.read_string(32767)?;
            Ok(ConfigurationClientbound::RegistryData {
                registry,
                raw_payload_len: payload.len(),
            })
        }
        ids::configuration::CLIENTBOUND_RESOURCE_PACK_POP => {
            let mut decoder = Decoder::new(payload);
            Ok(ConfigurationClientbound::ResourcePackPop(
                server_presentation::decode_resource_pack_pop(&mut decoder)?,
            ))
        }
        ids::configuration::CLIENTBOUND_RESOURCE_PACK_PUSH => {
            let mut decoder = Decoder::new(payload);
            Ok(ConfigurationClientbound::ResourcePackPush(
                server_presentation::decode_resource_pack_push(&mut decoder)?,
            ))
        }
        ids::configuration::CLIENTBOUND_SELECT_KNOWN_PACKS => {
            let mut decoder = Decoder::new(payload);
            Ok(ConfigurationClientbound::SelectKnownPacks {
                known_packs: decode_known_packs(&mut decoder)?,
            })
        }
        ids::configuration::CLIENTBOUND_UPDATE_ENABLED_FEATURES => {
            let mut decoder = Decoder::new(payload);
            Ok(ConfigurationClientbound::UpdateEnabledFeatures(
                decode_update_enabled_features(&mut decoder)?,
            ))
        }
        ids::configuration::CLIENTBOUND_STORE_COOKIE => {
            let mut decoder = Decoder::new(payload);
            Ok(ConfigurationClientbound::StoreCookie(decode_store_cookie(
                &mut decoder,
            )?))
        }
        ids::configuration::CLIENTBOUND_TRANSFER => {
            let mut decoder = Decoder::new(payload);
            Ok(ConfigurationClientbound::Transfer(decode_transfer(
                &mut decoder,
            )?))
        }
        ids::configuration::CLIENTBOUND_UPDATE_TAGS => {
            let mut decoder = Decoder::new(payload);
            Ok(ConfigurationClientbound::UpdateTags(
                super::tags::decode_update_tags(&mut decoder)?,
            ))
        }
        ids::configuration::CLIENTBOUND_CUSTOM_REPORT_DETAILS => {
            let mut decoder = Decoder::new(payload);
            Ok(ConfigurationClientbound::CustomReportDetails(
                decode_custom_report_details(&mut decoder)?,
            ))
        }
        ids::configuration::CLIENTBOUND_SERVER_LINKS => {
            let mut decoder = Decoder::new(payload);
            Ok(ConfigurationClientbound::ServerLinks(decode_server_links(
                &mut decoder,
            )?))
        }
        ids::configuration::CLIENTBOUND_CLEAR_DIALOG => {
            let decoder = Decoder::new(payload);
            client_common::decode_clear_dialog(&decoder)?;
            Ok(ConfigurationClientbound::ClearDialog)
        }
        ids::configuration::CLIENTBOUND_SHOW_DIALOG => {
            let mut decoder = Decoder::new(payload);
            Ok(ConfigurationClientbound::ShowDialog(
                client_common::decode_context_free_show_dialog(&mut decoder)?,
            ))
        }
        ids::configuration::CLIENTBOUND_CODE_OF_CONDUCT => {
            let mut decoder = Decoder::new(payload);
            Ok(ConfigurationClientbound::CodeOfConduct {
                text: decoder.read_string(32767)?,
            })
        }
        id => Ok(ConfigurationClientbound::Unknown {
            packet_id: id,
            len: payload.len(),
        }),
    }
}

fn decode_known_packs(decoder: &mut Decoder<'_>) -> Result<Vec<KnownPack>> {
    let count = decoder.read_len()?;
    let mut packs = Vec::with_capacity(count);
    for _ in 0..count {
        packs.push(KnownPack {
            namespace: decoder.read_string(32767)?,
            id: decoder.read_string(32767)?,
            version: decoder.read_string(32767)?,
        });
    }
    Ok(packs)
}

fn decode_update_enabled_features(decoder: &mut Decoder<'_>) -> Result<UpdateEnabledFeatures> {
    let count = decoder.read_len()?;
    let mut features = Vec::with_capacity(count);
    for _ in 0..count {
        features.push(decoder.read_string(32767)?);
    }
    features.sort();
    Ok(UpdateEnabledFeatures { features })
}

pub(super) fn decode_cookie_request(decoder: &mut Decoder<'_>) -> Result<CookieRequest> {
    Ok(CookieRequest {
        key: decoder.read_string(32767)?,
    })
}

pub(super) fn decode_store_cookie(decoder: &mut Decoder<'_>) -> Result<StoreCookie> {
    let key = decoder.read_string(32767)?;
    let len = decoder.read_len()?;
    if len > MAX_COOKIE_PAYLOAD_SIZE {
        return Err(ProtocolError::PacketTooLarge(len, MAX_COOKIE_PAYLOAD_SIZE));
    }
    Ok(StoreCookie {
        key,
        payload: decoder.read_exact(len, "cookie payload")?.to_vec(),
    })
}

pub(super) fn decode_transfer(decoder: &mut Decoder<'_>) -> Result<Transfer> {
    Ok(Transfer {
        host: decoder.read_string(32767)?,
        port: decoder.read_var_i32()?,
    })
}

pub(super) fn decode_custom_report_details(
    decoder: &mut Decoder<'_>,
) -> Result<CustomReportDetails> {
    let count = decoder.read_len()?;
    if count > MAX_CUSTOM_REPORT_DETAILS {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_CUSTOM_REPORT_DETAILS,
        ));
    }

    let mut details = BTreeMap::new();
    for _ in 0..count {
        let key = decoder.read_string(128)?;
        let value = decoder.read_string(4096)?;
        details.insert(key, value);
    }
    Ok(CustomReportDetails { details })
}

pub(super) fn decode_server_links(decoder: &mut Decoder<'_>) -> Result<ServerLinks> {
    let count = decoder.read_len()?;
    let mut links = Vec::with_capacity(count.min(MAX_SERVER_LINKS_INITIAL_CAPACITY));
    for _ in 0..count {
        links.push(decode_server_link_entry(decoder)?);
    }
    Ok(ServerLinks { links })
}

fn decode_server_link_entry(decoder: &mut Decoder<'_>) -> Result<ServerLinkEntry> {
    let link_type = if decoder.read_bool()? {
        ServerLinkType::Known(ServerLinkKnownType::from_vanilla_id(
            decoder.read_var_i32()?,
        ))
    } else {
        ServerLinkType::Custom {
            label: decode_component_summary_from_decoder(decoder)?,
        }
    };
    Ok(ServerLinkEntry {
        link_type,
        url: decoder.read_string(32767)?,
    })
}

pub(super) fn encode_cookie_response_payload(key: &str, payload: Option<&[u8]>) -> Vec<u8> {
    let mut out = Encoder::new();
    out.write_string(key);
    if let Some(payload) = payload {
        out.write_bool(true);
        out.write_var_i32(payload.len() as i32);
        out.write_bytes(payload);
    } else {
        out.write_bool(false);
    }
    out.into_inner()
}

fn decode_game_profile(decoder: &mut Decoder<'_>) -> Result<GameProfile> {
    let uuid = decoder.read_uuid()?;
    let name = decoder.read_string(16)?;
    let property_count = decoder.read_len()?;
    let mut properties = Vec::with_capacity(property_count);
    for _ in 0..property_count {
        let name = decoder.read_string(32767)?;
        let value = decoder.read_string(32767)?;
        let signature = if decoder.read_bool()? {
            Some(decoder.read_string(32767)?)
        } else {
            None
        };
        properties.push(GameProfileProperty {
            name,
            value,
            signature,
        });
    }
    Ok(GameProfile {
        uuid,
        name,
        properties,
    })
}
